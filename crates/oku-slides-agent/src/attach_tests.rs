use oku_slides_i18n::{Localize, Localizer};
use speculoos::prelude::*;

use super::distinct_within;

fn ids(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| (*n).to_owned()).collect()
}

#[test]
fn resolve_attachments_refuses_past_max_before_lookup() {
    let many: Vec<String> = (0..100_000).map(|n| format!("sha256:{n:064x}")).collect();
    let e = distinct_within(&many, 12).unwrap_err();
    let fr = Localizer::new(Some("fr"));
    let expected = fr.tr_with("agent-error-too-many-attachments", &[("max", 12u64.into())]);
    assert_that!(e.localize(&fr)).is_equal_to(expected);
}

#[test]
fn distinct_within_drops_duplicates_in_order() {
    let given = ids(&["b", "a", "b", "a", "c"]);
    let unique = distinct_within(&given, 3).unwrap();
    assert_that!(unique.iter().map(|s| s.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["b", "a", "c"]);
}

#[test]
fn distinct_within_duplicates_do_not_count_toward_max() {
    let given = ids(&["a", "a", "a", "b", "b"]);
    assert_that!(distinct_within(&given, 2)).is_ok();
    assert_that!(distinct_within(&given, 1)).is_err();
}
