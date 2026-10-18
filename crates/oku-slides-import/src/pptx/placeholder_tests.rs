use speculoos::prelude::*;

use super::*;
use crate::pptx::color::{ColorBase, ColorSpec};
use crate::pptx::text::{ParaProps, RunProps, empty_levels};
use crate::source::{Frame, Rgba};

fn sized(s: f64) -> LevelProps {
    let mut l = empty_levels();
    l[0] = ParaProps { run: RunProps { size: Some(s), ..RunProps::default() }, ..ParaProps::default() };
    l
}

fn solid(r: u8) -> Option<FillSpec> {
    Some(FillSpec::Solid(ColorSpec { base: ColorBase::Fixed(Rgba::new(r, 0, 0)), mods: vec![] }))
}

fn rect(name: &str) -> Option<Geometry> {
    Some(Geometry::Preset { name: name.into(), adj: Default::default() })
}

fn info(id: &str, ty: &str, idx: Option<u32>, x: f64, lst_size: f64) -> PhInfo {
    PhInfo {
        id: id.into(),
        key: PhKey { ty: ty.into(), idx },
        frame: Frame::new(x, 0.0, 1.0, 1.0),
        lst: sized(lst_size),
        body: Default::default(),
        fill: None,
        line: Default::default(),
        geom: None,
    }
}

fn master() -> PhTable {
    PhTable {
        entries: vec![
            info("m1/2", "title", None, 1.0, 30.0),
            info("m1/3", "body", Some(1), 2.0, 31.0),
            info("m1/4", "dt", Some(10), 4.0, 32.0),
        ],
    }
}

fn styles() -> TextStyles {
    TextStyles { pres: sized(10.0), title: sized(20.0), body: sized(21.0), other: sized(22.0) }
}

/// La taille du niveau 0 de chaque couche, les défauts en tête.
fn top_size(got: &Inherited) -> Vec<Option<f64>> {
    std::iter::once(&got.defaults).chain(&got.levels).map(|l| l[0].run.size).collect()
}

#[test]
fn find_prefers_index_then_exact_type_then_master_type() {
    let t =
        PhTable { entries: vec![info("l1/2", "ctrTitle", None, 9.0, 0.0), info("l1/3", "body", Some(2), 3.0, 0.0)] };
    assert_that!(t.find(&PhKey { ty: "obj".into(), idx: Some(2) }).map(|i| i.frame.x)).is_equal_to(Some(3.0));
    assert_that!(t.find(&PhKey { ty: "ctrTitle".into(), idx: None }).map(|i| i.frame.x)).is_equal_to(Some(9.0));
    assert_that!(t.find(&PhKey { ty: "title".into(), idx: None }).map(|i| i.frame.x)).is_equal_to(Some(9.0));
}

#[test]
fn find_master_uses_normalized_type_only() {
    let t = master();
    assert_that!(t.find_master(&PhKey { ty: "subTitle".into(), idx: Some(7) }).map(|i| i.frame.x))
        .is_equal_to(Some(2.0));
    assert_that!(t.find_master(&PhKey { ty: "ftr".into(), idx: Some(11) })).is_none();
}

#[test]
fn lookup_stacks_layers_in_order_and_links_both_parents() {
    let s = styles();
    let m = master();
    let layout = PhTable { entries: vec![info("l1/2", "ctrTitle", None, 9.0, 40.0)] };
    let got = PhInheritance { styles: &s, master: Some(&m), layout: Some(&layout) }
        .lookup(Some(&PhKey { ty: "ctrTitle".into(), idx: None }));
    assert_that!(top_size(&got)).is_equal_to(vec![Some(10.0), Some(20.0), Some(30.0), Some(40.0)]);
    assert_that!((got.bodies.len(), got.frame.map(|f| f.x))).is_equal_to((2, Some(9.0)));
    assert_that!(got.link).is_equal_to(Some(PlaceholderLink {
        idx: None,
        layout: Some("l1/2".into()),
        master: Some("m1/2".into()),
    }));
}

#[test]
fn lookup_for_plain_shape_is_presentation_then_other() {
    let s = styles();
    let got = PhInheritance { styles: &s, master: None, layout: None }.lookup(None);
    assert_that!((top_size(&got), got.link)).is_equal_to((vec![Some(10.0), Some(22.0)], None));
}

#[test]
fn lookup_for_body_uses_body_style_not_other() {
    let s = styles();
    let got = PhInheritance { styles: &s, master: None, layout: None }
        .lookup(Some(&PhKey { ty: "obj".into(), idx: Some(1) }));
    assert_that!(top_size(&got)).is_equal_to(vec![Some(10.0), Some(21.0)]);
}

#[test]
fn lookup_layout_fill_and_geometry_beat_the_master() {
    let s = styles();
    let mut m = master();
    m.entries[0].fill = solid(1);
    m.entries[0].geom = rect("rect");
    let mut layout = PhTable { entries: vec![info("l1/2", "title", None, 0.0, 0.0)] };
    layout.entries[0].fill = solid(2);
    layout.entries[0].geom = rect("roundRect");
    let got = PhInheritance { styles: &s, master: Some(&m), layout: Some(&layout) }
        .lookup(Some(&PhKey { ty: "title".into(), idx: None }));
    assert_that!((got.fill, got.geom)).is_equal_to((solid(2), rect("roundRect")));
}

#[test]
fn lookup_falls_back_to_master_fill_when_layout_has_none() {
    let s = styles();
    let mut m = master();
    m.entries[0].fill = solid(1);
    let layout = PhTable { entries: vec![info("l1/2", "title", None, 0.0, 0.0)] };
    let got = PhInheritance { styles: &s, master: Some(&m), layout: Some(&layout) }
        .lookup(Some(&PhKey { ty: "title".into(), idx: None }));
    assert_that!(got.fill).is_equal_to(solid(1));
}

#[test]
fn ph_key_defaults_to_obj_and_maps_roles() {
    let d = crate::pptx::xml::parse(r#"<p:ph xmlns:p="urn:p" idx="1"/>"#, "t", 10).unwrap();
    let k = PhKey::from_ph(d.root_element());
    assert_that!((k.ty.as_str(), k.idx, k.role(), k.master_type())).is_equal_to(("obj", Some(1), Role::Object, "body"));
}
