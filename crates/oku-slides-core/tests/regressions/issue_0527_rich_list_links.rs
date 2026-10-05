//! La validation ne parcourait que les paramètres rich de premier niveau : un lien
//! `javascript:` ou une pastille 0 dans une colonne rich d'un item de liste passait.

use oku_slides_core::{Deck, validate};
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common;

fn issues_of(run: Value) -> Vec<(Option<String>, Option<String>, String)> {
    let deck = json!({ "version": 3, "slides": [{ "id": "s", "kind": "content", "params": { "blocks": [
        { "id": "n", "kind": "liste", "params": { "etapes": [{ "id": "a", "note": [{ "runs": [run] }] }] } }
    ] } }] });
    let deck = Deck::from_json(&deck.to_string()).unwrap_or_else(|e| panic!("deck : {e}"));
    validate(&deck, &common::list_registry(), None)
        .into_iter()
        .map(|i| (i.slide.clone(), i.element.clone(), i.message.id().to_owned()))
        .collect()
}

#[test]
fn validate_javascript_link_in_a_list_rich_cell_is_reported_on_its_element() {
    let issues = issues_of(json!({ "t": "x", "link": "javascript:alert(1)" }));
    assert_that!(issues).is_equal_to(vec![(
        Some("s".to_owned()),
        Some("n".to_owned()),
        "issue-unsupported-link".to_owned(),
    )]);
}

#[test]
fn validate_zero_highlight_in_a_list_rich_cell_is_reported_on_its_element() {
    let issues = issues_of(json!({ "t": "x", "highlight": 0 }));
    assert_that!(issues).is_equal_to(vec![(
        Some("s".to_owned()),
        Some("n".to_owned()),
        "issue-highlight-zero".to_owned(),
    )]);
}

#[test]
fn validate_https_link_in_a_list_rich_cell_has_no_issue() {
    assert_that!(issues_of(json!({ "t": "x", "link": "https://oku.test" }))).is_empty();
}
