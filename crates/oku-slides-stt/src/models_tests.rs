use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
fn model_serde_name_is_its_id(
    #[values(Model::Apple, Model::Turbo, Model::Large, Model::Base, Model::Api)] model: Model,
) {
    assert_that!(serde_json::to_string(&model).unwrap()).is_equal_to(format!("\"{}\"", model.id()));
}

#[rstest]
fn model_from_id_round_trips(
    #[values(Model::Apple, Model::Turbo, Model::Large, Model::Base, Model::Api)] model: Model,
) {
    assert_that!(Model::from_id(model.id())).is_equal_to(Some(model));
}

#[test]
fn model_all_lists_every_variant_tested_above() {
    assert_that!(Model::ALL.len()).is_equal_to(5);
}

#[test]
fn model_from_id_unknown_is_none() {
    assert_that!(Model::from_id("gpt")).is_none();
}
