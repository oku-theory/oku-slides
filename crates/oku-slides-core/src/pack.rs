//! Packs de modèles portables : identité, version et fichiers.
//!
//! Un pack a un id ([`PackRef::id`], UUID v4) et des versions adressées par
//! contenu ([`PackVersion`]). Le contenu canonique d'une version : son
//! `pack.json` en JSON compact à clés triées, un octet `0`, puis son
//! `widgets.typ` tel quel (vide s'il est absent). La version en est le
//! sha-256, écrit `sha256:<hex>` (64 chiffres hexadécimaux minuscules).

use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::{PackVersionError, PackVersionSnafu};

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

/// Le pack d'un deck : son id et la version embarquée.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct PackRef {
    #[schemars(with = "String")]
    pub id: Uuid,
    pub version: PackVersion,
}

/// Les fichiers d'une version : `pack.json` (un objet) et `widgets.typ`.
#[derive(Debug, Clone, PartialEq)]
pub struct PackFiles {
    manifest: Map<String, Value>,
    widgets: Option<String>,
}

impl PackFiles {
    /// Un `widgets` vide vaut absent : même contenu canonique.
    pub fn new(manifest: Map<String, Value>, widgets: Option<String>) -> Self {
        Self { manifest, widgets: widgets.filter(|w| !w.is_empty()) }
    }

    /// Depuis des octets lus (zip, disque) ; `None` si `pack.json` n'est pas
    /// un objet JSON ou si `widgets.typ` n'est pas de l'UTF-8.
    pub fn from_bytes(pack_json: &[u8], widgets: Option<&[u8]>) -> Option<Self> {
        let Ok(Value::Object(manifest)) = serde_json::from_slice(pack_json) else { return None };
        let widgets = match widgets {
            Some(bytes) => Some(String::from_utf8(bytes.to_vec()).ok()?),
            None => None,
        };
        Some(Self::new(manifest, widgets))
    }

    /// Le `pack.json`, tel que reçu.
    pub fn manifest(&self) -> &Map<String, Value> {
        &self.manifest
    }

    /// Le `widgets.typ` ; `None` pour un pack sans widgets.
    pub fn widgets(&self) -> Option<&str> {
        self.widgets.as_deref()
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

    /// Contenu canonique : `pack.json` compact à clés triées, `0`, `widgets.typ`.
    pub fn canonical(&self) -> Vec<u8> {
        // Un `Value` s'écrit toujours : ses clés sont des chaînes.
        let mut out = serde_json::to_vec(&sorted(&Value::Object(self.manifest.clone()))).unwrap_or_default();
        out.push(0);
        out.extend_from_slice(self.widgets.as_deref().unwrap_or_default().as_bytes());
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
