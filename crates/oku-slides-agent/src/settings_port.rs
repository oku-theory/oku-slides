//! Les réglages de l'app pour l'agent : lus (dictée comprise), écrits dans `config.toml`
//! (au prochain lancement), sauf la dictée (`VoicePort`) et les réglages de l'agent,
//! réservés à l'utilisateur.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use oku_slides_i18n::{LiveLocalizer, Localize};
use oku_slides_mcp::{Applied, SettingsPort};
use oku_slides_platform::config_write::write_value;
use oku_slides_platform::storage::Storage;
use oku_slides_session::{Platform, Settings};
use oku_slides_stt::{Speaker, SpeechStatus, Stt, SttRequest, UnavailableReason};
use serde_json::{Value, json};

/// Implémentation de [`SettingsPort`] pour l'app et `oku-slides serve`.
#[derive(Clone)]
pub struct AppSettings {
    settings: Arc<Settings>,
    storage: Storage,
    config_file: PathBuf,
    stt: Option<Stt>,
    /// The read-aloud's voice: its status and reason.
    speaker: Option<Speaker>,
    localizer: LiveLocalizer,
}

impl std::fmt::Debug for AppSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppSettings").field("config_file", &self.config_file).finish_non_exhaustive()
    }
}

impl AppSettings {
    /// `stt` : la dictée de l'hôte (absente dans les tests).
    #[must_use]
    pub fn new(platform: &Platform, stt: Option<Stt>) -> Self {
        Self {
            settings: platform.settings.clone(),
            storage: platform.storage.clone(),
            config_file: platform.settings.config_file(),
            stt,
            speaker: None,
            localizer: platform.live_localizer.clone(),
        }
    }

    /// The same settings, with the read-aloud voice `speaker` for its status.
    #[must_use]
    pub fn with_speaker(mut self, speaker: Option<Speaker>) -> Self {
        self.speaker = speaker;
        self
    }

    /// The `speech.*` keys: the read-aloud settings, and whether a voice reads the interface language.
    fn read_speech(&self, out: &mut BTreeMap<String, Value>) {
        if let Some(stt) = &self.stt {
            let (read_aloud, config) = stt.speech_settings();
            out.insert("speech.read_aloud".into(), json!(read_aloud == Some(true)));
            out.insert("speech.voice".into(), json!(config.voice));
            out.insert("speech.rate".into(), json!(config.rate));
            out.insert("speech.in_presentation".into(), json!(config.in_presentation));
        }
        let l = self.localizer.current();
        let unavailable = SpeechStatus::Unavailable { reason: UnavailableReason::NoEngine };
        match self.speaker.as_ref().map_or(unavailable, |s| s.status(l.locale())) {
            SpeechStatus::Ready { .. } => {
                out.insert("speech.status".into(), json!("ready"));
            }
            SpeechStatus::Unavailable { reason } => {
                out.insert("speech.status".into(), json!("unavailable"));
                out.insert("speech.reason".into(), json!(reason.localize(&l)));
            }
        }
    }
}

impl SettingsPort for AppSettings {
    fn read(&self) -> BTreeMap<String, Value> {
        let mut out = BTreeMap::new();
        if let Some(stt) = &self.stt
            && let Ok(Value::Object(config)) = serde_json::to_value(stt.handle(SttRequest::Status).config)
        {
            // `version` serves the settings migration, not the agent; the read-aloud is `speech.*`.
            for (k, v) in config.into_iter().filter(|(k, _)| !matches!(k.as_str(), "version" | "read_aloud" | "speech"))
            {
                out.insert(format!("dictation.{k}"), v);
            }
        }
        self.read_speech(&mut out);
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
        // La dictée a un seul auteur de réglages : `AppVoice`, par `VoicePort`.
        if key.starts_with("dictation.") {
            return Err("dictation is set through set_dictation".to_owned());
        }
        if key.starts_with("speech.") {
            return Err("the read-aloud is set through set_speech".to_owned());
        }
        write_value(&self.storage, &self.config_file, key, value).map_err(|e| e.to_string())?;
        Ok(Applied::NextLaunch)
    }
}

#[cfg(test)]
#[path = "settings_port_tests.rs"]
mod settings_port_tests;
