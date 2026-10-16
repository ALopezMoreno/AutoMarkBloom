//! Run `BLESS=1 cargo test -p automark-checks` after a change that is meant to
//! alter flowers, and commit the updated `golden/digests.txt` with it.

use automark_checks::{bud_bytes, digest_table, SEEDS};
use std::path::PathBuf;

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("golden/digests.txt")
}

#[test]
fn same_seed_same_bud() {
    for &seed in &SEEDS[..3] {
        assert!(bud_bytes(seed) == bud_bytes(seed), "seed {seed} gave two different buds in one run");
    }
}

#[test]
fn different_seeds_differ() {
    let a = bud_bytes(SEEDS[0]);
    let distinct = SEEDS[1..].iter().filter(|&&s| bud_bytes(s) != a).count();
    assert_eq!(distinct, SEEDS.len() - 1, "some seeds gave the same bud as seed {}", SEEDS[0]);
}

#[test]
fn matches_golden_digests() {
    let actual = digest_table();
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
