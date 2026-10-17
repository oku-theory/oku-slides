//! Outils de l'ajustement d'un import par l'agent.
//!
//! `import_report` (le rapport et la file des slides) et `import_diff` (une slide, ou une de ses
//! régions, contre sa référence).
//!
//! Le rendu de référence (le témoin choisi à l'import, dans son propre
//! processus de rendu) est fait au premier besoin, hors du verrou de la
//! session : la session donne le travail ([`Session::import_reference_job`]),
//! il tourne dans une tâche bloquante et remplit l'état de l'import ; deux
//! appels concurrents n'en font qu'un, le second attendant le premier. Le
//! reste passe par `with_session`, un appel de la session à la fois, sans
//! jamais appeler le témoin. Les refus (pas d'import, plus d'essai…) sont
//! ceux de la session, localisés.
//!
//! [`Session::import_reference_job`]: oku_slides_session::Session::import_reference_job

use oku_slides_session::import::ImportDiff;
use rust_mcp_sdk::schema::{CallToolResult, TextContent};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use snafu::ResultExt;

use crate::tool_error::{ResponseSnafu, TaskSnafu, ToolError};
use crate::tools::{Tools, png};

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ImportDiffArgs {
    /// Identifiant de la slide (voir `import_report`).
    pub slide: String,
    /// Une région.
    ///
    /// L'id d'item source d'une région de la slide (voir `import_diff`) ; sans elle, la slide entière.
    #[serde(default)]
    pub region: Option<String>,
}

impl Tools {
    /// Fait le rendu de référence manquant, hors du verrou de la session.
    #[tracing::instrument(skip_all)]
    async fn ensure_reference(&self) -> Result<(), ToolError> {
        let Some(job) = self.with_session(|s| s.import_reference_job()).await? else { return Ok(()) };
        let platform = self.platform.clone();
        tokio::task::spawn_blocking(move || job.run(&platform)).await.context(TaskSnafu)
    }

    pub(crate) async fn import_report(&self) -> CallToolResult {
        if let Err(e) = self.ensure_reference().await {
            return self.fail_with(e);
        }
        let l = self.localizer.current();
        let report = self.with_session(move |s| s.import_report(&l)).await;
        self.reply(report, |v: Value| self.json_result(&v))
    }

    pub(crate) async fn import_diff(&self, args: Value) -> CallToolResult {
        let ImportDiffArgs { slide, region } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        if let Err(e) = self.ensure_reference().await {
            return self.fail_with(e);
        }
        let diff = self.with_session(move |s| s.import_diff(&slide, region.as_deref())).await;
        let ImportDiff { reference, oku, json } = match diff {
            Ok(d) => d,
            Err(e) => return self.fail_with(e),
        };
        let text = match serde_json::to_string(&json).context(ResponseSnafu) {
            Ok(t) => t,
            Err(e) => return self.fail_with(e),
        };
        let mut result = png(&reference);
        result.content.extend(png(&oku).content);
        result.content.push(TextContent::new(text, None, None).into());
        result
    }
}
