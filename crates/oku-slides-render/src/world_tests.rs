use speculoos::prelude::*;

use oku_slides_core::{Namespace, PackKind};

use super::{LIBRARY, Labels, main_source, typst_string};
use crate::{BuiltinPack, PackSet, PackWidgets};

#[test]
fn typst_string_escapes_quotes_backslashes_and_newlines() {
    assert_that!(typst_string("a \"b\" \\ c\nd")).is_equal_to(r#""a \"b\" \\ c\nd""#.to_owned());
}

#[test]
fn main_source_quotes_labels_instead_of_running_them() {
    let labels = Labels { missing_image: "#panic()".into(), chart_failed: "\") #panic() (\"".into() };
    let source = main_source(true, &PackSet::default(), ["ab12"].into_iter(), &labels);
    assert_that!(source).contains(r##"fallback("#panic()")"##);
    assert_that!(source).contains(r##"fallback("\") #panic() (\"")"##);
    assert_that!(source).contains(r#"#let available = ("ab12",)"#);
}

fn empty_pack(id: &str, ns: &str, kind: PackKind) -> PackWidgets {
    PackWidgets::new(id, Namespace::parse(ns).unwrap(), kind, std::collections::BTreeMap::new()).unwrap()
}

#[test]
fn main_source_with_packs_imports_each_by_kind() {
    let set = PackSet::new(vec![
        empty_pack("citations", "zeta", PackKind::Widgets),
        empty_pack("frises", "alpha", PackKind::Layouts),
    ])
    .unwrap();
    let source = main_source(false, &set, std::iter::empty(), &Labels::default());
    assert_that!(source).contains(r#"#import "/packs/frises/layouts.typ": registry as pack-registry-0"#);
    assert_that!(source).contains(r#"#import "/packs/citations/widgets.typ": registry as pack-registry-1"#);
    assert_that!(source).contains("widgets: by-name(pack-registry-0 + pack-registry-1)");
}

#[test]
fn main_source_imports_no_core_widgets() {
    let set = PackSet::new(vec![BuiltinPack::Essentiel.widgets().unwrap()]).unwrap();
    let src = main_source(false, &set, std::iter::empty(), &Labels::default());
    assert_that!((src.contains("src/widgets.typ"), src.contains("/layouts.typ"))).is_equal_to((false, true));
}

#[test]
fn main_source_without_pack_renders_with_no_widgets() {
    let source = main_source(false, &PackSet::default(), std::iter::empty(), &Labels::default());
    assert_that!(source).does_not_contain("/packs/");
    assert_that!(source).contains("widgets: by-name(())");
}

#[test]
fn main_source_with_theme_reads_theme_json() {
    let source = main_source(true, &PackSet::default(), std::iter::empty(), &Labels::default());
    assert_that!(source).contains(r#"theme: json("/theme.json")"#);
}

#[test]
fn main_source_without_theme_passes_none() {
    let source = main_source(false, &PackSet::default(), std::iter::empty(), &Labels::default());
    assert_that!(source).contains("theme: none");
}

/// La source embarquée de `file` : un fichier de la bibliothèque, ou du pack `base`
/// (`packs/base/widgets/<nom>.typ`).
fn embedded_source(file: &str) -> Option<&'static str> {
    if let Some(name) = file.strip_prefix("packs/base/widgets/").and_then(|f| f.strip_suffix(".typ")) {
        let (_, bytes) = crate::builtin_packs::BASE_WIDGETS.iter().find(|(n, _)| *n == name)?;
        return std::str::from_utf8(bytes).ok();
    }
    LIBRARY.iter().find(|(path, _)| *path == file).map(|(_, source)| *source)
}

/// La valeur de `#let <name> = <nombre>` dans le fichier embarqué `file`.
fn typst_bound(file: &str, name: &str) -> Option<f64> {
    let source = embedded_source(file)?;
    let prefix = format!("#let {name} = ");
    let line = source.lines().find_map(|l| l.strip_prefix(&prefix))?;
    line.split_whitespace().next()?.parse().ok()
}

/// Les bornes que la bibliothèque Typst recopie restent celles du cœur.
#[rstest::rstest]
#[case::inset("packs/base/widgets/text.typ", "_inset-max", oku_slides_core::STYLE_INSET_MAX)]
#[case::spacing("packs/base/widgets/text.typ", "_spacing-max", oku_slides_core::STYLE_SPACING_MAX)]
#[case::tracking("src/content.typ", "_tracking-max", oku_slides_core::STYLE_TRACKING_MAX)]
fn library_style_bounds_equal_the_core_constants(#[case] file: &str, #[case] name: &str, #[case] core: f64) {
    assert_that!(typst_bound(file, name)).is_equal_to(Some(core));
}
