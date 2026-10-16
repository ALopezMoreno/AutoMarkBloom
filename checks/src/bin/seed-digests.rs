//! Prints `<seed> <sha256>` for the reference seeds. CI runs this as
//! WebAssembly and compares it with `golden/digests.txt`; native builds are
//! checked by the tests.
//!
//! `seed-digests K/N` prints only shard K of N (every N-th seed from K), so
//! single-threaded WebAssembly runs can be split across processes.

use automark_checks::{digests, format_table, SEEDS};

fn main() {
    let seeds: Vec<u64> = match std::env::args().nth(1) {
        Some(shard) => {
            let (k, n) = shard.split_once('/').expect("shard must look like K/N");
            let (k, n): (usize, usize) = (k.parse().unwrap(), n.parse().unwrap());
            SEEDS.iter().copied().skip(k).step_by(n).collect()
        }
        None => SEEDS.to_vec(),
    };
    print!("{}", format_table(&seeds, &digests(&seeds)));
}
