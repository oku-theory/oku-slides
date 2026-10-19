use speculoos::prelude::*;

use crate::pack::{DEFAULT_PACK, PackManifest};
use crate::test_support::{adopt_deck_only_pack, fresh_dir, library_files, library_of, ns, pin, session_on};

#[test]
fn save_templates_creates_a_version_whose_parent_is_the_old_current() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    let builtin = crate::test_support::builtin(crate::pack::DEFAULT_PACK).version.clone();
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
            .join("widgets")
            .join(DEFAULT_PACK.to_string())
            .join("versions")
            .join(current.version.hex())
            .join("widgets.json")
            .exists()
    )
    .is_true();
    s.close_templates().unwrap();
    assert_that!(pin(&s, "essentiel").map(|r| r.version)).is_equal_to(Some(current.version.clone()));
    assert_that!(s.sources_for_tests().packs.contains_key(&current.version)).is_true();
}

/// Une version par session d'édition : les enregistrements suivants la remplacent.
#[test]
fn saving_twice_in_one_session_keeps_one_version_and_the_deck_follows_in_one_undo_step() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    let before = pin(&s, "essentiel");
    let builtin = crate::test_support::builtin(crate::pack::DEFAULT_PACK).version.clone();
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
    assert_that!(pin(&s, "essentiel").map(|r| r.version)).is_equal_to(Some(second.version));
    s.undo().unwrap();
    assert_that!(pin(&s, "essentiel")).is_equal_to(before);
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
    let edit = s.snapshot().editing.map(|e| e.mode.clone()).unwrap();
    assert_that!(edit.set).is_not_equal_to(DEFAULT_PACK);
    let copy = library_of(&s).current_version(edit.set).unwrap();
    assert_that!(copy.manifest.parent.clone()).is_none();
    assert_that!(copy.manifest.name.as_str()).is_equal_to("Essentiel (copie)");
    assert_that!(library_of(&s).current(DEFAULT_PACK))
        .is_equal_to(Some(crate::test_support::builtin(crate::pack::DEFAULT_PACK).version.clone()));
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
    let embedded = pin(&s, "essentiel").unwrap();
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
    let manifest = PackManifest::from_files(&crate::test_support::builtin(crate::pack::DEFAULT_PACK).files).unwrap();
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
    assert_that!(pin(&s, "essentiel").map(|r| r.version)).is_equal_to(Some(first));
}

/// Le titre du deck des modèles devient `t`.
fn rename(t: &str) -> oku_slides_core::Op {
    serde_json::from_value(serde_json::json!({ "op": "set_deck_param", "key": "title", "value": t })).unwrap()
}

#[test]
fn a_save_after_a_turn_begun_in_deck_mode_keeps_the_turn_version() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = crate::test_support::opened_on(data.path());
    let set = crate::DEFAULT_PACK;
    // Le tour groupe le deck, qui passe de côté pendant le tour.
    s.begin_turn(1, "modifie les modèles");
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
    assert_that!(pin(&s, "essentiel").map(|r| r.version)).is_equal_to(Some(turn));
}

fn open_tabs(s: &mut crate::Session) {
    let ui = crate::views::UiState {
        widgets: vec!["_commun".into()],
        widget: Some("_commun".into()),
        ..crate::views::UiState::default()
    };
    s.handle(crate::Request::Ui { ui, order: None }).unwrap();
}

/// Sortir du mode modèles : le deck ne garde aucun sous-onglet.
#[test]
fn close_templates_with_open_widget_tabs_clears_them() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    s.edit_templates(DEFAULT_PACK).unwrap();
    open_tabs(&mut s);
    s.close_templates().unwrap();
    assert_that!(s.ui.widgets).is_empty();
    assert_that!(s.ui.widget).is_none();
}

/// Passer à un autre pack : les sous-onglets de l'ancien ne le suivent pas.
#[test]
fn edit_templates_of_another_pack_clears_the_widget_tabs() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    s.edit_templates(DEFAULT_PACK).unwrap();
    open_tabs(&mut s);
    s.edit_templates(DEFAULT_PACK).unwrap();
    assert_that!(s.ui.widgets).is_empty();
}

/// En mode deck, l'interface ne peut pas poser de sous-onglets.
#[test]
fn ui_request_in_deck_mode_keeps_no_widget_tabs() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    open_tabs(&mut s);
    assert_that!(s.ui.widgets).is_empty();
}

/// L'édition en cours (mode modèles) de `s`.
fn edit_of(s: &mut crate::Session) -> crate::TemplateMode {
    s.snapshot().editing.map(|e| e.mode.clone()).unwrap_or_else(|| panic!("mode modèles attendu"))
}

/// Le deck mis de côté par le mode modèles de `s`.
fn stash_of(s: &crate::Session) -> &crate::Document {
    let crate::Mode::Templates { stash, .. } = &s.mode else { panic!("mode modèles attendu") };
    stash
}

/// Un widget minimal pour les écritures en mode modèles.
const KPI: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("kpi", params: (value: p.plain()), (args, ctx) => field(ctx, "value", args.value))
"#;

#[test]
fn edit_templates_of_a_deck_only_pack_starts_from_its_embedded_version() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    let r = adopt_deck_only_pack(&mut s);
    s.edit_templates(r.id).unwrap();
    let edit = edit_of(&mut s);
    assert_that!((edit.set, edit.base.clone(), edit.deck_only)).is_equal_to((r.id, r.version.clone(), true));
    assert_that!(edit.name.as_str()).is_equal_to("Importé");
    assert_that!(pin(&s, "deck")).is_equal_to(Some(r));
}

#[test]
fn edit_templates_of_a_library_pack_is_not_deck_only() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    s.edit_templates(DEFAULT_PACK).unwrap();
    assert_that!(edit_of(&mut s).deck_only).is_false();
}

#[test]
fn save_templates_of_a_deck_only_pack_embeds_a_deck_only_version_and_leaves_the_library_unchanged() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    let r = adopt_deck_only_pack(&mut s);
    let before = library_files(data.path());
    s.edit_templates(r.id).unwrap();
    s.apply("Nom", vec![rename("Un")], None).unwrap();
    s.save_templates(false).unwrap();
    let version = edit_of(&mut s).session_version.unwrap_or_else(|| panic!("version de séance"));
    assert_that!(library_files(data.path())).is_equal_to(before);
    assert_that!(library_of(&s).versions(r.id)).is_empty();
    let files = stash_of(&s).sources.packs.get(&version).cloned().unwrap_or_else(|| panic!("version embarquée"));
    let m = PackManifest::from_files(&files).unwrap();
    assert_that!((m.scope, m.parent.clone(), m.name.as_str())).is_equal_to((
        oku_slides_core::PackScope::Deck,
        Some(r.version.clone()),
        "Un",
    ));
    s.close_templates().unwrap();
    assert_that!(pin(&s, "deck").map(|p| (p.id, p.version))).is_equal_to(Some((r.id, version)));
    assert_that!(s.snapshot().deck_pack).is_true();
}

#[test]
fn save_templates_twice_on_a_deck_only_pack_replaces_the_session_version_in_sources_only() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    let r = adopt_deck_only_pack(&mut s);
    let before = library_files(data.path());
    s.edit_templates(r.id).unwrap();
    s.apply("Nom", vec![rename("Un")], None).unwrap();
    s.save_templates(false).unwrap();
    let first = edit_of(&mut s).session_version.unwrap_or_else(|| panic!("première version"));
    s.apply("Nom", vec![rename("Deux")], None).unwrap();
    s.save_templates(false).unwrap();
    let edit = edit_of(&mut s);
    let second = edit.session_version.clone().unwrap_or_else(|| panic!("seconde version"));
    assert_that!(second.clone()).is_not_equal_to(first.clone());
    assert_that!(edit.parent).is_equal_to(Some(r.version.clone()));
    assert_that!(library_files(data.path())).is_equal_to(before);
    // Fusionnée dans l'étape de la première : la première quitte le deck mis de côté.
    assert_that!(stash_of(&s).sources.packs.contains_key(&first)).is_false();
    assert_that!(stash_of(&s).sources.packs.contains_key(&second)).is_true();
    s.close_templates().unwrap();
    s.undo().unwrap();
    assert_that!(pin(&s, "deck")).is_equal_to(Some(r));
}

#[test]
fn undo_after_saving_a_deck_only_pack_writes_no_library() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    let r = adopt_deck_only_pack(&mut s);
    let before = library_files(data.path());
    s.edit_templates(r.id).unwrap();
    s.set_widget(None, "kpi", KPI.into()).unwrap();
    let log = crate::test_support::logged(|| s.undo().unwrap());
    assert_that!(log.as_str()).does_not_contain("ERROR");
    assert_that!(library_files(data.path())).is_equal_to(before);
    let edit = edit_of(&mut s);
    assert_that!((edit.base.clone(), edit.session_version.clone())).is_equal_to((r.version.clone(), None));
    assert_that!(stash_of(&s).sources.deck().widgets.get(&ns("deck")).cloned()).is_equal_to(Some(r));
}

#[test]
fn edit_templates_of_a_deck_only_pack_only_in_history_is_refused_as_deck_only() {
    use oku_slides_i18n::Localize;
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    let r = adopt_deck_only_pack(&mut s);
    // Le deck revient à son pack d'avant : la version du deck seul ne reste que dans l'historique.
    s.undo().unwrap();
    assert_that!(pin(&s, "deck").map(|t| t.id)).is_not_equal_to(Some(r.id));
    let err = s.edit_templates(r.id).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::DeckOnly);
    let de = oku_slides_i18n::Localizer::new(Some("de"));
    assert_that!(err.localize(&de)).is_not_equal_to(err.localize(&oku_slides_i18n::Localizer::new(Some("en"))));
    assert_that!(matches!(s.mode, crate::Mode::Deck)).is_true();
}

#[test]
fn templates_base_of_a_deck_only_pack_missing_its_embedded_version_is_pack_version_missing() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    let r = adopt_deck_only_pack(&mut s);
    // Des sources incohérentes : une autre version du deck seul est embarquée, pas celle du deck.
    let files = s.doc.sources.packs.get(&r.version).cloned().unwrap_or_else(|| panic!("version du deck"));
    let mut m = PackManifest::from_files(&files).unwrap();
    m.name = "Autre".into();
    s.doc.sources.add_pack(m.to_files(files.widgets().clone()).unwrap());
    s.doc.sources.packs.remove(&r.version);
    let err = s.templates_base(r.id).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::PackVersionMissing);
}

#[test]
fn save_as_copy_of_a_deck_only_pack_stays_deck_only() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    let r = adopt_deck_only_pack(&mut s);
    let before = library_files(data.path());
    s.edit_templates(r.id).unwrap();
    s.save_templates(true).unwrap();
    let edit = edit_of(&mut s);
    assert_that!(edit.set).is_not_equal_to(r.id);
    assert_that!(edit.deck_only).is_true();
    assert_that!(library_files(data.path())).is_equal_to(before);
    assert_that!(library_of(&s).versions(edit.set)).is_empty();
    let version = edit.session_version.clone().unwrap_or_else(|| panic!("version de la copie"));
    let files = stash_of(&s).sources.packs.get(&version).cloned().unwrap_or_else(|| panic!("copie embarquée"));
    let m = PackManifest::from_files(&files).unwrap();
    assert_that!((m.id, m.scope, m.parent)).is_equal_to((edit.set, oku_slides_core::PackScope::Deck, None));
    s.close_templates().unwrap();
    assert_that!(pin(&s, "deck").map(|t| t.id)).is_equal_to(Some(edit.set));
}

/// Une session sur `data` dont la bibliothèque a le pack `namespace`, sans besoins, au seul modèle
/// `texte` d'Essentiel ; son id.
fn session_on_pack_without_base(data: &std::path::Path, namespace: &str) -> (crate::Session, uuid::Uuid) {
    let s = session_on(data);
    let texte = crate::test_support::builtin(DEFAULT_PACK).manifest.templates.iter().find(|t| t.id == "texte").cloned();
    let m = PackManifest {
        templates: texte.into_iter().collect(),
        ..crate::test_support::manifest_ns(uuid::Uuid::new_v4(), namespace, &[])
    };
    crate::test_support::add_version(library_of(&s), &m, &[]);
    (s, m.id)
}

/// L'op qui ajoute un `base/text` aux blocs du premier modèle édité.
fn add_text_to_first_template(s: &crate::Session) -> oku_slides_core::Op {
    let slide = s.deck().slides[0].id.clone();
    serde_json::from_value(serde_json::json!({
        "op": "add_element", "slot": { "owner": { "slide": slide }, "param": "blocks" },
        "element": { "id": "t-neuf", "kind": oku_slides_core::kinds::base::TEXT,
            "params": { "body": [{ "runs": [{ "t": "Bonjour" }] }] } } }))
    .unwrap()
}

#[test]
fn edit_templates_of_a_pack_without_base_can_add_a_base_text() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let (mut s, id) = session_on_pack_without_base(data.path(), "acme");
    s.edit_templates(id).unwrap();
    let op = add_text_to_first_template(&s);
    let r = s.apply("Ajouter", vec![op], None);
    assert_that!(r.is_ok()).is_true();
}

#[test]
fn saved_pack_requires_the_namespaces_its_templates_use() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let (mut s, id) = session_on_pack_without_base(data.path(), "acme");
    s.edit_templates(id).unwrap();
    let op = add_text_to_first_template(&s);
    s.apply("Ajouter", vec![op], None).unwrap();
    s.save_templates(false).unwrap();
    s.close_templates().unwrap();
    let requires = library_of(&s).current_version(id).unwrap().manifest.requires;
    assert_that!(requires).is_equal_to(vec![oku_slides_core::kinds::base::namespace()]);
}

#[test]
fn edit_templates_deck_pinning_a_base_version_found_nowhere_returns_pack_version_missing() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let (mut s, id) = session_on_pack_without_base(data.path(), "acme");
    let mut deck = s.deck().clone();
    let base = deck.widgets.get_mut(&oku_slides_core::kinds::base::namespace()).unwrap();
    base.version = oku_slides_core::PackVersion::from_hex(&"0".repeat(64)).unwrap();
    s.doc.sources.apply_silent(deck).unwrap();
    let err = s.edit_templates(id).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::PackVersionMissing);
}
