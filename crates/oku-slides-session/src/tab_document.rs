//! What a tab edits: a deck, the shipped help, a pack or theme, or a deck's own pack.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::DocumentKind;

/// What a tab edits; its kind decides the pile, the tools and the border.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TabDocument {
    /// A deck, saved to `path` once it has a file.
    Deck {
        /// The deck's file, absolute and normalized.
        path: Option<PathBuf>,
        /// The deck's identity (its replica), known without waking the tab.
        deck_uid: Option<String>,
    },
    /// The shipped help, in this locale.
    Help {
        /// The help's locale.
        locale: String,
    },
    /// A pack of the library, edited in its own tab.
    Pack {
        /// The edited pack.
        pack: Uuid,
    },
    /// A theme of the library, edited in its own tab.
    Theme {
        /// The edited theme.
        theme: Uuid,
    },
    /// A pack embedded only in one deck, edited in its own tab.
    DeckPack {
        /// The owner deck's tab id.
        deck_tab: String,
        /// The edited pack's id.
        pack: Uuid,
    },
}

impl TabDocument {
    /// The kind of session this document opens in.
    #[must_use]
    pub fn kind(&self) -> DocumentKind {
        match self {
            Self::Deck { .. } => DocumentKind::Deck,
            Self::Help { .. } => DocumentKind::Help,
            Self::Pack { .. } | Self::DeckPack { .. } => DocumentKind::Pack,
            Self::Theme { .. } => DocumentKind::Theme,
        }
    }

    /// The deck's file, for a deck tab that has one.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Deck { path, .. } => path.as_deref(),
            Self::Help { .. } | Self::Pack { .. } | Self::Theme { .. } | Self::DeckPack { .. } => None,
        }
    }

    /// The deck's identity, for a deck tab.
    #[must_use]
    pub fn deck_uid(&self) -> Option<&str> {
        match self {
            Self::Deck { deck_uid, .. } => deck_uid.as_deref(),
            Self::Help { .. } | Self::Pack { .. } | Self::Theme { .. } | Self::DeckPack { .. } => None,
        }
    }

    /// The help's locale, for a help tab.
    #[must_use]
    pub fn help_locale(&self) -> Option<&str> {
        match self {
            Self::Help { locale } => Some(locale),
            Self::Deck { .. } | Self::Pack { .. } | Self::Theme { .. } | Self::DeckPack { .. } => None,
        }
    }

    /// The owner deck's tab, for a deck-pack tab.
    #[must_use]
    pub fn deck_tab(&self) -> Option<&str> {
        match self {
            Self::DeckPack { deck_tab, .. } => Some(deck_tab),
            Self::Deck { .. } | Self::Help { .. } | Self::Pack { .. } | Self::Theme { .. } => None,
        }
    }
}

#[cfg(test)]
#[path = "tab_document_tests.rs"]
mod tab_document_tests;
