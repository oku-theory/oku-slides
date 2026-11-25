use oku_slides_core::kinds::{base, essentiel};
use oku_slides_data::limits::MAX_DECK_OVERLAY;
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::test_support::{element_with, registry};

/// A content slide whose blocks are a table `t` of 4 rows of 3 numbers, in a stack `g`.
fn slide() -> Slide {
    let rows: Vec<Value> = (0..4).map(|i| json!([i, i, i])).collect();
    let mut source = vec![json!(["a", "b", "c"])];
    source.extend(rows);
    let table = element_with("t", base::TABLE, "data", json!({ "dataset": { "source": source } }));
    let mut stack = Element::new("g", base::STACK);
    stack.params.set("children", ParamValue::Slot(vec![table]));
    let mut slide = Slide::new("s1", essentiel::CONTENT);
    slide.params.set("blocks", ParamValue::Slot(vec![stack]));
    slide
}

/// The table option of [`slide`], if left.
fn option(slide: &Slide) -> Option<&Value> {
    let stack = &slide.params.slot("blocks").unwrap()[0];
    stack.params.slot("children").unwrap()[0].params.value("data")
}

/// With `overlay` cells left and `data` data in the deck, the table keeps `kept` rows (`None`:
/// no view left), and is reported unless whole.
#[rstest]
#[case::whole(MAX_DECK_OVERLAY, 0, Some(4))]
#[case::cut(5, 0, Some(1))]
#[case::no_row(2, 0, None)]
#[case::max_data(MAX_DECK_OVERLAY, MAX_DATA, None)]
fn fit_keeps_the_rows_left_and_reports_a_cut(#[case] left: usize, #[case] data: usize, #[case] kept: Option<usize>) {
    let used = DeckLoad { row_ids: 0, overlay: MAX_DECK_OVERLAY - left, text: 0 };
    let mut budget = DeckBudget { used, data };
    let mut slide = slide();
    let cut = budget.fit(&registry(), &mut slide);
    let rows = option(&slide).map(|o| o["dataset"]["source"].as_array().unwrap().len() - 1);
    assert_that!(rows).is_equal_to(kept);
    let expected = if kept == Some(4) { Vec::new() } else { vec![Some("t".to_owned())] };
    assert_that!(cut).is_equal_to(expected);
    let counted = usize::from(kept.is_some_and(|k| k > 0));
    assert_that!(budget.data).is_equal_to(data + counted);
    assert_that!(budget.used.overlay).is_equal_to(MAX_DECK_OVERLAY - left + 3 * kept.unwrap_or(0));
}

/// The budget of a deck starts from its data.
#[test]
fn of_counts_the_deck_data() {
    let budget = DeckBudget::of(&Deck::empty());
    assert_that!((budget.used, budget.data)).is_equal_to((DeckLoad::default(), 0));
}
