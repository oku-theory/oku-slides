//! The agent's PowerPoint import, and the tools that fine-tune one.
//!
//! `import` opens a `.pptx` in a new tab of the requester's window, through the host's
//! [`AgentWorkspace`](oku_slides_session::agent::AgentWorkspace): the user confirms first, and it
//! is the turn's one tab move. `import_report` (the report and the slide queue) and
//! `import_diff` (one slide, or one of its regions, against its reference) fine-tune it.
//!
//! The reference rendering (the witness chosen at import, in its own render process) is made
//! on first need, outside the session lock: the session hands out the job
//! ([`Session::import_reference_job`]), it runs in a blocking task and fills the import state;
//! two concurrent calls make one, the second waiting for the first. The rest goes through
//! `with_session`, one session call at a time, never calling the witness. Refusals (no import,
//! no try left…) are the session's own, localized.
//!
//! [`Session::import_reference_job`]: oku_slides_session::Session::import_reference_job

use std::path::{Path, PathBuf};
use std::sync::Arc;

use oku_slides_session::agent::{TabOpen, WorkspaceRefusal};
use oku_slides_session::import::{ImportDiff, ImportTarget, check_target};
use rust_mcp_sdk::schema::{CallToolResult, TextContent};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use snafu::{IntoError, ResultExt};
use uuid::Uuid;

use crate::hub::TabTarget;
use crate::paths::checked;
use crate::tab_gate::TabVerdict;
use crate::tool_error::{
    ImportExtensionSnafu, ImportHifiTargetSnafu, ResponseSnafu, SessionSnafu, TaskSnafu, ToolError, WorkspaceSnafu,
};
use crate::tools::{Tools, png};
use crate::{ConfirmVerb, McpEvent};

/// `import`: the presentation to import and how.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ImportArgs {
    /// Absolute path of a `.pptx`.
    pub path: String,
    /// `hifi`: the slides as drawn, fine-tuned against the original; `retrofit`: rebuilt on layouts.
    pub mode: ImportModeArg,
    /// Retrofit only: the layout packs (id or name, see `packs`), in order; absent, Essentiel.
    #[serde(default)]
    pub pack: Vec<String>,
    /// Retrofit only: a library theme (id or name); absent, the default theme.
    #[serde(default)]
    pub theme: Option<String>,
    /// Start the fine-tuning turn in the new tab once it opens.
    #[serde(default)]
    pub tune: bool,
}

/// The mode an agent asks of `import`.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ImportModeArg {
    /// The slides as drawn, on a pack generated for the presentation.
    Hifi,
    /// The slides rebuilt on layout packs and a library theme.
    Retrofit,
}

/// What an `import` call is about to do, checked before the user is asked.
struct ImportPlan {
    /// The normalized `.pptx` path, shown in full to the user.
    path: PathBuf,
    /// Where the slides go.
    target: ImportTarget,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ImportDiffArgs {
    /// Identifier of the slide (see `import_report`).
    pub slide: String,
    /// A region of the slide.
    ///
    /// The source item id of a region of the slide (see `import_diff`); without it, the whole slide.
    #[serde(default)]
    pub region: Option<String>,
}

impl Tools {
    /// Makes the missing reference rendering, outside the session lock.
    #[tracing::instrument(skip_all)]
    async fn ensure_reference(&self) -> Result<(), ToolError> {
        let Some(job) = self.with_session(oku_slides_session::Session::import_reference_job).await? else {
            return Ok(());
        };
        let platform = self.platform.clone();
        tokio::task::spawn_blocking(move || job.run(&platform)).await.context(TaskSnafu)
    }

    pub(crate) async fn import_report(&self) -> CallToolResult {
        if let Err(e) = self.ensure_reference().await {
            return self.fail_with(&e);
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
            return self.fail_with(&e);
        }
        let diff = self.with_session(move |s| s.import_diff(&slide, region.as_deref())).await;
        let ImportDiff { reference, oku, json } = match diff {
            Ok(d) => d,
            Err(e) => return self.fail_with(&e),
        };
        let text = match serde_json::to_string(&json).context(ResponseSnafu) {
            Ok(t) => t,
            Err(e) => return self.fail_with(&e),
        };
        let mut result = png(&reference);
        result.content.extend(png(&oku).content);
        result.content.push(TextContent::new(text, None, None).into());
        result
    }
}

impl Tools {
    /// `import`: a `.pptx` in a new tab of this tab's window, confirmed, the turn's one tab move.
    ///
    /// Everything that can be refused without the user is refused first: the path (absolute,
    /// normalized, `.pptx`), a hi-fi import given a pack or theme, an unknown pack or theme, then
    /// the turn's tab claim. The claim is held from before the question; the user's answer is
    /// checked against the still-open turn, and a call that opened no tab gives its claim back.
    /// The workspace's gate never calls the hub: the claim is taken before its lock.
    pub(crate) async fn import(&self, turn: Option<u32>, args: Value) -> CallToolResult {
        let args: ImportArgs = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let tune = args.tune;
        let ImportPlan { path, target } = match self.import_plan(args).await {
            Ok(plan) => plan,
            Err(e) => return self.fail_with(&e),
        };
        let claim = TabTarget::Import(Uuid::new_v4());
        let mut verdict = TabVerdict::default();
        if !verdict.ask(&self.hub, turn, claim) {
            return self.fail_with(&verdict.refusal());
        }
        let shown_path = path.display().to_string();
        let message = self.localizer.current().tr_with("agent-confirm-import", &[("path", shown_path.into())]);
        if !self.hub.confirm(message, Some(ConfirmVerb::Import)).await {
            verdict.settle_moved(&self.hub, turn, claim, false);
            return self.denied();
        }
        // The turn may have ended while the user answered: the claim then counts for nothing.
        if !verdict.ask(&self.hub, turn, claim) {
            verdict.settle_moved(&self.hub, turn, claim, false);
            return self.fail_with(&verdict.refusal());
        }
        let opened = self.import_tab(path, target).await;
        // A lost task may have inserted the tab before it panicked: its claim stays spent for the
        // rest of the turn, which fails closed (see `TabVerdict::settle`).
        let spent =
            matches!(opened, Ok(TabOpen::Opened { .. } | TabOpen::Focused { .. }) | Err(ToolError::Task { .. }));
        verdict.settle_moved(&self.hub, turn, claim, spent);
        let (tab, shown) = match opened {
            Ok(TabOpen::Opened { tab, shown } | TabOpen::Focused { tab, shown }) => (tab, shown),
            // The gate is pure, so no host answers these today: told as a tab the workspace lacks.
            Ok(other @ (TabOpen::Refused | TabOpen::Here)) => {
                tracing::debug!(outcome = ?other, "import: the workspace opened no tab");
                return self.fail_with(&WorkspaceSnafu.into_error(WorkspaceRefusal::unknown_tab()));
            }
            Err(e) => return self.fail_with(&e),
        };
        self.hub.emit(McpEvent::Changed);
        self.prune_surfaces().await;
        let tuned = if tune { Some(self.start_tune(&tab).await) } else { None };
        let shown_arg = if shown { "yes" } else { "no" };
        let tune_arg = match tuned {
            Some(true) => "started",
            Some(false) => "unavailable",
            None => "no",
        };
        let note_args = [("tab", tab.as_str().into()), ("shown", shown_arg.into()), ("tune", tune_arg.into())];
        let note = self.localizer.current().tr_with("agent-import-opened-in-tab", &note_args);
        let mut result = json!({ "ok": true, "tab": tab, "shown": shown, "note": note });
        if let Some(tuned) = tuned {
            result["tune"] = tuned.into();
        }
        self.json_result(&result)
    }

    /// The checked path and target of `args`, or why the call is refused before asking.
    async fn import_plan(&self, args: ImportArgs) -> Result<ImportPlan, ToolError> {
        let ImportArgs { path, mode, pack, theme, tune: _ } = args;
        let path = checked(&path)?;
        if !is_pptx(&path) {
            return ImportExtensionSnafu.fail();
        }
        let target = match mode {
            ImportModeArg::Hifi if !pack.is_empty() || theme.is_some() => return ImportHifiTargetSnafu.fail(),
            ImportModeArg::Hifi => ImportTarget::Hifi { witness: None },
            ImportModeArg::Retrofit => ImportTarget::Retrofit { layouts: pack, theme },
        };
        let (platform, wanted) = (self.platform.clone(), target.clone());
        tokio::task::spawn_blocking(move || check_target(&platform, &wanted))
            .await
            .context(TaskSnafu)?
            .context(SessionSnafu)?;
        Ok(ImportPlan { path, target })
    }

    /// Imports `path` through the host's workspace, in a blocking task; the claim is already held.
    async fn import_tab(&self, path: PathBuf, target: ImportTarget) -> Result<TabOpen, ToolError> {
        let (workspace, tab) = (Arc::clone(&self.workspace), self.session.tab().to_owned());
        // The gate is pure: asking the hub under the workspace lock could deadlock (see the module doc).
        tokio::task::spawn_blocking(move || workspace.import_tab(&tab, &path, target, &mut |_| true))
            .await
            .context(TaskSnafu)?
            .context(WorkspaceSnafu)
    }

    /// Starts tab `tab`'s fine-tuning turn through the [`crate::TunePort`]; `false` when refused.
    async fn start_tune(&self, tab: &str) -> bool {
        let (tune, target) = (Arc::clone(&self.tune), tab.to_owned());
        match tokio::task::spawn_blocking(move || tune.tune(&target)).await {
            Ok(Ok(())) => true,
            Ok(Err(e)) => {
                tracing::warn!(tab, error = %snafu::Report::from_error(&e), "import: no fine-tuning turn");
                false
            }
            Err(e) => {
                tracing::warn!(tab, error = %e, "import: fine-tuning task lost");
                false
            }
        }
    }
}

/// True when `path` names a `.pptx` file, whatever the extension's case.
fn is_pptx(path: &Path) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pptx"))
}

#[cfg(test)]
#[path = "import_tools_tests.rs"]
mod import_tools_tests;
