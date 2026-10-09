//! Le contenu d'un pack de modèles (`pack.json`) et son résumé.
//!
//! Un pack regroupe des modèles (des slides complètes, nommées). Il est
//! indépendant du thème : ses modèles ne parlent qu'en jetons.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use time::OffsetDateTime;
use time::macros::format_description;
use uuid::Uuid;

use oku_slides_core::{PackFiles, PackVersion, Slide, WidgetFile};

use crate::library_kind::VersionManifest;

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
    /// Valeurs d'exemple de chaque widget : celles de son aperçu et de ses réglages.
    ///
    /// Les valeurs ont la forme de celles de l'aperçu (texte riche : tableau de
    /// paragraphes) ; une valeur égale à la valeur neutre n'est pas gardée. Un
    /// widget sans valeurs n'a pas d'entrée ; vide, la clé n'est pas écrite (les
    /// versions sans valeurs gardent leur empreinte). Lu aussi dans sa forme
    /// d'avant t-541 (voir [`lenient_examples`]).
    #[serde(default, deserialize_with = "lenient_examples", skip_serializing_if = "BTreeMap::is_empty")]
    pub examples: BTreeMap<WidgetFile, Map<String, Value>>,
}

/// Les valeurs d'exemple d'un widget dans `pack.json`, telles qu'écrites.
#[derive(Deserialize)]
#[serde(untagged)]
enum StoredValues {
    /// Forme actuelle : les valeurs.
    Values(Map<String, Value>),
    /// Forme d'avant t-541 : une liste de cas nommés (`{"name", "values"}`).
    Cases(Vec<StoredCase>),
}

/// Un cas nommé d'avant t-541 : seules ses valeurs comptent.
#[derive(Deserialize)]
struct StoredCase {
    #[serde(default)]
    values: Map<String, Value>,
}

/// Lit `examples` sans rendre illisible le pack qui le porte (t-524) : d'un
/// pack d'avant t-541 (une liste de cas), le premier cas donne les valeurs du
/// widget (celles que montraient l'aperçu et les réglages par défaut), les
/// autres sont oubliés. Une entrée mal formée (nom qui n'est pas celui d'un
/// fichier de widget, valeurs qui ne sont pas un objet) est écartée avec un
/// diagnostic au journal ; les autres restent. Les valeurs vides sont omises.
///
/// Les bornes des valeurs (`PLAIN_MAX`…) dépendent du schéma du widget : elles
/// s'appliquent à leur usage (`trial_fits`), pas ici.
fn lenient_examples<'de, D>(de: D) -> std::result::Result<BTreeMap<WidgetFile, Map<String, Value>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let Value::Object(stored) = Value::deserialize(de)? else {
        diagnose("", "`examples` n'est pas un objet, ignoré");
        return Ok(BTreeMap::new());
    };
    Ok(stored
        .into_iter()
        .filter_map(|(name, v)| {
            let Some(widget) = WidgetFile::parse(&name) else {
                diagnose(&name, "valeurs d'exemple d'un nom de widget invalide, ignorées");
                return None;
            };
            let values = match serde_json::from_value(v) {
                Ok(StoredValues::Values(m)) => m,
                Ok(StoredValues::Cases(cases)) => cases.into_iter().next().map(|c| c.values).unwrap_or_default(),
                Err(e) => {
                    diagnose(&name, &format!("valeurs d'exemple mal formées, ignorées : {e}"));
                    return None;
                }
            };
            (!values.is_empty()).then_some((widget, values))
        })
        .collect())
}

/// Problèmes distincts déjà signalés : au-delà, plus rien n'est retenu.
const DIAGNOSED_MAX: usize = 256;

/// Signale un problème de `examples` : au journal (`warn`) la première fois
/// dans le processus, puis en `debug` (le manifeste est relu à chaque accès
/// à la bibliothèque).
fn diagnose(widget: &str, problem: &str) {
    static SEEN: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());
    let key = format!("{widget}\0{problem}");
    let first = {
        let mut seen = SEEN.lock().unwrap_or_else(PoisonError::into_inner);
        seen.len() < DIAGNOSED_MAX && seen.insert(key)
    };
    if first {
        tracing::warn!(widget, "pack.json : {problem}");
    } else {
        tracing::debug!(widget, "pack.json : {problem}");
    }
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
