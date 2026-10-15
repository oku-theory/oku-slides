use oku_slides_i18n::{Localize, Localizer, english};
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::fonts::{FontDecision, FontOrigin};
use crate::source::{Warning, WarningKind};

fn slide(losses: Vec<Loss>) -> SlideReport {
    SlideReport {
        source: 2,
        slide: "s2".into(),
        template: "texte".into(),
        placement: Placement::Flow,
        losses,
        fallbacks: vec!["stack".into()],
        score: None,
    }
}

#[rstest]
#[case(LossKind::ListLevels)]
#[case(LossKind::MergedCells)]
#[case(LossKind::PercentStacked)]
#[case(LossKind::RunStyle(RunEffect::Underline))]
#[case(LossKind::RunStyle(RunEffect::Strike))]
#[case(LossKind::RunStyle(RunEffect::Baseline))]
#[case(LossKind::RunStyle(RunEffect::Caps))]
#[case(LossKind::RunStyle(RunEffect::Highlight))]
#[case(LossKind::RunStyle(RunEffect::Font))]
#[case(LossKind::RunStyle(RunEffect::Tracking))]
#[case(LossKind::RunStyle(RunEffect::Weight))]
#[case(LossKind::RunStyle(RunEffect::Slant))]
#[case(LossKind::Transform)]
#[case(LossKind::ColorDropped("#ff0000".into()))]
#[case(LossKind::TextTruncated)]
#[case(LossKind::TableTruncated)]
#[case(LossKind::Opaque(OpaqueReason::Media))]
#[case(LossKind::Opaque(OpaqueReason::Ole))]
#[case(LossKind::Opaque(OpaqueReason::Math))]
#[case(LossKind::Opaque(OpaqueReason::SmartArt))]
#[case(LossKind::Opaque(OpaqueReason::Unknown("x".into())))]
#[case(LossKind::UnsupportedImage("image/x-emf".into()))]
#[case(LossKind::HiddenSlide)]
#[case(LossKind::Effects)]
#[case(LossKind::RepeatedDecor { text: "CONFIDENTIAL".into(), slides: 7 })]
#[case(LossKind::RepeatedDecor { text: "CONFIDENTIAL".into(), slides: 1 })]
#[case(LossKind::NoSlot)]
#[case(LossKind::SeriesDropped { kept: 1, dropped: 2 })]
#[case(LossKind::PageFitted { width: 720, height: 540 })]
#[case(LossKind::ParagraphStyle)]
#[case(LossKind::ShapeStyle(ShapeEffect::Gradient))]
#[case(LossKind::ShapeStyle(ShapeEffect::Picture))]
#[case(LossKind::ShapeStyle(ShapeEffect::Dash))]
#[case(LossKind::ShapeStyle(ShapeEffect::Geometry))]
#[case(LossKind::ShapeStyle(ShapeEffect::Arrow))]
#[case(LossKind::LayoutDropped { layout: "Titre".into() })]
#[case(LossKind::DecorTruncated { omitted: 12 })]
#[case(LossKind::TableStyle)]
#[case(LossKind::Overflow { element: "s3-2".into() })]
fn loss_localize_in_every_locale_formats_its_own_message(#[case] kind: LossKind) {
    let id = kind.message_id();
    let loss = Loss::slide(kind);
    for tag in oku_slides_i18n::available() {
        let l = Localizer::new(Some(tag));
        let msg = loss.localize(&l);
        assert_that!(l.has(id)).named(tag).is_true();
        assert_that!(msg.is_empty() || msg.contains('{')).named(tag).is_false();
    }
}

#[test]
fn loss_localize_color_dropped_names_the_color() {
    let msg = Loss::on("s1/4", LossKind::ColorDropped("#ff0000".into())).localize(english());
    assert_that!(msg).contains("#ff0000");
}

#[test]
fn loss_localize_run_style_in_french_names_the_effect() {
    let msg = Loss::slide(LossKind::RunStyle(RunEffect::Underline)).localize(&Localizer::new(Some("fr")));
    assert_that!(msg).is_equal_to("Soulignement perdu".to_owned());
}

#[rstest]
#[case(LossKind::RunStyle(RunEffect::Font), "Police d'un passage remplacée par celle de la boîte")]
#[case(LossKind::RunStyle(RunEffect::Tracking), "Espacement des lettres d'un passage remplacé par celui de la boîte")]
#[case(LossKind::RunStyle(RunEffect::Weight), "Passage maigre rendu gras comme la boîte")]
#[case(LossKind::RunStyle(RunEffect::Slant), "Passage droit rendu en italique comme la boîte")]
#[case(LossKind::ShapeStyle(ShapeEffect::Gradient), "Dégradé rendu en couleur unie")]
#[case(LossKind::ShapeStyle(ShapeEffect::Dash), "Trait en tirets rendu continu")]
#[case(LossKind::ShapeStyle(ShapeEffect::Geometry), "Forme inconnue rendue en rectangle")]
#[case(LossKind::ShapeStyle(ShapeEffect::Arrow), "Pointes de flèche du trait non dessinées")]
fn loss_localize_hifi_effects_in_french_name_the_effect(#[case] kind: LossKind, #[case] expected: &str) {
    let msg = Loss::slide(kind).localize(&Localizer::new(Some("fr")));
    assert_that!(msg).is_equal_to(expected.to_owned());
}

#[test]
fn loss_localize_opaque_in_english_names_the_reason() {
    let msg = Loss::slide(LossKind::Opaque(OpaqueReason::Math)).localize(english());
    assert_that!(msg).is_equal_to("Equation not imported (no fallback image)".to_owned());
}

#[test]
fn loss_localize_bounds_of_the_pack_name_the_layout_and_the_count() {
    let fr = Localizer::new(Some("fr"));
    let msg = Loss::slide(LossKind::LayoutDropped { layout: "Titre seul".into() }).localize(&fr);
    assert_that!(msg).contains("« Titre seul »");
    let msg = Loss::on("l3/9", LossKind::DecorTruncated { omitted: 12 }).localize(&fr);
    assert_that!(msg).contains("12 éléments omis");
}

#[test]
fn loss_localize_page_fitted_gives_the_source_size() {
    let msg = Loss::slide(LossKind::PageFitted { width: 720, height: 540 }).localize(english());
    assert_that!(msg).is_equal_to("Source page 720 × 540 pt is not 16:9: content scaled and centered".to_owned());
}

#[test]
fn loss_localize_series_dropped_counts_the_series() {
    let msg = Loss::on("s2/4", LossKind::SeriesDropped { kept: 1, dropped: 2 }).localize(english());
    assert_that!(msg).is_equal_to("A pie chart shows a single series: 1 kept, 2 dropped".to_owned());
}

#[test]
fn loss_localize_repeated_decor_names_the_text_and_counts_the_slides() {
    let msg = Loss::on("s2/18", LossKind::RepeatedDecor { text: "CONFIDENTIAL".into(), slides: 7 }).localize(english());
    assert_that!(msg).is_equal_to(
        "Header or footer “CONFIDENTIAL” repeated on 7 slides left out: the theme draws its own".to_owned(),
    );
}

#[test]
fn loss_localize_repeated_decor_in_french_agrees_with_one_slide() {
    let loss = Loss::on("s2/18", LossKind::RepeatedDecor { text: "Logo".into(), slides: 1 });
    let msg = loss.localize(&Localizer::new(Some("fr")));
    assert_that!(msg)
        .is_equal_to("En-tête ou pied « Logo » répété sur 1 slide omis : le thème dessine les siens".to_owned());
}

#[rstest]
#[case(WarningKind::EffectIgnored("effectLst".into()))]
#[case(WarningKind::Approximated("pattFill".into()))]
#[case(WarningKind::UnreadableItem("s1/3".into()))]
#[case(WarningKind::UnreadablePart("ppt/slides/slide9.xml".into()))]
#[case(WarningKind::MissingPart("ppt/media/image1.png".into()))]
#[case(WarningKind::UnsupportedChart("combo: barChart".into()))]
#[case(WarningKind::UnsupportedImage("image/x-emf".into()))]
#[case(WarningKind::OversizedImage("ppt/media/image1.png".into()))]
#[case(WarningKind::ExternalImage("https://images.example/photo.png".into()))]
#[case(WarningKind::UnsafeSvg("ppt/media/image1.svg".into()))]
#[case(WarningKind::UnknownTableStyle("{5C22544A}".into()))]
#[case(WarningKind::UnsafeLink("javascript:".into()))]
#[case(WarningKind::NestingTooDeep("grpSp".into()))]
#[case(WarningKind::FontEmbeddedUnreadable("font1.fntdata".into()))]
#[case(WarningKind::Truncated)]
fn warning_localize_in_every_locale_formats_its_own_message(#[case] kind: WarningKind) {
    let (id, detail) = warning_message(&kind);
    let warning = Warning { slide: Some(1), item: None, kind: kind.clone() };
    for tag in oku_slides_i18n::available() {
        let l = Localizer::new(Some(tag));
        let msg = warning.localize(&l);
        assert_that!(l.has(id)).named(tag).is_true();
        assert_that!(detail.is_none_or(|d| msg.contains(d))).named(tag).is_true();
        // Aucune variable laissée telle quelle hors du détail (un style de tableau est un GUID entre accolades).
        let bare = detail.map_or(msg.clone(), |d| msg.replace(d, ""));
        assert_that!(bare.contains('{')).named(tag).is_false();
    }
}

#[test]
fn loss_serializes_kind_as_type_and_detail() {
    let v = serde_json::to_value(Loss::on("s1/4", LossKind::ColorDropped("#ff0000".into()))).unwrap();
    assert_that!(v).is_equal_to(json!({ "item": "s1/4", "kind": { "type": "color_dropped", "detail": "#ff0000" } }));
}

#[test]
fn loss_without_detail_serializes_type_only() {
    let v = serde_json::to_value(Loss::slide(LossKind::HiddenSlide)).unwrap();
    assert_that!(v).is_equal_to(json!({ "item": null, "kind": { "type": "hidden_slide" } }));
}

#[test]
fn import_report_to_json_adds_a_message_beside_each_loss() {
    let mut report = ImportReport::new(Mode::Retrofit);
    report.slides.push(slide(vec![Loss::slide(LossKind::HiddenSlide), Loss::on("s2/3", LossKind::MergedCells)]));
    let v = report.to_json(english());
    assert_that!(v["mode"]).is_equal_to(json!("retrofit"));
    assert_that!(v["slides"][0]["placement"]).is_equal_to(json!("flow"));
    assert_that!(v["slides"][0]["losses"][0]["message"]).is_equal_to(json!("Hidden slide imported as visible"));
    assert_that!(v["slides"][0]["losses"][1]["message"]).is_equal_to(json!("Merged cells flattened"));
    assert_that!(v["slides"][0]["losses"][1]["item"]).is_equal_to(json!("s2/3"));
}

#[test]
fn import_report_to_json_adds_a_message_beside_each_warning() {
    let mut report = ImportReport::new(Mode::Retrofit);
    report.warnings.push(Warning {
        slide: Some(2),
        item: Some("s2/4".into()),
        kind: WarningKind::UnsupportedChart("combo: barChart".into()),
    });
    let v = report.to_json(&Localizer::new(Some("fr")));
    assert_that!(v["warnings"][0]["message"]).is_equal_to(json!("Graphique non pris en charge : combo: barChart"));
    assert_that!(v["warnings"][0]["kind"])
        .is_equal_to(json!({ "type": "unsupported_chart", "detail": "combo: barChart" }));
}

#[test]
fn import_report_to_json_writes_coverage_when_measured() {
    let mut report = ImportReport::new(Mode::Retrofit);
    report.coverage = Some(Coverage {
        runs: Ratio { found: 3, total: 4 },
        missing: vec![Missing::Run { slide: 1, text: "Perdu".into() }],
        ..Coverage::default()
    });
    let v = report.to_json(english());
    assert_that!(v["coverage"]["runs"]).is_equal_to(json!({ "found": 3, "total": 4 }));
    assert_that!(v["coverage"]["missing"][0]).is_equal_to(json!({ "type": "run", "slide": 1, "text": "Perdu" }));
}

#[test]
fn import_report_loss_count_sums_every_slide() {
    let mut report = ImportReport::new(Mode::Retrofit);
    report.slides.push(slide(vec![Loss::slide(LossKind::HiddenSlide)]));
    report.slides.push(slide(vec![Loss::slide(LossKind::Effects), Loss::slide(LossKind::Transform)]));
    assert_that!(report.loss_count()).is_equal_to(3);
}

/// Le choix pour Calibri ; une famille introuvable prend Carlito, la police du corps.
fn font(used: Option<&str>, decision: FontDecision, restricted: bool) -> FontChoice {
    let fallback = (decision == FontDecision::Missing).then(|| "Carlito".to_owned());
    FontChoice {
        family: "Calibri".into(),
        used: used.map(str::to_owned),
        decision,
        restricted,
        too_large: false,
        fallback,
    }
}

#[rstest]
#[case(font(Some("Carlito"), FontDecision::Substituted { origin: FontOrigin::App }, false), "import-font-substituted")]
#[case(font(Some("Carlito"), FontDecision::Substituted { origin: FontOrigin::App }, true), "import-font-restricted-substituted")]
#[case(font(None, FontDecision::Missing, false), "import-font-missing")]
#[case(font(None, FontDecision::Missing, true), "import-font-restricted-missing")]
#[case(font(Some("Calibri"), FontDecision::Embedded, true), "import-font-restricted")]
#[case(FontChoice { too_large: true, ..font(None, FontDecision::Missing, false) }, "import-font-too-large")]
#[case(FontChoice { fallback: None, ..font(None, FontDecision::Missing, true) }, "import-font-missing-no-fallback")]
fn font_notice_in_every_locale_names_the_families(#[case] choice: FontChoice, #[case] id: &str) {
    for tag in oku_slides_i18n::available() {
        let l = Localizer::new(Some(tag));
        let msg = font_notice(&choice, &l).unwrap_or_default();
        assert_that!(l.has(id)).named(tag).is_true();
        assert_that!(msg.contains("Calibri")).named(tag).is_true();
        assert_that!(msg.contains('{')).named(tag).is_false();
        if choice.decision != FontDecision::Missing {
            assert_that!(msg.contains("Carlito") || id == "import-font-restricted").named(tag).is_true();
        }
        // Une famille introuvable nomme le repli retenu, quand il y en a un.
        if let Some(fallback) = &choice.fallback {
            assert_that!(msg.contains(fallback.as_str())).named(tag).is_true();
        }
    }
}

#[rstest]
#[case(FontChoice { too_large: true, ..font(None, FontDecision::Missing, false) }, "Calibri : fichiers trop gros pour être embarqués dans le deck ; le texte prend Carlito")]
#[case(FontChoice { fallback: None, ..font(None, FontDecision::Missing, false) }, "Calibri introuvable, comme la police du corps du thème : le texte est rendu avec la police par défaut du moteur")]
fn font_notice_of_a_missing_family_says_what_the_text_takes(#[case] choice: FontChoice, #[case] expected: &str) {
    assert_that!(font_notice(&choice, &Localizer::new(Some("fr")))).is_equal_to(Some(expected.to_owned()));
}

#[test]
fn font_notice_of_the_missing_body_font_names_the_app_fallback() {
    // Calibri est la police du corps : le repli vient de `FALLBACKS`, le message le nomme.
    let choice = FontChoice { fallback: Some("Liberation Sans".into()), ..font(None, FontDecision::Missing, false) };
    assert_that!(font_notice(&choice, &Localizer::new(Some("fr"))))
        .is_equal_to(Some("Calibri introuvable : le texte prend Liberation Sans".to_owned()));
}

#[test]
fn font_notice_of_a_missing_family_other_than_the_body_names_the_body_font() {
    // Aptos introuvable, Calibri (corps) disponible en Carlito : le repli est Carlito.
    let choice = FontChoice { family: "Aptos".into(), ..font(None, FontDecision::Missing, false) };
    assert_that!(font_notice(&choice, &Localizer::new(Some("fr"))))
        .is_equal_to(Some("Aptos introuvable : le texte prend Carlito".to_owned()));
}

#[test]
fn font_notice_of_a_too_large_body_font_names_the_app_fallback() {
    let choice = FontChoice {
        too_large: true,
        fallback: Some("Liberation Sans".into()),
        ..font(None, FontDecision::Missing, false)
    };
    assert_that!(font_notice(&choice, &Localizer::new(Some("fr")))).is_equal_to(Some(
        "Calibri : fichiers trop gros pour être embarqués dans le deck ; le texte prend Liberation Sans".to_owned(),
    ));
}

#[test]
fn font_notice_of_a_restricted_missing_family_names_the_fallback() {
    let choice = FontChoice { fallback: Some("Liberation Sans".into()), ..font(None, FontDecision::Missing, true) };
    assert_that!(font_notice(&choice, &Localizer::new(Some("fr")))).is_equal_to(Some(
        "Calibri ne peut pas être embarquée dans le deck (licence) : le texte prend Liberation Sans".to_owned(),
    ));
}

#[rstest]
#[case(font(Some("Calibri"), FontDecision::Embedded, false))]
#[case(font(Some("Calibri"), FontDecision::Available { origin: FontOrigin::System }, false))]
fn font_notice_of_a_font_kept_as_is_is_none(#[case] choice: FontChoice) {
    assert_that!(font_notice(&choice, english())).is_none();
}

#[test]
fn import_report_to_json_lists_fonts_with_a_message_when_there_is_one() {
    let mut report = ImportReport::new(Mode::Retrofit);
    report.fonts.push(font(Some("Carlito"), FontDecision::Substituted { origin: FontOrigin::App }, false));
    report.fonts.push(FontChoice { family: "Inter".into(), ..font(Some("Inter"), FontDecision::Embedded, false) });
    let v = report.to_json(&Localizer::new(Some("fr")));
    assert_that!(v["fonts"]).is_equal_to(json!([
        {
            "family": "Calibri",
            "used": "Carlito",
            "decision": { "type": "substituted", "origin": "app" },
            "restricted": false,
            "message": "Calibri remplacée par Carlito, aux métriques compatibles"
        },
        { "family": "Inter", "used": "Inter", "decision": { "type": "embedded" }, "restricted": false }
    ]));
}

#[test]
fn import_report_to_json_without_fonts_writes_no_fonts_section() {
    let v = ImportReport::new(Mode::Retrofit).to_json(english());
    assert_that!(v.get("fonts")).is_none();
}

#[rstest]
#[case(Ratio { found: 0, total: 0 }, true)]
#[case(Ratio { found: 4, total: 4 }, true)]
#[case(Ratio { found: 3, total: 4 }, false)]
fn ratio_is_complete_when_everything_is_found(#[case] r: Ratio, #[case] complete: bool) {
    assert_that!(r.is_complete()).is_equal_to(complete);
}

#[test]
fn loss_localize_overflow_names_the_element() {
    let msg = Loss::slide(LossKind::Overflow { element: "s3-2".into() }).localize(english());
    assert_that!(msg).contains("s3-2");
}

fn scored(n: u32, score: f64) -> SlideReport {
    let regions = vec![crate::metrics::RegionScore { item: format!("s{n}/1"), score, shift: [0, 0] }];
    SlideReport { source: n, slide: format!("s{n}"), score: Some(SlideScore { score, regions }), ..slide(vec![]) }
}

fn with_reference(slides: Vec<SlideReport>, unavailable: Option<ReferenceCause>) -> ImportReport {
    let reference = ReferenceReport { witness: "office2pdf 0.8.1".into(), threshold: 5.0, unavailable };
    ImportReport { slides, reference: Some(reference), ..ImportReport::new(Mode::Hifi) }
}

#[test]
fn to_json_marks_the_slides_above_the_threshold() {
    let v = with_reference(vec![scored(1, 2.0), scored(2, 7.5)], None).to_json(english());
    let marks: Vec<Option<bool>> = (0..2).map(|i| v["slides"][i]["above_threshold"].as_bool()).collect();
    assert_that!(marks).is_equal_to(vec![None, Some(true)]);
}

#[test]
fn to_json_writes_the_score_and_its_regions() {
    let v = with_reference(vec![scored(1, 2.0)], None).to_json(english());
    assert_that!(v["slides"][0]["score"].clone())
        .is_equal_to(json!({"score": 2.0, "regions": [{"item": "s1/1", "score": 2.0, "shift": [0, 0]}]}));
}

#[test]
fn to_json_without_score_writes_no_score() {
    let v = with_reference(vec![slide(vec![])], None).to_json(english());
    assert_that!(v["slides"][0].get("score")).is_none();
}

#[test]
fn to_json_explains_a_missing_reference() {
    let expected = ReferenceCause::Timeout.localize(&Localizer::new(Some("fr")));
    let v = with_reference(vec![], Some(ReferenceCause::Timeout)).to_json(&Localizer::new(Some("fr")));
    assert_that!(v["reference"]["message"].as_str()).is_equal_to(Some(expected.as_str()));
}

#[test]
fn to_json_without_reference_writes_none() {
    let v = ImportReport::new(Mode::Retrofit).to_json(english());
    assert_that!(v.get("reference")).is_none();
}

#[test]
fn above_threshold_follows_the_worst_region_not_the_mean() {
    let regions = vec![crate::metrics::RegionScore { item: "s1/1".into(), score: 9.0, shift: [0, 0] }];
    let slide = SlideReport { score: Some(SlideScore { score: 1.0, regions }), ..scored(1, 1.0) };
    let r = with_reference(vec![slide], None);
    assert_that!(r.above_threshold().count()).is_equal_to(1);
}

#[test]
fn above_threshold_lists_the_slides_beyond_it() {
    let r = with_reference(vec![scored(1, 2.0), scored(2, 7.5), scored(3, 5.0)], None);
    let above: Vec<u32> = r.above_threshold().map(|s| s.source).collect();
    assert_that!(above).is_equal_to(vec![2]);
}

#[test]
fn mean_score_averages_the_scored_slides() {
    let r = with_reference(vec![scored(1, 2.0), slide(vec![]), scored(3, 5.0)], None);
    assert_that!(r.mean_score()).is_equal_to(Some(3.5));
}
