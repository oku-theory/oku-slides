//! t-1063 : un onglet non enregistré d'une version plus ancienne était mis de côté sans avis qui le nomme.

use std::path::Path;
use std::sync::Mutex;

use oku_slides_session::{Fonts, Platform, Request, Settings, Workspace, WorkspaceRequest};
use serde_json::json;
use speculoos::prelude::*;

use crate::common::data_dir;

fn platform(data: &Path) -> Platform {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.data = Some(data.to_owned());
    settings.paths.decks = Some(data.join("decks"));
    settings.i18n.locale = Some("fr".into());
    Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme : {e}"))
}

fn resumed(data: &Path) -> Mutex<Workspace> {
    let ws = Workspace::resume(Fonts::default(), &platform(data), &data.join("resume"))
        .unwrap_or_else(|e| panic!("reprise : {}", e.one_line()));
    Mutex::new(ws)
}

fn send(ws: &Mutex<Workspace>, v: serde_json::Value) {
    let req: WorkspaceRequest = serde_json::from_value(v).unwrap_or_else(|e| panic!("requête : {e}"));
    Workspace::apply(ws, req).unwrap_or_else(|e| panic!("requête refusée : {}", e.one_line()));
}

fn tab(ws: &Mutex<Workspace>, id: &str, v: serde_json::Value) -> serde_json::Value {
    let req: Request = serde_json::from_value(v).unwrap_or_else(|e| panic!("requête : {e}"));
    let reply = Workspace::request(ws, id, req).0.unwrap_or_else(|e| panic!("requête refusée : {}", e.one_line()));
    serde_json::from_str(&reply).unwrap_or_else(|e| panic!("réponse : {e}"))
}

/// L'état de reprise de l'onglet `id` réécrit comme l'écrivait la version 1 de l'état.
fn write_resume_state_of_version_1(data: &Path, id: &str) {
    let path = data.join("resume").join("tabs").join(id).join("state.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} : {e}", path.display()));
    let mut state: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("état : {e}"));
    state["version"] = json!(1);
    std::fs::write(&path, state.to_string()).unwrap_or_else(|e| panic!("{} : {e}", path.display()));
}

#[test]
fn issue_1063_old_tab_set_aside_raises_a_notice() {
    let data = data_dir();
    let ws = resumed(&data);
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"}));
    tab(
        &ws,
        "t-2",
        json!({"cmd": "apply", "label": "Titre", "ops": [
            {"op": "set_deck_param", "key": "title", "value": "Ancien"}
        ]}),
    );
    send(&ws, json!({"cmd": "activate", "tab": "t-1"}));
    drop(ws);
    write_resume_state_of_version_1(&data, "t-2");
    let ws = resumed(&data);
    let snap = tab(&ws, "t-2", json!({"cmd": "snapshot"}));
    let expected = platform(&data).localizer.tr_with("notice-tab-set-aside", &[("name", "Ancien".into())]);
    assert_that!(snap["notice"].as_str()).is_equal_to(Some(expected.as_str()));
}
