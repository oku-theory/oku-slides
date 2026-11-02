//! Host settings in pack mode, addressed by key (`host_settings`).
//!
//! They are those of the selected item's widget; the interface renders them under its preview.

use oku_slides_core::{ParamType, Params, WidgetSchema};
use oku_slides_i18n::Localizer;

use crate::Session;
use crate::agent::forms::{FormExtras, params_with};
use crate::agent::{ShowParams, Surface};
use crate::param_fields::{field, kind_name, param_label, schema_default};

/// Surface of the selected item's widget settings.
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
    /// The settings of `schema` at `values`; a missing value takes the schema's default.
    ///
    /// `title_key` takes `$name`, the widget's name. `skip_host` is for an element: it silently skips the
    /// host, slot and grid params (the data pane and the stage cover them), words a fixed param's note
    /// for an element rather than for the preview, and with nothing left to show points at the Data
    /// button when a host param was skipped.
    pub(crate) fn from_schema(
        schema: &WidgetSchema,
        values: &Params,
        slide: &str,
        title_key: &str,
        skip_host: bool,
        l: &Localizer,
    ) -> Self {
        let title = l.tr_with(title_key, &[("name", kind_name(&schema.name, schema.label.as_ref(), l).into())]);
        let fixed = if skip_host { "session-element-param-fixed" } else { "session-widget-param-fixed" };
        let (mut fields, mut keys, mut notes) = (Vec::new(), Vec::new(), Vec::new());
        let mut host_data = false;
        for p in &schema.params {
            if skip_host && matches!(p.ty, ParamType::Host { .. } | ParamType::Slot { .. } | ParamType::Grid) {
                host_data |= matches!(p.ty, ParamType::Host { .. });
                continue;
            }
            let shown = values.get(&p.name).cloned().or_else(|| schema_default(p));
            match field(&schema.name, p, shown.as_ref(), l) {
                Some(f) => {
                    keys.push(p.name.clone());
                    fields.push(f);
                }
                None => notes.push(l.tr_with(fixed, &[("name", param_label(&schema.name, p, l).into())])),
            }
        }
        // Nothing left to show: a host param's data is edited with the Data button, else there is none.
        if fields.is_empty() && notes.is_empty() {
            notes.push(l.tr(if host_data { "session-element-data-only" } else { "session-widget-no-params" }));
        }
        Self { spec: ShowParams { title, slide: slide.to_owned(), fields }, keys, notes }
    }

    pub(crate) fn extras(&self) -> FormExtras<'_> {
        FormExtras { keys: &self.keys, notes: &self.notes, closable: false }
    }
}

impl Session {
    /// Les couleurs de la slide (champs couleur des surfaces).
    pub(crate) fn slide_palette(&self) -> Vec<String> {
        self.appearance.slide.values().cloned().collect()
    }

    /// Host settings: those of `widget` in pack mode; `None` without it or elsewhere.
    ///
    /// A stale selected item (widget deleted or renamed) has no settings.
    pub(crate) fn host_settings(&self, widget: Option<&str>) -> Option<Surface> {
        if !self.in_pack_mode() {
            return None;
        }
        let p = widget.and_then(|w| self.widget_settings(w).ok().flatten())?;
        Some(params_with(HOST_WIDGET, &p.spec, &p.extras(), &self.slide_palette(), &self.localizer))
    }
}

#[cfg(test)]
#[path = "panel_tests.rs"]
mod panel_tests;
