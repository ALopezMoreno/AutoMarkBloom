//! Automark Bloom core: grows flower buds from a plant seed.
//!
//! Stage 1 (bud formation), in order:
//! 1. `shells`: nested egg-shaped shells, one per petal layer, sized by how
//!    many layers sit inside them and how thick petals are.
//! 2. `ca`: a stochastic cellular automaton on each shell's surface decides
//!    petal count, placement and outline.
//! 3. `sheet`: each petal becomes a thick spline sheet laid on its shell patch.
//! 4. `pack`: the sheets are grown past the room they have and relaxed by
//!    gradient descent on a smooth energy (`energy`), so they buckle and pack.
//!
//! Everything in stage 1 is seeded per plant, so all buds on a plant match.
//! Species-specific numbers live only in `species::Species`.
//! Output is the shared flower JSON (`export::flower_json`).

pub mod ca;
pub mod energy;
pub mod export;
pub mod math;
pub mod pack;
pub mod rng;
pub mod sheet;
pub mod shells;
pub mod species;
pub mod traits;

pub use pack::{PackReport, PackSettings};
pub use species::Species;
pub use traits::{PlantSeed, PlantTraits};

/// A finished, packed bud.
pub struct Bud {
    pub seed: PlantSeed,
    pub species: Species,
    pub traits: PlantTraits,
    pub shells: shells::Shells,
    pub layout: ca::BudLayout,
    pub sheets: Vec<sheet::Sheet>,
    pub report: PackReport,
}

pub fn grow_bud(seed: PlantSeed, species: &Species, settings: &PackSettings) -> Bud {
    let traits = seed.traits(species);
    let shells = shells::Shells::new(&traits);
    let layout = ca::run(&traits, &shells, &mut seed.rng(traits::STREAM_CA));
    let mut sheets: Vec<sheet::Sheet> = layout
        .petals
        .iter()
        .filter_map(|p| sheet::Sheet::from_patch(p, &shells, &traits))
        .collect();
    let report = pack::pack(&mut sheets, &shells, &traits, settings, &mut seed.rng(traits::STREAM_PACK));
    Bud { seed, species: species.clone(), traits, shells, layout, sheets, report }
}

/// Canonical output for a seed: peony species, default settings.
pub fn grow_bud_json(seed: u64) -> String {
    export::flower_json(&grow_bud(PlantSeed(seed), &Species::peony(), &PackSettings::default()))
}
