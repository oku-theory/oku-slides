//! Les éléments dans l'arbre du deck : lecture, réparations, doublons.

use serde_json::json;
use speculoos::prelude::*;

use loro::{LoroMap, LoroValue, TreeParentId};

use super::tree_tests::*;
use super::*;
use crate::Deck;
use oku_slides_i18n::Message;

#[test]
fn read_element_without_owner_entry_joins_the_first_sibling_slot() {
    let r = new(&two_slots());
    let x = forge(&r, "s-constat", None, &text_meta("x"), Some(&[]));
    let (tree, points) = node_of(&r.doc, "s-constat", "b-points");
    ok(tree.mov_before(x, points));
    r.doc.commit();
    let (read, issues) = reread(&r);
    assert_that!(ids_in(&read, "s-constat", None, "blocks")).is_equal_to(vec!["x".to_owned(), "b-points".into()]);
    assert_that!(ids_in(&read, "s-constat", None, "elements")).is_equal_to(vec!["n".to_owned()]);
    assert_that!(issue_ids(&issues)).is_equal_to(vec!["issue-slot-repaired"]);
    assert_that!((issues[0].slide.as_deref(), issues[0].element.as_deref()))
        .is_equal_to((Some("s-constat"), Some("x")));
}

/// Règle 3b : la seule entrée désigne un nœud vivant (`b-points`), pas le parent.
#[test]
fn read_element_with_only_another_owner_entry_keeps_that_slot() {
    let r = new(&roadmap());
    let points = node_of(&r.doc, "s-constat", "b-points").1.to_string();
    forge(&r, "s-title", None, &text_meta("x"), Some(&[(points.as_str(), "children")]));
    let (read, issues) = reread(&r);
    assert_that!(ids_in(&read, "s-title", None, "children")).is_equal_to(vec!["x".to_owned()]);
    assert_that!(issue_ids(&issues)).is_equal_to(vec!["issue-slot-repaired"]);
}

/// Règle 3a : la seule entrée désigne un nœud absent ; son slot est pris sans `Issue`.
#[test]
fn read_element_with_only_an_absent_parent_entry_keeps_that_slot_silently() {
    let r = new(&roadmap());
    forge(&r, "s-title", None, &text_meta("x"), Some(&[("7@1", "children")]));
    let (read, issues) = reread(&r);
    assert_that!(ids_in(&read, "s-title", None, "children")).is_equal_to(vec!["x".to_owned()]);
    assert_that!(issues).is_empty();
}

#[test]
fn read_element_without_any_slot_goes_to_orphans() {
    let r = new(&roadmap());
    forge(&r, "s-title", None, &text_meta("x"), None);
    let (read, issues) = reread(&r);
    assert_that!(ids_in(&read, "s-title", None, ORPHANS)).is_equal_to(vec!["x".to_owned()]);
    assert_that!(issue_ids(&issues)).is_equal_to(vec!["issue-slot-repaired"]);
}

#[test]
fn read_unknown_slot_name_is_kept_as_a_param() {
    let r = new(&roadmap());
    forge(&r, "s-constat", None, &text_meta("x"), Some(&[("", "zzz")]));
    let (read, issues) = reread(&r);
    assert_that!(issues).is_empty();
    assert_that!(ids_in(&read, "s-constat", None, "zzz")).is_equal_to(vec!["x".to_owned()]);
    let found = crate::validate(&read, &crate::test_registry::registry(), None);
    assert_that!(issue_ids(&found).contains(&"issue-unknown-param")).is_true();
}

#[test]
fn read_slot_clashing_with_a_param_keeps_the_elements() {
    let mut r = new(&roadmap());
    ok(as_map(slide_map(&r, "s-constat").get("params")).insert("blocks", "texte"));
    r.doc.commit();
    let (read, issues) = reread(&r);
    assert_that!(ids_in(&read, "s-constat", None, "blocks")).is_equal_to(vec!["b-points".to_owned()]);
    assert_that!(issue_ids(&issues)).is_equal_to(vec!["issue-param-type"]);
    assert_that!(issues[0].message).is_equal_to(Message::new("issue-param-type").arg("param", "blocks"));
    assert_that!(issues[0].slide.as_deref()).is_equal_to(Some("s-constat"));
    apply(&mut r, &read);
    assert_that!(as_map(slide_map(&r, "s-constat").get("params")).get("blocks").is_none()).is_true();
    assert_that!(r.deck().0).is_equal_to(&read);
}

#[test]
fn read_unreadable_element_drops_its_subtree_only() {
    let r = new(&roadmap());
    let meta = vec![("id", LoroValue::from("x")), ("kind", LoroValue::I64(42))];
    forge(&r, "s-constat", None, &meta, Some(&[("", "blocks")]));
    forge(&r, "s-constat", Some("x"), &text_meta("y"), Some(&[("x", "children")]));
    let (read, issues) = reread(&r);
    assert_that!(ids_in(&read, "s-constat", None, "blocks")).is_equal_to(vec!["b-points".to_owned()]);
    assert_that!(read.slide("s-constat").and_then(|s| s.locate("y")).is_none()).is_true();
    assert_that!(issue_ids(&issues)).is_equal_to(vec!["issue-unreadable-element"]);
    assert_that!(issues[0].element.as_deref()).is_equal_to(Some("x"));
}

/// La méta d'un texte `id` cadré (largeur `w`), pour distinguer deux copies.
fn framed_meta(id: &str, w: u32) -> Vec<(&str, LoroValue)> {
    let mut meta = text_meta(id);
    meta.push(("frame", super::super::write::json_to_loro(&json!({ "x": 0, "y": 0, "w": w, "h": 10 }))));
    meta
}

fn blocks_slot() -> &'static [(&'static str, &'static str)] {
    &[("", "blocks")]
}

fn elements_slot() -> &'static [(&'static str, &'static str)] {
    &[("", "elements")]
}

fn no_duplicate_id(deck: &Deck) {
    let found = crate::validate(deck, &crate::test_registry::registry(), None);
    assert_that!(issue_ids(&found).contains(&"issue-duplicate-element-id")).is_false();
}

#[test]
fn read_exact_copies_in_one_slot_keep_one() {
    let r = new(&roadmap());
    forge(&r, "s-constat", None, &text_meta("x"), Some(blocks_slot()));
    forge(&r, "s-constat", None, &text_meta("x"), Some(blocks_slot()));
    let (read, issues) = reread(&r);
    assert_that!(ids_in(&read, "s-constat", None, "blocks")).is_equal_to(vec!["b-points".to_owned(), "x".into()]);
    assert_that!(issues).is_empty();
    no_duplicate_id(&read);
}

#[test]
fn read_copies_in_two_slots_rename_the_second() {
    let r = new(&roadmap());
    forge(&r, "s-constat", None, &text_meta("x"), Some(blocks_slot()));
    forge(&r, "s-constat", None, &framed_meta("x", 10), Some(elements_slot()));
    let (read, issues) = reread(&r);
    assert_that!(ids_in(&read, "s-constat", None, "blocks")).is_equal_to(vec!["b-points".to_owned(), "x".into()]);
    assert_that!(ids_in(&read, "s-constat", None, "elements")).is_equal_to(vec!["x-2".to_owned()]);
    assert_that!(issues).is_empty();
    no_duplicate_id(&read);
}

#[test]
fn read_diverged_copies_rename_the_later_occurrence() {
    let r = new(&roadmap());
    forge(&r, "s-constat", None, &text_meta("x"), Some(blocks_slot()));
    forge(&r, "s-constat", None, &framed_meta("x", 77), Some(blocks_slot()));
    let (read, issues) = reread(&r);
    assert_that!(ids_in(&read, "s-constat", None, "blocks")).is_equal_to(vec![
        "b-points".to_owned(),
        "x".into(),
        "x-2".into(),
    ]);
    assert_that!(issues).is_empty();
    let slide = read.slide("s-constat").unwrap_or_else(|| panic!("s-constat absente"));
    let frame = |id: &str| slide.locate(id).unwrap_or_else(|| panic!("{id} absent")).element.frame;
    assert_that!(frame("x").is_none()).is_true();
    assert_that!(frame("x-2").map(|f| f.w)).is_equal_to(Some(77.0));
    no_duplicate_id(&read);
}

#[test]
fn duplicate_ids_round_trip_keeps_the_edited_node() {
    let deck = deck_of(json!({ "slides": [{ "id": "s", "kind": "essentiel/content", "params": {
        "blocks": [text("x", "A"), text("x", "B")] } }] }));
    let mut r = new(&deck);
    let tree = slide_tree(&r, "s");
    let s = TreeParentId::Node(slide_node(&r, "s"));
    let first = tree.children(s).unwrap_or_default()[0];
    let (read, issues) = reread(&r);
    assert_that!(issues).is_empty();
    assert_that!(ids_in(&read, "s", None, "blocks")).is_equal_to(vec!["x".to_owned(), "x-2".into()]);
    let text_of = |deck: &Deck, id: &str| {
        let slide = deck.slide("s").unwrap_or_else(|| panic!("s absente"));
        first_text(slide.locate(id).unwrap_or_else(|| panic!("{id} absent")).element)
    };
    assert_that!(text_of(&read, "x")).is_equal_to("A".to_owned());
    assert_that!(text_of(&read, "x-2")).is_equal_to("B".to_owned());

    apply(&mut r, &read);
    assert_that!(r.deck().0).is_equal_to(&read);
    assert_that!(reread(&r)).is_equal_to((read.clone(), Vec::new()));
    assert_that!(text_of(&r.deck().0, "x-2")).is_equal_to("B".to_owned());
    assert_that!(tree.children(s).unwrap_or_default()[0]).is_equal_to(first);
    let settled = ops_of(&r);
    apply(&mut r, &read);
    assert_that!(ops_of(&r)).is_equal_to(settled);
}

#[test]
fn read_slides_of_another_type_reads_no_slide() {
    let r = new(&roadmap());
    ok(r.doc.get_map(super::super::ROOT).insert_container("slides", LoroMap::new()));
    r.doc.commit();
    // Défense en profondeur : l'import et la fusion refusent ce replica, la lecture seule l'ignore.
    let (read, issues) = ok(super::super::read_deck(&r.doc));
    assert_that!(read.slides.is_empty()).is_true();
    assert_that!(read.params).is_equal_to(&roadmap().params);
    assert_that!(issues).is_empty();
}
