//! La vue de référence de la boîte des réglages, pour les tests d'autres crates.

use crate::voice_view::{AgentPresetView, AgentsView, ApiView, DictationView, ModelKind, ModelView, VoiceView};

/// La vue de référence : Turbo installé, Large à télécharger, Base indisponible, API sans clé.
#[must_use]
pub fn voice_view_fixture() -> VoiceView {
    let model =
        |id: &str, kind, size, installed, available| ModelView { id: id.into(), kind, size, installed, available };
    VoiceView {
        dictation: Some(DictationView {
            model: "turbo".into(),
            models: vec![
                model("turbo", ModelKind::Local, 817_887_676, true, true),
                model("large", ModelKind::Local, 3_090_835_702, false, true),
                model("base", ModelKind::Local, 148_067_931, false, false),
                model("api", ModelKind::Api, 0, false, true),
            ],
            language: None,
            detected: None,
            auto_stop: true,
            silence: 1.5,
            api: ApiView {
                url: "https://api.openai.com/v1".into(),
                model: "gpt-4o-transcribe".into(),
                key_env: "OPENAI_API_KEY".into(),
                key_found: false,
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
        },
    }
}
