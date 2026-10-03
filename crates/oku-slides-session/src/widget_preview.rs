//! Aperçu d'un widget seul : une slide temporaire au thème et aux jetons du
//! deck, rendue dans un emplacement d'essai du processus de rendu (un échec
//! borné n'y écarte pas le pack ; rien n'est ajouté au pack).
//!
//! Paramètres manquants : le `default` du schéma, sinon un exemple localisé
//! (`plain`, `rich`) ou la valeur neutre du type ; sans valeur neutre
//! (asset, slot, grille, hôte), le paramètre reste absent. Un widget
//! d'élément occupe toute la grille d'une slide `blank` ; l'aperçu donne la
//! boîte de ce qu'il y dessine (`bbox`), pour que l'interface s'y recadre.
//!
//! Les assets sont ceux du deck de l'utilisateur (mis de côté en mode
//! modèles) : un paramètre `asset` montre l'image du deck.
//!
//! Seul le pack rendu par `probe_pack` (widgets valides) est posé, avant
//! chaque rendu : après une relance du travailleur, l'emplacement d'essai
//! le retrouve. Un pack écarté malgré tout donne un diagnostic explicite.

use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use serde::Serialize;
use serde_json::{Map, Value};
use snafu::{ResultExt, ensure};

use oku_slides_core::{
    Category, Cell, Deck, Element, Paragraph, ParamType, ParamValue, Params, Slide, WidgetFile, WidgetRegistry,
    WidgetSchema,
};
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_render::{FileProbe, PackWidgets, RenderError};
use oku_slides_render_worker::RenderSlot;

use crate::error::{RegistrySnafu, RenderSnafu, UnknownWidgetSnafu};
use crate::preview_crop::{CROP_PIXEL_PER_PT, crop_box};
use crate::templates_mode::styled_like;
use crate::widgets_edit::{WidgetDiagnostic, parse_name};
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
    pub svg: Option<String>,
    #[serde(skip)]
    pub png: Option<Vec<u8>>,
    /// Boîte du dessin de l'élément sur la slide, en pt.
    ///
    /// `None` pour un widget de slide, et pour l'aperçu PNG de l'agent.
    pub bbox: Option<PreviewBox>,
    pub diagnostics: Vec<WidgetDiagnostic>,
}

/// Boîte d'un élément sur la slide d'aperçu, en pt.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct PreviewBox {
    pub x: f64,
    pub y: f64,
    pub w: f64,
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
        assets: &Assets,
    ) -> Result<Vec<u8>, RenderError> {
        let bare = without_elements(deck);
        let pack_key = pack.map(|p| {
            let mut h = DefaultHasher::new();
            p.files().iter().filter(|(n, _)| *n != shown).for_each(|f| f.hash(&mut h));
            h.finish()
        });
        let unversioned = Deck { templates: None, ..bare.clone() };
        let mut h = DefaultHasher::new();
        assets.keys().for_each(|k| k.hash(&mut h));
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
        let png = self.slot.render(&bare, assets)?.png(0, CROP_PIXEL_PER_PT)?;
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
        Mode::Deck | Mode::History { .. } => &doc.sources.assets,
        Mode::Templates { stash, .. } => &stash.sources.assets,
    }
}

/// Les paramètres de l'aperçu : ceux donnés, puis les défauts.
pub(crate) fn preview_params(schema: &WidgetSchema, given: &Map<String, Value>, l: &Localizer) -> Params {
    let mut params = Params::default();
    for p in &schema.params {
        let read = |v: &Value| serde_json::from_value::<ParamValue>(v.clone()).ok();
        let value = given
            .get(&p.name)
            .and_then(read)
            .or_else(|| p.default.as_ref().filter(|v| !v.is_null()).and_then(read))
            .or_else(|| neutral(&p.ty, l));
        if let Some(v) = value {
            params.set(p.name.clone(), v);
        }
    }
    params
}

/// La valeur d'exemple, ou neutre, d'un type de paramètre.
fn neutral(ty: &ParamType, l: &Localizer) -> Option<ParamValue> {
    match ty {
        ParamType::Plain => Some(ParamValue::Text(l.tr("widget-preview-plain"))),
        ParamType::Rich => Some(ParamValue::Rich(vec![Paragraph::plain(l.tr("widget-preview-rich"))])),
        ParamType::Flag => Some(ParamValue::Value(Value::Bool(false))),
        ParamType::Number { min, .. } => Some(ParamValue::Value(serde_json::json!(min.unwrap_or(0.0)))),
        ParamType::Choice { values } => values.first().map(|v| ParamValue::Text(v.clone())),
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
    /// Aperçu du widget `name` seul, sur une slide temporaire au style du deck.
    ///
    /// Un fichier cassé rend ses diagnostics ; un rendu raté (Typst, délai,
    /// mémoire) aussi, sans écarter le pack.
    pub fn preview_widget(
        &mut self,
        name: &str,
        params: &Map<String, Value>,
        format: PreviewFormat,
    ) -> Result<WidgetPreview> {
        let name = parse_name(name)?;
        let t = self.widget_target()?;
        // `_commun` n'est pas un widget : rien à montrer.
        ensure!(t.files.widget(&name).is_some() && !name.is_common(), UnknownWidgetSnafu { name: name.as_str() });
        let pack = PackWidgets::new(t.version.hex(), t.files.widgets().clone()).context(RegistrySnafu)?;
        let probe = self.worker.probe_pack(&pack).context(RegistrySnafu)?;
        let file = probe.files.get(&name);
        if !file.is_some_and(FileProbe::is_valid) {
            return Ok(broken(file));
        }
        let Some(schema) = probe.registry.get(name.as_str()).cloned() else { return Ok(no_schema(&name)) };
        let deck = self.preview_deck(&schema, params);
        self.render_preview(&name, probe.pack.as_ref(), probe.registry.clone(), &deck, format)
    }

    /// Le deck d'aperçu : la slide du widget, ou une `blank` qui porte l'élément.
    fn preview_deck(&self, schema: &WidgetSchema, given: &Map<String, Value>) -> Deck {
        let params = preview_params(schema, given, &self.localizer);
        let slide = match schema.category {
            Category::Slide => {
                let mut slide = Slide::new(PREVIEW_SLIDE, &schema.name);
                slide.params = params;
                slide
            }
            Category::Element => {
                let mut slide = Slide::new(PREVIEW_SLIDE, "blank");
                let grid = slide.grid();
                let mut element = Element::new(PREVIEW_ELEMENT, &schema.name);
                element.params = params;
                element.cell = Some(Cell { col: 0, row: 0, cols: grid.columns, rows: grid.rows });
                slide.params.set("elements", ParamValue::Slot(vec![element]));
                slide
            }
        };
        styled_like(self.user_deck(), vec![slide])
    }

    /// Rend `deck` dans l'emplacement d'essai, avec les widgets valides `pack`.
    fn render_preview(
        &mut self,
        name: &WidgetFile,
        pack: Option<&PackWidgets>,
        registry: Arc<WidgetRegistry>,
        deck: &Deck,
        format: PreviewFormat,
    ) -> Result<WidgetPreview> {
        let fonts = self.doc.sources.fonts.len();
        let kept = match self.widget_preview.take().filter(|p| p.fonts == fonts) {
            Some(p) => p,
            None => {
                let base = oku_slides_render::base_registry(&self.app_fonts).context(RegistrySnafu)?;
                let slot = self.worker.trial_slot(&self.render_fonts, &self.localizer, base).context(RenderSnafu)?;
                PreviewSlot { slot, fonts, bare: None }
            }
        };
        let preview = self.widget_preview.insert(kept);
        let assets = user_assets(&self.mode, &self.doc);
        // Posé avant chaque rendu : après une relance, le rejeu a pu l'écarter.
        let attempt = (|| -> std::result::Result<WidgetPreview, RenderError> {
            preview.slot.set_widgets(pack, registry)?;
            let compiled = preview.slot.render(deck, assets)?;
            let diagnostics = compiled.warnings.iter().map(WidgetDiagnostic::of).collect();
            // Seul un widget d'élément a une boîte : celle de l'élément d'aperçu.
            let cell = compiled.layout().boxes.iter().find(|b| b.id == PREVIEW_ELEMENT).map(|b| PreviewBox {
                x: b.x,
                y: b.y,
                w: b.w,
                h: b.h,
            });
            let mut out = match format {
                PreviewFormat::Svg => WidgetPreview { svg: Some(compiled.svg(0)?), diagnostics, ..Default::default() },
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
                let bare = || preview.bare_png(name, pack, deck, assets);
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
