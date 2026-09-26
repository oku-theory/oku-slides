//! Menu d'app sous macOS : le menu par défaut, sans « Fermer la fenêtre ».
//!
//! ⌘W appartient à la page (fermer l'onglet). Les autres systèmes n'ont pas
//! de menu d'app.

use oku_slides_session::Localizer;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Runtime};

/// Identifiant de l'entrée « Quitter » (voir `main.rs::on_menu_event`).
pub const QUIT_ID: &str = "quit";

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
    let window = Submenu::with_items(
        app,
        l.tr("menu-window"),
        true,
        &[&PredefinedMenuItem::minimize(app, None)?, &PredefinedMenuItem::fullscreen(app, None)?],
    )?;
    Menu::with_items(app, &[&app_menu, &edit, &window])
}
