use oku_slides_core::PackVersion;
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;
use uuid::Uuid;

use super::{WorkspaceOutcome, WorkspaceRequest};
use crate::deck_offer::DeckOffer;
use crate::workspace::packs::OpenerOffer;

#[rstest]
#[case::applied(WorkspaceOutcome::Applied("{}".into()), None)]
#[case::tune(WorkspaceOutcome::Tune { json: "{}".into(), tab: "t-2".into() }, None)]
#[case::deck_changed(WorkspaceOutcome::DeckChanged { json: "{}".into(), tab: "t-3".into() }, Some("t-3"))]
fn after_change_tab_names_the_changed_tab(#[case] outcome: WorkspaceOutcome, #[case] tab: Option<&str>) {
    assert_that!(outcome.after_change_tab()).is_equal_to(tab);
}

#[test]
fn tune_tab_still_names_an_import() {
    let outcome = WorkspaceOutcome::Tune { json: "{}".into(), tab: "t-2".into() };
    assert_that!(outcome.tune_tab()).is_equal_to(Some("t-2"));
}

#[test]
fn open_pack_request_parses_the_ui_json() {
    let pack = Uuid::new_v4();
    let req: WorkspaceRequest =
        serde_json::from_value(json!({"cmd": "open_pack", "tab": "t-1", "pack": pack, "widget": null})).unwrap();
    let parsed = matches!(req, WorkspaceRequest::OpenPack { tab, pack: p, widget: None } if tab == "t-1" && p == pack);
    assert_that!(parsed).is_true();
}

#[test]
fn pack_opener_answer_is_a_bare_object_or_null() {
    let pack = Uuid::new_v4();
    let version = PackVersion::from_hex(&"a".repeat(64)).unwrap();
    let deck = DeckOffer { title: "Deck".into(), deck_uid: "d-1".into() };
    let offer = OpenerOffer { tab: "t-1".into(), deck, pack, pack_name: "acme".into(), version: version.clone() };
    let expected = json!({
        "tab": "t-1", "title": "Deck", "deck_uid": "d-1", "pack": pack, "pack_name": "acme", "version": version,
    });
    let none: Option<OpenerOffer> = None;
    assert_that!((serde_json::to_value(Some(offer)).unwrap(), serde_json::to_value(none).unwrap()))
        .is_equal_to((expected, serde_json::Value::Null));
}
