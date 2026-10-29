use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::test_support::{add_version, library_with, manifest_ns};

#[rstest]
#[case::plain("Acme", "acme")]
#[case::accents("Éléments à l'œil", "elements-a-l-oeil")]
#[case::spaces("  Mes   cartes  ", "mes-cartes")]
#[case::leading_digit("2026 rapport", "pack-2026-rapport")]
#[case::nothing_usable("!!!", "pack")]
#[case::non_latin("報告", "pack")]
fn slug_name_gives_a_valid_segment(#[case] name: &str, #[case] expected: &str) {
    assert_that!(slug(name, MAX, "pack").as_str()).is_equal_to(expected);
    assert_that!(Namespace::parse(expected)).is_some();
}

#[rstest]
#[case::non_latin("テーマ", "theme")]
#[case::leading_digit("2026", "theme-2026")]
fn slug_with_theme_prefix_puts_it_in_front(#[case] name: &str, #[case] expected: &str) {
    assert_that!(slug(name, MAX, "theme")).is_equal_to(expected.to_owned());
}

#[test]
fn slug_long_name_is_cut_without_trailing_dash() {
    let name = format!("{} fin", "a".repeat(62));
    let out = slug(&name, MAX, "pack");
    assert_that!(out.len() <= MAX && !out.ends_with('-')).is_true();
}

#[test]
fn library_namespace_free_name_is_the_slug() {
    let lib = library_with(&[]);
    assert_that!(library_namespace(&lib, "Mes cartes", "pack").unwrap().to_string())
        .is_equal_to("mes-cartes".to_owned());
}

#[test]
fn library_namespace_taken_name_gets_a_suffix() {
    let lib = library_with(&[("cartes", &[]), ("cartes-2", &[])]);
    assert_that!(library_namespace(&lib, "Cartes", "pack").unwrap().to_string()).is_equal_to("cartes-3".to_owned());
}

#[test]
fn library_namespace_of_a_builtin_name_is_deduplicated() {
    let lib = library_with(&[]);
    assert_that!(library_namespace(&lib, "Essentiel", "pack").unwrap().to_string())
        .is_equal_to("essentiel-2".to_owned());
}

#[test]
fn library_namespace_reserved_deck_is_never_given() {
    let lib = library_with(&[]);
    assert_that!(library_namespace(&lib, "Deck", "pack").unwrap().to_string()).is_equal_to("deck-2".to_owned());
}

#[test]
fn library_namespace_new_pack_saves_without_namespace_taken() {
    let lib = library_with(&[("cartes", &[])]);
    let ns = library_namespace(&lib, "Cartes", "pack").unwrap();
    add_version(&lib, &manifest_ns(uuid::Uuid::new_v4(), ns.as_str(), &[]), &[]);
    assert_that!(lib.by_namespace(&ns)).is_some();
}

#[test]
fn rename_namespace_moves_slide_and_nested_elements_of_that_namespace_only() {
    let mut slide: Slide = serde_json::from_value(serde_json::json!({
        "id": "s", "kind": "acme/cover", "params": { "blocks": [
            { "id": "a", "kind": "acme/card", "params": { "inner": [{ "id": "b", "kind": "acme/note" }] } },
            { "id": "c", "kind": "autre/card" },
            { "id": "d", "kind": oku_slides_core::kinds::base::TEXT }
        ] }
    }))
    .unwrap();
    rename_namespace(&mut slide, &Namespace::parse("acme").unwrap(), &Namespace::parse("copie").unwrap());
    let kinds: Vec<String> =
        std::iter::once(slide.kind.clone()).chain(slide.walk().map(|l| l.element.kind.clone())).collect();
    assert_that!(kinds).contains_all_of(&vec![
        &"copie/cover".to_owned(),
        &"copie/card".to_owned(),
        &"copie/note".to_owned(),
        &"autre/card".to_owned(),
        &oku_slides_core::kinds::base::TEXT.to_owned(),
    ]);
}
