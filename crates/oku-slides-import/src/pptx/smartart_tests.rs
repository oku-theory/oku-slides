use speculoos::prelude::*;

use crate::pptx::test_support::{DATA, DRAWING, SLIDE_RELS, TestPkg, tree, tree_in};
use crate::source::{ItemKind, OpaqueReason, WarningKind};

/// Cadre SmartArt en (100, 100) pt, 250 × 50 pt.
const FRAME: &str = r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="15" name="Diagramme"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="1270000" y="1270000"/><a:ext cx="3175000" cy="635000"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/diagram"><dgm:relIds xmlns:dgm="urn:dgm" r:dm="rId6" r:lo="rId6" r:qs="rId6" r:cs="rId6"/></a:graphicData></a:graphic></p:graphicFrame>"#;

fn group_kids(items: &[crate::pptx::shapes::ParsedItem]) -> Vec<crate::source::Item> {
    match &items[0].item.kind {
        ItemKind::Group { items } => items.clone(),
        k => panic!("groupe attendu, trouvé {k:?}"),
    }
}

#[test]
fn smartart_becomes_a_group_identified_by_model_ids() {
    let (items, _, _) = tree(FRAME);
    assert_that!(group_kids(&items).iter().map(|k| k.id.clone()).collect::<Vec<_>>())
        .is_equal_to(vec!["s1/15/{A}".to_owned(), "s1/15/{B}".to_owned()]);
}

#[test]
fn smartart_shapes_are_placed_relative_to_the_frame() {
    let (items, _, _) = tree(FRAME);
    // 2ᵉ forme à 1 905 000 EMU = 150 pt du cadre (100, 100) → (250, 100).
    let k = &group_kids(&items)[1];
    assert_that!((k.frame.x, k.frame.y)).is_equal_to((250.0, 100.0));
}

#[test]
fn smartart_inside_a_scaled_group_keeps_the_group_scale() {
    // Groupe d'échelle 2 posé en (0,0) ; cadre à (50,50) pt dans le groupe → (100,100) ; la 2ᵉ forme à 150 pt × 2 = 300 pt du cadre.
    let frame = FRAME.replace(r#"<a:off x="1270000" y="1270000"/>"#, r#"<a:off x="635000" y="635000"/>"#);
    let g = format!(
        r#"<p:grpSp><p:nvGrpSpPr><p:cNvPr id="30" name="G"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="25400000" cy="25400000"/><a:chOff x="0" y="0"/><a:chExt cx="12700000" cy="12700000"/></a:xfrm></p:grpSpPr>{frame}</p:grpSp>"#
    );
    let (items, _, _) = tree(&g);
    let ItemKind::Group { items: shapes } = &group_kids(&items)[0].kind else { panic!("SmartArt attendu") };
    assert_that!((shapes[1].frame.x, shapes[1].frame.w)).is_equal_to((400.0, 200.0));
}

#[test]
fn smartart_with_unknown_relationship_is_opaque_and_warns() {
    let (items, w, _) = tree(&FRAME.replace(r#"r:dm="rId6""#, r#"r:dm="rIdNone""#));
    assert_that!(matches!(&items[0].item.kind, ItemKind::Opaque(o) if o.reason == OpaqueReason::SmartArt)).is_true();
    assert_that!(w.iter().any(|w| matches!(w.kind, WarningKind::MissingPart(_)))).is_true();
}

#[test]
fn smartart_whose_drawing_file_is_absent_warns_missing_part() {
    let t = TestPkg::with(&[
        ("ppt/slides/_rels/slide1.xml.rels", SLIDE_RELS.as_bytes()),
        ("ppt/diagrams/data1.xml", DATA.as_bytes()),
    ]);
    let (items, w, _) = tree_in(&t, FRAME);
    assert_that!(matches!(&items[0].item.kind, ItemKind::Opaque(o) if o.reason == OpaqueReason::SmartArt)).is_true();
    assert_that!(w.iter().any(|w| matches!(&w.kind, WarningKind::MissingPart(p) if p.ends_with("drawing1.xml"))))
        .is_true();
}

#[test]
fn smartart_inside_its_own_drawing_is_not_followed() {
    // Fichier hostile : le dessin contient lui-même un cadre SmartArt qui pointe vers ce même dessin.
    let looping = DRAWING
        .replace("</dsp:spTree>", &format!("{}</dsp:spTree>", FRAME.replace("<p:", "<dsp:").replace("</p:", "</dsp:")));
    let t = TestPkg::with(&[
        ("ppt/slides/_rels/slide1.xml.rels", SLIDE_RELS.as_bytes()),
        ("ppt/diagrams/data1.xml", DATA.as_bytes()),
        ("ppt/diagrams/drawing1.xml", looping.as_bytes()),
    ]);
    let (items, w, _) = tree_in(&t, FRAME);
    assert_that!(
        group_kids(&items).iter().any(|k| matches!(&k.kind, ItemKind::Opaque(o) if o.reason == OpaqueReason::SmartArt))
    )
    .is_true();
    assert_that!(w.iter().any(|w| matches!(w.kind, WarningKind::NestingTooDeep(_)))).is_true();
}

#[test]
fn smartart_drawing_over_the_node_limit_is_fatal_not_swallowed() {
    use crate::ReadLimits;
    use crate::pptx::package::Package;
    use crate::pptx::test_support::{LIMITS, run_ctx, slide_pkg};
    let t = slide_pkg(&[]);
    // Assez pour les relations et la part de données, pas pour le dessin.
    let limits = ReadLimits { max_xml_nodes: 30, ..LIMITS };
    let pkg = Package::open(t.bytes(), &limits).unwrap();
    let xml = format!(
        r#"<p:spTree xmlns:p="urn:p" xmlns:a="urn:a" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><p:nvGrpSpPr/><p:grpSpPr/>{FRAME}</p:spTree>"#
    );
    let doc = crate::pptx::xml::parse(&xml, "t", LIMITS.max_xml_nodes).unwrap();
    let r = run_ctx(&pkg, "ppt/slides/slide1.xml", None, |cx| {
        crate::pptx::shapes::parse_tree(doc.root_element(), cx, &crate::pptx::shapes::GroupCtx::ROOT)
    });
    assert_that!(r.err().map(|e| e.kind())).is_equal_to(Some(crate::ImportErrorKind::TooLarge));
}

#[test]
fn ole_object_is_opaque_with_its_picture() {
    let (items, _, assets) = tree(
        r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="16" name="Objet"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="0" y="0"/><a:ext cx="12700" cy="12700"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/presentationml/2006/ole"><p:oleObj r:id="rIdX" progId="Excel.Sheet.12"><p:embed/><p:pic><p:nvPicPr><p:cNvPr id="0" name=""/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rId2"/></p:blipFill><p:spPr/></p:pic></p:oleObj></a:graphicData></a:graphic></p:graphicFrame>"#,
    );
    let ItemKind::Opaque(o) = &items[0].item.kind else { panic!("opaque attendu") };
    assert_that!((
        o.reason.clone(),
        o.fallback.as_ref().is_some_and(|a| assets.get(a).is_some_and(|x| x.mime == "image/png"))
    ))
    .is_equal_to((OpaqueReason::Ole, true));
}

#[test]
fn smartart_with_unreadable_drawing_rels_is_still_drawn_and_warns() {
    let t = crate::pptx::test_support::slide_pkg(&[("ppt/diagrams/_rels/drawing1.xml.rels", b"<Relationships")]);
    let (items, w, _) = tree_in(&t, FRAME);
    assert_that!(group_kids(&items).len()).is_equal_to(2);
    assert_that!(
        w.iter().any(|w| matches!(&w.kind, WarningKind::UnreadableItem(p) if p.ends_with("drawing1.xml: rels")))
    )
    .is_true();
}
