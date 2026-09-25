use serde_json::json;
use slider_chart::*;
use slider_i18n::Localize;

const SIZE: Size = Size { width: 640.0, height: 400.0 };

fn render(v: serde_json::Value) -> String {
    let option = parse_option(&v).unwrap_or_else(|e| panic!("{e}\n{v}"));
    ChartsRsRenderer::default().render(&option, SIZE).unwrap_or_else(|e| panic!("rendu impossible : {e}"))
}

#[test]
fn each_v1_type_renders_to_svg() {
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
    for (name, option) in cases {
        let svg = render(option);
        assert!(svg.starts_with("<svg"), "{name} : pas un SVG");
        assert!(svg.contains("width=\"640\""), "{name} : taille non appliquée");
        std::fs::create_dir_all("target/slider-chart-svg").unwrap();
        std::fs::write(format!("target/slider-chart-svg/{name}.svg"), svg).unwrap();
    }
}

#[test]
fn title_and_labels_are_text_not_paths() {
    let svg = render(json!({
        "title": { "text": "Chiffre d'affaires" },
        "xAxis": { "data": ["Janvier", "Février"] },
        "series": [{ "type": "bar", "name": "CA", "data": [1, 2] }]
    }));
    assert!(svg.contains("Janvier"));
    assert!(svg.contains("<text"));
}

#[test]
fn pie_is_plain_by_default_unlike_charts_rs() {
    let option = parse_option(&json!({
        "dataset": { "source": [["n", "v"], ["A", 1], ["B", 3]] },
        "series": [{ "type": "pie" }]
    }))
    .unwrap();
    let ChartKind::Pie(p) = normalize(&option).unwrap().kind else { panic!() };
    assert!(!p.rose && p.inner.is_none());
}

#[test]
fn theme_palette_reaches_the_svg() {
    let theme = ChartTheme { palette: vec!["#123456".into()], ..Default::default() };
    let option = parse_option(&json!({
        "xAxis": { "data": ["a"] }, "series": [{ "type": "bar", "data": [1] }]
    }))
    .unwrap();
    let svg = ChartsRsRenderer::new(theme).render(&option, SIZE).unwrap();
    assert!(svg.to_lowercase().contains("#123456") || svg.contains("rgb(18,52,86"), "couleur absente");
}

#[test]
fn inconsistent_options_explain_the_fix() {
    let err = parse_option(&json!({
        "dataset": { "source": [["Mois", "Ventes"], ["Jan", 1]] },
        "series": [{ "type": "bar", "encode": { "y": "CA" } }]
    }))
    .unwrap_err()
    .localize(slider_i18n::english());
    assert!(err.contains("CA") && err.contains("Ventes"), "{err}");

    let err = parse_option(&json!({ "series": [{ "type": "bar", "itemStyle": { "color": "red" } }] }))
        .unwrap_err()
        .localize(slider_i18n::english());
    assert!(err.contains("itemStyle"), "{err}");
}
