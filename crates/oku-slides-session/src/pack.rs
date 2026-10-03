//! Le contenu d'un pack de modèles (`pack.json`) et son résumé.
//!
//! Un pack regroupe des modèles (des slides complètes, nommées). Il est
//! indépendant du thème : ses modèles ne parlent qu'en jetons.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use time::macros::format_description;
use uuid::Uuid;

use oku_slides_core::{PackFiles, PackVersion, Slide, WidgetFile};

/// Pack par défaut, `templates/essentiel` : id constant.
pub const DEFAULT_PACK: Uuid = Uuid::from_u128(0x6f3c1e2a_8b4d_4c1e_9a57_2d0e5b7c9f10);
/// Pack intégré `templates/grilles` : id constant.
pub const GRID_PACK: Uuid = Uuid::from_u128(0xb8e2d4f6_1a3c_4e5b_8d7f_9c0a2e4b6d81);

/// Un modèle : une slide complète, nommée.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub slide: Slide,
}

/// `pack.json` : un pack dans une version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackManifest {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Version dont celle-ci est issue ; `None` pour une première version.
    #[serde(default)]
    pub parent: Option<PackVersion>,
    /// Création, UTC à la seconde : `AAAA-MM-JJTHH:MM:SSZ`.
    pub created: String,
    pub templates: Vec<Template>,
}

impl PackManifest {
    /// Lit le `pack.json` de `files`.
    pub fn from_files(files: &PackFiles) -> serde_json::Result<Self> {
        serde_json::from_value(Value::Object(files.manifest().clone()))
    }

    /// Les fichiers de cette version, avec ses fichiers de widgets.
    pub fn to_files(&self, widgets: BTreeMap<WidgetFile, String>) -> serde_json::Result<PackFiles> {
        match serde_json::to_value(self)? {
            Value::Object(manifest) => Ok(PackFiles::new(manifest, widgets)),
            _ => Err(serde::ser::Error::custom("pack manifest is not an object")),
        }
    }

    /// Résumé de cette version pour les listes.
    pub fn info(&self, version: PackVersion, builtin: bool) -> PackInfo {
        PackInfo {
            id: self.id,
            version,
            name: self.name.clone(),
            description: self.description.clone(),
            builtin,
            default: self.id == DEFAULT_PACK,
            count: self.templates.len(),
        }
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
    pub count: usize,
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

/// Maintenant, au format de `created`.
pub fn now_created() -> String {
    OffsetDateTime::now_utc()
        .format(format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]Z"))
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "pack_tests.rs"]
mod pack_tests;
