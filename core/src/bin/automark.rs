//! automark --seed N [--flower K] --json out.json [--species species.json] [--relax N] [--preview] [--frames] [--open N]

use automark_core::bloom::{self, BloomSettings};
use automark_core::{export, grow_bud, PackSettings, PlantSeed, Species};
use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut seed: Option<u64> = None;
    let mut flower: u64 = 0;
    let mut json: Option<String> = None;
    let mut species = Species::peony();
    let mut settings = PackSettings::default();
    let mut open: Option<usize> = None;
    let mut i = 0;
    let value = |i: usize| -> &str {
        args.get(i + 1).map(|s| s.as_str()).unwrap_or_else(|| {
            eprintln!("missing value for {}", args[i]);
            exit(2)
        })
    };
    while i < args.len() {
        let mut step = 2;
        match args[i].as_str() {
            "--seed" => seed = Some(value(i).parse().expect("--seed takes a u64")),
            "--flower" => flower = value(i).parse().expect("--flower takes a u64"),
            "--json" => json = Some(value(i).to_string()),
            "--species" => {
                let text = std::fs::read_to_string(value(i)).expect("read species file");
                species = Species::from_json(&text).expect("parse species file");
            }
            "--relax" => settings.relax_iterations = value(i).parse().expect("--relax takes a number"),
            "--open" => open = Some(value(i).parse().expect("--open takes a frame count")),
            "--preview" => {
                let frames = settings.record_frames;
                settings = PackSettings { record_frames: frames, ..PackSettings::preview() };
                step = 1;
            }
            "--frames" => {
                settings.record_frames = true;
                step = 1;
            }
            other => {
                eprintln!("unknown argument {other}\nusage: automark --seed N [--flower K] --json out.json [--species file.json] [--relax N] [--preview] [--frames] [--open N]");
                exit(2);
            }
        }
        i += step;
    }
    let Some(seed) = seed else {
        eprintln!("usage: automark --seed N [--flower K] --json out.json [--species file.json] [--relax N] [--preview] [--frames] [--open N]");
        exit(2);
    };
    let bud = grow_bud(PlantSeed(seed), flower, &species, &settings);
    let r = &bud.report;
    eprintln!(
        "{} seed {} flower {} (vigour {:.2}): {} layers, {} petals, {} stages, growth mean {:.2} max {:.2}, envelope {:.2} -> {:.2}, max overlap {:.3}t, max band excess {:.3}t",
        species.name, seed, flower, bud.flower.vigour, bud.traits.layers, r.petals, r.stages, r.mean_growth, r.max_growth, r.envelope_start, r.envelope_end, r.max_overlap, r.max_band_excess
    );
    let doc = match open {
        Some(frames) => {
            let opening = bloom::open_flower(&bud, &BloomSettings { frames, ..BloomSettings::default() });
            let last = opening.frames.last().expect("at least one opening frame");
            eprintln!("opened over {} frames, final max overlap {:.3}t", opening.frames.len(), last.max_overlap);
            bloom::export::flower_json(&bud, &opening)
        }
        None => export::flower_json(&bud),
    };
    if let Some(path) = json {
        std::fs::write(&path, doc).expect("write JSON");
        eprintln!("wrote {path}");
    }
}
