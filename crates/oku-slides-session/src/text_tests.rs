use oku_slides_core::{Paragraph, Run};
use speculoos::prelude::*;

use super::*;

#[derive(serde::Deserialize)]
struct Case {
    name: String,
    markdown: String,
    paragraphs: Vec<Paragraph>,
}

fn cases() -> Vec<Case> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/markdown.json");
    let json = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("fixture {path} illisible : {e}"));
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    serde_json::from_value(v["cases"].clone()).unwrap()
}

#[test]
fn paragraphs_to_text_fixture_cases_give_their_markdown() {
    for c in cases() {
        asserting(&c.name).that(&paragraphs_to_text(&c.paragraphs)).is_equal_to(c.markdown);
    }
}

#[test]
fn paragraphs_to_text_link_with_space_and_parentheses_encodes_them() {
    let run = Run { link: Some("https://w.org/A (b)".into()), ..Run::plain("w") };
    let paras = [Paragraph { runs: vec![run], list: None }];
    assert_that!(paragraphs_to_text(&paras)).is_equal_to("[w](https://w.org/A%20%28b%29)".to_owned());
}

#[test]
fn paragraphs_to_text_adjacent_runs_with_same_format_are_merged() {
    let bold = |t: &str, color: Option<&str>| Run { bold: true, color: color.map(Into::into), ..Run::plain(t) };
    let paras = [Paragraph { runs: vec![bold("a", Some("accent")), bold("b", None), Run::plain("")], list: None }];
    assert_that!(paragraphs_to_text(&paras)).is_equal_to("**ab**".to_owned());
}
