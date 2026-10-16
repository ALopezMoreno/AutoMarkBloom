//! Stochastic cellular automaton on each shell's surface.
//!
//! Each shell is an NA x NP grid: azimuth around the bud by polar rows from
//! the attachment ring (row 0) up toward the top. Per shell, inner to outer:
//!
//! 1. Initiation: row-0 cells, in random order, become primordia where the
//!    Gaussian inhibition from this layer's primordia, plus the layer below's
//!    weighted by `stagger`, is under the threshold. Layers interleave.
//! 2. Growth: synchronous sweeps. An empty cell can be claimed by the petal
//!    below it (`up_prob`) or beside it (`lateral_prob`) unless that petal
//!    has reached its height. Petals compete for the surface, so outlines
//!    come from packing on the shell.

use crate::math;
use crate::rng::Pcg32;
use crate::shells::Shells;
use crate::traits::PlantTraits;
use std::f64::consts::TAU;

pub const NA: usize = 96;
pub const NP: usize = 28;
const EMPTY: i32 = -1;

#[derive(Clone, Debug)]
pub struct PetalPatch {
    pub id: usize,
    pub layer: usize,
    pub base_angle: f64,
    /// Rows the petal may reach.
    pub max_rows: usize,
    /// Per row: (circular-mean angle, angular span) of the cells it owns.
    pub rows: Vec<Option<(f64, f64)>>,
    pub cells: usize,
}

#[derive(Clone, Debug)]
pub struct BudLayout {
    /// Per layer, NP * NA owners (row-major), -1 for empty.
    pub grids: Vec<Vec<i32>>,
    pub petals: Vec<PetalPatch>,
}

fn theta(s: usize) -> f64 {
    (s as f64 + 0.5) * TAU / NA as f64
}

fn ang_diff(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(TAU);
    d.min(TAU - d)
}

pub fn run(t: &PlantTraits, shells: &Shells, rng: &mut Pcg32) -> BudLayout {
    let mut grids = Vec::with_capacity(t.layers);
    let mut base: Vec<(usize, f64)> = Vec::new(); // (layer, angle) of each primordium
    let mut max_rows: Vec<usize> = Vec::new();
    let two_s2 = 2.0 * t.inhibit_sigma * t.inhibit_sigma;

    for k in 0..t.layers {
        let a = shells.radius[k];
        let frac = if t.layers > 1 { k as f64 / (t.layers - 1) as f64 } else { 1.0 };
        let mut g = vec![EMPTY; NP * NA];

        for _pass in 0..4 {
            let mut order: Vec<usize> = (0..NA).collect();
            rng.shuffle(&mut order);
            for s in order {
                if g[s] != EMPTY {
                    continue;
                }
                let th = theta(s);
                let mut inh = 0.0;
                for &(l, b) in &base {
                    let w = if l == k { 1.0 } else if l + 1 == k { t.stagger } else { 0.0 };
                    if w > 0.0 {
                        let d = ang_diff(th, b) * a;
                        inh += w * math::exp(-d * d / two_s2);
                    }
                }
                if inh < t.inhibit_threshold && rng.f64() < t.init_prob {
                    let id = base.len();
                    base.push((k, th));
                    let h = NP as f64 * (t.inner_height + (1.0 - t.inner_height) * frac) * rng.range(0.85, 1.0);
                    max_rows.push((math::round(h) as usize).clamp(4, NP));
                    g[s] = id as i32;
                }
            }
        }

        for _sweep in 0..NP * 3 {
            let old = g.clone();
            let mut changed = false;
            for r in 0..NP {
                for s in 0..NA {
                    let i = r * NA + s;
                    if old[i] != EMPTY {
                        continue;
                    }
                    let mut cands: [(i32, f64); 3] = [(EMPTY, 0.0); 3];
                    let mut n = 0;
                    let fits = |p: i32| p >= 0 && r < max_rows[p as usize];
                    if r > 0 && fits(old[i - NA]) {
                        cands[n] = (old[i - NA], t.up_prob);
                        n += 1;
                    }
                    for nb in [r * NA + (s + NA - 1) % NA, r * NA + (s + 1) % NA] {
                        if fits(old[nb]) {
                            cands[n] = (old[nb], t.lateral_prob);
                            n += 1;
                        }
                    }
                    if n == 0 {
                        continue;
                    }
                    let start = rng.int(0, n as i64 - 1) as usize;
                    for j in 0..n {
                        let (p, prob) = cands[(start + j) % n];
                        if rng.f64() < prob {
                            g[i] = p;
                            changed = true;
                            break;
                        }
                    }
                }
            }
            if !changed && _sweep > NP {
                break;
            }
        }
        grids.push(g);
    }

    let dth = TAU / NA as f64;
    let mut petals = Vec::new();
    for (id, &(layer, ang)) in base.iter().enumerate() {
        let g = &grids[layer];
        let mut rows = vec![None; NP];
        let mut cells = 0;
        for (r, row) in rows.iter_mut().enumerate() {
            let (mut sx, mut sy, mut n) = (0.0, 0.0, 0usize);
            for s in 0..NA {
                if g[r * NA + s] == id as i32 {
                    sx += math::cos(theta(s));
                    sy += math::sin(theta(s));
                    n += 1;
                }
            }
            if n > 0 {
                *row = Some((math::atan2(sy, sx), n as f64 * dth));
                cells += n;
            }
        }
        petals.push(PetalPatch { id, layer, base_angle: ang, max_rows: max_rows[id], rows, cells });
    }
    BudLayout { grids, petals }
}
