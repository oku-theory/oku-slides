use serde_json::json;
use speculoos::prelude::*;

use super::{chart_palette, resolve_color};

#[test]
fn chart_palette_follows_color_names_and_drops_the_rest() {
    let tokens = json!({
        "color": { "accent": "#3F6B5E", "ink": "#2e2b26", "muted": "ink" },
        "chart": { "palette": ["accent", "#C4622D", "muted", "unknown", 5, "#12345"] }
    });
    assert_that!(chart_palette(&tokens)).is_equal_to(vec![
        "#3f6b5e".to_owned(),
        "#c4622d".to_owned(),
        "#2e2b26".to_owned(),
    ]);
}

#[test]
fn chart_palette_without_chart_tokens_is_empty() {
    assert_that!(chart_palette(&json!({ "color": { "accent": "#000000" } }))).is_empty();
}

#[test]
fn resolve_color_stops_on_reference_loops() {
    let colors = json!({ "a": "b", "b": "a" });
    assert_that!(resolve_color(colors.as_object().unwrap(), &json!("a"))).is_none();
}

/// t-128 : l'interface met la palette dans un attribut `style`. Rien d'autre
/// qu'un `#rgb` ou `#rrggbb` ne doit en sortir, même via un nom de couleur.
#[test]
fn chart_palette_rejects_everything_but_hex_colors() {
    for bad in [
        "#fff;background:url(https://x)",
        "#fff\"",
        "#fff onmouseover=alert(1)",
        "red",
        "rgb(0, 0, 0)",
        "var(--accent)",
        "#12345678",
        "#1234",
        "#ggg",
        "#",
        "",
        " #fff",
        "#fff ",
        "#ｆｆｆ",
    ] {
        let tokens = json!({ "color": { "evil": bad }, "chart": { "palette": [bad, "evil"] } });
        asserting(&format!("{bad:?}")).that(&chart_palette(&tokens)).is_empty();
    }
}

#[test]
fn chart_palette_keeps_short_and_long_hex_in_lowercase() {
    let tokens = json!({ "chart": { "palette": ["#ABC", "#A1B2C3"] } });
    assert_that!(chart_palette(&tokens)).is_equal_to(vec!["#abc".to_owned(), "#a1b2c3".to_owned()]);
}
