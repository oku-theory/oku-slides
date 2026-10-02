use rstest::rstest;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

use super::*;

fn object(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(m) => m,
        other => panic!("objet attendu : {other}"),
    }
}

#[test]
fn pack_version_of_known_content_is_stable() {
    // Canonique : {"a":[{"c":"x","d":2}],"b":1} 0x00 w
    let files = PackFiles::new(object(json!({ "b": 1, "a": [{ "d": 2, "c": "x" }] })), Some("w".into()));
    assert_that!(files.version().as_str())
        .is_equal_to("sha256:3356f340f7d5efcbd0e55f5f06a60729765c18942d666015444191d545839756");
}

#[test]
fn pack_version_without_widgets_hashes_an_empty_widgets_file() {
    let files = PackFiles::new(object(json!({ "b": 1, "a": [{ "d": 2, "c": "x" }] })), None);
    assert_that!(files.version().as_str())
        .is_equal_to("sha256:99b62953dd3d649f39732438515923d2db4f402b3e1ee7a15ab5b343a4974d3a");
    let empty = PackFiles::new(files.manifest().clone(), Some(String::new()));
    assert_that!(empty.version()).is_equal_to(files.version());
    assert_that!(empty.widgets()).is_none();
}

#[test]
fn pack_version_ignores_key_order_at_every_depth() {
    let a = PackFiles::new(object(json!({ "name": "P", "templates": [{ "id": "t", "name": "T" }] })), None);
    let b = PackFiles::new(object(json!({ "templates": [{ "name": "T", "id": "t" }], "name": "P" })), None);
    assert_that!(a.version()).is_equal_to(b.version());
}

#[test]
fn pack_version_changes_with_name_or_widgets() {
    let base = PackFiles::new(object(json!({ "name": "P" })), Some("#let registry = ()".into()));
    let renamed = PackFiles::new(object(json!({ "name": "Q" })), Some("#let registry = ()".into()));
    let other = PackFiles::new(object(json!({ "name": "P" })), Some("#let registry = () ".into()));
    assert_that!(renamed.version()).is_not_equal_to(base.version());
    assert_that!(other.version()).is_not_equal_to(base.version());
}

#[rstest]
#[case("3356f340f7d5efcbd0e55f5f06a60729765c18942d666015444191d545839756")]
#[case("sha256:3356F340F7D5EFCBD0E55F5F06A60729765C18942D666015444191D545839756")]
#[case("sha256:3356f340")]
#[case("sha256:gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg")]
#[case("sha256:../../../../../../../../../../../../../../../../../../../../../../")]
fn pack_version_parse_refuses_anything_but_lowercase_sha256(#[case] text: &str) {
    assert_that!(PackVersion::parse(text)).is_none();
}

#[test]
fn pack_version_hex_is_the_folder_name() {
    let v = PackVersion::from_hex(&"ab".repeat(32)).unwrap();
    assert_that!(v.hex()).is_equal_to(&*"ab".repeat(32));
    assert_that!(v.as_str()).starts_with("sha256:");
    assert_that!(PackVersion::parse(v.as_str())).is_equal_to(Some(v));
}

#[test]
fn pack_ref_round_trips_through_json() {
    let r = PackRef { id: Uuid::new_v4(), version: PackVersion::from_hex(&"0f".repeat(32)).unwrap() };
    let json = serde_json::to_value(&r).unwrap();
    assert_that!(json["version"].as_str()).is_equal_to(Some(r.version.as_str()));
    assert_that!(serde_json::from_value::<PackRef>(json).unwrap()).is_equal_to(r);
}

#[test]
fn pack_ref_with_a_bad_version_is_refused() {
    let json = json!({ "id": Uuid::new_v4(), "version": "sha256:court" });
    assert_that!(serde_json::from_value::<PackRef>(json).is_err()).is_true();
}

#[test]
fn pack_files_from_bytes_refuses_non_object_and_non_utf8() {
    assert_that!(PackFiles::from_bytes(b"[1]", None)).is_none();
    assert_that!(PackFiles::from_bytes(b"pas du json", None)).is_none();
    assert_that!(PackFiles::from_bytes(br#"{"name":"P"}"#, Some(&[0xff_u8, 0xfe][..]))).is_none();
    let files = PackFiles::from_bytes(br#"{"name":"P"}"#, Some(b"#let registry = ()")).unwrap();
    assert_that!(files.widgets()).is_equal_to(Some("#let registry = ()"));
}

#[test]
fn pack_files_pack_json_reads_back_to_the_same_version() {
    let files = PackFiles::new(object(json!({ "name": "P", "id": "x" })), Some("w".into()));
    let back = PackFiles::from_bytes(&files.pack_json().unwrap(), files.widgets().map(str::as_bytes)).unwrap();
    assert_that!(back.version()).is_equal_to(files.version());
}

#[test]
fn pack_files_matches_only_their_own_version() {
    let files = PackFiles::new(object(json!({ "name": "P" })), Some("w".into()));
    let tampered = PackFiles::new(object(json!({ "name": "P" })), Some("x".into()));
    assert_that!(files.matches(&files.version())).is_true();
    assert_that!(tampered.matches(&files.version())).is_false();
    assert_that!(files.version().verifies(&files)).is_true();
}

/// Un `pack.json` brut, espacé, avec un flottant, du non-ASCII et des échappements.
const SPACED: &[u8] = br#"{ "z" : [ 0.1 , -3 ],
  "ratio": 2.50,
  "name" : "Caf\u00e9 \"\u00dcn\u00ef\"\t\\ \u6f22" }"#;

/// t-328 : vecteur figé, calculé hors de Rust (`printf … | shasum -a 256`)
/// sur les octets canoniques ci-dessous.
#[test]
fn pack_version_of_floats_non_ascii_and_escapes_matches_a_frozen_vector() {
    let files = PackFiles::from_bytes(SPACED, Some(b"w")).unwrap();
    let canonical = "{\"name\":\"Café \\\"Ünï\\\"\\t\\\\ 漢\",\"ratio\":2.5,\"z\":[0.1,-3]}\0w";
    assert_that!(files.canonical()).is_equal_to(canonical.as_bytes().to_vec());
    assert_that!(files.version().as_str())
        .is_equal_to("sha256:d4b6f3456620b86d3b233252b1d2ba6594c2a57a2a325bb420293ac5dea9299f");
}

#[test]
fn pack_files_from_bytes_spaced_differently_give_the_same_version() {
    let compact = br#"{"name":"Caf\u00e9 \"\u00dcn\u00ef\"\t\\ \u6f22","ratio":2.5,"z":[0.1,-3]}"#;
    let pretty = "{\n    \"ratio\": 2.5e0,\n    \"name\": \"Café \\\"Ünï\\\"\\t\\\\ 漢\",\n    \"z\": [\n        1e-1,\n        -3\n    ]\n}\n";
    let a = PackFiles::from_bytes(SPACED, Some(b"w")).unwrap();
    let b = PackFiles::from_bytes(compact, Some(b"w")).unwrap();
    let c = PackFiles::from_bytes(pretty.as_bytes(), Some(b"w")).unwrap();
    assert_that!(b.version()).is_equal_to(a.version());
    assert_that!(c.version()).is_equal_to(a.version());
}
