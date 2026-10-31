//! Case and accent folding, to compare what people type or say with a text.
//!
//! One fold for the agent's shortcuts, the help search and pack namespaces:
//! lowercase, Latin, Greek and Cyrillic diacritics dropped, `œ`, `æ`, `ø` and `ß`
//! spelled out, the curly apostrophe made straight. Other scripts keep their
//! signs (Indic and Thai vowels, Hangul), recomposed as they came.

use unicode_normalization::UnicodeNormalization;

/// True for a combining diacritic of Latin, Greek or Cyrillic (U+0300 to U+036F).
fn is_accent(c: char) -> bool {
    ('\u{0300}'..='\u{036F}').contains(&c)
}

/// The letters a ligature or a letter with no decomposition stand for.
fn spelled(c: char) -> Option<&'static str> {
    Some(match c {
        'œ' => "oe",
        'æ' => "ae",
        'ø' => "o",
        'ß' => "ss",
        '’' => "'",
        _ => return None,
    })
}

/// `text` lowercased, without Latin, Greek or Cyrillic accents, ligatures spelled out.
///
/// Whitespace and punctuation stay: callers trim or split as they need.
#[must_use]
pub fn fold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.to_lowercase().nfd().filter(|c| !is_accent(*c)) {
        match spelled(c) {
            Some(s) => out.push_str(s),
            None => out.push(c),
        }
    }
    out.nfc().collect()
}

#[cfg(test)]
#[path = "fold_tests.rs"]
mod fold_tests;
