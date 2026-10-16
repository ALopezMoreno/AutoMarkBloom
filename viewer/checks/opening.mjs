// Plays the bundled seed 11 bud-to-open sample and screenshots bud, mid-opening and open.
import { chromium } from "playwright-core";
import { preview } from "vite";
const out = process.argv[2] ?? ".";
const server = await preview({ preview: { port: 5191 }, logLevel: "error" });
const browser = await chromium.launch({ executablePath: process.env.CHROMIUM ?? "/opt/pw-browsers/chromium", args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 900, height: 760 } });
page.on("pageerror", (e) => console.log("ERR", String(e)));
const t0 = Date.now();
await page.goto(`http://localhost:5191/?src=samples/flower_11_0.json`);
await page.waitForFunction(() => window.automark?.flower, null, { timeout: 120000 });
console.log("loaded in", Date.now() - t0, "ms; frames", await page.evaluate(() => window.automark.flower.frameCount), await page.evaluate(() => document.getElementById("err").textContent));
for (const f of [0, 7, 14, 22, 31]) {
  await page.evaluate((f) => window.automark.setTime(f), f);
  await page.waitForTimeout(400);
  console.log(f, await page.evaluate(() => document.getElementById("frame").textContent));
  await page.screenshot({ path: `${out}/open_${f}.png` });
}
await browser.close(); server.httpServer.close();
