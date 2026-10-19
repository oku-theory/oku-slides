//! Épinglage des packs de widgets d'un deck : ajout, retrait, utilisateurs.

use snafu::ensure;

use crate::error::{
    BaseRequiredSnafu, OpError, PackInUseSnafu, PackRequiredSnafu, PackRequirementMissingSnafu, UnknownNamespaceSnafu,
};
use crate::{Deck, Namespace, PackPin, kinds, split_kind};

/// Épingle `pin` sous `ns` ; refuse un besoin non épinglé.
pub(super) fn add(deck: &mut Deck, ns: Namespace, pin: PackPin) -> Result<(), OpError> {
    if let Some(missing) = pin.requires.iter().find(|r| **r != ns && !deck.widgets.contains_key(*r)) {
        return Err(PackRequirementMissingSnafu { namespace: ns.to_string(), required: missing.to_string() }
            .build()
            .into());
    }
    deck.widgets.insert(ns, pin);
    Ok(())
}

/// Retire le pack `ns` ; refuse `base`, un pack requis par un autre ou utilisé.
pub(super) fn remove(deck: &mut Deck, ns: &Namespace) -> Result<(), OpError> {
    ensure!(ns.as_str() != kinds::base::NAMESPACE, BaseRequiredSnafu);
    ensure!(deck.widgets.contains_key(ns), UnknownNamespaceSnafu { namespace: ns.to_string() });
    if let Some((by, _)) = deck.widgets.iter().find(|(other, p)| *other != ns && p.requires.contains(ns)) {
        return Err(PackRequiredSnafu { namespace: ns.to_string(), by: by.to_string() }.build().into());
    }
    let users = pack_users(deck, ns.as_str());
    ensure!(users.is_empty(), PackInUseSnafu { namespace: ns.to_string(), users });
    deck.widgets.remove(ns);
    Ok(())
}

/// Ids des slides et éléments (imbriqués compris) dont le widget est du pack `ns`.
pub fn pack_users(deck: &Deck, ns: &str) -> Vec<String> {
    let of_ns = |kind: &str| split_kind(kind).is_some_and(|(n, _)| n == ns);
    let mut out = Vec::new();
    for slide in &deck.slides {
        if of_ns(&slide.kind) {
            out.push(slide.id.clone());
        }
        out.extend(slide.walk().filter(|l| of_ns(&l.element.kind)).map(|l| l.element.id.clone()));
    }
    out
}

#[cfg(test)]
#[path = "packs_tests.rs"]
mod packs_tests;
