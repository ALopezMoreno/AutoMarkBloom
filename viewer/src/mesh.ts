// Turns a FlowerDoc into one solid three.js mesh: each petal's mid-surface is
// offset by +-thick/2 along its normal and the rim is closed. Topology is fixed
// across frames, so playback only rewrites the position buffer.

import * as THREE from "three";
import type { FlowerDoc } from "./format";

const BASE = new THREE.Color("#fbeedd");
const PINK = new THREE.Color("#e9849f");
const DEEP = new THREE.Color("#b2456c");

export class FlowerMesh {
  readonly mesh: THREE.Mesh;
  readonly geometry = new THREE.BufferGeometry();
  private readonly perPetal: number;
  private readonly mid: Float32Array;
  private readonly thick: Float32Array;

  constructor(readonly doc: FlowerDoc, material: THREE.Material) {
    const { nu, nv } = doc.grid;
    const n = nu * nv;
    this.perPetal = 2 * n;
    const P = doc.petals.length;
    this.mid = new Float32Array(3 * n);
    this.thick = new Float32Array(n);

    const pos = new Float32Array(P * this.perPetal * 3);
    const col = new Float32Array(P * this.perPetal * 3);
    const index: number[] = [];
    const g = (u: number, v: number) => u * nv + v;
    const loop: number[] = [];
    for (let v = 0; v < nv - 1; v++) loop.push(g(0, v));
    for (let u = 0; u < nu - 1; u++) loop.push(g(u, nv - 1));
    for (let v = nv - 1; v > 0; v--) loop.push(g(nu - 1, v));
    for (let u = nu - 1; u > 0; u--) loop.push(g(u, 0));

    const maxLayer = Math.max(1, ...doc.petals.map((p) => p.layer));
    const c = new THREE.Color();
    doc.petals.forEach((petal, pi) => {
      const o = pi * this.perPetal;
      for (let u = 0; u < nu - 1; u++)
        for (let v = 0; v < nv - 1; v++) {
          const a = g(u, v), b = g(u + 1, v), cc = g(u, v + 1), d = g(u + 1, v + 1);
          index.push(o + a, o + b, o + cc, o + b, o + d, o + cc); // outer face
          index.push(o + n + a, o + n + cc, o + n + b, o + n + b, o + n + cc, o + n + d); // inner face
        }
      for (let k = 0; k < loop.length; k++) {
        const i = loop[k], j = loop[(k + 1) % loop.length];
        index.push(o + i, o + j, o + n + j, o + i, o + n + j, o + n + i);
      }
      const inner = 1 - petal.layer / maxLayer;
      for (let u = 0; u < nu; u++)
        for (let v = 0; v < nv; v++) {
          const s = u / (nu - 1);
          c.copy(BASE).lerp(PINK, Math.min(1, s * 1.6));
          c.lerp(DEEP, 0.25 * (1 - inner) * s);
          c.lerp(BASE, 0.25 * inner);
          for (const side of [0, n]) c.toArray(col, 3 * (o + side + g(u, v)));
        }
    });

    this.geometry.setAttribute("position", new THREE.BufferAttribute(pos, 3));
    this.geometry.setAttribute("color", new THREE.BufferAttribute(col, 3));
    this.geometry.setIndex(index);
    this.mesh = new THREE.Mesh(this.geometry, material);
    this.mesh.name = `automark-${doc.seed}`;
    this.setTime(0);
  }

  get frameCount() {
    return this.doc.frames.length;
  }

  /** time in frames: 0 .. frameCount-1, fractional values interpolate. */
  setTime(time: number) {
    const pos = this.geometry.getAttribute("position") as THREE.BufferAttribute;
    this.fill(time, pos.array as Float32Array);
    pos.needsUpdate = true;
    this.geometry.computeVertexNormals();
    this.geometry.computeBoundingSphere();
  }

  /** Writes positions for `time` into `out` (three.js frame, +y up). */
  fill(time: number, out: Float32Array) {
    const { nu, nv } = this.doc.grid;
    const n = nu * nv;
    const last = this.frameCount - 1;
    const tc = Math.min(last, Math.max(0, time));
    const f0 = Math.floor(tc), f1 = Math.min(last, f0 + 1), a = tc - f0;
    const A = this.doc.frames[f0].petals, B = this.doc.frames[f1].petals;
    const m = this.mid, h = this.thick;
    for (let pi = 0; pi < A.length; pi++) {
      const pa = A[pi], pb = B[pi];
      for (let j = 0; j < 3 * n; j += 3) {
        // core (x, y, z) z-up  ->  three (x, z, -y) y-up
        const x = pa.mid[j] + (pb.mid[j] - pa.mid[j]) * a;
        const y = pa.mid[j + 1] + (pb.mid[j + 1] - pa.mid[j + 1]) * a;
        const z = pa.mid[j + 2] + (pb.mid[j + 2] - pa.mid[j + 2]) * a;
        m[j] = x; m[j + 1] = z; m[j + 2] = -y;
      }
      for (let j = 0; j < n; j++) h[j] = pa.thick[j] + (pb.thick[j] - pa.thick[j]) * a;
      const o = pi * 2 * n;
      for (let u = 0; u < nu; u++)
        for (let v = 0; v < nv; v++) {
          const j = u * nv + v;
          const u0 = Math.max(0, u - 1) * nv + v, u1 = Math.min(nu - 1, u + 1) * nv + v;
          const v0 = u * nv + Math.max(0, v - 1), v1 = u * nv + Math.min(nv - 1, v + 1);
          const dux = m[3 * u1] - m[3 * u0], duy = m[3 * u1 + 1] - m[3 * u0 + 1], duz = m[3 * u1 + 2] - m[3 * u0 + 2];
          const dvx = m[3 * v1] - m[3 * v0], dvy = m[3 * v1 + 1] - m[3 * v0 + 1], dvz = m[3 * v1 + 2] - m[3 * v0 + 2];
          let nx = duy * dvz - duz * dvy, ny = duz * dvx - dux * dvz, nz = dux * dvy - duy * dvx;
          const l = Math.hypot(nx, ny, nz) || 1;
          const s = h[j] / 2 / l;
          nx *= s; ny *= s; nz *= s;
          const t = 3 * (o + j), b = 3 * (o + n + j);
          out[t] = m[3 * j] + nx; out[t + 1] = m[3 * j + 1] + ny; out[t + 2] = m[3 * j + 2] + nz;
          out[b] = m[3 * j] - nx; out[b + 1] = m[3 * j + 1] - ny; out[b + 2] = m[3 * j + 2] - nz;
        }
    }
  }

  dispose() {
    this.geometry.dispose();
  }
}
