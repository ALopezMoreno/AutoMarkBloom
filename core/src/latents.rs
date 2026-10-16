//! Petal latents the bud hands to the opening stage, as channels.
//!
//! Targets come from the plant's means, shaped by where the petal sits in
//! the bud (layer, how far its patch reached, how wide it was) and by the
//! flower's vigour, with per-petal noise from each channel's own stream.

use crate::channels::{channel_rng, Channels, Kind, Scope};
use crate::ca::{BudLayout, NA, NP};
use crate::growth::Growth;
use crate::sheet::Sheet;
use crate::species::Species;
use crate::traits::{FlowerTraits, PlantTraits};
use std::f64::consts::TAU;

fn noisy(r: &mut crate::rng::Pcg32, value: f64, rel: f64) -> f64 {
    value * (1.0 + rel * r.normal())
}

pub fn bud_channels(
    seed: u64,
    species: &Species,
    plant: &PlantTraits,
    flower: &FlowerTraits,
    layout: &BudLayout,
    sheets: &[Sheet],
    growth: &[Growth],
) -> Channels {
    let mut ch = Channels::default();
    let n = sheets.len();
    let layers = flower.layers.max(2) as f64 - 1.0;
    let depth: Vec<f64> = sheets.iter().map(|s| s.layer as f64 / layers).collect(); // 0 inner, 1 outer
    let noise = plant.latent_noise;

    let per_petal = |name: &str, f: &mut dyn FnMut(usize, &mut crate::rng::Pcg32) -> f64| -> Vec<f64> {
        let mut r = channel_rng(seed, name);
        (0..n).map(|i| f(i, &mut r)).collect()
    };

    let target_length = per_petal("target_length", &mut |i, r| {
        let p = &layout.petals[sheets[i].petal];
        let reach = (p.rows.iter().rposition(|x| x.is_some()).unwrap_or(0) + 1) as f64 / NP as f64;
        noisy(r, plant.size_potential * (0.5 + reach), noise).max(0.05)
    });
    ch.declare("target_length", Scope::Petal, 1, None, Kind::Target, target_length);

    let target_width = per_petal("target_width", &mut |i, r| {
        let p = &layout.petals[sheets[i].petal];
        let widest = p.rows.iter().flatten().map(|x| x.1).fold(0.0, f64::max) / (TAU / NA as f64);
        noisy(r, plant.size_potential * (0.3 + widest / 24.0), noise).max(0.05)
    });
    ch.declare("target_width", Scope::Petal, 1, None, Kind::Target, target_width);

    let edge_growth = per_petal("edge_growth", &mut |_, r| noisy(r, plant.edge_boost, noise).max(0.0));
    ch.declare("edge_growth", Scope::Petal, 1, None, Kind::Target, edge_growth);

    let cup = per_petal("cup", &mut |i, r| {
        let base = plant.cup_inner + (plant.cup_outer - plant.cup_inner) * depth[i];
        noisy(r, base, noise).clamp(0.0, 1.0)
    });
    ch.declare("cup", Scope::Petal, 1, Some([0.0, 1.0]), Kind::Target, cup);

    let base_fold = per_petal("base_fold", &mut |i, r| {
        let base = plant.fold_inner + (plant.fold_outer - plant.fold_inner) * depth[i];
        noisy(r, base, noise).clamp(0.0, 1.0)
    });
    ch.declare("base_fold", Scope::Petal, 1, Some([0.0, 1.0]), Kind::Target, base_fold);

    let onset = per_petal("onset", &mut |i, r| (plant.onset_spread * (1.0 - depth[i]) + 0.05 * r.normal()).clamp(0.0, 1.0));
    ch.declare("onset", Scope::Petal, 1, Some([0.0, 1.0]), Kind::Fixed, onset);

    let bud_growth = growth.iter().map(|g| g.scale.iter().sum::<f64>() / g.scale.len() as f64).collect();
    ch.declare("bud_growth", Scope::Petal, 1, None, Kind::State, bud_growth);

    let w = species.petal_type_weights;
    let total = (w[0] + w[1] + w[2]).max(1e-12);
    let petal_type = per_petal("petal_type", &mut |_, r| {
        let u = r.f64() * total;
        if u < w[0] {
            0.0
        } else if u < w[0] + w[1] {
            1.0
        } else {
            2.0
        }
    });
    ch.declare("petal_type", Scope::Petal, 1, Some([0.0, 2.0]), Kind::Fixed, petal_type);
    ch.label("petal_type", &["flat", "cupped", "rolled"]);

    let twist = per_petal("twist", &mut |_, r| if r.f64() < species.twist_clockwise { 1.0 } else { -1.0 });
    ch.declare("twist", Scope::Petal, 1, Some([-1.0, 1.0]), Kind::Fixed, twist);

    ch.declare("vigour", Scope::Flower, 1, Some(species.vigour), Kind::Fixed, vec![flower.vigour]);
    ch
}
