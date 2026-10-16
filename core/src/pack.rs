//! Bud packing: grow each sheet past the room it has, then relax the energy.
//!
//! Rest lengths are the sheet's laid-out lengths times the plant's
//! growth_factor, so a petal wants more area than its slot allows and must
//! buckle. A small plant-seeded nudge along the shell normal breaks symmetry
//! so buckling picks a direction. Relaxation uses FIRE (gradient descent with
//! adaptive inertia). The base row of every petal is pinned to the receptacle.

use crate::energy::{Params, Spring, System, Terms};
use crate::math;
use crate::rng::Pcg32;
use crate::sheet::{idx, Sheet, NU, NV};
use crate::shells::Shells;
use crate::traits::PlantTraits;

#[derive(Clone, Debug)]
pub struct PackSettings {
    pub iterations: usize,
    pub dt_max: f64,
}

impl Default for PackSettings {
    fn default() -> Self {
        PackSettings { iterations: 2000, dt_max: 0.03 }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PackReport {
    pub petals: usize,
    pub nodes: usize,
    pub iterations: usize,
    pub contact_rebuilds: usize,
    pub start: Terms,
    pub end: Terms,
    /// Worst overlap of two petals' thickness, in units of max_thickness.
    pub max_overlap: f64,
    /// Worst excursion outside a layer slot, in units of max_thickness.
    pub max_band_excess: f64,
    pub max_thickness: f64,
}

pub fn build_system(sheets: &[Sheet], shells: &Shells, t: &PlantTraits) -> (System, Vec<[f64; 3]>, Vec<bool>) {
    let mut x = Vec::new();
    let mut thick = Vec::new();
    let mut layer = Vec::new();
    let mut petal = Vec::new();
    let mut pinned = Vec::new();
    let mut springs = Vec::new();
    let mut tris = Vec::new();
    for (si, s) in sheets.iter().enumerate() {
        let o = x.len();
        for v in 0..NV - 1 {
            for u in 0..NU - 1 {
                let (a, b, c, d) = (o + idx(u, v), o + idx(u + 1, v), o + idx(u, v + 1), o + idx(u + 1, v + 1));
                tris.push([a, b, d]);
                tris.push([a, d, c]);
            }
        }
        x.extend_from_slice(&s.pos);
        thick.extend_from_slice(&s.thick);
        layer.extend(std::iter::repeat(s.layer).take(NU * NV));
        petal.extend(std::iter::repeat(si).take(NU * NV));
        pinned.extend((0..NU * NV).map(|i| i < NU));
        let tmean = s.thick.iter().sum::<f64>() / s.thick.len() as f64;
        let tr = tmean / t.max_thickness;
        let k_bend = 6.0 * tr * tr * tr;
        let mut add = |a: usize, b: usize, k: f64| {
            let d = [s.pos[a][0] - s.pos[b][0], s.pos[a][1] - s.pos[b][1], s.pos[a][2] - s.pos[b][2]];
            let l = math::sqrt(d[0] * d[0] + d[1] * d[1] + d[2] * d[2]);
            springs.push(Spring { a: o + a, b: o + b, rest: l * t.growth_factor, k });
        };
        for v in 0..NV {
            for u in 0..NU {
                if u + 1 < NU {
                    add(idx(u, v), idx(u + 1, v), 40.0);
                }
                if v + 1 < NV {
                    add(idx(u, v), idx(u, v + 1), 40.0);
                }
                if u + 1 < NU && v + 1 < NV {
                    add(idx(u, v), idx(u + 1, v + 1), 10.0);
                    add(idx(u + 1, v), idx(u, v + 1), 10.0);
                }
                if u + 2 < NU {
                    add(idx(u, v), idx(u + 2, v), k_bend);
                }
                if v + 2 < NV {
                    add(idx(u, v), idx(u, v + 2), k_bend);
                }
            }
        }
    }
    let sys = System { thick, layer, petal, springs, tris, shells: shells.clone(), params: Params::default() };
    (sys, x, pinned)
}

pub fn pack(sheets: &mut [Sheet], shells: &Shells, t: &PlantTraits, settings: &PackSettings, rng: &mut Pcg32) -> PackReport {
    let (sys, mut x, pinned) = build_system(sheets, shells, t);
    let n = x.len();

    for i in 0..n {
        if pinned[i] {
            continue;
        }
        let v = (i % (NU * NV)) / NU;
        let g = shells.grad_scaled_radius(x[i]);
        let gn = math::sqrt(g[0] * g[0] + g[1] * g[1] + g[2] * g[2]);
        let a = rng.normal() * 0.1 * shells.slot * v as f64 / (NV - 1) as f64;
        for c in 0..3 {
            x[i][c] += a * g[c] / gn;
        }
    }

    // Verlet-style contact list: candidates within thickness + skin are
    // rebuilt once any node has moved half the skin since the last build, so
    // no contact can be missed in between.
    let max_step = 0.25 * t.max_thickness;
    let skin = 2.0 * t.max_thickness;
    let reach = t.max_thickness + skin;
    let mut cands = sys.candidates(&x, reach);
    let mut anchor = x.clone();
    let mut rebuilds = 1usize;
    let mut grad = vec![[0.0; 3]; n];
    let start = sys.energy(&x, &cands, &mut grad);
    let mut vel = vec![[0.0f64; 3]; n];
    let (mut dt, mut alpha, mut npos) = (settings.dt_max * 0.5, 0.1, 0usize);
    for _ in 0..settings.iterations {
        let moved = x.iter().zip(&anchor).map(|(p, q)| {
            let d = [p[0] - q[0], p[1] - q[1], p[2] - q[2]];
            d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
        });
        if math::sqrt(moved.fold(0.0, f64::max)) * 2.0 > skin {
            cands = sys.candidates(&x, reach);
            anchor.copy_from_slice(&x);
            rebuilds += 1;
        }
        sys.energy(&x, &cands, &mut grad);
        for i in 0..n {
            if pinned[i] {
                grad[i] = [0.0; 3];
            }
        }
        let (mut pw, mut vn, mut fnn) = (0.0, 0.0, 0.0);
        for i in 0..n {
            for c in 0..3 {
                pw -= grad[i][c] * vel[i][c];
                vn += vel[i][c] * vel[i][c];
                fnn += grad[i][c] * grad[i][c];
            }
        }
        if pw > 0.0 {
            let (vn, fnn) = (math::sqrt(vn), math::sqrt(fnn).max(1e-12));
            for i in 0..n {
                for c in 0..3 {
                    vel[i][c] = (1.0 - alpha) * vel[i][c] - alpha * vn * grad[i][c] / fnn;
                }
            }
            npos += 1;
            if npos > 5 {
                dt = (dt * 1.1).min(settings.dt_max);
                alpha *= 0.99;
            }
        } else {
            npos = 0;
            dt *= 0.5;
            alpha = 0.1;
            vel.iter_mut().for_each(|v| *v = [0.0; 3]);
        }
        for i in 0..n {
            if pinned[i] {
                continue;
            }
            let mut step = [0.0; 3];
            for c in 0..3 {
                vel[i][c] -= dt * grad[i][c];
                step[c] = dt * vel[i][c];
            }
            let l = math::sqrt(step[0] * step[0] + step[1] * step[1] + step[2] * step[2]);
            let s = if l > max_step { max_step / l } else { 1.0 };
            for c in 0..3 {
                x[i][c] += s * step[c];
            }
        }
    }
    cands = sys.candidates(&x, reach);
    let end = sys.energy(&x, &cands, &mut grad);

    let max_overlap = sys.contacts(&x, &cands).iter().map(|c| c.dmin - c.d).fold(0.0, f64::max) / t.max_thickness;
    let max_band_excess = (0..n).map(|i| sys.band_excess(i, x[i]).0).fold(0.0, f64::max) / t.max_thickness;
    for (si, s) in sheets.iter_mut().enumerate() {
        s.pos.copy_from_slice(&x[si * NU * NV..(si + 1) * NU * NV]);
    }
    PackReport {
        petals: sheets.len(),
        nodes: n,
        iterations: settings.iterations,
        contact_rebuilds: rebuilds,
        start,
        end,
        max_overlap,
        max_band_excess,
        max_thickness: sheets.iter().flat_map(|s| s.thick.iter()).cloned().fold(0.0, f64::max),
    }
}
