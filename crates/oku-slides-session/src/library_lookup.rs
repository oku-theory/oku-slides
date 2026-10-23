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

use oku_slides_core::{Namespace, PackFiles, PackKind, SourceBundle, ThemePin, WidgetFile};
use oku_slides_platform::storage::{Storage, StorageErrorKind};

use crate::import::error::{
    BadPackFilesSnafu, LibrarySnafu, PackDirSnafu, PackTooLargeSnafu, Result, UnknownPackSnafu, UnknownThemeSnafu,
};
use crate::library::may_own;
use crate::theme_closure::theme_closure;
use crate::{PackLibrary, ThemeLibrary};

/// `n` octets en `u64`.
pub(crate) fn as_u64(n: usize) -> u64 {
    // Saturé, jamais atteint : `usize` tient dans `u64` sur toutes les cibles (64 bits au plus),
    // et les réglages en Mio sont déjà saturés en `usize`.
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// Les fichiers du pack rangé dans `dir`, `max_bytes` octets au plus.
///
/// Soit `widgets.json` et `widgets/<nom>.typ`, soit `layouts.json` et `layouts/<nom>.typ`.
///
/// C'est la disposition d'une version de pack ; `Ok(None)` sans l'un ni l'autre manifeste.
/// Un fichier qu'on ne lit pas, un dossier qu'on ne liste pas : `File` ;
/// au-delà de `max_bytes` : `TooLarge`, avant de lire le fichier de trop ;
/// un manifeste qui n'est pas un objet ou un widget qui n'est pas du texte :
/// `BadPack`.
pub fn read_pack_dir(storage: &Storage, dir: &Path, max_bytes: usize) -> Result<Option<PackFiles>> {
    let mut found = None;
    for kind in PackKind::ALL {
        let manifest = dir.join(kind.manifest_file());
        if storage.len(&manifest).context(PackDirSnafu { path: &manifest })?.is_some() {
            found = Some((kind, manifest));
            break;
        }
    }
    let Some((kind, manifest)) = found else { return Ok(None) };
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
    let widgets_dir = dir.join(kind.files_dir());
    let listed = storage.list_files(&widgets_dir).context(PackDirSnafu { path: &widgets_dir })?;
    let mut widgets = BTreeMap::new();
    for path in listed {
        let Some(name) = path.file_name().and_then(|n| n.to_str()).and_then(WidgetFile::from_file_name) else {
            continue;
        };
        widgets.insert(name, read(&path)?);
    }
    let files = PackFiles::from_bytes(kind, &pack_json, widgets.iter().map(|(n, b)| (n.clone(), b.as_slice())))
        .context(BadPackFilesSnafu { pack: dir.display().to_string() })?;
    Ok(Some(files))
}

/// Le pack de la bibliothèque désigné par `wanted` : un id, un nom, sinon un espace de noms.
///
/// Un id donne la version courante ; un nom est celui d'un pack intégré ou de
/// la bibliothèque, sans casse ; un espace de noms, celui du pack qui le porte.
/// Une copie de l'utilisateur sous un espace de noms intégré ne répond qu'à son id.
/// Jamais un dossier : c'est la cible de l'app.
/// Introuvable : `UnknownPack`.
pub fn library_pack(library: &PackLibrary, wanted: &str) -> Result<PackFiles> {
    let unknown = || UnknownPackSnafu { pack: wanted };
    let id = match uuid::Uuid::parse_str(wanted) {
        Ok(id) => id,
        Err(_) => match PackKind::ALL
            .into_iter()
            .flat_map(|k| library.packs(k))
            .find(|p| p.name.eq_ignore_ascii_case(wanted) && may_own(p.id, &p.namespace))
        {
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
/// Comme [`library_pack`], plus un dossier qui a un `widgets.json` ou un `layouts.json`, lu par
/// [`read_pack_dir`] en `max_bytes` octets au plus.
pub fn resolve_pack(storage: &Storage, library: &PackLibrary, wanted: &str, max_bytes: usize) -> Result<PackFiles> {
    if uuid::Uuid::parse_str(wanted).is_err()
        && let Some(files) = read_pack_dir(storage, Path::new(wanted), max_bytes)?
    {
        return Ok(files);
    }
    library_pack(library, wanted)
}

/// L'espace de noms du thème de `themes` désigné par `wanted`.
///
/// Son id, son espace de noms, sinon son nom (intégré ou de la bibliothèque, sans casse).
pub(crate) fn find_theme(themes: &ThemeLibrary, wanted: &str) -> Option<Namespace> {
    if let Ok(id) = uuid::Uuid::parse_str(wanted) {
        return themes.current_version(id).map(|s| s.manifest.namespace);
    }
    let by_namespace = Namespace::parse(wanted).filter(|ns| themes.by_namespace(ns).is_some());
    by_namespace
        .or_else(|| themes.themes().into_iter().find(|t| t.name.eq_ignore_ascii_case(wanted)).map(|t| t.namespace))
}

/// Embarque dans `bundle` la fermeture du thème désigné par `wanted`, versions courantes.
///
/// `wanted` est son id, son espace de noms, sinon son nom (voir [`find_theme`]) ;
/// rend son espace de noms et les épinglages de sa fermeture. Inconnu :
/// `UnknownTheme` ; un parent introuvable ou une boucle : `Library`.
pub fn embed_theme(
    themes: &ThemeLibrary,
    wanted: &str,
    bundle: &mut SourceBundle,
) -> Result<(Namespace, BTreeMap<Namespace, ThemePin>)> {
    let ns = find_theme(themes, wanted).context(UnknownThemeSnafu { theme: wanted })?;
    let (pins, files) = theme_closure(themes, &ns).context(LibrarySnafu)?;
    for f in files {
        bundle.add_theme(f);
    }
    Ok((ns, pins))
}

#[cfg(test)]
#[path = "library_lookup_tests.rs"]
mod library_lookup_tests;
