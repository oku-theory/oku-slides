//! Raccourcis de l'agent : un mot seul ouvre une surface de l'hôte.
//!
//! Aucun tour, aucun jeton dépensé. Les mots d'un raccourci sont une clé Fluent de
//! `ui.ftl` (lue aussi par l'interface, `ui/src/agent/shortcut.ts`, même règle),
//! séparés par des virgules ; chaque langue y met aussi les mots anglais. La
//! consigne entière doit être l'un d'eux, une fois normalisée.

use oku_slides_i18n::Localizer;
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

/// Clé Fluent des mots qui ouvrent le sélecteur de thème.
pub(crate) const THEME_SHORTCUT: &str = "ui-shortcut-theme";

/// Au-delà, une consigne n'est jamais un raccourci (et n'est pas normalisée).
const SHORTCUT_BYTES: usize = 64;

/// Vrai pour un diacritique latin, grec ou cyrillique.
fn is_accent(c: char) -> bool {
    ('\u{0300}'..='\u{036F}').contains(&c)
}

/// `text` en minuscules, sans accents, espaces resserrés, bords nettoyés.
pub(crate) fn normalize(text: &str) -> String {
    let folded: String =
        text.to_lowercase().nfd().filter(|c| !is_accent(*c)).map(|c| if c == '’' { '\'' } else { c }).collect();
    let edges = |c: char| !(c.is_alphanumeric() || is_combining_mark(c));
    folded.split_whitespace().collect::<Vec<_>>().join(" ").trim_matches(edges).nfc().collect()
}

/// Vrai si `text` entier est l'un des mots de `key`.
pub(crate) fn matches_shortcut(text: &str, key: &str, l: &Localizer) -> bool {
    if text.len() > SHORTCUT_BYTES {
        return false;
    }
    let said = normalize(text);
    !said.is_empty() && l.tr(key).split(',').any(|word| normalize(word) == said)
}

#[cfg(test)]
#[path = "shortcut_tests.rs"]
mod shortcut_tests;
