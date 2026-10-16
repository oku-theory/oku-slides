use speculoos::prelude::*;

use super::*;
use crate::test_support::{para, run, text_box};

/// Un run `text` du champ `field`.
fn field(text: &str, field: &str) -> crate::source::Run {
    crate::source::Run { field: Some(field.to_owned()), ..run(text) }
}

fn texts(tb: &TextBox) -> Vec<String> {
    tb.paragraphs.iter().flat_map(|p| &p.runs).map(|r| r.text.clone()).collect()
}

#[test]
fn numbered_writes_the_page_number_in_slide_number_fields_only() {
    let tb = text_box(vec![para(vec![run("Page "), field("7", SLIDE_NUMBER), field("12/10", "datetime1")])]);
    assert_that!(texts(&numbered(&tb))).is_equal_to(vec![
        "Page ".to_owned(),
        PAGE_NUMBER.to_owned(),
        "12/10".to_owned(),
    ]);
}

#[test]
fn numbered_without_field_keeps_the_text() {
    let tb = text_box(vec![para(vec![run("Confidentiel")])]);
    assert_that!(numbered(&tb)).is_equal_to(tb);
}
