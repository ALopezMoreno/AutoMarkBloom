// Plays the bundled seed 11 bud-to-open sample (.amb) and screenshots bud, mid-opening and open.
// Prints a few decoded numbers so they can be compared with the source JSON.
import { chromium } from "playwright-core";
import { preview } from "vite";
const out = process.argv[2] ?? ".";
const server = await preview({ preview: { port: 5191 }, logLevel: "error" });
const browser = await chromium.launch({ executablePath: process.env.CHROMIUM ?? "/opt/pw-browsers/chromium", args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 900, height: 760 } });
page.on("console", (m) => m.type() === "info" && console.log(m.text()));
page.on("pageerror", (e) => console.log("ERR", String(e)));
const t0 = Date.now();
await page.goto(`http://localhost:5191/?src=samples/flower_11_0.amb.txt`);
await page.waitForFunction(() => window.automark?.flower, null, { timeout: 120000 });
console.log("loaded in", Date.now() - t0, "ms; frames", await page.evaluate(() => window.automark.flower.frameCount), await page.evaluate(() => document.getElementById("err").textContent));
console.log("probe", JSON.stringify(await page.evaluate(() => { const d = window.automark.flower.doc; return [0, d.frames.length - 1].map((f) => Array.from(d.frames[f].petals.at(-1).mid.slice(600, 603)).concat(d.frames[f].petals.at(-1).thick[300])); })));
console.log("smooth step 20.5 ok", await page.evaluate(() => { window.automark.setTime(20.5); return !document.getElementById("err").textContent; }));
for (const f of [0, 12, 20, 28, 37]) {
  await page.evaluate((f) => window.automark.setTime(f), f);
  await page.waitForTimeout(400);
  console.log(f, await page.evaluate(() => document.getElementById("frame").textContent));
  await page.screenshot({ path: `${out}/open_${f}.png` });
}
await browser.close(); server.httpServer.close();
