//! Browser entry points. Build with `wasm-pack build wasm --release --target web`.
//!
//! Seeds cross into JavaScript as `BigInt` because a plant seed is a full u64.

use wasm_bindgen::prelude::*;

/// The packed bud for `seed` as flower JSON (see viewer/FORMAT.md).
#[wasm_bindgen]
pub fn bud_json(seed: u64) -> String {
    automark_core::grow_bud_json(seed)
}

/// SHA-256 of `bud_json(seed)`; matches `checks/golden/digests.txt` when the
/// browser build agrees with native builds.
#[wasm_bindgen]
pub fn bud_digest(seed: u64) -> String {
    automark_checks::digest(seed)
}
