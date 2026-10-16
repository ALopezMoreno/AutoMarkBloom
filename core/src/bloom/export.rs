//! Opening frames in the shared flower JSON (viewer/FORMAT.md).
//!
//! The bud's own document is taken as it is and extended: `flower_index`
//! is set, the exported channels are declared once in the top-level
//! `channels` list, and one frame per opening step is appended with
//! `stage: "open"` and t in (0, 1]. Each frame carries the exported
//! channels' values: flower scope on the frame, petal and point scope on
//! each petal (a bare number when dims = 1). Building on the bud's JSON
//! rather than re-writing it keeps the two stages' fields independent.

use super::channels::{Channels, Scope};
use super::Opening;
use crate::export::{SAMPLES_ACROSS, SAMPLES_ALONG};
use crate::sheet::Sheet;
use crate::Bud;
use serde_json::{json, Map, Value};

fn q(x: f64) -> f64 {
    crate::math::round(x * 1e5) / 1e5
}

fn num(x: f64) -> Value {
    serde_json::Number::from_f64(q(x)).map(Value::Number).unwrap_or(Value::Null)
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Same orientation rule as the bud export, decided once on the bud so every
/// frame of a petal keeps the same winding.
fn flips(bud: &Bud, nu: usize, nv: usize) -> Vec<bool> {
    bud.sheets
        .iter()
        .map(|s| {
            let (pts, _) = s.evaluate(SAMPLES_ALONG, SAMPLES_ACROSS);
            let mut facing = 0.0;
            for a in 0..nu - 1 {
                for c in 0..nv - 1 {
                    let p = pts[a * nv + c];
                    let (du, dv) = (sub(pts[(a + 1) * nv + c], p), sub(pts[a * nv + c + 1], p));
                    let n = [du[1] * dv[2] - du[2] * dv[1], du[2] * dv[0] - du[0] * dv[2], du[0] * dv[1] - du[1] * dv[0]];
                    let g = bud.shells.grad_scaled_radius(p);
                    facing += n[0] * g[0] + n[1] * g[1] + n[2] * g[2];
                }
            }
            facing < 0.0
        })
        .collect()
}

/// Linear refinement of a per-node field onto the evaluated grid, the same
/// sampling `Sheet::evaluate` uses for thickness.
fn refine_field(v: &[f64], cu: usize, cv: usize, dims: usize, flip: bool) -> Vec<Value> {
    let (nu, nv) = (Sheet::along(SAMPLES_ALONG), Sheet::across(SAMPLES_ACROSS));
    let mut out = Vec::with_capacity(nu * nv * dims);
    for a in 0..nu {
        let fa = a as f64 / SAMPLES_ALONG as f64;
        let (r0, r1, wa) = (fa as usize, (fa as usize + 1).min(cv - 1), fa - (fa as usize) as f64);
        for c in 0..nv {
            let c = if flip { nv - 1 - c } else { c };
            let fc = c as f64 / SAMPLES_ACROSS as f64;
            let (c0, c1, wc) = (fc as usize, (fc as usize + 1).min(cu - 1), fc - (fc as usize) as f64);
            for d in 0..dims {
                let g = |r: usize, k: usize| v[(r * cu + k) * dims + d];
                let lo = g(r0, c0) + wc * (g(r0, c1) - g(r0, c0));
                let hi = g(r1, c0) + wc * (g(r1, c1) - g(r1, c0));
                out.push(num(lo + wa * (hi - lo)));
            }
        }
    }
    out
}

fn value(v: &[f64]) -> Value {
    if v.len() == 1 {
        num(v[0])
    } else {
        Value::Array(v.iter().map(|&x| num(x)).collect())
    }
}

fn declare(ch: &Channels, existing: &mut Vec<Value>) {
    for s in ch.specs().filter(|s| s.export) {
        if existing.iter().any(|e| e.get("name").and_then(Value::as_str) == Some(&s.name)) {
            continue;
        }
        let mut d = json!({ "name": s.name, "scope": s.scope, "dims": s.dims, "kind": s.rule.kind() });
        if let Some(r) = s.range {
            d["range"] = json!(r);
        }
        if let Some(l) = &s.labels {
            d["labels"] = json!(l);
        }
        existing.push(d);
    }
}

/// The bud's document plus the opening frames.
pub fn flower_json(bud: &Bud, opening: &Opening) -> String {
    let mut doc: Value = serde_json::from_str(&crate::export::flower_json(bud)).expect("bud JSON parses");
    let (nu, nv) = (Sheet::along(SAMPLES_ALONG), Sheet::across(SAMPLES_ACROSS));
    let cu = crate::sheet::NU;
    let flip = flips(bud, nu, nv);
    doc["flower_index"] = json!(opening.flower_index);

    let mut decl = doc.get("channels").and_then(Value::as_array).cloned().unwrap_or_default();
    if let Some(f) = opening.frames.first() {
        declare(&f.channels, &mut decl);
    }
    doc["channels"] = Value::Array(decl);

    let frames = doc["frames"].as_array_mut().expect("bud frames");
    for f in frames.iter_mut() {
        if f.get("stage").is_none() {
            f["stage"] = json!("bud");
        }
    }
    for f in &opening.frames {
        let ch = &f.channels;
        let mut petals = Vec::new();
        for (p, s) in bud.sheets.iter().enumerate() {
            let sheet = Sheet { petal: s.petal, layer: s.layer, pos: f.pos[p].clone(), thick: f.thick[p].clone() };
            let (pts, th) = sheet.evaluate(SAMPLES_ALONG, SAMPLES_ACROSS);
            let cv = f.pos[p].len() / cu;
            let (mut mid, mut thick) = (Vec::with_capacity(nu * nv * 3), Vec::with_capacity(nu * nv));
            for a in 0..nu {
                for c in 0..nv {
                    let c = if flip[p] { nv - 1 - c } else { c };
                    let i = a * nv + c;
                    mid.extend(pts[i].iter().map(|&x| num(x)));
                    thick.push(num(th[i]));
                }
            }
            let mut pc = Map::new();
            for spec in ch.specs().filter(|s| s.export && s.scope != Scope::Flower) {
                let v = ch.get(&spec.name, p).expect("declared");
                let val = match spec.scope {
                    Scope::Point => Value::Array(refine_field(v, cu, cv, spec.dims, flip[p])),
                    _ => value(v),
                };
                pc.insert(spec.name.clone(), val);
            }
            petals.push(json!({ "mid": mid, "thick": thick, "channels": pc }));
        }
        let mut fc = Map::new();
        for spec in ch.specs().filter(|s| s.export && s.scope == Scope::Flower) {
            fc.insert(spec.name.clone(), value(ch.get(&spec.name, 0).expect("declared")));
        }
        frames.push(json!({ "t": num(f.t), "stage": "open", "petals": petals, "channels": fc }));
    }
    serde_json::to_string(&doc).expect("flower JSON serializes")
}

/// Canonical opening for a seed and flower: peony, default settings.
pub fn open_flower_json(seed: u64, flower_index: u64) -> String {
    let bud = crate::grow_bud(crate::PlantSeed(seed), flower_index, &crate::Species::peony(), &crate::PackSettings::default());
    let opening = super::open_flower(&bud, &super::BloomSettings::default());
    flower_json(&bud, &opening)
}
