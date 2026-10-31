//! Erreurs des manifestes de pack : entrées `tokens` mal formées, manifeste illisible.

use oku_slides_i18n::{Localize, Localizer};
use snafu::{Backtrace, Snafu};

/// Une entrée `tokens` d'un manifeste de pack mal formée.
#[derive(Debug, Snafu)]
pub struct TokenEntryError(TokenEntryInner);

/// Nature d'une [`TokenEntryError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TokenEntryErrorKind {
    /// Le chemin du jeton est vide.
    EmptyPath,
    /// Ni défaut ni `optional`.
    MissingDefault,
    /// Deux chemins dont l'un prolonge l'autre (ou égaux).
    PathConflict,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum TokenEntryInner {
    #[snafu(display("component token with an empty path"))]
    EmptyPath { backtrace: Backtrace },
    #[snafu(display("component token {path} has no default and is not optional"))]
    MissingDefault { path: String, backtrace: Backtrace },
    /// `path` prolonge `other`, ou lui est égal : leurs défauts ne s'imbriquent pas.
    #[snafu(display("component token {path} conflicts with {other}"))]
    PathConflict { path: String, other: String, backtrace: Backtrace },
}

impl TokenEntryError {
    /// Ce qui ne va pas dans l'entrée.
    pub fn kind(&self) -> TokenEntryErrorKind {
        match self.0 {
            TokenEntryInner::EmptyPath { .. } => TokenEntryErrorKind::EmptyPath,
            TokenEntryInner::MissingDefault { .. } => TokenEntryErrorKind::MissingDefault,
            TokenEntryInner::PathConflict { .. } => TokenEntryErrorKind::PathConflict,
        }
    }

    /// Le chemin de l'entrée refusée, ses segments joints par `.` ; vide pour un chemin vide.
    pub fn path(&self) -> &str {
        match &self.0 {
            TokenEntryInner::EmptyPath { .. } => "",
            TokenEntryInner::MissingDefault { path, .. } | TokenEntryInner::PathConflict { path, .. } => path,
        }
    }
}

impl Localize for TokenEntryError {
    fn localize(&self, l: &Localizer) -> String {
        l.tr_with("pack-token-entry-invalid", &[("path", self.path().into())])
    }
}

/// Un manifeste de pack (`widgets.json`, `layouts.json`) qui ne se lit pas.
#[derive(Debug, Snafu)]
pub struct PackManifestError(Box<PackManifestInner>);

/// Nature d'une [`PackManifestError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PackManifestErrorKind {
    /// Le JSON n'a pas la forme d'un manifeste.
    Json,
    /// Une entrée `tokens` est refusée ; voir [`PackManifestError::token_entry`].
    TokenEntry,
    /// `order` repeats a name, lists `_commun` or an unknown file.
    Order,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
#[expect(clippy::enum_variant_names, reason = "every variant is a failure to read the manifest")]
pub(crate) enum PackManifestInner {
    #[snafu(display("unreadable pack manifest"))]
    ManifestJson { source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("invalid component token in the pack manifest"))]
    ManifestTokens { source: TokenEntryError, backtrace: Backtrace },
    #[snafu(display("invalid item order in the pack manifest at {name}"))]
    ManifestOrder { name: String, backtrace: Backtrace },
}

impl PackManifestError {
    /// Ce qui empêche la lecture.
    #[must_use]
    pub fn kind(&self) -> PackManifestErrorKind {
        match *self.0 {
            PackManifestInner::ManifestJson { .. } => PackManifestErrorKind::Json,
            PackManifestInner::ManifestTokens { .. } => PackManifestErrorKind::TokenEntry,
            PackManifestInner::ManifestOrder { .. } => PackManifestErrorKind::Order,
        }
    }

    /// L'entrée `tokens` refusée, si c'est elle qui empêche la lecture.
    #[must_use]
    pub fn token_entry(&self) -> Option<&TokenEntryError> {
        match &*self.0 {
            PackManifestInner::ManifestTokens { source, .. } => Some(source),
            PackManifestInner::ManifestJson { .. } | PackManifestInner::ManifestOrder { .. } => None,
        }
    }

    /// The name that makes `order` invalid, if that is what prevents the read.
    #[must_use]
    pub fn order_name(&self) -> Option<&str> {
        match &*self.0 {
            PackManifestInner::ManifestOrder { name, .. } => Some(name),
            PackManifestInner::ManifestJson { .. } | PackManifestInner::ManifestTokens { .. } => None,
        }
    }
}

impl Localize for PackManifestError {
    fn localize(&self, l: &Localizer) -> String {
        match &*self.0 {
            PackManifestInner::ManifestJson { source, .. } => {
                l.tr_with("pack-manifest-json", &[("reason", source.to_string().into())])
            }
            PackManifestInner::ManifestTokens { source, .. } => source.localize(l),
            PackManifestInner::ManifestOrder { name, .. } => {
                l.tr_with("pack-manifest-order", &[("name", name.as_str().into())])
            }
        }
    }
}
