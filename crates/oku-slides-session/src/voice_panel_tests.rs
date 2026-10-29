use oku_slides_i18n::Localizer;
use rstest::rstest;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use super::*;
use crate::voice_fixture::voice_view_fixture;
use crate::voice_view::*;

fn en() -> Localizer {
    Localizer::new(Some("en"))
}

fn fr() -> Localizer {
    Localizer::new(Some("fr"))
}

fn json_of(p: &VoicePanel) -> Value {
    serde_json::to_value(p.surface()).unwrap()
}

fn components(p: &VoicePanel) -> Vec<Value> {
    let messages = json_of(p)["messages"].as_array().cloned().unwrap();
    messages.iter().find_map(|m| m["updateComponents"]["components"].as_array().cloned()).unwrap()
}

fn ids(p: &VoicePanel) -> Vec<String> {
    components(p).iter().map(|c| c["id"].as_str().unwrap().to_owned()).collect()
}

fn component(p: &VoicePanel, id: &str) -> Value {
    components(p).into_iter().find(|c| c["id"] == id).unwrap()
}

fn data(p: &VoicePanel) -> Value {
    let messages = json_of(p)["messages"].as_array().cloned().unwrap();
    messages.iter().find_map(|m| m.get("updateDataModel").cloned()).unwrap()["value"].clone()
}

fn with(f: impl FnOnce(&mut DictationView)) -> VoiceView {
    let mut v = voice_view_fixture();
    f(v.dictation.as_mut().unwrap());
    v
}

fn agents(f: impl FnOnce(&mut AgentsView)) -> VoiceView {
    let mut v = voice_view_fixture();
    f(&mut v.agents);
    v
}

fn option_ids(c: &Value) -> Vec<String> {
    c["options"].as_array().unwrap().iter().map(|o| o["id"].as_str().unwrap().to_owned()).collect()
}

fn option(p: &VoicePanel, chips: &str, id: &str) -> Value {
    let c = component(p, chips);
    c["options"].as_array().unwrap().iter().find(|o| o["id"] == id).cloned().unwrap()
}

/// Two models offered, `big` in use; `chosen` asked by the app.
fn offered(chosen: Option<&str>) -> AgentModelsView {
    let model = |id: &str, name: &str| AgentModelView { id: id.into(), name: name.into(), hint: None };
    AgentModelsView::Offered {
        models: vec![model("big", "Big"), model("mini", "Mini")],
        current: "big".into(),
        chosen: chosen.map(str::to_owned),
    }
}

/// Les vues qui couvrent toutes les actions de la boîte.
pub(crate) fn covering_views() -> Vec<VoiceView> {
    let api = with(|d| {
        d.model = "api".into();
        d.api.key_found = true;
        d.download = Some(DownloadView { model: "large".into(), received: 1, total: 4 });
    });
    let local = with(|d| {
        d.model = "large".into();
        d.auto_stop = false;
        d.silence = 2.5;
    });
    let mut agent = voice_view_fixture();
    agent.agents.locked = true;
    agent.agents.busy = true;
    agent.agents.pending = Some("local".into());
    agent.agents.local = Some(LocalModelView::Failed { size: 2_500_000_000, failure: LocalFailure::Integrity });
    let mut running = voice_view_fixture();
    running.agents.local = Some(LocalModelView::Running { percent: 40 });
    running.agents.models = Some(offered(Some("mini")));
    vec![api, local, agent, running]
}

#[test]
fn voice_panel_surface_is_live_outside_any_turn() {
    let s = VoicePanel::build(&voice_view_fixture(), &en()).surface();
    assert_that!((s.id.as_str(), s.live, s.turn)).is_equal_to((HOST_VOICE, true, None));
}

#[test]
fn voice_panel_ends_with_a_close_button() {
    assert_that!(ids(&VoicePanel::build(&voice_view_fixture(), &en())).last().cloned())
        .is_equal_to(Some("close".to_owned()));
}

#[test]
fn voice_panel_close_button_sends_dismiss() {
    let close = component(&VoicePanel::build(&voice_view_fixture(), &en()), "close");
    assert_that!(close["action"]["event"]["name"].clone()).is_equal_to(json!("dismiss"));
}

#[test]
fn voice_panel_unavailable_model_is_not_a_chip() {
    let p = VoicePanel::build(&voice_view_fixture(), &en());
    assert_that!(option_ids(&component(&p, "d-models"))).is_equal_to(vec![
        "turbo".to_owned(),
        "large".to_owned(),
        "api".to_owned(),
    ]);
}

#[test]
fn voice_panel_unavailable_model_is_named_below_the_chips() {
    let p = VoicePanel::build(&voice_view_fixture(), &fr());
    assert_that!(component(&p, "d-unavailable")["text"].clone())
        .is_equal_to(json!("Indisponible pour cette langue : Base"));
}

#[test]
fn voice_panel_chosen_unavailable_model_stays_a_chip() {
    let p = VoicePanel::build(&with(|d| d.model = "base".into()), &en());
    assert_that!(option_ids(&component(&p, "d-models"))).contains("base".to_owned());
}

#[test]
fn voice_panel_model_chip_label_is_the_short_name() {
    let p = VoicePanel::build(&voice_view_fixture(), &fr());
    assert_that!(option(&p, "d-models", "large")["label"].clone()).is_equal_to(json!("Large v3"));
}

#[test]
fn voice_panel_model_chip_hint_names_the_size() {
    let p = VoicePanel::build(&voice_view_fixture(), &fr());
    assert_that!(option(&p, "d-models", "large")["hint"].as_str().unwrap_or("")).contains("3,1 Go");
}

#[rstest]
#[case(false, "OPENAI_API_KEY")]
#[case(true, "Clé trouvée")]
fn voice_panel_api_chip_hint_says_whether_the_key_is_set(#[case] found: bool, #[case] text: &str) {
    let p = VoicePanel::build(&with(|d| d.api.key_found = found), &fr());
    assert_that!(option(&p, "d-models", "api")["hint"].as_str().unwrap_or("")).contains(text);
}

#[rstest]
#[case("d-api-url")]
#[case("d-api-model")]
#[case("d-api-key-env")]
#[case("d-api-version")]
#[case("d-api-headers")]
#[case("d-api-azure-note")]
fn voice_panel_api_model_shows_its_field(#[case] id: &str) {
    assert_that!(ids(&VoicePanel::build(&with(|d| d.model = "api".into()), &en()))).contains(id.to_owned());
}

#[test]
fn voice_panel_api_hint_names_the_missing_header_variable() {
    let p = VoicePanel::build(&with(|d| d.api.missing = Some("AZURE_OPENAI_API_KEY".into())), &fr());
    assert_that!(option(&p, "d-models", "api")["hint"].as_str().unwrap_or("")).contains("AZURE_OPENAI_API_KEY");
}

#[test]
fn voice_panel_local_model_hides_the_api_fields() {
    assert_that!(ids(&VoicePanel::build(&voice_view_fixture(), &en()))).does_not_contain("d-api-url".to_owned());
}

#[rstest]
#[case(true, 1.5, "1.5")]
#[case(false, 1.5, "off")]
#[case(true, 2.5, "2.5")]
#[case(true, 1.2, "1.2")]
fn voice_panel_silence_value_is_the_short_seconds(#[case] on: bool, #[case] secs: f32, #[case] value: &str) {
    let v = with(|d| {
        d.auto_stop = on;
        d.silence = secs;
    });
    assert_that!(data(&VoicePanel::build(&v, &en()))["silence"].clone()).is_equal_to(json!(value));
}

#[test]
fn voice_panel_custom_silence_is_offered_in_order() {
    let p = VoicePanel::build(&with(|d| d.silence = 2.5), &en());
    let expected: Vec<String> = ["off", "1", "1.5", "2", "2.5", "3", "5"].into_iter().map(String::from).collect();
    assert_that!(option_ids(&component(&p, "d-silence"))).is_equal_to(expected);
}

#[rstest]
#[case(1.5, "1,5 s")]
#[case(1.2, "1,2 s")]
fn seconds_text_is_short_with_the_french_comma(#[case] secs: f32, #[case] text: &str) {
    assert_that!(seconds_text(secs, &fr())).is_equal_to(text.to_owned());
}

#[test]
fn voice_panel_uninstalled_chosen_model_offers_its_download() {
    let p = VoicePanel::build(&with(|d| d.model = "large".into()), &fr());
    assert_that!(component(&p, "d-download")["label"].clone()).is_equal_to(json!("Télécharger maintenant (3,1 Go)"));
}

#[test]
fn voice_panel_installed_chosen_model_offers_no_download() {
    assert_that!(ids(&VoicePanel::build(&voice_view_fixture(), &en()))).does_not_contain("d-download".to_owned());
}

#[test]
fn voice_panel_busy_dictation_offers_no_download() {
    let v = with(|d| {
        d.model = "large".into();
        d.idle = false;
    });
    assert_that!(ids(&VoicePanel::build(&v, &en()))).does_not_contain("d-download".to_owned());
}

#[test]
fn voice_panel_progress_is_bound_to_the_data_model() {
    let p = VoicePanel::build(&voice_view_fixture(), &en());
    assert_that!(component(&p, "d-progress")["text"].clone()).is_equal_to(json!({ "path": "/progress/dictation" }));
}

#[test]
fn voice_panel_downloading_puts_the_percent_in_the_data_model() {
    let v = with(|d| d.download = Some(DownloadView { model: "large".into(), received: 1, total: 4 }));
    assert_that!(data(&VoicePanel::build(&v, &fr()))["progress"]["dictation"].clone())
        .is_equal_to(json!("Téléchargement de Large v3 · 25 %"));
}

#[test]
fn voice_panel_progress_change_keeps_the_layout() {
    let at = |received| {
        VoicePanel::build(
            &with(|d| d.download = Some(DownloadView { model: "large".into(), received, total: 4 })),
            &en(),
        )
    };
    assert_that!(at(1).same_layout(&at(2))).is_true();
}

#[rstest]
#[case::busy(|a: &mut AgentsView| a.busy = true)]
#[case::pending(|a: &mut AgentsView| a.pending = Some("local".into()))]
fn voice_panel_notice_change_keeps_the_layout(#[case] change: fn(&mut AgentsView)) {
    let a = VoicePanel::build(&voice_view_fixture(), &en());
    let b = VoicePanel::build(&agents(change), &en());
    assert_that!(a.same_layout(&b)).is_true();
}

#[test]
fn voice_panel_model_change_changes_the_layout() {
    let a = VoicePanel::build(&voice_view_fixture(), &en());
    let b = VoicePanel::build(&with(|d| d.model = "api".into()), &en());
    assert_that!(a.same_layout(&b)).is_false();
}

#[test]
fn voice_panel_data_messages_are_one_root_data_update() {
    let p = VoicePanel::build(&voice_view_fixture(), &en());
    let messages = serde_json::to_value(p.data_messages()).unwrap();
    assert_that!(messages[0]["updateDataModel"]["value"].clone()).is_equal_to(data(&p));
}

#[test]
fn voice_panel_busy_text_is_in_the_data_model() {
    let p = VoicePanel::build(&agents(|a| a.busy = true), &fr());
    assert_that!(data(&p)["notice"]["busy"].clone())
        .is_equal_to(json!("Changer d'agent interrompra le tour en cours."));
}

#[test]
fn voice_panel_pending_text_names_the_agent() {
    let p = VoicePanel::build(&agents(|a| a.pending = Some("local".into())), &fr());
    assert_that!(data(&p)["notice"]["pending"].clone())
        .is_equal_to(json!("Passage à Local dès qu'aucune réponse ne tourne."));
}

#[test]
fn voice_panel_apple_auto_language_names_the_detected_one() {
    let v = with(|d| {
        d.model = "apple".into();
        d.models.insert(
            0,
            ModelView { id: "apple".into(), kind: ModelKind::Apple, size: 0, installed: true, available: true },
        );
        d.detected = Some("fr".into());
    });
    let options = component(&VoicePanel::build(&v, &en()), "d-language")["options"].clone();
    assert_that!(options[0]["label"].as_str().unwrap_or("")).contains("Français");
}

#[test]
fn voice_panel_unlisted_language_is_still_an_option() {
    let p = VoicePanel::build(&with(|d| d.language = Some("ja".into())), &en());
    assert_that!(option_ids(&component(&p, "d-language"))).contains("ja".to_owned());
}

#[rstest]
#[case(LocalModelView::NeedsDownload { size: 2_500_000_000 }, "a-download")]
#[case(LocalModelView::Running { percent: 40 }, "a-cancel")]
#[case(LocalModelView::Failed { size: 2_500_000_000, failure: LocalFailure::Integrity }, "a-error")]
fn voice_panel_local_agent_state_shows_its_component(#[case] local: LocalModelView, #[case] id: &str) {
    assert_that!(ids(&VoicePanel::build(&agents(|a| a.local = Some(local)), &en()))).contains(id.to_owned());
}

#[test]
fn voice_panel_local_agent_reason_formats_gigabytes_in_french() {
    let p =
        VoicePanel::build(&agents(|a| a.local = Some(LocalModelView::NeedsDownload { size: 2_500_000_000 })), &fr());
    assert_that!(component(&p, "a-local-reason")["text"].as_str().unwrap_or("")).contains("2,5");
}

#[rstest]
#[case(false, true)]
#[case(true, false)]
fn voice_panel_agent_chips_send_select_unless_locked(#[case] locked: bool, #[case] sends: bool) {
    let chips = component(&VoicePanel::build(&agents(|a| a.locked = locked), &en()), "a-agents");
    assert_that!(chips.get("action").is_some()).is_equal_to(sends);
}

#[test]
fn voice_panel_builtin_agent_chip_has_no_hint() {
    let p = VoicePanel::build(&voice_view_fixture(), &en());
    assert_that!(option(&p, "a-agents", "local").get("hint").cloned()).is_none();
}

#[test]
fn voice_panel_without_dictation_has_no_dictation_component() {
    let mut v = voice_view_fixture();
    v.dictation = None;
    assert_that!(ids(&VoicePanel::build(&v, &en())).iter().filter(|i| i.starts_with("d-")).count()).is_equal_to(0);
}

#[test]
fn voice_panel_french_title_is_translated() {
    let p = VoicePanel::build(&voice_view_fixture(), &fr());
    assert_that!(component(&p, "d-title")["text"].clone()).is_equal_to(json!("Modèle de reconnaissance"));
}

#[rstest]
#[case(818_000_000, "818 MB")]
#[case(999_600_000, "1 GB")]
#[case(3_090_835_702, "3.1 GB")]
fn size_text_rounds_to_mb_or_tenths_of_gb(#[case] bytes: u64, #[case] text: &str) {
    assert_that!(size_text(bytes, &en())).is_equal_to(text.to_owned());
}

#[test]
fn size_text_french_uses_go_and_a_comma() {
    assert_that!(size_text(3_090_835_702, &fr())).is_equal_to("3,1 Go".to_owned());
}

/// Chaque action de la boîte, résolue contre son modèle de données, redonne un évènement.
#[test]
fn voice_panel_every_action_parses_back_into_an_event() {
    let mut seen = std::collections::BTreeSet::new();
    for view in covering_views() {
        let p = VoicePanel::build(&view, &en());
        let model = data(&p);
        for c in components(&p) {
            let Some(event) = c.get("action").and_then(|a| a.get("event")) else { continue };
            let name = event["name"].as_str().unwrap().to_owned();
            if name == "dismiss" {
                continue;
            }
            let empty = Map::new();
            let ctx: Map<String, Value> = event
                .get("context")
                .and_then(Value::as_object)
                .unwrap_or(&empty)
                .iter()
                .map(|(k, v)| match v.get("path").and_then(Value::as_str) {
                    Some(path) => (k.clone(), model.pointer(path).cloned().unwrap_or(Value::Null)),
                    None => (k.clone(), v.clone()),
                })
                .collect();
            assert_that!(voice_event(&name, &ctx)).named(&name).is_some();
            seen.insert(name);
        }
    }
    let expected: std::collections::BTreeSet<String> = [
        "model",
        "language",
        "silence",
        "api",
        "download",
        "cancel_download",
        "select",
        "download_agent_model",
        "cancel_agent_model_download",
        "agent_model",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    assert_that!(seen).is_equal_to(expected);
}

#[test]
fn voice_panel_agent_without_model_choice_shows_no_model() {
    let p = VoicePanel::build(&voice_view_fixture(), &en());
    assert_that!(ids(&p).iter().filter(|id| id.starts_with("a-model")).count()).is_equal_to(0);
}

#[test]
fn voice_panel_offered_models_are_the_options_of_the_model_select() {
    let p = VoicePanel::build(&agents(|a| a.models = Some(offered(None))), &en());
    assert_that!(option_ids(&component(&p, "a-model"))).is_equal_to(vec!["big".to_owned(), "mini".to_owned()]);
}

#[rstest]
#[case::none(None, "big")]
#[case::offered(Some("mini"), "mini")]
#[case::not_offered(Some("gone"), "big")]
fn voice_panel_model_select_shows_the_chosen_offered_model(#[case] chosen: Option<&str>, #[case] shown: &str) {
    let p = VoicePanel::build(&agents(|a| a.models = Some(offered(chosen))), &en());
    assert_that!(data(&p)["agent_model"].clone()).is_equal_to(json!(shown));
}

#[test]
fn voice_panel_chosen_model_not_offered_is_named_in_french() {
    let p = VoicePanel::build(&agents(|a| a.models = Some(offered(Some("gone")))), &fr());
    assert_that!(component(&p, "a-model-missing")["text"].as_str().unwrap_or("")).contains("gone");
}

#[test]
fn voice_panel_model_choice_change_keeps_the_layout() {
    let a = VoicePanel::build(&agents(|a| a.models = Some(offered(None))), &en());
    let b = VoicePanel::build(&agents(|a| a.models = Some(offered(Some("mini")))), &en());
    assert_that!(a.same_layout(&b)).is_true();
}

#[rstest]
#[case::nothing_asked(None, "voice-agent-model-later")]
#[case::asked(Some("gpt-5"), "voice-agent-model-asked")]
fn voice_panel_unknown_models_show_a_note_without_select(#[case] chosen: Option<&str>, #[case] key: &str) {
    let v = agents(|a| a.models = Some(AgentModelsView::Unknown { chosen: chosen.map(str::to_owned) }));
    let p = VoicePanel::build(&v, &fr());
    let expected = fr().tr_with(key, &[("model", chosen.unwrap_or_default().into())]);
    assert_that!(ids(&p)).does_not_contain("a-model".to_owned());
    assert_that!(component(&p, "a-model-note")["text"].clone()).is_equal_to(json!(expected));
}
