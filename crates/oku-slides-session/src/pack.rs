//! Le résumé d'un pack de modèles et les ids des packs intégrés (`essentiel`, `grilles`, `base`).
//!
//! Le contenu d'un pack (`widgets.json` : [`PackManifest`], ses [`Template`])
//! est défini dans le cœur, qui le partage avec l'import ; il est ré-exporté ici.

use serde::Serialize;
use uuid::Uuid;

use oku_slides_core::{Namespace, PackFiles, PackVersion};
pub use oku_slides_core::{PackManifest, PackScope, Template, now_created};

use crate::library_kind::VersionManifest;

/// Pack par défaut, `templates/essentiel` : id constant ; un deck neuf l'épingle sous `essentiel`.
pub const DEFAULT_PACK: Uuid = Uuid::from_u128(0x6f3c1e2a_8b4d_4c1e_9a57_2d0e5b7c9f10);
/// Pack intégré `templates/grilles` : id constant.
pub const GRID_PACK: Uuid = Uuid::from_u128(0xb8e2d4f6_1a3c_4e5b_8d7f_9c0a2e4b6d81);
/// L'id du pack de widgets intégré `base`.
pub const BASE_PACK: Uuid = Uuid::from_u128(0x3b1f6c2e_5a4d_4e8b_9c71_0d2e4f6a8b13);

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

    fn namespace(&self) -> Option<&Namespace> {
        Some(&self.namespace)
    }
}

/// Résumé d'un pack dans une version, pour les listes.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PackInfo {
    pub id: Uuid,
    pub version: PackVersion,
    pub name: String,
    /// Espace de noms de ses widgets : leur `kind` est `<namespace>/<nom>`.
    pub namespace: Namespace,
    /// Espaces de noms des packs dont il a besoin.
    pub requires: Vec<Namespace>,
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
            namespace: manifest.namespace.clone(),
            requires: manifest.requires.clone(),
            description: manifest.description.clone(),
            builtin,
            default: manifest.id == DEFAULT_PACK,
            received,
            count: manifest.templates.len(),
        }
    }
}

/// Une version par rapport à une autre, par la chaîne des `parent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
