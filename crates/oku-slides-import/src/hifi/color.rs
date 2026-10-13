//! Couleurs source écrites pour le thème généré.
//!
//! Une couleur du thème source, sans modificateur, devient la référence du
//! jeton de son emplacement ([`SLOT_TOKENS`] : `dk1` → `ink`, `accent2` →
//! `accent-2`…) : changer le thème du deck la suit. Toute autre couleur
//! (modifiée, écrite telle quelle, par défaut du lecteur) s'écrit en
//! hexadécimal résolu, `#rrggbb`, ou `#rrggbbaa` si elle n'est pas opaque
//! (`color-of` côté Typst et `is_hex_color` côté cœur admettent les deux).
//!
//! L'emplacement d'une [`Origin::Scheme`] est déjà celui d'après `clrMap`
//! (`tx1` → `dk1`, `bg2` → `lt2` : le lecteur résout les alias) ; seule la
//! table des 12 emplacements est donc lue. Le thème généré est celui d'un
//! seul master ([`super::theme::theme_master`]) : une couleur d'un autre
//! master dont le thème diffère n'a pas la valeur du jeton, elle passe en
//! hexadécimal. La comparaison des valeurs suffit à le voir.

use std::collections::BTreeMap;

use crate::source::{Color, Origin, Rgba, SourceTheme};

/// Emplacement du thème source → jeton `color` du thème généré.
pub(crate) const SLOT_TOKENS: [(&str, &str); 12] = [
    ("lt1", "bg"),
    ("dk1", "ink"),
    ("dk2", "ink-muted"),
    ("lt2", "surface"),
    ("accent1", "accent"),
    ("accent2", "accent-2"),
    ("accent3", "accent-3"),
    ("accent4", "accent-4"),
    ("accent5", "accent-5"),
    ("accent6", "accent-6"),
    ("hlink", "link"),
    ("folHlink", "link-visited"),
];

/// Le jeton de l'emplacement `slot`.
pub(crate) fn slot_token(slot: &str) -> Option<&'static str> {
    SLOT_TOKENS.iter().find(|(s, _)| *s == slot).map(|(_, t)| *t)
}

/// Les couleurs du thème généré : ses jetons d'emplacement et leur valeur.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct ThemeColors {
    /// Jeton → valeur, pour chaque emplacement présent dans le thème source.
    tokens: BTreeMap<&'static str, Rgba>,
}

impl ThemeColors {
    /// Les couleurs du thème source `theme`, celui du thème généré.
    pub(crate) fn new(theme: &SourceTheme) -> Self {
        let tokens = SLOT_TOKENS.iter().filter_map(|(slot, token)| Some((*token, *theme.colors.get(*slot)?))).collect();
        Self { tokens }
    }

    /// La valeur du jeton `token`, s'il vient d'un emplacement.
    pub(crate) fn get(&self, token: &str) -> Option<Rgba> {
        self.tokens.get(token).copied()
    }

    /// La palette des graphiques et des pastilles (`chart.palette`) : les
    /// jetons d'accent présents, dans l'ordre (`accent`, `accent-2`…).
    pub(crate) fn palette(&self) -> Vec<&'static str> {
        SLOT_TOKENS.iter().map(|(_, t)| *t).filter(|t| t.starts_with("accent") && self.get(t).is_some()).collect()
    }

    /// La couleur Oku de `c` : nom de jeton ou hexadécimal.
    pub(crate) fn write(&self, c: &Color) -> String {
        match &c.origin {
            Origin::Scheme { slot, mods, .. } if mods.is_empty() => slot_token(slot)
                .filter(|t| self.get(t) == Some(c.value))
                .map_or_else(|| c.value.to_hex(), str::to_owned),
            Origin::Scheme { .. } | Origin::Explicit | Origin::Default => c.value.to_hex(),
        }
    }
}

#[cfg(test)]
#[path = "color_tests.rs"]
mod color_tests;
