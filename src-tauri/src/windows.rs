//! Fenêtres de l'app : une par fenêtre de l'espace de travail, même label.
//!
//! Taille et position : `tauri-plugin-window-state`, par label. Curseur et
//! rectangles des fenêtres sont comparés dans le repère de l'hôte (voir
//! `POINTS`) : les points sous macOS, les pixels physiques du bureau
//! ailleurs. `sync`, `rects` et `emit_view` attendent un verrou (et, pour
//! `sync`, la fabrique ou la destruction d'une fenêtre) : jamais sur le fil
//! principal, toujours depuis `commands::blocking` (voir ce module).

use std::sync::PoisonError;

use oku_slides_session::{Rect, Workspace, WorkspaceView};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

use crate::state::{self, AppState};

/// Taille d'une fenêtre neuve, et taille minimale.
const SIZE: (f64, f64) = (1440.0, 900.0);
const MIN_SIZE: (f64, f64) = (1024.0, 640.0);
/// Décalage du coin de la fenêtre au pointeur, en points : l'onglet lâché
/// tombe dans sa barre, en bas de la fenêtre (36 points de haut), sur le
/// premier onglet.
const DROP_OFFSET: (f64, f64) = (80.0, SIZE.1 - 18.0);

/// Repère de l'hôte : les points (macOS), sinon les pixels physiques.
///
/// macOS : tao donne des mesures physiques propres à l'échelle de chaque
/// écran, seuls les points forment un repère commun. Windows : curseur
/// (`GetCursorPos`) et fenêtres (`GetWindowRect`) sont en pixels physiques
/// du bureau, un repère commun même entre écrans de densités différentes ;
/// les ramener en points avec l'échelle de chacun les décalerait. X11 : tao
/// multiplie les coordonnées GDK par l'échelle GDK, unique : curseur et
/// fenêtres restent dans un même repère. Une constante plutôt qu'un `cfg` :
/// les fonctions qui en dépendent la reçoivent en paramètre, et les tests
/// passent par les deux branches sur tous les systèmes.
const POINTS: bool = cfg!(target_os = "macos");

/// Coin de la fenêtre neuve, en points : le point lâché, décalé par `DROP_OFFSET`.
fn drop_corner(at: (f64, f64)) -> (f64, f64) {
    (at.0 - DROP_OFFSET.0, at.1 - DROP_OFFSET.1)
}

/// Coin de la fenêtre neuve, en pixels physiques, d'après ses mesures réelles.
///
/// `scale` : celle de la fenêtre ; `outer_height` : sa hauteur, décorations
/// comprises (barre de titre de Windows, par exemple). Le pointeur tombe à
/// mi-hauteur de la barre d'onglets, en bas de la fenêtre, sur le premier
/// onglet.
fn drop_corner_physical(at: (f64, f64), scale: f64, outer_height: f64) -> (f64, f64) {
    let tab_bar_middle = (SIZE.1 - DROP_OFFSET.1) * scale;
    (at.0 - DROP_OFFSET.0 * scale, at.1 - outer_height + tab_bar_middle)
}

/// Le curseur lu par tao est-il réel ?
///
/// Sous Wayland, tao renvoie `(0, 0)` faute de pouvoir le lire (ni la
/// position des fenêtres) : le prendre au mot viserait une fenêtre au
/// hasard. Sous Linux, `(0, 0)` compte donc pour illisible, et le lâcher se
/// replie sur une nouvelle fenêtre ; un vrai curseur pile au coin de l'écran
/// sous X11 subit le même repli, sans dommage.
fn cursor_readable(pos: PhysicalPosition<f64>, linux: bool) -> bool {
    !(linux && pos.x == 0.0 && pos.y == 0.0)
}

/// Ouvre la fenêtre `label`, sous le pointeur `at` (repère de l'hôte) s'il est donné.
fn open(app: &AppHandle, label: &str, at: Option<(f64, f64)>) -> tauri::Result<WebviewWindow> {
    let mut builder = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title("Oku Slides")
        .inner_size(SIZE.0, SIZE.1)
        .min_inner_size(MIN_SIZE.0, MIN_SIZE.1);
    // Hors macOS, la position est physique : posée une fois la fenêtre
    // créée (le constructeur n'en prend qu'une logique), avant de la montrer.
    let physical = at.filter(|_| !POINTS);
    if let Some(p) = at.filter(|_| POINTS) {
        let (x, y) = drop_corner(p);
        builder = builder.position(x, y);
    }
    if physical.is_some() {
        builder = builder.visible(false);
    }
    #[cfg(target_os = "macos")]
    let builder = builder.title_bar_style(tauri::TitleBarStyle::Overlay).hidden_title(true);
    let window = builder.build()?;
    if let Some(p) = physical {
        place_under(&window, p);
        // Une fenêtre restée cachée réapparaît au prochain `sync` qui la met devant.
        if let Err(e) = window.show() {
            tracing::warn!(label, error = %e, "fenêtre neuve non montrée");
        }
    }
    Ok(window)
}

/// Place une fenêtre cachée sous le pointeur `at` (pixels physiques).
///
/// D'abord sur l'écran visé : sa taille logique y est réappliquée, au cas où
/// le changement d'échelle (Windows, `WM_DPICHANGED`) ne suivrait pas une
/// fenêtre cachée. Puis au bon endroit, d'après son échelle et sa hauteur
/// réelles sur cet écran.
fn place_under(window: &WebviewWindow, at: (f64, f64)) {
    let placed = window
        .set_position(PhysicalPosition::new(at.0, at.1))
        .and_then(|()| window.set_size(tauri::LogicalSize::new(SIZE.0, SIZE.1)))
        .and_then(|()| Ok((window.scale_factor()?, window.outer_size()?)))
        .and_then(|(scale, outer)| {
            let (x, y) = drop_corner_physical(at, scale, f64::from(outer.height));
            window.set_position(PhysicalPosition::new(x, y))
        });
    if let Err(e) = placed {
        tracing::warn!(label = window.label(), error = %e, "fenêtre neuve non placée sous le pointeur");
    }
}

/// Rectangle logique d'une fenêtre, d'après ses mesures physiques.
///
/// `scale` : celle de l'écran de la fenêtre. Sous macOS, les mesures
/// physiques sont les points multipliés par cette échelle : chaque fenêtre
/// retrouve ainsi le repère commun en points.
fn logical_rect(pos: PhysicalPosition<i32>, size: PhysicalSize<u32>, scale: f64) -> Rect {
    let p = pos.to_logical::<f64>(scale);
    let s = size.to_logical::<f64>(scale);
    Rect { x: p.x, y: p.y, w: s.width, h: s.height }
}

/// Rectangle physique d'une fenêtre, tel quel (Windows, X11).
fn physical_rect(pos: PhysicalPosition<i32>, size: PhysicalSize<u32>) -> Rect {
    Rect { x: f64::from(pos.x), y: f64::from(pos.y), w: f64::from(size.width), h: f64::from(size.height) }
}

/// Rectangle d'une fenêtre dans le repère de l'hôte ; `points` : voir `POINTS`.
fn host_rect(points: bool, pos: PhysicalPosition<i32>, size: PhysicalSize<u32>, scale: f64) -> Rect {
    if points { logical_rect(pos, size, scale) } else { physical_rect(pos, size) }
}

/// Curseur en points, d'après sa position physique (macOS).
///
/// Sous macOS, tao l'exprime dans le repère des fenêtres (retourné par la
/// hauteur de l'écran principal) et en pixels de l'écran principal : d'où
/// `primary_scale`. Jamais `screenX/screenY` de la page : WKWebView ne les
/// donne pas dans ce repère (écrans au-dessus du principal).
fn cursor_points(pos: PhysicalPosition<f64>, primary_scale: f64) -> (f64, f64) {
    let p = pos.to_logical::<f64>(primary_scale);
    (p.x, p.y)
}

/// Le point `at` (écran, en points) dans la page de contenu `content` (macOS).
fn page_point(at: (f64, f64), content: &Rect) -> (f64, f64) {
    (at.0 - content.x, at.1 - content.y)
}

/// Le point `at` (pixels physiques) dans la page `content`, en pixels CSS.
fn page_point_physical(at: (f64, f64), content: &Rect, scale: f64) -> (f64, f64) {
    ((at.0 - content.x) / scale, (at.1 - content.y) / scale)
}

/// Le point `at` dans la page `content`, tous deux dans le repère de l'hôte.
fn page_point_host(points: bool, at: (f64, f64), content: &Rect, scale: f64) -> (f64, f64) {
    if points { page_point(at, content) } else { page_point_physical(at, content, scale) }
}

/// Le curseur, dans le repère de l'hôte ; `None` s'il ne se lit pas (Wayland).
pub fn cursor(app: &AppHandle) -> Option<(f64, f64)> {
    let pos = match app.cursor_position() {
        Ok(pos) => pos,
        Err(e) => {
            tracing::warn!(error = %e, "curseur illisible : le lâcher ouvrira une nouvelle fenêtre");
            return None;
        }
    };
    if !cursor_readable(pos, cfg!(target_os = "linux")) {
        tracing::debug!("curseur en (0, 0) sous Linux (Wayland) : ignoré");
        return None;
    }
    if !POINTS {
        return Some((pos.x, pos.y));
    }
    let Some(scale) = app.primary_monitor().ok().flatten().map(|m| m.scale_factor()) else {
        tracing::warn!("écran principal introuvable : curseur ignoré");
        return None;
    };
    Some(cursor_points(pos, scale))
}

/// Le point `at` (repère de l'hôte) dans la page de la fenêtre `label`.
pub fn page_point_in(app: &AppHandle, label: &str, at: (f64, f64)) -> Option<(f64, f64)> {
    let w = app.get_webview_window(label)?;
    let scale = w.scale_factor().ok()?;
    let content = host_rect(POINTS, w.inner_position().ok()?, w.inner_size().ok()?, scale);
    Some(page_point_host(POINTS, at, &content, scale))
}

/// Rectangle de chaque fenêtre visible, dans le repère de l'hôte.
pub fn rects(app: &AppHandle) -> Vec<(String, Rect)> {
    app.webview_windows()
        .into_iter()
        .filter(|(_, w)| !w.is_minimized().unwrap_or(false))
        .filter_map(|(label, w)| {
            let rect = host_rect(POINTS, w.outer_position().ok()?, w.outer_size().ok()?, w.scale_factor().ok()?);
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
/// Ouvre celles qui manquent (sous `at`, repère de l'hôte), ferme celles qui n'y sont plus,
/// prévient toutes les fenêtres ; `focus` : la fenêtre active passe au
/// premier plan. Sérialisé et toujours sur la vue la plus récente : deux
/// commandes simultanées ne s'appliquent jamais dans le désordre. Plus
/// aucune fenêtre (dernier onglet fermé) : fin propre (`state::shutdown`),
/// un seul chemin de sortie avec `commands::quit`, la dernière fenêtre
/// (`commands::workspace`) et sa fermeture native (`commands::close_requested`).
pub fn sync(app: &AppHandle, at: Option<(f64, f64)>, focus: bool) -> WorkspaceView {
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
