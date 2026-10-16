// Grows buds with the wasm core in headless Chromium and screenshots them.
// Needs the wasm build: wasm-pack build wasm --release --target web
import { chromium } from "playwright-core";
import { createServer } from "vite";
const out = process.argv[2] ?? ".";
const server = await createServer({ server: { port: 5196 }, logLevel: "error" });
await server.listen();
const browser = await chromium.launch({ executablePath: process.env.CHROMIUM ?? "/opt/pw-browsers/chromium", args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 900, height: 700 } });
page.on("pageerror", (e) => console.log("ERR", String(e)));
for (const seed of ["11", "2027", "18446744073709551615"]) {
  await page.goto(`http://localhost:5196/?seed=${seed}`);
  await page.waitForFunction(() => /Core bud|Error|must/.test(document.getElementById("info").textContent + document.getElementById("err").textContent), null, { timeout: 60000 });
  await page.waitForTimeout(600);
  console.log(seed, await page.evaluate(() => document.getElementById("info").textContent + " | " + document.getElementById("err").textContent));
  await page.screenshot({ path: `${out}/live_${seed}.png` });
}
await browser.close(); await server.close();
