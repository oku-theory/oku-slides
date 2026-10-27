use oku_slides_core::PackKind;
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use oku_slides_i18n::Localizer;

use super::{HOST_PACKS, PacksEvent, deck_packs_panel, packs_event};
use crate::deck_packs::deck_packs_tests::{component, component_ids, opened_on_newer, text_of};
use crate::lib_tests_support::session_with_fixture;
use crate::test_support::{ns, session};

#[rstest]
#[case("add_pack", json!({"kind": "widgets", "namespace": "acme"}), Some(PacksEvent::AddPack { kind: PackKind::Widgets, namespace: ns("acme") }))]
#[case("remove_pack", json!({"kind": "layouts", "namespace": "acme"}), Some(PacksEvent::RemovePack { kind: PackKind::Layouts, namespace: ns("acme") }))]
#[case("use_mine", json!({"kind": "widgets", "namespace": "acme"}), Some(PacksEvent::UseMine { kind: PackKind::Widgets, namespace: ns("acme") }))]
#[case("make_current", json!({"kind": "widgets", "namespace": "acme"}), Some(PacksEvent::MakeCurrent { kind: PackKind::Widgets, namespace: ns("acme") }))]
#[case("theme", json!({"theme": "pro"}), Some(PacksEvent::Theme { theme: ns("pro") }))]
#[case("theme_update", json!({}), Some(PacksEvent::ThemeUpdate))]
#[case("open_theme", json!({"theme": "00000000-0000-0000-0000-000000000000"}), None)]
#[case("add_pack", json!({"kind": "themes", "namespace": "acme"}), None)]
#[case("add_pack", json!({"kind": "widgets"}), None)]
#[case("add_pack", json!({"kind": "widgets", "namespace": "Not Valid"}), None)]
#[case("theme", json!({}), None)]
#[case("other", json!({}), None)]
fn packs_event_parses_or_rejects(#[case] name: &str, #[case] ctx: Value, #[case] expected: Option<PacksEvent>) {
    assert_that!(packs_event(name, ctx.as_object().unwrap())).is_equal_to(expected);
}

#[test]
fn deck_packs_panel_refused_pack_has_caption_not_button() {
    let view = session_with_fixture().deck_packs().unwrap();
    let ids = component_ids(&deck_packs_panel(&view, &Localizer::new(Some("en"))));
    assert_that!(ids.contains(&"widgets/base/remove".to_owned())).is_false();
    assert_that!(ids.contains(&"widgets/base/refusal".to_owned())).is_true();
}

#[test]
fn deck_packs_panel_refusal_is_the_op_error_text() {
    let s = session_with_fixture();
    let view = s.deck_packs().unwrap();
    let l = Localizer::new(Some("en"));
    let refusal = view.widgets.iter().find(|p| p.namespace == ns("base")).and_then(|p| p.remove_refusal.as_ref());
    let expected = refusal.map(|e| oku_slides_i18n::Localize::localize(e, &l)).unwrap();
    assert_that!(text_of(&deck_packs_panel(&view, &l), "widgets/base/refusal")).is_equal_to(expected);
}

#[test]
fn deck_packs_panel_is_a_live_thread_surface() {
    let view = session().deck_packs().unwrap();
    let surface = deck_packs_panel(&view, &Localizer::new(Some("en")));
    assert_that!((surface.id.as_str(), surface.live)).is_equal_to((HOST_PACKS, true));
    assert_that!(serde_json::to_value(&surface).unwrap()["place"].as_str()).is_equal_to(Some("thread"));
}

#[test]
fn deck_packs_panel_sections_come_in_order() {
    let view = session().deck_packs().unwrap();
    let root = component(&deck_packs_panel(&view, &Localizer::new(Some("en"))), "root");
    let children: Vec<&str> = root["children"].as_array().unwrap().iter().filter_map(Value::as_str).collect();
    assert_that!(children).is_equal_to(vec!["title", "layouts", "widgets", "theme", "close"]);
}

#[test]
fn deck_packs_panel_card_shows_name_short_version_and_namespace() {
    let s = session();
    let view = s.deck_packs().unwrap();
    let surface = deck_packs_panel(&view, &Localizer::new(Some("en")));
    let base = view.widgets.iter().find(|p| p.namespace == ns("base")).unwrap();
    assert_that!(text_of(&surface, "widgets/base/name")).is_equal_to(base.name.clone());
    assert_that!(text_of(&surface, "widgets/base/version")).is_equal_to(base.version[..7].to_owned());
    assert_that!(text_of(&surface, "widgets/base/ns")).is_equal_to("base".to_owned());
}

#[test]
fn deck_packs_panel_current_pack_has_no_version_buttons() {
    let view = session().deck_packs().unwrap();
    let ids = component_ids(&deck_packs_panel(&view, &Localizer::new(Some("en"))));
    for id in ["widgets/base/relation", "widgets/base/use-mine", "widgets/base/make-current"] {
        assert_that!(ids.contains(&id.to_owned())).named(id).is_false();
    }
}

#[test]
fn deck_packs_panel_newer_deck_pack_offers_both_version_buttons() {
    let (_me, _them, s, _theirs, _mine) = opened_on_newer();
    let view = s.deck_packs().unwrap();
    let pack = view.widgets.iter().find(|p| p.drift.is_some()).unwrap();
    let surface = deck_packs_panel(&view, &Localizer::new(Some("fr")));
    let prefix = format!("widgets/{}", pack.namespace);
    let event = |id: &str| component(&surface, &format!("{prefix}/{id}"))["action"]["event"]["name"].clone();
    assert_that!((event("use-mine"), event("make-current"))).is_equal_to((json!("use_mine"), json!("make_current")));
    let context = component(&surface, &format!("{prefix}/use-mine"))["action"]["event"]["context"].clone();
    assert_that!(context).is_equal_to(json!({"kind": "widgets", "namespace": pack.namespace.as_str()}));
    let relation = Localizer::new(Some("fr")).tr("packs-relation-newer");
    assert_that!(text_of(&surface, &format!("{prefix}/relation"))).is_equal_to(relation);
}

#[test]
fn deck_packs_panel_add_select_is_absent_without_addable_packs() {
    let view = session().deck_packs().unwrap();
    let ids = component_ids(&deck_packs_panel(&view, &Localizer::new(Some("en"))));
    assert_that!(ids.contains(&"widgets-add".to_owned())).is_false();
    assert_that!(ids.contains(&"theme-pick".to_owned())).is_true();
}

#[test]
fn deck_packs_panel_texts_follow_the_localizer() {
    let view = session().deck_packs().unwrap();
    let (fr, en) = (Localizer::new(Some("fr")), Localizer::new(Some("en")));
    let title = |l: &Localizer| text_of(&deck_packs_panel(&view, l), "title");
    assert_that!(title(&fr)).is_not_equal_to(title(&en));
}

#[test]
fn packs_action_theme_picks_the_theme() {
    let mut s = session();
    s.packs_action(PacksEvent::Theme { theme: ns("pro") }).unwrap();
    assert_that!(s.deck().theme.clone()).is_equal_to(Some(ns("pro")));
}

#[test]
fn packs_action_remove_routes_to_remove_deck_pack() {
    let mut s = session_with_fixture();
    let err = s.packs_action(PacksEvent::RemovePack { kind: PackKind::Widgets, namespace: ns("base") }).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::Op);
}

#[test]
fn deck_packs_panel_ids_stay_unique_for_namespaces_like_fixed_ids() {
    let mut view = session().deck_packs().unwrap();
    let card = |namespace: &str| super::PinnedPackView {
        kind: PackKind::Widgets,
        namespace: ns(namespace),
        name: namespace.to_owned(),
        version: "0123456789abcdef".to_owned(),
        drift: Some(super::Drift::Newer),
        builtin: false,
        deck_only: false,
        library_has_pinned: true,
        remove_refusal: None,
    };
    view.widgets.extend(["title", "add", "acme", "acme-body"].map(card));
    view.addable.insert(PackKind::Widgets, vec![(ns("other"), "Other".to_owned())]);
    let ids = component_ids(&deck_packs_panel(&view, &Localizer::new(Some("en"))));
    let unique: std::collections::BTreeSet<&String> = ids.iter().collect();
    assert_that!(unique.len()).is_equal_to(ids.len());
}

#[test]
fn theme_row_has_an_open_button_carrying_the_theme_id() {
    let s = session();
    let view = s.deck_packs().unwrap();
    let id = s.deck_theme_pin().map(|pin| pin.id).unwrap();
    let surface = deck_packs_panel(&view, &Localizer::new(Some("fr")));
    let open = component(&surface, "theme-open");
    let event = &open["action"]["event"];
    assert_that!(event["name"].as_str()).is_equal_to(Some("open_theme"));
    assert_that!(event["context"].clone()).is_equal_to(json!({"theme": id.to_string()}));
    assert_that!(open["label"].as_str()).is_equal_to(Some("Ouvrir"));
}

#[test]
fn theme_row_without_a_theme_pin_has_no_open_button() {
    let mut view = session().deck_packs().unwrap();
    view.theme.id = None;
    let ids = component_ids(&deck_packs_panel(&view, &Localizer::new(Some("en"))));
    assert_that!(ids.contains(&"theme-open".to_owned())).is_false();
}

#[test]
fn theme_row_places_open_after_the_theme_pick() {
    let view = session().deck_packs().unwrap();
    let theme = component(&deck_packs_panel(&view, &Localizer::new(Some("en"))), "theme");
    let children: Vec<&str> = theme["children"].as_array().unwrap().iter().filter_map(Value::as_str).collect();
    let at = |id: &str| children.iter().position(|c| *c == id);
    assert_that!(at("theme-open")).is_equal_to(at("theme-pick").map(|i| i + 1));
}
