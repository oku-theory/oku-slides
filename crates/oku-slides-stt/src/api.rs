//! Dictée par une API de transcription compatible OpenAI.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ResultExt};

use crate::api_check::{is_loopback, normalized_url, valid_api_model, valid_key_env};
use crate::error::{ApiConfigSnafu, ApiHttpSnafu, ApiKeyMissingSnafu, ApiResponseSnafu, ApiStatusSnafu, WavSnafu};
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

    /// La variable de la clé est posée et contient autre chose que des blancs ASCII.
    ///
    /// La clé n'est jamais copiée : seuls ses octets sont parcourus en place. Une valeur faite
    /// uniquement de blancs non ASCII compte comme présente ; [`ApiConfig::key`] la refuse à l'usage.
    pub(crate) fn key_present(&self) -> bool {
        std::env::var_os(&self.key_env).is_some_and(|v| v.as_encoded_bytes().iter().any(|b| !b.is_ascii_whitespace()))
    }
}

/// Encodes mono samples as 16-bit PCM WAV at 16 kHz.
///
/// A sentence too long for the header's 32-bit sizes (over 37 h) gives `Wav`.
pub(crate) fn wav(samples: &[f32]) -> Result<Vec<u8>, SttError> {
    let len = samples
        .len()
        .checked_mul(2)
        .and_then(|n| u32::try_from(n).ok())
        .filter(|&n| n <= u32::MAX - 36)
        .context(WavSnafu { reason: "phrase too long for a WAV header" })?;
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
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a sample clamped to ±1 then multiplied by i16::MAX fits in an i16"
        )]
        let sample = (s.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
        out.extend_from_slice(&sample.to_le_bytes());
    }
    Ok(out)
}

/// Assemble un corps `multipart/form-data` : champs texte, puis le WAV ; rend type et corps.
pub(crate) fn multipart(fields: &[(&str, &str)], wav: &[u8]) -> (String, Vec<u8>) {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    #[expect(clippy::cast_possible_truncation, reason = "the low 64 bits are enough: only uniqueness matters")]
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos()) as u64;
    let boundary = format!("oku-slides-{nanos:016x}{:016x}", COUNTER.fetch_add(1, Ordering::Relaxed));
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
    /// Adresse racine normalisée, la seule appelée.
    root: String,
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
    /// Refuses invalid settings, reads the key and prepares the HTTP agent.
    ///
    /// No redirects; no proxy to this machine, the environment's proxy otherwise;
    /// system certificates (t-1211).
    pub(crate) fn new(config: ApiConfig, timeout: Duration) -> Result<Self, SttError> {
        let root = match normalized_url(&config.url) {
            Some(root) if valid_key_env(&config.key_env) && valid_api_model(&config.model) => root,
            _ => return Err(ApiConfigSnafu.build().into()),
        };
        let key = config.key()?;
        let proxy = if is_loopback(&root) { None } else { ureq::Proxy::try_from_env() };
        let agent = ureq::Agent::config_builder()
            .proxy(proxy)
            .tls_config(oku_slides_platform::http::system_tls())
            .max_redirects(0)
            .timeout_global(Some(timeout))
            .http_status_as_error(false)
            .build()
            .into();
        Ok(Self { config, root, key, agent })
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
        let (content_type, body) = multipart(&fields, &wav(audio)?);
        let url = format!("{}/audio/transcriptions", self.root);
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
