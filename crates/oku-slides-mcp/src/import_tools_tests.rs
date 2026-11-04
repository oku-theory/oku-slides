use std::path::Path;

use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::{ImportArgs, ImportModeArg, is_pptx};

#[rstest]
#[case::lower("/d/a.pptx", true)]
#[case::upper("/d/A.PPTX", true)]
#[case::keynote("/d/a.key", false)]
#[case::pdf("/d/a.oku-slides.pdf", false)]
#[case::none("/d/pptx", false)]
fn is_pptx_reads_the_extension_whatever_its_case(#[case] path: &str, #[case] expected: bool) {
    assert_that!(is_pptx(Path::new(path))).is_equal_to(expected);
}

#[test]
fn import_args_without_options_take_the_defaults() {
    let args: ImportArgs = serde_json::from_value(json!({ "path": "/d/a.pptx", "mode": "retrofit" })).unwrap();
    assert_that!(matches!(args.mode, ImportModeArg::Retrofit)).is_true();
    assert_that!((args.pack.is_empty(), args.theme.is_none(), args.tune)).is_equal_to((true, true, false));
}

#[test]
fn import_args_with_an_unknown_field_are_refused() {
    let parsed = serde_json::from_value::<ImportArgs>(json!({ "path": "/d/a.pptx", "mode": "hifi", "layout": "x" }));
    assert_that!(parsed.is_err()).is_true();
}
