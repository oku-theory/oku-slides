use oku_slides_core::{Op, Target, kinds};
use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use speculoos::prelude::*;

use crate::pack::{BASE_PACK, ESSENTIEL_PACK, PackManifest};
use crate::test_support::{
    adopt_deck_only_pack, edit_of, editing_layout_pack, fresh_dir, library_files, library_of, ns, pin, rename_title,
    session, session_on, slide_widget, stash_of,
};
use crate::{Session, SessionErrorKind};

/// Les ids des slides du deck de `s`.
fn slide_ids(s: &Session) -> Vec<String> {
    s.deck().slides.iter().map(|sl| sl.id.clone()).collect()
}

#[test]
fn edit_pack_essentiel_working_deck_has_one_slide_per_layout() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    assert_that!(slide_ids(&s)).is_equal_to(
        ["layout-blank", "layout-chapter", "layout-content", "layout-end", "layout-title", "layout-toc"]
            .map(String::from)
            .to_vec(),
    );
}

#[test]
fn edit_pack_base_working_deck_has_only_preview_slides() {
    let mut s = session();
    s.edit_pack(BASE_PACK).unwrap();
    assert_that!(slide_ids(&s).iter().all(|id| id.starts_with("widget-"))).is_true();
}

#[test]
fn edit_pack_records_the_kind_of_the_edited_pack() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    assert_that!(edit_of(&s).kind).is_equal_to(oku_slides_core::PackKind::Layouts);
}

#[test]
fn pack_mode_content_slide_starts_from_its_example() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    let slide = s.deck().slides.iter().find(|sl| sl.id == "layout-content").unwrap();
    assert_that!(slide.params.text("title")).is_equal_to(Some("Titre de la slide"));
}

#[rstest]
#[case::remove(Op::RemoveSlide { slide: "layout-end".into() })]
#[case::move_slide(Op::MoveSlide { slide: "layout-end".into(), index: 0 })]
#[case::set_kind(Op::SetKind {
    target: Target { slide: "layout-end".into(), element: None },
    kind: kinds::essentiel::BLANK.into(),
})]
#[case::add(Op::AddSlide { index: None, defaults: false, slide: oku_slides_core::Slide::new("neuve", kinds::essentiel::BLANK) })]
fn pack_mode_structural_slide_op_is_refused(#[case] op: Op) {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    let err = s.apply("x", vec![op], None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::PackModeSlides);
}

#[test]
fn pack_mode_refused_transaction_runs_none_of_its_ops() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    let before = s.deck().clone();
    let err = s.apply("x", vec![rename_title("Autre"), Op::RemoveSlide { slide: "layout-end".into() }], None);
    assert_that!(err.map_err(|e| e.kind()).err()).is_equal_to(Some(SessionErrorKind::PackModeSlides));
    assert_that!(s.deck().clone()).is_equal_to(before);
}

#[test]
fn pack_mode_slides_message_is_localized_in_french() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    let err = s.apply("x", vec![Op::RemoveSlide { slide: "layout-end".into() }], None).unwrap_err();
    assert_that!(err.localize(&Localizer::new(Some("fr")))).is_not_equal_to(err.localize(oku_slides_i18n::english()));
}

#[test]
fn deck_mode_remove_slide_is_not_refused_as_pack_mode() {
    let mut s = session();
    let first = s.deck().slides[0].id.clone();
    assert_that!(s.apply("x", vec![Op::RemoveSlide { slide: first }], None).is_ok()).is_true();
}

#[test]
fn pack_mode_writing_a_layout_appends_its_slide() {
    let data = fresh_dir("oku-slides-pack-mode-");
    let (mut s, _) = editing_layout_pack(data.path(), "acme", &["one"]);
    s.set_widget(None, "two", slide_widget("two")).unwrap();
    assert_that!(slide_ids(&s)).is_equal_to(vec!["layout-one".to_owned(), "layout-two".to_owned()]);
}

#[test]
fn pack_mode_removing_a_layout_removes_its_slide() {
    let data = fresh_dir("oku-slides-pack-mode-");
    let (mut s, _) = editing_layout_pack(data.path(), "acme", &["one", "two"]);
    s.remove_widget(None, "one").unwrap();
    assert_that!(slide_ids(&s)).is_equal_to(vec!["layout-two".to_owned()]);
}

#[test]
fn pack_mode_renaming_a_layout_keeps_its_slide_edits() {
    let data = fresh_dir("oku-slides-pack-mode-");
    let (mut s, _) = editing_layout_pack(data.path(), "acme", &["one"]);
    let set = serde_json::json!({ "op": "set_param", "target": { "slide": "layout-one" }, "param": "title", "value": "Gardé" });
    s.apply("Titre", vec![serde_json::from_value(set).unwrap()], None).unwrap();
    s.rename_widget(None, "one", "uno").unwrap();
    let slide = s.deck().slides.iter().find(|sl| sl.id == "layout-uno").cloned();
    assert_that!(slide.as_ref().and_then(|sl| sl.params.text("title"))).is_equal_to(Some("Gardé"));
}

#[test]
fn pack_mode_example_value_of_a_layout_changes_its_slide() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.set_example_values("content", serde_json::Map::from_iter([("title".into(), serde_json::json!("Essai"))]))
        .unwrap();
    let slide = s.deck().slides.iter().find(|sl| sl.id == "layout-content").unwrap();
    assert_that!(slide.params.text("title")).is_equal_to(Some("Essai"));
}

#[test]
fn pack_mode_example_values_of_a_layout_are_read_on_its_slide() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    let set = serde_json::json!({ "op": "set_param", "target": { "slide": "layout-end" }, "param": "text", "value": "Au revoir" });
    s.apply("Texte", vec![serde_json::from_value(set).unwrap()], None).unwrap();
    let end = oku_slides_core::WidgetFile::parse("end").unwrap();
    let values = s.example_values(None, &end).unwrap();
    assert_that!(values.get("text")).is_equal_to(Some(&serde_json::json!("Au revoir")));
}

#[test]
fn edit_pack_of_an_unknown_pack_returns_unknown_pack() {
    let data = fresh_dir("oku-slides-pack-mode-");
    let mut s = session_on(data.path());
    let err = s.edit_pack(uuid::Uuid::new_v4()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::UnknownPack);
}

#[test]
fn layouts_view_of_the_deck_pack_shows_the_embedded_version() {
    let data = fresh_dir("oku-slides-pack-mode-");
    let mut s = session_on(data.path());
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.set_widget(None, "extra", slide_widget("extra")).unwrap();
    s.save_pack(false).unwrap();
    // Le deck mis de côté est passé à la nouvelle version : on revient à l'ancienne.
    s.close_pack().unwrap();
    s.undo().unwrap();
    let view = s.layouts_view().unwrap();
    let shown = view.packs.iter().find(|p| p.namespace == ns("essentiel")).map(|p| p.layouts.len());
    assert_that!(shown).is_equal_to(Some(kinds::essentiel::ALL.len()));
}

fn open_tabs(s: &mut crate::Session) {
    let ui = crate::views::UiState {
        widgets: vec!["_commun".into()],
        widget: Some("_commun".into()),
        ..crate::views::UiState::default()
    };
    s.handle(crate::Request::Ui { ui, order: None }).unwrap();
}

/// Sortir du mode pack : le deck ne garde aucun sous-onglet.
#[test]
fn close_pack_with_open_widget_tabs_clears_them() {
    let data = fresh_dir("oku-slides-pack-mode-");
    let mut s = session_on(data.path());
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    open_tabs(&mut s);
    s.close_pack().unwrap();
    assert_that!(s.ui.widgets).is_empty();
    assert_that!(s.ui.widget).is_none();
}

/// Passer à un autre pack : les sous-onglets de l'ancien ne le suivent pas.
#[test]
fn edit_pack_of_another_pack_clears_the_widget_tabs() {
    let data = fresh_dir("oku-slides-pack-mode-");
    let mut s = session_on(data.path());
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    open_tabs(&mut s);
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    assert_that!(s.ui.widgets).is_empty();
}

/// En mode deck, l'interface ne peut pas poser de sous-onglets.
#[test]
fn ui_request_in_deck_mode_keeps_no_widget_tabs() {
    let data = fresh_dir("oku-slides-pack-mode-");
    let mut s = session_on(data.path());
    open_tabs(&mut s);
    assert_that!(s.ui.widgets).is_empty();
}

/// Un widget minimal pour les écritures en mode pack.
const KPI: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("kpi", params: (value: p.plain()), (args, ctx) => field(ctx, "value", args.value))
"#;

#[test]
fn edit_pack_of_a_deck_only_pack_starts_from_its_embedded_version() {
    let data = fresh_dir("oku-slides-pack-mode-");
    let mut s = session_on(data.path());
    let r = adopt_deck_only_pack(&mut s);
    s.edit_pack(r.id).unwrap();
    let edit = edit_of(&s);
    assert_that!((edit.pack, edit.base.clone(), edit.deck_only)).is_equal_to((r.id, r.version.clone(), true));
    assert_that!(edit.name.as_str()).is_equal_to("Importé");
    assert_that!(pin(&s, "deck")).is_equal_to(Some(r));
}

#[test]
fn edit_pack_of_a_library_pack_is_not_deck_only() {
    let data = fresh_dir("oku-slides-pack-mode-");
    let mut s = session_on(data.path());
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    assert_that!(edit_of(&s).deck_only).is_false();
}

#[test]
fn undo_after_saving_a_deck_only_pack_writes_no_library() {
    let data = fresh_dir("oku-slides-pack-mode-");
    let mut s = session_on(data.path());
    let r = adopt_deck_only_pack(&mut s);
    let before = library_files(data.path());
    s.edit_pack(r.id).unwrap();
    s.set_widget(None, "kpi", KPI.into()).unwrap();
    let log = crate::test_support::logged(|| s.undo().unwrap());
    assert_that!(log.as_str()).does_not_contain("ERROR");
    assert_that!(library_files(data.path())).is_equal_to(before);
    let edit = edit_of(&s);
    assert_that!((edit.base.clone(), edit.session_version.clone())).is_equal_to((r.version.clone(), None));
    assert_that!(stash_of(&s).sources.deck().widgets.get(&ns("deck")).cloned()).is_equal_to(Some(r));
}

#[test]
fn edit_pack_of_a_deck_only_pack_only_in_history_is_refused_as_deck_only() {
    let data = fresh_dir("oku-slides-pack-mode-");
    let mut s = session_on(data.path());
    let r = adopt_deck_only_pack(&mut s);
    // Le deck revient à son pack d'avant : la version du deck seul ne reste que dans l'historique.
    s.undo().unwrap();
    assert_that!(pin(&s, "deck").map(|t| t.id)).is_not_equal_to(Some(r.id));
    let err = s.edit_pack(r.id).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::DeckOnly);
    let de = Localizer::new(Some("de"));
    assert_that!(err.localize(&de)).is_not_equal_to(err.localize(&Localizer::new(Some("en"))));
    assert_that!(matches!(s.mode, crate::Mode::Deck)).is_true();
}

#[test]
fn pack_base_of_a_deck_only_pack_missing_its_embedded_version_is_pack_version_missing() {
    let data = fresh_dir("oku-slides-pack-mode-");
    let mut s = session_on(data.path());
    let r = adopt_deck_only_pack(&mut s);
    // Des sources incohérentes : une autre version du deck seul est embarquée, pas celle du deck.
    let files = s.doc.sources.packs.get(&r.version).cloned().unwrap_or_else(|| panic!("version du deck"));
    let mut m = PackManifest::from_files(&files).unwrap();
    m.name = "Autre".into();
    s.doc.sources.add_pack(m.to_files(oku_slides_core::PackKind::Widgets, files.widgets().clone()).unwrap());
    s.doc.sources.packs.remove(&r.version);
    let err = s.pack_base(r.id).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::PackVersionMissing);
}

/// Une session sur `data` dont la bibliothèque a le pack de layouts `namespace`, sans besoins, au
/// seul layout `content` d'Essentiel ; son id.
fn session_on_pack_without_base(data: &std::path::Path, namespace: &str) -> (crate::Session, uuid::Uuid) {
    let s = session_on(data);
    let files = &crate::test_support::builtin(ESSENTIEL_PACK).files;
    let content = files.widget(&oku_slides_core::WidgetFile::parse("content").unwrap()).unwrap();
    let common = files.widget(&oku_slides_core::WidgetFile::common()).unwrap();
    // Les layouts d'Essentiel lisent ses jetons de composant : la copie les déclare aussi.
    let tokens = crate::test_support::builtin(ESSENTIEL_PACK).manifest.tokens.clone();
    let m =
        crate::pack::PackManifest { tokens, ..crate::test_support::manifest_ns(uuid::Uuid::new_v4(), namespace, &[]) };
    let kind = oku_slides_core::PackKind::Layouts;
    crate::test_support::add_version_of(library_of(&s), kind, &m, &[("content", content), ("_commun", common)]);
    (s, m.id)
}

/// L'op qui ajoute un `base/text` aux blocs de la première slide du deck de travail.
fn add_text_to_first_layout(s: &crate::Session) -> Op {
    let slide = s.deck().slides[0].id.clone();
    serde_json::from_value(serde_json::json!({
        "op": "add_element", "slot": { "owner": { "slide": slide }, "param": "blocks" },
        "element": { "id": "t-neuf", "kind": kinds::base::TEXT,
            "params": { "body": [{ "runs": [{ "t": "Bonjour" }] }] } } }))
    .unwrap()
}

#[test]
fn edit_pack_of_a_pack_without_base_can_add_a_base_text() {
    let data = fresh_dir("oku-slides-pack-mode-");
    let (mut s, id) = session_on_pack_without_base(data.path(), "acme");
    s.edit_pack(id).unwrap();
    let op = add_text_to_first_layout(&s);
    let r = s.apply("Ajouter", vec![op], None);
    assert_that!(r.is_ok()).is_true();
}

#[test]
fn save_pack_of_a_pack_without_base_requires_base_once_an_example_uses_it() {
    let data = fresh_dir("oku-slides-pack-mode-");
    let (mut s, id) = session_on_pack_without_base(data.path(), "acme");
    s.edit_pack(id).unwrap();
    let op = add_text_to_first_layout(&s);
    s.apply("Ajouter", vec![op], None).unwrap();
    s.save_pack(false).unwrap();
    s.close_pack().unwrap();
    let requires = library_of(&s).current_version(id).unwrap().manifest.requires;
    assert_that!(requires).is_equal_to(vec![kinds::base::namespace()]);
}

#[test]
fn edit_pack_deck_pinning_a_base_version_found_nowhere_returns_pack_version_missing() {
    let data = fresh_dir("oku-slides-pack-mode-");
    let (mut s, id) = session_on_pack_without_base(data.path(), "acme");
    let mut deck = s.deck().clone();
    let base = deck.widgets.get_mut(&kinds::base::namespace()).unwrap();
    base.version = oku_slides_core::PackVersion::from_hex(&"0".repeat(64)).unwrap();
    s.doc.sources.apply_silent(deck).unwrap();
    let err = s.edit_pack(id).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::PackVersionMissing);
}

#[test]
fn pin_all_namespace_pinned_under_the_other_kind_is_refused_and_pins_nothing() {
    let mut s = session();
    let mut deck = s.deck().clone();
    let files = crate::test_support::manifest_ns(uuid::Uuid::new_v4(), "base", &[])
        .to_files(oku_slides_core::PackKind::Layouts, std::collections::BTreeMap::default())
        .unwrap();
    let pin = oku_slides_core::PackPin { id: uuid::Uuid::new_v4(), version: files.version(), requires: Vec::new() };
    let before = deck.clone();
    let refused = super::pin_all(&mut deck, &mut s.doc.sources, vec![(ns("base"), pin, files)]);
    assert_that!(refused.map_err(|e| e.kind())).is_equal_to(Err(SessionErrorKind::NamespaceTaken));
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn pack_working_deck_keeps_the_deck_theme_and_overrides() {
    let data = fresh_dir("oku-slides-pack-style-");
    let mut s = session_on(data.path());
    s.handle(crate::Request::SetTheme { theme: "pro".into() }).unwrap();
    let ink = Op::SetToken { path: vec!["color".into(), "ink".into()], value: serde_json::json!("#101010") };
    s.apply("ink", vec![ink], None).unwrap();
    let accent = crate::theme_edit::theme_edit_tests::rendered_token(&s, "/color/accent");
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    let (user, working) = (stash_of(&s).sources.deck().clone(), s.deck().clone());
    assert_that!((&working.theme, &working.themes, &working.tokens)).is_equal_to((
        &user.theme,
        &user.themes,
        &user.tokens,
    ));
    let reached = user.themes.values().all(|pin| s.doc.sources.themes.contains_key(&pin.version));
    assert_that!(reached).is_true();
    assert_that!(crate::theme_edit::theme_edit_tests::rendered_token(&s, "/color/accent")).is_equal_to(accent);
    let ink = crate::theme_edit::theme_edit_tests::rendered_token(&s, "/color/ink");
    assert_that!(ink).is_equal_to(Some(serde_json::json!("#101010")));
}
