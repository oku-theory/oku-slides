//! The read-aloud section of the box: the switch, the voice, the rate, presentations.
//!
//! Without a voice for the interface language there is no switch: the reason shows instead.
//! Likewise for talking over Oku where echo cancellation is missing.

use oku_slides_a2ui::{Action, ChoiceOption, Component, ComponentKind, Dynamic, TextVariant};
use oku_slides_chart::NumberFormat;
use oku_slides_i18n::Localizer;
use serde_json::{Value, json};

use super::Builder;
use crate::voice_view::{SPEECH_RATES, SpeechStatusView, SpeechView};

/// The value of a rate in the data model: `1`, `1.25`.
fn rate_value(rate: f32) -> String {
    format!("{rate}")
}

/// A switch bound to `path` that sends `name {on}`.
fn switch(label: String, help: Option<String>, path: &str, name: &str) -> ComponentKind {
    ComponentKind::Switch {
        label: Dynamic::Literal(label),
        help: help.map(Dynamic::Literal),
        value: Dynamic::path(path),
        action: Some(Action::event(name, [("on", Dynamic::path(path))])),
    }
}

/// The read-aloud components and their data.
pub(super) fn section(s: &SpeechView, l: &Localizer, b: &mut Builder, data: &mut Value) {
    let SpeechStatusView::Ready { voices } = &s.status else {
        let reason = s.reason().unwrap_or_default();
        let text = l.tr_with("voice-read-aloud-unavailable", &[("reason", reason.into())]);
        b.text("s-unavailable", text, TextVariant::Caption);
        return;
    };
    data["read_aloud"] = json!(s.read_aloud);
    data["speech_voice"] = json!(s.voice.clone().unwrap_or_default());
    data["speech_rate"] = json!(rate_value(s.rate));
    data["speech_in_presentation"] = json!(s.in_presentation);
    data["speech_barge_in"] = json!(s.barge_in);
    let help = Some(l.tr("voice-read-aloud-help"));
    b.push(Component::new("s-read-aloud", switch(l.tr("voice-read-aloud"), help, "/read_aloud", "read_aloud")));

    let mut options = vec![ChoiceOption { id: String::new(), label: l.tr("voice-read-aloud-voice-auto"), hint: None }];
    options.extend(voices.iter().map(|v| ChoiceOption { id: v.id.clone(), label: v.name.clone(), hint: None }));
    let voice = ComponentKind::Select {
        label: Dynamic::Literal(l.tr("voice-read-aloud-voice")),
        help: None,
        value: Dynamic::path("/speech_voice"),
        options,
        action: Some(Action::event("speech_voice", [("id", Dynamic::path("/speech_voice"))])),
    };
    b.push(Component::new("s-voice", voice));

    let mut rates = SPEECH_RATES.to_vec();
    if !rates.iter().any(|r| rate_value(*r) == rate_value(s.rate)) {
        rates.push(s.rate);
    }
    rates.sort_by(f32::total_cmp);
    let numbers = NumberFormat::for_lang(Some(l.locale()));
    let options = rates
        .into_iter()
        .map(|r| {
            let plain = rate_value(r);
            let shown = numbers.localize(&plain).unwrap_or_else(|| plain.clone());
            ChoiceOption {
                id: plain,
                label: l.tr_with("voice-read-aloud-rate-value", &[("value", shown.into())]),
                hint: None,
            }
        })
        .collect();
    let rate = ComponentKind::Select {
        label: Dynamic::Literal(l.tr("voice-read-aloud-rate")),
        help: None,
        value: Dynamic::path("/speech_rate"),
        options,
        action: Some(Action::event("speech_rate", [("value", Dynamic::path("/speech_rate"))])),
    };
    b.push(Component::new("s-rate", rate));
    let presenting = l.tr("voice-read-aloud-in-presentation");
    let path = "/speech_in_presentation";
    b.push(Component::new("s-in-presentation", switch(presenting, None, path, "speech_in_presentation")));
    match &s.barge_in_unavailable {
        None => {
            let help = Some(l.tr("voice-barge-in-help"));
            let barge_in = switch(l.tr("voice-barge-in"), help, "/speech_barge_in", "speech_barge_in");
            b.push(Component::new("s-barge-in", barge_in));
        }
        Some(reason) => {
            let text = l.tr_with("voice-barge-in-unavailable", &[("reason", reason.clone().into())]);
            b.text("s-barge-in-unavailable", text, TextVariant::Caption);
        }
    }
}
