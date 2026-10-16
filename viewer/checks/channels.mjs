// Screenshots channel colouring, sibling flowers and bud-development frames.
// Optional: DEV_JSON=<path under public/> for a core export with --frames.
import { chromium } from "playwright-core";
import { createServer } from "vite";
const out = process.argv[2] ?? ".";
const server = await createServer({ server: { port: 5193 }, logLevel: "error" });
await server.listen();
const browser = await chromium.launch({ executablePath: process.env.CHROMIUM ?? "/opt/pw-browsers/chromium", args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 1100, height: 760 } });
page.on("pageerror", (e) => console.log("ERR", String(e)));
const set = (id, v) => page.evaluate(([id, v]) => {
  const e = document.getElementById(id);
  if (e.type === "checkbox") e.checked = v; else e.value = v;
  e.dispatchEvent(new Event(e.tagName === "SELECT" || e.type === "checkbox" ? "change" : "input"));
}, [id, v]);
const txt = () => page.evaluate(() => ["frame", "info", "err", "colourLegend"].map((id) => document.getElementById(id).textContent).join(" | "));
const shot = async (name) => { await page.waitForTimeout(500); await page.screenshot({ path: `${out}/${name}.png` }); console.log(name, await txt()); };

await page.goto(`http://localhost:5193/?seed=7`);
await page.waitForFunction(() => window.automark?.flower);
await set("source", "stub"); await page.waitForTimeout(300);
await set("colour", "onset"); await shot("c_onset");
await set("colour", "petal_type"); await shot("c_type");
await page.evaluate(() => window.automark.setTime(30)); await set("shells", true); await shot("c_type_open_sepals");
await set("shells", false); await set("colour", "natural");
await page.click("#sibling"); await page.waitForTimeout(300); await page.evaluate(() => window.automark.setTime(30)); await shot("c_sibling1");
if (process.env.DEV_JSON) {
  await page.goto(`http://localhost:5193/?src=${process.env.DEV_JSON}`);
  await page.waitForFunction(() => window.automark?.flower, null, { timeout: 60000 });
  await set("colour", "schematic");
  for (const f of [0, 6, 12]) { await page.evaluate((f) => window.automark.setTime(f), f); await shot(`c_dev_${f}`); }
  await set("section", "vertical"); await page.evaluate(() => window.automark.setTime(0)); await shot("c_dev_section0");
}
await browser.close(); await server.close();
