//! A deck's composed schema: the global schema, then its packs' component tokens.
//!
//! The packs are those the deck pins (`deck.widgets` and `deck.layouts`), in
//! the order of their namespaces; in pack mode, those of the working deck and
//! those of the set-aside deck. The working deck keeps that deck's theme and
//! token overrides (`pack_mode`), never edited there: its tokens for a pack
//! only the set-aside deck pins stay listed and valid. A `component.<ns>`
//! subtree whose pack neither pins is not checked: it does not render
//! ([`without_unpinned_components`]). Each manifest is read from the sources
//! (shown document, then set-aside deck), else among the built-in versions. A
//! pack whose version is missing or whose manifest does not read brings no
//! tokens: it is logged, as in the render layer (`deck_theme::pack_defaults`),
//! where it brings no defaults either.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use oku_slides_core::{Deck, Namespace, PackFiles, PackManifest, PackPin, PackVersion};

use crate::library::builtin_versions;
use crate::theme_schema::{PackTokens, TokenSpec, composed};
use crate::{Mode, Session};

/// Un espace de noms de pack intégré : ses jetons se nomment par Fluent.
pub(crate) fn is_builtin_namespace(ns: &Namespace) -> bool {
    builtin_versions().iter().any(|b| b.manifest.namespace == *ns)
}

/// La famille des jetons de composant, `component.<ns>`.
const COMPONENT: &str = "component";

/// `tokens` sans les sous-arbres `component.<ns>` des packs que `pinned` ne tient pas.
///
/// La vérification stricte d'un thème les accepte tels quels : aucun pack
/// épinglé ne les lit, ils ne se rendent pas.
pub(crate) fn without_unpinned_components(
    tokens: &Map<String, Value>,
    pinned: &BTreeSet<Namespace>,
) -> Map<String, Value> {
    let mut out = tokens.clone();
    if let Some(Value::Object(components)) = out.get_mut(COMPONENT) {
        components.retain(|ns, _| pinned.iter().any(|p| p.as_str() == ns));
    }
    out
}

impl Session {
    /// The packs whose tokens `deck`'s schema takes, by namespace.
    ///
    /// Those of `deck`, then in pack mode those of the set-aside deck, whose
    /// token overrides the working deck keeps (the first one wins).
    fn schema_pins<'a>(&'a self, deck: &'a Deck) -> BTreeMap<&'a Namespace, &'a PackPin> {
        let stash = match &self.mode {
            Mode::Pack { stash, .. } => Some(stash.sources.deck()),
            Mode::Deck | Mode::Help { .. } | Mode::History { .. } | Mode::Theme { .. } => None,
        };
        let mut pins = BTreeMap::new();
        for (_, ns, pin) in deck.all_pins().chain(stash.into_iter().flat_map(Deck::all_pins)) {
            pins.entry(ns).or_insert(pin);
        }
        pins
    }

    /// Les espaces de noms des packs du schéma de `deck` (voir [`Self::token_schema`]).
    pub(crate) fn schema_namespaces(&self, deck: &Deck) -> BTreeSet<Namespace> {
        self.schema_pins(deck).into_keys().cloned().collect()
    }

    /// Les fichiers de la version `version` d'un pack : embarqués, sinon intégrés.
    pub(crate) fn pack_version_files(&self, version: &PackVersion) -> Option<&PackFiles> {
        let stash = match &self.mode {
            Mode::Pack { stash, .. } | Mode::History { stash, .. } => Some(&stash.sources),
            Mode::Deck | Mode::Help { .. } | Mode::Theme { .. } => None,
        };
        self.doc
            .sources
            .packs
            .get(version)
            .or_else(|| stash?.packs.get(version))
            .or_else(|| builtin_versions().iter().find(|b| b.version == *version).map(|b| &b.files))
    }

    /// The component tokens of the packs `deck` pins, by namespace.
    ///
    /// In pack mode, with those of the set-aside deck (see [`Self::schema_pins`]).
    pub(crate) fn pack_tokens(&self, deck: &Deck) -> Vec<PackTokens> {
        let mut out = BTreeMap::new();
        for (ns, pin) in self.schema_pins(deck) {
            let manifest = self.pack_version_files(&pin.version).map(PackManifest::from_files);
            match manifest {
                Some(Ok(m)) => {
                    out.insert(ns.clone(), (m.name, m.tokens));
                }
                Some(Err(e)) => {
                    tracing::warn!(pack = %ns, error = %e, "manifeste illisible : pas de jetons de composant");
                }
                None => {
                    tracing::warn!(pack = %ns, version = %pin.version, "version absente : pas de jetons de composant");
                }
            }
        }
        out.into_iter().map(|(ns, (name, tokens))| (ns, name, tokens)).collect()
    }

    /// The composed schema of `deck`: global, then one section per pinned pack.
    ///
    /// In pack mode, a pack pinned by `deck` or by the set-aside deck.
    pub(crate) fn token_schema(&self, deck: &Deck) -> Vec<TokenSpec> {
        composed(&self.pack_tokens(deck))
    }
}

#[cfg(test)]
#[path = "theme_composed_tests.rs"]
mod theme_composed_tests;
