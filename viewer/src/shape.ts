// Bud outline shared by the shells, core, sepals and section view. Mirrors
// core/src/shells.rs: a shell of scaled radius a is the surface
// |q| = a * g(c), q = (x, y, z / aspect), c = q_z / |q|, with
// g(c) = 1 + egg * P3(c) + point * max(c, 0)^8. Older files have no egg or
// point, which gives the plain ellipsoid.
import type { ShellsInfo } from "./format";

export function shapeFactor(sh: ShellsInfo, c: number): number {
  const p3 = 0.5 * (5 * c * c * c - 3 * c);
  const tip = c > 0 ? c ** 8 : 0;
  return 1 + (sh.egg ?? 0) * p3 + (sh.point ?? 0) * tip;
}

/** Scaled radius of a core-frame point (z up). */
export function scaledRadius(sh: ShellsInfo, x: number, y: number, z: number): number {
  const zq = z / sh.aspect;
  const n = Math.sqrt(x * x + y * y + zq * zq);
  return n < 1e-12 ? n : n / shapeFactor(sh, zq / n);
}

/** (rho, z) of the shell of scaled radius a at polar angle phi (0 = bottom pole). */
export function shellProfile(sh: ShellsInfo, a: number, phi: number): [number, number] {
  const c = -Math.cos(phi);
  const r = a * shapeFactor(sh, c);
  return [r * Math.sin(phi), sh.aspect * r * c];
}
