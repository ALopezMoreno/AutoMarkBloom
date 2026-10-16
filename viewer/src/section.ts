// Planar cross-sections of the bud for the 2D packing schematic. Each petal's
// mid-surface grid is cut by the plane, giving polylines with the petal's
// thickness along them; shells, core and envelope are drawn from the bud outline.

import type { FlowerDoc } from "./format";
import type { PetalSample } from "./mesh";
import { layerCss } from "./palette";
import { shapeFactor, shellProfile } from "./shape";

export type SectionKind = "vertical" | "horizontal";

export interface Section {
  kind: SectionKind;
  /** vertical: azimuth of the plane through the axis (rad); horizontal: height z (core frame). */
  at: number;
}

interface Seg {
  a: [number, number];
  b: [number, number];
  ha: number;
  hb: number;
}

/** Signed distance to the plane and 2D in-plane coordinates for a core-frame point. */
function frame(sec: Section) {
  if (sec.kind === "horizontal")
    return { d: (x: number, y: number, z: number) => z - sec.at, uv: (x: number, y: number): [number, number] => [x, y] };
  const c = Math.cos(sec.at), s = Math.sin(sec.at);
  return {
    d: (x: number, y: number) => -s * x + c * y,
    uv: (x: number, y: number, z: number): [number, number] => [c * x + s * y, z],
  };
}

/** Cut segments for each petal (same order as doc.petals). */
export function cutPetals(doc: FlowerDoc, samples: PetalSample[], sec: Section): Seg[][] {
  const { nu, nv } = doc.grid;
  const f = frame(sec);
  return samples.map(({ mid, thick }) => {
    const n = nu * nv;
    const d = new Float32Array(n);
    for (let j = 0; j < n; j++) d[j] = f.d(mid[3 * j], mid[3 * j + 1], mid[3 * j + 2]);
    const segs: Seg[] = [];
    const cross = (i: number, j: number): [[number, number], number] => {
      const t = d[i] / (d[i] - d[j]);
      const x = mid[3 * i] + (mid[3 * j] - mid[3 * i]) * t;
      const y = mid[3 * i + 1] + (mid[3 * j + 1] - mid[3 * i + 1]) * t;
      const z = mid[3 * i + 2] + (mid[3 * j + 2] - mid[3 * i + 2]) * t;
      return [f.uv(x, y, z), thick[i] + (thick[j] - thick[i]) * t];
    };
    const tri = (a: number, b: number, c: number) => {
      const pts: [[number, number], number][] = [];
      for (const [i, j] of [[a, b], [b, c], [c, a]]) if (d[i] < 0 !== d[j] < 0) pts.push(cross(i, j));
      if (pts.length === 2) segs.push({ a: pts[0][0], b: pts[1][0], ha: pts[0][1], hb: pts[1][1] });
    };
    for (let u = 0; u < nu - 1; u++)
      for (let v = 0; v < nv - 1; v++) {
        const a = u * nv + v, b = a + nv, c = a + 1, e = b + 1;
        tri(a, b, c);
        tri(b, e, c);
      }
    return segs;
  });
}

const SVG = "http://www.w3.org/2000/svg";
const el = (name: string, attrs: Record<string, string | number>) => {
  const e = document.createElementNS(SVG, name);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, String(v));
  return e;
};

/** Draws the section schematic into `svg`. */
export function drawSchematic(svg: SVGSVGElement, doc: FlowerDoc, samples: PetalSample[], sec: Section, visibleLayers: number, envelope: number) {
  svg.replaceChildren();
  const sh = doc.shells;
  const { aspect, core, radius } = sh;
  const layers = [...new Set(doc.petals.map((p) => p.layer))].sort((a, b) => a - b);
  const L = layers.length;
  // Petals can sit a little outside the nominal envelope after packing.
  const R = envelope * 1.12;
  const H = sec.kind === "vertical" ? aspect * R * Math.max(shapeFactor(sh, 1), shapeFactor(sh, -1)) : R;
  const pad = 0.04 * R;
  svg.setAttribute("viewBox", `${-R - pad} ${-H - pad} ${2 * (R + pad)} ${2 * (H + pad)}`);
  const g = el("g", { transform: "scale(1,-1)" }); // +z up on screen
  svg.appendChild(g);

  // Shells cut by the plane: the outline itself in a vertical section, a
  // circle at the outline's width in a horizontal one.
  const N = 128;
  const conic = (a: number, cls: string) => {
    const prof = Array.from({ length: N + 1 }, (_, i) => shellProfile(sh, a, (Math.PI * i) / N));
    if (sec.kind === "vertical") {
      const pts = [...prof, ...prof.slice(1, N).reverse().map(([r, z]) => [-r, z])];
      return el("polygon", { points: pts.map(([r, z]) => `${r},${z}`).join(" "), class: cls });
    }
    let w = -1;
    for (let i = 0; i < N; i++) {
      const [r0, z0] = prof[i], [r1, z1] = prof[i + 1];
      if (z0 <= sec.at !== z1 <= sec.at) w = Math.max(w, r0 + ((r1 - r0) * (sec.at - z0)) / (z1 - z0));
    }
    return w > 0 ? el("circle", { cx: 0, cy: 0, r: w, class: cls }) : null;
  };
  const env = conic(envelope, "envelope");
  if (env) g.appendChild(env);
  radius.forEach((a) => {
    const c = conic(a, "shell");
    if (c) g.appendChild(c);
  });
  const coreEl = conic(core, "core");
  if (coreEl) g.appendChild(coreEl);

  // Petal cuts, outer layers first so inner ones sit on top. Strokes use the
  // petal's real thickness, so gaps and contacts read at true scale.
  const cuts = cutPetals(doc, samples, sec);
  const order = doc.petals.map((_, i) => i).sort((a, b) => doc.petals[b].layer - doc.petals[a].layer);
  for (const pi of order) {
    const li = layers.indexOf(doc.petals[pi].layer);
    const segs = cuts[pi];
    if (!segs.length) continue;
    const color = layerCss(li, L);
    const pg = el("g", { stroke: color, "stroke-linecap": "round", opacity: li < visibleLayers ? 1 : 0.18 });
    for (const s of segs) {
      pg.appendChild(el("line", { x1: s.a[0], y1: s.a[1], x2: s.b[0], y2: s.b[1], "stroke-width": (s.ha + s.hb) / 2 }));
    }
    g.appendChild(pg);
  }
  if (sec.kind === "vertical") g.appendChild(el("line", { x1: 0, y1: -H, x2: 0, y2: H, class: "axis" }));

  // Scale bar: 0.5 core radii.
  const bar = el("g", { class: "scale" });
  const y = H - pad * 0.2;
  bar.appendChild(el("line", { x1: -R, y1: -y, x2: -R + 0.5, y2: -y }));
  const t = el("text", { x: -R + 0.25, y: -y + 0.1 * R, "text-anchor": "middle", "font-size": 0.07 * R });
  t.textContent = "0.5 core radius";
  bar.appendChild(t);
  svg.appendChild(bar);
}
