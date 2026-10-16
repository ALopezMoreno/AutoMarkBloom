//! Species = the ranges a plant's traits are drawn from.
//!
//! Nothing species-specific is hard-coded elsewhere: a plant seed draws one
//! value from each range here. `peony()` is the only built-in preset; other
//! species (dahlia, ...) are JSON files with the same fields.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Species {
    pub name: String,
    /// Number of petal layers (shells), inclusive range.
    pub layers: [u32; 2],
    /// Bud height / width.
    pub aspect: [f64; 2],
    /// Hard cap on petal thickness, in core radii.
    pub max_thickness: [f64; 2],
    /// Thickness at the petal edge as a fraction of the maximum.
    pub edge_thickness: [f64; 2],
    /// Extra room per layer for crumpling, in multiples of max thickness.
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
    /// How much bigger petals grow than their patch, which forces buckling.
    pub growth_factor: [f64; 2],
    /// Polar angle (rad from the bottom pole) where petals attach.
    pub phi_base: [f64; 2],
    /// How far short of the top pole petals stop (rad).
    pub phi_top_gap: [f64; 2],
}

impl Species {
    pub fn peony() -> Self {
        Species {
            name: "peony".into(),
            layers: [6, 10],
            aspect: [1.05, 1.3],
            max_thickness: [0.035, 0.06],
            edge_thickness: [0.3, 0.5],
            crumple_room: [0.6, 1.2],
            inhibit_sigma: [0.45, 0.75],
            inhibit_threshold: [0.35, 0.6],
            init_prob: [0.2, 0.45],
            stagger: [0.5, 1.0],
            up_prob: [0.6, 0.9],
            lateral_prob: [0.25, 0.55],
            inner_height: [0.5, 0.7],
            growth_factor: [1.12, 1.35],
            phi_base: [0.45, 0.45],
            phi_top_gap: [0.10, 0.10],
        }
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}
