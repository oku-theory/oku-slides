//! t-902 : le pack du deck seul d'un import hi-fi s'édite dans l'atelier.
//!
//! Avant t-902, `edit_templates` le refusait (`DeckOnly`) : ses modèles ne
//! sont pas dans la bibliothèque. Désormais l'atelier part de la version
//! embarquée, et ses enregistrements restent dans le deck, jamais dans la
//! bibliothèque.

use oku_slides_core::{Namespace, Op, PackScope};
use oku_slides_session::{PackManifest, Request};
use speculoos::prelude::*;

use crate::common::data_dir;
use crate::hifi_import::{imported_hifi, library_files};

#[test]
fn workshop_on_an_imported_deck_only_pack_saves_in_the_deck_and_never_in_the_library() {
    let data = data_dir();
    let mut s = imported_hifi(&data);
    let pack = s.deck().widgets.get(&Namespace::deck()).cloned().unwrap_or_else(|| panic!("pack de l'import"));
    assert_that!(s.snapshot().deck_pack).is_true();
    let before = library_files(&data, "widgets");

    s.edit_templates(pack.id).unwrap_or_else(|e| panic!("atelier : {}", e.one_line()));
    let rename = serde_json::json!({ "op": "set_deck_param", "key": "title", "value": "Modèles importés" });
    let rename: Op = serde_json::from_value(rename).unwrap_or_else(|e| panic!("{e}"));
    s.apply("Nom", vec![rename], None).unwrap_or_else(|e| panic!("renommage : {}", e.one_line()));
    s.save_templates(false).unwrap_or_else(|e| panic!("enregistrement : {}", e.one_line()));
    s.close_templates().unwrap_or_else(|e| panic!("sortie : {}", e.one_line()));

    assert_that!(library_files(&data, "widgets")).is_equal_to(before.clone());
    let after = s.deck().widgets.get(&Namespace::deck()).cloned().unwrap_or_else(|| panic!("pack du deck"));
    assert_that!(after.id).is_equal_to(pack.id);
    assert_that!(after.version.clone()).is_not_equal_to(pack.version.clone());
    assert_that!(s.snapshot().deck_pack).is_true();

    // Enregistré puis rouvert : le deck garde sa version, du deck seul.
    let path = data.join("importe.oku-slides.pdf");
    s.handle(Request::Save { path: Some(path.clone()), overwrite: false })
        .unwrap_or_else(|e| panic!("enregistrement du deck : {}", e.one_line()));
    let mut reopened = crate::common::session_in(&data);
    reopened.open(&path).unwrap_or_else(|e| panic!("réouverture : {}", e.one_line()));
    assert_that!(reopened.deck().widgets.get(&Namespace::deck()).cloned()).is_equal_to(Some(after.clone()));
    let files = reopened.sources_for_tests().packs.get(&after.version).cloned();
    let manifest = files.map(|f| PackManifest::from_files(&f).unwrap_or_else(|e| panic!("pack.json : {e}")));
    let manifest = manifest.unwrap_or_else(|| panic!("version embarquée"));
    assert_that!((manifest.scope, manifest.name.as_str())).is_equal_to((PackScope::Deck, "Modèles importés"));
    assert_that!(library_files(&data, "widgets")).is_equal_to(before);
}
