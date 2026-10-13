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

#[test]
fn theme_ref_serializes_as_id_and_version() {
    let version = PackVersion::from_hex(&"b".repeat(64)).unwrap();
    let r = ThemeRef { id: Uuid::from_u128(3), version: version.clone() };
    let json = serde_json::to_value(&r).unwrap();
    assert_that!(json).is_equal_to(json!({ "id": Uuid::from_u128(3).to_string(), "version": version.as_str() }));
    assert_that!(serde_json::from_value::<ThemeRef>(json).unwrap()).is_equal_to(r);
}

/// Une version de thème : un `PackFiles` sans widget, au manifeste `theme.json`.
#[test]
fn theme_version_is_the_version_of_its_json_without_widgets() {
    let json = br#"{"id":"2a7d4c91-5e3b-4f08-b6a1-8c9e0d1f2a31","name":"Papier","tokens":{}}"#;
    let files = PackFiles::from_bytes(json, []).unwrap();
    assert_that!(files.widgets().is_empty()).is_true();
    assert_that!(files.version())
        .is_equal_to(PackFiles::new(object(serde_json::from_slice(json).unwrap()), BTreeMap::new()).version());
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

#[test]
fn pack_files_verified_with_matching_hex_returns_version_and_files() {
    let pack = PackFiles::new(object(json!({ "a": 1 })), files(&[("kpi", "w")]));
    let json = pack.pack_json().unwrap();
    let version = pack.version();
    let got = PackFiles::verified(version.hex(), &json, [(wf("kpi"), b"w".as_slice())]);
    assert_that!(got).is_equal_to(Some((version, pack)));
}

#[rstest]
#[case::bad_hex("not-a-hash", br#"{"a":1}"#.as_slice(), b"w".as_slice())]
#[case::wrong_content(&"0".repeat(64), br#"{"a":1}"#.as_slice(), b"w".as_slice())]
#[case::not_an_object("", b"[1]".as_slice(), b"w".as_slice())]
#[case::widget_changed("", br#"{"a":1}"#.as_slice(), b"x".as_slice())]
fn pack_files_verified_with_untrustworthy_input_returns_none(
    #[case] hex: &str,
    #[case] manifest: &[u8],
    #[case] widget: &[u8],
) {
    let pack = PackFiles::new(object(json!({ "a": 1 })), files(&[("kpi", "w")]));
    let hex = if hex.is_empty() { pack.version().hex().to_owned() } else { hex.to_owned() };
    assert_that!(PackFiles::verified(&hex, manifest, [(wf("kpi"), widget)])).is_none();
}

fn manifest(name: &str) -> PackManifest {
    PackManifest {
        id: Uuid::new_v4(),
        name: name.into(),
        description: String::new(),
        parent: None,
        created: "2026-10-01T10:00:00Z".into(),
        templates: Vec::new(),
        examples: BTreeMap::new(),
    }
}

#[test]
fn pack_manifest_round_trips_through_pack_files() {
    let m = manifest("Mien");
    let files = m.to_files(files(&[("kpi", "#let widget = none")])).unwrap();
    assert_that!(PackManifest::from_files(&files).unwrap()).is_equal_to(&m);
    assert_that!(files.widget(&wf("kpi"))).is_equal_to(Some("#let widget = none"));
}

#[test]
fn pack_manifest_with_a_template_round_trips_through_pack_files() {
    let mut m = manifest("Mien");
    m.templates.push(Template {
        id: "texte".into(),
        name: "Texte".into(),
        description: String::new(),
        slide: Slide::new("s-texte", "content"),
    });
    let files = m.to_files(BTreeMap::new()).unwrap();
    assert_that!(PackManifest::from_files(&files).unwrap().templates).is_equal_to(&m.templates);
}

#[test]
fn pack_manifest_without_uuid_id_is_refused() {
    let json = json!({ "id": "essentiel", "name": "E", "parent": null, "created": "x", "templates": [] });
    let files = PackFiles::new(object(json), BTreeMap::new());
    assert_that!(PackManifest::from_files(&files).is_err()).is_true();
}

#[test]
fn theme_manifest_round_trips_through_pack_files_without_widgets() {
    let m = ThemeManifest {
        id: Uuid::new_v4(),
        name: "Importé".into(),
        description: String::new(),
        parent: None,
        created: "2026-10-01T10:00:00Z".into(),
        tokens: object(json!({ "color": { "accent": "#2f5fb3" } })),
    };
    let files = m.to_files().unwrap();
    assert_that!(ThemeManifest::from_files(&files).unwrap()).is_equal_to(&m);
    assert_that!(files.widgets().len()).is_equal_to(0);
}

#[test]
fn now_created_is_utc_to_the_second() {
    let s = now_created();
    assert_that!(s.len()).is_equal_to(20);
    assert_that!(s.ends_with('Z') && s.as_bytes()[10] == b'T').is_true();
}

fn with_values(m: &mut PackManifest, widget: &str, values: Value) {
    m.examples.insert(wf(widget), object(values));
}

#[test]
fn pack_manifest_examples_round_trip_through_pack_files() {
    let mut m = manifest("Mien");
    with_values(&mut m, "kpi", json!({ "value": "12" }));
    let files = m.to_files(files(&[("kpi", "#let widget = none")])).unwrap();
    assert_that!(PackManifest::from_files(&files).unwrap()).is_equal_to(&m);
    assert_that!(files.manifest()["examples"]["kpi"]["value"].as_str()).is_equal_to(Some("12"));
}

#[test]
fn pack_manifest_examples_change_the_version() {
    let mut m = manifest("Mien");
    let before = m.to_files(BTreeMap::new()).unwrap().version();
    with_values(&mut m, "kpi", json!({ "value": "12" }));
    let after = m.to_files(BTreeMap::new()).unwrap().version();
    assert_that!(after).is_not_equal_to(before);
}

fn read_examples(examples: Value) -> PackManifest {
    let mut json = serde_json::to_value(manifest("Mien")).unwrap();
    json["examples"] = examples;
    PackManifest::from_files(&PackFiles::new(object(json), BTreeMap::new())).unwrap()
}

/// Un `pack.json` d'avant t-541 (une liste de cas nommés par widget) se lit : le
/// premier cas donne les valeurs du widget, les autres sont oubliés.
#[test]
fn pack_manifest_with_the_old_example_cases_keeps_the_first_ones_values() {
    let m = read_examples(json!({
        "kpi": [{ "name": "Court", "values": { "value": "12" } }, { "name": "Long", "values": { "value": "1 234" } }]
    }));
    assert_that!(m.examples.get(&wf("kpi")).cloned()).is_equal_to(Some(object(json!({ "value": "12" }))));
}

/// Un widget aux valeurs vides (ancien sans cas, premier cas sans valeurs, objet vide) n'a pas d'entrée.
#[test]
fn pack_manifest_with_empty_values_keeps_no_entry() {
    let m = read_examples(json!({ "kpi": [], "quote": [{ "name": "Exemple", "values": {} }], "carte": {} }));
    assert_that!(m.examples.is_empty()).is_true();
}

/// Une entrée mal formée est écartée ; les autres restent.
#[test]
fn pack_manifest_with_a_malformed_example_entry_keeps_the_others() {
    let m = read_examples(json!({ "Mauvais Nom": { "value": "1" }, "kpi": 3, "carte": { "title": "T" } }));
    assert_that!(m.examples.keys().cloned().collect::<Vec<_>>()).is_equal_to(vec![wf("carte")]);
}

/// Sans valeurs, `pack.json` n'a pas de clé `examples` : les versions existantes gardent leur empreinte.
#[test]
fn pack_manifest_without_examples_writes_no_examples_key() {
    let files = manifest("Mien").to_files(BTreeMap::new()).unwrap();
    assert_that!(files.manifest().contains_key("examples")).is_false();
}
