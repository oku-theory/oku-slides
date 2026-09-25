//! Graphiques reçus dans des ops (de l'interface ou d'un LLM) : rangés sous
//! forme canonique avant d'entrer dans le deck. Leurs données passent dans
//! `dataset`, où la grille les édite ; les en-têtes qui manquent sont écrits
//! dans la langue de l'utilisateur.

use slider_chart::{ChartLabels, canonicalize};
use slider_core::{Deck, Element, ElementBody, Op};

/// Canonicalise les graphiques des éléments et slides ajoutés ou remplacés.
pub(crate) fn canonicalize_charts(ops: &mut [Op], labels: &ChartLabels) {
    for op in ops {
        match op {
            Op::AddElement { element, .. } | Op::ReplaceElement { element, .. } => canonicalize_tree(element, labels),
            Op::AddSlide { slide, .. } => {
                for e in slide.blocks.iter_mut().chain(slide.elements.iter_mut()) {
                    canonicalize_tree(e, labels);
                }
            }
            _ => {}
        }
    }
}

/// Canonicalise tous les graphiques d'un deck (à l'ouverture).
pub(crate) fn canonicalize_deck(deck: &mut Deck, labels: &ChartLabels) {
    for slide in &mut deck.slides {
        for e in slide.blocks.iter_mut().chain(slide.elements.iter_mut()) {
            canonicalize_tree(e, labels);
        }
    }
}

fn canonicalize_tree(e: &mut Element, labels: &ChartLabels) {
    if let ElementBody::Chart { option } = &mut e.body {
        match canonicalize(option, labels) {
            Ok(canonical) => *option = canonical,
            // L'op refusera ce graphique avec la raison ; rien à ranger ici.
            Err(error) => tracing::debug!(element = %e.id, %error, "graphique non canonicalisé"),
        }
    }
    for child in e.children_mut().into_iter().flatten() {
        canonicalize_tree(child, labels);
    }
}

#[cfg(test)]
#[path = "charts_tests.rs"]
mod charts_tests;
