use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

use super::super::opening::{opening_waiters, with_opening};
use super::super::packs::workspace_packs_fixtures::within;
use crate::agent::{TabGate, TabOpen};
use crate::pack_lease::pack_leased;
use crate::test_support::{add_user_theme, apply_ws, fresh_dir, resumed_on, workspace_on};
use crate::test_support_packs::{assert_unleased, theme_scope_of};
use crate::theme::PAPIER_THEME;
use crate::theme_tab::theme_tab_tests::pro_tokens;
use crate::{BASE_PACK, Result, SessionErrorKind, TabDocument, Workspace, WorkspaceOutcome};

const WAIT: Duration = Duration::from_secs(10);

/// A workspace with one blank tab `t-1`, its data folder kept alive.
struct Themes {
    /// The workspace; dropped first.
    ws: Arc<Mutex<Workspace>>,
    /// The data folder; dropped last.
    _data: tempfile::TempDir,
}

/// The fixture without resume.
fn themes() -> Themes {
    let data = fresh_dir("oku-slides-ws-themes-");
    Themes { ws: Arc::new(workspace_on(data.path())), _data: data }
}

impl Themes {
    /// Sends a workspace request.
    fn send(&self, v: Value) -> Result<WorkspaceOutcome> {
        apply_ws(&self.ws, v)
    }

    /// Opens `theme` from `tab`.
    fn open(&self, tab: &str, theme: Uuid) -> Result<WorkspaceOutcome> {
        self.send(json!({"cmd": "open_theme", "tab": tab, "theme": theme}))
    }

    /// Opens `theme` from `tab` for an agent whose gate is `gate`.
    fn open_by_agent(&self, tab: &str, theme: Uuid, gate: &mut dyn FnMut(TabGate) -> bool) -> Result<TabOpen> {
        Workspace::open_theme_for_agent(&self.ws, tab, theme, gate)
    }

    /// Opens `theme` from `tab`, which must work.
    fn opened(&self, tab: &str, theme: Uuid) {
        self.open(tab, theme).unwrap_or_else(|e| panic!("open theme: {}", e.one_line()));
    }

    /// A blank tab at the end of `w-1`: its id.
    fn new_tab(&self) -> String {
        self.send(json!({"cmd": "new_tab", "window": "w-1"})).unwrap_or_else(|e| panic!("{}", e.one_line()));
        self.window().last().cloned().unwrap_or_else(|| panic!("a tab"))
    }

    /// Activates `tab`.
    fn activate(&self, tab: &str) {
        self.send(json!({"cmd": "activate", "tab": tab})).unwrap_or_else(|e| panic!("{}", e.one_line()));
    }

    /// The tabs of `w-1`, in bar order.
    fn window(&self) -> Vec<String> {
        Workspace::lock(&self.ws).layout.windows[0].tabs.clone()
    }

    /// The active tab of `w-1`.
    fn active(&self) -> String {
        Workspace::lock(&self.ws).layout.windows[0].active.clone()
    }

    /// The cached document of `tab`.
    fn document(&self, tab: &str) -> Option<TabDocument> {
        Workspace::lock(&self.ws).tabs.get(tab).map(|t| t.info.document.clone())
    }

    /// The tabs cached as theme tabs.
    fn theme_tabs(&self) -> Vec<String> {
        let w = Workspace::lock(&self.ws);
        let mut tabs: Vec<String> = w
            .tabs
            .iter()
            .filter(|(_, t)| matches!(t.info.document, TabDocument::Theme { .. }))
            .map(|(id, _)| id.clone())
            .collect();
        tabs.sort();
        tabs
    }

    /// The opener of `tab`.
    fn opener_of(&self, tab: &str) -> Option<String> {
        Workspace::lock(&self.ws).tabs.get(tab).and_then(|t| t.opener.clone())
    }

    /// The library's theme lease scope, read through `t-1`.
    fn theme_scope(&self) -> crate::pack_lease::LeaseScope {
        Workspace::with_session(&self.ws, "t-1", |s| theme_scope_of(s)).0.unwrap_or_else(|e| panic!("{}", e.one_line()))
    }
}

/// Opens `theme` from `tab` as the user on another thread; the receiver of its outcome.
fn open_on_a_thread(f: &Themes, tab: &str, theme: Uuid) -> mpsc::Receiver<Result<WorkspaceOutcome>> {
    let (ws, tab) = (Arc::clone(&f.ws), tab.to_owned());
    let (done, finished) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = done.send(apply_ws(&ws, json!({"cmd": "open_theme", "tab": tab, "theme": theme})));
    });
    finished
}

#[test]
fn open_theme_places_the_tab_after_the_requester_and_activates_it() {
    let f = themes();
    let other = f.new_tab();
    f.activate("t-1");
    f.opened("t-1", PAPIER_THEME);
    let tab = f.active();
    assert_that!(f.window()).is_equal_to(vec!["t-1".to_owned(), tab.clone(), other]);
    assert_that!(f.document(&tab)).is_equal_to(Some(TabDocument::Theme { theme: PAPIER_THEME }));
}

#[test]
fn open_theme_twice_focuses_existing_tab() {
    let f = themes();
    let other = f.new_tab();
    f.activate("t-1");
    f.opened("t-1", PAPIER_THEME);
    let tab = f.active();
    f.activate(&other);
    f.opened(&other, PAPIER_THEME);
    assert_that!((f.active(), f.theme_tabs())).is_equal_to((tab.clone(), vec![tab]));
}

#[test]
fn open_theme_concurrent_gives_one_tab() {
    let f = themes();
    let (a, b) = (open_on_a_thread(&f, "t-1", PAPIER_THEME), open_on_a_thread(&f, "t-1", PAPIER_THEME));
    let both = within(move || {
        let ok = |r: mpsc::Receiver<Result<WorkspaceOutcome>>| r.recv_timeout(WAIT).is_ok_and(|r| r.is_ok());
        (ok(a), ok(b))
    });
    assert_that!((both, f.theme_tabs().len())).is_equal_to(((true, true), 1));
}

#[test]
fn open_theme_unknown_is_refused_and_changes_nothing() {
    let f = themes();
    let before = (f.window(), Workspace::lock(&f.ws).layout.next_tab);
    let err = f.open("t-1", Uuid::new_v4()).err().map(|e| e.kind());
    let after = (f.window(), Workspace::lock(&f.ws).layout.next_tab);
    assert_that!((err, after)).is_equal_to((Some(SessionErrorKind::UnknownTheme), before));
}

/// Window `[t-1, T, t-2]`: the neighbour rule would give `t-2`.
#[test]
fn closing_a_theme_tab_activates_its_opener() {
    let f = themes();
    f.new_tab();
    f.activate("t-1");
    f.opened("t-1", PAPIER_THEME);
    let tab = f.active();
    f.send(json!({"cmd": "close", "tab": tab, "discard": true})).unwrap();
    assert_that!(f.active()).is_equal_to("t-1".to_owned());
}

#[test]
fn closing_a_theme_tab_frees_its_lease() {
    let f = themes();
    let scope = f.theme_scope();
    f.opened("t-1", PAPIER_THEME);
    let tab = f.active();
    assert_that!(pack_leased(&scope, PAPIER_THEME)).is_some();
    f.send(json!({"cmd": "close", "tab": tab, "discard": true})).unwrap();
    assert_unleased(&scope, PAPIER_THEME);
}

#[test]
fn pack_opener_of_a_theme_tab_is_none() {
    let f = themes();
    f.opened("t-1", PAPIER_THEME);
    let tab = f.active();
    let answer = f.send(json!({"cmd": "pack_opener", "tab": tab})).unwrap().into_json();
    assert_that!(answer.as_str()).is_equal_to("null");
}

#[test]
fn open_theme_from_a_help_tab_opens_a_theme_tab() {
    let f = themes();
    f.send(json!({"cmd": "open_help", "window": "w-1"})).unwrap();
    let help = f.active();
    f.opened(&help, PAPIER_THEME);
    let tab = f.active();
    let found = (f.document(&tab), f.opener_of(&tab));
    assert_that!(found).is_equal_to((Some(TabDocument::Theme { theme: PAPIER_THEME }), Some(help)));
}

#[test]
fn open_theme_of_a_user_theme_names_its_tab_after_it() {
    let f = themes();
    let maison = Uuid::new_v4();
    Workspace::with_session(&f.ws, "t-1", |s| add_user_theme(&s.themes, maison, "maison", &[], pro_tokens(), None))
        .0
        .unwrap();
    f.opened("t-1", maison);
    let tab = f.active();
    let title = Workspace::lock(&f.ws).tabs.get(&tab).map(|t| t.info.title.clone());
    assert_that!(title).is_equal_to(Some("maison".to_owned()));
}

/// A theme open waits on the theme's opening lock, held here; a pack open meanwhile goes on.
#[test]
fn open_pack_and_open_theme_share_the_opening_lock() {
    let f = themes();
    let theme_done = with_opening(PAPIER_THEME, || {
        let theme_done = open_on_a_thread(&f, "t-1", PAPIER_THEME);
        let deadline = Instant::now() + WAIT;
        while opening_waiters(PAPIER_THEME) != 1 {
            assert_that!(Instant::now() < deadline).is_true();
            std::thread::sleep(Duration::from_millis(2));
        }
        let ws = Arc::clone(&f.ws);
        let pack = within(move || apply_ws(&ws, json!({"cmd": "open_pack", "tab": "t-1", "pack": BASE_PACK})).is_ok());
        assert_that!((pack, theme_done.try_recv().is_err())).is_equal_to((true, true));
        theme_done
    });
    let theme = theme_done.recv_timeout(WAIT).is_ok_and(|r| r.is_ok());
    assert_that!((theme, f.theme_tabs().len())).is_equal_to((true, 1));
}

#[test]
fn workspace_layout_saves_a_theme_entry_with_null_legacy_keys() {
    let data = fresh_dir("oku-slides-ws-themes-");
    let resume = data.path().join("resume");
    let f = Themes { ws: Arc::new(resumed_on(data.path(), &resume)), _data: data };
    f.opened("t-1", PAPIER_THEME);
    let tab = f.active();
    let file: Value = serde_json::from_slice(&std::fs::read(resume.join("workspace.json")).unwrap()).unwrap();
    let entry = &file["info"][&tab];
    let keys = (entry["document"].clone(), entry["path"].clone(), entry["deck_uid"].clone(), entry["help"].clone());
    let expected = json!({"kind": "theme", "theme": PAPIER_THEME});
    assert_that!(keys).is_equal_to((expected, Value::Null, Value::Null, Value::Null));
    // Present and null, never missing: 6.0.0 reads these keys (the downgrade contract).
    let present: Vec<bool> = ["path", "deck_uid", "help"]
        .iter()
        .map(|k| entry.as_object().is_some_and(|o| o.get(*k) == Some(&Value::Null)))
        .collect();
    assert_that!(present).is_equal_to(vec![true, true, true]);
}

impl Themes {
    /// Asks `tab` for a new theme, which must open: its tab and its id.
    fn created(&self, tab: &str) -> (String, Uuid) {
        self.send(json!({"cmd": "new_theme", "tab": tab})).unwrap_or_else(|e| panic!("new theme: {}", e.one_line()));
        let window = self.window();
        let at = window.iter().position(|t| t == tab).unwrap_or_else(|| panic!("{tab} in w-1"));
        let new = window.get(at + 1).cloned().unwrap_or_else(|| panic!("a tab after {tab}"));
        match self.document(&new) {
            Some(TabDocument::Theme { theme }) => (new, theme),
            other => panic!("a theme tab after {tab}: {other:?}"),
        }
    }

    /// Whether the library still has theme `id`.
    fn has_theme(&self, id: Uuid) -> bool {
        Workspace::lock(&self.ws).library.themes().current(id).is_some()
    }

    /// The user's themes in the library.
    fn theme_count(&self) -> usize {
        Workspace::lock(&self.ws).library.themes().entries().len()
    }

    /// Closes `tab`, discarding its work.
    fn close(&self, tab: &str) {
        self.send(json!({"cmd": "close", "tab": tab, "discard": true})).unwrap_or_else(|e| panic!("{}", e.one_line()));
    }

    /// Sets a token in theme tab `tab`, then saves it (as a new theme with `as_new`): its document.
    fn saved(&self, tab: &str, as_new: bool) -> TabDocument {
        Workspace::with_session(&self.ws, tab, |s| {
            s.handle(crate::Request::Apply {
                label: "token".into(),
                ops: vec![oku_slides_core::Op::SetToken {
                    path: vec!["color".into(), "accent".into()],
                    value: json!("#ff0000"),
                }],
                merge: None,
            })
            .unwrap_or_else(|e| panic!("token: {}", e.one_line()));
            s.handle(crate::Request::SaveTheme { as_new }).unwrap_or_else(|e| panic!("save: {}", e.one_line()));
            s.tab_info().document
        })
        .0
        .unwrap_or_else(|e| panic!("{}", e.one_line()))
    }
}

#[rstest::rstest]
#[case::deck(false)]
#[case::help(true)]
fn new_theme_opens_created_theme_tab_after_requester(#[case] help: bool) {
    let f = themes();
    let tab = if help {
        f.send(json!({"cmd": "open_help", "window": "w-1"})).unwrap_or_else(|e| panic!("{}", e.one_line()));
        f.window().last().cloned().unwrap_or_else(|| panic!("a help tab"))
    } else {
        "t-1".to_owned()
    };
    let (new, id) = f.created(&tab);
    let created = Workspace::lock(&f.ws).tabs.get(&new).and_then(|t| t.info.created);
    assert_that!((f.active(), created, f.has_theme(id))).is_equal_to((new, Some(id), true));
}

#[test]
fn new_theme_closed_unsaved_is_deleted_from_library() {
    let f = themes();
    let (new, id) = f.created("t-1");
    f.close(&new);
    assert_that!(f.has_theme(id)).is_false();
}

#[test]
fn new_theme_closed_with_its_window_unsaved_is_deleted() {
    let f = themes();
    let (new, id) = f.created("t-1");
    f.send(json!({"cmd": "move_tab", "tab": new})).unwrap_or_else(|e| panic!("{}", e.one_line()));
    let window = Workspace::lock(&f.ws).layout.windows.iter().find(|w| w.tabs.contains(&new)).map(|w| w.label.clone());
    let window = window.unwrap_or_else(|| panic!("{new} in a window"));
    f.send(json!({"cmd": "close_window", "window": window})).unwrap_or_else(|e| panic!("{}", e.one_line()));
    assert_that!(f.has_theme(id)).is_false();
}

#[test]
fn new_theme_closed_after_save_stays_in_library() {
    let f = themes();
    let (new, id) = f.created("t-1");
    f.saved(&new, false);
    f.send(json!({"cmd": "close", "tab": new})).unwrap_or_else(|e| panic!("close: {}", e.one_line()));
    let open = Workspace::lock(&f.ws).tabs.contains_key(new.as_str());
    assert_that!((open, f.has_theme(id))).is_equal_to((false, true));
}

#[test]
fn new_theme_saved_then_undone_closed_stays_in_library() {
    let f = themes();
    let (new, id) = f.created("t-1");
    f.saved(&new, false);
    Workspace::with_session(&f.ws, &new, |s| s.handle(crate::Request::Undo))
        .0
        .unwrap_or_else(|e| panic!("{}", e.one_line()))
        .unwrap_or_else(|e| panic!("undo: {}", e.one_line()));
    f.close(&new);
    assert_that!(f.has_theme(id)).is_true();
}

#[test]
fn new_theme_saved_as_copy_closed_deletes_only_the_original() {
    let f = themes();
    let (new, id) = f.created("t-1");
    let copy = f.saved(&new, true);
    let TabDocument::Theme { theme: copy } = copy else { panic!("a theme tab: {copy:?}") };
    f.close(&new);
    assert_that!((copy != id, f.has_theme(id), f.has_theme(copy))).is_equal_to((true, false, true));
}

#[test]
fn new_theme_failed_open_deletes_the_draft() {
    let f = themes();
    let (before, tabs) = (f.theme_count(), f.window().len());
    super::super::tabs::FAIL_NEXT_OPEN.store(true, std::sync::atomic::Ordering::SeqCst);
    let r = f.send(json!({"cmd": "new_theme", "tab": "t-1"})).map(|_| ()).map_err(|e| e.kind());
    let expected = (Err(SessionErrorKind::PackTabNotFresh), before, tabs);
    assert_that!((r, f.theme_count(), f.window().len())).is_equal_to(expected);
}

#[test]
fn open_theme_for_agent_asks_new_tab_then_opens_it_after_the_requester() {
    let f = themes();
    let mut asked = Vec::new();
    let opened = f.open_by_agent("t-1", PAPIER_THEME, &mut |g| {
        asked.push(g);
        true
    });
    let tab = match opened {
        Ok(TabOpen::Opened { tab, shown: true }) => tab,
        other => panic!("{other:?}"),
    };
    assert_that!((asked, f.window(), f.active())).is_equal_to((
        vec![TabGate::NewTab],
        vec!["t-1".to_owned(), tab.clone()],
        tab,
    ));
}

#[test]
fn open_theme_for_agent_refused_by_new_tab_opens_nothing() {
    let f = themes();
    let opened = f.open_by_agent("t-1", PAPIER_THEME, &mut |_| false);
    assert_that!((opened.ok(), f.window())).is_equal_to((Some(TabOpen::Refused), vec!["t-1".to_owned()]));
}

#[test]
fn open_theme_for_agent_of_an_open_theme_asks_focus_and_focuses_it() {
    let f = themes();
    f.opened("t-1", PAPIER_THEME);
    let tab = f.theme_tabs()[0].clone();
    f.activate("t-1");
    let mut asked = Vec::new();
    let opened = f.open_by_agent("t-1", PAPIER_THEME, &mut |g| {
        asked.push(g);
        true
    });
    assert_that!((asked, opened.ok(), f.active())).is_equal_to((
        vec![TabGate::Focus],
        Some(TabOpen::Focused { tab: tab.clone(), shown: true }),
        tab,
    ));
}

#[test]
fn open_theme_for_agent_refused_by_focus_activates_nothing() {
    let f = themes();
    f.opened("t-1", PAPIER_THEME);
    f.activate("t-1");
    let opened = f.open_by_agent("t-1", PAPIER_THEME, &mut |_| false);
    assert_that!((opened.ok(), f.active())).is_equal_to((Some(TabOpen::Refused), "t-1".to_owned()));
}

#[test]
fn open_theme_for_agent_from_a_help_tab_opens_a_theme_tab() {
    let f = themes();
    f.send(json!({"cmd": "open_help", "window": "w-1"})).unwrap();
    let help = f.active();
    let tab = match f.open_by_agent(&help, PAPIER_THEME, &mut |_| true) {
        Ok(TabOpen::Opened { tab, shown: true }) => tab,
        other => panic!("{other:?}"),
    };
    let found = (f.document(&tab), f.opener_of(&tab));
    assert_that!(found).is_equal_to((Some(TabDocument::Theme { theme: PAPIER_THEME }), Some(help)));
}

/// Decision 20: a requester that is not its window's active tab opens behind.
#[test]
fn open_theme_for_agent_from_a_background_tab_opens_it_without_activating_it() {
    let f = themes();
    let other = f.new_tab();
    f.activate(&other);
    let opened = f.open_by_agent("t-1", PAPIER_THEME, &mut |_| true);
    assert_that!((matches!(opened, Ok(TabOpen::Opened { shown: false, .. })), f.active())).is_equal_to((true, other));
}

#[test]
fn open_theme_for_agent_from_a_background_tab_focuses_nothing() {
    let f = themes();
    f.opened("t-1", PAPIER_THEME);
    let other = f.new_tab();
    f.activate(&other);
    let opened = f.open_by_agent("t-1", PAPIER_THEME, &mut |_| true);
    assert_that!((matches!(opened, Ok(TabOpen::Focused { shown: false, .. })), f.active())).is_equal_to((true, other));
}

/// A Papier theme tab whose cached document lags (forged to a deck): the holder, `t-1` left active.
///
/// The candidates miss it while its session still holds the lease. The blank tab opened first
/// gives the window of `open_theme_twice_focuses_existing_tab`, `[t-1, theme tab, t-2]`, so the
/// theme tab sits between two other tabs, as when a second tab opens it.
fn lagging_theme_tab(f: &Themes) -> String {
    // A second tab, at the end: the theme tab then opens between `t-1` and it.
    f.new_tab();
    f.activate("t-1");
    f.opened("t-1", PAPIER_THEME);
    let holder = f.active();
    Workspace::lock(&f.ws).tabs.get_mut(&holder).unwrap().info.document =
        TabDocument::Deck { path: None, deck_uid: None };
    f.activate("t-1");
    holder
}

#[test]
fn open_theme_for_agent_with_a_refused_lease_asks_focus_first() {
    let f = themes();
    lagging_theme_tab(&f);
    let mut asked = Vec::new();
    let opened = f.open_by_agent("t-1", PAPIER_THEME, &mut |g| {
        asked.push(g);
        g == TabGate::NewTab
    });
    assert_that!((asked, opened.ok(), f.active())).is_equal_to((
        vec![TabGate::NewTab, TabGate::Focus],
        Some(TabOpen::Refused),
        "t-1".to_owned(),
    ));
}

#[test]
fn open_theme_for_agent_with_a_refused_lease_reports_focused_not_opened() {
    let f = themes();
    let holder = lagging_theme_tab(&f);
    let opened = f.open_by_agent("t-1", PAPIER_THEME, &mut |_| true);
    assert_that!((opened.ok(), f.active()))
        .is_equal_to((Some(TabOpen::Focused { tab: holder.clone(), shown: true }), holder));
}

#[test]
fn open_theme_ui_asks_no_gate_and_still_activates() {
    let f = themes();
    let other = f.new_tab();
    f.activate(&other);
    f.opened("t-1", PAPIER_THEME);
    let tab = f.theme_tabs()[0].clone();
    assert_that!((f.active(), f.window())).is_equal_to((tab.clone(), vec!["t-1".to_owned(), tab, other]));
}

/// As for packs, an agent's focus never makes the requester the theme tab's opener.
#[test]
fn open_theme_for_agent_focus_never_retargets_the_opener() {
    let f = themes();
    let other = f.new_tab();
    f.activate("t-1");
    f.opened("t-1", PAPIER_THEME);
    let tab = f.active();
    f.activate(&other);
    f.open_by_agent(&other, PAPIER_THEME, &mut |_| true).unwrap_or_else(|e| panic!("{}", e.one_line()));
    assert_that!((f.active(), f.opener_of(&tab))).is_equal_to((tab.clone(), Some("t-1".to_owned())));
}

/// Decision 20: the requester leaves the front after the ask, so the new tab opens behind.
#[test]
fn open_theme_for_agent_requester_hidden_after_the_ask_opens_it_behind() {
    let f = themes();
    let other = f.new_tab();
    f.activate("t-1");
    let opened = f.open_by_agent("t-1", PAPIER_THEME, &mut |_| {
        f.activate(&other);
        true
    });
    assert_that!((matches!(opened, Ok(TabOpen::Opened { shown: false, .. })), f.active())).is_equal_to((true, other));
}

/// Decision 20: the requester leaves the front after the ask, so the theme tab stays where it is.
#[test]
fn open_theme_for_agent_requester_hidden_after_the_ask_focuses_it_behind() {
    let f = themes();
    f.opened("t-1", PAPIER_THEME);
    let other = f.new_tab();
    f.activate("t-1");
    let opened = f.open_by_agent("t-1", PAPIER_THEME, &mut |_| {
        f.activate(&other);
        true
    });
    assert_that!((matches!(opened, Ok(TabOpen::Focused { shown: false, .. })), f.active())).is_equal_to((true, other));
}

/// Once inserted the tab exists: a failed view still reports it opened, as for packs.
#[test]
fn open_theme_for_agent_whose_view_fails_after_insert_reports_opened() {
    let f = themes();
    super::super::tabs::FAIL_FINISH_VIEW.store(true, std::sync::atomic::Ordering::SeqCst);
    let opened = match f.open_by_agent("t-1", PAPIER_THEME, &mut |_| true) {
        Ok(TabOpen::Opened { tab, shown }) => Some((tab, shown)),
        _ => None,
    };
    let tabs = f.theme_tabs();
    assert_that!(opened).is_equal_to(tabs.first().map(|t| (t.clone(), true)));
}
