use speculoos::prelude::*;

use crate::pptx::test_support::{LAYOUT, master_and_layout};
use crate::source::{Align, Anchor, Bullet, Fill, ItemKind, Origin, Role};

#[test]
fn master_splits_placeholders_from_decor() {
    let (m, _) = master_and_layout(LAYOUT);
    assert_that!((m.master.id.as_str(), m.master.theme)).is_equal_to(("m1", 0));
    assert_that!(m.master.placeholders.iter().map(|p| p.role).collect::<Vec<_>>())
        .is_equal_to(vec![Role::Title, Role::Body]);
    assert_that!((m.master.decor.len(), matches!(m.master.decor[0].kind, ItemKind::Shape(_)))).is_equal_to((1, true));
}

#[test]
fn master_background_comes_from_bg_ref_written_bg1() {
    let (m, _) = master_and_layout(LAYOUT);
    let Fill::Solid { color } = &m.master.background else { panic!("fond plein attendu") };
    assert_that!(color.origin.clone()).is_equal_to(Origin::Scheme {
        name: "bg1".into(),
        slot: "lt1".into(),
        mods: vec![],
    });
    assert_that!(m.bg.is_some()).is_true();
}

#[test]
fn master_title_level_has_title_style() {
    let (m, _) = master_and_layout(LAYOUT);
    let st = &m.master.placeholders[0].levels[0].run;
    assert_that!((st.size, st.font.origin.clone())).is_equal_to((44.0, Origin::scheme("major")));
}

#[test]
fn master_body_level_two_has_its_bullet() {
    let (m, _) = master_and_layout(LAYOUT);
    let body = &m.master.placeholders[1];
    assert_that!(body.levels[1].run.size).is_equal_to(24.0);
    assert_that!(matches!(&body.levels[1].para.bullet, Bullet::Char { char, .. } if char == "–")).is_true();
}

#[test]
fn layout_reads_kind_name_master_and_inherited_title_style() {
    let (_, l) = master_and_layout(LAYOUT);
    assert_that!((l.layout.id.as_str(), l.layout.kind.as_str(), l.layout.name.as_str(), l.layout.master.as_str()))
        .is_equal_to(("l1", "title", "Diapositive de titre", "m1"));
    let title = &l.layout.placeholders[0];
    assert_that!((title.id.as_str(), title.role, title.levels[0].run.size, title.levels[0].para.align)).is_equal_to((
        "l1/2",
        Role::Title,
        60.0,
        Align::Center,
    ));
    assert_that!((title.text.anchor, l.layout.show_master_decor)).is_equal_to((Anchor::Bottom, true));
}

#[test]
fn layout_subtitle_drops_the_master_bullet() {
    let (_, l) = master_and_layout(LAYOUT);
    let sub = &l.layout.placeholders[1];
    assert_that!((sub.levels[0].para.bullet.clone(), sub.levels[0].run.size)).is_equal_to((Bullet::None, 24.0));
}

#[test]
fn layout_color_map_override_reaches_inherited_text() {
    let dark = LAYOUT.replace(
        "<a:masterClrMapping/>",
        r#"<a:overrideClrMapping bg1="dk1" tx1="lt1" bg2="dk2" tx2="lt2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>"#,
    );
    let (_, l) = master_and_layout(&dark);
    assert_that!(l.layout.placeholders[0].levels[0].run.color.origin.clone()).is_equal_to(Origin::Scheme {
        name: "tx1".into(),
        slot: "lt1".into(),
        mods: vec![],
    });
}
