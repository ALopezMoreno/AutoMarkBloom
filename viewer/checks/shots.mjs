import { chromium } from "playwright-core";
import { createServer } from "vite";
const out = process.argv[2];
const server = await createServer({ server: { port: 5199 }, logLevel: "error" });
await server.listen();
const browser = await chromium.launch({ executablePath: "/opt/pw-browsers/chromium", args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 900, height: 700 } });
const errs = [];
page.on("pageerror", (e) => errs.push(String(e)));
page.on("console", (m) => m.type() === "error" && errs.push(m.text()));
for (const seed of [7, 42]) {
  await page.goto(`http://localhost:5199/?seed=${seed}`);
  await page.waitForFunction(() => window.automark?.flower);
  for (const t of [0, 0.5, 1]) {
    await page.evaluate((t) => window.automark.setTime(t * (window.automark.flower.frameCount - 1)), t);
    await page.waitForTimeout(400);
    await page.screenshot({ path: `${out}/seed${seed}_t${t}.png` });
  }
}
const sizes = await page.evaluate(async () => {
  const a = await window.automark.exportAnimated();
  const s = await window.automark.exportStatic();
  // reload the animated glb to confirm it parses with its clip
  const { GLTFLoader } = await import("/node_modules/three/examples/jsm/loaders/GLTFLoader.js");
  const g = await new GLTFLoader().parseAsync(a, "");
  let morphs = 0; g.scene.traverse((o) => { if (o.morphTargetInfluences) morphs = o.morphTargetInfluences.length; });
  return { animatedMB: (a.byteLength / 1e6).toFixed(2), staticMB: (s.byteLength / 1e6).toFixed(2), clips: g.animations.map((c) => `${c.name} ${c.duration.toFixed(2)}s`), morphs };
});
console.log(JSON.stringify(sizes), errs.length ? errs : "no page errors");
await browser.close(); await server.close();
