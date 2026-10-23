//! Dictée par une API de transcription compatible OpenAI.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use snafu::ResultExt;

use crate::error::{ApiHttpSnafu, ApiKeyMissingSnafu, ApiResponseSnafu, ApiStatusSnafu};
use crate::phrase_engine::PhraseEngine;
use crate::{SAMPLE_RATE, SttError, Transcript};

/// Réglages de l'API de transcription : adresse, modèle, variable de la clé.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    /// Adresse racine de l'API, sans `/audio/transcriptions`.
    pub url: String,
    /// Modèle de transcription demandé.
    pub model: String,
    /// Nom de la variable d'environnement qui porte la clé.
    pub key_env: String,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            url: "https://api.openai.com/v1".into(),
            model: "gpt-4o-transcribe".into(),
            key_env: "OPENAI_API_KEY".into(),
        }
    }
}

impl ApiConfig {
    /// La clé, lue dans sa variable ; absente ou vide : `ApiKeyMissing`.
    pub(crate) fn key(&self) -> Result<String, SttError> {
        match std::env::var(&self.key_env) {
            Ok(key) if !key.trim().is_empty() => Ok(key.trim().to_owned()),
            _ => Err(ApiKeyMissingSnafu { var: self.key_env.clone() }.build().into()),
        }
    }
}

/// Encode des échantillons mono en WAV PCM 16 bits à 16 kHz.
pub(crate) fn wav(samples: &[f32]) -> Vec<u8> {
    let len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&((s.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16).to_le_bytes());
    }
    out
}

/// Assemble un corps `multipart/form-data` : champs texte, puis le WAV ; rend type et corps.
pub(crate) fn multipart(fields: &[(&str, &str)], wav: &[u8]) -> (String, Vec<u8>) {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    let boundary = format!("oku-slides-{:016x}{:016x}", nanos as u64, COUNTER.fetch_add(1, Ordering::Relaxed));
    let mut body = Vec::with_capacity(wav.len() + 512);
    for (name, value) in fields {
        body.extend_from_slice(
            format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n").as_bytes(),
        );
    }
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"phrase.wav\"\r\nContent-Type: audio/wav\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(wav);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

/// Réponse de l'API : le texte transcrit.
#[derive(Deserialize)]
struct ApiReply {
    text: String,
}

/// Un client de l'API de transcription, prêt à servir (clé déjà lue).
pub(crate) struct ApiTranscriber {
    config: ApiConfig,
    key: String,
    agent: ureq::Agent,
}

impl fmt::Debug for ApiTranscriber {
    // La clé n'apparaît jamais.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApiTranscriber").field("config", &self.config).finish_non_exhaustive()
    }
}

impl ApiTranscriber {
    /// Lit la clé et prépare l'agent HTTP (proxy de l'environnement, délai global).
    pub(crate) fn new(config: ApiConfig, timeout: Duration) -> Result<Self, SttError> {
        let key = config.key()?;
        let agent = ureq::Agent::config_builder()
            .proxy(ureq::Proxy::try_from_env())
            .timeout_global(Some(timeout))
            .http_status_as_error(false)
            .build()
            .into();
        Ok(Self { config, key, agent })
    }
}

impl PhraseEngine for ApiTranscriber {
    fn transcribe(
        &self,
        audio: &[f32],
        language: Option<&str>,
        context: Option<&str>,
        _quick: bool,
    ) -> Result<Transcript, SttError> {
        let mut fields = vec![("model", self.config.model.as_str()), ("response_format", "json")];
        if let Some(language) = language {
            fields.push(("language", language));
        }
        if let Some(context) = context.map(str::trim).filter(|c| !c.is_empty()) {
            fields.push(("prompt", context));
        }
        let (content_type, body) = multipart(&fields, &wav(audio));
        let url = format!("{}/audio/transcriptions", self.config.url.trim_end_matches('/'));
        let mut response = self
            .agent
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.key))
            .header("Content-Type", content_type)
            .send(&body[..])
            .map_err(Box::new)
            .context(ApiHttpSnafu)?;
        let status = response.status().as_u16();
        let text = response.body_mut().read_to_string().map_err(Box::new).context(ApiHttpSnafu)?;
        if !(200..300).contains(&status) {
            // Seul le statut : un corps de 401 peut citer un fragment de la clé.
            tracing::warn!(status, "API de dictée");
            return Err(ApiStatusSnafu { status }.build().into());
        }
        let reply: ApiReply = serde_json::from_str(&text).context(ApiResponseSnafu)?;
        Ok(Transcript { text: reply.text, language: None, no_speech: 0.0 })
    }
}

#[cfg(test)]
#[path = "api_tests.rs"]
mod api_tests;
