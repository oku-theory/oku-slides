//! Lecture des documents : un `.oku-slides.pdf` (ou un deck JSON). Les dossiers
//! de l'app sont dans les réglages (`oku_slides_platform::config`), les accès
//! aux fichiers dans `oku_slides_platform::storage`.

use std::path::Path;

use oku_slides_core::{Issue, SourceBundle, SourceLimits};
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
    /// Ce que la relecture du replica n'a pas pu lire (vide pour un deck JSON).
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
    pub(crate) fn notice(&self, l: &Localizer) -> Option<String> {
        for issue in &self.issues {
            tracing::warn!(issue = %issue.localize(l), "partie illisible mise de côté");
        }
        let unreadable = (!self.issues.is_empty())
            .then(|| l.tr_with("notice-unreadable-parts", &[("count", self.issues.len().into())]));
        let elsewhere = self.modified_elsewhere.then(|| l.tr("notice-modified-elsewhere"));
        match (elsewhere, unreadable) {
            (Some(a), Some(b)) => Some(format!("{a}\n{b}")),
            (a, b) => a.or(b),
        }
    }
}

/// Lit un `.oku-slides.pdf` (reconnu à son contenu) ou un deck JSON.
/// `limits` borne les sources lues dans le PDF (réglages `[sources]
/// max_replica_mib` et `max_sources_mib`).
pub fn read_document(storage: &Storage, path: &Path, limits: SourceLimits) -> Result<Opened> {
    let bytes = storage.read(path).context(ReadSnafu { path })?;
    let hash = oku_slides_core::asset_id(&bytes);
    parse_document(&bytes, hash, limits)
}

/// Les bornes des sources lues dans un PDF, d'après les réglages `[sources]`.
pub(crate) fn source_limits(settings: &Settings) -> SourceLimits {
    SourceLimits {
        replica_bytes: settings.sources.max_replica_bytes(),
        total_bytes: settings.sources.max_sources_bytes(),
    }
}

/// Le `deck_uid` du document `path` ; `None` pour un deck JSON (sans
/// fichier Oku Slides, il n'est la copie d'aucun autre).
pub fn deck_uid_of(storage: &Storage, path: &Path, limits: SourceLimits) -> Result<Option<String>> {
    let opened = read_document(storage, path, limits)?;
    Ok(opened.pdf.then(|| opened.sources.replica().deck_uid()))
}

/// Lit un document déjà chargé : `bytes`, d'empreinte `hash`.
pub(crate) fn parse_document(bytes: &[u8], hash: String, limits: SourceLimits) -> Result<Opened> {
    if bytes.starts_with(b"%PDF") {
        let found = oku_slides_pdf::extract_sources(bytes).context(PdfSnafu)?.context(NotOkuSlidesSnafu)?;
        let (sources, issues) = SourceBundle::from_zip(&found.zip, limits).context(SourcesSnafu)?;
        return Ok(Opened { sources, issues, pdf: true, modified_elsewhere: found.modified_elsewhere(), hash });
    }
    let deck = serde_json::from_slice(bytes).context(DeckJsonSnafu)?;
    let sources = SourceBundle::new(deck).context(SourcesSnafu)?;
    Ok(Opened { sources, issues: Vec::new(), pdf: false, modified_elsewhere: false, hash })
}

#[cfg(test)]
#[path = "files_tests.rs"]
mod files_tests;
