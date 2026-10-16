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
//! 5. `latents`: per-petal and per-flower quantities for the opening stage,
//!    as named `channels`.
//!
//! Variation has three tiers: species (rules and ranges, `species`), plant
//! (heritable trait values, seeded by the plant seed) and flower (vigour,
//! layer jitter, and its own run of everything above, seeded per flower).
//! Output is the shared flower JSON (`export::flower_json`).

pub mod ca;
pub mod channels;
pub mod energy;
pub mod export;
pub mod growth;
pub mod latents;
pub mod math;
pub mod pack;
pub mod rng;
pub mod sheet;
pub mod shells;
pub mod species;
pub mod traits;

pub use pack::{PackReport, PackSettings};
pub use species::Species;
pub use traits::{FlowerTraits, PlantSeed, PlantTraits};

/// A finished, packed bud of one flower.
pub struct Bud {
    pub seed: PlantSeed,
    pub species: Species,
    /// The plant's heritable traits.
    pub plant: PlantTraits,
    /// What this flower adds (index, vigour, layer jitter).
    pub flower: FlowerTraits,
    /// The traits this flower actually grew with (plant + flower).
    pub traits: PlantTraits,
    pub shells: shells::Shells,
    pub layout: ca::BudLayout,
    pub sheets: Vec<sheet::Sheet>,
    pub report: PackReport,
    /// Per growth stage geometry, when `PackSettings::record_frames` is set.
    pub frames: Vec<pack::StageFrame>,
    /// Latent quantities handed to the opening stage.
    pub channels: channels::Channels,
}

/// Grow flower `flower_index` of plant `seed`. Traits are drawn per plant;
/// the bud automaton, growth, packing and latents are seeded per flower, so
/// siblings share a cultivar but are not clones.
pub fn grow_bud(seed: PlantSeed, flower_index: u64, species: &Species, settings: &PackSettings) -> Bud {
    use traits::{STREAM_CA, STREAM_CHANNELS, STREAM_FLOWER, STREAM_GROWTH, STREAM_PACK};
    let plant = seed.traits(species);
    let flower = FlowerTraits::draw(flower_index, species, &plant, &mut seed.flower_rng(flower_index, STREAM_FLOWER));
    let traits = flower.apply(&plant);
    let shells = shells::Shells::new(&traits);
    let layout = ca::run(&traits, &shells, &mut seed.flower_rng(flower_index, STREAM_CA));
    let mut sheets: Vec<sheet::Sheet> = layout
        .petals
        .iter()
        .filter_map(|p| sheet::Sheet::from_patch(p, &shells, &traits))
        .collect();
    let (report, frames, growth) = pack::develop(
        &mut sheets,
        &shells,
        &traits,
        settings,
        &mut seed.flower_rng(flower_index, STREAM_PACK),
        &mut seed.flower_rng(flower_index, STREAM_GROWTH),
    );
    let channel_seed = rng::derive(seed.flower_seed(flower_index), STREAM_CHANNELS);
    let mut channels = latents::bud_channels(channel_seed, species, &traits, &flower, &layout, &sheets, &growth);
    channels.declare(
        "sepal_radius",
        channels::Scope::Flower,
        1,
        None,
        channels::Kind::State,
        vec![report.envelope_end],
    );
    Bud { seed, species: species.clone(), plant, flower, traits, shells, layout, sheets, report, frames, channels }
}

/// Canonical output for a flower: peony species, default settings, final
/// bud only.
pub fn grow_bud_json(seed: u64, flower_index: u64) -> String {
    export::flower_json(&grow_bud(PlantSeed(seed), flower_index, &Species::peony(), &PackSettings::default()))
}
