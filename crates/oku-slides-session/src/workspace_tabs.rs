//! Les onglets : en créer (hors du verrou), en ouvrir, les fermer, les changer.
//!
//! Un onglet neuf est préparé sous le verrou ([`Pending`]), sa session
//! construite hors du verrou, puis insérée par [`Workspace::finish`].

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use oku_slides_platform::storage::Storage;
use snafu::{OptionExt, ResultExt, ensure};

use crate::error::{ImportSnafu, PackTabNotFreshSnafu};
use crate::import::{ImportRequest, ImportTarget, PackLookup, import};
use crate::pack_lease::PackLease;
use crate::pack_tab::PreviewSeed;

use super::{
    Fonts, Platform, Result, Session, Slot, TABS_DIR, Tab, TabPlace, UnknownTabSnafu, UnknownWindowSnafu, UnsavedSnafu,
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
    /// Un onglet à construire hors du verrou, puis à insérer (en boîte : `Step` reste petit).
    Build(Box<Pending>),
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
    /// A pack tab; its seed and lease are passed to [`Pending::build_pack`].
    Pack {
        /// The pack the new tab edits.
        pack: uuid::Uuid,
        /// A widget whose sub-tab opens.
        widget: Option<String>,
    },
    /// A theme tab; its lease is passed to [`Pending::build_theme`].
    Theme {
        /// The theme the new tab edits.
        theme: uuid::Uuid,
    },
    /// A deck-only pack's tab, linked to its owner deck.
    DeckPack {
        /// The owner deck's tab.
        deck_tab: String,
        /// The edited pack.
        pack: uuid::Uuid,
        /// The widget to open first, if any.
        widget: Option<String>,
    },
}

/// Makes the next deck-pack tab build fail, once (tests only).
///
/// Nextest only: one process per test, so no other test sees it set.
#[cfg(test)]
pub(super) static FAIL_DECK_PACK_BUILD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// A test hook run once by the next deck-pack tab build, before it builds.
///
/// Nextest only: one process per test, so no other test sees it set.
#[cfg(test)]
pub(super) static DURING_DECK_PACK_BUILD: std::sync::Mutex<Option<Box<dyn FnOnce() + Send>>> =
    std::sync::Mutex::new(None);

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
    /// Where the tab goes; a `Replace` tab is closed once the new one is ready.
    pub(super) place: TabPlace,
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
                // le pack cible, dans la bibliothèque seulement (jamais un dossier venu de l'interface) ;
                // le témoin, celui de `[import] witness` (t-905).
                let target = match &i.target {
                    ImportTarget::Hifi { .. } => {
                        ImportTarget::Hifi { witness: Some(self.platform.settings.import.witness) }
                    }
                    retrofit @ ImportTarget::Retrofit { .. } => retrofit.clone(),
                };
                let request =
                    ImportRequest { pptx: i.path.clone(), target, system_fonts: true, packs: PackLookup::Library };
                let imported = import(&self.platform, &self.fonts, &request).context(ImportSnafu)?;
                let name = i.path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                session.load_import(imported, &name)?;
            }
            // Only `build_pack`, `build_theme` and `build_deck_pack` build these tabs: they are given their seed or lease.
            Opening::Pack { .. } | Opening::Theme { .. } | Opening::DeckPack { .. } => {
                return PackTabNotFreshSnafu.fail()?;
            }
        }
        self.arm_resume(&mut session);
        Ok(session)
    }

    /// Builds this pending pack tab from its seed and lease.
    ///
    /// # Errors
    ///
    /// `PackTabNotFresh` unless the opening is `Opening::Pack`; the `init_pack_tab` errors.
    ///
    /// # Blocking
    ///
    /// Builds and renders a session.
    pub(super) fn build_pack(&self, seed: PreviewSeed, lease: PackLease) -> Result<Session> {
        let Opening::Pack { pack, widget } = &self.opening else { return PackTabNotFreshSnafu.fail()? };
        let mut session = tab_session(&self.fonts, &self.platform)?;
        session.init_pack_tab(seed, lease, *pack, widget.as_deref())?;
        self.arm_resume(&mut session);
        Ok(session)
    }

    /// Builds a deck-pack tab session from its owner's seed.
    ///
    /// # Errors
    ///
    /// `PackTabNotFresh` unless the opening is `Opening::DeckPack`; the `init_deck_pack_tab` errors.
    ///
    /// # Blocking
    ///
    /// Builds and renders a session.
    pub(super) fn build_deck_pack(&self, seed: Box<PreviewSeed>) -> Result<Session> {
        let Opening::DeckPack { deck_tab, pack, widget } = &self.opening else { return PackTabNotFreshSnafu.fail()? };
        #[cfg(test)]
        {
            // Taken out first: the hook never runs with the static's mutex held.
            let hook = DURING_DECK_PACK_BUILD.lock().unwrap_or_else(std::sync::PoisonError::into_inner).take();
            if let Some(hook) = hook {
                hook();
            }
        }
        #[cfg(test)]
        if FAIL_DECK_PACK_BUILD.swap(false, std::sync::atomic::Ordering::SeqCst) {
            return PackTabNotFreshSnafu.fail()?;
        }
        let mut session = tab_session(&self.fonts, &self.platform)?;
        session.init_deck_pack_tab(seed, deck_tab, *pack, widget.as_deref())?;
        self.arm_resume(&mut session);
        Ok(session)
    }

    /// Builds this pending theme tab, holding `lease` if given.
    ///
    /// # Errors
    ///
    /// `PackTabNotFresh` unless the opening is `Opening::Theme`; the `init_theme_tab` errors.
    ///
    /// # Blocking
    ///
    /// Builds a session, renders the specimen deck and freezes the theme's base.
    pub(super) fn build_theme(&self, lease: Option<PackLease>) -> Result<Session> {
        let Opening::Theme { theme } = &self.opening else { return PackTabNotFreshSnafu.fail()? };
        let mut session = tab_session(&self.fonts, &self.platform)?;
        session.init_theme_tab(lease, *theme)?;
        self.arm_resume(&mut session);
        Ok(session)
    }

    /// Turns resume on for a session this pending tab built.
    fn arm_resume(&self, session: &mut Session) {
        if let Some(dir) = &self.dir
            && !session.resume_fresh(dir.join(TABS_DIR).join(&self.id))
        {
            tracing::warn!(tab = %self.id, "reprise de l'onglet inactive");
        }
    }
}

impl Workspace {
    /// Un onglet vierge à la fin de `window`.
    pub(super) fn new_tab(&mut self, window: String) -> Result<Pending> {
        self.window_index(&window)?;
        Ok(self.pending(Some(window), Opening::Blank, TabPlace::End))
    }

    /// Ouvre `path` : son onglet s'il existe déjà, sinon un nouvel onglet
    /// (qui remplace l'onglet actif s'il est vierge).
    pub(super) fn open_tab(&mut self, window: String, path: &Path) -> Result<Step> {
        // Fin déjà engagée (`settle`) : refusé, comme si la
        // fenêtre visée n'existait plus (elle est en train de
        // disparaître). Sous ce même verrou : atomique avec `settle`.
        if self.is_closed() {
            return Err(UnknownWindowSnafu { label: window }.build().into());
        }
        let path = resolve(path);
        if let Some(id) = self.tab_with_path(&path) {
            self.activate(&id)?;
            self.save_layout();
            return self.view_json().map(Step::Done);
        }
        let active = self.layout.windows[self.window_index(&window)?].active.clone();
        Ok(Step::Build(Box::new(self.pending(Some(window), Opening::File(path), TabPlace::Replace(active)))))
    }

    /// Importe `path` vers `target` dans un nouvel onglet de `window`.
    ///
    /// Il remplace l'onglet actif s'il est vierge, comme un fichier ouvert.
    ///
    /// Jamais l'onglet d'un import précédent : chaque import est un deck neuf.
    pub(super) fn import_tab(
        &mut self,
        window: String,
        path: &Path,
        target: ImportTarget,
        tune: bool,
    ) -> Result<Pending> {
        // Comme `OpenTab` : refusé une fois la fin engagée.
        if self.is_closed() {
            return Err(UnknownWindowSnafu { label: window }.build().into());
        }
        let active = self.layout.windows[self.window_index(&window)?].active.clone();
        let opening = Opening::Import(Box::new(ImportOpening { path: resolve(path), target, tune }));
        Ok(self.pending(Some(window), opening, TabPlace::Replace(active)))
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
        Ok(Step::Build(Box::new(w.help_pending(window, locale, theme))))
    }

    /// Insère un onglet construit hors du verrou (seconde moitié d'`apply`) ;
    /// rend aussi l'onglet du fichier (le nouveau, ou celui qui l'avait déjà).
    pub(super) fn finish(ws: &Mutex<Self>, pending: Pending, session: Session) -> Result<(WorkspaceOutcome, String)> {
        let Pending { id, window, opening, place, .. } = pending;
        let mut w = Self::lock(ws);
        // `settle` passé pendant la construction : refusé comme avant elle ;
        // la session construite pour rien part à la corbeille.
        if w.is_closed() {
            let trash = w.trash();
            drop(w);
            trash.empty(vec![(id, Tab::new(session))]);
            return Err(UnknownWindowSnafu { label: window.unwrap_or_default() }.build().into());
        }
        // The owner closed during the build: its deck-pack tab would be an
        // orphan. Refused; `open_deck_pack` cancels the pending link.
        if let Opening::DeckPack { deck_tab, .. } = &opening
            && !w.tabs.contains_key(deck_tab)
        {
            let trash = w.trash();
            drop(w);
            trash.empty(vec![(id, Tab::new(session))]);
            return Err(UnknownTabSnafu { id: deck_tab }.build().into());
        }
        // Le même fichier a pu être ouvert pendant la construction (Finder et
        // seconde instance, double clic) : le premier onglet reste, seul.
        // De même pour l'aide : deux demandes rapprochées (menu et ⌘?) n'ouvrent qu'un onglet.
        // A pack or theme has no dedupe here: its opening lock and single lease forbid a second tab.
        let existing = match &opening {
            Opening::Blank
            | Opening::Import(_)
            | Opening::Pack { .. }
            | Opening::Theme { .. }
            | Opening::DeckPack { .. } => None,
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
        let (window, place, opener) = w.resolve_place(window, place);
        w.insert_tab(window.as_deref(), id.clone(), session, &place);
        if let Some(tab) = w.tabs.get_mut(&id) {
            tab.opener = opener;
        }
        let mut gone = Vec::new();
        if let TabPlace::Replace(r) = place {
            if let Err(e) = w.detach(&r) {
                tracing::warn!(tab = %r, error = %e.one_line(), "onglet vierge non retiré");
            }
            gone.extend(w.remove_tab(&r));
        }
        w.save_layout();
        let (json, trash) = (w.view_json(), w.trash());
        drop(w);
        trash.empty(gone);
        json.map(|json| (WorkspaceOutcome::Applied(json), id))
    }

    /// Where a built tab lands now: its window, its place and its opener.
    ///
    /// A vanished window gives the focused one (no opener); an `After` anchor
    /// that closed, or moved to another window, gives the end (an opener only
    /// if it still exists); a `Replace` tab no longer pristine gives the end.
    fn resolve_place(&self, window: Option<String>, place: TabPlace) -> (Option<String>, TabPlace, Option<String>) {
        let window = match window {
            Some(label) if self.window_index(&label).is_ok() => Some(label),
            // The window vanished while the tab was built: the focused one.
            Some(_) => {
                let focused = Some(self.layout.focused.clone()).filter(|l| self.window_index(l).is_ok());
                return (focused, TabPlace::End, None);
            }
            None => None,
        };
        let place = match place {
            TabPlace::After(anchor) => {
                return match self.tab_window(&anchor) {
                    Some(at) if window.as_deref() == Some(at) => {
                        (window, TabPlace::After(anchor.clone()), Some(anchor))
                    }
                    Some(_) => (window, TabPlace::End, Some(anchor)),
                    None => (window, TabPlace::End, None),
                };
            }
            // Still pristine? The session's real state decides, without waiting
            // under the lock: a busy session (a render running) is being changed,
            // so not pristine; a dormant session has not been reread, so nothing
            // proves it is.
            TabPlace::Replace(r) if self.is_pristine(&r) => TabPlace::Replace(r),
            TabPlace::Replace(_) | TabPlace::End => TabPlace::End,
        };
        (window, place, None)
    }

    /// Whether tab `r` is pristine now, read without waiting.
    fn is_pristine(&self, r: &str) -> bool {
        let pristine = |slot: &Slot| slot.info().is_some_and(|i| i.pristine);
        self.session(r).is_ok_and(|s| match s.try_lock() {
            Ok(slot) => pristine(&slot),
            Err(std::sync::TryLockError::Poisoned(p)) => pristine(&p.into_inner()),
            Err(std::sync::TryLockError::WouldBlock) => false,
        })
    }

    /// Réserve un numéro d'onglet et prépare sa construction hors du verrou.
    pub(super) fn pending(&mut self, window: Option<String>, opening: Opening, place: TabPlace) -> Pending {
        Pending {
            id: self.next_tab_id(),
            window,
            opening,
            place,
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
            | WorkspaceRequest::CarryAssets { .. }
            | WorkspaceRequest::Clip
            | WorkspaceRequest::SetClip { .. }
            | WorkspaceRequest::NewTab { .. }
            | WorkspaceRequest::OpenTab { .. }
            | WorkspaceRequest::ImportTab { .. }
            | WorkspaceRequest::OpenHelp { .. }
            | WorkspaceRequest::NewWindow
            | WorkspaceRequest::ImportPicked { .. }
            | WorkspaceRequest::OpenPack { .. }
            | WorkspaceRequest::OpenTheme { .. }
            | WorkspaceRequest::PackOpener { .. }
            // Handled in `apply` itself (flushed outside the lock first, the last
            // window decided under the same lock as the close): never reached here.
            | WorkspaceRequest::Close { .. }
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

    /// The tabs whose real state `req` must read.
    ///
    /// Closed without discarding, with the tabs linked to them, or the blank
    /// tab a file opened or imported would replace (if it still is).
    pub(super) fn to_wake(&self, req: &WorkspaceRequest) -> Vec<String> {
        match req {
            WorkspaceRequest::Close { tab, discard: false } => {
                std::iter::once(tab.clone()).chain(self.linked_tabs(tab)).collect()
            }
            // La seule fenêtre ne se ferme pas (l'app quitte en gardant tout) :
            // rien à relire, la reprise paresseuse reste paresseuse.
            WorkspaceRequest::CloseWindow { window, .. } if self.is_only_window(window) => Vec::new(),
            WorkspaceRequest::CloseWindow { window, discard } => {
                let (closing, discarded) = self.window_closing(window, discard);
                closing.into_iter().filter(|t| !discarded.contains(t)).collect()
            }
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
