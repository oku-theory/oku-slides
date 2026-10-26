use rstest::rstest;
use speculoos::prelude::*;

use super::super::{MAX_JSON_BYTES, Resume, ResumeError};
use crate::Storage;
use crate::test_support::fresh_dir;

/// A resume folder holding `name` with `bytes`.
fn folder_with(name: &str, bytes: &[u8]) -> (tempfile::TempDir, Resume) {
    let dir = fresh_dir("oku-slides-resume-versions-");
    std::fs::write(dir.path().join(name), bytes).unwrap();
    let resume = Resume::new(Storage::local().unwrap(), dir.path().to_path_buf());
    (dir, resume)
}

#[rstest]
#[case(3)]
#[case(4)]
fn read_json_versions_accepts_each_listed_version(#[case] version: u32) {
    let (_dir, r) = folder_with("x.json", format!(r#"{{"version":{version}}}"#).as_bytes());
    let read: Option<serde_json::Value> = r.read_json_versions("x.json", 3..=4).unwrap();
    assert_that!(read.map(|v| v["version"].clone())).is_equal_to(Some(serde_json::json!(version)));
}

#[rstest]
#[case(2)]
#[case(5)]
fn read_json_versions_refuses_a_version_outside_the_range(#[case] version: u32) {
    let (_dir, r) = folder_with("x.json", format!(r#"{{"version":{version}}}"#).as_bytes());
    let read = r.read_json_versions::<serde_json::Value>("x.json", 3..=4);
    assert_that!(matches!(read, Err(ResumeError::Version { version: v, .. }) if v == version)).is_true();
}

#[test]
fn read_json_versions_keeps_the_size_limit() {
    let mut bytes = br#"{"version":3}"#.to_vec();
    bytes.resize(usize::try_from(MAX_JSON_BYTES).unwrap() + 1, b' ');
    let (_dir, r) = folder_with("x.json", &bytes);
    let read = r.read_json_versions::<serde_json::Value>("x.json", 3..=4);
    assert_that!(matches!(read, Err(ResumeError::Access { .. }))).is_true();
}

#[rstest]
#[case(3, true)]
#[case(4, false)]
fn read_json_still_reads_its_single_version(#[case] version: u32, #[case] read: bool) {
    let (_dir, r) = folder_with("x.json", format!(r#"{{"version":{version}}}"#).as_bytes());
    assert_that!(r.read_json::<serde_json::Value>("x.json", 3).is_ok()).is_equal_to(read);
}
