use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::retrofit::cluster::units;
use crate::retrofit::heading::headings;
use crate::source::{Fill, Geometry, Opaque, OpaqueReason, Picture, Rgba, Size, Styled};
use crate::test_support::{essentiel, grilles, item, layout, registry, source_slide, text_item, with_role};

/// La forme de `slide` comme la calcule la projection : titres, items de
/// contenu, unités (cellules d'un pt : pas de fusion de textes éloignés).
fn shape_of(slide: &SourceSlide, layouts: &[Layout]) -> Shape {
    let items: Vec<&Item> = slide.items.iter().collect();
    let heads = headings(&items, Size { w: 960.0, h: 540.0 });
    let content = content_items(&items, &heads);
    let cell = |f: &Frame| oku_slides_core::Cell { col: f.x as u32, row: f.y as u32, cols: 1, rows: 1 };
    shape(slide, layouts, &heads, &units(&content, cell))
}

fn summary<'a>(p: &Profile<'a>) -> (&'a str, SlideKind, ContentLayout, usize, bool, bool) {
    (p.template.id.as_str(), p.kind, p.layout, p.blocks, p.has_title, p.has_kicker)
}

#[test]
fn profiles_of_essentiel_follow_their_sample_slides() {
    use ContentLayout as L;
    use SlideKind as K;
    let (templates, registry) = (essentiel(), registry());
    let rows: Vec<_> = profiles(&templates, &registry).iter().map(summary).collect();
    assert_that!(rows).is_equal_to(vec![
        ("texte", K::Content, L::Default, 1, true, true),
        ("cartes", K::Content, L::Columns, 3, true, true),
        ("colonnes", K::Content, L::Split, 2, true, false),
        ("citation", K::Content, L::Quote, 1, true, false),
        ("message", K::Content, L::Center, 1, false, false),
        ("libre", K::Content, L::Free, 2, true, false),
        ("chapitre", K::Chapter, L::Default, 0, true, false),
        ("sommaire", K::Toc, L::Default, 0, false, false),
        ("couverture", K::Title, L::Default, 0, true, false),
        ("fin", K::End, L::Default, 0, false, false),
    ]);
}

#[test]
fn profiles_of_grilles_carry_their_grids() {
    let (templates, registry) = (grilles(), registry());
    let profiles = profiles(&templates, &registry);
    let grids: Vec<_> = profiles
        .iter()
        .map(|p| (p.template.id.as_str(), p.layout, p.grid.as_ref().map(|g| (g.columns, g.rows)), p.blocks))
        .collect();
    let grid = |id, g, n| (id, ContentLayout::Grid, Some(g), n);
    assert_that!(grids).is_equal_to(vec![
        grid("grille-vide", (12, 6), 0),
        grid("grille-deux-tiers", (12, 6), 2),
        grid("grille-quatre", (2, 2), 4),
        grid("grille-bandeau", (3, 3), 4),
        grid("grille-mosaique", (12, 6), 4),
        grid("grille-chiffres", (3, 2), 3),
    ]);
}

#[test]
fn profile_of_a_blank_slide_is_a_grid_without_title() {
    let mut t = essentiel().remove(0);
    t.slide = oku_slides_core::Slide::new("tpl-vierge", "blank");
    let p = profile(&t, &registry());
    assert_that!((p.kind, p.layout, p.has_title)).is_equal_to((SlideKind::Blank, ContentLayout::Grid, false));
    assert_that!(p.grid.map(|g| (g.columns, g.rows))).is_equal_to(Some((12, 6)));
}

#[test]
fn profile_records_whether_the_widget_declares_a_slot() {
    let registry = registry();
    let by_id = |id: &str| essentiel().into_iter().find(|t| t.id == id).unwrap_or_else(|| panic!("{id}"));
    assert_that!(profile(&by_id("texte"), &registry).has_slot).is_true();
    assert_that!(profile(&by_id("couverture"), &registry).has_slot).is_false();
}

fn title(text: &str) -> Item {
    with_role(text_item("t", (40.0, 20.0, 880.0, 60.0), text), Role::Title)
}

fn picture(id: &str, frame: (f64, f64, f64, f64)) -> Item {
    item(id, frame, ItemKind::Picture(Picture { asset: "sha256:a".into(), crop: Default::default() }))
}

#[test]
fn shape_reads_the_layout_kind_title_and_content() {
    let s = source_slide(
        1,
        Some("l2"),
        vec![
            title("Titre"),
            text_item("b", (40.0, 100.0, 880.0, 400.0), "Corps"),
            picture("p", (500.0, 100.0, 400.0, 300.0)),
        ],
    );
    let shape = shape_of(&s, &[layout("l1", "title"), layout("l2", "obj")]);
    assert_that!(shape).is_equal_to(Shape {
        layout_kind: Some(LayoutKind::TitleAndContent),
        title: true,
        subtitle: false,
        content: vec![ContentKind::Text, ContentKind::Picture],
        columns: 1,
        row: Vec::new(),
    });
}

#[test]
fn shape_skips_empty_boxes_footers_and_untranslatable_items_without_fallback() {
    let opaque = |id: &str, fallback: Option<String>| {
        item(id, (0.0, 0.0, 10.0, 10.0), ItemKind::Opaque(Opaque { reason: OpaqueReason::Ole, fallback }))
    };
    let s = source_slide(
        3,
        None,
        vec![
            title(""),
            with_role(text_item("d", (0.0, 500.0, 100.0, 20.0), "10/10/2026"), Role::Date),
            with_role(text_item("f", (100.0, 500.0, 100.0, 20.0), "Pied"), Role::Footer),
            with_role(text_item("n", (900.0, 500.0, 40.0, 20.0), "3"), Role::SlideNum),
            text_item("vide", (0.0, 0.0, 10.0, 10.0), "  "),
            opaque("o1", None),
            opaque("o2", Some("sha256:b".into())),
        ],
    );
    let shape = shape_of(&s, &[]);
    assert_that!((shape.layout_kind, shape.title, shape.content)).is_equal_to((
        None,
        false,
        vec![ContentKind::Picture],
    ));
}

#[rstest]
#[case::side_by_side(vec![(40.0, 100.0, 400.0, 300.0), (480.0, 100.0, 400.0, 300.0)], 2)]
#[case::three_columns(vec![(40.0, 100.0, 260.0, 300.0), (340.0, 110.0, 260.0, 290.0), (640.0, 100.0, 260.0, 300.0)], 3)]
#[case::stacked(vec![(40.0, 100.0, 880.0, 150.0), (40.0, 280.0, 880.0, 150.0)], 1)]
#[case::uneven_heights(vec![(40.0, 100.0, 400.0, 300.0), (480.0, 100.0, 400.0, 100.0)], 1)]
#[case::overlapping(vec![(40.0, 100.0, 500.0, 300.0), (400.0, 100.0, 500.0, 300.0)], 1)]
#[case::single(vec![(40.0, 100.0, 400.0, 300.0)], 1)]
#[case::none(vec![], 0)]
fn shape_counts_columns_of_side_by_side_boxes(#[case] frames: Vec<(f64, f64, f64, f64)>, #[case] columns: usize) {
    let items = frames.into_iter().enumerate().map(|(i, f)| picture(&format!("p{i}"), f)).collect();
    assert_that!(shape_of(&source_slide(1, None, items), &[]).columns).is_equal_to(columns);
}

fn wants(layout_kind: Option<&str>, subtitle: bool, content: usize, columns: usize) -> Shape {
    Shape {
        layout_kind: layout_kind.map(LayoutKind::from_ooxml),
        title: true,
        subtitle,
        content: vec![ContentKind::Text; content],
        columns,
        row: if columns >= 2 { (0..columns).collect() } else { Vec::new() },
    }
}

fn ground(id: &str, frame: (f64, f64, f64, f64)) -> Item {
    let s = crate::source::Shape {
        geom: Geometry::Preset { name: "roundRect".into(), adj: Default::default() },
        fill: Fill::Solid { color: Styled::explicit(Rgba::new(255, 255, 255)) },
        stroke: None,
        text: None,
        connection: None,
    };
    item(id, frame, ItemKind::Shape(Box::new(s)))
}

/// Slide 4 d'alto : une introduction pleine largeur, trois cartes côte à côte
/// (fond et deux textes chacune), un bandeau pleine largeur : chaque carte compte
/// une fois, et la rangée de trois fait des colonnes.
#[test]
fn shape_counts_each_card_once_and_finds_a_row_between_wide_units() {
    let mut items = vec![text_item("intro", (64.0, 144.0, 772.0, 36.0), "Intro")];
    for (k, x) in [64.0, 346.0, 627.0].into_iter().enumerate() {
        items.push(ground(&format!("c{k}"), (x, 200.0, 269.0, 172.0)));
        items.push(text_item(&format!("t{k}"), (x + 16.0, 218.0, 243.0, 18.0), "Titre"));
        items.push(text_item(&format!("b{k}"), (x + 16.0, 284.0, 243.0, 52.0), "Corps"));
    }
    items.push(ground("banner", (64.0, 394.0, 832.0, 66.0)));
    items.push(text_item("result", (96.0, 418.0, 612.0, 19.0), "Résultat"));
    let shape = shape_of(&source_slide(4, None, items), &[]);
    assert_that!(shape.content).is_equal_to(vec![
        ContentKind::Text,
        ContentKind::Card,
        ContentKind::Card,
        ContentKind::Card,
        ContentKind::Card,
    ]);
    assert_that!((shape.columns, shape.row)).is_equal_to((3, vec![1, 2, 3]));
}

/// Une unité étroite au-dessous de la rangée : pas de colonnes.
#[test]
fn shape_without_wide_neighbours_has_no_row() {
    let items = vec![
        picture("a", (40.0, 100.0, 260.0, 200.0)),
        picture("b", (340.0, 100.0, 260.0, 200.0)),
        picture("c", (40.0, 350.0, 100.0, 50.0)),
    ];
    let shape = shape_of(&source_slide(1, None, items), &[]);
    assert_that!((shape.columns, shape.row.is_empty())).is_equal_to((1, true));
}

/// Les scores sont dans la doc de `score`. Par exemple, titre seul sur Essentiel :
/// `texte` = 80 (Default) − 1 (un bloc) = 79, `libre` = 40 − 2 = 38.
#[rstest]
#[case::cover(wants(Some("title"), true, 0, 0), "couverture")]
#[case::section(wants(Some("secHead"), false, 0, 0), "chapitre")]
#[case::subtitle_only(wants(None, true, 0, 0), "couverture")]
#[case::title_only(wants(Some("titleOnly"), false, 0, 0), "texte")]
#[case::one_body(wants(Some("obj"), false, 1, 1), "texte")]
#[case::two_columns(wants(Some("twoObj"), false, 2, 2), "colonnes")]
#[case::three_columns(wants(Some("obj"), false, 3, 3), "cartes")]
#[case::row_of_three_among_five(Shape { content: vec![ContentKind::Card; 5], ..wants(None, false, 3, 3) }, "cartes")]
#[case::row_of_two_among_four(Shape { content: vec![ContentKind::Card; 4], ..wants(None, false, 2, 2) }, "colonnes")]
#[case::scattered(wants(Some("blank"), false, 4, 1), "libre")]
fn choose_on_essentiel_picks_the_matching_template(#[case] shape: Shape, #[case] expected: &str) {
    let (templates, registry) = (essentiel(), registry());
    let profiles = profiles(&templates, &registry);
    assert_that!(choose(&shape, &profiles).map(|p| p.template.id.as_str())).is_equal_to(Some(expected));
}

/// Grilles n'a que des grilles : une couverture vaut 10 partout, `grille-vide` (0 bloc) l'emporte ;
/// trois colonnes : `60 − |blocs − 3|`, `grille-chiffres` (3 blocs) l'emporte.
#[rstest]
#[case::cover(wants(Some("title"), true, 0, 0), "grille-vide")]
#[case::title_only(wants(Some("titleOnly"), false, 0, 0), "grille-vide")]
#[case::one_body(wants(Some("obj"), false, 1, 1), "grille-vide")]
#[case::two_columns(wants(Some("twoObj"), false, 2, 2), "grille-deux-tiers")]
#[case::three_columns(wants(Some("obj"), false, 3, 3), "grille-chiffres")]
#[case::four_scattered(wants(Some("obj"), false, 4, 1), "grille-quatre")]
fn choose_on_grilles_picks_the_closest_grid(#[case] shape: Shape, #[case] expected: &str) {
    let (templates, registry) = (grilles(), registry());
    let profiles = profiles(&templates, &registry);
    assert_that!(choose(&shape, &profiles).map(|p| p.template.id.as_str())).is_equal_to(Some(expected));
}

#[test]
fn choose_breaks_ties_by_pack_order() {
    let (mut templates, registry) = (essentiel(), registry());
    let mut copy = templates[0].clone();
    copy.id = "texte-bis".into();
    templates.push(copy.clone());
    let one = wants(Some("obj"), false, 1, 1);
    let first = |templates: &[Template]| choose(&one, &profiles(templates, &registry)).map(|p| p.template.id.clone());
    assert_that!(first(&templates)).is_equal_to(Some("texte".to_owned()));
    templates.insert(0, copy);
    assert_that!(first(&templates)).is_equal_to(Some("texte-bis".to_owned()));
}

#[test]
fn choose_without_positive_score_takes_the_first_content_template_else_the_first() {
    let (templates, registry) = (essentiel(), registry());
    let all = profiles(&templates, &registry);
    let pick = |ids: &[&str]| -> Vec<Profile<'_>> {
        ids.iter().filter_map(|id| all.iter().find(|p| p.template.id == *id).cloned()).collect()
    };
    let one = wants(Some("obj"), false, 1, 1);
    let no_content = pick(&["sommaire", "fin"]);
    assert_that!(choose(&one, &no_content).map(|p| p.template.id.as_str())).is_equal_to(Some("sommaire"));
    // Chapitre demandé, `message` vaut 10 − 15 (titre) − 1 (bloc) = −6 : repli sur le premier contenu.
    let section = wants(Some("secHead"), false, 0, 0);
    let with_content = pick(&["fin", "message"]);
    assert_that!(choose(&section, &with_content).map(|p| p.template.id.as_str())).is_equal_to(Some("message"));
}

#[test]
fn choose_among_no_profile_is_none() {
    assert_that!(choose(&wants(None, false, 1, 1), &[])).is_none();
}

#[test]
fn score_penalizes_a_title_mismatch_and_a_block_gap() {
    let (templates, registry) = (essentiel(), registry());
    let all = profiles(&templates, &registry);
    let message = all.iter().find(|p| p.template.id == "message").unwrap();
    let cartes = all.iter().find(|p| p.template.id == "cartes").unwrap();
    // message : Center 40, sans titre (−15), un bloc (−0).
    assert_that!(score(&wants(Some("obj"), false, 1, 1), message)).is_equal_to(25);
    // cartes : Columns 20 pour un contenu, trois blocs (−2).
    assert_that!(score(&wants(Some("obj"), false, 1, 1), cartes)).is_equal_to(18);
}
