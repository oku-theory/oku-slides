//! A server on a real workspace: library pack tabs opened beside a deck tab.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use oku_slides_core::{Namespace, PackKind, PackScope};
use oku_slides_session::agent::Sessions;
use oku_slides_session::{Fonts, PackLibrary, Platform, Session, Workspace, WorkspaceRequest};
use serde_json::{Value, json};
use uuid::Uuid;

use super::{Harness, lock, platform, start_with};

/// The library pack `acme`.
pub const ACME: Uuid = Uuid::from_u128(0xac3e_0000_0000_4000_8000_0000_0000_0001);
/// The library pack `beta`.
pub const BETA: Uuid = Uuid::from_u128(0xac3e_0000_0000_4000_8000_0000_0000_0002);

/// Adds an empty library widget pack `namespace` (requiring `base`) under `id`.
fn add_library_pack(platform: &Platform, id: Uuid, namespace: &str) {
    let ns = |s: &str| Namespace::parse(s).unwrap_or_else(|| panic!("espace de noms invalide : {s}"));
    let manifest = oku_slides_session::PackManifest {
        id,
        name: namespace.to_owned(),
        namespace: ns(namespace),
        requires: vec![ns("base")],
        description: String::new(),
        parent: None,
        created: "2026-10-01T10:00:00Z".into(),
        scope: PackScope::Library,
        profiles: Vec::new(),
        examples: BTreeMap::new(),
        styles: BTreeMap::new(),
        tokens: Vec::new(),
    };
    let files = manifest.to_files(PackKind::Widgets, BTreeMap::new()).unwrap_or_else(|e| panic!("manifeste : {e}"));
    PackLibrary::user(platform).add_version(id, &files).unwrap_or_else(|e| panic!("pack de la bibliothèque : {e}"));
}

/// A real workspace over a library holding `acme` and `beta`; its deck tab is `t-1`.
async fn workspace() -> (Platform, tempfile::TempDir, Arc<Mutex<Workspace>>) {
    tokio::task::spawn_blocking(|| {
        let (platform, data) = platform();
        add_library_pack(&platform, ACME, "acme");
        add_library_pack(&platform, BETA, "beta");
        let workspace = Workspace::new(Fonts::default(), &platform, None)
            .unwrap_or_else(|e| panic!("espace de travail de test : {e}"));
        (platform, data, Arc::new(Mutex::new(workspace)))
    })
    .await
    .unwrap_or_else(|e| panic!("tâche bloquante de l'espace de travail : {e}"))
}

/// A server bound to the deck tab `t-1` of a real workspace.
pub async fn start_on_workspace() -> (Harness, Arc<Mutex<Workspace>>) {
    let (platform, data, ws) = workspace().await;
    let session = Arc::new(Mutex::new(placeholder(&platform).await));
    let sessions: Arc<dyn Sessions> = ws.clone();
    (start_with((platform, data), sessions, session, "t-1").await, ws)
}

/// A server bound to the pack tab of `acme`, opened beside the deck tab `t-1`; the tab's id.
pub async fn start_in_pack_tab() -> (Harness, Arc<Mutex<Workspace>>, String) {
    let (platform, data, ws) = workspace().await;
    let tab = open_pack_tab(&ws, "t-1", ACME).await;
    let session = Arc::new(Mutex::new(placeholder(&platform).await));
    let sessions: Arc<dyn Sessions> = ws.clone();
    (start_with((platform, data), sessions, session, &tab).await, ws, tab)
}

/// The session `Harness.session` holds: unrelated to the workspace's, as in `start_closed`.
async fn placeholder(platform: &Platform) -> Session {
    let platform = platform.clone();
    tokio::task::spawn_blocking(move || {
        Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session de test : {e}"))
    })
    .await
    .unwrap_or_else(|e| panic!("tâche bloquante de préparation : {e}"))
}

/// Sends the workspace request `req` as the interface would; blocking work, off the async thread.
async fn apply(ws: &Arc<Mutex<Workspace>>, req: Value) {
    let ws = ws.clone();
    tokio::task::spawn_blocking(move || {
        let request: WorkspaceRequest =
            serde_json::from_value(req).unwrap_or_else(|e| panic!("requête de l'espace de travail : {e}"));
        Workspace::apply(&ws, request).unwrap_or_else(|e| panic!("requête de l'espace de travail : {e}"));
    })
    .await
    .unwrap_or_else(|e| panic!("tâche bloquante de l'espace de travail : {e}"));
}

/// Opens `pack` from `from` as the user would (the interface's path); the new tab's id.
pub async fn open_pack_tab(ws: &Arc<Mutex<Workspace>>, from: &str, pack: Uuid) -> String {
    let before = tab_ids(ws);
    apply(ws, json!({ "cmd": "open_pack", "tab": from, "pack": pack })).await;
    let new: Vec<String> = tab_ids(ws).into_iter().filter(|t| !before.contains(t)).collect();
    match new.as_slice() {
        [tab] => tab.clone(),
        other => panic!("un seul nouvel onglet attendu, reçu {other:?}"),
    }
}

/// Makes `tab` the active tab of its window, as a click on the tab bar.
pub async fn activate(ws: &Arc<Mutex<Workspace>>, tab: &str) {
    apply(ws, json!({ "cmd": "activate", "tab": tab })).await;
    assert_eq!(active_tab(ws), tab, "l'onglet {tab} doit être actif");
}

/// The open tabs, in bar order.
pub fn tab_ids(ws: &Arc<Mutex<Workspace>>) -> Vec<String> {
    lock(ws).tab_ids()
}

/// The active tab of the first window.
pub fn active_tab(ws: &Arc<Mutex<Workspace>>) -> String {
    lock(ws).view().windows.first().map_or_else(|| panic!("aucune fenêtre"), |w| w.active.clone())
}
