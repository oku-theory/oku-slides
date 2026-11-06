//! Oku Slides: the desktop app.
//!
//! All the logic lives in `oku-slides-session`: the workspace (windows, tabs) and one session per
//! tab. The UI calls `request` (one tab), `workspace` (tabs and windows), tab drags between windows
//! (`tab_drag`), `quit` and `quit_ready`. Startup: logs → platform → workspace (resume) and
//! dictation → windows; each restores its size and position, saved once a gesture ends
//! (`window_state`). Files received at launch open afterwards, off the main thread.
//!
//! `state` holds the app state and the clean end (`state::shutdown`, `Workspace::settle`);
//! `commands` and `windows` both depend on it, never the reverse. Exit paths: `quit`, the menu
//! Quit (`on_menu_event`, never macOS's predefined `terminate:`), a last-window `CloseWindow`
//! (`commands::workspace`), its native close (`commands::close_requested`) and an update install
//! (`updates::install`, then `request_restart`) all go through `quit` (`quit::exit`,
//! `quit::guarded`): every window runs its close guards first (t-1377). The empty view
//! (`windows::sync`) shuts down directly: each tab left through its close flow, guards run.
//! `RunEvent::Exit` is the safety net for paths that escape all of these (Dock, system logout:
//! the webviews are going away, nothing can be waited for); `Workspace::settle` is called there
//! unconditionally.
//!
//! A single instance: a second one hands its files to the first; on macOS, Finder sends them
//! (`RunEvent::Opened`).
//!
//! The agent (`oku-slides-agent::Stack`): one per tab, sessions through `AppSessions`; command
//! `agent`, events `oku-slides:mcp` and `oku-slides:agent` emitted to every window.
//!
//! `updates` looks for new versions and installs them; `app_port` lets the agent check, put off,
//! install (after its turn) and quit. Each window may call only the commands its
//! capability grants (`capabilities/*.json`, checked by `build.rs`, t-980).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod agent_window;
mod app_port;
mod app_workspace;
mod commands;
mod host_focus;
mod menu;
mod pending;
mod quit;
mod state;
mod tab_drag;
mod updates;
mod window_state;
mod windows;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex};

use oku_slides_agent::{Stack, Voices};
use oku_slides_session::{Fonts, Platform, Storage, Workspace};
use oku_slides_stt::{Speaker, Stt};
use tauri::{Emitter, Manager, RunEvent, WindowEvent};

use host_focus::HostFocus;
use pending::Pending;
use state::{AppSessions, AppState};

/// Polices livrées avec l'app (ressource `fonts/`), plus `paths.fonts` des
/// réglages ; un dossier illisible est signalé et ignoré.
fn app_fonts(storage: &Storage, dirs: &[PathBuf]) -> Fonts {
    Fonts::new(
        dirs.iter()
            .flat_map(|dir| {
                storage.read_fonts(dir).unwrap_or_else(|e| {
                    tracing::warn!(dir = %dir.display(), error = %oku_slides_platform::errors::one_line(&e), "polices");
                    std::collections::BTreeMap::default()
                })
            })
            .map(|(_, bytes)| bytes),
    )
}

/// The font directories, in search order.
///
/// The `fonts/` resource, `paths.fonts` from the settings, then in development
/// the repository's fonts.
fn font_dirs(app: &tauri::AppHandle, platform: &Platform) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(res) = app.path().resource_dir() {
        dirs.push(res.join("fonts"));
    }
    dirs.extend(platform.settings.paths.fonts.clone());
    dirs.push(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../assets/fonts")));
    dirs
}

/// Fichiers reçus en arguments : relatifs au dossier de lancement, et des fichiers.
fn launch_files(args: impl Iterator<Item = PathBuf>, cwd: &Path) -> Vec<PathBuf> {
    args.map(|p| if p.is_absolute() { p } else { cwd.join(p) }).filter(|p| p.is_file()).collect()
}

/// Un bus de session D-Bus : adresse donnée, ou socket `bus` du dossier d'exécution.
///
/// `XDG_RUNTIME_DIR` seul ne prouve rien (systemd le pose sans bus) : il
/// faut que `$XDG_RUNTIME_DIR/bus`, l'adresse par défaut de libdbus, existe.
fn session_bus(address: Option<&std::ffi::OsStr>, runtime_dir: Option<&std::ffi::OsStr>) -> bool {
    address.is_some_and(|a| !a.is_empty()) || runtime_dir.is_some_and(|d| Path::new(d).join("bus").exists())
}

/// Handles a second instance launched with `argv` from `cwd`.
///
/// Its files open here; without files, the active window comes back to the
/// front.
fn second_instance(app: &tauri::AppHandle, argv: Vec<String>, cwd: &str) {
    let files = launch_files(argv.into_iter().skip(1).map(PathBuf::from), Path::new(cwd));
    // Always first: a second instance forwards its arguments to this
    // one, then exits before opening anything.
    if !files.is_empty() {
        if let Some(files) = app.state::<Pending>().receive(files) {
            commands::open_files(app, files);
        }
        return;
    }
    // Relaunched without files: the active window comes back to the front.
    let Some(state) = app.try_state::<AppState>() else {
        tracing::debug!("seconde instance sans fichier : état pas encore prêt");
        return;
    };
    let Some(focused) = state.try_workspace().map(|ws| ws.focused().to_owned()) else {
        tracing::debug!("seconde instance sans fichier : espace de travail occupé");
        return;
    };
    let Some(w) = app.get_webview_window(&focused) else {
        tracing::debug!(label = focused, "seconde instance sans fichier : fenêtre introuvable");
        return;
    };
    if let Err(e) = w.unminimize().and_then(|()| w.set_focus()) {
        tracing::warn!(label = focused, error = %e, "fenêtre non mise au premier plan");
    }
}

/// Native window events: state saved, focus tracked, closing decided by the host.
fn window_event(window: &tauri::Window, event: &WindowEvent) {
    match event {
        // Size and position are saved once the gesture ends, not only on close.
        WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
            if let Some(saver) = window.app_handle().try_state::<window_state::Saver>() {
                saver.changed();
            }
        }
        WindowEvent::Focused(true) => {
            if let Some(focus) = window.app_handle().try_state::<Arc<HostFocus>>() {
                focus.gained(window.label());
            }
            commands::focused(window.app_handle(), window.label());
        }
        // The voice router hears only stop and wake once no window has the focus.
        WindowEvent::Focused(false) => {
            if let Some(focus) = window.app_handle().try_state::<Arc<HostFocus>>() {
                focus.lost(window.label());
            }
        }
        // Its page reports go with it: a page of a later window starts over.
        WindowEvent::Destroyed => {
            if let Some(focus) = window.app_handle().try_state::<Arc<HostFocus>>() {
                focus.lost(window.label());
            }
            if let Some(state) = window.app_handle().try_state::<AppState>() {
                state.stack.window_destroyed(window.label());
            }
        }
        // Never a native close: the host (last window) or the page decides.
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            commands::close_requested(window.app_handle(), window.label());
        }
        _ => {}
    }
}

/// Clicks on the app menu.
///
/// Agent window, update check, "Quit".
fn menu_event(app: &tauri::AppHandle, event: &tauri::menu::MenuEvent) {
    if event.id() == menu::AGENT_ID {
        agent_window::toggle(app);
    }
    if event.id() == menu::CHECK_UPDATES_ID {
        let focused = app.try_state::<AppState>().and_then(|s| s.try_workspace().map(|ws| ws.focused().to_owned()));
        if let Some(label) = focused {
            if let Err(e) = app.emit_to(label.as_str(), "update-check-requested", ()) {
                tracing::warn!(error = %e, "recherche de mise à jour non transmise");
            }
        } else {
            tracing::debug!("recherche de mise à jour : espace de travail occupé, clic ignoré");
        }
    }
    if event.id() == menu::QUIT_ID {
        quit::exit(app);
    }
}

/// Adds the single-instance plugin where it can work.
///
/// On Linux, `tauri-plugin-single-instance` 2.4.5 unwraps the D-Bus session bus in its
/// plugin (`linux.rs`) and panics without one (headless CI, a container without D-Bus); it
/// would be useless anyway without a bus, since it talks to the other instance through it.
/// Enabled only when a bus is there.
fn with_single_instance(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    let available = cfg!(not(target_os = "linux"))
        || session_bus(
            std::env::var_os("DBUS_SESSION_BUS_ADDRESS").as_deref(),
            std::env::var_os("XDG_RUNTIME_DIR").as_deref(),
        );
    if available {
        builder.plugin(tauri_plugin_single_instance::init(|app, argv, cwd| second_instance(app, argv, &cwd)))
    } else {
        tracing::warn!("pas de bus D-Bus détecté : instance unique désactivée");
        builder
    }
}

/// Registers the app commands, exactly those of `build.rs`'s `COMMANDS`.
fn with_commands(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder.invoke_handler(tauri::generate_handler![
        commands::request,
        commands::workspace,
        tab_drag::drag_tab_over,
        tab_drag::drag_tab_leave,
        tab_drag::drop_tab,
        commands::quit,
        quit::quit_ready,
        commands::stt,
        commands::agent,
        commands::agent_link,
        commands::take_close_decision,
        updates::update_status,
        updates::check_update,
        updates::install_update,
        updates::open_release_page,
        updates::later_update,
        updates::take_update_notice
    ])
}

fn main() -> color_eyre::Result<()> {
    // Relancé en processus de rendu (`--render-worker`) : sert le tube, puis termine.
    oku_slides_render_worker::exit_if_worker();
    // Relancé en agent local (`--local-agent`) : sert ACP sur stdin/stdout, puis termine.
    oku_slides_local_agent::exit_if_local_agent();
    color_eyre::install()?;
    oku_slides_platform::telemetry::init();
    // Launched from the Finder, the app has launchd's `PATH`: the login shell's leads (t-956).
    let platform = Platform::load()?.with_login_path();
    let builder = tauri::Builder::default().manage(Pending::default()).manage(tab_drag::Hover::default());
    let builder = with_single_instance(builder)
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .on_window_event(window_event);
    #[cfg(target_os = "macos")]
    let builder = {
        let localizer = platform.localizer.clone();
        builder.menu(move |app| menu::build(app, &localizer))
    };
    let app = with_commands(builder)
        .setup(move |app| {
            let fonts = app_fonts(&platform.storage, &font_dirs(app.handle(), &platform));
            let workspace = Workspace::resume(fonts, &platform, &platform.settings.paths.data().join("resume"))?;
            let workspace = Arc::new(Mutex::new(workspace));
            let stt = Stt::new(&platform);
            // Les outils et l'agent vivent dans le runtime tokio de Tauri.
            let rt = tauri::async_runtime::handle().inner().clone();
            let sessions = AppSessions::new(app.handle().clone(), Arc::clone(&workspace));
            let focus = Arc::new(HostFocus::new(Arc::clone(&workspace)));
            app.manage(Arc::clone(&focus));
            let agent_workspace =
                Arc::new(app_workspace::AppWorkspace::new(app.handle().clone(), Arc::clone(&workspace), focus));
            let host = Arc::new(app_port::TauriApp::new(app.handle().clone()));
            // One read-aloud voice for the whole process, reading the focused tab.
            let voices =
                Voices { stt: Some(stt.clone()), speaker: Some(Speaker::for_platform(&platform, Some(stt.clone()))) };
            let stack = Stack::start(Arc::new(sessions), &platform, voices, agent_workspace, host, rt);
            let handle = app.handle().clone();
            stack.forward(move |name, json| {
                if let Err(e) = handle.emit(&format!("oku-slides:{name}"), json) {
                    tracing::debug!(name, error = %e, "évènement de l'agent non émis");
                }
            });
            app.manage(AppState {
                workspace,
                localizer: platform.localizer.clone(),
                sync: Mutex::new(()),
                focus_seq: AtomicU64::new(0),
                focus_applied: Mutex::new(0),
                closing: AtomicBool::new(false),
                stack,
            });
            app.manage(stt);
            app.manage(quit::Quit::default());
            app.manage(window_state::Saver::new(app.handle().clone()));
            app.manage(updates::Updates::new(platform.storage.clone(), &platform.settings.paths.data()));
            updates::spawn_checks(app.handle(), &platform.settings.updates);
            // « Quitter » (menu macOS) : jamais le `terminate:` natif, pour
            // toujours passer par `state::shutdown` (voir `menu.rs`).
            app.on_menu_event(|app, event| menu_event(app, &event));
            // Avant la boucle d'événements : aucune commande concurrente encore,
            // direct sur ce fil (ailleurs, toujours hors du fil principal).
            let view = windows::sync(app.handle(), None, true);
            // `Workspace::resume` insère toujours un onglet vierge si elle ne
            // restaure aucune fenêtre : `view.windows` n'est donc jamais vide
            // ici (contrairement à un espace de travail vidé plus tard par
            // l'usage, que `windows::sync` traite alors en le quittant).
            // Un `Err` ici fait paniquer Tauri (`app.run` capture le message) :
            // plus net qu'un `std::process::exit(1)` muet, et le seul moyen
            // de sortir de `setup` proprement (sa signature ne permet rien
            // d'autre).
            if !view.windows.is_empty() && app.webview_windows().is_empty() {
                tracing::error!("aucune fenêtre n'a pu être ouverte au démarrage");
                return Err("no window could be opened at startup".into());
            }
            // Ouvrir avec… (Windows, Linux) : chaque fichier dans un onglet, hors du fil principal.
            // S'y ajoutent les fichiers reçus avant la fin de `setup` (macOS,
            // seconde instance : voir `pending`). `ready` doit être le tout
            // dernier appel de `setup`, après `windows::sync` ci-dessus : elle
            // bascule aussi la file en mode direct pour tout ce qui arrive
            // ensuite (le rappel du greffon, `RunEvent::Opened`).
            let cwd = std::env::current_dir().unwrap_or_default();
            let mut files = launch_files(std::env::args_os().skip(1).map(PathBuf::from), &cwd);
            files.append(&mut app.state::<Pending>().ready());
            commands::open_files(app.handle(), files);
            Ok(())
        })
        .build(tauri::generate_context!())?;
    app.run(|app_handle, event| {
        // Filet de sécurité : Quitter depuis le Dock, ou une fermeture de
        // session système, appelle `terminate:` directement — jamais notre
        // menu, jamais `commands::quit`. Seul `RunEvent::Exit` le voit.
        // Sûr sur ce fil ici : la boucle d'événements s'arrête, aucun fil ne
        // peut donc plus détenir le verrou de l'espace de travail ou d'une
        // session en attendant que celui-ci avance.
        if let RunEvent::Exit = event
            && let Some(state) = app_handle.try_state::<AppState>()
        {
            Workspace::settle(&state.workspace);
        }
        // macOS : Finder ne relance pas l'app, il lui envoie les fichiers
        // (au lancement aussi : l'argument n'y est pas). `Pending::receive`
        // met en file si `setup` n'est pas encore fini (`ready` pas encore
        // appelée), sinon renvoie les fichiers pour ouverture immédiate.
        #[cfg(target_os = "macos")]
        if let RunEvent::Opened { urls } = &event {
            let files: Vec<PathBuf> =
                urls.iter().filter_map(|u| u.to_file_path().ok()).filter(|p| p.is_file()).collect();
            if let Some(files) = app_handle.state::<Pending>().receive(files) {
                commands::open_files(app_handle, files);
            }
        }
    });
    Ok(())
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod main_tests;
