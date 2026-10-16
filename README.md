# AutoMarkBloom

Procedural peonies grown from a seed: a cellular automaton lays out the bud,
petals are thick spline sheets packed by smooth energy minimisation, and a
Markov chain opens the flower.

## Layout

| Path | What it is |
|---|---|
| `core/` | Rust core: shells, automaton, petal sheets, packing, flower JSON output |
| `wasm/` | Browser build of the core (`wasm-pack build wasm --target web`) |
| `checks/` | Seed determinism tests and the `seed-digests` tool |
| `viewer/` | TypeScript + three.js viewer and glTF export |
| `tools/` | Python helpers for inspecting core output |
| `prototype-python/` | The original 2D Python prototype, kept for reference |
| `docs/` | LaTeX documentation of how each model stage works (`make -C docs`) |

## Building

```sh
cargo test --workspace --release                     # native build and all tests
wasm-pack build wasm --release --target web     # browser package in wasm/pkg
```

The Rust toolchain is pinned in `rust-toolchain.toml`.

## Same seed, same flower

A seed must give the same bud, bit for bit, in every build. CI enforces it:

- `checks/golden/digests.txt` lists a SHA-256 of the exported bud for each reference seed.
- CI checks those digests on Linux, macOS and Windows (in the tests) and under WebAssembly, and fails if any differ from the file.
- When a change is meant to alter flowers, run `BLESS=1 cargo test -p automark-checks` and commit the updated file with the change, so the diff shows which seeds changed.

Rules the core follows to keep this true:

- All randomness comes from `core/src/rng.rs` (PCG32 seeded through SplitMix64), never a library or OS RNG.
- Transcendental maths (`exp`, `ln`, `sin`, `cos`, `pow`, `atan2`, ...) goes through the `libm` crate, since platform maths libraries differ in the last bit. `sqrt` and basic arithmetic are exact everywhere.
- No hash-map iteration order or multithreaded float sums in anything that reaches the output.
