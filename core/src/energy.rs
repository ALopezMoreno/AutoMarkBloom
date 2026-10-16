//! The smooth packing energy and its exact gradient.
//!
//! E = springs (stretch, shear and bending, with bending stiffness ~ t^3)
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
//! The contact search uses a HashMap only for lookup; contacts are sorted
//! before use, so float sums never depend on hash order.

use crate::math;
use crate::shells::Shells;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

/// Cheap fixed hasher for grid cells (lookup only; never affects results).
#[derive(Default)]
pub struct CellHasher(u64);

impl Hasher for CellHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0.rotate_left(5) ^ b as u64).wrapping_mul(0x51_7C_C1_B7_27_22_0A_95);
        }
    }
    fn write_i64(&mut self, v: i64) {
        self.0 = (self.0.rotate_left(5) ^ v as u64).wrapping_mul(0x51_7C_C1_B7_27_22_0A_95);
    }
}

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
    /// `reach`. Built from a spatial hash; the result is sorted, so nothing
    /// downstream depends on hash order.
    pub fn candidates(&self, x: &[[f64; 3]], reach: f64) -> Vec<Candidate> {
        let cell = 0.25f64.max(reach);
        let key = |v: f64| math::floor(v / cell) as i64;
        let mut grid: HashMap<(i64, i64, i64), Vec<usize>, BuildHasherDefault<CellHasher>> = HashMap::default();
        for (i, &p) in x.iter().enumerate() {
            grid.entry((key(p[0]), key(p[1]), key(p[2]))).or_default().push(i);
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
            let owner = self.petal[a];
            for gx in key(lo[0])..=key(hi[0]) {
                for gy in key(lo[1])..=key(hi[1]) {
                    for gz in key(lo[2])..=key(hi[2]) {
                        let Some(list) = grid.get(&(gx, gy, gz)) else { continue };
                        for &i in list {
                            let p = x[i];
                            if self.petal[i] == owner || !(0..3).all(|k| p[k] >= lo[k] && p[k] <= hi[k]) {
                                continue;
                            }
                            let w = closest_on_triangle(p, x[a], x[b], x[c]);
                            let q = [
                                w[0] * x[a][0] + w[1] * x[b][0] + w[2] * x[c][0],
                                w[0] * x[a][1] + w[1] * x[b][1] + w[2] * x[c][1],
                                w[0] * x[a][2] + w[1] * x[b][2] + w[2] * x[c][2],
                            ];
                            if norm(sub(p, q)) < reach {
                                out.push(Candidate { node: i, tri: ti });
                            }
                        }
                    }
                }
            }
        }
        out.sort_unstable_by_key(|c| (c.node, c.tri));
        out
    }

    /// Candidates that are actually closer than their combined half-thickness.
    pub fn contacts(&self, x: &[[f64; 3]], cands: &[Candidate]) -> Vec<Contact> {
        let mut out = Vec::new();
        for c in cands {
            let [a, b, cc] = self.tris[c.tri];
            let bary = closest_on_triangle(x[c.node], x[a], x[b], x[cc]);
            let q = [
                bary[0] * x[a][0] + bary[1] * x[b][0] + bary[2] * x[cc][0],
                bary[0] * x[a][1] + bary[1] * x[b][1] + bary[2] * x[cc][1],
                bary[0] * x[a][2] + bary[1] * x[b][2] + bary[2] * x[cc][2],
            ];
            // Triangle-mean thickness keeps dmin independent of where the closest
            // point lands, so the contact gradient stays exact.
            let tq = (self.thick[a] + self.thick[b] + self.thick[cc]) / 3.0;
            let dmin = 0.5 * (self.thick[c.node] + tq);
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
    pub fn energy(&self, x: &[[f64; 3]], env: Envelope, cands: &[Candidate], grad: &mut [[f64; 3]], genv: &mut f64) -> Terms {
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
    pub fn pressure(&self, x: &[[f64; 3]], env: Envelope, cands: &[Candidate]) -> Vec<f64> {
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
