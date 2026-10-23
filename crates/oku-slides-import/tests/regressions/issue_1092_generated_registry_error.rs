//! t-1092 : un registre cible dont un widget n'a pas de nom court (`a/b`) faisait échouer
//! `debug_assert!` dans `hifi::project` ; en production, le projecteur se rabattait en silence
//! sur le registre cible. L'échec est maintenant une erreur d'import.

use oku_slides_core::Namespace;
use oku_slides_import::fonts::FontCatalog;
use oku_slides_import::{HifiTarget, ImportErrorKind, PackLimits, hifi, read_pptx};
use serde_json::json;
use speculoos::prelude::*;
use uuid::Uuid;

use crate::common;

#[test]
fn issue_1092_registry_with_a_nested_widget_name_is_a_generated_registry_error() {
    let source = read_pptx(&common::pptx("kitchen"), &common::limits()).unwrap_or_else(|e| panic!("lecture : {e}"));
    // `acme/a/b` : l'espace de noms ne se déduit plus du nom, `probe_name` refuse.
    let registry = common::registry_plus(vec![json!({
        "namespace": "acme", "kind": "widgets",
        "widgets": [{ "name": "a/b", "category": "element", "params": [] }]
    })]);
    let fonts = FontCatalog::new();
    let target = HifiTarget {
        registry: &registry,
        base: common::base_pin(),
        essentiel: common::essentiel_pin(),
        fonts: &fonts,
        pack_id: Uuid::from_u128(1),
        theme: Namespace::parse("neutre").unwrap_or_else(|| panic!("espace de noms")),
        name: "Import".to_owned(),
        created: "2026-10-23T08:00:00Z".to_owned(),
        limits: PackLimits::default(),
    };
    let kind = hifi::project(&source, &target).map(|_| ()).map_err(|e| e.kind());
    assert_that!(kind).is_equal_to(Err(ImportErrorKind::GeneratedRegistry));
}
