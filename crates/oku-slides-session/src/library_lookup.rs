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

/// The files of the pack stored in `dir`, at most `max_bytes` bytes.
///
/// Either `widgets.json` and `widgets/<name>.typ`, or `layouts.json` and `layouts/<name>.typ`.
///
/// This is the layout of a pack version; `Ok(None)` without either manifest.
/// A file that cannot be read, a folder that cannot be listed: `File`;
/// beyond `max_bytes`: `TooLarge`, before reading the file too many;
/// a manifest that is not an object or a widget that is not text:
/// `BadPack`.
///
/// # Errors
///
/// Fails (`File`) on an unreadable file or folder, (`TooLarge`) beyond `max_bytes`, (`BadPack`)
/// on a malformed manifest or widget.
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

/// The library pack named by `wanted`: an id, a name, or else a namespace.
///
/// An id gives the current version; a name is that of a built-in or library
/// pack, case-insensitive; a namespace, that of the pack that holds it.
/// A user copy under a built-in namespace answers only to its id.
/// Never a folder: that is the app's target.
/// Not found: `UnknownPack`.
///
/// # Errors
///
/// Fails (`UnknownPack`) if no pack matches `wanted`.
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

/// The pack named by `wanted` for the CLI.
///
/// An id, a pack folder, or else a name.
///
/// Like [`library_pack`], plus a folder that has a `widgets.json` or a `layouts.json`, read by
/// [`read_pack_dir`] in at most `max_bytes` bytes.
///
/// # Errors
///
/// Fails if folder `wanted` cannot be read (see [`read_pack_dir`]), or if no pack matches
/// (`UnknownPack`).
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

/// Embeds in `bundle` the closure of the theme named by `wanted`, at current versions.
///
/// `wanted` is its id, its namespace, or else its name (see [`find_theme`]);
/// returns its namespace and the pins of its closure. Unknown:
/// `UnknownTheme`; a parent not found or a cycle: `Library`.
///
/// # Errors
///
/// Fails (`UnknownTheme`) for an unknown theme, (`Library`) for a parent not found or a cycle.
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
