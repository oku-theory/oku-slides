//! L'onglet d'aide : un seul par langue, rouvert plutôt que dupliqué.

use oku_slides_i18n::Localizer;

use super::{Pending, Workspace};
use crate::help::help_locale;

impl Workspace {
    /// La langue de l'aide à montrer : celle de l'interface, sinon l'anglais.
    pub(super) fn help_language(&self) -> &'static str {
        help_locale(Localizer::new(self.chosen_locale).locale())
    }

    /// L'onglet qui montre déjà l'aide en `locale`.
    pub(super) fn tab_with_help(&self, locale: &str) -> Option<String> {
        self.tabs.iter().find(|(_, t)| t.info.help.as_deref() == Some(locale)).map(|(id, _)| id.clone())
    }

    /// Prépare l'onglet d'aide en `locale`, à la fin de `window`.
    pub(super) fn help_pending(&mut self, window: String, locale: &'static str) -> Pending {
        let mut pending = self.pending(Some(window), None, None);
        pending.help = Some(locale);
        pending
    }
}

#[cfg(test)]
#[path = "workspace_help_tests.rs"]
mod workspace_help_tests;
