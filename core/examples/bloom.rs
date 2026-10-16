//! Open one flower and write its JSON: `cargo run --release --example bloom -- <seed> <flower> <out.json>`
use automark_core::bloom::{self, BloomSettings};
use automark_core::{grow_bud, PackSettings, PlantSeed, Species};
use std::time::Instant;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let seed: u64 = a.get(1).map_or(11, |s| s.parse().unwrap());
    let flower: u64 = a.get(2).map_or(0, |s| s.parse().unwrap());
    let out = a.get(3).cloned().unwrap_or_else(|| format!("flower_{seed}_{flower}.json"));
    let t = Instant::now();
    let bud = grow_bud(PlantSeed(seed), flower, &Species::peony(), &PackSettings { record_frames: true, ..PackSettings::default() });
    eprintln!("bud: {} petals, {:.1}s", bud.sheets.len(), t.elapsed().as_secs_f64());
    let t = Instant::now();
    let mut s = BloomSettings::default();
    let env = |k: &str| std::env::var(k).ok().map(|v| v.parse::<f64>().unwrap());
    if let Some(v) = env("FRAMES") { s.frames = v as usize; }
    if let Some(v) = env("ITERS") { s.relax.iterations = v as usize; }
    if let Some(v) = env("KBEND") { s.relax.k_bend = v; }
    if let Some(v) = env("KPOSE") { s.relax.k_pose = v; }
    if let Some(v) = env("KSTRETCH") { s.relax.k_stretch = v; s.relax.k_shear = v / 4.0; }
    if let Some(v) = env("DT") { s.relax.dt_max = v; }
    let o = bloom::open_flower(&bud, &s);
    for f in &o.frames {
        eprintln!("t={:.3} E={:.3} overlap={:.2}", f.t, f.energy, f.max_overlap);
    }
    eprintln!("opening: {} frames, {:.1}s", o.frames.len(), t.elapsed().as_secs_f64());
    std::fs::write(&out, bloom::export::flower_json(&bud, &o)).unwrap();
}
