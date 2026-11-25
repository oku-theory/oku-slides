//! The base store's collection at launch: the bases no tab knows any more.

use std::collections::BTreeSet;
use std::path::Path;

use oku_slides_data::BaseId;
use oku_slides_platform::storage::Storage;
use serde::Deserialize;

use super::{TABS_DIR, Workspace};
use crate::resume::{DECK_DIR, MAX_JSON_BYTES, PACK_DIR};

/// The part of a tab document's `document.json` the collection reads.
#[derive(Deserialize)]
struct KnownBases {
    #[serde(default)]
    bases: BTreeSet<BaseId>,
}

impl Workspace {
    /// Deletes the stored bases no tab folder knows, nor any session leases.
    ///
    /// The open sessions and the pastes hold leases; the roots are the bases every tab folder's
    /// documents know, set-aside folders included. An unreadable folder skips the collection:
    /// its bases are unknown, and never deleted on a guess.
    pub(super) fn collect_bases(&self, dir: &Path) {
        let one_line = |e: &dyn std::error::Error| oku_slides_platform::errors::one_line(e);
        let store = match self.platform.bases() {
            Ok(store) => store,
            Err(e) => return tracing::warn!(error = %one_line(&e), "base store unavailable, no collection"),
        };
        let roots = match resumed_bases(&self.platform.storage, &dir.join(TABS_DIR)) {
            Ok(roots) => roots,
            Err(e) => return tracing::warn!(error = %e, "tab bases unreadable, no collection"),
        };
        match store.gc(&roots) {
            Ok(0) => {}
            Ok(count) => tracing::info!(count, "unused data bases collected"),
            Err(e) => tracing::warn!(error = %one_line(&e), "data base collection failed"),
        }
    }
}

/// The bases known by the documents of every folder in `tabs`; a missing document adds none.
fn resumed_bases(storage: &Storage, tabs: &Path) -> Result<BTreeSet<BaseId>, String> {
    let mut roots = BTreeSet::new();
    let folders = storage.list_dirs(tabs).map_err(|e| oku_slides_platform::errors::one_line(&e))?;
    for folder in folders {
        for sub in [DECK_DIR, PACK_DIR] {
            let path = folder.join(sub).join("document.json");
            let found = storage.is_file(&path).map_err(|e| oku_slides_platform::errors::one_line(&e))?;
            if !found {
                continue;
            }
            let bytes =
                storage.read_limited(&path, MAX_JSON_BYTES).map_err(|e| oku_slides_platform::errors::one_line(&e))?;
            let known: KnownBases = serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
            roots.extend(known.bases);
        }
    }
    Ok(roots)
}
