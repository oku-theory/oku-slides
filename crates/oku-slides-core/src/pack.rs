//! Packs de widgets portables : identité, version et fichiers.
//!
//! Un pack a un id ([`PackPin::id`], UUID v4), une sorte ([`PackKind`]) et des
//! versions adressées par contenu ([`PackVersion`]). Une version : un manifeste
//! (`widgets.json` ou `layouts.json`) et des fichiers de widgets
//! `<dossier>/<nom>.typ` ([`WidgetFile`], `_commun` pour le code partagé).
//! Contenu canonique : pour un pack de layouts seulement, `layouts` et un octet
//! `0` ; puis le manifeste en JSON compact à clés triées, un octet `0`, puis
//! pour chaque fichier dans l'ordre des noms : nom, `0`, source, `0`. La
//! version en est le sha-256, écrit `sha256:<hex>`. Une source ne contient pas
//! d'octet `0` (refusée à la lecture et à l'écriture), et le JSON compact
//! l'échappe : le contenu canonique est sans ambiguïté.
//!
//! Un thème suit le même modèle : une version de thème est un [`PackFiles`]
//! sans préfixe ni fichier de widget dont le manifeste est `theme.json`
//! ([`ThemeManifest`], construit par [`PackFiles::theme`]) ; un deck l'épingle
//! par un [`ThemePin`](crate::theme_chain::ThemePin).
//!
//! Le manifeste d'une version se lit en [`PackManifest`] : valeurs d'exemple
//! des widgets, jetons de composant déclarés ([`ComponentToken`]) et, pour un
//! pack de layouts, ses préréglages ([`LayoutProfile`]).
//!
//! Une version de pack porte sa portée ([`PackScope`]) : celles de la
//! bibliothèque (par défaut, non écrite dans le manifeste), ou celles d'un
//! deck seul (pack généré par l'import), que la bibliothèque n'enregistre
//! jamais ; une nouvelle version issue d'elles garde leur portée.

use std::collections::BTreeMap;
use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use time::macros::format_description;
use uuid::Uuid;

use snafu::{OptionExt, ResultExt};

use crate::error::{
    EmptyPathSnafu, ManifestJsonSnafu, ManifestTokensSnafu, PackManifestError, PackVersionError, PackVersionSnafu,
    TokenEntryError, WidgetFileError, WidgetFileSnafu,
};
use crate::layout_profile::LayoutProfile;
use crate::model::Style;
use crate::namespace::Namespace;
use crate::pack_kind::PackKind;
use crate::token_spec::{ComponentToken, check_entries};
use crate::tokens::merge;

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

/// Un pack épinglé par un deck (widgets ou layouts) : id, version embarquée, besoins.
///
/// `requires` recopie celui du manifeste de la version : le cœur refuse ainsi
/// un retrait sans lire les fichiers ; les sources vérifient la copie.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct PackPin {
    /// Id du pack (UUID v4), stable d'une version à l'autre.
    #[schemars(with = "String")]
    pub id: Uuid,
    /// Version embarquée par les sources du deck.
    pub version: PackVersion,
    /// Espaces de noms dont ce pack a besoin, épinglés aussi par le deck.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<Namespace>,
}

/// Les fichiers d'une version : sa sorte, son manifeste (un objet) et ses widgets.
#[derive(Debug, Clone, PartialEq)]
pub struct PackFiles {
    kind: PackKind,
    manifest: Map<String, Value>,
    widgets: BTreeMap<WidgetFile, String>,
}

impl PackFiles {
    /// Les fichiers lus sous l'empreinte annoncée `hex`, si leur contenu la donne.
    ///
    /// `None` si `hex` n'est pas une empreinte, si les octets ne se lisent pas
    /// (voir [`Self::from_bytes`]) ou si le contenu a une autre version.
    pub fn verified<'a>(
        kind: PackKind,
        hex: &str,
        manifest: &[u8],
        widgets: impl IntoIterator<Item = (WidgetFile, &'a [u8])>,
    ) -> Option<(PackVersion, Self)> {
        let claimed = PackVersion::from_hex(hex)?;
        Self::from_bytes(kind, manifest, widgets).filter(|f| f.matches(&claimed)).map(|f| (claimed, f))
    }

    /// Les fichiers d'un pack de sorte `kind` ; un fichier vide vaut absent.
    pub fn new(kind: PackKind, manifest: Map<String, Value>, mut widgets: BTreeMap<WidgetFile, String>) -> Self {
        widgets.retain(|_, source| !source.is_empty());
        Self { kind, manifest, widgets }
    }

    /// Depuis des octets lus (zip, disque) ; `None` si le manifeste n'est pas
    /// un objet JSON, ou si un fichier de widget n'est pas de l'UTF-8 ou
    /// contient un octet `0` (le séparateur du contenu canonique : deux
    /// versions différentes y auraient la même empreinte).
    pub fn from_bytes<'a>(
        kind: PackKind,
        pack_json: &[u8],
        widgets: impl IntoIterator<Item = (WidgetFile, &'a [u8])>,
    ) -> Option<Self> {
        let Ok(Value::Object(manifest)) = serde_json::from_slice(pack_json) else { return None };
        let widgets = widgets
            .into_iter()
            .map(|(name, bytes)| {
                let source = String::from_utf8(bytes.to_vec()).ok().filter(|s| !s.contains('\0'))?;
                Some((name, source))
            })
            .collect::<Option<BTreeMap<_, _>>>()?;
        Some(Self::new(kind, manifest, widgets))
    }

    /// Les fichiers d'une version de thème : son seul `theme.json`.
    ///
    /// Sans préfixe canonique : la version d'un thème ne dépend que de son `theme.json`.
    pub fn theme(manifest: Map<String, Value>) -> Self {
        Self::new(PackKind::Widgets, manifest, BTreeMap::new())
    }

    /// Une version de thème depuis les octets de son `theme.json` ; `None` s'il n'est pas un objet JSON.
    pub fn theme_from_bytes(json: &[u8]) -> Option<Self> {
        Self::from_bytes(PackKind::Widgets, json, [])
    }

    /// La version de thème lue sous l'empreinte annoncée `hex`, si son contenu la donne.
    pub fn verified_theme(hex: &str, json: &[u8]) -> Option<Self> {
        Self::verified(PackKind::Widgets, hex, json, []).map(|(_, files)| files)
    }

    /// La sorte du pack : elle fixe son manifeste, son dossier et sa version.
    pub fn kind(&self) -> PackKind {
        self.kind
    }

    /// Le manifeste, tel que reçu.
    pub fn manifest(&self) -> &Map<String, Value> {
        &self.manifest
    }

    /// La portée de cette version, lue de la seule clé `scope` du manifeste.
    ///
    /// Le reste (exemples, préréglages) n'est pas lu : un instantané la demande à
    /// chaque fois. Absente, la bibliothèque ; une valeur inconnue est une erreur.
    pub fn scope(&self) -> serde_json::Result<PackScope> {
        self.manifest.get("scope").map_or(Ok(PackScope::Library), PackScope::deserialize)
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

    /// Le manifeste de la sorte à écrire : indenté, clés dans l'ordre reçu.
    pub fn pack_json(&self) -> serde_json::Result<Vec<u8>> {
        serde_json::to_vec_pretty(&self.manifest)
    }

    /// Contenu canonique : préfixe de sorte, manifeste canonique, `0`, puis nom, `0`, source, `0` par fichier.
    ///
    /// Seul un pack de layouts a un préfixe (`layouts`, `0`) : un pack de
    /// widgets, et donc `base`, garde la version d'avant les sortes.
    pub fn canonical(&self) -> Vec<u8> {
        let mut out = match self.kind {
            PackKind::Widgets => Vec::new(),
            PackKind::Layouts => b"layouts\0".to_vec(),
        };
        // Un `Value` s'écrit toujours : ses clés sont des chaînes.
        out.extend(serde_json::to_vec(&sorted(&Value::Object(self.manifest.clone()))).unwrap_or_default());
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

/// Où vit un pack : la bibliothèque, ou un seul deck.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackScope {
    /// Rangé dans la bibliothèque, partagé entre les decks.
    #[default]
    Library,
    /// Embarqué dans un seul deck ; la bibliothèque ne l'enregistre jamais.
    Deck,
}

impl PackScope {
    /// Vrai pour la portée par défaut, celle de la bibliothèque (non écrite).
    pub fn is_library(&self) -> bool {
        matches!(self, Self::Library)
    }
}

/// Le manifeste d'un pack (`widgets.json` ou `layouts.json`) dans une version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackManifest {
    pub id: Uuid,
    pub name: String,
    /// Espace de noms des widgets du pack : leur `kind` est `<namespace>/<nom>`.
    pub namespace: Namespace,
    /// Espaces de noms des packs de widgets dont ce pack a besoin.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<Namespace>,
    #[serde(default)]
    pub description: String,
    /// Version dont celle-ci est issue ; `None` pour une première version.
    #[serde(default)]
    pub parent: Option<PackVersion>,
    /// Création, UTC à la seconde : `AAAA-MM-JJTHH:MM:SSZ`.
    pub created: String,
    /// Portée : écrite seulement pour un pack du deck seul (les autres gardent leur empreinte).
    #[serde(default, skip_serializing_if = "PackScope::is_library")]
    pub scope: PackScope,
    /// Préréglages d'un layout que l'import retrofit sait choisir (pack de layouts seulement).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub profiles: Vec<LayoutProfile>,
    /// Valeurs d'exemple de chaque widget : celles de son aperçu et de ses réglages.
    ///
    /// Les valeurs ont la forme de celles de l'aperçu (texte riche : tableau de
    /// paragraphes) ; une valeur égale à la valeur neutre n'est pas gardée. Un
    /// widget sans valeurs n'a pas d'entrée ; vide, la clé n'est pas écrite (les
    /// versions sans valeurs gardent leur empreinte). Lu aussi dans sa forme
    /// d'avant t-541 (voir `pack_examples::lenient_examples`).
    #[serde(
        default,
        deserialize_with = "crate::pack_examples::lenient_examples",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub examples: BTreeMap<WidgetFile, Map<String, Value>>,
    /// Style de départ des widgets ajoutés depuis la grille, par widget.
    #[serde(
        default,
        deserialize_with = "crate::pack_examples::lenient_styles",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub styles: BTreeMap<WidgetFile, Style>,
    /// Jetons de composant du pack, sous `component.<namespace>` ; vide, la clé
    /// n'est pas écrite (les versions sans jetons gardent leur empreinte).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tokens: Vec<ComponentToken>,
}

impl PackManifest {
    /// Lit le manifeste de `files` et vérifie ses entrées `tokens` ([`check_entries`]).
    ///
    /// Une entrée refusée fait échouer la lecture ([`PackManifestError::token_entry`]).
    pub fn from_files(files: &PackFiles) -> Result<Self, PackManifestError> {
        let manifest: Self =
            serde_json::from_value(Value::Object(files.manifest().clone())).context(ManifestJsonSnafu)?;
        check_entries(&manifest.tokens).context(ManifestTokensSnafu)?;
        Ok(manifest)
    }

    /// Les défauts des jetons de composant, imbriqués par chemin.
    ///
    /// C'est la couche `component.<namespace>` du pack, avant tout thème. Une
    /// entrée `optional` sans défaut n'y figure pas ; des entrées refusées par
    /// [`check_entries`] rendent l'erreur.
    pub fn component_defaults(&self) -> Result<Map<String, Value>, TokenEntryError> {
        check_entries(&self.tokens)?;
        let mut out = Map::new();
        for token in &self.tokens {
            let Some(default) = &token.default else { continue }; // `optional` sans défaut : documenté.
            let (first, rest) = token.path.split_first().context(EmptyPathSnafu)?;
            let value =
                rest.iter().rev().fold(default.clone(), |inner, g| Value::Object(Map::from_iter([(g.clone(), inner)])));
            merge(&mut out, &Map::from_iter([(first.clone(), value)]));
        }
        Ok(out)
    }

    /// Les fichiers de cette version, de sorte `kind`, avec ses fichiers de widgets.
    pub fn to_files(&self, kind: PackKind, widgets: BTreeMap<WidgetFile, String>) -> serde_json::Result<PackFiles> {
        match serde_json::to_value(self)? {
            Value::Object(manifest) => Ok(PackFiles::new(kind, manifest, widgets)),
            _ => Err(serde::ser::Error::custom("pack manifest is not an object")),
        }
    }
}

/// `theme.json` : un thème dans une version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeManifest {
    pub id: Uuid,
    pub name: String,
    /// Espace de noms du thème, unique parmi les trois sortes de la bibliothèque.
    pub namespace: Namespace,
    /// Thèmes parents, dans l'ordre : les suivants l'emportent ; vide, la clé n'est pas écrite.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extends: Vec<Namespace>,
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

    /// Les fichiers de cette version : le seul `theme.json` ([`PackFiles::theme`]).
    pub fn to_files(&self) -> serde_json::Result<PackFiles> {
        match serde_json::to_value(self)? {
            Value::Object(manifest) => Ok(PackFiles::theme(manifest)),
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
