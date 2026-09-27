use std::path::{Path, PathBuf};

use speculoos::prelude::*;

use super::*;

#[test]
fn safe_stem_replaces_path_separators() {
    assert_that!(safe_stem("Q1/Q2 : plan", "x")).is_equal_to("Q1-Q2 - plan".to_owned());
}

#[test]
fn safe_stem_of_blank_title_uses_fallback() {
    assert_that!(safe_stem("  \n ", "Sans titre")).is_equal_to("Sans titre".to_owned());
}

#[test]
fn safe_stem_drops_leading_dots() {
    assert_that!(safe_stem("..cache", "x")).is_equal_to("cache".to_owned());
}

#[test]
fn safe_stem_cuts_very_long_titles() {
    assert_that!(safe_stem(&"a".repeat(500), "x").chars().count()).is_equal_to(MAX_STEM_CHARS);
}

#[test]
fn free_path_skips_taken_names() {
    let taken = |p: &Path| p.ends_with("Budget.oku-slides.pdf") || p.ends_with("Budget 2.oku-slides.pdf");
    assert_that!(free_path(Path::new("/d"), "Budget", taken)).is_equal_to(PathBuf::from("/d/Budget 3.oku-slides.pdf"));
}
