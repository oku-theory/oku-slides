//! Le contenu d'un thème (`theme.json`), sa sorte de bibliothèque et ses thèmes intégrés.
//!
//! Un thème est une version de jetons de conception (couleurs, polices,
//! tailles…) sous un espace de noms, qui en étend d'autres (`extends`) ; il
//! n'a pas de fichiers de widgets. Papier, Pro et Neutre sont livrés avec
//! l'app, en lecture seule. Un thème de l'utilisateur que le manifeste actuel
//! ne lit pas (d'avant les espaces de noms) est écarté et compté
//! ([`crate::Library::skipped`]).

use std::sync::LazyLock;

use serde::Serialize;
use snafu::{OptionExt, ResultExt};
use uuid::Uuid;

pub use oku_slides_core::ThemeManifest;
use oku_slides_core::{Namespace, PackFiles, PackVersion};

use crate::error::{ThemeJsonSnafu, UnknownThemeSnafu};
use crate::library::Library;
use crate::library_kind::{LibraryKind, StoredVersion, VersionManifest};

/// Thème intégré Papier : id constant.
pub const PAPIER_THEME: Uuid = Uuid::from_u128(0x2a7d4c91_5e3b_4f08_b6a1_8c9e0d1f2a31);
/// Thème intégré Pro : id constant.
pub const PRO_THEME: Uuid = Uuid::from_u128(0x7e1b9c3d_2f4a_4d6e_a8b0_5c7d9e1f3b42);
/// Thème intégré Neutre, celui des imports hi-fi : id constant.
pub const NEUTRE_THEME: Uuid = Uuid::from_u128(0x9c4e2b7a_1d3f_4a6b_8e5c_2f7a0b9d4c61);
/// Thème d'un deck neuf.
pub const DEFAULT_THEME: Uuid = PAPIER_THEME;
/// Espace de noms du thème intégré Papier, celui d'un deck neuf.
pub const PAPIER_NAMESPACE: &str = "papier";
/// Espace de noms du thème intégré Pro.
pub const PRO_NAMESPACE: &str = "pro";
/// Espace de noms du thème intégré Neutre.
pub const NEUTRE_NAMESPACE: &str = "neutre";

/// L'espace de noms intégré `name` (une des constantes ci-dessus).
///
/// Invalide (une constante mal écrite, vue par les tests) : `UnknownTheme`.
pub(crate) fn builtin_namespace(name: &str) -> crate::Result<Namespace> {
    Ok(Namespace::parse(name).context(UnknownThemeSnafu { theme: name })?)
}

/// L'espace de noms du thème d'un deck neuf ([`PAPIER_NAMESPACE`]).
pub(crate) fn default_namespace() -> crate::Result<Namespace> {
    builtin_namespace(PAPIER_NAMESPACE)
}

impl VersionManifest for ThemeManifest {
    fn from_files(files: &PackFiles) -> crate::Result<Self> {
        let read = Self::from_files(files);
        // L'id se lit à part : un manifeste refusé a peut-être un id lisible.
        let id = files.manifest().get("id").and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok());
        Ok(read.context(ThemeJsonSnafu { id: id.unwrap_or_default() })?)
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn parent(&self) -> Option<&PackVersion> {
        self.parent.as_ref()
    }

    fn created(&self) -> &str {
        &self.created
    }

    fn namespace(&self) -> Option<&Namespace> {
        Some(&self.namespace)
    }
}

/// Résumé d'un thème dans une version, pour les listes.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ThemeInfo {
    /// Stable id of the theme, shared by all its versions.
    pub id: Uuid,
    /// Hash of this version.
    pub version: PackVersion,
    /// Displayed name of the theme.
    pub name: String,
    /// Son espace de noms, unique dans la bibliothèque : le deck l'épingle sous ce nom.
    pub namespace: Namespace,
    /// Les thèmes qu'il étend, dans l'ordre (les suivants l'emportent).
    pub extends: Vec<Namespace>,
    /// Theme shipped with the app, never replaced.
    pub builtin: bool,
    /// Le thème d'un deck neuf.
    pub default: bool,
}

impl ThemeInfo {
    /// Le résumé de la version `s`.
    pub(crate) fn of(s: StoredVersion<ThemeManifest>) -> Self {
        Self {
            id: s.manifest.id,
            version: s.version,
            builtin: ThemeLibrary::is_builtin(s.manifest.id),
            default: s.manifest.id == DEFAULT_THEME,
            name: s.manifest.name,
            namespace: s.manifest.namespace,
            extends: s.manifest.extends,
        }
    }
}

/// Les thèmes.
#[derive(Debug, Clone)]
pub struct Themes;

/// Thèmes intégrés, lus une fois par processus, dans l'ordre.
static BUILTIN_THEME_VERSIONS: LazyLock<Vec<StoredVersion<ThemeManifest>>> = LazyLock::new(|| {
    [
        include_str!("../../../typst/oku-slides/themes/papier.json"),
        include_str!("../../../typst/oku-slides/themes/pro.json"),
        include_str!("../../../typst/oku-slides/themes/neutre.json"),
    ]
    .iter()
    .filter_map(|json| {
        let read = PackFiles::theme_from_bytes(json.as_bytes())
            .ok_or_else(|| "theme.json is not a json object".to_owned())
            .and_then(StoredVersion::read);
        // Couvert par les tests : n'arrive qu'avec un fichier du dépôt cassé.
        read.inspect_err(|e| tracing::error!(error = %e, "thème intégré illisible")).ok()
    })
    .collect()
});

impl LibraryKind for Themes {
    const DIR: &'static str = "themes";
    const MANIFEST_FILE: &'static str = "theme.json";
    const PACK_KIND: Option<oku_slides_core::PackKind> = None;
    type Manifest = ThemeManifest;

    fn builtins() -> &'static [StoredVersion<ThemeManifest>] {
        &BUILTIN_THEME_VERSIONS
    }
}

/// La bibliothèque des thèmes.
pub type ThemeLibrary = Library<Themes>;

impl Library<Themes> {
    /// Les thèmes de l'utilisateur (`paths.themes` des réglages), et les intégrés.
    #[must_use]
    pub fn user(platform: &oku_slides_platform::Platform) -> Self {
        Self::new(Some(platform.settings.paths.themes()), platform.storage.clone())
    }
}

impl ThemeLibrary {
    /// Tous les thèmes dans leur version courante : intégrés d'abord, puis
    /// ceux de l'utilisateur, par nom.
    pub fn themes(&self) -> Vec<ThemeInfo> {
        self.entries().into_iter().map(ThemeInfo::of).collect()
    }

    /// La version courante du thème qui porte l'espace de noms `ns`.
    #[must_use]
    pub fn by_namespace(&self, ns: &Namespace) -> Option<StoredVersion<ThemeManifest>> {
        self.entries().into_iter().find(|s| s.manifest.namespace == *ns)
    }
}

#[cfg(test)]
#[path = "theme_tests.rs"]
mod theme_tests;
