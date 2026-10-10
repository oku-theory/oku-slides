//! Décor : ce que le retrofit omet d'une slide par règle, sans le compter
//! dans la couverture.
//!
//! - les placeholders date, pied de page et numéro de slide ;
//! - l'en-tête et le pied répétés : un item dans la bande haute ou basse de
//!   la page (son centre dans les 12 % hauts ou bas, hors du corps source)
//!   dont le texte (chiffres ôtés : un numéro de page varie) ou l'image revient
//!   sur au moins la moitié des slides (deux au moins). Le thème cible dessine
//!   ses propres en-tête et pied. Perte [`LossKind::RepeatedDecor`], une fois
//!   par motif, sur sa première slide ;
//! - un numéro seul dans la bande (texte fait de chiffres et de ponctuation).
//!
//! La projection ([`super::slide`]) et la couverture ([`crate::coverage`])
//! s'en remettent au même prédicat, [`Decor::skips`].

use std::collections::{BTreeMap, BTreeSet};

use crate::report::{Loss, LossKind};
use crate::source::{Frame, Item, ItemKind, Role, Size, SourceDeck};

use super::text::plain;

/// Part haute et basse de la page où vivent en-têtes et pieds.
pub(crate) const BAND: f64 = 0.12;

/// Part de la page au-delà de laquelle une forme sans texte est un fond.
const BACKDROP: f64 = 0.5;

/// Le centre de `frame` est-il dans la bande haute ou basse d'une page `page` ?
pub(crate) fn in_band(frame: &Frame, page: Size) -> bool {
    let center = frame.y + frame.h / 2.0;
    center < BAND * page.h || center > (1.0 - BAND) * page.h
}

/// Une forme sans texte qui couvre au moins la moitié de la page : un fond.
pub(crate) fn is_backdrop(item: &Item, page: Size) -> bool {
    matches!(item.kind, ItemKind::Shape(_))
        && !item.has_text()
        && item.frame.w * item.frame.h >= BACKDROP * page.w * page.h
}

/// Ce qui identifie un item répété : son texte sans chiffres, espaces
/// normalisées, ou son image ; `None` pour un autre item.
fn key(item: &Item) -> Option<String> {
    match &item.kind {
        ItemKind::Picture(p) => Some(format!("image:{}", p.asset)),
        _ => item.text().map(|tb| {
            let text = plain(tb).0;
            let kept: String = text.chars().filter(|c| !c.is_ascii_digit()).collect();
            kept.split_whitespace().collect::<Vec<_>>().join(" ")
        }),
    }
}

/// Un numéro : un texte non vide fait de chiffres, de ponctuation et d'espaces.
fn is_number(item: &Item, key: &str) -> bool {
    item.has_text() && !key.chars().any(char::is_alphanumeric)
}

/// Un motif de bande : les slides où il apparaît, ses items, sa première occurrence.
#[derive(Debug, Default)]
struct Motif<'a> {
    slides: BTreeSet<u32>,
    items: Vec<String>,
    first: Option<(u32, &'a Item)>,
}

/// Ce que la règle omet dans un deck source.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Decor {
    /// Ids des items omis.
    skipped: BTreeSet<String>,
    /// Pertes `RepeatedDecor`, par numéro de slide source.
    losses: BTreeMap<u32, Vec<Loss>>,
}

impl Decor {
    /// Le décor de `deck`.
    pub(crate) fn of(deck: &SourceDeck) -> Self {
        let mut decor = Self::default();
        let mut seen: BTreeMap<String, Motif<'_>> = BTreeMap::new();
        for slide in &deck.slides {
            for item in &slide.items {
                if matches!(item.role, Some(Role::Date | Role::Footer | Role::SlideNum)) {
                    decor.skipped.insert(item.id.clone());
                    continue;
                }
                if !in_band(&item.frame, deck.size) {
                    continue;
                }
                let Some(k) = key(item) else { continue };
                if is_number(item, &k) {
                    decor.skipped.insert(item.id.clone());
                    continue;
                }
                if k.is_empty() {
                    continue;
                }
                let e = seen.entry(k).or_default();
                e.slides.insert(slide.number);
                e.items.push(item.id.clone());
                e.first.get_or_insert((slide.number, item));
            }
        }
        let needed = deck.slides.len().div_ceil(2).max(2);
        for Motif { slides, items, first } in seen.into_values() {
            let Some((number, item)) = first.filter(|_| slides.len() >= needed) else { continue };
            decor.skipped.extend(items);
            let text = match &item.kind {
                ItemKind::Picture(_) => {
                    item.alt.clone().filter(|a| !a.trim().is_empty()).unwrap_or_else(|| item.name.clone())
                }
                _ => item.text().map(|tb| plain(tb).0).unwrap_or_default(),
            };
            // Au plus autant de slides que le deck en a (borné par le lecteur) : la conversion est exacte.
            let count = u32::try_from(slides.len()).unwrap_or(u32::MAX);
            decor
                .losses
                .entry(number)
                .or_default()
                .push(Loss::on(&item.id, LossKind::RepeatedDecor { text, slides: count }));
        }
        decor
    }

    /// L'item est-il omis par règle ?
    pub(crate) fn skips(&self, item: &Item) -> bool {
        self.skipped.contains(&item.id)
    }

    /// Les pertes de décor répété à dire sur la slide source `number`.
    pub(crate) fn losses(&self, number: u32) -> &[Loss] {
        self.losses.get(&number).map_or(&[], Vec::as_slice)
    }
}

#[cfg(test)]
#[path = "decor_tests.rs"]
mod decor_tests;
