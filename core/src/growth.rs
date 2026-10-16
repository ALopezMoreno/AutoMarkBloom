//! The growth automaton: a stochastic CA on each petal's own node grid.
//!
//! Each stage, every node is either active or quiet. A node's chance of being
//! active rises with the fraction of its active neighbours (so growth comes
//! in patches that spread and fade) and falls with the contact pressure it
//! felt in the last relaxation (crowded tissue slows down). Active nodes grow
//! by `growth_rate`, quiet ones by a fraction of it, and margins get an extra
//! `edge_boost`. The stage's growth increments are then smoothed over
//! neighbours (growth signals diffuse in tissue), so folds come out as
//! smooth bulges rather than node-sized kinks. Uneven growth is what makes
//! the sheet ruffle and crumple when it is relaxed; the pressure feedback
//! ties growth to the packing.
//!
//! The coin flip behind each node's state is not independent noise:
//! - **Correlated:** it reads a Gaussian noise field smoothed over
//!   `growth_correlation` nodes, so neighbours tend to switch together.
//! - **Persistent:** the field drifts from stage to stage (an AR(1) process
//!   with coefficient `growth_persistence`), so a patch that is growing tends
//!   to keep growing.
//! - **Directional:** growth is split between length and width by
//!   `growth_anisotropy`, so springs along the petal and across it lengthen
//!   at different rates.

use crate::math;
use crate::rng::Pcg32;
use crate::sheet::{idx, NU, NV};
use crate::traits::PlantTraits;

/// Smoothing passes applied to each stage's growth increments.
const DIFFUSION_PASSES: usize = 3;

#[derive(Clone, Debug)]
pub struct Growth {
    /// Accumulated linear growth of each node (1 = laid-out size), the
    /// geometric mean of `along` and `across`.
    pub scale: Vec<f64>,
    /// Accumulated growth along the petal (base to tip).
    pub along: Vec<f64>,
    /// Accumulated growth across the petal.
    pub across: Vec<f64>,
    pub active: Vec<bool>,
    /// The persistent noise field behind the coin flips (unit variance).
    noise: Vec<f64>,
}

impl Default for Growth {
    fn default() -> Self {
        let n = NU * NV;
        Growth { scale: vec![1.0; n], along: vec![1.0; n], across: vec![1.0; n], active: vec![false; n], noise: Vec::new() }
    }
}

/// Neighbour averaging over the growing nodes (v >= 1); 4-neighbour stencil.
fn smooth(f: &mut [f64], passes: usize) {
    for _ in 0..passes {
        let prev = f.to_vec();
        for v in 1..NV {
            for u in 0..NU {
                let (mut sum, mut n) = (prev[idx(u, v)], 1.0);
                for j in [
                    (u > 0).then(|| idx(u - 1, v)),
                    (u + 1 < NU).then(|| idx(u + 1, v)),
                    (v > 1).then(|| idx(u, v - 1)),
                    (v + 1 < NV).then(|| idx(u, v + 1)),
                ]
                .into_iter()
                .flatten()
                {
                    sum += prev[j];
                    n += 1.0;
                }
                f[idx(u, v)] = sum / n;
            }
        }
    }
}

/// Fresh Gaussian noise smoothed over `corr` nodes and rescaled to unit
/// variance, so correlation changes the patch size, not the activity level.
fn correlated_noise(corr: f64, rng: &mut Pcg32) -> Vec<f64> {
    let mut f: Vec<f64> = (0..NU * NV).map(|_| rng.normal()).collect();
    // Each averaging pass spreads a node's influence by about one node in
    // every direction, so the patch radius grows like sqrt(passes).
    let passes = math::round(corr * corr) as usize;
    smooth(&mut f, passes);
    let m = (NU * (NV - 1)) as f64;
    let mean = f[NU..].iter().sum::<f64>() / m;
    let var = f[NU..].iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / m;
    let sd = math::sqrt(var).max(1e-12);
    f.iter().map(|x| (x - mean) / sd).collect()
}

/// 1 on the petal margin, fading to 0 two nodes in. The attached base
/// (v = 0) is not a margin.
fn margin(u: usize, v: usize) -> f64 {
    let side = u.min(NU - 1 - u) as f64;
    let tip = (NV - 1 - v) as f64;
    (1.0 - side.min(tip) / 2.0).max(0.0)
}

impl Growth {
    /// One stage. `pressure` is per node, already scaled so 1 = fully crowded.
    pub fn step(&mut self, pressure: &[f64], t: &PlantTraits, rng: &mut Pcg32) {
        let old = self.active.clone();
        let fresh = correlated_noise(t.growth_correlation, rng);
        let rho = t.growth_persistence;
        self.noise = if self.noise.is_empty() {
            fresh
        } else {
            let k = math::sqrt(1.0 - rho * rho);
            self.noise.iter().zip(&fresh).map(|(z, e)| rho * z + k * e).collect()
        };
        let mut inc = vec![0.0; NU * NV];
        for v in 1..NV {
            for u in 0..NU {
                let i = idx(u, v);
                let mut n = 0.0;
                let mut on = 0.0;
                let nbrs = [
                    (u > 0).then(|| idx(u - 1, v)),
                    (u + 1 < NU).then(|| idx(u + 1, v)),
                    Some(idx(u, v - 1)),
                    (v + 1 < NV).then(|| idx(u, v + 1)),
                ];
                for j in nbrs.into_iter().flatten() {
                    n += 1.0;
                    if old[j] {
                        on += 1.0;
                    }
                }
                let p = t.activity_base + t.activity_neighbor * on / n - t.pressure_inhibition * pressure[i].min(1.0);
                // Standard normal CDF: a uniform draw, but shared by neighbours
                // and carried between stages through the noise field.
                let coin = 0.5 * (1.0 + math::erf(self.noise[i] / std::f64::consts::SQRT_2));
                self.active[i] = coin < p;
                let rate = t.growth_rate * if self.active[i] { 1.0 } else { t.quiet_growth };
                // The claw next to the receptacle grows less.
                let claw = (v as f64 / 3.0).min(1.0);
                inc[i] = rate * claw * (1.0 + t.edge_boost * margin(u, v));
            }
        }
        smooth(&mut inc, DIFFUSION_PASSES);
        let a = t.growth_anisotropy;
        for (i, d) in inc.iter().enumerate() {
            self.along[i] *= 1.0 + d * (1.0 + a);
            self.across[i] *= 1.0 + d * (1.0 - a);
            self.scale[i] = math::sqrt(self.along[i] * self.across[i]);
        }
    }
}
