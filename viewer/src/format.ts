// Data contract between the Rust core and the viewer. See FORMAT.md.

export interface ShellsInfo {
  aspect: number;
  core: number;
  envelope: number;
  radius: number[];
}

export interface PetalInfo {
  id: number;
  layer: number;
  theta: number;
}

export interface PetalFrame {
  /** nu*nv evaluated mid-surface points, flat xyz, row-major (u outer, v inner). Core frame, +z up. */
  mid: number[] | Float32Array;
  /** nu*nv full thickness per point. */
  thick: number[] | Float32Array;
}

export interface Frame {
  t: number;
  petals: PetalFrame[];
}

export interface FlowerDoc {
  format: "automark-flower";
  version: 1;
  seed: string;
  flower_index: number | null;
  units: "core_radius";
  up: "+z";
  shells: ShellsInfo;
  grid: { nu: number; nv: number };
  petals: PetalInfo[];
  frames: Frame[];
}

/** Throws with a readable message if `d` does not match the contract. */
export function validate(d: any): FlowerDoc {
  const fail = (m: string): never => {
    throw new Error(`Not a valid automark-flower document: ${m}`);
  };
  if (!d || d.format !== "automark-flower") fail('"format" must be "automark-flower"');
  if (d.version !== 1) fail(`unsupported version ${d.version}`);
  const { nu, nv } = d.grid ?? {};
  if (!(nu >= 2 && nv >= 2)) fail("grid.nu and grid.nv must be at least 2");
  if (!Array.isArray(d.petals) || d.petals.length === 0) fail("no petals");
  if (!Array.isArray(d.frames) || d.frames.length === 0) fail("no frames");
  const n = nu * nv;
  d.frames.forEach((f: any, fi: number) => {
    if (f.petals?.length !== d.petals.length) fail(`frame ${fi} has ${f.petals?.length} petals, expected ${d.petals.length}`);
    f.petals.forEach((p: any, pi: number) => {
      if (p.mid?.length !== 3 * n) fail(`frame ${fi} petal ${pi}: mid has ${p.mid?.length} numbers, expected ${3 * n}`);
      if (p.thick?.length !== n) fail(`frame ${fi} petal ${pi}: thick has ${p.thick?.length} numbers, expected ${n}`);
    });
  });
  return d as FlowerDoc;
}
