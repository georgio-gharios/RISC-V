// Headless browser smoke test for the WASM front end.
// Usage: ./build.sh && npm run smoke   (serves web/www itself on a random port)
import { chromium } from "playwright-core";
import http from "node:http";
import fs from "node:fs";
import path from "node:path";

const ROOT = new URL("./www/", import.meta.url).pathname;
const TYPES = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".wasm": "application/wasm" };
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

// Fallback programs: switch to Mandelbrot and check its UART output reaches the console.
await page.selectOption("#program", "programs/mandel.bin");
await page.waitForFunction(() => document.getElementById("status").textContent.includes("exited with code 0"),
  null, { timeout: 30000 });
const mandel = await page.evaluate(() => document.getElementById("console").textContent);
console.log(mandel.split("\n").slice(10, 16).join("\n"));
if (mandel.split("\n").filter((l) => l.length === 72).length !== 28) fail("mandelbrot output wrong");

await browser.close();
server.close();
console.log(process.exitCode ? "SMOKE TEST FAILED" : "SMOKE TEST PASSED");
