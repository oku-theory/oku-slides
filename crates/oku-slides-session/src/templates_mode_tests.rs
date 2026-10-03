use speculoos::prelude::*;

use crate::library::builtin_versions;
use crate::pack::{DEFAULT_PACK, PackManifest};
use crate::test_support::{fresh_dir, library_of, session_on};

#[test]
fn save_templates_creates_a_version_whose_parent_is_the_old_current() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    let builtin = builtin_versions()[0].version.clone();
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.apply(
        "Nom",
        vec![
            serde_json::from_value(serde_json::json!(
        { "op": "set_deck_param", "key": "title", "value": "Essentiel à moi" }))
            .unwrap(),
        ],
        None,
    )
    .unwrap();
    s.save_templates(false).unwrap();
    let lib = library_of(&s);
    let current = lib.current_version(DEFAULT_PACK).unwrap();
    assert_that!(current.manifest.parent.clone()).is_equal_to(Some(builtin));
    assert_that!(current.manifest.name.as_str()).is_equal_to("Essentiel à moi");
    // Modifier un pack intégré : la version de l'utilisateur, sous l'id du pack intégré.
    assert_that!(
        data.path()
            .join("templates")
            .join(DEFAULT_PACK.to_string())
            .join("versions")
            .join(current.version.hex())
            .join("pack.json")
            .exists()
    )
    .is_true();
    s.close_templates().unwrap();
    assert_that!(s.deck().templates.as_ref().map(|r| r.version.clone())).is_equal_to(Some(current.version.clone()));
    assert_that!(s.sources_for_tests().packs.contains_key(&current.version)).is_true();
}

/// Une version par session d'édition : les enregistrements suivants la remplacent.
#[test]
fn saving_twice_in_one_session_keeps_one_version_and_the_deck_follows_in_one_undo_step() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    let before = s.deck().templates.clone();
    let builtin = builtin_versions()[0].version.clone();
    s.edit_templates(DEFAULT_PACK).unwrap();
    let rename = |t: &str| {
        serde_json::from_value(serde_json::json!(
        { "op": "set_deck_param", "key": "title", "value": t }))
        .unwrap()
    };
    s.apply("Nom", vec![rename("Un")], None).unwrap();
    s.save_templates(false).unwrap();
    let first = library_of(&s).current(DEFAULT_PACK).unwrap();
    s.apply("Nom", vec![rename("Deux")], None).unwrap();
    s.save_templates(false).unwrap();
    let second = library_of(&s).current_version(DEFAULT_PACK).unwrap();
    // Intégrée + une seule version de la session ; la première a disparu.
    assert_that!(library_of(&s).versions(DEFAULT_PACK).len()).is_equal_to(2);
    assert_that!(library_of(&s).version(DEFAULT_PACK, &first)).is_none();
    assert_that!(second.manifest.parent.clone()).is_equal_to(Some(builtin));
    assert_that!(s.snapshot().editing.and_then(|e| e.session_version.clone()))
        .is_equal_to(Some(second.version.clone()));
    s.close_templates().unwrap();
    assert_that!(s.deck().templates.as_ref().map(|r| r.version.clone())).is_equal_to(Some(second.version));
    s.undo().unwrap();
    assert_that!(s.deck().templates.clone()).is_equal_to(before);
}

/// Quitter puis revenir : une nouvelle session, donc une nouvelle version, fille de la précédente.
#[test]
fn a_new_editing_session_creates_a_second_version() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    let rename = |t: &str| {
        serde_json::from_value(serde_json::json!(
        { "op": "set_deck_param", "key": "title", "value": t }))
        .unwrap()
    };
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.apply("Nom", vec![rename("Un")], None).unwrap();
    s.save_templates(false).unwrap();
    let first = library_of(&s).current(DEFAULT_PACK).unwrap();
    s.close_templates().unwrap();
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.apply("Nom", vec![rename("Deux")], None).unwrap();
    s.save_templates(false).unwrap();
    let second = library_of(&s).current_version(DEFAULT_PACK).unwrap();
    assert_that!(library_of(&s).versions(DEFAULT_PACK).len()).is_equal_to(3);
    assert_that!(second.manifest.parent).is_equal_to(Some(first));
}

#[test]
fn save_templates_as_copy_creates_a_new_id_and_the_session_continues_on_it() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.save_templates(true).unwrap();
    let edit = s.snapshot().editing.cloned().unwrap();
    assert_that!(edit.set).is_not_equal_to(DEFAULT_PACK);
    let copy = library_of(&s).current_version(edit.set).unwrap();
    assert_that!(copy.manifest.parent.clone()).is_none();
    assert_that!(copy.manifest.name.as_str()).is_equal_to("Essentiel (copie)");
    assert_that!(library_of(&s).current(DEFAULT_PACK)).is_equal_to(Some(builtin_versions()[0].version.clone()));
    // Les enregistrements suivants remplacent la version de la copie, toujours sans parente.
    s.apply(
        "Nom",
        vec![
            serde_json::from_value(serde_json::json!(
        { "op": "set_deck_param", "key": "title", "value": "Ma charte" }))
            .unwrap(),
        ],
        None,
    )
    .unwrap();
    s.save_templates(false).unwrap();
    let again = library_of(&s).current_version(edit.set).unwrap();
    assert_that!(library_of(&s).versions(edit.set).len()).is_equal_to(1);
    assert_that!(again.manifest.parent).is_none();
    assert_that!(again.manifest.name.as_str()).is_equal_to("Ma charte");
}

#[test]
fn edit_templates_of_an_unknown_pack_returns_unknown_pack() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    let err = s.edit_templates(uuid::Uuid::new_v4()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::UnknownPack);
}

#[test]
fn templates_view_of_the_deck_pack_shows_the_embedded_version() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    let embedded = s.deck().templates.clone().unwrap();
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.apply(
        "Nom",
        vec![
            serde_json::from_value(serde_json::json!(
        { "op": "remove_slide", "slide": "texte" }))
            .unwrap(),
        ],
        None,
    )
    .unwrap();
    s.save_templates(false).unwrap();
    // Le deck mis de côté est passé à la nouvelle version : on revient à l'ancienne.
    s.close_templates().unwrap();
    s.undo().unwrap();
    let view = s.templates_view(None).unwrap();
    assert_that!(view.set.version.clone()).is_equal_to(embedded.version.clone());
    let manifest = PackManifest::from_files(&builtin_versions()[0].files).unwrap();
    assert_that!(view.templates.len()).is_equal_to(manifest.templates.len());
}

/// Chaque session d'édition a sa propre étape d'annulation dans le deck.
#[test]
fn two_editing_sessions_make_two_undo_steps_in_the_deck() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    let rename = |t: &str| {
        serde_json::from_value(serde_json::json!({ "op": "set_deck_param", "key": "title", "value": t })).unwrap()
    };
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.apply("Nom", vec![rename("Un")], None).unwrap();
    s.save_templates(false).unwrap();
    let first = library_of(&s).current(DEFAULT_PACK).unwrap();
    s.close_templates().unwrap();
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.apply("Nom", vec![rename("Deux")], None).unwrap();
    s.save_templates(false).unwrap();
    s.close_templates().unwrap();
    s.undo().unwrap();
    assert_that!(s.deck().templates.as_ref().map(|r| r.version.clone())).is_equal_to(Some(first));
}

/// Le titre du deck des modèles devient `t`.
fn rename(t: &str) -> oku_slides_core::Op {
    serde_json::from_value(serde_json::json!({ "op": "set_deck_param", "key": "title", "value": t })).unwrap()
}

#[test]
fn a_save_after_a_turn_begun_in_deck_mode_keeps_the_turn_version() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = crate::test_support::opened_on(data.path());
    let set = s.deck().templates.clone().unwrap().id;
    // Le tour groupe le deck, qui passe de côté pendant le tour.
    s.begin_turn("modifie les modèles");
    s.edit_templates(set).unwrap();
    s.apply("Nom", vec![rename("Un")], None).unwrap();
    s.save_templates(false).unwrap();
    s.end_turn();
    let turn = library_of(&s).current(set).unwrap();
    // Après le tour : nouvelle étape du deck mis de côté, pas de fusion.
    s.apply("Nom", vec![rename("Deux")], None).unwrap();
    s.save_templates(false).unwrap();
    let after = library_of(&s).current(set).unwrap();
    assert_that!(after).is_not_equal_to(turn.clone());
    assert_that!(library_of(&s).version(set, &turn).is_some()).is_true();
    assert_that!(s.doc.sources.packs.contains_key(&turn)).is_true();
    let crate::Mode::Templates { stash, .. } = &s.mode else { panic!("mode modèles attendu") };
    assert_that!(stash.sources.packs.contains_key(&turn)).is_true();
    // Annuler revient à la version du tour, présente dans les sources.
    s.close_templates().unwrap();
    s.undo().unwrap();
    assert_that!(s.deck().templates.as_ref().map(|r| r.version.clone())).is_equal_to(Some(turn));
}
