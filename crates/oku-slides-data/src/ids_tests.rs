use std::cell::Cell;

use speculoos::prelude::*;

use super::*;

#[test]
fn new_id_skips_taken_ids() {
    let refused = Cell::new(0);
    let taken = |_: &str| {
        refused.set(refused.get() + 1);
        refused.get() <= 3
    };
    let id = new_id(&taken);
    assert_that!(refused.get()).is_equal_to(4);
    assert_that!(id.len()).is_equal_to(NEW_ID_CHARS);
}

#[test]
fn new_id_coerces_to_an_id_generator() {
    let ids: &mut IdGen<'_> = &mut new_id;
    let id = ids(&|_| false);
    assert_that!(valid_id(&id)).is_true();
}

#[rstest::rstest]
#[case::taken_once(&["a"], "a", "a_2")]
#[case::suffix_taken(&["a", "a_2"], "a", "a_3")]
#[case::long_id_cut(&["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"],
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa_2")]
#[case::many_suffixes_taken(&["a", "a_2", "a_3", "a_4", "a_5", "a_6", "a_7", "a_8", "a_9", "a_10", "a_11"], "a", "a_12")]
#[case::invalid_starts_from_x(&[], "a/b", "x")]
#[case::invalid_and_x_taken(&["x"], "a/b", "x_2")]
#[case::free_id_kept(&["b"], "a", "a")]
fn repair_id_gives_smallest_free_suffix(#[case] taken: &[&str], #[case] id: &str, #[case] expected: &str) {
    let mut repair = IdRepair::new();
    for t in taken {
        assert_that!(repair.repair_id(t)).is_equal_to(t.to_string());
    }
    let got = repair.repair_id(id);
    assert_that!(got.as_str()).is_equal_to(expected);
    assert_that!(valid_id(&got)).is_true();
    assert_that!(got.len() <= MAX_ID_CHARS).is_true();
}

#[test]
fn repair_10000_duplicates_is_linear() {
    let mut repair = IdRepair::new();
    let ids: std::collections::HashSet<String> = (0..10_000).map(|_| repair.repair_id("r")).collect();
    assert_that!(ids.len()).is_equal_to(10_000);
    assert_that!(repair.probes()).is_less_than_or_equal_to(30_000);
}

/// Distinct long ids sharing their cut stem: one counter per stem keeps the repair linear.
#[test]
fn repair_10000_distinct_long_ids_sharing_a_stem_is_linear() {
    let mut repair = IdRepair::new();
    let inputs: Vec<String> = (0..10_000).map(|i| format!("{}{i:04}", "a".repeat(60))).collect();
    let mut out = std::collections::HashSet::new();
    for id in inputs.iter().chain(&inputs) {
        let got = repair.repair_id(id);
        assert_that!(valid_id(&got)).is_true();
        out.insert(got);
    }
    assert_that!(out.len()).is_equal_to(20_000);
    assert_that!(repair.probes()).is_less_than_or_equal_to(3 * 20_000);
}

/// A long id's stem at two-digit suffixes is a shorter id's stem at one-digit ones: the
/// counters never mix them, so the shorter id still gets its smallest free suffix.
#[test]
fn repair_short_id_sharing_a_long_ids_stem_gets_smallest_free_suffix() {
    let mut repair = IdRepair::new();
    let long = "a".repeat(64);
    for _ in 0..11 {
        repair.repair_id(&long);
    }
    let short = "a".repeat(61);
    assert_that!(repair.repair_id(&short)).is_equal_to(short.clone());
    assert_that!(repair.repair_id(&short)).is_equal_to(format!("{short}_2"));
}

#[test]
fn repair_is_the_same_on_every_run() {
    let run = || {
        let mut repair = IdRepair::new();
        ["a", "a", "a_2", "", "x", "a"].iter().map(|id| repair.repair_id(id)).collect::<Vec<_>>()
    };
    assert_that!(run()).is_equal_to(run());
    assert_that!(run())
        .is_equal_to(["a", "a_2", "a_2_2", "x", "x_2", "a_3"].iter().map(ToString::to_string).collect::<Vec<_>>());
}
