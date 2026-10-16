// Generic latent channels: carry each channel's last value forward through
// the frames, and turn any channel into per-vertex colours. Nothing here knows
// what a channel means.

import * as THREE from "three";
import type { ChannelDecl, ChannelValue, FlowerDoc } from "./format";

const INFERNO = ["#000004", "#1b0c41", "#4a0c6b", "#781c6d", "#a52c60", "#cf4446", "#ed6925", "#fb9b06", "#f7d13d", "#fcffa4"].map(
  (h) => new THREE.Color(h),
);
const CATEGORICAL = ["#4e79a7", "#f28e2b", "#59a14f", "#e15759", "#76b7b2", "#edc948", "#b07aa1", "#ff9da7"].map((h) => new THREE.Color(h));
const MISSING = new THREE.Color("#9a9a9a");

export function rampColor(x: number, out = new THREE.Color()) {
  const t = Math.min(1, Math.max(0, x)) * (INFERNO.length - 1);
  const i = Math.min(INFERNO.length - 2, Math.floor(t));
  return out.copy(INFERNO[i]).lerp(INFERNO[i + 1], t - i);
}
export const rampCss = (x: number) => `#${rampColor(x).getHexString()}`;
export const categoryCss = (k: number) => `#${CATEGORICAL[((k % CATEGORICAL.length) + CATEGORICAL.length) % CATEGORICAL.length].getHexString()}`;

/** Resolved values: [frame][petal] for petal/point scope, [frame] for flower scope; undefined until first set. */
export class Channels {
  readonly decls: ChannelDecl[];
  private readonly flower = new Map<string, (ChannelValue | undefined)[]>();
  private readonly petal = new Map<string, (ChannelValue | undefined)[][]>();
  /** Observed min/max of the scalar values, for channels with no declared range. */
  private readonly observed = new Map<string, [number, number]>();

  constructor(readonly doc: FlowerDoc) {
    this.decls = doc.channels ?? [];
    for (const c of this.decls) {
      if (c.scope === "flower") {
        let last: ChannelValue | undefined;
        this.flower.set(c.name, doc.frames.map((f) => (last = f.channels?.[c.name] ?? last)));
      } else {
        const last: (ChannelValue | undefined)[] = doc.petals.map(() => undefined);
        this.petal.set(
          c.name,
          doc.frames.map((f) => f.petals.map((p, pi) => (last[pi] = p.channels?.[c.name] ?? last[pi]))),
        );
      }
      if (c.dims === 1) {
        let lo = Infinity, hi = -Infinity;
        const see = (v: ChannelValue | undefined) => {
          for (const x of v === undefined ? [] : Array.isArray(v) ? v : [v]) {
            if (x < lo) lo = x;
            if (x > hi) hi = x;
          }
        };
        this.flower.get(c.name)?.forEach(see);
        this.petal.get(c.name)?.forEach((fr) => fr.forEach(see));
        if (lo <= hi) this.observed.set(c.name, [lo, hi]);
      }
    }
  }

  get(name: string) {
    return this.decls.find((c) => c.name === name);
  }

  /** Flower-scope scalar at `time` (frames, fractional), interpolated; undefined if never set. */
  flowerScalar(name: string, time: number): number | undefined {
    const vals = this.flower.get(name);
    if (!vals) return undefined;
    const last = vals.length - 1;
    const tc = Math.min(last, Math.max(0, time));
    const f0 = Math.floor(tc), f1 = Math.min(last, f0 + 1);
    const a = vals[f0], b = vals[f1] ?? a;
    if (typeof a !== "number") return undefined;
    return typeof b === "number" ? a + (b - a) * (tc - f0) : a;
  }

  /** The value range a colour ramp spans for a scalar channel. */
  range(c: ChannelDecl): [number, number] {
    const r = c.range ?? this.observed.get(c.name) ?? [0, 1];
    return r[1] > r[0] ? r : [r[0] - 0.5, r[0] + 0.5];
  }

  /**
   * Per-vertex colours for `name` at the nearest frame, laid out like FlowerMesh
   * (petal pi owns vertices pi*2n .. pi*2n+2n-1: outer face then inner face).
   */
  colors(name: string, time: number, out: Float32Array) {
    const c = this.get(name);
    if (!c) return;
    const { nu, nv } = this.doc.grid;
    const n = nu * nv;
    const f = Math.round(Math.min(this.doc.frames.length - 1, Math.max(0, time)));
    const [lo, hi] = c.dims === 1 ? this.range(c) : [0, 1];
    const col = new THREE.Color();
    const pick = (v: ChannelValue | undefined, j: number) => {
      if (v === undefined) return col.copy(MISSING);
      const arr = Array.isArray(v) ? v : [v];
      if (c.dims === 3) {
        const k = c.scope === "point" ? 3 * j : 0;
        return col.setRGB(arr[k], arr[k + 1], arr[k + 2], THREE.SRGBColorSpace);
      }
      const x = arr[c.scope === "point" ? j : 0];
      if (c.labels) return col.copy(CATEGORICAL[((Math.round(x) % 8) + 8) % 8]);
      return rampColor((x - lo) / (hi - lo), col);
    };
    this.doc.petals.forEach((_, pi) => {
      const v = c.scope === "flower" ? this.flower.get(name)![f] : this.petal.get(name)![f][pi];
      for (let j = 0; j < n; j++) {
        pick(v, j);
        for (const side of [0, n]) col.toArray(out, 3 * (pi * 2 * n + side + j));
      }
    });
  }

  /** Whether the channel is set again after the first frame, so its colours can change. */
  varies(name: string): boolean {
    return this.doc.frames.slice(1).some((f) => f.channels?.[name] !== undefined || f.petals.some((p) => p.channels?.[name] !== undefined));
  }
}
