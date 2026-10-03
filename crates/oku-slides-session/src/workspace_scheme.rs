//! Schéma de l'interface (clair, sombre ou celui du système), choisi dans
//! les réglages et gardé dans `config.toml` comme la langue.

use snafu::ResultExt;

use crate::error::{ResponseSnafu, Result};

use super::Workspace;

/// Les schémas qu'on peut imposer ; un autre nom vaut « celui du système ».
pub(super) fn known(scheme: &str) -> Option<&'static str> {
    ["light", "dark"].into_iter().find(|s| *s == scheme)
}

impl Workspace {
    /// Le schéma choisi (`null` : celui du système).
    pub(super) fn scheme_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&serde_json::json!({ "scheme": self.chosen_scheme })).context(ResponseSnafu)?)
    }

    /// Retient le schéma de l'interface (inconnu : celui du système) et
    /// l'écrit dans `config.toml` ; un échec d'écriture ne coûte que le
    /// prochain lancement.
    pub(super) fn set_scheme(&mut self, scheme: Option<&str>) {
        self.chosen_scheme = scheme.and_then(known);
        let value = self.chosen_scheme.map_or(serde_json::Value::Null, serde_json::Value::from);
        let file = self.platform.settings.config_file();
        if let Err(e) =
            oku_slides_platform::config_write::write_value(&self.platform.storage, &file, "appearance.scheme", &value)
        {
            tracing::warn!(path = %file.display(), error = %e, "schéma non gardé pour le prochain lancement");
        }
    }
}
