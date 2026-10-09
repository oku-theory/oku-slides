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

/// Graphique en barres titré, avec les tailles de texte d'un thème de slides.
fn large_text_bars() -> String {
    register_fonts(&[include_bytes!("../../../assets/fonts/inter-400.ttf")]).unwrap_or_else(|e| panic!("{e}"));
    let theme = ChartTheme {
        font_family: "Inter".into(),
        title_size: 40.0,
        subtitle_size: 32.0,
        legend_size: 32.0,
        axis_size: 32.0,
        label_size: 32.0,
        ..ChartTheme::default()
    };
    let option = parse_option(&json!({
        "title": { "text": "Ventes", "subtext": "2026" },
        "dataset": { "source": [["Mois", "Ventes"], ["Jan", 3], ["Fév", 5]] },
        "xAxis": { "type": "category" }, "series": [{ "type": "bar" }]
    }))
    .unwrap_or_else(|e| panic!("{e}"));
    ChartsRsRenderer::new(theme).render(&option, SIZE).unwrap_or_else(|e| panic!("{e}"))
}

/// Valeur numérique de l'attribut `name` d'une balise SVG.
fn attr(tag: &str, name: &str) -> Option<f32> {
    let key = format!(" {name}=\"");
    let start = tag.find(&key)? + key.len();
    tag[start..start + tag[start..].find('"')?].parse().ok()
}

/// Ordonnée du centre d'un `<text>` centré verticalement.
fn middle(tag: &str) -> Option<f32> {
    tag.contains("dominant-baseline=\"middle\"").then(|| Some(attr(tag, "y")? + attr(tag, "dy").unwrap_or(0.0)))?
}

/// Bornes verticales (haut, bas) de chaque `<text>` d'un SVG charts-rs.
fn text_extents(svg: &str) -> Vec<(f32, f32)> {
    svg.split("<text")
        .skip(1)
        .filter_map(|rest| {
            let tag = &rest[..rest.find('>')?];
            let (size, y) = (attr(tag, "font-size")?, attr(tag, "y")?);
            Some(if let Some(center) = middle(tag) {
                (center - size / 2.0, center + size / 2.0)
            } else {
                // `y` est la ligne de base : les jambages sont ignorés.
                (y - size * 0.75, y)
            })
        })
        .collect()
}

#[test]
fn render_large_theme_texts_stay_inside_the_chart() {
    let extents = text_extents(&large_text_bars());
    assert_that!(extents.len()).is_greater_than(4);
    for (top, bottom) in extents {
        asserting(&format!("texte de {top} à {bottom}")).that(&(top >= 0.0 && bottom <= SIZE.height)).is_true();
    }
}

#[test]
fn render_legend_marker_scales_with_the_legend_text_and_is_centered_on_it() {
    let svg = large_text_bars();
    let legend = svg
        .split("<text")
        .skip(1)
        .find(|t| t.contains("font-size=\"32\"") && t.split('>').nth(1).is_some_and(|c| c.trim().starts_with("Ventes")))
        .unwrap_or_else(|| panic!("texte de légende absent"));
    let center = middle(&legend[..legend.find('>').unwrap_or(0)]).unwrap_or_else(|| panic!("légende non centrée"));
    let marker = svg
        .split("<circle")
        .skip(1)
        .filter_map(|c| Some((attr(c, "cy")?, attr(c, "r")?)))
        .find(|(cy, _)| (cy - center).abs() < 1.0);
    let (_, r) = marker.unwrap_or_else(|| panic!("aucun repère centré sur la légende ({center})"));
    assert_that!(r).is_greater_than_or_equal_to(32.0 * 0.4);
}

fn labelled_bars(numbers: NumberFormat) -> String {
    let option = parse_option(&json!({
        "dataset": { "source": [["Mois", "Ventes"], ["Jan", 12345.5], ["Fév", 0.5]] },
        "xAxis": { "type": "category" },
        "series": [{ "type": "bar", "label": { "show": true } }]
    }))
    .unwrap_or_else(|e| panic!("{e}"));
    let renderer = ChartsRsRenderer { numbers, ..ChartsRsRenderer::default() };
    renderer.render(&option, SIZE).unwrap_or_else(|e| panic!("rendu impossible : {e}"))
}

#[test]
fn render_value_labels_follow_the_deck_language() {
    let fr = labelled_bars(NumberFormat::for_lang(Some("fr")));
    assert_that!(fr.as_str()).contains("12\u{202F}345,5");
    assert_that!(fr.as_str()).contains(">\n0,5\n<");
    let en = labelled_bars(NumberFormat::for_lang(Some("en")));
    assert_that!(en.as_str()).contains("12,345.5");
}

#[test]
fn render_without_language_keeps_plain_numbers() {
    let svg = labelled_bars(NumberFormat::default());
    assert_that!(svg.as_str()).contains("12345.5");
}

#[test]
fn render_text_categories_that_look_like_numbers_stay_as_typed() {
    let option = parse_option(&json!({
        "dataset": { "source": [["Code", "Ventes"], ["75001", 12345], ["1.5", 20]] },
        "xAxis": { "type": "category" },
        "series": [{ "type": "bar", "label": { "show": true } }]
    }))
    .unwrap_or_else(|e| panic!("{e}"));
    let renderer = ChartsRsRenderer { numbers: NumberFormat::for_lang(Some("fr")), ..ChartsRsRenderer::default() };
    let svg = renderer.render(&option, SIZE).unwrap_or_else(|e| panic!("rendu impossible : {e}"));
    assert_that!(svg.as_str()).contains(">\n75001\n<");
    assert_that!(svg.as_str()).contains(">\n1.5\n<");
    assert_that!(svg.as_str()).does_not_contain("75\u{202F}001");
    assert_that!(svg.as_str()).contains("12\u{202F}345");
}

fn render_fr(option: serde_json::Value) -> String {
    let option = parse_option(&option).unwrap_or_else(|e| panic!("{e}"));
    let renderer = ChartsRsRenderer { numbers: NumberFormat::for_lang(Some("fr")), ..ChartsRsRenderer::default() };
    renderer.render(&option, SIZE).unwrap_or_else(|e| panic!("rendu impossible : {e}"))
}

#[test]
fn render_pie_labels_localize_their_value() {
    let svg = render_fr(json!({
        "dataset": { "source": [["Nom", "Ventes"], ["A", 12345.5], ["B", 20000]] },
        "series": [{ "type": "pie", "label": { "show": true } }]
    }));
    // Étiquette « {a}: {d} » de charts-rs : nom et pourcentage.
    assert_that!(svg.as_str()).contains(">\nA: 38,2%\n<");
    assert_that!(svg.as_str()).contains(">\nB: 61,8%\n<");
}

#[test]
fn render_negative_value_labels_use_the_minus_sign() {
    let svg = render_fr(json!({
        "dataset": { "source": [["Mois", "Ventes"], ["Jan", -12345.5], ["Fév", 3]] },
        "xAxis": { "type": "category" },
        "series": [{ "type": "bar", "label": { "show": true } }]
    }));
    assert_that!(svg.as_str()).contains("\u{2212}12\u{202F}345,5");
}

#[test]
fn render_values_equal_to_a_row_id_are_still_localized() {
    let svg = render_fr(json!({
        "dataset": {
            "source": [["Mois", "Ventes"], ["Jan", 12345678]],
            "ids": ["00000000", "12345678"],
            "columns": ["87654321", "11223344"]
        },
        "xAxis": { "type": "category" },
        "series": [{ "type": "bar", "label": { "show": true } }]
    }));
    assert_that!(svg.as_str()).contains("12\u{202F}345\u{202F}678");
}

#[test]
fn render_without_language_uses_the_minus_sign_without_groups() {
    let renderer = ChartsRsRenderer::default();
    let draw = |v: serde_json::Value| {
        let option = parse_option(&v).unwrap_or_else(|e| panic!("{e}"));
        renderer.render(&option, SIZE).unwrap_or_else(|e| panic!("rendu impossible : {e}"))
    };
    let bars = draw(json!({
        "dataset": { "source": [["Mois", "Ventes"], ["Jan", -12345.5], ["Fév", 3]] },
        "xAxis": { "type": "category" },
        "series": [{ "type": "bar", "label": { "show": true } }]
    }));
    assert_that!(bars.as_str()).contains(">\n\u{2212}12345.5\n<");
    assert_that!(bars.as_str()).does_not_contain(">\n-12345.5\n<");
    let pie = draw(json!({
        "dataset": { "source": [["Nom", "Ventes"], ["A", 12345.5], ["B", 20000]] },
        "series": [{ "type": "pie", "label": { "show": true } }]
    }));
    assert_that!(pie.as_str()).contains(">\nA: 38.2%\n<");
}
