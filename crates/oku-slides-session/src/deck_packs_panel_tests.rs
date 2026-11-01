use oku_slides_core::{GuideKind, Guides, PackKind};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use oku_slides_i18n::Localizer;

use super::{HOST_PACKS, PacksEvent, deck_packs_panel, packs_event};
use crate::deck_packs::deck_packs_tests::{component, component_ids, opened_on_newer, received_file, text_of};
use crate::guide_approvals::VersionRef;
use crate::lib_tests_support::session_with_fixture;
use crate::test_support::{files_of, fresh_dir, manifest, session_on};
use crate::test_support::{ns, session};
use crate::{Request, Session};

#[rstest]
#[case("add_pack", json!({"kind": "widgets", "namespace": "acme"}), Some(PacksEvent::AddPack { kind: PackKind::Widgets, namespace: ns("acme") }))]
#[case("remove_pack", json!({"kind": "layouts", "namespace": "acme"}), Some(PacksEvent::RemovePack { kind: PackKind::Layouts, namespace: ns("acme") }))]
#[case("use_mine", json!({"kind": "widgets", "namespace": "acme"}), Some(PacksEvent::UseMine { kind: PackKind::Widgets, namespace: ns("acme") }))]
#[case("make_current", json!({"kind": "widgets", "namespace": "acme"}), Some(PacksEvent::MakeCurrent { kind: PackKind::Widgets, namespace: ns("acme") }))]
#[case("theme", json!({"theme": "pro"}), Some(PacksEvent::Theme { theme: ns("pro") }))]
#[case("theme_update", json!({}), Some(PacksEvent::ThemeUpdate))]
#[case("open_theme", json!({"theme": "00000000-0000-0000-0000-000000000000"}), None)]
#[case("open_pack", json!({"pack": "00000000-0000-0000-0000-000000000000"}), None)]
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

#[rstest]
#[case("en")]
#[case("fr")]
fn deck_packs_panel_drifted_pack_open_names_my_version(#[case] lang: &str) {
    let (_me, _them, s, _theirs, _mine) = opened_on_newer();
    let view = s.deck_packs().unwrap();
    let pack = view.widgets.iter().find(|p| p.drift.is_some()).unwrap();
    let l = Localizer::new(Some(lang));
    let open = component(&deck_packs_panel(&view, &l), &format!("widgets/{}/open", pack.namespace))["label"].clone();
    assert_that!(open).is_equal_to(json!(l.tr("packs-pack-open-mine")));
}

#[test]
fn deck_packs_panel_current_pack_open_is_plain_open() {
    let view = session().deck_packs().unwrap();
    let l = Localizer::new(Some("fr"));
    let open = component(&deck_packs_panel(&view, &l), "widgets/base/open")["label"].clone();
    assert_that!(open).is_equal_to(json!(l.tr("packs-pack-open")));
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
        id: uuid::Uuid::from_u128(1),
        guide: None,
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

#[test]
fn pack_row_has_an_open_button_carrying_the_pack_id() {
    let view = session().deck_packs().unwrap();
    let base = view.widgets.iter().find(|p| p.namespace == ns("base")).unwrap();
    let surface = deck_packs_panel(&view, &Localizer::new(Some("fr")));
    let open = component(&surface, "widgets/base/open");
    let event = &open["action"]["event"];
    assert_that!(event["name"].as_str()).is_equal_to(Some("open_pack"));
    assert_that!(event["context"].clone()).is_equal_to(json!({"pack": base.id.to_string()}));
    assert_that!(open["label"].as_str()).is_equal_to(Some("Ouvrir"));
}

#[test]
fn pack_row_open_comes_first_in_actions() {
    let view = session().deck_packs().unwrap();
    let surface = deck_packs_panel(&view, &Localizer::new(Some("en")));
    let actions = component(&surface, "widgets/base/actions");
    let first = actions["children"].as_array().and_then(|c| c.first()).and_then(Value::as_str);
    assert_that!(first).is_equal_to(Some("widgets/base/open"));
}

/// A session that opened a deck bringing a widgets pack with `use_` as use guide, never seen before.
fn opened_with_received_guide(use_: &str) -> (tempfile::TempDir, tempfile::TempDir, Session, uuid::Uuid) {
    opened_with_received_guides("", use_)
}

/// A session that opened a deck bringing a widgets pack with guides `edit` and `use_`, never seen before.
fn opened_with_received_guides(edit: &str, use_: &str) -> (tempfile::TempDir, tempfile::TempDir, Session, uuid::Uuid) {
    let (me, them) = (fresh_dir("oku-slides-panel-me-"), fresh_dir("oku-slides-panel-them-"));
    let id = uuid::Uuid::new_v4();
    let mut m = manifest(id, "Brought", None, "2026-10-01T10:00:00Z");
    m.guides = Guides { edit: edit.to_owned(), use_: use_.to_owned() };
    let file = received_file(them.path(), &files_of(&m));
    let mut s = session_on(me.path());
    s.open(&file).unwrap();
    (me, them, s, id)
}

fn brought(s: &Session) -> super::PinnedPackView {
    s.deck_packs().unwrap().widgets.into_iter().find(|p| p.name == "Brought").unwrap()
}

#[test]
fn deck_packs_panel_pack_with_a_use_guide_shows_it_folded() {
    let (_me, _them, s, _id) = opened_with_received_guide("use me");
    let view = s.deck_packs().unwrap();
    let surface = deck_packs_panel(&view, &Localizer::new(Some("en")));
    let ns = brought(&s).namespace;
    let fold = component(&surface, &format!("widgets/{ns}/guide"));
    assert_that!(fold["child"].as_str()).is_equal_to(Some(format!("widgets/{ns}/guide-text").as_str()));
    assert_that!(fold["open"].is_null()).is_true();
    assert_that!(text_of(&surface, &format!("widgets/{ns}/guide-text"))).is_equal_to("use me".to_owned());
}

#[test]
fn deck_packs_panel_guide_summary_names_the_origin() {
    let (_me, _them, s, _id) = opened_with_received_guide("use me");
    let (en, fr) = (Localizer::new(Some("en")), Localizer::new(Some("fr")));
    let ns = brought(&s).namespace;
    let summary = |l: &Localizer| {
        component(&deck_packs_panel(&s.deck_packs().unwrap(), l), &format!("widgets/{ns}/guide"))["summary"].clone()
    };
    assert_that!(summary(&en).as_str().unwrap().contains("eceived")).is_true();
    assert_that!(summary(&fr)).is_not_equal_to(summary(&en));
}

#[test]
fn deck_packs_panel_guide_text_is_clipped_to_the_limit() {
    let long = "x".repeat(200_000);
    let (_me, _them, s, _id) = opened_with_received_guide(&long);
    let ns = brought(&s).namespace;
    let surface = deck_packs_panel(&s.deck_packs().unwrap(), &Localizer::new(Some("en")));
    let shown = text_of(&surface, &format!("widgets/{ns}/guide-text"));
    assert_that!(shown.chars().count()).is_equal_to(s.guide_limit);
}

#[test]
fn deck_packs_panel_pack_without_a_guide_has_no_guide_rows() {
    let mut s = session();
    crate::test_support::adopt_deck_only_pack(&mut s);
    let view = s.deck_packs().unwrap();
    let ids = component_ids(&deck_packs_panel(&view, &Localizer::new(Some("en"))));
    assert_that!(ids.contains(&"widgets/deck/remove".to_owned())).is_true();
    assert_that!(ids.iter().any(|i| i.starts_with("widgets/deck/guide"))).is_false();
}

/// A built-in's use guide shows, trusted: no approval button (t-1411).
#[test]
fn deck_packs_panel_builtin_base_shows_its_use_guide_without_approval() {
    let view = session().deck_packs().unwrap();
    let surface = deck_packs_panel(&view, &Localizer::new(Some("en")));
    let ids = component_ids(&surface);
    assert_that!(text_of(&surface, "widgets/base/guide-text")).starts_with("# Using Base");
    assert_that!(ids.contains(&"widgets/base/guide-approve".to_owned())).is_false();
}

#[test]
fn deck_packs_panel_received_unapproved_guide_offers_approval() {
    let (_me, _them, s, id) = opened_with_received_guide("use me");
    let ns = brought(&s).namespace;
    let surface = deck_packs_panel(&s.deck_packs().unwrap(), &Localizer::new(Some("en")));
    let button = component(&surface, &format!("widgets/{ns}/guide-approve"));
    let event = &button["action"]["event"];
    assert_that!(event["name"].as_str()).is_equal_to(Some("approve_guides"));
    assert_that!(event["context"]["id"].as_str()).is_equal_to(Some(id.to_string().as_str()));
    assert_that!(button["label"].as_str()).is_equal_to(Some("Approve this guide"));
}

/// The approval covers both guides: while it awaits, the entry holds both texts (I2).
#[test]
fn deck_packs_received_unapproved_entry_holds_both_guides() {
    let (_me, _them, s, _id) = opened_with_received_guides("hostile edit", "harmless use");
    let texts = brought(&s).guide.map(|g| g.texts).unwrap_or_default();
    let expected = vec![(GuideKind::Use, "harmless use".to_owned()), (GuideKind::Edit, "hostile edit".to_owned())];
    assert_that!(texts).is_equal_to(expected);
}

#[test]
fn deck_packs_panel_received_unapproved_pack_shows_its_edit_guide_folded_and_clipped() {
    let long = format!("hostile edit{}", "x".repeat(200_000));
    let (_me, _them, s, _id) = opened_with_received_guides(&long, "harmless use");
    let ns = brought(&s).namespace;
    let surface = deck_packs_panel(&s.deck_packs().unwrap(), &Localizer::new(Some("en")));
    let fold = component(&surface, &format!("widgets/{ns}/guide-edit"));
    assert_that!(fold["child"].as_str()).is_equal_to(Some(format!("widgets/{ns}/guide-edit-text").as_str()));
    assert_that!(fold["open"].is_null()).is_true();
    let shown = text_of(&surface, &format!("widgets/{ns}/guide-edit-text"));
    assert_that!((shown.starts_with("hostile edit"), shown.chars().count())).is_equal_to((true, s.guide_limit));
}

#[test]
fn deck_packs_approved_entry_shows_only_the_use_guide() {
    let (_me, _them, mut s, id) = opened_with_received_guides("hostile edit", "harmless use");
    let version = brought(&s).guide.unwrap().version;
    s.handle(Request::ApproveGuides { versions: vec![VersionRef { id, version }] }).unwrap();
    let texts = brought(&s).guide.map(|g| g.texts).unwrap_or_default();
    assert_that!(texts).is_equal_to(vec![(GuideKind::Use, "harmless use".to_owned())]);
}

#[test]
fn packs_event_approve_guides_parses_id_and_version() {
    let (_me, _them, s, id) = opened_with_received_guide("use me");
    let version = brought(&s).version;
    let ctx = json!({"id": id.to_string(), "version": version});
    let event = packs_event("approve_guides", ctx.as_object().unwrap());
    let expected = PacksEvent::ApproveGuides { id, version: oku_slides_core::PackVersion::from_hex(&version).unwrap() };
    assert_that!(event).is_equal_to(Some(expected));
}

#[rstest]
#[case(json!({}))]
#[case(json!({"id": "nope", "version": "ab"}))]
#[case(json!({"id": "00000000-0000-0000-0000-000000000001"}))]
fn packs_event_approve_guides_rejects_a_malformed_context(#[case] ctx: Value) {
    assert_that!(packs_event("approve_guides", ctx.as_object().unwrap())).is_none();
}

#[test]
fn packs_action_approve_guides_approves_the_version_and_the_button_goes() {
    let (_me, _them, mut s, id) = opened_with_received_guide("use me");
    let pack = brought(&s);
    let version = pack.guide.as_ref().map(|g| g.version.clone()).unwrap();
    s.packs_action(PacksEvent::ApproveGuides { id, version: version.clone() }).unwrap();
    assert_that!(s.guide_active(id, &version, false)).is_true();
    let surface = deck_packs_panel(&s.deck_packs().unwrap(), &Localizer::new(Some("en")));
    let ids = component_ids(&surface);
    assert_that!(ids.contains(&format!("widgets/{}/guide-approve", pack.namespace))).is_false();
    assert_that!(s.snapshot().guides_to_approve).is_empty();
}

#[test]
fn packs_action_approve_guides_refuses_a_version_the_deck_does_not_pin() {
    let (_me, _them, mut s, id) = opened_with_received_guide("use me");
    let forged = oku_slides_core::PackVersion::from_hex(&"ab".repeat(32)).unwrap();
    let err = s.packs_action(PacksEvent::ApproveGuides { id, version: forged.clone() }).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::UnknownNamespace);
    assert_that!(s.guide_active(id, &forged, false)).is_false();
}

#[test]
fn packs_action_approve_guides_has_the_effect_of_the_request() {
    let (_me, _them, mut s, _id) = opened_with_received_guide("use me");
    let versions: Vec<VersionRef> =
        s.snapshot().guides_to_approve.iter().map(|r| VersionRef { id: r.id, version: r.version.clone() }).collect();
    s.handle(Request::LaterGuides { versions: versions.clone() }).unwrap();
    let r = &versions[0];
    s.packs_action(PacksEvent::ApproveGuides { id: r.id, version: r.version.clone() }).unwrap();
    assert_that!(s.guide_active(r.id, &r.version, false)).is_true();
}

#[test]
fn deck_packs_panel_theme_guide_rows_use_theme_ids() {
    let mut view = session().deck_packs().unwrap();
    view.theme.guide = Some(super::PinnedGuideView {
        id: uuid::Uuid::from_u128(7),
        version: oku_slides_core::PackVersion::from_hex(&"cd".repeat(32)).unwrap(),
        texts: vec![(GuideKind::Use, "theme advice".to_owned()), (GuideKind::Edit, "theme edit".to_owned())],
        origin: crate::GuideOrigin::Received,
        awaiting: true,
        max_chars: 100,
    });
    let surface = deck_packs_panel(&view, &Localizer::new(Some("en")));
    assert_that!(text_of(&surface, "theme-guide-text")).is_equal_to("theme advice".to_owned());
    assert_that!(text_of(&surface, "theme-guide-edit-text")).is_equal_to("theme edit".to_owned());
    let event = &component(&surface, "theme-guide-approve")["action"]["event"];
    assert_that!(event["name"].as_str()).is_equal_to(Some("approve_guides"));
}
