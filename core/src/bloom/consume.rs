//! The geometry consumer: the only code that knows what channels mean.
//!
//! Each step it builds a pose for every petal, the shape that petal's
//! growth asks for, and the relaxation then pulls the petals toward their
//! poses while contact keeps them apart. A pose is made like this:
//!
//! 1. The petal as first laid on its shell is a stack of rows, each row an
//!    arc at one height. Its centre column is a curve in the meridian
//!    plane (distance from the axis, height): segment lengths and turning
//!    angles. The opened centre curve keeps the bud's turning scaled by
//!    `memory`, bends back by `reflex` at the base hinge, and stretches by
//!    `length`. Starting from the same base point and base direction, the
//!    curve is integrated again, so with no growth the bud comes back exactly.
//! 2. Each row keeps its arc length times `width` and moves to the new
//!    distance from the axis. `wrap` blends the row between that arc (1) and
//!    a flat line (0); `cupping` bends it toward the axis.
//! 3. What packing added on top of the laid sheet (crumples, folds, swelling)
//!    is carried in the petal's local frame and re-applied, scaled by `ruffle`.
//!
//! Then `edge` and `asymmetry` lengthen the margins' rest lengths beyond the
//! pose, so the margins must ruffle (the lily mechanism), and thickness drops
//! as area grows (`thinning`). Missing channels take the neutral value in
//! brackets: `length` [1], `width` [1], `memory` [1], `reflex` [0 rad],
//! `wrap` [1], `cupping` [0 rad], `ruffle` [1], `edge` [0], `asymmetry` [0],
//! `thinning` [0.6]. Every other channel passes through untouched.

use super::channels::Channels;
use crate::math;

/// Rest shape for one petal at one step.
pub struct Targets {
    /// Node positions the growth asks for.
    pub pose: Vec<[f64; 3]>,
    /// Rest length per spring, in the order of `Petal::springs`.
    pub rest: Vec<f64>,
    /// Thickness per node.
    pub thick: Vec<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Along,
    Across,
    Diagonal,
}

/// Rest data of one petal, taken from the bud.
#[derive(Clone, Debug)]
pub struct Petal {
    pub nu: usize,
    pub nv: usize,
    /// (a, b, direction), node indices within the petal.
    pub springs: Vec<(usize, usize, Dir)>,
    pub thick0: Vec<f64>,
    /// Laid sheet, per row: distance from the axis and height of the centre.
    rho: Vec<f64>,
    z: Vec<f64>,
    /// Laid sheet, per node: angle around the axis, relative to the row's centre.
    dtheta: Vec<f64>,
    /// Angle of each row's centre.
    theta_c: Vec<f64>,
    /// Packed minus laid, per node, in the laid local frame (normal, across, along).
    crumple: Vec<[f64; 3]>,
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn unit(a: [f64; 3]) -> [f64; 3] {
    let l = math::sqrt(dot(a, a)).max(1e-12);
    [a[0] / l, a[1] / l, a[2] / l]
}

fn wrap_pi(a: f64) -> f64 {
    let t = std::f64::consts::TAU;
    (a + std::f64::consts::PI).rem_euclid(t) - std::f64::consts::PI
}

/// Orthonormal frame per node (normal, across, along) of a petal grid,
/// normal times `sign`.
fn frames(x: &[[f64; 3]], nu: usize, nv: usize, sign: f64) -> Vec<[[f64; 3]; 3]> {
    let mut out = Vec::with_capacity(nu * nv);
    for v in 0..nv {
        for u in 0..nu {
            let (u0, u1) = (u.saturating_sub(1), (u + 1).min(nu - 1));
            let (v0, v1) = (v.saturating_sub(1), (v + 1).min(nv - 1));
            let du = sub(x[v * nu + u1], x[v * nu + u0]);
            let dv = unit(sub(x[v1 * nu + u], x[v0 * nu + u]));
            let n = unit(cross(du, dv));
            let n = [n[0] * sign, n[1] * sign, n[2] * sign];
            let across = unit(cross(dv, n));
            out.push([n, across, dv]);
        }
    }
    out
}

/// +1 or -1 so that frame normals point away from the axis on average.
fn sign_of(x: &[[f64; 3]], nu: usize, nv: usize) -> f64 {
    let f = frames(x, nu, nv, 1.0);
    let s: f64 = (0..nu * nv).map(|i| dot(f[i][0], [x[i][0], x[i][1], 0.0])).sum();
    if s < 0.0 {
        -1.0
    } else {
        1.0
    }
}

fn ramp(x: f64, w: f64) -> f64 {
    (x / w).clamp(0.0, 1.0)
}

/// Where the base fold sits along the petal.
fn hinge(fv: f64) -> f64 {
    let x = (fv - 0.12) / 0.1;
    math::exp(-x * x)
}

impl Petal {
    /// Rest data from a packed petal `x` and the same petal as first laid on
    /// its shell, `laid` (both v-major: index v * nu + u, v = 0 at the base;
    /// every laid row sits at one height).
    pub fn from_bud(x: &[[f64; 3]], laid: &[[f64; 3]], thick: &[f64], nu: usize, nv: usize) -> Petal {
        let id = |u: usize, v: usize| v * nu + u;
        let mut springs = Vec::new();
        for v in 0..nv {
            for u in 0..nu {
                if u + 1 < nu {
                    springs.push((id(u, v), id(u + 1, v), Dir::Across));
                }
                if v + 1 < nv {
                    springs.push((id(u, v), id(u, v + 1), Dir::Along));
                }
                if u + 1 < nu && v + 1 < nv {
                    springs.push((id(u, v), id(u + 1, v + 1), Dir::Diagonal));
                    springs.push((id(u + 1, v), id(u, v + 1), Dir::Diagonal));
                }
            }
        }
        let c = nu / 2;
        let at = |v: usize| laid[id(c, v)];
        let rho = (0..nv).map(|v| math::sqrt(at(v)[0] * at(v)[0] + at(v)[1] * at(v)[1])).collect();
        let z = (0..nv).map(|v| at(v)[2]).collect();
        let theta_c: Vec<f64> = (0..nv).map(|v| math::atan2(at(v)[1], at(v)[0])).collect();
        let dtheta = (0..nu * nv).map(|i| wrap_pi(math::atan2(laid[i][1], laid[i][0]) - theta_c[i / nu])).collect();
        let fr = frames(laid, nu, nv, sign_of(laid, nu, nv));
        let crumple = (0..nu * nv)
            .map(|i| {
                let d = sub(x[i], laid[i]);
                [dot(d, fr[i][0]), dot(d, fr[i][1]), dot(d, fr[i][2])]
            })
            .collect();
        Petal { nu, nv, springs, thick0: thick.to_vec(), rho, z, dtheta, theta_c, crumple }
    }

    pub fn coords(&self, i: usize) -> (f64, f64) {
        let (u, v) = (i % self.nu, i / self.nu);
        (u as f64 / (self.nu - 1) as f64, v as f64 / (self.nv - 1) as f64)
    }

    /// The pose and rest shape the channels ask for.
    pub fn targets(&self, ch: &Channels, p: usize) -> Targets {
        let g = |n: &str, d: f64| ch.scalar(n, p, d);
        let (l, w, mem, refl) = (g("length", 1.0), g("width", 1.0), g("memory", 1.0), g("reflex", 0.0));
        let (wrap, cup, ruf) = (g("wrap", 1.0), g("cupping", 0.0), g("ruffle", 1.0));
        let (edge, asym, beta) = (g("edge", 0.0), g("asymmetry", 0.0), g("thinning", 0.6));
        let (nu, nv) = (self.nu, self.nv);
        let fv = |v: usize| v as f64 / (nv - 1) as f64;
        let len_at = |f: f64| 1.0 + (l - 1.0) * ramp(f, 0.15);
        let wid_at = |f: f64| 1.0 + (w - 1.0) * ramp(f, 0.25);

        // 1. Centre curve in the meridian plane.
        let seg: Vec<f64> = (0..nv - 1)
            .map(|v| {
                let (dr, dz) = (self.rho[v + 1] - self.rho[v], self.z[v + 1] - self.z[v]);
                math::sqrt(dr * dr + dz * dz)
            })
            .collect();
        let psi: Vec<f64> = (0..nv - 1).map(|v| math::atan2(self.z[v + 1] - self.z[v], self.rho[v + 1] - self.rho[v])).collect();
        let hsum: f64 = (1..nv - 1).map(|v| hinge(fv(v))).sum::<f64>().max(1e-9);
        let (mut rho, mut z) = (vec![self.rho[0]], vec![self.z[0]]);
        let mut ang = psi[0];
        for v in 0..nv - 1 {
            if v > 0 {
                ang += mem * wrap_pi(psi[v] - psi[v - 1]) - refl * hinge(fv(v)) / hsum;
            }
            let s = seg[v] * len_at(0.5 * (fv(v) + fv(v + 1)));
            rho.push(rho[v] + s * math::cos(ang));
            z.push(z[v] + s * math::sin(ang));
        }

        // 2. Rows: arc length times width, at the new distance; wrap and cup.
        let mut pose = Vec::with_capacity(nu * nv);
        for v in 0..nv {
            let (tc, r) = (self.theta_c[v], rho[v].max(1e-6));
            let wv = wid_at(fv(v));
            let half = (0..nu).map(|u| (self.dtheta[v * nu + u] * self.rho[v]).abs()).fold(0.0, f64::max).max(1e-9) * wv;
            let (ct, st) = (math::cos(tc), math::sin(tc));
            // Cupping: total turning `cup` across the row, toward the axis.
            let k = cup * ramp(fv(v), 0.2) / (2.0 * half);
            for u in 0..nu {
                let arc = self.dtheta[v * nu + u] * self.rho[v] * wv;
                let a = arc / r;
                let on_arc = [r * math::cos(tc + a), r * math::sin(tc + a)];
                let flat = [r * ct - arc * st, r * st + arc * ct];
                let dip = 0.5 * k * arc * arc;
                pose.push([
                    wrap * on_arc[0] + (1.0 - wrap) * flat[0] - dip * ct,
                    wrap * on_arc[1] + (1.0 - wrap) * flat[1] - dip * st,
                    z[v],
                ]);
            }
        }

        // 3. Crumples in the new local frame.
        let fr = frames(&pose, nu, nv, sign_of(&pose, nu, nv));
        for (i, q) in pose.iter_mut().enumerate() {
            let d = self.crumple[i];
            for c in 0..3 {
                q[c] += ruf * (d[0] * fr[i][0][c] + d[1] * fr[i][1][c] + d[2] * fr[i][2][c]);
            }
        }

        // Margins grow beyond the pose along the petal.
        let along = |fu: f64, f: f64| {
            let m = (2.0 * fu - 1.0) * (2.0 * fu - 1.0);
            (1.0 + edge * m * f) * (1.0 + asym * (2.0 * fu - 1.0) * f)
        };
        let rest = self
            .springs
            .iter()
            .map(|&(a, b, dir)| {
                let d = sub(pose[a], pose[b]);
                let len = math::sqrt(dot(d, d));
                let (ua, va) = self.coords(a);
                let (ub, vb) = self.coords(b);
                let e = along(0.5 * (ua + ub), 0.5 * (va + vb));
                len * match dir {
                    Dir::Along => e,
                    Dir::Across => 1.0,
                    Dir::Diagonal => math::sqrt(0.5 * (1.0 + e * e)),
                }
            })
            .collect();

        let thick = (0..nu * nv)
            .map(|i| {
                let (fu, f) = self.coords(i);
                let area = len_at(f) * wid_at(f) * along(fu, f);
                self.thick0[i] * math::pow(area.max(1.0), -beta)
            })
            .collect();
        Targets { pose, rest, thick }
    }
}
