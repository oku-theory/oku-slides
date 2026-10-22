//! Widgets dans une session : liste du snapshot (`pack_widgets`, `add_widgets`) et réglages de l'hôte en mode pack.

#[allow(dead_code, reason = "aides partagées : chaque binaire de test n'en utilise qu'une partie")]
mod common;

use common::{Isolated, fixture_path, req, session, snapshot_json};
use oku_slides_session::{Request, Session};
use serde_json::json;
use speculoos::prelude::*;

const KPI_WIDGET: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("kpi", params: (body: p.plain()),
  (args, ctx) => field(ctx, "body", args.body))
"#;

/// Les entrées de `list` (noms, ou objets à `name`) hors des packs intégrés, que tout deck neuf épingle.
fn non_base(list: &serde_json::Value) -> serde_json::Value {
    let of_base = |v: &serde_json::Value| {
        let name = v.as_str().or_else(|| v["name"].as_str()).unwrap_or_default();
        oku_slides_core::kinds::is_builtin_kind(name)
    };
    json!(list.as_array().into_iter().flatten().filter(|v| !of_base(v)).cloned().collect::<Vec<_>>())
}

/// Le snapshot nomme les widgets du pack du deck, sans `_commun`.
#[test]
fn snapshot_lists_the_pack_widget_files_without_common() {
    let mut s = session();
    s.handle(req(json!({"cmd": "set_widget", "name": "_commun", "source": "#let x = 1"}))).unwrap();
    s.handle(req(json!({"cmd": "set_widget", "name": "kpi", "source": KPI_WIDGET}))).unwrap();
    assert_that!(non_base(&snapshot_json(&mut s)["pack_widgets"])).is_equal_to(json!(["deck/kpi"]));
}

/// En mode pack, le snapshot nomme les widgets du pack édité, sans `_commun`.
#[test]
fn snapshot_lists_the_edited_pack_widgets_in_pack_mode() {
    let mut s = session();
    s.handle(req(json!({"cmd": "edit_pack", "pack": oku_slides_session::BASE_PACK}))).unwrap();
    s.handle(req(json!({"cmd": "set_widget", "name": "kpi", "source": KPI_WIDGET}))).unwrap();
    let snap = snapshot_json(&mut s);
    let names: Vec<&str> = snap["pack_widgets"].as_array().into_iter().flatten().filter_map(|v| v.as_str()).collect();
    assert_that!(snap["editing"].is_null()).is_false();
    assert_that!((names.contains(&"base/kpi"), names.iter().any(|n| n.ends_with("_commun"))))
        .is_equal_to((true, false));
}

/// Sans autre widget embarqué, la liste n'a que ceux des packs intégrés.
#[test]
fn snapshot_pack_widgets_is_only_base_without_another_embedded_pack() {
    let mut s = session();
    assert_that!(non_base(&snapshot_json(&mut s)["pack_widgets"])).is_equal_to(json!([]));
}

const KPI_LABELLED: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("kpi", label: "KPI", params: (body: p.plain()),
  (args, ctx) => field(ctx, "body", args.body))
"#;

const KPI_DESCRIBED: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("kpi", label: "KPI", description: "Une valeur en grand", params: (body: p.plain()),
  (args, ctx) => field(ctx, "body", args.body))
"#;

const COVER_SLIDE: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("cover", category: "slide", params: (title: p.plain()),
  (args, ctx) => args.title)
"#;

/// La barre d'ajout propose les widgets d'élément du pack, avec leur label.
#[test]
fn snapshot_add_widgets_lists_the_pack_element_widgets_with_their_label() {
    let mut s = session();
    s.handle(req(json!({"cmd": "set_widget", "name": "kpi", "source": KPI_LABELLED}))).unwrap();
    assert_that!(non_base(&snapshot_json(&mut s)["add_widgets"]))
        .is_equal_to(json!([{"name": "deck/kpi", "label": "KPI", "description": null}]));
}

/// La barre d'ajout porte la description d'un widget du pack, pour le filtre.
#[test]
fn snapshot_add_widgets_carries_the_widget_description() {
    let mut s = session();
    s.handle(req(json!({"cmd": "set_widget", "name": "kpi", "source": KPI_DESCRIBED}))).unwrap();
    assert_that!(non_base(&snapshot_json(&mut s)["add_widgets"])[0]["description"])
        .is_equal_to(json!("Une valeur en grand"));
}

/// Un fichier au label mal typé est écarté seul : les autres widgets du pack restent proposés.
#[test]
fn snapshot_add_widgets_keeps_the_pack_when_one_file_has_a_non_text_label() {
    let mut s = session();
    s.handle(req(json!({"cmd": "set_widget", "name": "kpi", "source": KPI_DESCRIBED}))).unwrap();
    let change = s.set_widget(None, "casse", NON_TEXT_LABEL.into()).unwrap_or_else(|e| panic!("{}", e.one_line()));
    let set_aside = change.status.schema.is_none()
        && change.status.diagnostics.iter().any(|d| d.message.contains("le label doit être un texte"));
    let added = non_base(&snapshot_json(&mut s)["add_widgets"]);
    let names: Vec<serde_json::Value> = added.as_array().into_iter().flatten().map(|w| w["name"].clone()).collect();
    assert_that!((change.saved, set_aside, names)).is_equal_to((true, true, vec![json!("deck/kpi")]));
}

const NON_TEXT_LABEL: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("casse", label: 42, params: (body: p.plain()),
  (args, ctx) => field(ctx, "body", args.body))
"#;

/// Un widget de slide du pack ne s'ajoute pas à une slide : absent de la liste.
#[test]
fn snapshot_add_widgets_leaves_out_slide_widgets() {
    let mut s = session();
    s.handle(req(json!({"cmd": "set_widget", "name": "cover", "source": COVER_SLIDE}))).unwrap();
    assert_that!(non_base(&snapshot_json(&mut s)["add_widgets"])).is_equal_to(json!([]));
}

/// Un widget du pack qui ne compile pas n'est pas au registre : absent de la liste.
#[test]
fn snapshot_add_widgets_leaves_out_broken_widgets() {
    let mut s = session();
    let _ = s.handle(req(json!({"cmd": "set_widget", "name": "kpi", "source": "#let widget = ("})));
    assert_that!(non_base(&snapshot_json(&mut s)["add_widgets"])).is_equal_to(json!([]));
}

const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
"#;

/// Une session en édition du pack de widgets `base`.
fn editing_widgets() -> Isolated {
    let mut s = session();
    s.edit_pack(oku_slides_session::BASE_PACK).unwrap_or_else(|e| panic!("édition du pack refusée : {}", e.one_line()));
    s
}

/// La surface de `host_settings`, en JSON.
fn host(s: &mut Session, widget: Option<&str>) -> serde_json::Value {
    let reply = s
        .handle(Request::HostSettings { widget: widget.map(Into::into) })
        .unwrap_or_else(|e| panic!("réglages de l'hôte refusés : {}", e.one_line()));
    serde_json::from_str(&reply).unwrap_or_else(|e| panic!("réglages de l'hôte illisibles : {e}"))
}

#[test]
fn host_settings_outside_pack_mode_is_null() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    assert_that!(host(&mut s, Some("quote"))).is_equal_to(json!(null));
}

#[test]
fn host_settings_of_a_widget_shows_its_example_values() {
    let mut s = editing_widgets();
    s.set_widget(None, "quote", QUOTE.into()).unwrap();
    s.handle(Request::SetExampleValue { name: "quote".into(), param: "source".into(), value: json!("Essai") }).unwrap();
    assert_that!(host(&mut s, Some("quote")).to_string().as_str()).contains("Essai");
}

#[test]
fn host_settings_of_a_broken_widget_keeps_the_widget_card() {
    let mut s = editing_widgets();
    s.set_widget(None, "quote", "#let widget = (".into()).unwrap();
    assert_that!(host(&mut s, Some("quote"))["id"].as_str()).is_equal_to(Some("host-widget"));
}

#[test]
fn host_settings_of_a_stale_widget_is_null() {
    let mut s = editing_widgets();
    assert_that!(host(&mut s, Some("absent"))).is_equal_to(json!(null));
}

#[test]
fn host_settings_without_a_widget_is_null() {
    let mut s = editing_widgets();
    assert_that!(host(&mut s, None)).is_equal_to(json!(null));
}

#[test]
fn set_example_value_request_leaves_the_working_deck_unchanged() {
    let mut s = editing_widgets();
    s.set_widget(None, "quote", QUOTE.into()).unwrap();
    let before = s.deck().slides.clone();
    s.handle(Request::SetExampleValue { name: "quote".into(), param: "source".into(), value: json!("Essai") }).unwrap();
    assert_that!(s.deck().slides.clone()).is_equal_to(before);
}
