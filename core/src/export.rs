//! The shared flower JSON (format "automark-flower", v1; see
//! viewer/FORMAT.md). This is the core's one canonical output: the same seed,
//! species and settings always give byte-identical JSON on every platform.

use crate::Bud;
use serde::Serialize;

/// Samples per control-grid span along the petal and across it.
pub const SAMPLES_ALONG: usize = 2;
pub const SAMPLES_ACROSS: usize = 2;

#[derive(Serialize)]
struct Grid {
    nu: usize,
    nv: usize,
}

#[derive(Serialize)]
struct PetalInfo {
    id: usize,
    layer: usize,
    theta: f64,
}

#[derive(Serialize)]
struct PetalFrame {
    mid: Vec<f64>,
    thick: Vec<f64>,
}

#[derive(Serialize)]
struct Frame {
    t: f64,
    stage: &'static str,
    /// Sepal envelope scaled radius at this frame.
    envelope: f64,
    petals: Vec<PetalFrame>,
}

#[derive(Serialize)]
struct Meta<'a> {
    species: &'a crate::species::Species,
    traits: &'a crate::traits::PlantTraits,
    pack: &'a crate::pack::PackReport,
}

#[derive(Serialize)]
struct FlowerDoc<'a> {
    format: &'static str,
    version: u32,
    seed: String,
    flower_index: Option<u64>,
    units: &'static str,
    up: &'static str,
    shells: &'a crate::shells::Shells,
    grid: Grid,
    petals: Vec<PetalInfo>,
    frames: Vec<Frame>,
    meta: Meta<'a>,
}

/// Round to 1e-5 so the file stays small; still far below petal thickness.
fn q(x: f64) -> f64 {
    crate::math::round(x * 1e5) / 1e5
}

/// Evaluate one petal's control points into format order (rows base -> tip,
/// columns edge -> edge), flipping columns so d/du x d/dv faces outward.
fn petal_frame(sheet: &crate::sheet::Sheet, shells: &crate::shells::Shells, nu: usize, nv: usize) -> PetalFrame {
    let (pts, th) = sheet.evaluate(SAMPLES_ALONG, SAMPLES_ACROSS);
    let mut facing = 0.0;
    for a in 0..nu - 1 {
        for c in 0..nv - 1 {
            let p = pts[a * nv + c];
            let n = cross(sub(pts[(a + 1) * nv + c], p), sub(pts[a * nv + c + 1], p));
            let g = shells.grad_scaled_radius(p);
            facing += n[0] * g[0] + n[1] * g[1] + n[2] * g[2];
        }
    }
    let flip = facing < 0.0;
    let mut mid = Vec::with_capacity(nu * nv * 3);
    let mut thick = Vec::with_capacity(nu * nv);
    for a in 0..nu {
        for c in 0..nv {
            let c = if flip { nv - 1 - c } else { c };
            let p = pts[a * nv + c];
            mid.extend_from_slice(&[q(p[0]), q(p[1]), q(p[2])]);
            thick.push(q(th[a * nv + c]));
        }
    }
    PetalFrame { mid, thick }
}

/// The flower JSON. With recorded frames, the bud's development is included
/// as frames with t from -1 (first stage) to 0 (closed bud); otherwise one
/// frame at t = 0.
pub fn flower_json(bud: &Bud) -> String {
    let nu = crate::sheet::Sheet::along(SAMPLES_ALONG);
    let nv = crate::sheet::Sheet::across(SAMPLES_ACROSS);
    let infos = bud
        .sheets
        .iter()
        .enumerate()
        .map(|(i, s)| PetalInfo { id: i, layer: s.layer, theta: q(bud.layout.petals[s.petal].base_angle) })
        .collect();
    let mut frames = Vec::new();
    let last = bud.frames.len().saturating_sub(1).max(1) as f64;
    for f in &bud.frames {
        let petals = bud
            .sheets
            .iter()
            .zip(&f.pos)
            .map(|(s, pos)| {
                let mut s = s.clone();
                s.pos.clone_from(pos);
                petal_frame(&s, &bud.shells, nu, nv)
            })
            .collect();
        frames.push(Frame { t: q(f.stage as f64 / last - 1.0), stage: "bud", envelope: q(f.envelope), petals });
    }
    if frames.is_empty() {
        let petals = bud.sheets.iter().map(|s| petal_frame(s, &bud.shells, nu, nv)).collect();
        frames.push(Frame { t: 0.0, stage: "bud", envelope: q(bud.report.envelope_end), petals });
    }
    let doc = FlowerDoc {
        format: "automark-flower",
        version: 1,
        seed: bud.seed.0.to_string(),
        flower_index: None,
        units: "core_radius",
        up: "+z",
        shells: &bud.shells,
        grid: Grid { nu, nv },
        petals: infos,
        frames,
        meta: Meta { species: &bud.species, traits: &bud.traits, pack: &bud.report },
    };
    serde_json::to_string(&doc).expect("flower JSON serializes")
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
