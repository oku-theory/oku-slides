//! Le vrai deck d'exemple (`fixtures/import/alto-wealth.pptx`, généré par PptxGenJS).

mod common;

use oku_slides_import::read_pptx;
use oku_slides_import::source::{Fill, ItemKind, Origin, SourceDeck, Spacing, WarningKind};
use speculoos::prelude::*;

fn alto() -> SourceDeck {
    read_pptx(&common::file("alto-wealth.pptx"), &common::limits())
        .unwrap_or_else(|e| panic!("lecture d'alto-wealth : {e}"))
}

#[test]
fn alto_reads_its_eight_slides_on_its_single_layout() {
    let d = alto();
    assert_that!(d.slides.len()).is_equal_to(8);
    assert_that!(d.slides.iter().all(|s| s.layout.as_deref() == Some("l1"))).is_true();
}

#[test]
fn alto_has_no_structural_warning() {
    let d = alto();
    let structural = d.warnings.iter().filter(|w| {
        matches!(w.kind, WarningKind::UnreadablePart(_) | WarningKind::MissingPart(_) | WarningKind::UnreadableItem(_))
    });
    assert_that!(structural.count()).is_equal_to(0);
}

#[test]
fn alto_kicker_keeps_letter_spacing_line_spacing_and_explicit_color() {
    let d = alto();
    let it =
        d.slides[2].items.iter().find(|i| i.name == "Text 0").unwrap_or_else(|| panic!("pas de « Text 0 » en slide 3"));
    let ItemKind::Text(tb) = &it.kind else { panic!("texte attendu") };
    let p = &tb.paragraphs[0];
    let r = &p.runs[0].style;
    assert_that!(p.runs[0].text.as_str()).is_equal_to("THE SHIFT");
    assert_that!((r.size, r.bold, r.char_spacing, r.font.value.as_str())).is_equal_to((9.5, true, 0.5, "Inter"));
    assert_that!((r.color.value.to_hex(), r.color.origin.clone(), p.style.line_spacing)).is_equal_to((
        "#00bef4".to_owned(),
        Origin::Explicit,
        Spacing::Points(13.3),
    ));
}

#[test]
fn alto_third_slide_background_is_explicit_navy() {
    let d = alto();
    assert_that!(matches!(&d.slides[2].background, Fill::Solid { color } if color.value.to_hex() == "#00154e"))
        .is_true();
}

#[test]
fn alto_repeated_images_are_stored_once() {
    let d = alto();
    // image-2-1, image-3-5, image-4-1 … image-8-1 : le même fichier de 6 334 octets.
    assert_that!(d.assets.values().filter(|a| a.bytes.len() == 6334).count()).is_equal_to(1);
}

#[test]
fn alto_uses_inter_without_embedding_it() {
    let d = alto();
    let inter = d.fonts.iter().find(|f| f.family == "Inter").unwrap_or_else(|| panic!("Inter attendue"));
    assert_that!(inter.faces.is_empty()).is_true();
}

#[test]
fn alto_summary_matches() {
    let d = alto();
    let lines: Vec<String> = d
        .slides
        .iter()
        .map(|s| {
            let kinds: Vec<&str> = s
                .items
                .iter()
                .map(|i| match &i.kind {
                    ItemKind::Text(_) => "text",
                    ItemKind::Shape(_) => "shape",
                    ItemKind::Picture(_) => "picture",
                    _ => "other",
                })
                .collect();
            format!("{}: {}", s.id, kinds.join(" "))
        })
        .chain(d.warnings.iter().map(|w| format!("warning: {:?}", w.kind)))
        .collect();
    insta::assert_json_snapshot!("alto_summary", lines);
}
