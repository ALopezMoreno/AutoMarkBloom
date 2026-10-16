use automark_core::energy::Terms;
use automark_core::pack::build_system;
use automark_core::{export, grow_bud, PackSettings, PlantSeed, Species};

fn quick() -> PackSettings {
    PackSettings { iterations: 60, ..PackSettings::default() }
}

#[test]
fn same_seed_same_json() {
    let s = Species::peony();
    let a = export::flower_json(&grow_bud(PlantSeed(11), &s, &quick()));
    let b = export::flower_json(&grow_bud(PlantSeed(11), &s, &quick()));
    assert_eq!(a, b);
}

#[test]
fn different_plants_differ() {
    let s = Species::peony();
    let a = grow_bud(PlantSeed(11), &s, &quick());
    let b = grow_bud(PlantSeed(2027), &s, &quick());
    assert_ne!(export::flower_json(&a), export::flower_json(&b));
}

#[test]
fn thickness_never_exceeds_cap() {
    let s = Species::peony();
    for seed in [1, 11, 2027] {
        let bud = grow_bud(PlantSeed(seed), &s, &quick());
        let cap = bud.traits.max_thickness;
        assert!(bud.sheets.iter().flat_map(|p| p.thick.iter()).all(|&t| t > 0.0 && t <= cap));
    }
}

#[test]
fn gradient_matches_finite_differences() {
    let s = Species::peony();
    let bud = grow_bud(PlantSeed(11), &s, &quick());
    let (sys, mut x, _) = build_system(&bud.sheets, &bud.shells, &bud.traits);
    // Squeeze everything a little so band, envelope and contact terms are active.
    for p in x.iter_mut() {
        for c in p.iter_mut() {
            *c *= 1.03;
        }
    }
    let cands = sys.candidates(&x, 0.5);
    let mut g = vec![[0.0; 3]; x.len()];
    let t: Terms = sys.energy(&x, &cands, &mut g);
    assert!(t.band > 0.0 || t.envelope > 0.0);
    let mut scratch = vec![[0.0; 3]; x.len()];
    let h = 1e-6;
    let mut worst: f64 = 0.0;
    for k in 0..60 {
        let i = (k * 7919) % x.len();
        let c = k % 3;
        let mut xp = x.clone();
        xp[i][c] += h;
        let mut xm = x.clone();
        xm[i][c] -= h;
        let fd = (sys.energy(&xp, &cands, &mut scratch).total() - sys.energy(&xm, &cands, &mut scratch).total()) / (2.0 * h);
        let err = (fd - g[i][c]).abs() / (1.0 + g[i][c].abs());
        worst = worst.max(err);
    }
    assert!(worst < 1e-4, "worst relative gradient error {worst}");
}

#[test]
fn json_matches_shared_format() {
    let bud = grow_bud(PlantSeed(11), &Species::peony(), &quick());
    let v: serde_json::Value = serde_json::from_str(&export::flower_json(&bud)).unwrap();
    assert_eq!(v["format"], "automark-flower");
    assert_eq!(v["version"], 1);
    assert_eq!(v["seed"], "11");
    let (nu, nv) = (v["grid"]["nu"].as_u64().unwrap() as usize, v["grid"]["nv"].as_u64().unwrap() as usize);
    let n = v["petals"].as_array().unwrap().len();
    let frame = &v["frames"][0]["petals"];
    assert_eq!(frame.as_array().unwrap().len(), n);
    for p in frame.as_array().unwrap() {
        let mid: Vec<f64> = p["mid"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
        assert_eq!(mid.len(), nu * nv * 3);
        assert_eq!(p["thick"].as_array().unwrap().len(), nu * nv);
        // Outer face: d/du x d/dv at the petal's middle points away from the axis.
        let at = |a: usize, c: usize| [mid[(a * nv + c) * 3], mid[(a * nv + c) * 3 + 1], mid[(a * nv + c) * 3 + 2]];
        let (a, c) = (nu / 2, nv / 2);
        let (p0, pu, pv) = (at(a, c), at(a + 1, c), at(a, c + 1));
        let du = [pu[0] - p0[0], pu[1] - p0[1], pu[2] - p0[2]];
        let dv = [pv[0] - p0[0], pv[1] - p0[1], pv[2] - p0[2]];
        let nrm = [du[1] * dv[2] - du[2] * dv[1], du[2] * dv[0] - du[0] * dv[2], du[0] * dv[1] - du[1] * dv[0]];
        assert!(nrm[0] * p0[0] + nrm[1] * p0[1] > 0.0, "petal normal faces inward");
    }
}

#[test]
fn species_round_trips_through_json() {
    let s = Species::peony();
    let text = serde_json::to_string(&s).unwrap();
    assert_eq!(Species::from_json(&text).unwrap(), s);
}
