//! What the detached agent window may ask the host: an allowlist (t-1469).
//!
//! The agent window shows one tab's agent: it reads the workspace view, the interface's
//! messages and scheme, that tab's snapshot and slide SVGs, opens a pack or theme the
//! agent names beside a tab, and attaches the agent back. Everything else (saving,
//! closing, moving or importing tabs, editing a deck) is refused, so a compromised agent
//! page cannot act on the workspace. Both predicates match exhaustively: a new request
//! variant must say whether the agent window may send it.

use std::fmt::{self, Write};

use snafu::ensure;

use crate::error::AgentWindowForbiddenSnafu;
use crate::{Request, Result, Workspace, WorkspaceRequest, is_window_label};

/// Label of the detached agent window, mirrored by the interface.
///
/// Not a workspace window (`w-<n>`): [`is_window_label`] refuses it.
pub const AGENT_WINDOW_LABEL: &str = "agent";

/// Refuses a workspace request the detached agent window may not send.
///
/// Allowed: `View`, `Messages`, `Scheme`, `AttachAgent`, and `OpenTheme` or `OpenPack`
/// from a tab a workspace window holds.
///
/// # Errors
/// `AgentWindowForbidden` for any other request.
pub fn check_agent_workspace(ws: &Workspace, req: &WorkspaceRequest) -> Result<()> {
    let allowed = match req {
        WorkspaceRequest::View
        | WorkspaceRequest::Messages
        | WorkspaceRequest::Scheme
        | WorkspaceRequest::AttachAgent => true,
        WorkspaceRequest::OpenTheme { tab, .. } | WorkspaceRequest::OpenPack { tab, .. } => held(ws, tab),
        WorkspaceRequest::SetLocale { .. }
        | WorkspaceRequest::SetScheme { .. }
        | WorkspaceRequest::NewTab { .. }
        | WorkspaceRequest::OpenTab { .. }
        | WorkspaceRequest::ImportTab { .. }
        | WorkspaceRequest::OpenHelp { .. }
        | WorkspaceRequest::SameDeck { .. }
        | WorkspaceRequest::CarryAssets { .. }
        | WorkspaceRequest::Clip
        | WorkspaceRequest::SetClip { .. }
        | WorkspaceRequest::Activate { .. }
        | WorkspaceRequest::Close { .. }
        | WorkspaceRequest::Reorder { .. }
        | WorkspaceRequest::MoveTab { .. }
        | WorkspaceRequest::NewWindow
        | WorkspaceRequest::CloseWindow { .. }
        | WorkspaceRequest::DetachAgent
        | WorkspaceRequest::ImportPicked { .. }
        | WorkspaceRequest::DismissImportPicks { .. }
        | WorkspaceRequest::NewPack { .. }
        | WorkspaceRequest::NewTheme { .. }
        | WorkspaceRequest::PackOpener { .. } => false,
    };
    ensure!(allowed, AgentWindowForbiddenSnafu { op: variant_name(req) });
    Ok(())
}

/// Refuses a tab request the detached agent window may not send.
///
/// Allowed: `Snapshot` and `Svg`, on a tab a workspace window holds.
///
/// # Errors
/// `AgentWindowForbidden` for any other request, or a tab no workspace window holds.
pub fn check_agent_request(ws: &Workspace, tab: &str, req: &Request) -> Result<()> {
    let allowed = match req {
        Request::Snapshot | Request::Svg { .. } => held(ws, tab),
        Request::Save { .. }
        | Request::Slide { .. }
        | Request::Search { .. }
        | Request::Apply { .. }
        | Request::SetPreviewTheme { .. }
        | Request::Undo
        | Request::Redo
        | Request::UndoTurn { .. }
        | Request::Schema
        | Request::Widgets
        | Request::HostSettings { .. }
        | Request::HostTheme { .. }
        | Request::HostToken { .. }
        | Request::DataPanel { .. }
        | Request::DataEdit { .. }
        | Request::ElementPanel { .. }
        | Request::ElementParam { .. }
        | Request::ElementList { .. }
        | Request::ChartKind { .. }
        | Request::ShowAs { .. }
        | Request::Layouts
        | Request::Packs { .. }
        | Request::SavePack { .. }
        | Request::RenameEditedTheme { .. }
        | Request::SetEditedThemeExtends { .. }
        | Request::EditThemeExtends { .. }
        | Request::SaveTheme { .. }
        | Request::EditTheme { .. }
        | Request::DeletePack { .. }
        | Request::Ui { .. }
        | Request::Merge { .. }
        | Request::HistoryOpen
        | Request::HistorySeek { .. }
        | Request::HistoryRestore
        | Request::HistoryClose
        | Request::HistoryVersions
        | Request::Move { .. }
        | Request::SaveCopy { .. }
        | Request::WidgetStatus { .. }
        | Request::WidgetSource { .. }
        | Request::SetWidget { .. }
        | Request::SetGuide { .. }
        | Request::ApproveGuides { .. }
        | Request::AcceptReceived { .. }
        | Request::LaterReceived { .. }
        | Request::RemoveWidget { .. }
        | Request::RenameWidget { .. }
        | Request::DuplicateWidget { .. }
        | Request::ReorderWidgets { .. }
        | Request::PreviewWidget { .. }
        | Request::SetExampleValue { .. }
        | Request::EditExampleList { .. }
        | Request::UsePackVersion { .. }
        | Request::UsePackCopy { .. }
        | Request::UseThemeVersion { .. } => false,
    };
    ensure!(allowed, AgentWindowForbiddenSnafu { op: variant_name(req) });
    Ok(())
}

/// Is `tab` held by a workspace window (`w-<n>`)?
fn held(ws: &Workspace, tab: &str) -> bool {
    ws.tab_window(tab).is_some_and(is_window_label)
}

/// The longest variant name kept for the refusal's log line, in bytes.
const NAME_MAX: usize = 64;

/// The variant name of `req`, for the log line: its `Debug` text up to the first field.
///
/// Written into a bounded buffer that stops at the first space or brace, so a large
/// request (a whole element, many ops) is never formatted in full.
fn variant_name(req: &impl fmt::Debug) -> String {
    let mut name = VariantName(String::new());
    // `Err` is how the buffer stops the formatting once the name is read: not a failure.
    let _stopped = write!(name, "{req:?}");
    name.0
}

/// A `fmt::Write` that keeps leading identifier characters and stops at the first other one.
struct VariantName(String);

impl Write for VariantName {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            if !c.is_ascii_alphanumeric() || self.0.len() >= NAME_MAX {
                return Err(fmt::Error);
            }
            self.0.push(c);
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "agent_window_allow_tests.rs"]
mod agent_window_allow_tests;
