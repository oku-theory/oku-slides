use std::collections::HashSet;

use speculoos::prelude::*;

use super::*;
use crate::model::{Cell, Element, Frame, Slide};

fn ids(issues: &[Issue]) -> Vec<&'static str> {
    issues.iter().map(|i| i.message.id()).collect()
}

fn deck(slides: Vec<Slide>) -> Deck {
    let mut d = Deck::from_json(r#"{"slides":[]}"#).unwrap();
    d.slides = slides;
    d
}

#[test]
fn validate_slide_problems_are_each_reported() {
    let mut chapter = Slide::new("c", SlideKind::Chapter);
    chapter.blocks.push(Element::text("b", vec![]));
    chapter
        .elements
        .push(Element { frame: Some(Frame { x: 0.0, y: 0.0, w: 1.0, h: 1.0 }), ..Element::text("e", vec![]) });
    let issues =
        validate(&deck(vec![Slide::new("", SlideKind::Title), Slide::new("", SlideKind::Title), chapter]), None);
    assert_that!(ids(&issues)).is_equal_to(vec![
        "issue-empty-slide-id",
        "issue-empty-slide-id",
        "issue-duplicate-slide-id",
        "issue-chapter-needs-title",
        "issue-blocks-and-elements",
    ]);
}

#[test]
fn validate_element_problems_are_each_reported() {
    let mut s = Slide::new("s", SlideKind::Content);
    s.elements = vec![
        Element::text("", vec![]),
        Element { frame: Some(Frame { x: 0.0, y: 0.0, w: 0.0, h: 5.0 }), ..Element::text("zero", vec![]) },
        Element { cell: Some(Cell { col: 11, row: 0, cols: 4, rows: 1 }), ..Element::text("out", vec![]) },
        Element::text("dup", vec![]),
        Element::text("dup", vec![]),
    ];
    let issues = validate(&deck(vec![s]), None);
    let found = ids(&issues);
    for expected in [
        "issue-empty-element-id",
        "issue-empty-frame",
        "issue-cell-outside-grid",
        "issue-duplicate-element-id",
        "issue-free-without-frame",
    ] {
        asserting(expected).that(&found).contains(expected);
    }
}

#[test]
fn validate_missing_asset_is_reported_only_when_assets_are_checked() {
    let mut s = Slide::new("s", SlideKind::Content);
    s.blocks.push(Element {
        body: crate::model::ElementBody::Image { asset: "sha256:ab".into(), fit: None },
        ..Element::text("img", vec![])
    });
    let d = deck(vec![s]);
    assert_that!(validate(&d, None)).is_empty();
    let issues = validate(&d, Some(&HashSet::new()));
    assert_that!(issues.first().map(ToString::to_string))
        .is_equal_to(Some("[s/img] missing asset: sha256:ab".to_owned()));
}

#[test]
fn issue_localize_prefixes_slide_and_element() {
    let issue = Issue { slide: Some("s".into()), element: None, message: Message::new("issue-chapter-needs-title") };
    let fr = slider_i18n::Localizer::new(Some("fr"));
    assert_that!(issue.localize(&fr)).is_equal_to("[s] un chapitre doit avoir un titre".to_owned());
    let bare = Issue { slide: None, element: None, message: Message::new("issue-empty-frame") };
    assert_that!(bare.to_string()).is_equal_to("zero-size frame".to_owned());
}
