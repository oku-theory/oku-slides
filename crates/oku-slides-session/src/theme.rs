//! Le contenu d'un thème (`theme.json`), sa sorte de bibliothèque et ses thèmes intégrés.
//!
//! Un thème est une version de jetons de conception (couleurs, polices,
//! tailles…) ; il n'a pas de fichiers de widgets. Papier et Pro sont livrés
//! avec l'app, en lecture seule.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

use oku_slides_core::{PackFiles, PackVersion};

use crate::library::Library;
use crate::library_kind::{LibraryKind, StoredVersion, VersionManifest};

/// Thème intégré Papier : id constant.
pub const PAPIER_THEME: Uuid = Uuid::from_u128(0x2a7d4c91_5e3b_4f08_b6a1_8c9e0d1f2a31);
/// Thème intégré Pro : id constant.
pub const PRO_THEME: Uuid = Uuid::from_u128(0x7e1b9c3d_2f4a_4d6e_a8b0_5c7d9e1f3b42);
/// Thème d'un deck neuf.
pub const DEFAULT_THEME: Uuid = PAPIER_THEME;

/// `theme.json` : un thème dans une version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeManifest {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Version dont celle-ci est issue ; `None` pour une première version.
    #[serde(default)]
    pub parent: Option<PackVersion>,
    /// Création, UTC à la seconde : `AAAA-MM-JJTHH:MM:SSZ`.
    pub created: String,
    /// Jetons de conception, par groupe (`color`, `font`, …).
    pub tokens: Map<String, Value>,
}

impl ThemeManifest {
    /// Les fichiers de cette version : le seul `theme.json`.
    pub fn to_files(&self) -> Result<PackFiles, serde_json::Error> {
        match serde_json::to_value(self)? {
            Value::Object(manifest) => Ok(PackFiles::new(manifest, Default::default())),
            _ => Err(serde::ser::Error::custom("theme manifest is not an object")),
        }
    }
}

impl VersionManifest for ThemeManifest {
    fn from_files(files: &PackFiles) -> Result<Self, serde_json::Error> {
        serde_json::from_value(Value::Object(files.manifest().clone()))
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
}

/// Résumé d'un thème dans une version, pour les listes.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ThemeInfo {
    pub id: Uuid,
    pub version: PackVersion,
    pub name: String,
    pub builtin: bool,
    /// Le thème d'un deck neuf.
    pub default: bool,
}

/// Les thèmes.
#[derive(Debug, Clone)]
pub struct Themes;

/// Thèmes intégrés, lus une fois par processus, dans l'ordre.
static BUILTIN_THEME_VERSIONS: LazyLock<Vec<StoredVersion<ThemeManifest>>> = LazyLock::new(|| {
    [
        include_str!("../../../typst/oku-slides/themes/papier.json"),
        include_str!("../../../typst/oku-slides/themes/pro.json"),
    ]
    .iter()
    .filter_map(|json| {
        let read = PackFiles::from_bytes(json.as_bytes(), [])
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
    const HAS_WIDGETS: bool = false;
    type Manifest = ThemeManifest;

    fn builtins() -> &'static [StoredVersion<ThemeManifest>] {
        &BUILTIN_THEME_VERSIONS
    }
}

/// La bibliothèque des thèmes.
pub type ThemeLibrary = Library<Themes>;

impl ThemeLibrary {
    /// Tous les thèmes dans leur version courante : intégrés d'abord, puis
    /// ceux de l'utilisateur, par nom.
    pub fn themes(&self) -> Vec<ThemeInfo> {
        self.entries()
            .into_iter()
            .map(|s| ThemeInfo {
                id: s.manifest.id,
                version: s.version,
                name: s.manifest.name,
                builtin: Self::is_builtin(s.manifest.id),
                default: s.manifest.id == DEFAULT_THEME,
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "theme_tests.rs"]
mod theme_tests;
