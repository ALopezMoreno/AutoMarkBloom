import { chromium } from "playwright-core";
import { createServer } from "vite";
const out = process.argv[2];
const server = await createServer({ server: { port: 5198 }, logLevel: "error" });
await server.listen();
const browser = await chromium.launch({ executablePath: "/opt/pw-browsers/chromium", args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
for (const [name, vp] of [["phone", { width: 390, height: 844 }], ["desktop", { width: 1200, height: 750 }]]) {
  const page = await browser.newPage({ viewport: vp, deviceScaleFactor: 1 });
  await page.goto(`http://localhost:5198/?seed=7&t=0.6`);
  await page.waitForFunction(() => window.automark?.flower);
  await page.waitForTimeout(500);
  await page.screenshot({ path: `${out}/${name}.png` });
  if (name === "phone") { await page.click("summary"); await page.waitForTimeout(1500); console.log(await page.evaluate(() => { const c = document.querySelector("canvas"); return [c.width, c.height, c.getBoundingClientRect().top, document.getElementById("app").clientHeight]; })); await page.screenshot({ path: `${out}/phone_more.png` }); }
}
await browser.close(); await server.close();
