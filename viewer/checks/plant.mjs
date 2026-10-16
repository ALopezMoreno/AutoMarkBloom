// Screenshots the stem and sepals on a closed sample bud and on the opening stub.
import { chromium } from "playwright-core";
import { createServer } from "vite";
const out = process.argv[2] ?? ".";
const server = await createServer({ server: { port: 5190 }, logLevel: "error" });
await server.listen();
const browser = await chromium.launch({ executablePath: process.env.CHROMIUM ?? "/opt/pw-browsers/chromium", args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 900, height: 760 } });
page.on("pageerror", (e) => console.log("ERR", String(e)));
const shot = async (n) => { await page.waitForTimeout(500); await page.screenshot({ path: `${out}/${n}.png` }); };
await page.goto(`http://localhost:5190/?src=samples/bud_11.json`);
await page.waitForFunction(() => window.automark?.flower);
await shot("p_bud");
await page.goto(`http://localhost:5190/?seed=7`);
await page.waitForFunction(() => window.automark?.flower);
await page.evaluate(() => { const e = document.getElementById("source"); e.value = "stub"; e.dispatchEvent(new Event("change")); });
await page.waitForTimeout(300);
for (const f of [0, 12, 47]) { await page.evaluate((f) => window.automark.setTime(f), f); await shot(`p_stub_${f}`); }
await browser.close(); await server.close();
