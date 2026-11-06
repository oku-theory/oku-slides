//! Tauri build script: generates the app context (configuration, icons, capabilities).
//!
//! Every app command is declared in the [`tauri_build::AppManifest`]: tauri-build generates its
//! `allow-<command>` permission, and the runtime refuses a command that no capability of the
//! calling window grants (t-980). The build also fails when the declared list drifts from
//! `generate_handler!` or a capability grants more than its window needs: app tests run in no gate,
//! clippy builds this script on every `just check`.

use std::collections::BTreeSet;

/// The app commands, exactly those of `main.rs`'s `generate_handler!`.
const COMMANDS: &[&str] = &[
    "request",
    "workspace",
    "drag_tab_over",
    "drag_tab_leave",
    "drop_tab",
    "quit",
    "quit_ready",
    "stt",
    "agent",
    "agent_link",
    "take_close_decision",
    "update_status",
    "check_update",
    "install_update",
    "open_release_page",
    "later_update",
    "take_update_notice",
];

/// What the agent window may call: its tab's requests, the workspace, the agent, dictation, the
/// quit answer and the agent link relay.
///
/// It shows content from the agent and imports: no update, quit, tab drag or file dialog.
const AGENT_COMMANDS: &[&str] = &["agent", "agent_link", "quit_ready", "request", "stt", "workspace"];

/// Core permissions the agent window may not hold: each grants emitting events to any window (t-1469).
///
/// It reaches the main window only through `agent_link`, which the host bounds.
const AGENT_DENIED: &[&str] =
    &["core:default", "core:event:default", "core:event:allow-emit", "core:event:allow-emit-to"];

type BuildError = Box<dyn std::error::Error + Send + Sync>;

fn main() -> Result<(), BuildError> {
    println!("cargo:rerun-if-changed=src/main.rs");
    check_handlers(&std::fs::read_to_string("src/main.rs")?)?;
    check_capability_files()?;
    check_capability("capabilities/default.json", "w-*", COMMANDS)?;
    check_capability("capabilities/agent.json", "agent", AGENT_COMMANDS)?;
    check_denied("capabilities/agent.json", AGENT_DENIED)?;
    let attributes = tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(COMMANDS));
    tauri_build::try_build(attributes)?;
    Ok(())
}

/// The command names of `generate_handler!` in `main`, without their module path, equal `COMMANDS`.
fn check_handlers(main: &str) -> Result<(), BuildError> {
    let start = main.find("generate_handler![").ok_or("main.rs: no generate_handler!")?;
    let rest = &main[start + "generate_handler![".len()..];
    let list = &rest[..rest.find(']').ok_or("main.rs: generate_handler! not closed")?];
    let registered: BTreeSet<&str> =
        list.split(',').map(str::trim).filter(|c| !c.is_empty()).filter_map(|c| c.rsplit("::").next()).collect();
    let declared: BTreeSet<&str> = COMMANDS.iter().copied().collect();
    if registered != declared {
        return Err(format!("build.rs COMMANDS {declared:?} differ from generate_handler! {registered:?}").into());
    }
    Ok(())
}

/// Tauri loads every file of `capabilities/`: only the two checked ones may exist.
fn check_capability_files() -> Result<(), BuildError> {
    let names: BTreeSet<String> = std::fs::read_dir("capabilities")?
        .map(|entry| entry.map(|e| e.file_name().to_string_lossy().into_owned()))
        .collect::<Result<_, _>>()?;
    let expected: BTreeSet<String> = ["agent.json", "default.json"].map(str::to_owned).into();
    if names != expected {
        return Err(
            format!("capabilities/ holds {names:?}, expected {expected:?}: check any new file in build.rs").into()
        );
    }
    Ok(())
}

/// The keys a checked capability may have; `remote`, `webviews` or `local` would widen it.
const CAPABILITY_KEYS: &[&str] = &["$schema", "identifier", "description", "windows", "permissions"];

/// The capability at `path` covers only `window` and grants exactly the app commands `commands`.
///
/// No create permission either: a window a webview made could take a `w-*` label and every command.
/// An app permission other than `allow-<cmd>` (a set) is refused: it would grant unchecked commands.
fn check_capability(path: &str, window: &str, commands: &[&str]) -> Result<(), BuildError> {
    let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let keys = json.as_object().ok_or(format!("{path}: not an object"))?.keys();
    if let Some(key) = keys.into_iter().find(|k| !CAPABILITY_KEYS.contains(&k.as_str())) {
        return Err(format!("{path}: key {key:?} is not checked by build.rs").into());
    }
    let windows: Vec<&str> =
        json["windows"].as_array().ok_or(format!("{path}: no windows"))?.iter().filter_map(|w| w.as_str()).collect();
    if windows != [window] {
        return Err(format!("{path}: windows {windows:?}, expected [{window:?}]").into());
    }
    let permissions: Vec<&str> = json["permissions"]
        .as_array()
        .ok_or(format!("{path}: no permissions"))?
        .iter()
        .map(|p| p.as_str().ok_or(format!("{path}: permission {p} is not a name")))
        .collect::<Result<_, _>>()?;
    if let Some(create) = permissions.iter().find(|p| p.contains("create")) {
        return Err(format!("{path}: {create} lets a webview create a window").into());
    }
    if let Some(set) = permissions.iter().find(|p| !p.contains(':') && !p.starts_with("allow-")) {
        return Err(format!("{path}: app permission {set} is not an allow-<command>").into());
    }
    let granted: BTreeSet<String> =
        permissions.iter().filter_map(|p| p.strip_prefix("allow-")).map(|c| c.replace('-', "_")).collect();
    let expected: BTreeSet<String> = commands.iter().map(|c| (*c).to_owned()).collect();
    if granted != expected {
        return Err(format!("{path}: grants {granted:?}, expected {expected:?}").into());
    }
    Ok(())
}

/// The capability at `path` grants none of `denied`.
fn check_denied(path: &str, denied: &[&str]) -> Result<(), BuildError> {
    let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let permissions = json["permissions"].as_array().ok_or(format!("{path}: no permissions"))?;
    if let Some(p) = permissions.iter().filter_map(|p| p.as_str()).find(|p| denied.contains(p)) {
        return Err(format!("{path}: {p} lets the window emit events to any window").into());
    }
    Ok(())
}
