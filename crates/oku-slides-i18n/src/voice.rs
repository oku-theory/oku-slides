//! Spoken command words per locale, read from `voice.ftl`.
//!
//! The words are comma-separated forms in plain messages, folded here once so the
//! host can match dictated text without caring for case or accents.

use std::sync::{LazyLock, OnceLock};

use crate::{LOCALES, Localizer, fold, shipped};

/// One lazily built localizer per shipped locale, with no English fallback.
static VOICE: LazyLock<Vec<(&'static str, OnceLock<Localizer>)>> =
    LazyLock::new(|| LOCALES.iter().map(|(locale, _)| (*locale, OnceLock::new())).collect());

/// The folded spoken forms of message `key` in `locale`, or none if either is unknown.
///
/// `locale` may be a regional tag (`fr-CA`). A locale never borrows English forms:
/// a missing key gives no form, so one language's words never leak into another's.
#[must_use]
pub fn voice_forms(locale: &str, key: &str) -> Vec<String> {
    let Some(code) = shipped(locale) else { return Vec::new() };
    let Some((_, cell)) = VOICE.iter().find(|(l, _)| *l == code) else { return Vec::new() };
    let own = cell.get_or_init(|| Localizer::build(code, None));
    if !own.has(key) {
        return Vec::new();
    }
    own.tr(key).split(',').map(|form| fold(form.trim())).filter(|form| !form.is_empty()).collect()
}

#[cfg(test)]
#[path = "voice_tests.rs"]
mod voice_tests;
