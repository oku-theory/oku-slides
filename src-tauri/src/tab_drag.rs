//! Glisser un onglet d'une fenêtre à l'autre : survol, puis lâcher.
//!
//! La page d'origine prévient l'hôte tant que l'onglet est hors de sa
//! fenêtre (`drag_tab_over`), quand il y revient (`drag_tab_leave`) et au
//! lâcher (`drop_tab`). L'hôte lit lui-même le curseur (`windows::cursor`),
//! dans le repère des fenêtres, et prévient la fenêtre visée (curseur
//! illisible, sous Wayland : aucune, et le lâcher ouvre une nouvelle fenêtre) :
//! `tab-drag` (`{tab, x, y}` dans sa page, ou `null` quand le curseur s'en
//! va) pour le fantôme et la place de dépôt. Au lâcher, l'hôte déplace
//! l'onglet (fin de barre, ou nouvelle fenêtre sous le curseur), puis la
//! page d'arrivée le range à la place visée, d'après ses propres onglets
//! (`tab-drop`, puis `reorder`).

use std::sync::{Mutex, PoisonError};

use oku_slides_session::{Rect, Workspace, WorkspaceRequest};
use serde_json::json;
use tauri::ipc::Response;
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::blocking;
use crate::state::{AppState, CLOSING};
use crate::windows;

/// Fenêtre survolée en dernier : prévenue quand le curseur la quitte.
#[derive(Debug, Default)]
pub struct Hover(Mutex<Option<String>>);

/// Où vise l'onglet `tab` : le curseur, et la fenêtre dessous (hors la sienne).
struct Aim {
    /// `None` : curseur illisible (Wayland), aucune fenêtre visée.
    at: Option<(f64, f64)>,
    /// Les autres fenêtres, pour le journal.
    others: Vec<(String, Rect)>,
    window: Option<String>,
}

fn aim(app: &AppHandle, state: &AppState, tab: &str) -> Aim {
    let Some(at) = windows::cursor(app) else {
        return Aim { at: None, others: Vec::new(), window: None };
    };
    // Avant le verrou : chaque mesure passe par le fil principal.
    let rects = windows::rects(app);
    let ws = Workspace::lock(&state.workspace);
    let from = ws.tab_window(tab);
    let others: Vec<_> = rects.into_iter().filter(|(l, _)| Some(l.as_str()) != from).collect();
    let window = ws.window_at(at.0, at.1, &others);
    Aim { at: Some(at), others, window }
}

/// La fenêtre survolée devient `next` ; l'ancienne efface son fantôme.
fn hover(app: &AppHandle, next: Option<(String, serde_json::Value)>) {
    let Some(state) = app.try_state::<Hover>() else { return };
    let mut current = state.0.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(prev) = current.as_deref()
        && next.as_ref().is_none_or(|(l, _)| l != prev)
        && let Err(e) = app.emit_to(prev, "tab-drag", serde_json::Value::Null)
    {
        tracing::debug!(label = prev, error = %e, "fin de survol non transmise");
    }
    if let Some((label, payload)) = &next
        && let Err(e) = app.emit_to(label.as_str(), "tab-drag", payload)
    {
        tracing::debug!(label, error = %e, "survol non transmis");
    }
    *current = next.map(|(l, _)| l);
}

/// L'onglet `tab`, hors de sa fenêtre, est sous le curseur.
#[tauri::command]
pub async fn drag_tab_over(app: AppHandle, tab: String) -> Result<(), String> {
    blocking(&app, move |app, state| {
        let aim = aim(app, state, &tab);
        let next = aim.window.zip(aim.at).and_then(|(w, at)| {
            let (x, y) = windows::page_point_in(app, &w, at)?;
            Some((w, json!({ "tab": tab, "x": x, "y": y })))
        });
        hover(app, next);
        Ok(())
    })
    .await
}

/// L'onglet est revenu dans sa fenêtre, ou le glisser est annulé.
#[tauri::command]
pub async fn drag_tab_leave(app: AppHandle) -> Result<(), String> {
    blocking(&app, |app, _| {
        hover(app, None);
        Ok(())
    })
    .await
}

/// Onglet lâché hors de sa fenêtre, au curseur.
///
/// L'hôte le déplace lui-même, sans dépendre d'aucune page : sur une autre
/// fenêtre, en fin de sa barre ; ailleurs, dans une nouvelle fenêtre sous le
/// curseur (l'écran visé), ou n'importe où si le curseur ne se lit pas
/// (Wayland). La fenêtre d'arrivée est ensuite prévenue
/// (`tab-drop`) pour le ranger à la place visée : au pire, il reste en fin
/// de barre, et une erreur de rangement s'affiche dans sa page.
#[tauri::command]
pub async fn drop_tab(app: AppHandle, tab: String) -> Result<Response, String> {
    let reply = blocking(&app, move |app, state| {
        if state.closing.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(CLOSING.to_owned());
        }
        hover(app, None);
        let aim = aim(app, state, &tab);
        tracing::debug!(tab, at = ?aim.at, rects = ?aim.others, window = ?aim.window, "onglet lâché hors de sa fenêtre");
        let target = aim.window.clone();
        Workspace::apply(&state.workspace, WorkspaceRequest::MoveTab { tab: tab.clone(), window: aim.window, index: None })
            .map_err(|e| state.refuse(e))?;
        let view = windows::sync(app, aim.at, true);
        if let (Some(w), Some(at)) = (target, aim.at) {
            match windows::page_point_in(app, &w, at) {
                Some((x, y)) => {
                    if let Err(e) = app.emit_to(w.as_str(), "tab-drop", json!({ "tab": tab, "x": x, "y": y })) {
                        tracing::warn!(label = %w, error = %e, "place visée non transmise : onglet en fin de barre");
                    }
                }
                None => tracing::warn!(label = %w, "fenêtre visée sans mesure : onglet en fin de barre"),
            }
        }
        serde_json::to_string(&view).map_err(|e| e.to_string())
    })
    .await?;
    Ok(Response::new(reply))
}
