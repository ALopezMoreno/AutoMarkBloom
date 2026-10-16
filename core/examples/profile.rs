use automark_core::pack::build_system;
use automark_core::{grow_bud, PackSettings, PlantSeed, Species};
use std::time::Instant;
fn main() {
    let bud = grow_bud(PlantSeed(11), &Species::peony(), &PackSettings { iterations: 1, ..Default::default() });
    let (sys, x, _) = build_system(&bud.sheets, &bud.shells, &bud.traits);
    let reach = bud.traits.max_thickness * 3.5;
    let t = Instant::now();
    let c = sys.candidates(&x, reach);
    println!("nodes {} tris {} springs {} cands {} candidates() {:?}", x.len(), sys.tris.len(), sys.springs.len(), c.len(), t.elapsed());
    let mut g = vec![[0.0; 3]; x.len()];
    let t = Instant::now();
    for _ in 0..20 { sys.energy(&x, &c, &mut g); }
    println!("energy() {:?}", t.elapsed() / 20);
}
