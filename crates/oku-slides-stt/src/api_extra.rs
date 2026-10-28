//! Custom headers of the dictation API, and its redacted error messages (t-1213).
//!
//! Gateways need their own headers, set by the user only: a header may carry the secret of an
//! environment variable. The settings box edits them in one line: `Name: value; Name: $VARIABLE`.

use serde::{Deserialize, Serialize};

use crate::ApiConfig;
use crate::api_check::{valid_api_version, valid_key_env};

/// Most headers one API setting may hold.
const MAX_ENTRIES: usize = 8;
/// Longest header name, in bytes.
const MAX_NAME: usize = 64;
/// Longest header value, in bytes.
const MAX_VALUE: usize = 256;
/// Longest API error message shown, in characters.
const MAX_DETAIL: usize = 200;
/// Shortest secret whose echo can be masked without guessing; below, no message is shown.
pub(crate) const MIN_SECRET: usize = 8;
/// Headers the client sets itself (the key included) or that change the framing or proxying.
const RESERVED: [&str; 13] = [
    "authorization",
    "api-key",
    "host",
    "content-type",
    "content-length",
    "transfer-encoding",
    "connection",
    "proxy-authorization",
    "te",
    "trailer",
    "upgrade",
    "expect",
    "keep-alive",
];

/// Where a custom header takes its value from.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HeaderValue {
    /// A value stored and shown in clear.
    Literal(String),
    /// The name of an environment variable read when dictation starts, never stored.
    Env(String),
}

/// A query parameter saved by 7.0.0 or earlier, read only so such settings are refused (t-1233).
///
/// The Azure `api-version` and other parameters are not migrated: the user enters the
/// settings again, and saving any API field drops these.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LegacyQueryParam {
    /// Parameter name, such as `api-version`.
    pub name: String,
    /// Its value, stored in clear.
    pub value: String,
}

/// Why API settings cannot be used together, from [`api_access_problem`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiAccessProblem {
    /// The key variable is missing or not a variable name.
    KeyEnv,
    /// The Azure version is malformed, or query parameters of 7.0.0 are still saved.
    Version,
    /// A header is malformed, reserved or repeated, or there are too many.
    Headers,
    /// The Azure deployment (the model) is not a single path segment.
    Deployment,
}

/// A header sent with every transcription request.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ApiHeader {
    /// Header name, such as `X-Tenant`.
    pub name: String,
    /// Its value, or the variable holding it.
    pub value: HeaderValue,
}

impl ApiConfig {
    /// Every environment variable these settings read: the key's, then the headers'.
    #[must_use]
    pub fn variables(&self) -> Vec<&str> {
        let headers = self.headers.iter().filter_map(|h| match &h.value {
            HeaderValue::Env(var) => Some(var.as_str()),
            HeaderValue::Literal(_) => None,
        });
        std::iter::once(self.key_env.as_str()).filter(|k| !k.is_empty()).chain(headers).collect()
    }
}

/// Parses `Name: value; Name: $VARIABLE`; blank gives no header, a malformed one `None`.
///
/// Empty segments are skipped. Names are HTTP tokens; a literal is printable ASCII and
/// cannot hold `;`. Reserved names and duplicates are checked by [`valid_api_access`].
#[must_use]
pub fn parse_api_headers(text: &str) -> Option<Vec<ApiHeader>> {
    let headers = text
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|segment| {
            let (name, value) = segment.split_once(':')?;
            let (name, value) = (name.trim(), value.trim());
            let value = match value.strip_prefix('$') {
                Some(var) => valid_key_env(var).then(|| HeaderValue::Env(var.into()))?,
                None => valid_literal(value).then(|| HeaderValue::Literal(value.into()))?,
            };
            valid_header_name(name).then(|| ApiHeader { name: name.into(), value })
        })
        .collect::<Option<Vec<_>>>()?;
    (headers.len() <= MAX_ENTRIES).then_some(headers)
}

/// The one-line form of `headers`, read back by [`parse_api_headers`].
#[must_use]
pub fn format_api_headers(headers: &[ApiHeader]) -> String {
    let one = |h: &ApiHeader| match &h.value {
        HeaderValue::Literal(v) => format!("{}: {v}", h.name),
        HeaderValue::Env(var) => format!("{}: ${var}", h.name),
    };
    headers.iter().map(one).collect::<Vec<_>>().join("; ")
}

/// No reserved and no duplicate header name: checked on every write.
#[must_use]
pub fn valid_api_headers(headers: &[ApiHeader]) -> bool {
    let mut names: Vec<String> = headers.iter().map(|h| h.name.to_ascii_lowercase()).collect();
    let reserved = names.iter().any(|n| RESERVED.contains(&n.as_str()));
    names.sort_unstable();
    let duplicate = names.windows(2).any(|w| w.first() == w.last());
    !reserved && !duplicate
}

/// The key variable, Azure version, model and headers can be used together.
#[must_use]
pub fn valid_api_access(config: &ApiConfig) -> bool {
    api_access_problem(config).is_none()
}

/// The first reason these settings cannot be used, or `None`.
///
/// The key variable is required, and no query parameter of 7.0.0 may remain. For Azure the model
/// is the deployment name, a path segment: letters, digits, `.`, `_` and `-`, not only dots.
#[must_use]
pub fn api_access_problem(config: &ApiConfig) -> Option<ApiAccessProblem> {
    let headers_ok = config.headers.len() <= MAX_ENTRIES
        && parse_api_headers(&format_api_headers(&config.headers)).is_some_and(|h| h == config.headers)
        && valid_api_headers(&config.headers);
    let model = config.model.as_bytes();
    let deployment_ok = !config.is_azure()
        || (model.iter().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
            && !model.iter().all(|&b| b == b'.'));
    if !valid_key_env(&config.key_env) {
        Some(ApiAccessProblem::KeyEnv)
    } else if !valid_api_version(&config.api_version) || !config.legacy_query.is_empty() {
        Some(ApiAccessProblem::Version)
    } else if !headers_ok {
        Some(ApiAccessProblem::Headers)
    } else if !deployment_ok {
        Some(ApiAccessProblem::Deployment)
    } else {
        None
    }
}

/// The API's own error `message`, cleaned, cut and with `secrets` masked.
///
/// Each secret and each of its words of 8 characters or more is masked; if a secret is shorter
/// than that, or 8 characters of one are still in the text (a partial echo), `None`.
#[must_use]
pub fn api_error_detail(message: &str, secrets: &[&str]) -> Option<String> {
    let clean: String = message.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut text: String = clean.chars().filter(|c| !c.is_control()).collect();
    let secrets: Vec<Vec<char>> = secrets.iter().map(|s| s.chars().collect()).collect();
    if secrets.iter().any(|s| s.len() < MIN_SECRET) {
        return None;
    }
    for secret in &secrets {
        let whole: String = secret.iter().collect();
        text = text.replace(&whole, "***");
        for word in whole.split_whitespace().filter(|w| w.chars().count() >= MIN_SECRET) {
            text = text.replace(word, "***");
        }
    }
    let echoed = secrets.iter().any(|s| s.windows(MIN_SECRET).any(|w| text.contains(&w.iter().collect::<String>())));
    if echoed || text.is_empty() {
        return None;
    }
    Some(match text.char_indices().nth(MAX_DETAIL) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text,
    })
}

fn valid_header_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_NAME
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
}

fn valid_literal(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_VALUE && value.bytes().all(|b| (0x20..0x7f).contains(&b) && b != b';')
}

#[cfg(test)]
#[path = "api_extra_tests.rs"]
mod api_extra_tests;
