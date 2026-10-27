//! The themes a theme tab's draft extends: the `extends` list of `ThemeEdit`.
//!
//! The list changes whole (`set_edited_theme_extends`, the agent's tool) or
//! by a delta on the current list (`edit_theme_extends`, the buttons of the
//! `host-theme` surface), so that two close clicks do not overwrite each
//! other. Each accepted change is an undo step under a new rank of
//! `DRAFT_SWITCH_VAR` (`apply_draft_step`), without changing the tokens:
//! undo or redo brings the list back through `realign_theme_edit`.
//!
//! The checks, cheapest first: a theme tab; no duplicate (a list longer
//! than the library has one, or an unknown theme); each theme in the
//! library; last the draft's closure, where a loop (through its own
//! namespace) is `ThemeCycle`. Resuming a state goes through the same ones
//! (`check_theme_extends`).

use std::collections::BTreeSet;

use serde::Deserialize;
use serde_json::Value;
use snafu::{OptionExt, ensure};

use oku_slides_core::{Namespace, Op};

use crate::error::{ThemeExtendsDuplicateSnafu, ThemeExtendsMoveSnafu, UnknownThemeSnafu};
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
    /// Fails outside a theme tab (`WrongDocument`), if the list is rejected (see the module), or if the
    /// transaction fails.
    pub fn set_edited_theme_extends(&mut self, extends: Vec<Namespace>) -> Result<()> {
        // Outside a theme tab, `ensure_theme_tab` says why.
        let Mode::Theme { edit, switches, .. } = &self.mode else { return self.ensure_theme_tab() };
        let from = edit.draft.clone();
        let rank = next_switch_rank(self.doc.sources.deck(), switches);
        self.check_theme_extends(&from, &extends)?;
        if from.extends == extends {
            return Ok(());
        }
        let to = ThemeEdit { extends, ..from.clone() };
        let label = self.localizer.tr("session-undo-theme-extends");
        // The list changes before the step, which renders the draft and writes
        // the resume state; a failure puts it back.
        self.set_theme_edit(to.clone());
        let ops = vec![Op::SetVar { key: DRAFT_SWITCH_VAR.to_owned(), value: Value::from(rank) }];
        if let Err(e) = self.apply_draft_step(&label, ops) {
            self.set_theme_edit(from);
            return Err(e);
        }
        if let Mode::Theme { switches, .. } = &mut self.mode {
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
    /// Fails outside a theme tab (`WrongDocument`), on an impossible move (`ThemeExtendsMove`), a
    /// missing theme (`UnknownTheme`), or like `set_edited_theme_extends`.
    pub fn edit_theme_extends(&mut self, change: ExtendsEdit) -> Result<()> {
        let Mode::Theme { edit, .. } = &self.mode else { return self.ensure_theme_tab() };
        let list = edited_list(edit.draft.extends.clone(), change)?;
        self.set_edited_theme_extends(list)
    }
}

#[cfg(test)]
#[path = "theme_extends_tests.rs"]
pub(crate) mod theme_extends_tests;
