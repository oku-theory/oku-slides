use rstest::rstest;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use super::{Control, Section, default_tokens, ensure_unused, merge, parse_schema, schema, spec, users, validate};
use crate::{Localize, Localizer, SessionErrorKind};

fn p(path: &[&str]) -> Vec<String> {
    path.iter().map(|s| (*s).to_owned()).collect()
}

fn defaults_map() -> Map<String, Value> {
    default_tokens().clone()
}

#[test]
fn default_tokens_parse() {
    assert_that!(default_tokens().contains_key("color")).is_true();
}

/// Chemins des feuilles d'un arbre de jetons (les tableaux sont des feuilles).
fn leaf_paths(v: &Value) -> Vec<Vec<String>> {
    fn walk(v: &Value, prefix: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
        match v {
            Value::Object(m) => {
                for (k, child) in m {
                    prefix.push(k.clone());
                    walk(child, prefix, out);
                    prefix.pop();
                }
            }
            _ => out.push(prefix.clone()),
        }
    }
    let mut out = Vec::new();
    walk(v, &mut Vec::new(), &mut out);
    out
}

/// La raison du refus, lue à la fin du message (`…: <raison>`).
fn reason(path: &[&str], value: Value, tokens: &Map<String, Value>) -> String {
    let err = validate(&p(path), &value, tokens).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::InvalidTokenValue);
    let line = err.one_line();
    assert_that!(line.as_str()).starts_with(format!("invalid value for {}: ", path.join(".")));
    line.rsplit(": ").next().unwrap().to_owned()
}

#[test]
fn schema_json_parses() {
    assert_that!(parse_schema().map(|s| s.len())).is_ok().is_greater_than(0);
}

#[test]
fn schema_covers_exactly_default_tokens_paths() {
    let defaults = Value::Object(defaults_map());
    let mut leaves = leaf_paths(&defaults);
    leaves.retain(|p| !(p[0] == "page" && (p[1] == "width" || p[1] == "height")));
    // Les entrées facultatives (variantes d'interface, champs lus avec une
    // valeur de repli) sont absentes des valeurs par défaut.
    let mut covered: Vec<Vec<String>> = schema().iter().filter(|s| !s.optional).map(|s| s.path.clone()).collect();
    leaves.sort();
    covered.sort();
    assert_that!(covered).is_equal_to(leaves);
}

#[test]
fn schema_optional_entries_are_absent_from_defaults() {
    let defaults = Value::Object(defaults_map());
    let optional: Vec<_> = schema().iter().filter(|s| s.optional).collect();
    assert_that!(optional.len()).is_greater_than(0);
    for s in optional {
        let found = s.path.iter().try_fold(&defaults, |v, k| v.get(k));
        asserting(&s.path.join(".")).that(&found).is_none();
    }
}

#[test]
fn schema_chapter_number_color_is_an_optional_color() {
    let s = spec(&p(&["component", "chapter", "number-color"])).unwrap();
    assert_that!((s.control.clone(), s.section, s.nullable, s.optional)).is_equal_to((
        Control::Color,
        Section::Components,
        false,
        true,
    ));
}

#[test]
fn users_of_a_color_include_the_chapter_number_color() {
    let papier: Value = serde_json::from_str(include_str!("../../../typst/oku-slides/themes/papier.json")).unwrap();
    let mut tokens = defaults_map();
    merge(&mut tokens, papier["tokens"].as_object().unwrap());
    assert_that!(users(&p(&["color", "accent"]), &tokens)).contains("component.chapter.number-color".to_owned());
}

#[test]
fn schema_app_variants_mirror_app_colors() {
    let app_colors: Vec<&str> = schema()
        .iter()
        .filter(|s| s.path[0] == "app" && s.control == Control::Color)
        .map(|s| s.path[1].as_str())
        .collect();
    for (block, section) in [("app-dark", Section::AppDark), ("app-light", Section::AppLight)] {
        let entries: Vec<_> = schema().iter().filter(|s| s.path[0] == block).collect();
        let names: Vec<&str> = entries.iter().map(|s| s.path[1].as_str()).collect();
        asserting(block).that(&names).is_equal_to(&app_colors);
        for s in entries {
            asserting(&s.path.join(".")).that(&(s.control.clone(), s.section, s.nullable, s.optional)).is_equal_to((
                Control::Color,
                section,
                true,
                true,
            ));
        }
    }
}

#[test]
fn schema_has_no_duplicate_paths() {
    let mut paths: Vec<_> = schema().iter().map(|s| s.path.clone()).collect();
    let n = paths.len();
    paths.sort();
    paths.dedup();
    assert_that!(paths.len()).is_equal_to(n);
}

/// Les valeurs d'un arbre de jetons, par chemin du schéma : la descente
/// s'arrête au premier chemin connu (un filet de titre est une valeur).
fn spec_values(v: &Value) -> Vec<(Vec<String>, Value)> {
    fn walk(v: &Value, prefix: &mut Vec<String>, out: &mut Vec<(Vec<String>, Value)>) {
        match v {
            Value::Object(m) if spec(prefix).is_none() => {
                for (k, child) in m {
                    prefix.push(k.clone());
                    walk(child, prefix, out);
                    prefix.pop();
                }
            }
            _ => out.push((prefix.clone(), v.clone())),
        }
    }
    let mut out = Vec::new();
    walk(v, &mut Vec::new(), &mut out);
    out
}

#[test]
fn schema_accepts_every_default_value() {
    let tokens = defaults_map();
    for (path, value) in spec_values(&Value::Object(tokens.clone())) {
        if path[0] == "page" && (path[1] == "width" || path[1] == "height") {
            continue;
        }
        asserting(&path.join(".")).that(&validate(&path, &value, &tokens).map_err(|e| e.one_line())).is_ok();
    }
}

#[test]
fn schema_accepts_every_builtin_theme_value() {
    for theme in [
        include_str!("../../../typst/oku-slides/themes/papier.json"),
        include_str!("../../../typst/oku-slides/themes/pro.json"),
    ] {
        let theme: Value = serde_json::from_str(theme).unwrap();
        let mut tokens = defaults_map();
        merge(&mut tokens, theme["tokens"].as_object().unwrap());
        for (path, value) in spec_values(&theme["tokens"]) {
            asserting(&path.join(".")).that(&validate(&path, &value, &tokens).map_err(|e| e.one_line())).is_ok();
        }
    }
}

#[rstest]
#[case(&["color", "accent"], json!("#3f6b5e"), true)]
#[case(&["color", "accent"], json!("ink"), true)]
#[case(&["color", "accent"], json!("#zzz"), false)]
#[case(&["size", "body"], json!(-4), false)]
#[case(&["weight", "title"], json!(450), false)]
#[case(&["component", "header", "left"], Value::Null, true)]
#[case(&["size", "body"], Value::Null, false)]
#[case(&["color", "accent"], json!("#3F6B5E80"), true)]
#[case(&["color", "brand-ink"], json!("#00154e"), true)]
#[case(&["size", "body"], json!(400), true)]
#[case(&["size", "body"], json!(401), false)]
#[case(&["size", "body"], json!("title"), true)]
#[case(&["page", "margin-x"], json!(1080), true)]
#[case(&["page", "margin-x"], json!("lg"), false)]
#[case(&["page", "width"], json!(1920), false)]
#[case(&["leading", "tight"], json!(2), true)]
#[case(&["leading", "tight"], json!(2.5), false)]
#[case(&["component", "title", "leading"], json!("loose"), true)]
#[case(&["weight", "title"], json!(700), true)]
#[case(&["weight", "title"], json!(400.0), false)]
#[case(&["component", "card", "title-weight"], json!(600.0), false)]
#[case(&["weight", "title"], json!("body"), false)]
#[case(&["component", "card", "title-weight"], json!("strong"), true)]
#[case(&["component", "card", "title-weight"], json!(1000), false)]
#[case(&["font", "body"], json!(["Inter", "Libertinus Serif"]), true)]
#[case(&["font", "body"], json!([]), false)]
#[case(&["font", "body"], json!([""]), false)]
#[case(&["script-style", "display"], json!("serif"), true)]
#[case(&["script-style", "display"], json!("mono"), false)]
#[case(&["component", "header", "rule"], json!(true), true)]
#[case(&["component", "header", "rule"], json!("yes"), false)]
#[case(&["component", "footer", "left"], json!("{n} · {title}"), true)]
#[case(&["component", "footer", "left"], json!(3), false)]
#[case(&["chart", "palette"], json!(["accent", "#c4622d"]), true)]
#[case(&["chart", "palette"], json!(["accent", "nope"]), false)]
#[case(&["chart", "palette"], json!([]), false)]
#[case(&["component", "title-slide", "band-ratio"], json!(0.62), true)]
#[case(&["component", "title-slide", "band-ratio"], json!(1.2), false)]
#[case(&["component", "kicker", "tracking"], json!(-10), true)]
#[case(&["component", "kicker", "tracking"], json!(21), false)]
#[case(&["component", "card", "border-width"], json!("hairline"), true)]
#[case(&["component", "card", "border-width"], json!("md"), false)]
#[case(&["component", "title", "rule"], json!({"width": 64, "height": 5, "color": "accent"}), true)]
#[case(&["component", "title", "rule"], json!({"width": 64, "color": "accent"}), false)]
#[case(&["app", "desk"], Value::Null, true)]
#[case(&["app-dark", "desk"], json!("#000c2e"), true)]
#[case(&["app", "scheme"], json!("dark"), true)]
#[case(&["app", "scheme"], json!("sepia"), false)]
fn validate_checks_control(#[case] path: &[&str], #[case] value: Value, #[case] ok: bool) {
    let tokens = defaults_map();
    asserting(&format!("{path:?} = {value}")).that(&validate(&p(path), &value, &tokens).is_ok()).is_equal_to(ok);
}

#[rstest]
#[case(&["nope", "x"], json!(1), "unknown-token")]
#[case(&["color", "Bad Name"], json!("#fff"), "bad-name")]
#[case(&["size", "body"], Value::Null, "not-nullable")]
#[case(&["color", "accent"], json!("#zzz"), "not-a-color")]
#[case(&["color", "accent"], json!("nowhere"), "unknown-reference")]
#[case(&["size", "body"], json!(-4), "out-of-range")]
#[case(&["size", "body"], json!(true), "wrong-type")]
#[case(&["weight", "title"], json!(450), "not-a-weight")]
#[case(&["weight", "title"], json!(400.0), "not-a-weight")]
#[case(&["font", "body"], json!([]), "bad-fonts")]
#[case(&["app", "scheme"], json!("sepia"), "not-a-choice")]
#[case(&["chart", "palette"], json!([]), "bad-palette")]
#[case(&["component", "title", "rule"], json!({"width": -1, "height": 5, "color": "accent"}), "bad-rule")]
fn validate_names_the_reason(#[case] path: &[&str], #[case] value: Value, #[case] expected: &str) {
    assert_that!(reason(path, value, &defaults_map()).as_str()).is_equal_to(expected);
}

#[test]
fn validate_refuses_color_reference_loops() {
    let tokens = defaults_map();
    // ink-muted → ink, puis ink → ink-muted : boucle.
    let mut looped = tokens.clone();
    looped["color"]["ink-muted"] = json!("ink");
    assert_that!(reason(&["color", "ink"], json!("ink-muted"), &looped).as_str()).is_equal_to("reference-loop");
    assert_that!(reason(&["color", "ink"], json!("ink"), &tokens).as_str()).is_equal_to("reference-loop");
}

#[test]
fn validate_refuses_size_reference_loops() {
    let mut tokens = defaults_map();
    tokens["size"]["title"] = json!("body");
    assert_that!(reason(&["size", "body"], json!("title"), &tokens).as_str()).is_equal_to("reference-loop");
}

#[test]
fn validate_references_follow_the_given_tokens() {
    let tokens = json!({"color": {"brand-ink": "#00154e"}});
    let tokens = tokens.as_object().unwrap();
    assert_that!(validate(&p(&["component", "title", "color"]), &json!("brand-ink"), tokens)).is_ok();
    assert_that!(validate(&p(&["component", "title", "color"]), &json!("accent"), tokens)).is_err();
}

#[test]
fn spec_gives_a_generic_color_for_free_names() {
    let s = spec(&p(&["color", "surface-dark"])).unwrap();
    assert_that!((s.control.clone(), s.section, s.nullable)).is_equal_to((Control::Color, Section::Colors, false));
    assert_that!(spec(&p(&["color", "9lives"]))).is_none();
    assert_that!(spec(&p(&["color", "a", "b"]))).is_none();
}

#[test]
fn spec_finds_a_listed_path() {
    let s = spec(&p(&["size", "body"])).unwrap();
    assert_that!(s.control.clone()).is_equal_to(Control::Pt { min: 0.0, max: 400.0, refs: Some("size".to_owned()) });
    assert_that!(s.section).is_equal_to(Section::Typography);
}

#[test]
fn users_lists_tokens_referencing_a_color() {
    let tokens = json!({"color": {"brand-ink": "#00154e"}, "component": {"title": {"color": "brand-ink"}}});
    assert_that!(users(&p(&["color", "brand-ink"]), tokens.as_object().unwrap()))
        .is_equal_to(vec!["component.title.color".to_owned()]);
}

#[test]
fn users_follows_palettes_colors_and_rules() {
    let tokens = json!({
        "color": {"accent": "#00bef4", "accent-text": "accent", "ink": "#000"},
        "chart": {"palette": ["ink", "accent"]},
        "app-dark": {"accent": "accent"},
        "component": {"title": {"rule": {"width": 64, "height": 5, "color": "accent"}}, "kicker": {"color": "ink"}}
    });
    assert_that!(users(&p(&["color", "accent"]), tokens.as_object().unwrap())).is_equal_to(vec![
        "app-dark.accent".to_owned(),
        "chart.palette".to_owned(),
        "color.accent-text".to_owned(),
        "component.title.rule".to_owned(),
    ]);
}

#[test]
fn users_lists_tokens_referencing_a_size_or_weight() {
    let tokens = defaults_map();
    assert_that!(users(&p(&["size", "caption"]), &tokens))
        .is_equal_to(vec!["component.footer.size".to_owned(), "component.header.size".to_owned()]);
    assert_that!(users(&p(&["weight", "strong"]), &tokens))
        .is_equal_to(vec!["component.card.title-weight".to_owned(), "component.table.header-weight".to_owned()]);
    // Même nom, autre catégorie : `space.lg` n'est pas `size.lg`.
    assert_that!(users(&p(&["radius", "lg"]), &tokens)).is_empty();
}

#[test]
fn ensure_unused_names_the_users() {
    let tokens = json!({"color": {"brand-ink": "#00154e"}, "component": {"title": {"color": "brand-ink"}}});
    let tokens = tokens.as_object().unwrap();
    let err = ensure_unused(&p(&["color", "brand-ink"]), tokens).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::TokenInUse);
    assert_that!(err.one_line()).contains("component.title.color");
    assert_that!(ensure_unused(&p(&["color", "accent"]), tokens)).is_ok();
}

#[rstest]
#[case("unknown-token")]
#[case("bad-name")]
#[case("not-nullable")]
#[case("wrong-type")]
#[case("not-a-color")]
#[case("unknown-reference")]
#[case("reference-loop")]
#[case("out-of-range")]
#[case("not-a-weight")]
#[case("bad-fonts")]
#[case("not-a-choice")]
#[case("bad-palette")]
#[case("bad-rule")]
fn token_reasons_are_translated(#[case] reason: &str) {
    for lang in ["en", "fr"] {
        let text = Localizer::new(Some(lang)).tr(&format!("session-token-reason-{reason}"));
        asserting(&format!("{lang} {reason}")).that(&text.as_str()).does_not_contain("session-");
    }
}

#[test]
fn invalid_token_value_is_localized_with_its_reason() {
    let err = validate(&p(&["color", "accent"]), &json!("#zzz"), &defaults_map()).unwrap_err();
    assert_that!(err.localize(&Localizer::new(Some("fr"))).as_str())
        .is_equal_to("Valeur refusée pour color.accent : couleur hexadécimale attendue (#rgb, #rrggbb ou #rrggbbaa)");
}

#[test]
fn token_in_use_is_localized_with_its_users() {
    let tokens = json!({"color": {"brand-ink": "#00154e"}, "component": {"title": {"color": "brand-ink"}}});
    let err = ensure_unused(&p(&["color", "brand-ink"]), tokens.as_object().unwrap()).unwrap_err();
    assert_that!(err.localize(&Localizer::new(Some("en"))).as_str())
        .is_equal_to("color.brand-ink is still used by: component.title.color");
}
