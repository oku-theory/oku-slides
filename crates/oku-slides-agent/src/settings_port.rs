//! Les réglages de l'app pour l'agent : la dictée tout de suite, le reste
//! dans `config.toml` (au prochain lancement), sauf les réglages de l'agent,
//! réservés à l'utilisateur.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use oku_slides_mcp::{Applied, SettingsPort};
use oku_slides_platform::config_write::write_value;
use oku_slides_platform::storage::Storage;
use oku_slides_session::{Platform, Settings};
use oku_slides_stt::{Stt, SttRequest};
use serde_json::{Value, json};

/// Implémentation de [`SettingsPort`] pour l'app et `oku-slides serve`.
#[derive(Clone)]
pub struct AppSettings {
    settings: Arc<Settings>,
    storage: Storage,
    config_file: PathBuf,
    stt: Option<Stt>,
}

impl std::fmt::Debug for AppSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppSettings").field("config_file", &self.config_file).finish_non_exhaustive()
    }
}

impl AppSettings {
    /// `stt` : la dictée de l'hôte (absente dans les tests).
    pub fn new(platform: &Platform, stt: Option<Stt>) -> Self {
        Self {
            settings: platform.settings.clone(),
            storage: platform.storage.clone(),
            config_file: platform.settings.config_file(),
            stt,
        }
    }
}

impl SettingsPort for AppSettings {
    fn read(&self) -> BTreeMap<String, Value> {
        let mut out = BTreeMap::new();
        if let Some(stt) = &self.stt
            && let Ok(Value::Object(config)) = serde_json::to_value(stt.handle(SttRequest::Status).config)
        {
            // `version` sert à la migration des réglages, pas à l'agent.
            for (k, v) in config.into_iter().filter(|(k, _)| k != "version") {
                out.insert(format!("dictation.{k}"), v);
            }
        }
        out.insert("i18n.locale".into(), json!(self.settings.i18n.locale));
        out.insert("history.limit".into(), json!(self.settings.history.limit));
        let vad = &self.settings.stt.vad;
        for (k, v) in [
            ("threshold", json!(vad.threshold)),
            ("onset_frames", json!(vad.onset_frames)),
            ("pause", json!(vad.pause)),
            ("min_phrase", json!(vad.min_phrase)),
            ("long_pause", json!(vad.long_pause)),
            ("pre_roll", json!(vad.pre_roll)),
            ("post_roll", json!(vad.post_roll)),
            ("max_phrase", json!(vad.max_phrase)),
        ] {
            out.insert(format!("stt.vad.{k}"), v);
        }
        out
    }

    fn write(&self, key: &str, value: &Value) -> Result<Applied, String> {
        // L'agent ne choisit ni ne lance un autre agent : réservé au panneau de l'utilisateur.
        if key == "agent" || key.starts_with("agent.") {
            return Err("agent settings are reserved to the user".to_owned());
        }
        if let Some(field) = key.strip_prefix("dictation.") {
            let stt = self.stt.as_ref().ok_or_else(|| "dictation unavailable".to_owned())?;
            let mut config = serde_json::to_value(stt.handle(SttRequest::Status).config).map_err(|e| e.to_string())?;
            config[field] = value.clone();
            let config = serde_json::from_value(config).map_err(|e| format!("invalid {key}: {e}"))?;
            stt.handle(SttRequest::Configure(config));
            return Ok(Applied::Now);
        }
        write_value(&self.storage, &self.config_file, key, value).map_err(|e| e.to_string())?;
        Ok(Applied::NextLaunch)
    }
}

#[cfg(test)]
#[path = "settings_port_tests.rs"]
mod settings_port_tests;
