use oku_slides_core::{Op, PackScope, WidgetFile, kinds};
use serde_json::json;
use speculoos::prelude::*;

use crate::pack::{ESSENTIEL_PACK, PackManifest};
use crate::test_support::{
    adopt_deck_only_pack, builtin, edit_of, fresh_dir, library_files, library_of, opened_on, pin, rename_title,
    session, session_on, stash_of,
};

/// L'op qui règle le titre de la slide `slide` à `title`.
fn set_title(slide: &str, title: &str) -> Op {
    serde_json::from_value(json!({ "op": "set_param", "target": { "slide": slide }, "param": "title", "value": title }))
        .unwrap()
}

/// Le manifeste de la version `version` qu'embarque le document de `s`.
fn saved_manifest(s: &crate::Session, version: &oku_slides_core::PackVersion) -> PackManifest {
    let files = s.doc.sources.packs.get(version).unwrap_or_else(|| panic!("version enregistrée embarquée"));
    PackManifest::from_files(files).unwrap()
}

#[test]
fn save_pack_writes_the_edited_title_into_the_layout_example() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.apply("t", vec![set_title("layout-content", "Nouveau")], None).unwrap();
    let saved = s.save_pack(true).unwrap();
    let m = saved_manifest(&s, &saved);
    assert_that!(m.examples[&WidgetFile::parse("content").unwrap()].get("title")).is_equal_to(Some(&json!("Nouveau")));
}

#[test]
fn save_pack_requires_the_namespaces_its_examples_use() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    let saved = s.save_pack(true).unwrap();
    let m = saved_manifest(&s, &saved);
    assert_that!(m.requires).is_equal_to(vec![kinds::base::namespace()]);
}

#[test]
fn save_pack_leaves_neutral_values_out_of_the_examples() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    let schema = s.registry.get(kinds::essentiel::CHAPTER).cloned().unwrap();
    let neutral = crate::widget_preview::preview_params(&schema, &serde_json::Map::new(), &s.localizer);
    let title = neutral.text("title").map(str::to_owned).unwrap();
    s.apply("t", vec![set_title("layout-chapter", &title)], None).unwrap();
    let saved = s.save_pack(true).unwrap();
    let m = saved_manifest(&s, &saved);
    assert_that!(m.examples[&WidgetFile::parse("chapter").unwrap()].get("title")).is_none();
}

#[test]
fn save_pack_creates_a_version_whose_parent_is_the_old_current() {
    let data = fresh_dir("oku-slides-pack-mode-");
    let mut s = session_on(data.path());
    let builtin = builtin(ESSENTIEL_PACK).version.clone();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.apply("Nom", vec![rename_title("Essentiel à moi")], None).unwrap();
    s.save_pack(false).unwrap();
    let lib = library_of(&s);
    let current = lib.current_version(ESSENTIEL_PACK).unwrap();
    assert_that!(current.manifest.parent.clone()).is_equal_to(Some(builtin));
    assert_that!(current.manifest.name.as_str()).is_equal_to("Essentiel à moi");
    // Modifier un pack intégré : la version de l'utilisateur, sous l'id du pack intégré.
    let dir = data.path().join("layouts").join(ESSENTIEL_PACK.to_string()).join("versions");
    assert_that!(dir.join(current.version.hex()).join("layouts.json").exists()).is_true();
    s.close_pack().unwrap();
    assert_that!(pin(&s, "essentiel").map(|r| r.version)).is_equal_to(Some(current.version.clone()));
    assert_that!(s.sources_for_tests().packs.contains_key(&current.version)).is_true();
}

/// Une version par session d'édition : les enregistrements suivants la remplacent.
#[test]
fn saving_twice_in_one_session_keeps_one_version_and_the_deck_follows_in_one_undo_step() {
    let data = fresh_dir("oku-slides-pack-mode-");
    let mut s = session_on(data.path());
    let before = pin(&s, "essentiel");
    let builtin = builtin(ESSENTIEL_PACK).version.clone();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.apply("Nom", vec![rename_title("Un")], None).unwrap();
    s.save_pack(false).unwrap();
    let first = library_of(&s).current(ESSENTIEL_PACK).unwrap();
    s.apply("Nom", vec![rename_title("Deux")], None).unwrap();
    s.save_pack(false).unwrap();
    let second = library_of(&s).current_version(ESSENTIEL_PACK).unwrap();
    // Intégrée + une seule version de la session ; la première a disparu.
    assert_that!(library_of(&s).versions(ESSENTIEL_PACK).len()).is_equal_to(2);
    assert_that!(library_of(&s).version(ESSENTIEL_PACK, &first)).is_none();
    assert_that!(second.manifest.parent.clone()).is_equal_to(Some(builtin));
    assert_that!(edit_of(&s).session_version).is_equal_to(Some(second.version.clone()));
    s.close_pack().unwrap();
    assert_that!(pin(&s, "essentiel").map(|r| r.version)).is_equal_to(Some(second.version));
    s.undo().unwrap();
    assert_that!(pin(&s, "essentiel")).is_equal_to(before);
}

/// Quitter puis revenir : une nouvelle session, donc une nouvelle version, fille de la précédente.
#[test]
fn a_new_editing_session_creates_a_second_version() {
    let data = fresh_dir("oku-slides-pack-mode-");
    let mut s = session_on(data.path());
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.apply("Nom", vec![rename_title("Un")], None).unwrap();
    s.save_pack(false).unwrap();
    let first = library_of(&s).current(ESSENTIEL_PACK).unwrap();
    s.close_pack().unwrap();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.apply("Nom", vec![rename_title("Deux")], None).unwrap();
    s.save_pack(false).unwrap();
    let second = library_of(&s).current_version(ESSENTIEL_PACK).unwrap();
    assert_that!(library_of(&s).versions(ESSENTIEL_PACK).len()).is_equal_to(3);
    assert_that!(second.manifest.parent).is_equal_to(Some(first));
}

#[test]
fn save_pack_as_copy_creates_a_new_id_and_the_session_continues_on_it() {
    let data = fresh_dir("oku-slides-pack-mode-");
    let mut s = session_on(data.path());
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.save_pack(true).unwrap();
    let edit = edit_of(&s);
    assert_that!(edit.pack).is_not_equal_to(ESSENTIEL_PACK);
    let copy = library_of(&s).current_version(edit.pack).unwrap();
    assert_that!(copy.manifest.parent.clone()).is_none();
    assert_that!(copy.manifest.name.as_str()).is_equal_to("Essentiel (copie)");
    assert_that!(library_of(&s).current(ESSENTIEL_PACK)).is_equal_to(Some(builtin(ESSENTIEL_PACK).version.clone()));
    // Les enregistrements suivants remplacent la version de la copie, toujours sans parente.
    s.apply("Nom", vec![rename_title("Ma charte")], None).unwrap();
    s.save_pack(false).unwrap();
    let again = library_of(&s).current_version(edit.pack).unwrap();
    assert_that!(library_of(&s).versions(edit.pack).len()).is_equal_to(1);
    assert_that!(again.manifest.parent).is_none();
    assert_that!(again.manifest.name.as_str()).is_equal_to("Ma charte");
}

/// Chaque session d'édition a sa propre étape d'annulation dans le deck.
#[test]
fn two_editing_sessions_make_two_undo_steps_in_the_deck() {
    let data = fresh_dir("oku-slides-pack-mode-");
    let mut s = session_on(data.path());
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.apply("Nom", vec![rename_title("Un")], None).unwrap();
    s.save_pack(false).unwrap();
    let first = library_of(&s).current(ESSENTIEL_PACK).unwrap();
    s.close_pack().unwrap();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.apply("Nom", vec![rename_title("Deux")], None).unwrap();
    s.save_pack(false).unwrap();
    s.close_pack().unwrap();
    s.undo().unwrap();
    assert_that!(pin(&s, "essentiel").map(|r| r.version)).is_equal_to(Some(first));
}

#[test]
fn a_save_after_a_turn_begun_in_deck_mode_keeps_the_turn_version() {
    let data = fresh_dir("oku-slides-pack-mode-");
    let mut s = opened_on(data.path());
    let set = ESSENTIEL_PACK;
    // Le tour groupe le deck, qui passe de côté pendant le tour.
    s.begin_turn(1, "modifie le pack");
    s.edit_pack(set).unwrap();
    s.apply("Nom", vec![rename_title("Un")], None).unwrap();
    s.save_pack(false).unwrap();
    s.end_turn();
    let turn = library_of(&s).current(set).unwrap();
    // Après le tour : nouvelle étape du deck mis de côté, pas de fusion.
    s.apply("Nom", vec![rename_title("Deux")], None).unwrap();
    s.save_pack(false).unwrap();
    let after = library_of(&s).current(set).unwrap();
    assert_that!(after).is_not_equal_to(turn.clone());
    assert_that!(library_of(&s).version(set, &turn).is_some()).is_true();
    assert_that!(s.doc.sources.packs.contains_key(&turn)).is_true();
    assert_that!(stash_of(&s).sources.packs.contains_key(&turn)).is_true();
    // Annuler revient à la version du tour, présente dans les sources.
    s.close_pack().unwrap();
    s.undo().unwrap();
    assert_that!(pin(&s, "essentiel").map(|r| r.version)).is_equal_to(Some(turn));
}

#[test]
fn save_pack_of_a_deck_only_pack_embeds_a_deck_only_version_and_leaves_the_library_unchanged() {
    let data = fresh_dir("oku-slides-pack-mode-");
    let mut s = session_on(data.path());
    let r = adopt_deck_only_pack(&mut s);
    let before = library_files(data.path());
    s.edit_pack(r.id).unwrap();
    s.apply("Nom", vec![rename_title("Un")], None).unwrap();
    s.save_pack(false).unwrap();
    let version = edit_of(&s).session_version.unwrap_or_else(|| panic!("version de séance"));
    assert_that!(library_files(data.path())).is_equal_to(before);
    assert_that!(library_of(&s).versions(r.id)).is_empty();
    let files = stash_of(&s).sources.packs.get(&version).cloned().unwrap_or_else(|| panic!("version embarquée"));
    let m = PackManifest::from_files(&files).unwrap();
    assert_that!((m.scope, m.parent.clone(), m.name.as_str())).is_equal_to((
        PackScope::Deck,
        Some(r.version.clone()),
        "Un",
    ));
    s.close_pack().unwrap();
    assert_that!(pin(&s, "deck").map(|p| (p.id, p.version))).is_equal_to(Some((r.id, version)));
    assert_that!(s.snapshot().deck_pack).is_true();
}

#[test]
fn save_pack_twice_on_a_deck_only_pack_replaces_the_session_version_in_sources_only() {
    let data = fresh_dir("oku-slides-pack-mode-");
    let mut s = session_on(data.path());
    let r = adopt_deck_only_pack(&mut s);
    let before = library_files(data.path());
    s.edit_pack(r.id).unwrap();
    s.apply("Nom", vec![rename_title("Un")], None).unwrap();
    s.save_pack(false).unwrap();
    let first = edit_of(&s).session_version.unwrap_or_else(|| panic!("première version"));
    s.apply("Nom", vec![rename_title("Deux")], None).unwrap();
    s.save_pack(false).unwrap();
    let edit = edit_of(&s);
    let second = edit.session_version.clone().unwrap_or_else(|| panic!("seconde version"));
    assert_that!(second.clone()).is_not_equal_to(first.clone());
    assert_that!(edit.parent).is_equal_to(Some(r.version.clone()));
    assert_that!(library_files(data.path())).is_equal_to(before);
    // Fusionnée dans l'étape de la première : la première quitte le deck mis de côté.
    assert_that!(stash_of(&s).sources.packs.contains_key(&first)).is_false();
    assert_that!(stash_of(&s).sources.packs.contains_key(&second)).is_true();
    s.close_pack().unwrap();
    s.undo().unwrap();
    assert_that!(pin(&s, "deck")).is_equal_to(Some(r));
}

#[test]
fn save_as_copy_of_a_deck_only_pack_stays_deck_only() {
    let data = fresh_dir("oku-slides-pack-mode-");
    let mut s = session_on(data.path());
    let r = adopt_deck_only_pack(&mut s);
    let before = library_files(data.path());
    s.edit_pack(r.id).unwrap();
    s.save_pack(true).unwrap();
    let edit = edit_of(&s);
    assert_that!(edit.pack).is_not_equal_to(r.id);
    assert_that!(edit.deck_only).is_true();
    assert_that!(library_files(data.path())).is_equal_to(before);
    assert_that!(library_of(&s).versions(edit.pack)).is_empty();
    let version = edit.session_version.clone().unwrap_or_else(|| panic!("version de la copie"));
    let files = stash_of(&s).sources.packs.get(&version).cloned().unwrap_or_else(|| panic!("copie embarquée"));
    let m = PackManifest::from_files(&files).unwrap();
    assert_that!((m.id, m.scope, m.parent)).is_equal_to((edit.pack, PackScope::Deck, None));
    s.close_pack().unwrap();
    assert_that!(pin(&s, "deck").map(|t| t.id)).is_equal_to(Some(edit.pack));
}

/// Les valeurs d'exemple du layout `name` que montre `s` (mode pack).
fn layout_example(s: &crate::Session, name: &str) -> serde_json::Map<String, serde_json::Value> {
    s.example_values(None, &WidgetFile::parse(name).unwrap()).unwrap()
}

/// Vide le titre de l'exemple du layout `content` (un champ vidé sur l'aperçu).
fn empty_content_title(s: &mut crate::Session) {
    s.set_example_values("content", serde_json::Map::from_iter([("title".into(), json!(""))])).unwrap();
}

#[test]
fn set_example_values_emptied_layout_text_stays_empty() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    empty_content_title(&mut s);
    assert_that!(layout_example(&s, "content").get("title")).is_equal_to(Some(&json!("")));
}

#[test]
fn save_pack_emptied_layout_text_is_saved_empty() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    empty_content_title(&mut s);
    let saved = s.save_pack(true).unwrap();
    let m = saved_manifest(&s, &saved);
    assert_that!(m.examples[&WidgetFile::parse("content").unwrap()].get("title")).is_equal_to(Some(&json!("")));
}

#[test]
fn edit_pack_after_saving_an_emptied_layout_text_keeps_it_empty() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    empty_content_title(&mut s);
    s.save_pack(false).unwrap();
    s.close_pack().unwrap();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    assert_that!(layout_example(&s, "content").get("title")).is_equal_to(Some(&json!("")));
}

#[test]
fn set_example_values_after_emptying_keeps_the_new_text() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    empty_content_title(&mut s);
    s.set_example_values("content", serde_json::Map::from_iter([("title".into(), json!("Retour"))])).unwrap();
    assert_that!(layout_example(&s, "content").get("title")).is_equal_to(Some(&json!("Retour")));
}
