//! Outils de la dictée et de l'agent : la boîte, les réglages, les téléchargements, l'agent.
//!
//! Tout passe par le port du hub, vérifié avant toute question. L'adresse et la variable
//! de l'API ne sont pas des arguments : seul l'utilisateur les règle (spec t-1144).

use oku_slides_i18n::Localizer;
use oku_slides_session::agent::bounds::clip;
use oku_slides_session::{
    DictationLanguage, DictationPatch, HOST_VOICE, ModelKind, ModelView, VoiceEvent, model_label, size_text,
};
use rust_mcp_sdk::schema::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use snafu::{IntoError, ResultExt};

use crate::tool_error::{EmptyDictationChangeSnafu, TaskSnafu, ToolError, VoiceSnafu};
use crate::tools::Tools;
use crate::{ApiTarget, Checked, Origin, VoiceApplied, VoiceError, VoiceErrorKind, VoicePort};

/// Argument keys that would target the API address, variables, query or headers.
const API_TARGET_KEYS: [&str; 9] =
    ["api", "api_url", "url", "api_key_env", "key_env", "api_query", "query", "api_headers", "headers"];

/// Dictation settings to change; absent: unchanged.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SetDictationArgs {
    /// Dictation model: apple | turbo | large | base | api.
    pub model: Option<String>,
    /// Language code (fr, en…) or "auto".
    pub language: Option<String>,
    /// Stop listening on its own after a silence.
    pub auto_stop: Option<bool>,
    /// Seconds of silence before the automatic stop, from 0.5 to 10.
    pub silence: Option<f32>,
    /// Model of the transcription service.
    pub api_model: Option<String>,
}

/// A dictation model.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ModelArg {
    /// Dictation model: apple | turbo | large | base.
    pub model: String,
}

/// An agent of the configuration.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct AgentArg {
    /// Key of the agent.presets table (see the settings tool).
    pub id: String,
}

/// Un refus de sorte `kind`, en erreur d'outil.
fn refusal(kind: VoiceErrorKind) -> ToolError {
    VoiceSnafu.into_error(Box::new(VoiceError::new(kind)))
}

impl Tools {
    /// Ouvre la boîte hors tour ; elle reste jusqu'à ce que l'utilisateur la ferme.
    pub(crate) async fn show_dictation(&self) -> CallToolResult {
        let hub = self.hub.clone();
        let shown = tokio::task::spawn_blocking(move || hub.show_settings()).await.context(TaskSnafu);
        self.reply(shown, |()| self.json_result(&json!({ "shown": HOST_VOICE })))
    }

    /// Change des réglages de la dictée ; passer à l'API ou à un modèle absent est confirmé.
    pub(crate) async fn set_dictation(&self, args: Value) -> CallToolResult {
        // Avant l'analyse : l'agent apprend que ces réglages sont à l'utilisateur, pas « champ inconnu ».
        if args.as_object().is_some_and(|o| API_TARGET_KEYS.iter().any(|k| o.contains_key(*k))) {
            return self.fail_with(&refusal(VoiceErrorKind::ApiReservedToUser));
        }
        let a: SetDictationArgs = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let patch = DictationPatch {
            model: a.model,
            language: a
                .language
                .map(|l| if l == "auto" { DictationLanguage::Auto } else { DictationLanguage::Code(l) }),
            auto_stop: a.auto_stop,
            silence: a.silence,
            api_url: None,
            api_model: a.api_model,
            api_key_env: None,
            api_query: None,
            api_headers: None,
        };
        if patch.is_empty() {
            return self.fail_with(&EmptyDictationChangeSnafu.build());
        }
        let event = VoiceEvent::Dictation(patch);
        let checked = match self.voice_check(event.clone()).await {
            Ok(c) => c,
            Err(e) => return self.fail_with(&e),
        };
        if checked.unchanged {
            return self.json_result(&json!({ "applied": VoiceApplied::Unchanged }));
        }
        let l = self.localizer.current();
        let question = match (&checked.api, &checked.download) {
            (Some(t), _) => Some((api_question(t, &l), true)),
            (None, Some((m, api_model))) => Some((download_question(m, api_model, &l), false)),
            (None, None) => None,
        };
        let confirmed = question.is_some();
        if let Some((message, danger)) = question
            && !self.hub.confirm(message, danger).await
        {
            return self.denied();
        }
        if confirmed {
            // La question a pu durer : ce que l'utilisateur a accepté doit être ce qui sera appliqué.
            match self.voice_check(event.clone()).await {
                Ok(now) if now.api == checked.api && now.download == checked.download => {}
                Ok(_) => return self.fail_with(&refusal(VoiceErrorKind::Changed)),
                Err(e) => return self.fail_with(&e),
            }
        }
        let applied = self.voice_apply(event, confirmed).await;
        match (applied, checked.download) {
            (Ok(_), Some((m, _))) => self.apply_voice(VoiceEvent::DownloadDictation { model: m.id }, true).await,
            (applied, _) => self.reply(applied, |a| self.json_result(&json!({ "applied": a }))),
        }
    }

    /// Télécharge un modèle ou une langue de dictée, après confirmation.
    pub(crate) async fn download_dictation_model(&self, args: Value) -> CallToolResult {
        let a: ModelArg = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let event = VoiceEvent::DownloadDictation { model: a.model };
        let checked = match self.voice_check(event.clone()).await {
            Ok(c) => c,
            Err(e) => return self.fail_with(&e),
        };
        let Some((m, api_model)) = &checked.download else {
            // `check` remplit `download` pour tout modèle qu'il accepte de télécharger.
            tracing::error!("modèle de dictée vérifié sans téléchargement");
            return self.fail_with(&refusal(VoiceErrorKind::Installed));
        };
        if !self.hub.confirm(download_question(m, api_model, &self.localizer.current()), false).await {
            return self.denied();
        }
        self.apply_voice(event, true).await
    }

    /// Arrête le téléchargement d'un modèle de dictée.
    pub(crate) async fn cancel_dictation_download(&self) -> CallToolResult {
        self.checked_then_apply(VoiceEvent::CancelDictationDownload).await
    }

    /// Change l'agent de l'app après confirmation, dès qu'aucune réponse ne tourne.
    pub(crate) async fn select_agent(&self, args: Value) -> CallToolResult {
        let a: AgentArg = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let event = VoiceEvent::SelectAgent { id: a.id };
        let checked = match self.voice_check(event.clone()).await {
            Ok(c) => c,
            Err(e) => return self.fail_with(&e),
        };
        if checked.unchanged {
            return self.apply_voice(event, false).await;
        }
        let Some(preset) = &checked.agent else {
            tracing::error!("agent vérifié sans sa fiche");
            return self.fail_with(&refusal(VoiceErrorKind::UnknownAgent));
        };
        let l = self.localizer.current();
        let name = clip(&preset.name, 80);
        let message = match &preset.hint {
            Some(command) => {
                l.tr_with("agent-confirm-agent", &[("name", name.into()), ("command", clip(command, 120).into())])
            }
            None => l.tr_with("agent-confirm-agent-builtin", &[("name", name.into())]),
        };
        if !self.hub.confirm(message, false).await {
            return self.denied();
        }
        self.apply_voice(event, true).await
    }

    /// Télécharge le modèle de l'agent local, après confirmation.
    pub(crate) async fn download_agent_model(&self) -> CallToolResult {
        let checked = match self.voice_check(VoiceEvent::DownloadAgentModel).await {
            Ok(c) => c,
            Err(e) => return self.fail_with(&e),
        };
        let Some(size) = checked.agent_model_size else {
            tracing::error!("modèle de l'agent vérifié sans sa taille");
            return self.fail_with(&refusal(VoiceErrorKind::NothingToDownload));
        };
        let l = self.localizer.current();
        if !self
            .hub
            .confirm(l.tr_with("agent-confirm-agent-model", &[("size", size_text(size, &l).into())]), false)
            .await
        {
            return self.denied();
        }
        self.apply_voice(VoiceEvent::DownloadAgentModel, true).await
    }

    /// Arrête le téléchargement du modèle de l'agent local.
    pub(crate) async fn cancel_agent_model_download(&self) -> CallToolResult {
        self.checked_then_apply(VoiceEvent::CancelAgentModelDownload).await
    }

    /// Vérifie puis applique sans question.
    async fn checked_then_apply(&self, event: VoiceEvent) -> CallToolResult {
        if let Err(e) = self.voice_check(event.clone()).await {
            return self.fail_with(&e);
        }
        self.apply_voice(event, false).await
    }

    /// Ce qu'une demande changerait, vérifié par le port hors du fil async.
    async fn voice_check(&self, event: VoiceEvent) -> Result<Checked, ToolError> {
        let origin = self.origin(false);
        self.on_port(move |v| v.check(&event, &origin)).await
    }

    /// Applique une demande par le port, hors du fil async.
    async fn voice_apply(&self, event: VoiceEvent, confirmed: bool) -> Result<VoiceApplied, ToolError> {
        let origin = self.origin(confirmed);
        self.on_port(move |v| v.apply(event, origin)).await
    }

    /// Applique et rend `{applied}`.
    async fn apply_voice(&self, event: VoiceEvent, confirmed: bool) -> CallToolResult {
        let done = self.voice_apply(event, confirmed).await;
        self.reply(done, |a| self.json_result(&json!({ "applied": a })))
    }

    /// L'agent de cet onglet ; `confirmed` : l'utilisateur a dit oui à la question.
    fn origin(&self, confirmed: bool) -> Origin {
        Origin::Agent { tab: self.hub.tab().to_owned(), confirmed }
    }

    /// Un appel bloquant au port, hors du fil async.
    async fn on_port<T: Send + 'static>(
        &self,
        f: impl FnOnce(&dyn VoicePort) -> Result<T, VoiceError> + Send + 'static,
    ) -> Result<T, ToolError> {
        let voice = self.hub.voice().clone();
        tokio::task::spawn_blocking(move || f(voice.as_ref()))
            .await
            .context(TaskSnafu)?
            .map_err(|e| VoiceSnafu.into_error(Box::new(e)))
    }
}

/// La question d'un passage à l'API : hôte, adresse, modèle et variables de l'utilisateur.
fn api_question(t: &ApiTarget, l: &Localizer) -> String {
    l.tr_with(
        "agent-confirm-dictation-api",
        &[
            ("host", t.host.as_str().into()),
            ("url", t.url.as_str().into()),
            ("model", t.model.as_str().into()),
            ("var", t.variables.join(", ").into()),
        ],
    )
}

/// La question d'un modèle à télécharger, nommé et pesé.
fn download_question(m: &ModelView, api_model: &str, l: &Localizer) -> String {
    match m.kind {
        ModelKind::Apple => l.tr("agent-confirm-dictation-language"),
        ModelKind::Local | ModelKind::Api => l.tr_with(
            "agent-confirm-dictation-download",
            &[("name", model_label(m, api_model, l).into()), ("size", size_text(m.size, l).into())],
        ),
    }
}
