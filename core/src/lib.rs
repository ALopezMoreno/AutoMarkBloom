//! Automark Bloom core: grows flower buds from a plant seed.
//!
//! Stage 1 (bud formation), in order:
//! 1. `shells`: nested egg-shaped shells, one per petal layer, sized by how
//!    many layers sit inside them and how thick petals are.
//! 2. `ca`: a stochastic cellular automaton on each shell's surface decides
//!    petal count, placement and outline.
//! 3. `sheet`: each petal becomes a thick spline sheet laid on its shell patch.
//! 4. `pack` + `growth`: the bud develops over growth stages. A second
//!    stochastic automaton on each petal decides where it grows, feeding back
//!    on contact pressure; after each stage the sheets and the sepal envelope
//!    are relaxed by gradient descent on a smooth energy (`energy`), so petals
//!    ruffle, crumple, fold across layers and swell the bud.
//!
//! Everything in stage 1 is seeded per plant, so all buds on a plant match.
//! Species-specific numbers live only in `species::Species`.
//! Output is the shared flower JSON (`export::flower_json`).

pub mod ca;
pub mod energy;
pub mod export;
pub mod growth;
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
    /// Per growth stage geometry, when `PackSettings::record_frames` is set.
    pub frames: Vec<pack::StageFrame>,
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
    let (report, frames) = pack::develop(
        &mut sheets,
        &shells,
        &traits,
        settings,
        &mut seed.rng(traits::STREAM_PACK),
        &mut seed.rng(traits::STREAM_GROWTH),
    );
    Bud { seed, species: species.clone(), traits, shells, layout, sheets, report, frames }
}

/// Canonical output for a seed: peony species, default settings, final
/// bud only.
pub fn grow_bud_json(seed: u64) -> String {
    export::flower_json(&grow_bud(PlantSeed(seed), &Species::peony(), &PackSettings::default()))
}
