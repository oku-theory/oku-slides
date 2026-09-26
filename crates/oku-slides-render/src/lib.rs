//! Rendu des decks Oku Slides avec Typst embarqué.
//!
//! Le deck est exposé à Typst en JSON (`/deck.json`) et rendu par la
//! bibliothèque `oku-slides` (`typst/oku-slides`), embarquée dans le binaire. Aucun
//! texte du deck n'est interprété comme du code Typst.
//!
//! Le même document compilé donne le SVG de chaque slide (affichage) et le PDF
//! (enregistrement) : ce qu'on voit est ce qui sort.

mod charts;
mod diagnostic;
mod error;
mod fonts;
mod world;

use std::cell::OnceCell;
use std::collections::BTreeMap;

use serde_json::Value as Json;
use snafu::OptionExt;
use typst::WorldExt;
use typst::diag::SourceDiagnostic;
use typst::foundations::{Label, Selector};
use typst::introspection::Introspector;
use typst::utils::PicoStr;
use typst_layout::PagedDocument;

use oku_slides_core::Deck;

pub use diagnostic::{Diagnostic, Severity};
use error::{CompileSnafu, NoPageSnafu, PdfSnafu};
pub use error::{RenderError, RenderErrorKind};
pub use fonts::Fonts;
use world::OkuSlidesWorld;

/// Thèmes fournis par la bibliothèque Typst.
pub const THEMES: &[&str] = &["papier", "pro"];

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

/// Zone des éléments libres d'une page (pt), et sa grille éventuelle.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ZoneBox {
    pub page: usize,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    #[serde(default)]
    pub grid: Option<GridBox>,
}

/// Grille résolue : colonnes, lignes et gouttière en pt.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GridBox {
    pub columns: u32,
    pub rows: u32,
    pub gap: f64,
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
    world: OkuSlidesWorld,
}

impl std::fmt::Debug for Renderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Renderer").finish_non_exhaustive()
    }
}

impl Renderer {
    pub fn new(fonts: Fonts) -> Result<Self, RenderError> {
        Ok(Self { world: OkuSlidesWorld::new(fonts)? })
    }

    /// Remplace le deck et les assets à rendre.
    pub fn set_deck(&mut self, deck: &Deck, assets: &BTreeMap<String, Vec<u8>>) -> Result<(), RenderError> {
        self.world.set_deck(deck, assets)
    }

    pub fn compile(&self) -> Result<Compiled, RenderError> {
        let result = typst::compile::<PagedDocument>(&self.world);
        let warnings = result.warnings.iter().map(|d| self.diagnostic(d)).collect();
        match result.output {
            Ok(doc) => Ok(Compiled { doc, warnings, layout: OnceCell::new() }),
            Err(errors) => {
                Err(CompileSnafu { diagnostics: errors.iter().map(|d| self.diagnostic(d)).collect::<Vec<_>>() }
                    .build()
                    .into())
            }
        }
    }

    /// Langue des cadres de repli (image absente, graphique non dessiné) et
    /// des libellés par défaut des graphiques ; les graphiques du deck
    /// courant sont redessinés.
    pub fn set_language(&mut self, l: &oku_slides_i18n::Localizer) -> Result<(), RenderError> {
        self.world.set_language(l)
    }

    fn diagnostic(&self, d: &SourceDiagnostic) -> Diagnostic {
        let place = d.span.id().and_then(|id| {
            let range = self.world.range(d.span)?;
            let source = self.world.source_of(id)?;
            let line = source.lines().byte_to_line(range.start)? + 1;
            Some(format!("{}:{line}", id.vpath().get_without_slash()))
        });
        Diagnostic {
            severity: match d.severity {
                typst::diag::Severity::Error => Severity::Error,
                typst::diag::Severity::Warning => Severity::Warning,
            },
            place,
            message: d.message.to_string(),
            hints: d.hints.iter().map(|h| h.v.to_string()).collect(),
        }
    }
}

/// Ce que le rendu dit du document : page de chaque slide, boîtes des
/// éléments, zones libres et jetons effectifs. Boîtes et zones sont rangées
/// par page.
#[derive(Debug, Default, serde::Serialize)]
pub struct Layout {
    /// Page de chaque slide, dans l'ordre du document.
    pub slides: Vec<SlidePage>,
    /// Boîtes réelles des éléments, pour la couche de sélection.
    pub boxes: Vec<ElementBox>,
    /// Zones des éléments libres (une par slide qui en a), avec leur grille.
    pub zones: Vec<ZoneBox>,
    /// Jetons de design effectifs du deck (défauts, thème et surcharges fusionnés).
    #[serde(skip)]
    pub tokens: Option<Json>,
}

impl Layout {
    /// Page (à partir de 1) d'une slide.
    pub fn page_of(&self, slide: &str) -> Option<usize> {
        self.slides.iter().find(|p| p.slide == slide).map(|p| p.page)
    }

    /// Boîtes des éléments d'une page.
    pub fn boxes_on(&self, page: usize) -> &[ElementBox] {
        let start = self.boxes.partition_point(|b| b.page < page);
        let end = self.boxes.partition_point(|b| b.page <= page);
        &self.boxes[start..end]
    }

    /// Zone des éléments libres d'une page.
    pub fn zone_on(&self, page: usize) -> Option<&ZoneBox> {
        self.zones.iter().find(|z| z.page == page)
    }
}

/// Les métadonnées qui se lisent comme `T` ; les autres sont ignorées.
fn parsed<T: serde::de::DeserializeOwned>(values: Vec<Json>) -> Vec<T> {
    values.into_iter().filter_map(|v| serde_json::from_value(v).ok()).collect()
}

/// Un document compilé.
pub struct Compiled {
    doc: PagedDocument,
    pub warnings: Vec<Diagnostic>,
    /// Lu dans le document une fois, au premier besoin.
    layout: OnceCell<Layout>,
}

impl std::fmt::Debug for Compiled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Compiled")
            .field("pages", &self.page_count())
            .field("warnings", &self.warnings)
            .finish_non_exhaustive()
    }
}

impl Compiled {
    pub fn page_count(&self) -> usize {
        self.doc.pages().len()
    }

    /// SVG d'une page (à partir de 0).
    pub fn svg(&self, index: usize) -> Result<String, RenderError> {
        let page = self.doc.pages().get(index).context(NoPageSnafu { index })?;
        Ok(typst_svg::svg(page, &typst_svg::SvgOptions::default()))
    }

    /// Empreinte d'une page : sert de clé de cache pour son SVG.
    pub fn page_hash(&self, index: usize) -> Option<u128> {
        self.doc.pages().get(index).map(typst::utils::hash128)
    }

    pub fn pdf(&self) -> Result<Vec<u8>, RenderError> {
        let options = typst_pdf::PdfOptions::default();
        typst_pdf::pdf(&self.doc, &options).map_err(|errs| {
            let diagnostics = errs
                .iter()
                .map(|d| Diagnostic {
                    severity: Severity::Error,
                    place: None,
                    message: d.message.to_string(),
                    hints: d.hints.iter().map(|h| h.v.to_string()).collect(),
                })
                .collect::<Vec<_>>();
            PdfSnafu { diagnostics }.build().into()
        })
    }

    /// Pages, boîtes, zones et jetons du document.
    pub fn layout(&self) -> &Layout {
        self.layout.get_or_init(|| {
            let intro = self.doc.introspector();
            let query = |label: &str| {
                // Les étiquettes sont des constantes non vides : `None` n'arrive pas.
                Label::new(PicoStr::intern(label)).map(|label| intro.query(&Selector::Label(label))).unwrap_or_default()
            };
            let values = |label: &str| -> Vec<Json> {
                query(label)
                    .iter()
                    .filter_map(|c| c.get_by_name("value").ok())
                    .filter_map(|v| serde_json::to_value(&v).ok())
                    .collect()
            };
            // Le marqueur de slide ne porte que l'id ; la page vient de sa position.
            let slides = query("oku-slides-slide")
                .iter()
                .filter_map(|c| {
                    let v = serde_json::to_value(c.get_by_name("value").ok()?).ok()?;
                    let slide = v.get("slide")?.as_str()?.to_owned();
                    let page = intro.page(c.location()?)?.get();
                    Some(SlidePage { slide, page })
                })
                .collect();
            let mut boxes: Vec<ElementBox> = parsed(values("oku-slides-box"));
            boxes.sort_by_key(|b| b.page);
            let zones = parsed(values("oku-slides-zone"));
            let tokens = values("oku-slides-tokens").into_iter().next();
            Layout { slides, boxes, zones, tokens }
        })
    }
}
