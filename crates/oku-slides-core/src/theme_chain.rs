//! La chaîne d'un thème : ses épinglages dans un deck et leur ordre d'application.
//!
//! Un deck épingle son thème et toute la fermeture de ses `extends`
//! ([`ThemePin`] par espace de noms). [`linearize`] donne l'ordre
//! d'application : parcours en profondeur, post-ordre, chaque thème une seule
//! fois à sa première position, le thème du deck en dernier ; les parents
//! suivants l'emportent sur les précédents. [`check_closure`] vérifie qu'une
//! carte d'épinglages est exactement la fermeture d'un thème. Le parcours est
//! itératif : un deck non fiable ne peut pas épuiser la pile.

use std::collections::{BTreeMap, BTreeSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::namespace::Namespace;
use crate::pack::PackVersion;

/// Un thème épinglé par un deck : id, version embarquée et parents.
///
/// `extends` recopie celui du `theme.json` de la version : le cœur vérifie
/// la fermeture sans lire les fichiers ; les sources vérifient la copie.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct ThemePin {
    /// Id du thème (UUID), stable d'une version à l'autre.
    #[schemars(with = "String")]
    pub id: Uuid,
    /// Version embarquée par les sources du deck.
    pub version: PackVersion,
    /// Espaces de noms des thèmes parents, dans l'ordre, épinglés aussi par le deck.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extends: Vec<Namespace>,
}

/// Pourquoi une carte d'épinglages de thèmes n'est pas une chaîne valide.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ThemeChainError {
    /// Les `extends` bouclent : `chain` va du premier thème répété à sa répétition.
    Cycle {
        /// Les thèmes de la boucle, le premier répété à la fin.
        chain: Vec<Namespace>,
    },
    /// Un thème atteint (le thème lui-même ou un parent) n'est pas épinglé.
    Missing {
        /// Le thème atteint sans épinglage.
        namespace: Namespace,
    },
    /// Un thème épinglé n'est pas dans la fermeture du thème du deck.
    Extra {
        /// Le thème épinglé de trop.
        namespace: Namespace,
    },
}

/// L'ordre d'application des thèmes de `theme`, lui-même en dernier.
///
/// Post-ordre en profondeur sur `extends`, chaque thème une seule fois à sa
/// première position. Une boucle est [`ThemeChainError::Cycle`] ; un thème
/// atteint absent de `pins` est [`ThemeChainError::Missing`].
pub fn linearize(theme: &Namespace, pins: &BTreeMap<Namespace, ThemePin>) -> Result<Vec<Namespace>, ThemeChainError> {
    let root = pins.get(theme).ok_or_else(|| ThemeChainError::Missing { namespace: theme.clone() })?;
    let mut out = Vec::new();
    let mut done: BTreeSet<&Namespace> = BTreeSet::new();
    // Le chemin courant : chaque thème, son épinglage et le prochain parent à visiter.
    let mut path: Vec<(&Namespace, &ThemePin, usize)> = vec![(theme, root, 0)];
    while let Some(&mut (ns, pin, ref mut next)) = path.last_mut() {
        let Some(parent) = pin.extends.get(*next) else {
            done.insert(ns);
            out.push(ns.clone());
            path.pop();
            continue;
        };
        *next += 1;
        if let Some(start) = path.iter().position(|(n, _, _)| *n == parent) {
            let mut chain: Vec<Namespace> = path[start..].iter().map(|(n, _, _)| (*n).clone()).collect();
            chain.push(parent.clone());
            return Err(ThemeChainError::Cycle { chain });
        }
        if done.contains(parent) {
            continue;
        }
        let parent_pin = pins.get(parent).ok_or_else(|| ThemeChainError::Missing { namespace: parent.clone() })?;
        path.push((parent, parent_pin, 0));
    }
    Ok(out)
}

/// Vérifie que `pins` est exactement la fermeture de `theme`.
///
/// `None` (le thème par défaut) demande une carte vide. Sinon la chaîne doit
/// se linéariser ([`linearize`]) et chaque épinglage y être atteint : un
/// épinglage de trop est [`ThemeChainError::Extra`]. Les `extends` parcourus
/// sont ceux des épinglages : la fermeture suit leur copie.
pub fn check_closure(theme: Option<&Namespace>, pins: &BTreeMap<Namespace, ThemePin>) -> Result<(), ThemeChainError> {
    let reached: BTreeSet<Namespace> = match theme {
        Some(theme) => linearize(theme, pins)?.into_iter().collect(),
        None => BTreeSet::new(),
    };
    match pins.keys().find(|ns| !reached.contains(*ns)) {
        Some(extra) => Err(ThemeChainError::Extra { namespace: extra.clone() }),
        None => Ok(()),
    }
}

#[cfg(test)]
#[path = "theme_chain_tests.rs"]
mod theme_chain_tests;
