//! Modèle de données du deck, sérialisé en JSON.
//!
//! Le JSON produit est exactement celui que consomme la bibliothèque Typst
//! `oku-slides` (`render-deck`) : les noms de champs et les valeurs par défaut
//! doivent rester alignés sur `typst/oku-slides/src/data.typ`.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::ValueReason;
use crate::namespace::Namespace;
use crate::pack::PackPin;
use crate::pack_kind::PackKind;
use crate::params::{ParamValue, Params};
use crate::theme_chain::ThemePin;
use crate::widget::{ParamType, WidgetRegistry};

/// Version du format de deck.
pub const DECK_VERSION: u32 = 9;

/// A complete deck.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Deck {
    /// Format version; a readable deck carries the current version.
    #[serde(default = "default_version")]
    pub version: u32,
    /// Namespace of the deck's theme, pinned in `themes`; absent: the default theme.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<Namespace>,
    /// Pinned themes, by namespace: exactly the closure of the `extends` of `theme`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub themes: BTreeMap<Namespace, ThemePin>,
    /// Deck parameters: title, author, date and free keys.
    #[serde(default)]
    pub params: DeckParams,
    /// Variables usable in texts: `{{client}}`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vars: BTreeMap<String, Value>,
    /// Design token overrides, deep-merged on the Typst side.
    #[serde(default, skip_serializing_if = "is_empty_object")]
    pub tokens: Value,
    /// Widget packs of the deck, by namespace, including the closure of their `requires`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub widgets: BTreeMap<Namespace, PackPin>,
    /// Layout packs of the deck, by namespace; their `requires` name widget packs.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub layouts: BTreeMap<Namespace, PackPin>,
    /// Slides of the deck, in presentation order.
    pub slides: Vec<Slide>,
}

fn default_version() -> u32 {
    DECK_VERSION
}

fn is_empty_object(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Object(m) => m.is_empty(),
        _ => false,
    }
}

/// Deck parameters, passed as is to `deck.with(..)`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DeckParams {
    /// Title of the deck.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Subtitle of the deck.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    /// Author of the deck.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Displayed date, as free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// Organization of the author.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    /// Language of the text, BCP 47 code (`fr`, `en-US`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    /// Additional parameters (`numbering`, `toc-title`…).
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl DeckParams {
    /// Champs nommés, les autres clés vont dans `extra`.
    const FIELDS: [&'static str; 6] = ["title", "subtitle", "author", "date", "organization", "lang"];

    /// `key` est-il un champ nommé (et non une clé de `extra`) ?
    #[must_use]
    pub fn is_field(key: &str) -> bool {
        Self::FIELDS.contains(&key)
    }

    /// Texte du paramètre `key` : un champ nommé, sinon une chaîne de
    /// `extra`. Seul accès par nom, pour les ops, `validate` et la session.
    pub fn field(&self, key: &str) -> Option<&str> {
        match key {
            "title" => self.title.as_deref(),
            "subtitle" => self.subtitle.as_deref(),
            "author" => self.author.as_deref(),
            "date" => self.date.as_deref(),
            "organization" => self.organization.as_deref(),
            "lang" => self.lang.as_deref(),
            _ => self.extra.get(key).and_then(Value::as_str),
        }
    }

    /// Pose (`Some`) ou retire (`None`) le texte du paramètre `key`.
    pub fn set_field(&mut self, key: &str, text: Option<String>) {
        let slot = match key {
            "title" => &mut self.title,
            "subtitle" => &mut self.subtitle,
            "author" => &mut self.author,
            "date" => &mut self.date,
            "organization" => &mut self.organization,
            "lang" => &mut self.lang,
            _ => {
                match text {
                    Some(t) => self.extra.insert(key.to_owned(), Value::String(t)),
                    None => self.extra.remove(key),
                };
                return;
            }
        };
        *slot = text;
    }
}

/// A slide: a widget of category `slide` and its parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Slide {
    /// Stable id of the slide, unique in the deck.
    pub id: String,
    /// Slide widget, qualified by its pack (`essentiel/title`).
    pub kind: String,
    /// Widget parameters, element slots included.
    #[serde(default, skip_serializing_if = "Params::is_empty")]
    pub params: Params,
    /// Speaker notes; empty: no notes.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

impl Slide {
    /// Slide `id` of widget `kind`, with no parameters or notes.
    #[must_use]
    pub fn new(id: impl Into<String>, kind: &str) -> Self {
        Self { id: id.into(), kind: kind.to_owned(), params: Params::default(), notes: String::new() }
    }

    /// Le paramètre `grid` de la slide, sinon la grille 12 × 6 par défaut : ce n'est
    /// pas toujours la grille rendue, car elle ignore le défaut déclaré par le widget
    /// (voir `resolved_grid`).
    pub(crate) fn grid(&self) -> Grid {
        self.params
            .value("grid")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_else(|| DEFAULT_GRID.clone())
    }

    /// Grille d'une slide : paramètre, défaut déclaré par son widget, sinon 12 × 6.
    #[must_use]
    pub fn resolved_grid(&self, registry: &WidgetRegistry) -> Grid {
        if self.params.value("grid").is_some() {
            return self.grid();
        }
        self.declared_grid(registry).and_then(|v| serde_json::from_value(v).ok()).unwrap_or_else(|| self.grid())
    }

    /// Défaut du paramètre `grid` déclaré par le widget de la slide, tel qu'écrit.
    #[must_use]
    pub fn declared_grid(&self, registry: &WidgetRegistry) -> Option<Value> {
        registry
            .get(&self.kind)
            .and_then(|w| w.param("grid"))
            .filter(|p| matches!(p.ty, ParamType::Grid))
            .and_then(|p| p.default.clone())
            .filter(serde_json::Value::is_object)
    }
}

/// Frame in pt on the 1920 × 1080 grid, relative to the body area.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Frame {
    /// Abscissa of the top-left corner, in pt.
    pub x: f64,
    /// Ordinate of the top-left corner, in pt.
    pub y: f64,
    /// Width in pt, positive.
    pub w: f64,
    /// Height in pt, positive.
    pub h: f64,
}

/// Layout grid: the body area cut into equal columns and rows.
///
/// A gutter separates them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Grid {
    /// Number of columns, from 1 to 24.
    pub columns: u32,
    /// Number of rows, from 1 to 24.
    pub rows: u32,
    /// Gutter: spacing token (`"lg"`) or pt; default `"lg"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap: Option<TokenValue>,
}

/// Place of an element on the grid.
///
/// First column and first row (from 0), then the number of columns and rows spanned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Cell {
    /// First column, from 0.
    pub col: u32,
    /// First row, from 0.
    pub row: u32,
    /// Columns spanned; default 1.
    #[serde(default = "one")]
    pub cols: u32,
    /// Rows spanned; default 1.
    #[serde(default = "one")]
    pub rows: u32,
}

fn one() -> u32 {
    1
}

/// Grille de toute slide qui n'en précise pas : 12 colonnes × 6 lignes.
static DEFAULT_GRID: Grid = Grid { columns: 12, rows: 6, gap: None };

impl Cell {
    /// La cellule tient-elle dans la grille ?
    #[must_use]
    pub fn fits(&self, grid: &Grid) -> bool {
        self.cols >= 1 && self.rows >= 1 && self.col + self.cols <= grid.columns && self.row + self.rows <= grid.rows
    }
}

/// A token value: a name (`"accent"`, `"body"`) or a number (pt).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum TokenValue {
    /// Numeric value, in pt.
    Number(f64),
    /// Name of a theme token.
    Name(String),
}

/// Style of an element; each absent field inherits from the theme.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Style {
    /// Text size: size token or pt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<TokenValue>,
    /// Theme color name or `#rrggbb`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Weight: token (`strong`) or number (100 to 900).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<TokenValue>,
    /// Font role (`display`, `body`, `mono`) or family.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    /// Italic text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    /// Horizontal alignment: `left`, `center` or `right`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
    /// Line spacing: token or pt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leading: Option<TokenValue>,
    /// Fill: theme color name or `#rrggbb`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    /// Stroke color: theme name or `#rrggbb`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<String>,
    /// Stroke thickness: token or pt.
    #[serde(default, rename = "stroke-width", skip_serializing_if = "Option::is_none")]
    pub stroke_width: Option<TokenValue>,
    /// Corner radius: token or pt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<TokenValue>,
    /// Vertical anchoring of a text in its frame: `top`, `middle` or `bottom`.
    ///
    /// Without a frame, no effect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valign: Option<String>,
    /// Inner margins of a text, in pt: top, right, bottom, left.
    ///
    /// Each is finite, from 0 to 4096.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inset: Option<[f64; 4]>,
    /// Spacing added between the letters of a text, in pt; negative: tighter.
    ///
    /// Finite, from -4096 to 4096.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tracking: Option<f64>,
    /// Space between two paragraphs of a text, in pt, list items included.
    ///
    /// It runs from the last baseline of one to the top of the first
    /// line of the next; finite, from 0 to 4096. Without the key,
    /// the theme's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spacing: Option<f64>,
    /// Paragraph indents of a text, in pt.
    ///
    /// First the left margin (from 0 to
    /// 4096), then the first-line offset (from -4096 to 4096; negative: hanging
    /// indent, where the bullet of a list item sits). Without the key, the theme's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indent: Option<[f64; 2]>,
}

/// Valeurs de [`Style::valign`].
pub const STYLE_VALIGNS: [&str; 3] = ["top", "middle", "bottom"];

/// Plus grande marge intérieure ([`Style::inset`]), en pt : au-delà de
/// toute page raisonnable.
pub const STYLE_INSET_MAX: f64 = 4096.0;

/// Plus grand espacement des lettres ([`Style::tracking`]), en pt, dans un
/// sens ou dans l'autre : au-delà de toute page raisonnable.
pub const STYLE_TRACKING_MAX: f64 = 4096.0;

/// Plus grand espace entre paragraphes ([`Style::spacing`]), en pt : au-delà
/// de toute page raisonnable.
pub const STYLE_SPACING_MAX: f64 = 4096.0;

/// Plus grand retrait ([`Style::indent`]), en pt, dans un sens ou dans
/// l'autre : au-delà de toute page raisonnable.
pub const STYLE_INDENT_MAX: f64 = 4096.0;

impl Style {
    /// Applique les champs définis de `patch` sur `self`.
    pub fn merge(&mut self, patch: Style) {
        macro_rules! take {
            ($($f:ident),*) => { $( if patch.$f.is_some() { self.$f = patch.$f; } )* };
        }
        take!(
            size,
            color,
            weight,
            font,
            italic,
            align,
            leading,
            fill,
            stroke,
            stroke_width,
            radius,
            valign,
            inset,
            tracking,
            spacing,
            indent
        );
    }

    /// Le premier champ hors de son domaine (`style.<champ>`) et pourquoi ;
    /// `None` si tout est permis. Seuls `valign`, `inset`, `tracking`,
    /// `spacing` et `indent` ont un domaine fermé : les autres champs se résolvent au rendu (jeton ou
    /// repli). Un nombre non fini ne se relirait pas (JSON l'écrit `null`).
    #[must_use]
    pub fn invalid(&self) -> Option<(&'static str, ValueReason)> {
        if self.valign.as_deref().is_some_and(|v| !STYLE_VALIGNS.contains(&v)) {
            return Some(("style.valign", ValueReason::Valign));
        }
        let fits = |v: &f64| v.is_finite() && (0.0..=STYLE_INSET_MAX).contains(v);
        if self.inset.is_some_and(|inset| !inset.iter().all(fits)) {
            return Some(("style.inset", ValueReason::OutOfBounds));
        }
        if self.tracking.is_some_and(|v| !(v.is_finite() && v.abs() <= STYLE_TRACKING_MAX)) {
            return Some(("style.tracking", ValueReason::OutOfBounds));
        }
        if self.spacing.is_some_and(|v| !(v.is_finite() && (0.0..=STYLE_SPACING_MAX).contains(&v))) {
            return Some(("style.spacing", ValueReason::OutOfBounds));
        }
        let indent_fits = |[margin, first]: [f64; 2]| {
            margin.is_finite()
                && (0.0..=STYLE_INDENT_MAX).contains(&margin)
                && first.is_finite()
                && first.abs() <= STYLE_INDENT_MAX
        };
        if self.indent.is_some_and(|i| !indent_fits(i)) {
            return Some(("style.indent", ValueReason::OutOfBounds));
        }
        None
    }
}

/// Kind of list of an item paragraph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ListKind {
    /// Bulleted list.
    Bullet,
    /// Numbered list.
    Number,
}

/// A paragraph: a sequence of runs, possibly in a list.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Paragraph {
    /// Runs of the paragraph, in text order.
    #[serde(default)]
    pub runs: Vec<Run>,
    /// List the paragraph is an item of; absent: plain paragraph.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list: Option<ListKind>,
}

impl Paragraph {
    /// Paragraph of a single run, with no formatting or list.
    #[must_use]
    pub fn plain(t: impl Into<String>) -> Self {
        Self { runs: vec![Run::plain(t)], list: None }
    }
}

/// A run: a text with a single formatting in a paragraph.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Run {
    /// Text of the run.
    pub t: String,
    /// Bold text.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bold: bool,
    /// Italic text.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub italic: bool,
    /// Text color: theme name or `#rrggbb`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Text size: size token or pt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<TokenValue>,
    /// Link target (`http`, `https` or `mailto`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    /// Highlight: number (from 1) of the background color in the theme's palette.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highlight: Option<u8>,
}

impl Run {
    /// Run of text `t`, with no formatting.
    #[must_use]
    pub fn plain(t: impl Into<String>) -> Self {
        Self { t: t.into(), ..Default::default() }
    }
}

/// An element: a widget of category `element`, placed and styled.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Element {
    /// Stable id of the element, unique in its slide.
    pub id: String,
    /// Element widget, qualified by its pack (`base/text`).
    pub kind: String,
    /// Free frame on the slide; absent: placed by the layout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<Frame>,
    /// Attachment to the slide's grid; takes precedence over `frame`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell: Option<Cell>,
    // Boxed: an element keeps a modest size on the stack, where nested
    // slots are read and written recursively.
    /// Style of the element; each absent field inherits from the theme.
    #[serde(default, skip_serializing_if = "is_default_style")]
    pub style: Box<Style>,
    /// Widget parameters, child element slots included.
    #[serde(default, skip_serializing_if = "Params::is_empty")]
    pub params: Params,
}

#[expect(clippy::borrowed_box, reason = "signature imposée par skip_serializing_if sur un Box<Style>")]
fn is_default_style(s: &Box<Style>) -> bool {
    **s == Style::default()
}

impl Element {
    /// Element `id` of widget `kind`, with no frame, style or parameters.
    #[must_use]
    pub fn new(id: impl Into<String>, kind: &str) -> Self {
        Self {
            id: id.into(),
            kind: kind.to_owned(),
            frame: None,
            cell: None,
            style: Box::default(),
            params: Params::default(),
        }
    }

    /// Un élément `base/text` dont le corps (`body`) est `paragraphs`.
    pub fn text(id: impl Into<String>, paragraphs: Vec<Paragraph>) -> Self {
        let mut e = Self::new(id, crate::kinds::base::TEXT);
        e.params.set("body", ParamValue::Rich(paragraphs));
        e
    }
}

impl Deck {
    /// Deck vide de la version courante : aucune slide.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            version: DECK_VERSION,
            theme: None,
            themes: BTreeMap::new(),
            params: DeckParams::default(),
            vars: BTreeMap::new(),
            tokens: Value::Null,
            widgets: BTreeMap::new(),
            layouts: BTreeMap::new(),
            slides: Vec::new(),
        }
    }

    /// Les épinglages du deck de sorte `kind`.
    #[must_use]
    pub fn pins(&self, kind: PackKind) -> &BTreeMap<Namespace, PackPin> {
        match kind {
            PackKind::Widgets => &self.widgets,
            PackKind::Layouts => &self.layouts,
        }
    }

    /// Les épinglages du deck de sorte `kind`, à modifier.
    pub fn pins_mut(&mut self, kind: PackKind) -> &mut BTreeMap<Namespace, PackPin> {
        match kind {
            PackKind::Widgets => &mut self.widgets,
            PackKind::Layouts => &mut self.layouts,
        }
    }

    /// Chaque épinglage du deck avec sa sorte : les widgets, puis les layouts.
    pub fn all_pins(&self) -> impl Iterator<Item = (PackKind, &Namespace, &PackPin)> {
        PackKind::ALL.into_iter().flat_map(move |kind| self.pins(kind).iter().map(move |(ns, pin)| (kind, ns, pin)))
    }

    /// The slide `id`, if the deck contains it.
    #[must_use]
    pub fn slide(&self, id: &str) -> Option<&Slide> {
        self.slides.iter().find(|s| s.id == id)
    }

    /// The slide `id`, mutable.
    pub fn slide_mut(&mut self, id: &str) -> Option<&mut Slide> {
        self.slides.iter_mut().find(|s| s.id == id)
    }

    /// Position of the slide `id` in the deck.
    #[must_use]
    pub fn slide_index(&self, id: &str) -> Option<usize> {
        self.slides.iter().position(|s| s.id == id)
    }

    /// Reads a deck from JSON. Refuses a deck of a version other than
    /// [`DECK_VERSION`]: read as a v4, it would silently lose fields.
    ///
    /// # Errors
    ///
    /// Invalid JSON, JSON that does not have the shape of a deck, or another version.
    pub fn from_json(s: &str) -> serde_json::Result<Self> {
        let deck: Self = serde_json::from_str(s)?;
        if deck.version != DECK_VERSION {
            return Err(serde::de::Error::custom(format!(
                "unsupported deck version {} (expected {DECK_VERSION})",
                deck.version
            )));
        }
        Ok(deck)
    }

    /// Readable (indented) JSON, as stored in the sources.
    ///
    /// # Errors
    ///
    /// `serde_json` failure, impossible in practice: all keys are strings.
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod model_tests;
