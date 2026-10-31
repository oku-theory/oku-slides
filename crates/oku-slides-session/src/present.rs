//! Whether a tab can present its deck, and why not.
//!
//! The one presentation rule: the page's Present (menu, ⌘⇧↩) reads it through
//! `Snapshot.can_present`, the agent's `present` tool through [`Session::can_present`].

use crate::error::CannotPresentSnafu;
use crate::{Mode, Result, Session};

/// Why a tab cannot present its deck (error [`crate::SessionErrorKind::CannotPresent`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PresentRefusal {
    /// The help is read, never presented.
    Help,
    /// A pack tab shows its widgets and layouts, not a deck.
    Pack,
    /// A theme tab shows its specimen deck, not a deck.
    Theme,
    /// History open: it shows a past version.
    History,
    /// The deck has no slide.
    NoSlide,
}

impl PresentRefusal {
    /// The Fluent key of the refusal.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Help => "session-present-help",
            Self::Pack => "session-present-pack",
            Self::Theme => "session-present-theme",
            Self::History => "session-present-history",
            Self::NoSlide => "session-present-no-slide",
        }
    }
}

impl Session {
    /// Can this tab present its deck? If not, why.
    ///
    /// Only a deck tab with at least one slide presents; the page's open text edit is its own
    /// to commit first.
    ///
    /// # Errors
    ///
    /// `CannotPresent`, its [`PresentRefusal`] naming the cause: help, pack or theme tab,
    /// history open, or no slide.
    pub fn can_present(&self) -> Result<()> {
        let refusal = match self.mode {
            Mode::Help { .. } => PresentRefusal::Help,
            Mode::Pack { .. } => PresentRefusal::Pack,
            Mode::Theme { .. } => PresentRefusal::Theme,
            Mode::History { .. } => PresentRefusal::History,
            Mode::Deck if self.deck().slides.is_empty() => PresentRefusal::NoSlide,
            Mode::Deck => return Ok(()),
        };
        Ok(CannotPresentSnafu { reason: refusal }.fail()?)
    }
}

#[cfg(test)]
#[path = "present_tests.rs"]
mod present_tests;
