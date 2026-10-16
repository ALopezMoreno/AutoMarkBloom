import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { RoomEnvironment } from "three/examples/jsm/environments/RoomEnvironment.js";
import { validate, type FlowerDoc } from "./format";
import { FlowerMesh } from "./mesh";
import { stubFlower } from "./stub";
import { download, exportAnimated, exportStatic } from "./export";

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true, preserveDrawingBuffer: true });
renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
renderer.toneMapping = THREE.ACESFilmicToneMapping;
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
const coreMat = new THREE.MeshStandardMaterial({ color: "#e7c25a", roughness: 0.8 });
const shellMat = new THREE.MeshBasicMaterial({ color: "#b2456c", wireframe: true, transparent: true, opacity: 0.12 });

let flower: FlowerMesh | null = null;
const group = new THREE.Group();
scene.add(group);
let time = 0;
let playing = false;

function show(doc: FlowerDoc, label: string) {
  group.clear();
  flower?.dispose();
  flower = new FlowerMesh(doc, petalMat);
  group.add(flower.mesh);
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
  shells.visible = $<HTMLInputElement>("shells").checked;
  group.add(shells);
  const layers = new Set(doc.petals.map((p) => p.layer)).size;
  $("info").textContent = `${label} · ${doc.petals.length} petals in ${layers} layers · ${doc.frames.length} steps · grid ${doc.grid.nu}×${doc.grid.nv}`;
  $("err").textContent = "";
  frame(flower);
  setTime(Math.min(time, flower.frameCount - 1));
}

/** Point the camera so the fully opened flower fits the view. */
function frame(fm: FlowerMesh) {
  fm.setTime(fm.frameCount - 1);
  const sphere = fm.geometry.boundingSphere!.clone();
  const dist = sphere.radius / Math.sin(THREE.MathUtils.degToRad(camera.fov / 2)) * 1.05;
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
  $("frame").textContent = `step ${Math.round(time)} / ${flower.frameCount - 1}`;
}

function loadSeed(seed: number) {
  show(stubFlower(seed), `Stub flower, seed ${seed}`);
}

async function loadJsonText(text: string, name: string) {
  try {
    show(validate(JSON.parse(text)), name);
  } catch (e) {
    $("err").textContent = String((e as Error).message ?? e);
  }
}

// UI
$("seed").addEventListener("change", () => loadSeed(Number($<HTMLInputElement>("seed").value) | 0));
$("reroll").addEventListener("click", () => {
  const s = Math.floor(Math.random() * 1e6);
  $<HTMLInputElement>("seed").value = String(s);
  loadSeed(s);
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
$("wire").addEventListener("change", (e) => (petalMat.wireframe = (e.target as HTMLInputElement).checked));
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

function resize() {
  renderer.setSize(innerWidth, innerHeight);
  camera.aspect = innerWidth / innerHeight;
  camera.updateProjectionMatrix();
}
addEventListener("resize", resize);
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
  fetch(src).then((r) => r.text()).then((t) => loadJsonText(t, src)).catch((e) => ($("err").textContent = String(e)));
} else {
  const seed = Number(q.get("seed") ?? 7) | 0;
  $<HTMLInputElement>("seed").value = String(seed);
  loadSeed(seed);
}
const loaded = flower as FlowerMesh | null;
if (q.get("t") && loaded) setTime(Number(q.get("t")) * (loaded.frameCount - 1));

// Hooks for headless checks.
(window as any).automark = { setTime, exportStatic: () => flower && exportStatic(flower, time), exportAnimated: () => flower && exportAnimated(flower), get flower() { return flower; } };
