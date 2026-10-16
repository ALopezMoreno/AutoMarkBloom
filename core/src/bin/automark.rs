//! automark --seed N --json out.json [--species species.json] [--iterations N]

use automark_core::{export, grow_bud, PackSettings, PlantSeed, Species};
use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut seed: Option<u64> = None;
    let mut json: Option<String> = None;
    let mut species = Species::peony();
    let mut settings = PackSettings::default();
    let mut i = 0;
    let value = |i: usize| -> &str {
        args.get(i + 1).map(|s| s.as_str()).unwrap_or_else(|| {
            eprintln!("missing value for {}", args[i]);
            exit(2)
        })
    };
    while i < args.len() {
        match args[i].as_str() {
            "--seed" => seed = Some(value(i).parse().expect("--seed takes a u64")),
            "--json" => json = Some(value(i).to_string()),
            "--species" => {
                let text = std::fs::read_to_string(value(i)).expect("read species file");
                species = Species::from_json(&text).expect("parse species file");
            }
            "--iterations" => settings.iterations = value(i).parse().expect("--iterations takes a number"),
            other => {
                eprintln!("unknown argument {other}\nusage: automark --seed N --json out.json [--species file.json] [--iterations N]");
                exit(2);
            }
        }
        i += 2;
    }
    let Some(seed) = seed else {
        eprintln!("usage: automark --seed N --json out.json [--species file.json] [--iterations N]");
        exit(2);
    };
    let bud = grow_bud(PlantSeed(seed), &species, &settings);
    let r = &bud.report;
    eprintln!(
        "{} seed {}: {} layers, {} petals, energy {:.3} -> {:.3}, max overlap {:.3}t, max slot excess {:.3}t",
        species.name, seed, bud.traits.layers, r.petals, r.start.total(), r.end.total(), r.max_overlap, r.max_band_excess
    );
    if let Some(path) = json {
        std::fs::write(&path, export::flower_json(&bud)).expect("write JSON");
        eprintln!("wrote {path}");
    }
}
