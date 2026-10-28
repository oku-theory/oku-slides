use std::path::{Path, PathBuf};

use oku_slides_core::{Namespace, Op, OpError, OpErrorKind, PackFiles, PackKind, PackPin, PackVersion};
use rstest::rstest;
use serde_json::Value;
use speculoos::prelude::*;
use uuid::Uuid;

use oku_slides_i18n::{Localize, Localizer};

use crate::agent::Surface;
use crate::deck_packs::Drift;
use crate::deck_packs_panel::deck_packs_panel;
use crate::lib_tests_support::session_with_fixture;
use crate::pack::PackManifest;
use crate::test_support::{
    add_user_theme, add_version, adopt_deck_only_pack, fresh_dir, manifest, manifest_ns, namespace_of, ns, opened_on,
    session, session_on,
};
use crate::{Request, Session, SessionErrorKind};

/// Les fichiers d'une version du pack de widgets `id`, sans widgets.
pub(crate) fn files(id: Uuid, name: &str, parent: Option<&PackVersion>, created: &str) -> PackFiles {
    manifest(id, name, parent, created).to_files(PackKind::Widgets, std::collections::BTreeMap::new()).unwrap()
}

/// L'op qui épingle la version `pack` du pack `id`, sous l'espace de noms de son id.
pub(crate) fn pin_op(id: Uuid, pack: &PackFiles) -> Op {
    let pin = PackPin { id, version: pack.version(), requires: Vec::new() };
    Op::AddPack { kind: PackKind::Widgets, namespace: namespace_of(id), pin }
}

/// La version du pack `id` que le deck de `s` épingle.
pub(crate) fn pinned_version(s: &Session, id: Uuid) -> Option<PackVersion> {
    s.deck().widgets.get(&namespace_of(id)).map(|r| r.version.clone())
}

/// L'id du pack de l'utilisateur épinglé par le deck de `s` (aucun pack intégré).
fn user_pack_id(s: &Session) -> Uuid {
    s.deck().widgets.values().map(|r| r.id).find(|id| !crate::PackLibrary::is_builtin(*id)).unwrap()
}

/// Un fichier enregistré par une autre session (dossier `them`), dont le deck embarque `pack`.
pub(crate) fn received_file(them: &Path, pack: &PackFiles) -> PathBuf {
    let mut other = session_on(them);
    let id = PackManifest::from_files(pack).unwrap().id;
    other.library.add_version(id, pack).unwrap();
    other.apply("Pack", vec![pin_op(id, pack)], None).unwrap();
    let path = them.join("recu.oku-slides.pdf");
    other.handle(Request::Save { path: Some(path.clone()), overwrite: false }).unwrap();
    path
}

/// v0 ← v1 dans la bibliothèque (courante `mine`), le fichier épingle `theirs` : la session ouverte.
fn opened_with(mine: usize, theirs: usize) -> (tempfile::TempDir, tempfile::TempDir, Session, PackFiles, PackFiles) {
    let (me, them) = (fresh_dir("oku-slides-packs-me-"), fresh_dir("oku-slides-packs-them-"));
    let id = Uuid::new_v4();
    let v0 = files(id, "P", None, "2026-10-01T10:00:00Z");
    let v1 = files(id, "P", Some(&v0.version()), "2026-10-02T10:00:00Z");
    let v = [v0, v1];
    let path = received_file(them.path(), &v[theirs]);
    let mut s = session_on(me.path());
    s.library.add_version(id, &v[0]).unwrap();
    s.library.add_version(id, &v[1]).unwrap();
    s.library.set_current(id, &v[mine].version()).unwrap();
    s.open(&path).unwrap();
    let [v0, v1] = v;
    let (theirs, mine) = if theirs == 1 { (v1, v0) } else { (v0, v1) };
    (me, them, s, theirs, mine)
}

/// Ma version courante v0, le fichier en v1 : la session ouverte, sa version et la mienne.
pub(crate) fn opened_on_newer() -> (tempfile::TempDir, tempfile::TempDir, Session, PackFiles, PackFiles) {
    opened_with(0, 1)
}

/// Ma version courante v1, le fichier en v0 (plus ancien) : la session ouverte, sa version et la mienne.
pub(crate) fn opened_older_than_library() -> (tempfile::TempDir, tempfile::TempDir, Session, PackFiles, PackFiles) {
    opened_with(1, 0)
}

/// L'espace de noms du pack `files`.
pub(crate) fn namespace_of_pack(files: &PackFiles) -> Namespace {
    PackManifest::from_files(files).unwrap().namespace
}

/// Une session dont la bibliothèque a, pour chaque `(espace de noms, besoins)`, un pack de widgets vide.
fn session_with_library_widget_packs(packs: &[(&str, &[&str])]) -> (tempfile::TempDir, Session) {
    let data = fresh_dir("oku-slides-packs-library-");
    let s = opened_on(data.path());
    for (namespace, requires) in packs {
        add_version(&s.library, &manifest_ns(Uuid::new_v4(), namespace, requires), &[]);
    }
    (data, s)
}

/// Le libellé de l'étape à annuler du deck de `s`.
fn undo_label(s: &Session) -> Option<String> {
    s.sources_for_tests().replica().undo_label()
}

/// Les composants de la surface `surface`, en JSON.
pub(crate) fn components(surface: &Surface) -> Vec<Value> {
    let json = serde_json::to_value(surface).unwrap();
    let messages = json["messages"].as_array().cloned().unwrap_or_default();
    let update = messages.iter().find_map(|m| m.get("updateComponents")).cloned().unwrap_or_default();
    update["components"].as_array().cloned().unwrap_or_default()
}

/// Les ids des composants de `surface`.
pub(crate) fn component_ids(surface: &Surface) -> Vec<String> {
    components(surface).iter().filter_map(|c| c["id"].as_str().map(str::to_owned)).collect()
}

/// Le composant `id` de `surface` (`null` s'il n'y est pas).
pub(crate) fn component(surface: &Surface, id: &str) -> Value {
    components(surface).into_iter().find(|c| c["id"] == id).unwrap_or_default()
}

/// Le texte du composant `id` de `surface`.
pub(crate) fn text_of(surface: &Surface, id: &str) -> String {
    component(surface, id)["text"].as_str().unwrap_or_default().to_owned()
}

#[rstest]
#[case::base("base", OpErrorKind::BaseRequired)]
#[case::essentiel_in_use("essentiel", OpErrorKind::PackInUse)]
fn deck_packs_builtin_refusal_has_its_reason(#[case] pack: &str, #[case] expected: OpErrorKind) {
    let s = session_with_fixture();
    let view = s.deck_packs().unwrap();
    let mut all = view.layouts.iter().chain(&view.widgets);
    let pinned = all.find(|p| p.namespace.as_str() == pack).unwrap();
    assert_that!(pinned.remove_refusal.as_ref().map(OpError::kind)).is_equal_to(Some(expected));
}

#[test]
fn deck_packs_refusal_text_is_french_in_french() {
    let s = session_with_fixture();
    let view = s.deck_packs().unwrap();
    let surface_fr = deck_packs_panel(&view, &Localizer::new(Some("fr")));
    let surface_en = deck_packs_panel(&view, &Localizer::new(Some("en")));
    assert_that!(text_of(&surface_fr, "widgets/base/refusal"))
        .is_not_equal_to(text_of(&surface_en, "widgets/base/refusal"));
    assert_that!(text_of(&surface_fr, "widgets/base/refusal")).does_not_contain("0");
}

#[test]
fn deck_packs_pinned_packs_are_listed_by_kind() {
    let s = session();
    let view = s.deck_packs().unwrap();
    let names = |packs: &[super::PinnedPackView]| packs.iter().map(|p| p.namespace.to_string()).collect::<Vec<_>>();
    assert_that!((names(&view.layouts), names(&view.widgets)))
        .is_equal_to((vec!["essentiel".to_owned()], vec!["base".to_owned()]));
}

#[test]
fn deck_packs_addable_lists_unpinned_library_packs_only() {
    let (_data, s) = session_with_library_widget_packs(&[("acme", &[])]);
    let view = s.deck_packs().unwrap();
    let widgets: Vec<String> =
        view.addable.get(&PackKind::Widgets).into_iter().flatten().map(|(n, _)| n.to_string()).collect();
    assert_that!(widgets).is_equal_to(vec!["acme".to_owned()]);
    assert_that!(view.addable.get(&PackKind::Layouts).is_none_or(Vec::is_empty)).is_true();
}

#[test]
fn add_library_pack_with_requirement_is_one_undo_step() {
    let (_data, mut s) = session_with_library_widget_packs(&[("dep", &[]), ("acme", &["dep"])]);
    s.add_library_pack(PackKind::Widgets, &ns("acme")).unwrap();
    assert_that!(s.deck().widgets.contains_key(&ns("dep"))).is_true();
    s.undo().unwrap();
    assert_that!((s.deck().widgets.contains_key(&ns("acme")), s.deck().widgets.contains_key(&ns("dep"))))
        .is_equal_to((false, false));
}

#[test]
fn add_library_pack_labels_its_undo_step() {
    let (_data, mut s) = session_with_library_widget_packs(&[("acme", &[])]);
    s.add_library_pack(PackKind::Widgets, &ns("acme")).unwrap();
    let expected = s.localizer.tr_with("packs-undo-add", &[("name", "acme".into())]);
    assert_that!(undo_label(&s)).is_equal_to(Some(expected));
}

#[test]
fn add_library_pack_already_pinned_namespace_is_refused() {
    let mut s = session();
    let err = s.add_library_pack(PackKind::Layouts, &ns("base")).unwrap_err();
    // `PackKindConflict`, refusé par le cœur.
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Op);
}

#[test]
fn add_library_pack_pinned_under_the_same_kind_is_refused() {
    let mut s = session();
    let err = s.add_library_pack(PackKind::Widgets, &ns("base")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NamespaceTaken);
}

#[test]
fn add_library_pack_unknown_namespace_is_refused() {
    let mut s = session();
    let err = s.add_library_pack(PackKind::Widgets, &ns("nulle-part")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownNamespace);
}

#[test]
fn remove_deck_pack_refused_leaves_deck_and_history() {
    let mut s = session_with_fixture();
    let before = (s.deck().clone(), undo_label(&s));
    assert_that!(s.remove_deck_pack(PackKind::Widgets, &ns("base"))).is_err();
    assert_that!((s.deck().clone(), undo_label(&s))).is_equal_to(before);
}

#[test]
fn remove_deck_pack_unused_pack_is_one_labelled_undo_step() {
    let (_data, mut s) = session_with_library_widget_packs(&[("acme", &[])]);
    s.add_library_pack(PackKind::Widgets, &ns("acme")).unwrap();
    s.remove_deck_pack(PackKind::Widgets, &ns("acme")).unwrap();
    assert_that!(s.deck().widgets.contains_key(&ns("acme"))).is_false();
    let expected = s.localizer.tr_with("packs-undo-remove", &[("name", "acme".into())]);
    assert_that!(undo_label(&s)).is_equal_to(Some(expected));
    s.undo().unwrap();
    assert_that!(s.deck().widgets.contains_key(&ns("acme"))).is_true();
}

#[test]
fn pack_make_current_deck_only_pack_is_refused() {
    let mut s = session();
    let pin = adopt_deck_only_pack(&mut s);
    let ns = s.deck().widgets.iter().find(|(_, p)| **p == pin).map(|(n, _)| n.clone()).unwrap();
    let err = s.pack_make_current(PackKind::Widgets, &ns).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::PackNotMakeCurrent);
    let (fr, en) = (err.localize(&Localizer::new(Some("fr"))), err.localize(&Localizer::new(Some("en"))));
    assert_that!(fr).is_not_equal_to(en);
}

#[test]
fn pack_make_current_builtin_pack_is_refused() {
    let mut s = session();
    let err = s.pack_make_current(PackKind::Widgets, &ns("base")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::PackNotMakeCurrent);
}

#[test]
fn pack_make_current_changes_library_not_deck() {
    let (_a, _b, mut s, theirs, _mine) = opened_on_newer();
    let deck = s.deck().clone();
    s.pack_make_current(PackKind::Widgets, &namespace_of_pack(&theirs)).unwrap();
    assert_that!(s.deck().clone()).is_equal_to(deck);
    assert_that!(undo_label(&s)).is_none();
    let id = user_pack_id(&s);
    assert_that!(s.library.current(id)).is_equal_to(Some(theirs.version()));
}

#[test]
fn pack_make_current_unpinned_namespace_is_refused() {
    let mut s = session();
    let err = s.pack_make_current(PackKind::Widgets, &ns("nulle-part")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownNamespace);
}

#[test]
fn pack_make_current_is_refused_while_editing_pack() {
    let (me, _them, mut opener, theirs, mine) = opened_on_newer();
    let id = user_pack_id(&opener);
    let mut s = crate::test_support_packs::pack_tab_from(&mut opener, me.path(), crate::pack::ESSENTIEL_PACK);
    let err = s.pack_make_current(PackKind::Widgets, &namespace_of_pack(&theirs)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::EditingPack);
    assert_that!(s.library.current(id)).is_equal_to(Some(mine.version()));
}

/// t-328 : une version reçue que la bibliothèque n'a pas pu ajouter n'empêche
/// pas l'ouverture ; le panneau ne propose pas d'en faire ma version courante.
#[test]
fn deck_packs_received_version_missing_from_library_offers_no_make_current() {
    let (me, them) = (fresh_dir("oku-slides-packs-me-"), fresh_dir("oku-slides-packs-them-"));
    let id = Uuid::new_v4();
    let v0 = files(id, "Charte", None, "2026-10-01T10:00:00Z");
    let v1 = files(id, "Charte", Some(&v0.version()), "2026-10-02T10:00:00Z");
    let path = received_file(them.path(), &v1);
    let mut s = session_on(me.path());
    s.library.add_version(id, &v0).unwrap();
    // Un fichier à la place du dossier de v1 : son écriture échoue.
    let versions = me.path().join("widgets").join(id.to_string()).join("versions");
    std::fs::write(versions.join(v1.version().hex()), b"").unwrap();
    s.open(&path).unwrap();
    assert_that!(s.library.version(id, &v1.version())).is_none();
    let view = s.deck_packs().unwrap();
    let pack = view.widgets.iter().find(|p| p.namespace == namespace_of(id)).unwrap();
    assert_that!((pack.name.as_str(), pack.library_has_pinned)).is_equal_to(("Charte", false));
    let ids = component_ids(&deck_packs_panel(&view, &s.localizer));
    let prefix = format!("widgets/{}", namespace_of(id));
    assert_that!(ids.contains(&format!("{prefix}/use-mine"))).is_true();
    assert_that!(ids.contains(&format!("{prefix}/make-current"))).is_false();
}

#[test]
fn deck_packs_deck_only_pack_has_no_relation() {
    let mut s = session();
    adopt_deck_only_pack(&mut s);
    let view = s.deck_packs().unwrap();
    let own = view.widgets.iter().find(|p| p.deck_only).unwrap();
    assert_that!(own.drift).is_none();
}

/// v0 ← v1 ; v0 ← v2. `mine` : ma version courante ; `theirs` : celle du fichier.
#[rstest]
#[case::newer(0, 1, Some(Drift::Newer))]
#[case::older(1, 0, Some(Drift::Older))]
#[case::parallel(1, 2, Some(Drift::Parallel))]
#[case::same(1, 1, None)]
fn deck_packs_relation_compares_the_pin_with_my_current_version(
    #[case] mine: usize,
    #[case] theirs: usize,
    #[case] expected: Option<Drift>,
) {
    let (me, them) = (fresh_dir("oku-slides-packs-me-"), fresh_dir("oku-slides-packs-them-"));
    let id = Uuid::new_v4();
    let v0 = files(id, "Charte", None, "2026-10-01T10:00:00Z");
    let v1 = files(id, "Charte", Some(&v0.version()), "2026-10-02T10:00:00Z");
    let v2 = files(id, "Charte", Some(&v0.version()), "2026-10-03T10:00:00Z");
    let v = [v0, v1, v2];
    let mut s = session_on(me.path());
    s.library.add_version(id, &v[0]).unwrap();
    s.library.add_version(id, &v[1]).unwrap();
    s.library.set_current(id, &v[mine].version()).unwrap();
    s.open(&received_file(them.path(), &v[theirs])).unwrap();
    let view = s.deck_packs().unwrap();
    let pack = view.widgets.iter().find(|p| p.namespace == namespace_of(id)).unwrap();
    assert_that!(pack.drift).is_equal_to(expected);
    assert_that!(s.library.current(id)).is_equal_to(Some(v[mine].version()));
}

#[test]
fn deck_packs_outside_deck_mode_is_refused() {
    let data = crate::test_support::fresh_dir("oku-slides-pack-tab-");
    let s = crate::test_support_packs::pack_tab_of(data.path(), crate::pack::ESSENTIEL_PACK);
    assert_that!(s.deck_packs().unwrap_err().kind()).is_equal_to(SessionErrorKind::EditingPack);
}

#[test]
fn pack_use_mine_switches_the_deck_and_undo_brings_theirs_back() {
    let (_me, _them, mut s, theirs, mine) = opened_on_newer();
    let id = user_pack_id(&s);
    s.pack_use_mine(PackKind::Widgets, &namespace_of_pack(&theirs)).unwrap();
    assert_that!(pinned_version(&s, id)).is_equal_to(Some(mine.version()));
    assert_that!(s.sources_for_tests().packs.contains_key(&mine.version())).is_true();
    assert_that!(undo_label(&s)).is_equal_to(Some(s.localizer.tr("packs-undo-use-mine")));
    assert_that!(s.library.current(id)).is_equal_to(Some(mine.version()));
    s.undo().unwrap();
    assert_that!(pinned_version(&s, id)).is_equal_to(Some(theirs.version()));
}

#[test]
fn pack_use_mine_is_refused_while_editing_pack() {
    let (me, _them, mut opener, theirs, _mine) = opened_on_newer();
    let mut s = crate::test_support_packs::pack_tab_from(&mut opener, me.path(), crate::pack::ESSENTIEL_PACK);
    let err = s.pack_use_mine(PackKind::Widgets, &namespace_of_pack(&theirs)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::EditingPack);
}

/// Le panneau ne propose « ma version » qu'avec une relation : un contexte forgé sur la version courante est refusé.
#[test]
fn pack_use_mine_on_the_current_version_is_refused_and_changes_nothing() {
    let (_me, _them, mut s, theirs, _mine) = opened_on_newer();
    s.pack_use_mine(PackKind::Widgets, &namespace_of_pack(&theirs)).unwrap();
    let after = (s.deck().clone(), undo_label(&s));
    let err = s.pack_use_mine(PackKind::Widgets, &namespace_of_pack(&theirs)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::PackAlreadyCurrent);
    assert_that!((s.deck().clone(), undo_label(&s))).is_equal_to(after);
    let (fr, en) = (err.localize(&Localizer::new(Some("fr"))), err.localize(&Localizer::new(Some("en"))));
    assert_that!(fr).is_not_equal_to(en);
}

#[test]
fn pack_use_mine_deck_only_pack_is_refused() {
    let mut s = session();
    adopt_deck_only_pack(&mut s);
    let err = s.pack_use_mine(PackKind::Widgets, &Namespace::deck()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::DeckOnly);
}

#[test]
fn pack_updates_newer_library_counts_one_and_use_mine_clears_it() {
    let (_a, _b, mut s, theirs, _mine) = opened_older_than_library();
    assert_that!(s.pack_updates()).is_equal_to(1);
    s.pack_use_mine(PackKind::Widgets, &namespace_of_pack(&theirs)).unwrap();
    assert_that!(s.pack_updates()).is_equal_to(0);
}

#[test]
fn pack_updates_newer_deck_counts_nothing() {
    let (_a, _b, s, _theirs, _mine) = opened_on_newer();
    assert_that!(s.pack_updates()).is_equal_to(0);
}

#[test]
fn pack_updates_follow_undo_and_redo() {
    let (_a, _b, mut s, theirs, _mine) = opened_older_than_library();
    s.pack_use_mine(PackKind::Widgets, &namespace_of_pack(&theirs)).unwrap();
    s.undo().unwrap();
    assert_that!(s.pack_updates()).is_equal_to(1);
    s.redo().unwrap();
    assert_that!(s.pack_updates()).is_equal_to(0);
}

#[test]
fn pack_updates_make_current_of_the_older_pin_clears_it() {
    let (_a, _b, mut s, theirs, _mine) = opened_older_than_library();
    s.pack_make_current(PackKind::Widgets, &namespace_of_pack(&theirs)).unwrap();
    assert_that!(s.pack_updates()).is_equal_to(0);
}

#[test]
fn pack_updates_snapshot_reads_the_count() {
    let (_a, _b, s, _theirs, _mine) = opened_older_than_library();
    assert_that!(s.snapshot().pack_updates).is_equal_to(1);
}

/// Une session dont le deck épingle la version v0 du thème `marque`, la bibliothèque passée à v1.
fn session_with_outdated_theme() -> (tempfile::TempDir, Session) {
    let data = fresh_dir("oku-slides-packs-theme-");
    let mut s = opened_on(data.path());
    let id = Uuid::new_v4();
    let v0 = add_user_theme(&s.themes, id, "marque", &[], serde_json::Map::default(), None);
    s.set_theme(&ns("marque")).unwrap();
    let mut tokens = serde_json::Map::new();
    tokens.insert("color".into(), serde_json::json!({"accent": "#204060"}));
    add_user_theme(&s.themes, id, "marque", &[], tokens, Some(&v0));
    s.theme_list = s.themes.themes();
    s.refresh_pack_updates();
    (data, s)
}

#[test]
fn deck_packs_theme_older_than_library_is_outdated_and_counted() {
    let (_data, s) = session_with_outdated_theme();
    let view = s.deck_packs().unwrap();
    assert_that!((view.theme.active.as_str(), view.theme.outdated)).is_equal_to(("marque", true));
    assert_that!(s.pack_updates()).is_equal_to(1);
}

#[test]
fn update_deck_theme_pins_my_current_version_in_one_labelled_step() {
    let (_data, mut s) = session_with_outdated_theme();
    s.update_deck_theme().unwrap();
    assert_that!(s.deck_packs().unwrap().theme.outdated).is_false();
    assert_that!(s.pack_updates()).is_equal_to(0);
    let expected = s.localizer.tr_with("packs-undo-theme-update", &[("name", "marque".into())]);
    assert_that!(undo_label(&s)).is_equal_to(Some(expected));
}

/// Le panneau ne propose la mise à jour qu'à un thème plus ancien : un contexte forgé est refusé.
#[test]
fn update_deck_theme_on_a_current_theme_is_refused_and_changes_nothing() {
    let mut s = session();
    let before = (s.deck().clone(), undo_label(&s));
    let err = s.update_deck_theme().unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::ThemeNotOutdated);
    assert_that!((s.deck().clone(), undo_label(&s))).is_equal_to(before);
    let (fr, en) = (err.localize(&Localizer::new(Some("fr"))), err.localize(&Localizer::new(Some("en"))));
    assert_that!(fr).is_not_equal_to(en);
}

#[test]
fn update_deck_theme_after_updating_is_refused() {
    let (_data, mut s) = session_with_outdated_theme();
    s.update_deck_theme().unwrap();
    assert_that!(s.update_deck_theme().unwrap_err().kind()).is_equal_to(SessionErrorKind::ThemeNotOutdated);
}

#[test]
fn deck_packs_panel_offers_no_theme_update_on_a_current_theme() {
    let s = session();
    let view = s.deck_packs().unwrap();
    let ids = component_ids(&deck_packs_panel(&view, &Localizer::new(Some("en"))));
    assert_that!(ids.contains(&"theme-update".to_owned())).is_false();
}

#[test]
fn deck_packs_theme_lists_the_library_themes() {
    let s = session();
    let view = s.deck_packs().unwrap();
    let choices: Vec<&str> = view.theme.choices.iter().map(|(n, _)| n.as_str()).collect();
    assert_that!(choices).is_equal_to(vec!["papier", "pro", "neutre"]);
    assert_that!((view.theme.active.as_str(), view.theme.overrides)).is_equal_to(("papier", 0));
}

#[rstest]
#[case::make_current(true)]
#[case::delete(false)]
fn leased_pack_cannot_be_made_current_or_deleted(#[case] make_current: bool) {
    let data = fresh_dir("oku-slides-packs-leased-");
    let (mut s, _deck, acme) = crate::test_support_packs::opener_on(data.path());
    let mut tab = crate::test_support_packs::pack_tab_from(&mut s, data.path(), acme);
    tab.apply("x", vec![crate::test_support::rename_title("acme 2")], None).unwrap();
    let current = tab.save_pack(false).unwrap();
    let ns = s.deck().widgets.iter().find(|(_, p)| p.id == acme).map(|(ns, _)| ns.clone()).unwrap();
    let err = if make_current {
        s.pack_make_current(PackKind::Widgets, &ns).unwrap_err()
    } else {
        s.handle(Request::DeletePack { pack: acme }).unwrap_err()
    };
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::PackOpenInTab);
    assert_that!(s.library.current(acme)).is_equal_to(Some(current));
}
