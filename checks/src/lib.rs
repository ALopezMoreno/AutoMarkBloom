//! Seed determinism: a plant seed must always produce the same bud, bit for
//! bit, in every build (Linux, macOS, Windows, WebAssembly) and every run.
//!
//! The check hashes the core's canonical output (the flower JSON the viewer
//! and glTF export read), so anything that changes what they receive changes
//! the digest.

use sha2::{Digest, Sha256};

/// Reference seeds. Spread across the u64 range on purpose: small, large and
/// bit-pattern seeds catch mistakes in seed derivation.
pub const SEEDS: [u64; 8] = [0, 1, 2, 7, 42, 1234, 0xDEAD_BEEF, u64::MAX];

/// Canonical output for `seed`: flower 0, peony species, default settings,
/// as flower JSON.
pub fn bud_bytes(seed: u64) -> Vec<u8> {
    automark_core::grow_bud_json(seed, 0).into_bytes()
}

pub fn digest(seed: u64) -> String {
    let hash = Sha256::digest(bud_bytes(seed));
    hash.iter().map(|b| format!("{b:02x}")).collect()
}

/// One `<seed> <sha256>` line per reference seed.
pub fn digest_table() -> String {
    SEEDS.iter().map(|&s| format!("{s} {}\n", digest(s))).collect()
}
