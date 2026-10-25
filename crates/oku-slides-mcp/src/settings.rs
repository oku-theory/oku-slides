//! Réglages que l'agent peut lire et, sur liste blanche, écrire.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

/// Quand un réglage écrit prend effet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Applied {
    /// Au prochain lancement de l'app.
    NextLaunch,
}

/// Implémenter ce trait pour donner à l'agent les réglages de l'app : `config.toml`
/// (au prochain lancement). La dictée et l'agent de l'app passent par `VoicePort`.
pub trait SettingsPort: Send + Sync + std::fmt::Debug {
    /// Valeurs par clé pointée (`dictation.model`, `history.limit`…).
    fn read(&self) -> BTreeMap<String, Value>;
    /// Writes an already allowed key ([`writable`]); the agent reads the error.
    ///
    /// # Errors
    ///
    /// Fails if the value is refused or if `config.toml` cannot be written; the text is for the agent.
    fn write(&self, key: &str, value: &Value) -> Result<Applied, String>;
}

/// Clés que l'agent peut écrire, hors `stt.vad.*`.
const WRITABLE: &[&str] = &["history.limit"];
/// Clés de `[stt.vad]`.
const VAD: &[&str] =
    &["threshold", "onset_frames", "pause", "min_phrase", "long_pause", "pre_roll", "post_roll", "max_phrase"];

/// La clé est sur la liste blanche de `set_setting`.
pub(crate) fn writable(key: &str) -> bool {
    WRITABLE.contains(&key) || key.strip_prefix("stt.vad.").is_some_and(|k| VAD.contains(&k))
}
