//! Erreurs des outils MCP, dites à l'agent dans la langue de l'utilisateur.
//!
//! Toutes les réponses d'erreur des outils passent par [`Tools::fail_with`] : l'erreur complète
//! (cause, chemins, message de l'hôte) va au journal, l'agent reçoit le texte Fluent, qui ne
//! reprend que des arguments sûrs (nom d'outil, clé de réglage, nom d'action, reproche de serde
//! aux arguments de l'agent). Une erreur de la session est dite par [`say_session`] : un fichier
//! illisible ou impossible à écrire, sans son chemin.

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_session::SessionError;
use rust_mcp_sdk::schema::CallToolResult;
use snafu::{Backtrace, Snafu};

use crate::error::say_session;
use crate::tools::{Tools, fail};

/// Un appel d'outil refusé ou échoué.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum ToolError {
    #[snafu(display("invalid arguments"))]
    Arguments { source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("unknown tool {name}"))]
    UnknownTool { name: String, backtrace: Backtrace },
    #[snafu(display("not a template set id"))]
    PackId { source: uuid::Error, backtrace: Backtrace },
    #[snafu(display("the session refused the call"))]
    Session { source: SessionError, backtrace: Backtrace },
    #[snafu(display("the task of the tool failed"))]
    Task { source: tokio::task::JoinError, backtrace: Backtrace },
    #[snafu(display("denied by user"))]
    Denied { backtrace: Backtrace },
    #[snafu(display("no turn in progress: {tool} is only shown during a turn"))]
    NoTurn { tool: &'static str, backtrace: Backtrace },
    #[snafu(display("the turn of this call is over: nothing was shown"))]
    TurnOver { backtrace: Backtrace },
    #[snafu(display("a widget conversation cannot anchor on a slide"))]
    AnchorScope { backtrace: Backtrace },
    #[snafu(display("file already open in another tab"))]
    OpenElsewhere { backtrace: Backtrace },
    #[snafu(display("no file yet: give an absolute path"))]
    NoFile { backtrace: Backtrace },
    #[snafu(display("path must be absolute: {path}"))]
    PathRelative { path: String, backtrace: Backtrace },
    #[snafu(display("path must not contain '..': {path}"))]
    PathParent { path: String, backtrace: Backtrace },
    #[snafu(display("{key} cannot be changed by the agent"))]
    SettingLocked { key: String, backtrace: Backtrace },
    #[snafu(display("cannot change {key}: {message}"))]
    Setting { key: String, message: String, backtrace: Backtrace },
    #[snafu(display("rewrite answer without text"))]
    RewriteText { backtrace: Backtrace },
    #[snafu(display("invalid paragraphs"))]
    Paragraphs { source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("unexpected action {action}"))]
    UnexpectedAction { action: String, backtrace: Backtrace },
    #[snafu(display("the tab did not open the history in time"))]
    HistoryNotOpened { backtrace: Backtrace },
    #[snafu(display("the deck pins no template pack"))]
    NoPack { backtrace: Backtrace },
    #[snafu(display("history seek needs an index"))]
    HistoryIndex { backtrace: Backtrace },
    #[snafu(display("nothing to present in this mode"))]
    NothingToPresent { backtrace: Backtrace },
    #[snafu(display("unknown locale"))]
    UnknownLocale { backtrace: Backtrace },
    #[snafu(display("empty interface change"))]
    EmptyInterface { backtrace: Backtrace },
    #[snafu(display("the tab is not the one shown in its window"))]
    NotShown { backtrace: Backtrace },
    #[snafu(display("the theme picker opens in deck mode only"))]
    ThemesDeckMode { backtrace: Backtrace },
    /// Mis en boîte : `HubError` grossirait toutes les réponses d'erreur des outils.
    #[snafu(display("the surface hub refused the call"))]
    Hub { source: Box<crate::HubError>, backtrace: Backtrace },
    #[snafu(display("response not serializable"))]
    Response { source: serde_json::Error, backtrace: Backtrace },
}

impl Localize for ToolError {
    fn localize(&self, l: &Localizer) -> String {
        use ToolError::*;
        match self {
            Arguments { source, .. } => {
                l.tr_with("agent-error-arguments", &[("detail", argument_detail(source).into())])
            }
            UnknownTool { name, .. } => l.tr_with("agent-error-unknown-tool", &[("name", name.as_str().into())]),
            PackId { .. } => l.tr("agent-error-pack-id"),
            Session { source, .. } => say_session(source, l),
            Task { .. } => l.tr("agent-tool-failed"),
            Denied { .. } => l.tr("agent-error-denied"),
            NoTurn { tool, .. } => l.tr_with("agent-error-no-turn", &[("tool", (*tool).into())]),
            TurnOver { .. } => l.tr("agent-error-turn-over"),
            AnchorScope { .. } => l.tr("agent-error-anchor-scope"),
            OpenElsewhere { .. } => l.tr("agent-open-elsewhere"),
            NoFile { .. } => l.tr("agent-error-no-file"),
            PathRelative { .. } => l.tr("agent-error-path-relative"),
            PathParent { .. } => l.tr("agent-error-path-parent"),
            SettingLocked { key, .. } => l.tr_with("agent-error-setting-locked", &[("key", key.as_str().into())]),
            Setting { key, .. } => l.tr_with("agent-error-setting", &[("key", key.as_str().into())]),
            RewriteText { .. } => l.tr("agent-error-rewrite-text"),
            Paragraphs { .. } => l.tr("agent-error-paragraphs"),
            UnexpectedAction { action, .. } => {
                l.tr_with("agent-error-unexpected-action", &[("action", action.as_str().into())])
            }
            HistoryNotOpened { .. } => l.tr("agent-error-history-not-opened"),
            NoPack { .. } => l.tr("agent-error-no-pack"),
            HistoryIndex { .. } => l.tr("agent-error-history-index"),
            NothingToPresent { .. } => l.tr("agent-error-present"),
            UnknownLocale { .. } => l.tr("agent-error-locale"),
            EmptyInterface { .. } => l.tr("agent-error-empty-interface"),
            NotShown { .. } => l.tr("agent-error-not-shown"),
            ThemesDeckMode { .. } => l.tr("agent-error-themes-deck-mode"),
            Hub { source, .. } => source.localize(l),
            Response { .. } => l.tr("session-response"),
        }
    }
}

/// Ce que serde reproche aux arguments (champ manquant ou inconnu, type attendu, valeur de
/// l'agent), sans position : l'agent corrige son appel. Les arguments viennent de l'agent ;
/// le message ne contient ni chemin ni état de l'hôte.
fn argument_detail(e: &serde_json::Error) -> String {
    let message = e.to_string();
    let suffix = format!(" at line {} column {}", e.line(), e.column());
    match message.strip_suffix(&suffix) {
        Some(detail) => detail.to_owned(),
        None => message,
    }
}

impl Tools {
    /// La réponse d'erreur de `e` : journalisée en entier, dite à l'agent dans la langue de
    /// l'utilisateur.
    pub(crate) fn fail_with(&self, e: ToolError) -> CallToolResult {
        tracing::info!(error = %snafu::Report::from_error(&e), "outil de l'agent refusé");
        fail(e.localize(&self.localizer.current()))
    }
}

#[cfg(test)]
#[path = "tool_error_tests.rs"]
mod tool_error_tests;
