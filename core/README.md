# automark-core

Rust core for Automark Bloom. Stage 1 (bud formation and development) is built; opening is next.

```
cargo run --release -- --seed 11 --json bud.json            # peony preset, flower 0
cargo run --release -- --seed 11 --flower 3 --json bud.json # a sibling on the same plant
cargo run --release -- --seed 11 --preview --json bud.json  # about 2.5x faster, less settled
cargo run --release -- --seed 11 --json bud.json --species dahlia.json
cargo run --release -- --seed 11 --json bud.json --frames      # include the bud's growth stages
cargo test --release
```

Library entry points:
- `grow_bud(PlantSeed, flower_index, &Species, &PackSettings) -> Bud`: traits per plant; bud automaton, growth, packing and latents per flower.
- `export::flower_json(&Bud) -> String`: the shared flower JSON (`viewer/FORMAT.md`), canonical and byte-identical across platforms.
- `grow_bud_json(seed: u64, flower_index: u64) -> String`: peony preset with default settings, for wasm and seed tests.
- `grow_bud_preview_json(seed, flower_index)`: the same with `PackSettings::preview()` (wasm `bud_preview_json`). Same plant, petal count and layout, but the packing is less settled, so geometry differs slightly and it has no golden digest.

How a bud is grown (`src/lib.rs` has the overview):
1. `traits`/`species`: the plant seed draws one value from each range in a `Species` (only place species numbers live).
2. `shells`: nested egg-shaped shells, one slot per petal layer; slot width = max thickness x (1 + crumple room).
3. `ca`: stochastic cellular automaton on each shell's surface: primordia spaced by inhibition (staggered against the layer below), then petals compete for cells.
4. `sheet`: each petal patch becomes an 11 x 19 spline control grid with a thickness per point, capped at `max_thickness`.
5. `pack` + `growth`: the bud develops over growth stages. Each stage a stochastic automaton on every petal's grid decides where it grows (active patches spread, contact pressure slows growth, margins grow faster, increments diffuse); spring rest lengths follow the growth; then preconditioned FIRE gradient descent on `energy` relaxes all nodes plus the sepal radius (each node gets a mass equal to its local stiffness, so stiff contacts don't throttle the soft folding; see PERFORMANCE.md). Energy terms: springs (bending ~ t^3 x `petal_stiffness`), a loose layer band that spreads as the bud swells, sepal envelope with its own stiffness, point-triangle contact at (t_i + t_j)/2. Gradient (including the envelope radius) is exact and checked by a finite-difference test.

Determinism: one specified PRNG (`rng`), all transcendental math through libm (`math`), no hash-order or threaded sums.

Latents for the opening stage are named channels (`channels`, `latents`): petal scope `target_length`, `target_width`, `edge_growth`, `cup`, `base_fold`, `onset`, `bud_growth`, `petal_type` (labelled), `twist`; flower scope `vigour`, `sepal_radius`. Each channel draws from its own sub-stream keyed by its name, so adding one never shifts the others.
