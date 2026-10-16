# automark-core

Rust core for Automark Bloom. Stage 1 (bud formation) is built; flowering is next.

```
cargo run --release -- --seed 11 --json bud.json            # peony preset
cargo run --release -- --seed 11 --json bud.json --species dahlia.json
cargo test --release
```

Library entry points:
- `grow_bud(PlantSeed, &Species, &PackSettings) -> Bud`
- `export::flower_json(&Bud) -> String`: the shared flower JSON (`viewer/FORMAT.md`), canonical and byte-identical across platforms.
- `grow_bud_json(seed: u64) -> String`: peony preset with default settings, for wasm and seed tests.

How a bud is grown (`src/lib.rs` has the overview):
1. `traits`/`species`: the plant seed draws one value from each range in a `Species` (only place species numbers live).
2. `shells`: nested egg-shaped shells, one slot per petal layer; slot width = max thickness x (1 + crumple room).
3. `ca`: stochastic cellular automaton on each shell's surface: primordia spaced by inhibition (staggered against the layer below), then petals compete for cells.
4. `sheet`: each petal patch becomes a 7 x 11 spline control grid with a thickness per point, capped at `max_thickness`.
5. `pack`: rest lengths are grown by `growth_factor`, then FIRE gradient descent on `energy` (springs with bending ~ t^3, layer slot, sepal envelope, point-triangle contact at (t_i + t_j)/2). Gradient is exact and checked by a finite-difference test.

Determinism: one specified PRNG (`rng`), all transcendental math through libm (`math`), no hash-order or threaded sums.
