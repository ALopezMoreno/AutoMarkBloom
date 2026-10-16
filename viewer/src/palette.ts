// Layer colours, inner -> outer: a single rose ramp from light pink to deep
// plum, so layers read as depth rather than as unrelated categories.
import * as THREE from "three";

const INNER = new THREE.Color("#f4a7c0");
const OUTER = new THREE.Color("#6a1c42");

/** Colour for layer k of n (0 = innermost). */
export function layerColor(k: number, n: number, out = new THREE.Color()): THREE.Color {
  return out.copy(INNER).lerp(OUTER, n <= 1 ? 0 : k / (n - 1));
}

export function layerCss(k: number, n: number): string {
  return `#${layerColor(k, n).getHexString()}`;
}
