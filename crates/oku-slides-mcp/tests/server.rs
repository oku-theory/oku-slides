//! Real MCP server on 127.0.0.1.
//!
//! Required token, tool listing and read tools.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]
mod common;

use common::*;
use rust_mcp_sdk::McpClient;
use speculoos::prelude::*;

// `ClientRuntime` n'implémente pas `Debug` : `ResultAssertions` l'exige même
// pour `is_err`, donc on efface le côté `Ok` avant l'assertion.
#[tokio::test(flavor = "multi_thread")]
async fn without_token_client_is_refused() {
    let h = start().await;
    assert_that!(client(h.server.url(), None).await.map(|_| ())).is_err();
    assert_that!(client(h.server.url(), Some("Bearer faux".into())).await.map(|_| ())).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn tool_list_names_every_read_tool() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let names: Vec<String> = c.request_tool_list(None).await.unwrap().tools.into_iter().map(|t| t.name).collect();
    for n in ["deck_outline", "slide", "render_slide", "packs", "layouts", "settings"] {
        assert_that!(names.contains(&n.to_owned())).is_true();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn tools_list_has_pack_tools_and_no_template_tool() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let names: Vec<String> = c.request_tool_list(None).await.unwrap().tools.into_iter().map(|t| t.name).collect();
    for t in ["packs", "edit_pack", "save_pack", "close_pack", "delete_pack", "layouts"] {
        assert_that!(names.contains(&t.to_owned())).named(t).is_true();
    }
    assert_that!(names.iter().any(|n| n.contains("template"))).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn packs_of_kind_layouts_lists_essentiel_with_its_profiles() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let packs = json_of(&call(&c, "packs", serde_json::json!({ "kind": "layouts" })).await);
    let packs = packs.as_array().cloned().unwrap_or_default();
    assert_that!(packs.iter().all(|p| p["kind"] == "layouts")).is_true();
    let essentiel = packs.iter().find(|p| p["namespace"] == "essentiel").cloned().unwrap();
    assert_that!(essentiel["profiles"].as_u64().unwrap_or_default()).is_greater_than(0);
}

#[tokio::test(flavor = "multi_thread")]
async fn packs_without_kind_lists_widget_packs_then_layout_packs() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let packs = json_of(&call(&c, "packs", serde_json::json!({})).await);
    let kinds: Vec<&str> = packs.as_array().into_iter().flatten().filter_map(|p| p["kind"].as_str()).collect();
    assert_that!(kinds.first().copied()).is_equal_to(Some("widgets"));
    assert_that!(kinds.last().copied()).is_equal_to(Some("layouts"));
}

#[tokio::test(flavor = "multi_thread")]
async fn layouts_lists_the_essentiel_kinds_without_previews() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let layouts = json_of(&call(&c, "layouts", serde_json::json!({})).await);
    let pack = layouts["packs"].as_array().and_then(|p| p.first()).cloned().unwrap();
    assert_that!(pack["namespace"].as_str()).is_equal_to(Some("essentiel"));
    let kinds: Vec<&str> =
        pack["layouts"].as_array().into_iter().flatten().filter_map(|l| l["kind"].as_str()).collect();
    assert_that!(kinds).contains(oku_slides_core::kinds::essentiel::CONTENT);
    assert_that!(text_of(&call(&c, "layouts", serde_json::json!({})).await)).does_not_contain("svg");
}

#[tokio::test(flavor = "multi_thread")]
async fn deck_outline_lists_slides_with_titles() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let outline = json_of(&call(&c, "deck_outline", serde_json::json!({})).await);
    assert_that!(outline["slides"].as_array().map(Vec::len)).is_equal_to(Some(8));
    assert_that!(outline["mode"].as_str()).is_equal_to(Some("deck"));
}

#[tokio::test(flavor = "multi_thread")]
async fn deck_outline_names_the_deck_theme_and_lists_the_themes() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let outline = json_of(&call(&c, "deck_outline", serde_json::json!({})).await);
    assert_that!(outline["theme"])
        .is_equal_to(serde_json::json!({ "namespace": oku_slides_session::PAPIER_NAMESPACE, "name": "Papier" }));
    let names: Vec<&str> = outline["themes"].as_array().unwrap().iter().filter_map(|t| t["name"].as_str()).collect();
    assert_that!(names).is_equal_to(vec!["Papier", "Pro", "Neutre"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_sets_pro_with_only_what_deck_outline_returns() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let outline = json_of(&call(&c, "deck_outline", serde_json::json!({})).await);
    let pro = outline["themes"].as_array().unwrap().iter().find(|t| t["name"] == "Pro").unwrap().clone();
    let r = call(&c, "apply", serde_json::json!({ "ops": [{ "op": "set_theme", "theme": pro["namespace"] }] })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    let s = h.session.lock().unwrap();
    let ns = s.deck().theme.clone().unwrap();
    assert_that!(ns.as_str()).is_equal_to(oku_slides_session::PRO_NAMESPACE);
    let pin = s.deck().themes.get(&ns).cloned().unwrap();
    assert_that!(s.sources_for_tests().themes.contains_key(&pin.version)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn render_slide_returns_png_image() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "render_slide", serde_json::json!({ "id": "s-libre" })).await;
    let block = serde_json::to_value(&r.content).unwrap();
    assert_that!(block[0]["type"].as_str()).is_equal_to(Some("image"));
    assert_that!(block[0]["mimeType"].as_str()).is_equal_to(Some("image/png"));
}

#[tokio::test(flavor = "multi_thread")]
async fn slide_unknown_id_returns_tool_error() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "slide", serde_json::json!({ "id": "inconnue" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r).as_str()).is_equal_to("Slide introuvable : inconnue");
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_returns_port_values() {
    let h = start().await;
    h.settings.values.lock().unwrap().insert("dictation.model".into(), serde_json::json!("turbo"));
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let v = json_of(&call(&c, "settings", serde_json::json!({})).await);
    assert_that!(v["dictation.model"].as_str()).is_equal_to(Some("turbo"));
}

/// Every `description` in `v`, with the JSON path that leads to it.
fn schema_descriptions(v: &serde_json::Value, path: &str, out: &mut Vec<(String, String)>) {
    match v {
        serde_json::Value::Object(map) => {
            if let Some(d) = map.get("description").and_then(serde_json::Value::as_str) {
                out.push((path.to_owned(), d.to_owned()));
            }
            for (k, x) in map {
                schema_descriptions(x, &format!("{path}/{k}"), out);
            }
        }
        serde_json::Value::Array(items) => {
            for (i, x) in items.iter().enumerate() {
                schema_descriptions(x, &format!("{path}/{i}"), out);
            }
        }
        _ => {}
    }
}

/// Accented letters that betray French text left in a schema description.
const FRENCH_LETTERS: &[char] =
    &['é', 'è', 'ê', 'à', 'â', 'ç', 'ù', 'û', 'ô', 'î', 'œ', 'É', 'È', 'Ê', 'À', 'Â', 'Ç', 'Ù', 'Û', 'Ô', 'Î', 'Œ'];

/// Every input schema description is plain English: no rustdoc link, no French.
///
/// The descriptions come from doc comments and go to the LLM as is.
#[tokio::test(flavor = "multi_thread")]
async fn tools_list_input_schema_descriptions_are_plain_english() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let tools = c.request_tool_list(None).await.unwrap().tools;
    let mut all = Vec::new();
    for tool in tools {
        let schema = serde_json::to_value(&tool.input_schema).unwrap();
        schema_descriptions(&schema, &tool.name, &mut all);
    }
    assert_that!(all.len()).is_greater_than(0);
    let bad: Vec<String> = all
        .into_iter()
        .filter(|(_, d)| d.contains("](") || d.contains("[`") || d.contains(FRENCH_LETTERS))
        .map(|(p, d)| format!("{p}: {d}"))
        .collect();
    assert_that!(bad).is_empty();
}
