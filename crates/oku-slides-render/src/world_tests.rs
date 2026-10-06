use speculoos::prelude::*;

use super::{Labels, main_source, typst_string};

#[test]
fn typst_string_escapes_quotes_backslashes_and_newlines() {
    assert_that!(typst_string("a \"b\" \\ c\nd")).is_equal_to(r#""a \"b\" \\ c\nd""#.to_owned());
}

#[test]
fn main_source_quotes_labels_instead_of_running_them() {
    let labels = Labels { missing_image: "#panic()".into(), chart_failed: "\") #panic() (\"".into() };
    let source = main_source(true, None, ["ab12"].into_iter(), &labels);
    assert_that!(source).contains(r##"fallback("#panic()")"##);
    assert_that!(source).contains(r##"fallback("\") #panic() (\"")"##);
    assert_that!(source).contains(r#"#let available = ("ab12",)"#);
}

#[test]
fn main_source_with_pack_imports_its_widgets_after_the_base() {
    let pack = crate::PackWidgets::new("citations", std::collections::BTreeMap::new()).unwrap();
    let source = main_source(false, Some(&pack), std::iter::empty(), &Labels::default());
    assert_that!(source).contains(r#"#import "/packs/citations/widgets.typ": registry as pack-registry"#);
    assert_that!(source).contains("widgets: by-name(base-registry + pack-registry)");
}

#[test]
fn main_source_without_pack_renders_the_base_widgets() {
    let source = main_source(false, None, std::iter::empty(), &Labels::default());
    assert_that!(source).does_not_contain("/packs/");
    assert_that!(source).contains("widgets: by-name(base-registry)");
}

#[test]
fn main_source_with_theme_reads_theme_json() {
    let source = main_source(true, None, std::iter::empty(), &Labels::default());
    assert_that!(source).contains(r#"theme: json("/theme.json")"#);
}

#[test]
fn main_source_without_theme_passes_none() {
    let source = main_source(false, None, std::iter::empty(), &Labels::default());
    assert_that!(source).contains("theme: none");
}
