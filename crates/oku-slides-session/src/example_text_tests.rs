use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;
use uuid::Uuid;

use super::{localized_examples, resolved_examples};
use crate::Localizer;
use crate::pack::{BASE_PACK as BASE_ID, ESSENTIEL_PACK};

#[rstest]
#[case("en", "New text")]
#[case("fr", "Nouveau texte")]
fn localized_examples_builtin_marker_is_translated(#[case] locale: &str, #[case] expected: &str) {
    let values = json!({"body": [{"runs": [{"t": "ftl:session-example-base-text-body"}]}]});
    let out = localized_examples(BASE_ID, values.as_object().unwrap(), &Localizer::new(Some(locale)));
    assert_that!(out["body"][0]["runs"][0]["t"].as_str()).is_equal_to(Some(expected));
}

#[test]
fn localized_examples_pack_not_builtin_keeps_marker() {
    let values = json!({"title": "ftl:session-example-base-card-title"});
    let out = localized_examples(Uuid::new_v4(), values.as_object().unwrap(), &Localizer::new(Some("en")));
    assert_that!(out["title"].as_str()).is_equal_to(Some("ftl:session-example-base-card-title"));
}

#[test]
fn localized_examples_key_outside_prefix_is_kept() {
    let values = json!({"title": "ftl:ui-toolbar-open"});
    let out = localized_examples(BASE_ID, values.as_object().unwrap(), &Localizer::new(Some("en")));
    assert_that!(out["title"].as_str()).is_equal_to(Some("ftl:ui-toolbar-open"));
}

#[test]
fn localized_examples_missing_key_is_kept() {
    let values = json!({"title": "ftl:session-example-nothing-here"});
    let out = localized_examples(BASE_ID, values.as_object().unwrap(), &Localizer::new(Some("en")));
    assert_that!(out["title"].as_str()).is_equal_to(Some("ftl:session-example-nothing-here"));
}

#[test]
fn localized_examples_essentiel_marker_in_a_nested_element_is_translated() {
    let values = json!({"blocks": [{"params": {"body": [{"runs": [{"t": "ftl:session-example-essentiel-content-blocks-1"}]}]}}]});
    let out = localized_examples(ESSENTIEL_PACK, values.as_object().unwrap(), &Localizer::new(Some("fr")));
    assert_that!(out["blocks"][0]["params"]["body"][0]["runs"][0]["t"].as_str()).is_equal_to(Some("Premier point"));
}

#[test]
fn localized_examples_plain_text_and_numbers_are_kept() {
    let values = json!({"title": "ftl", "n": [3, "ftl:"]});
    let out = localized_examples(BASE_ID, values.as_object().unwrap(), &Localizer::new(Some("en")));
    assert_that!(serde_json::Value::Object(out)).is_equal_to(json!({"title": "ftl", "n": [3, "ftl:"]}));
}

#[test]
fn resolved_examples_writes_markers_in_plain_text() {
    let values = json!({"title": "ftl:session-example-base-card-title"});
    let out = resolved_examples(values.as_object().unwrap(), &Localizer::new(Some("fr")));
    assert_that!(out["title"].as_str()).is_equal_to(Some("Titre de la carte"));
}
