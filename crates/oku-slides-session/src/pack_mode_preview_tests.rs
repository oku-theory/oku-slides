use oku_slides_core::{Element, Op, ParamValue, SlotRef, Target, kinds};
use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use crate::pack::{BASE_PACK, ESSENTIEL_PACK};
use crate::test_support::fresh_dir;
use crate::{Session, SessionErrorKind};

const SWATCH: &str = r#"#import "/oku-slides/lib.typ": widget, color-of, params as p
#let widget = widget("pastille", params: (body: p.plain()),
  (args, ctx) => rect(width: 40pt, height: 40pt, fill: color-of(ctx.tokens, "accent")))
"#;

/// A pack tab of `base`, and its data folder.
fn editing_base() -> (tempfile::TempDir, Session) {
    let data = fresh_dir("oku-slides-preview-slides-");
    let s = crate::test_support_packs::pack_tab_of(data.path(), BASE_PACK);
    (data, s)
}

fn slide_ids(s: &Session) -> Vec<String> {
    s.deck().slides.iter().map(|sl| sl.id.clone()).collect()
}

#[test]
fn widget_pack_working_deck_has_one_preview_slide_per_widget() {
    let (_data, s) = editing_base();
    let want =
        ["widget-card", "widget-chart", "widget-image", "widget-shape", "widget-stack", "widget-table", "widget-text"];
    assert_that!(slide_ids(&s)).is_equal_to(want.map(String::from).to_vec());
}

#[test]
fn preview_slide_is_a_content_slide_holding_one_block_of_its_widget() {
    let (_data, s) = editing_base();
    let slide = s.deck().slide("widget-card").unwrap();
    let blocks = match slide.params.get("blocks") {
        Some(ParamValue::Slot(blocks)) => blocks.clone(),
        other => panic!("blocs attendus : {other:?}"),
    };
    assert_that!(slide.kind.as_str()).is_equal_to(kinds::essentiel::CONTENT);
    assert_that!(blocks.iter().map(|e| e.kind.as_str()).collect::<Vec<_>>()).is_equal_to(vec![kinds::base::CARD]);
}

#[test]
fn widget_pack_working_deck_renders_its_preview_slides() {
    let (_data, s) = editing_base();
    assert_that!(s.compiled.as_ref().map(oku_slides_render_worker::RemoteCompiled::page_count)).is_equal_to(Some(7));
}

#[test]
fn layout_pack_working_deck_has_no_preview_slide() {
    let data = crate::test_support::fresh_dir("oku-slides-pack-tab-");
    let s = crate::test_support_packs::pack_tab_of(data.path(), ESSENTIEL_PACK);
    assert_that!(slide_ids(&s).iter().any(|id| id.starts_with("widget-"))).is_false();
}

fn card(element: Option<&str>) -> Target {
    Target { slide: "widget-card".into(), element: element.map(str::to_owned) }
}

#[rstest]
#[case::set_param(Op::SetParam { target: card(Some("widget-card-block")), param: "title".into(), value: json!("x") })]
#[case::remove_slide(Op::RemoveSlide { slide: "widget-card".into() })]
#[case::notes(Op::SetNotes { slide: "widget-card".into(), notes: "x".into() })]
#[case::remove_element(Op::RemoveElement { slide: "widget-card".into(), element: "widget-card-block".into() })]
#[case::add_element(Op::AddElement {
    slot: SlotRef { owner: card(None), param: "blocks".into() },
    index: None,
    defaults: false,
    element: Element::new("neuf", kinds::base::TEXT),
})]
fn op_on_a_preview_slide_is_refused(#[case] op: Op) {
    let (_data, mut s) = editing_base();
    let before = s.deck().clone();
    let err = s.apply("x", vec![op], None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::PackModePreview);
    assert_that!(s.deck().clone()).is_equal_to(before);
}

#[test]
fn preview_refusal_is_localized_in_french() {
    let (_data, mut s) = editing_base();
    let err = s.apply("x", vec![Op::RemoveSlide { slide: "widget-card".into() }], None).unwrap_err();
    assert_that!(err.localize(&Localizer::new(Some("fr"))).as_str())
        .is_equal_to("En mode pack, les slides d'aperçu des widgets sont en lecture seule");
}

#[test]
fn writing_a_widget_file_adds_its_preview_slide() {
    let data = fresh_dir("oku-slides-preview-slides-");
    let mut s = crate::test_support_packs::pack_tab_of(data.path(), BASE_PACK);
    s.set_widget(None, "pastille", SWATCH.into()).unwrap();
    assert_that!(slide_ids(&s)).contains("widget-pastille".to_owned());
}

#[test]
fn removing_a_widget_file_removes_its_preview_slide() {
    let data = fresh_dir("oku-slides-preview-slides-");
    let mut s = crate::test_support_packs::pack_tab_of(data.path(), BASE_PACK);
    s.set_widget(None, "pastille", SWATCH.into()).unwrap();
    s.remove_widget(None, "pastille").unwrap();
    assert_that!(slide_ids(&s)).does_not_contain("widget-pastille".to_owned());
}

#[test]
fn undoing_a_widget_write_takes_its_preview_slide_back() {
    let data = fresh_dir("oku-slides-preview-slides-");
    let mut s = crate::test_support_packs::pack_tab_of(data.path(), BASE_PACK);
    s.set_widget(None, "pastille", SWATCH.into()).unwrap();
    s.undo().unwrap();
    assert_that!(slide_ids(&s)).does_not_contain("widget-pastille".to_owned());
}
