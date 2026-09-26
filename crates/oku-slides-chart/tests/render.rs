use oku_slides_chart::*;
use oku_slides_i18n::Localize;
use serde_json::json;
use speculoos::prelude::*;

const SIZE: Size = Size { width: 640.0, height: 400.0 };

fn render(v: serde_json::Value) -> String {
    let option = parse_option(&v).unwrap_or_else(|e| panic!("{e}\n{v}"));
    ChartsRsRenderer::default().render(&option, SIZE).unwrap_or_else(|e| panic!("rendu impossible : {e}"))
}

#[test]
fn render_each_v1_type_produces_svg() {
    let cases = [
        (
            "barres",
            json!({
                "title": { "text": "Ventes 2026" },
                "dataset": { "source": [["Mois", "Ventes", "Objectif"], ["Jan", 10, 11], ["Fév", 12, 11], ["Mar", 15, 12]] },
                "xAxis": { "type": "category" }, "yAxis": { "type": "value" },
                "series": [{ "type": "bar" }, { "type": "line" }]
            }),
        ),
        (
            "lignes",
            json!({
                "dataset": { "source": [["Mois", "Visites"], ["Jan", 100], ["Fév", null], ["Mar", 140]] },
                "xAxis": { "type": "category" },
                "series": [{ "type": "line", "smooth": true, "areaStyle": {} }]
            }),
        ),
        (
            "barres horizontales",
            json!({
                "dataset": { "source": [["Pays", "Parts"], ["France", 40], ["Japon", 25]] },
                "xAxis": { "type": "value" }, "yAxis": { "type": "category" },
                "series": [{ "type": "bar" }]
            }),
        ),
        (
            "donut",
            json!({
                "dataset": { "source": [["Canal", "Part"], ["Web", 60], ["Boutique", 30], ["Salon", 10]] },
                "series": [{ "type": "pie", "radius": ["40%", "70%"] }]
            }),
        ),
        (
            "nuage",
            json!({
                "dataset": { "source": [["Prix", "Ventes"], [10, 200], [20, 150], [30, 90]] },
                "series": [{ "type": "scatter" }]
            }),
        ),
        (
            "radar",
            json!({
                "radar": { "indicator": [{ "name": "Vitesse", "max": 10 }] },
                "dataset": { "source": [["Critère", "Offre A", "Offre B"], ["Vitesse", 7, 5], ["Coût", 4, 8], ["Qualité", 9, 6]] },
                "series": [{ "type": "radar" }]
            }),
        ),
    ];
    // Cargo fournit CARGO_TARGET_TMPDIR à la compilation des tests d'intégration : env! est voulu.
    let out_dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("oku-slides-chart-svg");
    for (name, option) in cases {
        let svg = render(option);
        asserting(&format!("{name} : pas un SVG")).that(&svg.as_str()).starts_with("<svg");
        asserting(&format!("{name} : taille non appliquée")).that(&svg.as_str()).contains("width=\"640\"");
        std::fs::create_dir_all(&out_dir).unwrap();
        std::fs::write(out_dir.join(format!("{name}.svg")), svg).unwrap();
    }
}

#[test]
fn render_title_and_labels_emits_text_not_paths() {
    let svg = render(json!({
        "title": { "text": "Chiffre d'affaires" },
        "xAxis": { "data": ["Janvier", "Février"] },
        "series": [{ "type": "bar", "name": "CA", "data": [1, 2] }]
    }));
    assert_that!(svg.as_str()).contains("Janvier");
    assert_that!(svg.as_str()).contains("<text");
}

#[test]
fn normalize_pie_by_default_is_plain_unlike_charts_rs() {
    let option = parse_option(&json!({
        "dataset": { "source": [["n", "v"], ["A", 1], ["B", 3]] },
        "series": [{ "type": "pie" }]
    }))
    .unwrap();
    let ChartKind::Pie(p) = normalize(&option, ChartLabels::english()).unwrap().kind else { panic!() };
    assert_that!(p.rose).is_false();
    assert_that!(p.inner).is_none();
}

#[test]
fn render_theme_palette_reaches_the_svg() {
    let theme = ChartTheme { palette: vec!["#123456".into()], ..Default::default() };
    let option = parse_option(&json!({
        "xAxis": { "data": ["a"] }, "series": [{ "type": "bar", "data": [1] }]
    }))
    .unwrap();
    let svg = ChartsRsRenderer::new(theme).render(&option, SIZE).unwrap();
    asserting("couleur absente")
        .that(&(svg.to_lowercase().contains("#123456") || svg.contains("rgb(18,52,86")))
        .is_true();
}

#[test]
fn parse_option_inconsistent_options_explain_the_fix() {
    let err = parse_option(&json!({
        "dataset": { "source": [["Mois", "Ventes"], ["Jan", 1]] },
        "series": [{ "type": "bar", "encode": { "y": "CA" } }]
    }))
    .unwrap_err()
    .localize(oku_slides_i18n::english());
    asserting(&err).that(&err.as_str()).contains("CA");
    asserting(&err).that(&err.as_str()).contains("Ventes");

    let err = parse_option(&json!({ "series": [{ "type": "bar", "itemStyle": { "color": "red" } }] }))
        .unwrap_err()
        .localize(oku_slides_i18n::english());
    asserting(&err).that(&err.as_str()).contains("red");
    asserting(&err).that(&err.as_str()).contains("#rrggbb");
}

/// Abscisses distinctes des barres (rectangles pleins) d'un SVG.
fn bar_columns(svg: &str) -> std::collections::BTreeSet<String> {
    svg.split("<rect")
        .skip(1)
        .filter(|r| !r.contains("fill=\"none\"") && !r.contains("fill-opacity=\"0\""))
        .filter_map(|r| r.split(" x=\"").nth(1).and_then(|x| x.split('"').next()).map(str::to_owned))
        .collect()
}

#[test]
fn render_stacked_bars_share_one_column_per_category() {
    let chart = |stack: Option<&str>| {
        json!({
            "legend": { "show": false },
            "dataset": { "source": [["Mois", "Nord", "Sud"], ["Jan", 3, 4], ["Fév", 5, 2], ["Mar", 7, 6]] },
            "xAxis": { "type": "category" },
            "series": [{ "type": "bar", "stack": stack }, { "type": "bar", "stack": stack }]
        })
    };
    let side_by_side = bar_columns(&render(chart(None)));
    let stacked = bar_columns(&render(chart(Some("total"))));
    assert_that!((side_by_side.len(), stacked.len())).is_equal_to((6, 3));
}

#[test]
fn render_waterfall_draws_one_bar_per_step_without_legend() {
    let svg = render(json!({
        "dataset": { "source": [
            ["Poste", "Montant", "Total"],
            ["Début", 100, false], ["Ventes", 30, false], ["Coûts", -20, false], ["Fin", null, true]
        ] },
        "series": [{ "type": "waterfall" }]
    }));
    assert_that!(bar_columns(&svg).len()).is_equal_to(4);
    assert_that!(svg.contains("Montant")).is_false();
}

/// Le SVG contient la couleur, en hexadécimal ou en rgb().
fn has_color(svg: &str, hex: &str) -> bool {
    let h = hex.trim_start_matches('#');
    let byte = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0);
    svg.to_lowercase().contains(&hex.to_lowercase())
        || svg.contains(&format!("rgb({},{},{}", byte(0), byte(2), byte(4)))
}

#[test]
fn render_series_color_overrides_the_palette_for_that_series_only() {
    let theme = ChartTheme { palette: vec!["#123456".into(), "#654321".into()], ..Default::default() };
    let option = parse_option(&json!({
        "xAxis": { "data": ["a"] },
        "series": [{ "type": "bar", "data": [1] }, { "type": "bar", "data": [2], "itemStyle": { "color": "#ABCDEF" } }]
    }))
    .unwrap();
    let svg = ChartsRsRenderer::new(theme).render(&option, SIZE).unwrap();
    assert_that!((has_color(&svg, "#123456"), has_color(&svg, "#ABCDEF"), has_color(&svg, "#654321")))
        .is_equal_to((true, true, false));
}

#[test]
fn render_option_palette_replaces_the_theme_palette() {
    let theme = ChartTheme { palette: vec!["#123456".into()], ..Default::default() };
    let option = parse_option(&json!({
        "color": ["#0A0B0C"],
        "xAxis": { "data": ["a"] }, "series": [{ "type": "bar", "data": [1] }]
    }))
    .unwrap();
    let svg = ChartsRsRenderer::new(theme).render(&option, SIZE).unwrap();
    assert_that!((has_color(&svg, "#0A0B0C"), has_color(&svg, "#123456"))).is_equal_to((true, false));
}

/// Cascade à trois couleurs : total, baisse, hausse (dans cet ordre).
fn render_waterfall_with_total(total: bool) -> String {
    let theme =
        ChartTheme { palette: vec!["#111111".into(), "#222222".into(), "#333333".into()], ..Default::default() };
    let v = json!({
        "dataset": { "source": [["Poste", "Montant", "Total"], ["Début", 100, false], ["Fin", null, total]] },
        "series": [{ "type": "waterfall" }]
    });
    let option = parse_option(&v).unwrap_or_else(|e| panic!("{e}\n{v}"));
    ChartsRsRenderer::new(theme).render(&option, SIZE).unwrap_or_else(|e| panic!("rendu impossible : {e}"))
}

#[test]
fn render_waterfall_draws_the_total_color_only_for_rows_marked_total() {
    assert_that!(has_color(&render_waterfall_with_total(true), "#111111")).is_true();
    assert_that!(has_color(&render_waterfall_with_total(false), "#111111")).is_false();
}
