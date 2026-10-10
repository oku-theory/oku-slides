use std::collections::BTreeMap;
use std::rc::Rc;

use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::pptx::color::{ColorBase, ColorCtx, ColorSpec, default_map};
use crate::pptx::text::{
    BodyProps, BulletKind, FontRef, ParaProps, RawParagraph, RawRun, RawText, RunProps, empty_levels,
};
use crate::source::{Align, Anchor, Bullet, Rgba, Styled};

fn colors() -> ColorCtx {
    ColorCtx::new(
        BTreeMap::from([("dk1".to_owned(), Rgba::new(0, 0, 0)), ("lt1".to_owned(), Rgba::new(255, 255, 255))]),
        default_map(),
    )
}

fn env(c: &ColorCtx) -> TextEnv<'_> {
    TextEnv { major: "Calibri Light", minor: "Calibri", colors: c, font_ref: RunProps::default() }
}

fn levels_with(level: usize, p: ParaProps) -> LevelProps {
    let mut l = empty_levels();
    l[level] = p;
    l
}

fn one_run(level: u8, para: ParaProps, run: RunProps) -> RawText {
    RawText {
        body: BodyProps::default(),
        lst: empty_levels(),
        paragraphs: vec![RawParagraph {
            level,
            props: para,
            runs: vec![RawRun { text: "x".into(), field: None, props: run }],
        }],
    }
}

fn size(s: f64) -> ParaProps {
    ParaProps { run: RunProps { size: Some(s), ..RunProps::default() }, ..ParaProps::default() }
}

fn no_chain() -> TextChain<'static> {
    TextChain { levels: vec![], bodies: vec![] }
}

#[test]
fn empty_chain_gives_reader_defaults() {
    let c = colors();
    let tb = resolve_textbox(&one_run(0, ParaProps::default(), RunProps::default()), &no_chain(), &env(&c));
    let r = &tb.paragraphs[0].runs[0].style;
    assert_that!((r.size, r.char_spacing, r.bold)).is_equal_to((18.0, 0.0, false));
    assert_that!(r.font.clone()).is_equal_to(Styled::scheme("Calibri".to_owned(), "minor"));
    assert_that!(r.color.clone()).is_equal_to(Styled::mapped(Rgba::new(0, 0, 0), "tx1", "dk1", vec![]));
    assert_that!((tb.wrap, tb.columns)).is_equal_to((true, 1));
}

#[rstest]
// Une valeur distincte par couche : on vérifie laquelle gagne.
#[case(Some(10.0), Some(20.0), None, None, None, 20.0)]
#[case(Some(10.0), Some(20.0), Some(30.0), None, None, 30.0)]
#[case(Some(10.0), Some(20.0), Some(30.0), Some(40.0), None, 40.0)]
#[case(Some(10.0), Some(20.0), Some(30.0), Some(40.0), Some(50.0), 50.0)]
#[case(Some(10.0), None, None, None, None, 10.0)]
fn size_follows_the_stronger_layer(
    #[case] first: Option<f64>,
    #[case] second: Option<f64>,
    #[case] lst: Option<f64>,
    #[case] ppr: Option<f64>,
    #[case] rpr: Option<f64>,
    #[case] expected: f64,
) {
    let c = colors();
    let a = levels_with(0, first.map(size).unwrap_or_default());
    let b = levels_with(0, second.map(size).unwrap_or_default());
    let mut raw = one_run(0, ppr.map(size).unwrap_or_default(), RunProps { size: rpr, ..RunProps::default() });
    raw.lst = levels_with(0, lst.map(size).unwrap_or_default());
    let chain = TextChain { levels: vec![&a, &b], bodies: vec![] };
    assert_that!(resolve_textbox(&raw, &chain, &env(&c)).paragraphs[0].runs[0].style.size).is_equal_to(expected);
}

#[test]
fn level_picks_its_own_layer_entry() {
    let c = colors();
    let mut m = empty_levels();
    m[0] = size(32.0);
    m[1] = size(28.0);
    let tb = resolve_textbox(
        &one_run(1, ParaProps::default(), RunProps::default()),
        &TextChain { levels: vec![&m], bodies: vec![] },
        &env(&c),
    );
    assert_that!((tb.paragraphs[0].level, tb.paragraphs[0].runs[0].style.size)).is_equal_to((1, 28.0));
}

#[test]
fn font_ref_beats_the_chain() {
    let c = colors();
    let m = levels_with(
        0,
        ParaProps {
            run: RunProps { font: Some(FontRef::Named("Arial".into())), ..RunProps::default() },
            ..ParaProps::default()
        },
    );
    let mut e = env(&c);
    e.font_ref.font = Some(FontRef::Major);
    let tb = resolve_textbox(
        &one_run(0, ParaProps::default(), RunProps::default()),
        &TextChain { levels: vec![&m], bodies: vec![] },
        &e,
    );
    assert_that!(tb.paragraphs[0].runs[0].style.font.clone())
        .is_equal_to(Styled::scheme("Calibri Light".to_owned(), "major"));
}

#[test]
fn shape_lst_style_beats_the_font_ref() {
    let c = colors();
    let mut e = env(&c);
    e.font_ref.font = Some(FontRef::Major);
    let mut raw = one_run(0, ParaProps::default(), RunProps::default());
    raw.lst[0].run.font = Some(FontRef::Named("Georgia".into()));
    assert_that!(resolve_textbox(&raw, &no_chain(), &e).paragraphs[0].runs[0].style.font.clone())
        .is_equal_to(Styled::explicit("Georgia".to_owned()));
}

#[test]
fn explicit_false_overrides_inherited_bold() {
    let c = colors();
    let m =
        levels_with(0, ParaProps { run: RunProps { bold: Some(true), ..RunProps::default() }, ..ParaProps::default() });
    let raw = one_run(0, ParaProps::default(), RunProps { bold: Some(false), ..RunProps::default() });
    assert_that!(
        resolve_textbox(&raw, &TextChain { levels: vec![&m], bodies: vec![] }, &env(&c)).paragraphs[0].runs[0]
            .style
            .bold
    )
    .is_false();
}

#[test]
fn inherited_scheme_color_resolves_with_the_consumer_color_map() {
    let mut map = default_map();
    map.insert("tx1".into(), "lt1".into());
    let dark = ColorCtx { map: Rc::new(map), ..colors() };
    let tx1 = ColorSpec { base: ColorBase::Scheme("tx1".into()), mods: vec![] };
    let m =
        levels_with(0, ParaProps { run: RunProps { color: Some(tx1), ..RunProps::default() }, ..ParaProps::default() });
    let tb = resolve_textbox(
        &one_run(0, ParaProps::default(), RunProps::default()),
        &TextChain { levels: vec![&m], bodies: vec![] },
        &env(&dark),
    );
    assert_that!(tb.paragraphs[0].runs[0].style.color.clone()).is_equal_to(Styled::mapped(
        Rgba::new(255, 255, 255),
        "tx1",
        "lt1",
        vec![],
    ));
}

#[test]
fn bullet_parts_inherit_separately() {
    let c = colors();
    let mut m = ParaProps::default();
    m.bullet.kind = Some(BulletKind::Char("•".into()));
    m.bullet.size_pct = Some(0.8);
    let master = levels_with(0, m);
    let mut p = ParaProps::default();
    p.bullet.color = Some(Some(ColorSpec { base: ColorBase::Fixed(Rgba::new(255, 0, 0)), mods: vec![] }));
    let tb = resolve_textbox(
        &one_run(0, p, RunProps::default()),
        &TextChain { levels: vec![&master], bodies: vec![] },
        &env(&c),
    );
    let Bullet::Char { char, color, size_pct, font } = &tb.paragraphs[0].style.bullet else { panic!("puce attendue") };
    assert_that!((char.as_str(), *size_pct, font.clone(), color.as_ref().map(|c| c.value))).is_equal_to((
        "•",
        0.8,
        None,
        Some(Rgba::new(255, 0, 0)),
    ));
}

#[test]
fn body_props_cascade_through_bodies_then_own() {
    let c = colors();
    let layout_body =
        BodyProps { anchor: Some(Anchor::Bottom), insets: [Some(0.0), None, None, None], ..BodyProps::default() };
    let mut raw = one_run(0, ParaProps::default(), RunProps::default());
    raw.body.anchor = Some(Anchor::Middle);
    let tb = resolve_textbox(&raw, &TextChain { levels: vec![], bodies: vec![&layout_body] }, &env(&c));
    assert_that!((tb.anchor, tb.insets.l, tb.insets.t)).is_equal_to((Anchor::Middle, 0.0, 3.6));
}

#[test]
fn resolve_levels_gives_nine_level_styles() {
    let c = colors();
    let m = levels_with(2, ParaProps { align: Some(Align::Center), ..size(20.0) });
    let lv = resolve_levels(&TextChain { levels: vec![&m], bodies: vec![] }, &empty_levels(), &env(&c));
    assert_that!((lv.len(), lv[2].para.align, lv[2].run.size, lv[0].run.size)).is_equal_to((
        9,
        Align::Center,
        20.0,
        18.0,
    ));
}
