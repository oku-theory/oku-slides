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
//!   versions livrées, de confiance ([`is_trusted_pack`]).

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

pub use builtin_packs::{BuiltinPack, builtin_pack_set, is_trusted_pack};
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

/// Boîte d'un champ éditable : un paramètre texte d'un élément ou d'une slide.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FieldBox {
    /// Id de l'élément ou de la slide.
    pub owner: String,
    /// Nom du paramètre.
    pub field: String,
    pub page: usize,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    #[serde(default)]
    pub ax: Option<f64>,
    #[serde(default)]
    pub aw: Option<f64>,
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
    pub page: usize,
    pub x: f64,
    pub y: f64,
    pub w: f64,
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
    /// Un rendu sans pack : aucun widget, chaque slide rend le cadre « slide inconnue ».
    pub fn new(fonts: Fonts) -> Result<Self, RenderError> {
        Ok(Self { world: OkuSlidesWorld::new(fonts)?, registry: registry::empty_registry() })
    }

    /// Un rendu aux packs intégrés : `base` (widgets) et `essentiel` (layouts).
    pub fn with_builtins(fonts: Fonts) -> Result<Self, RenderError> {
        let registry = registry::builtin_registry(&fonts).map_err(Box::new).context(BaseRegistrySnafu)?;
        let set = builtin_pack_set().map_err(Box::new).context(BaseRegistrySnafu)?;
        let mut r = Self::new(fonts)?;
        r.set_widgets(&set, registry)?;
        Ok(r)
    }

    /// Widgets rendus : ceux de chaque pack de `set`, sous leur nom qualifié. `registry` est le registre de cet ensemble ([`Registries::get`]) ;
    /// il dit aussi quels éléments portent un graphique, redessinés ici.
    pub fn set_widgets(&mut self, set: &PackSet, registry: Arc<WidgetRegistry>) -> Result<(), RenderError> {
        self.world.set_packs(set, &registry)?;
        self.registry = registry;
        Ok(())
    }

    /// Registre des widgets rendus.
    pub fn registry(&self) -> &Arc<WidgetRegistry> {
        &self.registry
    }

    /// Remplace le deck et les assets à rendre.
    pub fn set_deck(&mut self, deck: &Deck, assets: &BTreeMap<String, Vec<u8>>) -> Result<(), RenderError> {
        self.world.set_deck(deck, assets, &self.registry)
    }

    /// Jetons du thème des rendus suivants (`None` : jetons par défaut). Ils
    /// remplacent `deck.theme`, que le rendu ne lit plus ; les graphiques du
    /// deck courant sont redessinés.
    pub fn set_theme(&mut self, tokens: Option<serde_json::Map<String, Json>>) -> Result<(), RenderError> {
        self.world.set_theme(tokens, &self.registry)
    }

    /// Rend `deck`. Les graphiques sont dessinés avant Typst, avec la palette,
    /// la police, les tailles de texte et de boîte de la compilation
    /// précédente : si les jetons compilés en donnent d'autres (thème changé,
    /// jetons surchargés) ou si la boîte d'un graphique a une autre taille
    /// (graphique dans le flux ou sur une cellule), ils sont redessinés et le
    /// deck recompilé une fois.
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

    /// Langue des cadres de repli (image absente, graphique non dessiné) et
    /// des libellés par défaut des graphiques ; les graphiques du deck
    /// courant sont redessinés.
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

    /// Champs éditables d'une page.
    pub fn fields_on(&self, page: usize) -> &[FieldBox] {
        let start = self.fields.partition_point(|f| f.page < page);
        let end = self.fields.partition_point(|f| f.page <= page);
        &self.fields[start..end]
    }

    /// Boîtes des items de liste d'une page.
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

/// Un document compilé.
/// Échelle admise pour une image : au-delà, l'image serait démesurée.
const PNG_SCALE: std::ops::RangeInclusive<f32> = 0.1..=10.0;

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

    /// PNG de la page `index` (0 = première), à `pixel_per_pt` pixels par point.
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
