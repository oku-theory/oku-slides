//! t-937: a deck-only pack removed from its deck could not be added back.
//!
//! The packs panel's add choices came from the library, which never had it;
//! its versions stay in the deck's sources. They are now offered from there,
//! and adding one pins the embedded version without writing the library.

use oku_slides_core::{Namespace, Op, PackKind};
use oku_slides_session::{PacksEvent, Session};
use serde_json::json;
use speculoos::prelude::*;

use crate::common::data_dir;
use crate::hifi_import::library_files;
use crate::workspace_support::{import_hifi, on, workspace};

/// Replaces every slide of `s` with one `essentiel` slide, so the deck's own layouts are unused.
fn only_an_essentiel_slide(s: &mut Session) {
    let add =
        json!({ "op": "add_slide", "slide": { "id": "s-seule", "kind": oku_slides_core::kinds::essentiel::CONTENT } });
    let mut ops: Vec<Op> = vec![serde_json::from_value(add).unwrap_or_else(|e| panic!("{e}"))];
    ops.extend(s.deck().slides.iter().map(|x| Op::RemoveSlide { slide: x.id.clone() }));
    s.apply("slides", ops, None).unwrap_or_else(|e| panic!("slides: {}", e.one_line()));
}

#[test]
fn issue_0937_removed_deck_only_layouts_are_offered_and_added_back_without_the_library() {
    let data = data_dir();
    let ws = workspace(&data);
    let deck = import_hifi(&ws, &data);
    let ns = Namespace::deck_layouts();
    let pin = on(&ws, &deck, |s| s.deck().layouts.get(&ns).cloned()).unwrap_or_else(|| panic!("import pack"));
    on(&ws, &deck, |s| {
        only_an_essentiel_slide(s);
        s.remove_deck_pack(PackKind::Layouts, &ns).unwrap_or_else(|e| panic!("removal: {}", e.one_line()));
    });
    let offered = on(&ws, &deck, |s| s.deck_packs().map(|v| v.addable.get(&PackKind::Layouts).cloned()));
    let offered = offered.unwrap_or_else(|e| panic!("packs: {}", e.one_line())).unwrap_or_default();
    assert_that!(offered.iter().any(|(n, _)| *n == ns)).is_true();

    let before = library_files(&data, "layouts");
    on(&ws, &deck, |s| s.packs_action(PacksEvent::AddPack { kind: PackKind::Layouts, namespace: ns.clone() }))
        .unwrap_or_else(|e| panic!("add: {}", e.one_line()));
    assert_that!(on(&ws, &deck, |s| s.deck().layouts.get(&ns).cloned())).is_equal_to(Some(pin));
    assert_that!(library_files(&data, "layouts")).is_equal_to(before);
}
