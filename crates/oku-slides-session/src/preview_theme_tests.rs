use rstest::rstest;
use serde_json::{Map, Value};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::document_kind::DocumentKind;
use crate::pack::PackManifest;
use crate::test_support::{adopt_deck_only_pack, fresh_dir, library_of, opened_on, rename_title, session_on};
use crate::test_support_packs::{
    deck_pack_tab_of, live_editor, ok, opener_on, pack_tab_from, park, read_only_pack_tab, scope_of,
};
use crate::theme::PAPIER_THEME;
use crate::{Localize, Request, Session, SessionErrorKind};

/// Which pack tab a case works on.
#[derive(Debug, Clone, Copy)]
enum Tab {
    /// A tab on a library pack.
    Library,
    /// A tab on a deck-only pack of its owner.
    DeckPack,
}

/// A pack tab and what keeps it alive.
struct Fixture {
    _data: tempfile::TempDir,
    _owner: Session,
    _deck_dir: Option<tempfile::TempDir>,
    tab: Session,
    pack: Uuid,
}

fn fixture(kind: Tab) -> Fixture {
    let data = fresh_dir("oku-slides-preview-theme-");
    match kind {
        Tab::Library => {
            let (mut owner, deck_dir, pack) = opener_on(data.path());
            let tab = pack_tab_from(&mut owner, data.path(), pack);
            Fixture { _data: data, _owner: owner, _deck_dir: Some(deck_dir), tab, pack }
        }
        Tab::DeckPack => {
            let mut owner = opened_on(data.path());
            let pin = adopt_deck_only_pack(&mut owner);
            let tab = deck_pack_tab_of(&mut owner, data.path(), pin.id);
            Fixture { _data: data, _owner: owner, _deck_dir: None, tab, pack: pin.id }
        }
    }
}

fn choose(s: &mut Session, theme: &str) -> crate::Result<String> {
    s.handle(Request::SetPreviewTheme { theme: theme.into() })
}

fn theme_of(s: &Session) -> Option<String> {
    s.snapshot().theme.map(|ns| ns.as_str().to_owned())
}

const PRO: &str = "pro";

fn versions(s: &Session, pack: Uuid) -> usize {
    library_of(s).versions(pack).len()
}

#[rstest]
#[case::library(Tab::Library)]
#[case::deck_pack(Tab::DeckPack)]
fn set_preview_theme_in_pack_tab_changes_working_deck_theme(#[case] kind: Tab) {
    let mut f = fixture(kind);
    ok(choose(&mut f.tab, "pro"), "preview theme");
    assert_that!(theme_of(&f.tab)).is_equal_to(Some(PRO.to_owned()));
}

#[test]
fn set_preview_theme_leaves_tab_clean() {
    let mut f = fixture(Tab::Library);
    ok(choose(&mut f.tab, "pro"), "preview theme");
    assert_that!(f.tab.pack_tab_info().map(|i| i.dirty)).is_equal_to(Some(false));
    assert_that!(f.tab.unsaved()).is_false();
}

#[test]
fn set_preview_theme_adds_no_undo_step() {
    let mut f = fixture(Tab::Library);
    ok(choose(&mut f.tab, "pro"), "preview theme");
    ok(f.tab.undo(), "undo");
    assert_that!(theme_of(&f.tab)).is_equal_to(Some(PRO.to_owned()));
}

#[test]
fn set_preview_theme_after_an_edit_keeps_it_through_undo() {
    let mut f = fixture(Tab::Library);
    let title = f.tab.doc.sources.deck().params.title.clone();
    ok(f.tab.apply("x", vec![rename_title("acme 2")], None), "edit");
    ok(choose(&mut f.tab, "pro"), "preview theme");
    ok(f.tab.undo(), "undo");
    assert_that!(f.tab.doc.sources.deck().params.title.clone()).is_equal_to(title);
    assert_that!(theme_of(&f.tab)).is_equal_to(Some(PRO.to_owned()));
}

#[rstest]
#[case::library(Tab::Library)]
#[case::deck_pack(Tab::DeckPack)]
fn set_preview_theme_writes_no_library_version(#[case] kind: Tab) {
    let mut f = fixture(kind);
    let before = versions(&f.tab, f.pack);
    ok(choose(&mut f.tab, "pro"), "preview theme");
    assert_that!(versions(&f.tab, f.pack)).is_equal_to(before);
    assert_that!(f.tab.unsent()).is_false();
    assert_that!(f.tab.take_deck_pack_send().is_none()).is_true();
}

#[test]
fn set_preview_theme_leaves_preview_deck_unchanged() {
    let mut f = fixture(Tab::Library);
    let opener = f.tab.pack_tab_info().map(|i| i.theme);
    ok(choose(&mut f.tab, "pro"), "preview theme");
    assert_that!(f.tab.pack_tab_info().map(|i| i.theme)).is_equal_to(opener);
}

#[test]
fn set_preview_theme_survives_resume() {
    let data = fresh_dir("oku-slides-preview-theme-");
    let mut live = live_editor(data.path());
    ok(choose(&mut live.editor, "pro"), "preview theme");
    let dormant = park(live);
    let mut back = session_on(data.path());
    let result = back.resume_checked(dormant.dir.path().to_path_buf(), None);
    assert_that!(result).is_equal_to(crate::resume::Resumed::Restored);
    assert_that!(theme_of(&back)).is_equal_to(Some(PRO.to_owned()));
}

#[test]
fn set_preview_theme_without_lease_is_refused() {
    let data = fresh_dir("oku-slides-preview-theme-");
    let mut r = read_only_pack_tab(data.path());
    let before = theme_of(&r.tab);
    let refused = choose(&mut r.tab, "pro").unwrap_err();
    assert_that!(refused.kind()).is_equal_to(SessionErrorKind::PackOpenInTab);
    assert_that!(theme_of(&r.tab)).is_equal_to(before);
}

#[test]
fn set_preview_theme_after_holder_closed_adds_no_version() {
    let data = fresh_dir("oku-slides-preview-theme-");
    let r = read_only_pack_tab(data.path());
    let (mut tab, pack) = (r.tab, r.pack);
    drop(r.holder);
    let before = versions(&tab, pack);
    ok(choose(&mut tab, "pro"), "preview theme");
    assert_that!(theme_of(&tab)).is_equal_to(Some(PRO.to_owned()));
    assert_that!(versions(&tab, pack)).is_equal_to(before);
    assert_that!(crate::pack_lease::pack_leased(&scope_of(&tab), pack).is_some()).is_true();
}

/// What a saved pack tab version holds: its manifest, widget sources and requirements.
type Saved = (Map<String, Value>, Vec<(String, String)>, Vec<String>);

/// The manifest (without the ids of the pack and of its parent) and widgets of the version a fresh
/// pack tab saves after `before` ran on it, with its requirements.
fn saved_after(before: impl FnOnce(&mut Session)) -> Saved {
    let data = fresh_dir("oku-slides-preview-theme-");
    let (mut owner, _deck_dir, pack) = opener_on(data.path());
    let mut tab = pack_tab_from(&mut owner, data.path(), pack);
    before(&mut tab);
    ok(tab.apply("x", vec![rename_title("acme 2")], None), "edit");
    let version = ok(tab.save_pack(false), "save");
    let files = library_of(&tab).version(pack, &version).unwrap_or_else(|| panic!("saved version missing"));
    let mut manifest = files.manifest().clone();
    // Each fixture makes its own pack (id, parent) at its own second (created).
    manifest.remove("id");
    manifest.remove("parent");
    manifest.remove("created");
    let widgets = files.widgets().iter().map(|(name, source)| (name.as_str().to_owned(), source.clone())).collect();
    let requires = PackManifest::from_files(&files).unwrap().requires.iter().map(ToString::to_string).collect();
    (manifest, widgets, requires)
}

#[test]
fn set_preview_theme_then_save_writes_same_pack_files() {
    let with = saved_after(|t| drop(ok(choose(t, "pro"), "preview theme")));
    let without = saved_after(|_| ());
    let keys: Vec<_> = with.0.keys().chain(without.0.keys()).collect();
    for key in keys {
        assert_that!(with.0.get(key)).named(key).is_equal_to(without.0.get(key));
    }
    assert_that!(with.1).is_equal_to(without.1);
    assert_that!(with.2).is_equal_to(without.2);
}

#[rstest]
#[case::deck(DocumentKind::Deck)]
#[case::theme(DocumentKind::Theme)]
#[case::help(DocumentKind::Help)]
fn set_preview_theme_outside_pack_tab_is_refused(#[case] kind: DocumentKind) {
    let data = fresh_dir("oku-slides-preview-theme-");
    let mut s = session_on(data.path());
    match kind {
        DocumentKind::Theme => s.open_theme_here(PAPIER_THEME).unwrap(),
        DocumentKind::Help => s.open_help("fr", None).unwrap(),
        _ => {}
    }
    let refused = choose(&mut s, "pro").unwrap_err();
    assert_that!(refused.kind()).is_equal_to(SessionErrorKind::WrongDocument);
    let expected = s.localizer.tr_with("session-wrong-document", &[("kind", kind.as_str().into())]);
    assert_that!(refused.localize(&s.localizer)).is_equal_to(expected);
}

#[test]
fn set_preview_theme_in_history_is_refused() {
    let data = fresh_dir("oku-slides-preview-theme-");
    let mut s = session_on(data.path());
    s.history_open().unwrap();
    let refused = choose(&mut s, "pro").unwrap_err();
    assert_that!(refused.kind()).is_equal_to(SessionErrorKind::InHistory);
}

#[test]
fn set_preview_theme_unknown_theme_is_refused() {
    let mut f = fixture(Tab::Library);
    let refused = choose(&mut f.tab, "inconnu").unwrap_err();
    assert_that!(refused.kind()).is_equal_to(SessionErrorKind::UnknownTheme);
}
