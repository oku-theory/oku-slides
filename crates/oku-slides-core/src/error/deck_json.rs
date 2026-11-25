//! A deck's JSON refused by [`Deck::from_json`](crate::Deck::from_json).

use oku_slides_i18n::{Localize, Localizer};
use snafu::{Backtrace, Snafu};

use super::OpError;

/// A deck's JSON that does not read as a deck.
#[derive(Debug, Snafu)]
pub struct DeckJsonError(DeckJsonInner);

/// Nature of a [`DeckJsonError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeckJsonErrorKind {
    /// Over its byte cap, refused before parsing.
    TooLarge,
    /// Invalid JSON, not the shape of a deck, or a data bound passed while parsing.
    Unreadable,
    /// Another deck version.
    UnsupportedVersion,
    /// Over a bound on the deck's data (see [`crate::check_deck_bounds`]).
    DeckFull,
    /// A chart or table stores a `dataset`: decks keep their data apart.
    InlineData,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum DeckJsonInner {
    #[snafu(display("deck JSON over {max} bytes"))]
    #[snafu(context(name(DeckJsonTooLargeSnafu)))]
    TooLarge { max: usize, backtrace: Backtrace },
    #[snafu(display("unreadable deck JSON"))]
    #[snafu(context(name(DeckJsonUnreadableSnafu)))]
    Unreadable { source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("unsupported deck version {found}"))]
    #[snafu(context(name(DeckJsonVersionSnafu)))]
    Version { found: u32, backtrace: Backtrace },
    #[snafu(display("deck over its data bounds"))]
    #[snafu(context(name(DeckJsonFullSnafu)))]
    Full { source: OpError, backtrace: Backtrace },
    #[snafu(display("{element} stores an inline dataset"))]
    #[snafu(context(name(DeckJsonInlineSnafu)))]
    InlineData { element: String, backtrace: Backtrace },
}

impl DeckJsonError {
    /// Kind of the error, to handle it without reading the message.
    #[must_use]
    pub fn kind(&self) -> DeckJsonErrorKind {
        match self.0 {
            DeckJsonInner::TooLarge { .. } => DeckJsonErrorKind::TooLarge,
            DeckJsonInner::Unreadable { .. } => DeckJsonErrorKind::Unreadable,
            DeckJsonInner::Version { .. } => DeckJsonErrorKind::UnsupportedVersion,
            DeckJsonInner::Full { .. } => DeckJsonErrorKind::DeckFull,
            DeckJsonInner::InlineData { .. } => DeckJsonErrorKind::InlineData,
        }
    }
}

impl Localize for DeckJsonError {
    fn localize(&self, l: &Localizer) -> String {
        match &self.0 {
            DeckJsonInner::TooLarge { .. } => l.tr("sources-too-large"),
            DeckJsonInner::Unreadable { .. } => l.tr("sources-unreadable"),
            DeckJsonInner::Version { .. } => l.tr("replica-unsupported-version"),
            DeckJsonInner::Full { source, .. } => source.localize(l),
            DeckJsonInner::InlineData { element, .. } => {
                l.tr_with("deck-inline-data", &[("element", oku_slides_data::quoted(element).into())])
            }
        }
    }
}
