use speculoos::prelude::*;

use super::{NOTES_MIN, PILE_MAX, PILE_MIN};

/// The static layout bounds, shared with the page's `columns.ts` (t-1001).
const FIXTURE: &str = include_str!("../../../fixtures/ui/layout-bounds.json");

#[test]
fn layout_bounds_fixture_matches_rust() {
    let fixture: serde_json::Value =
        serde_json::from_str(FIXTURE).unwrap_or_else(|e| panic!("layout-bounds.json : {e}"));
    let rust = serde_json::json!({ "pile_min": PILE_MIN, "pile_max": PILE_MAX, "notes_min": NOTES_MIN });
    assert_that!(fixture).is_equal_to(rust);
}
