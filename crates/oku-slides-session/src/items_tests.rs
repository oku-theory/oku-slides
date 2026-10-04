use serde_json::json;
use speculoos::prelude::*;

use crate::lib_tests_support::{add_frise, session_with_fixture};

#[test]
fn item_views_follow_list_order_with_bounds() {
    let mut s = session_with_fixture();
    s.apply("frise", vec![add_frise()], None).unwrap();
    let view = s.slide("s-constat").unwrap();
    let items: Vec<(&str, usize)> = view.items.iter().map(|i| (i.item, i.index)).collect();
    assert_that!(items).is_equal_to(vec![("d0", 0), ("d1", 1), ("d2", 2), ("d3", 3)]);
    let i = &view.items[0];
    assert_that!((i.owner, i.list, i.len, i.min, i.max, i.item_label, i.first_text)).is_equal_to((
        "fr",
        "etapes",
        4,
        0,
        Some(8),
        Some("étape"),
        Some("date"),
    ));
    assert_that!(i.w).is_greater_than(0.0);
}

#[test]
fn item_views_of_a_slide_without_list_are_empty() {
    let s = session_with_fixture();
    assert_that!(s.slide("s-priorites").unwrap().items.is_empty()).is_true();
}

#[test]
fn list_views_of_a_seeded_frise_count_its_items() {
    let mut s = session_with_fixture();
    s.apply("frise", vec![add_frise()], None).unwrap();
    let view = s.slide("s-constat").unwrap();
    assert_that!(view.lists.len()).is_equal_to(1);
    let l = &view.lists[0];
    assert_that!((l.owner, l.list, l.len, l.min, l.max, l.item_label, l.first_text)).is_equal_to((
        "fr",
        "etapes",
        4,
        0,
        Some(8),
        Some("étape"),
        Some("date"),
    ));
}

#[test]
fn list_views_of_an_emptied_frise_keep_the_list_with_zero_items() {
    let mut s = session_with_fixture();
    s.apply("frise", vec![add_frise()], None).unwrap();
    let empty: oku_slides_core::Op = serde_json::from_value(json!({
        "op": "set_param", "target": { "slide": "s-constat", "element": "fr" },
        "param": "etapes", "value": []
    }))
    .unwrap();
    s.apply("vider", vec![empty], None).unwrap();
    let view = s.slide("s-constat").unwrap();
    assert_that!(view.items.is_empty()).is_true();
    assert_that!(view.lists.len()).is_equal_to(1);
    assert_that!((view.lists[0].owner, view.lists[0].len)).is_equal_to(("fr", 0));
}

#[test]
fn list_views_of_a_slide_without_list_are_empty() {
    let s = session_with_fixture();
    assert_that!(s.slide("s-priorites").unwrap().lists.is_empty()).is_true();
}
