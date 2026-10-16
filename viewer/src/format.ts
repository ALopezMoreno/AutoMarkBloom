// Data contract between the Rust core and the viewer. See FORMAT.md.

export interface ShellsInfo {
  aspect: number;
  /** Bud outline terms (optional, 0 = ellipsoid); see shape.ts. */
  egg?: number;
  point?: number;
  core: number;
  radius: number[];
  /** Layer slot width (optional). */
  slot?: number;
  /** Legacy: replaced by the `sepal_radius` flower channel. */
  envelope?: number;
}

export interface PetalInfo {
  id: number;
  layer: number;
  theta: number;
}

/** Number for dims 1, [x, y, z] for dims 3 (flower and petal scope); flat nu*nv*dims array for point scope. */
export type ChannelValue = number | number[];

export interface ChannelDecl {
  name: string;
  scope: "point" | "petal" | "flower";
  dims: 1 | 3;
  range?: [number, number];
  /** Informational: growth, drift, fixed, target, state, ... The viewer never branches on it. */
  kind?: string;
  /** Category names for fixed categorical channels; value k means labels[k]. */
  labels?: string[];
}

export interface PetalFrame {
  /** nu*nv evaluated mid-surface points, flat xyz, row-major (u outer, v inner). Core frame, +z up. */
  mid: number[] | Float32Array;
  /** nu*nv full thickness per point. */
  thick: number[] | Float32Array;
  /** Petal- and point-scope channel values that changed at this frame. */
  channels?: Record<string, ChannelValue>;
}

export interface Frame {
  /** -1..0 bud development, 0 closed bud, (0, 1] opening. */
  t: number;
  stage?: "bud" | "open" | string;
  petals: PetalFrame[];
  /** Flower-scope channel values that changed at this frame. */
  channels?: Record<string, ChannelValue>;
  /** Legacy per-frame sepal radius (before the sepal_radius channel). */
  envelope?: number;
}

export interface FlowerDoc {
  format: "automark-flower";
  version: 1;
  /** Plant seed, u64 as a decimal string. */
  seed: string;
  flower_index: number | null;
  units: "core_radius";
  up: "+z";
  shells: ShellsInfo;
  grid: { nu: number; nv: number };
  petals: PetalInfo[];
  channels?: ChannelDecl[];
  frames: Frame[];
}

/** Throws with a readable message if `d` does not match the contract. Unknown fields are ignored. */
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
  const decls = new Map<string, ChannelDecl>();
  if (d.channels !== undefined) {
    if (!Array.isArray(d.channels)) fail('"channels" must be a list');
    for (const c of d.channels) {
      if (typeof c?.name !== "string") fail("every channel needs a name");
      if (decls.has(c.name)) fail(`channel "${c.name}" is declared twice`);
      if (!["point", "petal", "flower"].includes(c.scope)) fail(`channel "${c.name}": unknown scope ${c.scope}`);
      if (c.dims !== 1 && c.dims !== 3) fail(`channel "${c.name}": dims must be 1 or 3`);
      decls.set(c.name, c);
    }
  }
  const checkValues = (vals: any, scope: string, where: string) => {
    if (!vals) return;
    for (const [name, v] of Object.entries(vals)) {
      const c = decls.get(name);
      if (!c) continue; // undeclared: ignored
      if (c.scope !== scope) fail(`${where}: channel "${name}" is ${c.scope}-scope`);
      const len = Array.isArray(v) ? v.length : 1;
      const want = scope === "point" ? n * c.dims : c.dims;
      if (len !== want) fail(`${where}: channel "${name}" has ${len} numbers, expected ${want}`);
    }
  };
  d.frames.forEach((f: any, fi: number) => {
    if (typeof f.t !== "number") fail(`frame ${fi} has no t`);
    if (f.petals?.length !== d.petals.length) fail(`frame ${fi} has ${f.petals?.length} petals, expected ${d.petals.length}`);
    checkValues(f.channels, "flower", `frame ${fi}`);
    f.petals.forEach((p: any, pi: number) => {
      if (p.mid?.length !== 3 * n) fail(`frame ${fi} petal ${pi}: mid has ${p.mid?.length} numbers, expected ${3 * n}`);
      if (p.thick?.length !== n) fail(`frame ${fi} petal ${pi}: thick has ${p.thick?.length} numbers, expected ${n}`);
      if (p.channels) {
        const point: Record<string, any> = {}, petal: Record<string, any> = {};
        for (const [k, v] of Object.entries(p.channels)) (decls.get(k)?.scope === "point" ? point : petal)[k] = v;
        checkValues(petal, "petal", `frame ${fi} petal ${pi}`);
        checkValues(point, "point", `frame ${fi} petal ${pi}`);
      }
    });
  });
  return d as FlowerDoc;
}
