//! Jetons de design d'un thème : fusion des couches.
//!
//! Les jetons effectifs d'un deck sont les défauts du rendu
//! (`default-tokens.json`), puis la couche de thème ([`theme_layer`] : défauts
//! des jetons de composant des packs, puis la chaîne du thème linéarisée),
//! fusionnés objet par objet comme `merge` côté Typst. La session (éditeur de
//! thème), la CLI et l'import partagent ce calcul.

use serde_json::{Map, Value};

use crate::namespace::Namespace;

/// Groupe des jetons de composant : `component.<namespace>.<groupe>`.
const COMPONENT: &str = "component";

/// Fusion profonde de `over` dans `base` : deux objets se fusionnent clé par
/// clé, toute autre valeur de `over` remplace celle de `base`.
pub fn merge(base: &mut Map<String, Value>, over: &Map<String, Value>) {
    for (k, v) in over {
        match (base.get_mut(k), v) {
            (Some(Value::Object(b)), Value::Object(o)) => merge(b, o),
            _ => {
                base.insert(k.clone(), v.clone());
            }
        }
    }
}

/// Les jetons effectifs : `defaults`, puis `theme` fusionné par-dessus.
pub fn effective_tokens(defaults: &Map<String, Value>, theme: &Map<String, Value>) -> Map<String, Value> {
    let mut tokens = defaults.clone();
    merge(&mut tokens, theme);
    tokens
}

/// La couche de thème envoyée au rendu : composants des packs, puis chaque thème de la chaîne.
///
/// `components` donne, par espace de noms de pack, ses défauts de composant
/// (`PackManifest::component_defaults`), posés sous `component.<namespace>` ;
/// un pack sans défaut n'ajoute rien. `chain` donne les jetons de chaque thème
/// dans l'ordre de `theme_chain::linearize` (le thème du deck en dernier) :
/// chacun est fusionné par-dessus, un objet clé par clé, toute autre valeur
/// (liste comprise) remplaçant la précédente.
pub fn theme_layer<'a>(
    components: impl IntoIterator<Item = (&'a Namespace, Map<String, Value>)>,
    chain: impl IntoIterator<Item = &'a Map<String, Value>>,
) -> Map<String, Value> {
    let mut packs = Map::new();
    for (ns, defaults) in components.into_iter().filter(|(_, d)| !d.is_empty()) {
        merge(&mut packs, &Map::from_iter([(ns.as_str().to_owned(), Value::Object(defaults))]));
    }
    let mut layer = Map::new();
    if !packs.is_empty() {
        layer.insert(COMPONENT.to_owned(), Value::Object(packs));
    }
    for tokens in chain {
        merge(&mut layer, tokens);
    }
    layer
}

#[cfg(test)]
#[path = "tokens_tests.rs"]
mod tokens_tests;
