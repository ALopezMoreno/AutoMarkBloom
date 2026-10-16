// The green parts around the bud, for context: a stem with a swollen
// receptacle under the bud, and five sepals wrapping it. Sepals hug the sepal
// envelope (sepal_radius) as the bud swells and peel back as it opens. These
// are drawn by the viewer only; the core does not model them yet.

import * as THREE from "three";

const SEPALS = 5;
const NU = 18, NV = 11;
const PHI_START = 0.12; // sepals start just above the bottom pole
const PHI_TIP = 1.65; // and reach a little past the bud's equator

/** Core (x, y, z) z-up -> three (x, z, -y) y-up. */
const put = (out: Float32Array, i: number, x: number, y: number, z: number) => {
  out[3 * i] = x;
  out[3 * i + 1] = z;
  out[3 * i + 2] = -y;
};

export class PlantParts {
  readonly group = new THREE.Group();
  private readonly sepals: THREE.Mesh[] = [];
  private readonly stem: THREE.Mesh;

  constructor(private readonly aspect: number, private readonly core: number, seed: string, material: { stem: THREE.Material; sepal: THREE.Material }) {
    this.group.name = "plant";
    // Stem: a tapered tube from the receptacle down, with a gentle bend.
    const len = 6 * core;
    const curve = new THREE.CatmullRomCurve3([
      new THREE.Vector3(0, -aspect * core * 0.95, 0),
      new THREE.Vector3(0.05 * core, -aspect * core - len * 0.35, 0.02 * core),
      new THREE.Vector3(0.18 * core, -aspect * core - len * 0.7, 0.08 * core),
      new THREE.Vector3(0.35 * core, -aspect * core - len, 0.15 * core),
    ]);
    const tube = new THREE.TubeGeometry(curve, 48, 0.16 * core, 16, false);
    this.stem = new THREE.Mesh(tube, material.stem);
    // Receptacle: the swelling where the stem meets the bud.
    const rec = new THREE.Mesh(new THREE.SphereGeometry(0.42 * core, 32, 16), material.stem);
    rec.scale.set(1, 0.7, 1);
    rec.position.set(0, -aspect * core * 0.92, 0);
    this.group.add(this.stem, rec);

    // Sepals, evenly spaced with a seed-dependent twist so siblings differ.
    let h = 0;
    for (const ch of seed) h = (h * 31 + ch.charCodeAt(0)) >>> 0;
    const offset = ((h % 1000) / 1000) * ((2 * Math.PI) / SEPALS);
    const index: number[] = [];
    for (let u = 0; u < NU - 1; u++)
      for (let v = 0; v < NV - 1; v++) {
        const a = u * NV + v, b = a + NV;
        index.push(a, b, a + 1, b, b + 1, a + 1);
      }
    for (let k = 0; k < SEPALS; k++) {
      const geo = new THREE.BufferGeometry();
      geo.setAttribute("position", new THREE.BufferAttribute(new Float32Array(NU * NV * 3), 3));
      geo.setIndex(index);
      const m = new THREE.Mesh(geo, material.sepal);
      m.userData.theta = offset + (2 * Math.PI * k) / SEPALS;
      m.userData.width = ((2 * Math.PI) / SEPALS) * (1.05 + 0.15 * (((h >> k) & 7) / 7));
      this.sepals.push(m);
      this.group.add(m);
    }
  }

  /**
   * Lay the sepals on the envelope of scaled radius `envelope`.
   * `open` in [0, 1] peels them back (0 = closed bud); `explode` pushes them outward.
   */
  update(envelope: number, open: number, explode: number) {
    const R = envelope * 1.03;
    for (const m of this.sepals) {
      const pos = m.geometry.getAttribute("position") as THREE.BufferAttribute;
      const arr = pos.array as Float32Array;
      const th0 = m.userData.theta as number, w = m.userData.width as number;
      const push = explode * envelope * 1.1;
      for (let iu = 0; iu < NU; iu++) {
        const u = iu / (NU - 1);
        const phi = PHI_START + u * (PHI_TIP - PHI_START);
        // Ovate leaf: broad below the middle, pointed tip.
        const half = 0.5 * w * Math.pow(Math.sin(Math.PI * Math.min(1, u * 0.92 + 0.04)), 0.8) * (1 - 0.85 * Math.pow(u, 3));
        // Opening: the sepal bends outward and down from its base.
        const peel = open * Math.pow(u, 1.5) * 1.0;
        for (let iv = 0; iv < NV; iv++) {
          const v = iv / (NV - 1) - 0.5;
          const th = th0 + 2 * v * half;
          // Slightly cupped across the width.
          const r = R * (1 + 0.03 * (1 - 4 * v * v)) ;
          let rho = r * Math.sin(phi);
          let z = -this.aspect * r * Math.cos(phi);
          // Rotate outward about the base ring as the flower opens.
          const baseRho = r * Math.sin(PHI_START), baseZ = -this.aspect * r * Math.cos(PHI_START);
          const dr = rho - baseRho, dz = z - baseZ;
          const c = Math.cos(peel), s = Math.sin(peel);
          rho = baseRho + dr * c + dz * s;
          z = baseZ - dr * s + dz * c;
          rho += push;
          put(arr, iu * NV + iv, rho * Math.cos(th), rho * Math.sin(th), z);
        }
      }
      pos.needsUpdate = true;
      m.geometry.computeVertexNormals();
      m.geometry.computeBoundingSphere();
    }
  }

  setSepalsVisible(v: boolean) {
    for (const m of this.sepals) m.visible = v;
  }

  dispose() {
    this.group.traverse((o) => (o as THREE.Mesh).geometry?.dispose());
  }
}
