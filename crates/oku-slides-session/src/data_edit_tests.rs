use oku_slides_chart::ColumnAlign;
use serde_json::json;
use speculoos::prelude::*;

use super::{align_patch, color_patch, stack_patch};

#[test]
fn align_patch_keeps_other_columns_and_trims_trailing_auto() {
    let option = json!({ "dataset": { "source": [["A", "B", "C"]] }, "align": ["end", "center"] });
    assert_that!(align_patch(&option, 1, ColumnAlign::Auto)).is_equal_to(json!({ "align": ["end"] }));
}

#[test]
fn align_patch_all_auto_is_null() {
    let option = json!({ "dataset": { "source": [["A"]] }, "align": ["end"] });
    assert_that!(align_patch(&option, 0, ColumnAlign::Auto)).is_equal_to(json!({ "align": null }));
}

#[test]
fn stack_patch_unstacked_drops_every_stack() {
    let option = json!({ "series": [{ "type": "bar", "stack": "a" }, { "type": "bar", "smooth": true }] });
    assert_that!(stack_patch(&option, false))
        .is_equal_to(json!({ "series": [{ "type": "bar" }, { "type": "bar", "smooth": true }] }));
}

#[test]
fn color_patch_touches_only_its_series() {
    let option = json!({ "series": [{ "type": "bar" }, { "type": "bar", "itemStyle": { "color": "#000000" } }] });
    assert_that!(color_patch(&option, 0, Some("#112233"))).is_equal_to(
        json!({ "series": [{ "type": "bar", "itemStyle": { "color": "#112233" } }, { "type": "bar", "itemStyle": { "color": "#000000" } }] }),
    );
}
