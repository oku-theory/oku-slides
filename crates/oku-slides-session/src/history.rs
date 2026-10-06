//! Historique du deck : lire, parcourir et restaurer une version.
//!
//! Comme pendant l'édition des modèles, le deck attend de côté (`stash`) ;
//! un document d'aperçu montre la version, en lecture seule. Images, polices
//! et packs passent d'un document à l'autre sans copie : chaque étape qui
//! peut échouer se fait avant de les déplacer, rien ne les perd.

use std::collections::BTreeMap;

use snafu::ResultExt;
use time::macros::format_description;
use time::{OffsetDateTime, UtcOffset};

use oku_slides_core::{HistoryPreview, PackFiles, PackVersion, SlideChanges, SourceBundle, Version, slide_changes};
use oku_slides_render_worker::RemoteCompiled;

use crate::error::{EditingTemplatesSnafu, HistoryNotOpenSnafu, InHistorySnafu, SourcesSnafu, VersionSnafu};
use crate::views::HistoryView;
use crate::{Document, Mode, Result, Session};

/// L'historique ouvert : les versions, et celle qui est montrée.
pub(crate) struct HistoryMode {
    preview: HistoryPreview,
    index: usize,
    version: Version,
    /// Slides de la version montrée comparées au deck mis de côté.
    changes: SlideChanges,
}

impl HistoryMode {
    pub(crate) fn view(&self) -> HistoryView<'_> {
        HistoryView { index: self.index, count: self.preview.len(), version: &self.version, changes: &self.changes }
    }
}

/// Versions de packs ou de thèmes, par empreinte.
type Versions = BTreeMap<PackVersion, PackFiles>;

/// Images, polices, packs et thèmes d'un document.
type Blobs = (BTreeMap<String, Vec<u8>>, BTreeMap<String, Vec<u8>>, Versions, Versions);

fn take_blobs(from: &mut Document) -> Blobs {
    let s = &mut from.sources;
    let (assets, fonts) = (std::mem::take(&mut s.assets), std::mem::take(&mut s.fonts));
    (assets, fonts, std::mem::take(&mut s.packs), std::mem::take(&mut s.themes))
}

fn put_blobs(to: &mut Document, (assets, fonts, packs, themes): Blobs) {
    to.sources.assets = assets;
    to.sources.fonts = fonts;
    to.sources.packs = packs;
    to.sources.themes = themes;
}

/// La version `index` de `preview` ; une version vide si la liste l'est.
fn version_at(preview: &HistoryPreview, index: usize) -> Version {
    preview.versions().get(index).cloned().unwrap_or(Version {
        author: None,
        timestamp: 0,
        label: None,
        session_start: false,
        merge: false,
    })
}

/// La date d'une version, à l'heure de `offset`, à la minute
/// (« AAAA-MM-JJ HH:MM ») ; hors des dates représentables : l'époque Unix.
pub(crate) fn restore_date(timestamp: i64, offset: UtcOffset) -> String {
    let at = OffsetDateTime::from_unix_timestamp(timestamp)
        .ok()
        .and_then(|t| t.checked_to_offset(offset))
        .unwrap_or(OffsetDateTime::UNIX_EPOCH);
    at.format(format_description!("[year]-[month]-[day] [hour]:[minute]")).unwrap_or_default()
}

impl Session {
    /// Libellé du commit de restauration : la date de la version, à l'heure locale.
    pub(crate) fn restore_label(&self, timestamp: i64) -> String {
        let offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
        self.localizer.tr_with("history-restore-label", &[("date", restore_date(timestamp, offset).into())])
    }

    /// Le document d'aperçu de la version `index`, et son rendu. Il prend les
    /// images et polices de `self.doc` une fois tout le reste réussi : en cas
    /// d'échec, `self.doc` les garde.
    fn version_document(&mut self, preview: &HistoryPreview, index: usize) -> Result<(Document, RemoteCompiled)> {
        let (deck, issues) = preview.deck_at(index).context(VersionSnafu)?;
        if !issues.is_empty() {
            tracing::debug!(index, issues = issues.len(), "version lue en partie");
        }
        let mut sources = SourceBundle::new(deck, self.registry.clone()).context(SourcesSnafu)?;
        let compiled = self.render_deck(&sources.deck().clone())?;
        // Un pack écarté pendant ce rendu : la base, pour ces sources aussi.
        sources.set_registry(self.registry.clone());
        let mut doc = Document::new(sources, self.doc.path.clone(), &self.doc_defaults);
        doc.dirty = self.doc.dirty;
        put_blobs(&mut doc, take_blobs(&mut self.doc));
        Ok((doc, compiled))
    }

    /// Sort du mode historique : le deck mis de côté et l'historique ; la
    /// session reste en mode deck jusqu'à ce que l'appelant la replace.
    fn take_history(&mut self) -> Result<(Box<Document>, HistoryMode)> {
        match std::mem::replace(&mut self.mode, Mode::Deck) {
            Mode::History { stash, history } => Ok((stash, history)),
            other => {
                self.mode = other;
                HistoryNotOpenSnafu.fail()?
            }
        }
    }

    /// Rend le deck mis de côté, avec les images et polices de l'aperçu, et
    /// l'avis que l'aperçu n'a pas encore montré.
    fn back_to_deck(&mut self, mut stash: Box<Document>) {
        put_blobs(&mut stash, take_blobs(&mut self.doc));
        let notice = self.doc.notice.take();
        self.doc = *stash;
        // Un pack écarté pendant l'historique : le deck mis de côté suit.
        self.doc.sources.set_registry(self.registry.clone());
        if let Some(notice) = notice {
            self.set_notice(notice);
        }
    }

    /// Refuse ce qui changerait le deck pendant l'historique (`InHistory`)
    /// ou dans l'aide (`ReadOnly`). `handle` filtre déjà les requêtes ;
    /// l'agent, lui, appelle les méthodes directement (`TabSession::with`) :
    /// chacune se garde avant tout changement.
    pub(crate) fn ensure_editable(&self) -> Result<()> {
        snafu::ensure!(!matches!(self.mode, Mode::History { .. }), InHistorySnafu);
        self.ensure_not_help()
    }

    /// Ouvre l'historique sur la dernière version ; le deck est mis de côté.
    pub fn history_open(&mut self) -> Result<()> {
        self.ensure_not_help()?;
        match self.mode {
            Mode::Deck => {}
            Mode::Templates { .. } => return EditingTemplatesSnafu.fail()?,
            Mode::History { .. } => return InHistorySnafu.fail()?,
        }
        let preview = self.doc.sources.history();
        let index = preview.len().saturating_sub(1);
        let (doc, compiled) = self.version_document(&preview, index)?;
        let changes = slide_changes(doc.sources.deck(), self.doc.sources.deck());
        let version = version_at(&preview, index);
        let stash = Box::new(std::mem::replace(&mut self.doc, doc));
        self.mode = Mode::History { stash, history: HistoryMode { preview, index, version, changes } };
        self.install(compiled);
        Ok(())
    }

    /// Montre la version `index` (bornée à la dernière).
    pub fn history_seek(&mut self, index: usize) -> Result<()> {
        let (stash, mut history) = self.take_history()?;
        let index = index.min(history.preview.len().saturating_sub(1));
        let shown = self.version_document(&history.preview, index);
        if let Ok((doc, _)) = &shown {
            history.changes = slide_changes(doc.sources.deck(), stash.sources.deck());
            history.version = version_at(&history.preview, index);
            history.index = index;
        }
        self.mode = Mode::History { stash, history };
        let (doc, compiled) = shown?;
        self.doc = doc;
        self.install(compiled);
        Ok(())
    }

    /// Ramène le deck à la version montrée, par un nouveau commit annulable,
    /// et ferme l'historique.
    pub fn history_restore(&mut self) -> Result<()> {
        let (mut stash, history) = self.take_history()?;
        let label = self.restore_label(history.version.timestamp);
        // En cas d'échec, le deck mis de côté est intact (sa vue est la
        // relecture de son replica, rien en attente) : l'historique reste ouvert.
        let issues = match stash.sources.restore(&history.preview, history.index, &label) {
            Ok(issues) => issues,
            Err(e) => {
                self.mode = Mode::History { stash, history };
                return Err(e).context(SourcesSnafu)?;
            }
        };
        // Comme à la fusion : la version est restaurée telle que l'aperçu l'a lue.
        // Parties illisibles journalisées comme avant ; les constats de
        // validation ne sont pas des pertes.
        crate::files::log_issues(&issues, &self.localizer, crate::files::IssuesAt::Restore);
        stash.dirty = true;
        self.back_to_deck(stash);
        // La version restaurée peut avoir un autre ensemble de modèles.
        self.sync_widgets()?;
        self.rerender()
    }

    /// Ferme l'historique : le deck revient tel qu'il était.
    pub fn history_close(&mut self) -> Result<()> {
        let (stash, _) = self.take_history()?;
        self.back_to_deck(stash);
        self.rerender()
    }

    /// Les versions du deck, dans l'ordre.
    pub fn history_versions(&self) -> Result<&[Version]> {
        match &self.mode {
            Mode::History { history, .. } => Ok(history.preview.versions()),
            _ => HistoryNotOpenSnafu.fail()?,
        }
    }
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod history_tests;
