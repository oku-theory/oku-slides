//! Rendu des decks Oku Slides avec Typst embarqué.
//!
//! Le deck est exposé à Typst en JSON (`/deck.json`) et rendu par la
//! bibliothèque `oku-slides` (`typst/oku-slides`), embarquée dans le binaire. Aucun
//! texte du deck n'est interprété comme du code Typst.
//!
//! Le même document compilé donne le SVG de chaque slide (affichage) et le PDF
//! (enregistrement) : ce qu'on voit est ce qui sort.
//!
//! - [`Renderer`] : garde un monde Typst vivant entre deux rendus
//!   ([`Renderer::render`], [`Renderer::set_deck`]) ; ses widgets ([`Renderer::set_widgets`],
//!   [`Renderer::registry`]) sont ceux d'un [`PackSet`], qualifiés (le noyau n'en a aucun) ;
//!   [`Renderer::with_builtins`] y met les packs intégrés `base` et `essentiel` ; rend un
//!   [`Compiled`] (SVG, PNG, PDF, [`Layout`]).
//! - [`Layout`] : ce que le rendu dit du document — [`SlidePage`], [`ElementBox`],
//!   [`FieldBox`], [`FieldSide`], [`ItemBox`], [`ZoneBox`]/[`GridBox`], [`TextStyle`] — lu une fois depuis le document
//!   compilé.
//! - Le registre des widgets ([`load_registry`], [`Registries`]) se lit par une sonde
//!   Typst (packs assemblés) ou fichier par fichier ([`probe_file`], [`assemble`]) ;
//!   [`base_probe`] (`base` seul), [`builtin_registry`] (`base` et `essentiel`).
//! - Les packs intégrés ([`BuiltinPack`] : `base`, widgets ; `essentiel`, layouts) et leurs
//!   versions livrées, de confiance ([`is_trusted_pack`]) ; ceux qu'un échec borné n'accuse
//!   jamais ([`is_never_blamed`]).

mod builtin_packs;
mod charts;
mod diagnostic;
mod error;
mod fonts;
mod palette;
mod probe_cache;
mod registry;
mod tables;
mod widget_probe;
mod wire;
mod world;

use std::cell::OnceCell;
use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::Value as Json;
use snafu::{OptionExt, ResultExt};
use typst::foundations::{Label, Selector};
use typst::introspection::Introspector;
use typst::utils::PicoStr;
use typst_layout::PagedDocument;

use oku_slides_core::{Deck, WidgetRegistry};

pub use builtin_packs::{BuiltinPack, builtin_component_defaults, builtin_pack_set, is_never_blamed, is_trusted_pack};
pub use diagnostic::{Diagnostic, Severity, SourceAt};
use error::{BaseRegistrySnafu, CompileSnafu, ImageSnafu, NoPageSnafu, PdfSnafu};
pub use error::{RegistryError, RegistryErrorKind, RenderError, RenderErrorKind};
pub use fonts::Fonts;
pub use palette::{chart_palette, resolve_color};
pub use probe_cache::FileProbeCache;
pub use registry::{
    PackSet, PackWidgets, Registries, base_probe, builtin_registry, is_pack_id, load_registry, probe_registry,
    registry_from_probe,
};
pub use widget_probe::{FileProbe, PROBE_PACK, ProbedPack, assemble, assemble_set, probe_file, probe_key};
pub use wire::WireError;
use world::OkuSlidesWorld;

/// Boîte réelle d'un élément, en pt, mesurée par Typst.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ElementBox {
    /// Id of the element in the deck.
    pub id: String,
    /// Page, starting at 1.
    pub page: usize,
    /// Left edge, from the left edge of the page.
    pub x: f64,
    /// Top edge, from the top of the page.
    pub y: f64,
    /// Rendered width.
    pub w: f64,
    /// Rendered height.
    pub h: f64,
    /// Largeur disponible (la boîte peut s'étendre jusque-là en tapant).
    #[serde(default)]
    pub aw: Option<f64>,
    /// Start of the available area (the text may be centered or right-aligned in it).
    #[serde(default)]
    pub ax: Option<f64>,
    /// Typographie effective : l'éditeur de l'app la reprend telle quelle.
    #[serde(default)]
    pub style: Option<TextStyle>,
}

/// Boîte d'un champ éditable : un paramètre texte d'un élément ou d'une slide.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FieldBox {
    /// Id de l'élément ou de la slide.
    pub owner: String,
    /// Nom du paramètre.
    pub field: String,
    /// Page, starting at 1.
    pub page: usize,
    /// Left edge, from the left edge of the page.
    pub x: f64,
    /// Top edge, from the top of the page.
    pub y: f64,
    /// Rendered width.
    pub w: f64,
    /// Rendered height.
    pub h: f64,
    /// Start of the available area (the text may be centered or right-aligned in it).
    #[serde(default)]
    pub ax: Option<f64>,
    /// Available width (the field may grow up to it while typing).
    #[serde(default)]
    pub aw: Option<f64>,
    /// Effective typography, reused by the in-place editor.
    #[serde(default)]
    pub style: Option<TextStyle>,
    /// Champ sans texte : placeholder à dessiner par l'interface.
    #[serde(default)]
    pub empty: bool,
    /// Champ vide sans place au rendu : dessiné avant ou après son ancre (`y`).
    #[serde(default)]
    pub side: Option<FieldSide>,
}

/// Boîte d'un item de liste (`list-item`), pour les poignées de la slide.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ItemBox {
    /// Id de l'élément ou de la slide.
    pub owner: String,
    /// Nom du paramètre `list`.
    pub list: String,
    /// Id de l'item.
    pub item: String,
    /// Page, starting at 1.
    pub page: usize,
    /// Left edge, from the left edge of the page.
    pub x: f64,
    /// Top edge, from the top of the page.
    pub y: f64,
    /// Rendered width.
    pub w: f64,
    /// Rendered height.
    pub h: f64,
}

/// Où se dessine un champ vide sans place au rendu, par rapport à son ancre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldSide {
    /// Au-dessus : l'ancre est le haut de l'item suivant.
    Before,
    /// Au-dessous : l'ancre est le bas de l'item précédent.
    After,
}

/// Zone des éléments libres d'une page (pt), et sa grille éventuelle.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ZoneBox {
    /// Page, starting at 1.
    pub page: usize,
    /// Left edge, from the left edge of the page.
    pub x: f64,
    /// Top edge, from the top of the page.
    pub y: f64,
    /// Width of the area.
    pub w: f64,
    /// Height of the area.
    pub h: f64,
    /// Placement grid, if the layout declares one.
    #[serde(default)]
    pub grid: Option<GridBox>,
}

/// Grille résolue : colonnes, lignes et gouttière en pt.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GridBox {
    /// Number of columns.
    pub columns: u32,
    /// Number of rows.
    pub rows: u32,
    /// Gutter between cells, in pt.
    pub gap: f64,
}

/// Typographie mesurée par Typst (tailles en pt).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TextStyle {
    /// Font size.
    pub size: f64,
    /// Typst line spacing (`par.leading`), the space between two lines.
    pub leading: f64,
    /// Tracking (letter spacing).
    #[serde(default)]
    pub tracking: f64,
    /// Font families, in order of preference.
    #[serde(default)]
    pub font: Vec<String>,
    /// 400, 700… ou un nom (« bold »).
    #[serde(default)]
    pub weight: serde_json::Value,
    /// Text color, as Typst writes it.
    #[serde(default)]
    pub fill: Option<String>,
    /// Horizontal alignment ("left", "center"…).
    #[serde(default)]
    pub align: Option<String>,
    /// Forced case ("upper", "lower"), if the style imposes one.
    #[serde(default)]
    pub case: Option<String>,
    /// Fond de la page sous le texte.
    #[serde(default)]
    pub bg: Option<String>,
}

/// Correspondance slide → page.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SlidePage {
    /// Id of the slide.
    pub slide: String,
    /// Page of the slide, starting at 1.
    pub page: usize,
}

/// Un rendu : garde un monde Typst vivant pour profiter de la compilation
/// incrémentale entre deux versions du deck. Les slides et les éléments sont
/// rendus par les widgets des packs, sous leur nom qualifié
/// ([`Renderer::set_widgets`]) ; le noyau n'en déclare aucun.
pub struct Renderer {
    world: OkuSlidesWorld,
    /// Registre des widgets rendus (widgets qualifiés des packs).
    registry: Arc<WidgetRegistry>,
}

impl std::fmt::Debug for Renderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Renderer").finish_non_exhaustive()
    }
}

impl Renderer {
    /// A renderer without packs: no widget, each slide renders the "unknown slide" frame.
    ///
    /// # Errors
    ///
    /// Fails if the Typst world cannot be built (unreadable embedded library).
    pub fn new(fonts: Fonts) -> Result<Self, RenderError> {
        Ok(Self { world: OkuSlidesWorld::new(fonts)?, registry: registry::empty_registry() })
    }

    /// A renderer with the built-in packs: `base` (widgets) and `essentiel` (layouts).
    ///
    /// # Errors
    ///
    /// Fails if the built-in pack registry cannot be probed or if the world cannot be built.
    pub fn with_builtins(fonts: Fonts) -> Result<Self, RenderError> {
        let registry = registry::builtin_registry(&fonts).map_err(Box::new).context(BaseRegistrySnafu)?;
        let set = builtin_pack_set().map_err(Box::new).context(BaseRegistrySnafu)?;
        let mut r = Self::new(fonts)?;
        r.set_widgets(&set, registry)?;
        Ok(r)
    }

    /// Rendered widgets: those of each pack of `set`, under their qualified name. `registry` is this set's registry ([`Registries::get`]);
    /// it also tells which elements carry a chart, redrawn here.
    ///
    /// # Errors
    ///
    /// Fails if a pack path is invalid or if a chart cannot be redrawn.
    pub fn set_widgets(&mut self, set: &PackSet, registry: Arc<WidgetRegistry>) -> Result<(), RenderError> {
        self.world.set_packs(set, &registry)?;
        self.registry = registry;
        Ok(())
    }

    /// Registre des widgets rendus.
    pub fn registry(&self) -> &Arc<WidgetRegistry> {
        &self.registry
    }

    /// Replaces the deck and assets to render.
    ///
    /// # Errors
    ///
    /// Fails if the deck cannot be serialized or if one of its charts cannot be drawn.
    pub fn set_deck(&mut self, deck: &Deck, assets: &BTreeMap<String, Vec<u8>>) -> Result<(), RenderError> {
        self.world.set_deck(deck, assets, &self.registry)
    }

    /// Theme tokens for the next renders (`None`: default tokens). They
    /// replace `deck.theme`, which the renderer no longer reads; the charts of the
    /// current deck are redrawn.
    ///
    /// # Errors
    ///
    /// Fails if a chart of the current deck cannot be redrawn.
    pub fn set_theme(&mut self, tokens: Option<serde_json::Map<String, Json>>) -> Result<(), RenderError> {
        self.world.set_theme(tokens, &self.registry)
    }

    /// Renders `deck`. Charts are drawn before Typst, with the palette,
    /// font, text and box sizes of the previous compilation: if the
    /// compiled tokens give different ones (theme changed, tokens overridden)
    /// or if a chart's box has another size (chart in the flow or on a
    /// cell), they are redrawn and the deck recompiled once.
    ///
    /// # Errors
    ///
    /// Fails if the deck cannot be prepared or if Typst returns compilation errors.
    pub fn render(&mut self, deck: &Deck, assets: &BTreeMap<String, Vec<u8>>) -> Result<Compiled, RenderError> {
        self.set_deck(deck, assets)?;
        let compiled = self.compile()?;
        let layout = compiled.layout();
        let boxes = layout.boxes.iter().map(|b| (b.id.as_str(), b.w, b.h));
        if self.world.fit_charts(layout.tokens.as_ref(), boxes, &self.registry)? {
            return self.compile();
        }
        Ok(compiled)
    }

    /// Palette des graphiques du dernier rendu (celle du thème du deck).
    pub fn chart_palette(&self) -> &[String] {
        self.world.chart_palette()
    }

    /// Compiles the current deck as is, without redrawing the charts.
    ///
    /// # Errors
    ///
    /// Fails with the Typst diagnostics if the compilation returns errors.
    pub fn compile(&self) -> Result<Compiled, RenderError> {
        let result = typst::compile::<PagedDocument>(&self.world);
        let warnings = result.warnings.iter().map(|d| self.world.diagnostic(d)).collect();
        match result.output {
            Ok(doc) => Ok(Compiled { doc, warnings, layout: OnceCell::new() }),
            Err(errors) => {
                Err(CompileSnafu { diagnostics: errors.iter().map(|d| self.world.diagnostic(d)).collect::<Vec<_>>() }
                    .build()
                    .into())
            }
        }
    }

    /// Language of the fallback frames and of the default chart labels.
    ///
    /// Fallback frames stand for a missing image or an undrawn chart; the charts of
    /// the current deck are redrawn.
    ///
    /// # Errors
    ///
    /// Fails if a chart of the current deck cannot be redrawn.
    pub fn set_language(&mut self, l: &oku_slides_i18n::Localizer) -> Result<(), RenderError> {
        self.world.set_language(l, &self.registry)
    }
}

/// Ce que le rendu dit du document : page de chaque slide, boîtes des
/// éléments, zones libres et jetons effectifs. Boîtes et zones sont rangées
/// par page.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Layout {
    /// Page de chaque slide, dans l'ordre du document.
    pub slides: Vec<SlidePage>,
    /// Boîtes réelles des éléments, pour la couche de sélection.
    pub boxes: Vec<ElementBox>,
    /// Champs éditables, pour l'édition en place.
    pub fields: Vec<FieldBox>,
    /// Boîtes des items de liste, pour leurs poignées.
    #[serde(default)]
    pub items: Vec<ItemBox>,
    /// Zones des éléments libres (une par slide qui en a), avec leur grille.
    pub zones: Vec<ZoneBox>,
    /// Jetons de design effectifs du deck (défauts, thème et surcharges fusionnés).
    #[serde(skip)]
    pub tokens: Option<Json>,
}

impl Layout {
    /// Page (à partir de 1) d'une slide.
    #[must_use]
    pub fn page_of(&self, slide: &str) -> Option<usize> {
        self.slides.iter().find(|p| p.slide == slide).map(|p| p.page)
    }

    /// Boîtes des éléments d'une page.
    #[must_use]
    pub fn boxes_on(&self, page: usize) -> &[ElementBox] {
        let start = self.boxes.partition_point(|b| b.page < page);
        let end = self.boxes.partition_point(|b| b.page <= page);
        &self.boxes[start..end]
    }

    /// Zone des éléments libres d'une page.
    #[must_use]
    pub fn zone_on(&self, page: usize) -> Option<&ZoneBox> {
        self.zones.iter().find(|z| z.page == page)
    }

    /// Champs éditables d'une page.
    #[must_use]
    pub fn fields_on(&self, page: usize) -> &[FieldBox] {
        let start = self.fields.partition_point(|f| f.page < page);
        let end = self.fields.partition_point(|f| f.page <= page);
        &self.fields[start..end]
    }

    /// Boîtes des items de liste d'une page.
    #[must_use]
    pub fn items_on(&self, page: usize) -> &[ItemBox] {
        let start = self.items.partition_point(|i| i.page < page);
        let end = self.items.partition_point(|i| i.page <= page);
        &self.items[start..end]
    }
}

/// Les métadonnées qui se lisent comme `T` ; les autres sont ignorées.
fn parsed<T: serde::de::DeserializeOwned>(values: Vec<Json>) -> Vec<T> {
    values.into_iter().filter_map(|v| serde_json::from_value(v).ok()).collect()
}

/// Échelle admise pour une image : au-delà, l'image serait démesurée.
const PNG_SCALE: std::ops::RangeInclusive<f32> = 0.1..=10.0;

/// A compiled document, from which SVG, PNG, PDF and [`Layout`] are drawn.
pub struct Compiled {
    doc: PagedDocument,
    /// Warnings from the Typst compilation.
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
    /// Number of pages of the document.
    pub fn page_count(&self) -> usize {
        self.doc.pages().len()
    }

    /// SVG of a page (0-based).
    ///
    /// # Errors
    ///
    /// Fails if page `index` does not exist.
    pub fn svg(&self, index: usize) -> Result<String, RenderError> {
        let page = self.doc.pages().get(index).context(NoPageSnafu { index })?;
        Ok(typst_svg::svg(page, &typst_svg::SvgOptions::default()))
    }

    /// PNG of page `index` (0 = first), at `pixel_per_pt` pixels per point.
    ///
    /// # Errors
    ///
    /// Fails if the scale is outside the allowed range, if the page does not exist or if PNG encoding fails.
    pub fn png(&self, index: usize, pixel_per_pt: f32) -> Result<Vec<u8>, RenderError> {
        if !pixel_per_pt.is_finite() || !PNG_SCALE.contains(&pixel_per_pt) {
            return Err(ImageSnafu { message: format!("pixel_per_pt out of range: {pixel_per_pt}") }.build().into());
        }
        let page = self.doc.pages().get(index).context(NoPageSnafu { index })?;
        let options =
            typst_render::RenderOptions { pixel_per_pt: f64::from(pixel_per_pt).into(), ..Default::default() };
        typst_render::render(page, &options)
            .encode_png()
            .map_err(|e| ImageSnafu { message: e.to_string() }.build().into())
    }

    /// Empreinte d'une page : sert de clé de cache pour son SVG.
    pub fn page_hash(&self, index: usize) -> Option<u128> {
        self.doc.pages().get(index).map(typst::utils::hash128)
    }

    /// PDF of the whole document.
    ///
    /// # Errors
    ///
    /// Fails with the `typst_pdf` diagnostics if the export is rejected.
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
                    at: None,
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
            let mut fields: Vec<FieldBox> = parsed(values("oku-slides-field"));
            fields.sort_by_key(|f| f.page);
            let mut items: Vec<ItemBox> = parsed(values("oku-slides-item"));
            items.sort_by_key(|i| i.page);
            let zones = parsed(values("oku-slides-zone"));
            let tokens = values("oku-slides-tokens").into_iter().next();
            Layout { slides, boxes, fields, items, zones, tokens }
        })
    }
}
