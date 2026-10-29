//! Host settings in pack mode, addressed by key (`host_settings`).
//!
//! They are those of the selected item's widget; the interface renders them under its preview.

use crate::Session;
use crate::agent::forms::{FormExtras, params_with};
use crate::agent::{ShowParams, Surface};

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
