//! The kind of document a session holds, and the requests each kind refuses.
//!
//! A theme tab accepts only the requests [`Request::theme_tab_allows`]
//! lists; the theme draft requests make sense only there. Any other
//! request that does not fit is refused with `WrongDocument`, naming the
//! session's kind.

use oku_slides_core::Op;
use serde::Serialize;
use snafu::ensure;

use crate::error::WrongDocumentSnafu;
use crate::theme_edit::{DRAFT_SWITCH_VAR, is_token_op};
use crate::{Mode, Request, Result, Session};

/// The kind of document a session edits or shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentKind {
    /// A deck, its history included.
    Deck,
    /// The shipped help.
    Help,
    /// A pack, in a deck's workshop or its own tab.
    Pack,
    /// A library theme, in its own tab.
    Theme,
}

impl DocumentKind {
    /// The kind's name, as serde writes it and Fluent selects on.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Deck => "deck",
            Self::Help => "help",
            Self::Pack => "pack",
            Self::Theme => "theme",
        }
    }
}

impl Request {
    /// True for a request that edits or saves a theme tab's draft.
    fn drafts_theme(&self) -> bool {
        matches!(
            self,
            Self::HostToken { .. }
                | Self::RenameEditedTheme { .. }
                | Self::SetEditedThemeExtends { .. }
                | Self::EditThemeExtends { .. }
                | Self::SaveTheme { .. }
        )
    }
}

/// The refusal of what a theme tab does not do (`WrongDocument`).
pub(crate) fn refused_in_theme_tab<T>() -> Result<T> {
    Ok(WrongDocumentSnafu { kind: DocumentKind::Theme }.fail()?)
}

impl Session {
    /// The kind of document this session holds; history browses a deck.
    #[must_use]
    pub fn document_kind(&self) -> DocumentKind {
        match &self.mode {
            Mode::Deck | Mode::History { .. } => DocumentKind::Deck,
            Mode::Help { .. } => DocumentKind::Help,
            Mode::Pack { .. } => DocumentKind::Pack,
            Mode::Theme { .. } => DocumentKind::Theme,
        }
    }

    /// Refuses `req` when it makes no sense for this session's kind (`WrongDocument`).
    ///
    /// An `apply` is refused too when one of its ops does not fit (`ensure_caller_ops`).
    pub(crate) fn ensure_fits(&self, req: &Request) -> Result<()> {
        let kind = self.document_kind();
        let fits = match kind {
            DocumentKind::Theme => req.theme_tab_allows(),
            DocumentKind::Deck | DocumentKind::Help | DocumentKind::Pack => !req.drafts_theme(),
        };
        ensure!(fits, WrongDocumentSnafu { kind });
        // Before any lease is taken: a refused op takes nothing and freezes nothing.
        if let Request::Apply { ops, .. } = req {
            self.ensure_caller_ops(ops)?;
        }
        Ok(())
    }

    /// Refuses caller ops that do not fit this session's kind (`WrongDocument`).
    ///
    /// A theme tab takes only token ops; a pack workshop neither token ops
    /// nor a theme choice, its theme being the deck's. No caller writes the
    /// draft's rank ([`DRAFT_SWITCH_VAR`]): only the theme tab's rank steps do.
    pub(crate) fn ensure_caller_ops(&self, ops: &[Op]) -> Result<()> {
        let kind = self.document_kind();
        let fits = |op: &Op| match op {
            Op::SetVar { key, .. } if key == DRAFT_SWITCH_VAR => false,
            _ => match kind {
                DocumentKind::Theme => is_token_op(op),
                DocumentKind::Pack => !is_token_op(op) && !matches!(op, Op::SetTheme { .. }),
                DocumentKind::Deck | DocumentKind::Help => true,
            },
        };
        ensure!(ops.iter().all(fits), WrongDocumentSnafu { kind });
        Ok(())
    }

    /// Refuses a document change where `ensure_editable` would, a theme tab's draft aside.
    ///
    /// A theme tab's specimen deck changes only through its draft: the token
    /// ops `ensure_caller_ops` kept, the rank steps, and their undo.
    pub(crate) fn ensure_document_editable(&self) -> Result<()> {
        if self.document_kind() == DocumentKind::Theme { Ok(()) } else { self.ensure_editable() }
    }

    /// Refuses outside a theme tab (`WrongDocument`, naming this session's kind).
    pub(crate) fn ensure_theme_tab(&self) -> Result<()> {
        let kind = self.document_kind();
        ensure!(kind == DocumentKind::Theme, WrongDocumentSnafu { kind });
        Ok(())
    }
}

#[cfg(test)]
#[path = "document_kind_tests.rs"]
mod document_kind_tests;
