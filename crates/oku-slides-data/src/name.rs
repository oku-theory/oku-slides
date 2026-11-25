//! Data names: what queries reference, unique in a deck case-insensitively.

use oku_slides_i18n::fold;

use crate::limits::MAX_NAME_CHARS;

/// Characters a default name is cut to, so a `_NN` suffix keeps it within `MAX_NAME_CHARS`.
const DEFAULT_NAME_CHARS: usize = 60;

/// A valid data name: `[a-z_][a-z0-9_]*`, 1 to `MAX_NAME_CHARS` characters.
#[must_use]
pub fn valid_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    name.len() <= MAX_NAME_CHARS
        && bytes.next().is_some_and(|b| b.is_ascii_lowercase() || b == b'_')
        && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// A valid name made from `text`, not `taken`.
///
/// `text` folded (lowercase, accents dropped), each run of other characters than `[a-z0-9_]`
/// turned into `_`, trimmed of `_`, prefixed with `_` before a digit, `data` if nothing is left,
/// cut to 60 characters; then `_first`, `_first + 1`… (from `_2` at least) until `taken`
/// refuses it. Returns the name and its suffix, 1 for none: a caller that knows the suffixes
/// below `first` are taken skips them. `taken` compares case-insensitively, as names are
/// unique in a deck.
#[must_use]
pub fn default_name(text: &str, first: usize, taken: impl Fn(&str) -> bool) -> (String, usize) {
    let mut base = String::new();
    for c in fold(text).chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' {
            base.push(c);
        } else if !base.ends_with('_') {
            base.push('_');
        }
    }
    let trimmed = base.trim_matches('_');
    let mut base = match trimmed.chars().next() {
        None => "data".to_owned(),
        Some(c) if c.is_ascii_digit() => format!("_{trimmed}"),
        Some(_) => trimmed.to_owned(),
    };
    base.truncate(DEFAULT_NAME_CHARS);
    if !taken(&base) {
        return (base, 1);
    }
    let mut n = first.max(2);
    loop {
        let name = format!("{base}_{n}");
        if !taken(&name) {
            return (name, n);
        }
        n += 1;
    }
}

#[cfg(test)]
#[path = "name_tests.rs"]
mod name_tests;
