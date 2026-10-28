//! Dictation through the OpenAI or Azure OpenAI transcription API, with async-openai.

use std::fmt;
use std::time::Duration;

use async_openai::Client;
use async_openai::config::{AzureConfig, Config, OpenAIConfig};
use async_openai::error::OpenAIError;
use async_openai::types::InputSource;
use async_openai::types::audio::{AudioInput, AudioResponseFormat, CreateTranscriptionRequest};
use reqwest::header::{HeaderMap, HeaderName};
use serde::{Deserialize, Serialize};
use snafu::{IntoError, OptionExt, ResultExt};

use crate::api_check::{is_loopback, normalized_url, valid_api_model};
use crate::api_extra::{ApiHeader, HeaderValue, LegacyQueryParam, MIN_SECRET, api_error_detail, valid_api_access};
use crate::api_http::{ApiReply, send};
use crate::error::{
    ApiClientSnafu, ApiConfigSnafu, ApiHttpSnafu, ApiKeyInvalidSnafu, ApiKeyMissingSnafu, ApiResponseSnafu,
    ApiRuntimeSnafu, ApiStatusSnafu, WavSnafu,
};
use crate::phrase_engine::PhraseEngine;
use crate::{SAMPLE_RATE, SttError, Transcript};

/// Settings of the transcription API: address, model, key variable, Azure version, headers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    /// Root address of the API: `…/v1` for OpenAI, the resource address for Azure.
    pub url: String,
    /// Requested transcription model; the deployment name for Azure.
    pub model: String,
    /// Name of the environment variable that holds the key.
    pub key_env: String,
    /// Azure OpenAI `api-version`; empty: the OpenAI API.
    pub api_version: String,
    /// Extra headers sent with every request, literal or read from a variable.
    pub headers: Vec<ApiHeader>,
    /// Query parameters saved by 7.0.0 or earlier, kept only to refuse the settings: never migrated.
    #[serde(rename = "query", skip_serializing_if = "Vec::is_empty")]
    pub legacy_query: Vec<LegacyQueryParam>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            url: "https://api.openai.com/v1".into(),
            model: "gpt-4o-transcribe".into(),
            key_env: "OPENAI_API_KEY".into(),
            api_version: String::new(),
            headers: Vec::new(),
            legacy_query: Vec::new(),
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

    /// Azure OpenAI rather than OpenAI: an `api-version` is set.
    pub(crate) fn is_azure(&self) -> bool {
        !self.api_version.is_empty()
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

/// A transcription API client, ready to serve (secrets already read).
pub(crate) struct ApiTranscriber {
    config: ApiConfig,
    client: Client<Box<dyn Config>>,
    /// Values read from variables, and literals of 8 characters or more, masked in API messages.
    secrets: Vec<String>,
    /// Runs the async client on the dictation thread.
    runtime: tokio::runtime::Runtime,
}

impl fmt::Debug for ApiTranscriber {
    // Neither the key nor any header appears.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApiTranscriber").field("config", &self.config).finish_non_exhaustive()
    }
}

impl ApiTranscriber {
    /// Refuses invalid settings, reads the secrets and prepares the client.
    ///
    /// The key goes as `Authorization: Bearer` (OpenAI) or `api-key` (Azure), the extra headers
    /// as the HTTP client's defaults. No redirects and no retry; no proxy to this machine, the
    /// environment's proxy otherwise; system certificates (t-1211).
    pub(crate) fn new(config: ApiConfig, timeout: Duration) -> Result<Self, SttError> {
        let root = match normalized_url(&config.url) {
            Some(root) if valid_api_access(&config) && valid_api_model(&config.model) => root,
            _ => return Err(ApiConfigSnafu.build().into()),
        };
        let key = ApiConfig::secret(&config.key_env)?;
        // async-openai panics on a key that is not a header value: refuse it here.
        reqwest::header::HeaderValue::from_str(&key)
            .ok()
            .context(ApiKeyInvalidSnafu { var: config.key_env.as_str() })?;
        let mut secrets = vec![key.clone()];
        let mut headers = HeaderMap::with_capacity(config.headers.len());
        for header in &config.headers {
            let value = match &header.value {
                HeaderValue::Literal(value) => value.clone(),
                HeaderValue::Env(var) => ApiConfig::secret(var)?,
            };
            let name = HeaderName::from_bytes(header.name.as_bytes()).ok().context(ApiConfigSnafu)?;
            let Ok(mut sent) = reqwest::header::HeaderValue::from_str(&value) else {
                return Err(match &header.value {
                    HeaderValue::Env(var) => ApiKeyInvalidSnafu { var: var.as_str() }.build().into(),
                    HeaderValue::Literal(_) => ApiConfigSnafu.build().into(),
                });
            };
            // A variable's value, or a literal long enough to be a pasted key: masked in API
            // messages and marked sensitive so no HTTP debug log shows it.
            if matches!(header.value, HeaderValue::Env(_)) || value.chars().count() >= MIN_SECRET {
                sent.set_sensitive(true);
                secrets.push(value);
            }
            headers.insert(name, sent);
        }
        let mut http = reqwest::Client::builder()
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(timeout);
        if is_loopback(&root) {
            http = http.no_proxy();
        }
        let http = http.build().context(ApiHttpSnafu)?;
        // An explicit key, base and empty ids: no OPENAI_* variable reaches another host.
        let api: Box<dyn Config> = if config.is_azure() {
            Box::new(
                AzureConfig::new()
                    .with_api_base(root)
                    .with_api_key(key)
                    .with_deployment_id(config.model.as_str())
                    .with_api_version(config.api_version.as_str()),
            )
        } else {
            Box::new(OpenAIConfig::new().with_api_base(root).with_api_key(key).with_org_id("").with_project_id(""))
        };
        // `build` and not `with_config`, which first makes a throwaway client that can panic.
        let client = Client::build(http.clone(), api)
            .with_http_service(tower::service_fn(move |request| send(http.clone(), request)));
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().context(ApiRuntimeSnafu)?;
        Ok(Self { config, client, secrets, runtime })
    }

    /// The localized error for a failed call; no API message on 401/403.
    fn error(&self, error: OpenAIError) -> SttError {
        match error {
            OpenAIError::ApiError(reply) => {
                let status = reply.status_code.as_u16();
                // No message on 401/403: their body may quote a fragment of a secret.
                let detail = if matches!(status, 401 | 403) {
                    None
                } else {
                    let secrets: Vec<&str> = self.secrets.iter().map(String::as_str).collect();
                    api_error_detail(&reply.api_error.message, &secrets)
                };
                tracing::warn!(status, detail = detail.as_deref().unwrap_or_default(), "API de dictée");
                ApiStatusSnafu { status, detail }.build().into()
            }
            OpenAIError::Reqwest(source) => ApiHttpSnafu.into_error(source).into(),
            // The body it carries may echo a secret: only the parse error is kept.
            OpenAIError::JSONDeserialize(source, _) => ApiResponseSnafu.into_error(source).into(),
            other => ApiClientSnafu.into_error(Box::new(other)).into(),
        }
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
        let request = CreateTranscriptionRequest {
            file: AudioInput { source: InputSource::VecU8 { filename: "phrase.wav".into(), vec: wav(audio)? } },
            model: self.config.model.clone(),
            language: language.map(str::to_owned),
            prompt: context.map(str::trim).filter(|c| !c.is_empty()).map(str::to_owned),
            response_format: Some(AudioResponseFormat::Json),
            ..CreateTranscriptionRequest::default()
        };
        let reply: ApiReply = self
            .runtime
            .block_on(self.client.audio().transcription().create_byot(request))
            .map_err(|e| self.error(e))?;
        Ok(Transcript { text: reply.text, language: None, no_speech: 0.0 })
    }
}

#[cfg(test)]
#[path = "api_tests.rs"]
mod api_tests;
