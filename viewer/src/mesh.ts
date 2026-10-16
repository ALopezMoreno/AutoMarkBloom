// Turns a FlowerDoc into one solid three.js mesh: each petal's mid-surface is
// offset by +-thick/2 along its normal and the rim is closed. Topology is fixed
// across frames, so playback only rewrites the position buffer.
//
// The index buffer is ordered by layer (inner first), so showing only the
// inner layers ("peeling") is just a draw range.

import * as THREE from "three";
import type { FlowerDoc } from "./format";
import { layerColor } from "./palette";
import { Channels } from "./channels";

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
  /** Petal outlines (rim of each outer face) for the schematic view; shares the mesh's buffers. */
  readonly rims: THREE.LineSegments;
  readonly geometry = new THREE.BufferGeometry();
  readonly layerCount: number;
  readonly channels: Channels;
  /** 0 = packed; larger pushes each layer outward along its petals' attachment angle. */
  explode = 0;
  /** Curved (Catmull-Rom) interpolation between saved steps instead of straight lines. */
  smooth = true;
  private readonly perPetal: number;
  private readonly natural: Float32Array;
  private readonly byLayer: Float32Array;
  /** Index count that draws layers 0..k. */
  private readonly layerEnds: number[] = [];
  private readonly rimEnds: number[] = [];
  private readonly samples: PetalSample[];
  /** Rank of each petal's layer among the layers present (0 = innermost). */
  private readonly layerIndex: number[];

  constructor(readonly doc: FlowerDoc, material: THREE.Material, rimMaterial: THREE.Material = new THREE.LineBasicMaterial({ vertexColors: true })) {
    const { nu, nv } = doc.grid;
    const n = nu * nv;
    this.perPetal = 2 * n;
    const P = doc.petals.length;
    this.samples = doc.petals.map(() => ({ mid: new Float32Array(3 * n), thick: new Float32Array(n) }));
    const layers = [...new Set(doc.petals.map((p) => p.layer))].sort((a, b) => a - b);
    this.layerCount = layers.length;
    this.channels = new Channels(doc);
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
    const rimIndex: number[] = [];
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
        rimIndex.push(o + i, o + j);
      }
      const li = layers.indexOf(petal.layer);
      this.layerEnds[li] = index.length;
      this.rimEnds[li] = rimIndex.length;
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
            lc.toArray(this.byLayer, 3 * (o + side + g(u, v)));
          }
        }
    }
    for (let k = 1; k < layers.length; k++) {
      this.layerEnds[k] ??= this.layerEnds[k - 1];
      this.rimEnds[k] ??= this.rimEnds[k - 1];
    }

    this.geometry.setAttribute("position", new THREE.BufferAttribute(pos, 3));
    this.geometry.setAttribute("color", new THREE.BufferAttribute(this.natural.slice(), 3));
    this.geometry.setIndex(index);
    this.mesh = new THREE.Mesh(this.geometry, material);
    const rimGeo = new THREE.BufferGeometry();
    rimGeo.setAttribute("position", this.geometry.getAttribute("position"));
    rimGeo.setAttribute("color", this.geometry.getAttribute("color"));
    rimGeo.setIndex(rimIndex);
    this.rims = new THREE.LineSegments(rimGeo, rimMaterial);
    this.rims.visible = false;
    this.rims.frustumCulled = false;
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

  /** "natural", "layer", or the name of a declared channel (coloured at `time`). */
  setColorMode(mode: string, time = 0) {
    const attr = this.geometry.getAttribute("color") as THREE.BufferAttribute;
    const arr = attr.array as Float32Array;
    if (mode === "natural") arr.set(this.natural);
    else if (mode === "layer" || mode === "schematic") arr.set(this.byLayer);
    else this.channels.colors(mode, time, arr);
    attr.needsUpdate = true;
  }

  /**
   * Sepal (envelope) scaled radius at `time`: the sepal_radius channel, else the
   * legacy per-frame or shells envelope, else just outside the outermost shell.
   */
  envelope(time: number): number {
    const ch = this.channels.flowerScalar("sepal_radius", time);
    if (ch !== undefined) return ch;
    const fr = this.doc.frames, last = fr.length - 1;
    const tc = Math.min(last, Math.max(0, time));
    const f0 = Math.floor(tc), f1 = Math.min(last, f0 + 1);
    const a = fr[f0].envelope, b = fr[f1].envelope;
    if (a !== undefined) return b !== undefined ? a + (b - a) * (tc - f0) : a;
    const sh = this.doc.shells;
    if (sh.envelope !== undefined) return sh.envelope;
    const r = sh.radius;
    const slot = sh.slot ?? (r.length > 1 ? r[1] - r[0] : 0.1);
    return (r[r.length - 1] ?? sh.core) + slot / 2;
  }

  /** Largest scaled radius reached by any petal surface at `time` (outer face included). */
  outerRadius(time: number): number {
    const a = this.doc.shells.aspect;
    let best = 0;
    for (const { mid, thick } of this.sample(time)) {
      for (let j = 0; j < thick.length; j++) {
        const x = mid[3 * j], y = mid[3 * j + 1], z = mid[3 * j + 2] / a;
        const r = Math.sqrt(x * x + y * y + z * z) + thick[j] / 2;
        if (r > best) best = r;
      }
    }
    return best;
  }

  /** "step k: bud t=-0.50" style label for a frame. */
  frameLabel(time: number): string {
    const f = this.doc.frames[Math.round(Math.min(this.frameCount - 1, Math.max(0, time)))];
    return `${f.stage ?? (f.t <= 0 ? "bud" : "open")} t=${f.t.toFixed(2)}`;
  }

  /** Draw only layers 0..k-1 (k = layerCount shows everything). */
  showLayers(k: number) {
    const i = Math.max(1, Math.min(this.layerCount, k)) - 1;
    this.geometry.setDrawRange(0, this.layerEnds[i]);
    this.rims.geometry.setDrawRange(0, this.rimEnds[i]);
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
    const frames = this.doc.frames;
    const A = frames[f0].petals, B = frames[f1].petals;
    if (!this.smooth || a === 0 || last < 2) {
      A.forEach((pa, pi) => {
        const pb = B[pi], s = this.samples[pi];
        for (let j = 0; j < 3 * n; j++) s.mid[j] = pa.mid[j] + (pb.mid[j] - pa.mid[j]) * a;
        for (let j = 0; j < n; j++) s.thick[j] = pa.thick[j] + (pb.thick[j] - pa.thick[j]) * a;
      });
      return this.samples;
    }
    // Catmull-Rom through the neighbouring steps (ends repeat), so motion has no corners at saved steps.
    const Z = frames[Math.max(0, f0 - 1)].petals, C = frames[Math.min(last, f1 + 1)].petals;
    const a2 = a * a, a3 = a2 * a;
    const w0 = 0.5 * (-a3 + 2 * a2 - a), w1 = 0.5 * (3 * a3 - 5 * a2 + 2);
    const w2 = 0.5 * (-3 * a3 + 4 * a2 + a), w3 = 0.5 * (a3 - a2);
    A.forEach((pa, pi) => {
      const pz = Z[pi], pb = B[pi], pc = C[pi], s = this.samples[pi];
      for (let j = 0; j < 3 * n; j++) s.mid[j] = w0 * pz.mid[j] + w1 * pa.mid[j] + w2 * pb.mid[j] + w3 * pc.mid[j];
      // Thickness stays linear: a cubic could overshoot below zero.
      for (let j = 0; j < n; j++) s.thick[j] = pa.thick[j] + (pb.thick[j] - pa.thick[j]) * a;
    });
    return this.samples;
  }

  /** Writes positions for `time` into `out` (three.js frame, +y up). */
  fill(time: number, out: Float32Array, explode = this.explode) {
    const { nu, nv } = this.doc.grid;
    const n = nu * nv;
    const m = new Float32Array(3 * n);
    const env = this.envelope(time);
    this.sample(time).forEach((s, pi) => {
      const petal = this.doc.petals[pi];
      const li = this.layerIndex[pi];
      // Outer layers slide outward, inner layers rise out of the bud.
      const frac = li / Math.max(1, this.layerCount - 1);
      const push = explode * env * (0.1 + 0.9 * frac);
      const lift = explode * env * this.doc.shells.aspect * 1.2 * (1 - frac);
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
    this.rims.geometry.dispose();
  }
}
