//! The shared flower JSON (format "automark-flower", v1; see
//! viewer/FORMAT.md). This is the core's one canonical output: the same seed,
//! species and settings always give byte-identical JSON on every platform.

use crate::channels::{ChannelDecl, Scope};
use crate::Bud;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

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
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    channels: BTreeMap<String, Value>,
}

#[derive(Serialize)]
struct Frame {
    t: f64,
    stage: &'static str,
    /// Flower-scope channel values; an absent channel keeps its last value.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    channels: BTreeMap<String, Value>,
    petals: Vec<PetalFrame>,
}

#[derive(Serialize)]
struct Meta<'a> {
    species: &'a crate::species::Species,
    plant: &'a crate::traits::PlantTraits,
    flower: &'a crate::traits::FlowerTraits,
    pack: &'a crate::pack::PackReport,
}

#[derive(Serialize)]
struct FlowerDoc<'a> {
    format: &'static str,
    version: u32,
    seed: String,
    flower_index: u64,
    units: &'static str,
    up: &'static str,
    shells: &'a crate::shells::Shells,
    grid: Grid,
    channels: &'a [ChannelDecl],
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
    PetalFrame { mid, thick, channels: BTreeMap::new() }
}

/// One channel value: a bare number for dims 1, else an array.
fn value(v: &[f64]) -> Value {
    if v.len() == 1 {
        Value::from(q(v[0]))
    } else {
        Value::from(v.iter().map(|&x| q(x)).collect::<Vec<f64>>())
    }
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
        let mut channels = BTreeMap::new();
        channels.insert("sepal_radius".to_string(), Value::from(q(f.envelope)));
        frames.push(Frame { t: q(f.stage as f64 / last - 1.0), stage: "bud", channels, petals });
    }
    if frames.is_empty() {
        let petals = bud.sheets.iter().map(|s| petal_frame(s, &bud.shells, nu, nv)).collect();
        frames.push(Frame { t: 0.0, stage: "bud", channels: BTreeMap::new(), petals });
    }
    // Every channel's value goes in the first frame. Frames that recorded
    // their own sepal radius keep it.
    let first = &mut frames[0];
    for d in &bud.channels.decls {
        let vals = bud.channels.get(&d.name).unwrap_or(&[]);
        match d.scope {
            Scope::Flower => {
                first.channels.entry(d.name.clone()).or_insert_with(|| value(vals));
            }
            Scope::Petal => {
                for (i, p) in first.petals.iter_mut().enumerate() {
                    p.channels.insert(d.name.clone(), value(&vals[i * d.dims..(i + 1) * d.dims]));
                }
            }
            Scope::Point => {
                let per = vals.len() / first.petals.len().max(1);
                for (i, p) in first.petals.iter_mut().enumerate() {
                    p.channels.insert(d.name.clone(), Value::from(vals[i * per..(i + 1) * per].iter().map(|&x| q(x)).collect::<Vec<f64>>()));
                }
            }
        }
    }
    let doc = FlowerDoc {
        format: "automark-flower",
        version: 1,
        seed: bud.seed.0.to_string(),
        flower_index: bud.flower.index,
        units: "core_radius",
        up: "+z",
        shells: &bud.shells,
        grid: Grid { nu, nv },
        channels: &bud.channels.decls,
        petals: infos,
        frames,
        meta: Meta { species: &bud.species, plant: &bud.plant, flower: &bud.flower, pack: &bud.report },
    };
    serde_json::to_string(&doc).expect("flower JSON serializes")
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
