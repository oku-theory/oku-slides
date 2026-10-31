//! The words of the agent's shortcuts, in every shipped language.
//!
//! The host recognizes a shortcut in the interface language or the dictation's;
//! the interface gets every language's words with its messages to do the same.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use crate::{Localizer, available};

/// Fluent key of the words that open the theme picker, comma-separated.
pub const THEME_SHORTCUT: &str = "ui-shortcut-theme";

/// The theme shortcut's words, by shipped locale (built once).
static THEME_WORDS: LazyLock<BTreeMap<&'static str, String>> =
    LazyLock::new(|| available().into_iter().map(|l| (l, Localizer::new(Some(l)).tr(THEME_SHORTCUT))).collect());

/// The theme shortcut's comma-separated words, by shipped locale.
#[must_use]
pub fn theme_shortcuts() -> &'static BTreeMap<&'static str, String> {
    &THEME_WORDS
}

#[cfg(test)]
#[path = "shortcut_tests.rs"]
mod shortcut_tests;
