//! Espaces de noms des packs et noms qualifiés des widgets de pack.
//!
//! Un widget de pack s'appelle `<namespace>/<nom>` dans un deck ; seul
//! [`qualify`] forme ce nom et seul [`split_kind`] le lit. Un nom sans `/`
//! est un widget du noyau.

use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{NamespaceError, NamespaceSnafu};

/// Longueur maximale d'un espace de noms, en octets.
const MAX_NAMESPACE: usize = 64;

/// L'espace de noms réservé au pack propre d'un deck.
const DECK: &str = "deck";

/// L'espace de noms réservé au pack de layouts propre d'un deck.
const DECK_LAYOUTS: &str = "deck-layouts";

/// Namespace of a pack: `[a-z][a-z0-9-]*`, optionally `publisher.name`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
#[schemars(with = "String")]
pub struct Namespace(String);

/// Un segment : `[a-z][a-z0-9-]*`.
fn segment(s: &str) -> bool {
    let mut bytes = s.bytes();
    bytes.next().is_some_and(|b| b.is_ascii_lowercase())
        && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

impl Namespace {
    /// Lit un espace de noms ; `None` hors de la syntaxe permise.
    pub fn parse(s: &str) -> Option<Self> {
        let ok = s.len() <= MAX_NAMESPACE && {
            let mut parts = s.split('.');
            let first = parts.next().is_some_and(segment);
            let second = parts.next().is_none_or(segment);
            first && second && parts.next().is_none()
        };
        ok.then(|| Self(s.to_owned()))
    }

    /// L'espace de noms `deck`, celui du pack propre d'un deck.
    #[must_use]
    pub fn deck() -> Self {
        Self(DECK.to_owned())
    }

    /// L'espace de noms `deck-layouts`, celui du pack de layouts propre d'un deck.
    #[must_use]
    pub fn deck_layouts() -> Self {
        Self(DECK_LAYOUTS.to_owned())
    }

    /// Un espace de noms intégré, écrit dans le code ; `kinds_tests` vérifie sa syntaxe.
    pub(crate) fn builtin(s: &'static str) -> Self {
        Self(s.to_owned())
    }

    /// Vrai pour l'espace de noms réservé `deck`.
    #[must_use]
    pub fn is_deck(&self) -> bool {
        self.0 == DECK
    }

    /// Vrai pour l'espace de noms réservé `deck-layouts`.
    #[must_use]
    pub fn is_deck_layouts(&self) -> bool {
        self.0 == DECK_LAYOUTS
    }

    /// L'espace de noms sous forme de texte.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Namespace {
    type Error = NamespaceError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(&s).ok_or_else(|| NamespaceSnafu { value: s }.build())
    }
}

impl From<Namespace> for String {
    fn from(n: Namespace) -> Self {
        n.0
    }
}

impl fmt::Display for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Le nom qualifié d'un widget de pack : `<namespace>/<nom>`.
#[must_use]
pub fn qualify(ns: &Namespace, name: &str) -> String {
    format!("{ns}/{name}")
}

/// Espace de noms et nom d'un `kind` qualifié ; `None` pour un widget du noyau.
#[must_use]
pub fn split_kind(kind: &str) -> Option<(&str, &str)> {
    let (ns, name) = kind.split_once('/')?;
    (Namespace::parse(ns).is_some() && !name.is_empty() && !name.contains('/')).then_some((ns, name))
}

#[cfg(test)]
#[path = "namespace_tests.rs"]
mod namespace_tests;
