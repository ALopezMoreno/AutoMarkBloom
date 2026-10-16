//! Bud development: grow the petals stage by stage and keep them packed.
//!
//! Each stage:
//! 1. the growth automaton (`growth`) updates every petal's per-node growth,
//!    using the contact pressure each node felt in the previous stage;
//! 2. every spring's rest length becomes its laid-out length times the mean
//!    growth of its two ends, so uneven growth must ruffle or crumple;
//! 3. the sepals' rest size grows a little;
//! 4. the energy is relaxed with FIRE (gradient descent with adaptive
//!    inertia) over all node positions plus the envelope radius, so petal
//!    pressure swells the bud.
//!
//! A small flower-seeded nudge along the shell normal breaks symmetry at the
//! start. The base row of every petal is pinned to the receptacle.

use crate::energy::{Candidate, Envelope, Params, Spring, System, Terms};
use crate::growth::Growth;
use crate::math;
use crate::rng::Pcg32;
use crate::sheet::{idx, Sheet, NU, NV};
use crate::shells::Shells;
use crate::traits::PlantTraits;

#[derive(Clone, Debug)]
pub struct PackSettings {
    /// FIRE iterations after each growth stage.
    pub relax_iterations: usize,
    /// Extra iterations after the last stage.
    pub settle_iterations: usize,
    pub dt_max: f64,
    /// Keep every stage's geometry (for the swelling animation).
    pub record_frames: bool,
}

impl Default for PackSettings {
    fn default() -> Self {
        PackSettings { relax_iterations: 250, settle_iterations: 600, dt_max: 0.03, record_frames: false }
    }
}

impl PackSettings {
    /// Much shorter relaxation, for tests.
    pub fn quick() -> Self {
        PackSettings { relax_iterations: 15, settle_iterations: 15, ..Self::default() }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PackReport {
    pub petals: usize,
    pub nodes: usize,
    pub stages: usize,
    pub iterations: usize,
    pub contact_rebuilds: usize,
    pub start: Terms,
    pub end: Terms,
    /// Worst overlap of two petals' thickness, in units of max_thickness.
    pub max_overlap: f64,
    /// Worst excursion outside a layer's allowed band, in units of max_thickness.
    pub max_band_excess: f64,
    pub max_thickness: f64,
    /// Sepal envelope scaled radius at the start and end of the bud stage.
    pub envelope_start: f64,
    pub envelope_end: f64,
    /// Mean and largest accumulated linear growth of petal nodes.
    pub mean_growth: f64,
    pub max_growth: f64,
}

/// Geometry after one growth stage.
#[derive(Clone, Debug)]
pub struct StageFrame {
    pub stage: usize,
    /// Control points of each sheet.
    pub pos: Vec<Vec<[f64; 3]>>,
    pub envelope: f64,
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
        layer.extend(std::iter::repeat_n(s.layer, NU * NV));
        petal.extend(std::iter::repeat_n(si, NU * NV));
        pinned.extend((0..NU * NV).map(|i| i < NU));
        let mut add = |a: usize, b: usize, k: f64| {
            let d = [s.pos[a][0] - s.pos[b][0], s.pos[a][1] - s.pos[b][1], s.pos[a][2] - s.pos[b][2]];
            let l = math::sqrt(d[0] * d[0] + d[1] * d[1] + d[2] * d[2]);
            springs.push(Spring { a: o + a, b: o + b, rest: l, rest0: l, k });
        };
        for v in 0..NV {
            for u in 0..NU {
                let tr = s.thick[idx(u, v)] / t.max_thickness;
                let k_bend = t.petal_stiffness * tr * tr * tr;
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
    let params = Params { k_sepal: t.sepal_stiffness, layer_freedom: t.layer_freedom, ..Params::default() };
    let sys = System { thick, layer, petal, springs, tris, shells: shells.clone(), params };
    (sys, x, pinned)
}

/// FIRE relaxation over node positions and the envelope radius, with a
/// Verlet-style contact list rebuilt once any node has moved half the skin.
struct Relaxer {
    max_step: f64,
    skin: f64,
    reach: f64,
    cands: Vec<Candidate>,
    anchor: Vec<[f64; 3]>,
    rebuilds: usize,
    iterations: usize,
}

impl Relaxer {
    fn new(sys: &System, x: &[[f64; 3]], t: &PlantTraits) -> Self {
        let skin = 2.0 * t.max_thickness;
        let reach = t.max_thickness + skin;
        Relaxer {
            max_step: 0.25 * t.max_thickness,
            skin,
            reach,
            cands: sys.candidates(x, reach),
            anchor: x.to_vec(),
            rebuilds: 1,
            iterations: 0,
        }
    }

    fn refresh(&mut self, sys: &System, x: &[[f64; 3]], force: bool) {
        let moved = x.iter().zip(&self.anchor).map(|(p, q)| {
            let d = [p[0] - q[0], p[1] - q[1], p[2] - q[2]];
            d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
        });
        if force || math::sqrt(moved.fold(0.0, f64::max)) * 2.0 > self.skin {
            self.cands = sys.candidates(x, self.reach);
            self.anchor.copy_from_slice(x);
            self.rebuilds += 1;
        }
    }

    fn run(&mut self, sys: &System, x: &mut [[f64; 3]], env: &mut Envelope, pinned: &[bool], iters: usize, dt_max: f64) {
        let n = x.len();
        let mut grad = vec![[0.0; 3]; n];
        let mut genv = 0.0;
        let mut vel = vec![[0.0f64; 3]; n];
        let mut venv = 0.0;
        let (mut dt, mut alpha, mut npos) = (dt_max * 0.5, 0.1, 0usize);
        for _ in 0..iters {
            self.refresh(sys, x, false);
            sys.energy(x, *env, &self.cands, &mut grad, &mut genv);
            for i in 0..n {
                if pinned[i] {
                    grad[i] = [0.0; 3];
                }
            }
            let (mut pw, mut vn, mut fnn) = (-genv * venv, venv * venv, genv * genv);
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
                venv = (1.0 - alpha) * venv - alpha * vn * genv / fnn;
                npos += 1;
                if npos > 5 {
                    dt = (dt * 1.1).min(dt_max);
                    alpha *= 0.99;
                }
            } else {
                npos = 0;
                dt *= 0.5;
                alpha = 0.1;
                vel.iter_mut().for_each(|v| *v = [0.0; 3]);
                venv = 0.0;
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
                let s = if l > self.max_step { self.max_step / l } else { 1.0 };
                for c in 0..3 {
                    x[i][c] += s * step[c];
                }
            }
            venv -= dt * genv;
            env.r += (dt * venv).clamp(-self.max_step, self.max_step);
            self.iterations += 1;
        }
    }
}

fn snapshot(x: &[[f64; 3]], sheets: usize) -> Vec<Vec<[f64; 3]>> {
    (0..sheets).map(|s| x[s * NU * NV..(s + 1) * NU * NV].to_vec()).collect()
}

pub fn develop(
    sheets: &mut [Sheet],
    shells: &Shells,
    t: &PlantTraits,
    settings: &PackSettings,
    rng_pack: &mut Pcg32,
    rng_growth: &mut Pcg32,
) -> (PackReport, Vec<StageFrame>, Vec<Growth>) {
    let (mut sys, mut x, pinned) = build_system(sheets, shells, t);
    let n = x.len();

    for i in 0..n {
        if pinned[i] {
            continue;
        }
        let v = (i % (NU * NV)) / NU;
        let g = shells.grad_scaled_radius(x[i]);
        let gn = math::sqrt(g[0] * g[0] + g[1] * g[1] + g[2] * g[2]);
        let a = rng_pack.normal() * 0.1 * shells.slot * v as f64 / (NV - 1) as f64;
        for c in 0..3 {
            x[i][c] += a * g[c] / gn;
        }
    }

    let mut env = Envelope { r: shells.envelope, rest: shells.envelope };
    let mut relax = Relaxer::new(&sys, &x, t);
    let mut scratch = vec![[0.0; 3]; n];
    let mut genv = 0.0;
    let start = sys.energy(&x, env, &relax.cands, &mut scratch, &mut genv);
    relax.run(&sys, &mut x, &mut env, &pinned, settings.relax_iterations, settings.dt_max);

    let mut frames = Vec::new();
    if settings.record_frames {
        frames.push(StageFrame { stage: 0, pos: snapshot(&x, sheets.len()), envelope: env.r });
    }
    let mut growth = vec![Growth::default(); sheets.len()];
    let p_ref = sys.params.k_contact * 0.25 * t.max_thickness;
    for stage in 1..=t.growth_stages {
        relax.refresh(&sys, &x, true);
        let pressure = sys.pressure(&x, env, &relax.cands);
        let mut scale = Vec::with_capacity(n);
        for (si, g) in growth.iter_mut().enumerate() {
            let p: Vec<f64> = pressure[si * NU * NV..(si + 1) * NU * NV].iter().map(|f| f / p_ref).collect();
            g.step(&p, t, rng_growth);
            scale.extend_from_slice(&g.scale);
        }
        for s in sys.springs.iter_mut() {
            s.rest = s.rest0 * 0.5 * (scale[s.a] + scale[s.b]);
        }
        env.rest = shells.envelope * (1.0 + t.sepal_growth * stage as f64 / t.growth_stages as f64);
        relax.run(&sys, &mut x, &mut env, &pinned, settings.relax_iterations, settings.dt_max);
        if settings.record_frames {
            frames.push(StageFrame { stage, pos: snapshot(&x, sheets.len()), envelope: env.r });
        }
    }
    relax.run(&sys, &mut x, &mut env, &pinned, settings.settle_iterations, settings.dt_max);
    if let Some(last) = frames.last_mut() {
        last.pos = snapshot(&x, sheets.len());
        last.envelope = env.r;
    }

    relax.refresh(&sys, &x, true);
    let end = sys.energy(&x, env, &relax.cands, &mut scratch, &mut genv);
    let max_overlap = sys.contacts(&x, &relax.cands).iter().map(|c| c.dmin - c.d).fold(0.0, f64::max) / t.max_thickness;
    let max_band_excess = (0..n).map(|i| sys.band_excess(i, x[i], env.r).0).fold(0.0, f64::max) / t.max_thickness;
    for (si, s) in sheets.iter_mut().enumerate() {
        s.pos.copy_from_slice(&x[si * NU * NV..(si + 1) * NU * NV]);
    }
    let all: Vec<f64> = growth.iter().flat_map(|g| g.scale.iter().cloned()).collect();
    let report = PackReport {
        petals: sheets.len(),
        nodes: n,
        stages: t.growth_stages,
        iterations: relax.iterations,
        contact_rebuilds: relax.rebuilds,
        start,
        end,
        max_overlap,
        max_band_excess,
        max_thickness: sheets.iter().flat_map(|s| s.thick.iter()).cloned().fold(0.0, f64::max),
        envelope_start: shells.envelope,
        envelope_end: env.r,
        mean_growth: all.iter().sum::<f64>() / all.len().max(1) as f64,
        max_growth: all.iter().cloned().fold(0.0, f64::max),
    };
    (report, frames, growth)
}
