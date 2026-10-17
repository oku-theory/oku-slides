//! Les groupes source répétés d'une présentation.
//!
//! Candidats hi-fi à un widget d'élément du pack généré.
//!
//! Un groupe de premier niveau d'une slide (pas un SmartArt dessiné, qui
//! reste tel quel) a la signature de ses enfants ([`signature`]) ; une
//! signature présente sur [`GROUP_MIN_SLIDES`] slides au moins fait un
//! candidat. L'agent le propose à l'utilisateur ; rien n'est changé ici.

use std::collections::{BTreeMap, BTreeSet};

use crate::pattern::signature;
use crate::plan::GroupCandidate;
use crate::source::{ItemKind, SourceDeck};

/// Slides distinctes, au moins, où un groupe doit se répéter pour être candidat.
pub const GROUP_MIN_SLIDES: usize = 3;

/// Les groupes répétés de `deck`, les plus répandus d'abord, puis par signature.
///
/// Chaque occurrence nomme la slide Oku (`s<numéro>`) et l'item source du groupe.
pub(crate) fn repeated(deck: &SourceDeck) -> Vec<GroupCandidate> {
    let mut by_signature: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for slide in &deck.slides {
        for item in &slide.items {
            if let ItemKind::Group { items, smartart: None } = &item.kind
                && !items.is_empty()
            {
                let occurrence = (format!("s{}", slide.number), item.id.clone());
                by_signature.entry(signature(items)).or_default().push(occurrence);
            }
        }
    }
    let mut groups: Vec<GroupCandidate> = by_signature
        .into_iter()
        .filter(|(_, occurrences)| {
            occurrences.iter().map(|(slide, _)| slide).collect::<BTreeSet<_>>().len() >= GROUP_MIN_SLIDES
        })
        .map(|(signature, occurrences)| GroupCandidate { signature, occurrences })
        .collect();
    groups.sort_by(|a, b| b.occurrences.len().cmp(&a.occurrences.len()).then_with(|| a.signature.cmp(&b.signature)));
    groups
}

#[cfg(test)]
#[path = "groups_tests.rs"]
mod groups_tests;
