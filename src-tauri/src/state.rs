//! État de l'app et fin propre : `commands` et `windows` en dépendent tous
//! les deux (jamais l'inverse), pour ne pas boucler entre les deux modules.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};

use oku_slides_agent::Stack;
use oku_slides_session::agent::{Sessions, TabGate, TabOpen};
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
    /// (voir `Stack`) ; `shutdown` arrête les agents (`Stack::shutdown`).
    pub stack: Stack,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState").finish_non_exhaustive()
    }
}

impl AppState {
    /// Une erreur dans la langue de l'utilisateur ; journalisée en anglais.
    pub fn refuse(&self, e: &SessionError) -> String {
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

    /// Lance le fine-tuning par l'agent de l'onglet importé `tab`.
    pub fn tune(&self, tab: &str) {
        self.stack.tune(tab);
    }
}

/// The tabs' sessions, for the agent, through the workspace ([`Workspace::with_session`]).
///
/// The tab bar is redrawn in every window when a tool changed it (title,
/// modified), and the windows follow a file the agent opens beside its tab
/// (`open_tab`) and a pack or theme tab it opens or focuses.
pub struct AppSessions {
    app: AppHandle,
    workspace: Arc<Mutex<Workspace>>,
}

impl AppSessions {
    pub fn new(app: AppHandle, workspace: Arc<Mutex<Workspace>>) -> Self {
        Self { app, workspace }
    }

    /// The windows follow an agent's open: a shown tab comes forward, a tab opened here re-reads its view.
    fn follow(&self, opened: &TabOpen) {
        match opened {
            TabOpen::Opened { shown, .. } | TabOpen::Focused { shown, .. } => {
                crate::windows::sync(&self.app, None, *shown);
            }
            TabOpen::Here => crate::windows::emit_view(&self.app),
            TabOpen::Refused => {}
        }
    }
}

impl std::fmt::Debug for AppSessions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppSessions").finish_non_exhaustive()
    }
}

impl Sessions for AppSessions {
    /// A deck-pack forward that changed owner decks: the view is emitted, and
    /// each owner's `after_change` deferred to a blocking task, never inline
    /// (the stack calls this method).
    fn with(&self, tab: &str, f: &mut dyn FnMut(&mut Session)) -> oku_slides_session::Result<()> {
        let (done, changed) = Workspace::with_session(&self.workspace, tab, |s| f(s));
        if changed {
            crate::windows::emit_view(&self.app);
        }
        let mut owners = Vec::new();
        Workspace::drain_touched(
            &self.workspace,
            || crate::windows::emit_view(&self.app),
            |o| owners.push(o.to_owned()),
        );
        if !owners.is_empty() {
            let app = self.app.clone();
            tauri::async_runtime::spawn_blocking(move || {
                let state = app.state::<AppState>();
                for owner in &owners {
                    state.stack.after_change(owner);
                }
            });
        }
        done
    }

    fn open_elsewhere(&self, tab: &str, path: &std::path::Path) -> bool {
        Workspace::lock(&self.workspace).open_elsewhere(tab, path)
    }

    fn shown(&self, tab: &str) -> bool {
        Workspace::lock(&self.workspace).tab_shown(tab)
    }

    /// The file in a new tab of `tab`'s window; the windows follow and the
    /// opened tab comes forward (off the main thread: a tool).
    fn open_tab(&self, tab: &str, path: &std::path::Path) -> oku_slides_session::Result<String> {
        let opened = Workspace::open_beside(&self.workspace, tab, path)?;
        crate::windows::sync(&self.app, None, true);
        Ok(opened)
    }

    /// Opens a pack for the agent; the windows follow a pack tab, the view one opened here.
    fn open_pack(
        &self,
        tab: &str,
        pack: uuid::Uuid,
        widget: Option<&str>,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> oku_slides_session::Result<TabOpen> {
        let opened = Workspace::open_pack(&self.workspace, tab, pack, widget, gate)?;
        self.follow(&opened);
        Ok(opened)
    }

    /// Opens a theme for the agent; the windows follow a theme tab, the view one opened here.
    fn open_theme(
        &self,
        tab: &str,
        theme: uuid::Uuid,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> oku_slides_session::Result<TabOpen> {
        let opened = Workspace::open_theme_for_agent(&self.workspace, tab, theme, gate)?;
        self.follow(&opened);
        Ok(opened)
    }

    /// Read from the workspace's cached tab: wakes nothing.
    fn document_kind(&self, tab: &str) -> oku_slides_session::Result<oku_slides_session::DocumentKind> {
        Workspace::document_kind(&self.workspace, tab)
    }
}

/// Prépare la fin sans quitter : marque `closing`, garde les fenêtres, arrête
/// les agents et enregistre l'espace de travail.
///
/// Jamais sur le fil principal : `Workspace::settle` attend un verrou qui peut
/// dépendre d'une opération de fenêtre dispatchée sur ce fil. Appelée par
/// `shutdown` et par `updates::install` (avant d'installer) ; rappelée sans
/// danger, `RunEvent::Exit` rappelant aussi `Workspace::settle`.
pub fn settle(app: &AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        state.closing.store(true, Ordering::SeqCst);
        // Les fenêtres existent encore : taille et position gardées sans attendre.
        crate::window_state::save(app);
        // Hors d'un contexte async : `block_on` y est permis.
        tauri::async_runtime::block_on(state.stack.shutdown());
        Workspace::settle(&state.workspace);
    }
}

/// Termine proprement : marque la fin (`closing`), arrête les agents (le
/// sous-processus ACP est tué), attend les requêtes en cours, enregistre
/// l'espace de travail ([`settle`]), puis quitte.
///
/// Jamais sur le fil principal (voir [`settle`]), sauf le filet de sécurité
/// de `main.rs` (`RunEvent::Exit`). Appelée depuis `commands::quit`, depuis le menu « Quitter »
/// (`main.rs::on_menu_event`), depuis la dernière fenêtre
/// (`commands::workspace` et `commands::close_requested`, chacune hors du
/// fil principal), et depuis `windows::sync` quand la vue est vide (déjà
/// hors du fil principal, dans `commands::blocking`).
pub fn shutdown(app: &AppHandle) {
    settle(app);
    app.exit(0);
}
