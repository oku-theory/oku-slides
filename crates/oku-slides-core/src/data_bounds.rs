//! Bounds on all the data of a deck, past those of each data.

use oku_slides_data::Data;
use oku_slides_data::limits::{MAX_DATA, MAX_DECK_OVERLAY, MAX_DECK_ROW_IDS, MAX_DECK_TEXT_BYTES};
use snafu::ensure;

use crate::error::DataDeckFullSnafu;
use crate::{Deck, OpError};

/// What a deck's data weigh against the deck bounds (the data count apart, [`MAX_DATA`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DeckLoad {
    /// Row ids.
    pub row_ids: usize,
    /// Overlay entries.
    pub overlay: usize,
    /// UTF-8 bytes of overlay text, names, labels and column names.
    pub text: usize,
}

impl DeckLoad {
    /// What `data` weighs.
    #[must_use]
    pub fn of(data: &Data) -> DeckLoad {
        DeckLoad {
            row_ids: data.table.rows.len(),
            overlay: data.table.overlay_len(),
            text: data.table.text_bytes() + data.name.len() + data.label.as_ref().map_or(0, String::len),
        }
    }

    /// The deck bounds, as a load.
    pub(crate) const MAX: DeckLoad =
        DeckLoad { row_ids: MAX_DECK_ROW_IDS, overlay: MAX_DECK_OVERLAY, text: MAX_DECK_TEXT_BYTES };

    /// What is left of the deck bounds once `self` is used.
    pub(crate) fn left(self) -> DeckLoad {
        DeckLoad {
            row_ids: Self::MAX.row_ids.saturating_sub(self.row_ids),
            overlay: Self::MAX.overlay.saturating_sub(self.overlay),
            text: Self::MAX.text.saturating_sub(self.text),
        }
    }

    /// What all the data of `deck` weigh.
    #[must_use]
    pub fn of_deck(deck: &Deck) -> DeckLoad {
        deck.data.values().fold(DeckLoad::default(), |load, data| load.plus(DeckLoad::of(data)))
    }

    /// Whether `self` is within the deck bounds.
    #[must_use]
    pub fn fits(self) -> bool {
        self.check().is_ok()
    }

    /// The sum of `self` and `more`.
    #[must_use]
    pub fn plus(self, more: DeckLoad) -> DeckLoad {
        DeckLoad {
            row_ids: self.row_ids.saturating_add(more.row_ids),
            overlay: self.overlay.saturating_add(more.overlay),
            text: self.text.saturating_add(more.text),
        }
    }

    fn check(self) -> Result<(), OpError> {
        ensure!(self.row_ids <= MAX_DECK_ROW_IDS, DataDeckFullSnafu { what: "row-ids", max: MAX_DECK_ROW_IDS });
        ensure!(self.overlay <= MAX_DECK_OVERLAY, DataDeckFullSnafu { what: "overlay", max: MAX_DECK_OVERLAY });
        ensure!(self.text <= MAX_DECK_TEXT_BYTES, DataDeckFullSnafu { what: "text", max: MAX_DECK_TEXT_BYTES });
        Ok(())
    }
}

/// Checks the deck bounds: at most [`MAX_DATA`] data, [`MAX_DECK_ROW_IDS`] row ids,
/// [`MAX_DECK_OVERLAY`] overlay entries and [`MAX_DECK_TEXT_BYTES`] of text over all its data.
///
/// # Errors
///
/// `op-data-deck-full` naming the first bound passed (`data`, `row-ids`, `overlay`, `text`).
pub fn check_deck_bounds(deck: &Deck) -> Result<(), OpError> {
    ensure!(deck.data.len() <= MAX_DATA, DataDeckFullSnafu { what: "data", max: MAX_DATA });
    DeckLoad::of_deck(deck).check()
}

#[cfg(test)]
#[path = "data_bounds_tests.rs"]
mod data_bounds_tests;
