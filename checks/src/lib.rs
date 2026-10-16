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

/// Digests of `seeds`, in order. Each seed is grown on its own thread where
/// threads exist; a bud is single-threaded, so this cannot change its bits.
pub fn digests(seeds: &[u64]) -> Vec<String> {
    #[cfg(not(target_family = "wasm"))]
    {
        std::thread::scope(|scope| {
            let handles: Vec<_> = seeds
                .iter()
                .map(|&s| scope.spawn(move || digest(s)))
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().expect("bud thread panicked"))
                .collect()
        })
    }
    #[cfg(target_family = "wasm")]
    {
        seeds.iter().map(|&s| digest(s)).collect()
    }
}

/// `<seed> <sha256>` lines for `seeds`.
pub fn format_table(seeds: &[u64], digests: &[String]) -> String {
    seeds
        .iter()
        .zip(digests)
        .map(|(s, d)| format!("{s} {d}\n"))
        .collect()
}

/// One `<seed> <sha256>` line per reference seed.
pub fn digest_table() -> String {
    format_table(&SEEDS, &digests(&SEEDS))
}
