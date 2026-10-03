//! Aperçu d'un widget seul : une slide temporaire au thème et aux jetons du
//! deck, rendue dans un emplacement d'essai du processus de rendu (un échec
//! borné n'y écarte pas le pack ; rien n'est ajouté au pack).
//!
//! Paramètres manquants : le `default` du schéma, sinon un exemple localisé
//! (`plain`, `rich`) ou la valeur neutre du type ; sans valeur neutre
//! (asset, slot, grille, hôte), le paramètre reste absent. Un widget
//! d'élément occupe toute la grille d'une slide `blank`.
//!
//! Seul le pack rendu par `probe_pack` (widgets valides) est posé, avant
//! chaque rendu : après une relance du travailleur, l'emplacement d'essai
//! le retrouve. Un pack écarté malgré tout donne un diagnostic explicite.

use std::collections::BTreeMap;
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
use crate::templates_mode::styled_like;
use crate::widgets_edit::{WidgetDiagnostic, parse_name};
use crate::{Result, Session};

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
    pub diagnostics: Vec<WidgetDiagnostic>,
}

/// L'emplacement d'essai de l'aperçu et le nombre de polices du deck.
pub(crate) struct PreviewSlot {
    slot: RenderSlot,
    /// Nombre de polices du deck à sa création.
    fonts: usize,
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
        let slot = match self.widget_preview.take().filter(|p| p.fonts == fonts) {
            Some(p) => p.slot,
            None => {
                let base = oku_slides_render::base_registry(&self.app_fonts).context(RegistrySnafu)?;
                self.worker.trial_slot(&self.render_fonts, &self.localizer, base).context(RenderSnafu)?
            }
        };
        let preview = self.widget_preview.insert(PreviewSlot { slot, fonts });
        // Posé avant chaque rendu : après une relance, le rejeu a pu l'écarter.
        let attempt = (|| -> std::result::Result<WidgetPreview, RenderError> {
            preview.slot.set_widgets(pack, registry)?;
            let compiled = preview.slot.render(deck, &BTreeMap::new())?;
            let diagnostics = compiled.warnings.iter().map(WidgetDiagnostic::of).collect();
            Ok(match format {
                PreviewFormat::Svg => WidgetPreview { svg: Some(compiled.svg(0)?), png: None, diagnostics },
                PreviewFormat::Png => {
                    WidgetPreview { svg: None, png: Some(compiled.png(0, PREVIEW_PIXEL_PER_PT)?), diagnostics }
                }
            })
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
