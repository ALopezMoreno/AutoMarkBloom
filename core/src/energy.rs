//! The smooth packing energy and its exact gradient.
//!
//! E = springs (stretch, shear and bending, with bending stiffness ~ t^3;
//!     bending uses skip-one springs plus second-difference terms that
//!     resist single-node zigzags, scaled by `zigzag_bending`)
//!   + band   (a loose pull toward the petal's own layer: it may stray
//!     `layer_freedom` slots, so petals can fold across layers; layers
//!     spread out in proportion as the envelope swells)
//!   + envelope / core (stay inside the sepals and outside the stamens)
//!   + sepal  (the sepal envelope's radius R is itself a variable; it
//!     resists growing past its rest size, so petal pressure swells it)
//!   + contact (a node of one petal stays (t_i + t_j)/2 from the control
//!     mesh of every other petal: point-triangle distance, not node-node)
//!
//! Every term is a squared hinge or spring, so E is C1 and its gradient is
//! exact. `tests/` checks it against finite differences.
//!
//! The contact search bins nodes into a flat grid and sorts its result, so
//! float sums never depend on search order.

use crate::math;
use crate::shells::Shells;

#[derive(Clone, Copy, Debug)]
pub struct Spring {
    pub a: usize,
    pub b: usize,
    /// Current rest length (laid-out length times growth).
    pub rest: f64,
    /// Laid-out length.
    pub rest0: f64,
    pub k: f64,
}

/// Three consecutive nodes of a grid line, resisting the change of their
/// second difference |a - 2b + c| from its rest value. Unlike a skip-one
/// spring, this also resists an alternating zigzag of single nodes.
#[derive(Clone, Copy, Debug)]
pub struct Bend {
    pub a: usize,
    pub b: usize,
    pub c: usize,
    pub rest: f64,
    pub rest0: f64,
    pub k: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub k_band: f64,
    pub k_env: f64,
    pub k_contact: f64,
    /// Sepal stiffness against swelling.
    pub k_sepal: f64,
    /// Slot half-width multiplier for the band term.
    pub layer_freedom: f64,
}

impl Default for Params {
    fn default() -> Self {
        Params { k_band: 150.0, k_env: 400.0, k_contact: 1500.0, k_sepal: 50.0, layer_freedom: 1.0 }
    }
}

/// The sepal envelope: current scaled radius and rest radius.
#[derive(Clone, Copy, Debug)]
pub struct Envelope {
    pub r: f64,
    pub rest: f64,
}

#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct Terms {
    pub springs: f64,
    pub band: f64,
    pub envelope: f64,
    pub contact: f64,
    pub sepal: f64,
}

impl Terms {
    pub fn total(&self) -> f64 {
        self.springs + self.band + self.envelope + self.contact + self.sepal
    }
}

#[derive(Clone, Debug)]
pub struct System {
    pub thick: Vec<f64>,
    pub layer: Vec<usize>,
    pub petal: Vec<usize>,
    pub springs: Vec<Spring>,
    pub bends: Vec<Bend>,
    /// Triangles of every petal's control mesh, used for contact.
    pub tris: Vec<[usize; 3]>,
    pub shells: Shells,
    pub params: Params,
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn norm(a: [f64; 3]) -> f64 {
    math::sqrt(a[0] * a[0] + a[1] * a[1] + a[2] * a[2])
}

fn add_scaled(g: &mut [f64; 3], s: f64, d: [f64; 3]) {
    g[0] += s * d[0];
    g[1] += s * d[1];
    g[2] += s * d[2];
}

/// A node of one petal near a triangle of another petal's control mesh.
#[derive(Clone, Copy, Debug)]
pub struct Candidate {
    pub node: usize,
    pub tri: usize,
    /// Node-triangle distance when the list was built.
    pub d0: f64,
}

/// Candidate pairs, sorted by (node, tri), and the positions they were found at.
#[derive(Clone, Debug, Default)]
pub struct CandidateList {
    pub pairs: Vec<Candidate>,
    anchor: Vec<[f64; 3]>,
}

impl CandidateList {
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }
}

/// Barycentric weights of the point on triangle abc closest to p (Ericson).
pub fn closest_on_triangle(p: [f64; 3], a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> [f64; 3] {
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);
    let (d1, d2) = (dot(ab, ap), dot(ac, ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return [1.0, 0.0, 0.0];
    }
    let bp = sub(p, b);
    let (d3, d4) = (dot(ab, bp), dot(ac, bp));
    if d3 >= 0.0 && d4 <= d3 {
        return [0.0, 1.0, 0.0];
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return [1.0 - v, v, 0.0];
    }
    let cp = sub(p, c);
    let (d5, d6) = (dot(ab, cp), dot(ac, cp));
    if d6 >= 0.0 && d5 <= d6 {
        return [0.0, 0.0, 1.0];
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return [1.0 - w, 0.0, w];
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return [0.0, 1.0 - w, w];
    }
    let den = 1.0 / (va + vb + vc);
    let (v, w) = (vb * den, vc * den);
    [1.0 - v - w, v, w]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub struct Contact {
    pub node: usize,
    pub tri: usize,
    pub bary: [f64; 3],
    pub d: f64,
    pub dmin: f64,
}

impl System {
    /// Node/triangle pairs (different petals) whose distance may be under
    /// `reach`, sorted by (node, triangle).
    ///
    /// Nodes are binned into a flat grid by a counting sort (cell order, then
    /// node order), and each triangle scans the cells its padded bounding box
    /// covers. Every node lies in exactly one cell and the final list is
    /// sorted, so the result does not depend on the grid at all; the grid
    /// only decides how many pairs get the exact distance test.
    pub fn candidates(&self, x: &[[f64; 3]], reach: f64) -> CandidateList {
        let mut out = Vec::new();
        if x.is_empty() {
            return CandidateList { pairs: out, anchor: vec![] };
        }
        let mut lo0 = [f64::MAX; 3];
        let mut hi0 = [f64::MIN; 3];
        for p in x {
            for k in 0..3 {
                lo0[k] = lo0[k].min(p[k]);
                hi0[k] = hi0[k].max(p[k]);
            }
        }
        // Cells about `reach` wide, and few enough to stay small.
        let extent = (0..3).map(|k| hi0[k] - lo0[k]).fold(0.0, f64::max);
        let mut cell = reach.max(extent / 64.0).max(1e-9);
        let dims = |cell: f64| [0, 1, 2].map(|k| math::floor((hi0[k] - lo0[k]) / cell) as usize + 1);
        while dims(cell).iter().product::<usize>() > 4 * x.len() + 64 {
            cell *= 1.5;
        }
        let n = dims(cell);
        let key = |v: f64, k: usize| math::floor((v - lo0[k]) / cell);
        let cell_of = |p: [f64; 3]| {
            let c = [0, 1, 2].map(|k| key(p[k], k) as usize);
            (c[2] * n[1] + c[1]) * n[0] + c[0]
        };
        // Counting sort of nodes into cells; nodes keep their order within a cell.
        let mut start = vec![0usize; n[0] * n[1] * n[2] + 1];
        let cells: Vec<usize> = x.iter().map(|&p| cell_of(p)).collect();
        for &c in &cells {
            start[c + 1] += 1;
        }
        for c in 1..start.len() {
            start[c] += start[c - 1];
        }
        let mut fill = start.clone();
        let mut order = vec![0usize; x.len()];
        for (i, &c) in cells.iter().enumerate() {
            order[fill[c]] = i;
            fill[c] += 1;
        }
        // Positions and petals in cell order, for locality.
        let pts: Vec<[f64; 3]> = order.iter().map(|&i| x[i]).collect();
        let pet: Vec<usize> = order.iter().map(|&i| self.petal[i]).collect();
        let range = |lo: f64, hi: f64, k: usize| {
            let a = key(lo, k).max(0.0) as usize;
            let b = (key(hi, k).max(0.0) as usize).min(n[k] - 1);
            a..=b
        };
        for (ti, &[a, b, c]) in self.tris.iter().enumerate() {
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
            let owner = self.petal[a];
            for gz in range(lo[2], hi[2], 2) {
                for gy in range(lo[1], hi[1], 1) {
                    let row = (gz * n[1] + gy) * n[0];
                    let xs = range(lo[0], hi[0], 0);
                    // Cells along x are contiguous, so scan them as one run.
                    for j in start[row + xs.start()]..start[row + xs.end() + 1] {
                        let p = pts[j];
                        if pet[j] == owner || !(0..3).all(|k| p[k] >= lo[k] && p[k] <= hi[k]) {
                            continue;
                        }
                        let w = closest_on_triangle(p, x[a], x[b], x[c]);
                        let q = [
                            w[0] * x[a][0] + w[1] * x[b][0] + w[2] * x[c][0],
                            w[0] * x[a][1] + w[1] * x[b][1] + w[2] * x[c][1],
                            w[0] * x[a][2] + w[1] * x[b][2] + w[2] * x[c][2],
                        ];
                        let d = norm(sub(p, q));
                        if d < reach {
                            out.push(Candidate { node: order[j], tri: ti, d0: d });
                        }
                    }
                }
            }
        }
        out.sort_unstable_by_key(|c| (c.node, c.tri));
        CandidateList { pairs: out, anchor: x.to_vec() }
    }

    /// Candidates that are actually closer than their combined half-thickness.
    ///
    /// Point-triangle distance changes by at most how far the point and the
    /// triangle's corners have moved, so a pair whose distance at build time,
    /// less those moves, is still at least `dmin` cannot be in contact. Those
    /// pairs are skipped without the closest-point test. This drops nothing
    /// that the test would keep, so the result is unchanged, only cheaper:
    /// in a packed bud about 99% of candidates are skipped.
    pub fn contacts(&self, x: &[[f64; 3]], cands: &CandidateList) -> Vec<Contact> {
        let moved: Vec<f64> = x.iter().zip(&cands.anchor).map(|(p, q)| norm(sub(*p, *q))).collect();
        let mut out = Vec::new();
        for c in &cands.pairs {
            let [a, b, cc] = self.tris[c.tri];
            let tq = (self.thick[a] + self.thick[b] + self.thick[cc]) / 3.0;
            let dmin = 0.5 * (self.thick[c.node] + tq);
            // The small margin covers rounding in the bound itself.
            if c.d0 - moved[c.node] - moved[a].max(moved[b]).max(moved[cc]) - 1e-9 >= dmin {
                continue;
            }
            let bary = closest_on_triangle(x[c.node], x[a], x[b], x[cc]);
            let q = [
                bary[0] * x[a][0] + bary[1] * x[b][0] + bary[2] * x[cc][0],
                bary[0] * x[a][1] + bary[1] * x[b][1] + bary[2] * x[cc][1],
                bary[0] * x[a][2] + bary[1] * x[b][2] + bary[2] * x[cc][2],
            ];
            // Triangle-mean thickness (tq above) keeps dmin independent of where
            // the closest point lands, so the contact gradient stays exact.
            let d = norm(sub(x[c.node], q));
            if d < dmin {
                out.push(Contact { node: c.node, tri: c.tri, bary, d, dmin });
            }
        }
        out
    }

    /// How much a layer's radius moves per unit of envelope radius: layers
    /// keep their relative place between the stamens and the sepals.
    pub fn layer_weight(&self, layer: usize) -> f64 {
        (self.shells.radius[layer] - self.shells.core) / (self.shells.envelope - self.shells.core)
    }

    /// How far a node sits outside its layer's band (negative when inside),
    /// and which side it is on.
    pub fn band_excess(&self, i: usize, p: [f64; 3], env_r: f64) -> (f64, f64) {
        let a = self.shells.core + self.layer_weight(self.layer[i]) * (env_r - self.shells.core);
        let halfw = ((self.shells.slot * self.params.layer_freedom - self.thick[i]) * 0.5).max(0.0);
        let r = self.shells.scaled_radius(p);
        ((r - a).abs() - halfw, (r - a).signum())
    }

    /// Returns the energy terms and writes dE/dx into `grad`. Contact uses
    /// the given candidate list (see `candidates`); its gradient follows the
    /// envelope theorem for point-triangle distance, so it is exact wherever
    /// the closest point is unique.
    pub fn energy(&self, x: &[[f64; 3]], env: Envelope, cands: &CandidateList, grad: &mut [[f64; 3]], genv: &mut f64) -> Terms {
        for g in grad.iter_mut() {
            *g = [0.0; 3];
        }
        let mut t = Terms::default();
        let p = self.params;
        let ds = env.r - env.rest;
        t.sepal = 0.5 * p.k_sepal * ds * ds;
        *genv = p.k_sepal * ds;

        for s in &self.springs {
            let d = sub(x[s.a], x[s.b]);
            let l = norm(d).max(1e-12);
            let e = l - s.rest;
            t.springs += 0.5 * s.k * e * e;
            let c = s.k * e / l;
            add_scaled(&mut grad[s.a], c, d);
            add_scaled(&mut grad[s.b], -c, d);
        }

        for b in &self.bends {
            let d = [
                x[b.a][0] - 2.0 * x[b.b][0] + x[b.c][0],
                x[b.a][1] - 2.0 * x[b.b][1] + x[b.c][1],
                x[b.a][2] - 2.0 * x[b.b][2] + x[b.c][2],
            ];
            let l = norm(d).max(1e-12);
            let e = l - b.rest;
            t.springs += 0.5 * b.k * e * e;
            let c = b.k * e / l;
            add_scaled(&mut grad[b.a], c, d);
            add_scaled(&mut grad[b.b], -2.0 * c, d);
            add_scaled(&mut grad[b.c], c, d);
        }

        for (i, &xi) in x.iter().enumerate() {
            let gr = self.shells.grad_scaled_radius(xi);
            let r = self.shells.scaled_radius(xi);
            let (ex, sign) = self.band_excess(i, xi, env.r);
            if ex > 0.0 {
                t.band += 0.5 * p.k_band * ex * ex;
                add_scaled(&mut grad[i], p.k_band * ex * sign, gr);
                *genv -= p.k_band * ex * sign * self.layer_weight(self.layer[i]);
            }
            let half = 0.5 * self.thick[i];
            let out = r - (env.r - half);
            if out > 0.0 {
                t.envelope += 0.5 * p.k_env * out * out;
                add_scaled(&mut grad[i], p.k_env * out, gr);
                *genv -= p.k_env * out;
            }
            let inn = (self.shells.core + half) - r;
            if inn > 0.0 {
                t.envelope += 0.5 * p.k_env * inn * inn;
                add_scaled(&mut grad[i], -p.k_env * inn, gr);
            }
        }

        for c in self.contacts(x, cands) {
            let e = c.dmin - c.d;
            t.contact += 0.5 * p.k_contact * e * e;
            let [a, b, cc] = self.tris[c.tri];
            let q = [
                c.bary[0] * x[a][0] + c.bary[1] * x[b][0] + c.bary[2] * x[cc][0],
                c.bary[0] * x[a][1] + c.bary[1] * x[b][1] + c.bary[2] * x[cc][1],
                c.bary[0] * x[a][2] + c.bary[1] * x[b][2] + c.bary[2] * x[cc][2],
            ];
            let n = sub(x[c.node], q);
            let k = -p.k_contact * e / c.d.max(1e-12);
            add_scaled(&mut grad[c.node], k, n);
            for (m, &v) in [a, b, cc].iter().enumerate() {
                add_scaled(&mut grad[v], -k * c.bary[m], n);
            }
            // dmin depends on thickness only, which is fixed, so no extra terms.
        }
        t
    }

    /// Contact plus envelope force magnitude felt by each node.
    pub fn pressure(&self, x: &[[f64; 3]], env: Envelope, cands: &CandidateList) -> Vec<f64> {
        let p = self.params;
        let mut out = vec![0.0; x.len()];
        for c in self.contacts(x, cands) {
            let f = p.k_contact * (c.dmin - c.d);
            out[c.node] += f;
            let [a, b, cc] = self.tris[c.tri];
            for (m, v) in [a, b, cc].into_iter().enumerate() {
                out[v] += f * c.bary[m];
            }
        }
        for (i, &xi) in x.iter().enumerate() {
            let o = self.shells.scaled_radius(xi) - (env.r - 0.5 * self.thick[i]);
            if o > 0.0 {
                out[i] += p.k_env * o;
            }
        }
        out
    }
}
