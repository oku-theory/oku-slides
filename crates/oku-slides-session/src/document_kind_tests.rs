use rstest::rstest;
use speculoos::prelude::*;

use super::DocumentKind;
use crate::pack::ESSENTIEL_PACK;
use crate::test_support::{fresh_dir, session_on};
use crate::theme::PAPIER_THEME;

#[test]
fn document_kind_of_a_deck_tab_is_deck() {
    let data = fresh_dir("oku-slides-kind-");
    let s = session_on(data.path());
    assert_that!(s.document_kind()).is_equal_to(DocumentKind::Deck);
}

#[test]
fn document_kind_in_history_is_deck() {
    let data = fresh_dir("oku-slides-kind-");
    let mut s = session_on(data.path());
    s.history_open().unwrap();
    assert_that!(s.document_kind()).is_equal_to(DocumentKind::Deck);
}

#[test]
fn document_kind_of_a_help_tab_is_help() {
    let data = fresh_dir("oku-slides-kind-");
    let mut s = session_on(data.path());
    s.open_help("fr", None).unwrap();
    assert_that!(s.document_kind()).is_equal_to(DocumentKind::Help);
}

#[test]
fn document_kind_of_a_pack_tab_is_pack() {
    let data = fresh_dir("oku-slides-kind-");
    let mut s = session_on(data.path());
    s.open_pack_tab_here(ESSENTIEL_PACK, None).unwrap();
    assert_that!(s.document_kind()).is_equal_to(DocumentKind::Pack);
}

#[test]
fn document_kind_of_a_theme_tab_is_theme() {
    let data = fresh_dir("oku-slides-kind-");
    let mut s = session_on(data.path());
    s.open_theme_here(PAPIER_THEME).unwrap();
    assert_that!(s.document_kind()).is_equal_to(DocumentKind::Theme);
}

#[rstest]
#[case::deck(DocumentKind::Deck, "\"deck\"")]
#[case::help(DocumentKind::Help, "\"help\"")]
#[case::pack(DocumentKind::Pack, "\"pack\"")]
#[case::theme(DocumentKind::Theme, "\"theme\"")]
fn document_kind_serializes_as_its_name(#[case] kind: DocumentKind, #[case] json: &str) {
    assert_that!(serde_json::to_string(&kind).unwrap().as_str()).is_equal_to(json);
    assert_that!(format!("\"{}\"", kind.as_str()).as_str()).is_equal_to(json);
}
