//! The dictation API's HTTP transport: one attempt, a bounded reply, the status kept (t-1233).
//!
//! async-openai's default transport retries 429 and 5xx up to three times and honours any
//! `Retry-After`, which could hold dictation for an hour and upload the audio again. It also
//! loses the status of a non-OpenAI error body and logs raw bodies it cannot parse. This
//! transport replaces it through the `middleware` feature.

use async_openai::error::{ApiError, ApiErrorResponse, OpenAIError, WrappedError};
use async_openai::middleware::HttpRequestFactory;
use reqwest::{Response, StatusCode};
use serde::Deserialize;

/// Largest reply read, in bytes: a sentence's transcription is a few hundred bytes of JSON.
pub(crate) const MAX_REPLY: usize = 1 << 20;

/// The API's answer: only the text, so a service that sends no `usage` still works.
#[derive(Deserialize)]
pub(crate) struct ApiReply {
    pub(crate) text: String,
}

/// Sends one request with `client`, never retried.
///
/// A failed status becomes `ApiError` with that status whatever the body; only an
/// OpenAI-shaped error message is kept, never a raw page. A success must be a `{ "text": … }`
/// object of at most [`MAX_REPLY`] bytes, checked here so async-openai never logs a body.
pub(crate) async fn send(client: reqwest::Client, factory: HttpRequestFactory) -> Result<Response, OpenAIError> {
    let request = factory.build().await?;
    let mut response = client.execute(request).await.map_err(OpenAIError::Reqwest)?;
    let status = response.status();
    let body = bounded_body(&mut response).await?;
    if !status.is_success() {
        let message = body
            .as_deref()
            .and_then(|b| serde_json::from_slice::<WrappedError>(b).ok())
            .map(|wrapped| wrapped.error.message)
            .unwrap_or_default();
        return Err(status_error(status, message));
    }
    let body = body.ok_or_else(|| invalid_reply("reply larger than MAX_REPLY"))?;
    // The text it carries is not kept: the parse error alone names the problem.
    serde_json::from_slice::<ApiReply>(&body).map_err(|e| OpenAIError::JSONDeserialize(e, String::new()))?;
    let mut rebuilt = http::Response::new(body);
    *rebuilt.status_mut() = status;
    *rebuilt.headers_mut() = response.headers().clone();
    Ok(Response::from(rebuilt))
}

/// The body, or `None` once it passes [`MAX_REPLY`] bytes (the rest is not read).
async fn bounded_body(response: &mut Response) -> Result<Option<Vec<u8>>, OpenAIError> {
    if response.content_length().is_some_and(|n| n > MAX_REPLY as u64) {
        return Ok(None);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(OpenAIError::Reqwest)? {
        if body.len() + chunk.len() > MAX_REPLY {
            return Ok(None);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(Some(body))
}

/// An `ApiError` carrying `status` and the API's own `message` (empty when it gave none).
fn status_error(status: StatusCode, message: String) -> OpenAIError {
    OpenAIError::ApiError(ApiErrorResponse {
        status_code: status,
        api_error: ApiError { message, r#type: None, param: None, code: None, misalignment: None },
    })
}

/// A reply that cannot be a transcription, reported as a parse error.
fn invalid_reply(reason: &str) -> OpenAIError {
    OpenAIError::JSONDeserialize(<serde_json::Error as serde::de::Error>::custom(reason), String::new())
}

#[cfg(test)]
#[path = "api_http_tests.rs"]
mod api_http_tests;
