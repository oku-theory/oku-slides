use std::sync::Mutex;

use serde_json::json;
use speculoos::prelude::*;

use super::workspace_fixtures::{Data, ids, saved_pdf, set_theme, view};
use super::*;
use crate::test_support::{apply_ws, workspace_on, workspace_platform};
use crate::{Request, SessionErrorKind};

#[test]
fn new_workspace_has_one_window_with_one_pristine_tab() {
    let data = Data::new();
    let v = view(&workspace_on(&data.0));
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
    let ws = Mutex::new(Workspace::new(Fonts::default(), &workspace_platform(&data.0), Some(&file)).unwrap());
    let w = &view(&ws).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-1"]);
    assert_that!(w.tabs[0].info.document.path()).is_equal_to(Some(file.as_path()));
    assert_that!(w.tabs[0].info.pristine).is_false();
}

/// Un deck vierge enregistré a un fichier : il n'est plus remplacé.
#[test]
fn open_tab_after_saving_the_blank_deck_adds_a_tab() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    let saved = data.0.join("vierge.oku-slides.pdf");
    let save: Request = serde_json::from_value(json!({"cmd": "save", "path": saved})).unwrap();
    Workspace::request(&ws, "t-1", save).0.unwrap();
    let file = saved_pdf(&data, "essai");
    apply_ws(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": file})).unwrap();
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1", "t-2"]);
}

/// Jamais enregistré à la main : le deck d'un onglet va de lui-même au dossier des decks.
#[test]
fn save_without_path_in_a_tab_writes_into_the_decks_dir() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    Workspace::request(&ws, "t-1", set_theme()).0.unwrap();
    let save: Request = serde_json::from_value(json!({"cmd": "save", "path": null})).unwrap();
    Workspace::request(&ws, "t-1", save).0.unwrap();
    let path = view(&ws).windows[0].tabs[0].info.document.path().unwrap().to_owned();
    assert_that!(path.starts_with(data.0.join("decks"))).is_true();
    assert_that!(path.exists()).is_true();
}

#[test]
fn move_request_moves_the_tab_file() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    let first = data.0.join("avant.oku-slides.pdf");
    let save: Request = serde_json::from_value(json!({"cmd": "save", "path": first})).unwrap();
    Workspace::request(&ws, "t-1", save).0.unwrap();
    let moved = data.0.join("apres.oku-slides.pdf");
    let mv: Request = serde_json::from_value(json!({"cmd": "move", "path": moved})).unwrap();
    Workspace::request(&ws, "t-1", mv).0.unwrap();
    assert_that!(first.exists()).is_false();
    assert_that!(view(&ws).windows[0].tabs[0].info.document.path()).is_equal_to(Some(moved.as_path()));
}

#[test]
fn save_copy_request_keeps_the_tab_file() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    let file = data.0.join("deck.oku-slides.pdf");
    let save: Request = serde_json::from_value(json!({"cmd": "save", "path": file})).unwrap();
    Workspace::request(&ws, "t-1", save).0.unwrap();
    let copy = data.0.join("copie.oku-slides.pdf");
    let req: Request = serde_json::from_value(json!({"cmd": "save_copy", "path": copy})).unwrap();
    Workspace::request(&ws, "t-1", req).0.unwrap();
    assert_that!(copy.exists()).is_true();
    assert_that!(view(&ws).windows[0].tabs[0].info.document.path()).is_equal_to(Some(file.as_path()));
}

/// Un verrou empoisonné (panique d'un autre fil) est repris : l'app continue.
#[test]
fn workspace_lock_poisoned_by_a_panic_still_serves_requests() {
    let data = Data::new();
    let ws = std::sync::Arc::new(workspace_on(&data.0));
    let poisoner = std::sync::Arc::clone(&ws);
    let _ = std::thread::spawn(move || {
        let _guard = poisoner.lock();
        panic!("panique volontaire");
    })
    .join();
    assert_that!(ws.is_poisoned()).is_true();
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    assert_that!(view(&ws).windows[0].tabs).has_length(2);
}

#[test]
fn new_tab_appends_and_activates_it() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    let w = &view(&ws).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-1", "t-2"]);
    assert_that!(w.active.as_str()).is_equal_to("t-2");
}

#[test]
fn open_tab_with_pristine_active_tab_replaces_it_with_a_new_tab() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    let file = saved_pdf(&data, "essai");
    apply_ws(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": file})).unwrap();
    let w = &view(&ws).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-2"]);
    assert_that!(w.active.as_str()).is_equal_to("t-2");
    assert_that!(w.tabs[0].info.document.path()).is_equal_to(Some(file.as_path()));
}

#[test]
fn open_tab_with_edited_active_tab_adds_a_tab_holding_the_file() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    let file = saved_pdf(&data, "essai");
    Workspace::request(&ws, "t-1", set_theme()).0.unwrap();
    apply_ws(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": file})).unwrap();
    let w = &view(&ws).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-1", "t-2"]);
    assert_that!(w.active.as_str()).is_equal_to("t-2");
    assert_that!(w.tabs[0].info.dirty).is_true();
    assert_that!(w.tabs[0].info.document.path()).is_none();
    assert_that!(w.tabs[1].info.document.path()).is_equal_to(Some(file.as_path()));
}

/// La session vierge est occupée (`try_lock` échoue) : elle n'est pas
/// remplacée, l'onglet du fichier s'ajoute à côté.
#[test]
fn open_tab_while_the_pristine_tab_is_busy_adds_a_tab() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    let file = saved_pdf(&data, "essai");
    let session = Workspace::lock(&ws).session("t-1").unwrap();
    let guard = session.lock().unwrap();
    apply_ws(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": file})).unwrap();
    drop(guard);
    let w = &view(&ws).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-1", "t-2"]);
    assert_that!(w.tabs[1].info.document.path()).is_equal_to(Some(file.as_path()));
}

#[test]
fn open_tab_already_open_in_other_window_activates_it_and_focuses_its_window() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    let file = saved_pdf(&data, "essai");
    apply_ws(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": file})).unwrap();
    apply_ws(&ws, json!({"cmd": "new_window"})).unwrap();
    assert_that!(Workspace::lock(&ws).focused()).is_equal_to("w-2");
    apply_ws(&ws, json!({"cmd": "open_tab", "window": "w-2", "path": file})).unwrap();
    let v = view(&ws);
    assert_that!(v.focused.as_str()).is_equal_to("w-1");
    assert_that!(v.windows[1].tabs).has_length(1);
    assert_that!(v.windows[1].tabs[0].info.pristine).is_true();
}

#[test]
fn open_tab_same_file_spelled_differently_activates_the_existing_tab() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    let file = saved_pdf(&data, "essai");
    apply_ws(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": file})).unwrap();
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    let other = data.0.join("x/../essai.oku-slides.pdf");
    apply_ws(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": other})).unwrap();
    let w = &view(&ws).windows[0];
    assert_that!(w.tabs).has_length(2);
    assert_that!(w.active.as_str()).is_equal_to("t-2");
}

#[test]
fn open_tab_unreadable_file_returns_error_and_keeps_the_tabs() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    let missing = data.0.join("absent.oku-slides.pdf");
    assert_that!(apply_ws(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": missing}))).is_err();
    let w = &view(&ws).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-1"]);
    assert_that!(w.tabs[0].info.pristine).is_true();
}

#[test]
fn close_active_tab_activates_its_right_neighbour() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    apply_ws(&ws, json!({"cmd": "activate", "tab": "t-2"})).unwrap();
    apply_ws(&ws, json!({"cmd": "close", "tab": "t-2"})).unwrap();
    let w = &view(&ws).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-1", "t-3"]);
    assert_that!(w.active.as_str()).is_equal_to("t-3");
}

#[test]
fn close_rightmost_active_tab_activates_its_left_neighbour() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    apply_ws(&ws, json!({"cmd": "close", "tab": "t-2"})).unwrap();
    assert_that!(view(&ws).windows[0].active.as_str()).is_equal_to("t-1");
}

#[test]
fn close_edited_tab_without_discard_returns_unsaved_and_keeps_it() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    Workspace::request(&ws, "t-1", set_theme()).0.unwrap();
    let err = apply_ws(&ws, json!({"cmd": "close", "tab": "t-1"})).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Unsaved);
    let w = &view(&ws).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-1"]);
    assert_that!(w.tabs[0].info.dirty).is_true();
}

#[test]
fn close_edited_tab_with_discard_closes_it() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    Workspace::request(&ws, "t-2", set_theme()).0.unwrap();
    apply_ws(&ws, json!({"cmd": "close", "tab": "t-2", "discard": true})).unwrap();
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1"]);
}

/// La session est verrouillée (un rendu en cours) : la fermeture n'attend
/// jamais, elle refuse comme si le travail n'était pas enregistré.
#[test]
fn close_tab_while_its_session_is_busy_returns_unsaved() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    let session = Workspace::lock(&ws).session("t-2").unwrap();
    let guard = session.lock().unwrap();
    let err = apply_ws(&ws, json!({"cmd": "close", "tab": "t-2"})).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Unsaved);
    drop(guard);
    apply_ws(&ws, json!({"cmd": "close", "tab": "t-2", "discard": true})).unwrap();
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1"]);
}

/// Une session qui a paniqué (verrou empoisonné) ne se ferme jamais en
/// silence : il faut l'abandonner explicitement.
#[test]
fn close_tab_with_a_poisoned_session_needs_discard() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    let session = Workspace::lock(&ws).session("t-2").unwrap();
    let poisoner = std::sync::Arc::clone(&session);
    let _ = std::thread::spawn(move || {
        let _guard = poisoner.lock();
        panic!("panique volontaire");
    })
    .join();
    assert_that!(session.is_poisoned()).is_true();
    let err = apply_ws(&ws, json!({"cmd": "close", "tab": "t-2"})).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Unsaved);
    apply_ws(&ws, json!({"cmd": "close", "tab": "t-2", "discard": true})).unwrap();
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1"]);
}

#[test]
fn close_only_tab_of_a_window_removes_the_window() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    apply_ws(&ws, json!({"cmd": "new_window"})).unwrap();
    apply_ws(&ws, json!({"cmd": "close", "tab": "t-2"})).unwrap();
    let v = view(&ws);
    assert_that!(v.windows).has_length(1);
    assert_that!(v.focused.as_str()).is_equal_to("w-1");
}

#[test]
fn close_window_with_an_edited_tab_not_discarded_returns_unsaved_and_closes_nothing() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    apply_ws(&ws, json!({"cmd": "new_window"})).unwrap();
    // Un onglet propre d'abord, le modifié ensuite : rien ne doit être fermé.
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-2"})).unwrap();
    Workspace::request(&ws, "t-3", set_theme()).0.unwrap();
    let err = apply_ws(&ws, json!({"cmd": "close_window", "window": "w-2"})).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Unsaved);
    assert_that!(ids(&view(&ws).windows[1])).is_equal_to(vec!["t-2", "t-3"]);
    apply_ws(&ws, json!({"cmd": "close_window", "window": "w-2", "discard": ["t-3"]})).unwrap();
    assert_that!(view(&ws).windows).has_length(1);
    let gone = Workspace::request(&ws, "t-2", Request::Snapshot).0.unwrap_err();
    assert_that!(gone.kind()).is_equal_to(SessionErrorKind::UnknownTab);
}

/// La seule fenêtre : jamais fermée, même avec `discard` ; l'appelant le
/// sait (`WorkspaceOutcome::LastWindow`) sans erreur nouvelle.
#[test]
fn close_window_on_the_last_window_is_kept_and_reported() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    let outcome = apply_ws(&ws, json!({"cmd": "close_window", "window": "w-1", "discard": ["t-1"]})).unwrap();
    assert_that!(outcome.is_last_window()).is_true();
    assert_that!(view(&ws).windows).has_length(1);
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1"]);
}

/// Une autre fenêtre encore ouverte : celle-ci se ferme normalement.
#[test]
fn close_window_with_another_window_left_is_removed() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    apply_ws(&ws, json!({"cmd": "new_window"})).unwrap();
    let outcome = apply_ws(&ws, json!({"cmd": "close_window", "window": "w-2"})).unwrap();
    assert_that!(outcome.is_last_window()).is_false();
    let v = view(&ws);
    assert_that!(v.windows).has_length(1);
    assert_that!(v.windows[0].label.as_str()).is_equal_to("w-1");
}

#[test]
fn shown_drafts_names_the_pack_and_theme_tabs_only() {
    let (pack, theme, linked) = (uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
    let info = |document: crate::TabDocument| crate::TabInfo {
        title: String::new(),
        dirty: false,
        pristine: false,
        theme: None,
        document,
        created: None,
    };
    let infos = [
        info(crate::TabDocument::Deck { path: None, deck_uid: None }),
        info(crate::TabDocument::Help { locale: "fr".into() }),
        info(crate::TabDocument::Pack { pack }),
        info(crate::TabDocument::Theme { theme }),
        info(crate::TabDocument::DeckPack { deck_tab: "t-1".into(), pack: linked }),
    ];
    assert_that!(super::tabs::shown_drafts(infos.iter())).is_equal_to(vec![pack, theme]);
}
