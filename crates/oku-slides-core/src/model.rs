//! Modèle de données du deck, sérialisé en JSON.
//!
//! Le JSON produit est exactement celui que consomme la bibliothèque Typst
//! `oku-slides` (`render-deck`) : les noms de champs et les valeurs par défaut
//! doivent rester alignés sur `typst/oku-slides/src/data.typ`.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::pack::{PackRef, ThemeRef};
use crate::params::{ParamValue, Params};
use crate::widget::{ParamType, WidgetRegistry};

/// Version du format de deck.
pub const DECK_VERSION: u32 = 5;

/// Un deck complet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Deck {
    #[serde(default = "default_version")]
    pub version: u32,
    /// Thème du deck, dans la version embarquée par ses sources ; `None` :
    /// le thème par défaut.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<ThemeRef>,
    #[serde(default)]
    pub params: DeckParams,
    /// Variables utilisables dans les textes : `{{client}}`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vars: BTreeMap<String, Value>,
    /// Surcharges de jetons de design (fusion profonde côté Typst).
    #[serde(default, skip_serializing_if = "is_empty_object")]
    pub tokens: Value,
    /// Pack de modèles du deck, dans la version embarquée par ses sources ;
    /// `None` : le pack par défaut, dans sa version courante (deck neuf).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub templates: Option<PackRef>,
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

/// Paramètres du deck, transmis tels quels à `deck.with(..)`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DeckParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    /// Paramètres supplémentaires (`numbering`, `toc-title`…).
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl DeckParams {
    /// Champs nommés, les autres clés vont dans `extra`.
    const FIELDS: [&'static str; 6] = ["title", "subtitle", "author", "date", "organization", "lang"];

    /// `key` est-il un champ nommé (et non une clé de `extra`) ?
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

/// Une slide : un widget de catégorie `slide` et ses paramètres.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Slide {
    pub id: String,
    /// Widget de slide (`title`, `content`…).
    pub kind: String,
    #[serde(default, skip_serializing_if = "Params::is_empty")]
    pub params: Params,
    /// Nom et description, pour une slide qui sert de modèle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

impl Slide {
    pub fn new(id: impl Into<String>, kind: &str) -> Self {
        Self {
            id: id.into(),
            kind: kind.to_owned(),
            params: Params::default(),
            name: None,
            description: None,
            notes: String::new(),
        }
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
    pub fn resolved_grid(&self, registry: &WidgetRegistry) -> Grid {
        if self.params.value("grid").is_some() {
            return self.grid();
        }
        self.declared_grid(registry).and_then(|v| serde_json::from_value(v).ok()).unwrap_or_else(|| self.grid())
    }

    /// Défaut du paramètre `grid` déclaré par le widget de la slide, tel qu'écrit.
    pub fn declared_grid(&self, registry: &WidgetRegistry) -> Option<Value> {
        registry
            .get(&self.kind)
            .and_then(|w| w.param("grid"))
            .filter(|p| matches!(p.ty, ParamType::Grid))
            .and_then(|p| p.default.clone())
            .filter(|v| v.is_object())
    }
}

/// Cadre en pt sur la grille 1920 × 1080, relatif à la zone de corps.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Frame {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Grille de mise en page : la zone de corps découpée en colonnes et lignes
/// égales, séparées par une gouttière.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Grid {
    pub columns: u32,
    pub rows: u32,
    /// Gouttière : jeton d'espacement (`"lg"`) ou pt ; défaut `"lg"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap: Option<TokenValue>,
}

/// Place d'un élément sur la grille : première colonne et première ligne
/// (à partir de 0), puis nombre de colonnes et de lignes couvertes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Cell {
    pub col: u32,
    pub row: u32,
    #[serde(default = "one")]
    pub cols: u32,
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
    pub fn fits(&self, grid: &Grid) -> bool {
        self.cols >= 1 && self.rows >= 1 && self.col + self.cols <= grid.columns && self.row + self.rows <= grid.rows
    }
}

/// Une valeur de jeton : un nom (`"accent"`, `"body"`) ou un nombre (pt).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum TokenValue {
    Number(f64),
    Name(String),
}

/// Style d'un élément. Chaque champ absent hérite du thème.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Style {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<TokenValue>,
    /// Nom de couleur du thème ou `#rrggbb`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<TokenValue>,
    /// Rôle de police (`display`, `body`, `mono`) ou famille.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    /// `left`, `center` ou `right`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leading: Option<TokenValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<String>,
    #[serde(default, rename = "stroke-width", skip_serializing_if = "Option::is_none")]
    pub stroke_width: Option<TokenValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<TokenValue>,
}

impl Style {
    /// Applique les champs définis de `patch` sur `self`.
    pub fn merge(&mut self, patch: Style) {
        macro_rules! take {
            ($($f:ident),*) => { $( if patch.$f.is_some() { self.$f = patch.$f; } )* };
        }
        take!(size, color, weight, font, italic, align, leading, fill, stroke, stroke_width, radius);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ListKind {
    Bullet,
    Number,
}

/// Un paragraphe : une suite de runs, éventuellement dans une liste.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Paragraph {
    #[serde(default)]
    pub runs: Vec<Run>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list: Option<ListKind>,
}

impl Paragraph {
    pub fn plain(t: impl Into<String>) -> Self {
        Self { runs: vec![Run::plain(t)], list: None }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Run {
    pub t: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bold: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub italic: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<TokenValue>,
    /// Cible du lien (`http`, `https` ou `mailto`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    /// Pastille : numéro (dès 1) de la couleur de fond dans la palette du thème.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highlight: Option<u8>,
}

impl Run {
    pub fn plain(t: impl Into<String>) -> Self {
        Self { t: t.into(), ..Default::default() }
    }
}

/// Un élément : un widget de catégorie `element`, placé et stylé.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Element {
    pub id: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<Frame>,
    /// Accroche à la grille de la slide ; prioritaire sur `frame`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell: Option<Cell>,
    #[serde(default, skip_serializing_if = "is_default_style")]
    pub style: Style,
    #[serde(default, skip_serializing_if = "Params::is_empty")]
    pub params: Params,
}

fn is_default_style(s: &Style) -> bool {
    *s == Style::default()
}

impl Element {
    pub fn new(id: impl Into<String>, kind: &str) -> Self {
        Self {
            id: id.into(),
            kind: kind.to_owned(),
            frame: None,
            cell: None,
            style: Style::default(),
            params: Params::default(),
        }
    }

    /// Un élément `text` dont le corps (`body`) est `paragraphs`.
    pub fn text(id: impl Into<String>, paragraphs: Vec<Paragraph>) -> Self {
        let mut e = Self::new(id, "text");
        e.params.set("body", ParamValue::Rich(paragraphs));
        e
    }
}

impl Deck {
    /// Deck vide de la version courante : aucune slide.
    pub fn empty() -> Self {
        Self {
            version: DECK_VERSION,
            theme: None,
            params: DeckParams::default(),
            vars: BTreeMap::new(),
            tokens: Value::Null,
            templates: None,
            slides: Vec::new(),
        }
    }

    pub fn slide(&self, id: &str) -> Option<&Slide> {
        self.slides.iter().find(|s| s.id == id)
    }

    pub fn slide_mut(&mut self, id: &str) -> Option<&mut Slide> {
        self.slides.iter_mut().find(|s| s.id == id)
    }

    pub fn slide_index(&self, id: &str) -> Option<usize> {
        self.slides.iter().position(|s| s.id == id)
    }

    /// Lit un deck JSON. Refuse un deck d'une autre version que
    /// [`DECK_VERSION`] : relu comme un v4, il perdrait des champs en silence.
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

    /// JSON lisible (indenté), tel qu'il est enregistré dans les sources.
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod model_tests;
