//! Fermeture des besoins des packs épinglés : `requires`, pris dans la bibliothèque.
//!
//! `Op::AddPack` n'épingle qu'un pack et refuse un besoin absent des packs de
//! widgets du deck ; la session ajoute d'abord, dans la même transaction,
//! chaque espace de noms requis que le deck n'épingle pas, dans la version
//! courante du pack de widgets de la bibliothèque qui le porte (les
//! `requires` des deux sortes ne nomment que des packs de widgets). Parcours en profondeur, besoins d'abord
//! (ordre postfixe), chaque espace de noms une fois ; une boucle de
//! `requires` est refusée (`RequiresCycle`), un espace de noms inconnu de la
//! bibliothèque aussi (`UnknownNamespace`).

use std::collections::{BTreeMap, BTreeSet};

use snafu::OptionExt;

use oku_slides_core::{
    Deck, Element, Namespace, Op, PackFiles, PackKind, PackManifest, PackPin, PackVersion, ParamValue, split_kind,
};

use crate::Result;
use crate::error::{PackVersionMissingSnafu, RequiresCycleSnafu, UnknownNamespaceSnafu};
use crate::library_packs::PackLibrary;

/// L'épinglage de la version courante du pack de widgets de la bibliothèque qui porte `ns`.
pub(crate) fn current_pin(library: &PackLibrary, ns: &Namespace) -> Result<PackPin> {
    let found = library.entries(PackKind::Widgets).into_iter().find(|s| s.manifest.namespace == *ns);
    let stored = found.context(UnknownNamespaceSnafu { namespace: ns.as_str() })?;
    Ok(PackPin { id: stored.manifest.id, version: stored.version, requires: stored.manifest.requires })
}

/// Les épinglages du deck des deux sortes, en une carte : un espace de noms n'est que d'une sorte.
pub(crate) fn combined_pins(deck: &Deck) -> BTreeMap<Namespace, PackPin> {
    deck.all_pins().map(|(_, ns, pin)| (ns.clone(), pin.clone())).collect()
}

/// Les packs de `pins` à écarter avec ceux de `off` : ceux qui requièrent, même de loin,
/// un pack de `off` ou un pack que le deck n'épingle pas.
///
/// Le rendu de la session et celui hors session (`ProbedWidgets`) s'en servent tous deux.
pub(crate) fn dependents(pins: &BTreeMap<Namespace, PackPin>, off: &BTreeSet<Namespace>) -> BTreeSet<Namespace> {
    let mut out = BTreeSet::new();
    loop {
        let gone = |r: &Namespace| off.contains(r) || out.contains(r) || !pins.contains_key(r);
        let more: Vec<Namespace> = pins
            .iter()
            .filter(|(ns, _)| !off.contains(*ns) && !out.contains(*ns))
            .filter(|(ns, p)| p.requires.iter().any(|r| r != *ns && gone(r)))
            .map(|(ns, _)| ns.clone())
            .collect();
        if more.is_empty() {
            return out;
        }
        out.extend(more);
    }
}

/// Le parcours des besoins : espaces de noms déjà là, ops produites, chemin en cours.
struct Closure<'a> {
    library: &'a PackLibrary,
    /// Épinglés par le deck, ou ajoutés par ce parcours.
    present: BTreeSet<Namespace>,
    ops: Vec<Op>,
    /// Le chemin en cours depuis l'ajout demandé : une boucle y repasse.
    path: Vec<Namespace>,
}

impl Closure<'_> {
    /// Ajoute les besoins de `requires` qui manquent, besoins d'abord.
    fn require(&mut self, requires: &[Namespace]) -> Result<()> {
        for ns in requires {
            // Se requérir soi-même ne demande rien (le cœur l'admet).
            if self.path.last() == Some(ns) {
                continue;
            }
            if self.path.contains(ns) {
                let chain: Vec<&str> = self.path.iter().chain([ns]).map(Namespace::as_str).collect();
                return RequiresCycleSnafu { chain: chain.join(" → ") }.fail()?;
            }
            if self.present.contains(ns) {
                continue;
            }
            let pin = current_pin(self.library, ns)?;
            self.path.push(ns.clone());
            self.require(&pin.requires)?;
            self.path.pop();
            self.present.insert(ns.clone());
            self.ops.push(Op::AddPack { kind: PackKind::Widgets, namespace: ns.clone(), pin });
        }
        Ok(())
    }
}

/// `wanted`, chaque `AddPack` précédé de l'ajout des besoins que `deck` n'épingle pas.
///
/// Les besoins viennent de la bibliothèque (version courante), récursivement,
/// besoins d'abord, chaque espace de noms une fois ; les autres ops restent à
/// leur place.
pub(crate) fn closure_ops(library: &PackLibrary, deck: &Deck, wanted: &[Op]) -> Result<Vec<Op>> {
    let present = deck.widgets.keys().cloned().collect();
    let mut closure = Closure { library, present, ops: Vec::with_capacity(wanted.len()), path: Vec::new() };
    for op in wanted {
        if let Op::AddPack { kind, namespace, pin } = op {
            closure.path.push(namespace.clone());
            closure.require(&pin.requires)?;
            closure.path.pop();
            // Seul un pack de widgets peut être requis.
            if *kind == PackKind::Widgets {
                closure.present.insert(namespace.clone());
            }
        }
        closure.ops.push(op.clone());
    }
    Ok(closure.ops)
}

/// Les `requires` du pack `manifest` enregistré sous l'espace de noms `own`, d'après ses exemples.
///
/// Ses `requires` d'avant, puis, dans l'ordre où ses exemples les emploient et
/// chacun une fois, les espaces de noms des kinds qualifiés de leurs valeurs
/// de slot (slots imbriqués compris), le sien excepté.
pub(crate) fn examples_requires(own: &Namespace, manifest: &PackManifest) -> Vec<Namespace> {
    fn walk(elements: &[Element], out: &mut Vec<Namespace>) {
        for e in elements {
            if let Some(ns) = split_kind(&e.kind).and_then(|(ns, _)| Namespace::parse(ns))
                && !out.contains(&ns)
            {
                out.push(ns);
            }
            for (_, children) in e.params.slots() {
                walk(children, out);
            }
        }
    }
    let mut used = Vec::new();
    // Une valeur d'exemple de slot porte des éléments ; toute autre n'a pas de kind.
    for value in manifest.examples.values().flat_map(|m| m.values()) {
        if let Ok(ParamValue::Slot(elements)) = serde_json::from_value(value.clone()) {
            walk(&elements, &mut used);
        }
    }
    let mut out = manifest.requires.clone();
    out.extend(used.into_iter().filter(|ns| ns != own && !manifest.requires.contains(ns)));
    out
}

/// Un pack à épingler : son espace de noms, son épinglage et ses fichiers.
pub(crate) type Pinned = (Namespace, PackPin, PackFiles);

/// Le pack `files` épinglé `pin` sous `ns` dans `deck`, précédé des besoins qui y manquent.
///
/// Les fichiers d'un besoin viennent de `held` (des sources déjà là), sinon de
/// la bibliothèque. Rien n'est changé : à l'appelant d'embarquer et d'épingler.
pub(crate) fn closure_with_files(
    library: &PackLibrary,
    deck: &Deck,
    (ns, pin, files): Pinned,
    held: &BTreeMap<PackVersion, PackFiles>,
) -> Result<Vec<Pinned>> {
    let wanted = Op::AddPack { kind: files.kind(), namespace: ns.clone(), pin };
    let mut out = Vec::new();
    for op in closure_ops(library, deck, &[wanted])? {
        let Op::AddPack { namespace, pin, .. } = op else { continue };
        let found = if namespace == ns {
            files.clone()
        } else {
            let held = held.get(&pin.version).cloned();
            let found = held.or_else(|| library.version(pin.id, &pin.version));
            found.context(PackVersionMissingSnafu { id: pin.id, version: pin.version.clone() })?
        };
        out.push((namespace, pin, found));
    }
    Ok(out)
}

#[cfg(test)]
#[path = "deck_closure_tests.rs"]
mod deck_closure_tests;
