use serde_json::json;
use speculoos::prelude::*;

use super::appearance;

#[test]
fn appearance_app_dark_tokens_become_the_dark_variant() {
    let tokens = json!({
        "color": {"ink": "#2e2b26", "bg": "#fffdf9"},
        "app": {"scheme": "light", "desk": "#ece8e0"},
        "app-dark": {"desk": "#1c1a17", "ink": "bg", "radius-sm": 4},
    });
    let a = appearance(&tokens);
    let dark = &a.variants["dark"];
    assert_that!(dark.get("desk").map(String::as_str)).is_equal_to(Some("#1c1a17"));
    assert_that!(dark.get("ink").map(String::as_str)).is_equal_to(Some("#fffdf9"));
    assert_that!(dark.contains_key("radius-sm")).is_false();
}

#[test]
fn appearance_without_variant_tokens_has_no_variants() {
    let tokens = json!({"color": {"ink": "#2e2b26"}, "app": {"scheme": "light", "desk": "#ece8e0"}});
    assert_that!(appearance(&tokens).variants.is_empty()).is_true();
}

#[test]
fn appearance_variant_of_the_theme_own_scheme_is_ignored() {
    let tokens = json!({"color": {}, "app": {"scheme": "dark"}, "app-dark": {"desk": "#111111"}});
    assert_that!(appearance(&tokens).variants.contains_key("dark")).is_false();
}
