//! Secret values the process knows, masked in every captured log or stderr line.
//!
//! Agents inherit the app's environment and may print it; the dictation key and the tabs'
//! MCP tokens are registered here once known, and [`redact_secrets`] masks them before
//! diagnosis shows any text.

use std::collections::BTreeSet;
use std::sync::{Mutex, PoisonError};

/// What a registered secret becomes in captured text.
pub const REDACTED: &str = "<redacted>";

/// Characters a value needs to be registered: shorter ones would mask ordinary words.
pub const SECRET_MIN_CHARS: usize = 8;

/// The registered secrets of this process.
static SECRETS: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

/// Registers `secret` (trimmed) so captured text never shows it; values under
/// [`SECRET_MIN_CHARS`] characters are ignored.
pub fn register_secret(secret: &str) {
    let secret = secret.trim();
    if secret.chars().count() >= SECRET_MIN_CHARS {
        SECRETS.lock().unwrap_or_else(PoisonError::into_inner).insert(secret.to_owned());
    }
}

/// `text` with every registered secret replaced by [`REDACTED`], the longest first.
#[must_use]
pub fn redact_secrets(text: &str) -> String {
    let secrets = SECRETS.lock().unwrap_or_else(PoisonError::into_inner);
    let mut ordered: Vec<&String> = secrets.iter().filter(|s| text.contains(s.as_str())).collect();
    ordered.sort_by_key(|s| std::cmp::Reverse(s.len()));
    ordered.into_iter().fold(text.to_owned(), |out, secret| out.replace(secret.as_str(), REDACTED))
}

#[cfg(test)]
#[path = "log_secrets_tests.rs"]
mod log_secrets_tests;
