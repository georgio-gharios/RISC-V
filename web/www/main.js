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
let lastStats = { t: 0, instret: 0, frames: 0 };
let prevRegs = new Uint32Array(33);

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

let lastPanel = 0;
function tick(now) {
  if (running && !halted) {
    // Run in slices until ~12 ms of this animation frame are used.
    const start = performance.now();
    do {
      halted = wasm.run(100000, Math.floor(performance.now())) !== 0;
    } while (!halted && performance.now() - start < 12);
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
  select.onchange = () => loadProgram(select.value).catch(showError);
  $("run").onclick = () => setRunning(!running);
  $("reset").onclick = () => loadProgram(select.value).catch(showError);
  $("step").onclick = () => {
    setRunning(false);
    if (!halted) halted = wasm.run(1, Math.floor(performance.now())) !== 0;
    draw(); drainConsole(); updatePanel();
  };

  for (const type of ["keydown", "keyup"]) {
    window.addEventListener(type, (e) => {
      if (e.target === select) return;
      const code = doomKey(e);
      if (!code) return;
      e.preventDefault();
      if (type === "keydown" && e.repeat) return;
      wasm.key(code, type === "keydown" ? 1 : 0);
    });
  }

  // Default to Doom; fall back to the Phase 3 programs if doom.bin isn't deployed.
  try { await loadProgram(PROGRAMS[0][0]); }
  catch (e) { select.value = PROGRAMS[1][0]; await loadProgram(select.value).catch(showError); }
  canvas.focus();
  requestAnimationFrame(tick);
}

function showError(e) { $("status").textContent = String(e); }

main().catch(showError);
