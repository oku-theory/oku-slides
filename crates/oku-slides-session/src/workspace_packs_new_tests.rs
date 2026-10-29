//! `NewPack`: an empty library pack opens in its own tab, and goes when that tab leaves untouched.

use std::sync::atomic::Ordering;

use oku_slides_core::{PackKind, PackVersion};
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;
use uuid::Uuid;

use super::workspace_packs_fixtures::{Packs, deck_packs, duplicate_tab, packs, packs_resumed};
use crate::test_support::{edit_of, rename_title};
use crate::workspace::tabs::FAIL_NEXT_OPEN;
use crate::{PackLibrary, Request, Result, SessionErrorKind, TabDocument, Workspace, WorkspaceOutcome};

/// The requester of a new pack in the fixture.
#[derive(Debug, Clone, Copy)]
enum Requester {
    /// The opener deck `t-1`.
    Deck,
    /// A library pack tab, of `acme`.
    PackTab,
    /// A deck-only pack's tab, linked to `t-1`.
    DeckPackTab,
}

/// The fixture for `requester`, and the requesting tab.
fn requested_by(requester: Requester) -> (Packs, String) {
    match requester {
        Requester::Deck => (packs(), "t-1".to_owned()),
        Requester::PackTab => {
            let f = packs();
            let tab = f.pack_tab();
            (f, tab)
        }
        Requester::DeckPackTab => {
            let (f, pin) = deck_packs();
            let tab = f.deck_pack_tab(pin.id);
            (f, tab)
        }
    }
}

impl Packs {
    /// Asks `tab` for a new pack of `kind`.
    fn new_pack(&self, tab: &str, kind: PackKind) -> Result<WorkspaceOutcome> {
        let kind = serde_json::to_value(kind).unwrap_or_else(|e| panic!("{e}"));
        let req = serde_json::from_value(json!({"cmd": "new_pack", "tab": tab, "kind": kind}))
            .unwrap_or_else(|e| panic!("request: {e}"));
        Workspace::apply(&self.ws, req)
    }

    /// A new pack of `kind` from `tab`, which must open: its tab and its id.
    fn created(&self, tab: &str, kind: PackKind) -> (String, Uuid) {
        self.new_pack(tab, kind).unwrap_or_else(|e| panic!("new pack: {}", e.one_line()));
        let window = self.window_of(tab);
        let at = window.iter().position(|t| t == tab).unwrap_or_else(|| panic!("{tab} in its window"));
        let new = window.get(at + 1).cloned().unwrap_or_else(|| panic!("a tab after {tab}"));
        match self.document(&new) {
            Some(TabDocument::Pack { pack }) => (new, pack),
            other => panic!("a pack tab after {tab}: {other:?}"),
        }
    }

    /// The tabs of the window holding `tab`.
    fn window_of(&self, tab: &str) -> Vec<String> {
        let w = Workspace::lock(&self.ws);
        let found = w.layout.windows.iter().find(|win| win.tabs.iter().any(|t| t == tab));
        found.map(|win| win.tabs.clone()).unwrap_or_default()
    }

    /// The workspace's library.
    fn library(&self) -> PackLibrary {
        Workspace::lock(&self.ws).library.clone()
    }

    /// The library packs, both kinds.
    fn pack_count(&self) -> usize {
        let library = self.library();
        PackKind::ALL.into_iter().map(|kind| library.packs(kind).len()).sum()
    }

    /// Whether the library still has pack `id`.
    fn has_pack(&self, id: Uuid) -> bool {
        self.library().current(id).is_some()
    }

    /// The cached `created` of `tab`.
    fn created_of(&self, tab: &str) -> Option<Uuid> {
        Workspace::lock(&self.ws).tabs.get(tab).and_then(|t| t.info.created)
    }

    /// A rename then a save in pack tab `tab`: the saved version.
    fn saved(&self, tab: &str, as_new: bool) -> PackVersion {
        self.with(tab, |s| {
            s.apply("x", vec![rename_title("Saved")], None).unwrap_or_else(|e| panic!("edit: {}", e.one_line()));
            s.save_pack(as_new).unwrap_or_else(|e| panic!("save: {}", e.one_line()))
        })
    }

    /// Sends a workspace request; its outcome.
    fn apply(&self, v: serde_json::Value) -> Result<WorkspaceOutcome> {
        Workspace::apply(&self.ws, serde_json::from_value(v).unwrap_or_else(|e| panic!("request: {e}")))
    }

    /// Moves `tab` to `window`, or to a new window: the label of its window.
    fn move_to(&self, tab: &str, window: Option<&str>) -> String {
        self.send(json!({"cmd": "move_tab", "tab": tab, "window": window}));
        let w = Workspace::lock(&self.ws);
        let found = w.layout.windows.iter().find(|win| win.tabs.iter().any(|t| t == tab));
        found.map_or_else(|| panic!("{tab} in a window"), |win| win.label.clone())
    }
}

#[rstest]
fn new_pack_opens_created_pack_tab_after_requester(
    #[values(Requester::Deck, Requester::PackTab, Requester::DeckPackTab)] requester: Requester,
    #[values(PackKind::Widgets, PackKind::Layouts)] kind: PackKind,
) {
    let (f, tab) = requested_by(requester);
    let (new, id) = f.created(&tab, kind);
    let (slides, edited) = f.with(&new, |s| (s.snapshot().slides.len(), edit_of(s).kind));
    assert_that!((slides, edited, f.created_of(&new))).is_equal_to((0, kind, Some(id)));
}

/// G9: an empty working deck renders, snapshots and resumes as the tab of its pack.
#[test]
fn new_pack_empty_working_deck_snapshots_and_resumes() {
    let f = packs_resumed();
    let (new, id) = f.created("t-1", PackKind::Widgets);
    Workspace::request(&f.ws, &new, Request::Snapshot).0.unwrap_or_else(|e| panic!("{}", e.one_line()));
    let f = f.restart(|_, _| {});
    Workspace::request(&f.ws, &new, Request::Snapshot).0.unwrap_or_else(|e| panic!("{}", e.one_line()));
    let info = f.with(&new, |s| s.tab_info());
    assert_that!((info.document, info.created)).is_equal_to((TabDocument::Pack { pack: id }, Some(id)));
}

/// Opening freezes the empty version (an adopt marker): the first save is its child.
#[test]
fn new_pack_first_save_is_child_of_frozen_empty_version() {
    let f = packs();
    let (new, id) = f.created("t-1", PackKind::Widgets);
    let empty = f.library().current(id).unwrap_or_else(|| panic!("the empty version"));
    let saved = f.saved(&new, false);
    let parent = f.library().versions(id).into_iter().find(|v| v.version == saved).and_then(|v| v.manifest.parent);
    assert_that!((parent, f.library().versions(id).len())).is_equal_to((Some(empty), 2));
}

/// The requester that may not create a pack: the setup, its tab and the expected refusal.
#[derive(Debug, Clone, Copy)]
enum Refused {
    /// A help tab.
    Help,
    /// A theme tab.
    Theme,
    /// A deck in history.
    History,
    /// A tab whose library has no lease scope.
    NoScope,
}

#[rstest]
#[case::help(Refused::Help, SessionErrorKind::ReadOnly)]
#[case::theme(Refused::Theme, SessionErrorKind::WrongDocument)]
#[case::history(Refused::History, SessionErrorKind::InHistory)]
#[case::no_scope(Refused::NoScope, SessionErrorKind::SingleDocument)]
fn new_pack_refused_requester_writes_nothing(#[case] requester: Refused, #[case] expected: SessionErrorKind) {
    let f = packs();
    let tab = match requester {
        Refused::Help => {
            f.send(json!({"cmd": "open_help", "window": "w-1"}));
            f.window(0).last().cloned().unwrap_or_else(|| panic!("a help tab"))
        }
        Refused::Theme => {
            f.send(json!({"cmd": "open_theme", "tab": "t-1", "theme": crate::theme::PAPIER_THEME}));
            f.window(0)[1].clone()
        }
        Refused::History => {
            f.with("t-1", |s| s.handle(Request::HistoryOpen)).unwrap_or_else(|e| panic!("{}", e.one_line()));
            "t-1".to_owned()
        }
        Refused::NoScope => {
            f.with("t-1", |s| {
                let storage = crate::Storage::local().unwrap_or_else(|e| panic!("{e}"));
                s.library = PackLibrary::new(None, None, s.library.themes().clone(), storage);
            });
            "t-1".to_owned()
        }
    };
    let (before, tabs) = (f.pack_count(), f.tab_count());
    let err = f.new_pack(&tab, PackKind::Widgets).map(|_| ()).map_err(|e| e.kind());
    assert_that!((err, f.pack_count(), f.tab_count())).is_equal_to((Err(expected), before, tabs));
}

#[test]
fn new_pack_failed_open_deletes_the_draft() {
    let f = packs();
    let (before, tabs) = (f.pack_count(), f.tab_count());
    FAIL_NEXT_OPEN.store(true, Ordering::SeqCst);
    let r = f.new_pack("t-1", PackKind::Layouts).map(|_| ()).map_err(|e| e.kind());
    assert_that!((r, f.pack_count(), f.tab_count())).is_equal_to((
        Err(SessionErrorKind::PackTabNotFresh),
        before,
        tabs,
    ));
}

#[test]
fn new_pack_closed_unsaved_is_deleted_from_library() {
    let f = packs();
    let (new, id) = f.created("t-1", PackKind::Widgets);
    f.close_keeping(&new).unwrap_or_else(|e| panic!("close: {}", e.one_line()));
    assert_that!(f.has_pack(id)).is_false();
}

#[test]
fn new_pack_closed_with_its_window_unsaved_is_deleted() {
    let f = packs();
    let (new, id) = f.created("t-1", PackKind::Widgets);
    let window = f.move_to(&new, None);
    f.apply(json!({"cmd": "close_window", "window": window})).unwrap_or_else(|e| panic!("{}", e.one_line()));
    assert_that!(f.has_pack(id)).is_false();
}

/// The new pack's tab closes in one batch with a deck and its linked deck-pack tab.
#[test]
fn new_pack_closed_as_linked_tab_unsaved_is_deleted() {
    let (f, pin) = deck_packs();
    let linked = f.deck_pack_tab(pin.id);
    let (new, id) = f.created(&linked, PackKind::Widgets);
    f.new_tab();
    let window = f.move_to("t-1", None);
    f.move_to(&new, Some(&window));
    f.apply(json!({"cmd": "close_window", "window": window})).unwrap_or_else(|e| panic!("{}", e.one_line()));
    let gone = [&linked, &new].map(|t| Workspace::lock(&f.ws).tabs.contains_key(t.as_str()));
    assert_that!((gone, f.has_pack(id))).is_equal_to(([false, false], false));
}

#[test]
fn new_pack_last_window_closed_keeps_the_draft() {
    let f = packs();
    let (_, id) = f.created("t-1", PackKind::Widgets);
    let outcome = f.apply(json!({"cmd": "close_window", "window": "w-1"}));
    let last = matches!(outcome, Ok(WorkspaceOutcome::LastWindow(_)));
    assert_that!((last, f.has_pack(id))).is_equal_to((true, true));
}

#[test]
fn new_pack_closed_after_save_stays_in_library() {
    let f = packs();
    let (new, id) = f.created("t-1", PackKind::Widgets);
    f.saved(&new, false);
    f.close_keeping(&new).unwrap_or_else(|e| panic!("close: {}", e.one_line()));
    assert_that!(f.has_pack(id)).is_true();
}

#[test]
fn new_pack_saved_then_undone_closed_stays_in_library() {
    let f = packs();
    let (new, id) = f.created("t-1", PackKind::Widgets);
    f.saved(&new, false);
    f.with(&new, |s| s.handle(Request::Undo)).unwrap_or_else(|e| panic!("undo: {}", e.one_line()));
    f.close(&new);
    assert_that!(f.has_pack(id)).is_true();
}

#[test]
fn new_pack_saved_as_copy_closed_deletes_only_the_original() {
    let f = packs();
    let (new, id) = f.created("t-1", PackKind::Widgets);
    f.saved(&new, true);
    let copy = match f.with(&new, |s| s.tab_info().document) {
        TabDocument::Pack { pack } => pack,
        other => panic!("a pack tab: {other:?}"),
    };
    f.close(&new);
    assert_that!((copy != id, f.has_pack(id), f.has_pack(copy))).is_equal_to((true, false, true));
}

/// A second tab of the draft (restored read-only) still shows it: the first one's close keeps it.
#[test]
fn new_pack_shown_by_another_tab_is_kept_at_close() {
    let f = packs_resumed();
    let (new, id) = f.created("t-1", PackKind::Widgets);
    let f = f.restart(|resume, layout| duplicate_tab(resume, layout, &new, "t-9"));
    Workspace::request(&f.ws, &new, Request::Snapshot).0.unwrap_or_else(|e| panic!("{}", e.one_line()));
    f.close(&new);
    assert_that!((f.document("t-9"), f.has_pack(id))).is_equal_to((Some(TabDocument::Pack { pack: id }), true));
}

/// A restored tab closed before it wakes: `created` comes from `workspace.json`.
#[test]
fn new_pack_resumed_dormant_tab_closed_unsaved_is_deleted() {
    let f = packs_resumed();
    let (new, id) = f.created("t-1", PackKind::Layouts);
    let f = f.restart(|_, _| {});
    let dormant = f.is_dormant(&new);
    f.close(&new);
    assert_that!((dormant, f.has_pack(id))).is_equal_to((true, false));
}

#[test]
fn new_pack_saved_then_resumed_closed_is_kept() {
    let f = packs_resumed();
    let (new, id) = f.created("t-1", PackKind::Widgets);
    f.saved(&new, false);
    let f = f.restart(|_, _| {});
    f.close(&new);
    assert_that!(f.has_pack(id)).is_true();
}

/// The workspace speaks French: the pack is named and namespaced in it.
#[test]
fn new_pack_from_fr_requester_uses_localized_name() {
    let f = packs();
    let (new, id) = f.created("t-1", PackKind::Widgets);
    let title = f.with(&new, |s| s.snapshot().title);
    let ns = f.library().current_version(id).map(|s| s.manifest.namespace.to_string());
    assert_that!((title, ns)).is_equal_to((Some("Mon pack".to_owned()), Some("mon-pack".to_owned())));
}
