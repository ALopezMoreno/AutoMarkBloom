use automark_core::bloom::channels::{window, ChannelError};
use automark_core::bloom::{ChannelSpec, Channels, Input, Param, Rule, Scope};

fn growth(target: Param, rate: Param, onset: Param) -> Rule {
    Rule::Growth { target, gain: Param::Const(1.0), bias: Param::Const(0.0), rate, onset, duration: Param::Const(0.5), monotone: true }
}

fn specs() -> Vec<ChannelSpec> {
    vec![
        ChannelSpec::new("rate", Scope::Petal, 1, Rule::Drift { mean: 1.0.into(), revert: 3.0.into(), sigma: 0.4.into() }, vec![1.0]),
        ChannelSpec::new("length", Scope::Petal, 1, growth("target_length".into(), "rate".into(), "onset".into()), vec![1.0]).exported(),
        ChannelSpec::new("tint", Scope::Point, 3, growth(Param::Const(0.8), Param::Const(1.0), "onset".into()), vec![0.1, 0.2, 0.3]).exported(),
    ]
}

fn inputs() -> Vec<Input> {
    vec![
        Input { name: "target_length".into(), scope: Scope::Petal, dims: 1, values: vec![vec![2.0], vec![3.0], vec![2.5]], labels: None },
        Input { name: "onset".into(), scope: Scope::Petal, dims: 1, values: vec![vec![0.0], vec![0.2], vec![0.4]], labels: None },
        Input { name: "petal_type".into(), scope: Scope::Petal, dims: 1, values: vec![vec![0.0], vec![1.0], vec![0.0]], labels: Some(vec!["flat".into(), "cupped".into()]) },
    ]
}

const NODES: [usize; 3] = [4, 6, 5];

fn run(specs: &[ChannelSpec], seed: u64) -> Vec<Channels> {
    let mut c = Channels::new(specs, &inputs(), &NODES, seed).unwrap();
    let mut out = vec![c.clone()];
    let k = 40;
    for i in 0..k {
        c.step(i as f64 / k as f64, (i + 1) as f64 / k as f64);
        out.push(c.clone());
    }
    out
}

fn trace(r: &[Channels], name: &str, petal: usize) -> Vec<f64> {
    r.iter().map(|c| c.scalar(name, petal, f64::NAN)).collect()
}

#[test]
fn same_seed_same_chain_different_seed_differs() {
    let a = run(&specs(), 7);
    let b = run(&specs(), 7);
    let c = run(&specs(), 8);
    assert_eq!(trace(&a, "length", 1), trace(&b, "length", 1));
    assert_ne!(trace(&a, "length", 1), trace(&c, "length", 1));
}

#[test]
fn adding_a_channel_changes_nothing_else() {
    let base = run(&specs(), 7);
    let mut more = specs();
    more.insert(0, ChannelSpec::new("aaa_noise", Scope::Petal, 1, Rule::Drift { mean: 0.0.into(), revert: 1.0.into(), sigma: 1.0.into() }, vec![0.0]));
    let extra = run(&more, 7);
    for p in 0..3 {
        assert_eq!(trace(&base, "length", p), trace(&extra, "length", p));
        assert_eq!(trace(&base, "rate", p), trace(&extra, "rate", p));
    }
}

#[test]
fn growth_is_monotone_and_bounded_by_target() {
    let r = run(&specs(), 3);
    for p in 0..3 {
        let l = trace(&r, "length", p);
        assert!(l.windows(2).all(|w| w[1] >= w[0]), "{l:?}");
        assert!(l.iter().all(|&x| x <= [2.0, 3.0, 2.5][p] + 1e-12));
    }
}

#[test]
fn growth_with_unit_rate_lands_on_target() {
    let r = run(&specs(), 3);
    let last = r.last().unwrap();
    for (p, &n) in NODES.iter().enumerate() {
        let v = last.get("tint", p).unwrap();
        assert_eq!(v.len(), n * 3);
        assert!(v.iter().all(|&x| (x - 0.8).abs() < 1e-9), "{v:?}");
    }
}

#[test]
fn onset_orders_the_opening() {
    let r = run(&specs(), 3);
    // Petal 0 opens at t=0, petal 2 not before t=0.4 (step 16 of 40).
    assert!(trace(&r, "length", 0)[8] > 1.0);
    assert_eq!(trace(&r, "length", 2)[16], 1.0);
}

#[test]
fn inputs_without_specs_are_kept_fixed_and_exported() {
    let r = run(&specs(), 3);
    let last = r.last().unwrap();
    assert_eq!(last.scalar("petal_type", 1, -1.0), 1.0);
    let s = last.spec("petal_type").unwrap();
    assert!(s.export && s.rule == Rule::Fixed);
    assert_eq!(s.labels.as_deref().unwrap().len(), 2);
}

#[test]
fn window_is_smooth_step() {
    assert_eq!(window(0.0, 0.2, 0.5), 0.0);
    assert_eq!(window(0.7, 0.2, 0.5), 1.0);
    assert!((window(0.45, 0.2, 0.5) - 0.5).abs() < 1e-12);
}

#[test]
fn bad_wiring_is_rejected() {
    let mut s = specs();
    s.push(ChannelSpec::new("bad", Scope::Petal, 1, growth("nope".into(), 1.0.into(), 0.0.into()), vec![0.0]));
    assert!(matches!(Channels::new(&s, &inputs(), &NODES, 1), Err(ChannelError::UnknownChannel { .. })));
    let mut s = specs();
    s.push(ChannelSpec::new("coarse", Scope::Petal, 1, growth("tint".into(), 1.0.into(), 0.0.into()), vec![0.0]));
    assert!(matches!(Channels::new(&s, &inputs(), &NODES, 1), Err(ChannelError::Scope { .. })));
}
