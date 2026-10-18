//! Aide de l'app : un deck livré avec elle, montré en lecture seule.
//!
//! Le deck d'aide (`docs/aide/<langue>.json`) est compilé dans l'app. Une
//! session qui le montre refuse toute modification (`ReadOnly`), de
//! l'interface comme de l'agent ; sa reprise ne garde que la langue et
//! relit le deck livré, jamais une ancienne copie. Il se rend avec la
//! version livrée du pack par défaut, pas celle que l'utilisateur a pu modifier,
//! et au thème du deck d'où on l'ouvre (version courante de la bibliothèque,
//! embarquée), sinon au thème par défaut. Le JSON livré ne porte pas de thème.

use std::borrow::Cow;

use oku_slides_core::{Deck, PackPin, SourceBundle, ThemeRef};
use oku_slides_i18n::platform_keys;
use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use crate::error::{
    EditingTemplatesSnafu, HelpDeckSnafu, NotHelpSnafu, ReadOnlySnafu, SourcesSnafu, UnknownPackSnafu,
    UnknownThemeSnafu,
};
use crate::library::builtin_versions;
use crate::pack::DEFAULT_PACK;
use crate::{Mode, Result, Session};

/// Les decks d'aide livrés, par langue ; le premier sert aux autres langues.
const HELP_DECKS: [(&str, &str); 2] =
    [("en", include_str!("../../../docs/aide/en.json")), ("fr", include_str!("../../../docs/aide/fr.json"))];

/// Langues dont l'aide est livrée.
#[cfg(test)]
pub(crate) fn help_locales() -> impl Iterator<Item = &'static str> {
    HELP_DECKS.iter().map(|(l, _)| *l)
}

/// La langue de l'aide montrée pour `locale` : la sienne, sinon l'anglais.
pub(crate) fn help_locale(locale: &str) -> &'static str {
    HELP_DECKS.iter().find(|(l, _)| *l == locale).unwrap_or(&HELP_DECKS[0]).0
}

/// Le deck d'aide d'une langue livrée (voir [`help_locale`]), ses raccourcis
/// dans la notation de la plateforme.
pub(crate) fn help_deck(locale: &str) -> Result<Deck> {
    help_deck_with(locale, platform_keys)
}

/// Le deck d'aide, ses raccourcis (écrits à la macOS) passés par `keys`.
pub(crate) fn help_deck_with(locale: &str, keys: fn(&str) -> Cow<'_, str>) -> Result<Deck> {
    let (_, json) = HELP_DECKS.iter().find(|(l, _)| *l == help_locale(locale)).unwrap_or(&HELP_DECKS[0]);
    // Les symboles des touches n'apparaissent que dans les textes du JSON.
    Ok(Deck::from_json(&keys(json)).context(HelpDeckSnafu)?)
}

impl Session {
    /// Montre l'aide dans la langue `locale` (sinon en anglais), en lecture
    /// seule, au thème `theme` (id du thème du deck d'où on l'ouvre ; inconnu
    /// ou absent : le thème par défaut).
    ///
    /// Pour une session neuve (onglet d'aide, reprise) : le deck ouvert est
    /// remplacé sans question.
    pub fn open_help(&mut self, locale: &str, theme: Option<Uuid>) -> Result<()> {
        // Le deck mis de côté par l'édition des modèles serait perdu.
        ensure!(!matches!(self.mode, Mode::Templates { .. }), EditingTemplatesSnafu);
        let locale = help_locale(locale);
        let builtin = builtin_versions()
            .iter()
            .find(|v| v.manifest.id == DEFAULT_PACK)
            .context(UnknownPackSnafu { id: DEFAULT_PACK })?;
        let mut sources = SourceBundle::new(help_deck(locale)?, self.registry.clone()).context(SourcesSnafu)?;
        let version = sources.add_pack(builtin.files.clone());
        let mut deck = sources.deck().clone();
        let pin = PackPin { id: DEFAULT_PACK, version, requires: builtin.manifest.requires.clone() };
        deck.widgets.insert(builtin.manifest.namespace.clone(), pin);
        if let Some(current) = theme.and_then(|id| self.themes.current_version(id)) {
            let id = current.manifest.id;
            deck.theme = Some(ThemeRef { id, version: sources.add_theme(current.files) });
        }
        sources.apply_silent(deck).context(SourcesSnafu)?;
        self.load(sources, None)?;
        self.mode = Mode::Help { locale };
        Ok(())
    }

    /// L'aide prend le thème `id` (celui du deck d'où on la rouvre), dans sa
    /// version courante, embarquée.
    ///
    /// Ni annulable ni à enregistrer : l'aide reste en lecture seule. Hors de
    /// l'aide, ou thème inconnu de la bibliothèque : refusé.
    pub fn set_help_theme(&mut self, id: Uuid) -> Result<()> {
        ensure!(self.help().is_some(), NotHelpSnafu);
        let current = self.themes.current_version(id).context(UnknownThemeSnafu { id })?;
        let mut deck = self.doc.sources.deck().clone();
        if deck.theme.as_ref().is_some_and(|t| t.id == id && t.version == current.version) {
            return Ok(());
        }
        deck.theme = Some(ThemeRef { id, version: self.doc.sources.add_theme(current.files) });
        self.doc.sources.apply_silent(deck).context(SourcesSnafu)?;
        self.rerender()?;
        self.persist(false);
        Ok(())
    }

    /// La langue de l'aide montrée ; `None` : pas l'aide.
    pub(crate) fn help(&self) -> Option<&'static str> {
        match self.mode {
            Mode::Help { locale } => Some(locale),
            Mode::Deck | Mode::Templates { .. } | Mode::History { .. } => None,
        }
    }

    /// Refuse (`ReadOnly`) ce qui changerait l'aide.
    pub(crate) fn ensure_not_help(&self) -> Result<()> {
        snafu::ensure!(self.help().is_none(), ReadOnlySnafu);
        Ok(())
    }
}

#[cfg(test)]
#[path = "help_tests.rs"]
mod help_tests;
