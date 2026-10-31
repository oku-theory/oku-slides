//! Raccourcis de l'agent : un mot seul ouvre une surface de l'hôte.
//!
//! Aucun tour, aucun jeton dépensé. Les mots d'un raccourci sont une clé Fluent de
//! `ui.ftl` (lue aussi par l'interface, `ui/src/agent/shortcut.ts`, même règle),
//! séparés par des virgules ; chaque langue y met aussi les mots anglais. La
//! consigne entière doit être l'un d'eux, une fois normalisée, dans la langue de
//! l'interface ou dans celle de la dictée (choisie, sinon détectée).

use oku_slides_i18n::{Localizer, THEME_SHORTCUT, fold, theme_shortcuts};
use unicode_normalization::char::is_combining_mark;

/// Au-delà, une consigne n'est jamais un raccourci (et n'est pas normalisée).
const SHORTCUT_BYTES: usize = 64;

/// `text` folded (`oku_slides_i18n::fold`), spaces collapsed, edges trimmed.
pub(crate) fn normalize(text: &str) -> String {
    let edges = |c: char| !(c.is_alphanumeric() || is_combining_mark(c));
    fold(text).split_whitespace().collect::<Vec<_>>().join(" ").trim_matches(edges).to_owned()
}

/// True if the normalized `said` is one of the comma-separated `words`.
fn is_one_of(said: &str, words: &str) -> bool {
    !said.is_empty() && words.split(',').any(|word| normalize(word) == said)
}

/// The shipped locale of the dictation: its chosen language, else the one it detected.
///
/// A chosen language that ships no messages has no fallback, as for the dictation itself.
pub(crate) fn dictation_locale(chosen: Option<&str>, detected: Option<&str>) -> Option<&'static str> {
    match chosen.filter(|c| !c.is_empty()) {
        Some(chosen) => oku_slides_i18n::shipped(chosen),
        None => detected.and_then(oku_slides_i18n::shipped),
    }
}

/// True if `text` is the theme shortcut in the interface language or the dictation's.
///
/// `dictation` (the dictation's locale) is asked only when the interface's words miss.
pub(crate) fn is_spoken_theme_shortcut(
    text: &str,
    ui: &Localizer,
    dictation: impl FnOnce() -> Option<&'static str>,
) -> bool {
    if text.len() > SHORTCUT_BYTES {
        return false;
    }
    let said = normalize(text);
    is_one_of(&said, &ui.tr(THEME_SHORTCUT))
        || dictation()
            .filter(|d| *d != ui.locale())
            .and_then(|d| theme_shortcuts().get(d))
            .is_some_and(|words| is_one_of(&said, words))
}

#[cfg(test)]
#[path = "shortcut_tests.rs"]
mod shortcut_tests;
