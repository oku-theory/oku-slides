//! Les outils de l'ajustement d'un import : `import_report` et `import_diff`,
//! sur un onglet importé d'alto en hi-fi, de `fixtures/import/minimal` en retrofit (le témoin
//! office2pdf rend pour de vrai).

mod common;

use base64::Engine as _;
use common::*;
use oku_slides_session::import::ImportTarget;
use rust_mcp_sdk::McpClient;
use rust_mcp_sdk::schema::{CallToolResult, ContentBlock};
use serde_json::{Value, json};
use speculoos::prelude::*;

/// Largeur et hauteur du PNG du bloc `index` (en-tête IHDR).
fn png_size(r: &CallToolResult, index: usize) -> (u32, u32) {
    let Some(ContentBlock::ImageContent(image)) = r.content.get(index) else { panic!("image attendue en {index}") };
    let bytes = base64::engine::general_purpose::STANDARD.decode(&image.data).unwrap_or_else(|e| panic!("{e}"));
    let be = |at: usize| u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    (be(16), be(20))
}

/// Le JSON du bloc texte qui suit les deux images d'un diff.
fn diff_json(r: &CallToolResult) -> Value {
    let Some(ContentBlock::TextContent(t)) = r.content.get(2) else { panic!("texte attendu après les images") };
    serde_json::from_str(&t.text).unwrap_or_else(|e| panic!("JSON du diff : {e}"))
}

/// Le premier élément des slots de la slide `slide`, lu dans la session.
fn first_element(h: &Harness, slide: &str) -> String {
    let session = h.session.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let s = session.deck().slide(slide).unwrap_or_else(|| panic!("slide {slide}"));
    s.params.slots().flat_map(|(_, list)| list.iter().map(|e| e.id.clone())).next().unwrap_or_default()
}

#[tokio::test(flavor = "multi_thread")]
async fn import_tools_are_listed() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let names: Vec<String> = c.request_tool_list(None).await.unwrap().tools.into_iter().map(|t| t.name).collect();
    for tool in ["import_report", "import_diff"] {
        assert_that!(names.contains(&tool.to_owned())).named(tool).is_true();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn import_tools_on_a_deck_not_imported_say_so_in_the_user_language() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let expected = h.localizer.tr("session-import-tune-none");
    let english = oku_slides_i18n::Localizer::new(Some("en")).tr("session-import-tune-none");
    assert_that!(expected).is_not_equal_to(english);
    for (tool, args) in [("import_report", json!({})), ("import_diff", json!({ "slide": "s1" }))] {
        let r = call(&c, tool, args).await;
        assert_that!(r.is_error).named(tool).is_equal_to(Some(true));
        assert_that!(text_of(&r)).named(tool).is_equal_to(expected.clone());
    }
}

/// La fixture des tests hi-fi : alto, que le témoin office2pdf signale (s7, l'encadré, est `pending`).
const ALTO: &str = "alto-wealth";

/// Une slide `pending` d'alto, avec des éléments à retoucher.
const PENDING: &str = "s7";

/// Les essais restants de la slide `slide` dans la file du rapport `report`.
fn tries_left(report: &Value, slide: &str) -> Option<u64> {
    report["slides"].as_array()?.iter().find(|s| s["slide"] == slide)?["tries_left"].as_u64()
}

/// Déplace de 12 pt le premier élément cadré de la slide `slide`, par l'outil `apply`.
async fn nudge(c: &rust_mcp_sdk::mcp_client::ClientRuntime, h: &Harness, slide: &str) {
    let (element, mut frame) = {
        let session = h.session.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let s = session.deck().slide(slide).unwrap_or_else(|| panic!("slide {slide}"));
        let e = s.params.slots().flat_map(|(_, list)| list.iter()).find(|e| e.frame.is_some());
        let e = e.unwrap_or_else(|| panic!("élément cadré dans {slide}"));
        (e.id.clone(), e.frame.unwrap_or_else(|| panic!("cadre")))
    };
    frame.x += 12.0;
    let ops = json!([{ "op": "set_frame", "slide": slide, "element": element, "frame": frame }]);
    let applied = call(c, "apply", json!({ "label": "Déplacer", "ops": ops })).await;
    assert_that!(applied.is_error).is_not_equal_to(Some(true));
}

#[tokio::test(flavor = "multi_thread")]
async fn import_report_hifi_gives_the_queue_worst_first() {
    let h = start_imported(ALTO, ImportTarget::Hifi, |_| {}).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let report = json_of(&call(&c, "import_report", json!({})).await);
    assert_that!(report["mode"].as_str()).is_equal_to(Some("hifi"));
    assert_that!(report["witness"].as_str().unwrap_or_default()).starts_with("office2pdf");
    assert_that!(report["threshold"].as_f64()).is_equal_to(Some(7.0));
    assert_that!(report["global_tries_left"].as_u64()).is_equal_to(Some(3));
    let slides = report["slides"].as_array().cloned().unwrap_or_default();
    assert_that!(slides.len() as u64 + report["done"].as_u64().unwrap_or_default()).is_equal_to(8);
    let worst: Vec<f64> = slides.iter().filter_map(|s| s["worst"].as_f64()).collect();
    assert_that!(worst.windows(2).all(|w| w[0] >= w[1])).is_true();
    // Seule une slide `pending` a des essais.
    for s in &slides {
        let expected = if s["state"] == "pending" { 3 } else { 0 };
        assert_that!(s["tries_left"].as_u64())
            .named(s["slide"].as_str().unwrap_or_default())
            .is_equal_to(Some(expected));
    }
    assert_that!(tries_left(&report, PENDING)).is_equal_to(Some(3));
}

#[tokio::test(flavor = "multi_thread")]
async fn import_diff_gives_both_pages_then_the_regions_and_bounds_the_tries() {
    let h = start_imported(ALTO, ImportTarget::Hifi, |s| s.import.tune_iterations = std::num::NonZeroU32::MIN).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let first = call(&c, "import_diff", json!({ "slide": PENDING })).await;
    assert_that!(first.is_error).is_not_equal_to(Some(true));
    assert_that!(png_size(&first, 0)).is_equal_to((960, 540));
    assert_that!(png_size(&first, 1)).is_equal_to((960, 540));
    let json = diff_json(&first);
    assert_that!((json["counted"].as_bool(), json["tries_left"].as_u64())).is_equal_to((Some(false), Some(1)));
    // Une slide modifiée : le diff suivant compte l'essai, le dernier ; puis le refus, zoom compris.
    let element = first_element(&h, PENDING);
    let ops = json!([{ "op": "remove_element", "slide": PENDING, "element": element }]);
    let applied = call(&c, "apply", json!({ "label": "Retirer", "ops": ops })).await;
    assert_that!(applied.is_error).is_not_equal_to(Some(true));
    let second = diff_json(&call(&c, "import_diff", json!({ "slide": PENDING })).await);
    assert_that!((second["counted"].as_bool(), second["tries_left"].as_u64())).is_equal_to((Some(true), Some(0)));
    let item = json["regions"][0]["item"].as_str().unwrap_or_default().to_owned();
    for args in [json!({ "slide": PENDING }), json!({ "slide": PENDING, "region": item })] {
        let refused = call(&c, "import_diff", args).await;
        assert_that!(refused.is_error).is_equal_to(Some(true));
        assert_that!(text_of(&refused)).contains(PENDING);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn import_diff_of_a_slide_whose_neighbour_moved_counts_no_try() {
    // Retoucher s6 changeait l'empreinte Typst de la page de s7 (introspection) : un essai de s7 compté à tort.
    let h = start_imported(ALTO, ImportTarget::Hifi, |_| {}).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    json_of(&call(&c, "import_report", json!({})).await);
    nudge(&c, &h, "s6").await;
    let diff = diff_json(&call(&c, "import_diff", json!({ "slide": PENDING })).await);
    assert_that!(diff["charge"].as_str()).is_equal_to(Some("unchanged"));
    assert_that!((diff["counted"].as_bool(), diff["tries_left"].as_u64())).is_equal_to((Some(false), Some(3)));
    // Le rapport suivant note s6 (son essai à elle, si elle en a), pas s7.
    let report = json_of(&call(&c, "import_report", json!({})).await);
    assert_that!(tries_left(&report, PENDING)).is_equal_to(Some(3));
}

#[tokio::test(flavor = "multi_thread")]
async fn import_report_after_a_theme_edit_draws_a_global_try_not_slide_tries() {
    let h = start_imported(ALTO, ImportTarget::Hifi, |_| {}).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let first = json_of(&call(&c, "import_report", json!({})).await);
    assert_that!(first["global_tries_left"].as_u64()).is_equal_to(Some(3));
    // Une retouche du thème généré, comme celle de l'interface : toutes les pages bougent.
    let session = h.session.clone();
    tokio::task::spawn_blocking(move || {
        let mut s = session.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let id = s.deck().theme.as_ref().map(|t| t.id).unwrap_or_else(|| panic!("thème du deck"));
        let path = vec!["page".to_owned(), "margin-top".to_owned()];
        let op = oku_slides_session::TokenOp::SetToken { path, value: json!(30) };
        s.edit_theme(id, None, vec![op]).unwrap_or_else(|e| panic!("edit_theme : {e}"));
    })
    .await
    .unwrap_or_else(|e| panic!("{e}"));
    let edited = json_of(&call(&c, "import_report", json!({})).await);
    assert_that!(edited["global_tries_left"].as_u64()).is_equal_to(Some(2));
    assert_that!(edited["mean"].as_f64()).is_not_equal_to(first["mean"].as_f64());
    let slides = edited["slides"].as_array().cloned().unwrap_or_default();
    let pending: Vec<&Value> = slides.iter().filter(|s| s["state"] == "pending").collect();
    assert_that!(pending).is_not_empty();
    for s in pending {
        assert_that!(s["tries_left"].as_u64()).named(s["slide"].as_str().unwrap_or_default()).is_equal_to(Some(3));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn import_diff_of_a_region_crops_it_at_twice_the_size() {
    let h = start_imported(ALTO, ImportTarget::Hifi, |_| {}).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let whole = diff_json(&call(&c, "import_diff", json!({ "slide": PENDING })).await);
    let item = whole["regions"][0]["item"].as_str().unwrap_or_default().to_owned();
    let zoom = call(&c, "import_diff", json!({ "slide": PENDING, "region": item })).await;
    let json = diff_json(&zoom);
    let crop: Vec<u32> = (0..4).map(|i| json["crop"][i].as_u64().unwrap_or_default() as u32).collect();
    assert_that!(png_size(&zoom, 0)).is_equal_to((crop[2] * 2, crop[3] * 2));
    assert_that!(png_size(&zoom, 1)).is_equal_to((crop[2] * 2, crop[3] * 2));
    assert_that!(json["region"]["item"].as_str()).is_equal_to(Some(item.as_str()));
}

#[tokio::test(flavor = "multi_thread")]
async fn import_report_retrofit_gives_the_widget_requests_and_import_diff_is_refused() {
    let target = ImportTarget::Retrofit { pack: "Essentiel".into(), theme: None };
    let h = start_imported("minimal", target, |_| {}).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let report = json_of(&call(&c, "import_report", json!({})).await);
    assert_that!(report["mode"].as_str()).is_equal_to(Some("retrofit"));
    assert_that!(report["widget_requests"].is_array()).is_true();
    let r = call(&c, "import_diff", json!({ "slide": "s1" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("session-import-tune-hifi-only"));
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_theme_of_the_hifi_theme_asks_no_confirmation() {
    let h = start_imported(ALTO, ImportTarget::Hifi, |_| {}).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let theme = h.session.lock().unwrap_or_else(std::sync::PoisonError::into_inner).deck().theme.clone();
    let theme = theme.unwrap_or_else(|| panic!("thème du deck"));
    let mut rx = h.hub.subscribe();
    let args = json!({ "theme": theme.id, "name": "Alto retouché" });
    // Une confirmation demandée laisserait l'appel en attente : la borne le dirait.
    let r = tokio::time::timeout(std::time::Duration::from_secs(60), call(&c, "edit_theme", args))
        .await
        .unwrap_or_else(|_| panic!("edit_theme attend une réponse de l'utilisateur"));
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(json_of(&r)["adopted"]).is_equal_to(json!(true));
    let mut surfaces = Vec::new();
    while let Ok(event) = rx.try_recv() {
        if let oku_slides_mcp::McpEvent::Surface { surface } = event {
            surfaces.push(surface.id);
        }
    }
    assert_that!(surfaces).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn theme_tokens_reads_the_hifi_theme_and_edit_theme_returns_its_new_tokens() {
    let h = start_imported(ALTO, ImportTarget::Hifi, |_| {}).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let (theme, tokens) = {
        let s = h.session.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let theme = s.deck().theme.clone().unwrap_or_else(|| panic!("thème du deck"));
        let tokens = s.theme_tokens_of(theme.id).unwrap_or_else(|e| panic!("jetons : {e}"));
        (theme, tokens)
    };
    let read = json_of(&call(&c, "theme_tokens", json!({ "theme": theme.id })).await);
    assert_that!(read["tokens"]).is_equal_to(Value::Object(tokens));
    assert_that!(read["deck_only"]).is_equal_to(json!(true));
    assert_that!(read["version"]).is_equal_to(json!(theme.version));
    let op = json!({ "op": "set_token", "path": ["page", "margin-top"], "value": 30 });
    let edited = json_of(&call(&c, "edit_theme", json!({ "theme": theme.id, "ops": [op] })).await);
    assert_that!(edited["tokens"]["page"]["margin-top"]).is_equal_to(json!(30));
    let again = json_of(&call(&c, "theme_tokens", json!({ "theme": theme.id })).await);
    assert_that!(again["tokens"]).is_equal_to(edited["tokens"].clone());
}

#[tokio::test(flavor = "multi_thread")]
async fn theme_tokens_of_an_unknown_theme_says_so_in_the_user_language() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let id = "00000000-0000-4000-8000-000000000001";
    let r = call(&c, "theme_tokens", json!({ "theme": id })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    let expected = h.localizer.tr_with("session-unknown-theme", &[("theme", id.into())]);
    let english = oku_slides_i18n::Localizer::new(Some("en")).tr_with("session-unknown-theme", &[("theme", id.into())]);
    assert_that!(expected.clone()).is_not_equal_to(english);
    assert_that!(text_of(&r)).is_equal_to(expected);
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_templates_of_the_hifi_pack_is_refused_as_deck_only() {
    let h = start_imported(ALTO, ImportTarget::Hifi, |_| {}).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let pack = h.session.lock().unwrap_or_else(std::sync::PoisonError::into_inner).deck().templates.clone();
    let pack = pack.unwrap_or_else(|| panic!("pack du deck"));
    let r = call(&c, "edit_templates", json!({ "set": pack.id })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    let expected = h.localizer.tr_with("session-deck-only", &[("id", pack.id.to_string().into())]);
    assert_that!(text_of(&r)).is_equal_to(expected);
}

/// Les messages d'une référence manquante par le temps, la mémoire ou un échec, dans la langue `locale`.
fn reference_unavailable(locale: &str) -> Vec<String> {
    let l = oku_slides_i18n::Localizer::new(Some(locale));
    ["timeout", "memory", "failed"]
        .into_iter()
        .map(|cause| {
            let args = [("cause", cause.into()), ("reference", 0.into()), ("deck", 0.into())];
            l.tr_with("import-reference-unavailable", &args)
        })
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn import_report_without_a_reference_says_why_in_the_live_language() {
    // Un témoin borné à 1 ms ne rend jamais : la référence manque, et le rapport dit pourquoi.
    let h = start_imported("minimal", ImportTarget::Hifi, |s| s.import.reference_timeout_ms = 1).await;
    h.live.set(std::sync::Arc::new(oku_slides_i18n::Localizer::new(Some("de"))));
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let report = json_of(&call(&c, "import_report", json!({})).await);
    let message = report["reference_unavailable"].as_str().unwrap_or_default().to_owned();
    assert_that!(reference_unavailable("de")).contains(message.clone());
    assert_that!(reference_unavailable("fr").contains(&message)).is_false();
    assert_that!(report["witness"].is_null()).is_true();
}
