// Layer colours, inner -> outer, on the viridis ramp (ordered and colour-blind safe).
import * as THREE from "three";

const VIRIDIS = ["#440154", "#482878", "#3e4989", "#31688e", "#26828e", "#1f9e89", "#35b779", "#6ece58", "#b5de2b", "#fde725"].map(
  (h) => new THREE.Color(h),
);

/** Colour for layer k of n (0 = innermost). */
export function layerColor(k: number, n: number, out = new THREE.Color()): THREE.Color {
  const x = n <= 1 ? 0 : (k / (n - 1)) * (VIRIDIS.length - 1);
  const i = Math.min(VIRIDIS.length - 2, Math.floor(x));
  return out.copy(VIRIDIS[i]).lerp(VIRIDIS[i + 1], x - i);
}

export function layerCss(k: number, n: number): string {
  return `#${layerColor(k, n).getHexString()}`;
}
