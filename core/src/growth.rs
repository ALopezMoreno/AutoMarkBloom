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

use crate::rng::Pcg32;
use crate::sheet::{idx, NU, NV};
use crate::traits::PlantTraits;

/// Smoothing passes applied to each stage's growth increments.
const DIFFUSION_PASSES: usize = 3;

#[derive(Clone, Debug)]
pub struct Growth {
    /// Accumulated linear growth of each node (1 = laid-out size).
    pub scale: Vec<f64>,
    pub active: Vec<bool>,
}

impl Default for Growth {
    fn default() -> Self {
        Growth { scale: vec![1.0; NU * NV], active: vec![false; NU * NV] }
    }
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
                self.active[i] = rng.f64() < p;
                let rate = t.growth_rate * if self.active[i] { 1.0 } else { t.quiet_growth };
                // The claw next to the receptacle grows less.
                let claw = (v as f64 / 3.0).min(1.0);
                inc[i] = rate * claw * (1.0 + t.edge_boost * margin(u, v));
            }
        }
        for _ in 0..DIFFUSION_PASSES {
            let prev = inc.clone();
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
                    inc[idx(u, v)] = sum / n;
                }
            }
        }
        for (s, d) in self.scale.iter_mut().zip(&inc) {
            *s *= 1.0 + d;
        }
    }
}
