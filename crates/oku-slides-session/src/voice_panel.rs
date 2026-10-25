//! La boîte « Dictée et agent » de l'hôte, traduite à chaque construction.
//!
//! Ses ids sont fixes. L'avancement et les avis (occupé, en attente) sont des textes
//! toujours présents, liés au modèle de données : leur changement ne touche que les données.

use oku_slides_a2ui::{
    Action, ButtonIcon, ChoiceOption, Component, ComponentKind, Dynamic, ServerMessage, TextVariant,
};
use oku_slides_chart::NumberFormat;
use oku_slides_core::num::size_f64;
use oku_slides_i18n::Localizer;
use serde_json::{Value, json};

use crate::agent::Surface;
use crate::agent::forms::{Place, lit, surface};
use crate::voice_view::{
    AgentsView, DICTATION_LANGUAGES, DictationView, LocalFailure, LocalModelView, ModelKind, ModelView, VoiceView,
};

/// Surface de la boîte des réglages de la dictée et de l'agent.
pub const HOST_VOICE: &str = "host-voice";

/// Les durées de silence proposées, en secondes.
const SILENCES: [f32; 5] = [1.0, 1.5, 2.0, 3.0, 5.0];

/// La boîte : ses composants et son modèle de données.
#[derive(Debug, Clone, PartialEq)]
pub struct VoicePanel {
    /// Les composants, la racine d'abord.
    components: Vec<Component>,
    /// Le modèle de données de la surface.
    data: Value,
}

/// Les composants d'une boîte en construction.
#[derive(Default)]
struct Builder {
    children: Vec<String>,
    components: Vec<Component>,
}

impl Builder {
    /// Ajoute `c` à la colonne racine.
    fn push(&mut self, c: Component) {
        self.children.push(c.id.clone());
        self.components.push(c);
    }

    /// Ajoute un texte de variante `variant`.
    fn text(&mut self, id: &str, text: String, variant: TextVariant) {
        self.push(Component::text(id, text, Some(variant)));
    }

    /// Ajoute un texte lié au chemin `path` du modèle de données.
    fn bound(&mut self, id: &str, path: &str) {
        let kind = ComponentKind::Text { text: Dynamic::path(path), variant: None };
        self.push(Component::new(id, kind));
    }

    /// Ajoute un bouton secondaire qui envoie `action`.
    fn button(&mut self, id: &str, label: String, action: Action) {
        self.push(Component::button(id, label, false, action));
    }
}

impl VoicePanel {
    /// La boîte de `view`, traduite par `l`.
    pub fn build(view: &VoiceView, l: &Localizer) -> Self {
        let mut data = json!({
            "choice": view.agents.active,
            "progress": { "dictation": "", "agent": "" },
            "notice": { "busy": "", "pending": "" },
        });
        let mut b = Builder::default();
        if let Some(d) = &view.dictation {
            dictation(d, l, &mut b, &mut data);
        }
        agents(&view.agents, l, &mut b, &mut data);
        let close = Component::button("close", l.tr("agent-surface-close"), false, Action::named("dismiss"));
        b.push(close.icon(ButtonIcon::Close));
        let refs: Vec<&str> = b.children.iter().map(String::as_str).collect();
        let root = Component::column("root", &refs);
        let components = std::iter::once(root).chain(b.components).collect();
        Self { components, data }
    }

    /// La surface à montrer, dans le fil, vivante.
    #[must_use]
    pub fn surface(&self) -> Surface {
        surface(HOST_VOICE, Place::Thread, None, self.components.clone(), self.data.clone())
    }

    /// Les deux boîtes ont les mêmes composants : seules les données ont pu changer.
    #[must_use]
    pub fn same_layout(&self, other: &Self) -> bool {
        self.components == other.components
    }

    /// Les messages qui remplacent les données de la surface.
    #[must_use]
    pub fn data_messages(&self) -> Vec<ServerMessage> {
        vec![ServerMessage::set_data_model(HOST_VOICE, self.data.clone())]
    }
}

/// Le nom court d'un modèle de dictée ; l'API montre son modèle.
pub fn model_label(model: &ModelView, api_model: &str, l: &Localizer) -> String {
    match model.kind {
        ModelKind::Api => l.tr_with("voice-model-api", &[("model", api_model.into())]),
        ModelKind::Apple | ModelKind::Local => l.tr(&format!("voice-model-{}", model.id)),
    }
}

/// Une taille en Mo (entiers) ou en Go (dixièmes), écrite dans la langue.
pub fn size_text(bytes: u64, l: &Localizer) -> String {
    let mb = (size_f64(bytes) / 1e6).round();
    let numbers = NumberFormat::for_lang(Some(l.locale()));
    if mb >= 1000.0 {
        l.tr_with("voice-size-gb", &[("value", numbers.format((mb / 100.0).round() / 10.0).into())])
    } else {
        l.tr_with("voice-size-mb", &[("value", numbers.format(mb).into())])
    }
}

/// Une durée en secondes, forme la plus courte, écrite dans la langue.
pub fn seconds_text(seconds: f32, l: &Localizer) -> String {
    let plain = secs_value(seconds);
    let value = NumberFormat::for_lang(Some(l.locale())).localize(&plain).unwrap_or(plain);
    l.tr_with("voice-seconds", &[("value", value.into())])
}

/// Des gigaoctets au dixième, écrits dans la langue.
pub fn gigabytes_text(bytes: u64, l: &Localizer) -> String {
    let gb = (size_f64(bytes) / 1e8).round() / 10.0;
    NumberFormat::for_lang(Some(l.locale())).format(gb)
}

/// La valeur d'une durée dans le modèle de données : `1.5`, `3`, `1.2`.
fn secs_value(v: f32) -> String {
    format!("{v}")
}

/// Un nombre entier écrit dans la langue.
fn count_text(n: u64, l: &Localizer) -> String {
    NumberFormat::for_lang(Some(l.locale())).format(size_f64(n))
}

/// Un pourcentage entier écrit dans la langue.
fn percent_text(percent: u64, l: &Localizer) -> String {
    NumberFormat::for_lang(Some(l.locale())).format(size_f64(percent.min(100)))
}

/// L'astuce d'une pastille de modèle : note, taille, état, séparés par ` · `.
fn model_hint(m: &ModelView, d: &DictationView, l: &Localizer) -> String {
    let mut parts = vec![l.tr(&format!("voice-note-{}", m.id))];
    if m.size > 0 {
        parts.push(size_text(m.size, l));
    }
    match m.kind {
        ModelKind::Api if d.api.key_found => parts.push(l.tr("voice-api-ready")),
        ModelKind::Api => {
            let var = d.api.missing.as_deref().unwrap_or(d.api.key_env.as_str());
            parts.push(l.tr_with("voice-api-key-missing", &[("var", var.into())]));
        }
        ModelKind::Local if m.installed => parts.push(l.tr("voice-installed")),
        ModelKind::Local => parts.push(l.tr("voice-not-installed")),
        ModelKind::Apple => {}
    }
    parts.join(" · ")
}

/// Les composants de la dictée.
fn dictation(d: &DictationView, l: &Localizer, b: &mut Builder, data: &mut Value) {
    let chosen = d.models.iter().find(|m| m.id == d.model);
    let apple = d.models.iter().any(|m| m.kind == ModelKind::Apple);
    data["model"] = json!(d.model);
    data["language"] = json!(d.language.clone().unwrap_or_default());
    data["silence"] = json!(if d.auto_stop { secs_value(d.silence) } else { "off".to_owned() });
    data["api"] = json!({
        "url": d.api.url,
        "model": d.api.model,
        "key_env": d.api.key_env,
        "query": d.api.query,
        "headers": d.api.headers,
    });

    b.text("d-title", l.tr("voice-dictation-title"), TextVariant::H4);
    let intro = if apple { "voice-dictation-intro-apple" } else { "voice-dictation-intro" };
    b.text("d-intro", l.tr(intro), TextVariant::Caption);

    let options = d
        .models
        .iter()
        .filter(|m| m.available || m.id == d.model)
        .map(|m| ChoiceOption {
            id: m.id.clone(),
            label: model_label(m, &d.api.model, l),
            hint: Some(model_hint(m, d, l)),
        })
        .collect();
    let chips = ComponentKind::ChoiceChips {
        options,
        value: Dynamic::path("/model"),
        action: Some(Action::event("model", [("id", Dynamic::path("/model"))])),
    };
    b.push(Component::new("d-models", chips));
    let unavailable: Vec<String> =
        d.models.iter().filter(|m| !m.available && m.id != d.model).map(|m| model_label(m, &d.api.model, l)).collect();
    if !unavailable.is_empty() {
        let text = l.tr_with("voice-unavailable", &[("models", unavailable.join(", ").into())]);
        b.text("d-unavailable", text, TextVariant::Caption);
    }

    if d.model == "api" {
        for (field, id, key) in [
            ("url", "d-api-url", "voice-api-url"),
            ("model", "d-api-model", "voice-api-model"),
            ("key_env", "d-api-key-env", "voice-api-key-env"),
            ("query", "d-api-query", "voice-api-query"),
            ("headers", "d-api-headers", "voice-api-headers"),
        ] {
            let path = format!("/api/{field}");
            let action = Action::event("api", [("field", lit(json!(field))), ("value", Dynamic::path(path.as_str()))]);
            let kind = ComponentKind::TextField {
                label: Dynamic::Literal(l.tr(key)),
                value: Dynamic::path(path.as_str()),
                action: Some(action),
            };
            b.push(Component::new(id, kind));
        }
        b.text("d-api-note", l.tr("voice-api-note"), TextVariant::Caption);
        b.text("d-api-azure-note", l.tr("voice-api-azure-note"), TextVariant::Caption);
    }

    b.push(Component::new("d-language", language_select(d, chosen, l)));
    b.push(Component::new("d-silence", silence_select(d, l)));
    b.text("d-note", l.tr("voice-phrases-note"), TextVariant::Caption);

    b.bound("d-progress", "/progress/dictation");
    if let Some(dl) = &d.download {
        let name = d
            .models
            .iter()
            .find(|m| m.id == dl.model)
            .map_or_else(|| dl.model.clone(), |m| model_label(m, &d.api.model, l));
        let percent = dl.received.saturating_mul(100) / dl.total.max(1);
        let text =
            l.tr_with("voice-downloading", &[("name", name.into()), ("percent", percent_text(percent, l).into())]);
        data["progress"]["dictation"] = json!(text);
        b.button("d-cancel", l.tr("voice-cancel"), Action::named("cancel_download"));
    } else if d.idle
        && let Some(m) = chosen.filter(|m| !m.installed)
    {
        let label = match m.kind {
            ModelKind::Local if m.available => {
                Some(l.tr_with("voice-download-now", &[("size", size_text(m.size, l).into())]))
            }
            ModelKind::Apple => Some(l.tr("voice-download-language")),
            ModelKind::Local | ModelKind::Api => None,
        };
        if let Some(label) = label {
            let action = Action::event("download", [("model", lit(json!(m.id)))]);
            b.button("d-download", label, action);
        }
    }
}

/// The list of dictation languages.
///
/// The automatic entry names the language Apple detected.
fn language_select(d: &DictationView, chosen: Option<&ModelView>, l: &Localizer) -> ComponentKind {
    let auto = match d.detected.as_deref().and_then(|c| DICTATION_LANGUAGES.iter().find(|(code, _)| *code == c)) {
        Some((_, name)) if chosen.is_some_and(|m| m.kind == ModelKind::Apple) => {
            l.tr_with("voice-language-auto-resolved", &[("language", (*name).into())])
        }
        _ => l.tr("voice-language-auto"),
    };
    let mut languages = vec![ChoiceOption { id: String::new(), label: auto, hint: None }];
    languages.extend(DICTATION_LANGUAGES.iter().map(|(code, name)| ChoiceOption {
        id: (*code).to_owned(),
        label: (*name).to_owned(),
        hint: None,
    }));
    if let Some(code) = d.language.as_deref().filter(|c| !DICTATION_LANGUAGES.iter().any(|(k, _)| k == c)) {
        languages.push(ChoiceOption { id: code.to_owned(), label: code.to_owned(), hint: None });
    }
    ComponentKind::Select {
        label: Dynamic::Literal(l.tr("voice-language")),
        value: Dynamic::path("/language"),
        options: languages,
        action: Some(Action::event("language", [("code", Dynamic::path("/language"))])),
    }
}

/// The list of silence delays, plus the configured one if missing from it.
fn silence_select(d: &DictationView, l: &Localizer) -> ComponentKind {
    let mut seconds = SILENCES.to_vec();
    if !seconds.iter().any(|s| secs_value(*s) == secs_value(d.silence)) {
        seconds.push(d.silence);
    }
    seconds.sort_by(f32::total_cmp);
    let mut silences = vec![ChoiceOption { id: "off".into(), label: l.tr("voice-silence-off"), hint: None }];
    silences.extend(seconds.into_iter().map(|v| ChoiceOption {
        id: secs_value(v),
        label: seconds_text(v, l),
        hint: None,
    }));
    ComponentKind::Select {
        label: Dynamic::Literal(l.tr("voice-silence")),
        value: Dynamic::path("/silence"),
        options: silences,
        action: Some(Action::event("silence", [("value", Dynamic::path("/silence"))])),
    }
}

/// Les composants des agents.
fn agents(a: &AgentsView, l: &Localizer, b: &mut Builder, data: &mut Value) {
    b.text("a-title", l.tr("voice-agent-title"), TextVariant::H4);
    let options = a
        .presets
        .iter()
        .map(|p| ChoiceOption { id: p.id.clone(), label: p.name.clone(), hint: p.hint.clone() })
        .collect();
    let chips = ComponentKind::ChoiceChips {
        options,
        value: Dynamic::path("/choice"),
        action: (!a.locked).then(|| Action::event("select", [("id", Dynamic::path("/choice"))])),
    };
    b.push(Component::new("a-agents", chips));

    let reason = |size: u64| l.tr_with("agent-local-needs-download", &[("gigabytes", gigabytes_text(size, l).into())]);
    let download = || Action::named("download_agent_model");
    match &a.local {
        None => {}
        Some(LocalModelView::NeedsDownload { size }) => {
            b.text("a-local-reason", reason(*size), TextVariant::Caption);
            b.button("a-download", l.tr("agent-model-download"), download());
        }
        Some(LocalModelView::Running { percent }) => {
            let text =
                l.tr_with("agent-model-downloading", &[("percent", percent_text(u64::from(*percent), l).into())]);
            data["progress"]["agent"] = json!(text);
        }
        Some(LocalModelView::Failed { size, failure }) => {
            b.text("a-local-reason", reason(*size), TextVariant::Caption);
            let error = match failure {
                LocalFailure::Failed => l.tr("agent-model-download-failed"),
                LocalFailure::Integrity => l.tr("agent-model-download-integrity"),
                LocalFailure::Incomplete { got, expected } => l.tr_with(
                    "agent-model-download-incomplete",
                    &[("got", count_text(*got, l).into()), ("expected", count_text(*expected, l).into())],
                ),
                LocalFailure::Cancelled => l.tr("agent-model-download-cancelled"),
            };
            b.text("a-error", error, TextVariant::Caption);
            b.button("a-download", l.tr("agent-model-download"), download());
        }
    }
    b.bound("a-progress", "/progress/agent");
    if matches!(a.local, Some(LocalModelView::Running { .. })) {
        b.button("a-cancel", l.tr("agent-model-cancel"), Action::named("cancel_agent_model_download"));
    }
    b.bound("a-pending", "/notice/pending");
    if let Some(id) = &a.pending {
        let name = a.presets.iter().find(|p| &p.id == id).map_or(id.as_str(), |p| p.name.as_str());
        data["notice"]["pending"] = json!(l.tr_with("voice-agent-pending", &[("name", name.into())]));
    }
    b.bound("a-busy", "/notice/busy");
    if a.busy {
        data["notice"]["busy"] = json!(l.tr("voice-agent-busy"));
    }
    if a.locked {
        b.text("a-locked", l.tr("voice-agent-locked"), TextVariant::Caption);
    }
}

#[cfg(test)]
#[path = "voice_panel_tests.rs"]
pub(crate) mod voice_panel_tests;
