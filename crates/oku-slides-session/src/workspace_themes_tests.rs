use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

use super::super::opening::{opening_waiters, with_opening};
use super::super::packs::workspace_packs_fixtures::within;
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
