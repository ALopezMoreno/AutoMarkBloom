//! Nested egg-shaped shells, one slot per petal layer.
//!
//! Distances use a "scaled radius" r~ = sqrt(x^2 + y^2 + (z/aspect)^2), so
//! every shell is the set r~ = const. Each layer owns a slot of width
//! max_thickness * (1 + crumple_room), stacked outward from the core, so the
//! bud's size follows from how many layers it packs and how thick they are.

use crate::traits::PlantTraits;
use crate::math;
use serde::Serialize;
use std::f64::consts::PI;

pub const CORE_RADIUS: f64 = 1.0;

#[derive(Clone, Debug, Serialize)]
pub struct Shells {
    pub aspect: f64,
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
        let radius = (0..t.layers).map(|k| CORE_RADIUS + slot * (k as f64 + 0.5)).collect();
        Shells {
            aspect: t.aspect,
            slot,
            radius,
            core: CORE_RADIUS,
            envelope: CORE_RADIUS + slot * t.layers as f64,
            phi_base: t.phi_base,
            phi_top_gap: t.phi_top_gap,
        }
    }

    pub fn scaled_radius(&self, p: [f64; 3]) -> f64 {
        let z = p[2] / self.aspect;
        math::sqrt(p[0] * p[0] + p[1] * p[1] + z * z)
    }

    pub fn grad_scaled_radius(&self, p: [f64; 3]) -> [f64; 3] {
        let r = self.scaled_radius(p).max(1e-12);
        [p[0] / r, p[1] / r, p[2] / (self.aspect * self.aspect * r)]
    }

    /// Point on the shell of scaled radius `a`; phi = 0 is the bottom pole.
    pub fn point(&self, a: f64, theta: f64, phi: f64) -> [f64; 3] {
        let (sp, cp) = (math::sin(phi), math::cos(phi));
        [a * sp * math::cos(theta), a * sp * math::sin(theta), -self.aspect * a * cp]
    }

    /// Polar angle for a fractional row position in [0, 1].
    pub fn phi_at(&self, frac: f64) -> f64 {
        self.phi_base + frac * (PI - self.phi_top_gap - self.phi_base)
    }
}
