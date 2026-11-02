//! Capability manifest of a pack or theme version, read from its files alone.
//!
//! [`Capabilities::of`] reads no library and parses no Typst: it lists what a
//! version declares (items, requirements, tokens, font families, profiles) and
//! the size of its Typst code, so the user can review a version a deck brings
//! before it enters the library. Strings stay raw; the session cleans them.
//!
//! A theme version is a [`PackFiles`] of kind `widgets` (see [`PackFiles::theme`]):
//! it is told apart by its manifest, whose `tokens` is an object (token groups),
//! where a pack manifest's `tokens` is an array of [`ComponentToken`]s. The two
//! shapes are exclusive, so a version is never read as the other kind.

use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::{Map, Value};
use snafu::ResultExt;

use crate::error::{ManifestJsonSnafu, PackManifestError};
use crate::pack::{PackFiles, PackManifest, ThemeManifest};
use crate::pack_kind::PackKind;
use crate::pack_order::item_order;
use crate::token_spec::{ComponentToken, Control};

/// Path segments whose string values name font families in theme tokens.
const FONT_SEGMENTS: [&str; 2] = ["font", "family"];

/// The kind of version a [`Capabilities`] describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityKind {
    /// A pack of element widgets.
    Widgets,
    /// A pack of slide layouts.
    Layouts,
    /// A theme.
    Theme,
}

impl CapabilityKind {
    /// The kind's serialized name: `widgets`, `layouts` or `theme`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Widgets => "widgets",
            Self::Layouts => "layouts",
            Self::Theme => "theme",
        }
    }
}

impl From<PackKind> for CapabilityKind {
    /// The capability kind of a pack of kind `kind`.
    fn from(kind: PackKind) -> Self {
        match kind {
            PackKind::Widgets => Self::Widgets,
            PackKind::Layouts => Self::Layouts,
        }
    }
}

/// What a pack or theme version holds, read from its files alone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Capabilities {
    /// Pack of widgets, of layouts, or theme.
    pub kind: CapabilityKind,
    /// Name from the manifest, raw; the session cleans it.
    pub name: String,
    /// Namespace from the manifest.
    pub namespace: String,
    /// One-sentence description from the manifest, raw; empty if absent.
    pub description: String,
    /// Widget or layout names, `_commun` excluded, in `order` then by name; empty for a theme.
    pub items: Vec<String>,
    /// Namespaces a pack requires, or the themes a theme extends, in manifest order.
    pub requires: Vec<String>,
    /// Packs: component token paths joined by dots; themes: token group names.
    pub tokens: Vec<String>,
    /// Font families named in the tokens, sorted and deduplicated.
    pub fonts: Vec<String>,
    /// Number of layout profiles the import can pick (0 for others).
    pub profiles: usize,
    /// Total bytes of the Typst sources, `_commun` included (0 for a theme).
    pub code_bytes: usize,
}

impl Capabilities {
    /// Reads the capabilities of version `files`; fails on an unreadable manifest.
    ///
    /// Fonts of a pack: the strings of each `fonts`-control token's default.
    /// Fonts of a theme: every string under a token path segment `font` or
    /// `family`, at any depth (the `font` group, and such keys in `component`).
    ///
    /// # Errors
    ///
    /// A pack manifest refused by [`PackManifest::from_files`], or a theme
    /// manifest that does not have the shape of a [`ThemeManifest`]
    /// ([`PackManifestError`] of kind `Json`).
    pub fn of(files: &PackFiles) -> Result<Self, PackManifestError> {
        if is_theme(files) { Self::of_theme(files) } else { Self::of_pack(files) }
    }

    /// The capabilities of a pack version (widgets or layouts).
    fn of_pack(files: &PackFiles) -> Result<Self, PackManifestError> {
        let manifest = PackManifest::from_files(files)?;
        let fonts = manifest.tokens.iter().filter(|t| matches!(t.control, Control::Fonts)).flat_map(token_fonts);
        Ok(Self {
            kind: files.kind().into(),
            items: item_order(&manifest.order, files).into_iter().map(|w| w.as_str().to_owned()).collect(),
            requires: manifest.requires.iter().map(|ns| ns.as_str().to_owned()).collect(),
            tokens: manifest.tokens.iter().map(|t| t.path.join(".")).collect(),
            fonts: sorted_unique(fonts),
            profiles: manifest.profiles.len(),
            code_bytes: files.widgets().values().map(String::len).sum(),
            name: manifest.name,
            namespace: manifest.namespace.as_str().to_owned(),
            description: manifest.description,
        })
    }

    /// The capabilities of a theme version.
    fn of_theme(files: &PackFiles) -> Result<Self, PackManifestError> {
        let manifest = ThemeManifest::from_files(files).context(ManifestJsonSnafu)?;
        let mut fonts = Vec::new();
        font_strings(&manifest.tokens, false, &mut fonts);
        Ok(Self {
            kind: CapabilityKind::Theme,
            items: Vec::new(),
            requires: manifest.extends.iter().map(|ns| ns.as_str().to_owned()).collect(),
            tokens: manifest.tokens.keys().cloned().collect(),
            fonts: sorted_unique(fonts),
            profiles: 0,
            code_bytes: 0,
            name: manifest.name,
            namespace: manifest.namespace.as_str().to_owned(),
            description: manifest.description,
        })
    }
}

/// True for a theme version: no widget file and an object `tokens` (token groups).
fn is_theme(files: &PackFiles) -> bool {
    files.widgets().is_empty() && files.manifest().get("tokens").is_some_and(Value::is_object)
}

/// The strings of a component token's default: one family or a list of them.
fn token_fonts(token: &ComponentToken) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(default) = &token.default {
        strings_in(default, &mut out);
    }
    out
}

/// Collects the strings under a key of [`FONT_SEGMENTS`] in `tokens`, at any depth.
fn font_strings(tokens: &Map<String, Value>, under_font: bool, out: &mut Vec<String>) {
    for (key, value) in tokens {
        let under = under_font || FONT_SEGMENTS.contains(&key.as_str());
        match value {
            Value::Object(inner) => font_strings(inner, under, out),
            other if under => strings_in(other, out),
            _ => {}
        }
    }
}

/// Collects every string of `value`: itself, or the strings of an array or object.
fn strings_in(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(s) => out.push(s.clone()),
        Value::Array(items) => items.iter().for_each(|v| strings_in(v, out)),
        Value::Object(map) => map.values().for_each(|v| strings_in(v, out)),
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

/// `names` sorted and deduplicated.
fn sorted_unique(names: impl IntoIterator<Item = String>) -> Vec<String> {
    names.into_iter().collect::<BTreeSet<_>>().into_iter().collect()
}

#[cfg(test)]
#[path = "capabilities_tests.rs"]
mod capabilities_tests;
