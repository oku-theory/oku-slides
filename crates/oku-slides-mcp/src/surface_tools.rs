//! Outils de surface : `ask_*` attend l'utilisateur, `show_*` rend la main tout de suite.

use oku_slides_core::Paragraph;
use oku_slides_session::agent::{
    AskChoice, AskConfirm, AskRewrite, Render, Report, ShowParams, ShowVariants, check_len, forms, render,
};
use rust_mcp_sdk::schema::CallToolResult;
use serde_json::{Value, json};

use crate::McpEvent;
use crate::hub::{Answer, Freeze};
use crate::tools::{Tools, fail, json_result, parse};

impl Tools {
    /// Résultat commun : réponse dictée, abandon, ou `f` pour l'action attendue.
    fn answered(
        answer: Answer,
        f: impl FnOnce(&str, &serde_json::Map<String, Value>) -> CallToolResult,
    ) -> CallToolResult {
        match answer {
            Answer::Cancelled => json_result(&json!({ "cancelled": true })),
            Answer::Action { name, context } if name == "answer" => {
                json_result(&json!({ "answer_text": context.get("text").cloned().unwrap_or(Value::Null) }))
            }
            Answer::Action { name, context } => f(&name, &context),
        }
    }

    pub(crate) async fn ask_choice(&self, args: Value) -> CallToolResult {
        let spec: AskChoice = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        if let Err(e) = check_len("options", spec.options.len(), forms::CHOICES) {
            return fail(e.one_line());
        }
        let id = self.hub.next_id("ask");
        let surface = forms::choice(&id, &spec, &self.localizer);
        let answer = if spec.anchor.is_none() {
            let (question, options, l) = (spec.question.clone(), spec.options.clone(), self.localizer.clone());
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
            self.hub.ask_then(surface, freeze).await
        } else {
            self.hub.ask(surface).await
        };
        Self::answered(answer, |_, ctx| json_result(&json!({ "choice": ctx.get("choice") })))
    }

    pub(crate) async fn ask_confirm(&self, args: Value) -> CallToolResult {
        let spec: AskConfirm = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let surface = forms::confirm(&self.hub.next_id("ask"), &spec, &self.localizer);
        Self::answered(self.hub.ask(surface).await, |_, ctx| {
            json_result(&json!({ "confirmed": ctx.get("confirmed") == Some(&Value::Bool(true)) }))
        })
    }

    pub(crate) async fn ask_rewrite(&self, args: Value) -> CallToolResult {
        let spec: AskRewrite = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let id = self.hub.next_id("ask");
        let built = {
            let spec = spec.clone();
            self.with_session(move |s| s.rewrite_surface(&id, &spec).map_err(|e| e.one_line())).await
        };
        let surface = match built {
            Ok(s) => s,
            Err(e) => return fail(e),
        };
        let (kept, text, paragraphs) = match self.hub.ask(surface).await {
            Answer::Action { name, context } if name == "rewrite" => {
                let kept = context.get("kept").cloned().unwrap_or(Value::Null);
                let Some(text) = context.get("text").and_then(Value::as_str) else {
                    return fail("rewrite answer without text");
                };
                let paragraphs = match context.get("paragraphs") {
                    Some(v) => match serde_json::from_value::<Vec<Paragraph>>(v.clone()) {
                        Ok(p) => Some(p),
                        Err(e) => return fail(format!("invalid paragraphs: {e}")),
                    },
                    None => None,
                };
                (kept, text.to_owned(), paragraphs)
            }
            other => return Self::answered(other, |n, _| fail(format!("unexpected action {n}"))),
        };
        if kept != json!("original") {
            let (spec, t, p) = (spec.clone(), text.clone(), paragraphs.clone());
            let label = self.localizer.tr("agent-undo-rewrite");
            let done = self
                .with_session(move |s| {
                    let ops = s.rewrite_ops(&spec, &t, p).map_err(|e| e.one_line())?;
                    s.apply(&label, ops, None).map_err(|e| e.one_line())
                })
                .await;
            if let Err(e) = done {
                return fail(e);
            }
            self.hub.emit(McpEvent::Changed);
        }
        json_result(&json!({ "kept": kept, "text": text }))
    }

    pub(crate) async fn show_params(&self, args: Value) -> CallToolResult {
        let spec: ShowParams = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let id = self.hub.next_id("show");
        let built = {
            let (spec, id) = (spec.clone(), id.clone());
            self.with_session(move |s| s.params_surface(&id, &spec).map_err(|e| e.one_line())).await
        };
        match built {
            Ok(surface) => {
                self.hub.show_params(surface, spec);
                json_result(&json!({ "shown": id }))
            }
            Err(e) => fail(e),
        }
    }

    pub(crate) async fn show_variants(&self, args: Value) -> CallToolResult {
        let spec: ShowVariants = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let id = self.hub.next_id("show");
        let built = {
            let (spec, id) = (spec.clone(), id.clone());
            self.with_session(move |s| s.variants_surface(&id, &spec).map_err(|e| e.one_line())).await
        };
        match built {
            Ok(surface) => {
                self.hub.show_variants(surface, spec);
                json_result(&json!({ "shown": id }))
            }
            Err(e) => fail(e),
        }
    }

    pub(crate) async fn report(&self, args: Value) -> CallToolResult {
        let spec: Report = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let id = self.hub.next_id("report");
        let built = {
            let (spec, id) = (spec.clone(), id.clone());
            self.with_session(move |s| s.report_surface(&id, &spec).map_err(|e| e.one_line())).await
        };
        match built {
            // Pas de garde `if` qui déplacerait `report` (E0507) : le test se fait dans le bras.
            Ok(report) => {
                if self.hub.show_report(report) {
                    json_result(&json!({ "shown": id }))
                } else {
                    fail("no turn in progress: report is only shown during a turn")
                }
            }
            Err(e) => fail(e),
        }
    }

    pub(crate) async fn render(&self, args: Value) -> CallToolResult {
        let spec: Render = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let id = self.hub.next_id("render");
        match render::render(&id, &spec) {
            Ok(surface) => {
                if self.hub.show_static(surface) {
                    json_result(&json!({ "shown": id }))
                } else {
                    fail("no turn in progress: render is only shown during a turn")
                }
            }
            Err(e) => fail(e.one_line()),
        }
    }
}
