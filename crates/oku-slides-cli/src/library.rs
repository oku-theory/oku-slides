//! Packs et thèmes pour la CLI : un pack lu dans un dossier, un pack ou un
//! thème désigné par son id, son dossier ou son nom, embarqué dans des sources.

use std::collections::BTreeMap;
use std::path::Path;

use color_eyre::eyre::{Result, eyre};

use oku_slides_core::{PackFiles, PackRef, SourceBundle, ThemeRef, WidgetFile};
use oku_slides_i18n::Localizer;
use oku_slides_platform::Platform;
use oku_slides_platform::errors::one_line;
use oku_slides_session::{PackLibrary, Storage, ThemeLibrary, check_pack_id};

/// Le contenu d'un fichier existant, `None` s'il manque ou ne se lit pas (journalisé).
fn read_existing(storage: &Storage, path: &Path) -> Option<Vec<u8>> {
    match storage.len(path) {
        Ok(Some(_)) => storage
            .read(path)
            .inspect_err(|e| tracing::warn!(path = %path.display(), error = %e, "fichier du pack illisible"))
            .ok(),
        _ => None,
    }
}

/// Les fichiers du pack rangé dans `dir` (`pack.json`, `widgets/<nom>.typ`),
/// la disposition d'une version de pack ; `None` sans `pack.json` lisible.
pub(crate) fn read_pack_dir(storage: &Storage, dir: &Path) -> Option<PackFiles> {
    let mut widgets = BTreeMap::new();
    let listed = storage.list_files(&dir.join("widgets")).unwrap_or_default();
    for path in listed {
        let Some(name) = path.file_name().and_then(|n| n.to_str()).and_then(WidgetFile::from_file_name) else {
            continue;
        };
        widgets.insert(name, read_existing(storage, &path)?);
    }
    PackFiles::from_bytes(
        &read_existing(storage, &dir.join("pack.json"))?,
        widgets.iter().map(|(n, b)| (n.clone(), b.as_slice())),
    )
}

/// La version `r` du deck : d'abord extraite à côté de lui
/// (`<deck_dir>/packs/<hex>`, par `open --out`) si elle y est intacte, sinon
/// celle de la bibliothèque. D'où qu'elle vienne, son `pack.json` doit être
/// celui du pack `r.id` ; sinon elle est ignorée (journalisé).
pub(crate) fn deck_pack_version(
    storage: &Storage,
    library: &PackLibrary,
    deck_dir: &Path,
    r: &PackRef,
) -> Option<PackFiles> {
    let of_pack = |files: PackFiles, from: &str| match check_pack_id(r, &files) {
        Ok(()) => Some(files),
        Err(e) => {
            tracing::warn!(from, error = %one_line(&e), "version de pack d'un autre pack : ignorée");
            None
        }
    };
    beside_deck(storage, deck_dir, r)
        .and_then(|files| of_pack(files, "deck"))
        .or_else(|| library.version(r.id, &r.version).and_then(|files| of_pack(files, "library")))
}

/// La version `r` extraite à côté du deck, si elle s'y trouve et que son
/// contenu donne bien son empreinte.
fn beside_deck(storage: &Storage, deck_dir: &Path, r: &PackRef) -> Option<PackFiles> {
    let dir = deck_dir.join("packs").join(r.version.hex());
    let files = read_pack_dir(storage, &dir)?;
    if !files.matches(&r.version) {
        tracing::warn!(dir = %dir.display(), "pack altéré à côté du deck : ignoré");
        return None;
    }
    Some(files)
}

/// Le pack désigné par `wanted` : un id (version courante de la bibliothèque),
/// un dossier de pack (`pack.json`, `widgets/`), sinon un nom de pack (intégré
/// ou de la bibliothèque, sans casse). Introuvable : erreur localisée.
pub(crate) fn resolve_pack(storage: &Storage, library: &PackLibrary, l: &Localizer, wanted: &str) -> Result<PackFiles> {
    let unknown = || eyre!(l.tr_with("cli-import-unknown-pack", &[("pack", wanted.into())]));
    if let Ok(id) = uuid::Uuid::parse_str(wanted) {
        return library.current_version(id).map(|s| s.files).ok_or_else(unknown);
    }
    let dir = Path::new(wanted);
    if matches!(storage.len(&dir.join("pack.json")), Ok(Some(_))) {
        return read_pack_dir(storage, dir).ok_or_else(unknown);
    }
    let info = library.packs().into_iter().find(|p| p.name.eq_ignore_ascii_case(wanted)).ok_or_else(unknown)?;
    library.current_version(info.id).map(|s| s.files).ok_or_else(unknown)
}

/// Embarque dans `bundle` la version courante du thème désigné par `wanted` :
/// son id, sinon son nom (intégré ou de la bibliothèque, sans casse) ; sa
/// référence. Inconnu : erreur localisée.
pub(crate) fn embed_theme(
    themes: &ThemeLibrary,
    l: &Localizer,
    wanted: &str,
    bundle: &mut SourceBundle,
) -> Result<ThemeRef> {
    let unknown = || eyre!(l.tr_with("session-unknown-theme", &[("theme", wanted.into())]));
    let id = match uuid::Uuid::parse_str(wanted) {
        Ok(id) => id,
        Err(_) => themes.themes().into_iter().find(|t| t.name.eq_ignore_ascii_case(wanted)).ok_or_else(unknown)?.id,
    };
    let current = themes.current_version(id).ok_or_else(unknown)?;
    Ok(ThemeRef { id, version: bundle.add_theme(current.files) })
}

/// La bibliothèque des thèmes de l'utilisateur (et les intégrés).
pub(crate) fn theme_library(platform: &Platform) -> ThemeLibrary {
    ThemeLibrary::new(Some(platform.settings.paths.themes()), platform.storage.clone())
}

/// La bibliothèque des packs de l'utilisateur (et les intégrés).
pub(crate) fn pack_library(platform: &Platform) -> PackLibrary {
    PackLibrary::new(Some(platform.settings.paths.templates()), platform.storage.clone())
}

#[cfg(test)]
#[path = "library_tests.rs"]
mod library_tests;
