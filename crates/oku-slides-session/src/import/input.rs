//! La présentation lue.
//!
//! Sa taille vue avant toute lecture, ses octets, sa source et les bornes de l'import. La lecture
//! se fait sur un fil dédié à pile connue ([`read_on_own_thread`]) ; une panique du lecteur y
//! devient une erreur localisée de l'import, jamais une panique de l'app.

use std::path::Path;
use std::sync::Arc;

use snafu::{IntoError, ResultExt};

use oku_slides_import::source::SourceDeck;
use oku_slides_import::{ReadLimits, read_pptx};
use oku_slides_platform::Platform;
use oku_slides_platform::storage::StorageErrorKind;

use super::error::{PresentationSnafu, ReadSnafu, ReadThreadSnafu, ReaderPanicSnafu, Result, TooLargeSnafu};
use crate::library_lookup::as_u64;

/// La présentation lue, et les bornes de son import.
///
/// Ses octets servent à l'identité du hi-fi et au témoin ; le budget de
/// sortie de `limits` borne le plan.
#[derive(Debug)]
pub(crate) struct Input {
    /// Les octets du `.pptx`, partagés avec le rendu de référence.
    pub(crate) bytes: Arc<[u8]>,
    /// Sa source lue.
    pub(crate) source: SourceDeck,
    pub(crate) limits: ReadLimits,
}

/// Lit `path`, borné par [`ReadLimits::default`].
///
/// La sortie l'est, approximativement, par la taille des sources et de
/// l'historique qu'un deck peut avoir pour se rouvrir.
pub(crate) fn read_input(platform: &Platform, path: &Path) -> Result<Input> {
    let sources = &platform.settings.sources;
    let limits = ReadLimits {
        max_output_bytes: as_u64(sources.max_sources_bytes()),
        max_replica_bytes: as_u64(sources.max_replica_bytes()),
        ..ReadLimits::default()
    };
    // Taille vue avant toute lecture : un fichier trop gros n'entre jamais en mémoire.
    let bytes = match platform.storage.read_limited(path, limits.max_input_bytes) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == StorageErrorKind::TooLarge => return Err(TooLargeSnafu.build().into()),
        Err(e) => return Err(ReadSnafu { path }.into_error(e).into()),
    };
    let source = read_source(path, &bytes, &limits, read_pptx)?;
    Ok(Input { bytes: bytes.into(), source, limits })
}

/// Le lecteur d'une présentation : [`read_pptx`], ou celui d'un test.
pub(crate) type Reader = fn(&[u8], &ReadLimits) -> std::result::Result<SourceDeck, oku_slides_import::ImportError>;

/// La source de `bytes` (le `.pptx` `path`) lue par `reader` sur son fil.
///
/// Une panique du lecteur est journalisée avec son message et devient
/// `ReaderPanic` : un bogue du lecteur sur ce fichier, pas un plantage.
pub(crate) fn read_source(path: &Path, bytes: &[u8], limits: &ReadLimits, reader: Reader) -> Result<SourceDeck> {
    match read_on_own_thread(bytes, limits, reader).context(ReadThreadSnafu { path })? {
        Ok(read) => Ok(read.context(PresentationSnafu)?),
        Err(_) => Ok(ReaderPanicSnafu { path }.fail()?),
    }
}

/// Pile du fil de lecture.
///
/// La lecture est bornée en profondeur (XML, groupes), et cette pile ne dépend ni du fil appelant
/// ni d'un runtime.
const READ_STACK_BYTES: usize = 16 << 20;

/// `reader` sur un fil dédié à pile connue ([`READ_STACK_BYTES`]).
///
/// L'erreur externe est celle du lancement du fil ; `Err` au milieu : la
/// lecture a paniqué (son message est journalisé).
pub(crate) fn read_on_own_thread(
    bytes: &[u8],
    limits: &ReadLimits,
    reader: Reader,
) -> std::io::Result<std::thread::Result<std::result::Result<SourceDeck, oku_slides_import::ImportError>>> {
    std::thread::scope(|scope| {
        let thread = std::thread::Builder::new()
            .name("import-read".to_owned())
            .stack_size(READ_STACK_BYTES)
            .spawn_scoped(scope, || reader(bytes, limits))?;
        Ok(thread.join().inspect_err(|panic| {
            let message = panic
                .downcast_ref::<&str>()
                .map(|s| (*s).to_owned())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_default();
            tracing::error!(%message, "le lecteur de la présentation a paniqué");
        }))
    })
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod input_tests;
