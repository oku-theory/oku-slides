use std::sync::Mutex;

use serde_json::json;
use speculoos::prelude::*;

use crate::test_support::{fresh_dir, settings_on};
use crate::{Fonts, Platform, Workspace, WorkspaceOutcome, WorkspaceView};

fn workspace(data: &std::path::Path, locale: &str) -> Mutex<Workspace> {
    let mut settings = settings_on(data);
    settings.i18n.locale = Some(locale.into());
    let platform = Platform::with_settings(settings).unwrap();
    Mutex::new(Workspace::new(Fonts::default(), &platform, None).unwrap())
}

fn send(ws: &Mutex<Workspace>, v: serde_json::Value) -> WorkspaceOutcome {
    Workspace::apply(ws, serde_json::from_value(v).unwrap()).unwrap()
}

fn view(ws: &Mutex<Workspace>) -> WorkspaceView {
    Workspace::lock(ws).view()
}

#[test]
fn open_help_adds_an_active_help_tab_beside_the_pristine_one() {
    let data = fresh_dir("oku-slides-workspace-help-");
    let ws = workspace(data.path(), "fr");
    send(&ws, json!({"cmd": "open_help", "window": "w-1"}));
    let v = view(&ws);
    let w = &v.windows[0];
    assert_that!(w.tabs).has_length(2);
    let help = w.tabs.iter().find(|t| t.info.help.is_some()).unwrap();
    assert_that!(help.info.help.as_deref()).is_equal_to(Some("fr"));
    assert_that!(w.active.as_str()).is_equal_to(help.id.as_str());
}

#[test]
fn open_help_twice_activates_the_help_tab_already_open() {
    let data = fresh_dir("oku-slides-workspace-help-");
    let ws = workspace(data.path(), "fr");
    send(&ws, json!({"cmd": "open_help", "window": "w-1"}));
    let help = view(&ws).windows[0].active.clone();
    send(&ws, json!({"cmd": "activate", "tab": "t-1"}));
    send(&ws, json!({"cmd": "open_help", "window": "w-1"}));
    let v = view(&ws);
    assert_that!(v.windows[0].tabs).has_length(2);
    assert_that!(v.windows[0].active).is_equal_to(help);
}

#[test]
fn open_help_in_an_unshipped_language_opens_the_english_help() {
    let data = fresh_dir("oku-slides-workspace-help-");
    let ws = workspace(data.path(), "de");
    send(&ws, json!({"cmd": "open_help", "window": "w-1"}));
    let v = view(&ws);
    let help = v.windows[0].tabs.iter().find_map(|t| t.info.help.clone());
    assert_that!(help.as_deref()).is_equal_to(Some("en"));
}

#[test]
fn open_help_after_a_language_change_opens_the_help_in_the_new_language() {
    let data = fresh_dir("oku-slides-workspace-help-");
    let ws = workspace(data.path(), "en");
    send(&ws, json!({"cmd": "open_help", "window": "w-1"}));
    send(&ws, json!({"cmd": "set_locale", "locale": "fr"}));
    send(&ws, json!({"cmd": "open_help", "window": "w-1"}));
    let v = view(&ws);
    let mut helps: Vec<String> = v.windows[0].tabs.iter().filter_map(|t| t.info.help.clone()).collect();
    helps.sort();
    assert_that!(helps).is_equal_to(vec!["en".to_owned(), "fr".to_owned()]);
}

/// Deux demandes rapprochées : la seconde, construite pendant la première,
/// rejoint l'onglet déjà inséré au lieu d'en ouvrir un autre.
#[test]
fn open_help_built_twice_concurrently_keeps_a_single_help_tab() {
    let data = fresh_dir("oku-slides-workspace-help-");
    let ws = workspace(data.path(), "fr");
    let (first, second) = {
        let mut w = Workspace::lock(&ws);
        let locale = w.help_language();
        (w.help_pending("w-1".into(), locale, None), w.help_pending("w-1".into(), locale, None))
    };
    let (a, b) = (first.build().unwrap(), second.build().unwrap());
    Workspace::finish(&ws, first, a).unwrap();
    Workspace::finish(&ws, second, b).unwrap();
    let v = view(&ws);
    let helps = v.windows[0].tabs.iter().filter(|t| t.info.help.is_some()).count();
    assert_that!(helps).is_equal_to(1);
}

/// Les messages de l'interface écrivent les raccourcis comme la plateforme.
#[test]
fn messages_write_shortcuts_in_the_platform_notation() {
    let data = fresh_dir("oku-slides-workspace-help-");
    let ws = workspace(data.path(), "fr");
    let WorkspaceOutcome::Applied(json) = send(&ws, json!({"cmd": "messages"})) else { panic!("messages") };
    let mac = cfg!(target_os = "macos");
    assert_that!(json.contains("⌘Z")).is_equal_to(mac);
    assert_that!(json.contains("Ctrl+Z")).is_equal_to(!mac);
}

fn set_theme(ws: &Mutex<Workspace>, tab: &str, theme: &str) {
    let req =
        serde_json::from_value(json!({"cmd": "apply", "label": "Thème", "ops": [{"op": "set_theme", "theme": theme}]}));
    let (out, _) = Workspace::with_session(ws, tab, |s| s.handle(req.unwrap()).map(|_| ()));
    out.unwrap().unwrap();
}

fn help_tab(v: &WorkspaceView) -> &crate::TabView {
    v.windows[0].tabs.iter().find(|t| t.info.help.is_some()).unwrap()
}

#[test]
fn open_help_from_a_pro_deck_shows_the_help_in_pro() {
    let data = fresh_dir("oku-slides-workspace-help-");
    let ws = workspace(data.path(), "fr");
    set_theme(&ws, "t-1", "pro");
    send(&ws, json!({"cmd": "open_help", "window": "w-1"}));
    assert_that!(help_tab(&view(&ws)).info.theme.as_deref()).is_equal_to(Some("pro"));
}

#[test]
fn open_help_again_from_another_deck_takes_its_theme() {
    let data = fresh_dir("oku-slides-workspace-help-");
    let ws = workspace(data.path(), "fr");
    set_theme(&ws, "t-1", "pro");
    send(&ws, json!({"cmd": "open_help", "window": "w-1"}));
    set_theme(&ws, "t-1", "papier");
    send(&ws, json!({"cmd": "activate", "tab": "t-1"}));
    send(&ws, json!({"cmd": "open_help", "window": "w-1"}));
    assert_that!(help_tab(&view(&ws)).info.theme.as_deref()).is_equal_to(Some("papier"));
}

#[test]
fn open_help_from_the_help_tab_keeps_its_theme() {
    let data = fresh_dir("oku-slides-workspace-help-");
    let ws = workspace(data.path(), "fr");
    set_theme(&ws, "t-1", "pro");
    send(&ws, json!({"cmd": "open_help", "window": "w-1"}));
    send(&ws, json!({"cmd": "open_help", "window": "w-1"}));
    assert_that!(help_tab(&view(&ws)).info.theme.as_deref()).is_equal_to(Some("pro"));
}
