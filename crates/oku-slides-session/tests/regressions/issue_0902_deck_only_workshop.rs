//! t-902: an imported hi-fi deck's own pack is editable, saved in the deck only.
//!
//! Before t-902, editing it was refused (`DeckOnly`): its layouts are not in the
//! library. It now opens from the embedded version, in a tab linked to its deck
//! (t-1228), and its saves stay in the deck, never in the library.

use oku_slides_core::{Namespace, Op, PackScope};
use oku_slides_session::{PackManifest, Request, Workspace};
use speculoos::prelude::*;

use crate::common::data_dir;
use crate::hifi_import::library_files;
use crate::workspace_support::{import_hifi, on, open_pack, workspace};

#[test]
fn deck_pack_tab_on_an_imported_deck_only_pack_saves_in_the_deck_and_never_in_the_library() {
    let data = data_dir();
    let ws = workspace(&data);
    let deck = import_hifi(&ws, &data);
    let deck_pack = |ws: &std::sync::Mutex<Workspace>| {
        on(ws, &deck, |s| s.deck().layouts.get(&Namespace::deck_layouts()).cloned())
            .unwrap_or_else(|| panic!("pack de l'import"))
    };
    let pack = deck_pack(&ws);
    assert_that!(on(&ws, &deck, |s| s.snapshot().deck_pack)).is_true();
    let before = library_files(&data, "layouts");

    let tab = open_pack(&ws, &deck, pack.id);
    let rename = serde_json::json!({ "op": "set_deck_param", "key": "title", "value": "Modèles importés" });
    let rename: Op = serde_json::from_value(rename).unwrap_or_else(|e| panic!("{e}"));
    on(&ws, &tab, |s| s.apply("Nom", vec![rename], None)).unwrap_or_else(|e| panic!("renommage : {}", e.one_line()));
    let (saved, _) = Workspace::request(&ws, &tab, Request::SavePack { as_new: false });
    saved.unwrap_or_else(|e| panic!("enregistrement : {}", e.one_line()));

    assert_that!(library_files(&data, "layouts")).is_equal_to(before.clone());
    let after = deck_pack(&ws);
    assert_that!(after.id).is_equal_to(pack.id);
    assert_that!(after.version.clone()).is_not_equal_to(pack.version.clone());
    assert_that!(on(&ws, &deck, |s| s.snapshot().deck_pack)).is_true();

    // Enregistré puis rouvert : le deck garde sa version, du deck seul.
    let path = data.join("importe.oku-slides.pdf");
    on(&ws, &deck, |s| s.handle(Request::Save { path: Some(path.clone()), overwrite: false }))
        .unwrap_or_else(|e| panic!("enregistrement du deck : {}", e.one_line()));
    let mut reopened = crate::common::session_in(&data);
    reopened.open(&path).unwrap_or_else(|e| panic!("réouverture : {}", e.one_line()));
    assert_that!(reopened.deck().layouts.get(&Namespace::deck_layouts()).cloned()).is_equal_to(Some(after.clone()));
    let files = reopened.sources_for_tests().packs.get(&after.version).cloned();
    let manifest = files.map(|f| PackManifest::from_files(&f).unwrap_or_else(|e| panic!("pack.json : {e}")));
    let manifest = manifest.unwrap_or_else(|| panic!("version embarquée"));
    assert_that!((manifest.scope, manifest.name.as_str())).is_equal_to((PackScope::Deck, "Modèles importés"));
    assert_that!(library_files(&data, "layouts")).is_equal_to(before);
}
