//! Fichiers des documents : lire un `.echo.pdf` (ou un deck JSON), écrire
//! sans risque. Les dossiers de l'app sont dans les réglages
//! (`slider_platform::config`).

use std::path::Path;

use slider_core::SourceBundle;

use snafu::{OptionExt, ResultExt};

use crate::Result;
use crate::error::{DeckJsonSnafu, NotEchoSnafu, PdfSnafu, ReadSnafu, SourcesSnafu};

/// Extension d'un document Écho.
pub const EXTENSION: &str = ".echo.pdf";

/// Écrit dans un fichier temporaire à côté, puis renomme.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let tmp = dir.join(format!(".{}.tmp", path.file_name().unwrap_or_default().to_string_lossy()));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

/// Un document lu sur disque.
#[derive(Debug)]
pub struct Opened {
    pub sources: SourceBundle,
    /// Lu dans un `.echo.pdf` (sinon, un deck JSON de développement).
    pub pdf: bool,
    /// Le PDF a été modifié par un autre outil après Écho.
    pub modified_elsewhere: bool,
}

/// Lit un `.echo.pdf` (reconnu à son contenu) ou un deck JSON.
pub fn read_document(path: &Path) -> Result<Opened> {
    let bytes = std::fs::read(path).context(ReadSnafu { path })?;
    if bytes.starts_with(b"%PDF") {
        let found = slider_pdf::extract_sources(&bytes).context(PdfSnafu)?.context(NotEchoSnafu)?;
        let sources = SourceBundle::from_zip(&found.zip).context(SourcesSnafu)?;
        return Ok(Opened { sources, pdf: true, modified_elsewhere: found.modified_elsewhere() });
    }
    let deck = serde_json::from_slice(&bytes).context(DeckJsonSnafu)?;
    Ok(Opened { sources: SourceBundle::new(deck), pdf: false, modified_elsewhere: false })
}
