// rv-doom emulator worker: owns rv_doom_web.wasm and runs the guest in its own loop, independent of
// the page's animation frames. Talks to main.js by postMessage only (see the `on` table below).
// Frames go to the page in two transferable buffers (double buffering): the page draws one and hands
// it back while the worker may already be filling the other. No SharedArrayBuffer, so plain static
// hosting (python3 -m http.server, no COOP/COEP headers) works.
"use strict";

const SLICE = 100000;            // instructions per wasm.run() call
const BATCH_MS = 8;              // run this long, then yield so key/control messages get handled
const FRAME_BYTES = 320 * 200 * 4;

let wasm, loaded = false, running = true, halted = false, isDoom = false;
const free = [new ArrayBuffer(FRAME_BYTES), new ArrayBuffer(FRAME_BYTES)];  // buffers the page has returned
let sentFrames = -1;             // guest frame counter of the last frame posted
let lastPanel = 0, lastStats = 0;
let gameticAddr = 0, waitRanges = [];
const decoder = new TextDecoder();
const text = (len) => decoder.decode(new Uint8Array(wasm.memory.buffer, wasm.text_ptr(), len));

// Stats for the debug overlay, measured over 1 s windows (worker clock).
const st = { win: null, busy: 0, copy: 0, maxJump: 0, lastTimer: 0, pcSamples: 0, pcWait: 0, load: null, tic0: null };

// Yield with a MessageChannel instead of setTimeout (no 4 ms clamp). Incoming messages queue as
// tasks too, so keys are handled between batches.
const ticker = new MessageChannel();
let scheduled = false;
ticker.port1.onmessage = loop;
function schedule() {
  if (!scheduled && loaded && running && !halted) { scheduled = true; ticker.port2.postMessage(0); }
}

function loop() {
  scheduled = false;
  if (!loaded || !running || halted) return;
  const start = performance.now();
  do {
    const t = Math.floor(performance.now());
    if (st.lastTimer) st.maxJump = Math.max(st.maxJump, t - st.lastTimer);
    st.lastTimer = t;
    halted = wasm.run(SLICE, t) !== 0;
    samplePc();
  } while (!halted && performance.now() - start < BATCH_MS);
  st.busy += performance.now() - start;
  sendFrame(false);
  sendOutput(halted);
  sendStats();
  schedule();
}

// Post the framebuffer if the guest finished a new frame (or `force`) and a buffer is free.
function sendFrame(force) {
  const frames = wasm.frames();
  if ((!force && frames === sentFrames) || !free.length) return;
  const t0 = performance.now();
  const buf = free.pop();
  new Uint8Array(buf).set(new Uint8Array(wasm.memory.buffer, wasm.frame_rgba(), FRAME_BYTES));
  st.copy += performance.now() - t0;
  sentFrames = frames;
  postMessage({ type: "frame", buf, frames }, [buf]);
}

function sendOutput(forcePanel) {
  const uart = text(wasm.take_uart());
  if (uart) postMessage({ type: "uart", text: uart });
  const now = performance.now();
  if (!forcePanel && now - lastPanel < 100) return;
  lastPanel = now;
  const regs = new Uint32Array(wasm.memory.buffer, wasm.regs(), 33).slice();
  const disasm = text(wasm.disasm_current());
  postMessage({ type: "panel", regs, disasm, status: text(wasm.status_text()), halted });
}

// Is the guest waiting on the clock (DG_SleepMs, I_GetTime ...) rather than doing work?
function samplePc() {
  if (!isDoom || !waitRanges.length) return;
  const pc = new Uint32Array(wasm.memory.buffer, wasm.regs(), 33)[32];
  st.pcSamples++;
  if (waitRanges.some(([lo, len]) => pc >= lo && pc < lo + len)) st.pcWait++;
}

// Live counters every 250 ms; a full window snapshot every second.
function sendStats(force) {
  const now = performance.now();
  if (!force && now - lastStats < 250) return;
  lastStats = now;
  const instret = wasm.instret(), frames = wasm.frames();
  const tic = isDoom && gameticAddr ? wasm.peek(gameticAddr) : 0;
  if (!st.load) st.load = { wall: now, timer: wasm.timer_ms() };
  if (tic && !st.tic0) st.tic0 = { wall: now, tic };
  const w = st.win ??= { t: now, instret, frames, tic };
  if (!w.tic) w.tic = tic;
  let snap = null;
  const dt = (now - w.t) / 1000;
  if (dt >= 1) {
    snap = {
      guestFps: (frames - w.frames) / dt,                       // DG_DrawFrame calls
      mips: (instret - w.instret) / dt / 1e6,
      busyShare: st.busy / (dt * 1000),                          // share of worker wall time inside run batches
      mipsWhileRunning: st.busy ? (instret - w.instret) / st.busy / 1e3 : null,
      instrPerGuestFrame: frames > w.frames ? (instret - w.instret) / (frames - w.frames) : null,
      copyMsPerSec: st.copy / dt,                                // framebuffer -> transfer buffer
      maxJumpMs: st.maxJump,                                     // largest guest-clock step between run() calls
      clockDriftMs: (wasm.timer_ms() - st.load.timer) - (now - st.load.wall),
      ticsPerSec: tic && w.tic ? (tic - w.tic) / dt : null,
      gameDriftMs: st.tic0 ? (tic - st.tic0.tic) * 1000 / 35 - (now - st.tic0.wall) : null,
      gametic: tic,
      waitShare: st.pcSamples ? st.pcWait / st.pcSamples : null,
    };
    st.win = { t: now, instret, frames, tic };
    st.busy = st.copy = st.maxJump = st.pcSamples = st.pcWait = 0;
  }
  postMessage({ type: "stats", instret, frames, keysDelivered: wasm.keys_delivered(), snap });
}

async function load(path) {
  const resp = await fetch(path);
  if (!resp.ok) throw new Error(`${path}: ${resp.status}`);
  const bytes = new Uint8Array(await resp.arrayBuffer());
  const ptr = wasm.upload_buffer(bytes.length);
  new Uint8Array(wasm.memory.buffer, ptr, bytes.length).set(bytes);
  if (wasm.load() !== 0) throw new Error("image does not fit in RAM");
  loaded = true; halted = false; isDoom = path === "doom.bin";
  Object.assign(st, { win: null, busy: 0, copy: 0, maxJump: 0, lastTimer: 0, pcSamples: 0, pcWait: 0, load: null, tic0: null });
  sendFrame(true);
  sendOutput(true);
  schedule();
}

const on = {
  load: (m) => load(m.path).then(() => postMessage({ type: "loaded", id: m.id }),
                                 (e) => postMessage({ type: "loaded", id: m.id, error: String(e) })),
  key: (m) => wasm.key(m.code, m.pressed),
  run: (m) => { running = m.on; if (!running && loaded) sendOutput(true); schedule(); },
  step: () => {
    running = false;
    if (loaded && !halted) halted = wasm.run(1, Math.floor(performance.now())) !== 0;
    sendFrame(true); sendOutput(true); sendStats(true);
  },
  buffer: (m) => { free.push(m.buf); if (loaded) sendFrame(halted || !running); },
  // Fresh guest reads for the tests and overlay: words at `peek` addresses plus the key counter.
  query: (m) => postMessage({ type: "query", id: m.id, values: m.peek.map((a) => wasm.peek(a)),
                              keysDelivered: wasm.keys_delivered() }),
};

const ready = (async () => {
  const { instance } = await WebAssembly.instantiateStreaming(fetch("rv_doom_web.wasm"), {});
  wasm = instance.exports;
  try {
    const syms = await (await fetch("doom.syms.json")).json();
    gameticAddr = parseInt(syms.gametic, 16) || 0;
    waitRanges = (syms.wait ?? []).map(([a, n]) => [parseInt(a, 16), parseInt(n, 16)]);
  } catch { /* no Doom build: overlay shows n/a for the Doom rows */ }
})();

onmessage = (e) => ready.then(() => on[e.data.type](e.data)).catch((err) => postMessage({ type: "error", error: String(err) }));
