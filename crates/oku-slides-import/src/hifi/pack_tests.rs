use std::fmt::Write as _;

use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::fonts::resolve;
use crate::hifi::page::Page;
use crate::hifi::theme::{theme_colors, theme_master};
use crate::hifi::widget::DECOR_PARAM;
use crate::report::LossKind;
use crate::source::{
    Align, Anchor, Asset, Dash, Fill, Frame, Geometry, ItemKind, Layout, LevelStyle, Master, MasterTextStyles,
    Placeholder, Role, Shape, SourceSlide, Stroke, Styled, TextBox,
};
use crate::test_support::{
    app_fonts, bullet, fixture, item, layout, office_theme, para, registry, run_style, source_deck, source_slide,
};
use oku_slides_core::qualify;

/// Le pack généré pour `deck`, au thème de son master de thème.
fn generated(deck: &SourceDeck) -> GeneratedPack {
    generated_within(deck, PackLimits::default())
}

/// [`generated`] aux bornes `limits`.
fn generated_within(deck: &SourceDeck, limits: PackLimits) -> GeneratedPack {
    let parts = ctx_of(deck);
    generate_pack(deck, &parts.ctx(), limits)
}

/// Ce que [`GenCtx`] emprunte pour `deck`, au thème de son master de thème.
struct CtxParts<'a> {
    deck: &'a SourceDeck,
    page: Page,
    colors: crate::hifi::color::ThemeColors,
    fonts: crate::fonts::FontPlan,
}

impl CtxParts<'_> {
    fn ctx(&self) -> GenCtx<'_> {
        let theme_fonts = theme_master(self.deck).map(|m| &self.deck.themes[m.theme].fonts);
        GenCtx {
            page: &self.page,
            colors: &self.colors,
            fonts: &self.fonts,
            assets: &self.deck.assets,
            theme_fonts,
            lang: None,
        }
    }
}

fn ctx_of(deck: &SourceDeck) -> CtxParts<'_> {
    let Some(master) = theme_master(deck) else { panic!("aucun master") };
    let theme = &deck.themes[master.theme];
    let fonts = resolve(&deck.fonts, &[&theme.fonts.major, &theme.fonts.minor], &app_fonts());
    CtxParts { deck, page: Page::new(deck.size), colors: theme_colors(deck, master), fonts }
}

/// Le deck synthétique, plus `n` layouts `x1`… `x<n>` employés chacun par
/// une slide, sauf `x<n>` par trois.
pub(super) fn many_layouts(n: usize) -> SourceDeck {
    let mut deck = synthetic();
    for i in 1..=n {
        let id = format!("x{i}");
        deck.layouts.push(Layout { name: format!("Layout {i}"), ..layout(&id, "obj") });
        let uses = if i == n { 3 } else { 1 };
        for _ in 0..uses {
            let number = u32::try_from(deck.slides.len() + 1).unwrap();
            deck.slides.push(source_slide(number, Some(&id), Vec::new()));
        }
    }
    deck
}

/// Un décor de `n` filets.
fn rules(owner: &str, n: usize) -> Vec<crate::source::Item> {
    let geom = || Geometry::Preset { name: "rect".into(), adj: std::collections::BTreeMap::default() };
    let fill = Fill::Solid { color: Styled::scheme(office_theme().colors["accent1"], "accent1") };
    (0..n)
        .map(|i| {
            let shape = Shape { geom: geom(), fill: fill.clone(), stroke: None, text: None, connection: None };
            item(
                &format!("{owner}/{i}"),
                (0.0, oku_slides_core::num::count_f64(i), 960.0, 1.0),
                ItemKind::Shape(Box::new(shape)),
            )
        })
        .collect()
}

/// Les fichiers du pack en un seul texte, chacun après une ligne
/// `// ==== <fichier> ====` (que `oku-slides-render/tests/import_pack.rs` découpe).
fn files_text(pack: &GeneratedPack) -> String {
    let mut out = String::new();
    for (name, source) in &pack.files {
        writeln!(out, "// ==== {} ====", name.file_name()).unwrap();
        out.push_str(source);
    }
    out
}

/// Un deck synthétique : master `m1` (thème Office), layouts `l1` (« Title
/// Slide ») et `l2` (« Blank »), slides sur `l2` puis `l2`.
fn synthetic() -> SourceDeck {
    let level = LevelStyle { para: para(Vec::new()).style, run: run_style() };
    let master = Master {
        id: "m1".into(),
        name: "Office".into(),
        theme: 0,
        background: Fill::None,
        decor: Vec::new(),
        placeholders: Vec::new(),
        text_styles: MasterTextStyles { title: level.clone(), body: level.clone(), other: level },
    };
    let mut deck = source_deck(vec![source_slide(1, Some("l2"), Vec::new()), source_slide(2, Some("l2"), Vec::new())]);
    deck.themes = vec![office_theme()];
    deck.masters = vec![master];
    deck.layouts = vec![Layout { name: "Title Slide".into(), ..layout("l1", "title") }, layout("l2", "blank")];
    deck
}

/// Le deck synthétique, plus un layout `l3` utilisé : fond image, filet de
/// décor, titre, deux corps à puce, image, date, pied et numéro ; le master
/// dessine un bandeau.
fn synthetic_full() -> SourceDeck {
    let mut deck = synthetic();
    deck.assets.insert("sha256:aa".into(), Asset { mime: "image/png".into(), bytes: Vec::new() });
    let accent = Fill::Solid { color: Styled::scheme(office_theme().colors["accent1"], "accent1") };
    let shape = |id: &str, frame, name: &str| {
        let geom = Geometry::Preset { name: name.into(), adj: std::collections::BTreeMap::default() };
        let stroke = Some(Stroke {
            width: 1.0,
            color: Styled::scheme(office_theme().colors["accent2"], "accent2"),
            dash: Dash::Solid,
            head: None,
            tail: None,
        });
        item(
            id,
            frame,
            ItemKind::Shape(Box::new(Shape { geom, fill: accent.clone(), stroke, text: None, connection: None })),
        )
    };
    deck.masters[0].decor = vec![shape("m1/bandeau", (0.0, 520.0, 960.0, 20.0), "rect")];
    let level = |size: f64, align: Align| {
        let mut l = LevelStyle { para: para(Vec::new()).style, run: run_style() };
        (l.run.size, l.para.align) = (size, align);
        l
    };
    let mut title = level(40.0, Align::Center);
    title.run.bold = true;
    let mut body = level(24.0, Align::Left);
    body.para = bullet(0, Vec::new()).style;
    (body.para.margin_left, body.para.indent) = (27.0, -27.0);
    let ph = |id: &str, role: Role, idx: Option<u32>, (x, y, w, h): (f64, f64, f64, f64), l: &LevelStyle| Placeholder {
        id: id.into(),
        role,
        idx,
        frame: Frame::new(x, y, w, h),
        levels: vec![l.clone()],
        text: TextBox { anchor: if role == Role::Title { Anchor::Middle } else { Anchor::Top }, ..TextBox::default() },
    };
    let small = level(12.0, Align::Right);
    deck.layouts.push(Layout {
        name: "Titre, deux contenus et pied".into(),
        background: Some(Fill::Picture { asset: "sha256:aa".into() }),
        decor: vec![shape("l3/filet", (60.0, 110.0, 840.0, 0.0), "line")],
        placeholders: vec![
            ph("l3/t", Role::Title, None, (60.0, 30.0, 840.0, 70.0), &title),
            ph("l3/b1", Role::Body, Some(1), (60.0, 130.0, 410.0, 340.0), &body),
            ph("l3/b2", Role::Object, Some(2), (490.0, 130.0, 410.0, 200.0), &body),
            ph("l3/pic", Role::Picture, Some(3), (490.0, 340.0, 410.0, 130.0), &body),
            ph("l3/d", Role::Date, Some(10), (60.0, 490.0, 200.0, 24.0), &small),
            ph("l3/f", Role::Footer, Some(11), (280.0, 490.0, 400.0, 24.0), &small),
            ph("l3/n", Role::SlideNum, Some(12), (700.0, 490.0, 200.0, 24.0), &small),
        ],
        ..layout("l3", "twoObj")
    });
    // La slide masque le décor : le widget de `l3` le commande par `decor`.
    deck.slides.push(SourceSlide { show_layout_decor: false, ..source_slide(3, Some("l3"), Vec::new()) });
    deck
}

#[test]
fn generate_pack_of_a_full_layout() {
    let pack = generated(&synthetic_full());
    insta::assert_snapshot!("synthetic_pack", files_text(&pack));
    let schemas: Vec<&WidgetSchema> = pack.schemas().collect();
    insta::assert_json_snapshot!("synthetic_schemas", schemas);
    assert_that!(pack.assets.iter().collect::<Vec<_>>()).is_equal_to(vec![&"sha256:aa".to_owned()]);
    let params: Vec<&str> = pack.widget("l3").unwrap().params.iter().map(|p| p.param.as_str()).collect();
    assert_that!(params).is_equal_to(vec!["titre", "corps", "corps-2", "date", "pied", "numero"]);
    assert_that!(pack.widget("l3").unwrap().schema.param(DECOR_PARAM).is_some()).is_true();
    assert_that!(pack.widget("l2").unwrap().schema.param(DECOR_PARAM)).is_none();
}

#[rstest]
#[case("kitchen")]
#[case("minimal")]
fn generate_pack_of_fixture(#[case] name: &str) {
    let pack = generated(&fixture(name));
    insta::assert_snapshot!(format!("{name}_pack"), files_text(&pack));
    let schemas: Vec<&WidgetSchema> = pack.schemas().collect();
    insta::assert_json_snapshot!(format!("{name}_schemas"), schemas);
    insta::assert_debug_snapshot!(format!("{name}_drawn"), (&pack.assets, &pack.losses));
}

#[test]
fn generate_pack_makes_a_widget_per_used_layout_only() {
    let pack = generated(&synthetic());
    let names: Vec<&str> = pack.files.keys().map(WidgetFile::as_str).collect();
    assert_that!(names).is_equal_to(vec!["_commun", "l2-blank"]);
    assert_that!(pack.widget("l2").map(|w| w.name.as_str())).is_equal_to(Some("l2-blank"));
    assert_that!(pack.widget("l1")).is_none();
}

#[test]
fn generate_pack_defines_the_decor_of_called_masters_only() {
    let mut deck = synthetic();
    assert_that!(generated(&deck).files[&WidgetFile::common()]).contains("#let master-1(t, ctx) = {");
    deck.layouts[1].show_master_decor = false;
    assert_that!(generated(&deck).files[&WidgetFile::common()]).does_not_contain("master-1");
}

#[test]
fn generate_pack_of_fixture_names_are_unique_and_valid() {
    let pack = generated(&fixture("kitchen"));
    let names: BTreeSet<&str> = pack.widgets.iter().map(|w| w.name.as_str()).collect();
    assert_that!(names.len()).is_equal_to(pack.widgets.len());
    assert_that!(pack.widgets.is_empty()).is_false();
}

#[test]
fn generated_registry_adds_the_declared_widgets_to_the_builtins() {
    let pack = generated(&fixture("kitchen"));
    let base = registry();
    let r = generated_registry(&base, &pack).unwrap();
    assert_that!(r.len()).is_equal_to(base.len() + pack.widgets.len());
    for w in &pack.widgets {
        let qualified = qualify(&Namespace::deck_layouts(), w.name.as_str());
        let schema = WidgetSchema { name: qualified.clone(), ..w.schema.clone() };
        assert_that!(r.get(&qualified)).is_equal_to(Some(&schema));
    }
    assert_that!(r.get(kinds::essentiel::BLANK)).is_equal_to(base.get(kinds::essentiel::BLANK));
    assert_that!(r.get(kinds::base::TEXT)).is_equal_to(base.get(kinds::base::TEXT));
}

#[test]
fn probe_name_of_an_unqualified_registry_widget_is_a_bad_schema() {
    let w = WidgetSchema { name: "nu".into(), ..registry().get(kinds::essentiel::BLANK).unwrap().clone() };
    let e = probe_name(&w).unwrap_err();
    assert_that!((e.kind(), e.widget())).is_equal_to((
        oku_slides_core::WidgetErrorKind::BadSchema(oku_slides_core::SchemaFault::Unqualified),
        Some("nu"),
    ));
}

#[test]
fn pack_manifest_examples_are_the_texts_of_the_example_slide() {
    let pack = generated(&synthetic());
    let mut slide = Slide::new("s1", "l2-blank");
    slide.params.set("titre", ParamValue::Rich(vec![oku_slides_core::Paragraph::default()]));
    slide.params.set("pied", ParamValue::Text("Acme".into()));
    slide.params.set("numero", ParamValue::Text(String::new()));
    let id = Uuid::from_u128(7);
    let m = pack_manifest(&pack, id, "Import", "2026-10-11T08:00:00Z", &BTreeMap::from([("l2".to_owned(), slide)]));
    assert_that!((m.id, m.name.as_str(), m.parent.is_none())).is_equal_to((id, "Import", true));
    let values = &m.examples[&WidgetFile::parse("l2-blank").unwrap()];
    assert_that!(values.keys().map(String::as_str).collect::<Vec<_>>()).is_equal_to(vec!["pied", "titre"]);
}

#[test]
fn pack_manifest_without_example_has_no_values() {
    let pack = generated(&synthetic());
    let m = pack_manifest(&pack, Uuid::from_u128(1), "Import", "2026-10-11T08:00:00Z", &BTreeMap::new());
    assert_that!(m.examples.is_empty()).is_true();
}

#[test]
fn pack_manifest_has_no_templates_and_layouts_files() {
    let pack = generated(&synthetic());
    let m = pack_manifest(&pack, Uuid::from_u128(1), "Import", "2026-10-11T08:00:00Z", &BTreeMap::new());
    let files = m.to_files(PackKind::Layouts, pack.files.clone()).unwrap();
    assert_that!(files.kind()).is_equal_to(PackKind::Layouts);
    assert_that!(serde_json::to_value(&m).unwrap().get("templates")).is_none();
    assert_that!((m.namespace.is_deck_layouts(), m.scope)).is_equal_to((true, PackScope::Deck));
}

#[test]
fn generate_pack_of_more_layouts_than_the_probe_loads_keeps_the_most_used() {
    // 71 layouts employés (`l2` et `x1`..`x70`) : 63 widgets au plus avec `_commun`.
    let pack = generated(&many_layouts(70));
    assert_that!(pack.files.len()).is_equal_to(PackLimits::default().max_files);
    assert_that!(pack.widgets.len()).is_equal_to(63);
    // `x70` (trois slides) et `l2` (deux) passent d'abord, puis les premiers de la source.
    assert_that!(pack.widget("x70").is_some()).is_true();
    assert_that!(pack.widget("l2").is_some()).is_true();
    assert_that!(pack.widget("x61").is_some()).is_true();
    let dropped: Vec<&str> = pack.dropped.iter().map(String::as_str).collect();
    assert_that!(dropped).is_equal_to(vec!["x62", "x63", "x64", "x65", "x66", "x67", "x68", "x69"]);
}

#[test]
fn generate_pack_within_the_probe_limits_is_unchanged() {
    let deck = many_layouts(3);
    assert_that!(generated(&deck).dropped.is_empty()).is_true();
    let tight = generated_within(&deck, PackLimits { max_files: 3, ..PackLimits::default() });
    // `x3` (trois slides) et `l2` (deux) gardés.
    let names: Vec<&str> = tight.widgets.iter().map(|w| w.layout.as_str()).collect();
    assert_that!(names).is_equal_to(vec!["l2", "x3"]);
    assert_that!(tight.dropped.clone()).is_equal_to(vec!["x1".to_owned(), "x2".to_owned()]);
}

#[test]
fn generate_pack_cuts_the_decor_that_would_exceed_the_file_size() {
    let mut deck = synthetic();
    deck.masters[0].decor = rules("m1", 400);
    deck.layouts[1].decor = rules("l2", 400);
    let full = generated(&deck);
    assert_that!(full.losses.is_empty()).is_true();
    let max = 16 * 1024;
    assert_that!(full.files.values().all(|f| f.len() > max)).is_true();

    let pack = generated_within(&deck, PackLimits { max_file_bytes: max, ..PackLimits::default() });
    for (name, source) in &pack.files {
        assert_that!(source.len()).named(name.as_str()).is_less_than_or_equal_to(max);
    }
    // Une perte par décor coupé, sur son premier item omis, qui compte les omis.
    let cut: Vec<(&str, u32)> = pack
        .losses
        .iter()
        .filter_map(|l| match l.kind {
            LossKind::DecorTruncated { omitted } => Some((l.item.as_deref()?, omitted)),
            _ => None,
        })
        .collect();
    assert_that!(cut.len()).is_equal_to(2);
    // La coupe est au plus juste : le premier filet omis, même au niveau le plus profond (3), ne tenait plus.
    let rule =
        crate::hifi::typst::indented_len(&crate::hifi::typst::decor(&rules("x", 1), &ctx_of(&deck).ctx()).code, 3);
    for (name, source) in &pack.files {
        assert_that!(max - source.len()).named(name.as_str()).is_less_than(rule);
    }
    assert_that!(
        cut.iter()
            .all(|(item, omitted)| *omitted > 0 && *omitted < 400 && item.ends_with(&format!("/{}", 400 - omitted)))
    )
    .is_true();
}

#[test]
fn generate_pack_drops_a_widget_too_large_even_without_decor() {
    // `l3` (placeholders, fond image) dépasse `_commun` même sans son décor ; `l2` (vierge) tient.
    let deck = synthetic_full();
    let max = generated(&deck).files[&WidgetFile::common()].len();
    let pack = generated_within(&deck, PackLimits { max_file_bytes: max, ..PackLimits::default() });
    for (name, source) in &pack.files {
        assert_that!(source.len()).named(name.as_str()).is_less_than_or_equal_to(max);
    }
    let kept: Vec<&str> = pack.widgets.iter().map(|w| w.layout.as_str()).collect();
    assert_that!(kept).is_equal_to(vec!["l2"]);
    assert_that!(pack.dropped).is_equal_to(vec!["l3".to_owned()]);
}

#[test]
fn pack_manifest_requires_base() {
    let deck = synthetic();
    let m = pack_manifest(&generated(&deck), Uuid::nil(), "Import", "2026-10-18T00:00:00Z", &BTreeMap::new());
    assert_that!(m.requires).is_equal_to(vec![oku_slides_core::kinds::base::namespace()]);
}
