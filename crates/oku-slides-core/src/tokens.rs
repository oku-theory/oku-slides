//! Jetons de design d'un thème : fusion des couches.
//!
//! Les jetons effectifs d'un deck sont les défauts du rendu
//! (`default-tokens.json`), puis ceux de son thème, fusionnés objet par objet
//! comme `merge` côté Typst. La session (éditeur de thème), la CLI et l'import
//! partagent ce calcul.

use serde_json::{Map, Value};

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

#[cfg(test)]
#[path = "tokens_tests.rs"]
mod tokens_tests;
