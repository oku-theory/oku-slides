//! Packs de modèles portables : identité, version et fichiers.
//!
//! Un pack a un id ([`PackRef::id`], UUID v4) et des versions adressées par
//! contenu ([`PackVersion`]). Une version : un `pack.json` et des fichiers de
//! widgets `widgets/<nom>.typ` ([`WidgetFile`], `_commun` pour le code
//! partagé). Contenu canonique : `pack.json` en JSON compact à clés triées,
//! un octet `0`, puis pour chaque fichier dans l'ordre des noms : nom, `0`,
//! source, `0`. La version en est le sha-256, écrit `sha256:<hex>`. Une
//! source ne contient pas d'octet `0` (refusée à la lecture et à l'écriture),
//! et le JSON compact l'échappe : le contenu canonique est sans ambiguïté.
//!
//! Un thème ([`ThemeRef`]) suit le même modèle : une version de thème est un
//! [`PackFiles`] dont le manifeste est `theme.json` ([`ThemeManifest`]), sans
//! fichier de widget.
//!
//! Le contenu d'une version de pack, `pack.json`, se lit en [`PackManifest`] :
//! des modèles ([`Template`], des slides complètes, nommées), indépendants du
//! thème (ils ne parlent qu'en jetons).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::{Mutex, PoisonError};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use time::macros::format_description;
use uuid::Uuid;

use crate::error::{PackVersionError, PackVersionSnafu, WidgetFileError, WidgetFileSnafu};
use crate::model::Slide;

/// Préfixe d'une [`PackVersion`].
const PREFIX: &str = "sha256:";

/// 64 chiffres hexadécimaux minuscules : une empreinte, et un nom de dossier sûr.
fn is_hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Version d'un pack : `sha256:<hex>` de son contenu canonique.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
#[schemars(with = "String")]
pub struct PackVersion(String);

impl PackVersion {
    /// La version de `files`.
    pub fn of(files: &PackFiles) -> Self {
        Self(format!("{PREFIX}{}", hex::encode(Sha256::digest(files.canonical()))))
    }

    /// Lit `sha256:<hex>` ; `None` pour toute autre forme.
    pub fn parse(s: &str) -> Option<Self> {
        s.strip_prefix(PREFIX).and_then(Self::from_hex)
    }

    /// Depuis l'empreinte seule (nom de dossier `packs/<hex>`).
    pub fn from_hex(hex: &str) -> Option<Self> {
        is_hex64(hex).then(|| Self(format!("{PREFIX}{hex}")))
    }

    /// L'empreinte sans préfixe : nom de dossier, id du pack dans Typst.
    pub fn hex(&self) -> &str {
        self.0.strip_prefix(PREFIX).unwrap_or(&self.0)
    }

    /// La forme écrite, `sha256:<hex>`.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Vrai si `files` a bien cette version (vérification d'intégrité).
    pub fn verifies(&self, files: &PackFiles) -> bool {
        Self::of(files) == *self
    }
}

impl TryFrom<String> for PackVersion {
    type Error = PackVersionError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(&s).ok_or_else(|| PackVersionSnafu { value: s }.build())
    }
}

impl From<PackVersion> for String {
    fn from(v: PackVersion) -> Self {
        v.0
    }
}

impl fmt::Display for PackVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Le fichier de code partagé d'un pack : `widgets/_commun.typ`.
pub const COMMON_WIDGET_FILE: &str = "_commun";

/// Longueur maximale d'un nom de fichier de widget.
const MAX_WIDGET_NAME: usize = 64;

/// Nom d'un fichier de widget : `[a-z][a-z0-9-]*`, ou `_commun`.
///
/// C'est le nom du widget qu'il déclare ; `_commun` n'est pas un widget.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
#[schemars(with = "String")]
pub struct WidgetFile(String);

impl WidgetFile {
    /// Lit un nom ; `None` hors de `[a-z][a-z0-9-]*` et de `_commun`.
    pub fn parse(s: &str) -> Option<Self> {
        let mut bytes = s.bytes();
        let plain = s.len() <= MAX_WIDGET_NAME
            && bytes.next().is_some_and(|b| b.is_ascii_lowercase())
            && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        (plain || s == COMMON_WIDGET_FILE).then(|| Self(s.to_owned()))
    }

    /// Le fichier partagé `_commun`.
    pub fn common() -> Self {
        Self(COMMON_WIDGET_FILE.to_owned())
    }

    /// Vrai pour `_commun`, qui n'est pas un widget.
    pub fn is_common(&self) -> bool {
        self.0 == COMMON_WIDGET_FILE
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Nom sur disque et dans le zip : `<nom>.typ`.
    pub fn file_name(&self) -> String {
        format!("{}.typ", self.0)
    }

    /// Depuis `<nom>.typ` ; `None` pour tout autre nom.
    pub fn from_file_name(name: &str) -> Option<Self> {
        name.strip_suffix(".typ").and_then(Self::parse)
    }
}

impl TryFrom<String> for WidgetFile {
    type Error = WidgetFileError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(&s).ok_or_else(|| WidgetFileSnafu { value: s }.build())
    }
}

impl From<WidgetFile> for String {
    fn from(w: WidgetFile) -> Self {
        w.0
    }
}

impl fmt::Display for WidgetFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Le pack d'un deck : son id et la version embarquée.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct PackRef {
    #[schemars(with = "String")]
    pub id: Uuid,
    pub version: PackVersion,
}

/// Le thème d'un deck : son id et la version embarquée.
///
/// Une version de thème est un [`PackFiles`] sans widget dont le manifeste
/// est `theme.json` : même contenu canonique, même empreinte.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct ThemeRef {
    #[schemars(with = "String")]
    pub id: Uuid,
    pub version: PackVersion,
}

/// Les fichiers d'une version : `pack.json` (un objet) et ses widgets.
#[derive(Debug, Clone, PartialEq)]
pub struct PackFiles {
    manifest: Map<String, Value>,
    widgets: BTreeMap<WidgetFile, String>,
}

impl PackFiles {
    /// Les fichiers lus sous l'empreinte annoncée `hex`, si leur contenu la donne.
    ///
    /// `None` si `hex` n'est pas une empreinte, si les octets ne se lisent pas
    /// (voir [`Self::from_bytes`]) ou si le contenu a une autre version.
    pub fn verified<'a>(
        hex: &str,
        manifest: &[u8],
        widgets: impl IntoIterator<Item = (WidgetFile, &'a [u8])>,
    ) -> Option<(PackVersion, Self)> {
        let claimed = PackVersion::from_hex(hex)?;
        Self::from_bytes(manifest, widgets).filter(|f| f.matches(&claimed)).map(|f| (claimed, f))
    }

    /// Un fichier vide vaut absent : même contenu canonique.
    pub fn new(manifest: Map<String, Value>, mut widgets: BTreeMap<WidgetFile, String>) -> Self {
        widgets.retain(|_, source| !source.is_empty());
        Self { manifest, widgets }
    }

    /// Depuis des octets lus (zip, disque) ; `None` si `pack.json` n'est pas
    /// un objet JSON, ou si un fichier de widget n'est pas de l'UTF-8 ou
    /// contient un octet `0` (le séparateur du contenu canonique : deux
    /// versions différentes y auraient la même empreinte).
    pub fn from_bytes<'a>(pack_json: &[u8], widgets: impl IntoIterator<Item = (WidgetFile, &'a [u8])>) -> Option<Self> {
        let Ok(Value::Object(manifest)) = serde_json::from_slice(pack_json) else { return None };
        let widgets = widgets
            .into_iter()
            .map(|(name, bytes)| {
                let source = String::from_utf8(bytes.to_vec()).ok().filter(|s| !s.contains('\0'))?;
                Some((name, source))
            })
            .collect::<Option<BTreeMap<_, _>>>()?;
        Some(Self::new(manifest, widgets))
    }

    /// Le `pack.json`, tel que reçu.
    pub fn manifest(&self) -> &Map<String, Value> {
        &self.manifest
    }

    /// Les fichiers de widgets, par nom (`_commun` compris).
    pub fn widgets(&self) -> &BTreeMap<WidgetFile, String> {
        &self.widgets
    }

    /// La source du fichier `name` ; `None` s'il est absent.
    pub fn widget(&self, name: &WidgetFile) -> Option<&str> {
        self.widgets.get(name).map(String::as_str)
    }

    /// La version de ces fichiers.
    pub fn version(&self) -> PackVersion {
        PackVersion::of(self)
    }

    /// Vrai si ces fichiers ont la version `claimed`.
    pub fn matches(&self, claimed: &PackVersion) -> bool {
        claimed.verifies(self)
    }

    /// `pack.json` à écrire : indenté, clés dans l'ordre reçu.
    pub fn pack_json(&self) -> serde_json::Result<Vec<u8>> {
        serde_json::to_vec_pretty(&self.manifest)
    }

    /// Contenu canonique : `pack.json` canonique, `0`, puis nom, `0`, source, `0` par fichier.
    pub fn canonical(&self) -> Vec<u8> {
        // Un `Value` s'écrit toujours : ses clés sont des chaînes.
        let mut out = serde_json::to_vec(&sorted(&Value::Object(self.manifest.clone()))).unwrap_or_default();
        out.push(0);
        for (name, source) in &self.widgets {
            out.extend_from_slice(name.as_str().as_bytes());
            out.push(0);
            out.extend_from_slice(source.as_bytes());
            out.push(0);
        }
        out
    }
}

/// `v` aux clés triées à toute profondeur (`preserve_order` garde l'ordre d'insertion).
fn sorted(v: &Value) -> Value {
    match v {
        Value::Object(m) => {
            let mut entries: Vec<(&String, &Value)> = m.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            Value::Object(entries.into_iter().map(|(k, v)| (k.clone(), sorted(v))).collect())
        }
        Value::Array(a) => Value::Array(a.iter().map(sorted).collect()),
        other => other.clone(),
    }
}

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
}

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
    /// Lit le `theme.json` de `files`.
    pub fn from_files(files: &PackFiles) -> serde_json::Result<Self> {
        serde_json::from_value(Value::Object(files.manifest().clone()))
    }

    /// Les fichiers de cette version : le seul `theme.json`.
    pub fn to_files(&self) -> serde_json::Result<PackFiles> {
        match serde_json::to_value(self)? {
            Value::Object(manifest) => Ok(PackFiles::new(manifest, BTreeMap::new())),
            _ => Err(serde::ser::Error::custom("theme manifest is not an object")),
        }
    }
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
