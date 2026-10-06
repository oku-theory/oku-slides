use speculoos::prelude::*;

use super::{append, escape, rows};

#[test]
fn rows_wrap_entries_that_overflow_the_width() {
    let r = rows(&[40.0, 40.0, 40.0], 10.0, 100.0);
    assert_that!(r).is_equal_to(vec![(0..2, 90.0), (2..3, 40.0)]);
}

#[test]
fn rows_keep_an_entry_wider_than_the_width_on_its_own_row() {
    let r = rows(&[150.0, 20.0], 10.0, 100.0);
    assert_that!(r).is_equal_to(vec![(0..1, 150.0), (1..2, 20.0)]);
}

#[test]
fn escape_neutralizes_xml_markup() {
    assert_that!(escape(r#"<a href="x">&'"#)).is_equal_to("&lt;a href=&quot;x&quot;&gt;&amp;&apos;".to_owned());
}

#[test]
fn append_puts_the_header_before_the_closing_tag() {
    let svg = append("<svg>\n<rect/>\n</svg>".into(), "<g/>\n");
    assert_that!(svg).is_equal_to("<svg>\n<rect/>\n<g/>\n</svg>".to_owned());
}

#[test]
fn append_without_header_keeps_the_svg() {
    assert_that!(append("<svg></svg>".into(), "")).is_equal_to("<svg></svg>".to_owned());
}
