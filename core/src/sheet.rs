//! A petal as a thick sheet: an NU x NV grid of spline control points along
//! the petal's mid-surface, plus a thickness at each point.
//!
//! The sheet is laid onto the petal's CA patch on its shell: each row follows
//! the patch's center and span at that height, smoothed and given a narrow
//! claw at the base and a rounded tip. Thickness is thickest at the base and
//! midline and never exceeds the plant's max_thickness.

use crate::ca::{PetalPatch, NP};
use crate::math;
use crate::shells::Shells;
use crate::traits::PlantTraits;
use std::f64::consts::{PI, TAU};

pub const NU: usize = 11;
pub const NV: usize = 19;

#[derive(Clone, Debug)]
pub struct Sheet {
    pub petal: usize,
    pub layer: usize,
    /// Control points, index v * NU + u (v = 0 is the attached base).
    pub pos: Vec<[f64; 3]>,
    pub thick: Vec<f64>,
}

pub fn idx(u: usize, v: usize) -> usize {
    v * NU + u
}

fn wrap_pi(a: f64) -> f64 {
    (a + PI).rem_euclid(TAU) - PI
}

fn smooth(v: &mut [f64]) {
    for _ in 0..2 {
        let old = v.to_vec();
        for i in 1..v.len().saturating_sub(1) {
            v[i] = 0.25 * old[i - 1] + 0.5 * old[i] + 0.25 * old[i + 1];
        }
    }
}

impl Sheet {
    pub fn from_patch(p: &PetalPatch, shells: &Shells, t: &PlantTraits) -> Option<Sheet> {
        let last = p.rows.iter().rposition(|r| r.is_some())?;
        let h = last + 1;
        if h < 3 {
            return None;
        }
        // Per-row center (unwrapped around the base angle) and span, gaps interpolated.
        let known: Vec<(usize, f64, f64)> = p.rows[..h]
            .iter()
            .enumerate()
            .filter_map(|(r, x)| x.map(|(c, s)| (r, p.base_angle + wrap_pi(c - p.base_angle), s)))
            .collect();
        let mut center = vec![0.0; h];
        let mut span = vec![0.0; h];
        for r in 0..h {
            let j = known.partition_point(|k| k.0 < r);
            let (c, s) = if j < known.len() && known[j].0 == r {
                (known[j].1, known[j].2)
            } else if j == 0 {
                (known[0].1, known[0].2)
            } else if j == known.len() {
                (known[j - 1].1, known[j - 1].2)
            } else {
                let (a, b) = (known[j - 1], known[j]);
                let f = (r - a.0) as f64 / (b.0 - a.0) as f64;
                (a.1 + f * (b.1 - a.1), a.2 + f * (b.2 - a.2))
            };
            center[r] = c;
            span[r] = s;
        }
        smooth(&mut center);
        smooth(&mut span);

        let a = shells.radius[p.layer];
        let mut pos = Vec::with_capacity(NU * NV);
        let mut thick = Vec::with_capacity(NU * NV);
        for v in 0..NV {
            let fv = v as f64 / (NV - 1) as f64;
            let rr = fv * (h - 1) as f64;
            let r0 = math::floor(rr) as usize;
            let r1 = (r0 + 1).min(h - 1);
            let f = rr - r0 as f64;
            let c = center[r0] + f * (center[r1] - center[r0]);
            let taper = (0.5 + 0.5 * (fv / 0.2).min(1.0)) * (0.3 + 0.7 * math::sqrt((1.0 - fv * fv * fv * fv).max(0.0)));
            let sp = (span[r0] + f * (span[r1] - span[r0])) * 0.92 * taper;
            let phi = shells.phi_at((rr + 0.5) / NP as f64);
            for u in 0..NU {
                let fu = u as f64 / (NU - 1) as f64;
                pos.push(shells.point(a, c + (fu - 0.5) * sp, phi));
                let body = math::pow(1.0 - fv, 1.2) * (1.0 - 0.6 * (2.0 * fu - 1.0) * (2.0 * fu - 1.0));
                thick.push(t.max_thickness * (t.edge_thickness + (1.0 - t.edge_thickness) * body));
            }
        }
        Some(Sheet { petal: p.id, layer: p.layer, pos, thick })
    }
}

fn catmull_rom(p0: [f64; 3], p1: [f64; 3], p2: [f64; 3], p3: [f64; 3], t: f64) -> [f64; 3] {
    let (t2, t3) = (t * t, t * t * t);
    let mut out = [0.0; 3];
    for c in 0..3 {
        out[c] = 0.5
            * (2.0 * p1[c]
                + (p2[c] - p0[c]) * t
                + (2.0 * p0[c] - 5.0 * p1[c] + 4.0 * p2[c] - p3[c]) * t2
                + (3.0 * p1[c] - p0[c] - 3.0 * p2[c] + p3[c]) * t3);
    }
    out
}

/// Interpolating spline through `pts`, `s` samples per span, ends extrapolated.
fn refine(pts: &[[f64; 3]], s: usize) -> Vec<[f64; 3]> {
    let n = pts.len();
    let get = |i: isize| -> [f64; 3] {
        if i < 0 {
            let (a, b) = (pts[0], pts[1]);
            [2.0 * a[0] - b[0], 2.0 * a[1] - b[1], 2.0 * a[2] - b[2]]
        } else if i as usize >= n {
            let (a, b) = (pts[n - 1], pts[n - 2]);
            [2.0 * a[0] - b[0], 2.0 * a[1] - b[1], 2.0 * a[2] - b[2]]
        } else {
            pts[i as usize]
        }
    };
    let mut out = Vec::with_capacity((n - 1) * s + 1);
    for i in 0..n - 1 {
        let i = i as isize;
        for k in 0..s {
            out.push(catmull_rom(get(i - 1), get(i), get(i + 1), get(i + 2), k as f64 / s as f64));
        }
    }
    out.push(pts[n - 1]);
    out
}

fn refine_scalar(v: &[f64], s: usize) -> Vec<f64> {
    let mut out = Vec::with_capacity((v.len() - 1) * s + 1);
    for i in 0..v.len() - 1 {
        for k in 0..s {
            let f = k as f64 / s as f64;
            out.push(v[i] + f * (v[i + 1] - v[i]));
        }
    }
    out.push(v[v.len() - 1]);
    out
}

impl Sheet {
    /// Control grid size: (across, along) = (NU, NV). Use this rather than
    /// hard-coding the constants.
    pub fn grid(&self) -> (usize, usize) {
        (NU, NV)
    }

    pub fn along(sa: usize) -> usize {
        (NV - 1) * sa + 1
    }

    pub fn across(sc: usize) -> usize {
        (NU - 1) * sc + 1
    }

    /// Evaluate the spline sheet on a finer grid: `sa` samples per span along
    /// the petal and `sc` across. Returns points and thicknesses, row-major
    /// with rows running base -> tip and columns edge -> edge.
    pub fn evaluate(&self, sa: usize, sc: usize) -> (Vec<[f64; 3]>, Vec<f64>) {
        let rows_pts: Vec<Vec<[f64; 3]>> = (0..NV).map(|v| refine(&self.pos[v * NU..(v + 1) * NU], sc)).collect();
        let rows_t: Vec<Vec<f64>> = (0..NV).map(|v| refine_scalar(&self.thick[v * NU..(v + 1) * NU], sc)).collect();
        let nc = Self::across(sc);
        let na = Self::along(sa);
        let mut pts = vec![[0.0; 3]; na * nc];
        let mut th = vec![0.0; na * nc];
        for c in 0..nc {
            let col: Vec<[f64; 3]> = rows_pts.iter().map(|r| r[c]).collect();
            let colt: Vec<f64> = rows_t.iter().map(|r| r[c]).collect();
            for (a, (p, t)) in refine(&col, sa).into_iter().zip(refine_scalar(&colt, sa)).enumerate() {
                pts[a * nc + c] = p;
                th[a * nc + c] = t;
            }
        }
        (pts, th)
    }
}
