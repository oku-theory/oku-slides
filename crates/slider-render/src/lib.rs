//! Rendu des decks Écho avec Typst embarqué.
//!
//! Le deck est exposé à Typst en JSON (`/deck.json`) et rendu par la
//! bibliothèque `slider` (`typst/slider`), embarquée dans le binaire. Aucun
//! texte du deck n'est interprété comme du code Typst.
//!
//! Le même document compilé donne le SVG de chaque slide (affichage) et le PDF
//! (enregistrement) : ce qu'on voit est ce qui sort.

mod world;

use std::collections::BTreeMap;

use serde_json::Value as Json;
use thiserror::Error;
use typst::WorldExt;
use typst::diag::{Severity, SourceDiagnostic};
use typst::foundations::{Label, Selector};
use typst::introspection::Introspector;
use typst::utils::PicoStr;
use typst_layout::PagedDocument;

use slider_core::Deck;

pub use world::SliderWorld;

/// Thèmes fournis par la bibliothèque Typst.
pub const THEMES: &[&str] = &["papier", "pro"];

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("la compilation Typst a échoué :\n{}", .0.join("\n"))]
    Compile(Vec<String>),
    #[error("l'export PDF a échoué :\n{}", .0.join("\n"))]
    Pdf(Vec<String>),
    #[error("page inexistante : {0}")]
    NoPage(usize),
}

/// Boîte réelle d'un élément, en pt, mesurée par Typst.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ElementBox {
    pub id: String,
    /// Page, à partir de 1.
    pub page: usize,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// Largeur disponible (la boîte peut s'étendre jusque-là en tapant).
    #[serde(default)]
    pub aw: Option<f64>,
    /// Début de la zone disponible (le texte peut y être centré ou à droite).
    #[serde(default)]
    pub ax: Option<f64>,
    /// Typographie effective : l'éditeur de l'app la reprend telle quelle.
    #[serde(default)]
    pub style: Option<TextStyle>,
}

/// Typographie mesurée par Typst (tailles en pt).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TextStyle {
    pub size: f64,
    pub leading: f64,
    #[serde(default)]
    pub tracking: f64,
    #[serde(default)]
    pub font: Vec<String>,
    /// 400, 700… ou un nom (« bold »).
    #[serde(default)]
    pub weight: serde_json::Value,
    #[serde(default)]
    pub fill: Option<String>,
    #[serde(default)]
    pub align: Option<String>,
    #[serde(default)]
    pub case: Option<String>,
    /// Fond de la page sous le texte.
    #[serde(default)]
    pub bg: Option<String>,
}

/// Correspondance slide → page.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SlidePage {
    pub slide: String,
    pub page: usize,
}

/// Un rendu : garde un monde Typst vivant pour profiter de la compilation
/// incrémentale entre deux versions du deck.
pub struct Renderer {
    world: SliderWorld,
}

impl Renderer {
    /// `fonts` : fichiers de police (TTF/OTF) en plus des polices de Typst.
    pub fn new(fonts: impl IntoIterator<Item = Vec<u8>>) -> Self {
        Self { world: SliderWorld::new(fonts) }
    }

    /// Remplace le deck et les assets à rendre.
    pub fn set_deck(&mut self, deck: &Deck, assets: &BTreeMap<String, Vec<u8>>) {
        self.world.set_deck(deck, assets);
    }

    pub fn world(&self) -> &SliderWorld {
        &self.world
    }

    pub fn compile(&self) -> Result<Compiled, RenderError> {
        let result = typst::compile::<PagedDocument>(&self.world);
        let warnings = result.warnings.iter().map(|d| self.format(d)).collect();
        match result.output {
            Ok(doc) => Ok(Compiled { doc, warnings }),
            Err(errors) => Err(RenderError::Compile(errors.iter().map(|d| self.format(d)).collect())),
        }
    }

    fn format(&self, d: &SourceDiagnostic) -> String {
        let level = match d.severity {
            Severity::Error => "erreur",
            Severity::Warning => "avertissement",
        };
        let place = d
            .span
            .id()
            .and_then(|id| {
                let range = self.world.range(d.span)?;
                let source = self.world.source_of(id)?;
                let line = source.lines().byte_to_line(range.start)? + 1;
                Some(format!(" ({}:{line})", id.vpath().get_without_slash()))
            })
            .unwrap_or_default();
        let mut s = format!("{level}{place} : {}", d.message);
        for h in &d.hints {
            s.push_str(&format!("\n  indice : {}", h.v));
        }
        s
    }
}

/// Un document compilé.
pub struct Compiled {
    doc: PagedDocument,
    pub warnings: Vec<String>,
}

impl Compiled {
    pub fn page_count(&self) -> usize {
        self.doc.pages().len()
    }

    /// SVG d'une page (à partir de 0).
    pub fn svg(&self, index: usize) -> Result<String, RenderError> {
        let page = self.doc.pages().get(index).ok_or(RenderError::NoPage(index))?;
        Ok(typst_svg::svg(page, &typst_svg::SvgOptions::default()))
    }

    /// Empreinte d'une page : sert de clé de cache pour son SVG.
    pub fn page_hash(&self, index: usize) -> Option<u128> {
        self.doc.pages().get(index).map(typst::utils::hash128)
    }

    pub fn pdf(&self) -> Result<Vec<u8>, RenderError> {
        let options = typst_pdf::PdfOptions::default();
        typst_pdf::pdf(&self.doc, &options)
            .map_err(|errs| RenderError::Pdf(errs.iter().map(|d| d.message.to_string()).collect()))
    }

    fn metadata(&self, label: &str) -> Vec<Json> {
        let label = Label::new(PicoStr::intern(label)).expect("étiquette valide");
        self.doc
            .introspector()
            .query(&Selector::Label(label))
            .iter()
            .filter_map(|c| c.get_by_name("value").ok())
            .filter_map(|v| serde_json::to_value(&v).ok())
            .collect()
    }

    /// Jetons de design effectifs du deck (défauts, thème et surcharges fusionnés).
    pub fn tokens(&self) -> Option<Json> {
        self.metadata("slider-tokens").into_iter().next()
    }

    /// Boîtes réelles des éléments, pour la couche de sélection.
    pub fn element_boxes(&self) -> Vec<ElementBox> {
        self.metadata("slider-box").into_iter().filter_map(|v| serde_json::from_value(v).ok()).collect()
    }

    /// Page de chaque slide.
    pub fn slide_pages(&self) -> Vec<SlidePage> {
        // Le marqueur ne porte que l'id ; la page vient de sa position.
        let label = Label::new(PicoStr::intern("slider-slide")).expect("étiquette valide");
        let intro = self.doc.introspector();
        intro
            .query(&Selector::Label(label))
            .iter()
            .filter_map(|c| {
                let v = serde_json::to_value(c.get_by_name("value").ok()?).ok()?;
                let slide = v.get("slide")?.as_str()?.to_owned();
                let page = intro.page(c.location()?)?.get();
                Some(SlidePage { slide, page })
            })
            .collect()
    }
}

/// Raccourci : rend un deck en une fois.
pub fn render(deck: &Deck, assets: &BTreeMap<String, Vec<u8>>, fonts: Vec<Vec<u8>>) -> Result<Compiled, RenderError> {
    let mut r = Renderer::new(fonts);
    r.set_deck(deck, assets);
    r.compile()
}
