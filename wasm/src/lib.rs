//! Browser entry points. Build with `wasm-pack build wasm --release --target web`.
//!
//! Seeds cross into JavaScript as `BigInt` because a plant seed is a full u64.

use wasm_bindgen::prelude::*;

/// The packed bud of flower `flower_index` on plant `seed`, as flower JSON
/// (see viewer/FORMAT.md).
#[wasm_bindgen]
pub fn bud_json(seed: u64, flower_index: u32) -> String {
    automark_core::grow_bud_json(seed, flower_index as u64)
}

/// A faster, less settled version of `bud_json` for browsing: same plant,
/// petal count and layout, slightly different geometry.
#[wasm_bindgen]
pub fn bud_preview_json(seed: u64, flower_index: u32) -> String {
    automark_core::grow_bud_preview_json(seed, flower_index as u64)
}

/// SHA-256 of `bud_json(seed, 0)`; matches `checks/golden/digests.txt` when the
/// browser build agrees with native builds.
#[wasm_bindgen]
pub fn bud_digest(seed: u64) -> String {
    automark_checks::digest(seed)
}
