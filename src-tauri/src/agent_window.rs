//! La fenêtre de l'agent détaché : une seule pour l'app, label `agent`.
//!
//! Elle charge la même page que les fenêtres de l'espace de travail ; l'interface s'y
//! reconnaît à son label. Ouverte ou fermée par `windows::sync` d'après
//! `WorkspaceView::agent_detached` ; sa géométrie est gardée par `tauri-plugin-window-state`,
//! sous ce label. Ce n'est pas une fenêtre de l'espace de travail : son focus ne change pas la
//! fenêtre active (`commands::focused`), et sa fermeture système passe par sa page
//! (`close-requested`), qui vide la frappe du brouillon puis rattache l'agent.

use std::sync::atomic::Ordering;

use oku_slides_session::{Workspace, WorkspaceRequest};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::commands::blocking;
use crate::menu;
use crate::state::AppState;
use crate::windows;

/// Label de la fenêtre de l'agent (aussi lu par l'interface).
pub const LABEL: &str = "agent";

/// Taille d'une fenêtre neuve, en points.
const SIZE: (f64, f64) = (420.0, 800.0);
/// Taille minimale, en points : le fil et la barre de dictée y tiennent.
const MIN_SIZE: (f64, f64) = (320.0, 400.0);

/// Ce que `reconcile` fait de la fenêtre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Agent détaché, fenêtre absente : l'ouvrir.
    Open,
    /// Agent rattaché, fenêtre ouverte : la fermer.
    Close,
    /// Déjà d'accord.
    Keep,
}

/// Que faire, l'agent étant détaché ou non et la fenêtre ouverte ou non.
pub fn step(detached: bool, open: bool) -> Step {
    match (detached, open) {
        (true, false) => Step::Open,
        (false, true) => Step::Close,
        _ => Step::Keep,
    }
}

fn open(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let title = match app.try_state::<AppState>() {
        Some(state) => state.localizer.tr("window-agent-title"),
        None => {
            tracing::debug!("état de l'app absent : fenêtre de l'agent sans titre");
            String::new()
        }
    };
    let builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
        .title(title)
        .inner_size(SIZE.0, SIZE.1)
        .min_inner_size(MIN_SIZE.0, MIN_SIZE.1);
    // Comme les fenêtres de l'espace de travail : la page pose sa barre de saisie (`.titlebar`).
    windows::overlay_title_bar(builder).build()
}

/// Met la fenêtre de l'agent d'accord avec `detached`.
pub fn reconcile(app: &AppHandle, detached: bool) {
    let window = app.get_webview_window(LABEL);
    match step(detached, window.is_some()) {
        Step::Open => {
            if let Err(e) = open(app) {
                tracing::warn!(error = %e, "fenêtre de l'agent non ouverte");
            }
        }
        Step::Close => {
            if let Some(w) = window
                && let Err(e) = w.destroy()
            {
                tracing::warn!(error = %e, "fenêtre de l'agent non fermée");
            }
        }
        Step::Keep => {}
    }
}

/// Détache l'agent s'il est rattaché, et inversement (menu), hors du fil principal.
pub fn toggle(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let synced = blocking(&app, |app, state| {
            // macOS a déjà basculé la case : sans `sync`, on la remet d'après l'espace de travail.
            if state.closing.load(Ordering::SeqCst) {
                menu::set_agent_checked(app, Workspace::lock(&state.workspace).agent_detached());
                return Ok(());
            }
            // Lu puis appliqué sous deux verrous : deux clics très rapprochés peuvent faire la même
            // bascule ; sans danger, `sync` remet la case d'accord avec l'espace de travail.
            let detached = Workspace::lock(&state.workspace).agent_detached();
            let req = if detached { WorkspaceRequest::AttachAgent } else { WorkspaceRequest::DetachAgent };
            if let Err(e) = Workspace::apply(&state.workspace, req) {
                menu::set_agent_checked(app, detached);
                return Err(state.refuse(e));
            }
            windows::sync(app, None, false);
            Ok(())
        })
        .await;
        if let Err(e) = synced {
            tracing::warn!(error = %e, "agent non détaché ou rattaché");
        }
    });
}

#[cfg(test)]
#[path = "agent_window_tests.rs"]
mod agent_window_tests;
