use speculoos::prelude::*;

use super::*;

fn marks() -> (tempfile::TempDir, UpdateMarks) {
    let dir = tempfile::tempdir().unwrap();
    let marks = UpdateMarks::new(Storage::local().unwrap(), dir.path());
    (dir, marks)
}

#[test]
fn record_failure_keeps_the_version_and_one_notice() {
    let (_dir, marks) = marks();
    marks.record_failure("8.1.0").unwrap();
    assert_that!(marks.failed_version()).is_equal_to(Some("8.1.0".to_owned()));
    assert_that!(marks.notice()).is_equal_to(Some("8.1.0".to_owned()));
}

#[test]
fn notice_read_twice_is_kept_until_cleared() {
    let (_dir, marks) = marks();
    marks.record_failure("8.1.0").unwrap();
    let _ = marks.notice();
    assert_that!(marks.notice()).is_equal_to(Some("8.1.0".to_owned()));
}

#[test]
fn clear_notice_after_failure_forgets_it_but_keeps_the_failed_version() {
    let (_dir, marks) = marks();
    marks.record_failure("8.1.0").unwrap();
    marks.clear_notice().unwrap();
    assert_that!(marks.notice()).is_none();
    assert_that!(marks.failed_version()).is_equal_to(Some("8.1.0".to_owned()));
}

#[test]
fn notice_without_failure_returns_none() {
    let (_dir, marks) = marks();
    assert_that!(marks.notice()).is_none();
}

#[test]
fn notice_malformed_is_removed_from_disk() {
    let (dir, marks) = marks();
    std::fs::create_dir_all(dir.path().join("updates")).unwrap();
    std::fs::write(dir.path().join(FAILED_NOTICE_FILE), b"not a version").unwrap();
    assert_that!(marks.notice()).is_none();
    assert_that!(dir.path().join(FAILED_NOTICE_FILE).exists()).is_false();
}

#[test]
fn later_set_then_clear_round_trips() {
    let (_dir, marks) = marks();
    marks.set_later("8.2.0").unwrap();
    assert_that!(marks.later()).is_equal_to(Some("8.2.0".to_owned()));
    marks.clear_later().unwrap();
    assert_that!(marks.later()).is_none();
}

#[test]
fn clear_later_without_mark_is_ok() {
    let (_dir, marks) = marks();
    assert_that!(marks.clear_later()).is_ok();
}

#[test]
fn later_malformed_mark_reads_as_absent() {
    let (dir, marks) = marks();
    std::fs::create_dir_all(dir.path().join("updates")).unwrap();
    std::fs::write(dir.path().join(LATER_VERSION_FILE), b"<b>8.2</b>").unwrap();
    assert_that!(marks.later()).is_none();
}

#[test]
fn later_oversized_mark_reads_as_absent() {
    let (dir, marks) = marks();
    std::fs::create_dir_all(dir.path().join("updates")).unwrap();
    // A valid version padded past the cap: only the size limit refuses it (`trim` would not).
    let mut mark = b"8.2.0".to_vec();
    mark.extend([b' '; 100]);
    std::fs::write(dir.path().join(LATER_VERSION_FILE), mark).unwrap();
    assert_that!(marks.later()).is_none();
}
