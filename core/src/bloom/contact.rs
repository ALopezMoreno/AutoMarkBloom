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
use crate::par;

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
        if self.samples.is_empty() {
            return;
        }
        let moved: Vec<f64> = x.iter().zip(anchor).map(|(p, q)| norm(sub(*p, *q))).collect();
        let pts: Vec<[f64; 3]> = self.samples.iter().map(|s| s.at(x)).collect();
        let mut lo0 = [f64::MAX; 3];
        let mut hi0 = [f64::MIN; 3];
        for p in &pts {
            for k in 0..3 {
                lo0[k] = lo0[k].min(p[k]);
                hi0[k] = hi0[k].max(p[k]);
            }
        }
        let extent = (0..3).map(|k| hi0[k] - lo0[k]).fold(0.0, f64::max);
        let mut cell = reach.max(extent / 96.0).max(1e-9);
        let dims = |cell: f64| [0, 1, 2].map(|k| math::floor((hi0[k] - lo0[k]) / cell) as usize + 1);
        while dims(cell).iter().product::<usize>() > 4 * pts.len() + 64 {
            cell *= 1.5;
        }
        let n = dims(cell);
        let key = |v: f64, k: usize| math::floor((v - lo0[k]) / cell);
        let cell_of = |p: &[f64; 3]| {
            let c = [0, 1, 2].map(|k| key(p[k], k) as usize);
            (c[2] * n[1] + c[1]) * n[0] + c[0]
        };
        let cells: Vec<usize> = pts.iter().map(cell_of).collect();
        // Counting sort of the chosen samples into cells (sample order kept
        // within a cell): cell starts, sample indices, positions, petals.
        let bin = |take: &dyn Fn(usize) -> bool| {
            let mut start = vec![0usize; n[0] * n[1] * n[2] + 1];
            for (i, &c) in cells.iter().enumerate() {
                if take(i) {
                    start[c + 1] += 1;
                }
            }
            for c in 1..start.len() {
                start[c] += start[c - 1];
            }
            let mut fill = start.clone();
            let mut order = vec![0usize; start[start.len() - 1]];
            for (i, &c) in cells.iter().enumerate() {
                if take(i) {
                    order[fill[c]] = i;
                    fill[c] += 1;
                }
            }
            let p: Vec<[f64; 3]> = order.iter().map(|&i| pts[i]).collect();
            let pet: Vec<usize> = order.iter().map(|&i| self.samples[i].petal).collect();
            (start, order, p, pet)
        };
        let full = bin(&|_| true);
        let only_dirty = bin(&|i| dirty[self.samples[i].petal]);
        let range = |lo: f64, hi: f64, k: usize| {
            let a = key(lo, k).max(0.0) as usize;
            let b = (key(hi, k).max(0.0) as usize).min(n[k] - 1);
            a..=b
        };
        // Triangles go to threads in fixed chunks, joined in order and then
        // sorted, so the list is the same on any number of cores.
        let mut out = par::flat_map_chunks(self.tris.len(), |tris| {
            let mut out = Vec::new();
            for ti in tris {
                let [a, b, c] = self.tris[ti];
                let owner = self.tri_petal[ti];
                let mut lo = [f64::MAX; 3];
                let mut hi = [f64::MIN; 3];
                for v in [a, b, c] {
                    for k in 0..3 {
                        lo[k] = lo[k].min(x[v][k] - reach);
                        hi[k] = hi[k].max(x[v][k] + reach);
                    }
                }
                if (0..3).any(|k| hi[k] < lo0[k] || lo[k] > hi0[k]) {
                    continue;
                }
                let (start, order, spts, spet) = if dirty[owner] { &full } else { &only_dirty };
                let mt = moved[a].max(moved[b]).max(moved[c]);
                // Distance to the triangle is at least distance to its plane.
                let (e1, e2) = (sub(x[b], x[a]), sub(x[c], x[a]));
                let nrm = [e1[1] * e2[2] - e1[2] * e2[1], e1[2] * e2[0] - e1[0] * e2[2], e1[0] * e2[1] - e1[1] * e2[0]];
                let nl = norm(nrm);
                let plane = nl > 1e-12 * (norm(e1) * norm(e2)).max(1e-300);
                let nh = if plane { [nrm[0] / nl, nrm[1] / nl, nrm[2] / nl] } else { [0.0; 3] };
                let far = reach + 1e-9 * (1.0 + reach);
                for gz in range(lo[2], hi[2], 2) {
                    for gy in range(lo[1], hi[1], 1) {
                        let row = (gz * n[1] + gy) * n[0];
                        let xs = range(lo[0], hi[0], 0);
                        for j in start[row + xs.start()]..start[row + xs.end() + 1] {
                            let sp = spet[j];
                            if sp == owner {
                                continue;
                            }
                            let p = spts[j];
                            if !(0..3).all(|k| p[k] >= lo[k] && p[k] <= hi[k]) {
                                continue;
                            }
                            if plane {
                                let ap = sub(p, x[a]);
                                if (ap[0] * nh[0] + ap[1] * nh[1] + ap[2] * nh[2]).abs() > far {
                                    continue;
                                }
                            }
                            let w = closest_on_triangle(p, x[a], x[b], x[c]);
                            let q = [0, 1, 2].map(|k| w[0] * x[a][k] + w[1] * x[b][k] + w[2] * x[c][k]);
                            let d = norm(sub(p, q));
                            if d < reach {
                                let i = order[j];
                                let ms = self.samples[i].nodes.iter().map(|&v| moved[v]).fold(0.0, f64::max);
                                out.push((i, ti, d - ms - mt));
                            }
                        }
                    }
                }
            }
            out
        });
        out.sort_unstable_by_key(|&(si, ti, _)| (si, ti));
        // Kept pairs are already sorted, and never coincide with new ones
        // (a new pair has a dirty petal, a kept one has none): merge.
        let samples = &self.samples;
        let tri_petal = &self.tri_petal;
        let kept: Vec<(usize, usize, f64)> = self.cands.iter().zip(&self.bound).filter(|(&(si, ti), _)| !dirty[samples[si].petal] && !dirty[tri_petal[ti]]).map(|(&(si, ti), &b)| (si, ti, b)).collect();
        let mut merged = Vec::with_capacity(out.len() + kept.len());
        let (mut i, mut j) = (0, 0);
        while i < out.len() || j < kept.len() {
            if j == kept.len() || (i < out.len() && (out[i].0, out[i].1) < (kept[j].0, kept[j].1)) {
                merged.push(out[i]);
                i += 1;
            } else {
                merged.push(kept[j]);
                j += 1;
            }
        }
        let out = merged;
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
        // Touching pairs are found in fixed chunks across threads and joined
        // in pair order; the sums below then run in that order in one thread,
        // so the result is the same on any number of cores.
        let touching = par::flat_map_chunks(self.cands.len(), |range| {
            let mut out = Vec::new();
            for (&(si, ti), &bound) in self.cands[range.clone()].iter().zip(&self.bound[range]) {
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
                if d < dmin {
                    out.push((si, ti, w, sub(p, q), d, dmin));
                }
            }
            out
        });
        let mut e = 0.0;
        for (si, ti, w, n, d, dmin) in touching {
            let s = &self.samples[si];
            let [a, b, c] = self.tris[ti];
            let ex = dmin - d;
            e += 0.5 * k * ex * ex;
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
