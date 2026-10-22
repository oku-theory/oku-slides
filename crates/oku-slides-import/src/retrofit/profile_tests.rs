use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::retrofit::cluster::units;
use crate::retrofit::heading::headings;
use oku_slides_core::kinds;
use serde_json::{Value, json};

use crate::source::{Fill, Geometry, Opaque, OpaqueReason, Picture, Rgba, Size, Styled};
use crate::test_support::{essentiel_profiles, item, layout, registry, source_slide, text_item, with_role};

/// La forme de `slide` comme la calcule la projection : titres, items de
/// contenu, unités (cellules d'un pt : pas de fusion de textes éloignés).
fn shape_of(slide: &SourceSlide, layouts: &[Layout]) -> Shape {
    let items: Vec<&Item> = slide.items.iter().collect();
    let heads = headings(&items, Size { w: 960.0, h: 540.0 });
    let content = content_items(&items, &heads);
    let cell = |f: &Frame| oku_slides_core::Cell { col: f.x as u32, row: f.y as u32, cols: 1, rows: 1 };
    shape(slide, layouts, &heads, &units(&content, cell))
}

fn summary<'a>(p: &Profile<'a>) -> (&'a str, SlideRole, Arrangement, usize, bool, bool) {
    (p.source.1.id.as_str(), p.role, p.arrangement, p.blocks, p.has_title, p.has_kicker)
}

fn ns(s: &str) -> Namespace {
    Namespace::parse(s).unwrap()
}

/// Un profil `id` du layout `layout`, avec titre, sans surtitre ni paramètre.
fn profile_json(id: &str, layout: &str, role: &str, arrangement: &str, blocks: u8) -> LayoutProfile {
    serde_json::from_value(json!({
        "id": id, "name": id, "layout": layout, "role": role, "arrangement": arrangement, "blocks": blocks, "title": true,
    }))
    .unwrap()
}

/// Le registre des packs intégrés, plus les layouts d'`essentiel` copiés sous chaque espace de noms de `names`.
fn registry_with_layout_packs(names: &[&str]) -> WidgetRegistry {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/builtin-registry.json");
    let mut probe: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let packs = probe["packs"].as_array_mut().unwrap();
    let essentiel = packs.iter().find(|p| p["namespace"] == "essentiel").unwrap().clone();
    for name in names {
        let mut copy = essentiel.clone();
        copy["namespace"] = json!(name);
        packs.push(copy);
    }
    WidgetRegistry::from_probe(probe).unwrap()
}

/// Une slide titrée à un seul texte.
fn shape_single_text() -> Shape {
    wants(Some("obj"), false, 1, 1)
}

/// Une forme de slide qui demande `want`.
fn shape_wanting(want: Want) -> Shape {
    match want {
        Want::Title => wants(Some("title"), true, 0, 0),
        Want::Chapter => wants(Some("secHead"), false, 0, 0),
        Want::Empty => wants(None, false, 0, 0),
        Want::Single => shape_single_text(),
        Want::Columns(n) => wants(None, false, n, n),
        Want::Many => wants(None, false, 4, 1),
    }
}

/// Le profil d'un layout `content` de rôle `role`, agencement par défaut, un bloc.
fn profile_with_role(role: SlideRole) -> Profile<'static> {
    let role = serde_json::to_value(role).unwrap();
    let p = profile_json("p", "content", role.as_str().unwrap(), "default", 1);
    let source: &'static (Namespace, LayoutProfile) = Box::leak(Box::new((kinds::essentiel::namespace(), p)));
    profile(source, &registry())
}

#[test]
fn profiles_of_essentiel_read_their_hints() {
    use Arrangement as A;
    use SlideRole as R;
    let (list, registry) = (essentiel_profiles(), registry());
    let rows: Vec<_> = profiles(&list, &registry).iter().map(summary).collect();
    assert_that!(rows).is_equal_to(vec![
        ("texte", R::Content, A::Default, 1, true, true),
        ("cartes", R::Content, A::Columns, 3, true, true),
        ("colonnes", R::Content, A::Split, 2, true, false),
        ("citation", R::Content, A::Quote, 1, true, false),
        ("message", R::Content, A::Center, 1, false, false),
        ("libre", R::Content, A::Free, 2, true, false),
        ("chapitre", R::Chapter, A::Default, 0, true, false),
        ("sommaire", R::Toc, A::Default, 0, false, false),
        ("couverture", R::Title, A::Default, 0, true, false),
        ("fin", R::End, A::Default, 0, false, false),
    ]);
}

#[test]
fn profile_grid_comes_from_its_params() {
    let mut p = profile_json("libre", "blank", "blank", "grid", 0);
    p.params.insert("grid".into(), json!({"columns": 4, "rows": 2}));
    let list = vec![(kinds::essentiel::namespace(), p)];
    let profiles = profiles(&list, &registry());
    assert_that!(profiles[0].grid.as_ref().map(|g| (g.columns, g.rows))).is_equal_to(Some((4, 2)));
}

#[test]
fn profile_grid_without_param_takes_the_layout_default() {
    let list = vec![(kinds::essentiel::namespace(), profile_json("vierge", "blank", "blank", "grid", 0))];
    let profiles = profiles(&list, &registry());
    assert_that!(profiles[0].grid.as_ref().map(|g| (g.columns, g.rows))).is_equal_to(Some((12, 6)));
}

#[test]
fn profile_without_grid_arrangement_has_no_grid() {
    let list = vec![(kinds::essentiel::namespace(), profile_json("texte", "content", "content", "default", 1))];
    assert_that!(profiles(&list, &registry())[0].grid).is_none();
}

#[test]
fn profile_records_whether_the_layout_declares_a_slot() {
    let (list, registry) = (essentiel_profiles(), registry());
    let all = profiles(&list, &registry);
    let by_id = |id: &str| all.iter().find(|p| p.source.1.id == id).unwrap_or_else(|| panic!("{id}"));
    assert_that!((by_id("texte").has_slot, by_id("couverture").has_slot)).is_equal_to((true, false));
}

#[test]
fn profile_names_and_kind_are_qualified_by_the_pack() {
    let (list, registry) = (essentiel_profiles(), registry());
    let all = profiles(&list, &registry);
    assert_that!((all[0].name(), all[0].kind()))
        .is_equal_to(("essentiel/texte".to_owned(), kinds::essentiel::CONTENT.to_owned()));
}

#[test]
fn slide_params_copy_every_profile_param() {
    let mut p = profile_json("p", "content", "content", "columns", 3);
    p.params.insert("layout".into(), json!("columns"));
    p.params.insert("grid".into(), json!({"columns": 3, "rows": 1}));
    let params = slide_params(&p);
    assert_that!((params.text("layout"), params.value("grid").is_some())).is_equal_to((Some("columns"), true));
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
fn choose_on_essentiel_picks_the_matching_profile(#[case] shape: Shape, #[case] expected: &str) {
    let (list, registry) = (essentiel_profiles(), registry());
    let profiles = profiles(&list, &registry);
    assert_that!(choose(&shape, &profiles).map(|p| p.source.1.id.as_str())).is_equal_to(Some(expected));
}

#[test]
fn choose_breaks_ties_by_manifest_order() {
    let (mut list, registry) = (essentiel_profiles(), registry());
    let mut copy = list[0].clone();
    copy.1.id = "texte-bis".into();
    list.push(copy.clone());
    let first = |list: &[(Namespace, LayoutProfile)]| {
        choose(&shape_single_text(), &profiles(list, &registry)).map(|p| p.source.1.id.clone())
    };
    assert_that!(first(&list)).is_equal_to(Some("texte".to_owned()));
    list.insert(0, copy);
    assert_that!(first(&list)).is_equal_to(Some("texte-bis".to_owned()));
}

#[test]
fn profiles_of_two_packs_keep_target_order_on_ties() {
    let a = (ns("un"), profile_json("p", "content", "content", "default", 1));
    let b = (ns("deux"), profile_json("p", "content", "content", "default", 1));
    let list = vec![a, b];
    let registry = registry_with_layout_packs(&["un", "deux"]);
    let profiles = profiles(&list, &registry);
    let chosen = choose(&shape_single_text(), &profiles).unwrap();
    assert_that!(chosen.source.0.as_str()).is_equal_to("un");
}

#[test]
fn choose_without_positive_score_takes_the_first_content_profile_else_the_first() {
    let (list, registry) = (essentiel_profiles(), registry());
    let all = profiles(&list, &registry);
    let pick = |ids: &[&str]| -> Vec<Profile<'_>> {
        ids.iter().filter_map(|id| all.iter().find(|p| p.source.1.id == *id).cloned()).collect()
    };
    let no_content = pick(&["sommaire", "fin"]);
    assert_that!(choose(&shape_single_text(), &no_content).map(|p| p.source.1.id.as_str()))
        .is_equal_to(Some("sommaire"));
    // Chapitre demandé, `message` vaut 10 − 15 (titre) − 1 (bloc) = −6 : repli sur le premier contenu.
    let section = wants(Some("secHead"), false, 0, 0);
    let with_content = pick(&["fin", "message"]);
    assert_that!(choose(&section, &with_content).map(|p| p.source.1.id.as_str())).is_equal_to(Some("message"));
}

#[test]
fn choose_among_no_profile_is_none() {
    assert_that!(choose(&wants(None, false, 1, 1), &[])).is_none();
}

#[test]
fn score_penalizes_a_title_mismatch_and_a_block_gap() {
    let (list, registry) = (essentiel_profiles(), registry());
    let all = profiles(&list, &registry);
    let by_id = |id: &str| all.iter().find(|p| p.source.1.id == id).unwrap();
    // message : Center 40, sans titre (−15), un bloc (−0).
    assert_that!(score(&shape_single_text(), by_id("message"))).is_equal_to(25);
    // cartes : Columns 20 pour un contenu, trois blocs (−2).
    assert_that!(score(&shape_single_text(), by_id("cartes"))).is_equal_to(18);
}

#[rstest]
#[case(SlideRole::Title, Want::Title, 100)]
#[case(SlideRole::Toc, Want::Single, 0)]
#[case(SlideRole::End, Want::Single, 0)]
fn score_by_role_matches_the_former_slide_kinds(#[case] role: SlideRole, #[case] want: Want, #[case] expected: i32) {
    let p = profile_with_role(role);
    assert_that!(score(&shape_wanting(want), &p)).is_equal_to(expected);
}
