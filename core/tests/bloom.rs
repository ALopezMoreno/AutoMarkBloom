use automark_core::bloom::relax::{Flower, RelaxSettings};
use automark_core::bloom::{self, program, BloomSettings, ChannelSpec, Channels, Opening, Param, Rule, Scope};
use automark_core::{grow_bud, Bud, PackSettings, PlantSeed, Species};

fn bud(seed: u64, flower: u64) -> Bud {
    grow_bud(PlantSeed(seed), flower, &Species::peony(), &PackSettings::quick())
}

fn quick() -> BloomSettings {
    BloomSettings { frames: 4, relax: RelaxSettings { iterations: 25, ..RelaxSettings::default() }, ..BloomSettings::default() }
}

fn open(b: &Bud) -> Opening {
    bloom::open_flower(b, &quick())
}

#[test]
fn same_flower_same_json() {
    let b = bud(11, 0);
    let a = bloom::export::flower_json(&b, &open(&b));
    let c = bloom::export::flower_json(&b, &open(&b));
    assert_eq!(a, c);
}

#[test]
fn sibling_flowers_differ() {
    let (a, b) = (bud(11, 0), bud(11, 1));
    assert_ne!(bloom::export::flower_json(&a, &open(&a)), bloom::export::flower_json(&b, &open(&b)));
}

#[test]
fn with_no_growth_the_pose_is_the_packed_bud() {
    let b = bud(11, 0);
    let grids = bloom::bud_grids(&b);
    let flower = Flower::new(&grids, &b.shells, &RelaxSettings::default());
    let nodes: Vec<usize> = grids.iter().map(|g| g.pos.len()).collect();
    let none = Channels::new(&[], &[], &nodes, 1).unwrap();
    for (p, pt) in flower.petals.iter().enumerate() {
        let t = pt.targets(&none, p);
        for (q, x) in t.pose.iter().zip(&grids[p].pos) {
            for c in 0..3 {
                assert!((q[c] - x[c]).abs() < 1e-9, "petal {p}: {q:?} vs {x:?}");
            }
        }
    }
}

#[test]
fn adding_a_channel_leaves_the_flower_unchanged() {
    let b = bud(11, 0);
    let specs = program::default_program();
    let inputs = bloom::bud_inputs(&b);
    let base = bloom::open_with(&b, &specs, &inputs, &quick()).unwrap();
    let mut more = specs.clone();
    more.push(
        ChannelSpec::new(
            "tint",
            Scope::Point,
            3,
            Rule::Growth {
                target: Param::Const(1.0),
                gain: Param::Const(1.0),
                bias: Param::Const(0.0),
                rate: "rate".into(),
                onset: "onset".into(),
                duration: Param::Const(0.5),
                monotone: true,
            },
            vec![0.2, 0.1, 0.3],
        )
        .exported(),
    );
    more.push(ChannelSpec::new("noise", Scope::Petal, 1, Rule::Drift { mean: 0.0.into(), revert: 1.0.into(), sigma: 1.0.into() }, vec![0.0]));
    let extra = bloom::open_with(&b, &more, &inputs, &quick()).unwrap();
    for (f, g) in base.frames.iter().zip(&extra.frames) {
        assert_eq!(f.pos, g.pos);
        assert_eq!(f.thick, g.thick);
    }
    let json = bloom::export::flower_json(&b, &extra);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(v["channels"].as_array().unwrap().iter().any(|c| c["name"] == "tint" && c["scope"] == "point" && c["dims"] == 3));
    let (nu, nv) = (v["grid"]["nu"].as_u64().unwrap(), v["grid"]["nv"].as_u64().unwrap());
    let last = v["frames"].as_array().unwrap().last().unwrap();
    assert_eq!(last["petals"][0]["channels"]["tint"].as_array().unwrap().len() as u64, nu * nv * 3);
}

#[test]
fn sizes_never_shrink_and_thickness_stays_capped() {
    let b = bud(2027, 0);
    let o = open(&b);
    let cap = b.sheets.iter().flat_map(|s| s.thick.iter()).cloned().fold(0.0, f64::max);
    let mut last: Vec<(f64, f64)> = vec![(1.0, 1.0); b.sheets.len()];
    for f in &o.frames {
        for (p, l) in last.iter_mut().enumerate() {
            let now = (f.channels.scalar("length", p, 0.0), f.channels.scalar("width", p, 0.0));
            assert!(now.0 >= l.0 && now.1 >= l.1, "petal {p} shrank: {l:?} -> {now:?}");
            *l = now;
        }
        assert!(f.thick.iter().flatten().all(|&t| t > 0.0 && t <= cap + 1e-12));
    }
}

#[test]
fn outer_petals_open_first() {
    let b = bud(11, 0);
    let o = open(&b);
    let top = b.sheets.iter().map(|s| s.layer).max().unwrap();
    let mean = |f: &bloom::Frame, pick: &dyn Fn(usize) -> bool| {
        let v: Vec<f64> = (0..b.sheets.len()).filter(|&p| pick(b.sheets[p].layer)).map(|p| f.channels.scalar("length", p, 1.0)).collect();
        v.iter().sum::<f64>() / v.len() as f64
    };
    let early = &o.frames[0];
    assert!(mean(early, &|l| l == top) > mean(early, &|l| l == 0));
}

#[test]
fn relax_gradient_matches_finite_differences() {
    let b = bud(11, 0);
    let grids = bloom::bud_grids(&b);
    let s = RelaxSettings::default();
    let mut flower = Flower::new(&grids, &b.shells, &s);
    let nodes: Vec<usize> = grids.iter().map(|g| g.pos.len()).collect();
    let mut ch = Channels::new(&program::default_program(), &bloom::bud_inputs(&b), &nodes, 3).unwrap();
    ch.step(0.0, 0.5);
    let t: Vec<_> = flower.petals.iter().enumerate().map(|(p, pt)| pt.targets(&ch, p)).collect();
    let pose: Vec<[f64; 3]> = t.iter().flat_map(|x| x.pose.iter().cloned()).collect();
    for (p, tp) in t.iter().enumerate() {
        let o = flower.offset[p];
        flower.thick[o..o + tp.thick.len()].copy_from_slice(&tp.thick);
    }
    // Squeeze toward the axis so contact and core terms are active.
    let x: Vec<[f64; 3]> = flower.x.iter().map(|p| [p[0] * 0.97, p[1] * 0.97, p[2]]).collect();
    flower.contacts.rebuild(&x, 3.0 * flower.cap);
    let mut g = vec![[0.0; 3]; x.len()];
    flower.energy(&x, &t, &pose, &s, &mut g);
    let mut scratch = vec![[0.0; 3]; x.len()];
    let h = 1e-6;
    let mut worst: f64 = 0.0;
    for k in 0..80 {
        let i = (k * 7919) % x.len();
        let c = k % 3;
        let (mut xp, mut xm) = (x.clone(), x.clone());
        xp[i][c] += h;
        xm[i][c] -= h;
        let fd = (flower.energy(&xp, &t, &pose, &s, &mut scratch) - flower.energy(&xm, &t, &pose, &s, &mut scratch)) / (2.0 * h);
        worst = worst.max((fd - g[i][c]).abs() / (1.0 + g[i][c].abs()));
    }
    assert!(worst < 1e-4, "worst relative gradient error {worst}");
}

#[test]
fn json_continues_the_bud_frames() {
    let b = grow_bud(PlantSeed(11), 0, &Species::peony(), &PackSettings { record_frames: true, ..PackSettings::quick() });
    let v: serde_json::Value = serde_json::from_str(&bloom::export::flower_json(&b, &open(&b))).unwrap();
    let frames = v["frames"].as_array().unwrap();
    let ts: Vec<f64> = frames.iter().map(|f| f["t"].as_f64().unwrap()).collect();
    assert!(ts.windows(2).all(|w| w[1] > w[0]), "{ts:?}");
    assert_eq!(frames.iter().filter(|f| f["stage"] == "open").count(), 4);
    assert_eq!(*ts.last().unwrap(), 1.0);
    let n = v["petals"].as_array().unwrap().len();
    let per = (v["grid"]["nu"].as_u64().unwrap() * v["grid"]["nv"].as_u64().unwrap()) as usize;
    for f in frames {
        assert_eq!(f["petals"].as_array().unwrap().len(), n);
        assert_eq!(f["petals"][0]["thick"].as_array().unwrap().len(), per);
    }
    assert_eq!(v["flower_index"], 0);
}
