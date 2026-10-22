//! Épinglage des packs d'un deck, par sorte : ajout, retrait, utilisateurs.

use snafu::ensure;

use crate::error::{
    BaseRequiredSnafu, OpError, PackInUseSnafu, PackKindConflictSnafu, PackRequiredSnafu, PackRequirementMissingSnafu,
    UnknownNamespaceSnafu,
};
use crate::{Deck, Namespace, PackKind, PackPin, kinds, split_kind};

/// L'autre sorte que `kind`.
fn other(kind: PackKind) -> PackKind {
    match kind {
        PackKind::Widgets => PackKind::Layouts,
        PackKind::Layouts => PackKind::Widgets,
    }
}

/// Épingle `pin` sous `ns` parmi les packs de sorte `kind`.
///
/// Refuse un espace de noms déjà épinglé sous l'autre sorte, puis un besoin
/// absent des packs de widgets (un pack de widgets peut se nommer lui-même).
pub(super) fn add(deck: &mut Deck, kind: PackKind, ns: Namespace, pin: PackPin) -> Result<(), OpError> {
    ensure!(!deck.pins(other(kind)).contains_key(&ns), PackKindConflictSnafu { namespace: ns.to_string() });
    let met = |r: &Namespace| (kind == PackKind::Widgets && *r == ns) || deck.widgets.contains_key(r);
    if let Some(missing) = pin.requires.iter().find(|r| !met(r)) {
        return Err(PackRequirementMissingSnafu { namespace: ns.to_string(), required: missing.to_string() }
            .build()
            .into());
    }
    deck.pins_mut(kind).insert(ns, pin);
    Ok(())
}

/// Retire le pack `ns` de sorte `kind` ; refuse `base`, un pack requis par un autre ou utilisé.
pub(super) fn remove(deck: &mut Deck, kind: PackKind, ns: &Namespace) -> Result<(), OpError> {
    ensure!(!(kind == PackKind::Widgets && ns.as_str() == kinds::base::NAMESPACE), BaseRequiredSnafu);
    ensure!(deck.pins(kind).contains_key(ns), UnknownNamespaceSnafu { namespace: ns.to_string() });
    let mut all = deck.widgets.iter().chain(&deck.layouts);
    if let Some((by, _)) = all.find(|(owner, p)| *owner != ns && p.requires.contains(ns)) {
        return Err(PackRequiredSnafu { namespace: ns.to_string(), by: by.to_string() }.build().into());
    }
    let users = pack_users(deck, ns.as_str());
    ensure!(users.is_empty(), PackInUseSnafu { namespace: ns.to_string(), users });
    deck.pins_mut(kind).remove(ns);
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
