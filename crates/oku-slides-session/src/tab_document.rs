//! What a tab edits: a deck, the shipped help, or a pack or theme of the library.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
}

impl TabDocument {
    /// The deck's file, for a deck tab that has one.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Deck { path, .. } => path.as_deref(),
            Self::Help { .. } | Self::Pack { .. } | Self::Theme { .. } => None,
        }
    }

    /// The deck's identity, for a deck tab.
    #[must_use]
    pub fn deck_uid(&self) -> Option<&str> {
        match self {
            Self::Deck { deck_uid, .. } => deck_uid.as_deref(),
            Self::Help { .. } | Self::Pack { .. } | Self::Theme { .. } => None,
        }
    }

    /// The help's locale, for a help tab.
    #[must_use]
    pub fn help_locale(&self) -> Option<&str> {
        match self {
            Self::Help { locale } => Some(locale),
            Self::Deck { .. } | Self::Pack { .. } | Self::Theme { .. } => None,
        }
    }
}

#[cfg(test)]
#[path = "tab_document_tests.rs"]
mod tab_document_tests;
