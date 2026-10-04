use oku_slides_core::{Grid, Slide};
use serde_json::json;
use speculoos::prelude::*;

use super::template_params;

fn l() -> oku_slides_i18n::Localizer {
    oku_slides_i18n::Localizer::new(Some("fr"))
}

fn grid() -> Option<Grid> {
    Some(Grid { columns: 12, rows: 6, gap: None })
}

#[test]
fn template_params_with_grid_has_four_keys() {
    let p = template_params(&Slide::new("t-1", "blank"), grid(), &l());
    assert_that!(p.keys).is_equal_to(["name", "description", "columns", "rows"].map(String::from).to_vec());
}

#[test]
fn template_params_name_field_shows_the_name() {
    let mut slide = Slide::new("t-1", "blank");
    slide.name = Some("Libre".into());
    assert_that!(template_params(&slide, grid(), &l()).spec.fields[0].value.clone()).is_equal_to(json!("Libre"));
}

#[test]
fn template_params_columns_field_applies_set_grid() {
    let p = template_params(&Slide::new("t-1", "blank"), grid(), &l());
    assert_that!(p.spec.fields[2].op_template.clone())
        .is_equal_to(json!({ "op": "set_grid", "slide": "t-1", "columns": "$value" }));
}

#[test]
fn template_params_grid_fields_are_bounded_by_grid_tracks() {
    let p = template_params(&Slide::new("t-1", "blank"), grid(), &l());
    assert_that!((p.spec.fields[2].min, p.spec.fields[2].max)).is_equal_to((Some(1.0), Some(24.0)));
}

#[test]
fn template_params_without_grid_has_two_keys() {
    assert_that!(template_params(&Slide::new("t-1", "blank"), None, &l()).keys.len()).is_equal_to(2);
}
