//! Plant seed and the named traits drawn from it.
//!
//! A plant is one u64. Its traits are an explicit list of numbers, drawn from
//! a species' ranges, so later inheritance can mutate and cross them directly
//! instead of the raw seed.

use crate::math;
use crate::rng::{derive, Pcg32};
use crate::species::Species;
use serde::Serialize;

pub const STREAM_TRAITS: u64 = 0;
pub const STREAM_CA: u64 = 1;
pub const STREAM_PACK: u64 = 2;
const STREAM_FLOWER_BASE: u64 = 1_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlantSeed(pub u64);

impl PlantSeed {
    pub fn rng(&self, stream: u64) -> Pcg32 {
        Pcg32::new(derive(self.0, stream), stream)
    }

    /// Per-flower stream, for the flowering stage.
    pub fn flower_rng(&self, index: u64) -> Pcg32 {
        self.rng(STREAM_FLOWER_BASE + index)
    }

    pub fn traits(&self, species: &Species) -> PlantTraits {
        PlantTraits::draw(species, &mut self.rng(STREAM_TRAITS))
    }
}

/// One plant's values; field meanings are documented on `Species`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PlantTraits {
    pub layers: usize,
    pub aspect: f64,
    pub max_thickness: f64,
    pub edge_thickness: f64,
    pub crumple_room: f64,
    pub inhibit_sigma: f64,
    pub inhibit_threshold: f64,
    pub init_prob: f64,
    pub stagger: f64,
    pub up_prob: f64,
    pub lateral_prob: f64,
    pub inner_height: f64,
    pub growth_factor: f64,
    pub phi_base: f64,
    pub phi_top_gap: f64,
}

impl PlantTraits {
    pub fn draw(s: &Species, r: &mut Pcg32) -> Self {
        let mut f = |x: [f64; 2]| r.range(x[0], x[1]);
        let layers = math::floor(f([s.layers[0] as f64, s.layers[1] as f64 + 1.0])).min(s.layers[1] as f64) as usize;
        PlantTraits {
            layers: layers.max(1),
            aspect: f(s.aspect),
            max_thickness: f(s.max_thickness),
            edge_thickness: f(s.edge_thickness),
            crumple_room: f(s.crumple_room),
            inhibit_sigma: f(s.inhibit_sigma),
            inhibit_threshold: f(s.inhibit_threshold),
            init_prob: f(s.init_prob),
            stagger: f(s.stagger),
            up_prob: f(s.up_prob),
            lateral_prob: f(s.lateral_prob),
            inner_height: f(s.inner_height),
            growth_factor: f(s.growth_factor),
            phi_base: f(s.phi_base),
            phi_top_gap: f(s.phi_top_gap),
        }
    }
}
