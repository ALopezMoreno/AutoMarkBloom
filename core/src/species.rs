//! Species = the ranges a plant's traits are drawn from.
//!
//! Nothing species-specific is hard-coded elsewhere: a plant seed draws one
//! value from each range here. `peony()` is the only built-in preset; other
//! species (dahlia, ...) are JSON files with the same fields.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Species {
    pub name: String,

    // Layout: shells and the placement automaton.
    /// Number of petal layers (shells), inclusive range.
    pub layers: [u32; 2],
    /// Bud height / width.
    pub aspect: [f64; 2],
    /// Hard cap on petal thickness, in core radii.
    pub max_thickness: [f64; 2],
    /// Thickness at the petal edge as a fraction of the maximum.
    pub edge_thickness: [f64; 2],
    /// Initial room per layer beyond its thickness, in multiples of max thickness.
    pub crumple_room: [f64; 2],
    /// Reach of a primordium's inhibition, as arc length on its shell.
    pub inhibit_sigma: [f64; 2],
    pub inhibit_threshold: [f64; 2],
    pub init_prob: [f64; 2],
    /// Weight of the layer below's primordia, which staggers layers.
    pub stagger: [f64; 2],
    pub up_prob: [f64; 2],
    pub lateral_prob: [f64; 2],
    /// Fraction of the shell's height the innermost petals reach.
    pub inner_height: [f64; 2],
    /// Polar angle (rad from the bottom pole) where petals attach.
    pub phi_base: [f64; 2],
    /// How far short of the top pole petals stop (rad).
    pub phi_top_gap: [f64; 2],

    // Development: the growth automaton on each petal, and the sepals.
    /// Number of growth stages, inclusive range.
    pub growth_stages: [u32; 2],
    /// Linear growth per stage of an active cell.
    pub growth_rate: [f64; 2],
    /// Growth of a quiet cell, as a fraction of an active one.
    pub quiet_growth: [f64; 2],
    /// Extra growth at the petal margin (ruffling), as a multiple.
    pub edge_boost: [f64; 2],
    /// Base chance a cell is active in a stage.
    pub activity_base: [f64; 2],
    /// Added chance per fraction of active neighbours (patches spread).
    pub activity_neighbor: [f64; 2],
    /// Chance removed by contact pressure (crowded cells slow down).
    pub pressure_inhibition: [f64; 2],
    /// Bending stiffness of a petal at full thickness (scales with t^3).
    /// Stiffer petals fold in broader waves and push the sepals out more.
    pub petal_stiffness: [f64; 2],
    /// How far a petal may stray from its layer, in layer slots.
    pub layer_freedom: [f64; 2],
    /// Sepal resistance to swelling.
    pub sepal_stiffness: [f64; 2],
    /// Sepal rest-size growth over the whole bud stage, as a fraction.
    pub sepal_growth: [f64; 2],

    // Petal latents handed to the opening stage (plant-level means).
    /// Overall size potential of the plant's petals.
    pub size_potential: [f64; 2],
    /// Cupping target of the innermost and outermost layers (0 flat, 1 deep cup).
    pub cup_inner: [f64; 2],
    pub cup_outer: [f64; 2],
    /// Base fold (reflex) target of the innermost and outermost layers.
    pub fold_inner: [f64; 2],
    pub fold_outer: [f64; 2],
    /// Spread of opening onset from outer to inner layers (0 all at once).
    pub onset_spread: [f64; 2],
    /// Relative per-petal noise on latents around the plant targets.
    pub latent_noise: [f64; 2],

    // Rules applied per flower (fixed for the species, not drawn per plant).
    /// Range of a flower's vigour (main vs. side bud, conditions).
    pub vigour: [f64; 2],
    /// Chance a flower has one layer fewer or one more than the plant mean.
    pub layer_jitter: f64,
    /// Relative weights of petal types: flat, cupped, rolled.
    pub petal_type_weights: [f64; 3],
    /// Chance a petal twists clockwise (else anticlockwise).
    pub twist_clockwise: f64,

    // Opening (stage 2).
    /// How far petals open: angle of a fully open petal's base-to-tip chord
    /// from straight up, for the innermost and outermost layers (radians;
    /// pi/2 is horizontal, out to the sepals).
    /// Not written into the bud's JSON, so bud output is unchanged.
    #[serde(default = "default_open_angle", skip_serializing)]
    pub open_angle: [f64; 2],
}

fn default_open_angle() -> [f64; 2] {
    [1.0, 1.65]
}

impl Species {
    pub fn peony() -> Self {
        Species {
            name: "peony".into(),
            open_angle: default_open_angle(),
            layers: [6, 10],
            aspect: [1.0, 1.2],
            max_thickness: [0.035, 0.06],
            edge_thickness: [0.3, 0.5],
            crumple_room: [0.3, 0.7],
            inhibit_sigma: [0.45, 0.75],
            inhibit_threshold: [0.35, 0.6],
            init_prob: [0.2, 0.45],
            stagger: [0.5, 1.0],
            up_prob: [0.6, 0.9],
            lateral_prob: [0.25, 0.55],
            inner_height: [0.75, 0.9],
            phi_base: [0.45, 0.45],
            phi_top_gap: [0.10, 0.10],
            growth_stages: [10, 14],
            growth_rate: [0.05, 0.08],
            quiet_growth: [0.15, 0.4],
            edge_boost: [0.6, 1.4],
            activity_base: [0.15, 0.3],
            activity_neighbor: [0.4, 0.7],
            pressure_inhibition: [0.3, 0.8],
            petal_stiffness: [60.0, 120.0],
            layer_freedom: [1.5, 3.0],
            sepal_stiffness: [5.0, 25.0],
            sepal_growth: [0.04, 0.12],
            size_potential: [0.85, 1.15],
            cup_inner: [0.6, 0.9],
            cup_outer: [0.1, 0.35],
            fold_inner: [0.0, 0.15],
            fold_outer: [0.2, 0.5],
            onset_spread: [0.4, 0.8],
            latent_noise: [0.05, 0.15],
            vigour: [0.85, 1.1],
            layer_jitter: 0.4,
            petal_type_weights: [0.3, 0.6, 0.1],
            twist_clockwise: 0.5,
        }
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}
