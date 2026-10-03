use std::collections::BTreeMap;

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

fn wf(name: &str) -> WidgetFile {
    WidgetFile::parse(name).unwrap_or_else(|| panic!("nom invalide : {name}"))
}

/// Des fichiers de widgets, par nom.
fn files(entries: &[(&str, &str)]) -> BTreeMap<WidgetFile, String> {
    entries.iter().map(|(n, s)| (wf(n), (*s).to_owned())).collect()
}

#[test]
fn pack_version_of_known_content_is_stable() {
    // Canonique : {"a":[{"c":"x","d":2}],"b":1} 0 kpi 0 w 0
    let pack = PackFiles::new(object(json!({ "b": 1, "a": [{ "d": 2, "c": "x" }] })), files(&[("kpi", "w")]));
    assert_that!(pack.version().as_str())
        .is_equal_to("sha256:e74910e70629b0f16334d49b9a4ce504ea98c4d68e87510241480412ac18e73d");
}

/// t-329 : vecteur figé, calculé hors de Rust (`printf … | shasum -a 256`).
#[test]
fn pack_version_of_several_files_follows_name_order_and_matches_a_frozen_vector() {
    let pack = PackFiles::new(
        object(json!({ "b": 1, "a": [{ "d": 2, "c": "x" }] })),
        files(&[("kpi", "K"), ("_commun", "C")]),
    );
    let canonical = "{\"a\":[{\"c\":\"x\",\"d\":2}],\"b\":1}\0_commun\0C\0kpi\0K\0";
    assert_that!(pack.canonical()).is_equal_to(canonical.as_bytes().to_vec());
    assert_that!(pack.version().as_str())
        .is_equal_to("sha256:b6aaacfa8364494dd04f71654aa607b38c26ef327b5a8a36e06fc100c644608d");
}

#[test]
fn pack_version_without_widgets_hashes_the_manifest_and_one_zero() {
    let pack = PackFiles::new(object(json!({ "b": 1, "a": [{ "d": 2, "c": "x" }] })), BTreeMap::new());
    assert_that!(pack.version().as_str())
        .is_equal_to("sha256:99b62953dd3d649f39732438515923d2db4f402b3e1ee7a15ab5b343a4974d3a");
    let empty = PackFiles::new(pack.manifest().clone(), files(&[("kpi", "")]));
    assert_that!(empty.version()).is_equal_to(pack.version());
    assert_that!(empty.widgets().is_empty()).is_true();
}

#[test]
fn pack_version_ignores_key_order_at_every_depth() {
    let a = PackFiles::new(object(json!({ "name": "P", "templates": [{ "id": "t", "name": "T" }] })), BTreeMap::new());
    let b = PackFiles::new(object(json!({ "templates": [{ "name": "T", "id": "t" }], "name": "P" })), BTreeMap::new());
    assert_that!(a.version()).is_equal_to(b.version());
}

#[test]
fn pack_version_changes_with_name_source_or_file_name() {
    let base = PackFiles::new(object(json!({ "name": "P" })), files(&[("kpi", "#let widget = none")]));
    let renamed = PackFiles::new(object(json!({ "name": "Q" })), files(&[("kpi", "#let widget = none")]));
    let other = PackFiles::new(object(json!({ "name": "P" })), files(&[("kpi", "#let widget = none ")]));
    let moved = PackFiles::new(object(json!({ "name": "P" })), files(&[("kpj", "#let widget = none")]));
    for changed in [renamed, other, moved] {
        assert_that!(changed.version()).is_not_equal_to(base.version());
    }
}

#[rstest]
#[case("kpi")]
#[case("_commun")]
#[case("kpi-2")]
#[case("a")]
fn widget_file_parse_accepts_lowercase_names_and_common(#[case] name: &str) {
    assert_that!(WidgetFile::parse(name).map(|w| w.as_str().to_owned())).is_equal_to(Some(name.to_owned()));
}

#[rstest]
#[case("Kpi")]
#[case("../x")]
#[case("_autre")]
#[case("")]
#[case("2kpi")]
#[case("kpi.typ")]
#[case("k_pi")]
#[case("é")]
#[case(&"a".repeat(65))]
fn widget_file_parse_refuses_anything_else(#[case] name: &str) {
    assert_that!(WidgetFile::parse(name)).is_none();
}

#[test]
fn widget_file_round_trips_through_json_and_refuses_a_bad_name() {
    let json = serde_json::to_value(wf("kpi")).unwrap();
    assert_that!(json.clone()).is_equal_to(json!("kpi"));
    assert_that!(serde_json::from_value::<WidgetFile>(json).unwrap()).is_equal_to(wf("kpi"));
    assert_that!(serde_json::from_value::<WidgetFile>(json!("Kpi")).is_err()).is_true();
}

#[test]
fn widget_file_file_name_adds_and_strips_typ() {
    assert_that!(wf("kpi").file_name()).is_equal_to("kpi.typ".to_owned());
    assert_that!(WidgetFile::from_file_name("_commun.typ")).is_equal_to(Some(WidgetFile::common()));
    assert_that!(WidgetFile::from_file_name("kpi")).is_none();
    assert_that!(WidgetFile::from_file_name("../x.typ")).is_none();
    assert_that!(WidgetFile::common().is_common()).is_true();
    assert_that!(wf("kpi").is_common()).is_false();
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
    assert_that!(PackFiles::from_bytes(b"[1]", [])).is_none();
    assert_that!(PackFiles::from_bytes(b"pas du json", [])).is_none();
    assert_that!(PackFiles::from_bytes(br#"{"name":"P"}"#, [(wf("kpi"), &[0xff_u8, 0xfe][..])])).is_none();
    let pack = PackFiles::from_bytes(br#"{"name":"P"}"#, [(wf("kpi"), &b"#let widget = none"[..])]).unwrap();
    assert_that!(pack.widget(&wf("kpi"))).is_equal_to(Some("#let widget = none"));
    assert_that!(pack.widget(&wf("autre"))).is_none();
}

#[test]
fn pack_files_pack_json_reads_back_to_the_same_version() {
    let pack = PackFiles::new(object(json!({ "name": "P", "id": "x" })), files(&[("kpi", "w"), ("_commun", "c")]));
    let widgets = pack.widgets().iter().map(|(n, s)| (n.clone(), s.as_bytes()));
    let back = PackFiles::from_bytes(&pack.pack_json().unwrap(), widgets).unwrap();
    assert_that!(back.version()).is_equal_to(pack.version());
}

#[test]
fn pack_files_matches_only_their_own_version() {
    let pack = PackFiles::new(object(json!({ "name": "P" })), files(&[("kpi", "w")]));
    let tampered = PackFiles::new(object(json!({ "name": "P" })), files(&[("kpi", "x")]));
    assert_that!(pack.matches(&pack.version())).is_true();
    assert_that!(tampered.matches(&pack.version())).is_false();
    assert_that!(pack.version().verifies(&pack)).is_true();
}

/// Un `pack.json` brut, espacé, avec un flottant, du non-ASCII et des échappements.
const SPACED: &[u8] = r#"{ "z" : [ 0.1 , -3 ],
  "ratio": 2.50,
  "name" : "Café \"Ünï\"\t\\ 漢" }"#
    .as_bytes();

/// t-329 : vecteur figé, calculé hors de Rust (`printf … | shasum -a 256`)
/// sur les octets canoniques ci-dessous.
#[test]
fn pack_version_of_floats_non_ascii_and_escapes_matches_a_frozen_vector() {
    let pack = PackFiles::from_bytes(SPACED, [(wf("kpi"), &b"w"[..])]).unwrap();
    let canonical = "{\"name\":\"Café \\\"Ünï\\\"\\t\\\\ 漢\",\"ratio\":2.5,\"z\":[0.1,-3]}\0kpi\0w\0";
    assert_that!(pack.canonical()).is_equal_to(canonical.as_bytes().to_vec());
    assert_that!(pack.version().as_str())
        .is_equal_to("sha256:e855c082bca6d0f2ef7a4e107750238ea482de41a53a8e92cc57f2787287e928");
}

#[test]
fn pack_files_from_bytes_spaced_differently_give_the_same_version() {
    let compact = r#"{"name":"Café \"Ünï\"\t\\ 漢","ratio":2.5,"z":[0.1,-3]}"#.as_bytes();
    let pretty = "{\n    \"ratio\": 2.5e0,\n    \"name\": \"Café \\\"Ünï\\\"\\t\\\\ 漢\",\n    \"z\": [\n        1e-1,\n        -3\n    ]\n}\n";
    let a = PackFiles::from_bytes(SPACED, [(wf("kpi"), &b"w"[..])]).unwrap();
    let b = PackFiles::from_bytes(compact, [(wf("kpi"), &b"w"[..])]).unwrap();
    let c = PackFiles::from_bytes(pretty.as_bytes(), [(wf("kpi"), &b"w"[..])]).unwrap();
    assert_that!(b.version()).is_equal_to(a.version());
    assert_that!(c.version()).is_equal_to(a.version());
}

#[test]
fn pack_files_from_bytes_refuses_a_widget_source_with_nul() {
    let json = br#"{"name":"P"}"#;
    assert_that!(PackFiles::from_bytes(json, [(wf("kpi"), &b"w\0b\0x"[..])])).is_none();
    // Sans NUL, aucune lecture ne produit le contenu canonique de deux fichiers.
    let two = PackFiles::from_bytes(json, [(wf("b"), &b"x"[..]), (wf("kpi"), &b"w"[..])]).unwrap();
    let forged = PackFiles::from_bytes(json, [(wf("b"), &b"x\0kpi\0w"[..])]);
    assert_that!(forged).is_none();
    assert_that!(PackFiles::from_bytes(json, [(wf("b"), &b"x"[..])]).unwrap().version()).is_not_equal_to(two.version());
}
