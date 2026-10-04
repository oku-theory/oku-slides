//! Aide de l'app : un deck livré avec elle, montré en lecture seule.
//!
//! Le deck d'aide (`docs/aide/<langue>.json`) est compilé dans l'app. Une
//! session qui le montre refuse toute modification (`ReadOnly`), de
//! l'interface comme de l'agent ; sa reprise ne garde que la langue et
//! relit le deck livré, jamais une ancienne copie. Il se rend avec la
//! version livrée du pack par défaut, pas celle que l'utilisateur a pu modifier.

use oku_slides_core::{Deck, PackRef, SourceBundle};
use snafu::{OptionExt, ResultExt};

use crate::error::{HelpDeckSnafu, ReadOnlySnafu, SourcesSnafu, UnknownPackSnafu};
use crate::library::builtin_versions;
use crate::pack::DEFAULT_PACK;
use crate::{Result, Session};

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

/// Le deck d'aide d'une langue livrée (voir [`help_locale`]).
pub(crate) fn help_deck(locale: &str) -> Result<Deck> {
    let (_, json) = HELP_DECKS.iter().find(|(l, _)| *l == help_locale(locale)).unwrap_or(&HELP_DECKS[0]);
    Ok(Deck::from_json(json).context(HelpDeckSnafu)?)
}

impl Session {
    /// Montre l'aide dans la langue `locale` (sinon en anglais), en lecture seule.
    ///
    /// Pour une session neuve (onglet d'aide, reprise) : le deck ouvert est
    /// remplacé sans question.
    pub fn open_help(&mut self, locale: &str) -> Result<()> {
        let locale = help_locale(locale);
        let builtin = builtin_versions()
            .iter()
            .find(|v| v.manifest.id == DEFAULT_PACK)
            .context(UnknownPackSnafu { id: DEFAULT_PACK })?;
        let mut sources = SourceBundle::new(help_deck(locale)?, self.registry.clone()).context(SourcesSnafu)?;
        let version = sources.add_pack(builtin.files.clone());
        let mut deck = sources.deck().clone();
        deck.templates = Some(PackRef { id: DEFAULT_PACK, version });
        sources.apply_silent(deck).context(SourcesSnafu)?;
        self.load(sources, None)?;
        self.help = Some(locale);
        Ok(())
    }

    /// Refuse (`ReadOnly`) ce qui changerait l'aide.
    pub(crate) fn ensure_not_help(&self) -> Result<()> {
        snafu::ensure!(self.help.is_none(), ReadOnlySnafu);
        Ok(())
    }
}

#[cfg(test)]
#[path = "help_tests.rs"]
mod help_tests;
