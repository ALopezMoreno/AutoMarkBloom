//! Deterministic data parallelism.
//!
//! Work over `0..n` is cut into a fixed number of chunks that depends only
//! on `n`, never on how many cores there are. Threads take chunks as they
//! free up, and the results are handed back in chunk order. A caller that
//! joins them in that order and does its floating-point sums afterwards, in
//! one thread, gets the same bits on any machine. On wasm, or with
//! `AUTOMARK_THREADS=1`, the chunks simply run one after another.

use std::ops::Range;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

/// Below this many items a call runs in the calling thread: spawning costs
/// more than it saves.
const MIN_PARALLEL: usize = 2048;

/// Chunks per call. Enough for threads to balance uneven chunks.
const CHUNKS: usize = 64;

/// Worker threads: `AUTOMARK_THREADS` if set, else the core count, at most 16.
pub fn threads() -> usize {
    static N: OnceLock<usize> = OnceLock::new();
    *N.get_or_init(|| {
        if cfg!(target_family = "wasm") {
            return 1;
        }
        std::env::var("AUTOMARK_THREADS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, |n| n.get()))
            .clamp(1, 16)
    })
}

/// `f` over the fixed chunks of `0..n`, results in chunk order.
pub fn map_chunks<T: Send>(n: usize, f: impl Fn(Range<usize>) -> T + Sync) -> Vec<T> {
    let chunks = if n < MIN_PARALLEL { 1 } else { CHUNKS };
    let range = |i: usize| i * n / chunks..(i + 1) * n / chunks;
    let workers = threads().min(chunks);
    if workers <= 1 {
        return (0..chunks).map(|i| f(range(i))).collect();
    }
    let next = AtomicUsize::new(0);
    let work = || {
        let mut mine = Vec::new();
        loop {
            let i = next.fetch_add(1, Ordering::Relaxed);
            if i >= chunks {
                return mine;
            }
            mine.push((i, f(range(i))));
        }
    };
    let mut done: Vec<(usize, T)> = std::thread::scope(|s| {
        let helpers: Vec<_> = (1..workers).map(|_| s.spawn(work)).collect();
        let mut all = work();
        for h in helpers {
            all.extend(h.join().expect("worker thread panicked"));
        }
        all
    });
    done.sort_unstable_by_key(|&(i, _)| i);
    done.into_iter().map(|(_, t)| t).collect()
}

/// `f` over the fixed chunks of `0..n`, each producing items, joined in order.
pub fn flat_map_chunks<T: Send>(n: usize, f: impl Fn(Range<usize>) -> Vec<T> + Sync) -> Vec<T> {
    let parts = map_chunks(n, f);
    let mut out = Vec::with_capacity(parts.iter().map(Vec::len).sum());
    for p in parts {
        out.extend(p);
    }
    out
}
