//! Run `BLESS=1 cargo test -p automark-checks` after a change that is meant to
//! alter flowers, and commit the updated `golden/digests.txt` with it.

use automark_checks::{digest, digests, format_table, SEEDS};
use std::path::PathBuf;
use std::sync::OnceLock;

/// Reference digests, grown once and shared by every test here, since each
/// bud takes seconds.
fn reference() -> &'static [String] {
    static DIGESTS: OnceLock<Vec<String>> = OnceLock::new();
    DIGESTS.get_or_init(|| digests(&SEEDS))
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("golden/digests.txt")
}

#[test]
fn same_seed_same_bud() {
    // Regrown on this thread, while the reference was grown on its own thread.
    let seed = SEEDS[0];
    assert_eq!(
        digest(seed),
        reference()[0],
        "seed {seed} gave two different buds in one run"
    );
}

#[test]
fn different_seeds_differ() {
    let mut sorted = reference().to_vec();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        SEEDS.len(),
        "two reference seeds gave the same bud"
    );
}

#[test]
fn matches_golden_digests() {
    let actual = format_table(&SEEDS, reference());
    let path = golden_path();
    if std::env::var_os("BLESS").is_some() {
        std::fs::write(&path, &actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing {}; run with BLESS=1 to create it", path.display()));
    if expected.replace("\r\n", "\n") != actual {
        let diff: Vec<String> = expected
            .lines()
            .zip(actual.lines())
            .filter(|(e, a)| e != a)
            .map(|(e, a)| format!("  expected {e}\n  actual   {a}"))
            .collect();
        panic!(
            "flowers changed for some seeds:\n{}\nIf this change is intended, re-run with BLESS=1 and commit golden/digests.txt.",
            diff.join("\n")
        );
    }
}
