// rv-doom browser front end. The emulator runs in worker.js; this thread draws the latest finished
// frame on each animation frame, forwards keys, and shows the panel, console and debug overlay.
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
const worker = new Worker("worker.js");
let running = true, halted = false;
const held = new Set();  // Doom keycodes currently down in the guest
let lastStats = { t: 0, instret: 0, frames: 0 };
let prevRegs = new Uint32Array(33);
let pendingFrame = null;  // latest frame buffer from the worker, not drawn yet
let live = { instret: 0, frames: 0, keysDelivered: 0 };  // latest worker counters

// Debug overlay (toggle with the ` key, or start with ?debug). The worker measures the emulator side,
// this thread the display side; the merged snapshot is also exposed as window.rvDebug for the tests.
const DEBUG_KEY = "Backquote";  // unused by Doom; never forwarded to the guest
const dbg = {
  on: new URLSearchParams(location.search).has("debug"),
  keys: [], sent: 0,                       // key log, events queued since load
  win: performance.now(), raf: 0, draws: 0, lastRaf: 0,
  t: { draw: 0, ui: 0, interval: 0 },      // main-thread ms per window, see tick()
  load: 0, worker: {}, snap: {},
};

// Request/response to the worker (program loads and fresh guest reads).
let nextId = 1;
const pending = new Map();
function request(msg) {
  return new Promise((resolve) => { const id = nextId++; pending.set(id, resolve); worker.postMessage({ ...msg, id }); });
}
// For tests and debugging: read guest words through the worker. Resolves to {values, keysDelivered}.
window.rvQuery = (peek = []) => request({ type: "query", peek });

worker.onmessage = ({ data: m }) => {
  switch (m.type) {
    case "frame":
      if (pendingFrame) worker.postMessage({ type: "buffer", buf: pendingFrame }, [pendingFrame]);  // superseded
      pendingFrame = m.buf;
      break;
    case "uart": appendConsole(m.text); break;
    case "panel": updatePanel(m); break;
    case "stats":
      live = m;
      if (m.snap) dbg.worker = m.snap;
      break;
    case "loaded": case "query": pending.get(m.id)?.(m); pending.delete(m.id); break;
    case "error": showError(m.error); break;
  }
};
worker.onerror = (e) => showError(e.message);

async function loadProgram(path) {
  $("console").textContent = "";
  held.clear(); dbg.keys = []; dbg.sent = 0; dbg.load = performance.now();
  lastStats = { t: performance.now(), instret: 0, frames: 0 };
  const r = await request({ type: "load", path });
  if (r.error) throw new Error(r.error);
  halted = false;
  worker.postMessage({ type: "run", on: running });
}

function draw() {
  if (!pendingFrame) return false;
  image.data.set(new Uint8Array(pendingFrame));
  ctx.putImageData(image, 0, 0);
  worker.postMessage({ type: "buffer", buf: pendingFrame }, [pendingFrame]);
  pendingFrame = null;
  return true;
}

function appendConsole(out) {
  const el = $("console");
  el.textContent = (el.textContent + out).slice(-20000);
  el.scrollTop = el.scrollHeight;
}

function buildRegs() {
  $("regs").innerHTML = ABI.map((name, i) =>
    `<div class="reg" id="r${i}"><span class="name">${name}</span><span class="val">00000000</span></div>`).join("");
}

function updatePanel({ regs, disasm, status, halted: h }) {
  for (let i = 0; i < 32; i++) {
    const el = $("r" + i);
    el.lastChild.textContent = regs[i].toString(16).padStart(8, "0");
    el.classList.toggle("changed", regs[i] !== prevRegs[i]);
  }
  $("pc").textContent = regs[32].toString(16).padStart(8, "0");
  prevRegs = regs;
  $("disasm").textContent = disasm;
  $("status").textContent = status;
  halted = h;
}

function updateStats(now) {
  const dt = (now - lastStats.t) / 1000;
  if (dt < 0.5) return;
  const { instret, frames } = live;
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
  const t = ((k.t - dbg.load) / 1000).toFixed(2).padStart(7);
  const doom = k.doom ? "0x" + k.doom.toString(16).padStart(2, "0") : "    ";
  const fate = k.fate !== "sent" ? "DROPPED (" + k.fate + ")"
    : live.keysDelivered >= k.seq ? `delivered #${k.seq}` : `queued #${k.seq}`;
  return `${t}s ${k.type.padEnd(4)} ${k.code.padEnd(11)} ${(k.mods || "-").padEnd(10)} ${k.repeat ? "rep" : "   "} ${doom} ${fate}`;
}

// Main-thread side of the overlay, once per animation frame.
function debugSample(now) {
  const dt = (now - dbg.win) / 1000;
  if (dt < 1) return;
  const n = dbg.raf || 1, t = dbg.t, interval = t.interval / n;
  dbg.snap = window.rvDebug = {
    ...dbg.worker,
    rafFps: dbg.raf / dt,                       // animation frames the browser gave us
    drawFps: dbg.draws / dt,                    // canvas updates that showed a new guest frame
    // Per animation frame, in ms. idle = rAF interval minus our JS work (compositing, GC, true idle).
    perRaf: { interval, draw: t.draw / n, ui: t.ui / n, idle: interval - (t.draw + t.ui) / n },
    keys: dbg.keys.map(keyLine),
    held: [...held],
    inWorker: true,
  };
  dbg.win = now; dbg.raf = dbg.draws = 0;
  dbg.t = { draw: 0, ui: 0, interval: 0 };
}

function renderDebug() {
  const s = dbg.snap, f = (v, d = 1) => (v == null ? "n/a" : v.toFixed(d));
  const el = $("debug");
  el.hidden = false;
  el.textContent = [
    `display  ${f(s.rafFps)} rAF/s   ${f(s.drawFps)} new frames drawn/s   guest ${f(s.guestFps)} fps`,
    `emulator ${f(s.mips)} MIPS (in worker)   worker busy ${f(100 * s.busyShare, 0)}% of its wall time`,
    `main     per rAF ${f(s.perRaf?.interval)} ms = draw ${f(s.perRaf?.draw, 2)} + ui ${f(s.perRaf?.ui, 2)}` +
      ` + idle ${f(s.perRaf?.idle)}`,
    `budget   ${f(s.mipsWhileRunning)} MIPS while running   ${f(s.instrPerGuestFrame / 1e6, 2)}M instr/guest frame   ` +
      `frame copy ${f(s.copyMsPerSec)} ms/s   waiting on clock ${f(100 * s.waitShare, 1)}%`,
    `timer    guest-vs-wall drift ${f(s.clockDriftMs, 0)} ms   max clock jump ${f(s.maxJumpMs, 0)} ms`,
    `doom     gametic ${s.gametic ?? "n/a"}   ${f(s.ticsPerSec)} tics/s (want 35)   game-time drift ${f(s.gameDriftMs, 0)} ms`,
    `held     ${s.held?.length ? s.held.map((c) => "0x" + c.toString(16)).join(" ") : "-"}`,
    `last ${dbg.keys.length} key events (t, type, code, modifiers, repeat, doom code, fate):`,
    ...dbg.keys.map(keyLine),
  ].join("\n");
}

function tick(now) {
  const t0 = performance.now();
  if (draw()) dbg.draws++;
  const t1 = performance.now();
  updateStats(now);
  debugSample(t1);
  if (dbg.on) renderDebug();
  dbg.raf++;
  if (dbg.lastRaf) dbg.t.interval += now - dbg.lastRaf;
  dbg.lastRaf = now;
  dbg.t.draw += t1 - t0;
  dbg.t.ui += performance.now() - t1;
  requestAnimationFrame(tick);
}

function setRunning(on) {
  running = on;
  $("run").textContent = on ? "Pause" : "Run";
  worker.postMessage({ type: "run", on });
}

function sendKey(code, pressed) { worker.postMessage({ type: "key", code, pressed }); }

async function main() {
  buildRegs();

  const select = $("program");
  for (const [path, label] of PROGRAMS) select.add(new Option(label, path));
  // Blur after a pick so the dropdown never keeps focus (and arrow keys) during play.
  select.onchange = () => { select.blur(); canvas.focus(); loadProgram(select.value).catch(showError); };
  $("run").onclick = () => setRunning(!running);
  $("reset").onclick = () => loadProgram(select.value).catch(showError);
  $("step").onclick = () => { setRunning(false); worker.postMessage({ type: "step" }); };

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
      sendKey(code, down ? 1 : 0);
    }, { capture: true });
  }

  // Keyups that happen while the page has no focus are never delivered, so release everything.
  const releaseAll = (why) => {
    for (const code of held) {
      logKey({ type: "keyup", code: "(" + why + ")", key: "", repeat: false }, code, "sent");
      sendKey(code, 0);
    }
    held.clear();
  };
  window.addEventListener("blur", () => releaseAll("blur"));
  window.addEventListener("pagehide", () => releaseAll("pagehide"));
  document.addEventListener("visibilitychange", () => { if (document.hidden) releaseAll("hidden"); });

  fetch("doom.syms.json").then((r) => r.json()).then((syms) => { window.rvSyms = syms; }).catch(() => {});
  requestAnimationFrame(tick);
  // Default to Doom; fall back to the Phase 3 programs if doom.bin isn't deployed.
  try { await loadProgram(PROGRAMS[0][0]); }
  catch (e) { select.value = PROGRAMS[1][0]; await loadProgram(select.value).catch(showError); }
  canvas.focus();
}

function showError(e) { $("status").textContent = String(e); }

main().catch(showError);
