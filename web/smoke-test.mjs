// Headless browser smoke test for the WASM front end.
// Usage: ./build.sh && npm run smoke   (serves web/www itself on a random port)
import { chromium } from "playwright-core";
import http from "node:http";
import fs from "node:fs";
import path from "node:path";

const ROOT = new URL("./www/", import.meta.url).pathname;
const TYPES = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".wasm": "application/wasm", ".json": "application/json" };
const server = http.createServer((req, res) => {
  const file = path.join(ROOT, decodeURIComponent(req.url.split("?")[0]) === "/" ? "index.html" : req.url);
  fs.readFile(file, (err, data) => {
    if (err) { res.writeHead(404); return res.end(); }
    res.writeHead(200, { "content-type": TYPES[path.extname(file)] ?? "application/octet-stream" });
    res.end(data);
  });
}).listen(0);
const url = `http://127.0.0.1:${server.address().port}/`;

const fail = (msg) => { console.error("FAIL:", msg); process.exitCode = 1; };
const browser = await chromium.launch({ executablePath: process.env.CHROMIUM ?? "/opt/pw-browsers/chromium" });
const page = await browser.newPage({ viewport: { width: 1400, height: 820 } });
page.on("pageerror", (e) => fail("page error: " + e.message));

await page.goto(url);
// Wait for Doom to finish booting (~260M instructions), then let a few frames render.
await page.waitForFunction(() => document.getElementById("console").textContent.includes("I_InitGraphics"),
  null, { timeout: 90000 });
await page.waitForTimeout(3000);
await page.keyboard.press("Enter");           // open the menu
await page.waitForTimeout(1500);

const state = await page.evaluate(() => {
  const ctx = document.getElementById("screen").getContext("2d");
  const px = ctx.getImageData(0, 0, 320, 200).data;
  const colors = new Set();
  for (let i = 0; i < px.length; i += 4) colors.add((px[i] << 16) | (px[i + 1] << 8) | px[i + 2]);
  return {
    program: document.getElementById("program").value,
    colors: colors.size,
    mips: parseFloat(document.getElementById("mips").textContent),
    fps: parseInt(document.getElementById("fps").textContent),
    instret: document.getElementById("instret").textContent,
    pc: document.getElementById("pc").textContent,
    disasm: document.getElementById("disasm").textContent,
    sp: document.querySelector("#r2 .val").textContent,
    status: document.getElementById("status").textContent,
    console: document.getElementById("console").textContent.slice(-120),
  };
});
await page.screenshot({ path: process.env.SCREENSHOT ?? "/tmp/rv-doom-web.png" });
console.log(JSON.stringify(state, null, 2));

if (state.program !== "doom.bin") fail("Doom was not loaded");
if (state.colors < 32) fail(`canvas looks blank (${state.colors} colors)`);
if (!(state.mips > 1)) fail(`MIPS not updating (${state.mips})`);
if (!/^[0-9a-f]{8}$/.test(state.pc) || !state.pc.startsWith("80")) fail(`bad pc ${state.pc}`);
if (!state.disasm.startsWith(state.pc + ":")) fail(`disasm doesn't match pc: ${state.disasm}`);
if (state.status) fail(`machine halted: ${state.status}`);
if (!state.console.includes("I_InitGraphics")) fail("Doom console output missing");

// Stuck keys: start E1M1, hold Right+Ctrl, blur the window, and check Doom's own key state
// (gamekeydown[], int array, address from doom.syms.json) shows both pressed, then both released.
for (let i = 0; i < 3; i++) { await page.keyboard.press("Enter"); await page.waitForTimeout(400); } // new game, E1, skill
await page.waitForTimeout(2000);
// The emulator lives in worker.js: the page has no wasm instance, and every guest read below goes
// through the worker (window.rvQuery -> postMessage -> wasm.peek), as do the keys.
const onMain = await page.evaluate(() => ({ wasm: typeof wasm, inWorker: window.rvDebug?.inWorker }));
console.log("main thread:", onMain);
if (onMain.wasm !== "undefined" || onMain.inWorker !== true) fail("emulator is not running in the worker");
const guestKeys = () => page.evaluate(async () => {
  const base = parseInt(window.rvSyms.gamekeydown, 16);
  const { values } = await window.rvQuery([base + 4 * 0xae, base + 4 * 0xa3]);
  return { right: values[0], fire: values[1] };
});
// Poll rather than sleep: during the level-start screen wipe Doom processes no input at all.
const waitKeys = async (want) => {
  for (const end = Date.now() + 10000; Date.now() < end; await page.waitForTimeout(100)) {
    const k = await guestKeys();
    if (k.right === want && k.fire === want) return;
  }
};
await page.keyboard.down("ArrowRight");
await page.keyboard.down("Control");
await waitKeys(1);
const whileHeld = await guestKeys();
await page.evaluate(() => window.dispatchEvent(new Event("blur")));
await waitKeys(0);
const afterBlur = await guestKeys();
await page.keyboard.up("Control");
await page.keyboard.up("ArrowRight");
console.log("guest gamekeydown while held:", whileHeld, "after blur:", afterBlur);
if (whileHeld.right !== 1 || whileHeld.fire !== 1) fail("Right+Ctrl did not reach the guest");
if (afterBlur.right !== 0 || afterBlur.fire !== 0) fail("keys still held in the guest after window blur");

// Dropdown focus: arrow keys must go to the game, not change the selected program.
const delivered = () => page.evaluate(async () => (await window.rvQuery()).keysDelivered);
const delivered0 = await delivered();
await page.focus("#program");
await page.keyboard.press("ArrowDown");
await page.waitForTimeout(300);
const dd = { value: await page.evaluate(() => document.getElementById("program").value), delivered: await delivered() };
console.log("dropdown after ArrowDown:", dd.value, "guest key events:", dd.delivered - delivered0);
if (dd.value !== "doom.bin") fail("ArrowDown on the focused dropdown changed the program");
if (dd.delivered - delivered0 !== 2) fail("ArrowDown on the focused dropdown did not reach the guest");

// Fallback programs: switch to Mandelbrot and check its UART output reaches the console.
await page.selectOption("#program", "programs/mandel.bin");
await page.waitForFunction(() => document.getElementById("status").textContent.includes("exited with code 0"),
  null, { timeout: 30000 });
if (await page.evaluate(() => document.activeElement.id === "program")) fail("dropdown kept focus after a selection");
const mandel = await page.evaluate(() => document.getElementById("console").textContent);
console.log(mandel.split("\n").slice(10, 16).join("\n"));
if (mandel.split("\n").filter((l) => l.length === 72).length !== 28) fail("mandelbrot output wrong");

await browser.close();
server.close();
console.log(process.exitCode ? "SMOKE TEST FAILED" : "SMOKE TEST PASSED");
