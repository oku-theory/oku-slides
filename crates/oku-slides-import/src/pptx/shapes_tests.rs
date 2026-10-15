use speculoos::prelude::*;

use crate::pptx::test_support::{PNG, SLIDE_RELS, TestPkg, tree, tree_in};
use crate::source::{Connection, Fill, ItemKind, OpaqueReason, Origin, Site, WarningKind};

const TEXTBOX: &str = r#"<p:sp><p:nvSpPr><p:cNvPr id="4" name="ZoneTexte 3" descr="Légende"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="1270000" y="635000"/><a:ext cx="2540000" cy="508000"/></a:xfrm><a:prstGeom prst="rect"/><a:noFill/></p:spPr><p:txBody><a:bodyPr/><a:p><a:r><a:t>Bonjour</a:t></a:r></a:p></p:txBody></p:sp>"#;
const PT: i64 = 12_700;
const P14: &str = r#"xmlns:p14="http://schemas.microsoft.com/office/powerpoint/2010/main""#;

fn textbox_at(id: u32, off: (i64, i64), ext: (i64, i64)) -> String {
    TEXTBOX.replace(r#"id="4""#, &format!(r#"id="{id}""#)).replace(
        r#"<a:off x="1270000" y="635000"/><a:ext cx="2540000" cy="508000"/>"#,
        &format!(r#"<a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/>"#, off.0, off.1, ext.0, ext.1),
    )
}

fn group(id: u32, off: (i64, i64), ext: (i64, i64), ch_off: (i64, i64), ch_ext: (i64, i64), inner: &str) -> String {
    format!(
        r#"<p:grpSp><p:nvGrpSpPr><p:cNvPr id="{id}" name="Groupe {id}"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/><a:chOff x="{}" y="{}"/><a:chExt cx="{}" cy="{}"/></a:xfrm></p:grpSpPr>{inner}</p:grpSp>"#,
        off.0, off.1, ext.0, ext.1, ch_off.0, ch_off.1, ch_ext.0, ch_ext.1
    )
}

fn kids(items: &[crate::pptx::shapes::ParsedItem]) -> Vec<crate::source::Item> {
    match &items[0].item.kind {
        ItemKind::Group { items, .. } => items.clone(),
        k => panic!("groupe attendu, trouvé {k:?}"),
    }
}

#[test]
fn textbox_becomes_text_item_with_frame_alt_and_id() {
    let (items, _, _) = tree(TEXTBOX);
    let it = &items[0].item;
    assert_that!((it.id.as_str(), it.name.as_str(), it.alt.as_deref())).is_equal_to((
        "s1/4",
        "ZoneTexte 3",
        Some("Légende"),
    ));
    assert_that!((it.frame.x, it.frame.y, it.frame.w, it.frame.h)).is_equal_to((100.0, 50.0, 200.0, 40.0));
    let ItemKind::Text(tb) = &it.kind else { panic!("texte attendu") };
    assert_that!(tb.paragraphs[0].runs[0].text.as_str()).is_equal_to("Bonjour");
}

#[test]
fn rotation_is_normalized_and_flips_are_kept() {
    let (items, _, _) = tree(&TEXTBOX.replace("<a:xfrm>", r#"<a:xfrm rot="-5400000" flipH="1">"#));
    let f = items[0].item.frame;
    assert_that!((f.rot, f.flip_h, f.flip_v)).is_equal_to((270.0, true, false));
}

#[test]
fn styled_rectangle_takes_fill_and_line_from_style_refs() {
    let (items, _, _) = tree(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="5" name="Rectangle 4"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="127000" cy="127000"/></a:xfrm><a:prstGeom prst="rect"/></p:spPr><p:style><a:lnRef idx="2"><a:schemeClr val="accent1"><a:shade val="50000"/></a:schemeClr></a:lnRef><a:fillRef idx="1"><a:schemeClr val="accent1"/></a:fillRef><a:effectRef idx="0"><a:schemeClr val="accent1"/></a:effectRef><a:fontRef idx="minor"><a:schemeClr val="lt1"/></a:fontRef></p:style></p:sp>"#,
    );
    let ItemKind::Shape(s) = &items[0].item.kind else { panic!("forme attendue") };
    assert_that!(matches!(&s.fill, Fill::Solid { color } if color.origin == Origin::scheme("accent1"))).is_true();
    let stroke = s.stroke.clone().unwrap();
    assert_that!((stroke.width, stroke.color.value.to_hex())).is_equal_to((1.0, "#2f528f".to_owned()));
}

#[test]
fn own_fill_beats_the_style_fill() {
    let (items, _, _) = tree(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="5" name="R"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:prstGeom prst="rect"/><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></p:spPr><p:style><a:lnRef idx="0"><a:schemeClr val="accent1"/></a:lnRef><a:fillRef idx="1"><a:schemeClr val="accent1"/></a:fillRef><a:effectRef idx="0"><a:schemeClr val="accent1"/></a:effectRef><a:fontRef idx="minor"/></p:style></p:sp>"#,
    );
    let ItemKind::Shape(s) = &items[0].item.kind else { panic!("forme attendue") };
    assert_that!(matches!(&s.fill, Fill::Solid { color } if color.value.to_hex() == "#ff0000")).is_true();
}

#[test]
fn empty_non_placeholder_rect_and_blank_text_are_dropped() {
    let (items, _, _) = tree(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="6" name="Vide"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:prstGeom prst="rect"/></p:spPr><p:txBody><a:bodyPr/><a:p><a:r><a:t>   </a:t></a:r></a:p></p:txBody></p:sp>"#,
    );
    assert_that!(items).is_empty();
}

#[test]
fn empty_placeholder_is_kept_as_text() {
    let (items, _, _) = tree(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="7" name="Titre"/><p:cNvSpPr/><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr><p:spPr/></p:sp>"#,
    );
    assert_that!((matches!(items[0].item.kind, ItemKind::Text(_)), items[0].ph.as_ref().map(|d| d.key.ty.as_str())))
        .is_equal_to((true, Some("title")));
}

#[test]
fn hidden_shape_is_skipped() {
    let (items, _, _) = tree(&TEXTBOX.replace(r#"name="ZoneTexte 3""#, r#"name="ZoneTexte 3" hidden="1""#));
    assert_that!(items).is_empty();
}

#[test]
fn connector_is_a_shape_with_its_line() {
    let (items, _, _) = tree(
        r#"<p:cxnSp><p:nvCxnSpPr><p:cNvPr id="8" name="Connecteur"/><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr><p:spPr><a:prstGeom prst="line"/><a:ln w="25400"><a:solidFill><a:srgbClr val="000000"/></a:solidFill></a:ln></p:spPr></p:cxnSp>"#,
    );
    let ItemKind::Shape(s) = &items[0].item.kind else { panic!("forme attendue") };
    assert_that!(s.stroke.as_ref().map(|st| st.width)).is_equal_to(Some(2.0));
}

#[test]
fn z_follows_document_order() {
    let (items, _, _) = tree(&format!("{}{}", textbox_at(4, (0, 0), (PT, PT)), textbox_at(5, (0, 0), (PT, PT))));
    assert_that!(items.iter().map(|p| p.item.z).collect::<Vec<_>>()).is_equal_to(vec![0, 1]);
}

#[test]
fn picture_reads_asset_and_bounded_crop() {
    let (items, _, assets) = tree(
        r#"<p:pic><p:nvPicPr><p:cNvPr id="7" name="Image 6"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rId2"/><a:srcRect l="10000" r="20000" t="60000" b="60000"/><a:stretch/></p:blipFill><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="12700" cy="12700"/></a:xfrm></p:spPr></p:pic>"#,
    );
    let ItemKind::Picture(p) = &items[0].item.kind else { panic!("image attendue") };
    // l + r = 0,3 : gardés ; t + b = 1,2 : impossible, remis à 0.
    assert_that!((p.crop.l, p.crop.r, p.crop.t, p.crop.b)).is_equal_to((0.1, 0.2, 0.0, 0.0));
    assert_that!(assets.get(&p.asset).map(|a| a.mime.as_str())).is_equal_to(Some("image/png"));
}

#[test]
fn picture_prefers_its_svg_version() {
    let (items, _, assets) = tree(
        r#"<p:pic><p:nvPicPr><p:cNvPr id="7" name="Icône"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rId2"><a:extLst><a:ext uri="{96DAC541-7B7A-43D3-8B79-37D633B846F1}"><asvg:svgBlip r:embed="rId4"/></a:ext></a:extLst></a:blip></p:blipFill><p:spPr/></p:pic>"#,
    );
    let ItemKind::Picture(p) = &items[0].item.kind else { panic!("image attendue") };
    assert_that!(assets.get(&p.asset).map(|a| a.mime.as_str())).is_equal_to(Some("image/svg+xml"));
}

#[test]
fn picture_falls_back_to_its_raster_when_svg_is_rejected() {
    let bomb = br#"<!DOCTYPE svg [<!ENTITY a "a"><!ENTITY b "&a;&a;">]><svg xmlns="http://www.w3.org/2000/svg"><text>&b;</text></svg>"#;
    let t = TestPkg::with(&[
        ("ppt/slides/_rels/slide1.xml.rels", SLIDE_RELS.as_bytes()),
        ("ppt/media/image1.png", PNG),
        ("ppt/media/image1.svg", bomb),
    ]);
    let (items, w, assets) = tree_in(
        &t,
        r#"<p:pic><p:nvPicPr><p:cNvPr id="7" name="Icône"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rId2"><a:extLst><a:ext uri="{96DAC541-7B7A-43D3-8B79-37D633B846F1}"><asvg:svgBlip r:embed="rId4"/></a:ext></a:extLst></a:blip></p:blipFill><p:spPr/></p:pic>"#,
    );
    let ItemKind::Picture(p) = &items[0].item.kind else { panic!("image attendue") };
    assert_that!(assets.get(&p.asset).map(|a| a.mime.as_str())).is_equal_to(Some("image/png"));
    assert_that!(w.iter().any(|w| matches!(w.kind, WarningKind::UnsafeSvg(_)))).is_true();
}

#[test]
fn video_picture_is_opaque_media_with_its_poster() {
    let (items, _, assets) = tree(
        r#"<p:pic><p:nvPicPr><p:cNvPr id="9" name="Vidéo"/><p:cNvPicPr/><p:nvPr><a:videoFile r:link="rIdV"/></p:nvPr></p:nvPicPr><p:blipFill><a:blip r:embed="rId2"/></p:blipFill><p:spPr/></p:pic>"#,
    );
    let ItemKind::Opaque(o) = &items[0].item.kind else { panic!("opaque attendu") };
    assert_that!((o.reason.clone(), o.fallback.as_ref().is_some_and(|a| assets.contains_key(a))))
        .is_equal_to((OpaqueReason::Media, true));
}

#[test]
fn picture_with_missing_part_warns_and_is_dropped() {
    let (items, w, _) = tree(
        r#"<p:pic><p:nvPicPr><p:cNvPr id="8" name="X"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rId3"/></p:blipFill><p:spPr/></p:pic>"#,
    );
    assert_that!(items).is_empty();
    assert_that!(w.iter().any(|w| matches!(w.kind, WarningKind::MissingPart(_)))).is_true();
}

#[test]
fn group_with_zero_child_offset_scales_children() {
    // Groupe en (100, 100) pt, 200×100 pt ; espace enfant 100×50 pt depuis 0 : échelle 2.
    let child = textbox_at(4, (50 * PT, 25 * PT), (50 * PT, 25 * PT));
    let (items, _, _) =
        tree(&group(10, (100 * PT, 100 * PT), (200 * PT, 100 * PT), (0, 0), (100 * PT, 50 * PT), &child));
    let f = kids(&items)[0].frame;
    assert_that!((f.x, f.y, f.w, f.h)).is_equal_to((200.0, 150.0, 100.0, 50.0));
}

#[test]
fn group_with_offset_child_space_and_scale_maps_children() {
    // off (100,100), ext 200×100, chOff (50,50), chExt 100×50 : échelle 2 ; enfant en (60,60) → 100 + 2×(60−50) = 120.
    let child = textbox_at(4, (60 * PT, 60 * PT), (10 * PT, 10 * PT));
    let (items, _, _) =
        tree(&group(10, (100 * PT, 100 * PT), (200 * PT, 100 * PT), (50 * PT, 50 * PT), (100 * PT, 50 * PT), &child));
    let f = kids(&items)[0].frame;
    assert_that!((f.x, f.y, f.w, f.h)).is_equal_to((120.0, 120.0, 20.0, 20.0));
}

#[test]
fn nested_groups_compose_their_transforms() {
    // Extérieur : échelle 2 depuis (0,0). Intérieur, posé en (10,10) pt dans l'extérieur, chOff (5,5), chExt 20×20 pour 40×40 : échelle 2.
    let leaf = textbox_at(4, (10 * PT, 10 * PT), (10 * PT, 10 * PT));
    let inner = group(11, (10 * PT, 10 * PT), (40 * PT, 40 * PT), (5 * PT, 5 * PT), (20 * PT, 20 * PT), &leaf);
    let outer = group(10, (0, 0), (200 * PT, 200 * PT), (0, 0), (100 * PT, 100 * PT), &inner);
    let (items, _, _) = tree(&outer);
    let ItemKind::Group { items: leaves, smartart } = &kids(&items)[0].kind else { panic!("groupe attendu") };
    assert_that!(*smartart).is_false();
    // feuille : dans l'extérieur 10 + 2×(10 − 5) = 20, puis ×2 → 40 ; taille 10 × 2 × 2 = 40.
    let f = leaves[0].frame;
    assert_that!((f.x, f.y, f.w, f.h)).is_equal_to((40.0, 40.0, 40.0, 40.0));
}

#[test]
fn absurd_group_scale_stays_finite_and_bounded() {
    let child = textbox_at(4, (1, 1), (1, 1));
    let (items, _, _) = tree(&group(10, (0, 0), (9_000_000_000_000, 9_000_000_000_000), (0, 0), (1, 1), &child));
    let f = kids(&items)[0].frame;
    assert_that!(f.x.is_finite() && f.w.is_finite() && f.w <= 1e6 && f.x <= 1e6).is_true();
}

#[test]
fn deep_group_nesting_is_cut_with_a_warning() {
    // 100 groupes : au-delà de MAX_NESTING (32), en deçà de la profondeur XML admise (256).
    let open = group(100, (0, 0), (PT, PT), (0, 0), (PT, PT), "").replace("</p:grpSp>", "");
    let xml = format!("{}{}{}", open.repeat(100), textbox_at(4, (0, 0), (PT, PT)), "</p:grpSp>".repeat(100));
    let (items, w, _) = tree(&xml);
    assert_that!(items.len()).is_equal_to(1);
    assert_that!(w.iter().any(|w| matches!(w.kind, WarningKind::NestingTooDeep(_)))).is_true();
}

#[test]
fn deep_alternate_content_nesting_is_cut_with_a_warning() {
    let open = format!(r#"<mc:AlternateContent {P14}><mc:Choice Requires="p14">"#).repeat(100);
    let close = "</mc:Choice></mc:AlternateContent>".repeat(100);
    let (_, w, _) = tree(&format!("{open}{}{close}", textbox_at(4, (0, 0), (PT, PT))));
    assert_that!(w.iter().any(|w| matches!(w.kind, WarningKind::NestingTooDeep(_)))).is_true();
}

#[test]
fn group_rotation_and_flip_are_reported() {
    let g = group(10, (0, 0), (PT, PT), (0, 0), (PT, PT), &textbox_at(4, (0, 0), (PT, PT)))
        .replace("<a:xfrm>", r#"<a:xfrm rot="60000" flipV="1">"#);
    let (_, w, _) = tree(&g);
    let k: Vec<_> = w.iter().map(|w| w.kind.clone()).collect();
    assert_that!(
        k.contains(&WarningKind::EffectIgnored("group rotation".into()))
            && k.contains(&WarningKind::EffectIgnored("group flip".into()))
    )
    .is_true();
}

#[test]
fn group_fill_is_used_by_children_with_grp_fill() {
    let child = r#"<p:sp><p:nvSpPr><p:cNvPr id="4" name="R"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:prstGeom prst="ellipse"/><a:grpFill/></p:spPr></p:sp>"#;
    let g = group(10, (0, 0), (PT, PT), (0, 0), (PT, PT), child).replace(
        "</a:xfrm></p:grpSpPr>",
        r#"</a:xfrm><a:solidFill><a:srgbClr val="00FF00"/></a:solidFill></p:grpSpPr>"#,
    );
    let (items, _, _) = tree(&g);
    let ItemKind::Shape(s) = &kids(&items)[0].kind else { panic!("forme attendue") };
    assert_that!(matches!(&s.fill, Fill::Solid { color } if color.value.to_hex() == "#00ff00")).is_true();
}

#[test]
fn effects_and_approximations_are_reported() {
    let (_, w, _) = tree(&TEXTBOX.replace("<a:noFill/>", r#"<a:pattFill prst="pct5"><a:fgClr><a:srgbClr val="000000"/></a:fgClr></a:pattFill><a:effectLst><a:outerShdw blurRad="1"/></a:effectLst>"#));
    let k: Vec<_> = w.iter().map(|w| w.kind.clone()).collect();
    assert_that!(
        k.contains(&WarningKind::EffectIgnored("effectLst".into()))
            && k.contains(&WarningKind::Approximated("pattFill".into()))
    )
    .is_true();
}

#[test]
fn math_alternate_content_becomes_opaque_with_its_fallback_picture() {
    let (items, _, assets) = tree(
        r#"<mc:AlternateContent><mc:Choice Requires="a14" xmlns:a14="http://schemas.microsoft.com/office/drawing/2010/main"><p:sp><p:nvSpPr><p:cNvPr id="11" name="Eq"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:p><m:oMathPara/></a:p></p:txBody></p:sp></mc:Choice><mc:Fallback><p:sp><p:nvSpPr><p:cNvPr id="11" name="Eq"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="12700" y="25400"/><a:ext cx="12700" cy="12700"/></a:xfrm><a:blipFill><a:blip r:embed="rId2"/></a:blipFill></p:spPr></p:sp></mc:Fallback></mc:AlternateContent>"#,
    );
    let it = &items[0].item;
    let ItemKind::Opaque(o) = &it.kind else { panic!("opaque attendu") };
    assert_that!((o.reason.clone(), it.id.as_str(), it.frame.x, it.frame.y)).is_equal_to((
        OpaqueReason::Math,
        "s1/11",
        1.0,
        2.0,
    ));
    assert_that!(o.fallback.as_ref().is_some_and(|a| assets.get(a).is_some_and(|x| x.mime == "image/png"))).is_true();
}

#[test]
fn alternate_content_with_understood_choice_reads_only_the_choice() {
    let (items, _, _) = tree(&format!(
        r#"<mc:AlternateContent {P14}><mc:Choice Requires="p14">{}</mc:Choice><mc:Fallback>{}</mc:Fallback></mc:AlternateContent>"#,
        textbox_at(4, (0, 0), (PT, PT)),
        textbox_at(5, (0, 0), (PT, PT))
    ));
    assert_that!(items.iter().map(|p| p.item.id.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["s1/4"]);
}

#[test]
fn alternate_content_with_unknown_requirement_reads_only_the_fallback() {
    let choice = r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="12" name="Chartex"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="0" y="0"/><a:ext cx="12700" cy="12700"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.microsoft.com/office/drawing/2014/chartex"><cx:chart xmlns:cx="urn:cx" r:id="rId3"/></a:graphicData></a:graphic></p:graphicFrame>"#;
    let fallback = r#"<p:pic><p:nvPicPr><p:cNvPr id="12" name="Chartex"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rId2"/></p:blipFill><p:spPr/></p:pic>"#;
    let (items, w, assets) = tree(&format!(
        r#"<mc:AlternateContent xmlns:cx1="http://schemas.microsoft.com/office/drawing/2015/9/8/chartex"><mc:Choice Requires="cx1">{choice}</mc:Choice><mc:Fallback>{fallback}</mc:Fallback></mc:AlternateContent>"#
    ));
    // Le Choice n'est pas lu : l'id du repli n'a pas de suffixe, aucun avertissement ni asset du Choice.
    assert_that!((
        matches!(items[0].item.kind, ItemKind::Picture(_)),
        items[0].item.id.as_str(),
        w.len(),
        assets.len()
    ))
    .is_equal_to((true, "s1/12", 0, 1));
}

#[test]
fn unknown_graphic_frame_is_opaque_with_its_uri() {
    let (items, _, _) = tree(
        r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="12" name="Objet"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="0" y="0"/><a:ext cx="12700" cy="12700"/></p:xfrm><a:graphic><a:graphicData uri="urn:custom"/></a:graphic></p:graphicFrame>"#,
    );
    let ItemKind::Opaque(o) = &items[0].item.kind else { panic!("opaque attendu") };
    assert_that!(o.reason.clone()).is_equal_to(OpaqueReason::Unknown("urn:custom".into()));
}

#[test]
fn table_graphic_frame_becomes_table_item() {
    let (items, _, _) = tree(
        r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="13" name="Tableau"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="0" y="0"/><a:ext cx="12700" cy="12700"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblGrid><a:gridCol w="12700"/></a:tblGrid><a:tr h="12700"><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>x</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc></a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame>"#,
    );
    assert_that!(matches!(items[0].item.kind, ItemKind::Table(_))).is_true();
}

const CHART_FRAME: &str = r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="14" name="Graphique"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="0" y="0"/><a:ext cx="12700" cy="12700"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart xmlns:c="urn:c" r:id="rId5"/></a:graphicData></a:graphic></p:graphicFrame>"#;

#[test]
fn chart_graphic_frame_reads_its_part() {
    let (items, _, _) = tree(CHART_FRAME);
    let ItemKind::Chart(c) = &items[0].item.kind else { panic!("graphique attendu") };
    assert_that!(c.series.len()).is_equal_to(2);
}

#[test]
fn chart_with_unknown_relationship_warns_missing_part() {
    let (items, w, _) = tree(&CHART_FRAME.replace("rId5", "rIdNone"));
    assert_that!((
        matches!(items[0].item.kind, ItemKind::Opaque(_)),
        w.iter().any(|w| matches!(w.kind, WarningKind::MissingPart(_)))
    ))
    .is_equal_to((true, true));
}

#[test]
fn chart_with_unsupported_type_is_opaque_and_warns() {
    let bubble =
        r#"<c:chartSpace xmlns:c="urn:c"><c:chart><c:plotArea><c:bubbleChart/></c:plotArea></c:chart></c:chartSpace>"#;
    let t = crate::pptx::test_support::TestPkg::with(&[
        ("ppt/slides/_rels/slide1.xml.rels", crate::pptx::test_support::SLIDE_RELS.as_bytes()),
        ("ppt/charts/chart1.xml", bubble.as_bytes()),
    ]);
    let (items, w, _) = crate::pptx::test_support::tree_in(&t, CHART_FRAME);
    assert_that!((
        matches!(&items[0].item.kind, ItemKind::Opaque(o) if o.reason == OpaqueReason::Unknown("bubbleChart".into())),
        w.iter().any(|w| w.kind == WarningKind::UnsupportedChart("bubbleChart".into()))
    ))
    .is_equal_to((true, true));
}

#[test]
fn chart_with_unreadable_xml_warns_unreadable_item() {
    let t = crate::pptx::test_support::TestPkg::with(&[
        ("ppt/slides/_rels/slide1.xml.rels", crate::pptx::test_support::SLIDE_RELS.as_bytes()),
        ("ppt/charts/chart1.xml", b"<c:chartSpace"),
    ]);
    let (items, w, _) = crate::pptx::test_support::tree_in(&t, CHART_FRAME);
    assert_that!((
        matches!(items[0].item.kind, ItemKind::Opaque(_)),
        w.iter().any(|w| matches!(w.kind, WarningKind::UnreadableItem(_)))
    ))
    .is_equal_to((true, true));
}

#[test]
fn chart_part_over_the_node_limit_is_fatal_not_swallowed() {
    use crate::ReadLimits;
    use crate::pptx::package::Package;
    use crate::pptx::test_support::{LIMITS, run_ctx, slide_pkg};
    let t = slide_pkg(&[]);
    let limits = ReadLimits { max_xml_nodes: 30, ..LIMITS };
    let pkg = Package::open(t.bytes(), &limits).unwrap();
    let xml = format!(
        r#"<p:spTree xmlns:p="urn:p" xmlns:a="urn:a" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:c="urn:c"><p:nvGrpSpPr/><p:grpSpPr/>{CHART_FRAME}</p:spTree>"#
    );
    let doc = crate::pptx::xml::parse(&xml, "t", LIMITS.max_xml_nodes).unwrap();
    let r = run_ctx(&pkg, "ppt/slides/slide1.xml", None, |cx| {
        crate::pptx::shapes::parse_tree(doc.root_element(), cx, &crate::pptx::shapes::GroupCtx::ROOT)
    });
    assert_that!(r.err().map(|e| e.kind())).is_equal_to(Some(crate::ImportErrorKind::TooLarge));
}

#[test]
fn picture_linked_to_an_external_image_warns_and_is_dropped() {
    let (items, w, _) = tree(
        r#"<p:pic><p:nvPicPr><p:cNvPr id="8" name="Lien"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:link="rId8"/></p:blipFill><p:spPr/></p:pic>"#,
    );
    assert_that!(items).is_empty();
    assert_that!(w.iter().map(|w| (w.item.clone(), w.kind.clone())).collect::<Vec<_>>()).is_equal_to(vec![(
        Some("s1/8".to_owned()),
        WarningKind::ExternalImage("https://images.example/photo.png".into()),
    )]);
}

#[test]
fn connector_reads_its_connected_shapes() {
    let (items, _, _) = tree(
        r#"<p:cxnSp><p:nvCxnSpPr><p:cNvPr id="8" name="Connecteur"/><p:cNvCxnSpPr><a:stCxn id="4" idx="3"/><a:endCxn id="5" idx="1"/></p:cNvCxnSpPr><p:nvPr/></p:nvCxnSpPr><p:spPr><a:prstGeom prst="straightConnector1"/><a:ln><a:solidFill><a:srgbClr val="000000"/></a:solidFill><a:tailEnd type="triangle"/></a:ln></p:spPr></p:cxnSp>"#,
    );
    let ItemKind::Shape(s) = &items[0].item.kind else { panic!("forme attendue") };
    let owner = items[0].item.id.rsplit_once('/').map(|(o, _)| o.to_owned()).unwrap();
    let site = |id: u32, idx: u32| Some(Site { item: format!("{owner}/{id}"), idx });
    assert_that!(s.connection).is_equal_to(Some(Connection { start: site(4, 3), end: site(5, 1) }));
    assert_that!(s.stroke.as_ref().and_then(|st| st.tail).is_some()).is_true();
}

#[test]
fn connector_without_connected_shapes_has_no_connection() {
    let (items, _, _) = tree(
        r#"<p:cxnSp><p:nvCxnSpPr><p:cNvPr id="8" name="Connecteur"/><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr><p:spPr><a:prstGeom prst="line"/></p:spPr></p:cxnSp>"#,
    );
    let ItemKind::Shape(s) = &items[0].item.kind else { panic!("forme attendue") };
    assert_that!(s.connection).is_none();
}
