//! La version du pack d'un deck JSON.
//!
//! Extraite à côté de lui (`open --out`) ou dans la bibliothèque, relue par `check` et `save`.

use std::path::Path;

use oku_slides_core::{PackFiles, PackRef};
use oku_slides_platform::errors::one_line;
use oku_slides_session::{PackLibrary, Storage, check_pack_id, read_pack_dir};

/// La version `r` du deck.
///
/// D'abord extraite à côté de lui (`<deck_dir>/packs/<hex>`, par `open --out`) si elle y est
/// intacte, sinon celle de la bibliothèque. D'où qu'elle vienne, son `pack.json` doit être celui du
/// pack `r.id` ; sinon elle est ignorée (journalisé). La copie à côté du deck se lit en `max_bytes`
/// octets au plus (`[sources] max_sources_mib`).
pub(crate) fn deck_pack_version(
    storage: &Storage,
    library: &PackLibrary,
    (deck_dir, max_bytes): (&Path, usize),
    r: &PackRef,
) -> Option<PackFiles> {
    let of_pack = |files: PackFiles, from: &str| match check_pack_id(r, &files) {
        Ok(()) => Some(files),
        Err(e) => {
            tracing::warn!(from, error = %one_line(&e), "version de pack d'un autre pack : ignorée");
            None
        }
    };
    beside_deck(storage, deck_dir, max_bytes, r)
        .and_then(|files| of_pack(files, "deck"))
        .or_else(|| library.version(r.id, &r.version).and_then(|files| of_pack(files, "library")))
}

/// La version `r` extraite à côté du deck, si elle y est intacte.
///
/// Intacte : son contenu donne bien son empreinte ; illisible ou trop grosse, elle est ignorée
/// (journalisé).
fn beside_deck(storage: &Storage, deck_dir: &Path, max_bytes: usize, r: &PackRef) -> Option<PackFiles> {
    let dir = deck_dir.join("packs").join(r.version.hex());
    let files = match read_pack_dir(storage, &dir, max_bytes) {
        Ok(files) => files?,
        Err(e) => {
            tracing::warn!(dir = %dir.display(), error = %e.one_line(), "pack à côté du deck illisible : ignoré");
            return None;
        }
    };
    if !files.matches(&r.version) {
        tracing::warn!(dir = %dir.display(), "pack altéré à côté du deck : ignoré");
        return None;
    }
    Some(files)
}

#[cfg(test)]
#[path = "library_tests.rs"]
mod library_tests;
