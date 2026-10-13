use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::hifi::typst::mod_tests::Fixture;
use crate::source::{Fill, Frame, LevelStyle, Placeholder, TextBox};
use crate::test_support::{layout, para, run_style};

/// Un style de niveau neutre.
fn level() -> LevelStyle {
    LevelStyle { para: para(Vec::new()).style, run: run_style() }
}

/// Un placeholder `id` de rôle `role`, sans style.
fn ph(id: &str, role: Role, idx: Option<u32>) -> Placeholder {
    Placeholder {
        id: id.into(),
        role,
        idx,
        frame: Frame::new(40.0, 40.0, 400.0, 100.0),
        levels: Vec::new(),
        text: TextBox::default(),
    }
}

/// Un layout `l1` de master `m1` aux placeholders `phs`.
fn layout_with(phs: Vec<Placeholder>) -> Layout {
    Layout { name: "Titre et contenu".into(), placeholders: phs, ..layout("l1", "obj") }
}

#[rstest]
#[case(1, "Title Slide", "l1-title-slide")]
#[case(2, "Titre et contenu", "l2-titre-et-contenu")]
#[case(3, "Diapositive de titre — Été", "l3-diapositive-de-titre-ete")]
#[case(4, "  --Ünïcode__ Œuvre 2 ", "l4-unicode-oeuvre-2")]
#[case(5, "", "l5")]
#[case(6, "标题幻灯片", "l6")]
#[case(12, "Заголовок 1", "l12-1")]
fn widget_name_is_the_rank_and_an_ascii_slug(#[case] rank: usize, #[case] name: &str, #[case] expected: &str) {
    assert_that!(widget_name(rank, name).map(|w| w.as_str().to_owned())).is_equal_to(Some(expected.to_owned()));
}

#[test]
fn widget_name_is_cut_to_64_bytes_without_a_trailing_dash() {
    let name = format!("{} b", "a".repeat(60));
    let w = widget_name(7, &name).unwrap();
    // `l7-` + 60 `a` = 63 octets ; le tiret suivant tombe à la coupe.
    assert_that!(w.as_str().len()).is_equal_to(63);
    assert_that!(widget_name(7, &"z".repeat(200)).unwrap().as_str().len()).is_equal_to(64);
}

#[test]
fn placeholder_params_name_text_placeholders_and_number_repeats() {
    let l = layout_with(vec![
        ph("t", Role::Title, None),
        ph("b1", Role::Body, Some(1)),
        ph("pic", Role::Picture, Some(2)),
        ph("b2", Role::Object, Some(3)),
        ph("st", Role::Subtitle, Some(4)),
        ph("d", Role::Date, Some(10)),
        ph("f", Role::Footer, Some(11)),
        ph("n", Role::SlideNum, Some(12)),
        ph("b3", Role::Body, Some(5)),
    ]);
    let params = placeholder_params(&l);
    let names: Vec<(&str, &str)> = params.iter().map(|p| (p.placeholder.as_str(), p.param.as_str())).collect();
    assert_that!(names).is_equal_to(vec![
        ("t", "titre"),
        ("b1", "corps"),
        ("b2", "corps-2"),
        ("st", "sous-titre"),
        ("d", "date"),
        ("f", "pied"),
        ("n", "numero"),
        ("b3", "corps-3"),
    ]);
}

#[test]
fn layout_widget_declares_rich_plain_grid_and_extras() {
    let l = layout_with(vec![
        ph("t", Role::Title, None),
        ph("f", Role::Footer, Some(11)),
        ph("n", Role::SlideNum, Some(12)),
    ]);
    let (w, _) = layout_widget(1, &l, None, false, &Fixture::new().ctx(), usize::MAX).unwrap();
    let s = &w.schema;
    assert_that!((s.name.as_str(), s.label.as_deref(), s.category)).is_equal_to((
        "l1-titre-et-contenu",
        Some("Titre et contenu"),
        Category::Slide,
    ));
    let types: Vec<(&str, &ParamType, Option<&serde_json::Value>)> =
        s.params.iter().map(|p| (p.name.as_str(), &p.ty, p.default.as_ref())).collect();
    let grid = json!({ "columns": 12, "rows": 6 });
    let free = ParamType::Slot { mode: SlotMode::Free, accepts: None };
    let n = json!("{n}");
    assert_that!(types).is_equal_to(vec![
        ("titre", &ParamType::Rich, None),
        ("pied", &ParamType::Plain, None),
        ("numero", &ParamType::Plain, Some(&n)),
        ("grid", &ParamType::Grid, Some(&grid)),
        ("extras", &free, None),
    ]);
}

#[test]
fn layout_widget_calls_the_master_decor_only_when_the_layout_shows_it() {
    let f = Fixture::new();
    let master = Master {
        id: "m1".into(),
        name: "Office".into(),
        theme: 0,
        background: Fill::None,
        decor: Vec::new(),
        placeholders: Vec::new(),
        text_styles: crate::source::MasterTextStyles { title: level(), body: level(), other: level() },
    };
    let mut l = layout_with(Vec::new());
    let (shown, _) = layout_widget(1, &l, Some((2, &master)), false, &f.ctx(), usize::MAX).unwrap();
    assert_that!(shown.source).contains("#import \"_commun.typ\": champ, master-2\n");
    assert_that!(shown.source).contains("master-2(t, ctx)\n");
    l.show_master_decor = false;
    let (hidden, _) = layout_widget(1, &l, Some((2, &master)), false, &f.ctx(), usize::MAX).unwrap();
    assert_that!(hidden.source).does_not_contain("master-");
}

#[test]
fn layout_widget_hideable_puts_its_decor_behind_the_decor_flag() {
    let f = Fixture::new();
    let master = Master {
        id: "m1".into(),
        name: "Office".into(),
        theme: 0,
        background: Fill::None,
        decor: Vec::new(),
        placeholders: Vec::new(),
        text_styles: crate::source::MasterTextStyles { title: level(), body: level(), other: level() },
    };
    let l = layout_with(Vec::new());
    let (w, _) = layout_widget(1, &l, Some((2, &master)), true, &f.ctx(), usize::MAX).unwrap();
    let decor = w.schema.param(DECOR_PARAM).unwrap();
    assert_that!((&decor.ty, &decor.default)).is_equal_to((&ParamType::Flag, &Some(json!(true))));
    assert_that!(w.source).contains("\"decor\": p.flag(default: true),");
    assert_that!(w.source).contains("if args.at(\"decor\") {\n      master-2(t, ctx)\n    }");
    // Sans décor à masquer, pas de paramètre.
    let (bare, _) = layout_widget(1, &l, None, true, &f.ctx(), usize::MAX).unwrap();
    assert_that!(bare.schema.param(DECOR_PARAM)).is_none();
}

#[test]
fn layout_widget_draws_its_own_background_before_the_master_one() {
    let f = Fixture::new();
    let red = Fill::Solid { color: crate::source::Styled::explicit(crate::source::Rgba::new(255, 0, 0)) };
    let l = Layout { background: Some(red), ..layout_with(Vec::new()) };
    let (w, drawn) = layout_widget(1, &l, None, false, &f.ctx(), usize::MAX).unwrap();
    assert_that!(w.source).contains("fill: color-of(t, \"#ff0000\")");
    assert_that!(drawn).is_equal_to(Drawn::default());
}

#[test]
fn layout_widget_name_cannot_inject_typst() {
    let l = Layout { name: "x\") #panic(\"".into(), ..layout_with(Vec::new()) };
    let (w, _) = layout_widget(1, &l, None, false, &Fixture::new().ctx(), usize::MAX).unwrap();
    assert_that!(w.source).contains(r#"label: "x\") #panic(\"""#);
    assert_that!(w.name.as_str()).is_equal_to("l1-x-panic");
}
