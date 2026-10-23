//! Le schéma composé d'un deck : le schéma global, puis les jetons de composant de ses packs.
//!
//! Les packs sont ceux que le deck épingle (`deck.widgets` et `deck.layouts`),
//! dans l'ordre de leurs espaces de noms ; en mode pack, ceux du deck de
//! travail et ceux du deck mis de côté (un thème choisi ou édité là sert à
//! ce deck : ses jetons pour un pack que seul lui épingle restent visibles et
//! valides). Un sous-arbre `component.<ns>` dont aucun des deux n'épingle le
//! pack n'est pas vérifié : il ne se rend pas
//! ([`without_unpinned_components`]). Chaque manifeste se lit dans les
//! sources (document montré, puis deck mis de côté), sinon parmi les versions
//! intégrées. Un pack dont la version manque ou dont le manifeste ne se lit pas
//! n'apporte pas de jetons : il est journalisé, comme au calque de rendu
//! (`deck_theme::pack_defaults`), où il n'apporte pas non plus de défauts.

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
    /// Les packs dont le schéma de `deck` prend les jetons, par espace de noms :
    /// ceux de `deck`, puis en mode pack ceux du deck mis de côté (le premier l'emporte).
    fn schema_pins<'a>(&'a self, deck: &'a Deck) -> BTreeMap<&'a Namespace, &'a PackPin> {
        let stash = match &self.mode {
            Mode::Pack { stash, .. } => Some(stash.sources.deck()),
            Mode::Deck | Mode::Help { .. } | Mode::History { .. } => None,
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
            Mode::Deck | Mode::Help { .. } => None,
        };
        self.doc
            .sources
            .packs
            .get(version)
            .or_else(|| stash?.packs.get(version))
            .or_else(|| builtin_versions().iter().find(|b| b.version == *version).map(|b| &b.files))
    }

    /// Les jetons de composant des packs que `deck` épingle (en mode pack, avec
    /// ceux du deck mis de côté), par espace de noms.
    pub(crate) fn pack_tokens(&self, deck: &Deck) -> Vec<PackTokens> {
        let mut out = BTreeMap::new();
        for (ns, pin) in self.schema_pins(deck) {
            let manifest = self.pack_version_files(&pin.version).map(PackManifest::from_files);
            match manifest {
                Some(Ok(m)) => {
                    out.insert(ns.clone(), (m.name, m.tokens));
                }
                Some(Err(e)) => {
                    tracing::warn!(pack = %ns, error = %e, "manifeste illisible : pas de jetons de composant")
                }
                None => {
                    tracing::warn!(pack = %ns, version = %pin.version, "version absente : pas de jetons de composant")
                }
            }
        }
        out.into_iter().map(|(ns, (name, tokens))| (ns, name, tokens)).collect()
    }

    /// Le schéma composé de `deck` : global, puis une section par pack épinglé
    /// (en mode pack, par `deck` ou par le deck mis de côté).
    pub(crate) fn token_schema(&self, deck: &Deck) -> Vec<TokenSpec> {
        composed(&self.pack_tokens(deck))
    }
}

#[cfg(test)]
#[path = "theme_composed_tests.rs"]
mod theme_composed_tests;
