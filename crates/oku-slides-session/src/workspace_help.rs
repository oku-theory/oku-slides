//! L'onglet d'aide : un seul par langue, rouvert plutôt que dupliqué, au
//! thème du deck d'où on l'ouvre.

use std::sync::Mutex;

use uuid::Uuid;

use super::{Opening, Pending, TabPlace, Workspace};
use crate::help::help_locale;

impl Workspace {
    /// La langue de l'aide à montrer : celle de l'interface, sinon l'anglais.
    pub(super) fn help_language(&self) -> &'static str {
        help_locale(oku_slides_i18n::resolve(self.chosen_locale))
    }

    /// The tab that already shows the help in `locale`.
    pub(super) fn tab_with_help(&self, locale: &str) -> Option<String> {
        self.tabs.iter().find(|(_, t)| t.info.document.help_locale() == Some(locale)).map(|(id, _)| id.clone())
    }

    /// The theme of the deck in `window`'s active tab; `None`: the help itself, or no theme.
    pub(super) fn help_theme(&self, window: &str) -> Option<Uuid> {
        let active = &self.layout.windows.iter().find(|w| w.label == window)?.active;
        let info = &self.tabs.get(active)?.info;
        info.document.help_locale().is_none().then_some(info.theme).flatten()
    }

    /// Prépare l'onglet d'aide en `locale`, au thème `theme`, à la fin de `window`.
    pub(super) fn help_pending(&mut self, window: String, locale: &'static str, theme: Option<Uuid>) -> Pending {
        self.pending(Some(window), Opening::Help { locale, theme }, TabPlace::End)
    }

    /// L'onglet d'aide `tab` prend le thème `theme` ; un échec est journalisé.
    pub(super) fn retheme_help(ws: &Mutex<Self>, tab: &str, theme: Uuid) {
        match Self::with_session(ws, tab, |s| s.set_help_theme(theme)).0 {
            Ok(Ok(())) => {}
            Ok(Err(e)) | Err(e) => tracing::warn!(tab, %theme, error = %e.one_line(), "thème de l'aide inchangé"),
        }
    }
}

#[cfg(test)]
#[path = "workspace_help_tests.rs"]
mod workspace_help_tests;
