use automark_core::pack::build_system;
use automark_core::{grow_bud, PackSettings, PlantSeed, Species};
use std::time::Instant;
fn main() {
    let bud = grow_bud(PlantSeed(11), &Species::peony(), &PackSettings::quick());
    let (sys, x, _) = build_system(&bud.sheets, &bud.shells, &bud.traits);
    let reach = bud.traits.max_thickness * 3.5;
    let t = Instant::now();
    let c = sys.candidates(&x, reach);
    println!("nodes {} tris {} springs {} cands {} candidates() {:?}", x.len(), sys.tris.len(), sys.springs.len(), c.len(), t.elapsed());
    let mut g = vec![[0.0; 3]; x.len()];
    let mut ge = 0.0;
    let env = automark_core::energy::Envelope { r: bud.shells.envelope, rest: bud.shells.envelope };
    let t = Instant::now();
    for _ in 0..20 { sys.energy(&x, env, &c, &mut g, &mut ge); }
    println!("energy() {:?}", t.elapsed() / 20);
}
