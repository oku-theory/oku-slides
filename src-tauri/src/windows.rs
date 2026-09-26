//! Fenêtres de l'app : une par fenêtre de l'espace de travail, même label.
//!
//! Taille et position : `tauri-plugin-window-state`, par label. Toutes les
//! coordonnées ici sont logiques (points sous macOS). `sync`, `rects` et
//! `emit_view` attendent un verrou (et, pour `sync`, la fabrique ou la
//! destruction d'une fenêtre) : jamais sur le fil principal, toujours depuis
//! `commands::blocking` (voir ce module).

use std::sync::PoisonError;

use oku_slides_session::{Rect, Workspace, WorkspaceView};
use tauri::{
    AppHandle, Emitter, LogicalPosition, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use crate::state::{self, AppState};

/// Taille d'une fenêtre neuve, et taille minimale.
const SIZE: (f64, f64) = (1440.0, 900.0);
const MIN_SIZE: (f64, f64) = (1024.0, 640.0);
/// Décalage du coin de la fenêtre au pointeur : l'onglet lâché tombe dans sa
/// barre, en bas de la fenêtre (36 points de haut), sur le premier onglet.
const DROP_OFFSET: (f64, f64) = (80.0, SIZE.1 - 18.0);

/// Coin de la fenêtre neuve : le point lâché, décalé par `DROP_OFFSET`.
fn drop_corner(at: LogicalPosition<f64>) -> (f64, f64) {
    (at.x - DROP_OFFSET.0, at.y - DROP_OFFSET.1)
}

/// Ouvre la fenêtre `label`, sous le pointeur `at` s'il est donné.
fn open(app: &AppHandle, label: &str, at: Option<LogicalPosition<f64>>) -> tauri::Result<WebviewWindow> {
    let mut builder = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title("Oku Slides")
        .inner_size(SIZE.0, SIZE.1)
        .min_inner_size(MIN_SIZE.0, MIN_SIZE.1);
    if let Some(p) = at {
        let (x, y) = drop_corner(p);
        builder = builder.position(x, y);
    }
    #[cfg(target_os = "macos")]
    let builder = builder.title_bar_style(tauri::TitleBarStyle::Overlay).hidden_title(true);
    builder.build()
}

/// Rectangle logique d'une fenêtre, d'après ses mesures physiques.
///
/// `scale` : celle de l'écran de la fenêtre. Sous macOS, les mesures
/// physiques sont les points multipliés par cette échelle : chaque fenêtre
/// retrouve ainsi le repère commun en points, celui de `screenX/screenY`.
fn logical_rect(pos: PhysicalPosition<i32>, size: PhysicalSize<u32>, scale: f64) -> Rect {
    let p = pos.to_logical::<f64>(scale);
    let s = size.to_logical::<f64>(scale);
    Rect { x: p.x, y: p.y, w: s.width, h: s.height }
}

/// Curseur en points, d'après sa position physique.
///
/// Sous macOS, tao l'exprime dans le repère des fenêtres (retourné par la
/// hauteur de l'écran principal) et en pixels de l'écran principal : d'où
/// `primary_scale`. Jamais `screenX/screenY` de la page : WKWebView ne les
/// donne pas dans ce repère (écrans au-dessus du principal).
fn cursor_points(pos: PhysicalPosition<f64>, primary_scale: f64) -> (f64, f64) {
    let p = pos.to_logical::<f64>(primary_scale);
    (p.x, p.y)
}

/// Le point `at` (écran, en points) dans la page de contenu `content`.
fn page_point(at: (f64, f64), content: &Rect) -> (f64, f64) {
    (at.0 - content.x, at.1 - content.y)
}

/// Le curseur, en points ; `None` si l'hôte ne sait pas le lire.
pub fn cursor(app: &AppHandle) -> Option<(f64, f64)> {
    let scale = app.primary_monitor().ok().flatten()?.scale_factor();
    Some(cursor_points(app.cursor_position().ok()?, scale))
}

/// Le point `at` (écran, en points) dans la page de la fenêtre `label`.
pub fn page_point_in(app: &AppHandle, label: &str, at: (f64, f64)) -> Option<(f64, f64)> {
    let w = app.get_webview_window(label)?;
    let content = logical_rect(w.inner_position().ok()?, w.inner_size().ok()?, w.scale_factor().ok()?);
    Some(page_point(at, &content))
}

/// Rectangle logique de chaque fenêtre visible.
pub fn rects(app: &AppHandle) -> Vec<(String, Rect)> {
    app.webview_windows()
        .into_iter()
        .filter(|(_, w)| !w.is_minimized().unwrap_or(false))
        .filter_map(|(label, w)| {
            let rect = logical_rect(w.outer_position().ok()?, w.outer_size().ok()?, w.scale_factor().ok()?);
            Some((label, rect))
        })
        .collect()
}

/// Diffuse une vue à toutes les fenêtres.
fn emit(app: &AppHandle, view: &WorkspaceView) {
    if let Err(e) = app.emit("workspace", view) {
        tracing::warn!(error = %e, "vue de l'espace de travail non diffusée");
    }
}

/// Diffuse la vue courante (un onglet a changé de titre ou d'état).
///
/// Sérialisé avec `sync` : une vue plus ancienne n'arrive jamais après.
pub fn emit_view(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else { return };
    let _serial = state.sync.lock().unwrap_or_else(PoisonError::into_inner);
    let view = Workspace::lock(&state.workspace).view();
    emit(app, &view);
}

/// Met les fenêtres d'accord avec la vue courante, et la renvoie.
///
/// Ouvre celles qui manquent (sous `at`), ferme celles qui n'y sont plus,
/// prévient toutes les fenêtres ; `focus` : la fenêtre active passe au
/// premier plan. Sérialisé et toujours sur la vue la plus récente : deux
/// commandes simultanées ne s'appliquent jamais dans le désordre. Plus
/// aucune fenêtre (dernier onglet fermé) : fin propre (`state::shutdown`),
/// un seul chemin de sortie avec `commands::quit`, la dernière fenêtre
/// (`commands::workspace`) et sa fermeture native (`commands::close_requested`).
pub fn sync(app: &AppHandle, at: Option<LogicalPosition<f64>>, focus: bool) -> WorkspaceView {
    let Some(state) = app.try_state::<AppState>() else {
        return WorkspaceView { generation: 0, focused: String::new(), windows: Vec::new() };
    };
    let _serial = state.sync.lock().unwrap_or_else(PoisonError::into_inner);
    let view = Workspace::lock(&state.workspace).view();
    for w in &view.windows {
        if app.get_webview_window(&w.label).is_none()
            && let Err(e) = open(app, &w.label, at)
        {
            tracing::warn!(label = %w.label, error = %e, "fenêtre non ouverte");
        }
    }
    for (label, w) in app.webview_windows() {
        if !view.windows.iter().any(|v| v.label == label)
            && let Err(e) = w.destroy()
        {
            tracing::warn!(label, error = %e, "fenêtre non fermée");
        }
    }
    if focus && let Some(w) = app.get_webview_window(&view.focused) {
        let shown = w.unminimize().and_then(|()| w.show()).and_then(|()| w.set_focus());
        if let Err(e) = shown {
            tracing::warn!(label = %view.focused, error = %e, "fenêtre non mise au premier plan");
        }
    }
    emit(app, &view);
    if view.windows.is_empty() {
        state::shutdown(app);
    }
    view
}

#[cfg(test)]
#[path = "windows_tests.rs"]
mod windows_tests;
