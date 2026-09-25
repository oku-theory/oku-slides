use speculoos::prelude::*;

use super::{Labels, main_source, typst_string};

#[test]
fn typst_string_escapes_quotes_backslashes_and_newlines() {
    assert_that!(typst_string("a \"b\" \\ c\nd")).is_equal_to(r#""a \"b\" \\ c\nd""#.to_owned());
}

#[test]
fn main_source_quotes_labels_instead_of_running_them() {
    let labels = Labels { missing_image: "#panic()".into(), chart_failed: "\") #panic() (\"".into() };
    let source = main_source(Some("pro"), ["ab12"].into_iter(), &labels);
    assert_that!(source).contains(r##"fallback("#panic()")"##);
    assert_that!(source).contains(r##"fallback("\") #panic() (\"")"##);
    assert_that!(source).contains(r#"#let available = ("ab12",)"#);
}
