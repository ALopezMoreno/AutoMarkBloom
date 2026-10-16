//! Nested egg-shaped shells, one slot per petal layer.
//!
//! Distances use a "scaled radius" r~ = |q| / g(c), with q = (x, y, z/aspect)
//! and c = q_z / |q| (+1 at the top pole, -1 at the bottom), so every shell
//! is the set r~ = const and all shells are scaled copies of one shape.
//! The shape factor g(c) = 1 + egg * P3(c) + point * max(c, 0)^8 is 1 for a
//! plain ellipsoid; `egg` (the Legendre P3 term) broadens the lower body and
//! narrows the top, and `point` draws the top out into a pointed tip.
//! Each layer owns a slot of width max_thickness * (1 + crumple_room),
//! stacked outward from the core, so the bud's size follows from how many
//! layers it packs and how thick they are.

use crate::traits::PlantTraits;
use crate::math;
use serde::Serialize;
use std::f64::consts::PI;

/// Nominal core radius, the unit of length; a plant's core is
/// `core_size` times this.
pub const CORE_RADIUS: f64 = 1.0;

#[derive(Clone, Debug, Serialize)]
pub struct Shells {
    pub aspect: f64,
    /// Shape factor terms, see the module doc. Both 0 give an ellipsoid.
    pub egg: f64,
    pub point: f64,
    pub slot: f64,
    /// Scaled radius of each layer's mid-surface, inner to outer.
    pub radius: Vec<f64>,
    pub core: f64,
    /// Inner side of the sepal envelope.
    pub envelope: f64,
    #[serde(skip)]
    pub phi_base: f64,
    #[serde(skip)]
    pub phi_top_gap: f64,
}

impl Shells {
    pub fn new(t: &PlantTraits) -> Self {
        let slot = t.max_thickness * (1.0 + t.crumple_room);
        let core = CORE_RADIUS * t.core_size;
        let radius = (0..t.layers).map(|k| core + slot * (k as f64 + 0.5)).collect();
        Shells {
            aspect: t.aspect,
            egg: t.bud_egg,
            point: t.bud_point,
            slot,
            radius,
            core,
            envelope: core + slot * t.layers as f64,
            phi_base: t.phi_base,
            phi_top_gap: t.phi_top_gap,
        }
    }

    /// Shape factor g(c) and its derivative dg/dc.
    pub fn shape(&self, c: f64) -> (f64, f64) {
        let p3 = 0.5 * (5.0 * c * c * c - 3.0 * c);
        let dp3 = 0.5 * (15.0 * c * c - 3.0);
        // c^8 by plain products, so every platform rounds it the same way.
        let (tip, dtip) = if c > 0.0 {
            let c2 = c * c;
            let c4 = c2 * c2;
            (c4 * c4, 8.0 * c4 * c2 * c)
        } else {
            (0.0, 0.0)
        };
        (1.0 + self.egg * p3 + self.point * tip, self.egg * dp3 + self.point * dtip)
    }

    pub fn scaled_radius(&self, p: [f64; 3]) -> f64 {
        let z = p[2] / self.aspect;
        let n = math::sqrt(p[0] * p[0] + p[1] * p[1] + z * z);
        if n < 1e-12 {
            return n;
        }
        n / self.shape(z / n).0
    }

    pub fn grad_scaled_radius(&self, p: [f64; 3]) -> [f64; 3] {
        let z = p[2] / self.aspect;
        let n = math::sqrt(p[0] * p[0] + p[1] * p[1] + z * z).max(1e-12);
        let u = [p[0] / n, p[1] / n, z / n];
        let (g, dg) = self.shape(u[2]);
        // d(n/g)/dq = u/g - (dg/g^2) (e_z - c u), with dq_z/dz = 1/aspect.
        let k = dg / (g * g);
        [
            u[0] / g + k * u[2] * u[0],
            u[1] / g + k * u[2] * u[1],
            (u[2] / g - k * (1.0 - u[2] * u[2])) / self.aspect,
        ]
    }

    /// Point on the shell of scaled radius `a`; phi = 0 is the bottom pole.
    pub fn point(&self, a: f64, theta: f64, phi: f64) -> [f64; 3] {
        let (sp, cp) = (math::sin(phi), math::cos(phi));
        let r = a * self.shape(-cp).0;
        [r * sp * math::cos(theta), r * sp * math::sin(theta), -self.aspect * r * cp]
    }

    /// Polar angle for a fractional row position in [0, 1].
    pub fn phi_at(&self, frac: f64) -> f64 {
        self.phi_base + frac * (PI - self.phi_top_gap - self.phi_base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn egg_shells() -> Shells {
        Shells { aspect: 1.15, egg: 0.12, point: 0.25, slot: 0.1, radius: vec![1.05], core: 1.0, envelope: 1.1, phi_base: 0.45, phi_top_gap: 0.1 }
    }

    #[test]
    fn points_lie_on_their_shell() {
        let s = egg_shells();
        for i in 0..40 {
            let phi = 0.05 + 3.0 * i as f64 / 40.0;
            let p = s.point(1.3, 0.7 * i as f64, phi);
            assert!((s.scaled_radius(p) - 1.3).abs() < 1e-12, "phi {phi}");
        }
    }

    #[test]
    fn gradient_matches_finite_differences() {
        let s = egg_shells();
        let h = 1e-6;
        for p in [[0.3, -0.2, 0.9], [0.8, 0.1, -0.4], [0.05, 0.02, 1.2], [-0.6, 0.5, 0.2]] {
            let g = s.grad_scaled_radius(p);
            for k in 0..3 {
                let (mut a, mut b) = (p, p);
                a[k] += h;
                b[k] -= h;
                let fd = (s.scaled_radius(a) - s.scaled_radius(b)) / (2.0 * h);
                assert!((g[k] - fd).abs() < 1e-7, "{p:?} axis {k}: {} vs {fd}", g[k]);
            }
        }
    }
}
