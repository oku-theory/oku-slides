//! Les onglets : en créer (hors du verrou), en ouvrir, les fermer, les changer.
//!
//! Un onglet neuf est préparé sous le verrou ([`Pending`]), sa session
//! construite hors du verrou, puis insérée par [`Workspace::finish`].

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use oku_slides_platform::storage::Storage;
use snafu::{OptionExt, ResultExt, ensure};

use crate::error::ImportSnafu;
use crate::import::{ImportRequest, ImportTarget, PackLookup, import};

use super::{
    Fonts, Platform, Result, Session, Slot, TABS_DIR, Tab, UnknownTabSnafu, UnknownWindowSnafu, UnsavedSnafu,
    Workspace, WorkspaceOutcome, WorkspaceRequest, is_tab_id, relock, resolve, tab_session,
};

/// De quoi supprimer les dossiers d'onglets fermés, verrou rendu.
pub(super) struct Trash {
    pub(super) storage: Storage,
    pub(super) dir: Option<PathBuf>,
}

impl Trash {
    /// Chaque session n'écrit plus rien, puis son dossier est supprimé.
    ///
    /// Toujours après l'écriture de `workspace.json` : jamais d'onglet cité
    /// sans dossier.
    pub(super) fn empty(&self, gone: Vec<(String, Tab)>) {
        for (id, tab) in gone {
            relock(&tab.session).stop_resume(&id);
            let Some(dir) = &self.dir else { continue };
            // Double contrôle : jamais de suppression hors de `tabs/`.
            if !is_tab_id(&id) {
                tracing::warn!(tab = %id, "identifiant d'onglet refusé : rien n'est supprimé");
                continue;
            }
            let path = dir.join(TABS_DIR).join(&id);
            if let Err(e) = self.storage.delete_dir(&path) {
                tracing::warn!(path = %path.display(), error = %oku_slides_platform::errors::one_line(&e), "dossier d'onglet");
            }
        }
    }
}

/// Ce que devient une requête d'onglet sous le verrou.
pub(super) enum Step {
    /// Réglée sous le verrou : la vue à jour, en JSON.
    Done(String),
    /// Un onglet à construire hors du verrou, puis à insérer.
    Build(Pending),
}

/// Ce que montre un onglet à créer.
pub(super) enum Opening {
    /// Un deck vierge.
    Blank,
    /// Le deck de ce fichier (chemin résolu).
    File(PathBuf),
    /// L'aide dans cette langue, au thème du deck d'où on l'ouvre.
    Help { locale: &'static str, theme: Option<uuid::Uuid> },
    /// Un import (en boîte : la cible est lourde, `Step` reste petit).
    Import(Box<ImportOpening>),
}

/// L'import qu'un onglet à créer montrera.
pub(super) struct ImportOpening {
    /// Le `.pptx` (chemin résolu).
    pub(super) path: PathBuf,
    pub(super) target: ImportTarget,
    /// L'agent fine-tunera l'onglet importé.
    pub(super) tune: bool,
}

/// Un onglet à créer hors du verrou : tout ce qu'il faut pour sa session.
pub(super) struct Pending {
    pub(super) id: String,
    /// Fenêtre d'arrivée ; `None` : une nouvelle fenêtre.
    pub(super) window: Option<String>,
    pub(super) opening: Opening,
    /// Onglet vierge à fermer une fois le nouveau prêt.
    pub(super) replace: Option<String>,
    pub(super) fonts: Fonts,
    pub(super) platform: Platform,
    pub(super) dir: Option<PathBuf>,
}

impl Pending {
    /// L'onglet à créer est un import que l'agent doit fine-tuner.
    pub(super) fn tunes(&self) -> bool {
        matches!(&self.opening, Opening::Import(i) if i.tune)
    }

    /// La session de l'onglet : créée, fichier ouvert, reprise activée.
    ///
    /// Un onglet neuf n'a rien à relire : un échec de reprise ne lui coûte
    /// pas le fichier ouvert, il reste seulement sans reprise.
    pub(super) fn build(&self) -> Result<Session> {
        let mut session = tab_session(&self.fonts, &self.platform)?;
        match &self.opening {
            Opening::Blank => {}
            Opening::File(path) => session.open(path)?,
            Opening::Help { locale, theme } => session.open_help(locale, *theme)?,
            Opening::Import(i) => {
                // Les polices de la présentation sont cherchées aussi parmi celles du système, comme dans la CLI ;
                // le pack cible, dans la bibliothèque seulement (jamais un dossier venu de l'interface).
                let request = ImportRequest {
                    pptx: i.path.clone(),
                    target: i.target.clone(),
                    system_fonts: true,
                    packs: PackLookup::Library,
                };
                let imported = import(&self.platform, &self.fonts, &request).context(ImportSnafu)?;
                let name = i.path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                session.load_import(imported, &name)?;
            }
        }
        if let Some(dir) = &self.dir
            && !session.resume_fresh(dir.join(TABS_DIR).join(&self.id))
        {
            tracing::warn!(tab = %self.id, "reprise de l'onglet inactive");
        }
        Ok(session)
    }
}

impl Workspace {
    /// Un onglet vierge à la fin de `window`.
    pub(super) fn new_tab(&mut self, window: String) -> Result<Pending> {
        self.window_index(&window)?;
        Ok(self.pending(Some(window), Opening::Blank, None))
    }

    /// Ouvre `path` : son onglet s'il existe déjà, sinon un nouvel onglet
    /// (qui remplace l'onglet actif s'il est vierge).
    pub(super) fn open_tab(&mut self, window: String, path: PathBuf) -> Result<Step> {
        // Fin déjà engagée (`settle`) : refusé, comme si la
        // fenêtre visée n'existait plus (elle est en train de
        // disparaître). Sous ce même verrou : atomique avec `settle`.
        if self.is_closed() {
            return Err(UnknownWindowSnafu { label: window }.build().into());
        }
        let path = resolve(&path);
        if let Some(id) = self.tab_with_path(&path) {
            self.activate(&id)?;
            self.save_layout();
            return self.view_json().map(Step::Done);
        }
        let active = self.layout.windows[self.window_index(&window)?].active.clone();
        Ok(Step::Build(self.pending(Some(window), Opening::File(path), Some(active))))
    }

    /// Importe `path` vers `target` dans un nouvel onglet de `window`.
    ///
    /// Il remplace l'onglet actif s'il est vierge, comme un fichier ouvert.
    ///
    /// Jamais l'onglet d'un import précédent : chaque import est un deck neuf.
    pub(super) fn import_tab(
        &mut self,
        window: String,
        path: PathBuf,
        target: ImportTarget,
        tune: bool,
    ) -> Result<Pending> {
        // Comme `OpenTab` : refusé une fois la fin engagée.
        if self.is_closed() {
            return Err(UnknownWindowSnafu { label: window }.build().into());
        }
        let active = self.layout.windows[self.window_index(&window)?].active.clone();
        let opening = Opening::Import(Box::new(ImportOpening { path: resolve(&path), target, tune }));
        Ok(self.pending(Some(window), opening, Some(active)))
    }

    /// Ouvre l'aide de `window` : son onglet s'il existe déjà (au thème du
    /// deck d'où on vient), sinon un nouvel onglet.
    pub(super) fn open_help(ws: &Mutex<Self>, mut w: MutexGuard<'_, Self>, window: String) -> Result<Step> {
        // Comme `OpenTab` : refusé une fois la fin engagée.
        if w.is_closed() {
            return Err(UnknownWindowSnafu { label: window }.build().into());
        }
        w.window_index(&window)?;
        let (locale, theme) = (w.help_language(), w.help_theme(&window));
        if let Some(id) = w.tab_with_help(locale) {
            w.activate(&id)?;
            w.save_layout();
            // L'aide rouverte prend le thème du deck d'où on vient,
            // hors du verrou de l'espace de travail.
            drop(w);
            if let Some(theme) = theme {
                Self::retheme_help(ws, &id, theme);
            }
            return Self::lock(ws).view_json().map(Step::Done);
        }
        Ok(Step::Build(w.help_pending(window, locale, theme)))
    }

    /// Insère un onglet construit hors du verrou (seconde moitié d'`apply`) ;
    /// rend aussi l'onglet du fichier (le nouveau, ou celui qui l'avait déjà).
    pub(super) fn finish(ws: &Mutex<Self>, pending: Pending, session: Session) -> Result<(WorkspaceOutcome, String)> {
        let Pending { id, window, opening, replace, .. } = pending;
        let mut w = Self::lock(ws);
        // `settle` passé pendant la construction : refusé comme avant elle ;
        // la session construite pour rien part à la corbeille.
        if w.is_closed() {
            let trash = w.trash();
            drop(w);
            trash.empty(vec![(id, Tab::new(session))]);
            return Err(UnknownWindowSnafu { label: window.unwrap_or_default() }.build().into());
        }
        // Le même fichier a pu être ouvert pendant la construction (Finder et
        // seconde instance, double clic) : le premier onglet reste, seul.
        // De même pour l'aide : deux demandes rapprochées (menu et ⌘?) n'ouvrent qu'un onglet.
        let existing = match &opening {
            Opening::Blank | Opening::Import(_) => None,
            Opening::File(path) => w.tab_with_path(path),
            Opening::Help { locale, .. } => w.tab_with_help(locale),
        };
        if let Some(existing) = existing {
            let activated = w.activate(&existing);
            w.save_layout();
            let (json, trash) = (w.view_json(), w.trash());
            drop(w);
            // La session construite pour rien part à la corbeille, même en cas d'erreur.
            trash.empty(vec![(id, Tab::new(session))]);
            activated?;
            return json.map(|json| (WorkspaceOutcome::Applied(json), existing));
        }
        let window = match window {
            Some(label) if w.window_index(&label).is_ok() => Some(label),
            // Fenêtre disparue pendant la construction : la fenêtre active.
            Some(_) => Some(w.layout.focused.clone()).filter(|l| w.window_index(l).is_ok()),
            None => None,
        };
        // Toujours vierge ? L'état réel de la session fait foi. Sans attendre
        // sous le verrou : une session occupée (un rendu en cours) est en
        // train d'être modifiée, donc pas vierge ; une session endormie n'a
        // pas été relue, donc rien ne prouve qu'elle l'est.
        let replaced = replace.filter(|r| {
            let pristine = |slot: &Slot| slot.info().is_some_and(|i| i.pristine);
            w.session(r).is_ok_and(|s| match s.try_lock() {
                Ok(slot) => pristine(&slot),
                Err(std::sync::TryLockError::Poisoned(p)) => pristine(&p.into_inner()),
                Err(std::sync::TryLockError::WouldBlock) => false,
            })
        });
        w.insert_tab(window.as_deref(), id.clone(), session, replaced.as_deref());
        let mut gone = Vec::new();
        if let Some(r) = replaced {
            if let Err(e) = w.detach(&r) {
                tracing::warn!(tab = %r, error = %e.one_line(), "onglet vierge non retiré");
            }
            gone.extend(w.tabs.remove(&r).map(|t| (r, t)));
        }
        w.save_layout();
        let (json, trash) = (w.view_json(), w.trash());
        drop(w);
        trash.empty(gone);
        json.map(|json| (WorkspaceOutcome::Applied(json), id))
    }

    /// Réserve un numéro d'onglet et prépare sa construction hors du verrou.
    pub(super) fn pending(&mut self, window: Option<String>, opening: Opening, replace: Option<String>) -> Pending {
        Pending {
            id: self.next_tab_id(),
            window,
            opening,
            replace,
            fonts: self.fonts.clone(),
            platform: self.platform.clone(),
            dir: self.dir.clone(),
        }
    }

    fn next_tab_id(&mut self) -> String {
        let id = format!("t-{}", self.layout.next_tab);
        self.layout.next_tab = self.layout.next_tab.saturating_add(1);
        id
    }

    /// Traite les requêtes qui ne créent rien.
    ///
    /// Renvoie les onglets à supprimer après l'écriture de `workspace.json`.
    pub(super) fn change(&mut self, req: WorkspaceRequest) -> Result<Vec<(String, Tab)>> {
        match req {
            WorkspaceRequest::Activate { tab } => self.activate(&tab)?,
            WorkspaceRequest::Close { tab, discard } => {
                self.ensure_closable(&tab, discard)?;
                self.detach(&tab)?;
                return Ok(self.tabs.remove(&tab).map(|t| (tab, t)).into_iter().collect());
            }
            WorkspaceRequest::Reorder { tab, index } => self.reorder(&tab, index)?,
            WorkspaceRequest::MoveTab { tab, window, index } => self.move_tab(&tab, window.as_deref(), index)?,
            WorkspaceRequest::DetachAgent => self.set_agent_detached(true),
            WorkspaceRequest::AttachAgent => self.set_agent_detached(false),
            WorkspaceRequest::View
            | WorkspaceRequest::Messages
            | WorkspaceRequest::SetLocale { .. }
            | WorkspaceRequest::Scheme
            | WorkspaceRequest::SetScheme { .. }
            | WorkspaceRequest::SameDeck { .. }
            | WorkspaceRequest::NewTab { .. }
            | WorkspaceRequest::OpenTab { .. }
            | WorkspaceRequest::ImportTab { .. }
            | WorkspaceRequest::OpenHelp { .. }
            | WorkspaceRequest::NewWindow
            // Traitée directement dans `apply` (décision atomique de la
            // dernière fenêtre, sous le même verrou) : jamais atteinte ici.
            | WorkspaceRequest::CloseWindow { .. } => {}
        }
        Ok(Vec::new())
    }

    /// Refuse de fermer un onglet non enregistré, sans jamais attendre sa session.
    ///
    /// `discard` : l'utilisateur abandonne le travail, rien à lire. Sinon,
    /// seul `try_lock` est tenté : verrouillée (un rendu en cours),
    /// empoisonnée (une panique) ou encore endormie (réveil en échec, voir
    /// [`Workspace::wake`]) refuse aussi la fermeture, pour ne jamais fermer
    /// en silence un onglet dont on ne connaît pas l'état ; l'utilisateur
    /// retente, ou abandonne.
    pub(super) fn ensure_closable(&self, tab: &str, discard: bool) -> Result<()> {
        let t = self.tabs.get(tab).context(UnknownTabSnafu { id: tab })?;
        if discard {
            return Ok(());
        }
        let info = t.session.try_lock().ok().and_then(|slot| slot.info());
        let (dirty, title) = info.map_or((true, t.info.title.clone()), |i| (i.dirty, i.title));
        ensure!(!dirty, UnsavedSnafu { title });
        Ok(())
    }

    /// Onglets dont `req` doit lire l'état réel.
    ///
    /// Fermés sans abandon, ou l'onglet vierge qu'un fichier ouvert ou importé remplacerait (s'il
    /// l'était).
    pub(super) fn to_wake(&self, req: &WorkspaceRequest) -> Vec<String> {
        match req {
            WorkspaceRequest::Close { tab, discard: false } => vec![tab.clone()],
            // La seule fenêtre ne se ferme pas (l'app quitte en gardant tout) :
            // rien à relire, la reprise paresseuse reste paresseuse.
            WorkspaceRequest::CloseWindow { window, .. } if self.is_only_window(window) => Vec::new(),
            WorkspaceRequest::CloseWindow { window, discard } => self
                .layout
                .windows
                .iter()
                .find(|w| &w.label == window)
                .map(|w| w.tabs.iter().filter(|t| !discard.contains(t)).cloned().collect())
                .unwrap_or_default(),
            // Fin engagée, ou fichier déjà ouvert : aucun onglet ne sera remplacé.
            WorkspaceRequest::OpenTab { path, .. }
                if self.is_closed() || self.tab_with_path(&resolve(path)).is_some() =>
            {
                Vec::new()
            }
            WorkspaceRequest::OpenTab { window, .. } | WorkspaceRequest::ImportTab { window, .. } => self
                .layout
                .windows
                .iter()
                .find(|w| &w.label == window)
                .map(|w| w.active.clone())
                .filter(|id| self.tabs.get(id).is_some_and(|t| t.info.pristine))
                .into_iter()
                .collect(),
            _ => Vec::new(),
        }
    }
}
