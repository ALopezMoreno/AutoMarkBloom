//! Petal-petal contact for the opening, on densely sampled petals.
//!
//! Opening moves petals much further than packing does, and two coarse
//! control meshes can pass through each other edge-first without any node
//! coming near the other mesh. So contact here tests sample points spread
//! over every grid quad (bilinear in its four nodes), not just the nodes,
//! against the other petals' control triangles. A sample's force goes back
//! to its four nodes by the same weights, so the gradient stays exact.
//!
//! The candidate search uses a BTreeMap grid and sorted output, so nothing
//! depends on hash order.

use crate::energy::closest_on_triangle;
use crate::math;
use std::collections::BTreeMap;

/// A point on a petal: four nodes and their bilinear weights.
#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub nodes: [usize; 4],
    pub w: [f64; 4],
    pub petal: usize,
}

pub struct Contacts {
    pub samples: Vec<Sample>,
    pub tris: Vec<[usize; 3]>,
    pub tri_petal: Vec<usize>,
    /// (sample, triangle) pairs within reach at the last rebuild.
    pub cands: Vec<(usize, usize)>,
    /// Per pair, a lower bound on its distance with every node at its
    /// anchor (see `refresh`), so `energy` can skip pairs that cannot touch.
    bound: Vec<f64>,
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn norm(a: [f64; 3]) -> f64 {
    math::sqrt(a[0] * a[0] + a[1] * a[1] + a[2] * a[2])
}

impl Sample {
    pub fn at(&self, x: &[[f64; 3]]) -> [f64; 3] {
        let mut p = [0.0; 3];
        for k in 0..4 {
            for c in 0..3 {
                p[c] += self.w[k] * x[self.nodes[k]][c];
            }
        }
        p
    }
}

impl Contacts {
    /// `grids`: (offset of the petal's first node, nu, nv) per petal.
    /// `per_side` samples per quad side (1 = nodes only).
    pub fn new(grids: &[(usize, usize, usize)], per_side: usize) -> Contacts {
        let s = per_side.max(1);
        let mut samples = Vec::new();
        let mut tris = Vec::new();
        let mut tri_petal = Vec::new();
        for (p, &(o, nu, nv)) in grids.iter().enumerate() {
            let id = |u: usize, v: usize| o + v * nu + u;
            for v in 0..nv - 1 {
                for u in 0..nu - 1 {
                    let q = [id(u, v), id(u + 1, v), id(u, v + 1), id(u + 1, v + 1)];
                    tris.push([q[0], q[1], q[3]]);
                    tris.push([q[0], q[3], q[2]]);
                    tri_petal.extend([p, p]);
                    // Corner (0, 0) of each quad, plus the far rim on the last row/column.
                    let ia = if u + 2 == nu { s + 1 } else { s };
                    let ib = if v + 2 == nv { s + 1 } else { s };
                    for j in 0..ib {
                        for i in 0..ia {
                            let (a, b) = (i as f64 / s as f64, j as f64 / s as f64);
                            let w = [(1.0 - a) * (1.0 - b), a * (1.0 - b), (1.0 - a) * b, a * b];
                            samples.push(Sample { nodes: q, w, petal: p });
                        }
                    }
                }
            }
        }
        Contacts { samples, tris, tri_petal, cands: vec![], bound: vec![] }
    }

    /// Rebuild the candidate list: samples within `reach` of a triangle of another petal.
    pub fn rebuild(&mut self, x: &[[f64; 3]], reach: f64) {
        let all = vec![true; self.tri_petal.last().map_or(0, |&p| p + 1)];
        self.refresh(x, reach, &all, x);
    }

    /// Rebuild only the pairs that involve a `dirty` petal (as sample or as
    /// triangle) and keep the rest. With a Verlet skin this is exact if every
    /// petal that moved more than half the skin since its last refresh is
    /// dirty: a kept pair's two petals have together moved less than the
    /// skin since it was found. The list stays sorted.
    ///
    /// `anchor` holds each node's position at its petal's last refresh,
    /// already updated for the dirty petals. Each new pair stores its
    /// distance minus how far its nodes are from their anchors, a lower
    /// bound on the distance at the anchors.
    pub fn refresh(&mut self, x: &[[f64; 3]], reach: f64, dirty: &[bool], anchor: &[[f64; 3]]) {
        let moved = |v: usize| norm(sub(x[v], anchor[v]));
        let cell = 0.25f64.max(reach);
        let key = |v: f64| math::floor(v / cell) as i64;
        // Every sample, for the dirty petals' triangles; only the dirty
        // petals' samples, for the other triangles.
        let mut grid: BTreeMap<(i64, i64, i64), Vec<usize>> = BTreeMap::new();
        let mut dirty_grid: BTreeMap<(i64, i64, i64), Vec<usize>> = BTreeMap::new();
        let pts: Vec<[f64; 3]> = self.samples.iter().map(|s| s.at(x)).collect();
        for (i, p) in pts.iter().enumerate() {
            let k = (key(p[0]), key(p[1]), key(p[2]));
            if dirty[self.samples[i].petal] {
                dirty_grid.entry(k).or_default().push(i);
            }
            grid.entry(k).or_default().push(i);
        }
        let mut out = Vec::new();
        for (ti, &[a, b, c]) in self.tris.iter().enumerate() {
            let mut lo = [f64::MAX; 3];
            let mut hi = [f64::MIN; 3];
            for v in [a, b, c] {
                for k in 0..3 {
                    lo[k] = lo[k].min(x[v][k] - reach);
                    hi[k] = hi[k].max(x[v][k] + reach);
                }
            }
            let owner = self.tri_petal[ti];
            let grid = if dirty[owner] { &grid } else { &dirty_grid };
            for gx in key(lo[0])..=key(hi[0]) {
                for gy in key(lo[1])..=key(hi[1]) {
                    for gz in key(lo[2])..=key(hi[2]) {
                        let Some(list) = grid.get(&(gx, gy, gz)) else { continue };
                        for &i in list {
                            let p = pts[i];
                            if self.samples[i].petal == owner || !(0..3).all(|k| p[k] >= lo[k] && p[k] <= hi[k]) {
                                continue;
                            }
                            let w = closest_on_triangle(p, x[a], x[b], x[c]);
                            let q = [0, 1, 2].map(|k| w[0] * x[a][k] + w[1] * x[b][k] + w[2] * x[c][k]);
                            let d = norm(sub(p, q));
                            if d < reach {
                                let ms = self.samples[i].nodes.iter().map(|&v| moved(v)).fold(0.0, f64::max);
                                let mt = [a, b, c].iter().map(|&v| moved(v)).fold(0.0, f64::max);
                                out.push((i, ti, d - ms - mt));
                            }
                        }
                    }
                }
            }
        }
        let samples = &self.samples;
        let tri_petal = &self.tri_petal;
        out.extend(self.cands.iter().zip(&self.bound).filter(|(&(si, ti), _)| !dirty[samples[si].petal] && !dirty[tri_petal[ti]]).map(|(&(si, ti), &b)| (si, ti, b)));
        out.sort_unstable_by_key(|&(si, ti, _)| (si, ti));
        self.cands = out.iter().map(|&(si, ti, _)| (si, ti)).collect();
        self.bound = out.iter().map(|&(_, _, b)| b).collect();
    }

    fn thick_at(&self, s: &Sample, thick: &[f64]) -> f64 {
        (0..4).map(|k| s.w[k] * thick[s.nodes[k]]).sum()
    }

    /// Contact energy; adds its gradient into `grad`. `anchor` is as in
    /// `refresh`: a pair whose stored bound, less how far its nodes have
    /// since moved from their anchors, is still at least the contact
    /// distance cannot be touching and is skipped without the exact test.
    pub fn energy(&self, x: &[[f64; 3]], thick: &[f64], k: f64, grad: &mut [[f64; 3]], anchor: &[[f64; 3]]) -> f64 {
        let moved: Vec<f64> = x.iter().zip(anchor).map(|(p, q)| norm(sub(*p, *q))).collect();
        let mut e = 0.0;
        for (&(si, ti), &bound) in self.cands.iter().zip(&self.bound) {
            let s = &self.samples[si];
            let [a, b, c] = self.tris[ti];
            let tq = (thick[a] + thick[b] + thick[c]) / 3.0;
            let dmin = 0.5 * (self.thick_at(s, thick) + tq);
            let ms = s.nodes.iter().map(|&v| moved[v]).fold(0.0, f64::max);
            let mt = moved[a].max(moved[b]).max(moved[c]);
            if bound - ms - mt - 1e-9 >= dmin {
                continue;
            }
            let p = s.at(x);
            let w = closest_on_triangle(p, x[a], x[b], x[c]);
            let q = [0, 1, 2].map(|m| w[0] * x[a][m] + w[1] * x[b][m] + w[2] * x[c][m]);
            let d = norm(sub(p, q));
            if d >= dmin {
                continue;
            }
            let ex = dmin - d;
            e += 0.5 * k * ex * ex;
            let n = sub(p, q);
            let f = -k * ex / d.max(1e-12);
            for m in 0..4 {
                for c3 in 0..3 {
                    grad[s.nodes[m]][c3] += f * s.w[m] * n[c3];
                }
            }
            for (m, &v) in [a, b, c].iter().enumerate() {
                for c3 in 0..3 {
                    grad[v][c3] -= f * w[m] * n[c3];
                }
            }
        }
        e
    }

    /// Worst overlap among current candidates.
    pub fn max_overlap(&self, x: &[[f64; 3]], thick: &[f64]) -> f64 {
        let mut worst: f64 = 0.0;
        for &(si, ti) in &self.cands {
            let s = &self.samples[si];
            let [a, b, c] = self.tris[ti];
            let p = s.at(x);
            let w = closest_on_triangle(p, x[a], x[b], x[c]);
            let q = [0, 1, 2].map(|m| w[0] * x[a][m] + w[1] * x[b][m] + w[2] * x[c][m]);
            let dmin = 0.5 * (self.thick_at(s, thick) + (thick[a] + thick[b] + thick[c]) / 3.0);
            worst = worst.max(dmin - norm(sub(p, q)));
        }
        worst
    }
}
