//! Les versions des packs épinglés par un deck JSON.
//!
//! Extraite à côté de lui (`open --out`) ou dans la bibliothèque, relue par `check` et `save`.

use std::path::Path;

use std::collections::BTreeMap;

use oku_slides_core::{Deck, Namespace, PackFiles, PackKind, PackPin, PackVersion};
use oku_slides_platform::errors::one_line;
use oku_slides_session::{PackLibrary, Storage, check_pack_id, read_pack_dir};

/// Les versions des packs que `deck` épingle, widgets puis layouts, par empreinte.
///
/// Chacune vient de [`deck_pack_version`]. La première introuvable est rendue en erreur
/// avec son espace de noms.
pub(crate) fn deck_pack_versions(
    storage: &Storage,
    library: &PackLibrary,
    deck_dir: (&Path, usize),
    deck: &Deck,
) -> Result<BTreeMap<PackVersion, PackFiles>, (Namespace, PackPin)> {
    let mut out = BTreeMap::new();
    let pins =
        deck.widgets.iter().map(|p| (PackKind::Widgets, p)).chain(deck.layouts.iter().map(|p| (PackKind::Layouts, p)));
    for (kind, (ns, pin)) in pins {
        let files =
            deck_pack_version(storage, library, deck_dir, (kind, ns), pin).ok_or_else(|| (ns.clone(), pin.clone()))?;
        out.insert(pin.version.clone(), files);
    }
    Ok(out)
}

/// La version épinglée `pin` sous `ns` par le deck, de sorte `kind`.
///
/// D'abord extraite à côté de lui (`<deck_dir>/widgets/<hex>` ou `<deck_dir>/layouts/<hex>`,
/// par `open --out`) si elle y est intacte, sinon celle de la bibliothèque. D'où qu'elle vienne,
/// son manifeste doit être celui du pack `pin.id` sous `ns` ; sinon elle est ignorée (journalisé).
/// La copie à côté du deck se lit en `max_bytes` octets au plus (`[sources] max_sources_mib`).
fn deck_pack_version(
    storage: &Storage,
    library: &PackLibrary,
    (deck_dir, max_bytes): (&Path, usize),
    (kind, ns): (PackKind, &Namespace),
    pin: &PackPin,
) -> Option<PackFiles> {
    let of_pack = |files: PackFiles, from: &str| match check_pack_id(ns, pin, &files) {
        Ok(()) => Some(files),
        Err(e) => {
            tracing::warn!(from, error = %one_line(&e), "version de pack d'un autre pack : ignorée");
            None
        }
    };
    beside_deck(storage, deck_dir.join(kind.files_dir()), max_bytes, pin)
        .and_then(|files| of_pack(files, "deck"))
        .or_else(|| library.version(pin.id, &pin.version).and_then(|files| of_pack(files, "library")))
}

/// La version `pin` extraite dans `kind_dir` (le dossier de sa sorte à côté du deck), si elle y est intacte.
///
/// Intacte : son contenu donne bien son empreinte ; illisible ou trop grosse, elle est ignorée
/// (journalisé).
fn beside_deck(storage: &Storage, kind_dir: std::path::PathBuf, max_bytes: usize, pin: &PackPin) -> Option<PackFiles> {
    let dir = kind_dir.join(pin.version.hex());
    let files = match read_pack_dir(storage, &dir, max_bytes) {
        Ok(files) => files?,
        Err(e) => {
            tracing::warn!(dir = %dir.display(), error = %e.one_line(), "pack à côté du deck illisible : ignoré");
            return None;
        }
    };
    if !files.matches(&pin.version) {
        tracing::warn!(dir = %dir.display(), "pack altéré à côté du deck : ignoré");
        return None;
    }
    Some(files)
}

#[cfg(test)]
#[path = "library_tests.rs"]
mod library_tests;
