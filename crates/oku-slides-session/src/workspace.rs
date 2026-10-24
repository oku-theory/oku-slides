//! Espace de travail : les fenêtres, leurs onglets, et une session par onglet.
//!
//! Chaque fenêtre a ses onglets, dans l'ordre de sa barre, et un onglet
//! actif ; un onglet n'est que dans une fenêtre. Les sessions sont partagées
//! (`Arc<Mutex<_>>`) : une requête d'onglet ne verrouille que la sienne.
//! Reprise, un onglet dort (`workspace_slot.rs`) jusqu'à sa première
//! requête : le lancement ne relit aucune session, la barre vient de
//! `workspace.json`.
//!
//! Ordre des verrous : l'espace de travail, puis une session, jamais
//! l'inverse. Seules [`Workspace::apply`], [`Workspace::request`] (par
//! [`Workspace::with_session`], que l'agent emprunte aussi),
//! [`Workspace::open_files`], [`Workspace::settle`] et `post_notice`
//! combinent les deux, en séquence, jamais imbriqués ; une session neuve se
//! construit hors du verrou de l'espace de travail.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use snafu::ResultExt;

use oku_slides_platform::storage::resolve;

use crate::error::{ResponseSnafu, Result, UnknownTabSnafu, UnknownWindowSnafu, UnsavedSnafu};
use crate::{Fonts, Platform, Session};

/// Sous-dossier des onglets, dans le dossier de reprise.
const TABS_DIR: &str = "tabs";

/// Les fenêtres, leurs onglets et une session par onglet.
pub struct Workspace {
    fonts: Fonts,
    platform: Platform,
    layout: Layout,
    /// Fenêtres, de la moins à la plus récemment active.
    recent: Vec<String>,
    tabs: HashMap<String, Tab>,
    /// Dossier de reprise ; `None` : rien n'est écrit (serveur de dev, tests).
    dir: Option<PathBuf>,
    /// Posé par [`Workspace::settle`] : plus aucun fichier ne s'ouvre ensuite
    /// (voir [`Workspace::open_files`] et `apply` sur `OpenTab`), sous ce même
    /// verrou, pour ne jamais courir après `settle`. Partagé : relu aussi par
    /// [`Workspace::with_session`] sous le verrou d'une session, sans celui-ci.
    closed: Arc<AtomicBool>,
    /// Numéro de la dernière vue construite (voir [`WorkspaceView::generation`]).
    generation: AtomicU64,
    /// Langue choisie dans les réglages ; `None` : celle du système.
    chosen_locale: Option<&'static str>,
    /// Schéma choisi dans les réglages ; `None` : celui du système.
    chosen_scheme: Option<&'static str>,
    /// Le dernier `.pptx` reçu de l'OS (voir [`ImportPick`]), pas encore montré ; jamais enregistré.
    import_pick: Option<ImportPick>,
    /// Numéro du dernier choix d'import : jamais réutilisé, même une fois le choix oublié.
    last_pick: u64,
}

impl std::fmt::Debug for Workspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Workspace").field("layout", &self.layout).field("dir", &self.dir).finish_non_exhaustive()
    }
}

impl Workspace {
    /// Une fenêtre et un onglet `t-1`, avec `file` ouvert ; rien n'est écrit.
    ///
    /// Sans fichier, l'onglet est vierge (tests, serveur lancé sans fichier).
    pub fn new(fonts: Fonts, platform: &Platform, file: Option<&Path>) -> Result<Self> {
        let mut ws = Self::empty(fonts, platform, None);
        let pending = ws.pending(None, file.map_or(Opening::Blank, |f| Opening::File(resolve(f))), None);
        let session = pending.build()?;
        ws.insert_tab(None, pending.id, session, None);
        Ok(ws)
    }

    fn empty(fonts: Fonts, platform: &Platform, dir: Option<PathBuf>) -> Self {
        Self {
            fonts,
            platform: platform.clone(),
            layout: Layout::new(),
            recent: Vec::new(),
            tabs: HashMap::new(),
            dir,
            closed: Arc::new(AtomicBool::new(false)),
            generation: AtomicU64::new(0),
            chosen_locale: platform.settings.i18n.locale.as_deref().and_then(oku_slides_i18n::shipped),
            chosen_scheme: platform.settings.appearance.scheme.as_deref().and_then(scheme::known),
            import_pick: None,
            last_pick: 0,
        }
    }

    /// Dernière fenêtre active.
    pub fn focused(&self) -> &str {
        &self.layout.focused
    }

    /// `label` passe au premier plan ; rien n'est écrit (le prochain changement le fera).
    pub fn focus(&mut self, label: &str) -> Result<()> {
        self.window_index(label)?;
        self.touch(label);
        Ok(())
    }

    /// Les fenêtres et leurs onglets, avec un numéro neuf.
    ///
    /// Le numéro est pris sous le verrou de l'espace de travail : une vue
    /// construite plus tard reflète un état au moins aussi récent.
    pub fn view(&self) -> WorkspaceView {
        WorkspaceView {
            generation: self.generation.fetch_add(1, Ordering::SeqCst) + 1,
            focused: self.layout.focused.clone(),
            agent_detached: self.layout.agent_detached,
            import_pick: self.import_pick.clone(),
            windows: self
                .layout
                .windows
                .iter()
                .map(|w| WindowView {
                    label: w.label.clone(),
                    active: w.active.clone(),
                    tabs: w
                        .tabs
                        .iter()
                        .filter_map(|id| self.tabs.get(id).map(|t| TabView { id: id.clone(), info: t.info.clone() }))
                        .collect(),
                })
                .collect(),
        }
    }

    /// Les onglets ouverts, toutes fenêtres confondues.
    pub fn tab_ids(&self) -> Vec<String> {
        self.layout.windows.iter().flat_map(|w| w.tabs.iter().cloned()).collect()
    }

    /// `f` sur l'espace de travail, et les onglets qu'elle a fermés (ou
    /// vierges remplacés par un fichier) : l'hôte arrête leur agent.
    pub fn closed_by<R>(ws: &Mutex<Self>, f: impl FnOnce(&Mutex<Self>) -> R) -> (R, Vec<String>) {
        let before = Self::lock(ws).tab_ids();
        let out = f(ws);
        let after = Self::lock(ws).tab_ids();
        (out, before.into_iter().filter(|id| !after.contains(id)).collect())
    }

    /// Traite une requête de structure ; renvoie la vue (ou les messages), en JSON.
    ///
    /// Un onglet neuf est construit hors du verrou : numéro réservé, session
    /// créée et ouverte, puis insérée. Les dossiers des onglets fermés sont
    /// supprimés après l'écriture de `workspace.json`, hors du verrou aussi.
    ///
    /// `CloseWindow` sur la seule fenêtre restante : refusée atomiquement
    /// (sous ce même verrou, voir [`Workspace::is_only_window`]) ; deux
    /// fermetures concurrentes ne peuvent jamais vider l'espace de travail.
    pub fn apply(ws: &Mutex<Self>, req: WorkspaceRequest) -> Result<WorkspaceOutcome> {
        // Lit un fichier : jamais sous le verrou (voir `workspace_same_deck.rs`).
        if let WorkspaceRequest::SameDeck { path } = &req {
            return Self::same_deck(ws, path);
        }
        // Prend deux sessions l'une après l'autre : jamais sous le verrou (voir `workspace_carry.rs`).
        if let WorkspaceRequest::CarryAssets { from, to, element } = &req {
            return Self::carry_assets(ws, from, to, element);
        }
        let waking = Self::lock(ws).to_wake(&req);
        Self::wake(ws, &waking);
        let pending = {
            let mut w = Self::lock(ws);
            match req {
                WorkspaceRequest::View => return w.view_json().map(WorkspaceOutcome::Applied),
                WorkspaceRequest::Messages => return w.messages_json().map(WorkspaceOutcome::Applied),
                WorkspaceRequest::SetLocale { locale } => {
                    w.set_locale(locale.as_deref());
                    return w.messages_json().map(WorkspaceOutcome::Applied);
                }
                WorkspaceRequest::Scheme => return w.scheme_json().map(WorkspaceOutcome::Applied),
                WorkspaceRequest::SetScheme { scheme } => {
                    w.set_scheme(scheme.as_deref());
                    return w.scheme_json().map(WorkspaceOutcome::Applied);
                }
                WorkspaceRequest::NewTab { window } => w.new_tab(window)?,
                WorkspaceRequest::OpenTab { window, path } => match w.open_tab(window, path)? {
                    Step::Done(json) => return Ok(WorkspaceOutcome::Applied(json)),
                    Step::Build(pending) => *pending,
                },
                WorkspaceRequest::ImportTab { window, path, target, tune } => {
                    w.import_tab(window, path, target, tune)?
                }
                WorkspaceRequest::OpenHelp { window } => match Self::open_help(ws, w, window)? {
                    Step::Done(json) => return Ok(WorkspaceOutcome::Applied(json)),
                    Step::Build(pending) => *pending,
                },
                WorkspaceRequest::NewWindow => w.pending(None, Opening::Blank, None),
                WorkspaceRequest::CloseWindow { window, discard } => return Self::close_window(w, &window, &discard),
                WorkspaceRequest::ImportPicked { id } => {
                    w.import_picked(id);
                    return w.view_json().map(WorkspaceOutcome::Applied);
                }
                other => {
                    let gone = w.change(other)?;
                    w.save_layout();
                    let (json, trash) = (w.view_json(), w.trash());
                    drop(w);
                    trash.empty(gone);
                    return json.map(WorkspaceOutcome::Applied);
                }
            }
        };
        let tune = pending.tunes();
        let session = pending.build()?;
        match Self::finish(ws, pending, session)? {
            (WorkspaceOutcome::Applied(json), tab) if tune => Ok(WorkspaceOutcome::Tune { json, tab }),
            (outcome, _) => Ok(outcome),
        }
    }

    /// Attend les requêtes en cours puis enregistre l'état courant ; à
    /// appeler juste avant de quitter, jamais ailleurs.
    ///
    /// `focus` seul n'écrit rien (voir [`Workspace::focus`]) : cette méthode
    /// force l'écriture pour que la fenêtre au premier plan soit reprise même
    /// si rien d'autre n'a changé depuis. Verrouille l'espace de travail (ce
    /// qui attend un `apply` en cours), puis chaque session une fois (ce qui
    /// attend une requête en cours, puis l'écriture de sa reprise par son
    /// fil), jamais en même temps que celui de
    /// l'espace de travail. Pose aussi `closed`, sous ce verrou : plus aucun
    /// fichier reçu ensuite (`open_files`, `apply` sur `OpenTab`) ne s'ouvre,
    /// et un onglet dont la construction chevauche cet appel n'est pas inséré ;
    /// plus aucune session ne change ensuite ([`Workspace::with_session`]).
    pub fn settle(ws: &Mutex<Self>) {
        let w = Self::lock(ws);
        w.closed.store(true, Ordering::SeqCst);
        w.save_layout();
        let sessions: Vec<Arc<Mutex<Slot>>> = w.tabs.values().map(|t| Arc::clone(&t.session)).collect();
        drop(w);
        // Chaque session : sa requête en cours finie, puis sa reprise écrite.
        for session in sessions {
            relock(&session).flush();
        }
    }

    /// De quoi supprimer des onglets fermés une fois le verrou rendu.
    fn trash(&self) -> Trash {
        Trash { storage: self.platform.storage.clone(), dir: self.dir.clone() }
    }

    fn view_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self.view()).context(ResponseSnafu)?)
    }

    /// Vrai si `path` (normalisé comme pour `OpenTab`) est le fichier d'un
    /// autre onglet que `tab`.
    pub fn open_elsewhere(&self, tab: &str, path: &Path) -> bool {
        let path = resolve(path);
        self.tabs.iter().any(|(id, t)| id != tab && t.info.path.as_deref() == Some(path.as_path()))
    }

    /// `settle` est passée : plus rien ne s'ouvre ni ne change.
    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    fn tab_with_path(&self, path: &Path) -> Option<String> {
        self.tabs.iter().find(|(_, t)| t.info.path.as_deref() == Some(path)).map(|(id, _)| id.clone())
    }
}

#[path = "workspace_resume.rs"]
mod resume;

#[path = "workspace_slot.rs"]
mod slot;

#[path = "workspace_same_deck.rs"]
mod same_deck;

#[path = "workspace_carry.rs"]
mod carry;

#[path = "workspace_scheme.rs"]
mod scheme;

#[path = "workspace_help.rs"]
mod help;

#[path = "workspace_agent.rs"]
mod agent;

#[path = "workspace_layout.rs"]
mod layout;

#[path = "workspace_tabs.rs"]
mod tabs;

#[path = "workspace_windows.rs"]
mod windows;

#[path = "workspace_import_pick.rs"]
mod import_pick;

#[path = "workspace_request.rs"]
mod request;

#[path = "workspace_ids.rs"]
mod ids;

#[path = "workspace_locale.rs"]
mod locale;

#[path = "workspace_session.rs"]
mod session;

pub use ids::is_window_label;
pub use import_pick::ImportPick;
pub use layout::Rect;
pub use request::{TabView, WindowView, WorkspaceOutcome, WorkspaceRequest, WorkspaceView};

use ids::{MAX_NUMBER, is_tab_id, number};
use layout::{LAYOUT_VERSION, Layout, WindowLayout};
use session::relock;
use slot::{Dormant, Slot, Tab};
use tabs::{Opening, Pending, Step, Trash};

/// La session d'un onglet : un deck sans fichier s'enregistre de lui-même
/// dans le dossier des decks (`paths.decks`).
///
/// Sans rendu : le premier est fait à la première requête de l'onglet
/// (`Slot::ready`), ou par le fichier ouvert, ou par la reprise.
pub(crate) fn tab_session(fonts: &Fonts, platform: &Platform) -> Result<Session> {
    let mut session = Session::unrendered(fonts.clone(), platform)?;
    session.set_decks_dir(platform.settings.paths.decks());
    Ok(session)
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod workspace_tests;

#[cfg(test)]
#[path = "workspace_detach_tests.rs"]
mod workspace_detach_tests;

#[cfg(test)]
#[path = "workspace_import_tests.rs"]
mod workspace_import_tests;
