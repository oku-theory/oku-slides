//! Le résumé d'un pack de modèles et les ids des packs intégrés.
//!
//! Le contenu d'un pack (`pack.json` : [`PackManifest`], ses [`Template`])
//! est défini dans le cœur, qui le partage avec l'import ; il est ré-exporté ici.

use serde::Serialize;
use uuid::Uuid;

use oku_slides_core::{PackFiles, PackVersion};
pub use oku_slides_core::{PackManifest, PackScope, Template, now_created};

use crate::library_kind::VersionManifest;

/// Pack par défaut, `templates/essentiel` : id constant.
pub const DEFAULT_PACK: Uuid = Uuid::from_u128(0x6f3c1e2a_8b4d_4c1e_9a57_2d0e5b7c9f10);
/// Pack intégré `templates/grilles` : id constant.
pub const GRID_PACK: Uuid = Uuid::from_u128(0xb8e2d4f6_1a3c_4e5b_8d7f_9c0a2e4b6d81);

impl VersionManifest for PackManifest {
    fn from_files(files: &PackFiles) -> serde_json::Result<Self> {
        Self::from_files(files)
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

    fn scope(&self) -> PackScope {
        self.scope
    }
}

/// Résumé d'un pack dans une version, pour les listes.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PackInfo {
    pub id: Uuid,
    pub version: PackVersion,
    pub name: String,
    pub description: String,
    pub builtin: bool,
    /// Le pack d'un deck neuf ([`DEFAULT_PACK`]).
    pub default: bool,
    /// Cette version est arrivée avec un deck reçu (pas créée par l'utilisateur).
    pub received: bool,
    pub count: usize,
}

impl PackInfo {
    /// Résumé de la version `version` de `manifest` pour les listes.
    pub fn of(manifest: &PackManifest, version: PackVersion, builtin: bool, received: bool) -> Self {
        Self {
            id: manifest.id,
            version,
            name: manifest.name.clone(),
            description: manifest.description.clone(),
            builtin,
            default: manifest.id == DEFAULT_PACK,
            received,
            count: manifest.templates.len(),
        }
    }
}

/// Une version par rapport à une autre, par la chaîne des `parent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    Same,
    /// Plus ancienne : l'autre en descend.
    Ancestor,
    /// Plus récente : elle descend de l'autre.
    Descendant,
    /// Parallèle : aucune ne descend de l'autre.
    Sibling,
}

#[cfg(test)]
#[path = "pack_tests.rs"]
mod pack_tests;
