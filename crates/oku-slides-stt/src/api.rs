//! Dictée par une API de transcription compatible OpenAI.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ResultExt};

use crate::api_check::{is_loopback, normalized_url, valid_api_model};
use crate::api_extra::{ApiHeader, ApiParam, HeaderValue, api_error_detail, valid_api_access};
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
    /// Nom de la variable d'environnement qui porte la clé ; vide : pas d'en-tête Bearer.
    pub key_env: String,
    /// Query parameters appended to the transcription address, such as `api-version`.
    pub query: Vec<ApiParam>,
    /// Headers sent with every request, literal or read from a variable.
    pub headers: Vec<ApiHeader>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            url: "https://api.openai.com/v1".into(),
            model: "gpt-4o-transcribe".into(),
            key_env: "OPENAI_API_KEY".into(),
            query: Vec::new(),
            headers: Vec::new(),
        }
    }
}

impl ApiConfig {
    /// The value of variable `var`; unset or blank gives `ApiKeyMissing`.
    fn secret(var: &str) -> Result<String, SttError> {
        match std::env::var(var) {
            Ok(value) if !value.trim().is_empty() => Ok(value.trim().to_owned()),
            _ => Err(ApiKeyMissingSnafu { var }.build().into()),
        }
    }

    /// The first variable of [`ApiConfig::variables`] unset or blank, never read in full.
    ///
    /// A value made only of non-ASCII blanks counts as set; [`ApiConfig::secret`] refuses it at use.
    #[must_use]
    pub fn missing_variable(&self) -> Option<&str> {
        self.variables().into_iter().find(|var| {
            !std::env::var_os(var).is_some_and(|v| v.as_encoded_bytes().iter().any(|b| !b.is_ascii_whitespace()))
        })
    }

    /// Every variable these settings read is set, without copying any value.
    pub(crate) fn key_present(&self) -> bool {
        !self.variables().is_empty() && self.missing_variable().is_none()
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

/// Un client de l'API de transcription, prêt à servir (secrets déjà lus).
pub(crate) struct ApiTranscriber {
    config: ApiConfig,
    /// Normalized transcription address with its query: the only one called.
    url: String,
    /// Headers sent, Bearer included; values read from variables are secrets.
    headers: Vec<(String, String)>,
    /// Values read from variables, and long literals, masked in API error messages.
    secrets: Vec<String>,
    agent: ureq::Agent,
}

impl fmt::Debug for ApiTranscriber {
    // Neither the key nor any header appears.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApiTranscriber").field("config", &self.config).finish_non_exhaustive()
    }
}

impl ApiTranscriber {
    /// Refuses invalid settings, reads the secrets and prepares the HTTP agent.
    ///
    /// No redirects; no proxy to this machine, the environment's proxy otherwise;
    /// system certificates (t-1211).
    pub(crate) fn new(config: ApiConfig, timeout: Duration) -> Result<Self, SttError> {
        let root = match normalized_url(&config.url) {
            Some(root) if valid_api_access(&config) && valid_api_model(&config.model) => root,
            _ => return Err(ApiConfigSnafu.build().into()),
        };
        let mut url = url::Url::parse(&format!("{root}/audio/transcriptions")).ok().context(ApiConfigSnafu)?;
        if !config.query.is_empty() {
            url.query_pairs_mut().extend_pairs(config.query.iter().map(|p| (&p.name, &p.value)));
        }
        let mut headers = Vec::with_capacity(config.headers.len() + 1);
        let mut secrets = Vec::new();
        if !config.key_env.is_empty() {
            let key = ApiConfig::secret(&config.key_env)?;
            headers.push(("Authorization".to_owned(), format!("Bearer {key}")));
            secrets.push(key);
        }
        for header in &config.headers {
            let value = match &header.value {
                HeaderValue::Literal(value) => {
                    // A key pasted as a literal is still masked in API messages.
                    if value.chars().count() >= 8 {
                        secrets.push(value.clone());
                    }
                    value.clone()
                }
                HeaderValue::Env(var) => {
                    let value = ApiConfig::secret(var)?;
                    secrets.push(value.clone());
                    value
                }
            };
            headers.push((header.name.clone(), value));
        }
        let proxy = if is_loopback(&root) { None } else { ureq::Proxy::try_from_env() };
        let agent = ureq::Agent::config_builder()
            .proxy(proxy)
            .tls_config(oku_slides_platform::http::system_tls())
            .max_redirects(0)
            .timeout_global(Some(timeout))
            .http_status_as_error(false)
            .build()
            .into();
        Ok(Self { config, url: url.into(), headers, secrets, agent })
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
        let mut request = self.agent.post(&self.url).header("Content-Type", content_type);
        for (name, value) in &self.headers {
            request = request.header(name, value);
        }
        let mut response = request.send(&body[..]).map_err(Box::new).context(ApiHttpSnafu)?;
        let status = response.status().as_u16();
        let text = response.body_mut().read_to_string().map_err(Box::new).context(ApiHttpSnafu)?;
        if !(200..300).contains(&status) {
            // No message on 401/403: their body may quote a fragment of a secret.
            let detail = if matches!(status, 401 | 403) {
                None
            } else {
                let secrets: Vec<&str> = self.secrets.iter().map(String::as_str).collect();
                api_error_detail(&text, &secrets)
            };
            tracing::warn!(status, detail = detail.as_deref().unwrap_or_default(), "API de dictée");
            return Err(ApiStatusSnafu { status, detail }.build().into());
        }
        let reply: ApiReply = serde_json::from_str(&text).context(ApiResponseSnafu)?;
        Ok(Transcript { text: reply.text, language: None, no_speech: 0.0 })
    }
}

#[cfg(test)]
#[path = "api_tests.rs"]
mod api_tests;
