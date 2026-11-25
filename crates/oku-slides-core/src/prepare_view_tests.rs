use serde_json::json;
use speculoos::prelude::*;

use super::*;

#[test]
fn strip_inline_removes_dataset_series_and_axis_data_only() {
    let mut option = json!({ "data": "d", "dataset": { "source": [["A"], [1]] },
        "xAxis": { "type": "category", "data": ["a"] }, "yAxis": { "data": [1] },
        "series": [{ "type": "bar", "data": [1] }, { "type": "line" }] });
    strip_inline(&mut option);
    assert_that!(option).is_equal_to(json!({ "data": "d", "xAxis": { "type": "category" }, "yAxis": {},
        "series": [{ "type": "bar" }, { "type": "line" }] }));
    assert_that!(brings_inline(&option)).is_false();
}
