use std::collections::BTreeMap;

use oku_slides_core::{Cell, Deck, Template, asset_id};
use speculoos::prelude::*;

use super::*;
use crate::report::RunEffect;
use crate::retrofit::template::profiles;
use crate::source::{Asset, Fill, Geometry, ItemKind, Picture, Rgba, Role, Styled};
use crate::test_support::{
    PAPIER, applied, colors, essentiel, grilles, item, layout, para, registry, run_with, source_deck, source_slide,
    text_box, text_item, with_role,
};

/// Layouts des slides de test : couverture, contenu, chapitre.
fn layouts() -> Vec<Layout> {
    vec![layout("l1", "title"), layout("l2", "obj"), layout("l3", "secHead")]
}

fn png() -> String {
    asset_id(b"png")
}

/// Les slides `slides` projetées sur `templates` et Papier, dans l'ordre ; le
/// deck obtenu par `apply_all` (aucune `Issue`).
fn project(templates: &[Template], slides: &[SourceSlide]) -> (Vec<Op>, Vec<SlideReport>, Deck) {
    let (registry, tokens, layouts) = (registry(), colors(PAPIER), layouts());
    let assets = BTreeMap::from([(png(), Asset { mime: "image/png".into(), bytes: b"png".to_vec() })]);
    let page = PageTokens::default();
    let decor = Decor::of(&source_deck(slides.to_vec()));
    let cx = SlideCx {
        registry: &registry,
        page: &page,
        content: ContentCx { tokens: &tokens, assets: &assets, track: (0.0, 0.0) },
        layouts: &layouts,
        size: Size { w: 960.0, h: 540.0 },
        scale: 2.0,
        decor: &decor,
    };
    let profiles = profiles(templates, &registry);
    let mut titled = false;
    let (mut ops, mut reports) = (Vec::new(), Vec::new());
    for s in slides {
        let p = project_slide(s, &profiles, &mut titled, &cx).unwrap();
        ops.extend(p.ops);
        reports.push(p.report);
    }
    let deck = applied(ops.clone(), &[png()]);
    (ops, reports, deck)
}

fn title(text: &str) -> Item {
    with_role(text_item("t", (66.0, 28.0, 828.0, 104.0), text), Role::Title)
}

fn subtitle(text: &str) -> Item {
    with_role(text_item("st", (120.0, 283.0, 720.0, 130.0), text), Role::Subtitle)
}

fn cover(number: u32, t: &str, st: &str) -> SourceSlide {
    source_slide(number, Some("l1"), vec![title(t), subtitle(st)])
}

fn block_texts(slide: &Slide, slot: &str) -> Vec<String> {
    slide
        .params
        .slot(slot)
        .unwrap_or_default()
        .iter()
        .map(|e| {
            let body = e.params.rich("body").unwrap_or_default();
            body.iter().flat_map(|p| &p.runs).map(|r| r.t.as_str()).collect()
        })
        .collect()
}

#[test]
fn project_slide_first_cover_fills_the_deck_params() {
    let (ops, reports, deck) = project(&essentiel(), &[cover(1, "Rapport annuel", "Exercice 2026")]);
    assert_that!(ops[..2].to_vec()).is_equal_to(vec![
        Op::SetDeckParam { key: "title".into(), value: "Rapport annuel".into() },
        Op::SetDeckParam { key: "subtitle".into(), value: "Exercice 2026".into() },
    ]);
    assert_that!((deck.slides[0].kind.as_str(), deck.slides[0].params.is_empty())).is_equal_to(("title", true));
    assert_that!((deck.params.title.as_deref(), reports[0].template.as_str()))
        .is_equal_to((Some("Rapport annuel"), "couverture"));
}

#[test]
fn project_slide_later_cover_becomes_a_chapter() {
    let (_, reports, deck) = project(&essentiel(), &[cover(1, "Rapport", "2026"), cover(2, "Partie 2", "Les comptes")]);
    let s2 = &deck.slides[1];
    assert_that!((s2.kind.as_str(), s2.params.text("title"), s2.params.text("subtitle"))).is_equal_to((
        "chapter",
        Some("Partie 2"),
        Some("Les comptes"),
    ));
    assert_that!((deck.params.title.as_deref(), reports[1].template.as_str()))
        .is_equal_to((Some("Rapport"), "chapitre"));
}

#[test]
fn project_slide_title_and_body_go_to_flow_blocks() {
    let body = text_item("b", (66.0, 143.0, 828.0, 342.0), "Chiffre d'affaires en hausse");
    let (_, reports, deck) = project(&essentiel(), &[source_slide(2, Some("l2"), vec![title("Résultats"), body])]);
    let s = &deck.slides[0];
    assert_that!((s.id.as_str(), s.kind.as_str(), s.params.text("title"))).is_equal_to((
        "s2",
        "content",
        Some("Résultats"),
    ));
    assert_that!(block_texts(s, "blocks")).is_equal_to(vec!["Chiffre d'affaires en hausse".to_owned()]);
    assert_that!(s.params.slot("blocks").unwrap()[0].id.as_str()).is_equal_to("s2-1");
    assert_that!((reports[0].template.as_str(), reports[0].placement)).is_equal_to(("texte", Placement::Flow));
}

#[test]
fn project_slide_two_columns_keep_the_split_layout_in_reading_order() {
    let right = text_item("r", (500.0, 143.0, 394.0, 300.0), "Droite");
    let left = text_item("l", (66.0, 143.0, 394.0, 300.0), "Gauche");
    let (_, reports, deck) =
        project(&essentiel(), &[source_slide(1, Some("l2"), vec![title("Avant, après"), right, left])]);
    let s = &deck.slides[0];
    assert_that!(s.params.text("layout")).is_equal_to(Some("split"));
    assert_that!(block_texts(s, "blocks")).is_equal_to(vec!["Gauche".to_owned(), "Droite".to_owned()]);
    assert_that!(reports[0].template.as_str()).is_equal_to("colonnes");
}

#[test]
fn project_slide_scattered_items_snap_to_the_template_grid() {
    let boxes = [(60.0, 125.0), (500.0, 125.0), (60.0, 300.0), (500.0, 300.0)];
    let mut items = vec![title("Quatre idées")];
    items.extend(
        boxes.iter().enumerate().map(|(i, (x, y))| text_item(&format!("i{i}"), (*x, *y, 400.0, 150.0), "Idée")),
    );
    let (_, reports, deck) = project(&grilles(), &[source_slide(1, Some("l2"), items)]);
    let s = &deck.slides[0];
    let cells: Vec<Option<Cell>> = s.params.slot("elements").unwrap().iter().map(|e| e.cell).collect();
    let cell = |col, row| Some(Cell { col, row, cols: 1, rows: 1 });
    assert_that!(cells).is_equal_to(vec![cell(0, 0), cell(1, 0), cell(0, 1), cell(1, 1)]);
    assert_that!(s.params.value("grid")).is_equal_to(Some(&serde_json::json!({ "columns": 2, "rows": 2 })));
    assert_that!((reports[0].template.as_str(), reports[0].placement)).is_equal_to(("grille-quatre", Placement::Grid));
}

#[test]
fn project_slide_notes_become_a_set_notes_op() {
    let mut s = source_slide(3, Some("l2"), vec![title("Résultats")]);
    s.notes = "Dire bonjour.".into();
    let (ops, _, deck) = project(&essentiel(), &[s]);
    assert_that!(ops.last()).is_equal_to(Some(&Op::SetNotes { slide: "s3".into(), notes: "Dire bonjour.".into() }));
    assert_that!(deck.slides[0].notes.as_str()).is_equal_to("Dire bonjour.");
}

#[test]
fn project_slide_hidden_slide_is_imported_with_a_loss() {
    let mut s = source_slide(1, Some("l2"), vec![title("Annexe")]);
    s.hidden = true;
    let (_, reports, deck) = project(&essentiel(), &[s]);
    assert_that!(deck.slides.len()).is_equal_to(1);
    assert_that!(reports[0].losses.clone()).is_equal_to(vec![Loss::slide(LossKind::HiddenSlide)]);
}

#[test]
fn project_slide_date_footer_and_number_are_skipped() {
    let items = vec![
        title("Résultats"),
        text_item("b", (66.0, 143.0, 828.0, 342.0), "Corps"),
        with_role(text_item("d", (66.0, 500.0, 200.0, 20.0), "10/10/2026"), Role::Date),
        with_role(text_item("f", (300.0, 500.0, 300.0, 20.0), "Confidentiel"), Role::Footer),
        with_role(text_item("n", (800.0, 500.0, 60.0, 20.0), "3"), Role::SlideNum),
    ];
    let (_, _, deck) = project(&essentiel(), &[source_slide(1, Some("l2"), items)]);
    assert_that!(block_texts(&deck.slides[0], "blocks")).is_equal_to(vec!["Corps".to_owned()]);
}

#[test]
fn project_slide_group_falls_back_to_a_stack_with_child_losses_on_source_ids() {
    let underlined = run_with("Droite", |s| s.underline = true);
    let children = vec![
        text_item("g/1", (66.0, 432.0, 200.0, 40.0), "Gauche"),
        bare("g/arrow", "rightArrow", (270.0, 442.0, 26.0, 20.0)),
        item("g/2", (300.0, 432.0, 200.0, 40.0), ItemKind::Text(text_box(vec![para(vec![underlined])]))),
    ];
    let group = item("g", (66.0, 432.0, 434.0, 40.0), ItemKind::Group { items: children, smartart: false });
    let (_, reports, deck) = project(&essentiel(), &[source_slide(1, Some("l2"), vec![title("Processus"), group])]);
    let stack = &deck.slides[0].params.slot("blocks").unwrap()[0];
    let ids: Vec<&str> = stack.params.slot("children").unwrap().iter().map(|c| c.id.as_str()).collect();
    assert_that!((stack.kind.as_str(), ids)).is_equal_to(("stack", vec!["s1-1-1", "s1-1-2"]));
    assert_that!(reports[0].fallbacks.clone()).is_equal_to(vec!["stack".to_owned()]);
    assert_that!(reports[0].losses.clone())
        .is_equal_to(vec![Loss::on("g/2", LossKind::RunStyle(RunEffect::Underline))]);
}

#[test]
fn project_slide_subtitle_without_a_param_becomes_content() {
    let (_, reports, deck) = project(&grilles(), &[cover(1, "Rapport annuel", "Exercice 2026")]);
    let s = &deck.slides[0];
    assert_that!((s.kind.as_str(), s.params.text("title"))).is_equal_to(("content", Some("Rapport annuel")));
    assert_that!(block_texts(s, "elements")).is_equal_to(vec!["Exercice 2026".to_owned()]);
    assert_that!((deck.params.subtitle.as_deref(), reports[0].template.as_str()))
        .is_equal_to((Some("Exercice 2026"), "grille-vide"));
}

#[test]
fn project_slide_cover_with_content_avoids_templates_without_a_slot() {
    let logo =
        item("p", (400.0, 420.0, 160.0, 90.0), ItemKind::Picture(Picture { asset: png(), crop: Default::default() }));
    let (_, reports, deck) = project(&essentiel(), &[source_slide(1, Some("l1"), vec![title("Rapport"), logo])]);
    let s = &deck.slides[0];
    assert_that!((s.kind.as_str(), s.params.text("title"))).is_equal_to(("content", Some("Rapport")));
    assert_that!(s.params.slot("blocks").unwrap()[0].params.text("asset")).is_equal_to(Some(png().as_str()));
    assert_that!((deck.params.title.as_deref(), reports[0].template.as_str())).is_equal_to((Some("Rapport"), "texte"));
}

#[test]
fn project_slide_collects_the_assets_of_its_images() {
    let registry = registry();
    let tokens = colors(PAPIER);
    let assets = BTreeMap::from([(png(), Asset { mime: "image/png".into(), bytes: b"png".to_vec() })]);
    let (page, layouts, templates, decor) = (PageTokens::default(), layouts(), essentiel(), Decor::default());
    let cx = SlideCx {
        registry: &registry,
        page: &page,
        content: ContentCx { tokens: &tokens, assets: &assets, track: (0.0, 0.0) },
        layouts: &layouts,
        size: Size { w: 960.0, h: 540.0 },
        scale: 2.0,
        decor: &decor,
    };
    let pic =
        item("p", (66.0, 143.0, 300.0, 200.0), ItemKind::Picture(Picture { asset: png(), crop: Default::default() }));
    let src = source_slide(1, Some("l2"), vec![title("Image"), pic]);
    let p = project_slide(&src, &profiles(&templates, &registry), &mut false, &cx).unwrap();
    assert_that!(p.assets.into_iter().collect::<Vec<_>>()).is_equal_to(vec![png()]);
}

#[test]
fn project_slide_title_run_effects_are_lost_on_the_title_item() {
    let t = with_role(
        item(
            "t",
            (66.0, 28.0, 828.0, 104.0),
            ItemKind::Text(text_box(vec![para(vec![run_with("Barré", |s| s.strike = true)])])),
        ),
        Role::Title,
    );
    let (_, reports, _) = project(
        &essentiel(),
        &[source_slide(1, Some("l2"), vec![t, text_item("b", (66.0, 143.0, 828.0, 342.0), "Corps")])],
    );
    assert_that!(reports[0].losses.clone()).is_equal_to(vec![Loss::on("t", LossKind::RunStyle(RunEffect::Strike))]);
}

/// Une forme sans texte remplie de blanc.
fn bare(id: &str, preset: &str, frame: (f64, f64, f64, f64)) -> Item {
    let s = crate::source::Shape {
        geom: Geometry::Preset { name: preset.into(), adj: BTreeMap::new() },
        fill: Fill::Solid { color: Styled::explicit(Rgba::new(255, 255, 255)) },
        stroke: None,
        text: None,
        connection: None,
    };
    item(id, frame, ItemKind::Shape(Box::new(s)))
}

/// Une boîte de texte d'un paragraphe en taille `size`.
fn sized(id: &str, frame: (f64, f64, f64, f64), text: &str, size: f64) -> Item {
    item(id, frame, ItemKind::Text(text_box(vec![para(vec![run_with(text, |s| s.size = size)])])))
}

/// Une slide d'alto sans placeholder : surtitre, titre 28 pt, filet d'accent,
/// trois cartes (fond, titre 13 pt, corps 10,5 pt).
fn conviction() -> SourceSlide {
    let mut items = vec![
        sized("k", (64.0, 56.0, 857.0, 15.0), "OUR CONVICTION", 9.5),
        bare("bar", "rect", (64.0, 78.0, 32.0, 2.5)),
        sized("t", (64.0, 90.0, 772.0, 66.0), "The agent becomes the only interface", 28.0),
    ];
    for (k, x) in [64.0, 341.0, 619.0].into_iter().enumerate() {
        items.push(bare(&format!("c{k}"), "rect", (x, 176.0, 277.0, 284.0)));
        items.push(sized(&format!("h{k}"), (x + 16.0, 196.0, 253.0, 18.0), &format!("Carte {k}"), 13.0));
        items.push(sized(&format!("b{k}"), (x + 16.0, 220.0, 253.0, 69.0), &format!("Corps {k}"), 10.5));
    }
    source_slide(2, None, items)
}

#[test]
fn project_slide_without_placeholders_infers_its_title_and_kicker_and_makes_cards() {
    let (_, reports, deck) = project(&essentiel(), &[conviction()]);
    let s = &deck.slides[0];
    assert_that!((s.params.text("kicker"), s.params.text("title")))
        .is_equal_to((Some("OUR CONVICTION"), Some("The agent becomes the only interface")));
    let blocks = s.params.slot("blocks").unwrap();
    let cards: Vec<(&str, Option<&str>)> = blocks.iter().map(|e| (e.kind.as_str(), e.params.text("title"))).collect();
    assert_that!(cards).is_equal_to(vec![
        ("card", Some("Carte 0")),
        ("card", Some("Carte 1")),
        ("card", Some("Carte 2")),
    ]);
    assert_that!(block_texts(s, "blocks")).is_equal_to(vec![
        "Corps 0".to_owned(),
        "Corps 1".to_owned(),
        "Corps 2".to_owned(),
    ]);
    assert_that!((reports[0].template.as_str(), s.params.text("layout"))).is_equal_to(("cartes", Some("columns")));
}

/// Une introduction et un bandeau pleine largeur autour de la rangée : agencement
/// `default`, la rangée en pile de colonnes à sa place de lecture.
#[test]
fn project_slide_row_between_wide_units_becomes_a_stack_of_columns() {
    let mut slide = conviction();
    slide.items.push(sized("intro", (64.0, 160.0, 772.0, 12.0), "Introduction", 10.5));
    for card in
        slide.items.iter_mut().filter(|i| i.id.starts_with('c') || i.id.starts_with('h') || i.id.starts_with('b'))
    {
        card.frame.y += 20.0;
        card.frame.h = card.frame.h.min(200.0);
    }
    slide.items.push(bare("banner", "rect", (64.0, 410.0, 832.0, 60.0)));
    slide.items.push(sized("result", (96.0, 420.0, 612.0, 20.0), "Résultat", 12.0));
    let (_, reports, deck) = project(&essentiel(), &[slide]);
    let s = &deck.slides[0];
    let blocks = s.params.slot("blocks").unwrap();
    let kinds: Vec<(&str, &str)> = blocks.iter().map(|e| (e.id.as_str(), e.kind.as_str())).collect();
    assert_that!(kinds).is_equal_to(vec![("s2-1", "text"), ("s2-2", "stack"), ("s2-3", "card")]);
    let row: Vec<&str> = blocks[1].params.slot("children").unwrap().iter().map(|c| c.id.as_str()).collect();
    assert_that!(row).is_equal_to(vec!["s2-2-1", "s2-2-2", "s2-2-3"]);
    assert_that!((reports[0].template.as_str(), s.params.text("layout"))).is_equal_to(("cartes", Some("default")));
}

/// `libre` n'a pas de grille : les unités sont posées en cadres, leur boîte
/// englobante ajustée dans la zone de corps.
#[test]
fn project_slide_on_a_free_template_places_frames_not_cells() {
    let items = vec![
        title("Libre"),
        text_item("a", (66.0, 150.0, 300.0, 60.0), "A"),
        text_item("b", (500.0, 260.0, 300.0, 60.0), "B"),
        text_item("c", (66.0, 380.0, 700.0, 60.0), "C"),
    ];
    let (_, reports, deck) = project(&essentiel(), &[source_slide(1, Some("l2"), items)]);
    let elements = deck.slides[0].params.slot("elements").unwrap();
    assert_that!(reports[0].template.as_str()).is_equal_to("libre");
    assert_that!(elements.iter().all(|e| e.frame.is_some() && e.cell.is_none())).is_true();
}

/// Un modèle sans aucun slot (une couverture seule dans le pack) : le contenu
/// est dit perdu, jamais omis en silence. Le titre, que le widget `title` lie au
/// deck sans être une couverture, devient un contenu lui aussi.
#[test]
fn project_slide_content_without_a_slot_is_reported() {
    let cover_only: Vec<Template> = essentiel().into_iter().filter(|t| t.id == "couverture").collect();
    let body = text_item("b", (66.0, 143.0, 828.0, 342.0), "Corps");
    let (_, reports, _) = project(&cover_only, &[source_slide(1, Some("l2"), vec![title("Seul"), body])]);
    assert_that!(reports[0].losses.clone())
        .is_equal_to(vec![Loss::on("t", LossKind::NoSlot), Loss::on("b", LossKind::NoSlot)]);
}

#[test]
fn project_slide_never_flows_a_bare_shape() {
    let items = vec![title("Formes"), bare("deco", "ellipse", (400.0, 200.0, 100.0, 100.0))];
    let (_, _, deck) = project(&essentiel(), &[source_slide(1, Some("l2"), items)]);
    assert_that!(deck.slides[0].params.slot("blocks")).is_none();
}
