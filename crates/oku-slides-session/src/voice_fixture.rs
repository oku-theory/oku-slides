//! The settings box reference view, for other crates' tests.

use crate::voice_view::{
    AgentPresetView, AgentsView, ApiView, DictationView, ModelKind, ModelView, SpeechStatusView, SpeechView,
    SpeechVoiceView, VoiceView,
};

/// The reference view: Apple unavailable, Parakeet installed and chosen, API without a key.
#[must_use]
pub fn voice_view_fixture() -> VoiceView {
    let model =
        |id: &str, kind, size, installed, available| ModelView { id: id.into(), kind, size, installed, available };
    VoiceView {
        dictation: Some(DictationView {
            model: "parakeet".into(),
            models: vec![
                model("apple", ModelKind::Apple, 0, false, false),
                model("parakeet", ModelKind::Local, 670_479_942, true, true),
                model("api", ModelKind::Api, 0, false, true),
            ],
            language: None,
            detected: None,
            auto_stop: true,
            silence: 1.5,
            hands_free: false,
            api: ApiView {
                url: "https://api.openai.com/v1".into(),
                model: "gpt-4o-transcribe".into(),
                key_env: "OPENAI_API_KEY".into(),
                api_version: String::new(),
                headers: String::new(),
                key_found: false,
                missing: Some("OPENAI_API_KEY".into()),
            },
            download: None,
            idle: true,
        }),
        agents: AgentsView {
            presets: vec![
                AgentPresetView { id: "faux".into(), name: "Faux".into(), hint: Some("fake --script s".into()) },
                AgentPresetView { id: "local".into(), name: "Local".into(), hint: None },
            ],
            active: "faux".into(),
            busy: false,
            locked: false,
            local: None,
            pending: None,
            models: None,
        },
        speech: Some(SpeechView {
            read_aloud: false,
            voice: None,
            rate: 1.0,
            in_presentation: false,
            barge_in: true,
            barge_in_unavailable: None,
            status: SpeechStatusView::Ready {
                voices: vec![SpeechVoiceView { id: "v-fr".into(), name: "Amélie".into() }],
            },
        }),
    }
}
