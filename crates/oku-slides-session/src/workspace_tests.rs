use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::{Request, SessionErrorKind, Settings};

/// Un dossier de données neuf, supprimé à la fin du test.
struct Data(PathBuf);

impl Data {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        Self(std::env::temp_dir().join(format!(
            "oku-slides-workspace-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }
}

impl Drop for Data {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn platform(data: &Path) -> Platform {
    let mut settings = Settings::defaults().unwrap();
    settings.paths.data = Some(data.to_owned());
    settings.i18n.locale = Some("fr".into());
    Platform::with_settings(settings).unwrap()
}

fn workspace_in(data: &Data) -> Mutex<Workspace> {
    Mutex::new(Workspace::new(Fonts::default(), &platform(&data.0), None).unwrap())
}

fn send(ws: &Mutex<Workspace>, v: serde_json::Value) -> Result<WorkspaceOutcome> {
    Workspace::apply(ws, serde_json::from_value(v).unwrap())
}

fn view(ws: &Mutex<Workspace>) -> WorkspaceView {
    Workspace::lock(ws).view()
}

fn ids(w: &WindowView) -> Vec<&str> {
    w.tabs.iter().map(|t| t.id.as_str()).collect()
}

fn set_theme() -> Request {
    serde_json::from_value(json!({"cmd": "apply", "label": "Thème", "ops": [{"op": "set_theme", "theme": "pro"}]}))
        .unwrap()
}

/// Un vrai `.oku-slides.pdf` : un deck JSON n'a pas de fichier associé.
fn saved_pdf(data: &Data, name: &str) -> PathBuf {
    let fixture = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"));
    let mut s = Session::new(Fonts::default(), &platform(&data.0)).unwrap();
    s.open(&fixture).unwrap();
    let path = data.0.join(format!("{name}.oku-slides.pdf"));
    s.save(Some(&path)).unwrap();
    path
}

#[test]
fn new_workspace_has_one_window_with_one_pristine_tab() {
    let data = Data::new();
    let v = view(&workspace_in(&data));
    assert_that!(v.windows).has_length(1);
    let w = &v.windows[0];
    assert_that!(w.label.as_str()).is_equal_to("w-1");
    assert_that!(ids(w)).is_equal_to(vec!["t-1"]);
    assert_that!(w.active.as_str()).is_equal_to("t-1");
    assert_that!(w.tabs[0].info.pristine).is_true();
    assert_that!(v.focused.as_str()).is_equal_to("w-1");
}

#[test]
fn new_workspace_with_file_opens_it_in_the_first_tab() {
    let data = Data::new();
    let file = saved_pdf(&data, "essai");
    let ws = Mutex::new(Workspace::new(Fonts::default(), &platform(&data.0), Some(&file)).unwrap());
    let w = &view(&ws).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-1"]);
    assert_that!(w.tabs[0].info.path).is_equal_to(Some(file));
    assert_that!(w.tabs[0].info.pristine).is_false();
}

/// Un deck vierge enregistré a un fichier : il n'est plus remplacé.
#[test]
fn open_tab_after_saving_the_blank_deck_adds_a_tab() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let saved = data.0.join("vierge.oku-slides.pdf");
    let save: Request = serde_json::from_value(json!({"cmd": "save", "path": saved})).unwrap();
    Workspace::request(&ws, "t-1", save).0.unwrap();
    let file = saved_pdf(&data, "essai");
    send(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": file})).unwrap();
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1", "t-2"]);
}

/// Un verrou empoisonné (panique d'un autre fil) est repris : l'app continue.
#[test]
fn workspace_lock_poisoned_by_a_panic_still_serves_requests() {
    let data = Data::new();
    let ws = std::sync::Arc::new(workspace_in(&data));
    let poisoner = std::sync::Arc::clone(&ws);
    let _ = std::thread::spawn(move || {
        let _guard = poisoner.lock();
        panic!("panique volontaire");
    })
    .join();
    assert_that!(ws.is_poisoned()).is_true();
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    assert_that!(view(&ws).windows[0].tabs).has_length(2);
}

#[test]
fn new_tab_appends_and_activates_it() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    let w = &view(&ws).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-1", "t-2"]);
    assert_that!(w.active.as_str()).is_equal_to("t-2");
}

#[test]
fn open_tab_with_pristine_active_tab_replaces_it_with_a_new_tab() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let file = saved_pdf(&data, "essai");
    send(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": file})).unwrap();
    let w = &view(&ws).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-2"]);
    assert_that!(w.active.as_str()).is_equal_to("t-2");
    assert_that!(w.tabs[0].info.path).is_equal_to(Some(file));
}

#[test]
fn open_tab_with_edited_active_tab_adds_a_tab_holding_the_file() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let file = saved_pdf(&data, "essai");
    Workspace::request(&ws, "t-1", set_theme()).0.unwrap();
    send(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": file})).unwrap();
    let w = &view(&ws).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-1", "t-2"]);
    assert_that!(w.active.as_str()).is_equal_to("t-2");
    assert_that!(w.tabs[0].info.dirty).is_true();
    assert_that!(w.tabs[0].info.path).is_none();
    assert_that!(w.tabs[1].info.path).is_equal_to(Some(file));
}

/// La session vierge est occupée (`try_lock` échoue) : elle n'est pas
/// remplacée, l'onglet du fichier s'ajoute à côté.
#[test]
fn open_tab_while_the_pristine_tab_is_busy_adds_a_tab() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let file = saved_pdf(&data, "essai");
    let session = Workspace::lock(&ws).session("t-1").unwrap();
    let guard = session.lock().unwrap();
    send(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": file})).unwrap();
    drop(guard);
    let w = &view(&ws).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-1", "t-2"]);
    assert_that!(w.tabs[1].info.path).is_equal_to(Some(file));
}

#[test]
fn open_tab_already_open_in_other_window_activates_it_and_focuses_its_window() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let file = saved_pdf(&data, "essai");
    send(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": file})).unwrap();
    send(&ws, json!({"cmd": "new_window"})).unwrap();
    assert_that!(Workspace::lock(&ws).focused()).is_equal_to("w-2");
    send(&ws, json!({"cmd": "open_tab", "window": "w-2", "path": file})).unwrap();
    let v = view(&ws);
    assert_that!(v.focused.as_str()).is_equal_to("w-1");
    assert_that!(v.windows[1].tabs).has_length(1);
    assert_that!(v.windows[1].tabs[0].info.pristine).is_true();
}

#[test]
fn open_tab_same_file_spelled_differently_activates_the_existing_tab() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let file = saved_pdf(&data, "essai");
    send(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": file})).unwrap();
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    let other = data.0.join("x/../essai.oku-slides.pdf");
    send(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": other})).unwrap();
    let w = &view(&ws).windows[0];
    assert_that!(w.tabs).has_length(2);
    assert_that!(w.active.as_str()).is_equal_to("t-2");
}

#[test]
fn open_tab_unreadable_file_returns_error_and_keeps_the_tabs() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let missing = data.0.join("absent.oku-slides.pdf");
    assert_that!(send(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": missing}))).is_err();
    let w = &view(&ws).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-1"]);
    assert_that!(w.tabs[0].info.pristine).is_true();
}

#[test]
fn close_active_tab_activates_its_right_neighbour() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    send(&ws, json!({"cmd": "activate", "tab": "t-2"})).unwrap();
    send(&ws, json!({"cmd": "close", "tab": "t-2"})).unwrap();
    let w = &view(&ws).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-1", "t-3"]);
    assert_that!(w.active.as_str()).is_equal_to("t-3");
}

#[test]
fn close_rightmost_active_tab_activates_its_left_neighbour() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    send(&ws, json!({"cmd": "close", "tab": "t-2"})).unwrap();
    assert_that!(view(&ws).windows[0].active.as_str()).is_equal_to("t-1");
}

#[test]
fn close_edited_tab_without_discard_returns_unsaved_and_keeps_it() {
    let data = Data::new();
    let ws = workspace_in(&data);
    Workspace::request(&ws, "t-1", set_theme()).0.unwrap();
    let err = send(&ws, json!({"cmd": "close", "tab": "t-1"})).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Unsaved);
    let w = &view(&ws).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-1"]);
    assert_that!(w.tabs[0].info.dirty).is_true();
}

/// Seul l'ensemble de modèles en cours est modifié : c'est du travail aussi.
#[test]
fn close_tab_with_only_edited_templates_returns_unsaved() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let edit: Request = serde_json::from_value(json!({"cmd": "edit_templates", "set": crate::DEFAULT_SET})).unwrap();
    Workspace::request(&ws, "t-1", edit).0.unwrap();
    let first = Workspace::lock(&ws).session("t-1").unwrap().lock().unwrap().deck().slides[0].id.clone();
    let remove: Request = serde_json::from_value(
        json!({"cmd": "apply", "label": "Retirer", "ops": [{"op": "remove_slide", "slide": first}]}),
    )
    .unwrap();
    Workspace::request(&ws, "t-1", remove).0.unwrap();
    let err = send(&ws, json!({"cmd": "close", "tab": "t-1"})).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Unsaved);
}

#[test]
fn close_edited_tab_with_discard_closes_it() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    Workspace::request(&ws, "t-2", set_theme()).0.unwrap();
    send(&ws, json!({"cmd": "close", "tab": "t-2", "discard": true})).unwrap();
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1"]);
}

/// La session est verrouillée (un rendu en cours) : la fermeture n'attend
/// jamais, elle refuse comme si le travail n'était pas enregistré.
#[test]
fn close_tab_while_its_session_is_busy_returns_unsaved() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    let session = Workspace::lock(&ws).session("t-2").unwrap();
    let guard = session.lock().unwrap();
    let err = send(&ws, json!({"cmd": "close", "tab": "t-2"})).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Unsaved);
    drop(guard);
    send(&ws, json!({"cmd": "close", "tab": "t-2", "discard": true})).unwrap();
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1"]);
}

/// Une session qui a paniqué (verrou empoisonné) ne se ferme jamais en
/// silence : il faut l'abandonner explicitement.
#[test]
fn close_tab_with_a_poisoned_session_needs_discard() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    let session = Workspace::lock(&ws).session("t-2").unwrap();
    let poisoner = std::sync::Arc::clone(&session);
    let _ = std::thread::spawn(move || {
        let _guard = poisoner.lock();
        panic!("panique volontaire");
    })
    .join();
    assert_that!(session.is_poisoned()).is_true();
    let err = send(&ws, json!({"cmd": "close", "tab": "t-2"})).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Unsaved);
    send(&ws, json!({"cmd": "close", "tab": "t-2", "discard": true})).unwrap();
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1"]);
}

#[test]
fn close_only_tab_of_a_window_removes_the_window() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_window"})).unwrap();
    send(&ws, json!({"cmd": "close", "tab": "t-2"})).unwrap();
    let v = view(&ws);
    assert_that!(v.windows).has_length(1);
    assert_that!(v.focused.as_str()).is_equal_to("w-1");
}

#[test]
fn close_window_with_an_edited_tab_not_discarded_returns_unsaved_and_closes_nothing() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_window"})).unwrap();
    // Un onglet propre d'abord, le modifié ensuite : rien ne doit être fermé.
    send(&ws, json!({"cmd": "new_tab", "window": "w-2"})).unwrap();
    Workspace::request(&ws, "t-3", set_theme()).0.unwrap();
    let err = send(&ws, json!({"cmd": "close_window", "window": "w-2"})).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Unsaved);
    assert_that!(ids(&view(&ws).windows[1])).is_equal_to(vec!["t-2", "t-3"]);
    send(&ws, json!({"cmd": "close_window", "window": "w-2", "discard": ["t-3"]})).unwrap();
    assert_that!(view(&ws).windows).has_length(1);
    let gone = Workspace::request(&ws, "t-2", Request::Snapshot).0.unwrap_err();
    assert_that!(gone.kind()).is_equal_to(SessionErrorKind::UnknownTab);
}

/// La seule fenêtre : jamais fermée, même avec `discard` ; l'appelant le
/// sait (`WorkspaceOutcome::LastWindow`) sans erreur nouvelle.
#[test]
fn close_window_on_the_last_window_is_kept_and_reported() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let outcome = send(&ws, json!({"cmd": "close_window", "window": "w-1", "discard": ["t-1"]})).unwrap();
    assert_that!(outcome.is_last_window()).is_true();
    assert_that!(view(&ws).windows).has_length(1);
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1"]);
}

/// Une autre fenêtre encore ouverte : celle-ci se ferme normalement.
#[test]
fn close_window_with_another_window_left_is_removed() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_window"})).unwrap();
    let outcome = send(&ws, json!({"cmd": "close_window", "window": "w-2"})).unwrap();
    assert_that!(outcome.is_last_window()).is_false();
    let v = view(&ws);
    assert_that!(v.windows).has_length(1);
    assert_that!(v.windows[0].label.as_str()).is_equal_to("w-1");
}

#[test]
fn reorder_moves_the_tab_to_its_new_index_clamped() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    send(&ws, json!({"cmd": "reorder", "tab": "t-3", "index": 0})).unwrap();
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-3", "t-1", "t-2"]);
    send(&ws, json!({"cmd": "reorder", "tab": "t-3", "index": 99})).unwrap();
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1", "t-2", "t-3"]);
}

#[test]
fn move_tab_to_other_window_at_index_activates_it_there() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    send(&ws, json!({"cmd": "new_window"})).unwrap();
    send(&ws, json!({"cmd": "move_tab", "tab": "t-1", "window": "w-2", "index": 0})).unwrap();
    let v = view(&ws);
    assert_that!(ids(&v.windows[0])).is_equal_to(vec!["t-2"]);
    assert_that!(ids(&v.windows[1])).is_equal_to(vec!["t-1", "t-3"]);
    assert_that!(v.windows[1].active.as_str()).is_equal_to("t-1");
    assert_that!(v.focused.as_str()).is_equal_to("w-2");
}

#[test]
fn move_tab_to_its_own_window_without_index_changes_nothing() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    send(&ws, json!({"cmd": "move_tab", "tab": "t-1", "window": "w-1"})).unwrap();
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1", "t-2"]);
    send(&ws, json!({"cmd": "move_tab", "tab": "t-1", "window": "w-1", "index": 1})).unwrap();
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-2", "t-1"]);
}

#[test]
fn move_tab_to_new_window_creates_a_window() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    send(&ws, json!({"cmd": "move_tab", "tab": "t-1", "window": null})).unwrap();
    let v = view(&ws);
    assert_that!(v.windows).has_length(2);
    assert_that!(ids(&v.windows[0])).is_equal_to(vec!["t-2"]);
    assert_that!(v.windows[1].label.as_str()).is_equal_to("w-2");
    assert_that!(ids(&v.windows[1])).is_equal_to(vec!["t-1"]);
}

#[test]
fn move_only_tab_to_new_window_removes_its_old_window() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "move_tab", "tab": "t-1", "window": null})).unwrap();
    let v = view(&ws);
    assert_that!(v.windows).has_length(1);
    assert_that!(v.windows[0].label.as_str()).is_equal_to("w-2");
}

#[test]
fn move_tab_to_unknown_window_returns_error_and_keeps_the_tab() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let err = send(&ws, json!({"cmd": "move_tab", "tab": "t-1", "window": "w-9"})).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownWindow);
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1"]);
}

#[test]
fn unknown_tab_returns_unknown_tab_error() {
    let data = Data::new();
    let err = send(&workspace_in(&data), json!({"cmd": "activate", "tab": "t-9"})).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownTab);
}

#[test]
fn unknown_window_returns_unknown_window_error() {
    let data = Data::new();
    let err = send(&workspace_in(&data), json!({"cmd": "new_tab", "window": "w-9"})).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownWindow);
}

#[test]
fn focus_unknown_window_returns_unknown_window_error() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let err = Workspace::lock(&ws).focus("w-9").unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownWindow);
}

fn rect(x: f64, y: f64) -> Rect {
    Rect { x, y, w: 100.0, h: 100.0 }
}

#[test]
fn window_at_inside_the_only_window_returns_it_edges_included() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let rects = vec![("w-1".to_owned(), rect(0.0, 0.0))];
    let w = Workspace::lock(&ws);
    assert_that!(w.window_at(0.0, 0.0, &rects)).is_equal_to(Some("w-1".to_owned()));
    assert_that!(w.window_at(100.0, 50.0, &rects)).is_none();
}

#[test]
fn window_at_overlap_prefers_the_most_recently_focused_window() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_window"})).unwrap();
    let rects = vec![("w-1".to_owned(), rect(0.0, 0.0)), ("w-2".to_owned(), rect(50.0, 0.0))];
    let mut w = Workspace::lock(&ws);
    w.focus("w-1").unwrap();
    assert_that!(w.window_at(75.0, 50.0, &rects)).is_equal_to(Some("w-1".to_owned()));
    w.focus("w-2").unwrap();
    assert_that!(w.window_at(75.0, 50.0, &rects)).is_equal_to(Some("w-2".to_owned()));
    // Seule w-1 contient ce point, même si w-2 est la plus récente.
    assert_that!(w.window_at(25.0, 50.0, &rects)).is_equal_to(Some("w-1".to_owned()));
}

/// Deux écrans, en points (macOS) : le portable (1512 × 982, origine du
/// repère), et un écran externe 2560 × 1440 à sa droite, haut aligné 200
/// points plus haut (y négatifs). Une fenêtre 1440 × 900 sur chacun.
fn two_screens() -> Vec<(String, Rect)> {
    vec![
        ("w-1".to_owned(), Rect { x: 36.0, y: 38.0, w: 1440.0, h: 900.0 }),
        ("w-2".to_owned(), Rect { x: 1800.0, y: -120.0, w: 1440.0, h: 900.0 }),
    ]
}

#[test]
fn window_at_two_screens_finds_the_window_on_the_other_screen() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_window"})).unwrap();
    let mut w = Workspace::lock(&ws);
    // L'onglet part de w-1, la plus récente.
    w.focus("w-1").unwrap();
    assert_that!(w.window_at(2000.0, 700.0, &two_screens())).is_equal_to(Some("w-2".to_owned()));
}

#[test]
fn window_at_two_screens_accepts_a_point_above_the_primary_screen() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_window"})).unwrap();
    let w = Workspace::lock(&ws);
    assert_that!(w.window_at(1900.0, -100.0, &two_screens())).is_equal_to(Some("w-2".to_owned()));
}

#[test]
fn window_at_two_screens_between_the_windows_returns_none() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_window"})).unwrap();
    let w = Workspace::lock(&ws);
    // Sur l'écran externe, à gauche de w-2 : une nouvelle fenêtre.
    assert_that!(w.window_at(1600.0, 300.0, &two_screens())).is_none();
}

#[test]
fn tab_window_returns_the_label_of_the_window_holding_the_tab() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "new_window"})).unwrap();
    let v = view(&ws);
    let tab = v.windows[1].tabs[0].id.clone();
    assert_that!(Workspace::lock(&ws).tab_window(&tab)).is_equal_to(Some(v.windows[1].label.as_str()));
}

#[test]
fn tab_window_unknown_tab_returns_none() {
    let data = Data::new();
    let ws = workspace_in(&data);
    assert_that!(Workspace::lock(&ws).tab_window("t-99")).is_none();
}

#[test]
fn window_at_ignores_rects_of_unknown_windows() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let rects = vec![("autre".to_owned(), rect(0.0, 0.0))];
    assert_that!(Workspace::lock(&ws).window_at(50.0, 50.0, &rects)).is_none();
}

#[test]
fn request_reports_a_change_of_tab_info_only_once() {
    let data = Data::new();
    let ws = workspace_in(&data);
    let (reply, changed) = Workspace::request(&ws, "t-1", Request::Snapshot);
    assert_that!(reply).is_ok();
    assert_that!(changed).is_false();
    assert_that!(Workspace::request(&ws, "t-1", set_theme()).1).is_true();
    assert_that!(Workspace::request(&ws, "t-1", Request::Snapshot).1).is_false();
    assert_that!(Workspace::request(&ws, "t-9", Request::Snapshot).1).is_false();
}

/// L'ordre des verrous : une requête d'onglet et la fermeture du même
/// onglet, en parallèle, ne s'interbloquent pas (un interblocage échoue au
/// bout de 10 s au lieu de bloquer la suite de tests).
#[test]
fn request_and_close_of_the_same_tab_in_parallel_do_not_deadlock() {
    let data = Data::new();
    let ws = std::sync::Arc::new(workspace_in(&data));
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    let (done, finished) = std::sync::mpsc::channel();
    {
        let (ws, done) = (std::sync::Arc::clone(&ws), done.clone());
        std::thread::spawn(move || {
            for _ in 0..20 {
                let _ = Workspace::request(&ws, "t-2", Request::Snapshot);
            }
            done.send(()).unwrap();
        });
    }
    {
        let ws = std::sync::Arc::clone(&ws);
        std::thread::spawn(move || {
            send(&ws, json!({"cmd": "close", "tab": "t-2", "discard": true})).unwrap();
            done.send(()).unwrap();
        });
    }
    for _ in 0..2 {
        finished.recv_timeout(std::time::Duration::from_secs(10)).expect("interblocage");
    }
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1"]);
}

#[test]
fn close_last_tab_of_last_window_leaves_no_window() {
    let data = Data::new();
    let ws = workspace_in(&data);
    send(&ws, json!({"cmd": "close", "tab": "t-1"})).unwrap();
    let v = view(&ws);
    assert_that!(v.windows).is_empty();
    assert_that!(v.focused.as_str()).is_equal_to("");
}

/// Une construction qui chevauche `settle` : l'onglet n'est pas inséré,
/// sa session part à la corbeille (son dossier de reprise avec elle).
#[test]
fn finish_after_settle_refuses_and_trashes_the_built_tab() {
    let data = Data::new();
    let dir = data.0.join("resume");
    let ws = Mutex::new(Workspace::resume(Fonts::default(), &platform(&data.0), dir.clone()).unwrap());
    let pending = Workspace::lock(&ws).pending(Some("w-1".into()), None, None);
    let session = pending.build().unwrap();
    let built = dir.join(TABS_DIR).join(&pending.id);
    assert_that!(built.exists()).is_true();
    Workspace::settle(&ws);
    let refused = Workspace::finish(&ws, pending, session);
    assert_that!(refused.map(|_| ())).is_err();
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1"]);
    assert_that!(built.exists()).is_false();
}

/// Un reste illisible au numéro d'un onglet neuf ne lui coûte pas le fichier
/// qu'il vient d'ouvrir : le reste est mis de côté, l'onglet garde le fichier.
#[test]
fn open_tab_over_a_leftover_resume_dir_keeps_the_opened_file() {
    let data = Data::new();
    let dir = data.0.join("resume");
    let file = saved_pdf(&data, "gardé");
    let ws = Mutex::new(Workspace::resume(Fonts::default(), &platform(&data.0), dir.clone()).unwrap());
    let leftover = dir.join(TABS_DIR).join("t-2");
    std::fs::create_dir_all(&leftover).unwrap();
    std::fs::write(leftover.join("state.json"), "pas du json").unwrap();
    send(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": file})).unwrap();
    let w = &view(&ws).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-2"]);
    assert_that!(w.tabs[0].info.path).is_equal_to(Some(file.clone()));
    // Le reste n'est ni relu ni écrasé : mis de côté.
    let names: Vec<String> = std::fs::read_dir(dir.join(TABS_DIR))
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
        .collect();
    assert_that!(names.iter().any(|n| n.starts_with("t-2.failed-"))).is_true();
    // Le reste mis de côté, la reprise écrit l'onglet : il survit au relancement.
    drop(ws);
    let again = Mutex::new(Workspace::resume(Fonts::default(), &platform(&data.0), dir).unwrap());
    let w = &view(&again).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-2"]);
    assert_that!(w.tabs[0].info.path).is_equal_to(Some(file));
}

#[test]
fn messages_give_the_ui_its_language_and_fluent_resources() {
    let data = Data::new();
    let json = send(&workspace_in(&data), json!({"cmd": "messages"})).unwrap().into_json();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_that!(v["locale"].as_str()).is_equal_to(Some("fr"));
    let resources: Vec<String> = serde_json::from_value(v["resources"].clone()).unwrap();
    let expected: Vec<String> = crate::Localizer::new(Some("fr")).ui_messages().into_iter().map(String::from).collect();
    assert_that!(resources).is_equal_to(expected);
}

#[test]
fn is_tab_id_accepts_only_t_and_digits() {
    assert_that!(is_tab_id("t-12")).is_true();
    assert_that!(is_tab_id("t-999999999999999999")).is_true();
    for bad in ["t-", "t-1a", "../t-1", "/tmp", "t-1234567890123456789", "t-12345678901234567890", "w-1"] {
        asserting(bad).that(&is_tab_id(bad)).is_false();
    }
    assert_that!(is_window_label("w-3")).is_true();
    assert_that!(is_window_label("main")).is_false();
}
