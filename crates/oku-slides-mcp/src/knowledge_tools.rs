//! The deck knowledge base: `knowledge_list`, `knowledge_read`, `knowledge_search`, the writes
//! `knowledge_write`, `knowledge_edit`, `knowledge_move`, `knowledge_delete`, and
//! `knowledge_import` of an attached document.
//!
//! Deck tabs only. The session decides what is open (approved texts only); a read attaches the
//! cited images that fit `[paste]`'s prompt limits, and lists every one with why it is not shown.
//! A write builds one op and goes through the agent's apply path: the session mints file ids,
//! refuses an edit or a move of an awaiting file and approves what the agent wrote. An import
//! locks the session to snapshot the attachment, extracts it unlocked in a child worker, then
//! locks again to write the reference files.

use base64::Engine as _;
use oku_slides_core::Op;
use oku_slides_platform::config::{ImageRefusal, PromptImageLimits};
use oku_slides_session::Request;
use oku_slides_session::agent::{PASTE_TYPES, TabOpen, WorkspaceRefusal};
use oku_slides_session::{ImportOutcome, KnowledgeEntry, KnowledgeHits, KnowledgeImage, KnowledgeRead};
use rust_mcp_sdk::schema::{CallToolResult, ImageContent};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use snafu::{IntoError, ResultExt};

use crate::McpEvent;
use crate::hub::TabTarget;
use crate::tab_gate::TabVerdict;
use crate::tool_error::{SessionSnafu, TabNoTurnSnafu, TaskSnafu, ToolError, WorkspaceSnafu};
use crate::tools::Tools;

/// The `knowledge_search` description; `{search_hits}` is the effective hit cap.
pub(crate) const SEARCH_DESCRIPTION: &str = "Search the deck's knowledge base for a text (substring, ignoring case and accents): hits with path, line number and the line. At most {search_hits} hits; more counts the others. Files awaiting approval are not searched.";

/// The `knowledge_search` description with its hit cap.
pub(crate) fn search_description(search_hits: usize) -> String {
    SEARCH_DESCRIPTION.replace("{search_hits}", &search_hits.to_string())
}

/// Why a cited image is listed without being shown, in the order they are checked.
const MISSING: &str = "missing from the deck";
/// Not a pasteable type (`PASTE_TYPES`), or its size in pixels could not be read.
const NOT_VISIBLE: &str = "not an image format you can see";
const TOO_LARGE: &str = "too large to show you";
const TOO_HEAVY_TOGETHER: &str = "too heavy together with the other images of this read";

/// The directory `knowledge_list` lists.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct KnowledgeListArgs {
    /// A directory of the knowledge base, such as /references; absent: every file.
    #[serde(default)]
    pub dir: Option<String>,
}

/// The file `knowledge_read` reads.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct KnowledgeReadArgs {
    /// Path of the file in the knowledge base, such as /references/roadmap.md.
    pub path: String,
}

/// The text `knowledge_search` looks for.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct KnowledgeSearchArgs {
    /// The text to find in the lines of the files, ignoring case and accents (at most 200 characters).
    pub query: String,
}

/// The file `knowledge_write` creates or replaces.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct KnowledgeWriteArgs {
    /// Bundle-absolute path ending in `.md`, such as /notes/pricing.md.
    pub path: String,
    /// The whole text of the file, frontmatter included.
    pub text: String,
}

/// The passage `knowledge_edit` replaces.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct KnowledgeEditArgs {
    /// Path of the file in the knowledge base.
    pub path: String,
    /// The exact passage to replace; it must appear once in the file.
    pub old: String,
    /// What replaces it.
    pub new: String,
}

/// The attached document `knowledge_import` imports.
///
/// Lengths are bounded downstream: `attachment` is only looked up among the tab's ids, `path` is
/// parsed as a knowledge path (at most `PATH_MAX_BYTES`), and errors quote neither raw.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct KnowledgeImportArgs {
    /// The attachment's id, as the prompt lists it.
    pub attachment: String,
    /// A /references/….md file an earlier import wrote, to replace; absent: a new file.
    #[serde(default)]
    pub path: Option<String>,
}

/// The file `knowledge_move` moves.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct KnowledgeMoveArgs {
    /// Current path of the file.
    pub from: String,
    /// New bundle-absolute path.
    pub to: String,
}

/// The file `knowledge_delete` deletes.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct KnowledgeDeleteArgs {
    /// Path of the file in the knowledge base.
    pub path: String,
}

/// One listed file: its frontmatter fields when approved, its path only when awaiting.
#[derive(Debug, Serialize)]
struct ListedFile {
    path: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    /// Absent when awaiting: nothing of an awaiting file but its path is told.
    #[serde(skip_serializing_if = "Option::is_none")]
    stale: Option<bool>,
    approval: &'static str,
}

impl From<KnowledgeEntry> for ListedFile {
    fn from(e: KnowledgeEntry) -> Self {
        if !e.approved {
            return Self {
                path: e.path,
                kind: None,
                title: None,
                description: None,
                status: None,
                stale: None,
                approval: "awaiting",
            };
        }
        // The frontmatter values are untrusted: bounded as the agent's index shows them.
        let b = e.bounded();
        Self {
            path: b.path,
            kind: b.kind,
            title: b.title,
            description: b.description,
            status: b.status,
            stale: Some(b.stale),
            approval: "approved",
        }
    }
}

/// One cited image of a read, shown or not.
#[derive(Debug, Serialize)]
struct CitedImage {
    asset: String,
    path: String,
    mime: &'static str,
    width: u32,
    height: u32,
    shown: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<&'static str>,
}

/// A read's JSON part and the image blocks it shows, built off the async thread.
struct ReadOutcome {
    reply: ReadReply,
    blocks: Vec<ImageContent>,
}

/// The JSON part of a read.
#[derive(Debug, Serialize)]
struct ReadReply {
    path: String,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    truncated_bytes: Option<usize>,
    images: Vec<CitedImage>,
}

#[derive(Debug, Serialize)]
struct Hit {
    path: String,
    line: usize,
    text: String,
}

#[derive(Debug, Serialize)]
struct SearchReply {
    hits: Vec<Hit>,
    more: usize,
}

impl Tools {
    /// The knowledge files, under `dir` when given.
    pub(crate) async fn knowledge_list(&self, args: Value) -> CallToolResult {
        let KnowledgeListArgs { dir } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let r = self.with_session(move |s| s.knowledge_entries(dir.as_deref())).await;
        self.reply(r, |entries| {
            let files: Vec<ListedFile> = entries.into_iter().map(ListedFile::from).collect();
            self.json_result(&serde_json::json!({ "files": files }))
        })
    }

    /// One approved file's text, then one image block per cited image shown.
    pub(crate) async fn knowledge_read(&self, args: Value) -> CallToolResult {
        let KnowledgeReadArgs { path } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let limits = self.platform.settings.paste.prompt_limits();
        // Base64 encoding of up to `total_bytes` runs in the blocking task too.
        let r = self.with_session(move |s| s.knowledge_file(&path).map(|read| read_outcome(read, limits))).await;
        self.reply(r, |ReadOutcome { reply, blocks }| {
            let mut result = self.json_result(&reply);
            if result.is_error != Some(true) {
                result.content.extend(blocks.into_iter().map(Into::into));
            }
            result
        })
    }

    /// The lines of approved files holding `query`.
    pub(crate) async fn knowledge_search(&self, args: Value) -> CallToolResult {
        let KnowledgeSearchArgs { query } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let r = self.with_session(move |s| s.knowledge_search(&query)).await;
        self.reply(r, |KnowledgeHits { hits, more }| {
            let hits = hits.into_iter().map(|h| Hit { path: h.path, line: h.line, text: h.text }).collect();
            self.json_result(&SearchReply { hits, more })
        })
    }

    /// Applies one knowledge op as the agent; the reply names the file's path after it.
    async fn knowledge_change(&self, op: Op, path: String) -> CallToolResult {
        let label = self.localizer.current().tr("agent-undo-apply");
        let applied = self.change(Request::Apply { label, ops: vec![op], merge: None }).await;
        if applied.is_error == Some(true) { applied } else { self.json_result(&json!({ "path": path })) }
    }

    /// Creates or replaces a file with its whole text; the session mints its id.
    pub(crate) async fn knowledge_write(&self, args: Value) -> CallToolResult {
        let KnowledgeWriteArgs { path, text } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        self.knowledge_change(Op::KnowledgeWrite { path: path.clone(), text, id: None }, path).await
    }

    /// Replaces one unique passage of an approved file.
    pub(crate) async fn knowledge_edit(&self, args: Value) -> CallToolResult {
        let KnowledgeEditArgs { path, old, new } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        self.knowledge_change(Op::KnowledgeEdit { path: path.clone(), old, new }, path).await
    }

    /// Moves or renames a file; the reply names the new path.
    pub(crate) async fn knowledge_move(&self, args: Value) -> CallToolResult {
        let KnowledgeMoveArgs { from, to } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        self.knowledge_change(Op::KnowledgeMove { from, to: to.clone() }, to).await
    }

    /// Deletes a file; the reply names the deleted path.
    pub(crate) async fn knowledge_delete(&self, args: Value) -> CallToolResult {
        let KnowledgeDeleteArgs { path } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        self.knowledge_change(Op::KnowledgeDelete { path: path.clone() }, path).await
    }

    /// Imports an attached document as reference files: job and apply under the session lock,
    /// the extraction between them without it.
    pub(crate) async fn knowledge_import(&self, args: Value) -> CallToolResult {
        let KnowledgeImportArgs { attachment, path } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let imported = self.import_attachment(attachment, path).await;
        let ok = imported.is_ok();
        let result = self.reply(imported, |o| {
            self.hub.emit(McpEvent::Changed);
            self.json_result(&json!({ "paths": o.paths, "images": o.images, "cut": o.cut }))
        });
        if ok {
            self.prune_surfaces().await;
        }
        result
    }

    /// `open_knowledge`: this deck's knowledge tab opened or shown; the turn's one tab move.
    ///
    /// It takes the workspace lock alone, never a session's. The tab is shown only if this deck
    /// tab is the one shown in its window.
    pub(crate) async fn open_knowledge(&self, turn: Option<u32>) -> CallToolResult {
        let Some(turn) = turn else { return self.fail_with(&TabNoTurnSnafu.build()) };
        let mut verdict = TabVerdict::default();
        if !verdict.ask(&self.hub, Some(turn), TabTarget::Knowledge) {
            return self.fail_with(&verdict.refusal());
        }
        let opened = self.on_workspace(move |ws, own| ws.open_knowledge(own)).await;
        // A lost task may have opened the tab: its claim stays spent (see `TabVerdict::settle_moved`).
        let spent = matches!(&opened, Ok(TabOpen::Opened { .. } | TabOpen::Focused { .. }))
            || matches!(opened, Err(ToolError::Task { .. }));
        verdict.settle_moved(&self.hub, Some(turn), TabTarget::Knowledge, spent);
        match opened {
            Ok(TabOpen::Opened { tab, shown } | TabOpen::Focused { tab, shown }) => {
                self.hub.emit(McpEvent::Changed);
                self.json_result(&json!({ "ok": true, "tab": tab, "shown": shown }))
            }
            // No host answers these (`AgentWorkspace::open_knowledge` asks no gate, and a knowledge
            // tab is another tab than the caller's): a bug, told as a tab the workspace lacks.
            Ok(other @ (TabOpen::Refused | TabOpen::Here)) => {
                tracing::error!(outcome = ?other, "open_knowledge: the workspace opened no tab");
                self.fail_with(&WorkspaceSnafu.into_error(WorkspaceRefusal::unknown_tab()))
            }
            Err(e) => self.fail_with(&e),
        }
    }

    /// The three steps of [`Self::knowledge_import`].
    async fn import_attachment(&self, attachment: String, path: Option<String>) -> Result<ImportOutcome, ToolError> {
        let job = self.with_session(move |s| s.knowledge_import_job(&attachment, path.as_deref())).await?;
        let (job, extracted) = tokio::task::spawn_blocking(move || job.run().map(|e| (job, e)))
            .await
            .context(TaskSnafu)?
            .context(SessionSnafu)?;
        let label = self.localizer.current().tr("agent-undo-apply");
        self.with_session(move |s| s.knowledge_import_apply(&label, job.ticket(), extracted)).await
    }
}

/// Why `image` is not shown, `total` being the encoded bytes already shown; `None`: shown,
/// its encoded size added to `total`.
fn refusal(image: &KnowledgeImage, limits: PromptImageLimits, total: &mut usize) -> Option<&'static str> {
    let Some(bytes) = &image.bytes else {
        return Some(MISSING);
    };
    if !PASTE_TYPES.contains(&image.mime) || image.width == 0 || image.height == 0 {
        return Some(NOT_VISIBLE);
    }
    match limits.admit(bytes.len(), image.width.max(image.height), *total) {
        Err(ImageRefusal::TooLarge) => Some(TOO_LARGE),
        Err(ImageRefusal::TooHeavyTogether) => Some(TOO_HEAVY_TOGETHER),
        Ok(encoded) => {
            *total += encoded;
            None
        }
    }
}

/// A read's JSON, then the images that fit the prompt limits, in the read's (asset id) order.
fn read_outcome(read: KnowledgeRead, limits: PromptImageLimits) -> ReadOutcome {
    let mut total = 0usize;
    let mut blocks = Vec::new();
    let mut images = Vec::with_capacity(read.images.len());
    for image in read.images {
        let reason = refusal(&image, limits, &mut total);
        let KnowledgeImage { asset, path, mime, width, height, bytes } = image;
        if let (None, Some(b)) = (reason, bytes) {
            let b64 = base64::engine::general_purpose::STANDARD.encode(b);
            blocks.push(ImageContent::new(b64, mime.into(), None, None));
        }
        images.push(CitedImage { asset, path, mime, width, height, shown: reason.is_none(), reason });
    }
    let truncated_bytes = (read.truncated > 0).then_some(read.truncated);
    ReadOutcome { reply: ReadReply { path: read.path, text: read.text, truncated_bytes, images }, blocks }
}

#[cfg(test)]
#[path = "knowledge_tools_tests.rs"]
mod knowledge_tools_tests;
