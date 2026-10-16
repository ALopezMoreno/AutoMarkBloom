# automark-core

Rust core for Automark Bloom. Stage 1 (bud formation and development) and stage 2 (opening) are built.

```
cargo run --release -- --seed 11 --json bud.json            # peony preset, flower 0
cargo run --release -- --seed 11 --flower 3 --json bud.json # a sibling on the same plant
cargo run --release -- --seed 11 --preview --json bud.json  # about 2.5x faster, less settled
cargo run --release -- --seed 11 --json bud.json --species dahlia.json
cargo run --release -- --seed 11 --json bud.json --frames      # include the bud's growth stages
cargo run --release -- --seed 11 --json flower.json --frames --open 24  # bud stages, then 24 opening frames
cargo test --release
```

Library entry points:
- `grow_bud(PlantSeed, flower_index, &Species, &PackSettings) -> Bud`: traits per plant; bud automaton, growth, packing and latents per flower.
- `export::flower_json(&Bud) -> String`: the shared flower JSON (`viewer/FORMAT.md`), canonical and byte-identical across platforms.
- `grow_bud_json(seed: u64, flower_index: u64) -> String`: peony preset with default settings, for wasm and seed tests.
- `grow_bud_preview_json(seed, flower_index)`: the same with `PackSettings::preview()` (wasm `bud_preview_json`). Same plant, petal count and layout, but the packing is less settled, so geometry differs slightly and it has no golden digest.
- `bloom::open_flower(&Bud, &BloomSettings) -> Opening` and `bloom::export::flower_json(&Bud, &Opening)`: the opening, appended to the bud's JSON as frames with `stage: "open"` and t in (0, 1].

How a bud is grown (`src/lib.rs` has the overview):
1. `traits`/`species`: the plant seed draws one value from each range in a `Species` (only place species numbers live).
2. `shells`: nested shells, one slot per petal layer; slot width = max thickness x (1 + crumple room). All shells share one outline: the aspect, plus the `bud_egg` and `bud_point` handles (broader base, pointed tip). The core's radius is the `core_size` trait.
3. `ca`: stochastic cellular automaton on each shell's surface: primordia spaced by inhibition (staggered against the layer below), then petals compete for cells.
4. `sheet`: each petal patch becomes an 11 x 19 spline control grid with a thickness per point, capped at `max_thickness`.
5. `pack` + `growth`: the bud develops over growth stages. Each stage a stochastic automaton on every petal's grid decides where it grows (active patches spread, contact pressure slows growth, margins grow faster, increments diffuse; the coin flips read a noise field that is spatially correlated over `growth_correlation` nodes and persists between stages by `growth_persistence`, and `growth_anisotropy` splits growth between petal length and width); spring rest lengths follow the growth; then preconditioned FIRE gradient descent on `energy` relaxes all nodes plus the sepal radius (each node gets a mass equal to its local stiffness, so stiff contacts don't throttle the soft folding; see PERFORMANCE.md). Energy terms: springs (bending ~ t^3 x `petal_stiffness`), a loose layer band that spreads as the bud swells, sepal envelope with its own stiffness, point-triangle contact at (t_i + t_j)/2. Gradient (including the envelope radius) is exact and checked by a finite-difference test.

Determinism: one specified PRNG (`rng`), all transcendental math through libm (`math`), no hash-order or threaded sums.

Latents for the opening stage are named channels (`channels`, `latents`): petal scope `target_length`, `target_width`, `edge_growth`, `cup`, `base_fold`, `onset`, `bud_growth`, `petal_type` (labelled), `twist`; flower scope `vigour`, `sepal_radius`. Each channel draws from its own sub-stream keyed by its name, so adding one never shifts the others.

How a flower opens (`src/bloom/mod.rs` has the overview). Opening is a Markov chain over the whole flower, seeded per flower:
1. `bloom::channels`: every latent is a named channel (flower, petal or point scope; 1 or 3 numbers). Each follows one rule: fixed, mean-reverting drift, or growth toward `gain * target + bias` along a smoothstep window starting at its onset (monotone for sizes). Rule parameters are constants or other channels' names, so channels are wired together by data. Each (channel, petal) has its own random stream keyed by the channel's name. Channels the bud hands over with no rule are carried as fixed and exported.
2. `bloom::program`: the default wiring (data, not code): a drifting per-petal `rate` around the flower's `vigour`; `length`, `width`, `edge`, `cupping`, `reflex`, `asymmetry`, `memory`, `wrap`, `ruffle` and `splay` grow from the bud's latents on each petal's own clock (`onset`, outer layers first). How each layer ends up is the species setting `opening`: `angle` (how far petals open, radians from straight up) and `curl` (how much of the bud's inward curl they keep), each for the innermost and outermost layer, with `profile` shaping the layers between. Peony keeps its inner petals incurved over the centre and opens its guard petals out to the sepals, just past horizontal. Any `derived` channel can take such a `power` profile.
3. `bloom::consume`: the only code that knows what channels mean. It turns them into a pose per petal: the bud petal's centre curve is re-integrated with its curl scaled by `memory` and folded back at the base by `reflex`, rows are widened, flattened (`wrap`) and cupped, and the bud's crumples are re-applied in the new frame; margins get extra rest length (`edge`) so they ruffle, and thickness thins with area.
4. `bloom::relax` + `bloom::contact`: each step moves the petals with their poses, then FIRE relaxes springs, bending, a soft pull to the pose, contact on densely sampled petals, and the stamen core. Exact gradient, checked by a finite-difference test.
