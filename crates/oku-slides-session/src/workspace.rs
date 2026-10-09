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
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ResultExt};

use oku_slides_platform::storage::resolve;

use crate::error::{ResponseSnafu, Result, UnknownTabSnafu, UnknownWindowSnafu, UnsavedSnafu};
use crate::views::TabInfo;
use crate::{Fonts, Localizer, Platform, Request, Session};

/// Version de `workspace.json`.
const LAYOUT_VERSION: u32 = 2;
/// Sous-dossier des onglets, dans le dossier de reprise.
const TABS_DIR: &str = "tabs";

/// Plus grand numéro d'onglet ou de fenêtre : 18 chiffres.
const MAX_NUMBER: u64 = 999_999_999_999_999_999;

/// `prefix` suivi de 1 à 18 chiffres (un `u64` sans débordement).
fn numbered(s: &str, prefix: &str) -> bool {
    s.strip_prefix(prefix).is_some_and(|n| (1..=18).contains(&n.len()) && n.bytes().all(|b| b.is_ascii_digit()))
}

/// Identifiant d'onglet sûr dans un chemin : `t-` et des chiffres.
fn is_tab_id(id: &str) -> bool {
    numbered(id, "t-")
}

/// Un label de fenêtre de l'espace de travail : `w-<n>`.
///
/// Accepté par Tauri et par la capacité `w-*`.
pub fn is_window_label(label: &str) -> bool {
    numbered(label, "w-")
}

/// Le numéro d'un identifiant déjà validé.
fn number(id: &str) -> u64 {
    id.get(2..).and_then(|n| n.parse().ok()).unwrap_or(0)
}

/// Un verrou, repris s'il est empoisonné : une panique ne bloque pas l'app.
fn relock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Une requête de la barre d'onglets ou de l'hôte.
#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum WorkspaceRequest {
    /// La vue, sans rien changer.
    View,
    /// Langue de l'utilisateur et messages Fluent de l'interface.
    Messages,
    /// Choisit la langue de l'interface (`None` : celle du système), gardée
    /// dans `config.toml` ; répond comme `Messages`. Les messages du cœur
    /// (erreurs, noms par défaut) suivent au prochain lancement.
    SetLocale { locale: Option<String> },
    /// Schéma de l'interface choisi : `{ "scheme": "light" | "dark" | null }`
    /// (`null` : celui du système).
    Scheme,
    /// Choisit le schéma de l'interface (`None` ou inconnu : celui du
    /// système), gardé dans `config.toml` ; répond comme `Scheme`.
    SetScheme { scheme: Option<String> },
    /// Un onglet vierge, à la fin de `window`.
    NewTab {
        /// Label de la fenêtre.
        window: String,
    },
    /// Ouvre un fichier : son onglet s'il est déjà ouvert, sinon un nouvel onglet.
    OpenTab {
        /// Fenêtre d'arrivée d'un nouvel onglet.
        window: String,
        /// Le fichier (normalisé à la réception).
        path: PathBuf,
    },
    /// L'aide de l'app, en lecture seule, dans la langue de l'interface :
    /// son onglet s'il est déjà ouvert, sinon un nouvel onglet.
    OpenHelp {
        /// Fenêtre d'arrivée d'un nouvel onglet.
        window: String,
    },
    /// L'onglet qui tient déjà le deck de `path` sous un autre fichier :
    /// `{ "tab": "t-3" | null, "title": … | null }` ; ne change rien.
    SameDeck {
        /// Le fichier (normalisé à la réception).
        path: PathBuf,
    },
    /// L'onglet devient actif dans sa fenêtre.
    Activate {
        /// Identifiant de l'onglet.
        tab: String,
    },
    /// Ferme un onglet.
    Close {
        /// Identifiant de l'onglet.
        tab: String,
        /// L'utilisateur abandonne le travail non enregistré.
        #[serde(default)]
        discard: bool,
    },
    /// Place l'onglet à `index` dans sa barre.
    Reorder {
        /// Identifiant de l'onglet.
        tab: String,
        /// Place finale (bornée à la barre).
        index: usize,
    },
    /// Vers une autre fenêtre, ou une nouvelle.
    MoveTab {
        /// Identifiant de l'onglet.
        tab: String,
        /// Fenêtre d'arrivée ; `None` : une nouvelle fenêtre.
        #[serde(default)]
        window: Option<String>,
        /// Place dans la barre d'arrivée ; `None` : à la fin.
        #[serde(default)]
        index: Option<usize>,
    },
    /// Une fenêtre avec un onglet vierge.
    NewWindow,
    /// Ferme une fenêtre et ses onglets.
    CloseWindow {
        /// Label de la fenêtre.
        window: String,
        /// Onglets dont l'utilisateur abandonne le travail non enregistré.
        #[serde(default)]
        discard: Vec<String>,
    },
    /// L'agent passe dans sa propre fenêtre.
    DetachAgent,
    /// L'agent revient dans la colonne de droite.
    AttachAgent,
}

/// Un onglet de la vue.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TabView {
    /// Identifiant `t-<n>`.
    pub id: String,
    /// Titre, travail non enregistré, fichier.
    #[serde(flatten)]
    pub info: TabInfo,
}

/// Une fenêtre de la vue : ses onglets et l'onglet actif.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WindowView {
    /// Label `w-<n>`, aussi celui de la fenêtre Tauri.
    pub label: String,
    /// Identifiant de l'onglet actif.
    pub active: String,
    /// Onglets, dans l'ordre de la barre.
    pub tabs: Vec<TabView>,
}

/// Ce que l'interface voit : les fenêtres et leurs onglets.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkspaceView {
    /// Numéro de la vue, croissant : une vue plus petite que celle déjà
    /// montrée est plus ancienne (réponse doublée par un événement) et écartée.
    pub generation: u64,
    /// Dernière fenêtre active.
    pub focused: String,
    /// Fenêtres, dans l'ordre de création.
    pub windows: Vec<WindowView>,
    /// L'agent est dans sa propre fenêtre : la colonne de droite ne le montre plus.
    pub agent_detached: bool,
}

/// Ce que [`Workspace::apply`] a fait : la vue à jour, en JSON, dans les deux cas.
#[derive(Debug)]
pub enum WorkspaceOutcome {
    /// La requête a été appliquée.
    Applied(String),
    /// Fermeture de la seule fenêtre refusée, sans y toucher : voir
    /// [`Workspace::is_only_window`], utilisé aussi par l'hôte (fermeture
    /// native) pour le même refus, en aperçu hors verrou.
    LastWindow(String),
}

impl WorkspaceOutcome {
    /// La vue à jour, qu'elle ait changé ou non.
    pub fn into_json(self) -> String {
        match self {
            Self::Applied(json) | Self::LastWindow(json) => json,
        }
    }

    /// La fenêtre visée était la seule de l'espace de travail : rien n'a été fermé.
    pub fn is_last_window(&self) -> bool {
        matches!(self, Self::LastWindow(_))
    }
}

/// Un rectangle à l'écran, dans les unités de l'hôte (celles du point comparé).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Bord gauche.
    pub x: f64,
    /// Bord haut.
    pub y: f64,
    /// Largeur.
    pub w: f64,
    /// Hauteur.
    pub h: f64,
}

impl Rect {
    /// Le point est dedans (bords gauche et haut compris).
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// Fenêtres et onglets, tels qu'écrits dans `workspace.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Layout {
    version: u32,
    /// Prochains numéros : jamais réutilisés.
    next_tab: u64,
    next_window: u64,
    focused: String,
    windows: Vec<WindowLayout>,
    /// Onglets cités mais non restaurés, dont le dossier existe encore : ils
    /// restent cités, leur dossier n'est jamais supprimé.
    #[serde(default)]
    kept: Vec<String>,
    /// L'agent est dans sa propre fenêtre (`agent`), pas dans la colonne de droite.
    #[serde(default)]
    agent_detached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WindowLayout {
    label: String,
    tabs: Vec<String>,
    active: String,
}

impl Layout {
    fn new() -> Self {
        Self {
            version: LAYOUT_VERSION,
            next_tab: 1,
            next_window: 1,
            focused: String::new(),
            windows: Vec::new(),
            kept: Vec::new(),
            agent_detached: false,
        }
    }
}

struct Tab {
    session: Arc<Mutex<Slot>>,
    /// Dernières infos connues : la vue se construit sans verrouiller les sessions.
    info: TabInfo,
}

impl Tab {
    fn new(session: Session) -> Self {
        Self { info: session.tab_info(), session: Arc::new(Mutex::new(Slot::Ready(Box::new(session)))) }
    }

    /// Un onglet repris qui dort : sa session sera relue à sa première requête.
    fn dormant(info: TabInfo, dormant: Dormant) -> Self {
        Self { info, session: Arc::new(Mutex::new(Slot::Dormant(Box::new(dormant)))) }
    }
}

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
        }
    }

    /// Les messages de l'interface dans la langue choisie, les langues
    /// livrées et le choix (`null` : langue du système).
    fn messages_json(&self) -> Result<String> {
        let l = Localizer::new(self.chosen_locale);
        let messages = serde_json::json!({
            "locale": l.locale(),
            // Les raccourcis des messages dans la notation de la plateforme (`⌘Z` ou `Ctrl+Z`).
            "resources": l.ui_messages().into_iter().map(oku_slides_i18n::platform_keys).collect::<Vec<_>>(),
            "available": oku_slides_i18n::available(),
            "chosen": self.chosen_locale,
        });
        Ok(serde_json::to_string(&messages).context(ResponseSnafu)?)
    }

    /// Retient la langue de l'interface (inconnue : celle du système) et
    /// l'écrit dans `config.toml` ; un échec d'écriture ne coûte que le
    /// prochain lancement.
    fn set_locale(&mut self, locale: Option<&str>) {
        self.chosen_locale = locale.and_then(oku_slides_i18n::shipped);
        let value = self.chosen_locale.map_or(serde_json::Value::Null, serde_json::Value::from);
        let file = self.platform.settings.config_file();
        if let Err(e) =
            oku_slides_platform::config_write::write_value(&self.platform.storage, &file, "i18n.locale", &value)
        {
            tracing::warn!(path = %file.display(), error = %e, "langue non gardée pour le prochain lancement");
        }
    }

    /// Le verrou de l'espace de travail, repris s'il est empoisonné.
    ///
    /// Ne jamais le garder pendant un appel à `apply` ou `request` : le
    /// verrou n'est pas réentrant.
    pub fn lock(ws: &Mutex<Self>) -> MutexGuard<'_, Self> {
        relock(ws)
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

    /// La fenêtre sous le point `(x, y)` : la plus récemment active qui le contient.
    pub fn window_at(&self, x: f64, y: f64, rects: &[(String, Rect)]) -> Option<String> {
        self.recent.iter().rev().find(|label| rects.iter().any(|(l, r)| l == *label && r.contains(x, y))).cloned()
    }

    /// Label de la fenêtre qui porte l'onglet `tab`.
    pub fn tab_window(&self, tab: &str) -> Option<&str> {
        self.layout.windows.iter().find(|w| w.tabs.iter().any(|t| t == tab)).map(|w| w.label.as_str())
    }

    /// `label` est-elle la seule fenêtre de l'espace de travail ?
    ///
    /// Prédicat unique, utilisé par [`Workspace::apply`] (`CloseWindow`, sous
    /// le verrou : la décision est atomique avec la suppression) et par
    /// l'hôte (`close_requested`, en aperçu hors verrou, avant de prévenir la
    /// page) : les deux points d'appel voient la même règle.
    pub fn is_only_window(&self, label: &str) -> bool {
        self.layout.windows.len() <= 1 && self.layout.windows.first().is_some_and(|w| w.label == label)
    }

    /// Une requête pour l'onglet `tab` ; `true` : la barre est à redessiner.
    ///
    /// Seule la session de l'onglet est verrouillée pendant le rendu ;
    /// l'espace de travail l'est avant et après, jamais en même temps.
    pub fn request(ws: &Mutex<Self>, tab: &str, req: Request) -> (Result<String>, bool) {
        let (reply, changed) = Self::with_session(ws, tab, |s| s.handle(req));
        (reply.and_then(|r| r), changed)
    }

    /// `f` sur la session de l'onglet (réveillée au besoin) ; `true` : la barre est à redessiner.
    ///
    /// Même discipline que [`Workspace::request`] : l'espace de travail est
    /// verrouillé avant et après, jamais pendant `f`.
    ///
    /// Après [`Workspace::settle`], refusée comme un onglet fermé
    /// (`UnknownTab`) : sa reprise est écrite, plus rien ne doit la changer.
    pub fn with_session<R>(ws: &Mutex<Self>, tab: &str, f: impl FnOnce(&mut Session) -> R) -> (Result<R>, bool) {
        let (session, closed) = {
            let w = Self::lock(ws);
            match w.session(tab) {
                Ok(session) => (session, Arc::clone(&w.closed)),
                Err(e) => return (Err(e), false),
            }
        };
        let (out, info) = {
            let mut slot = relock(&session);
            // Relu sous le verrou de la session : `settle` pose `closed` avant
            // de vider chaque session sous ce même verrou ; vu faux ici, le
            // changement de `f` sera donc vidé après.
            if closed.load(Ordering::SeqCst) {
                return (Err(UnknownTabSnafu { id: tab }.build().into()), false);
            }
            match slot.ready() {
                Ok(s) => {
                    let out = f(s);
                    (Ok(out), Some(s.tab_info()))
                }
                Err(e) => (Err(e), None),
            }
        };
        let changed = info.is_some_and(|info| Self::refresh(ws, tab, info));
        (out, changed)
    }

    /// Les infos d'un onglet après usage de sa session ; `true` si elles ont changé.
    ///
    /// Elles vivent aussi dans `workspace.json` (la barre du prochain
    /// lancement) : écrites dès qu'elles changent.
    fn refresh(ws: &Mutex<Self>, tab: &str, info: TabInfo) -> bool {
        let mut w = Self::lock(ws);
        let changed = w.note(tab, info);
        if changed {
            w.save_layout();
        }
        changed
    }

    /// Réveille les onglets endormis parmi `tabs`, hors du verrou de l'espace de travail.
    ///
    /// Avant une décision qui doit lire leur état réel (fermer sans
    /// question, remplacer un onglet vierge) : les infos enregistrées
    /// peuvent être en retard (arrêt brutal entre deux écritures). Un échec
    /// est journalisé ; l'onglet reste endormi et la décision prudente.
    fn wake(ws: &Mutex<Self>, tabs: &[String]) {
        let sessions: Vec<(String, Arc<Mutex<Slot>>)> = {
            let w = Self::lock(ws);
            tabs.iter().filter_map(|id| Some((id.clone(), w.session(id).ok()?))).collect()
        };
        for (id, session) in sessions {
            let info = {
                // Jamais d'attente : une session occupée n'est pas endormie
                // (déjà relue, ou en train de l'être), la décision reste prudente.
                let mut slot = match session.try_lock() {
                    Ok(slot) => slot,
                    Err(std::sync::TryLockError::Poisoned(p)) => p.into_inner(),
                    Err(std::sync::TryLockError::WouldBlock) => continue,
                };
                if !slot.is_dormant() {
                    continue;
                }
                match slot.ready() {
                    Ok(s) => s.tab_info(),
                    Err(e) => {
                        tracing::warn!(tab = %id, error = %e.one_line(), "onglet endormi non relu");
                        continue;
                    }
                }
            };
            Self::refresh(ws, &id, info);
        }
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
                    Step::Build(pending) => pending,
                },
                WorkspaceRequest::OpenHelp { window } => match Self::open_help(ws, w, window)? {
                    Step::Done(json) => return Ok(WorkspaceOutcome::Applied(json)),
                    Step::Build(pending) => pending,
                },
                WorkspaceRequest::NewWindow => w.pending(None, Opening::Blank, None),
                WorkspaceRequest::CloseWindow { window, discard } => return Self::close_window(w, &window, &discard),
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
        let session = pending.build()?;
        Self::finish(ws, pending, session).map(|(outcome, _)| outcome)
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

    /// La session d'un onglet, peut-être endormie.
    fn session(&self, tab: &str) -> Result<Arc<Mutex<Slot>>> {
        Ok(self.tabs.get(tab).map(|t| Arc::clone(&t.session)).context(UnknownTabSnafu { id: tab })?)
    }

    /// Les infos d'un onglet après une requête ; `true` si elles ont changé.
    fn note(&mut self, tab: &str, info: TabInfo) -> bool {
        match self.tabs.get_mut(tab) {
            Some(t) if t.info != info => {
                t.info = info;
                true
            }
            _ => false,
        }
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

use slot::{Dormant, Slot};
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
