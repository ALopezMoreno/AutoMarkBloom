use automark_core::energy::{Envelope, Terms};
use automark_core::pack::build_system;
use automark_core::{export, grow_bud, PackSettings, PlantSeed, Species};

fn quick() -> PackSettings {
    PackSettings::quick()
}

#[test]
fn same_seed_same_json() {
    let s = Species::peony();
    let a = export::flower_json(&grow_bud(PlantSeed(11), 0, &s, &quick()));
    let b = export::flower_json(&grow_bud(PlantSeed(11), 0, &s, &quick()));
    assert_eq!(a, b);
}

#[test]
fn different_plants_differ() {
    let s = Species::peony();
    let a = grow_bud(PlantSeed(11), 0, &s, &quick());
    let b = grow_bud(PlantSeed(2027), 0, &s, &quick());
    assert_ne!(export::flower_json(&a), export::flower_json(&b));
}

#[test]
fn thickness_never_exceeds_cap() {
    let s = Species::peony();
    for seed in [1, 11, 2027] {
        let bud = grow_bud(PlantSeed(seed), 0, &s, &quick());
        let cap = bud.traits.max_thickness;
        assert!(bud.sheets.iter().flat_map(|p| p.thick.iter()).all(|&t| t > 0.0 && t <= cap));
    }
}

#[test]
fn gradient_matches_finite_differences() {
    let s = Species::peony();
    let bud = grow_bud(PlantSeed(11), 0, &s, &quick());
    let (sys, mut x, _) = build_system(&bud.sheets, &bud.shells, &bud.traits);
    // Squeeze everything a little so band, envelope and contact terms are active.
    for p in x.iter_mut() {
        for c in p.iter_mut() {
            *c *= 1.03;
        }
    }
    let cands = sys.candidates(&x, 0.5);
    let env = Envelope { r: bud.shells.envelope * 0.97, rest: bud.shells.envelope * 1.02 };
    let mut g = vec![[0.0; 3]; x.len()];
    let mut genv = 0.0;
    let t: Terms = sys.energy(&x, env, &cands, &mut g, &mut genv);
    assert!(t.band > 0.0 || t.envelope > 0.0);
    let mut scratch = vec![[0.0; 3]; x.len()];
    let mut sg = 0.0;
    let h = 1e-6;
    let mut e = |x: &[[f64; 3]], env: Envelope| sys.energy(x, env, &cands, &mut scratch, &mut sg).total();
    let mut worst: f64 = 0.0;
    for k in 0..60 {
        let i = (k * 7919) % x.len();
        let c = k % 3;
        let mut xp = x.clone();
        xp[i][c] += h;
        let mut xm = x.clone();
        xm[i][c] -= h;
        let fd = (e(&xp, env) - e(&xm, env)) / (2.0 * h);
        let err = (fd - g[i][c]).abs() / (1.0 + g[i][c].abs());
        worst = worst.max(err);
    }
    // The envelope radius is a variable too.
    let fd = (e(&x, Envelope { r: env.r + h, ..env }) - e(&x, Envelope { r: env.r - h, ..env })) / (2.0 * h);
    worst = worst.max((fd - genv).abs() / (1.0 + genv.abs()));
    assert!(worst < 1e-4, "worst relative gradient error {worst}");
}

#[test]
fn json_matches_shared_format() {
    let bud = grow_bud(PlantSeed(11), 0, &Species::peony(), &quick());
    let v: serde_json::Value = serde_json::from_str(&export::flower_json(&bud)).unwrap();
    assert_eq!(v["format"], "automark-flower");
    assert_eq!(v["version"], 1);
    assert_eq!(v["seed"], "11");
    assert_eq!(v["flower_index"], 0);
    let decls = v["channels"].as_array().unwrap();
    for name in ["target_length", "cup", "onset", "petal_type", "vigour", "sepal_radius"] {
        assert!(decls.iter().any(|d| d["name"] == name), "missing channel {name}");
    }
    let (nu, nv) = (v["grid"]["nu"].as_u64().unwrap() as usize, v["grid"]["nv"].as_u64().unwrap() as usize);
    let n = v["petals"].as_array().unwrap().len();
    let frame = &v["frames"][0]["petals"];
    assert_eq!(frame.as_array().unwrap().len(), n);
    for p in frame.as_array().unwrap() {
        let mid: Vec<f64> = p["mid"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
        assert_eq!(mid.len(), nu * nv * 3);
        assert_eq!(p["thick"].as_array().unwrap().len(), nu * nv);
        // Outer face: d/du x d/dv points away from the bud centre. Checked
        // over the whole petal, since a fold can locally overturn it.
        let at = |a: usize, c: usize| [mid[(a * nv + c) * 3], mid[(a * nv + c) * 3 + 1], mid[(a * nv + c) * 3 + 2]];
        let mut outward = 0.0;
        for a in 0..nu - 1 {
            for c in 0..nv - 1 {
                let (p0, pu, pv) = (at(a, c), at(a + 1, c), at(a, c + 1));
                let du = [pu[0] - p0[0], pu[1] - p0[1], pu[2] - p0[2]];
                let dv = [pv[0] - p0[0], pv[1] - p0[1], pv[2] - p0[2]];
                let nrm = [du[1] * dv[2] - du[2] * dv[1], du[2] * dv[0] - du[0] * dv[2], du[0] * dv[1] - du[1] * dv[0]];
                outward += nrm[0] * p0[0] + nrm[1] * p0[1] + nrm[2] * p0[2];
            }
        }
        assert!(outward > 0.0, "petal normal faces inward");
    }
}

#[test]
fn species_round_trips_through_json() {
    let s = Species::peony();
    let text = serde_json::to_string(&s).unwrap();
    assert_eq!(Species::from_json(&text).unwrap(), s);
}

#[test]
fn development_frames_keep_topology() {
    let settings = PackSettings { record_frames: true, ..PackSettings::quick() };
    let bud = grow_bud(PlantSeed(7), 0, &Species::peony(), &settings);
    assert_eq!(bud.frames.len(), bud.traits.growth_stages + 1);
    let v: serde_json::Value = serde_json::from_str(&export::flower_json(&bud)).unwrap();
    let frames = v["frames"].as_array().unwrap();
    assert_eq!(frames.first().unwrap()["t"], -1.0);
    assert_eq!(frames.last().unwrap()["t"], 0.0);
    let n = v["petals"].as_array().unwrap().len();
    for f in frames {
        assert_eq!(f["petals"].as_array().unwrap().len(), n);
    }
}

#[test]
fn siblings_share_traits_but_not_buds() {
    let s = Species::peony();
    let a = grow_bud(PlantSeed(11), 0, &s, &quick());
    let b = grow_bud(PlantSeed(11), 1, &s, &quick());
    assert_eq!(a.plant, b.plant);
    assert_ne!(export::flower_json(&a), export::flower_json(&b));
    assert!((a.traits.layers as i64 - a.plant.layers as i64).abs() <= 1);
}

#[test]
fn adding_a_channel_does_not_shift_others() {
    use automark_core::channels::channel_rng;
    let mut a = channel_rng(99, "cup");
    let mut b = channel_rng(99, "cup");
    let _unrelated = channel_rng(99, "colour_gradient").f64();
    assert_eq!(a.f64(), b.f64());
    assert_ne!(channel_rng(99, "cup").f64(), channel_rng(99, "onset").f64());
}

#[test]
fn relax_budget_keeps_the_same_flower() {
    // Preview and canonical settings differ only in iteration counts, so the
    // plant, flower traits and petal layout must not depend on them.
    let s = Species::peony();
    let a = grow_bud(PlantSeed(11), 1, &s, &quick());
    let b = grow_bud(PlantSeed(11), 1, &s, &PackSettings { relax_iterations: 5, settle_iterations: 5, ..quick() });
    assert_eq!(a.traits, b.traits);
    assert_eq!(a.report.petals, b.report.petals);
    assert_eq!(a.layout.petals.len(), b.layout.petals.len());
}
