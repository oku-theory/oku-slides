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

use std::collections::BTreeMap;
use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::{PackVersionError, PackVersionSnafu, WidgetFileError, WidgetFileSnafu};

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

/// Les fichiers d'une version : `pack.json` (un objet) et ses widgets.
#[derive(Debug, Clone, PartialEq)]
pub struct PackFiles {
    manifest: Map<String, Value>,
    widgets: BTreeMap<WidgetFile, String>,
}

impl PackFiles {
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

#[cfg(test)]
#[path = "pack_tests.rs"]
mod pack_tests;
