//! Apple engine: Foundation Models through `oku-slides-apple-model`, one guided reply per completion.
//!
//! Route asks for `{families}` (`apple_schema::route`). Act, to fit the 4,096-token context,
//! takes up to three replies (`apple_steps`): pick a tool by name, pick the kinds of its
//! largest union if its schema is too big, then write the call `{tool, arguments}` with
//! that tool's schema alone (`apple_schema::call`). Each blocking reply runs on
//! `spawn_blocking`; the turn's token cancels it. A prompt that still overflows the context
//! is an `EngineError`: the turn ends with its failure report.

use async_trait::async_trait;
use oku_slides_apple_model::{ErrorKind, Request, respond};
use oku_slides_platform::config::LocalAgentSettings;
use tokio_util::sync::CancellationToken;

use serde_json::Value;

use crate::apple_prompt::{Rendered, render, render_call};
use crate::apple_schema::{call, route};
use crate::apple_steps::{FILL_BYTES, Pick, kinds, narrow, parse_kinds, parse_pick, pick, tagged_union};
use crate::engine::{CancelledSnafu, Completion, Engine, EngineError, FailedSnafu, Reply, Step};
use crate::error::{LocalAgentError, SettingsSnafu};

/// Apple's on-device model; it holds nothing but its sampling settings.
#[derive(Debug)]
pub(crate) struct AppleEngine {
    temperature: f64,
    max_tokens: u32,
}

impl AppleEngine {
    /// The engine for these settings; refuses a temperature that is not a finite number, 0 or more.
    pub(crate) fn new(settings: &LocalAgentSettings) -> Result<Self, LocalAgentError> {
        if !settings.temperature.is_finite() || settings.temperature < 0.0 {
            return Err(SettingsSnafu { message: "local_agent.temperature must be a finite number, 0 or more" }
                .build()
                .into());
        }
        Ok(Self { temperature: f64::from(settings.temperature), max_tokens: settings.max_reply_tokens })
    }
}

fn failed(e: impl std::fmt::Display) -> EngineError {
    FailedSnafu { message: e.to_string() }.build().into()
}

#[async_trait]
impl Engine for AppleEngine {
    async fn complete(&self, c: &Completion, cancel: &CancellationToken) -> Result<Reply, EngineError> {
        match c.step {
            // Routing is a choice among keys: no sampling.
            Step::Route => {
                let json = self.ask(render(c), route(&c.offered), 0.0, cancel).await?;
                Ok(Reply::Text(json))
            }
            Step::Act => self.act(c, cancel).await,
        }
    }
}

impl AppleEngine {
    /// Pick a tool, narrow its schema if it is too big, then write its call (`apple_steps`).
    async fn act(&self, c: &Completion, cancel: &CancellationToken) -> Result<Reply, EngineError> {
        let tool = match c.tools.as_slice() {
            [] => return Ok(Reply::Invalid("no tool to call".to_owned())),
            [one] => one,
            tools => {
                let json = self.ask(render(c), pick(tools), 0.0, cancel).await?;
                let picked = match parse_pick(&json, tools) {
                    Pick::Tool(name) => tools.iter().find(|t| t.name == name),
                    Pick::Invalid(reason) => return Ok(Reply::Invalid(reason)),
                };
                // `parse_pick` names only a tool of `tools`.
                let Some(tool) = picked else { return Ok(Reply::Invalid("unknown tool".to_owned())) };
                tool
            }
        };
        let name = &tool.name;
        let mut schema = call(tool);
        if schema.to_string().len() > FILL_BYTES
            && let Some((union, tags)) = tagged_union(&schema)
        {
            let short = union.rsplit("_defs_").next().unwrap_or(&union);
            let ask = format!("You call {name}. Pick the 1 to 3 kinds of {short} this call needs.");
            let json = self.ask(render_call(c, tool, &ask), kinds(&tags), 0.0, cancel).await?;
            schema = narrow(&schema, &union, &parse_kinds(&json, &tags));
        }
        let ask = format!("Call {name}: write its arguments.");
        let json = self.ask(render_call(c, tool, &ask), schema, self.temperature, cancel).await?;
        Ok(crate::apple_reply::parse(&json))
    }

    /// One guided reply on `spawn_blocking`, cancelled through `cancel`.
    async fn ask(
        &self,
        rendered: Rendered,
        schema: Value,
        temperature: f64,
        cancel: &CancellationToken,
    ) -> Result<String, EngineError> {
        if cancel.is_cancelled() {
            return Err(CancelledSnafu.build().into());
        }
        let (schema, max_tokens, token) = (schema.to_string(), self.max_tokens, cancel.clone());
        let replied = tokio::task::spawn_blocking(move || {
            let request = Request {
                instructions: &rendered.instructions,
                prompt: &rendered.prompt,
                schema: &schema,
                temperature,
                max_tokens,
            };
            respond(&request, &token)
        })
        .await
        .map_err(failed)?;
        match replied {
            Ok(json) => Ok(json),
            Err(e) if e.kind() == ErrorKind::Cancelled => Err(CancelledSnafu.build().into()),
            Err(e) => Err(failed(snafu::Report::from_error(&e))),
        }
    }
}

#[cfg(test)]
#[path = "apple_tests.rs"]
mod apple_tests;
