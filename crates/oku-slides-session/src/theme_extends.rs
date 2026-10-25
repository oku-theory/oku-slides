//! Les thèmes qu'étend le brouillon du mode pack : la liste `extends` de `ThemeEdit`.
//!
//! La liste se change en entier (`set_edited_theme_extends`, l'outil de
//! l'agent) ou par un delta sur la liste courante (`edit_theme_extends`, les
//! boutons de la surface `host-theme`), pour que deux clics rapprochés ne
//! s'écrasent pas. Chaque changement accepté est une étape d'annulation sous
//! un rang neuf de `DRAFT_SWITCH_VAR`, sans changer les jetons : annuler ou
//! rétablir ramène la liste par `realign_theme_edit`, comme un changement de base.
//!
//! Les vérifications, les moins chères d'abord : mode pack ; aucun doublon
//! (une liste plus longue que la bibliothèque en a forcément un, ou un
//! inconnu) ; chaque thème dans la bibliothèque ; enfin la fermeture du
//! brouillon, où une boucle (par son propre espace de noms) est `ThemeCycle`.
//! La reprise d'un état passe par les mêmes (`check_theme_extends`).

use std::collections::BTreeSet;

use serde::Deserialize;
use serde_json::Value;
use snafu::{OptionExt, ensure};

use oku_slides_core::{Namespace, Op};

use crate::error::{NotEditingPackSnafu, ThemeExtendsDuplicateSnafu, ThemeExtendsMoveSnafu, UnknownThemeSnafu};
use crate::theme_edit::{DRAFT_SWITCH_VAR, ThemeEdit, ThemeSwitch, next_switch_rank};
use crate::{Mode, Result, Session};

/// Un changement de la liste des thèmes qu'étend le brouillon.
///
/// En JSON : `{"edit": "add", "theme": "<espace de noms>"}`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "edit", rename_all = "lowercase")]
pub enum ExtendsEdit {
    /// Ajoute `theme` en dernier : il l'emporte sur les autres.
    Add {
        /// Le thème ajouté.
        theme: Namespace,
    },
    /// Retire `theme` de la liste.
    Remove {
        /// Le thème retiré.
        theme: Namespace,
    },
    /// Monte `theme` d'un rang (le premier ne peut pas monter).
    Up {
        /// Le thème monté.
        theme: Namespace,
    },
    /// Descend `theme` d'un rang (le dernier ne peut pas descendre).
    Down {
        /// Le thème descendu.
        theme: Namespace,
    },
}

/// La liste `list` changée par `edit`.
fn edited_list(mut list: Vec<Namespace>, edit: ExtendsEdit) -> Result<Vec<Namespace>> {
    let at = |list: &[Namespace], theme: &Namespace| -> Result<usize> {
        Ok(list.iter().position(|n| n == theme).context(UnknownThemeSnafu { theme: theme.as_str() })?)
    };
    match edit {
        ExtendsEdit::Add { theme } => list.push(theme),
        ExtendsEdit::Remove { theme } => {
            let i = at(&list, &theme)?;
            list.remove(i);
        }
        ExtendsEdit::Up { theme } => {
            let i = at(&list, &theme)?;
            ensure!(i > 0, ThemeExtendsMoveSnafu { theme: theme.as_str() });
            list.swap(i - 1, i);
        }
        ExtendsEdit::Down { theme } => {
            let i = at(&list, &theme)?;
            ensure!(i + 1 < list.len(), ThemeExtendsMoveSnafu { theme: theme.as_str() });
            list.swap(i, i + 1);
        }
    }
    Ok(list)
}

impl Session {
    /// Vérifie que le brouillon `edit` peut étendre `extends` (voir le module).
    pub(crate) fn check_theme_extends(&self, edit: &ThemeEdit, extends: &[Namespace]) -> Result<()> {
        let library: BTreeSet<Namespace> = self.themes.themes().into_iter().map(|t| t.namespace).collect();
        // Au-delà de la taille de la bibliothèque, ce préfixe a déjà un doublon ou un inconnu.
        let head = &extends[..extends.len().min(library.len() + 1)];
        let mut seen = BTreeSet::new();
        for ns in head {
            ensure!(seen.insert(ns), ThemeExtendsDuplicateSnafu { theme: ns.as_str() });
        }
        for ns in head {
            ensure!(library.contains(ns), UnknownThemeSnafu { theme: ns.as_str() });
        }
        let candidate = ThemeEdit { extends: extends.to_vec(), ..edit.clone() };
        self.draft_chain(&candidate).map(|_| ())
    }

    /// The draft now extends `extends`, in this order: one undo step.
    ///
    /// No effect if the list does not change. See the module for rejections.
    ///
    /// # Errors
    ///
    /// Fails outside pack mode (`NotEditingPack`), if the list is rejected (see the module), or if the
    /// transaction fails.
    pub fn set_edited_theme_extends(&mut self, extends: Vec<Namespace>) -> Result<()> {
        let Mode::Pack { theme, switches, .. } = &self.mode else {
            return NotEditingPackSnafu.fail()?;
        };
        let from = theme.clone();
        let rank = next_switch_rank(self.doc.sources.deck(), switches);
        self.check_theme_extends(&from, &extends)?;
        if from.extends == extends {
            return Ok(());
        }
        let to = ThemeEdit { extends, ..from.clone() };
        let label = self.localizer.tr("session-undo-theme-extends");
        // Comme un changement de base : la liste change avant `apply`, qui
        // rend le brouillon et écrit l'état de reprise ; un échec la remet.
        self.set_theme_edit(to.clone());
        let ops = vec![Op::SetVar { key: DRAFT_SWITCH_VAR.to_owned(), value: Value::from(rank) }];
        if let Err(e) = self.apply(&label, ops, None) {
            self.set_theme_edit(from);
            return Err(e);
        }
        if let Mode::Pack { switches, .. } = &mut self.mode {
            switches.push(ThemeSwitch { rank, from, to });
        }
        Ok(())
    }

    /// Applies `edit` to the draft's current list, then sets it (`set_edited_theme_extends`).
    ///
    /// Moving the first up or the last down is `ThemeExtendsMove`;
    /// removing or moving a theme missing from the list, `UnknownTheme`.
    ///
    /// # Errors
    ///
    /// Fails outside pack mode, on an impossible move (`ThemeExtendsMove`), a missing theme
    /// (`UnknownTheme`), or like `set_edited_theme_extends`.
    pub fn edit_theme_extends(&mut self, edit: ExtendsEdit) -> Result<()> {
        let Mode::Pack { theme, .. } = &self.mode else {
            return NotEditingPackSnafu.fail()?;
        };
        let list = edited_list(theme.extends.clone(), edit)?;
        self.set_edited_theme_extends(list)
    }
}

#[cfg(test)]
#[path = "theme_extends_tests.rs"]
pub(crate) mod theme_extends_tests;
