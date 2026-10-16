// rv-doom browser front end. Talks to rv_doom_web.wasm through plain exports + linear memory.
"use strict";

const PROGRAMS = [
  ["doom.bin", "Doom (shareware)"],
  ["programs/mandel.bin", "Mandelbrot (UART)"],
  ["programs/primes.bin", "Primes (UART)"],
  ["programs/hello.bin", "Hello (UART)"],
  ["programs/fbtest.bin", "Framebuffer test"],
];
const ABI = ["zero","ra","sp","gp","tp","t0","t1","t2","s0","s1","a0","a1","a2","a3","a4","a5",
             "a6","a7","s2","s3","s4","s5","s6","s7","s8","s9","s10","s11","t3","t4","t5","t6"];

// Browser key -> Doom keycode (doomkeys.h). Letters/digits map to lowercase ASCII.
const KEYS = {
  ArrowUp: 0xad, ArrowDown: 0xaf, ArrowLeft: 0xac, ArrowRight: 0xae,
  Enter: 13, Escape: 27, Tab: 9, Backspace: 0x7f, " ": 0xa2 /* use */,
  Control: 0xa3 /* fire */, Shift: 0xb6 /* run */, Alt: 0xb8 /* strafe */,
  "-": 0x2d, "=": 0x3d,
};
const doomKey = (e) => KEYS[e.key] ?? (e.key.length === 1 ? e.key.toLowerCase().charCodeAt(0) : 0);

const $ = (id) => document.getElementById(id);
const canvas = $("screen");
const ctx = canvas.getContext("2d");
const image = ctx.createImageData(320, 200);
const decoder = new TextDecoder();

let wasm, running = true, halted = false;
const held = new Set();  // Doom keycodes currently down in the guest
let lastStats = { t: 0, instret: 0, frames: 0 };
let prevRegs = new Uint32Array(33);

// Debug overlay (toggle with the ` key, or start with ?debug). Measurements always run; the
// latest snapshot is also exposed as window.rvDebug for the headless tests.
const DEBUG_KEY = "Backquote";  // unused by Doom; never forwarded to the guest
const dbg = {
  on: new URLSearchParams(location.search).has("debug"),
  gameticAddr: 0,
  keys: [], sent: 0,                       // key log, events queued since load
  win: null,                               // start of the current 1 s measurement window
  raf: 0, draws: 0, runMs: 0, maxJump: 0, lastTimer: 0,
  load: null,                              // {wall, timer} at program load
  tic0: null,                              // {wall, tic} when gametic was first seen non-zero
  snap: {},
};

const text = (len) => decoder.decode(new Uint8Array(wasm.memory.buffer, wasm.text_ptr(), len));

async function loadProgram(path) {
  const resp = await fetch(path);
  if (!resp.ok) throw new Error(`${path}: ${resp.status}`);
  const bytes = new Uint8Array(await resp.arrayBuffer());
  const ptr = wasm.upload_buffer(bytes.length);
  new Uint8Array(wasm.memory.buffer, ptr, bytes.length).set(bytes);
  if (wasm.load() !== 0) throw new Error("image does not fit in RAM");
  $("console").textContent = "";
  halted = false;
  dbg.isDoom = path === "doom.bin";
  dbg.load = null; dbg.tic0 = null; dbg.sent = 0; held.clear(); dbg.keys = []; dbg.win = null;
  lastStats = { t: performance.now(), instret: 0, frames: 0 };
  draw(); updatePanel();
}

function draw() {
  image.data.set(new Uint8Array(wasm.memory.buffer, wasm.frame_rgba(), 320 * 200 * 4));
  ctx.putImageData(image, 0, 0);
}

function drainConsole() {
  const out = text(wasm.take_uart());
  if (!out) return;
  const el = $("console");
  el.textContent = (el.textContent + out).slice(-20000);
  el.scrollTop = el.scrollHeight;
}

function buildRegs() {
  $("regs").innerHTML = ABI.map((name, i) =>
    `<div class="reg" id="r${i}"><span class="name">${name}</span><span class="val">00000000</span></div>`).join("");
}

function updatePanel() {
  const regs = new Uint32Array(wasm.memory.buffer, wasm.regs(), 33);
  for (let i = 0; i < 32; i++) {
    const el = $("r" + i);
    el.lastChild.textContent = regs[i].toString(16).padStart(8, "0");
    el.classList.toggle("changed", regs[i] !== prevRegs[i]);
  }
  $("pc").textContent = regs[32].toString(16).padStart(8, "0");
  prevRegs = regs.slice();
  $("disasm").textContent = text(wasm.disasm_current());
  $("status").textContent = text(wasm.status_text());
}

function updateStats(now) {
  const dt = (now - lastStats.t) / 1000;
  if (dt < 0.5) return;
  const instret = wasm.instret(), frames = wasm.frames();
  $("mips").textContent = ((instret - lastStats.instret) / dt / 1e6).toFixed(1);
  $("fps").textContent = Math.round((frames - lastStats.frames) / dt);
  $("instret").textContent = instret.toLocaleString("en-US");
  lastStats = { t: now, instret, frames };
}

function logKey(e, code, fate) {
  const mods = ["Ctrl", "Shift", "Alt", "Meta"].filter((m) => e[m.toLowerCase() + "Key"]).join("+");
  const entry = { t: performance.now(), type: e.type === "keydown" ? "down" : "up", code: e.code, key: e.key,
                  repeat: e.repeat, mods, doom: code, fate, seq: 0 };
  if (fate === "sent") entry.seq = ++dbg.sent;
  dbg.keys.push(entry);
  if (dbg.keys.length > 10) dbg.keys.shift();
}

function keyLine(k) {
  const t = ((k.t - (dbg.load?.wall ?? 0)) / 1000).toFixed(2).padStart(7);
  const doom = k.doom ? "0x" + k.doom.toString(16).padStart(2, "0") : "    ";
  const fate = k.fate !== "sent" ? "DROPPED (" + k.fate + ")"
    : wasm.keys_delivered() >= k.seq ? `delivered #${k.seq}` : `queued #${k.seq}`;
  return `${t}s ${k.type.padEnd(4)} ${k.code.padEnd(11)} ${(k.mods || "-").padEnd(10)} ${k.repeat ? "rep" : "   "} ${doom} ${fate}`;
}

// Called once per animation frame with the guest timer jump seen by this frame's slices.
function debugSample(sliceMs, newFrame) {
  const now = performance.now();  // not the rAF timestamp: it is stale on the first frame after a load
  if (!dbg.load) dbg.load = { wall: now, timer: wasm.timer_ms() };
  const w = dbg.win ??= { t: now, instret: wasm.instret(), frames: wasm.frames(), tic: 0 };
  dbg.raf++; dbg.runMs += sliceMs; if (newFrame) dbg.draws++;
  const tic = dbg.isDoom && dbg.gameticAddr ? wasm.peek(dbg.gameticAddr) : 0;
  if (tic && !dbg.tic0) dbg.tic0 = { wall: now, tic };
  if (!w.tic) w.tic = tic;
  const dt = (now - w.t) / 1000;
  if (dt >= 1) {
    const s = dbg.snap = {
      rafFps: dbg.raf / dt,                                   // animation frames the browser gave us
      drawFps: dbg.draws / dt,                                // canvas updates that showed a new guest frame
      guestFps: (wasm.frames() - w.frames) / dt,              // DG_DrawFrame calls
      mips: (wasm.instret() - w.instret) / dt / 1e6,
      cpuShare: dbg.runMs / (dt * 1000),                      // share of wall time spent inside wasm.run
      maxJumpMs: dbg.maxJump,                                 // largest guest-clock step between two run() calls
      clockDriftMs: (wasm.timer_ms() - dbg.load.timer) - (now - dbg.load.wall),
      ticsPerSec: tic && w.tic ? (tic - w.tic) / dt : null,
      gameDriftMs: dbg.tic0 ? (tic - dbg.tic0.tic) * 1000 / 35 - (now - dbg.tic0.wall) : null,
      gametic: tic,
      keys: dbg.keys.map(keyLine),
      held: [...held],
    };
    window.rvDebug = s;
    dbg.win = { t: now, instret: wasm.instret(), frames: wasm.frames(), tic };
    dbg.raf = dbg.draws = dbg.runMs = dbg.maxJump = 0;
  }
  if (dbg.on) renderDebug();
}

function renderDebug() {
  const s = dbg.snap, f = (v, d = 1) => (v == null ? "n/a" : v.toFixed(d));
  const el = $("debug");
  el.hidden = false;
  el.textContent = [
    `display  ${f(s.rafFps)} rAF/s   ${f(s.drawFps)} new frames drawn/s   guest ${f(s.guestFps)} fps`,
    `emulator ${f(s.mips)} MIPS   guest ran ${f(100 * s.cpuShare, 0)}% of wall time`,
    `timer    guest-vs-wall drift ${f(s.clockDriftMs, 0)} ms   max clock jump ${f(s.maxJumpMs, 0)} ms`,
    `doom     gametic ${s.gametic ?? "n/a"}   ${f(s.ticsPerSec)} tics/s (want 35)   game-time drift ${f(s.gameDriftMs, 0)} ms`,
    `held     ${s.held?.length ? s.held.map((c) => "0x" + c.toString(16)).join(" ") : "-"}`,
    `last ${dbg.keys.length} key events (t, type, code, modifiers, repeat, doom code, fate):`,
    ...dbg.keys.map(keyLine),
  ].join("\n");
}

let lastPanel = 0;
function tick(now) {
  if (running && !halted) {
    // Run in slices until ~12 ms of this animation frame are used.
    const start = performance.now(), frames0 = wasm.frames();
    do {
      const t = Math.floor(performance.now());
      if (dbg.lastTimer) dbg.maxJump = Math.max(dbg.maxJump, t - dbg.lastTimer);
      dbg.lastTimer = t;
      halted = wasm.run(100000, t) !== 0;
    } while (!halted && performance.now() - start < 12);
    debugSample(performance.now() - start, wasm.frames() !== frames0);
    draw();
    drainConsole();
    if (now - lastPanel > 100 || halted) { updatePanel(); lastPanel = now; }
  }
  updateStats(now);
  requestAnimationFrame(tick);
}

function setRunning(on) {
  running = on;
  $("run").textContent = on ? "Pause" : "Run";
  if (!on) updatePanel();
}

async function main() {
  const { instance } = await WebAssembly.instantiateStreaming(fetch("rv_doom_web.wasm"), {});
  wasm = instance.exports;
  buildRegs();

  const select = $("program");
  for (const [path, label] of PROGRAMS) select.add(new Option(label, path));
  // Blur after a pick so the dropdown never keeps focus (and arrow keys) during play.
  select.onchange = () => { select.blur(); canvas.focus(); loadProgram(select.value).catch(showError); };
  $("run").onclick = () => setRunning(!running);
  $("reset").onclick = () => loadProgram(select.value).catch(showError);
  $("step").onclick = () => {
    setRunning(false);
    if (!halted) halted = wasm.run(1, Math.floor(performance.now())) !== 0;
    draw(); drainConsole(); updatePanel();
  };

  // Game keys are routed at the document level in the capture phase, whatever has focus,
  // so a focused dropdown or button never sees (or acts on) them.
  for (const type of ["keydown", "keyup"]) {
    document.addEventListener(type, (e) => {
      if (e.code === DEBUG_KEY) {
        e.preventDefault();
        if (type === "keydown" && !e.repeat) { dbg.on = !dbg.on; $("debug").hidden = !dbg.on; if (dbg.on) renderDebug(); }
        return;
      }
      const code = doomKey(e);
      if (!code) return logKey(e, code, "unmapped");
      e.preventDefault();
      e.stopPropagation();
      const down = type === "keydown";
      // A repeat of a key we already released (e.g. held across a blur) counts as a fresh press.
      if (down && e.repeat && held.has(code)) return logKey(e, code, "repeat");
      if (!down && !held.has(code)) return logKey(e, code, "not held");
      logKey(e, code, "sent");
      if (down) held.add(code); else held.delete(code);
      wasm.key(code, down ? 1 : 0);
    }, { capture: true });
  }

  // Keyups that happen while the page has no focus are never delivered, so release everything.
  const releaseAll = (why) => {
    for (const code of held) {
      logKey({ type: "keyup", code: "(" + why + ")", key: "", repeat: false }, code, "sent");
      wasm.key(code, 0);
    }
    held.clear();
  };
  window.addEventListener("blur", () => releaseAll("blur"));
  window.addEventListener("pagehide", () => releaseAll("pagehide"));
  document.addEventListener("visibilitychange", () => { if (document.hidden) releaseAll("hidden"); });

  fetch("doom.syms.json").then((r) => r.json())
    .then((syms) => { dbg.gameticAddr = parseInt(syms.gametic, 16) || 0; window.rvSyms = syms; }).catch(() => {});
  // Default to Doom; fall back to the Phase 3 programs if doom.bin isn't deployed.
  try { await loadProgram(PROGRAMS[0][0]); }
  catch (e) { select.value = PROGRAMS[1][0]; await loadProgram(select.value).catch(showError); }
  canvas.focus();
  requestAnimationFrame(tick);
}

function showError(e) { $("status").textContent = String(e); }

main().catch(showError);
