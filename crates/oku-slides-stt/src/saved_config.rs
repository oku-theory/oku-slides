//! The dictation settings file: read once at launch, an unreadable one kept aside.

use std::path::{Path, PathBuf};

use oku_slides_platform::errors::one_line;
use oku_slides_platform::storage::{Storage, StorageErrorKind};

use crate::Config;

/// Most `-<n>` suffixes tried for the kept copy before giving up.
const MAX_SUFFIX: u32 = 100;

/// `config.json` in the models folder.
pub(crate) fn path(dir: &Path) -> PathBuf {
    dir.join("config.json")
}

/// The settings read at launch, and whether saving may replace the file.
#[derive(Debug)]
pub(crate) struct Loaded {
    pub(crate) config: Config,
    /// False while a file we could not read or keep aside is still there (t-1216).
    pub(crate) savable: bool,
}

/// The saved settings, or the defaults when the file is missing or invalid.
///
/// A file that does not parse is renamed `config.json.invalid-<secs>[-<n>]` first. If it
/// cannot be read or renamed, `savable` is false so `configure` never writes over it. The
/// log names the line and column, never serde's message: it can quote a literal header
/// value, which may be a secret.
pub(crate) fn load(storage: &Storage, dir: &Path) -> Loaded {
    let file = path(dir);
    let defaults = |savable| Loaded { config: Config::default(), savable };
    let bytes = match storage.read(&file) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == StorageErrorKind::NotFound => return defaults(true),
        Err(e) => {
            let error = one_line(&e);
            tracing::warn!(path = %file.display(), %error, "réglages de la dictée illisibles");
            return defaults(false);
        }
    };
    match serde_json::from_slice(&bytes) {
        Ok(config) => Loaded { config, savable: true },
        Err(e) => {
            let kept = keep_aside(storage, &file);
            tracing::warn!(
                path = %file.display(),
                line = e.line(),
                column = e.column(),
                kind = ?e.classify(),
                kept = ?kept,
                "réglages de la dictée invalides, réglages par défaut"
            );
            defaults(kept.is_some())
        }
    }
}

/// Renames `file` to a free `<name>.invalid-<secs>[-<n>]` beside it; `None` if that fails.
fn keep_aside(storage: &Storage, file: &Path) -> Option<PathBuf> {
    // Before the epoch (a broken clock) the name ends in `-0`: still unique thanks to `-<n>`.
    let secs =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or_default();
    let name = file.file_name()?.to_string_lossy().into_owned();
    let base = format!("{name}.invalid-{secs}");
    // Checked then renamed: another writer could take the name in between, which only
    // matters at launch with two app instances on the same data folder.
    let mut found = None;
    for n in 0..=MAX_SUFFIX {
        let to = file.with_file_name(if n == 0 { base.clone() } else { format!("{base}-{n}") });
        match storage.exists(&to) {
            Ok(true) => {}
            Ok(false) => {
                found = Some(to);
                break;
            }
            Err(e) => {
                let error = one_line(&e);
                tracing::warn!(path = %to.display(), %error, "réglages de la dictée non mis de côté");
                return None;
            }
        }
    }
    let Some(to) = found else {
        tracing::warn!(path = %file.display(), "réglages de la dictée non mis de côté : aucun nom libre");
        return None;
    };
    match storage.rename(file, &to) {
        Ok(()) => Some(to),
        Err(e) => {
            let error = one_line(&e);
            tracing::warn!(path = %file.display(), %error, "réglages de la dictée non mis de côté");
            None
        }
    }
}

#[cfg(test)]
#[path = "saved_config_tests.rs"]
mod saved_config_tests;
