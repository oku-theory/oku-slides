use speculoos::prelude::*;

use super::*;

#[test]
fn functions_table_is_sorted_for_binary_search() {
    let names: Vec<&str> = FUNCTIONS.iter().map(|(name, ..)| *name).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_that!(names).is_equal_to(sorted);
}

#[test]
fn lookup_ignores_case_and_refuses_unlisted() {
    assert_that!(lookup("SUM")).is_equal_to(Some(Function { kind: WINDOW_AGGREGATE, min: 1, max: 1 }));
    assert_that!(lookup("string_agg")).is_none();
}
