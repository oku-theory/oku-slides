//! Déclaration typée d'un jeton : son contrôle d'éditeur et, pour un pack, son défaut.
//!
//! [`Control`] est le format des entrées de `theme_schema.json` (la session le
//! lit) ; [`ComponentToken`] est une entrée de `tokens` d'un manifeste de pack
//! (`widgets.json` ou `layouts.json`) : le chemin sous `component.<namespace>`,
//! son contrôle, son défaut. Une même entrée donne le défaut du jeton, le
//! contrôle de l'éditeur et la validation de ses valeurs.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::error::{EmptyPathSnafu, MissingDefaultSnafu, PathConflictSnafu, TokenEntryError};

/// Contrôle de l'éditeur d'un jeton, et ses contraintes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "control", rename_all = "kebab-case")]
pub enum Control {
    /// `#rgb`, `#rrggbb`, `#rrggbbaa`, ou nom d'une couleur de `color`.
    Color,
    /// Nombre en pt borné, ou (si `refs`) nom d'un jeton de cette catégorie.
    Pt {
        /// Borne basse.
        #[serde(serialize_with = "number")]
        min: f64,
        /// Borne haute.
        #[serde(serialize_with = "number")]
        max: f64,
        /// Catégorie de jetons dont un nom est aussi accepté (`space`, `size`…).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        refs: Option<String>,
    },
    /// Nombre en em borné, ou (si `refs`) nom d'un jeton de cette catégorie.
    Em {
        /// Borne basse.
        #[serde(serialize_with = "number")]
        min: f64,
        /// Borne haute.
        #[serde(serialize_with = "number")]
        max: f64,
        /// Catégorie de jetons dont un nom est aussi accepté (`space`, `size`…).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        refs: Option<String>,
    },
    /// Graisse 100 à 900 par 100, ou nom d'un jeton `weight`.
    Weight,
    /// Liste ordonnée et non vide de familles de police.
    Fonts,
    /// Une des valeurs listées.
    Choice {
        /// Les valeurs permises.
        options: Vec<String>,
    },
    /// Un booléen.
    Bool,
    /// Chaîne libre (gabarits d'en-tête et de pied, marqueur de liste).
    Text,
    /// Liste non vide de couleurs (hex ou noms de `color`).
    Palette,
    /// Nombre sans unité borné ; `step` est le pas proposé par l'éditeur.
    Number {
        /// Borne basse.
        #[serde(serialize_with = "number")]
        min: f64,
        /// Borne haute.
        #[serde(serialize_with = "number")]
        max: f64,
        /// Pas proposé par l'éditeur.
        #[serde(serialize_with = "number")]
        step: f64,
    },
    /// Filet de titre : `{width, height, color}` (pt, pt, couleur).
    Rule,
}

/// Plus grand entier qu'un `f64` représente exactement (2^53).
const EXACT_INTEGER_MAX: f64 = 9_007_199_254_740_992.0;

/// Writes an integer bound without a fractional part: `0`, not `0.0`. A manifest
/// read back then rewritten thus keeps its canonical content, hence its version.
#[expect(clippy::trivially_copy_pass_by_ref, reason = "signature required by serde's serialize_with")]
#[expect(
    clippy::cast_possible_truncation,
    reason = "exact integer of at most 2^53 in absolute value, checked just before; \
              there is no fallible f64 to i64 conversion"
)]
fn number<S: Serializer>(v: &f64, s: S) -> Result<S::Ok, S::Error> {
    if v.fract() == 0.0 && v.abs() <= EXACT_INTEGER_MAX {
        // Exact : un entier d'au plus 2^53 en valeur absolue tient dans un i64.
        s.serialize_i64(*v as i64)
    } else {
        s.serialize_f64(*v)
    }
}

/// A component token declared by a pack manifest.
///
/// `path` is relative to `component.<namespace>`. `default` is required unless
/// `optional`; a written `null` is a value (`Some(Value::Null)`), not an
/// absence. `label` names the token in the editor for a non-built-in pack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentToken {
    /// Chemin du jeton sous `component.<namespace>` (`["card", "bg"]`).
    pub path: Vec<String>,
    /// Contrôle de l'éditeur et ses contraintes (clé `control`).
    #[serde(flatten)]
    pub control: Control,
    /// Valeur par défaut ; `Some(Value::Null)` pour un `null` écrit.
    #[serde(default, deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    /// `null` (« vider ») est permis.
    #[serde(default, skip_serializing_if = "is_false")]
    pub nullable: bool,
    /// Peut manquer : le code Typst du pack le lit avec une valeur de repli.
    #[serde(default, skip_serializing_if = "is_false")]
    pub optional: bool,
    /// Libellé de l'éditeur, pour un pack non intégré.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

impl ComponentToken {
    /// Checks the entry: a non-empty path, and a default unless `optional`.
    ///
    /// # Errors
    ///
    /// `EmptyPath` for an empty path, `MissingDefault` with neither default nor `optional`.
    pub fn check(&self) -> Result<(), TokenEntryError> {
        if self.path.is_empty() {
            return Err(EmptyPathSnafu.build().into());
        }
        if self.default.is_none() && !self.optional {
            return Err(MissingDefaultSnafu { path: self.path.join(".") }.build().into());
        }
        Ok(())
    }
}

/// Checks the `tokens` entries of a manifest.
///
/// Each entry passes [`ComponentToken::check`], then no path may extend or
/// equal another one (their defaults would not nest).
///
/// # Errors
///
/// The first refused entry, or `PathConflict` for two nested paths.
pub fn check_entries(tokens: &[ComponentToken]) -> Result<(), TokenEntryError> {
    tokens.iter().try_for_each(ComponentToken::check)?;
    let mut paths: Vec<&[String]> = tokens.iter().map(|t| t.path.as_slice()).collect();
    paths.sort();
    // Triés, un chemin et ceux qui le prolongent se suivent : comparer deux voisins suffit.
    match paths.windows(2).find_map(|w| match w {
        [a, b] if b.starts_with(a) => Some((*a, *b)),
        _ => None,
    }) {
        Some((other, path)) => Err(PathConflictSnafu { path: path.join("."), other: other.join(".") }.build().into()),
        None => Ok(()),
    }
}

/// Toute valeur présente, `null` compris, devient `Some` : seule l'absence vaut `None`.
fn present<'de, D: Deserializer<'de>>(de: D) -> Result<Option<Value>, D::Error> {
    Value::deserialize(de).map(Some)
}

#[expect(clippy::trivially_copy_pass_by_ref, reason = "signature required by serde's skip_serializing_if")]
fn is_false(b: &bool) -> bool {
    !*b
}

#[cfg(test)]
#[path = "token_spec_tests.rs"]
mod token_spec_tests;
