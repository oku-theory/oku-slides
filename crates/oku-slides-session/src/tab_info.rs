//! What the tab bar shows of a session: its title, dirty state, document and knowledge revision.

use crate::{Mode, Session, TabDocument, TabInfo};

impl Session {
    /// What the tab bar shows of this session.
    ///
    /// A pack tab reports its pack's name and a `Pack` (or `DeckPack`)
    /// document, a theme tab its theme's and a `Theme` document, a help tab a
    /// `Help` document. Otherwise, while history is open, the title and the
    /// file stay those of the stashed deck.
    pub fn tab_info(&self) -> TabInfo {
        if let Some(info) = self.pack_tab_info().or_else(|| self.theme_tab_info()) {
            return info;
        }
        let deck = match &self.mode {
            Mode::Deck | Mode::Help { .. } | Mode::Theme { .. } => &self.doc,
            Mode::Pack { preview: stash, .. } | Mode::History { stash, .. } => stash,
        };
        let document = match self.help() {
            Some(locale) => TabDocument::Help { locale: locale.to_owned() },
            None => TabDocument::Deck {
                path: deck.path.as_deref().map(oku_slides_platform::storage::resolve),
                deck_uid: Some(deck.sources.replica().deck_uid()),
            },
        };
        TabInfo {
            title: self.document_name(deck),
            dirty: self.unsaved(),
            dirty_revision: self.dirty_revision(),
            pristine: matches!(self.mode, Mode::Deck)
                && deck.blank
                && deck.path.is_none()
                && !deck.dirty
                && !deck.sources.replica().can_undo()
                && !deck.sources.replica().can_redo(),
            theme: {
                let user = deck.sources.deck();
                user.theme.as_ref().and_then(|ns| user.themes.get(ns)).map(|pin| pin.id)
            },
            document,
            created: None,
            knowledge: self.help().is_none().then(|| self.knowledge_revision(deck)),
        }
    }
}
