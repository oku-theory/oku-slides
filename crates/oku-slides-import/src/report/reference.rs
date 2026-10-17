//! Ce que le rapport dit du rendu de référence d'un import hi-fi.
//!
//! Le témoin, le seuil, pourquoi la référence manque (jamais une erreur de l'import) et pourquoi
//! une région est suspecte.

use oku_slides_i18n::{Localize, Localizer};
use serde::Serialize;
use serde_json::Value;

use crate::metrics::{RegionScore, WitnessLimit};

/// Le rendu de référence d'un import hi-fi.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReferenceReport {
    /// Nom et version du témoin (`office2pdf 0.8.1`, `LibreOffice 26.8.1.1`).
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
    /// Le témoin, un programme externe (LibreOffice), est introuvable.
    Missing,
    /// Le témoin et le deck n'ont pas le même nombre de pages.
    Pages {
        /// Pages du témoin.
        reference: u32,
        /// Pages attendues du deck pour la plage comparée (une tranche de 20 slides au plus).
        deck: u32,
    },
}

/// Le nom de la cause et ses nombres, pour le journal et les erreurs.
///
/// Le message dit, lui, se localise.
impl std::fmt::Display for ReferenceCause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed => f.write_str("failed"),
            Self::Timeout => f.write_str("timeout"),
            Self::Memory => f.write_str("memory"),
            Self::Missing => f.write_str("missing"),
            Self::Pages { reference, deck } => write!(f, "pages ({reference} rendered, {deck} expected)"),
        }
    }
}

impl Localize for ReferenceCause {
    fn localize(&self, l: &Localizer) -> String {
        let (cause, reference, deck) = match self {
            Self::Failed => ("failed", 0, 0),
            Self::Timeout => ("timeout", 0, 0),
            Self::Memory => ("memory", 0, 0),
            Self::Missing => ("missing", 0, 0),
            Self::Pages { reference, deck } => ("pages", *reference, *deck),
        };
        l.tr_with(
            "import-reference-unavailable",
            &[("cause", cause.into()), ("reference", reference.into()), ("deck", deck.into())],
        )
    }
}

impl Localize for WitnessLimit {
    fn localize(&self, l: &Localizer) -> String {
        l.tr_with("import-region-suspect", &[("limit", self.name().into())])
    }
}

/// Ajoute à chaque région suspecte de `written` le message de sa limite.
///
/// `written` : les premières régions de `regions`.
pub(super) fn suspect_messages(written: &mut [Value], regions: &[RegionScore], l: &Localizer) {
    for (w, region) in written.iter_mut().zip(regions) {
        if let (Some(o), Some(limit)) = (w.as_object_mut(), region.suspect) {
            o.insert("message".to_owned(), Value::String(limit.localize(l)));
        }
    }
}

#[cfg(test)]
#[path = "reference_tests.rs"]
mod reference_tests;
