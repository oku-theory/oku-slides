//! La boîte des réglages de la dictée et de l'agent, au niveau du hub.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use common::*;
use oku_slides_i18n::Localize;
use oku_slides_mcp::{
    ApiTarget, HubErrorKind, McpEvent, Origin, SurfaceAction, VoiceApplied, VoiceErrorKind, VoicePort,
};
use oku_slides_session::{
    AgentPresetView, DictationLanguage, DictationPatch, DownloadView, HOST_VOICE, VoiceEvent, VoicePanel, VoiceView,
};
use rstest::rstest;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

fn ctx(v: &Value) -> Map<String, Value> {
    v.as_object().cloned().unwrap_or_default()
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    tokio::task::spawn_blocking(f).await.unwrap_or_else(|e| panic!("tâche bloquante : {e}"))
}

async fn show(h: &Harness) {
    let hub = h.hub.clone();
    blocking(move || hub.show_settings()).await;
}

async fn refresh(h: &Harness) {
    let (hub, panel) = (h.hub.clone(), VoicePanel::build(&h.voice.view(), &h.localizer));
    blocking(move || hub.refresh_settings_with(&panel)).await;
}

async fn act(h: &Harness, name: &str, context: Value) -> Result<(), oku_slides_mcp::HubError> {
    let hub = h.hub.clone();
    let action = SurfaceAction { surface: HOST_VOICE.into(), name: name.into(), context: ctx(&context) };
    blocking(move || hub.act(action)).await
}

fn shown(h: &Harness) -> bool {
    h.hub.surfaces().iter().any(|s| s.id == HOST_VOICE)
}

fn is_box(e: &McpEvent) -> bool {
    matches!(e, McpEvent::Surface { surface } if surface.id == HOST_VOICE)
}

fn is_patch(e: &McpEvent) -> bool {
    matches!(e, McpEvent::SurfacePatch { id, .. } if id == HOST_VOICE)
}

/// Le modèle de données d'une boîte ou d'un patch de la boîte.
fn data_of(e: &McpEvent) -> Value {
    let messages = match e {
        McpEvent::Surface { surface } => serde_json::to_value(&surface.messages).unwrap_or_default(),
        McpEvent::SurfacePatch { messages, .. } => serde_json::to_value(messages).unwrap_or_default(),
        _ => Value::Null,
    };
    messages.as_array().and_then(|m| m.iter().find_map(|m| m.get("updateDataModel").cloned())).unwrap_or_default()["value"].clone()
}

#[tokio::test(flavor = "multi_thread")]
async fn show_settings_puts_the_box_live_outside_any_turn() {
    let h = start().await;
    show(&h).await;
    let s = h.hub.surfaces().into_iter().find(|s| s.id == HOST_VOICE).unwrap();
    assert_that!((s.live, s.turn)).is_equal_to((true, None));
}

#[tokio::test(flavor = "multi_thread")]
async fn show_settings_survives_a_new_conversation() {
    let h = start().await;
    show(&h).await;
    h.hub.clear_thread();
    assert_that!(shown(&h)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_click_applies_as_user() {
    let h = start().await;
    show(&h).await;
    act(&h, "model", json!({"id": "large"})).await.unwrap();
    let patch = DictationPatch { model: Some("large".into()), ..Default::default() };
    assert_that!(*lock(&h.voice.applied)).is_equal_to(vec![(VoiceEvent::Dictation(patch), Origin::User)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_click_with_the_same_layout_sends_a_patch() {
    let h = start().await;
    show(&h).await;
    lock(&h.voice.view).dictation.as_mut().unwrap().language = Some("fr".into());
    let mut rx = h.hub.subscribe();
    act(&h, "language", json!({"code": "fr"})).await.unwrap();
    assert_that!(rx.try_recv()).is_ok().matches(is_patch);
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_click_with_a_new_layout_sends_the_box() {
    let h = start().await;
    show(&h).await;
    lock(&h.voice.view).dictation.as_mut().unwrap().model = "api".into();
    let mut rx = h.hub.subscribe();
    act(&h, "model", json!({"id": "api"})).await.unwrap();
    assert_that!(rx.try_recv()).is_ok().matches(is_box);
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_dismiss_closes_the_box() {
    let h = start().await;
    show(&h).await;
    act(&h, "dismiss", json!({})).await.unwrap();
    assert_that!(shown(&h)).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn refresh_after_dismiss_does_not_reopen() {
    let h = start().await;
    show(&h).await;
    act(&h, "dismiss", json!({})).await.unwrap();
    refresh(&h).await;
    assert_that!(shown(&h)).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_click_on_a_closed_box_is_unknown() {
    let h = start().await;
    let r = act(&h, "model", json!({"id": "large"})).await;
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(HubErrorKind::UnknownSurface);
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_malformed_click_is_incomplete() {
    let h = start().await;
    show(&h).await;
    let r = act(&h, "model", json!({})).await;
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(HubErrorKind::Incomplete);
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_malformed_click_keeps_the_box() {
    let h = start().await;
    show(&h).await;
    let _ = act(&h, "model", json!({})).await;
    assert_that!(shown(&h)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_refused_click_is_a_voice_error() {
    let h = start().await;
    show(&h).await;
    *lock(&h.voice.refuse) = Some(VoiceErrorKind::Busy);
    let r = act(&h, "cancel_download", json!({})).await;
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(HubErrorKind::Voice);
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_refused_click_says_why_in_french() {
    let h = start().await;
    show(&h).await;
    *lock(&h.voice.refuse) = Some(VoiceErrorKind::Busy);
    let e = act(&h, "cancel_download", json!({})).await.unwrap_err();
    assert_that!(e.localize(&h.localizer))
        .is_equal_to("La dictée est en cours : réessayez une fois au repos".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_refused_click_resends_the_whole_box() {
    let h = start().await;
    show(&h).await;
    *lock(&h.voice.refuse) = Some(VoiceErrorKind::Busy);
    let mut rx = h.hub.subscribe();
    let _ = act(&h, "model", json!({"id": "large"})).await;
    assert_that!(rx.try_recv()).is_ok().matches(is_box);
}

#[tokio::test(flavor = "multi_thread")]
async fn refresh_unchanged_sends_nothing() {
    let h = start().await;
    show(&h).await;
    let mut rx = h.hub.subscribe();
    refresh(&h).await;
    assert_that!(rx.try_recv()).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn refresh_with_a_new_layout_sends_the_box() {
    let h = start().await;
    show(&h).await;
    lock(&h.voice.view).dictation.as_mut().unwrap().model = "api".into();
    let mut rx = h.hub.subscribe();
    refresh(&h).await;
    assert_that!(rx.try_recv()).is_ok().matches(is_box);
}

fn downloading(received: u64) -> impl Fn(&mut VoiceView) {
    move |v| {
        let dictation = v.dictation.as_mut().unwrap_or_else(|| panic!("vue de référence sans dictée"));
        dictation.download = Some(DownloadView { model: "large".into(), received, total: 4 });
    }
}

#[rstest]
#[case::busy(|v: &mut VoiceView| v.agents.busy = true, "/notice/busy", "Changer d'agent interrompra le tour en cours.")]
#[case::pending(|v: &mut VoiceView| v.agents.pending = Some("local".into()), "/notice/pending", "Passage à Local dès qu'aucune réponse ne tourne.")]
#[tokio::test(flavor = "multi_thread")]
async fn refresh_with_only_new_notices_sends_their_text_as_a_patch(
    #[case] change: fn(&mut VoiceView),
    #[case] path: &str,
    #[case] text: &str,
) {
    let h = start().await;
    show(&h).await;
    change(&mut lock(&h.voice.view));
    let mut rx = h.hub.subscribe();
    refresh(&h).await;
    let e = rx.try_recv().unwrap();
    assert_that!((is_patch(&e), data_of(&e).pointer(path).cloned())).is_equal_to((true, Some(json!(text))));
}

#[tokio::test(flavor = "multi_thread")]
async fn refresh_with_only_new_progress_sends_a_patch() {
    let h = start().await;
    downloading(0)(&mut lock(&h.voice.view));
    show(&h).await;
    downloading(1)(&mut lock(&h.voice.view));
    let mut rx = h.hub.subscribe();
    refresh(&h).await;
    assert_that!(rx.try_recv()).is_ok().matches(is_patch);
}

#[tokio::test(flavor = "multi_thread")]
async fn refresh_when_not_shown_sends_nothing() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    refresh(&h).await;
    assert_that!(rx.try_recv()).is_err();
}

fn api_target() -> ApiTarget {
    ApiTarget {
        host: "api.openai.com".into(),
        url: "https://api.openai.com/v1".into(),
        variables: vec!["OPENAI_API_KEY".into()],
        model: "gpt-4o-transcribe".into(),
    }
}

fn large() -> oku_slides_session::ModelView {
    let models =
        oku_slides_session::voice_fixture::voice_view_fixture().dictation.map(|d| d.models).unwrap_or_default();
    models.into_iter().find(|m| m.id == "large").unwrap_or_else(|| panic!("modèle large absent de la vue de référence"))
}

fn agent_tab(confirmed: bool) -> Origin {
    Origin::Agent { tab: "t-1".into(), confirmed }
}

#[tokio::test(flavor = "multi_thread")]
async fn show_dictation_tool_returns_the_box_id_at_once() {
    let h = start().await;
    let r = call_within(&server(&h).await, "show_dictation", json!({})).await;
    assert_that!(json_of(&r)).is_equal_to(json!({ "shown": "host-voice" }));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_plain_fields_apply_at_once() {
    let h = start().await;
    let r = call_within(&server(&h).await, "set_dictation", json!({"language": "auto", "silence": 2.0})).await;
    assert_that!(json_of(&r)).is_equal_to(json!({ "applied": "now" }));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_plain_fields_apply_unconfirmed_as_this_tab_agent() {
    let h = start().await;
    call_within(&server(&h).await, "set_dictation", json!({"language": "auto", "silence": 2.0})).await;
    let patch = DictationPatch { language: Some(DictationLanguage::Auto), silence: Some(2.0), ..Default::default() };
    assert_that!(*lock(&h.voice.applied)).is_equal_to(vec![(VoiceEvent::Dictation(patch), agent_tab(false))]);
}

#[rstest]
#[case("api.openai.com")]
#[case("https://api.openai.com/v1")]
#[case("OPENAI_API_KEY")]
#[case("gpt-4o-transcribe")]
#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_to_api_confirmation_names_the_target(#[case] part: &str) {
    let h = start().await;
    lock(&h.voice.checked).api = Some(api_target());
    let (text, _, _) = guarded_message(&h, "set_dictation", json!({"model": "api"}), false).await;
    assert_that!(text).contains(part);
}

#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_to_api_confirmation_is_a_danger() {
    let h = start().await;
    lock(&h.voice.checked).api = Some(api_target());
    let (_, danger, _) = guarded_message(&h, "set_dictation", json!({"model": "api"}), false).await;
    assert_that!(danger).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_to_api_declined_applies_nothing() {
    let h = start().await;
    lock(&h.voice.checked).api = Some(api_target());
    guarded_message(&h, "set_dictation", json!({"model": "api", "silence": 3.0}), false).await;
    assert_that!(*lock(&h.voice.applied)).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_to_api_accepted_applies_confirmed() {
    let h = start().await;
    lock(&h.voice.checked).api = Some(api_target());
    guarded_message(&h, "set_dictation", json!({"model": "api"}), true).await;
    let origins: Vec<Origin> = lock(&h.voice.applied).iter().map(|(_, o)| o.clone()).collect();
    assert_that!(origins).is_equal_to(vec![agent_tab(true)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_to_api_target_changed_while_asking_is_refused_unapplied() {
    let h = start().await;
    lock(&h.voice.checked).api = Some(api_target());
    let (_, _, r) = guarded_message_after(&h, "set_dictation", json!({"model": "api"}), true, || {
        lock(&h.voice.checked).api = Some(ApiTarget { url: "https://autre.example/v1".into(), ..api_target() });
    })
    .await;
    assert_that!(text_of(&r)).is_equal_to("Les réglages ont changé depuis la question : redemandez".to_owned());
    assert_that!(lock(&h.voice.applied).is_empty()).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_to_an_uninstalled_model_asks_with_its_size() {
    let h = start().await;
    lock(&h.voice.checked).download = Some((large(), "gpt-4o-transcribe".into()));
    let (text, _, _) = guarded_message(&h, "set_dictation", json!({"model": "large"}), false).await;
    assert_that!(text).is_equal_to("Utiliser Large v3 et le télécharger maintenant (3,1 Go) ?".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_to_an_uninstalled_model_accepted_also_downloads_it() {
    let h = start().await;
    lock(&h.voice.checked).download = Some((large(), "gpt-4o-transcribe".into()));
    guarded_message(&h, "set_dictation", json!({"model": "large"}), true).await;
    let events: Vec<VoiceEvent> = lock(&h.voice.applied).iter().map(|(e, _)| e.clone()).collect();
    assert_that!(events).contains(VoiceEvent::DownloadDictation { model: "large".into() });
}

#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_unchanged_asks_nothing() {
    let h = start().await;
    lock(&h.voice.checked).unchanged = true;
    let r = call_within(&server(&h).await, "set_dictation", json!({"model": "turbo"})).await;
    assert_that!(json_of(&r)).is_equal_to(json!({ "applied": "unchanged" }));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_refused_by_the_port_says_why_in_french() {
    let h = start().await;
    *lock(&h.voice.refuse) = Some(VoiceErrorKind::Busy);
    let r = call_within(&server(&h).await, "set_dictation", json!({"model": "large"})).await;
    assert_that!(text_of(&r)).is_equal_to("La dictée est en cours : réessayez une fois au repos".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_empty_is_refused_in_french() {
    let h = start().await;
    let r = call_within(&server(&h).await, "set_dictation", json!({})).await;
    assert_that!(text_of(&r)).is_equal_to("Rien à changer : donnez au moins un réglage de la dictée.".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_unknown_field_is_refused_without_echoing_it() {
    let h = start().await;
    let r = call_within(&server(&h).await, "set_dictation", json!({"key": "sk-secret-1144"})).await;
    assert_that!(text_of(&r)).does_not_contain("sk-secret-1144");
}

#[rstest]
#[case("api_url")]
#[case("key_env")]
#[case("headers")]
#[case("query")]
#[case("\"url\"")]
#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_schema_has_no_api_target_field(#[case] key: &str) {
    let h = start().await;
    let schema = tool_schema(&server(&h).await, "set_dictation").await.to_string();
    assert_that!(schema).does_not_contain(key);
}

#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_description_first_sentence_reserves_the_api_target() {
    let h = start().await;
    let description = tool_description(&server(&h).await, "set_dictation").await;
    let first = description.split(". ").next().unwrap_or_default().to_owned();
    assert_that!(first).contains("only the user sets");
}

#[tokio::test(flavor = "multi_thread")]
async fn download_dictation_model_confirmation_names_model_and_size() {
    let h = start().await;
    lock(&h.voice.checked).download = Some((large(), "gpt-4o-transcribe".into()));
    let (text, _, _) = guarded_message(&h, "download_dictation_model", json!({"model": "large"}), false).await;
    assert_that!(text).is_equal_to("Utiliser Large v3 et le télécharger maintenant (3,1 Go) ?".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn download_dictation_model_refused_opens_no_dialog() {
    let h = start().await;
    *lock(&h.voice.refuse) = Some(VoiceErrorKind::UnknownModel);
    let r = call_within(&server(&h).await, "download_dictation_model", json!({"model": "nope"})).await;
    assert_that!(text_of(&r)).is_equal_to("Modèle de dictée inconnu".to_owned());
}

#[rstest]
#[case("Bis")]
#[case("fake --bis")]
#[tokio::test(flavor = "multi_thread")]
async fn select_agent_confirmation_names_the_agent_and_its_command(#[case] part: &str) {
    let h = start().await;
    lock(&h.voice.checked).agent =
        Some(AgentPresetView { id: "bis".into(), name: "Bis".into(), hint: Some("fake --bis".into()) });
    let (text, _, _) = guarded_message(&h, "select_agent", json!({"id": "bis"}), false).await;
    assert_that!(text).contains(part);
}

#[tokio::test(flavor = "multi_thread")]
async fn select_agent_confirmed_reports_the_port_answer() {
    let h = start().await;
    lock(&h.voice.checked).agent = Some(AgentPresetView { id: "bis".into(), name: "Bis".into(), hint: None });
    *lock(&h.voice.answer) = Some(VoiceApplied::WhenIdle);
    let (_, _, r) = guarded_message(&h, "select_agent", json!({"id": "bis"}), true).await;
    assert_that!(json_of(&r)).is_equal_to(json!({ "applied": "when_idle" }));
}

#[tokio::test(flavor = "multi_thread")]
async fn select_agent_declined_applies_nothing() {
    let h = start().await;
    lock(&h.voice.checked).agent = Some(AgentPresetView { id: "bis".into(), name: "Bis".into(), hint: None });
    guarded_message(&h, "select_agent", json!({"id": "bis"}), false).await;
    assert_that!(*lock(&h.voice.applied)).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn select_agent_unchanged_goes_through_the_port_without_asking() {
    let h = start().await;
    lock(&h.voice.checked).unchanged = true;
    call_within(&server(&h).await, "select_agent", json!({"id": "faux"})).await;
    assert_that!(*lock(&h.voice.applied)).has_length(1);
}

#[rstest]
#[case(true, 1)]
#[case(false, 0)]
#[tokio::test(flavor = "multi_thread")]
async fn download_agent_model_is_confirmed(#[case] yes: bool, #[case] applied: usize) {
    let h = start().await;
    lock(&h.voice.checked).agent_model_size = Some(2_500_000_000);
    guarded_message(&h, "download_agent_model", json!({}), yes).await;
    assert_that!(*lock(&h.voice.applied)).has_length(applied);
}

#[rstest]
#[case("cancel_dictation_download", VoiceEvent::CancelDictationDownload)]
#[case("cancel_agent_model_download", VoiceEvent::CancelAgentModelDownload)]
#[tokio::test(flavor = "multi_thread")]
async fn cancel_tools_apply_at_once(#[case] tool: &str, #[case] event: VoiceEvent) {
    let h = start().await;
    call_within(&server(&h).await, tool, json!({})).await;
    assert_that!(*lock(&h.voice.applied)).is_equal_to(vec![(event, agent_tab(false))]);
}

#[tokio::test(flavor = "multi_thread")]
async fn cancel_with_nothing_running_says_so_in_french() {
    let h = start().await;
    *lock(&h.voice.refuse) = Some(VoiceErrorKind::NothingToCancel);
    let r = call_within(&server(&h).await, "cancel_dictation_download", json!({})).await;
    assert_that!(text_of(&r)).is_equal_to("Aucun téléchargement n'est en cours".to_owned());
}
