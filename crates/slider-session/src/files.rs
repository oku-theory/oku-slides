//! Lecture des documents : un `.oku-slides.pdf` (ou un deck JSON). Les dossiers
//! de l'app sont dans les réglages (`slider_platform::config`), les accès
//! aux fichiers dans `slider_platform::storage`.

use std::path::Path;

use slider_core::SourceBundle;
use slider_platform::storage::Storage;

use snafu::{OptionExt, ResultExt};

use crate::Result;
use crate::error::{DeckJsonSnafu, NotOkuSlidesSnafu, PdfSnafu, ReadSnafu, SourcesSnafu};

/// Extension d'un document Oku Slides.
pub const EXTENSION: &str = ".oku-slides.pdf";

/// Un document lu sur disque.
#[derive(Debug)]
pub struct Opened {
    pub sources: SourceBundle,
    /// Lu dans un `.oku-slides.pdf` (sinon, un deck JSON de développement).
    pub pdf: bool,
    /// Le PDF a été modifié par un autre outil après Oku Slides.
    pub modified_elsewhere: bool,
}

/// Lit un `.oku-slides.pdf` (reconnu à son contenu) ou un deck JSON.
pub fn read_document(storage: &Storage, path: &Path) -> Result<Opened> {
    let bytes = storage.read(path).context(ReadSnafu { path })?;
    if bytes.starts_with(b"%PDF") {
        let found = slider_pdf::extract_sources(&bytes).context(PdfSnafu)?.context(NotOkuSlidesSnafu)?;
        let sources = SourceBundle::from_zip(&found.zip).context(SourcesSnafu)?;
        return Ok(Opened { sources, pdf: true, modified_elsewhere: found.modified_elsewhere() });
    }
    let deck = serde_json::from_slice(&bytes).context(DeckJsonSnafu)?;
    Ok(Opened { sources: SourceBundle::new(deck), pdf: false, modified_elsewhere: false })
}
