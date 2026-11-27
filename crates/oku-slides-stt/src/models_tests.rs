use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
fn model_serde_name_is_its_id(#[values(Model::Parakeet, Model::Api)] model: Model) {
    assert_that!(serde_json::to_string(&model).unwrap()).is_equal_to(format!("\"{}\"", model.id()));
}

#[rstest]
fn model_from_id_round_trips(#[values(Model::Parakeet, Model::Api)] model: Model) {
    assert_that!(Model::from_id(model.id())).is_equal_to(Some(model));
}

#[test]
fn model_all_lists_every_variant_tested_above() {
    assert_that!(Model::ALL.len()).is_equal_to(2);
}

#[test]
fn model_from_id_unknown_is_none() {
    assert_that!(Model::from_id("gpt")).is_none();
}

#[rstest]
fn old_model_ids_do_not_deserialize(#[values("turbo", "large", "base", "apple")] id: &str) {
    assert_that!(serde_json::from_str::<Model>(&format!("\"{id}\""))).is_err();
}

#[rstest]
fn model_from_id_old_id_is_none(#[values("turbo", "large", "base", "apple")] id: &str) {
    assert_that!(Model::from_id(id)).is_none();
}

#[test]
fn default_model_is_parakeet() {
    assert_that!(Model::default()).is_equal_to(Model::Parakeet);
}

#[test]
fn parakeet_files_are_pinned() {
    let files = Model::Parakeet.files();
    let pins: Vec<_> = files.iter().map(|f| (f.repo, f.revision, f.name, f.sha256, f.size)).collect();
    let repo = "istupakov/parakeet-tdt-0.6b-v3-onnx";
    let revision = "8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce";
    assert_that!(pins).is_equal_to(vec![
        (
            repo,
            revision,
            "encoder-model.int8.onnx",
            Some("6139d2fa7e1b086097b277c7149725edbab89cc7c7ae64b23c741be4055aff09"),
            652_183_999,
        ),
        (
            repo,
            revision,
            "decoder_joint-model.int8.onnx",
            Some("eea7483ee3d1a30375daedc8ed83e3960c91b098812127a0d99d1c8977667a70"),
            18_202_004,
        ),
        (repo, revision, "vocab.txt", Some("d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d"), 93_939),
    ]);
    assert_that!(files.iter().map(|f| f.size).sum::<u64>()).is_equal_to(Model::Parakeet.size());
    assert_that!(Model::Parakeet.size()).is_equal_to(670_479_942);
}

#[rstest]
#[case("fr", true)]
#[case("FR", true)]
#[case("pt-BR", true)]
#[case("uk", true)]
#[case("ja", false)]
#[case("zh-Hans", false)]
#[case("", false)]
fn parakeet_supports_matches_the_primary_subtag(#[case] language: &str, #[case] expected: bool) {
    assert_that!(parakeet_supports(language)).is_equal_to(expected);
}

#[test]
fn parakeet_supports_every_listed_language() {
    for language in PARAKEET_LANGUAGES {
        assert_that!(parakeet_supports(language)).named(language).is_true();
    }
}
