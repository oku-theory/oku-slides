use std::path::PathBuf;

use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;
use uuid::Uuid;

use super::TabDocument;

const PACK: Uuid = Uuid::from_u128(0xa7e11e40_0000_4000_8000_000000000001);

fn deck() -> TabDocument {
    TabDocument::Deck { path: Some(PathBuf::from("/d/a.oku-slides.pdf")), deck_uid: Some("u-1".to_owned()) }
}

#[rstest]
#[case::deck(deck(), json!({"kind": "deck", "path": "/d/a.oku-slides.pdf", "deck_uid": "u-1"}))]
#[case::deck_without_file(
    TabDocument::Deck { path: None, deck_uid: None },
    json!({"kind": "deck", "path": null, "deck_uid": null})
)]
#[case::help(TabDocument::Help { locale: "fr".to_owned() }, json!({"kind": "help", "locale": "fr"}))]
#[case::pack(TabDocument::Pack { pack: PACK }, json!({"kind": "pack", "pack": "a7e11e40-0000-4000-8000-000000000001"}))]
#[case::theme(TabDocument::Theme { theme: PACK }, json!({"kind": "theme", "theme": "a7e11e40-0000-4000-8000-000000000001"}))]
fn tab_document_serializes_with_its_kind(#[case] document: TabDocument, #[case] expected: serde_json::Value) {
    assert_that!(serde_json::to_value(&document).unwrap()).is_equal_to(&expected);
    assert_that!(serde_json::from_value::<TabDocument>(expected).unwrap()).is_equal_to(&document);
}

#[rstest]
#[case::deck(deck(), Some("/d/a.oku-slides.pdf"), Some("u-1"), None)]
#[case::help(TabDocument::Help { locale: "fr".to_owned() }, None, None, Some("fr"))]
#[case::pack(TabDocument::Pack { pack: PACK }, None, None, None)]
#[case::theme(TabDocument::Theme { theme: PACK }, None, None, None)]
fn tab_document_path_only_for_a_deck(
    #[case] document: TabDocument,
    #[case] path: Option<&str>,
    #[case] uid: Option<&str>,
    #[case] locale: Option<&str>,
) {
    assert_that!(document.path()).is_equal_to(path.map(std::path::Path::new));
    assert_that!(document.deck_uid()).is_equal_to(uid);
    assert_that!(document.help_locale()).is_equal_to(locale);
}

#[test]
fn deck_pack_tab_document_round_trips_serde() {
    let document = TabDocument::DeckPack { deck_tab: "t-1".to_owned(), pack: PACK };
    let expected = json!({"kind": "deck_pack", "deck_tab": "t-1", "pack": "a7e11e40-0000-4000-8000-000000000001"});
    assert_that!(serde_json::to_value(&document).unwrap()).is_equal_to(&expected);
    assert_that!(serde_json::from_value::<TabDocument>(expected).unwrap()).is_equal_to(&document);
}

#[test]
fn deck_pack_tab_document_names_only_its_owner() {
    let document = TabDocument::DeckPack { deck_tab: "t-1".to_owned(), pack: PACK };
    assert_that!(document.deck_tab()).is_equal_to(Some("t-1"));
    assert_that!((document.path(), document.deck_uid(), document.help_locale())).is_equal_to((None, None, None));
    assert_that!(deck().deck_tab()).is_none();
}

#[test]
fn tab_document_unknown_kind_is_an_error() {
    let parsed = serde_json::from_value::<TabDocument>(json!({"kind": "slides", "path": "/x"}));
    assert_that!(parsed).is_err();
}
