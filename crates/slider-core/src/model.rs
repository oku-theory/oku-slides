//! Modèle de données du deck, sérialisé en JSON.
//!
//! Le JSON produit est exactement celui que consomme la bibliothèque Typst
//! `slider` (`render-deck`) : les noms de champs et les valeurs par défaut
//! doivent rester alignés sur `typst/slider/src/data.typ`.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Version du format de deck.
pub const DECK_VERSION: u32 = 1;

/// Un deck complet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Deck {
    #[serde(default = "default_version")]
    pub version: u32,
    /// Thème Typst fourni par la bibliothèque (`papier`, `pro`), ou aucun.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(default)]
    pub params: Params,
    /// Variables utilisables dans les textes : `{{client}}`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vars: BTreeMap<String, Value>,
    /// Surcharges de jetons de design (fusion profonde côté Typst).
    #[serde(default, skip_serializing_if = "is_empty_object")]
    pub tokens: Value,
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
pub struct Params {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SlideKind {
    Title,
    Toc,
    Chapter,
    #[default]
    Content,
    End,
    Blank,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SlideLayout {
    #[default]
    Default,
    Columns,
    Split,
    Center,
    Quote,
}

/// Une slide. Son type fixe sa structure ; seuls les champs utiles à ce type
/// sont lus par le rendu.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Slide {
    pub id: String,
    #[serde(default)]
    pub kind: SlideKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kicker: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<SlideLayout>,
    /// Texte de la slide de fin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Blocs mis en page par le layout de la slide.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<Element>,
    /// Éléments placés librement (cadres relatifs à la zone de corps).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub elements: Vec<Element>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

impl Slide {
    pub fn new(id: impl Into<String>, kind: SlideKind) -> Self {
        Self {
            id: id.into(),
            kind,
            title: None,
            subtitle: None,
            kicker: None,
            layout: None,
            text: None,
            blocks: Vec::new(),
            elements: Vec::new(),
            notes: String::new(),
        }
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
    pub fn merge(&mut self, patch: &Style) {
        macro_rules! take {
            ($($f:ident),*) => { $( if patch.$f.is_some() { self.$f = patch.$f.clone(); } )* };
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
}

impl Run {
    pub fn plain(t: impl Into<String>) -> Self {
        Self { t: t.into(), ..Default::default() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum GroupLayoutKind {
    #[default]
    Free,
    Columns,
    Rows,
    Grid,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct GroupLayout {
    #[serde(rename = "type", default)]
    pub kind: GroupLayoutKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap: Option<TokenValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ShapeKind {
    #[default]
    Rect,
    Ellipse,
    Line,
}

/// Un élément de slide, identifié par un `id` stable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Element {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<Frame>,
    #[serde(default, skip_serializing_if = "is_default_style")]
    pub style: Style,
    #[serde(flatten)]
    pub body: ElementBody,
}

fn is_default_style(s: &Style) -> bool {
    *s == Style::default()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ElementBody {
    Text {
        #[serde(default)]
        text: Vec<Paragraph>,
    },
    Image {
        /// Identifiant de l'asset (`sha256:…`) dans les sources.
        asset: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fit: Option<String>,
    },
    Shape {
        #[serde(default)]
        shape: ShapeKind,
    },
    Card {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kicker: Option<String>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        highlight: bool,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        text: Vec<Paragraph>,
    },
    Group {
        #[serde(default)]
        layout: GroupLayout,
        #[serde(default)]
        children: Vec<Element>,
    },
}

impl Element {
    pub fn text(id: impl Into<String>, paragraphs: Vec<Paragraph>) -> Self {
        Self { id: id.into(), frame: None, style: Style::default(), body: ElementBody::Text { text: paragraphs } }
    }

    pub fn children(&self) -> &[Element] {
        match &self.body {
            ElementBody::Group { children, .. } => children,
            _ => &[],
        }
    }

    pub fn children_mut(&mut self) -> Option<&mut Vec<Element>> {
        match &mut self.body {
            ElementBody::Group { children, .. } => Some(children),
            _ => None,
        }
    }
}

impl Deck {
    pub fn slide(&self, id: &str) -> Option<&Slide> {
        self.slides.iter().find(|s| s.id == id)
    }

    pub fn slide_mut(&mut self, id: &str) -> Option<&mut Slide> {
        self.slides.iter_mut().find(|s| s.id == id)
    }

    pub fn slide_index(&self, id: &str) -> Option<usize> {
        self.slides.iter().position(|s| s.id == id)
    }

    pub fn from_json(s: &str) -> serde_json::Result<Self> {
        serde_json::from_str(s)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("un deck se sérialise toujours")
    }
}
