// glTF (.glb) export: the current pose, or the whole opening as morph targets
// with a keyframed weight track that game engines play as an animation clip.

import * as THREE from "three";
import { GLTFExporter } from "three/examples/jsm/exporters/GLTFExporter.js";
import type { FlowerMesh } from "./mesh";

function exportMaterial() {
  return new THREE.MeshStandardMaterial({ vertexColors: true, roughness: 0.55, side: THREE.DoubleSide, name: "petal" });
}

async function toGlb(scene: THREE.Object3D, animations: THREE.AnimationClip[] = []): Promise<ArrayBuffer> {
  const res = await new GLTFExporter().parseAsync(scene, { binary: true, animations });
  return res as ArrayBuffer;
}

export async function exportStatic(fm: FlowerMesh, time: number): Promise<ArrayBuffer> {
  const geo = fm.geometry.clone();
  fm.fill(time, geo.getAttribute("position").array as Float32Array);
  geo.computeVertexNormals();
  const mesh = new THREE.Mesh(geo, exportMaterial());
  mesh.name = fm.mesh.name;
  const buf = await toGlb(mesh);
  geo.dispose();
  return buf;
}

/**
 * Keyframes after the first become morph targets; the clip plays core steps at
 * `stepsPerSecond`. Long openings are resampled to `maxKeys` evenly spaced keys
 * (interpolated between core steps) to keep the file size down.
 */
export async function exportAnimated(fm: FlowerMesh, stepsPerSecond = 12, maxKeys = 24): Promise<ArrayBuffer> {
  const last = fm.frameCount - 1;
  const F = Math.max(2, Math.min(fm.frameCount, maxKeys));
  const keyStep = (f: number) => (f * last) / (F - 1);
  const geo = fm.geometry.clone();
  const basePos = geo.getAttribute("position") as THREE.BufferAttribute;
  fm.fill(0, basePos.array as Float32Array);
  geo.computeVertexNormals();
  const morphPos: THREE.BufferAttribute[] = [];
  const morphNrm: THREE.BufferAttribute[] = [];
  const scratch = new THREE.BufferGeometry();
  scratch.setIndex(geo.getIndex());
  for (let f = 1; f < F; f++) {
    const p = new Float32Array(basePos.array.length);
    fm.fill(keyStep(f), p);
    const attr = new THREE.BufferAttribute(p, 3);
    scratch.setAttribute("position", attr);
    scratch.computeVertexNormals();
    attr.name = `frame${f}`;
    morphPos.push(attr);
    morphNrm.push((scratch.getAttribute("normal") as THREE.BufferAttribute).clone());
  }
  geo.morphAttributes.position = morphPos;
  geo.morphAttributes.normal = morphNrm;
  geo.morphTargetsRelative = false;

  const mesh = new THREE.Mesh(geo, exportMaterial());
  mesh.name = fm.mesh.name;
  mesh.updateMorphTargets();

  const K = F - 1;
  const times: number[] = [];
  const values: number[] = [];
  for (let f = 0; f < F; f++) {
    times.push(keyStep(f) / stepsPerSecond);
    for (let k = 0; k < K; k++) values.push(k === f - 1 ? 1 : 0);
  }
  const track = new THREE.NumberKeyframeTrack(`${mesh.name}.morphTargetInfluences`, times, values);
  const clip = new THREE.AnimationClip("opening", -1, [track]);
  const buf = await toGlb(mesh, [clip]);
  geo.dispose();
  return buf;
}

export function download(buf: ArrayBuffer, name: string) {
  const url = URL.createObjectURL(new Blob([buf], { type: "model/gltf-binary" }));
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
