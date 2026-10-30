//! Opening a pack or a theme in its own tab, for a deck's agent (`open_pack`, `open_theme`).
//!
//! Both go through one gate: at most one tab move (a new tab or a focused one) per turn, and
//! only inside a turn ([`TabVerdict`]). A call that moved no tab gives its claim back.

use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_core::{Namespace, PackPin};
use oku_slides_session::agent::{TabGate, TabOpen, TabSession};
use rust_mcp_sdk::schema::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use snafu::IntoError;

use crate::McpEvent;
use crate::draft::{DraftOutcome, fits, normalize_request};
use crate::edit_tools::PackIdArgs;
use crate::hub::TabTarget;
use crate::tab_gate::TabVerdict;
use crate::tool_error::{NoPackSnafu, SessionSnafu, TaskSnafu};
use crate::tools::Tools;

/// `open_pack`: the pack to open and the item to select in its tab.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct OpenPackArgs {
    /// Id of a pack (UUID, see `packs`); absent: the deck's main pack.
    #[serde(default)]
    pub pack: Option<String>,
    /// A widget or layout of the pack to select in its tab.
    #[serde(default)]
    pub widget: Option<String>,
    /// The user's request, restated for the opened tab; it waits in that tab's draft for the user to review and send.
    #[serde(default)]
    pub request: Option<String>,
}

/// `open_theme`: the library theme to open.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct OpenThemeArgs {
    /// The theme: its namespace (as `deck_outline` lists it), id or name.
    pub theme: String,
    /// The user's request, restated for the opened tab; it waits in that tab's draft for the user to review and send.
    #[serde(default)]
    pub request: Option<String>,
}

impl Tools {
    /// `open_pack`: the pack (or the deck's main one) in its own tab, gated.
    ///
    /// `turn` is the turn the call arrived in. A bad id or a deck without a pack fails before
    /// any gate is asked.
    pub(crate) async fn open_pack(&self, turn: Option<u32>, args: Value) -> CallToolResult {
        let OpenPackArgs { pack, widget, request } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let request = normalize_request(request);
        let pack = match pack {
            Some(pack) => PackIdArgs { pack }.id(),
            None => match self
                .with_session(|s| {
                    let snap = s.snapshot();
                    Ok(main_pack(snap.widgets, snap.layouts))
                })
                .await
            {
                Ok(found) => found.ok_or_else(|| NoPackSnafu.build()),
                Err(e) => Err(e),
            },
        };
        let pack = match pack {
            Ok(pack) => pack,
            Err(e) => return self.fail_with(&e),
        };
        let open =
            move |tab: &TabSession, gate: &mut dyn FnMut(TabGate) -> bool| tab.open_pack(pack, widget.as_deref(), gate);
        self.open_tab(turn, TabTarget::Pack(pack), open, request, "agent-pack-opened-in-tab").await
    }

    /// `open_theme`: a library theme in its own tab, gated.
    ///
    /// The theme is resolved in this tab's session first (namespace, id or name): an unknown
    /// theme fails before any gate is asked.
    pub(crate) async fn open_theme(&self, turn: Option<u32>, args: Value) -> CallToolResult {
        let OpenThemeArgs { theme, request } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let request = normalize_request(request);
        let id = match self.with_session(move |s| s.find_theme_info(&theme)).await {
            Ok(info) => info.id,
            Err(e) => return self.fail_with(&e),
        };
        let open = move |tab: &TabSession, gate: &mut dyn FnMut(TabGate) -> bool| tab.open_theme(id, gate);
        self.open_tab(turn, TabTarget::Theme(id), open, request, "agent-theme-opened-in-tab").await
    }

    /// One gated open: the verdict, the open in a blocking task, then the claim settled and the reply.
    ///
    /// The gate, its verdict and the hub it asks live in the blocking task: the open holds the
    /// document's opening lock, so the hub lock the gate takes is a leaf. Only `Opened` and
    /// `Focused` are moves; an error (a lease refused after a granted focus included) is told to
    /// the agent, its claim given back by [`TabVerdict::settle`].
    ///
    /// Only a move passes `request` (already normalized) to the moved tab's draft, once the
    /// verdict is settled and no lock is held; the result tells how (`draft`), never the text.
    async fn open_tab(
        &self,
        turn: Option<u32>,
        target: TabTarget,
        open: impl FnOnce(&TabSession, &mut dyn FnMut(TabGate) -> bool) -> oku_slides_session::Result<TabOpen>
        + Send
        + 'static,
        request: Option<String>,
        note_key: &str,
    ) -> CallToolResult {
        let (tab, hub) = (self.session.clone(), Arc::clone(&self.hub));
        let opened = tokio::task::spawn_blocking(move || {
            let mut verdict = TabVerdict::default();
            let result = open(&tab, &mut |_: TabGate| verdict.ask(&hub, turn, target));
            verdict.settle(&hub, turn, target, &result);
            (result, verdict)
        })
        .await;
        let (result, verdict) = match opened {
            Ok(done) => done,
            // The claim stays spent: the turn fails closed (see `TabVerdict::settle`).
            Err(e) => return self.fail_with(&TaskSnafu.into_error(e)),
        };
        let (tab, shown) = match result {
            Ok(TabOpen::Refused) => return self.fail_with(&verdict.refusal()),
            // This tab already holds the document: nothing moved.
            Ok(TabOpen::Here) => return self.json_result(&json!({ "ok": true })),
            Ok(TabOpen::Opened { tab, shown } | TabOpen::Focused { tab, shown }) => (tab, shown),
            Err(e) => return self.fail_with(&SessionSnafu.into_error(e)),
        };
        self.hub.emit(McpEvent::Changed);
        self.prune_surfaces().await;
        let draft = match request {
            Some(text) => Some(self.prefill(&tab, text).await),
            None => None,
        };
        let shown_arg = if shown { "yes" } else { "no" };
        let draft_arg = draft.map_or("none", DraftOutcome::as_str);
        let args = [("tab", tab.as_str().into()), ("shown", shown_arg.into()), ("draft", draft_arg.into())];
        let note = self.localizer.current().tr_with(note_key, &args);
        let mut result = json!({ "ok": true, "tab": tab, "shown": shown, "note": note });
        if let Some(draft) = draft {
            result["draft"] = draft.as_str().into();
        }
        self.json_result(&result)
    }

    /// Passes `text` to tab `tab`'s draft through the [`crate::DraftPort`], in a blocking task.
    ///
    /// A request over [`crate::MAX_REQUEST_BYTES`] never reaches the port. A lost task is
    /// logged and told as `Unavailable`: the open itself already happened.
    async fn prefill(&self, tab: &str, text: String) -> DraftOutcome {
        if !fits(&text) {
            return DraftOutcome::TooLong;
        }
        let (drafts, target) = (Arc::clone(&self.drafts), tab.to_owned());
        match tokio::task::spawn_blocking(move || drafts.prefill(&target, text)).await {
            Ok(prefill) => prefill.into(),
            Err(e) => {
                tracing::warn!(tab, error = %e, "draft prefill: task lost");
                DraftOutcome::Unavailable
            }
        }
    }
}

/// The pack to open when the agent names none: the deck's main pack, in the interface's order.
///
/// The deck's own layout pack, else its own widget pack, else the first non-built-in pack (layouts
/// first), else Essentiel, else `base`.
pub(crate) fn main_pack(
    widgets: &BTreeMap<Namespace, PackPin>,
    layouts: &BTreeMap<Namespace, PackPin>,
) -> Option<uuid::Uuid> {
    use oku_slides_core::kinds::{base, essentiel};
    let builtin = |ns: &Namespace| ns.as_str() == base::NAMESPACE || ns.as_str() == essentiel::NAMESPACE;
    let own = |(ns, _): &(&Namespace, &PackPin)| !ns.is_deck() && !ns.is_deck_layouts() && !builtin(ns);
    let named = |name: &'static str| move |(ns, _): &(&Namespace, &PackPin)| ns.as_str() == name;
    layouts
        .iter()
        .find(|(ns, _)| ns.is_deck_layouts())
        .or_else(|| widgets.iter().find(|(ns, _)| ns.is_deck()))
        .or_else(|| layouts.iter().find(own))
        .or_else(|| widgets.iter().find(own))
        .or_else(|| layouts.iter().find(named(essentiel::NAMESPACE)))
        .or_else(|| widgets.iter().find(named(base::NAMESPACE)))
        .map(|(_, pin)| pin.id)
}

#[cfg(test)]
#[path = "open_tools_tests.rs"]
mod open_tools_tests;
