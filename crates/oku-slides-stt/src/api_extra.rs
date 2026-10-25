//! Query parameters and custom headers of the dictation API (t-1213).
//!
//! Azure OpenAI and gateways need `?api-version=…` and their own headers. Both are set by
//! the user only: a header may carry the secret of an environment variable. The settings box
//! edits them in a one-line syntax: `a=b&c=d` and `Name: value; Name: $VARIABLE`.

use serde::{Deserialize, Serialize};

use crate::ApiConfig;
use crate::api_check::valid_key_env;

/// Most query parameters, and most headers, one API setting may hold.
const MAX_ENTRIES: usize = 8;
/// Longest name of a parameter or header, in bytes.
const MAX_NAME: usize = 64;
/// Longest value of a parameter or header, in bytes.
const MAX_VALUE: usize = 256;
/// Longest API error message shown, in characters.
const MAX_DETAIL: usize = 200;
/// Shortest secret whose echo can be masked without guessing; below, no message is shown.
const MIN_SECRET: usize = 8;
/// Headers the client sets itself or that change the framing or proxying: never user-set.
const RESERVED: [&str; 11] = [
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

/// A query parameter appended to the transcription address, stored in clear.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ApiParam {
    /// Parameter name, such as `api-version`.
    pub name: String,
    /// Parameter value, URL-encoded when sent.
    pub value: String,
}

/// Where a custom header takes its value from.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HeaderValue {
    /// A value stored and shown in clear.
    Literal(String),
    /// The name of an environment variable read when dictation starts, never stored.
    Env(String),
}

/// A header sent with every transcription request.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ApiHeader {
    /// Header name, such as `api-key`.
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

/// Parses `a=b&c=d`; blank gives no parameter, a malformed pair `None`.
///
/// Names and values are visible ASCII without `&`, `#` or `%`: they are encoded when sent.
/// Every pair needs `=`, so an empty pair (a trailing or doubled `&`) is refused.
#[must_use]
pub fn parse_api_query(text: &str) -> Option<Vec<ApiParam>> {
    if text.trim().is_empty() {
        return Some(Vec::new());
    }
    let params = text
        .split('&')
        .map(|pair| {
            let (name, value) = pair.trim().split_once('=')?;
            let plain = |s: &str| s.bytes().all(|b| b.is_ascii_graphic() && !matches!(b, b'&' | b'#' | b'%'));
            let ok =
                !name.is_empty() && name.len() <= MAX_NAME && value.len() <= MAX_VALUE && plain(name) && plain(value);
            ok.then(|| ApiParam { name: name.into(), value: value.into() })
        })
        .collect::<Option<Vec<_>>>()?;
    (params.len() <= MAX_ENTRIES).then_some(params)
}

/// The one-line form of `query`, read back by [`parse_api_query`].
#[must_use]
pub fn format_api_query(query: &[ApiParam]) -> String {
    query.iter().map(|p| format!("{}={}", p.name, p.value)).collect::<Vec<_>>().join("&")
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
///
/// `Authorization` is allowed here; whether it clashes with the key variable depends on the
/// other fields, so [`valid_api_access`] checks it at use.
#[must_use]
pub fn valid_api_headers(headers: &[ApiHeader]) -> bool {
    let mut names: Vec<String> = headers.iter().map(|h| h.name.to_ascii_lowercase()).collect();
    let reserved = names.iter().any(|n| RESERVED.contains(&n.as_str()));
    names.sort_unstable();
    let duplicate = names.windows(2).any(|w| w.first() == w.last());
    !reserved && !duplicate
}

/// The key variable, parameters and headers can be used together.
///
/// The key variable may be empty (no Bearer) only if a header names a variable: some secret
/// is always required. `Authorization` is user-set only without a key variable.
#[must_use]
pub fn valid_api_access(config: &ApiConfig) -> bool {
    let key_ok = config.key_env.is_empty() || valid_key_env(&config.key_env);
    let query_ok = config.query.len() <= MAX_ENTRIES
        && parse_api_query(&format_api_query(&config.query)).is_some_and(|q| q == config.query);
    let headers_ok = config.headers.len() <= MAX_ENTRIES
        && parse_api_headers(&format_api_headers(&config.headers)).is_some_and(|h| h == config.headers)
        && valid_api_headers(&config.headers);
    let bearer_clash =
        !config.key_env.is_empty() && config.headers.iter().any(|h| h.name.eq_ignore_ascii_case("authorization"));
    key_ok && query_ok && headers_ok && !bearer_clash && !config.variables().is_empty()
}

/// The API's own error message in `body`, cleaned, cut and with `secrets` masked.
///
/// Reads `error.message` or a string `error` (OpenAI, Azure); `None` without one. Each secret
/// and each of its words of 8 characters or more is masked; if a secret is shorter than that,
/// or 8 characters of one are still in the text (a partial echo), no message is shown.
#[must_use]
pub fn api_error_detail(body: &str, secrets: &[&str]) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(body).ok()?;
    let error = json.get("error")?;
    let message = error.get("message").unwrap_or(error).as_str()?;
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
