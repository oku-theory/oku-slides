//! Régression (t-753) : seule l'entrée était bornée ; un petit zip pouvait gonfler en un gros deck Oku.

use oku_slides_import::fonts::FontCatalog;
use oku_slides_import::{HifiTarget, ImportErrorKind, ImportPlan, PackLimits, ReadLimits, hifi, read_pptx};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::common;

/// Un `.pptx` minimal dont la slide 1 porte 40 zones d'un même texte répété, très compressible.
fn inflating_pptx() -> Vec<u8> {
    let text = "Oku ".repeat(2_000);
    let shapes: String = (100..140)
        .map(|id| {
            format!(
                r#"<p:sp><p:nvSpPr><p:cNvPr id="{id}" name="Texte {id}"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1270000" cy="1270000"/></a:xfrm><a:prstGeom prst="rect"/></p:spPr><p:txBody><a:bodyPr/><a:p><a:r><a:t>{text}</a:t></a:r></a:p></p:txBody></p:sp>"#
            )
        })
        .collect();
    common::minimal_slide1(&shapes, "")
}

/// Le plan hi-fi de `bytes`, sans police disponible.
fn plan(bytes: &[u8], limits: &ReadLimits) -> ImportPlan {
    let source = read_pptx(bytes, limits).unwrap_or_else(|e| panic!("lecture : {e}"));
    let (registry, fonts) = (common::registry(), FontCatalog::new());
    let target = HifiTarget {
        registry: &registry,
        fonts: &fonts,
        pack_id: Uuid::from_u128(1),
        theme_id: Uuid::from_u128(2),
        name: "Import".to_owned(),
        created: "2026-10-14T08:00:00Z".to_owned(),
        limits: PackLimits::default(),
    };
    hifi::project(&source, &target)
}

#[test]
fn import_output_beyond_budget_is_too_large() {
    let bytes = inflating_pptx();
    // La sortie dépasse dix fois l'entrée : un budget à cette taille refuse le plan.
    let limits = ReadLimits { max_output_bytes: 10 * bytes.len() as u64, ..common::limits() };
    let err = plan(&bytes, &limits).check_budget(&limits).map_err(|e| e.kind());
    assert_that!(err).is_equal_to(Err(ImportErrorKind::TooLarge));
}

#[test]
fn import_output_within_default_budget_is_ok() {
    let limits = common::limits();
    assert_that!(plan(&inflating_pptx(), &limits).check_budget(&limits).is_ok()).is_true();
}
