//! The opening program: which channels exist and how each evolves.
//!
//! This table is data. The stepper (`channels`) runs any table, and the
//! geometry consumer (`consume`) reads only the channel names listed in its
//! own docs. A species can swap this table for its own (it serializes as
//! JSON), and new channels such as colour gradients are added here or
//! handed over by the bud without touching either.

use super::channels::{ChannelSpec, Param, Rule, Scope};

/// How long one petal takes to open, as a fraction of the opening.
pub const OPEN_DURATION: f64 = 0.55;

/// Growth toward gain * target + bias, on the petal's own clock and rate.
fn growth(target: &str, gain: f64, bias: f64, monotone: bool) -> Rule {
    Rule::Growth {
        target: Param::Channel(target.into()),
        gain: gain.into(),
        bias: bias.into(),
        rate: "rate".into(),
        onset: "onset".into(),
        duration: OPEN_DURATION.into(),
        monotone,
    }
}

/// Default opening program (peony-like, single-opening flowers).
pub fn default_program() -> Vec<ChannelSpec> {
    let petal = |n: &str, rule: Rule, d: f64| ChannelSpec::new(n, Scope::Petal, 1, rule, vec![d]);
    vec![
        // Bud latents (see `latents`). The bud supplies these; the defaults
        // only apply if it does not. Lengths and widths are ratios to the
        // bud petal; cup and base_fold run 0..1; twist is a direction.
        ChannelSpec::new("vigour", Scope::Flower, 1, Rule::Fixed, vec![1.0]).exported(),
        petal("onset", Rule::Fixed, 0.0).exported(),
        petal("target_length", Rule::Fixed, 1.5),
        petal("target_width", Rule::Fixed, 1.3),
        petal("edge_growth", Rule::Fixed, 1.0),
        petal("cup", Rule::Fixed, 0.5),
        petal("base_fold", Rule::Fixed, 0.3),
        petal("twist", Rule::Fixed, 0.0),
        // Per-petal growth rate: slow mean-reverting noise around the
        // flower's vigour, so fast petals stay fast for a while.
        petal("rate", Rule::Drift { mean: "vigour".into(), revert: 3.0.into(), sigma: 0.35.into() }, 1.0).exported(),
        // Growth state, consumed by the geometry. Gains turn the bud's
        // 0..1 latents into the consumer's units (fractions and radians);
        // widths gain more than lengths so open petals stay broad and overlap.
        petal("length", growth("target_length", 1.0, 0.0, true), 1.0).exported(),
        petal("width", growth("target_width", 1.6, 0.0, true), 1.0).exported(),
        petal("edge", growth("edge_growth", 0.25, 0.0, true), 0.0),
        petal("cupping", growth("cup", 1.0, 0.0, false), 0.0),
        petal("reflex", growth("base_fold", 1.2, 0.0, false), 0.0),
        petal("asymmetry", growth("twist", 0.08, 0.0, false), 0.0),
        // How much of the bud's curl a petal still holds (1 = closed bud),
        // along the petal and across it: cupped inner petals keep most of
        // it, so the centre stays a bowl, and outer petals lose most of it.
        petal("memory", growth("cup", 0.6, 0.35, false), 1.0),
        petal("wrap", growth("cup", 0.5, 0.45, false), 1.0),
        // Crumples from the bud partly stretch out as the petal expands.
        petal("ruffle_target", Rule::Fixed, 0.6),
        petal("ruffle", growth("ruffle_target", 1.0, 0.0, false), 1.0),
    ]
}
