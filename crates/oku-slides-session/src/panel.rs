//! Réglages de l'hôte en mode modèles : le modèle affiché ou le widget du
//! sous-onglet actif, adressés par clé (`host_settings`, `host_param`).
//! L'interface les rend au dock de l'agent.

use oku_slides_core::{GRID_TRACKS, Grid, Op, Slide, SlideField};
use oku_slides_i18n::Localizer;
use serde_json::{Value, json};
use snafu::{OptionExt, ensure};

use crate::agent::forms::{FormExtras, params_with};
use crate::agent::{FieldKind, ParamField, ShowParams, Surface};
use crate::error::{HostKeySnafu, NotEditingTemplatesSnafu};
use crate::{Result, Session};

/// Surface des réglages du modèle affiché.
pub(crate) const HOST_TEMPLATE: &str = "host-template";
/// Surface des réglages du widget du sous-onglet actif.
pub(crate) const HOST_WIDGET: &str = "host-widget";

/// Réglages de l'hôte : champs, leurs clés et des lignes en lecture seule.
#[derive(Debug)]
pub(crate) struct HostParams {
    /// Les champs, rendus par `forms::params_with`.
    pub spec: ShowParams,
    /// Clé de chaque champ, même ordre.
    pub keys: Vec<String>,
    /// Lignes en lecture seule.
    pub notes: Vec<String>,
}

impl HostParams {
    fn extras(&self) -> FormExtras<'_> {
        FormExtras { keys: &self.keys, notes: &self.notes, closable: false }
    }
}

fn text_field(label: String, value: Value, op_template: Value) -> ParamField {
    ParamField {
        label,
        kind: FieldKind::Text,
        value,
        op_template,
        min: None,
        max: None,
        step: None,
        options: Vec::new(),
        columns: Vec::new(),
        item_label: None,
    }
}

/// Réglages d'un modèle : nom, description, puis grille si `grid`.
pub(crate) fn template_params(slide: &Slide, grid: Option<Grid>, l: &Localizer) -> HostParams {
    let id = slide.id.as_str();
    let field = |key: &str| json!({ "op": "set_slide_field", "slide": id, "field": key, "value": "$value" });
    let mut fields = vec![
        text_field(l.tr("session-panel-name"), json!(slide.name.as_deref().unwrap_or_default()), field("name")),
        text_field(
            l.tr("session-panel-description"),
            json!(slide.description.as_deref().unwrap_or_default()),
            field("description"),
        ),
    ];
    let mut keys = vec!["name".to_owned(), "description".to_owned()];
    if let Some(g) = grid {
        for (key, label, value) in
            [("columns", "session-panel-columns", g.columns), ("rows", "session-panel-rows", g.rows)]
        {
            fields.push(ParamField {
                label: l.tr(label),
                kind: FieldKind::Stepper,
                value: json!(value),
                op_template: json!({ "op": "set_grid", "slide": id, key: "$value" }),
                min: Some(f64::from(*GRID_TRACKS.start())),
                max: Some(f64::from(*GRID_TRACKS.end())),
                step: None,
                options: Vec::new(),
                columns: Vec::new(),
                item_label: None,
            });
            keys.push(key.to_owned());
        }
    }
    let spec = ShowParams { title: l.tr("session-panel-template"), slide: slide.id.clone(), fields };
    HostParams { spec, keys, notes: Vec::new() }
}

impl Session {
    /// Réglages de l'hôte : le widget `widget` s'il en a, sinon le modèle `slide`.
    pub(crate) fn host_settings(
        &self,
        slide: &str,
        widget: Option<&str>,
        example: Option<&str>,
    ) -> Result<Option<Surface>> {
        if !self.in_templates() {
            return Ok(None);
        }
        let palette: Vec<String> = self.appearance.slide.values().cloned().collect();
        // Un sous-onglet périmé (widget supprimé, renommé) donne la carte du
        // modèle ; un cas périmé, celle du premier cas.
        let widget = widget
            .and_then(|w| self.widget_settings(w, example).or_else(|_| self.widget_settings(w, None)).ok().flatten());
        if let Some(p) = widget {
            return Ok(Some(params_with(HOST_WIDGET, &p.spec, &p.extras(), &palette, &self.localizer)));
        }
        let Ok(p) = self.template_host_params(slide) else { return Ok(None) };
        Ok(Some(params_with(HOST_TEMPLATE, &p.spec, &p.extras(), &palette, &self.localizer)))
    }

    /// Applique le réglage `key` du modèle `slide` : un pas d'annulation.
    pub(crate) fn host_param(&mut self, slide: &str, key: &str, value: Value) -> Result<()> {
        ensure!(self.in_templates(), NotEditingTemplatesSnafu);
        let p = self.template_host_params(slide)?;
        let field =
            p.keys.iter().position(|k| k == key).and_then(|i| p.spec.fields.get(i)).context(HostKeySnafu { key })?;
        let op = self.param_op(field, &value)?;
        let label = self.host_label(slide, &op);
        self.apply(&label, vec![op], None)
    }

    fn template_host_params(&self, slide: &str) -> Result<HostParams> {
        let view = self.slide(slide)?;
        let s = view.slide;
        // Une grille cacherait des blocs ; un widget sans paramètre `grid` n'en a pas de réglable.
        let has_blocks = s.params.slot("blocks").is_some_and(|b| !b.is_empty());
        let declares_grid = self.registry.get(&s.kind).and_then(|w| w.param("grid")).is_some();
        let grid = (view.zone.is_some() && declares_grid && !has_blocks).then(|| s.resolved_grid(&self.registry));
        Ok(template_params(s, grid, &self.localizer))
    }

    /// Libellé d'annulation d'un réglage du modèle.
    fn host_label(&self, slide: &str, op: &Op) -> String {
        let l = &self.localizer;
        match op {
            Op::SetGrid { columns, rows, .. } => {
                let g = self.deck().slide(slide).map(|s| s.resolved_grid(&self.registry));
                let c = columns.or(g.as_ref().map(|g| g.columns)).unwrap_or_default();
                let r = rows.or(g.as_ref().map(|g| g.rows)).unwrap_or_default();
                l.tr_with("session-undo-grid", &[("columns", c.into()), ("rows", r.into())])
            }
            Op::SetSlideField { field: SlideField::Description, .. } => l.tr("session-undo-describe-template"),
            Op::SetSlideField { field: SlideField::Name, .. } => l.tr("session-undo-rename-template"),
            _ => l.tr("session-undo-host-setting"),
        }
    }
}

#[cfg(test)]
#[path = "panel_tests.rs"]
mod panel_tests;
