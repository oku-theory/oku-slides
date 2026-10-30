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
/// The one widget the library pack `acme` holds.
pub const ACME_WIDGET: &str = "badge";
/// The source of [`ACME_WIDGET`]: a `label` text shown in its field.
const BADGE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("badge", params: (label: p.plain()),
  (args, ctx) => field(ctx, "label", args.label))
"#;
/// The library pack `beta`.
pub const BETA: Uuid = Uuid::from_u128(0xac3e_0000_0000_4000_8000_0000_0000_0002);

/// Adds a library widget pack `namespace` (requiring `base`) under `id`, holding `widgets`.
fn add_library_pack(platform: &Platform, id: Uuid, namespace: &str, widgets: &[(&str, &str)]) {
    let ns = |s: &str| Namespace::parse(s).unwrap_or_else(|| panic!("invalid namespace: {s}"));
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
    let widgets = widgets
        .iter()
        .map(|(name, source)| {
            let file = oku_slides_core::WidgetFile::parse(name).unwrap_or_else(|| panic!("widget name: {name}"));
            (file, (*source).to_owned())
        })
        .collect();
    let files = manifest.to_files(PackKind::Widgets, widgets).unwrap_or_else(|e| panic!("manifest: {e}"));
    PackLibrary::user(platform).add_version(id, &files).unwrap_or_else(|e| panic!("library pack: {e}"));
}

/// A real workspace over a library holding `acme` and `beta`; its deck tab is `t-1`.
async fn workspace() -> (Platform, tempfile::TempDir, Arc<Mutex<Workspace>>) {
    tokio::task::spawn_blocking(|| {
        let (platform, data) = platform();
        add_library_pack(&platform, ACME, "acme", &[(ACME_WIDGET, BADGE)]);
        add_library_pack(&platform, BETA, "beta", &[]);
        let workspace =
            Workspace::new(Fonts::default(), &platform, None).unwrap_or_else(|e| panic!("test workspace: {e}"));
        (platform, data, Arc::new(Mutex::new(workspace)))
    })
    .await
    .unwrap_or_else(|e| panic!("workspace blocking task: {e}"))
}

/// A server bound to the deck tab `t-1` of a real workspace.
pub async fn start_on_workspace() -> (Harness, Arc<Mutex<Workspace>>) {
    let (platform, data, ws) = workspace().await;
    let session = Arc::new(Mutex::new(placeholder(&platform).await));
    let sessions: Arc<dyn Sessions> = ws.clone();
    (start_with((platform, data), sessions, session, "t-1").await, ws)
}

/// A server bound to the deck tab `t-1` of a real workspace, holding the import of fixture `name`.
pub async fn start_on_imported_workspace(name: &'static str) -> (Harness, Arc<Mutex<Workspace>>) {
    let (platform, data, ws) = imported_workspace(name).await;
    let session = Arc::new(Mutex::new(placeholder(&platform).await));
    let sessions: Arc<dyn Sessions> = ws.clone();
    (start_with((platform, data), sessions, session, "t-1").await, ws)
}

/// A server bound to the deck-pack tab of the import of `name`.
///
/// The tab is linked to the deck tab `t-1`; returns its id and its pack.
pub async fn start_in_deck_pack_tab(name: &'static str) -> (Harness, Arc<Mutex<Workspace>>, String, Uuid) {
    let (platform, data, ws) = imported_workspace(name).await;
    let pack = Workspace::with_session(&ws, "t-1", |s| s.deck().layouts.get(&Namespace::deck_layouts()).map(|p| p.id))
        .0
        .unwrap_or_else(|e| panic!("tab t-1: {e}"))
        .unwrap_or_else(|| panic!("the import's deck pack"));
    let tab = open_pack_tab(&ws, "t-1", pack).await;
    let session = Arc::new(Mutex::new(placeholder(&platform).await));
    let sessions: Arc<dyn Sessions> = ws.clone();
    (start_with((platform, data), sessions, session, &tab).await, ws, tab, pack)
}

/// [`workspace`], its deck tab `t-1` holding the hi-fi import of fixture `name`.
async fn imported_workspace(name: &'static str) -> (Platform, tempfile::TempDir, Arc<Mutex<Workspace>>) {
    let (platform, data, ws) = workspace().await;
    let (imported_on, root, tabs) = (platform.clone(), data.path().to_owned(), ws.clone());
    tokio::task::spawn_blocking(move || {
        use oku_slides_session::import::{ImportRequest, PackLookup, import};
        let pptx = super::pptx(&root, name);
        let request = ImportRequest { pptx, target: super::hifi(), system_fonts: false, packs: PackLookup::Library };
        let imported = import(&imported_on, &Fonts::default(), &request).unwrap_or_else(|e| panic!("import: {e}"));
        Workspace::with_session(&tabs, "t-1", |s| s.load_import(imported, name))
            .0
            .unwrap_or_else(|e| panic!("tab t-1: {e}"))
            .unwrap_or_else(|e| panic!("import load: {e}"));
    })
    .await
    .unwrap_or_else(|e| panic!("import blocking task: {e}"));
    (platform, data, ws)
}

/// The document of tab `tab`, as the tab bar shows it.
pub fn document_of(ws: &Arc<Mutex<Workspace>>, tab: &str) -> Option<oku_slides_session::TabDocument> {
    let view = lock(ws).view();
    view.windows.iter().flat_map(|w| &w.tabs).find(|t| t.id == tab).map(|t| t.info.document.clone())
}

/// A server bound to a help tab opened beside the deck tab `t-1`; the help tab's id.
pub async fn start_in_help_tab() -> (Harness, Arc<Mutex<Workspace>>, String) {
    let (platform, data, ws) = workspace().await;
    let tab = new_tab_by(&ws, json!({ "cmd": "open_help", "window": first_window(&ws) })).await;
    let session = Arc::new(Mutex::new(placeholder(&platform).await));
    let sessions: Arc<dyn Sessions> = ws.clone();
    (start_with((platform, data), sessions, session, &tab).await, ws, tab)
}

/// The item a pack tab `tab` selects (its `UiState.widget`), read in its session.
pub fn selected_item(ws: &Arc<Mutex<Workspace>>, tab: &str) -> Option<String> {
    tokio::task::block_in_place(|| {
        Workspace::with_session(ws, tab, |s| s.snapshot().ui.widget.clone())
            .0
            .unwrap_or_else(|e| panic!("tab {tab}: {e}"))
    })
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
        Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("test session: {e}"))
    })
    .await
    .unwrap_or_else(|e| panic!("setup blocking task: {e}"))
}

/// Sends the workspace request `req` as the interface would; blocking work, off the async thread.
async fn apply(ws: &Arc<Mutex<Workspace>>, req: Value) {
    let ws = ws.clone();
    tokio::task::spawn_blocking(move || {
        let request: WorkspaceRequest =
            serde_json::from_value(req).unwrap_or_else(|e| panic!("workspace request: {e}"));
        Workspace::apply(&ws, request).unwrap_or_else(|e| panic!("workspace request: {e}"));
    })
    .await
    .unwrap_or_else(|e| panic!("workspace blocking task: {e}"));
}

/// Opens `pack` from `from` as the user would (the interface's path); the new tab's id.
pub async fn open_pack_tab(ws: &Arc<Mutex<Workspace>>, from: &str, pack: Uuid) -> String {
    new_tab_by(ws, json!({ "cmd": "open_pack", "tab": from, "pack": pack })).await
}

/// Opens `theme` from `from` as the user would (the interface's path); the new tab's id.
pub async fn open_theme_tab(ws: &Arc<Mutex<Workspace>>, from: &str, theme: Uuid) -> String {
    new_tab_by(ws, json!({ "cmd": "open_theme", "tab": from, "theme": theme })).await
}

/// A blank tab at the end of the first window, made active as the interface does; its id.
pub async fn new_blank_tab(ws: &Arc<Mutex<Workspace>>) -> String {
    new_tab_by(ws, json!({ "cmd": "new_tab", "window": first_window(ws) })).await
}

/// Sends `req`, which must add exactly one tab; that tab's id.
async fn new_tab_by(ws: &Arc<Mutex<Workspace>>, req: Value) -> String {
    let before = tab_ids(ws);
    apply(ws, req).await;
    let new: Vec<String> = tab_ids(ws).into_iter().filter(|t| !before.contains(t)).collect();
    match new.as_slice() {
        [tab] => tab.clone(),
        other => panic!("exactly one new tab expected, got {other:?}"),
    }
}

/// The label of the first window.
fn first_window(ws: &Arc<Mutex<Workspace>>) -> String {
    lock(ws).view().windows.first().map_or_else(|| panic!("no window"), |w| w.label.clone())
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
    lock(ws).view().windows.first().map_or_else(|| panic!("no window"), |w| w.active.clone())
}

/// The pack id a pack tab edits, read in its session.
pub fn edited_pack(ws: &Arc<Mutex<Workspace>>, tab: &str) -> Option<Uuid> {
    tokio::task::block_in_place(|| {
        Workspace::with_session(ws, tab, |s| s.snapshot().editing.map(|e| e.mode.pack))
            .0
            .unwrap_or_else(|e| panic!("tab {tab}: {e}"))
    })
}

/// The preview theme namespace of a pack tab and whether the tab is dirty, read in its session.
pub fn preview_theme_and_dirty(ws: &Arc<Mutex<Workspace>>, tab: &str) -> (Option<String>, bool) {
    tokio::task::block_in_place(|| {
        Workspace::with_session(ws, tab, |s| {
            let snap = s.snapshot();
            (snap.theme.map(|ns| ns.as_str().to_owned()), snap.dirty)
        })
        .0
        .unwrap_or_else(|e| panic!("tab {tab}: {e}"))
    })
}

/// The next event of a hub subscription, within the usual wait; the test fails otherwise.
pub async fn next_event(
    rx: &mut tokio::sync::broadcast::Receiver<oku_slides_mcp::McpEvent>,
) -> oku_slides_mcp::McpEvent {
    super::wait_event(rx, Some).await
}

/// The files of the pack libraries (both kinds) under `data`, with their bytes, sorted.
pub fn library_files(data: &std::path::Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    use oku_slides_session::{Layouts, LibraryKind, Widgets};
    fn walk(dir: &std::path::Path, out: &mut Vec<(std::path::PathBuf, Vec<u8>)>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for e in entries {
            let p = e.unwrap_or_else(|e| panic!("{e}")).path();
            if p.is_dir() {
                walk(&p, out);
            } else {
                let bytes = std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
                out.push((p, bytes));
            }
        }
    }
    let mut out = Vec::new();
    for dir in [Widgets::DIR, Layouts::DIR] {
        walk(&data.join(dir), &mut out);
    }
    out.sort();
    out
}
