use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
#[case("Calibri Light", Some(300))]
#[case("Segoe UI Semibold", Some(600))]
#[case("Montserrat-SemiBold", Some(600))]
#[case("Open Sans Semi Bold", Some(600))]
#[case("Source Sans Extra Light Italic", Some(200))]
#[case("Arial Black", Some(900))]
#[case("Inter Medium", Some(500))]
#[case("Calibri", None)]
#[case("Light", None)]
#[case("Highlight Sans", None)]
fn family_weight_reads_the_weight_suffix(#[case] family: &str, #[case] expected: Option<u16>) {
    assert_that!(family_weight(family)).is_equal_to(expected);
}

#[test]
fn asset_is_drawable_by_its_real_type() {
    let asset = |mime: &str| Asset { mime: mime.to_owned(), bytes: Vec::new() };
    assert_that!((asset("image/webp").is_drawable(), asset("image/x-emf").is_drawable())).is_equal_to((true, false));
}
