//! Lecture des documents : un `.oku-slides.pdf` (ou un deck JSON). Les dossiers
//! de l'app sont dans les réglages (`oku_slides_platform::config`), les accès
//! aux fichiers dans `oku_slides_platform::storage`.

use std::path::Path;
use std::sync::Arc;

use oku_slides_core::{Issue, SourceBundle, SourceLimits, WidgetRegistry};
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_platform::config::Settings;
use oku_slides_platform::storage::Storage;

use snafu::{OptionExt, ResultExt};

use crate::Result;
use crate::error::{DeckJsonSnafu, NotOkuSlidesSnafu, PdfSnafu, ReadSnafu, SourcesSnafu};

/// Extension d'un document Oku Slides.
pub const EXTENSION: &str = ".oku-slides.pdf";

/// Un document lu sur disque.
#[derive(Debug)]
pub struct Opened {
    pub sources: SourceBundle,
    /// Ce que la relecture du replica n'a pas pu lire (vide pour un deck
    /// JSON). Seules les parties écartées sont des pertes ([`is_unreadable`]).
    pub issues: Vec<Issue>,
    /// Lu dans un `.oku-slides.pdf` (sinon, un deck JSON de développement).
    pub pdf: bool,
    /// Le PDF a été modifié par un autre outil après Oku Slides.
    pub modified_elsewhere: bool,
    /// Empreinte du fichier lu (`sha256:…`).
    pub hash: String,
}

impl Opened {
    /// L'avis à montrer à l'ouverture : PDF modifié par un autre outil,
    /// parties illisibles mises de côté (une ligne chacun). Les parties
    /// illisibles sont aussi journalisées, une par une.
    /// Un constat de validation (voir [`is_unreadable`]) n'est qu'une ligne
    /// du journal : rien n'a été perdu.
    pub(crate) fn notice(&self, l: &Localizer) -> Option<String> {
        let count = log_issues(&self.issues, l, IssuesAt::Open);
        let unreadable = (count > 0).then(|| l.tr_with("notice-unreadable-parts", &[("count", count.into())]));
        let elsewhere = self.modified_elsewhere.then(|| l.tr("notice-modified-elsewhere"));
        match (elsewhere, unreadable) {
            (Some(a), Some(b)) => Some(format!("{a}\n{b}")),
            (a, b) => a.or(b),
        }
    }
}

/// Une partie mise de côté à la lecture : une perte. Soit illisible
/// (`issue-unreadable-*`), soit un slot trop profond (`issue-slot-too-deep`)
/// dont la relecture a écarté les éléments, soit une version de pack ou de
/// thème écartée (`issue-pack-version-mismatch`, `issue-theme-version-mismatch`).
/// Tout autre problème est un constat de `validate` (paramètre inconnu, asset
/// manquant…), que la fusion et la restauration rendent aussi : le deck est lu en entier.
///
/// `validate` signale aussi `issue-slot-too-deep`, mais jamais sur un deck
/// relu : la relecture borne la profondeur comme lui (`MAX_SLOT_DEPTH`).
/// Ici, il vient donc toujours de la lecture.
pub(crate) fn is_unreadable(issue: &Issue) -> bool {
    let id = issue.message.id();
    id.starts_with("issue-unreadable-")
        || matches!(id, "issue-slot-too-deep" | "issue-pack-version-mismatch" | "issue-theme-version-mismatch")
}

/// Où des problèmes de lecture ont été rendus (message du journal).
#[derive(Debug, Clone, Copy)]
pub(crate) enum IssuesAt {
    Open,
    Merge,
    Restore,
}

/// Journalise `issues` : les parties illisibles en avertissement, les
/// constats de validation en information. Renvoie le nombre de parties
/// illisibles.
pub(crate) fn log_issues(issues: &[Issue], l: &Localizer, at: IssuesAt) -> usize {
    let mut unreadable = 0;
    for issue in issues {
        let text = issue.localize(l);
        match (is_unreadable(issue), at) {
            (true, IssuesAt::Open) => tracing::warn!(issue = %text, "partie illisible mise de côté"),
            (true, IssuesAt::Merge) => tracing::warn!(issue = %text, "partie illisible après fusion"),
            (true, IssuesAt::Restore) => tracing::warn!(issue = %text, "partie illisible de la version restaurée"),
            (false, _) => tracing::info!(issue = %text, ?at, "constat de validation"),
        }
        unreadable += usize::from(is_unreadable(issue));
    }
    unreadable
}

/// Lit un `.oku-slides.pdf` (reconnu à son contenu) ou un deck JSON.
/// `limits` borne les sources lues dans le PDF (réglages `[sources]
/// max_replica_mib` et `max_sources_mib`) ; `registry` : les widgets avec
/// lesquels le document s'écrit ensuite (la lecture n'en dépend pas).
pub fn read_document(
    storage: &Storage,
    path: &Path,
    limits: SourceLimits,
    registry: &Arc<WidgetRegistry>,
) -> Result<Opened> {
    let bytes = storage.read(path).context(ReadSnafu { path })?;
    let hash = oku_slides_core::asset_id(&bytes);
    parse_document(&bytes, hash, limits, registry)
}

/// Les bornes des sources lues dans un PDF, d'après les réglages `[sources]`.
pub(crate) fn source_limits(settings: &Settings) -> SourceLimits {
    SourceLimits {
        replica_bytes: settings.sources.max_replica_bytes(),
        total_bytes: settings.sources.max_sources_bytes(),
        compact_after_secs: settings.sources.compact_after_secs(),
    }
}

/// Le `deck_uid` du document `path` ; `None` pour un deck JSON (sans
/// fichier Oku Slides, il n'est la copie d'aucun autre).
pub fn deck_uid_of(
    storage: &Storage,
    path: &Path,
    limits: SourceLimits,
    registry: &Arc<WidgetRegistry>,
) -> Result<Option<String>> {
    let opened = read_document(storage, path, limits, registry)?;
    Ok(opened.pdf.then(|| opened.sources.replica().deck_uid()))
}

/// Lit un document déjà chargé : `bytes`, d'empreinte `hash`.
pub(crate) fn parse_document(
    bytes: &[u8],
    hash: String,
    limits: SourceLimits,
    registry: &Arc<WidgetRegistry>,
) -> Result<Opened> {
    if bytes.starts_with(b"%PDF") {
        let found = oku_slides_pdf::extract_sources(bytes).context(PdfSnafu)?.context(NotOkuSlidesSnafu)?;
        let (sources, issues) = SourceBundle::from_zip(&found.zip, limits, registry.clone()).context(SourcesSnafu)?;
        return Ok(Opened { sources, issues, pdf: true, modified_elsewhere: found.modified_elsewhere(), hash });
    }
    let deck = serde_json::from_slice(bytes).context(DeckJsonSnafu)?;
    let sources = SourceBundle::new(deck, registry.clone()).context(SourcesSnafu)?;
    Ok(Opened { sources, issues: Vec::new(), pdf: false, modified_elsewhere: false, hash })
}

#[cfg(test)]
#[path = "files_tests.rs"]
mod files_tests;
