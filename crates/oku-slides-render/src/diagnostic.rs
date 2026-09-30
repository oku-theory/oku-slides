//! Messages de Typst : erreurs et avertissements, avec leur place dans les
//! sources et leurs indices. Ils se disent dans la langue de l'utilisateur.

use oku_slides_i18n::{Localize, Localizer};

/// Gravité d'un message de Typst.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Severity {
    Error,
    Warning,
}

/// Un message de Typst.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    /// Fichier et ligne (`oku-slides/src/data.typ:12`), s'ils sont connus.
    pub place: Option<String>,
    /// Le message de Typst, tel quel (en anglais).
    pub message: String,
    pub hints: Vec<String>,
}

impl Localize for Diagnostic {
    fn localize(&self, l: &Localizer) -> String {
        let level = l.tr(match self.severity {
            Severity::Error => "diagnostic-error",
            Severity::Warning => "diagnostic-warning",
        });
        let mut out = match &self.place {
            Some(place) => l.tr_with(
                "diagnostic",
                &[("level", level.into()), ("place", place.as_str().into()), ("message", self.message.as_str().into())],
            ),
            None => {
                l.tr_with("diagnostic-unplaced", &[("level", level.into()), ("message", self.message.as_str().into())])
            }
        };
        for hint in &self.hints {
            out.push_str("\n  ");
            out.push_str(&l.tr_with("diagnostic-hint", &[("hint", hint.as_str().into())]));
        }
        out
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.localize(oku_slides_i18n::english()))
    }
}
