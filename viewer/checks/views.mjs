// Screenshots the bud views (layer colours, exploded, peeled, sections) on a core bud.
import { chromium } from "playwright-core";
import { createServer } from "vite";
const out = process.argv[2] ?? ".";
const server = await createServer({ server: { port: 5194 }, logLevel: "error" });
await server.listen();
const browser = await chromium.launch({ executablePath: process.env.CHROMIUM ?? "/opt/pw-browsers/chromium", args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 1200, height: 800 } });
page.on("pageerror", (e) => console.log("ERR", String(e)));
await page.goto(`http://localhost:5194/?src=samples/bud_11.json`);
await page.waitForFunction(() => window.automark?.flower);
const set = (id, v) => page.evaluate(([id, v]) => {
  const e = document.getElementById(id);
  e.value = v;
  e.dispatchEvent(new Event(e.tagName === "SELECT" || e.type === "checkbox" ? "change" : "input"));
}, [id, v]);
const shot = async (name) => { await page.waitForTimeout(500); await page.screenshot({ path: `${out}/${name}.png` }); };
await set("colour", "schematic"); await shot("v_layers"); await page.evaluate(() => window.automark.setTime(0)); await set("section","vertical"); await shot("v_schem_section"); await set("section","off");
await set("explode", "0.6"); await shot("v_exploded");
await set("explode", "0"); await set("peel", "4"); await shot("v_peel");
await set("peel", "99"); await set("section", "vertical"); await shot("v_vertical");
await set("section", "horizontal"); await set("secPos", "0.55"); await shot("v_horizontal");
await set("colour", "natural"); await set("section", "vertical"); await set("explode", "0.3"); await shot("v_vertical_natural_exploded");
await page.setViewportSize({ width: 390, height: 844 }); await page.goto(`http://localhost:5194/?src=samples/bud_11.json`); await page.waitForFunction(() => window.automark?.flower); await page.click("#views summary"); await set("section", "vertical"); await shot("v_phone");
await browser.close(); await server.close();
