use std::sync::Arc;

use oku_slides_data::limits::{MAX_DATA, MAX_DECK_OVERLAY, MAX_DECK_ROW_IDS, MAX_DECK_TEXT_BYTES, MAX_ROWS};
use oku_slides_data::{Column, ColumnType, Data, DataCell, DataId, DataKind, Rows, Table};
use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::OpErrorKind;

/// A data named `d<i>` with `rows` rows, `overlay` number cells and a label of `label` bytes.
fn sized_data(i: usize, rows: usize, overlay: usize, label: usize) -> Arc<Data> {
    let cells = (0..overlay)
        .map(|r| (format!("r{r}/c").try_into().unwrap(), DataCell::Number(1.0)))
        .collect::<imbl::OrdMap<_, _>>();
    Arc::new(Data {
        name: format!("d{i}"),
        label: (label > 0).then(|| "x".repeat(label)),
        inline: false,
        kind: DataKind::Manual,
        table: Table {
            generation: "g".to_owned(),
            epoch: String::new(),
            columns: vec![Column { id: "c".to_owned(), name: String::new(), ty: ColumnType::Number }],
            rows: Rows::repaired((0..rows).map(|r| format!("r{r}"))),
            base: None,
            cells,
        },
    })
}

fn id(i: usize) -> DataId {
    format!("d{i:04}").try_into().unwrap()
}

/// A deck exactly at the bound `what`, plus `extra` past it.
fn deck_at(what: &str, extra: usize) -> Deck {
    let mut deck = Deck::default();
    let data: Vec<Arc<Data>> = match what {
        "data" => (0..MAX_DATA + extra).map(|i| sized_data(i, 0, 0, 0)).collect(),
        "row-ids" => {
            let mut all: Vec<_> = (0..MAX_DECK_ROW_IDS / MAX_ROWS).map(|i| sized_data(i, MAX_ROWS, 0, 0)).collect();
            all.push(sized_data(99, extra, 0, 0));
            all
        }
        "overlay" => vec![
            sized_data(0, 0, MAX_DECK_OVERLAY / 2, 0),
            sized_data(1, 0, MAX_DECK_OVERLAY - MAX_DECK_OVERLAY / 2 + extra, 0),
        ],
        _ => vec![sized_data(0, 0, 0, MAX_DECK_TEXT_BYTES - "d0".len() + extra)],
    };
    deck.data = data.into_iter().enumerate().map(|(i, d)| (id(i), d)).collect();
    deck
}

#[rstest]
#[case::data("data", MAX_DATA)]
#[case::row_ids("row-ids", MAX_DECK_ROW_IDS)]
#[case::overlay("overlay", MAX_DECK_OVERLAY)]
#[case::text("text", MAX_DECK_TEXT_BYTES)]
fn check_deck_bounds_at_bound_passes_and_past_it_names_the_bound(#[case] what: &str, #[case] max: usize) {
    assert_that!(check_deck_bounds(&deck_at(what, 0))).is_ok();
    let err = check_deck_bounds(&deck_at(what, 1)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::DataDeckFull);
    assert_that!(err.subject()).is_equal_to(Some(what));
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    assert_that!(oku_slides_i18n::Localize::localize(&err, &fr))
        .is_equal_to(fr.tr_with("op-data-deck-full", &[("what", what.into()), ("max", max.into())]));
}

/// A deck's load fits the deck bounds at each bound, not one past it.
#[rstest]
#[case::row_ids("row-ids")]
#[case::overlay("overlay")]
#[case::text("text")]
fn deck_load_fits_at_the_bound_and_not_past_it(#[case] what: &str) {
    assert_that!(DeckLoad::of_deck(&deck_at(what, 0)).fits()).is_true();
    assert_that!(DeckLoad::of_deck(&deck_at(what, 1)).fits()).is_false();
}

/// The load of two parts added is that of the deck holding both.
#[test]
fn deck_load_plus_sums_each_part() {
    let deck = deck_at("overlay", 0);
    let parts = deck.data.values().fold(DeckLoad::default(), |load, d| load.plus(DeckLoad::of(d)));
    assert_that!(parts).is_equal_to(DeckLoad::of_deck(&deck));
    assert_that!(DeckLoad::of_deck(&deck).fits()).is_true();
}
