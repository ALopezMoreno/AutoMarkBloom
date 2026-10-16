import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { RoomEnvironment } from "three/examples/jsm/environments/RoomEnvironment.js";
import { validate, type FlowerDoc } from "./format";
import { FlowerMesh } from "./mesh";
import { stubFlower } from "./stub";
import { coreAvailable, growBud } from "./core";
import { download, exportAnimated, exportStatic } from "./export";
import { drawSchematic, type Section } from "./section";
import { layerCss } from "./palette";
import { PlantParts } from "./plant";
import { categoryCss, rampCss } from "./channels";

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true, preserveDrawingBuffer: true });
renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
renderer.toneMapping = THREE.ACESFilmicToneMapping;
renderer.localClippingEnabled = true;
$("app").appendChild(renderer.domElement);

const scene = new THREE.Scene();
scene.environment = new THREE.PMREMGenerator(renderer).fromScene(new RoomEnvironment(), 0.04).texture;
scene.environmentIntensity = 0.6;
const sun = new THREE.DirectionalLight(0xfff4ea, 2.2);
sun.position.set(3, 6, 4);
scene.add(sun, new THREE.HemisphereLight(0xfff6f0, 0x6b5a60, 0.6));

const camera = new THREE.PerspectiveCamera(35, 1, 0.05, 200);
camera.position.set(4.5, 3.2, 5.5);
const controls = new OrbitControls(camera, renderer.domElement);
controls.enableDamping = true;
controls.target.set(0, 0.2, 0);

const petalMat = new THREE.MeshPhysicalMaterial({
  vertexColors: true, roughness: 0.5, side: THREE.DoubleSide, sheen: 1, sheenRoughness: 0.4,
  sheenColor: new THREE.Color("#ffd7e4"), transmission: 0.0, thickness: 0.05,
});
// Schematic view: translucent petals tinted by layer, with crisp outlines, so
// the inner layers show through the outer ones.
const schematicMat = new THREE.MeshStandardMaterial({
  vertexColors: true, roughness: 0.9, side: THREE.DoubleSide, transparent: true, opacity: 0.22, depthWrite: false,
});
const rimMat = new THREE.LineBasicMaterial({ vertexColors: true, transparent: true, opacity: 0.9 });
const stemMat = new THREE.MeshStandardMaterial({ color: "#56853d", roughness: 0.7 });
const sepalMat = new THREE.MeshStandardMaterial({ color: "#4a7a36", roughness: 0.6, side: THREE.DoubleSide });
let plant: PlantParts | null = null;
const coreMat = new THREE.MeshStandardMaterial({ color: "#e7c25a", roughness: 0.8 });
const shellMat = new THREE.MeshBasicMaterial({ color: "#b2456c", wireframe: true, transparent: true, opacity: 0.12 });
const envelopeMat = new THREE.MeshBasicMaterial({ color: "#3f7d4f", wireframe: true, transparent: true, opacity: 0.25 });

let flower: FlowerMesh | null = null;
const group = new THREE.Group();
scene.add(group);
let time = 0;
let playing = false;

function show(doc: FlowerDoc, label: string) {
  group.clear();
  flower?.dispose();
  flower = new FlowerMesh(doc, petalMat, rimMat);
  group.add(flower.mesh, flower.rims);
  const { aspect, core, radius } = doc.shells;
  const coreMesh = new THREE.Mesh(new THREE.SphereGeometry(core * 0.97, 48, 32), coreMat);
  coreMesh.scale.set(1, aspect, 1);
  group.add(coreMesh);
  const shells = new THREE.Group();
  shells.name = "shells";
  for (const r of radius) {
    const s = new THREE.Mesh(new THREE.SphereGeometry(r, 32, 16), shellMat);
    s.scale.set(1, aspect, 1);
    shells.add(s);
  }
  // Sepal envelope: resized with the bud as it swells (see updateEnvelope).
  const env = new THREE.Mesh(new THREE.SphereGeometry(1, 48, 24), envelopeMat);
  env.name = "sepals";
  shells.add(env);
  shells.visible = $<HTMLInputElement>("shells").checked;
  group.add(shells);
  plant?.dispose();
  plant = new PlantParts(aspect, core, `${doc.seed}/${doc.flower_index ?? 0}`, { stem: stemMat, sepal: sepalMat });
  plant.group.visible = $<HTMLInputElement>("plantParts").checked;
  group.add(plant.group);
  // Colour menu: natural, by layer, then every declared channel.
  const sel = $<HTMLSelectElement>("colour");
  const keep = sel.value;
  sel.replaceChildren(new Option("Natural", "natural"), new Option("Schematic (translucent)", "schematic"));
  for (const c of flower.channels.decls) {
    const o = new Option(`${c.name} (${c.scope}${c.kind ? `, ${c.kind}` : ""})`, c.name);
    sel.appendChild(o);
  }
  sel.value = [...sel.options].some((o) => o.value === keep) ? keep : "natural";
  const peel = $<HTMLInputElement>("peel");
  peel.max = String(flower.layerCount);
  peel.value = String(flower.layerCount);
  $("legendBar").style.background = `linear-gradient(90deg, ${Array.from({ length: 8 }, (_, i) => layerCss(i, 8)).join(", ")})`;
  applyViews();
  const layers = new Set(doc.petals.map((p) => p.layer)).size;
  const who = doc.flower_index == null ? "" : ` · plant ${doc.seed}, flower ${doc.flower_index}`;
  $("info").textContent = `${label}${who} · ${doc.petals.length} petals in ${layers} layers · ${doc.frames.length} steps · grid ${doc.grid.nu}×${doc.grid.nv}`;
  $("err").textContent = "";
  frame(flower);
  // Start on the closed bud (t = 0) when the document has one.
  const closed = doc.frames.findIndex((f) => f.t === 0);
  setTime(closed >= 0 ? closed : 0);
  const still = flower.frameCount < 2;
  for (const id of ["time", "play", "prev", "next", "glbAnim"]) $<HTMLButtonElement>(id).disabled = still;
}

/** Point the camera so the fully opened flower fits the view. */
function frame(fm: FlowerMesh) {
  fm.setTime(fm.frameCount - 1);
  const sphere = fm.geometry.boundingSphere!.clone();
  const halfV = THREE.MathUtils.degToRad(camera.fov / 2);
  const halfH = Math.atan(Math.tan(halfV) * camera.aspect);
  const dist = sphere.radius / Math.sin(Math.min(halfV, halfH)) * 1.2;
  const dir = new THREE.Vector3(0.62, 0.55, 0.75).normalize();
  controls.target.copy(sphere.center);
  camera.position.copy(sphere.center).addScaledVector(dir, dist);
  camera.near = dist / 100;
  camera.far = dist * 10;
  camera.updateProjectionMatrix();
}

function setTime(t: number) {
  if (!flower) return;
  time = Math.min(flower.frameCount - 1, Math.max(0, t));
  flower.setTime(time);
  const last = Math.max(1, flower.frameCount - 1);
  $<HTMLInputElement>("time").value = String(time / last);
  $("frame").textContent = `step ${Math.round(time)} / ${flower.frameCount - 1} · ${flower.frameLabel(time)}`;
  const mode = $<HTMLSelectElement>("colour").value;
  if (flower.channels.get(mode) && flower.channels.varies(mode)) flower.setColorMode(mode, time);
  updateEnvelope();
  updateSection();
}

// ---- Bud views: colour by layer, exploded, peel, cross-section ----

const clipPlane = new THREE.Plane();
const clipMaterials: THREE.Material[] = [petalMat, schematicMat, rimMat, coreMat, shellMat, envelopeMat, stemMat, sepalMat];

function currentSection(): Section | null {
  const kind = $<HTMLSelectElement>("section").value;
  if (!flower || kind === "off") return null;
  const x = Number($<HTMLInputElement>("secPos").value);
  if (kind === "vertical") return { kind, at: x * Math.PI * 2 };
  const top = flower.doc.shells.aspect * flower.envelope(time);
  return { kind: "horizontal", at: (x * 2 - 1) * top };
}

function updateSection() {
  if (!flower) return;
  const sec = currentSection();
  $("sectionCtl").hidden = !sec;
  $("schem").hidden = !sec;
  const clip = !!sec && $<HTMLInputElement>("clip").checked;
  for (const m of clipMaterials) {
    m.clippingPlanes = clip ? [clipPlane] : [];
    m.needsUpdate = true;
  }
  if (!sec) return;
  const deg = Math.round((sec.at * 180) / Math.PI);
  $("secPosLabel").textContent = sec.kind === "vertical" ? "Angle" : "Height";
  $("secPosVal").textContent = sec.kind === "vertical" ? `${deg}°` : `${sec.at.toFixed(2)} core radii`;
  $("schemTitle").textContent =
    sec.kind === "vertical" ? `Vertical section through the axis at ${deg}°` : `Horizontal slice at height ${sec.at.toFixed(2)}`;
  // Three.js keeps the side where normal·p + constant >= 0. Core (x, y, z) maps to three (x, z, -y).
  if (sec.kind === "vertical") clipPlane.set(new THREE.Vector3(-Math.sin(sec.at), 0, -Math.cos(sec.at)), 0);
  else clipPlane.set(new THREE.Vector3(0, -1, 0), sec.at);
  drawSchematic($("schemSvg") as unknown as SVGSVGElement, flower.doc, flower.sample(time), sec, Number($<HTMLInputElement>("peel").value), flower.envelope(time));
}

function updateEnvelope() {
  const env = flower && group.getObjectByName("sepals");
  if (!flower || !env) return;
  const r = flower.envelope(time);
  env.scale.set(r, r * flower.doc.shells.aspect, r);
  // Sepals peel back over the first part of the opening.
  const t = flower.doc.frames[Math.round(time)]?.t ?? 0;
  const open = Math.min(1, Math.max(0, t / 0.4));
  // Sepals wrap whatever the petals reach, which can be a little past the nominal envelope;
  // they stay at the closed bud's size once the flower starts opening.
  const firstOpen = flower.doc.frames.findIndex((f) => f.t > 0);
  const closedAt = firstOpen > 0 ? Math.min(time, firstOpen - 1) : time;
  const wrap = Math.max(r, flower.outerRadius(closedAt) * 0.99);
  plant?.update(wrap, open * open * (3 - 2 * open), flower.explode);
}

/** Legend under the colour menu: ramp with its range, or category swatches. */
function colourLegend(mode: string) {
  const box = $("colourLegend");
  const c = flower?.channels.get(mode);
  box.hidden = !c;
  if (!c || !flower) return;
  if (c.dims === 3) {
    box.innerHTML = `<span>RGB colour channel</span>`;
  } else if (c.labels) {
    box.innerHTML = c.labels.map((l, k) => `<span class="sw" style="background:${categoryCss(k)}"></span>${l}`).join(" ");
  } else {
    const [lo, hi] = flower.channels.range(c);
    const stops = Array.from({ length: 9 }, (_, i) => rampCss(i / 8)).join(", ");
    box.innerHTML = `<span>${+lo.toPrecision(3)}</span><span class="bar" style="background:linear-gradient(90deg, ${stops})"></span><span>${+hi.toPrecision(3)}</span>`;
  }
}

function applyViews() {
  if (!flower) return;
  const e = Number($<HTMLInputElement>("explode").value);
  flower.explode = e;
  $("explodeVal").textContent = e === 0 ? "off" : `${Math.round(e * 100)}%`;
  const k = Number($<HTMLInputElement>("peel").value);
  flower.showLayers(k);
  plant?.setSepalsVisible(k === flower.layerCount);
  $("peelVal").textContent = k === flower.layerCount ? `all ${k}` : `inner ${k} of ${flower.layerCount}`;
  const mode = $<HTMLSelectElement>("colour").value;
  flower.setColorMode(mode, time);
  const schematic = mode === "schematic";
  flower.mesh.material = schematic ? schematicMat : petalMat;
  flower.rims.visible = schematic;
  colourLegend(mode);
  flower.setTime(time);
  updateEnvelope();
  updateSection();
}

for (const id of ["explode", "peel", "secPos"]) $(id).addEventListener("input", applyViews);
for (const id of ["colour", "clip"]) $(id).addEventListener("change", applyViews);
$("section").addEventListener("change", () => {
  // Sensible starting points: a cut facing the default camera, or the bud's equator.
  $<HTMLInputElement>("secPos").value = $<HTMLSelectElement>("section").value === "vertical" ? "0.11" : "0.5";
  applyViews();
});
$("schemMin").addEventListener("click", () => {
  const min = $("schem").classList.toggle("min");
  $("schemMin").textContent = min ? "+" : "–";
});

const sourceSel = $<HTMLSelectElement>("source");
if (coreAvailable) {
  sourceSel.options[0].hidden = false;
  sourceSel.value = "core";
} else {
  $("fullRow").style.display = "none";
}

let request = 0;
const U64_MAX = 18446744073709551615n;

/** `seed` is a decimal string: core seeds are full u64s, beyond JS number precision. */
async function loadSeed(seed: string) {
  seed = seed.trim() || "0";
  if (!/^\d+$/.test(seed) || BigInt(seed) > U64_MAX) {
    $("err").textContent = "Seed must be a whole number from 0 to 18446744073709551615.";
    return;
  }
  const fi = Math.max(0, Math.floor(Number($<HTMLInputElement>("flowerIdx").value) || 0));
  if (sourceSel.value !== "core") return show(stubFlower(Number(seed) | 0, fi), `Stub flower`);
  const mine = ++request;
  const t0 = performance.now();
  const tick = () => {
    if (mine === request) $("info").textContent = `Growing plant ${seed}, flower ${fi}… ${Math.round((performance.now() - t0) / 1000)} s`;
  };
  tick();
  const timer = setInterval(tick, 1000);
  try {
    const full = $<HTMLInputElement>("fullBud").checked;
    const { json, ms, preview } = await growBud(seed, fi, full).finally(() => clearInterval(timer));
    if (mine !== request) return; // a newer seed was asked for meanwhile
    show(validate(JSON.parse(json)), `Core bud${preview ? " (preview)" : ""}, ${(ms / 1000).toFixed(1)} s`);
  } catch (e) {
    if (mine === request) $("err").textContent = String((e as Error).message ?? e);
  }
}

/** Point the Flower menu at a loaded file that is not one of its entries. */
function markFileSource(name: string) {
  let opt = [...sourceSel.options].find((o) => o.value === "file");
  if (!opt) sourceSel.appendChild((opt = new Option("", "file")));
  opt.text = `File: ${name}`;
  sourceSel.value = "file";
  $("seedRow").style.display = "none";
  $("flowerRow").style.display = "none";
  $("fullRow").style.display = "none";
}

async function loadJsonText(text: string, name: string) {
  try {
    if (![...sourceSel.options].some((o) => o.value !== "file" && (o.text === name || o.value === name))) markFileSource(name);
    show(validate(JSON.parse(text)), name);
  } catch (e) {
    $("err").textContent = String((e as Error).message ?? e);
  }
}

// UI
$("source").addEventListener("change", async (e) => {
  const v = (e.target as HTMLSelectElement).value;
  if (v === "file") return;
  const seeded = v === "stub" || v === "core";
  $("seedRow").style.display = seeded ? "" : "none";
  $("flowerRow").style.display = seeded ? "" : "none";
  $("fullRow").style.display = v === "core" ? "" : "none";
  if (seeded) return loadSeed($<HTMLInputElement>("seed").value);
  try {
    loadJsonText(await (await fetch(v)).text(), sourceSel.selectedOptions[0].text);
  } catch (err) {
    $("err").textContent = String(err);
  }
});
$("seed").addEventListener("change", () => loadSeed($<HTMLInputElement>("seed").value));
$("flowerIdx").addEventListener("change", () => loadSeed($<HTMLInputElement>("seed").value));
$("fullBud").addEventListener("change", () => loadSeed($<HTMLInputElement>("seed").value));
$("sibling").addEventListener("click", () => {
  const el = $<HTMLInputElement>("flowerIdx");
  el.value = String((Number(el.value) || 0) + 1);
  loadSeed($<HTMLInputElement>("seed").value);
});
$("reroll").addEventListener("click", () => {
  const s = Math.floor(Math.random() * 1e6);
  $<HTMLInputElement>("seed").value = String(s);
  loadSeed(String(s));
});
$("time").addEventListener("input", (e) => {
  playing = false;
  $("play").textContent = "Play";
  setTime(Number((e.target as HTMLInputElement).value) * ((flower?.frameCount ?? 1) - 1));
});
$("play").addEventListener("click", () => {
  if (!flower) return;
  if (!playing && time >= flower.frameCount - 1) setTime(0);
  playing = !playing;
  $("play").textContent = playing ? "Pause" : "Play";
});
$("prev").addEventListener("click", () => setTime(Math.ceil(time) - 1));
$("next").addEventListener("click", () => setTime(Math.floor(time) + 1));
$("shells").addEventListener("change", (e) => {
  const s = group.getObjectByName("shells");
  if (s) s.visible = (e.target as HTMLInputElement).checked;
});
$("plantParts").addEventListener("change", (e) => {
  if (plant) plant.group.visible = (e.target as HTMLInputElement).checked;
});
$("wire").addEventListener("change", (e) => (petalMat.wireframe = schematicMat.wireframe = (e.target as HTMLInputElement).checked));
$("glbStatic").addEventListener("click", async () => {
  if (flower) download(await exportStatic(flower, time), `automark-${flower.doc.seed}-step${Math.round(time)}.glb`);
});
$("glbAnim").addEventListener("click", async () => {
  if (flower) download(await exportAnimated(flower, Number($<HTMLSelectElement>("speed").value)), `automark-${flower.doc.seed}-opening.glb`);
});
$("load").addEventListener("click", () => $("file").click());
$("file").addEventListener("change", async (e) => {
  const f = (e.target as HTMLInputElement).files?.[0];
  if (f) loadJsonText(await f.text(), f.name);
});
addEventListener("dragover", (e) => { e.preventDefault(); document.body.classList.add("dragging"); });
addEventListener("dragleave", () => document.body.classList.remove("dragging"));
addEventListener("drop", async (e) => {
  e.preventDefault();
  document.body.classList.remove("dragging");
  const f = e.dataTransfer?.files?.[0];
  if (f) loadJsonText(await f.text(), f.name);
});

// The canvas fills #app, which is the whole window on desktop and the space
// under the control bar on phones.
const narrow = matchMedia("(max-width: 700px)");
if (narrow.matches) {
  $<HTMLDetailsElement>("more").open = false;
  $<HTMLDetailsElement>("views").open = false;
}
function resize() {
  const el = $("app");
  const w = el.clientWidth || innerWidth, h = el.clientHeight || innerHeight;
  renderer.setSize(w, h);
  camera.aspect = w / h;
  camera.updateProjectionMatrix();
}
new ResizeObserver(resize).observe($("app"));
resize();

const clock = new THREE.Clock();
renderer.setAnimationLoop(() => {
  const dt = clock.getDelta();
  if (playing && flower) {
    setTime(time + dt * Number($<HTMLSelectElement>("speed").value));
    if (time >= flower.frameCount - 1) {
      playing = false;
      $("play").textContent = "Play";
    }
  }
  controls.update();
  renderer.render(scene, camera);
});

// ?src=flower.json loads a core export; ?seed=N&t=0..1 for the stub.
const q = new URLSearchParams(location.search);
const src = q.get("src");
if (src) {
  const opt = [...$<HTMLSelectElement>("source").options].find((o) => o.value === src);
  if (opt) { sourceSel.value = src; $("seedRow").style.display = "none"; $("flowerRow").style.display = "none"; $("fullRow").style.display = "none"; }
  fetch(src).then((r) => r.text()).then((t) => loadJsonText(t, src)).catch((e) => ($("err").textContent = String(e)));
} else {
  const seed = q.get("seed") ?? "7";
  $<HTMLInputElement>("seed").value = seed;
  $<HTMLInputElement>("flowerIdx").value = q.get("flower") ?? "0";
  loadSeed(seed);
}
const loaded = flower as FlowerMesh | null;
if (q.get("t") && loaded) setTime(Number(q.get("t")) * (loaded.frameCount - 1));

// Hooks for headless checks.
(window as any).automark = { setTime, exportStatic: () => flower && exportStatic(flower, time), exportAnimated: () => flower && exportAnimated(flower), get flower() { return flower; } };
