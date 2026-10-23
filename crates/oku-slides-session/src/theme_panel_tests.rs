use oku_slides_i18n::{Localizer, english};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

use super::{TokenAction, TokenState, token_state};
use crate::pack::{BASE_PACK, ESSENTIEL_PACK};
use crate::test_support::{add_user_theme, builtin, fresh_dir, ns, opened_on, session};
use crate::theme::{PAPIER_THEME, PRO_THEME};
use crate::theme_schema::schema;
use crate::{Request, Session, SessionErrorKind};

/// Le français, pour `has` (sans le secours anglais).
fn french() -> Localizer {
    Localizer::new(Some("fr"))
}

fn p(path: &[&str]) -> Vec<String> {
    path.iter().map(|s| (*s).to_owned()).collect()
}

/// Une session en mode pack dont le brouillon part du thème intégré `id`.
fn pack_session(id: Uuid) -> Session {
    let mut s = session();
    if s.deck_theme_pin().map(|r| r.id) != Some(id) {
        let namespace = <crate::theme::Themes as crate::library_kind::LibraryKind>::builtins()
            .iter()
            .find(|b| b.manifest.id == id)
            .unwrap()
            .manifest
            .namespace
            .clone();
        s.set_theme(&namespace).unwrap();
    }
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s
}

fn surface_json(s: &Session, filter: &str) -> String {
    serde_json::to_string(&s.host_theme(filter).unwrap().unwrap()).unwrap()
}

fn token(s: &Session, pointer: &str) -> Option<Value> {
    s.deck().tokens.pointer(pointer).cloned()
}

fn request(s: &mut Session, path: &[&str], action: &str, value: Option<Value>) -> crate::Result<String> {
    let mut req = json!({ "cmd": "host_token", "path": path, "action": action });
    if let Some(v) = value {
        req["value"] = v;
    }
    s.handle(serde_json::from_value::<Request>(req).unwrap())
}

#[rstest]
#[case(json!({}), json!({}), TokenState::Default)]
#[case(json!({"size": {"body": 30}}), json!({"size": {"body": 30}}), TokenState::Defined)]
#[case(json!({"size": {"body": 32}}), json!({"size": {"body": 30}}), TokenState::Modified)]
#[case(json!({"size": {"body": 32}}), json!({}), TokenState::Modified)]
fn token_state_compares_draft_and_base(#[case] draft: Value, #[case] base: Value, #[case] want: TokenState) {
    assert_that!(token_state(&p(&["size", "body"]), draft.as_object().unwrap(), base.as_object().unwrap()))
        .is_equal_to(want);
}

#[test]
fn host_theme_filter_keeps_matching_rows_only() {
    let s = pack_session(PAPIER_THEME);
    let json = surface_json(&s, "accent");
    assert_that!(json.contains("color.accent")).is_true();
    assert_that!(json.contains("size.body")).is_false();
}

#[test]
fn host_theme_filter_matches_the_translated_label() {
    let s = pack_session(PAPIER_THEME);
    let label = s.localizer.tr("theme-token-page-margin-x").to_lowercase();
    let json = surface_json(&s, &label);
    assert_that!(json.contains("page.margin-x")).is_true();
    assert_that!(json.contains("color.accent")).is_false();
}

#[test]
fn host_theme_filter_without_match_omits_every_section() {
    let s = pack_session(PAPIER_THEME);
    assert_that!(surface_json(&s, "zzz-aucun").contains("\"Disclosure\"")).is_false();
}

#[test]
fn host_theme_outside_pack_is_none() {
    assert_that!(session().host_theme("").unwrap()).is_none();
}

#[test]
fn host_theme_opens_only_the_colors_section_without_filter() {
    let s = pack_session(PAPIER_THEME);
    let surface = serde_json::to_value(s.host_theme("").unwrap().unwrap()).unwrap();
    let open: Vec<(String, bool)> = disclosures(&surface);
    assert_that!(open.first().cloned()).is_equal_to(Some(("s-colors".to_owned(), true)));
    assert_that!(open.iter().skip(1).all(|(_, o)| !o)).is_true();
    assert_that!(open.len()).is_greater_than(5);
}

#[test]
fn host_theme_opens_every_matching_section_with_a_filter() {
    let s = pack_session(PAPIER_THEME);
    let surface = serde_json::to_value(s.host_theme("accent").unwrap().unwrap()).unwrap();
    let open = disclosures(&surface);
    assert_that!(open.len()).is_greater_than(1);
    assert_that!(open.iter().all(|(_, o)| *o)).is_true();
}

/// Les sections de la surface : id et dépliée.
fn disclosures(surface: &Value) -> Vec<(String, bool)> {
    let components =
        surface["messages"].as_array().unwrap().iter().find_map(|m| m.get("updateComponents")).unwrap()["components"]
            .as_array()
            .unwrap()
            .clone();
    components
        .iter()
        .filter(|c| c["component"] == "Disclosure")
        .map(|c| (c["id"].as_str().unwrap().to_owned(), c["open"].as_bool().unwrap_or(false)))
        .collect()
}

#[test]
fn host_theme_rows_show_state_and_their_actions() {
    let mut s = pack_session(PAPIER_THEME);
    s.host_token(p(&["color", "accent"]), TokenAction::Set, Some(json!("#ff0000"))).unwrap();
    let json = surface_json(&s, "color.accent");
    assert_that!(json.contains(&s.localizer.tr("theme-state-modified"))).is_true();
    assert_that!(json.contains("color.accent:reset")).is_true();
    assert_that!(json.contains("color.accent:remove")).is_true();
    assert_that!(json.contains("color.accent:clear")).is_false();
}

#[test]
fn host_theme_optional_absent_token_is_default_without_remove() {
    let s = pack_session(PAPIER_THEME);
    let json = surface_json(&s, "app-light.desk");
    assert_that!(json.contains(&s.localizer.tr("theme-state-default"))).is_true();
    assert_that!(json.contains("app-light.desk:remove")).is_false();
}

#[test]
fn host_theme_lists_free_colors_and_the_new_color_row() {
    let s = pack_session(PRO_THEME);
    let json = surface_json(&s, "");
    assert_that!(json.contains("color.brand-ink")).is_true();
    assert_that!(json.contains("new-color:add")).is_true();
}

#[test]
fn host_token_reset_in_a_child_draft_unsets_the_token() {
    let mut s = pack_session(PAPIER_THEME);
    s.host_token(p(&["color", "accent"]), TokenAction::Set, Some(json!("#ff0000"))).unwrap();
    s.host_token(p(&["color", "accent"]), TokenAction::Reset, None).unwrap();
    assert_that!(token(&s, "/color/accent")).is_none();
}

#[test]
fn host_token_reset_in_a_user_theme_restores_its_value() {
    let data = fresh_dir("oku-slides-theme-panel-");
    let mut s = opened_on(data.path());
    let tokens = json!({"color": {"accent": "#204060"}}).as_object().unwrap().clone();
    add_user_theme(&s.themes, Uuid::new_v4(), "maison", &[], tokens, None);
    s.set_theme(&ns("maison")).unwrap();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    let before = token(&s, "/color/accent");
    s.host_token(p(&["color", "accent"]), TokenAction::Set, Some(json!("#ff0000"))).unwrap();
    s.host_token(p(&["color", "accent"]), TokenAction::Reset, None).unwrap();
    assert_that!(token(&s, "/color/accent")).is_equal_to(before);
}

#[test]
fn host_token_reset_of_a_key_absent_from_base_unsets_it() {
    let mut s = pack_session(PAPIER_THEME);
    s.host_token(p(&["color", "brand"]), TokenAction::Set, Some(json!("#123456"))).unwrap();
    s.host_token(p(&["color", "brand"]), TokenAction::Reset, None).unwrap();
    assert_that!(token(&s, "/color/brand")).is_none();
}

#[test]
fn host_token_set_is_one_undo_step_with_the_token_label() {
    let mut s = pack_session(PAPIER_THEME);
    s.host_token(p(&["color", "accent"]), TokenAction::Set, Some(json!("#ff0000"))).unwrap();
    let label = s.localizer.tr("theme-token-color-accent");
    assert_that!(s.snapshot().undo_label.unwrap_or_default().contains(&label)).is_true();
    s.undo().unwrap();
    assert_that!(token(&s, "/color/accent")).is_none();
}

#[test]
fn host_token_remove_of_a_referenced_token_is_refused() {
    let mut s = pack_session(PAPIER_THEME);
    s.host_token(p(&["color", "brand"]), TokenAction::Set, Some(json!("#123456"))).unwrap();
    s.host_token(p(&["color", "accent"]), TokenAction::Set, Some(json!("brand"))).unwrap();
    let err = s.host_token(p(&["color", "brand"]), TokenAction::Remove, None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::TokenInUse);
}

#[test]
fn host_token_clear_of_a_non_nullable_token_is_refused() {
    let mut s = pack_session(PAPIER_THEME);
    let err = s.host_token(p(&["color", "accent"]), TokenAction::Clear, None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::InvalidTokenValue);
}

#[test]
fn host_token_new_color_with_a_bad_name_is_refused() {
    let mut s = pack_session(PAPIER_THEME);
    let err = s.host_token(p(&["color", "Brand"]), TokenAction::Set, Some(json!("#123456"))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::InvalidTokenValue);
}

#[test]
fn host_token_outside_pack_is_refused() {
    let mut s = session();
    let err = s.host_token(p(&["color", "accent"]), TokenAction::Set, Some(json!("#ff0000"))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NotEditingPack);
}

#[rstest]
#[case(&["weight", "title"], json!("700"), "/weight/title", json!(700))]
#[case(&["size", "body"], json!("30.5"), "/size/body", json!(30.5))]
#[case(&["size", "lead"], json!("title"), "/size/lead", json!("title"))]
#[case(&["header", "rule"], json!("true"), "/header/rule", json!(true))]
#[case(&["component", "essentiel", "title", "rule", "height"], json!(6), "/component/essentiel/title/rule/height", json!(6))]
#[case(&["font", "body"], json!([{"id": "0", "family": "Inter"}]), "/font/body", json!(["Inter"]))]
fn host_token_set_converts_the_control_value(
    #[case] path: &[&str],
    #[case] value: Value,
    #[case] pointer: &str,
    #[case] want: Value,
) {
    let mut s = pack_session(PAPIER_THEME);
    s.host_token(p(path), TokenAction::Set, Some(value)).unwrap();
    assert_that!(token(&s, pointer)).is_equal_to(Some(want));
}

#[test]
fn host_token_list_changes_edit_the_font_families_in_order() {
    let mut s = pack_session(PAPIER_THEME);
    let path = p(&["font", "body"]);
    s.host_token(path.clone(), TokenAction::Set, Some(json!(["A", "B"]))).unwrap();
    let change = |s: &mut Session, c: Value| s.host_token(path.clone(), TokenAction::Set, Some(c)).unwrap();
    change(&mut s, json!({"op": "insert", "id": "new"}));
    assert_that!(token(&s, "/font/body")).is_equal_to(Some(json!(["A", "B", "B"])));
    change(&mut s, json!({"op": "set_field", "item": "2", "field": "family", "value": "C"}));
    change(&mut s, json!({"op": "move", "item": "2", "to": 0}));
    assert_that!(token(&s, "/font/body")).is_equal_to(Some(json!(["C", "A", "B"])));
    change(&mut s, json!({"op": "remove", "item": "1"}));
    change(&mut s, json!({"op": "replace", "items": [{"id": "x", "family": "D"}, {"id": "y", "family": "C"}]}));
    assert_that!(token(&s, "/font/body")).is_equal_to(Some(json!(["D", "C"])));
}

#[test]
fn host_token_rule_field_starts_a_cleared_rule() {
    let mut s = pack_session(PAPIER_THEME);
    s.host_token(p(&["component", "essentiel", "title", "rule"]), TokenAction::Clear, None).unwrap();
    let rule = p(&["component", "essentiel", "title", "rule", "color"]);
    s.host_token(rule, TokenAction::Set, Some(json!("ink"))).unwrap();
    let rule = token(&s, "/component/essentiel/title/rule").unwrap();
    assert_that!(rule["color"].clone()).is_equal_to(json!("ink"));
    assert_that!(rule["width"].is_number()).is_true();
}

#[test]
fn host_token_every_action_round_trips_through_the_request() {
    let mut s = pack_session(PAPIER_THEME);
    request(&mut s, &["color", "brand"], "set", Some(json!("#123456"))).unwrap();
    assert_that!(token(&s, "/color/brand")).is_equal_to(Some(json!("#123456")));
    request(&mut s, &["color", "brand"], "remove", None).unwrap();
    assert_that!(token(&s, "/color/brand")).is_none();
    request(&mut s, &["header", "left"], "clear", None).unwrap();
    assert_that!(token(&s, "/header/left")).is_equal_to(Some(Value::Null));
    request(&mut s, &["header", "left"], "reset", None).unwrap();
    let base = s.base_tokens();
    assert_that!(token(&s, "/header/left").as_ref()).is_equal_to(Value::Object(base).pointer("/header/left"));
}

#[test]
fn host_theme_request_returns_the_surface() {
    let mut s = pack_session(PAPIER_THEME);
    let json = s.handle(serde_json::from_value(json!({"cmd": "host_theme", "filter": "accent"})).unwrap()).unwrap();
    assert_that!(json.contains("host-theme")).is_true();
}

#[test]
fn every_schema_entry_has_an_english_and_french_label() {
    for spec in schema() {
        let id = format!("theme-token-{}", spec.path.join("-"));
        asserting(&id).that(&(english().has(&id), french().has(&id))).is_equal_to((true, true));
    }
}

#[test]
fn every_builtin_pack_entry_has_an_english_and_french_label() {
    for id in [BASE_PACK, ESSENTIEL_PACK] {
        let m = &builtin(id).manifest;
        assert_that!(m.tokens.is_empty()).is_false();
        for t in &m.tokens {
            let key = format!("theme-token-component-{}-{}", m.namespace.as_str(), t.path.join("-"));
            asserting(&key).that(&(english().has(&key), french().has(&key))).is_equal_to((true, true));
        }
    }
}

/// Les ids des sections de la surface, dans l'ordre.
fn section_ids(s: &Session) -> Vec<String> {
    let surface = serde_json::to_value(s.host_theme("").unwrap().unwrap()).unwrap();
    disclosures(&surface).into_iter().map(|(id, _)| id).collect()
}

#[test]
fn host_theme_sections_are_global_then_one_per_pack_then_interface() {
    let s = pack_session(PAPIER_THEME);
    let want = [
        "s-colors",
        "s-typography",
        "s-scripts",
        "s-spacing",
        "s-shapes",
        "s-page",
        "s-charts",
        "s-frame",
        "s-pack:base",
        "s-pack:essentiel",
        "s-app-light",
        "s-app-dark",
    ];
    assert_that!(section_ids(&s)).is_equal_to(want.map(String::from).to_vec());
}

#[test]
fn host_theme_pack_section_is_named_after_its_pack() {
    let s = pack_session(PAPIER_THEME);
    let json = surface_json(&s, "");
    assert_that!(json.contains(&format!("\"{}\"", builtin(BASE_PACK).manifest.name))).is_true();
}

#[test]
fn host_theme_labels_builtin_pack_tokens_through_fluent() {
    let s = pack_session(PAPIER_THEME);
    let label = s.localizer.tr("theme-token-component-base-card-bg");
    assert_that!(surface_json(&s, "component.base.card.bg").contains(&label)).is_true();
}

#[test]
fn host_theme_shows_the_extends_row_of_a_child_draft() {
    let s = pack_session(PAPIER_THEME);
    let text = s.localizer.tr_with("theme-extends", &[("names", "Papier".into())]);
    assert_that!(surface_json(&s, "").contains(&text)).is_true();
}

#[test]
fn host_theme_lists_the_stashed_deck_overrides() {
    let mut s = session();
    let op = oku_slides_core::Op::SetToken { path: p(&["color", "accent"]), value: json!("#123456") };
    s.apply("x", vec![op], None).unwrap();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    let text = s.localizer.tr_with("theme-deck-overrides", &[("paths", "color.accent".into())]);
    assert_that!(surface_json(&s, "").contains(&text)).is_true();
}

#[test]
fn host_theme_without_deck_overrides_has_no_overrides_row() {
    let s = pack_session(PAPIER_THEME);
    assert_that!(surface_json(&s, "").contains("deck-overrides")).is_false();
}

#[test]
fn host_theme_shows_the_inherited_value_of_a_token_absent_from_a_child_draft() {
    let s = pack_session(PAPIER_THEME);
    assert_that!(surface_json(&s, "color.accent").contains("#3f6b5e")).is_true();
}

#[test]
fn host_theme_keeps_the_typed_filter_in_the_data_model() {
    let s = pack_session(PAPIER_THEME);
    let surface = serde_json::to_value(s.host_theme("Taille ").unwrap().unwrap()).unwrap();
    let data = surface["messages"].as_array().unwrap().iter().find_map(|m| m.get("updateDataModel")).unwrap();
    assert_that!(data["value"]["filter"].clone()).is_equal_to(json!("Taille "));
}
