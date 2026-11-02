use std::collections::BTreeMap;

use rstest::rstest;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use super::*;
use crate::{PackFiles, PackKind, PackManifestErrorKind, WidgetFile};

const ID: &str = "6f1c1d8e-2a4b-4c5d-9e7f-0a1b2c3d4e5f";

fn object(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(m) => m,
        _ => panic!("not an object"),
    }
}

fn pack(kind: PackKind, manifest: Value, widgets: &[(&str, &str)]) -> PackFiles {
    let widgets: BTreeMap<WidgetFile, String> =
        widgets.iter().map(|(n, s)| (WidgetFile::parse(n).unwrap(), (*s).to_owned())).collect();
    PackFiles::new(kind, object(manifest), widgets)
}

fn strings(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_owned()).collect()
}

fn widgets_pack() -> PackFiles {
    pack(
        PackKind::Widgets,
        json!({
            "id": ID, "name": "Cartes", "namespace": "cartes", "requires": ["base"],
            "description": "Des cartes.", "created": "2026-11-01T10:00:00Z",
            "order": ["zeta"],
            "tokens": [
                {"path": ["card", "bg"], "control": "color", "default": "accent"},
                {"path": ["card", "font"], "control": "fonts", "default": ["Inter", "Libertinus Serif", "Inter"]},
            ],
        }),
        &[("_commun", "#let x = 1"), ("alpha", "#let a = 22"), ("zeta", "#let z = 333")],
    )
}

fn widgets_expected() -> Capabilities {
    Capabilities {
        kind: CapabilityKind::Widgets,
        name: "Cartes".into(),
        namespace: "cartes".into(),
        description: "Des cartes.".into(),
        items: strings(&["zeta", "alpha"]),
        requires: strings(&["base"]),
        tokens: strings(&["card.bg", "card.font"]),
        fonts: strings(&["Inter", "Libertinus Serif"]),
        profiles: 0,
        code_bytes: 10 + 11 + 12,
    }
}

fn layouts_pack() -> PackFiles {
    pack(
        PackKind::Layouts,
        json!({
            "id": ID, "name": "Mise en page", "namespace": "mise", "created": "2026-11-01T10:00:00Z",
            "profiles": [{"id": "texte", "name": "Texte", "layout": "content", "role": "content",
                "arrangement": "default", "blocks": 1, "title": true}],
        }),
        &[("content", "#let c = 1")],
    )
}

fn layouts_expected() -> Capabilities {
    Capabilities {
        kind: CapabilityKind::Layouts,
        name: "Mise en page".into(),
        namespace: "mise".into(),
        description: String::new(),
        items: strings(&["content"]),
        requires: Vec::new(),
        tokens: Vec::new(),
        fonts: Vec::new(),
        profiles: 1,
        code_bytes: 10,
    }
}

fn theme() -> PackFiles {
    PackFiles::theme(object(json!({
        "id": ID, "name": "Encre", "namespace": "encre", "extends": ["papier"],
        "description": "Un thème sombre.", "created": "2026-11-01T10:00:00Z",
        "tokens": {
            "color": {"accent": "#111"},
            "font": {"display": ["Newsreader", "Inter"], "body": ["Inter"]},
        },
    })))
}

fn theme_expected() -> Capabilities {
    Capabilities {
        kind: CapabilityKind::Theme,
        name: "Encre".into(),
        namespace: "encre".into(),
        description: "Un thème sombre.".into(),
        items: Vec::new(),
        requires: strings(&["papier"]),
        tokens: strings(&["color", "font"]),
        fonts: strings(&["Inter", "Newsreader"]),
        profiles: 0,
        code_bytes: 0,
    }
}

#[rstest]
#[case::widgets_pack(widgets_pack(), widgets_expected())]
#[case::layouts_pack(layouts_pack(), layouts_expected())]
#[case::theme(theme(), theme_expected())]
fn capabilities_of_version_lists_what_it_holds(#[case] files: PackFiles, #[case] expected: Capabilities) {
    assert_that!(Capabilities::of(&files)).is_ok().is_equal_to(expected);
}

#[rstest]
#[case::pack_without_namespace(pack(PackKind::Widgets, json!({"id": ID, "name": "X", "created": "c"}), &[]))]
#[case::theme_without_created(PackFiles::theme(object(json!({"id": ID, "name": "X", "namespace": "x", "tokens": {}}))))]
fn capabilities_of_unreadable_manifest_is_error(#[case] files: PackFiles) {
    let got = Capabilities::of(&files);
    assert_that!(got.as_ref().err().map(PackManifestError::kind)).is_equal_to(Some(PackManifestErrorKind::Json));
}

#[test]
fn capabilities_serialize_kind_in_snake_case() {
    let value = serde_json::to_value(theme_expected()).unwrap();
    assert_that!(value.get("kind")).is_equal_to(Some(&json!("theme")));
}

#[rstest]
#[case::widgets(PackKind::Widgets, CapabilityKind::Widgets)]
#[case::layouts(PackKind::Layouts, CapabilityKind::Layouts)]
fn capability_kind_from_pack_kind_is_the_same_kind(#[case] kind: PackKind, #[case] expected: CapabilityKind) {
    assert_that!(CapabilityKind::from(kind)).is_equal_to(expected);
}

#[rstest]
#[case::widgets(CapabilityKind::Widgets)]
#[case::layouts(CapabilityKind::Layouts)]
#[case::theme(CapabilityKind::Theme)]
fn capability_kind_as_str_is_its_serialized_name(#[case] kind: CapabilityKind) {
    assert_that!(serde_json::to_value(kind).ok()).is_equal_to(Some(json!(kind.as_str())));
}
