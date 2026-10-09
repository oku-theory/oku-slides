//! Menu d'app sous macOS : le menu par défaut, sans « Fermer la fenêtre ».
//!
//! ⌘W appartient à la page (fermer l'onglet). Les autres systèmes n'ont pas
//! de menu d'app.

use oku_slides_session::Localizer;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Manager, Runtime};

/// Identifiant de l'entrée « Quitter » (voir `main.rs::on_menu_event`).
pub const QUIT_ID: &str = "quit";
/// Identifiant de la case « Agent dans une fenêtre séparée ».
pub const AGENT_ID: &str = "agent-detached";

/// La case de l'agent, gardée par l'app : `Menu::get` ne cherche pas dans les sous-menus.
#[cfg_attr(not(target_os = "macos"), expect(dead_code, reason = "menu d'app : macOS seulement"))]
struct AgentCheck<R: Runtime>(CheckMenuItem<R>);

/// Le menu : application, édition (couper, copier, coller), fenêtre.
///
/// « Quitter » n'est pas `PredefinedMenuItem::quit` : celle-ci appelle le
/// `terminate:` natif de macOS directement, qui ne passe jamais par notre
/// gestionnaire d'événements de menu (donc jamais par `state::shutdown`, donc
/// jamais par `Workspace::settle`). Une entrée à nous, avec le même
/// raccourci (⌘Q), pour que quitter par le menu attende bien les requêtes en
/// cours et enregistre l'espace de travail. Quitter depuis le Dock reste un
/// autre chemin (jamais par un menu) : filet de sécurité dans
/// `main.rs::app.run`, sur `RunEvent::Exit`.
#[cfg_attr(not(target_os = "macos"), expect(dead_code, reason = "menu d'app : macOS seulement"))]
pub fn build<R: Runtime>(app: &AppHandle<R>, l: &Localizer) -> tauri::Result<Menu<R>> {
    let about = PredefinedMenuItem::about(app, None, None)?;
    let quit = MenuItem::with_id(app, QUIT_ID, l.tr("menu-quit"), true, Some("CmdOrCtrl+Q"))?;
    let app_menu = Submenu::with_items(
        app,
        "Oku Slides",
        true,
        &[
            &about,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::show_all(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;
    let edit = Submenu::with_items(
        app,
        l.tr("menu-edit"),
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;
    let agent = CheckMenuItem::with_id(app, AGENT_ID, l.tr("menu-agent-detached"), true, false, None::<&str>)?;
    let window = Submenu::with_items(
        app,
        l.tr("menu-window"),
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::fullscreen(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &agent,
        ],
    )?;
    app.manage(AgentCheck(agent.clone()));
    Menu::with_items(app, &[&app_menu, &edit, &window])
}

/// Coche la case de l'agent d'après l'espace de travail (macOS ; ailleurs, aucun menu : rien).
pub fn set_agent_checked<R: Runtime>(app: &AppHandle<R>, detached: bool) {
    #[cfg(target_os = "macos")]
    {
        let Some(check) = app.try_state::<AgentCheck<R>>() else { return };
        if let Err(e) = check.0.set_checked(detached) {
            tracing::warn!(error = %e, "case de l'agent non mise à jour");
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (app, detached);
}
