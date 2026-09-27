//! État de l'app et fin propre : `commands` et `windows` en dépendent tous
//! les deux (jamais l'inverse), pour ne pas boucler entre les deux modules.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};

use oku_slides_agent::Stack;
use oku_slides_session::agent::Sessions;
use oku_slides_session::{Localize, Localizer, Session, SessionError, Workspace};
use tauri::{AppHandle, Manager};

/// Réponse des commandes une fois la fin engagée (`AppState::closing`) : pas
/// besoin d'une traduction, l'app quitte déjà.
pub const CLOSING: &str = "app is closing";

/// État de l'app : l'espace de travail et ce qu'il faut autour.
pub struct AppState {
    /// Fenêtres, onglets, et une session par onglet ; partagé avec l'agent
    /// (`AppSessions`).
    pub workspace: Arc<Mutex<Workspace>>,
    /// Langue des erreurs renvoyées à l'interface.
    pub localizer: Arc<Localizer>,
    /// Sérialise `windows::sync`.
    pub sync: Mutex<()>,
    /// Prochain numéro d'événement de focus, pris sur le fil principal (voir
    /// `commands::focused`) : l'ordre d'arrivée fait foi, même si les tâches
    /// qui l'appliquent se terminent dans le désordre.
    pub focus_seq: AtomicU64,
    /// Dernier numéro de focus appliqué, comparé et mis à jour sous le
    /// verrou de l'espace de travail (`commands::focused`).
    pub focus_applied: Mutex<u64>,
    /// Fin engagée (`state::shutdown` l'a posé) : `request`, `workspace` et
    /// `tab_drag::drop_tab` refusent tout, sans plus toucher à l'espace de travail.
    pub closing: AtomicBool,
    /// L'agent de chaque onglet (outils MCP et agent ACP).
    ///
    /// Vit jusqu'à la fin du processus : `AppState` → `Stack` → `AppSessions`
    /// (et le rappel de `forward`) → `AppHandle` → `AppState` forment un
    /// cycle, jamais lâché. Aucun risque de le lâcher dans un contexte async
    /// (voir `Stack`) ; l'arrêt des agents repose sur la fin du processus.
    pub stack: Stack,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState").finish_non_exhaustive()
    }
}

impl AppState {
    /// Une erreur dans la langue de l'utilisateur ; journalisée en anglais.
    pub fn refuse(&self, e: SessionError) -> String {
        tracing::warn!(error = %e.one_line(), "requête refusée");
        e.localize(&self.localizer)
    }

    /// L'espace de travail s'il est libre (fil principal : jamais d'attente).
    ///
    /// Un verrou empoisonné est repris, comme partout.
    pub fn try_workspace(&self) -> Option<MutexGuard<'_, Workspace>> {
        match self.workspace.try_lock() {
            Ok(ws) => Some(ws),
            Err(TryLockError::Poisoned(p)) => Some(p.into_inner()),
            Err(TryLockError::WouldBlock) => None,
        }
    }

    /// Arrête l'agent de chaque onglet de `closed` (fermé, ou onglet vierge
    /// remplacé par un fichier : voir [`Workspace::closed_by`]).
    pub fn stop_closed_agents(&self, closed: &[String]) {
        for gone in closed {
            self.stack.tab_closed(gone);
        }
    }
}

/// Les sessions des onglets, pour l'agent : par l'espace de travail
/// ([`Workspace::with_session`]) ; la barre d'onglets est redessinée dans
/// toutes les fenêtres quand un outil l'a changée (titre, modifié).
pub struct AppSessions {
    app: AppHandle,
    workspace: Arc<Mutex<Workspace>>,
}

impl AppSessions {
    pub fn new(app: AppHandle, workspace: Arc<Mutex<Workspace>>) -> Self {
        Self { app, workspace }
    }
}

impl std::fmt::Debug for AppSessions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppSessions").finish_non_exhaustive()
    }
}

impl Sessions for AppSessions {
    fn with(&self, tab: &str, f: &mut dyn FnMut(&mut Session)) -> oku_slides_session::Result<()> {
        let (done, changed) = Workspace::with_session(&self.workspace, tab, |s| f(s));
        if changed {
            crate::windows::emit_view(&self.app);
        }
        done
    }

    fn open_elsewhere(&self, tab: &str, path: &std::path::Path) -> bool {
        Workspace::lock(&self.workspace).open_elsewhere(tab, path)
    }
}

/// Termine proprement : marque la fin (`closing`), attend les requêtes en
/// cours, enregistre l'espace de travail, puis quitte.
///
/// Jamais sur le fil principal, sauf le filet de sécurité documenté dans
/// `main.rs` (`RunEvent::Exit`, où plus aucun fil ne peut détenir ces
/// verrous) : ailleurs, `Workspace::settle` attend un verrou qui peut
/// dépendre d'une opération de fenêtre elle-même dispatchée sur ce fil (une
/// fermeture de fenêtre en cours, par exemple) — l'y appeler bloquerait les
/// deux à jamais. Appelée depuis `commands::quit`, depuis le menu « Quitter »
/// (`main.rs::on_menu_event`), depuis la dernière fenêtre
/// (`commands::workspace` et `commands::close_requested`, chacune hors du
/// fil principal), et depuis `windows::sync` quand la vue est vide (déjà
/// hors du fil principal, dans `commands::blocking`).
pub fn shutdown(app: &AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        state.closing.store(true, Ordering::SeqCst);
        // Les fenêtres existent encore : taille et position gardées sans attendre.
        crate::window_state::save(app);
        Workspace::settle(&state.workspace);
    }
    app.exit(0);
}
