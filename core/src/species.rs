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
}

impl Species {
    pub fn peony() -> Self {
        Species {
            name: "peony".into(),
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
        }
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}
