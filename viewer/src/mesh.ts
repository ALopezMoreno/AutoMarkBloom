// Turns a FlowerDoc into one solid three.js mesh: each petal's mid-surface is
// offset by +-thick/2 along its normal and the rim is closed. Topology is fixed
// across frames, so playback only rewrites the position buffer.
//
// The index buffer is ordered by layer (inner first), so showing only the
// inner layers ("peeling") is just a draw range.

import * as THREE from "three";
import type { FlowerDoc } from "./format";
import { layerColor } from "./palette";

const BASE = new THREE.Color("#fbeedd");
const PINK = new THREE.Color("#e9849f");
const DEEP = new THREE.Color("#b2456c");

/** One petal's mid-surface and thickness at some time, in the core frame (+z up). */
export interface PetalSample {
  mid: Float32Array;
  thick: Float32Array;
}

export class FlowerMesh {
  readonly mesh: THREE.Mesh;
  readonly geometry = new THREE.BufferGeometry();
  readonly layerCount: number;
  /** 0 = packed; larger pushes each layer outward along its petals' attachment angle. */
  explode = 0;
  private readonly perPetal: number;
  private readonly natural: Float32Array;
  private readonly byLayer: Float32Array;
  /** Index count that draws layers 0..k. */
  private readonly layerEnds: number[] = [];
  private readonly samples: PetalSample[];
  /** Rank of each petal's layer among the layers present (0 = innermost). */
  private readonly layerIndex: number[];

  constructor(readonly doc: FlowerDoc, material: THREE.Material) {
    const { nu, nv } = doc.grid;
    const n = nu * nv;
    this.perPetal = 2 * n;
    const P = doc.petals.length;
    this.samples = doc.petals.map(() => ({ mid: new Float32Array(3 * n), thick: new Float32Array(n) }));
    const layers = [...new Set(doc.petals.map((p) => p.layer))].sort((a, b) => a - b);
    this.layerCount = layers.length;
    this.layerIndex = doc.petals.map((p) => layers.indexOf(p.layer));

    const pos = new Float32Array(P * this.perPetal * 3);
    this.natural = new Float32Array(P * this.perPetal * 3);
    this.byLayer = new Float32Array(P * this.perPetal * 3);
    const g = (u: number, v: number) => u * nv + v;
    const loop: number[] = [];
    for (let v = 0; v < nv - 1; v++) loop.push(g(0, v));
    for (let u = 0; u < nu - 1; u++) loop.push(g(u, nv - 1));
    for (let v = nv - 1; v > 0; v--) loop.push(g(nu - 1, v));
    for (let u = nu - 1; u > 0; u--) loop.push(g(u, 0));

    const index: number[] = [];
    const order = doc.petals.map((_, i) => i).sort((a, b) => doc.petals[a].layer - doc.petals[b].layer || a - b);
    const maxLayer = Math.max(1, layers[layers.length - 1]);
    const c = new THREE.Color();
    const lc = new THREE.Color();
    for (const pi of order) {
      const petal = doc.petals[pi];
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
      const li = layers.indexOf(petal.layer);
      this.layerEnds[li] = index.length;
      const inner = 1 - petal.layer / maxLayer;
      layerColor(li, layers.length, lc);
      for (let u = 0; u < nu; u++)
        for (let v = 0; v < nv; v++) {
          const s = u / (nu - 1);
          c.copy(BASE).lerp(PINK, Math.min(1, s * 1.6));
          c.lerp(DEEP, 0.25 * (1 - inner) * s);
          c.lerp(BASE, 0.25 * inner);
          for (const side of [0, n]) {
            c.toArray(this.natural, 3 * (o + side + g(u, v)));
            // Inner face a little darker so the two sides read apart in sections.
            lc.clone().multiplyScalar(side ? 0.75 : 1).toArray(this.byLayer, 3 * (o + side + g(u, v)));
          }
        }
    }
    for (let k = 1; k < layers.length; k++) this.layerEnds[k] ??= this.layerEnds[k - 1];

    this.geometry.setAttribute("position", new THREE.BufferAttribute(pos, 3));
    this.geometry.setAttribute("color", new THREE.BufferAttribute(this.natural.slice(), 3));
    this.geometry.setIndex(index);
    this.mesh = new THREE.Mesh(this.geometry, material);
    this.mesh.name = `automark-${doc.seed}`;
    this.setTime(0);
  }

  get frameCount() {
    return this.doc.frames.length;
  }

  /** Natural petal colours, for exports. */
  get naturalColors(): Float32Array {
    return this.natural;
  }

  setColorMode(mode: "natural" | "layer") {
    const attr = this.geometry.getAttribute("color") as THREE.BufferAttribute;
    (attr.array as Float32Array).set(mode === "layer" ? this.byLayer : this.natural);
    attr.needsUpdate = true;
  }

  /** Draw only layers 0..k-1 (k = layerCount shows everything). */
  showLayers(k: number) {
    const i = Math.max(1, Math.min(this.layerCount, k)) - 1;
    this.geometry.setDrawRange(0, this.layerEnds[i]);
  }

  /** time in frames: 0 .. frameCount-1, fractional values interpolate. */
  setTime(time: number) {
    const pos = this.geometry.getAttribute("position") as THREE.BufferAttribute;
    this.fill(time, pos.array as Float32Array);
    pos.needsUpdate = true;
    this.geometry.computeVertexNormals();
    this.geometry.computeBoundingSphere();
  }

  /** Petal mid-surfaces and thickness at `time`, core frame. Reused buffers: copy if kept. */
  sample(time: number): PetalSample[] {
    const n = this.doc.grid.nu * this.doc.grid.nv;
    const last = this.frameCount - 1;
    const tc = Math.min(last, Math.max(0, time));
    const f0 = Math.floor(tc), f1 = Math.min(last, f0 + 1), a = tc - f0;
    const A = this.doc.frames[f0].petals, B = this.doc.frames[f1].petals;
    A.forEach((pa, pi) => {
      const pb = B[pi], s = this.samples[pi];
      for (let j = 0; j < 3 * n; j++) s.mid[j] = pa.mid[j] + (pb.mid[j] - pa.mid[j]) * a;
      for (let j = 0; j < n; j++) s.thick[j] = pa.thick[j] + (pb.thick[j] - pa.thick[j]) * a;
    });
    return this.samples;
  }

  /** Writes positions for `time` into `out` (three.js frame, +y up). */
  fill(time: number, out: Float32Array, explode = this.explode) {
    const { nu, nv } = this.doc.grid;
    const n = nu * nv;
    const m = new Float32Array(3 * n);
    this.sample(time).forEach((s, pi) => {
      const petal = this.doc.petals[pi];
      const li = this.layerIndex[pi];
      // Outer layers slide outward, inner layers rise out of the bud.
      const frac = li / Math.max(1, this.layerCount - 1);
      const push = explode * this.doc.shells.envelope * (0.1 + 0.9 * frac);
      const lift = explode * this.doc.shells.envelope * this.doc.shells.aspect * 1.2 * (1 - frac);
      const ox = push * Math.cos(petal.theta), oy = push * Math.sin(petal.theta);
      // core (x, y, z) z-up  ->  three (x, z, -y) y-up
      for (let j = 0; j < 3 * n; j += 3) {
        m[j] = s.mid[j] + ox;
        m[j + 1] = s.mid[j + 2] + lift;
        m[j + 2] = -(s.mid[j + 1] + oy);
      }
      const h = s.thick;
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
          const k = h[j] / 2 / l;
          nx *= k; ny *= k; nz *= k;
          const t = 3 * (o + j), b = 3 * (o + n + j);
          out[t] = m[3 * j] + nx; out[t + 1] = m[3 * j + 1] + ny; out[t + 2] = m[3 * j + 2] + nz;
          out[b] = m[3 * j] - nx; out[b + 1] = m[3 * j + 1] - ny; out[b + 2] = m[3 * j + 2] - nz;
        }
    });
  }

  dispose() {
    this.geometry.dispose();
  }
}
