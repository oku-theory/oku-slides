//! Langue de l'interface : ses messages, et le choix gardé dans `config.toml`.

use std::sync::Arc;

use snafu::ResultExt;

use super::Workspace;
use crate::Localizer;
use crate::error::{ResponseSnafu, Result};

impl Workspace {
    /// Les messages de l'interface dans la langue choisie, et le choix
    /// (`null` : langue du système).
    pub(super) fn messages_json(&self) -> Result<String> {
        let l = self.platform.live_localizer.current();
        let messages = serde_json::json!({
            "locale": l.locale(),
            // Les raccourcis des messages dans la notation de la plateforme (`⌘Z` ou `Ctrl+Z`).
            "resources": l.ui_messages().into_iter().map(oku_slides_i18n::platform_keys).collect::<Vec<_>>(),
            "chosen": self.chosen_locale,
        });
        Ok(serde_json::to_string(&messages).context(ResponseSnafu)?)
    }

    /// Retient la langue de l'interface (inconnue : celle du système), y passe
    /// les textes de l'agent et l'écrit dans `config.toml` ; un échec d'écriture
    /// ne coûte que le prochain lancement.
    pub(super) fn set_locale(&mut self, locale: Option<&str>) {
        self.chosen_locale = locale.and_then(oku_slides_i18n::shipped);
        self.platform.live_localizer.set(Arc::new(Localizer::new(self.chosen_locale)));
        let value = self.chosen_locale.map_or(serde_json::Value::Null, serde_json::Value::from);
        let file = self.platform.settings.config_file();
        if let Err(e) =
            oku_slides_platform::config_write::write_value(&self.platform.storage, &file, "i18n.locale", &value)
        {
            tracing::warn!(path = %file.display(), error = %e, "langue non gardée pour le prochain lancement");
        }
    }
}
