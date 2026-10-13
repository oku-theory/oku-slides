use speculoos::prelude::*;

use super::*;
use crate::hifi::typst::mod_tests::Fixture;
use crate::source::{Anchor, Frame, Role, Spacing, Styled, TextBox};
use crate::test_support::{bullet, para, run, run_style};

/// Un placeholder `role` à la boîte (60, 30, 840, 120) pt source, au style de premier niveau `level`.
fn ph(role: Role, level: LevelStyle) -> Placeholder {
    Placeholder {
        id: "ph".into(),
        role,
        idx: None,
        frame: Frame::new(60.0, 30.0, 840.0, 120.0),
        levels: vec![level],
        text: TextBox { anchor: Anchor::Bottom, ..TextBox::default() },
    }
}

#[test]
fn placeholder_title_sets_its_first_level_style_and_places_its_field() {
    let mut level = LevelStyle { para: para(Vec::new()).style, run: run_style() };
    (level.run.size, level.run.bold, level.run.caps) = (44.0, true, Caps::All);
    level.para.align = crate::source::Align::Center;
    let code = placeholder(&ph(Role::Title, level), "titre", &Fixture::new().ctx());
    // Les instantanés de ce dossier se compilent seuls (`import_decor.rs`) : pas d'instantané
    // ici, le code suppose `champ` et `args` ; `import_pack.rs` le compile dans son widget.
    let expected = [
        "{",
        r#"  set text(font: ("Carlito",), size: 88pt, fill: color-of(t, "ink"), weight: "bold", top-edge: 0.942em, bottom-edge: -0.258em)"#,
        "  set par(leading: 0pt, spacing: 0pt, justify: false)",
        "  set align(center)",
        "  show: upper",
        "  set list(indent: 0pt, spacing: 0pt)",
        "  set enum(indent: 0pt, body-indent: 0.5em, spacing: 0pt)",
        r#"  champ(ctx, "titre", args.at("titre"), 120pt, 60pt, 1680pt, 240pt, (top: 7.2pt, right: 14.4pt, bottom: 7.2pt, left: 14.4pt), bottom)"#,
        "}",
    ];
    assert_that!(code.lines().collect::<Vec<_>>()).is_equal_to(expected.to_vec());
}

#[test]
fn placeholder_body_hangs_its_bullet_and_spaces_its_paragraphs() {
    let mut style = bullet(0, vec![run("x")]).style;
    (style.margin_left, style.indent, style.space_before) = (27.0, -27.0, Spacing::Points(10.0));
    let level = LevelStyle { para: style, run: run_style() };
    let code = placeholder(&ph(Role::Body, level), "corps", &Fixture::new().ctx());
    // Puce dans une boîte de 54 pt page, texte à la marge ; entre deux paragraphes,
    // l'espace avant (20 pt page) : les bords de ligne font déjà le pas.
    assert_that!(code).contains("set list(indent: 0pt, spacing: 20pt, body-indent: 0pt, marker: box(width: 54pt,");
    assert_that!(code).contains("set par(leading: 0pt, spacing: 20pt, justify: false)");
    assert_that!(code).contains(r#"champ(ctx, "corps", args.at("corps"), 120pt, 60pt, 1680pt, 240pt,"#);
}

#[test]
fn placeholder_without_level_style_only_places_its_field() {
    let mut p = ph(Role::Footer, LevelStyle { para: para(Vec::new()).style, run: run_style() });
    p.levels.clear();
    let code = placeholder(&p, "pied", &Fixture::new().ctx());
    assert_that!(code).does_not_contain("set ");
    assert_that!(code).contains("bottom)");
}

#[test]
fn placeholder_param_name_is_a_string_literal() {
    let level = LevelStyle { para: para(Vec::new()).style, run: run_style() };
    let code = placeholder(&ph(Role::Body, level), "corps-2", &Fixture::new().ctx());
    assert_that!(code).contains(r#"champ(ctx, "corps-2", args.at("corps-2"),"#);
}

#[test]
fn background_covers_the_page() {
    let fill = Fill::Solid { color: Styled::explicit(crate::source::Rgba::new(0x10, 0x20, 0x30)) };
    let d = background("m1", &fill, &Fixture::new().ctx());
    assert_that!(d.code.as_str()).is_equal_to(
        "place(top + left, rect(width: 1920pt, height: 1080pt, fill: color-of(t, \"#102030\"), stroke: none))\n",
    );
}

#[test]
fn background_of_nothing_draws_nothing() {
    assert_that!(background("m1", &Fill::None, &Fixture::new().ctx())).is_equal_to(Decor::default());
}

#[test]
fn background_picture_records_its_asset() {
    let d = background("l1", &Fill::Picture { asset: "sha256:aa".into() }, &Fixture::new().ctx());
    assert_that!(d.assets.contains("sha256:aa")).is_true();
    assert_that!(d.code).contains("tiling(size: (1920pt, 1080pt)");
}
