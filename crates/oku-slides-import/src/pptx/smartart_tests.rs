use speculoos::prelude::*;

use super::{MAX_NODE_DEPTH, MAX_NODES};
use crate::pptx::test_support::{DATA, DRAWING, SLIDE_RELS, TestPkg, tree, tree_in};
use crate::source::{ItemKind, OpaqueReason, SmartArt, WarningKind};

/// Cadre SmartArt en (100, 100) pt, 250 × 50 pt.
const FRAME: &str = r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="15" name="Diagramme"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="1270000" y="1270000"/><a:ext cx="3175000" cy="635000"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/diagram"><dgm:relIds xmlns:dgm="urn:dgm" r:dm="rId6" r:lo="rId6" r:qs="rId6" r:cs="rId6"/></a:graphicData></a:graphic></p:graphicFrame>"#;

fn group_kids(items: &[crate::pptx::shapes::ParsedItem]) -> Vec<crate::source::Item> {
    match &items[0].item.kind {
        ItemKind::Group { items, .. } => items.clone(),
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
fn smartart_group_is_flagged_as_smartart() {
    let (items, _, _) = tree(FRAME);
    assert_that!(matches!(items[0].item.kind, ItemKind::Group { smartart: Some(_), .. })).is_true();
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
    let ItemKind::Group { items: shapes, .. } = &group_kids(&items)[0].kind else { panic!("SmartArt attendu") };
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

/// Un point de nœud `model` au texte `text`.
fn pt(model: &str, text: &str) -> String {
    format!(
        r#"<dgm:pt modelId="{model}"><dgm:prSet/><dgm:spPr/><dgm:t><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>{text}</a:t></a:r></a:p></dgm:t></dgm:pt>"#
    )
}

/// Un lien `parOf` de `src` vers `dest`, au rang `ord`.
fn par_of(src: &str, dest: &str, ord: u32) -> String {
    format!(r#"<dgm:cxn modelId="c{src}{dest}" srcId="{src}" destId="{dest}" srcOrd="{ord}" destOrd="0"/>"#)
}

/// Une part de données : document `{D}` (disposition `process1`), points `pts`, liens `cxns` ; `drawn` : avec dessin.
fn data(pts: &str, cxns: &str, drawn: bool) -> String {
    let ext = if drawn {
        r#"<dgm:extLst><a:ext uri="urn:x"><dsp:dataModelExt relId="rId7" minVer="urn:x"/></a:ext></dgm:extLst>"#
    } else {
        ""
    };
    format!(
        r#"<dgm:dataModel xmlns:dgm="urn:dgm" xmlns:a="urn:a" xmlns:dsp="urn:dsp"><dgm:ptLst><dgm:pt modelId="{{D}}" type="doc"><dgm:prSet loTypeId="urn:layout/process1" loCatId="process"/></dgm:pt>{pts}<dgm:pt modelId="{{P}}" type="parTrans"/><dgm:pt modelId="{{R}}" type="pres"><dgm:prSet presName="x"/></dgm:pt></dgm:ptLst><dgm:cxnLst>{cxns}</dgm:cxnLst>{ext}</dgm:dataModel>"#
    )
}

/// Le premier item lu de `FRAME` avec la part de données `data_xml`, le dessin et les parts `extra`.
fn read_with(data_xml: &str, drawing: bool, extra: &[(&str, &[u8])]) -> (ItemKind, Vec<crate::source::Warning>) {
    let mut parts: Vec<(&str, &[u8])> = vec![
        ("ppt/slides/_rels/slide1.xml.rels", SLIDE_RELS.as_bytes()),
        ("ppt/diagrams/data1.xml", data_xml.as_bytes()),
    ];
    if drawing {
        parts.push(("ppt/diagrams/drawing1.xml", DRAWING.as_bytes()));
    }
    parts.extend_from_slice(extra);
    let (items, w, _) = tree_in(&TestPkg::with(&parts), FRAME);
    (items[0].item.kind.clone(), w)
}

fn model_of(kind: &ItemKind) -> &SmartArt {
    match kind {
        ItemKind::SmartArt(sa) | ItemKind::Group { smartart: Some(sa), .. } => sa,
        k => panic!("SmartArt attendu, trouvé {k:?}"),
    }
}

/// (texte, profondeur) de chaque nœud, dans l'ordre.
fn outline(sa: &SmartArt) -> Vec<(String, u8)> {
    sa.nodes
        .iter()
        .map(|n| (n.text.paragraphs.iter().flat_map(|p| &p.runs).map(|r| r.text.as_str()).collect(), n.depth))
        .collect()
}

#[test]
fn smartart_without_drawing_keeps_its_node_texts() {
    let pts = format!("{}{}", pt("{1}", "Plan"), pt("{2}", "Build"));
    let xml = data(&pts, &format!("{}{}", par_of("{D}", "{1}", 0), par_of("{D}", "{2}", 1)), false);
    let (kind, w) = read_with(&xml, false, &[]);
    assert_that!(matches!(kind, ItemKind::SmartArt(_))).is_true();
    assert_that!(outline(model_of(&kind))).is_equal_to(vec![("Plan".to_owned(), 0), ("Build".to_owned(), 0)]);
    assert_that!(w.iter().any(|w| matches!(w.kind, WarningKind::MissingPart(_)))).is_false();
}

#[test]
fn smartart_node_order_follows_src_ord_then_depth_first() {
    // Fichier dans le désordre : les rangs `srcOrd` et l'arbre font l'ordre de lecture, pas `ptLst`.
    let pts = ["{c}", "{b1}", "{a}", "{b}", "{a1}", "{a2}"].map(|m| pt(m, m.trim_matches(['{', '}']))).concat();
    let cxns = [
        par_of("{D}", "{b}", 1),
        par_of("{a}", "{a2}", 1),
        par_of("{D}", "{c}", 2),
        par_of("{D}", "{a}", 0),
        par_of("{b}", "{b1}", 0),
        par_of("{a}", "{a1}", 0),
        r#"<dgm:cxn modelId="x" type="presOf" srcId="{c}" destId="{R}"/>"#.to_owned(),
    ]
    .concat();
    let (kind, _) = read_with(&data(&pts, &cxns, false), false, &[]);
    let names: Vec<(String, u8)> =
        ["a", "a1", "a2", "b", "b1", "c"].iter().zip([0, 1, 1, 0, 1, 0]).map(|(n, d)| ((*n).to_owned(), d)).collect();
    assert_that!(outline(model_of(&kind))).is_equal_to(names);
}

#[test]
fn smartart_orphan_nodes_follow_the_tree_in_file_order() {
    let pts = format!("{}{}{}", pt("{o}", "Orphan"), pt("{1}", "Root child"), pt("{2}", "Second"));
    let (kind, _) = read_with(&data(&pts, &par_of("{D}", "{1}", 0), false), false, &[]);
    let texts: Vec<String> = outline(model_of(&kind)).into_iter().map(|(t, _)| t).collect();
    assert_that!(texts).is_equal_to(vec!["Root child".to_owned(), "Orphan".to_owned(), "Second".to_owned()]);
}

#[test]
fn smartart_layout_and_category_come_from_the_doc_point_without_layout_part() {
    let (kind, _) = read_with(&data(&pt("{1}", "A"), "", false), false, &[]);
    let sa = model_of(&kind);
    assert_that!((sa.layout.as_deref(), sa.category.as_deref()))
        .is_equal_to((Some("urn:layout/process1"), Some("process")));
}

#[test]
fn smartart_layout_def_wins_over_the_doc_point() {
    let rels = SLIDE_RELS.replace(
        "</Relationships>",
        r#"<Relationship Id="rId9" Type="urn:diagramLayout" Target="../diagrams/layout1.xml"/></Relationships>"#,
    );
    let layout = r#"<dgm:layoutDef xmlns:dgm="urn:dgm" uniqueId="urn:layout/cycle2"><dgm:catLst><dgm:cat type="cycle" pri="1000"/></dgm:catLst></dgm:layoutDef>"#;
    let xml = data(&pt("{1}", "A"), "", false);
    let t = TestPkg::with(&[
        ("ppt/slides/_rels/slide1.xml.rels", rels.as_bytes()),
        ("ppt/diagrams/data1.xml", xml.as_bytes()),
        ("ppt/diagrams/layout1.xml", layout.as_bytes()),
    ]);
    let (items, _, _) = tree_in(&t, &FRAME.replace(r#"r:lo="rId6""#, r#"r:lo="rId9""#));
    let sa = model_of(&items[0].item.kind);
    assert_that!((sa.layout.as_deref(), sa.category.as_deref()))
        .is_equal_to((Some("urn:layout/cycle2"), Some("cycle")));
}

#[test]
fn smartart_drawn_group_carries_its_model_alongside() {
    let pts = format!("{}{}", pt("{A}", "Étape 1"), pt("{B}", "Étape 2"));
    let (kind, _) = read_with(&data(&pts, &par_of("{D}", "{A}", 0), true), true, &[]);
    let ItemKind::Group { items, smartart: Some(sa) } = &kind else { panic!("groupe dessiné attendu") };
    assert_that!(items.len()).is_equal_to(2);
    assert_that!(sa.nodes.iter().map(|n| n.id.clone()).collect::<Vec<_>>())
        .is_equal_to(vec!["{A}".to_owned(), "{B}".to_owned()]);
}

#[test]
fn smartart_without_drawing_nor_text_is_opaque_and_warns() {
    let (kind, w) = read_with(&data(&pt("{1}", " "), "", false), false, &[]);
    assert_that!(matches!(&kind, ItemKind::Opaque(o) if o.reason == OpaqueReason::SmartArt)).is_true();
    assert_that!(w.iter().any(|w| matches!(&w.kind, WarningKind::MissingPart(p) if p == "diagramDrawing"))).is_true();
}

#[test]
fn smartart_nodes_beyond_the_limit_are_dropped_and_warned() {
    let pts: String = (0..MAX_NODES + 5).map(|k| pt(&format!("{{n{k}}}"), "x")).collect();
    let (kind, w) = read_with(&data(&pts, "", false), false, &[]);
    assert_that!(model_of(&kind).nodes.len()).is_equal_to(MAX_NODES);
    assert_that!(w.iter().any(|w| matches!(w.kind, WarningKind::Approximated(_)))).is_true();
}

#[test]
fn smartart_node_tree_deeper_than_the_limit_is_cut_and_warned() {
    let n = usize::from(MAX_NODE_DEPTH) + 4;
    let pts: String = (0..n).map(|k| pt(&format!("{{n{k}}}"), "x")).collect();
    let mut cxns = par_of("{D}", "{n0}", 0);
    for k in 1..n {
        cxns.push_str(&par_of(&format!("{{n{}}}", k - 1), &format!("{{n{k}}}"), 0));
    }
    let (kind, w) = read_with(&data(&pts, &cxns, false), false, &[]);
    let depths: Vec<u8> = model_of(&kind).nodes.iter().map(|n| n.depth).collect();
    assert_that!(depths).is_equal_to((0..=MAX_NODE_DEPTH).collect::<Vec<_>>());
    assert_that!(w.iter().any(|w| matches!(w.kind, WarningKind::NestingTooDeep(_)))).is_true();
}

#[test]
fn smartart_cyclic_links_end_and_keep_every_node_once() {
    let pts = format!("{}{}", pt("{a}", "A"), pt("{b}", "B"));
    let cxns = format!("{}{}", par_of("{a}", "{b}", 0), par_of("{b}", "{a}", 0));
    let (kind, _) = read_with(&data(&pts, &cxns, false), false, &[]);
    assert_that!(outline(model_of(&kind))).is_equal_to(vec![("A".to_owned(), 0), ("B".to_owned(), 1)]);
}
