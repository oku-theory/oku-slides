use oku_slides_core::PackVersion;
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_platform::storage::Storage;
use speculoos::prelude::*;
use uuid::Uuid;

use crate::SessionErrorKind;
use crate::guide_approvals::{GuideApprovals, VersionRef};
use crate::test_support::{fresh_dir, logged};

const ID: Uuid = Uuid::from_u128(7);

fn storage() -> Storage {
    Storage::local().unwrap()
}

fn version(tag: &str) -> PackVersion {
    PackVersion::from_hex(&tag.repeat(64)).unwrap()
}

fn vref(id: Uuid, tag: &str) -> VersionRef {
    VersionRef { id, version: version(tag) }
}

#[test]
fn approve_then_load_is_approved() {
    let dir = fresh_dir("oku-slides-approvals-");
    let path = dir.path().join("guide-approvals.json");
    GuideApprovals::load(Some(path.clone()), storage()).approve(&[vref(ID, "a")]).unwrap();
    assert_that!(GuideApprovals::load(Some(path), storage()).is_approved(ID, &version("a"))).is_true();
}

#[test]
fn load_unreadable_file_approves_nothing() {
    let dir = fresh_dir("oku-slides-approvals-");
    let path = dir.path().join("guide-approvals.json");
    std::fs::write(&path, b"not json").unwrap();
    let mut approvals = None;
    let log = logged(|| approvals = Some(GuideApprovals::load(Some(path), storage())));
    assert_that!(approvals.unwrap().is_approved(ID, &version("a"))).is_false();
    assert_that!(log.contains("guide approvals unreadable")).is_true();
}

#[test]
fn load_missing_file_approves_nothing_silently() {
    let dir = fresh_dir("oku-slides-approvals-");
    let path = dir.path().join("guide-approvals.json");
    let mut approved = true;
    let log = logged(|| approved = GuideApprovals::load(Some(path), storage()).is_approved(ID, &version("a")));
    assert_that!(approved).is_false();
    assert_that!(log.contains("unreadable")).is_false();
}

#[test]
fn approve_without_path_stays_in_memory() {
    let a = GuideApprovals::load(None, storage());
    a.approve(&[vref(ID, "a")]).unwrap();
    assert_that!(a.is_approved(ID, &version("a"))).is_true();
}

#[test]
fn approve_writes_a_sorted_array_once() {
    let dir = fresh_dir("oku-slides-approvals-");
    let path = dir.path().join("guide-approvals.json");
    let a = GuideApprovals::load(Some(path.clone()), storage());
    a.approve(&[vref(ID, "b"), vref(ID, "a")]).unwrap();
    let list: Vec<VersionRef> = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let mut sorted = list.clone();
    sorted.sort();
    assert_that!(list.len()).is_equal_to(2);
    assert_that!(list).is_equal_to(sorted);
}

#[test]
fn approve_failed_write_is_a_file_error_and_approves_nothing() {
    let dir = fresh_dir("oku-slides-approvals-");
    // The path is a folder: it cannot be written as a file.
    let path = dir.path().join("guide-approvals.json");
    std::fs::create_dir(&path).unwrap();
    let a = GuideApprovals::load(Some(path), storage());
    let e = a.approve(&[vref(ID, "a")]).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::File);
    assert_that!(a.is_approved(ID, &version("a"))).is_false();
}

#[test]
fn approve_failed_write_error_is_localized_in_french() {
    let dir = fresh_dir("oku-slides-approvals-");
    let path = dir.path().join("guide-approvals.json");
    std::fs::create_dir(&path).unwrap();
    let e = GuideApprovals::load(Some(path), storage()).approve(&[vref(ID, "a")]).unwrap_err();
    let text = e.localize(&Localizer::new(Some("fr")));
    assert_that!(text.as_str()).contains("approbations de guides");
    assert_that!(text.as_str()).contains("guide-approvals.json");
}

#[test]
fn approve_from_two_loads_of_one_file_keeps_both() {
    let dir = fresh_dir("oku-slides-approvals-");
    let path = dir.path().join("guide-approvals.json");
    let (a, b) =
        (GuideApprovals::load(Some(path.clone()), storage()), GuideApprovals::load(Some(path.clone()), storage()));
    a.approve(&[vref(ID, "a")]).unwrap();
    b.approve(&[vref(ID, "b")]).unwrap();
    let third = GuideApprovals::load(Some(path.clone()), storage());
    let seen = |x: &GuideApprovals| (x.is_approved(ID, &version("a")), x.is_approved(ID, &version("b")));
    assert_that!([seen(&a), seen(&b), seen(&third)]).is_equal_to([(true, true); 3]);
    let list: Vec<VersionRef> = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_that!(list).is_equal_to(vec![vref(ID, "a"), vref(ID, "b")]);
}

#[test]
fn load_without_path_stays_private() {
    let (a, b) = (GuideApprovals::load(None, storage()), GuideApprovals::load(None, storage()));
    a.approve(&[vref(ID, "a")]).unwrap();
    assert_that!(b.is_approved(ID, &version("a"))).is_false();
}
