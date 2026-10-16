import { chromium } from "playwright-core";
import { createServer } from "vite";
const out = process.argv[2];
const server = await createServer({ server: { port: 5197 }, logLevel: "error" });
await server.listen();
const browser = await chromium.launch({ executablePath: "/opt/pw-browsers/chromium", args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 900, height: 700 } });
page.on("pageerror", (e) => console.log("ERR", String(e)));
for (const f of ["bud_11", "bud_2027"]) {
  await page.goto(`http://localhost:5197/?src=samples/${f}.json`);
  await page.waitForFunction(() => window.automark?.flower);
  await page.waitForTimeout(800);
  console.log(f, await page.evaluate(() => document.getElementById("info").textContent + " | " + document.getElementById("err").textContent));
  await page.screenshot({ path: `${out}/${f}.png` });
}
await browser.close(); await server.close();
