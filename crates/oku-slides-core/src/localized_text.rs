//! Localized text: a widget's label or description, plain or by language.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A shown text: one for every language, or one per language code.
///
/// Typst writes it as `"KPI"` or `(en: "Value", fr: "Valeur")`; [`LocalizedText::pick`] chooses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LocalizedText {
    /// The same text in every language.
    Plain(String),
    /// A text per language code (`en`, `fr`, `zh-Hant`…), sorted by code, not as written.
    ByLocale(BTreeMap<String, String>),
}

impl LocalizedText {
    /// The text for `locale`: exact, language without region, `en`, then the lowest code.
    ///
    /// Codes compare without case. `None` only for an empty dictionary.
    #[must_use]
    pub fn pick(&self, locale: &str) -> Option<&str> {
        let by = match self {
            Self::Plain(text) => return Some(text),
            Self::ByLocale(by) => by,
        };
        let find = |code: &str| by.iter().find(|(k, _)| k.eq_ignore_ascii_case(code)).map(|(_, v)| v.as_str());
        let language = locale.split('-').next().unwrap_or(locale);
        find(locale)
            .or_else(|| find(language))
            .or_else(|| find("en"))
            .or_else(|| by.values().next().map(String::as_str))
    }
}

#[cfg(test)]
#[path = "localized_text_tests.rs"]
mod localized_text_tests;
