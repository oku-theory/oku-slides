//! Réglages de l'hôte en mode pack : le widget du sous-onglet actif, adressé
//! par clé (`host_settings`). L'interface les rend au dock de l'agent. Une
//! slide du deck de travail se règle par sa carte de cible (`target_params`) :
//! c'est l'exemple de son layout.

use crate::agent::forms::{FormExtras, params_with};
use crate::agent::{ShowParams, Surface};
use crate::{Result, Session};

/// Surface des réglages du widget du sous-onglet actif.
pub(crate) const HOST_WIDGET: &str = "host-widget";
/// Surface des réglages de la cible : l'élément sélectionné, sinon le widget de la slide.
pub(crate) const HOST_TARGET: &str = "host-target";

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

    /// Réglages de l'hôte : ceux du widget `widget` en mode pack ; `None` sans lui ou ailleurs.
    ///
    /// Un sous-onglet périmé (widget supprimé, renommé) n'a pas de réglages.
    pub(crate) fn host_settings(&self, widget: Option<&str>) -> Result<Option<Surface>> {
        if !self.in_pack_mode() {
            return Ok(None);
        }
        let Some(p) = widget.and_then(|w| self.widget_settings(w).ok().flatten()) else { return Ok(None) };
        Ok(Some(params_with(HOST_WIDGET, &p.spec, &p.extras(), &self.slide_palette(), &self.localizer)))
    }
}
