//! Ce que le rapport dit du rendu de référence d'un import hi-fi : le témoin,
//! le seuil, et pourquoi la référence manque (jamais une erreur de l'import).

use oku_slides_i18n::{Localize, Localizer};
use serde::Serialize;

/// Le rendu de référence d'un import hi-fi.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReferenceReport {
    /// Nom et version du témoin (`office2pdf 0.8.1`).
    pub witness: String,
    /// Écart (ΔE OKLab ×100) de sa pire région au-delà duquel une slide est signalée.
    pub threshold: f64,
    /// Pourquoi les slides n'ont pas de score ; `None` : elles en ont un.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unavailable: Option<ReferenceCause>,
}

/// Pourquoi la référence manque.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ReferenceCause {
    /// Le témoin n'a pas rendu la présentation (fichier qu'il ne lit pas, erreur).
    Failed,
    /// Le témoin a dépassé le délai du processus de rendu.
    Timeout,
    /// Le témoin a dépassé la mémoire du processus de rendu.
    Memory,
    /// Le témoin et le deck n'ont pas le même nombre de pages.
    Pages {
        /// Pages du témoin.
        reference: u32,
        /// Pages attendues du deck pour la plage comparée (une tranche de 20 slides au plus).
        deck: u32,
    },
}

impl Localize for ReferenceCause {
    fn localize(&self, l: &Localizer) -> String {
        let (cause, reference, deck) = match self {
            Self::Failed => ("failed", 0, 0),
            Self::Timeout => ("timeout", 0, 0),
            Self::Memory => ("memory", 0, 0),
            Self::Pages { reference, deck } => ("pages", *reference, *deck),
        };
        l.tr_with(
            "import-reference-unavailable",
            &[("cause", cause.into()), ("reference", reference.into()), ("deck", deck.into())],
        )
    }
}

#[cfg(test)]
#[path = "reference_tests.rs"]
mod reference_tests;
