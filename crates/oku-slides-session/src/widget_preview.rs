//! Aperçu d'un widget seul : une slide temporaire au thème et aux jetons du
//! deck, rendue dans un emplacement d'essai du processus de rendu (un échec
//! borné n'y écarte pas le pack ; rien n'est ajouté au pack).
//!
//! Paramètres : ceux donnés, puis les valeurs d'exemple du widget que le
//! schéma admet encore (`widget_examples.rs`). Manquants : le `default` du schéma, sinon un exemple localisé
//! (`plain`, `rich`) ou la valeur neutre du type ; sans valeur neutre
//! (asset, slot, grille, hôte), le paramètre reste absent. Un widget
//! d'élément occupe toute la grille d'une slide `essentiel/blank` ; un widget
//! de slide est la slide elle-même. L'aperçu donne la boîte de ce qu'un
//! élément dessine (`bbox`), pour que l'interface s'y recadre.
//!
//! Les packs de l'aperçu sont ceux qu'épingle le deck ouvert (en mode pack,
//! le deck de travail), le pack montré dans sa version éditée à leur place.
//!
//! Les assets sont ceux du deck de l'utilisateur (mis de côté en mode
//! pack) : un paramètre `asset` montre l'image du deck.
//!
//! Du pack montré, seul ce que rend `probe_pack` (widgets valides) est posé, avant
//! chaque rendu : après une relance du travailleur, l'emplacement d'essai
//! le retrouve. Un pack écarté malgré tout donne un diagnostic explicite.

use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use serde::Serialize;
use serde_json::{Map, Value};
use snafu::{ResultExt, ensure};

use oku_slides_core::{
    Category, Cell, Deck, Element, Namespace, Paragraph, ParamType, ParamValue, Params, Slide, WidgetFile,
    WidgetRegistry, WidgetSchema, default_items, kinds, qualify,
};
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_render::{FileProbe, Layout, PackSet, PackWidgets, RenderError};
use oku_slides_render_worker::RenderSlot;

use crate::deck_theme::render_themed;
use crate::error::{RegistrySnafu, RenderSnafu, UnknownWidgetSnafu};
use crate::fields;
use crate::pack_mode::styled_like;
use crate::param_fields::schema_default;
use crate::preview_crop::{CROP_PIXEL_PER_PT, crop_box};
use crate::views::{PageHash, SLIDE_HEIGHT, SLIDE_WIDTH, SlideView};
use crate::widget_preview_items::preview_items;
use crate::widgets_edit::{WidgetDiagnostic, parse_name, trial_fits};
use crate::{Document, Mode, Result, Session};

/// Identifiant de la slide d'aperçu.
const PREVIEW_SLIDE: &str = "apercu";
/// Identifiant de l'élément d'aperçu.
const PREVIEW_ELEMENT: &str = "apercu-element";
/// Pixels par point de l'aperçu PNG : 960 × 540, comme `render_slide`.
const PREVIEW_PIXEL_PER_PT: f32 = 0.5;

/// Le format de l'image d'aperçu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewFormat {
    /// Pour l'interface.
    Svg,
    /// Pour l'agent.
    Png,
}

/// L'aperçu d'un widget : son image, ou les diagnostics qui l'en empêchent.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct WidgetPreview {
    /// SVG image for the interface; `None` without a render or in PNG.
    pub svg: Option<String>,
    /// PNG bytes for the agent, never serialized to the interface.
    #[serde(skip)]
    pub png: Option<Vec<u8>>,
    /// Boîte du dessin de l'élément sur la slide, en pt.
    ///
    /// `None` pour un widget de slide, et pour l'aperçu PNG de l'agent.
    pub bbox: Option<PreviewBox>,
    /// La slide d'aperçu entière, comme `Session::slide` (une `SlideView`
    /// sérialisée : le deck d'aperçu ne survit pas à l'appel), sans cibles.
    ///
    /// Ses champs (`fields`) et items (`items`, ceux du seul widget montré,
    /// comme `lists`) sont en pt de la slide (`SLIDE_WIDTH` × `SLIDE_HEIGHT`), origine
    /// en haut à gauche. L'image couvre `bbox` pour un widget d'élément :
    /// un champ s'y place en `(x - bbox.x, y - bbox.y)` ; sans `bbox`
    /// (widget de slide), l'image couvre toute la slide. `None` pour
    /// l'aperçu PNG de l'agent et sans rendu.
    pub view: Option<Value>,
    /// Typst or check messages on the widget; an error prevents the image.
    pub diagnostics: Vec<WidgetDiagnostic>,
}

/// Boîte d'un élément sur la slide d'aperçu, en pt.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct PreviewBox {
    /// Left edge, in pt from the left edge of the slide.
    pub x: f64,
    /// Top edge, in pt from the top of the slide.
    pub y: f64,
    /// Width of the drawing, in pt.
    pub w: f64,
    /// Height of the drawing, in pt.
    pub h: f64,
}

/// L'emplacement d'essai de l'aperçu et le nombre de polices du deck.
pub(crate) struct PreviewSlot {
    slot: RenderSlot,
    /// Nombre de polices du deck à sa création.
    fonts: usize,
    /// Le PNG de la slide sans éléments, et sa clé (`BareKey`).
    bare: Option<(BareKey, Vec<u8>)>,
}

/// Ce dont dépend le PNG de la slide sans éléments.
///
/// Les fichiers du pack sauf le widget montré, et le deck sans sa version
/// de pack : chaque écriture du widget en change l'id et la version, pas
/// ce que rend une slide sans éléments.
#[derive(Debug, Clone, PartialEq, Eq)]
struct BareKey {
    pack: Option<u64>,
    /// Les identifiants des assets du deck.
    assets: u64,
    deck: String,
}

impl PreviewSlot {
    /// Le PNG de recadrage de `deck` sans ses éléments, gardé tant que sa clé vaut.
    ///
    /// Rendu après celui de l'aperçu : celui-ci n'est plus lisible ensuite.
    fn bare_png(
        &mut self,
        shown: &WidgetFile,
        pack: Option<&PackWidgets>,
        deck: &Deck,
        layer: &Map<String, Value>,
        assets: &Assets,
    ) -> Result<Vec<u8>, RenderError> {
        let bare = without_elements(deck);
        let pack_key = pack.map(|p| {
            let mut h = DefaultHasher::new();
            p.files().iter().filter(|(n, _)| *n != shown).for_each(|f| f.hash(&mut h));
            h.finish()
        });
        let unversioned = Deck { widgets: BTreeMap::default(), ..bare.clone() };
        let mut h = DefaultHasher::new();
        for k in assets.keys() {
            k.hash(&mut h);
        }
        let key = BareKey {
            pack: pack_key,
            assets: h.finish(),
            deck: serde_json::to_string(&unversioned).map_err(|e| RenderError::frame(e.to_string()))?,
        };
        if let Some((k, png)) = &self.bare
            && *k == key
        {
            return Ok(png.clone());
        }
        let png = render_themed(&mut self.slot, &bare, layer, assets)?.png(0, CROP_PIXEL_PER_PT)?;
        self.bare = Some((key, png.clone()));
        Ok(png)
    }
}

/// Les assets d'un deck, par identifiant.
type Assets = BTreeMap<String, Vec<u8>>;

/// Les assets du deck de l'utilisateur, comme `Session::user_blobs`.
///
/// Par champs : l'emplacement d'essai est emprunté en même temps. En
/// historique, le document montré a pris les assets du deck mis de côté.
fn user_assets<'a>(mode: &'a Mode, doc: &'a Document) -> &'a Assets {
    match mode {
        Mode::Deck | Mode::Help { .. } | Mode::History { .. } | Mode::Theme { .. } => &doc.sources.assets,
        Mode::Pack { stash, .. } => &stash.sources.assets,
    }
}

/// Les paramètres de l'aperçu : ceux donnés, puis les défauts.
pub(crate) fn preview_params(schema: &WidgetSchema, given: &Map<String, Value>, l: &Localizer) -> Params {
    let mut params = Params::default();
    for p in &schema.params {
        let read = |v: &Value| serde_json::from_value::<ParamValue>(v.clone()).ok();
        // Le `default` d'une liste n'a pas d'ids : ce sont ses items amorcés (`d0`…).
        let value = given.get(&p.name).and_then(read).or_else(|| schema_default(p)).or_else(|| neutral(p, l));
        if let Some(v) = value {
            params.set(p.name.clone(), v);
        }
    }
    params
}

/// La valeur d'exemple, ou neutre, d'un paramètre.
fn neutral(p: &oku_slides_core::ParamSchema, l: &Localizer) -> Option<ParamValue> {
    match &p.ty {
        ParamType::Plain => Some(ParamValue::Text(l.tr("widget-preview-plain"))),
        ParamType::Rich => Some(ParamValue::Rich(vec![Paragraph::plain(l.tr("widget-preview-rich"))])),
        ParamType::Flag => Some(ParamValue::Value(Value::Bool(false))),
        ParamType::Number { min, .. } => Some(ParamValue::Value(serde_json::json!(min.unwrap_or(0.0)))),
        ParamType::Choice { values } => values.first().map(|v| ParamValue::Text(v.clone())),
        // Les items par défaut du schéma ; sans défaut, la liste reste absente (vide).
        ParamType::List { .. } => default_items(p).ok().filter(|items| !items.is_empty()).map(ParamValue::List),
        ParamType::Asset | ParamType::Slot { .. } | ParamType::Grid | ParamType::Host { .. } => None,
    }
}

/// Un rendu d'aperçu raté : les diagnostics de Typst, ou un seul (délai, mémoire…).
pub(crate) fn failure(name: &WidgetFile, e: &RenderError, l: &Localizer) -> Vec<WidgetDiagnostic> {
    if e.diagnostics().is_empty() {
        vec![WidgetDiagnostic {
            file: Some(name.clone()),
            line: None,
            column: None,
            error: true,
            message: e.localize(l),
        }]
    } else {
        e.diagnostics().iter().map(WidgetDiagnostic::of).collect()
    }
}

/// `deck` sans les éléments de ses slides : le fond seul, pour le recadrage.
fn without_elements(deck: &Deck) -> Deck {
    let mut bare = deck.clone();
    for slide in &mut bare.slides {
        slide.params.set("elements", ParamValue::Slot(Vec::new()));
    }
    bare
}

/// La vue de la slide d'aperçu de `deck`, sans cibles, avec les items du
/// seul widget `widget`.
fn preview_view(
    deck: &Deck,
    registry: &WidgetRegistry,
    layout: &Layout,
    hash: Option<u128>,
    widget: &str,
) -> std::result::Result<Option<Value>, RenderError> {
    let Some(slide) = deck.slide(PREVIEW_SLIDE) else { return Ok(None) };
    // La seule page du deck d'aperçu : un widget de slide ne la déclare pas
    // toujours dans `layout.slides`.
    let page = 1;
    let (items, lists) = preview_items(slide, registry, layout.items_on(page), widget);
    let view = SlideView {
        slide,
        page,
        hash: PageHash(hash),
        width: SLIDE_WIDTH,
        height: SLIDE_HEIGHT,
        targets: Vec::new(),
        fields: fields::field_views(deck, slide, registry, layout.fields_on(page)),
        // Les poignées des listes du widget : l'interface les écrit dans ses valeurs d'exemple.
        items,
        lists,
        zone: layout.zone_on(page),
    };
    serde_json::to_value(view).map(Some).map_err(|e| RenderError::frame(e.to_string()))
}

/// Un fichier qui ne se charge pas : ses diagnostics, sans image.
fn broken(file: Option<&FileProbe>) -> WidgetPreview {
    WidgetPreview {
        diagnostics: file.map(|f| f.diagnostics.iter().map(WidgetDiagnostic::of).collect()).unwrap_or_default(),
        ..WidgetPreview::default()
    }
}

/// Un fichier valide absent du registre : un diagnostic, jamais un aperçu vide.
fn no_schema(name: &WidgetFile) -> WidgetPreview {
    let message = format!("widget {name} has no schema in the pack registry");
    WidgetPreview {
        diagnostics: vec![WidgetDiagnostic {
            file: Some(name.clone()),
            line: None,
            column: None,
            error: true,
            message,
        }],
        ..WidgetPreview::default()
    }
}

impl Session {
    /// Preview of widget `name` alone, on a temporary slide in the deck's style.
    ///
    /// `name` is a file of pack `pack` (see `widget_target`: without a
    /// namespace, the edited pack, or in deck mode the `deck` pack).
    ///
    /// At the widget's example values, then at `params`.
    /// A broken file returns its diagnostics; so does a failed render (Typst,
    /// timeout, memory), without dropping the pack.
    ///
    /// # Errors
    ///
    /// Fails if the targeted pack cannot be found, if `name` is invalid or unknown, or if the deck's
    /// theme cannot be composed.
    pub fn preview_widget(
        &mut self,
        pack: Option<&Namespace>,
        name: &str,
        params: &Map<String, Value>,
        format: PreviewFormat,
    ) -> Result<WidgetPreview> {
        let name = parse_name(name)?;
        let t = self.widget_target(pack)?;
        // `_commun` n'est pas un widget : rien à montrer.
        ensure!(t.files.widget(&name).is_some() && !name.is_common(), UnknownWidgetSnafu { name: name.as_str() });
        let widgets = PackWidgets::new(t.version.hex(), t.namespace.clone(), t.kind, t.files.widgets().clone())
            .context(RegistrySnafu)?;
        let probe = self.worker.probe_pack(&widgets).context(RegistrySnafu)?;
        let file = probe.files.get(&name);
        if !file.is_some_and(FileProbe::is_valid) {
            return Ok(broken(file));
        }
        let kind = qualify(&t.namespace, name.as_str());
        let Some(schema) = probe.registry.get(&kind).cloned() else { return Ok(no_schema(&name)) };
        let inputs = self.preview_inputs(pack, &name, &schema, params)?;
        let deck = self.preview_deck(&schema, &inputs);
        let pack = probe.pack.as_ref();
        let set = self.preview_set(&t.namespace, pack)?;
        let registry = self.registry_for(&set)?;
        self.render_preview(&name, (pack, &set), &registry, &deck, format)
    }

    /// Les packs de l'aperçu : ceux du deck ouvert, `pack` (le pack montré, sous `ns`) à la place du sien.
    ///
    /// Les valeurs d'exemple du pack montré peuvent porter leurs éléments, et un élément s'y
    /// montre sur leur `essentiel/blank` : jamais les packs intégrés courants à leur place (t-1057).
    pub(crate) fn preview_set(&self, ns: &Namespace, pack: Option<&PackWidgets>) -> Result<PackSet> {
        let others = self.packs.packs().iter().filter(|p| p.namespace() != ns).cloned();
        Ok(PackSet::new(others.chain(pack.cloned()).collect()).context(RegistrySnafu)?)
    }

    /// Valeurs d'aperçu de `name` (pack `pack`) : ses valeurs d'exemple encore valides, puis `given`.
    pub(crate) fn preview_inputs(
        &self,
        pack: Option<&Namespace>,
        name: &WidgetFile,
        schema: &WidgetSchema,
        given: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        let mut inputs: Map<String, Value> =
            self.example_values(pack, name)?.into_iter().filter(|(k, v)| trial_fits(schema, k, v)).collect();
        inputs.extend(given.iter().map(|(k, v)| (k.clone(), v.clone())));
        Ok(inputs)
    }

    /// Le deck d'aperçu : la slide du widget, ou une `blank` qui porte
    /// l'élément, au style du deck.
    fn preview_deck(&self, schema: &WidgetSchema, given: &Map<String, Value>) -> Deck {
        let params = preview_params(schema, given, &self.localizer);
        let slide = match schema.category {
            Category::Slide => {
                let mut slide = Slide::new(PREVIEW_SLIDE, &schema.name);
                slide.params = params;
                slide
            }
            Category::Element => {
                let mut slide = Slide::new(PREVIEW_SLIDE, kinds::essentiel::BLANK);
                let grid = slide.resolved_grid(&self.registry);
                let mut element = Element::new(PREVIEW_ELEMENT, &schema.name);
                element.params = params;
                element.cell = Some(Cell { col: 0, row: 0, cols: grid.columns, rows: grid.rows });
                slide.params.set("elements", ParamValue::Slot(vec![element]));
                slide
            }
        };
        styled_like(self.style_deck(), vec![slide])
    }

    /// Rend `deck` dans l'emplacement d'essai, avec les widgets valides de `set` (registre `registry`).
    ///
    /// Le widget montré `name`, du pack `pack` de `set`, y porte son nom qualifié, celui de son schéma.
    fn render_preview(
        &mut self,
        name: &WidgetFile,
        (pack, set): (Option<&PackWidgets>, &PackSet),
        registry: &Arc<WidgetRegistry>,
        deck: &Deck,
        format: PreviewFormat,
    ) -> Result<WidgetPreview> {
        let fonts = self.doc.sources.fonts.len();
        let layer = self.theme_layer(deck)?;
        let kept = if let Some(p) = self.widget_preview.take().filter(|p| p.fonts == fonts) {
            p
        } else {
            let none = crate::deck_pack::empty_registry();
            let slot = self.worker.trial_slot(&self.render_fonts, &self.localizer, none).context(RenderSnafu)?;
            PreviewSlot { slot, fonts, bare: None }
        };
        let preview = self.widget_preview.insert(kept);
        let assets = user_assets(&self.mode, &self.doc);
        // Posé avant chaque rendu : après une relance, le rejeu a pu l'écarter.
        let attempt = (|| -> std::result::Result<WidgetPreview, RenderError> {
            preview.slot.set_widgets(set, registry.clone())?;
            let compiled = render_themed(&mut preview.slot, deck, &layer, assets)?;
            let diagnostics = compiled.warnings.iter().map(WidgetDiagnostic::of).collect();
            // Seul un widget d'élément a une boîte : celle de l'élément d'aperçu.
            let cell = compiled.layout().boxes.iter().find(|b| b.id == PREVIEW_ELEMENT).map(|b| PreviewBox {
                x: b.x,
                y: b.y,
                w: b.w,
                h: b.h,
            });
            let mut out = match format {
                PreviewFormat::Svg => {
                    let hash = compiled.page_hash(0);
                    let kind = pack.map_or_else(|| name.as_str().to_owned(), |p| qualify(p.namespace(), name.as_str()));
                    let view = preview_view(deck, registry, compiled.layout(), hash, &kind)?;
                    WidgetPreview { svg: Some(compiled.svg(0)?), view, diagnostics, ..Default::default() }
                }
                // L'agent voit la slide entière : pas de recadrage.
                PreviewFormat::Png => {
                    let png = Some(compiled.png(0, PREVIEW_PIXEL_PER_PT)?);
                    return Ok(WidgetPreview { png, diagnostics, ..Default::default() });
                }
            };
            if let Some(cell) = cell {
                // La boîte est toute la grille : recadrée sur ce que l'élément dessine.
                // Au mieux : un échec ici garde l'aperçu déjà rendu, sur la cellule.
                let with = || compiled.png(0, CROP_PIXEL_PER_PT);
                let bare = || preview.bare_png(name, pack, deck, &layer, assets);
                out.bbox = Some(crop_box(cell, with, bare));
            }
            Ok(out)
        })();
        Ok(attempt.unwrap_or_else(|e| WidgetPreview {
            diagnostics: failure(name, &e, &self.localizer),
            ..Default::default()
        }))
    }
}

#[cfg(test)]
#[path = "widget_preview_tests.rs"]
mod widget_preview_tests;
