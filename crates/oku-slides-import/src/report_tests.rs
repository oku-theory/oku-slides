use oku_slides_i18n::{Localize, Localizer, english};
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::source::{Warning, WarningKind};

fn slide(losses: Vec<Loss>) -> SlideReport {
    SlideReport {
        source: 2,
        slide: "s2".into(),
        template: "texte".into(),
        placement: Placement::Flow,
        losses,
        fallbacks: vec!["stack".into()],
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

#[test]
fn loss_localize_opaque_in_english_names_the_reason() {
    let msg = Loss::slide(LossKind::Opaque(OpaqueReason::Math)).localize(english());
    assert_that!(msg).is_equal_to("Equation not imported (no fallback image)".to_owned());
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

#[rstest]
#[case(Ratio { found: 0, total: 0 }, true)]
#[case(Ratio { found: 4, total: 4 }, true)]
#[case(Ratio { found: 3, total: 4 }, false)]
fn ratio_is_complete_when_everything_is_found(#[case] r: Ratio, #[case] complete: bool) {
    assert_that!(r.is_complete()).is_equal_to(complete);
}
