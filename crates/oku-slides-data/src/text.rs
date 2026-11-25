//! Untrusted text cleaned before it is stored or shown: labels and column names.

use oku_slides_i18n::{is_hidden_control, is_line_control};

use crate::limits::MAX_LABEL_CHARS;

/// `text` without bidi controls, invisible characters and line controls; ZWJ and ZWNJ stay.
#[must_use]
pub fn strip_controls(text: &str) -> String {
    text.chars().filter(|c| !is_hidden_control(*c) && !is_line_control(*c)).collect()
}

/// A label or column name: `text` without controls, cut to `MAX_LABEL_CHARS` characters.
#[must_use]
pub fn clean_label(text: &str) -> String {
    strip_controls(text).chars().take(MAX_LABEL_CHARS).collect()
}

/// `text` cut to `max` characters.
pub(crate) fn cut_chars(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((end, _)) => text[..end].to_owned(),
        None => text.to_owned(),
    }
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod text_tests;
