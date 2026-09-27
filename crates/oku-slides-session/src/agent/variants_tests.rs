use serde_json::{Value, json};
use speculoos::prelude::*;

use super::{MAX_TEMPLATE_DEPTH, fill_template};

#[test]
fn fill_template_replaces_every_value_marker() {
    let t = json!({ "op": "set_grid", "slide": "s-1", "grid": { "columns": "$value", "rows": 6 }, "tags": ["$value"] });
    assert_that!(fill_template(&t, &json!(8)))
        .is_equal_to(json!({ "op": "set_grid", "slide": "s-1", "grid": { "columns": 8, "rows": 6 }, "tags": [8] }));
}

#[test]
fn fill_template_leaves_other_strings() {
    let t = json!({ "op": "set_theme", "theme": "$value-x" });
    assert_that!(fill_template(&t, &json!("pro"))).is_equal_to(t);
}

/// `leaf` niché sous `n` tableaux imbriqués.
fn nested(n: usize, leaf: Value) -> Value {
    (0..n).fold(leaf, |v, _| json!([v]))
}

#[test]
fn fill_template_replaces_a_marker_within_the_depth_limit() {
    let n = MAX_TEMPLATE_DEPTH - 1;
    let t = nested(n, json!("$value"));
    assert_that!(fill_template(&t, &json!(1))).is_equal_to(nested(n, json!(1)));
}

#[test]
fn fill_template_leaves_a_marker_past_the_depth_limit_untouched() {
    let t = nested(MAX_TEMPLATE_DEPTH, json!("$value"));
    assert_that!(fill_template(&t, &json!(1))).is_equal_to(t);
}
