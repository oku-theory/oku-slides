use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::{PackFiles, PackKind, WidgetFile};

fn files(names: &[&str]) -> PackFiles {
    let widgets = names.iter().map(|n| (WidgetFile::parse(n).unwrap(), "x".to_owned())).collect();
    PackFiles::new(PackKind::Layouts, serde_json::Map::new(), widgets)
}

fn names(order: &[&str]) -> Vec<WidgetFile> {
    order.iter().map(|n| WidgetFile::parse(n).unwrap()).collect()
}

#[rstest]
#[case::empty_is_name_order(&[], &["a", "b", "c"])]
#[case::partial_then_name_order(&["c"], &["c", "a", "b"])]
#[case::full(&["b", "c", "a"], &["b", "c", "a"])]
#[case::missing_name_skipped(&["z", "b"], &["b", "a", "c"])]
#[case::repeat_kept_once(&["b", "a", "b"], &["b", "a", "c"])]
fn item_order_of_three_items_follows_the_list(#[case] order: &[&str], #[case] expected: &[&str]) {
    let f = files(&["_commun", "a", "b", "c"]);
    let order = names(order);
    let got: Vec<&str> = item_order(&order, &f).into_iter().map(WidgetFile::as_str).collect();
    assert_that!(got).is_equal_to(expected.to_vec());
}

#[rstest]
#[case::alphabetical_collapses(&["a", "b", "c"], &[])]
#[case::reordered_kept_whole(&["c", "a"], &["c", "a", "b"])]
#[case::missing_dropped(&["c", "gone", "a"], &["c", "a", "b"])]
fn normalized_order_of_three_items(#[case] order: &[&str], #[case] expected: &[&str]) {
    let f = files(&["a", "b", "c"]);
    assert_that!(normalized_order(&names(order), f.widgets().keys())).is_equal_to(names(expected));
}
