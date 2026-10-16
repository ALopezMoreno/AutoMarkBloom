//! Docs helper, not part of the model. Re-runs the shell automaton of
//! `core/src/ca.rs` for one flower, step for step with the same random
//! stream, records every intermediate state as JSON, and checks that the
//! final grids equal what `ca::run` produces.
//!
//! Usage: cargo run --release -- <plant seed> <flower index> > trace.json

use automark_core::ca::{self, NA, NP};
use automark_core::shells::Shells;
use automark_core::traits::{FlowerTraits, PlantSeed, STREAM_CA, STREAM_FLOWER};
use automark_core::{math, Species};
use serde_json::json;
use std::f64::consts::TAU;

const EMPTY: i32 = -1;

fn theta(s: usize) -> f64 {
    (s as f64 + 0.5) * TAU / NA as f64
}

fn ang_diff(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(TAU);
    d.min(TAU - d)
}

fn main() {
    let args: Vec<u64> = std::env::args().skip(1).map(|a| a.parse().unwrap()).collect();
    let (seed, index) = (PlantSeed(args[0]), args[1]);
    let species = Species::peony();
    let plant = seed.traits(&species);
    let flower = FlowerTraits::draw(index, &species, &plant, &mut seed.flower_rng(index, STREAM_FLOWER));
    let t = flower.apply(&plant);
    let shells = Shells::new(&t);
    let reference = ca::run(&t, &shells, &mut seed.flower_rng(index, STREAM_CA));

    let mut rng = seed.flower_rng(index, STREAM_CA);
    let mut base: Vec<(usize, f64)> = Vec::new();
    let mut max_rows: Vec<usize> = Vec::new();
    let two_s2 = 2.0 * t.inhibit_sigma * t.inhibit_sigma;
    let mut layers = Vec::new();

    for k in 0..t.layers {
        let a = shells.radius[k];
        let frac = if t.layers > 1 { k as f64 / (t.layers - 1) as f64 } else { 1.0 };
        let mut g = vec![EMPTY; NP * NA];
        let inhibition = |base: &[(usize, f64)], th: f64| -> f64 {
            let mut inh = 0.0;
            for &(l, b) in base {
                let w = if l == k { 1.0 } else if l + 1 == k { t.stagger } else { 0.0 };
                if w > 0.0 {
                    let d = ang_diff(th, b) * a;
                    inh += w * math::exp(-d * d / two_s2);
                }
            }
            inh
        };
        // Inhibition on row 0 from the layer below only, before any primordium of this layer.
        let below: Vec<f64> = (0..NA).map(|s| inhibition(&base, theta(s))).collect();
        let mut events = Vec::new();
        let mut after_pass = Vec::new();
        for pass in 0..4 {
            let mut order: Vec<usize> = (0..NA).collect();
            rng.shuffle(&mut order);
            for (visit, s) in order.into_iter().enumerate() {
                if g[s] != EMPTY {
                    continue;
                }
                let th = theta(s);
                let inh = inhibition(&base, th);
                if inh < t.inhibit_threshold && rng.f64() < t.init_prob {
                    let id = base.len();
                    base.push((k, th));
                    let h = NP as f64 * (t.inner_height + (1.0 - t.inner_height) * frac) * rng.range(0.85, 1.0);
                    let hr = (math::round(h) as usize).clamp(4, NP);
                    max_rows.push(hr);
                    g[s] = id as i32;
                    events.push(json!({"pass": pass, "visit": visit, "cell": s, "petal": id, "inhibition": inh, "max_rows": hr}));
                }
            }
            after_pass.push((0..NA).map(|s| inhibition(&base, theta(s))).collect::<Vec<f64>>());
        }

        let mut sweeps = vec![g.clone()];
        for sweep in 0..NP * 3 {
            let old = g.clone();
            let mut changed = false;
            for r in 0..NP {
                for s in 0..NA {
                    let i = r * NA + s;
                    if old[i] != EMPTY {
                        continue;
                    }
                    let mut cands: [(i32, f64); 3] = [(EMPTY, 0.0); 3];
                    let mut n = 0;
                    let fits = |p: i32| p >= 0 && r < max_rows[p as usize];
                    if r > 0 && fits(old[i - NA]) {
                        cands[n] = (old[i - NA], t.up_prob);
                        n += 1;
                    }
                    for nb in [r * NA + (s + NA - 1) % NA, r * NA + (s + 1) % NA] {
                        if fits(old[nb]) {
                            cands[n] = (old[nb], t.lateral_prob);
                            n += 1;
                        }
                    }
                    if n == 0 {
                        continue;
                    }
                    let start = rng.int(0, n as i64 - 1) as usize;
                    for j in 0..n {
                        let (p, prob) = cands[(start + j) % n];
                        if rng.f64() < prob {
                            g[i] = p;
                            changed = true;
                            break;
                        }
                    }
                }
            }
            sweeps.push(g.clone());
            if !changed && sweep > NP {
                break;
            }
        }
        assert_eq!(g, reference.grids[k], "layer {k} differs from ca::run");
        layers.push(json!({
            "layer": k, "radius": a, "below": below, "after_pass": after_pass,
            "events": events, "sweeps": sweeps,
        }));
    }

    let petals: Vec<_> = reference.petals.iter().map(|p| json!({
        "id": p.id, "layer": p.layer, "base_angle": p.base_angle, "max_rows": p.max_rows,
        "cells": p.cells, "rows": p.rows,
    })).collect();
    println!("{}", json!({
        "seed": args[0], "flower": index, "na": NA, "np": NP,
        "traits": t, "petals": petals, "layers": layers,
    }));
}
