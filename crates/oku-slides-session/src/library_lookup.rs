//! Packs et thèmes désignés par l'utilisateur.
//!
//! Un pack ou un thème trouvé par son id, son nom (ou, pour un pack, son espace
//! de noms) dans la bibliothèque, ou, pour la CLI
//! seulement, un pack lu dans un dossier, borné en taille ; un thème embarqué dans des sources.
//! L'import et la CLI passent par ici ; leurs erreurs sont celles de l'import
//! ([`ImportError`](crate::import::ImportError)).

use std::collections::BTreeMap;
use std::path::Path;

use snafu::{IntoError, OptionExt, ResultExt};

use oku_slides_core::{Namespace, PackFiles, SourceBundle, ThemeRef, WidgetFile};
use oku_slides_platform::storage::{Storage, StorageErrorKind};

use crate::import::error::{
    BadPackFilesSnafu, PackDirSnafu, PackTooLargeSnafu, Result, UnknownPackSnafu, UnknownThemeSnafu,
};
use crate::{PackLibrary, ThemeLibrary};

/// `n` octets en `u64`.
pub(crate) fn as_u64(n: usize) -> u64 {
    // Saturé, jamais atteint : `usize` tient dans `u64` sur toutes les cibles (64 bits au plus),
    // et les réglages en Mio sont déjà saturés en `usize`.
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// Les fichiers du pack rangé dans `dir`, `max_bytes` octets au plus.
///
/// Soit `widgets.json` et `widgets/<nom>.typ`.
///
/// C'est la disposition d'une version de pack ; `Ok(None)` sans `widgets.json`.
/// Un fichier qu'on ne lit pas, un dossier qu'on ne liste pas : `File` ;
/// au-delà de `max_bytes` : `TooLarge`, avant de lire le fichier de trop ;
/// un `widgets.json` qui n'est pas un objet ou un widget qui n'est pas du texte :
/// `BadPack`.
pub fn read_pack_dir(storage: &Storage, dir: &Path, max_bytes: usize) -> Result<Option<PackFiles>> {
    let manifest = dir.join("widgets.json");
    if storage.len(&manifest).context(PackDirSnafu { path: &manifest })?.is_none() {
        return Ok(None);
    }
    let max = as_u64(max_bytes);
    let mut left = max;
    let mut read = |path: &Path| -> Result<Vec<u8>> {
        let bytes = match storage.read_limited(path, left) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == StorageErrorKind::TooLarge => {
                return Err(PackTooLargeSnafu { path: dir, max }.build().into());
            }
            Err(e) => return Err(PackDirSnafu { path }.into_error(e).into()),
        };
        left = left.saturating_sub(as_u64(bytes.len()));
        Ok(bytes)
    };
    let pack_json = read(&manifest)?;
    let widgets_dir = dir.join("widgets");
    let listed = storage.list_files(&widgets_dir).context(PackDirSnafu { path: &widgets_dir })?;
    let mut widgets = BTreeMap::new();
    for path in listed {
        let Some(name) = path.file_name().and_then(|n| n.to_str()).and_then(WidgetFile::from_file_name) else {
            continue;
        };
        widgets.insert(name, read(&path)?);
    }
    let files = PackFiles::from_bytes(&pack_json, widgets.iter().map(|(n, b)| (n.clone(), b.as_slice())))
        .context(BadPackFilesSnafu { pack: dir.display().to_string() })?;
    Ok(Some(files))
}

/// Le pack de la bibliothèque désigné par `wanted` : un id, un nom, sinon un espace de noms.
///
/// Un id donne la version courante ; un nom est celui d'un pack intégré ou de
/// la bibliothèque, sans casse ; un espace de noms, celui du pack qui le porte.
/// Jamais un dossier : c'est la cible de l'app.
/// Introuvable : `UnknownPack`.
pub fn library_pack(library: &PackLibrary, wanted: &str) -> Result<PackFiles> {
    let unknown = || UnknownPackSnafu { pack: wanted };
    let id = match uuid::Uuid::parse_str(wanted) {
        Ok(id) => id,
        Err(_) => match library.packs().into_iter().find(|p| p.name.eq_ignore_ascii_case(wanted)) {
            Some(p) => p.id,
            None => Namespace::parse(wanted).and_then(|ns| library.by_namespace(&ns)).context(unknown())?.manifest.id,
        },
    };
    Ok(library.current_version(id).context(unknown())?.files)
}

/// Le pack désigné par `wanted` pour la CLI.
///
/// Un id, un dossier de pack, sinon un nom.
///
/// Comme [`library_pack`], plus un dossier qui a un `widgets.json`, lu par
/// [`read_pack_dir`] en `max_bytes` octets au plus.
pub fn resolve_pack(storage: &Storage, library: &PackLibrary, wanted: &str, max_bytes: usize) -> Result<PackFiles> {
    if uuid::Uuid::parse_str(wanted).is_err()
        && let Some(files) = read_pack_dir(storage, Path::new(wanted), max_bytes)?
    {
        return Ok(files);
    }
    library_pack(library, wanted)
}

/// Embarque dans `bundle` la version courante du thème désigné par `wanted`.
///
/// `wanted` est son id, sinon son nom (intégré ou de la bibliothèque, sans
/// casse) ; rend sa référence. Inconnu : `UnknownTheme`.
pub fn embed_theme(themes: &ThemeLibrary, wanted: &str, bundle: &mut SourceBundle) -> Result<ThemeRef> {
    let unknown = || UnknownThemeSnafu { theme: wanted };
    let id = match uuid::Uuid::parse_str(wanted) {
        Ok(id) => id,
        Err(_) => themes.themes().into_iter().find(|t| t.name.eq_ignore_ascii_case(wanted)).context(unknown())?.id,
    };
    let current = themes.current_version(id).context(unknown())?;
    Ok(ThemeRef { id, version: bundle.add_theme(current.files) })
}

#[cfg(test)]
#[path = "library_lookup_tests.rs"]
mod library_lookup_tests;
