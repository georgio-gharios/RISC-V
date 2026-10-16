// Headless run of the debug overlay: boots Doom with ?debug, starts E1M1, sends a few key
// patterns (hold, repeat, modifier combos) and prints the overlay once per second.
// Usage: ./build.sh && npm run measure   (headless numbers; a real browser will differ)
import { chromium } from "playwright-core";
import http from "node:http";
import fs from "node:fs";
import path from "node:path";

const ROOT = new URL("./www/", import.meta.url).pathname;
const server = http.createServer((req, res) => {
  const file = path.join(ROOT, req.url.split("?")[0] === "/" ? "index.html" : req.url.split("?")[0]);
  fs.readFile(file, (err, data) => {
    if (err) { res.writeHead(404); return res.end(); }
    res.writeHead(200, { "content-type": file.endsWith(".wasm") ? "application/wasm" : file.endsWith(".js") ? "text/javascript" : file.endsWith(".css") ? "text/css" : "text/html" });
    res.end(data);
  });
}).listen(0);

const browser = await chromium.launch({ executablePath: process.env.CHROMIUM ?? "/opt/pw-browsers/chromium" });
const page = await browser.newPage({ viewport: { width: 1400, height: 820 } });
page.on("pageerror", (e) => { console.error("page error:", e.message); process.exitCode = 1; });
await page.goto(`http://127.0.0.1:${server.address().port}/?debug`);
await page.waitForFunction(() => document.getElementById("console").textContent.includes("I_InitGraphics"),
  null, { timeout: 90000 });
const show = async (label) => {
  console.log(`--- ${label}\n` + await page.evaluate(() => document.getElementById("debug").textContent));
};
await page.waitForTimeout(3000);
await show("title screen, 3 s after I_InitGraphics");

for (let i = 0; i < 4; i++) { await page.keyboard.press("Enter"); await page.waitForTimeout(400); } // menu, new game, E1, skill
await page.waitForTimeout(2000);
await show("in E1M1 after 4x Enter");

await page.keyboard.down("ArrowUp"); await page.waitForTimeout(300);
await page.keyboard.down("ArrowUp"); await page.keyboard.down("ArrowUp");      // auto-repeat style events
await page.waitForTimeout(1000); await page.keyboard.up("ArrowUp");
await page.keyboard.down("Control"); await page.waitForTimeout(500); await page.keyboard.up("Control");
await page.keyboard.down("Shift"); await page.keyboard.press("ArrowLeft"); await page.keyboard.up("Shift");
await page.keyboard.press("F5");                                                 // unmapped key
await page.waitForTimeout(1200);
await show("after hold/repeat/fire/shift+left/F5");

await page.keyboard.press("Backquote");
const hidden = await page.evaluate(() => document.getElementById("debug").hidden);
console.log(`--- after \` toggle: overlay hidden = ${hidden}`);
await page.keyboard.press("Backquote");
await page.waitForTimeout(1100);
await page.screenshot({ path: process.env.SCREENSHOT ?? "/tmp/rv-doom-debug.png" });
await show("final");
await browser.close();
server.close();
