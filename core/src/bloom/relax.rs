//! Per-step relaxation of the opening flower.
//!
//! E = stretch springs toward the consumer's rest lengths
//!   + bending: each grid triple's second difference toward the pose's
//!     (stiffness ~ t^3 of the bud petal)
//!   + pose: a soft pull of every node toward its pose
//!   + contact between petals (`contact`: dense samples against triangles)
//!   + core (petals stay outside the stamens)
//!
//! The sepals have opened, so there is no envelope or layer band: petals
//! leave their bud slots, growth sets the pose and contact settles the rest.
//! Every term is quadratic or a squared hinge with an exact gradient, and is
//! minimised with FIRE as in the bud stage. Each step starts from the last
//! frame moved by how much the poses moved, so contact settles locally.
//! The base row of every petal is pinned to the receptacle.

use super::consume::{Dir, Petal, Targets};
use super::contact::Contacts;
use crate::math;
use crate::shells::Shells;

#[derive(Clone, Debug)]
pub struct RelaxSettings {
    pub iterations: usize,
    pub dt_max: f64,
    pub k_stretch: f64,
    pub k_shear: f64,
    pub k_bend: f64,
    pub k_pose: f64,
    pub k_contact: f64,
    pub k_core: f64,
    /// Contact samples per quad side.
    pub contact_samples: usize,
    /// Keep FIRE's velocity and step size from one settling step to the
    /// next, so the flower keeps moving as it grows instead of starting
    /// from rest every time.
    pub carry: bool,
}

impl Default for RelaxSettings {
    fn default() -> Self {
        RelaxSettings {
            iterations: 20,
            dt_max: 0.06,
            k_stretch: 40.0,
            k_shear: 10.0,
            k_bend: 20.0,
            k_pose: 2.0,
            k_contact: 1500.0,
            k_core: 400.0,
            contact_samples: 2,
            carry: true,
        }
    }
}

/// One packed bud petal handed to the opening.
#[derive(Clone, Debug)]
pub struct Grid {
    /// Packed control points, v-major (index v * nu + u, v = 0 at the base).
    pub pos: Vec<[f64; 3]>,
    /// The same petal as first laid on its shell, before packing.
    pub laid: Vec<[f64; 3]>,
    pub thick: Vec<f64>,
    pub nu: usize,
    pub nv: usize,
}

pub struct Flower {
    pub petals: Vec<Petal>,
    /// Start of each petal's nodes in `x`.
    pub offset: Vec<usize>,
    pub x: Vec<[f64; 3]>,
    pub thick: Vec<f64>,
    /// Owning petal of each node.
    pub owner: Vec<usize>,
    /// Concatenated poses of the last step.
    pub pose: Vec<[f64; 3]>,
    pub contacts: Contacts,
    /// Node positions at their petal's last contact refresh.
    anchor: Vec<[f64; 3]>,
    /// FIRE state carried between settling steps (see `RelaxSettings::carry`).
    fire: Option<(Vec<[f64; 3]>, f64, f64, usize)>,
    pub shells: Shells,
    pub cap: f64,
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

impl Flower {
    pub fn new(grids: &[Grid], shells: &Shells, s: &RelaxSettings) -> Flower {
        let mut petals = Vec::new();
        let mut offset = Vec::new();
        let (mut x, mut thick, mut owner) = (vec![], vec![], vec![]);
        for (p, g) in grids.iter().enumerate() {
            offset.push(x.len());
            petals.push(Petal::from_bud(&g.pos, &g.laid, &g.thick, g.nu, g.nv));
            x.extend_from_slice(&g.pos);
            thick.extend_from_slice(&g.thick);
            owner.extend(std::iter::repeat_n(p, g.pos.len()));
        }
        let cap = thick.iter().cloned().fold(0.0, f64::max);
        let layout: Vec<_> = grids.iter().zip(&offset).map(|(g, &o)| (o, g.nu, g.nv)).collect();
        let mut contacts = Contacts::new(&layout, s.contact_samples);
        contacts.rebuild(&x, Self::skin_reach(cap).1);
        let (pose, anchor) = (x.clone(), x.clone());
        Flower { petals, offset, x, thick, owner, pose, contacts, anchor, fire: None, shells: shells.clone(), cap }
    }

    /// Verlet skin and contact search reach for a thickness cap.
    fn skin_reach(cap: f64) -> (f64, f64) {
        let skin = cap;
        (skin, cap + skin)
    }

    /// Energy and gradient toward the given targets (`pose` concatenated).
    pub fn energy(&self, x: &[[f64; 3]], t: &[Targets], pose: &[[f64; 3]], s: &RelaxSettings, grad: &mut [[f64; 3]]) -> f64 {
        grad.iter_mut().for_each(|g| *g = [0.0; 3]);
        let mut e = 0.0;
        for (p, pt) in self.petals.iter().enumerate() {
            let o = self.offset[p];
            for (k, &(a, b, dir)) in pt.springs.iter().enumerate() {
                let kk = if dir == Dir::Diagonal { s.k_shear } else { s.k_stretch };
                let d = sub(x[o + a], x[o + b]);
                let l = norm(d).max(1e-12);
                let ex = l - t[p].rest[k];
                e += 0.5 * kk * ex * ex;
                let c = kk * ex / l;
                add_scaled(&mut grad[o + a], c, d);
                add_scaled(&mut grad[o + b], -c, d);
            }
            let (nu, nv) = (pt.nu, pt.nv);
            let mut bend = |a: usize, b: usize, c: usize| {
                let tb = pt.thick0[b - o] / self.cap;
                let kk = s.k_bend * tb * tb * tb;
                let mut r = [0.0; 3];
                for i in 0..3 {
                    r[i] = (x[a][i] - pose[a][i]) - 2.0 * (x[b][i] - pose[b][i]) + (x[c][i] - pose[c][i]);
                }
                e += 0.5 * kk * (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]);
                add_scaled(&mut grad[a], kk, r);
                add_scaled(&mut grad[b], -2.0 * kk, r);
                add_scaled(&mut grad[c], kk, r);
            };
            for v in 0..nv {
                for u in 0..nu {
                    let i = o + v * nu + u;
                    if u + 2 < nu {
                        bend(i, i + 1, i + 2);
                    }
                    if v + 2 < nv {
                        bend(i, i + nu, i + 2 * nu);
                    }
                }
            }
        }
        for (i, &xi) in x.iter().enumerate() {
            let d = sub(xi, pose[i]);
            e += 0.5 * s.k_pose * (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]);
            add_scaled(&mut grad[i], s.k_pose, d);
        }
        e += self.contacts.energy(x, &self.thick, s.k_contact, grad, &self.anchor);
        let sh = &self.shells;
        for (i, &xi) in x.iter().enumerate() {
            let inn = (sh.core + 0.5 * self.thick[i]) - sh.scaled_radius(xi);
            if inn > 0.0 {
                e += 0.5 * s.k_core * inn * inn;
                add_scaled(&mut grad[i], -s.k_core * inn, sh.grad_scaled_radius(xi));
            }
        }
        e
    }

    pub fn pinned(&self, i: usize) -> bool {
        let p = self.owner[i];
        i - self.offset[p] < self.petals[p].nu
    }

    /// Move with the new poses, then relax with FIRE. Returns the final energy.
    pub fn relax(&mut self, t: &[Targets], s: &RelaxSettings) -> f64 {
        let mut pose = Vec::with_capacity(self.x.len());
        for (p, tp) in t.iter().enumerate() {
            let o = self.offset[p];
            self.thick[o..o + tp.thick.len()].copy_from_slice(&tp.thick);
            pose.extend_from_slice(&tp.pose);
        }
        let n_nodes = self.x.len();
        let mut x = self.x.clone();
        for i in 0..n_nodes {
            let d = sub(pose[i], self.pose[i]);
            add_scaled(&mut x[i], 1.0, d);
        }
        self.pose = pose;
        let pose = &self.pose.clone();
        let max_step = 0.25 * self.cap;
        let (skin, reach) = Self::skin_reach(self.cap);
        self.refresh_contacts(&x, reach, skin);
        let mut grad = vec![[0.0; 3]; n_nodes];
        let (mut vel, mut dt, mut alpha, mut npos) = match self.fire.take() {
            Some(f) if s.carry => f,
            _ => (vec![[0.0f64; 3]; n_nodes], s.dt_max * 0.5, 0.1, 0usize),
        };
        for _ in 0..s.iterations {
            self.refresh_contacts(&x, reach, skin);
            self.energy(&x, t, pose, s, &mut grad);
            for (i, g) in grad.iter_mut().enumerate() {
                if self.pinned(i) {
                    *g = [0.0; 3];
                }
            }
            let (mut pw, mut vn, mut fnn) = (0.0, 0.0, 0.0);
            for i in 0..n_nodes {
                for c in 0..3 {
                    pw -= grad[i][c] * vel[i][c];
                    vn += vel[i][c] * vel[i][c];
                    fnn += grad[i][c] * grad[i][c];
                }
            }
            if pw > 0.0 {
                let (vn, fnn) = (math::sqrt(vn), math::sqrt(fnn).max(1e-12));
                for i in 0..n_nodes {
                    for c in 0..3 {
                        vel[i][c] = (1.0 - alpha) * vel[i][c] - alpha * vn * grad[i][c] / fnn;
                    }
                }
                npos += 1;
                if npos > 5 {
                    dt = (dt * 1.1).min(s.dt_max);
                    alpha *= 0.99;
                }
            } else {
                npos = 0;
                dt *= 0.5;
                alpha = 0.1;
                vel.iter_mut().for_each(|v| *v = [0.0; 3]);
            }
            for i in 0..n_nodes {
                if self.pinned(i) {
                    continue;
                }
                let mut step = [0.0; 3];
                for c in 0..3 {
                    vel[i][c] -= dt * grad[i][c];
                    step[c] = dt * vel[i][c];
                }
                let l = norm(step);
                let sc = if l > max_step { max_step / l } else { 1.0 };
                for c in 0..3 {
                    x[i][c] += sc * step[c];
                }
            }
        }
        self.refresh_contacts(&x, reach, skin);
        self.fire = Some((vel, dt, alpha, npos));
        self.x = x;
        let x = self.x.clone();
        self.energy(&x, t, &self.pose, s, &mut grad)
    }

    /// Refresh contact pairs for the petals that moved more than half the
    /// skin since their last refresh (see `Contacts::refresh`), and with
    /// them any petal that moved more than a quarter of it.
    fn refresh_contacts(&mut self, x: &[[f64; 3]], reach: f64, skin: f64) {
        let n = self.petals.len();
        let mut moved = vec![0.0f64; n];
        for (i, (p, q)) in x.iter().zip(&self.anchor).enumerate() {
            moved[self.owner[i]] = moved[self.owner[i]].max(norm(sub(*p, *q)));
        }
        // Once any petal must refresh, take those halfway there along too,
        // so refreshes come in batches rather than one petal at a time.
        if moved.iter().any(|&d| d * 2.0 > skin) {
            let dirty: Vec<bool> = moved.iter().map(|&d| d * 4.0 > skin).collect();
            for (i, a) in self.anchor.iter_mut().enumerate() {
                if dirty[self.owner[i]] {
                    *a = x[i];
                }
            }
            self.contacts.refresh(x, reach, &dirty, &self.anchor);
        }
    }

    /// Worst overlap of two petals, in units of the thickness cap.
    pub fn max_overlap(&self) -> f64 {
        self.contacts.max_overlap(&self.x, &self.thick) / self.cap
    }
}
