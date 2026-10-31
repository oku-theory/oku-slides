//! Outils de surface : `ask_*` attend l'utilisateur, `show_*` rend la main tout de suite.

use oku_slides_core::Paragraph;
use oku_slides_session::agent::{
    Anchor, AskChoice, AskConfirm, AskRewrite, Render, Report, ShowParams, ShowVariants, check_len, check_options,
    forms, render,
};
use oku_slides_session::{HOST_PACKS, HOST_THEME_PICKER};
use rust_mcp_sdk::schema::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use snafu::{IntoError, ResultExt};

use crate::McpEvent;
use crate::hub::{Answer, Freeze};
use crate::tool_error::{
    HubSnafu, NoTurnSnafu, PacksDeckModeSnafu, ParagraphsSnafu, RewriteTextSnafu, SessionSnafu, TaskSnafu,
    ThemesDeckModeSnafu, ToolError, TurnOverSnafu, UnexpectedActionSnafu,
};
use crate::tools::Tools;

/// `show_themes`: the slide that each theme shows.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ShowThemesArgs {
    /// Id of the slide to render in each theme; absent: the first one.
    ///
    /// An unknown slide is refused.
    #[serde(default)]
    pub slide: Option<String>,
}

impl Tools {
    /// Refuses a thread tool called outside a turn, or whose turn closed before showing.
    ///
    /// Used by `report`, `render` and `show_usage`.
    pub(crate) fn no_turn(&self, tool: &'static str) -> CallToolResult {
        self.fail_with(&NoTurnSnafu { tool }.build())
    }

    /// Résultat commun : réponse dictée, abandon, ou `f` pour l'action attendue.
    fn answered(
        &self,
        answer: Answer,
        f: impl FnOnce(&str, &serde_json::Map<String, Value>) -> CallToolResult,
    ) -> CallToolResult {
        match answer {
            Answer::Cancelled => self.json_result(&json!({ "cancelled": true })),
            Answer::Action { name, context } if name == "answer" => {
                self.json_result(&json!({ "answer_text": context.get("text").cloned().unwrap_or(Value::Null) }))
            }
            Answer::Action { name, context } => f(&name, &context),
        }
    }

    /// Une ancre de l'agent, résolue contre le deck actif.
    async fn check_anchor(&self, anchor: &Anchor) -> Result<(), ToolError> {
        let anchor = anchor.clone();
        self.with_session(move |s| s.check_anchor(&anchor)).await
    }

    /// `turn` : le tour de l'appel (`Tools::call`), où la question sans ancre est rangée.
    pub(crate) async fn ask_choice(&self, turn: Option<u32>, args: Value) -> CallToolResult {
        let spec: AskChoice = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        if let Err(e) = check_len("options", spec.options.len(), forms::CHOICES) {
            return self.fail_with(&SessionSnafu.into_error(e));
        }
        if let Err(e) = check_options("options", &spec.options) {
            return self.fail_with(&SessionSnafu.into_error(e));
        }
        if let Some(anchor) = &spec.anchor
            && let Err(e) = self.check_anchor(anchor).await
        {
            return self.fail_with(&e);
        }
        let id = self.hub.next_id("ask");
        let surface = forms::choice(&id, &spec, &self.localizer.current());
        let answer = if spec.anchor.is_none() {
            let (question, options, l) = (spec.question.clone(), spec.options.clone(), self.localizer.current());
            let freeze: Freeze = Box::new(move |a| {
                let label = match a {
                    Answer::Action { name, context } if name == "choose" => context
                        .get("choice")
                        .and_then(Value::as_str)
                        .and_then(|c| options.iter().find(|o| o.id == c))
                        .map(|o| o.label.clone()),
                    Answer::Action { name, context } if name == "answer" => {
                        context.get("text").and_then(Value::as_str).map(str::to_owned)
                    }
                    _ => None,
                };
                forms::frozen_choice(&id, &question, label.as_deref(), &l)
            });
            self.hub.ask_then(turn, surface, freeze).await
        } else {
            self.hub.ask(surface).await
        };
        self.answered(answer, |_, ctx| self.json_result(&json!({ "choice": ctx.get("choice") })))
    }

    pub(crate) async fn ask_confirm(&self, args: Value) -> CallToolResult {
        let spec: AskConfirm = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let surface = forms::confirm(&self.hub.next_id("ask"), &spec, &self.localizer.current());
        self.answered(self.hub.ask(surface).await, |_, ctx| {
            self.json_result(&json!({ "confirmed": ctx.get("confirmed") == Some(&Value::Bool(true)) }))
        })
    }

    /// Propose des reformulations d'un texte ; la réponse est appliquée par l'hôte.
    pub(crate) async fn ask_rewrite(&self, args: Value) -> CallToolResult {
        let spec: AskRewrite = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let id = self.hub.next_id("ask");
        let built = {
            let spec = spec.clone();
            self.with_session(move |s| s.rewrite_surface(&id, &spec)).await
        };
        let surface = match built {
            Ok(s) => s,
            Err(e) => return self.fail_with(&e),
        };
        let (kept, text, paragraphs) = match self.hub.ask(surface).await {
            Answer::Action { name, context } if name == "rewrite" => {
                let kept = context.get("kept").cloned().unwrap_or(Value::Null);
                let Some(text) = context.get("text").and_then(Value::as_str) else {
                    return self.fail_with(&RewriteTextSnafu.build());
                };
                let paragraphs = match context.get("paragraphs") {
                    Some(v) => match serde_json::from_value::<Vec<Paragraph>>(v.clone()).context(ParagraphsSnafu) {
                        Ok(p) => Some(p),
                        Err(e) => return self.fail_with(&e),
                    },
                    None => None,
                };
                (kept, text.to_owned(), paragraphs)
            }
            other => {
                return self.answered(other, |n, _| self.fail_with(&UnexpectedActionSnafu { action: n }.build()));
            }
        };
        if kept != json!("original") {
            let (spec, t, p) = (spec.clone(), text.clone(), paragraphs.clone());
            let label = self.localizer.current().tr("agent-undo-rewrite");
            let done = self
                .with_session(move |s| {
                    let ops = s.rewrite_ops(&spec, &t, p)?;
                    s.apply(&label, ops, None)
                })
                .await;
            if let Err(e) = done {
                return self.fail_with(&e);
            }
            self.hub.emit(McpEvent::Changed);
        }
        self.json_result(&json!({ "kept": kept, "text": text }))
    }

    pub(crate) async fn show_params(&self, turn: Option<u32>, args: Value) -> CallToolResult {
        let spec: ShowParams = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let id = self.hub.next_id("show");
        let built = {
            let (spec, id) = (spec.clone(), id.clone());
            self.with_session(move |s| s.params_surface(&id, &spec)).await
        };
        self.reply(built, |surface| {
            if self.hub.show_params(turn, surface, spec) {
                self.json_result(&json!({ "shown": id }))
            } else {
                self.fail_with(&TurnOverSnafu.build())
            }
        })
    }

    pub(crate) async fn show_variants(&self, args: Value) -> CallToolResult {
        let spec: ShowVariants = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let id = self.hub.next_id("show");
        let built = {
            let (spec, id) = (spec.clone(), id.clone());
            self.with_session(move |s| s.variants_surface(&id, &spec)).await
        };
        self.reply(built, |surface| {
            self.hub.show_variants(surface, spec);
            self.json_result(&json!({ "shown": id }))
        })
    }

    /// Le sélecteur de thème de l'hôte, ouvert hors tour : il reste après la fin du tour.
    ///
    /// Comme `show_variants`, l'appel n'est rangé dans aucun tour : un appel tardif l'ouvre aussi.
    ///
    /// Les miniatures se rendent sous le verrou de session : hors du fil async.
    pub(crate) async fn show_themes(&self, args: Value) -> CallToolResult {
        let spec: ShowThemesArgs = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        // Une slide inconnue est refusée : le sélecteur ne retombe jamais en silence sur la première.
        if let Some(id) = spec.slide.clone()
            && let Err(e) = self.with_session(move |s| s.check_slide(&id)).await
        {
            return self.fail_with(&e);
        }
        let hub = self.hub.clone();
        let shown = tokio::task::spawn_blocking(move || hub.show_themes(spec.slide.as_deref()))
            .await
            .context(TaskSnafu)
            .and_then(|r| r.map_err(Box::new).context(HubSnafu));
        self.reply(shown, |shown| {
            if shown {
                self.json_result(&json!({ "shown": HOST_THEME_PICKER }))
            } else {
                self.fail_with(&ThemesDeckModeSnafu.build())
            }
        })
    }

    /// Le panneau des packs du deck, ouvert hors tour comme le sélecteur de thème.
    ///
    /// Il lit la bibliothèque sous le verrou de session : hors du fil async.
    pub(crate) async fn show_packs(&self) -> CallToolResult {
        let hub = self.hub.clone();
        let shown = tokio::task::spawn_blocking(move || hub.show_packs())
            .await
            .context(TaskSnafu)
            .and_then(|r| r.map_err(Box::new).context(HubSnafu));
        self.reply(shown, |shown| {
            if shown {
                self.json_result(&json!({ "shown": HOST_PACKS }))
            } else {
                self.fail_with(&PacksDeckModeSnafu.build())
            }
        })
    }

    pub(crate) async fn report(&self, turn: Option<u32>, args: Value) -> CallToolResult {
        let spec: Report = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let Some(turn) = turn else { return self.no_turn("report") };
        let id = self.hub.next_id("report");
        let built = {
            let (spec, id) = (spec.clone(), id.clone());
            self.with_session(move |s| s.report_surface(&id, &spec)).await
        };
        self.reply(built, |report| {
            if self.hub.show_report(turn, report) {
                self.json_result(&json!({ "shown": id }))
            } else {
                self.no_turn("report")
            }
        })
    }

    pub(crate) fn render(&self, turn: Option<u32>, args: Value) -> CallToolResult {
        let spec: Render = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let Some(turn) = turn else { return self.no_turn("render") };
        let id = self.hub.next_id("render");
        let built: Result<_, ToolError> = render::render(&id, &spec).context(SessionSnafu);
        self.reply(built, |surface| {
            if self.hub.show_static(turn, surface) {
                self.json_result(&json!({ "shown": id }))
            } else {
                self.no_turn("render")
            }
        })
    }
}
