//! Oku Slides : application de bureau.
//!
//! Toute la logique est dans `oku-slides-session` : l'espace de travail
//! (fenêtres, onglets) et une session par onglet. L'interface appelle
//! `request` (un onglet), `workspace` (onglets et fenêtres), `drop_tab` et
//! `quit`. Démarrage : journaux → plateforme → espace de travail (reprise)
//! et dictée → fenêtres ; chacune reprend sa taille et sa position. Les
//! fichiers reçus au lancement s'ouvrent ensuite, hors du fil principal.
//!
//! `state` porte l'état de l'app et la fin propre (`state::shutdown`,
//! `Workspace::settle`) ; `commands` et `windows` en dépendent tous deux,
//! jamais l'inverse. Cinq chemins de sortie s'y retrouvent : `quit`, le menu
//! « Quitter » (`on_menu_event`, jamais le `terminate:` prédéfini de macOS),
//! la dernière fenêtre (`commands::workspace`), sa fermeture native
//! (`commands::close_requested`) et la vue vide (`windows::sync`). Un
//! sixième, `RunEvent::Exit`, sert de filet pour les chemins qui échappent à
//! tous ceux-là (Dock, session système) : lui seul revient toujours,
//! `Workspace::settle` y est donc rappelée sans condition.
//!
//! Une seule instance : une seconde transmet ses fichiers à la première ;
//! sous macOS, Finder les envoie (`RunEvent::Opened`).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod menu;
mod pending;
mod state;
mod windows;

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64};

use oku_slides_session::{Fonts, Platform, Storage, Workspace};
use oku_slides_stt::Stt;
use tauri::{Manager, RunEvent, WindowEvent};
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

use pending::Pending;
use state::AppState;

/// Polices livrées avec l'app (ressource `fonts/`), plus `paths.fonts` des
/// réglages ; un dossier illisible est signalé et ignoré.
fn app_fonts(storage: &Storage, dirs: &[PathBuf]) -> Fonts {
    Fonts::new(
        dirs.iter()
            .flat_map(|dir| {
                storage.read_fonts(dir).unwrap_or_else(|e| {
                    tracing::warn!(dir = %dir.display(), error = %oku_slides_platform::errors::one_line(&e), "polices");
                    Default::default()
                })
            })
            .map(|(_, bytes)| bytes),
    )
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

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    oku_slides_platform::telemetry::init();
    let platform = Platform::load()?;
    let builder = tauri::Builder::default().manage(Pending::default());
    // Linux : `tauri-plugin-single-instance` 2.4.5 fait un `unwrap()` sur le
    // bus de session D-Bus dans son greffon (`linux.rs`), et panique s'il n'y
    // en a pas (CI headless, conteneur sans D-Bus) ; il ne servirait de toute
    // façon à rien sans bus, puisque c'est par lui qu'il parle à l'autre
    // instance. Activé seulement si un bus est présent.
    let single_instance_available = cfg!(not(target_os = "linux"))
        || session_bus(
            std::env::var_os("DBUS_SESSION_BUS_ADDRESS").as_deref(),
            std::env::var_os("XDG_RUNTIME_DIR").as_deref(),
        );
    let builder = if single_instance_available {
        builder.plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            let files = launch_files(argv.into_iter().skip(1).map(PathBuf::from), Path::new(&cwd));
            // Toujours en premier : une seconde instance transmet ses
            // arguments à celle-ci, puis s'arrête avant d'avoir rien ouvert.
            if !files.is_empty() {
                if let Some(files) = app.state::<Pending>().receive(files) {
                    commands::open_files(app, files);
                }
                return;
            }
            // Relancée sans fichier : la fenêtre active revient au premier plan.
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
        }))
    } else {
        tracing::warn!("pas de bus D-Bus détecté : instance unique désactivée");
        builder
    };
    let builder = builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .on_window_event(|window, event| match event {
            // Taille et position gardées à chaque changement, pas seulement à la fermeture.
            WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
                if let Err(e) = window.app_handle().save_window_state(StateFlags::all()) {
                    tracing::warn!(error = %e, "état de la fenêtre non enregistré");
                }
            }
            WindowEvent::Focused(true) => commands::focused(window.app_handle(), window.label()),
            // Jamais de fermeture native : l'hôte (dernière fenêtre) ou la page décide.
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                commands::close_requested(window.app_handle(), window.label());
            }
            _ => {}
        });
    #[cfg(target_os = "macos")]
    let builder = {
        let localizer = platform.localizer.clone();
        builder.menu(move |app| menu::build(app, &localizer))
    };
    let app = builder
        .setup(move |app| {
            let mut dirs = Vec::new();
            if let Ok(res) = app.path().resource_dir() {
                dirs.push(res.join("fonts"));
            }
            dirs.extend(platform.settings.paths.fonts.clone());
            // En développement, les polices du dépôt.
            dirs.push(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../assets/fonts")));
            let fonts = app_fonts(&platform.storage, &dirs);
            let workspace = Workspace::resume(fonts, &platform, platform.settings.paths.data().join("resume"))?;
            app.manage(AppState {
                workspace: Mutex::new(workspace),
                localizer: platform.localizer.clone(),
                sync: Mutex::new(()),
                focus_seq: AtomicU64::new(0),
                focus_applied: Mutex::new(0),
                closing: AtomicBool::new(false),
            });
            app.manage(Stt::new(&platform));
            // « Quitter » (menu macOS) : jamais le `terminate:` natif, pour
            // toujours passer par `state::shutdown` (voir `menu.rs`).
            app.on_menu_event(|app, event| {
                if event.id() == menu::QUIT_ID {
                    let app = app.clone();
                    tauri::async_runtime::spawn_blocking(move || state::shutdown(&app));
                }
            });
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
        .invoke_handler(tauri::generate_handler![
            commands::request,
            commands::workspace,
            commands::drop_tab,
            commands::quit,
            commands::stt
        ])
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
