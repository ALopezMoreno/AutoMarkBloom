// Stand-in flower generator that emits the FORMAT.md contract until the Rust
// core produces real output. Shapes are plausible, not physically packed.

import type { FlowerDoc, PetalFrame, PetalInfo } from "./format";

function mulberry32(seed: number) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const PHI_BASE = 0.45;
const PHI_TOP_GAP = 0.1;
const smooth = (x: number) => {
  const c = Math.min(1, Math.max(0, x));
  return c * c * (3 - 2 * c);
};

export function stubFlower(seed: number, frames = 48, nu = 14, nv = 11): FlowerDoc {
  const rnd = mulberry32(seed * 2654435761 + 12345);
  const range = (a: number, b: number) => a + (b - a) * rnd();
  const layers = 6 + Math.floor(rnd() * 4);
  const aspect = range(1.15, 1.5);
  const maxThick = range(0.025, 0.04);
  const edgeThick = range(0.3, 0.5);
  const slot = maxThick * (1 + range(0.6, 1.2));
  const radius = Array.from({ length: layers }, (_, k) => 1 + slot * (k + 0.5));
  const growth = range(1.3, 1.6);

  interface P {
    info: PetalInfo;
    a: number;
    width: number;
    budLen: number;
    phiTop: number;
    phase: number;
    crumple: number;
  }
  const petals: P[] = [];
  let id = 0;
  let stagger = rnd() * Math.PI * 2;
  for (let k = 0; k < layers; k++) {
    const n = Math.round(range(3, 5) + k * range(0.6, 1.1));
    const frac = k / Math.max(1, layers - 1);
    const reach = 0.82 + 0.18 * frac; // outer petals close over the top
    const phiTop = PHI_BASE + reach * (Math.PI - PHI_TOP_GAP - PHI_BASE);
    stagger += Math.PI / n;
    for (let i = 0; i < n; i++) {
      const theta = stagger + (2 * Math.PI * i) / n + range(-0.15, 0.15);
      petals.push({
        info: { id: id++, layer: k, theta },
        a: radius[k],
        width: ((2 * Math.PI) / n) * range(1.15, 1.45),
        budLen: 0,
        phiTop: phiTop * range(0.95, 1.0),
        phase: rnd() * 10,
        crumple: range(0.5, 1),
      });
    }
  }

  // Tangent angle of the shell meridian at polar angle phi, in the (r, z) plane.
  const psiAt = (phi: number) => Math.atan2(aspect * Math.sin(phi), Math.cos(phi));

  const out: PetalFrame[][] = [];
  for (let f = 0; f < frames; f++) {
    const t = frames === 1 ? 0 : f / (frames - 1);
    const fr: PetalFrame[] = [];
    for (const p of petals) {
      const k = p.info.layer;
      const outer = k / Math.max(1, layers - 1);
      // Outer petals open first.
      const prog = smooth((t - (1 - outer) * 0.45) / 0.55);
      const r0 = p.a * Math.sin(PHI_BASE);
      const z0 = -aspect * p.a * Math.cos(PHI_BASE);
      // Bud curve is the shell meridian (sampled by arc length); the open curve
      // flares out then cups. Blending tangent angles keeps arc length sane.
      const steps = (nu - 1) * 4;
      const budPts: [number, number][] = [];
      const fine = 200;
      for (let i = 0; i <= fine; i++) {
        const phi = PHI_BASE + ((p.phiTop - PHI_BASE) * i) / fine;
        budPts.push([p.a * Math.sin(phi), -aspect * p.a * Math.cos(phi)]);
      }
      const cum = [0];
      for (let i = 1; i <= fine; i++) cum.push(cum[i - 1] + Math.hypot(budPts[i][0] - budPts[i - 1][0], budPts[i][1] - budPts[i - 1][1]));
      const budLen = cum[fine];
      const psiBud = (s: number) => {
        const target = s * budLen;
        let i = 1;
        while (i < fine && cum[i] < target) i++;
        return Math.atan2(budPts[i][1] - budPts[i - 1][1], budPts[i][0] - budPts[i - 1][0]);
      };
      const psiO0 = 0.25 + 0.8 * (1 - outer);
      const psiO1 = psiO0 + 1.1 + 1.0 * (1 - outer);
      const psi0 = psiBud(0) + (psiO0 - psiBud(0)) * prog;
      const len = budLen * (1 + (growth - 1) * prog);
      const rs: number[] = [r0];
      const zs: number[] = [z0];
      for (let s = 1; s <= steps; s++) {
        const u = (s - 0.5) / steps;
        const psi = psiBud(u) + (psiO0 + (psiO1 - psiO0) * u - psiBud(u)) * prog;
        rs.push(rs[s - 1] + Math.cos(psi) * (len / steps));
        zs.push(zs[s - 1] + Math.sin(psi) * (len / steps));
      }
      const mid = new Float32Array(nu * nv * 3);
      const thick = new Float32Array(nu * nv);
      for (let iu = 0; iu < nu; iu++) {
        const u = iu / (nu - 1);
        const r = Math.max(rs[iu * 4], 0.05);
        const z = zs[iu * 4];
        // Obovate width profile, absolute width grows with opening.
        const shape = Math.pow(u, 0.35) * Math.pow(Math.max(0, 1 - Math.pow(u, 5)), 0.3) + 0.12;
        const wAbs = p.width * p.a * shape * (1 + 0.7 * prog);
        const span = Math.min(wAbs / r, p.width * 1.1);
        for (let iv = 0; iv < nv; iv++) {
          const v = iv / (nv - 1) - 0.5;
          const th = p.info.theta - v * span; // v runs clockwise so du x dv points outward
          // Crumple in the bud, ironed out as the petal opens; ruffled tip stays.
          const crumple =
            slot * 0.35 * p.crumple * Math.sin(7 * v + 9 * u + p.phase) * u * (1 - prog) +
            0.04 * Math.sin(11 * v + p.phase) * Math.pow(u, 3) * prog;
          const rr = r + crumple * Math.cos(psi0);
          const zz = z - crumple * Math.sin(psi0) * 0.5;
          const j = iu * nv + iv;
          mid[3 * j] = rr * Math.cos(th);
          mid[3 * j + 1] = rr * Math.sin(th);
          mid[3 * j + 2] = zz;
          const edge = Math.max(Math.abs(v) * 2, u);
          thick[j] = maxThick * (1 - (1 - edgeThick) * edge);
        }
      }
      fr.push({ mid, thick });
    }
    out.push(fr);
  }

  return {
    format: "automark-flower",
    version: 1,
    seed: String(seed),
    flower_index: null,
    units: "core_radius",
    up: "+z",
    shells: { aspect, core: 1, envelope: 1 + slot * layers, radius },
    grid: { nu, nv },
    petals: petals.map((p) => p.info),
    frames: out.map((petals, f) => ({ t: frames === 1 ? 0 : f / (frames - 1), petals })),
  };
}
