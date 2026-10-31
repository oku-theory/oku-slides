//! Bidi controls and invisible characters, removed from interpolated values.
//!
//! The bundles run without isolation marks (`set_use_isolating(false)`): a value that carries an
//! override (U+202E) could reorder the sentence around it. `ui/src/i18n.ts` holds the same set.

/// Whether `c` is a bidi control or an invisible character a label must not carry.
///
/// Bidi marks, embeddings, overrides and isolates, U+061C, the zero-width space, the word
/// joiners, U+FEFF, U+180E, the soft hyphen and the tag characters (U+E0000..U+E007F, text a
/// model reads but nobody sees). U+200C (ZWNJ) and U+200D (ZWJ) are kept: Indic scripts and
/// emoji sequences need them.
#[must_use]
pub fn is_hidden_control(c: char) -> bool {
    matches!(
        c,
        '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
            | '\u{200B}' | '\u{2060}'..='\u{2064}' | '\u{FEFF}' | '\u{180E}' | '\u{00AD}'
            | '\u{E0000}'..='\u{E007F}'
    )
}

/// `s` without the characters [`is_hidden_control`] matches.
#[must_use]
pub fn strip_hidden(s: &str) -> String {
    s.chars().filter(|c| !is_hidden_control(*c)).collect()
}

#[cfg(test)]
#[path = "hidden_tests.rs"]
mod hidden_tests;
