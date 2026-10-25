//! Le résumé d'un pack et les ids des packs intégrés (`base`, `essentiel`).
//!
//! Le contenu d'un pack (`widgets.json` ou `layouts.json` : [`PackManifest`])
//! est défini dans le cœur, qui le partage avec l'import ; il est ré-exporté ici.

use serde::Serialize;
use snafu::ResultExt;
use uuid::Uuid;

use oku_slides_core::{Namespace, PackFiles, PackKind, PackVersion};
pub use oku_slides_core::{PackManifest, PackScope, now_created};

use crate::error::PackManifestSnafu;
use crate::library_kind::VersionManifest;

/// L'id du pack de widgets intégré `base`.
pub const BASE_PACK: Uuid = Uuid::from_u128(0x3b1f6c2e_5a4d_4e8b_9c71_0d2e4f6a8b13);
/// L'id du pack de layouts intégré `essentiel`.
pub const ESSENTIEL_PACK: Uuid = Uuid::from_u128(0xd5eb8a57_07b4_47f7_a818_dde5347160cf);

impl VersionManifest for PackManifest {
    fn from_files(files: &PackFiles) -> crate::Result<Self> {
        let read = Self::from_files(files);
        // L'id se lit à part : un manifeste refusé a peut-être un id lisible.
        let id = || files.manifest().get("id").and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok());
        Ok(read.context(PackManifestSnafu { id: id().unwrap_or_default() })?)
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
    /// Sa sorte : widgets d'éléments ou layouts de slide.
    pub kind: PackKind,
    /// Stable id of the pack, shared by all its versions.
    pub id: Uuid,
    /// Hash of this version.
    pub version: PackVersion,
    /// Displayed name of the pack.
    pub name: String,
    /// Espace de noms de ses widgets : leur `kind` est `<namespace>/<nom>`.
    pub namespace: Namespace,
    /// Espaces de noms des packs dont il a besoin.
    pub requires: Vec<Namespace>,
    /// Free description of the pack, as its manifest gives it.
    pub description: String,
    /// Pack shipped with the app, never replaced.
    pub builtin: bool,
    /// Un pack qu'épingle un deck neuf : [`BASE_PACK`] ou [`ESSENTIEL_PACK`].
    pub default: bool,
    /// Cette version est arrivée avec un deck reçu (pas créée par l'utilisateur).
    pub received: bool,
    /// Le nombre de ses fichiers de widgets, `_commun` excepté.
    pub widgets: usize,
    /// Le nombre de ses profils d'import (pack de layouts).
    pub profiles: usize,
}

impl PackInfo {
    /// Résumé de la version `files` du pack `manifest`, pour les listes.
    #[must_use]
    pub fn of(files: &PackFiles, manifest: &PackManifest, builtin: bool, received: bool) -> Self {
        Self {
            kind: files.kind(),
            id: manifest.id,
            version: files.version(),
            name: manifest.name.clone(),
            namespace: manifest.namespace.clone(),
            requires: manifest.requires.clone(),
            description: manifest.description.clone(),
            builtin,
            default: manifest.id == BASE_PACK || manifest.id == ESSENTIEL_PACK,
            received,
            widgets: files.widgets().keys().filter(|n| !n.is_common()).count(),
            profiles: manifest.profiles.len(),
        }
    }
}

/// Une version par rapport à une autre, par la chaîne des `parent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    /// The same version.
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
