//! Bidi controls and invisible characters removed, line breaks flattened, in interpolated values.
//!
//! The bundles run without isolation marks (`set_use_isolating(false)`): a value that carries an
//! override (U+202E) could reorder the sentence around it, and a line break (or ESC) would inject
//! lines into a dialog or a terminal. `ui/src/i18n.ts` holds the same sets.

/// Whether `c` is a bidi control or an invisible character a label must not carry.
///
/// Bidi marks, embeddings, overrides and isolates, U+061C, the zero-width space, the word
/// joiners, U+FEFF, U+180E, the soft hyphen, the Hangul fillers U+3164 and U+FFA0 (blank
/// letters, t-1383) and the tag characters (U+E0000..U+E007F, text a model reads but nobody
/// sees). U+200C (ZWNJ) and U+200D (ZWJ) are kept: Indic scripts and emoji sequences need them;
/// so are the variation selectors (U+FE0F asks for an emoji's colour form) and the conjoining
/// fillers U+115F and U+1160, which old Hangul syllables use.
#[must_use]
pub fn is_hidden_control(c: char) -> bool {
    matches!(
        c,
        '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
            | '\u{200B}' | '\u{2060}'..='\u{2064}' | '\u{FEFF}' | '\u{180E}' | '\u{00AD}'
            | '\u{3164}' | '\u{FFA0}' | '\u{E0000}'..='\u{E007F}'
    )
}

/// Whether `c` breaks a line or drives a terminal.
///
/// A `Cc` control, U+2028 or U+2029.
///
/// `Cc` covers U+0000..U+001F and U+007F..U+009F: LF, CR, TAB, ESC, U+0085 (NEL).
#[must_use]
pub fn is_line_control(c: char) -> bool {
    c.is_control() || matches!(c, '\u{2028}' | '\u{2029}')
}

/// `s` made safe to interpolate: hidden controls removed, line controls turned into spaces.
#[must_use]
pub fn clean_arg(s: &str) -> String {
    s.chars().filter(|c| !is_hidden_control(*c)).map(|c| if is_line_control(c) { ' ' } else { c }).collect()
}

#[cfg(test)]
#[path = "hidden_tests.rs"]
mod hidden_tests;
