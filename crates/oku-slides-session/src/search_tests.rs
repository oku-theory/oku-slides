use speculoos::prelude::*;

use super::*;

#[test]
fn fold_removes_case_and_accents() {
    assert_that!(fold("Éditer à la Main")).is_equal_to("editer a la main".to_owned());
}

#[test]
fn fold_spells_ligatures_out() {
    assert_that!(fold("Cœur")).is_equal_to("coeur".to_owned());
}

#[test]
fn excerpt_of_a_short_line_is_the_line() {
    assert_that!(excerpt("Fusion sans conflit", "conflit")).is_equal_to(Some("Fusion sans conflit".to_owned()));
}

#[test]
fn excerpt_without_the_needle_is_none() {
    assert_that!(excerpt("Fusion sans conflit", "grille")).is_none();
}

#[test]
fn excerpt_of_a_long_line_keeps_the_match_with_ellipses() {
    let line = format!("{} cœur du sujet {}", "a".repeat(120), "b".repeat(120));
    let out = excerpt(&line, "coeur").unwrap();
    assert_that!(out.contains("cœur")).is_true();
    assert_that!(out.starts_with('…') && out.ends_with('…')).is_true();
    assert_that!(out.chars().count()).is_less_than_or_equal_to(SEARCH_EXCERPT + 2);
}
