use speculoos::prelude::*;

use super::*;
use crate::pptx::color::ColorBase;
use crate::pptx::xml::parse;
use crate::source::{Align, Anchor, Autofit, Spacing};

fn body(xml: &str) -> RawText {
    let full = format!(
        r#"<p:txBody xmlns:p="urn:p" xmlns:a="urn:a" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">{xml}</p:txBody>"#
    );
    let d = parse(&full, "t", 100_000).unwrap();
    parse_txbody(d.root_element(), &mut |id| (id == "rId7").then(|| "https://oku.example".to_owned()))
}

fn first_run(xml: &str) -> RunProps {
    body(&format!("<a:bodyPr/><a:p><a:r>{xml}<a:t>x</a:t></a:r></a:p>")).paragraphs[0].runs[0].props.clone()
}

#[test]
fn txbody_splits_paragraphs_runs_and_levels() {
    let t = body(
        r#"<a:bodyPr/><a:p><a:r><a:t>Bon</a:t></a:r><a:r><a:rPr b="1"/><a:t>jour</a:t></a:r></a:p><a:p><a:pPr lvl="1"/><a:r><a:t>Deux</a:t></a:r></a:p>"#,
    );
    assert_that!(t.paragraphs.iter().map(|p| (p.level, p.runs.len())).collect::<Vec<_>>())
        .is_equal_to(vec![(0, 2), (1, 1)]);
    assert_that!(t.paragraphs[0].runs[1].props.bold).is_equal_to(Some(true));
}

#[test]
fn line_break_becomes_a_newline_run() {
    let t = body(r#"<a:bodyPr/><a:p><a:r><a:t>a</a:t></a:r><a:br/><a:r><a:t>b</a:t></a:r></a:p>"#);
    assert_that!(t.paragraphs[0].runs.iter().map(|r| r.text.as_str()).collect::<Vec<_>>())
        .is_equal_to(vec!["a", "\n", "b"]);
}

#[test]
fn field_keeps_its_type_and_cached_text() {
    let t = body(r#"<a:bodyPr/><a:p><a:fld id="{1}" type="slidenum"><a:t>3</a:t></a:fld></a:p>"#);
    let r = &t.paragraphs[0].runs[0];
    assert_that!((r.text.as_str(), r.field.as_deref())).is_equal_to(("3", Some(crate::source::SLIDE_NUMBER)));
}

#[test]
fn rpr_reads_size_style_and_spacing() {
    let p = first_run(r#"<a:rPr sz="2400" i="1" u="sng" baseline="30000" cap="all" spc="50"/>"#);
    assert_that!((p.size, p.italic, p.underline, p.baseline, p.caps, p.char_spacing)).is_equal_to((
        Some(24.0),
        Some(true),
        Some(true),
        Some(30),
        Some(Caps::All),
        Some(0.5),
    ));
}

#[test]
fn rpr_explicit_false_is_kept_to_override_inheritance() {
    let p = first_run(r#"<a:rPr b="0" u="none"/>"#);
    assert_that!((p.bold, p.underline)).is_equal_to((Some(false), Some(false)));
}

#[test]
fn rpr_keeps_theme_font_and_unresolved_scheme_color() {
    let p =
        first_run(r#"<a:rPr><a:solidFill><a:schemeClr val="tx1"/></a:solidFill><a:latin typeface="+mj-lt"/></a:rPr>"#);
    assert_that!((p.font.clone(), p.color.map(|c| c.base)))
        .is_equal_to((Some(FontRef::Major), Some(ColorBase::Scheme("tx1".into()))));
}

#[test]
fn rpr_clips_absurd_font_names() {
    let p = first_run(&format!(r#"<a:rPr><a:latin typeface="{}"/></a:rPr>"#, "x".repeat(10_000)));
    let Some(FontRef::Named(n)) = p.font else { panic!("police nommée attendue") };
    assert_that!(n.chars().count()).is_equal_to(MAX_FONT_NAME);
}

#[test]
fn rpr_link_goes_through_the_link_resolver() {
    assert_that!(first_run(r#"<a:rPr><a:hlinkClick r:id="rId7"/></a:rPr>"#).link.as_deref())
        .is_equal_to(Some("https://oku.example"));
}

#[test]
fn ppr_reads_alignment_indents_spacing_and_char_bullet() {
    let t = body(
        r#"<a:bodyPr/><a:p><a:pPr algn="ctr" marL="342900" indent="-342900"><a:lnSpc><a:spcPct val="90000"/></a:lnSpc><a:spcBef><a:spcPts val="1000"/></a:spcBef><a:buFont typeface="Arial"/><a:buChar char="•"/></a:pPr><a:r><a:t>x</a:t></a:r></a:p>"#,
    );
    let p = &t.paragraphs[0].props;
    assert_that!((p.align, p.margin_left, p.indent)).is_equal_to((Some(Align::Center), Some(27.0), Some(-27.0)));
    assert_that!((p.line_spacing, p.space_before))
        .is_equal_to((Some(Spacing::Percent(0.9)), Some(Spacing::Points(10.0))));
    assert_that!((p.bullet.kind.clone(), p.bullet.font.clone()))
        .is_equal_to((Some(BulletKind::Char("•".into())), Some(Some("Arial".into()))));
}

#[test]
fn ppr_clips_a_long_bullet_character() {
    let t = body(&format!(r#"<a:bodyPr/><a:p><a:pPr><a:buChar char="{}"/></a:pPr></a:p>"#, "•".repeat(1000)));
    let Some(BulletKind::Char(c)) = t.paragraphs[0].props.bullet.kind.clone() else { panic!("puce attendue") };
    assert_that!(c.chars().count()).is_equal_to(8);
}

#[test]
fn ppr_reads_auto_numbering_none_and_text_colored_bullet() {
    let t = body(
        r#"<a:bodyPr/><a:p><a:pPr><a:buClrTx/><a:buAutoNum type="alphaLcParenR" startAt="3"/></a:pPr></a:p><a:p><a:pPr><a:buNone/></a:pPr></a:p>"#,
    );
    assert_that!(t.paragraphs[0].props.bullet.kind.clone())
        .is_equal_to(Some(BulletKind::AutoNum { scheme: "alphaLcParenR".into(), start: 3 }));
    assert_that!((t.paragraphs[0].props.bullet.color.clone(), t.paragraphs[1].props.bullet.kind.clone()))
        .is_equal_to((Some(None), Some(BulletKind::None)));
}

#[test]
fn lst_style_puts_def_ppr_under_each_level() {
    let t = body(r#"<a:bodyPr/><a:lstStyle><a:defPPr algn="r"/><a:lvl2pPr marL="12700"/></a:lstStyle><a:p/>"#);
    assert_that!((t.lst.len(), t.lst[1].align, t.lst[1].margin_left, t.lst[0].margin_left)).is_equal_to((
        LEVELS,
        Some(Align::Right),
        Some(1.0),
        None,
    ));
}

#[test]
fn body_pr_reads_insets_anchor_wrap_columns_and_autofit() {
    let t = body(
        r#"<a:bodyPr lIns="0" anchor="ctr" wrap="none" numCol="2"><a:normAutofit fontScale="62500" lnSpcReduction="20000"/></a:bodyPr><a:p/>"#,
    );
    assert_that!((t.body.insets[0], t.body.insets[1], t.body.anchor, t.body.wrap, t.body.columns)).is_equal_to((
        Some(0.0),
        None,
        Some(Anchor::Middle),
        Some(false),
        Some(2),
    ));
    assert_that!(t.body.autofit).is_equal_to(Some(Autofit::Shrink { font_scale: 0.625, line_reduction: 0.2 }));
}

#[test]
fn body_pr_bounds_column_counts() {
    assert_that!(body(r#"<a:bodyPr numCol="4294967295"/><a:p/>"#).body.columns).is_equal_to(Some(16));
    assert_that!(body(r#"<a:bodyPr numCol="-1"/><a:p/>"#).body.columns).is_equal_to(Some(1));
}

#[test]
fn norm_autofit_without_scale_is_a_neutral_shrink() {
    assert_that!(body(r#"<a:bodyPr><a:normAutofit/></a:bodyPr><a:p/>"#).body.autofit)
        .is_equal_to(Some(Autofit::Shrink { font_scale: 1.0, line_reduction: 0.0 }));
}

#[test]
fn para_overlay_keeps_lower_values_not_overridden() {
    let mut low = ParaProps { align: Some(Align::Left), ..ParaProps::default() };
    low.run.size = Some(18.0);
    let mut top = ParaProps { align: Some(Align::Right), ..ParaProps::default() };
    top.run.bold = Some(true);
    low.overlay(&top);
    assert_that!((low.align, low.run.size, low.run.bold)).is_equal_to((Some(Align::Right), Some(18.0), Some(true)));
}
