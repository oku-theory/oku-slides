use lopdf::dictionary;
use speculoos::prelude::*;

use super::*;

#[test]
fn resource_missing_key_absent_broken_reference_err() {
    let mut pdf = Document::with_version("1.5");
    let image = pdf.add_object(dictionary! { "Subtype" => "Image" });
    let found = dictionary! { "XObject" => dictionary! { "Im1" => image } };
    let broken = dictionary! { "XObject" => dictionary! { "Im1" => Object::Reference((999, 0)) } };
    let none = dictionary! { "Font" => dictionary! {} };
    assert_that!(resource(&pdf, &[&none], b"XObject", b"Im1")).is_equal_to(Ok(None));
    let hit = resource(&pdf, &[&none, &found], b"XObject", b"Im1").map(|r| r.and_then(|(id, _)| id));
    assert_that!(hit).is_equal_to(Ok(Some(image)));
    assert_that!(resource(&pdf, &[&broken, &found], b"XObject", b"Im1").map(|_| ())).is_equal_to(Err(()));
    let dangling = dictionary! { "XObject" => Object::Reference((998, 0)) };
    assert_that!(resource(&pdf, &[&dangling], b"XObject", b"Im1").map(|_| ())).is_equal_to(Err(()));
}

#[test]
fn failure_limit_over_other_unreadable() {
    let limit = Error::Decompress(DecompressError::MemoryLimitExceeded { limit: 1 });
    assert_that!(failure(&limit)).is_equal_to(Failure::Over);
    assert_that!(failure(&Error::MissingXrefEntry)).is_equal_to(Failure::Unreadable);
}
