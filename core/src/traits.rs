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
pub const STREAM_GROWTH: u64 = 3;
pub const STREAM_FLOWER: u64 = 4;
pub const STREAM_CHANNELS: u64 = 5;
const STREAM_FLOWER_BASE: u64 = 1_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlantSeed(pub u64);

impl PlantSeed {
    pub fn rng(&self, stream: u64) -> Pcg32 {
        Pcg32::new(derive(self.0, stream), stream)
    }

    /// Seed of one flower on this plant. Traits stay per plant; everything
    /// a flower grows (its bud automaton, growth, packing, latents) draws
    /// from sub-streams of this.
    pub fn flower_seed(&self, index: u64) -> u64 {
        derive(self.0, STREAM_FLOWER_BASE.wrapping_add(index))
    }

    /// A named sub-stream of one flower (`STREAM_*` constants).
    pub fn flower_rng(&self, index: u64, stream: u64) -> Pcg32 {
        Pcg32::new(derive(self.flower_seed(index), stream), stream)
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
    pub phi_base: f64,
    pub phi_top_gap: f64,
    pub growth_stages: usize,
    pub growth_rate: f64,
    pub quiet_growth: f64,
    pub edge_boost: f64,
    pub activity_base: f64,
    pub activity_neighbor: f64,
    pub pressure_inhibition: f64,
    pub petal_stiffness: f64,
    pub layer_freedom: f64,
    pub sepal_stiffness: f64,
    pub sepal_growth: f64,
    pub growth_correlation: f64,
    pub growth_persistence: f64,
    pub growth_anisotropy: f64,
    pub zigzag_bending: f64,
    pub bud_egg: f64,
    pub bud_point: f64,
    pub core_size: f64,
    pub size_potential: f64,
    pub cup_inner: f64,
    pub cup_outer: f64,
    pub fold_inner: f64,
    pub fold_outer: f64,
    pub onset_spread: f64,
    pub latent_noise: f64,
}

/// What one flower adds on top of its plant's traits.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FlowerTraits {
    pub index: u64,
    /// Size and growth factor of this flower.
    pub vigour: f64,
    /// Layer count after this flower's jitter.
    pub layers: usize,
}

impl FlowerTraits {
    pub fn draw(index: u64, s: &Species, plant: &PlantTraits, r: &mut Pcg32) -> Self {
        let vigour = r.range(s.vigour[0], s.vigour[1]);
        let mut layers = plant.layers as i64;
        if r.f64() < s.layer_jitter {
            layers += if r.f64() < 0.5 { -1 } else { 1 };
        }
        FlowerTraits { index, vigour, layers: layers.max(1) as usize }
    }

    /// The plant's traits as this flower grows them.
    pub fn apply(&self, plant: &PlantTraits) -> PlantTraits {
        let mut t = plant.clone();
        t.layers = self.layers;
        t.growth_rate *= self.vigour;
        t.size_potential *= self.vigour;
        t
    }
}

fn int_in(r: &mut Pcg32, x: [u32; 2]) -> usize {
    let v = math::floor(r.range(x[0] as f64, x[1] as f64 + 1.0)).min(x[1] as f64);
    (v as usize).max(1)
}

impl PlantTraits {
    pub fn draw(s: &Species, r: &mut Pcg32) -> Self {
        let layers = int_in(r, s.layers);
        let mut f = |x: [f64; 2]| r.range(x[0], x[1]);
        let aspect = f(s.aspect);
        let max_thickness = f(s.max_thickness);
        let edge_thickness = f(s.edge_thickness);
        let crumple_room = f(s.crumple_room);
        let inhibit_sigma = f(s.inhibit_sigma);
        let inhibit_threshold = f(s.inhibit_threshold);
        let init_prob = f(s.init_prob);
        let stagger = f(s.stagger);
        let up_prob = f(s.up_prob);
        let lateral_prob = f(s.lateral_prob);
        let inner_height = f(s.inner_height);
        let phi_base = f(s.phi_base);
        let phi_top_gap = f(s.phi_top_gap);
        let growth_rate = f(s.growth_rate);
        let quiet_growth = f(s.quiet_growth);
        let edge_boost = f(s.edge_boost);
        let activity_base = f(s.activity_base);
        let activity_neighbor = f(s.activity_neighbor);
        let pressure_inhibition = f(s.pressure_inhibition);
        let layer_freedom = f(s.layer_freedom);
        let petal_stiffness = f(s.petal_stiffness);
        let sepal_stiffness = f(s.sepal_stiffness);
        let sepal_growth = f(s.sepal_growth);
        let size_potential = f(s.size_potential);
        let cup_inner = f(s.cup_inner);
        let cup_outer = f(s.cup_outer);
        let fold_inner = f(s.fold_inner);
        let fold_outer = f(s.fold_outer);
        let onset_spread = f(s.onset_spread);
        let latent_noise = f(s.latent_noise);
        let growth_stages = int_in(r, s.growth_stages);
        // Drawn last so adding them left every earlier trait unchanged.
        let growth_correlation = r.range(s.growth_correlation[0], s.growth_correlation[1]);
        let growth_persistence = r.range(s.growth_persistence[0], s.growth_persistence[1]);
        let growth_anisotropy = r.range(s.growth_anisotropy[0], s.growth_anisotropy[1]);
        let zigzag_bending = r.range(s.zigzag_bending[0], s.zigzag_bending[1]);
        let bud_egg = r.range(s.bud_egg[0], s.bud_egg[1]);
        let bud_point = r.range(s.bud_point[0], s.bud_point[1]);
        let core_size = r.range(s.core_size[0], s.core_size[1]);
        PlantTraits {
            layers,
            aspect,
            max_thickness,
            edge_thickness,
            crumple_room,
            inhibit_sigma,
            inhibit_threshold,
            init_prob,
            stagger,
            up_prob,
            lateral_prob,
            inner_height,
            phi_base,
            phi_top_gap,
            growth_stages,
            growth_rate,
            quiet_growth,
            edge_boost,
            activity_base,
            activity_neighbor,
            pressure_inhibition,
            petal_stiffness,
            layer_freedom,
            sepal_stiffness,
            sepal_growth,
            growth_correlation,
            growth_persistence,
            growth_anisotropy,
            zigzag_bending,
            bud_egg,
            bud_point,
            core_size,
            size_potential,
            cup_inner,
            cup_outer,
            fold_inner,
            fold_outer,
            onset_spread,
            latent_noise,
        }
    }
}
