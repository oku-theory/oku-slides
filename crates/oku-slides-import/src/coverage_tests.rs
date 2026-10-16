use std::collections::BTreeMap;

use oku_slides_core::{Deck, Element, Paragraph, Run as OkuRun};
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::retrofit::decor::Decor;
use crate::source::{
    Caps, Cell, Chart, ChartKind, Grouping, Opaque, OpaqueReason, Picture, Role, SLIDE_NUMBER, Series, Size,
    SourceSlide, Table, TableFlags,
};
use crate::test_support::{
    element_with, item, para, registry, run, run_with, source_slide, text_box, text_item, with_role,
};

/// La couverture comme la mesure le retrofit : le décor qu'il omet est hors du compte.
fn coverage(
    source: &SourceDeck,
    deck: &Deck,
    registry: &WidgetRegistry,
    origins: &BTreeMap<String, String>,
    deck_slide: Option<u32>,
) -> Coverage {
    let decor = Decor::of(source);
    super::coverage(source, deck, registry, origins, deck_slide, &|i| decor.skips(i))
}

fn source(slides: Vec<SourceSlide>) -> SourceDeck {
    SourceDeck {
        size: Size { w: 960.0, h: 540.0 },
        themes: Vec::new(),
        masters: Vec::new(),
        layouts: Vec::new(),
        slides,
        assets: BTreeMap::new(),
        fonts: Vec::new(),
        warnings: Vec::new(),
    }
}

/// Un deck d'une slide `content` `s1`, de titre `title`, aux blocs `blocks`.
fn deck(title: Option<&str>, blocks: Vec<Element>) -> Deck {
    let mut s = Slide::new("s1", "content");
    if let Some(t) = title {
        s.params.set("title", ParamValue::Text(t.into()));
    }
    if !blocks.is_empty() {
        s.params.set("blocks", ParamValue::Slot(blocks));
    }
    let mut d = Deck::empty();
    d.slides.push(s);
    d
}

fn text(id: &str, t: &str) -> Element {
    Element::text(id, vec![Paragraph { runs: vec![OkuRun { t: t.into(), ..OkuRun::default() }], list: None }])
}

fn image(id: &str, asset: &str) -> Element {
    let mut e = Element::new(id, "image");
    e.params.set("asset", ParamValue::Text(asset.into()));
    e
}

fn cell(t: &str, covered: bool) -> Cell {
    Cell {
        text: text_box(vec![para(vec![run(t)])]),
        fill: Fill::None,
        borders: Default::default(),
        row_span: 1,
        col_span: if t == "Titre" { 2 } else { 1 },
        covered,
    }
}

fn table_item() -> Item {
    let t = Table {
        col_widths: vec![100.0, 100.0],
        row_heights: vec![20.0, 20.0],
        flags: TableFlags::default(),
        rows: vec![vec![cell("Titre", false), cell("", true)], vec![cell("A", false), cell("1", false)]],
    };
    item("tb", (0.0, 0.0, 200.0, 40.0), ItemKind::Table(t))
}

fn chart_item(kind: ChartKind) -> Item {
    let series = |name: &str, v: [f64; 2]| Series {
        name: name.into(),
        values: v.map(Some).to_vec(),
        color: None,
        x: None,
        format: None,
    };
    let c = Chart {
        kind,
        grouping: Grouping::Clustered,
        title: None,
        categories: vec!["T1".into(), "T2".into()],
        series: vec![series("Nord", [1.0, 2.0]), series("Sud", [3.0, 4.0])],
    };
    item("ch", (0.0, 0.0, 200.0, 100.0), ItemKind::Chart(c))
}

#[test]
fn coverage_finds_runs_in_plain_rich_and_deck_params() {
    let title = with_role(text_item("t", (0.0, 0.0, 100.0, 20.0), "Résultats"), Role::Title);
    let body = item(
        "b",
        (0.0, 30.0, 100.0, 20.0),
        ItemKind::Text(text_box(vec![para(vec![run("Chiffre  d'affaires"), run(" en hausse")])])),
    );
    let sub = with_role(text_item("st", (0.0, 60.0, 100.0, 20.0), "Exercice 2026"), Role::Subtitle);
    let mut d = deck(Some("Résultats"), vec![text("s1-1", "Chiffre d'affaires en hausse")]);
    d.params.subtitle = Some("Exercice 2026".into());
    let c = coverage(
        &source(vec![source_slide(1, None, vec![title, body, sub])]),
        &d,
        &registry(),
        &BTreeMap::new(),
        Some(1),
    );
    assert_that!((c.runs, c.missing.is_empty())).is_equal_to((Ratio { found: 4, total: 4 }, true));
}

#[test]
fn coverage_reports_a_missing_run_with_its_slide() {
    let src = source(vec![source_slide(1, None, vec![text_item("b", (0.0, 0.0, 10.0, 10.0), "Perdu")])]);
    let c = coverage(&src, &deck(Some("Autre"), Vec::new()), &registry(), &BTreeMap::new(), Some(1));
    assert_that!((c.runs, c.missing))
        .is_equal_to((Ratio { found: 0, total: 1 }, vec![Missing::Run { slide: 1, text: "Perdu".into() }]));
}

#[test]
fn coverage_compares_each_source_slide_with_its_own_slide() {
    let src = source(vec![
        source_slide(1, None, vec![text_item("a", (0.0, 200.0, 10.0, 10.0), "Ici")]),
        source_slide(2, None, vec![text_item("b", (0.0, 200.0, 10.0, 10.0), "Ici")]),
    ]);
    let c = coverage(&src, &deck(None, vec![text("s1-1", "Ici")]), &registry(), &BTreeMap::new(), Some(1));
    assert_that!(c.missing).is_equal_to(vec![Missing::Run { slide: 2, text: "Ici".into() }]);
}

#[test]
fn coverage_skips_date_footer_and_slide_number() {
    let items = [Role::Date, Role::Footer, Role::SlideNum]
        .into_iter()
        .map(|r| with_role(text_item("x", (0.0, 0.0, 10.0, 10.0), "10/10/2026"), r))
        .collect();
    let c = coverage(
        &source(vec![source_slide(1, None, items)]),
        &deck(None, Vec::new()),
        &registry(),
        &BTreeMap::new(),
        Some(1),
    );
    assert_that!(c.runs).is_equal_to(Ratio::default());
}

#[test]
fn coverage_without_omission_counts_date_footer_and_slide_number() {
    let items: Vec<Item> = [Role::Date, Role::Footer]
        .into_iter()
        .map(|r| with_role(text_item("x", (0.0, 0.0, 10.0, 10.0), "10/10/2026"), r))
        .collect();
    let src = source(vec![source_slide(1, None, items)]);
    let c = super::coverage(&src, &deck(None, Vec::new()), &registry(), &BTreeMap::new(), None, &|_| false);
    assert_that!(c.runs).is_equal_to(Ratio { found: 0, total: 2 });
}

#[test]
fn coverage_skips_a_slide_number_field() {
    let mut number = run("7");
    number.field = Some(SLIDE_NUMBER.into());
    let tb = text_box(vec![para(vec![run("Page "), number])]);
    let src = source(vec![source_slide(1, None, vec![item("n", (0.0, 200.0, 10.0, 10.0), ItemKind::Text(tb))])]);
    let c = super::coverage(
        &src,
        &deck(None, vec![text("s1-1", "Page {n}")]),
        &registry(),
        &BTreeMap::new(),
        None,
        &|_| false,
    );
    assert_that!((c.runs, c.missing.is_empty())).is_equal_to((Ratio { found: 1, total: 1 }, true));
}

#[test]
fn coverage_counts_runs_inside_groups() {
    let group = item(
        "g",
        (0.0, 0.0, 100.0, 20.0),
        ItemKind::Group {
            items: vec![
                text_item("g/1", (0.0, 0.0, 50.0, 20.0), "Étape 1"),
                text_item("g/2", (50.0, 0.0, 50.0, 20.0), "Étape 2"),
            ],
            smartart: None,
        },
    );
    let mut card = Element::new("s1-1-1", "card");
    card.params.set("title", ParamValue::Text("Étape 1".into()));
    let stack = {
        let mut e = Element::new("s1-1", "stack");
        e.params.set("children", ParamValue::Slot(vec![card, text("s1-1-2", "Étape 2")]));
        e
    };
    let c = coverage(
        &source(vec![source_slide(1, None, vec![group])]),
        &deck(None, vec![stack]),
        &registry(),
        &BTreeMap::new(),
        Some(1),
    );
    assert_that!(c.runs).is_equal_to(Ratio { found: 2, total: 2 });
}

#[test]
fn coverage_counts_images_and_fallbacks_by_asset() {
    let pic = item(
        "p",
        (0.0, 0.0, 10.0, 10.0),
        ItemKind::Picture(Picture { asset: "sha256:a".into(), crop: Default::default() }),
    );
    let math = item(
        "m",
        (0.0, 0.0, 10.0, 10.0),
        ItemKind::Opaque(Opaque { reason: OpaqueReason::Math, fallback: Some("sha256:b".into()) }),
    );
    let c = coverage(
        &source(vec![source_slide(1, None, vec![pic, math])]),
        &deck(None, vec![image("s1-1", "sha256:a")]),
        &registry(),
        &BTreeMap::new(),
        Some(1),
    );
    assert_that!((c.images, c.missing))
        .is_equal_to((Ratio { found: 1, total: 2 }, vec![Missing::Image { slide: 1, item: "m".into() }]));
}

#[test]
fn coverage_finds_a_table_by_its_grid_and_its_cell_runs() {
    let data = json!({ "dataset": { "source": [["Titre", null], ["A", "1"]] } });
    let d = deck(None, vec![element_with("s1-1", "table", "data", data)]);
    let c =
        coverage(&source(vec![source_slide(1, None, vec![table_item()])]), &d, &registry(), &BTreeMap::new(), Some(1));
    assert_that!((c.tables, c.runs)).is_equal_to((Ratio { found: 1, total: 1 }, Ratio { found: 3, total: 3 }));
}

#[test]
fn coverage_misses_a_table_whose_grid_differs() {
    let data = json!({ "dataset": { "source": [["Titre", null], ["A", "2"]] } });
    let d = deck(None, vec![element_with("s1-1", "table", "data", data)]);
    let c =
        coverage(&source(vec![source_slide(1, None, vec![table_item()])]), &d, &registry(), &BTreeMap::new(), Some(1));
    assert_that!(c.missing).contains(Missing::Table { slide: 1, item: "tb".into() });
}

#[test]
fn coverage_finds_a_chart_when_every_series_has_its_column() {
    let option = json!({
        "dataset": { "source": [["", "Nord", "Sud"], ["T1", 1.0, 3.0], ["T2", 2.0, 4.0]] },
        "series": [{ "type": "bar" }, { "type": "bar" }],
    });
    let d = deck(None, vec![element_with("s1-1", "chart", "option", option)]);
    let c = coverage(
        &source(vec![source_slide(1, None, vec![chart_item(ChartKind::Column)])]),
        &d,
        &registry(),
        &BTreeMap::new(),
        Some(1),
    );
    assert_that!(c.charts).is_equal_to(Ratio { found: 1, total: 1 });
}

#[test]
fn coverage_misses_a_pie_that_kept_one_series_of_two() {
    let option =
        json!({ "dataset": { "source": [["", "Nord"], ["T1", 1.0], ["T2", 2.0]] }, "series": [{ "type": "pie" }] });
    let d = deck(None, vec![element_with("s1-1", "chart", "option", option)]);
    let c = coverage(
        &source(vec![source_slide(1, None, vec![chart_item(ChartKind::Pie)])]),
        &d,
        &registry(),
        &BTreeMap::new(),
        Some(1),
    );
    assert_that!((c.charts, c.missing))
        .is_equal_to((Ratio { found: 0, total: 1 }, vec![Missing::Chart { slide: 1, item: "ch".into() }]));
}

fn origins(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs.iter().map(|(i, e)| ((*i).to_owned(), (*e).to_owned())).collect()
}

/// « art » est dans « smart » mais pas comme mot : le run manque ; le run
/// « Wealth » collé à « 's » au milieu d'un mot se trouve dans « Wealth's ».
#[test]
fn coverage_matches_runs_on_word_boundaries() {
    let glued = item(
        "g",
        (0.0, 30.0, 100.0, 20.0),
        ItemKind::Text(text_box(vec![para(vec![
            run("Alto Wealth"),
            run_with("'s", |s| s.bold = true),
            run(" layer"),
        ])])),
    );
    let src = source(vec![source_slide(1, None, vec![text_item("a", (0.0, 0.0, 10.0, 10.0), "art"), glued])]);
    let d = deck(None, vec![text("s1-1", "smart"), text("s1-2", "Alto Wealth's layer")]);
    let c = coverage(&src, &d, &registry(), &BTreeMap::new(), Some(1));
    assert_that!(c.missing).is_equal_to(vec![Missing::Run { slide: 1, text: "art".into() }]);
    assert_that!(c.runs).is_equal_to(Ratio { found: 3, total: 4 });
}

/// « AI » (2 caractères) est dans « MAIN » ; seul un paragraphe « AI » entier le retrouve.
#[test]
fn coverage_needs_the_whole_paragraph_for_a_short_run() {
    let src = source(vec![source_slide(1, None, vec![text_item("a", (0.0, 0.0, 10.0, 10.0), "AI")])]);
    let missed =
        coverage(&src, &deck(None, vec![text("s1-1", "MAIN AI ROAD")]), &registry(), &BTreeMap::new(), Some(1));
    assert_that!(missed.missing).is_equal_to(vec![Missing::Run { slide: 1, text: "AI".into() }]);
    let found = coverage(&src, &deck(None, vec![text("s1-1", "AI")]), &registry(), &BTreeMap::new(), Some(1));
    assert_that!(found.runs).is_equal_to(Ratio { found: 1, total: 1 });
}

/// Projeté dans `s1-1`, le texte « Corps » ne compte pas s'il n'est que dans `s1-2`.
#[test]
fn coverage_looks_for_a_run_in_the_element_projected_from_its_item() {
    let src = source(vec![source_slide(1, None, vec![text_item("b", (0.0, 0.0, 10.0, 10.0), "Corps")])]);
    let d = deck(None, vec![text("s1-1", "Autre chose"), text("s1-2", "Corps")]);
    let c = coverage(&src, &d, &registry(), &origins(&[("b", "s1-1")]), Some(1));
    assert_that!(c.missing).is_equal_to(vec![Missing::Run { slide: 1, text: "Corps".into() }]);
    let c = coverage(&src, &d, &registry(), &origins(&[("b", "s1-2")]), Some(1));
    assert_that!(c.missing).is_empty();
}

#[test]
fn coverage_compares_all_caps_runs_without_case_and_skips_punctuation() {
    let tb = text_box(vec![para(vec![run_with("the shift", |s| s.caps = Caps::All), run(" — ")])]);
    let src = source(vec![source_slide(1, None, vec![item("k", (0.0, 0.0, 10.0, 10.0), ItemKind::Text(tb))])]);
    let c = coverage(&src, &deck(Some("THE SHIFT"), Vec::new()), &registry(), &BTreeMap::new(), Some(1));
    assert_that!((c.runs, c.missing.is_empty())).is_equal_to((Ratio { found: 1, total: 1 }, true));
}

/// Le pied « n · Rapport » répété dans la bande basse de deux slides sur deux est omis par règle.
#[test]
fn coverage_skips_repeated_footers_like_the_projection() {
    let footer = |n: u32| text_item(&format!("s{n}/f"), (64.0, 497.0, 270.0, 12.0), &format!("{n} · Rapport"));
    let src = source(vec![source_slide(1, None, vec![footer(1)]), source_slide(2, None, vec![footer(2)])]);
    let c = coverage(&src, &deck(None, Vec::new()), &registry(), &BTreeMap::new(), Some(1));
    assert_that!(c.runs).is_equal_to(Ratio::default());
}

#[test]
fn coverage_does_not_fall_back_to_the_slide_when_the_projected_element_has_no_text() {
    let src = source(vec![source_slide(1, None, vec![text_item("b", (0.0, 0.0, 10.0, 10.0), "Corps")])]);
    let empty = Element::new("s1-1", "card");
    let d = deck(None, vec![empty, text("s1-2", "Corps")]);
    let c = coverage(&src, &d, &registry(), &origins(&[("b", "s1-1")]), None);
    assert_that!(c.missing).is_equal_to(vec![Missing::Run { slide: 1, text: "Corps".into() }]);
}

#[test]
fn coverage_gives_the_deck_title_only_to_the_slide_that_filled_it() {
    let src = source(vec![
        source_slide(1, None, vec![text_item("a", (0.0, 200.0, 10.0, 10.0), "Rapport annuel")]),
        source_slide(2, None, vec![text_item("b", (0.0, 300.0, 10.0, 10.0), "Rapport annuel")]),
    ]);
    let mut d = deck(Some("Rapport annuel"), Vec::new());
    d.params.title = Some("Rapport annuel".into());
    let c = coverage(&src, &d, &registry(), &BTreeMap::new(), Some(1));
    assert_that!(c.missing).is_equal_to(vec![Missing::Run { slide: 2, text: "Rapport annuel".into() }]);
}

fn two_series_chart(second: [f64; 2], categories: [&str; 2]) -> Item {
    let series = |name: &str, v: [f64; 2]| Series {
        name: name.into(),
        values: v.map(Some).to_vec(),
        color: None,
        x: None,
        format: None,
    };
    let c = Chart {
        kind: ChartKind::Column,
        grouping: Grouping::Clustered,
        title: None,
        categories: categories.map(String::from).to_vec(),
        series: vec![series("Nord", [1.0, 2.0]), series("Sud", second)],
    };
    item("ch", (0.0, 0.0, 200.0, 100.0), ItemKind::Chart(c))
}

fn chart_deck(option: serde_json::Value) -> Deck {
    deck(None, vec![element_with("s1-1", "chart", "option", option)])
}

#[test]
fn coverage_needs_a_distinct_column_per_series() {
    let option =
        json!({ "dataset": { "source": [["", "Nord"], ["T1", 1.0], ["T2", 2.0]] }, "series": [{ "type": "bar" }] });
    let src = source(vec![source_slide(1, None, vec![two_series_chart([1.0, 2.0], ["T1", "T2"])])]);
    let c = coverage(&src, &chart_deck(option), &registry(), &BTreeMap::new(), None);
    assert_that!(c.charts).is_equal_to(Ratio { found: 0, total: 1 });
}

#[test]
fn coverage_misses_a_chart_whose_categories_differ() {
    let option = json!({
        "dataset": { "source": [["", "Nord", "Sud"], ["A", 1.0, 3.0], ["B", 2.0, 4.0]] },
        "series": [{ "type": "bar" }, { "type": "bar" }],
    });
    let src = source(vec![source_slide(1, None, vec![two_series_chart([3.0, 4.0], ["T1", "T2"])])]);
    let c = coverage(&src, &chart_deck(option), &registry(), &BTreeMap::new(), None);
    assert_that!(c.charts).is_equal_to(Ratio { found: 0, total: 1 });
}
