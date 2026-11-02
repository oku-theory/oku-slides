use std::path::Path;
use std::sync::Arc;

use rstest::rstest;
use speculoos::prelude::*;

use super::{Microphone, ModelFiles};
use crate::stream::NoStreamEngine;
use crate::test_support::{french_platform, fresh_dir};
use crate::{ApiConfig, Config, Model, Stt};

/// Whisper dictation only (Turbo by default), its models in `dir`.
fn stt(dir: &Path) -> Stt {
    Stt::with_engine(&french_platform(dir, |s| s.apple = false), Arc::new(NoStreamEngine), None)
}

/// Writes an empty `name` in the Turbo model's folder.
fn turbo_file(stt: &Stt, name: &str) {
    let folder = Model::Turbo.path(stt.models_dir());
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join(name), b"").unwrap();
}

#[rstest]
#[case(0, Microphone::Undetermined)]
#[case(1, Microphone::Restricted)]
#[case(2, Microphone::Denied)]
#[case(3, Microphone::Granted)]
#[case(-1, Microphone::Unknown)]
#[case(7, Microphone::Unknown)]
fn microphone_from_code_cases_follow_the_bridge_table(#[case] code: i32, #[case] expected: Microphone) {
    assert_that!(Microphone::from_code(code)).is_equal_to(expected);
}

#[rstest]
#[case(Microphone::Undetermined)]
#[case(Microphone::Restricted)]
#[case(Microphone::Denied)]
#[case(Microphone::Granted)]
#[case(Microphone::Unknown)]
fn microphone_id_cases_equal_the_serde_name(#[case] mic: Microphone) {
    assert_that!(serde_json::to_value(mic).unwrap()).is_equal_to(serde_json::json!(mic.id()));
}

#[rstest]
#[case(ModelFiles::Missing)]
#[case(ModelFiles::Partial)]
#[case(ModelFiles::Complete)]
fn model_files_id_cases_equal_the_serde_name(#[case] files: ModelFiles) {
    assert_that!(serde_json::to_value(files).unwrap()).is_equal_to(serde_json::json!(files.id()));
}

#[test]
fn model_files_nothing_on_disk_is_missing() {
    let dir = fresh_dir("mic-missing");
    let stt = stt(dir.path());
    assert_that!(stt.model_files(&stt.status())).is_equal_to(ModelFiles::Missing);
}

#[test]
fn model_files_part_left_is_partial() {
    let dir = fresh_dir("mic-partial");
    let stt = stt(dir.path());
    turbo_file(&stt, "model.bin.part");
    assert_that!(stt.model_files(&stt.status())).is_equal_to(ModelFiles::Partial);
}

#[test]
fn model_files_complete_marker_is_complete() {
    let dir = fresh_dir("mic-complete");
    let stt = stt(dir.path());
    turbo_file(&stt, oku_slides_platform::models::COMPLETE);
    assert_that!(stt.model_files(&stt.status())).is_equal_to(ModelFiles::Complete);
}

/// The API model, read through `key_env`: no files, so the key stands for them.
#[rstest]
#[case::key_set("HOME", ModelFiles::Complete)]
#[case::key_unset("OKU_T1462_UNSET_7F3A", ModelFiles::Missing)]
fn model_files_api_follows_the_key_variable(#[case] key_env: &str, #[case] expected: ModelFiles) {
    let dir = fresh_dir("mic-api");
    let stt = stt(dir.path());
    let api = ApiConfig {
        url: "http://127.0.0.1:9/v1".into(),
        model: "whisper-1".into(),
        key_env: key_env.into(),
        ..ApiConfig::default()
    };
    // Dictation idle: what `configure` applied does not matter here.
    let _applied = stt.configure(Config { model: Model::Api, api, ..Config::default() });
    assert_that!(stt.model_files(&stt.status())).is_equal_to(expected);
}
