//! Outils qui modifient le deck ; tout s'annule (dans le groupe du tour s'il est ouvert).

use oku_slides_chart::SeriesType;
use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_core::{Namespace, Op, PackKind, PackPin};
use oku_slides_session::agent::{PackGate, PackOpen};
use oku_slides_session::{PackHost, Request};
use rust_mcp_sdk::schema::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use snafu::{IntoError, ResultExt};

use crate::hub::Reserve;
use crate::tool_error::{
    NoPackSnafu, PackChangeSnafu, PackIdSnafu, PackTabLimitSnafu, PackTabNoTurnSnafu, SessionSnafu, TaskSnafu,
    ToolError,
};
use crate::tools::Tools;
use crate::{McpEvent, SurfaceHub};

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ApplyArgs {
    /// Deck ops, applied together (all or nothing); see each op's schema.
    pub ops: Vec<Op>,
    /// Undo label outside a turn; within a turn, the turn's label is used.
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ShowAsArgs {
    /// Id of the slide that holds the element.
    pub slide: String,
    /// Id of the chart or table element.
    pub element: String,
    /// `table` or `chart`; the data is kept.
    pub view: oku_slides_session::View,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ChartKindArgs {
    /// Id of the slide that holds the chart.
    pub slide: String,
    /// Id of the chart element.
    pub element: String,
    /// New series type; the data is converted.
    pub kind: SeriesType,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PackIdArgs {
    /// Id of a pack (UUID, see `packs`).
    pub pack: String,
}

impl PackIdArgs {
    /// L'id du pack ; un texte qui n'est pas un UUID est refusé par un
    /// message qui renvoie l'agent vers `packs`.
    pub(crate) fn id(&self) -> Result<uuid::Uuid, ToolError> {
        uuid::Uuid::parse_str(&self.pack).context(PackIdSnafu)
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct EditPackArgs {
    /// Id of a pack (UUID, see `packs`); absent: the deck's main pack.
    ///
    /// The main pack is the deck's own layout pack, otherwise its own widget
    /// pack, otherwise a non-built-in pack, otherwise Essentiel.
    #[serde(default)]
    pub pack: Option<String>,
}

/// `add_pack` and `remove_pack`: the targeted pack.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PackRefArgs {
    /// `widgets` (elements) or `layouts` (slides).
    pub kind: PackKind,
    /// Namespace of the pack (see `packs` and `deck_outline`).
    pub namespace: Namespace,
}

/// `set_theme_extends`: the parents of the edited theme.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ThemeExtendsArgs {
    /// Namespaces of library themes, in order; the later ones win.
    pub extends: Vec<Namespace>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct FocusArgs {
    /// Id of the slide to show.
    pub slide: String,
    /// Id of an element of that slide to show; absent: the slide alone.
    #[serde(default)]
    pub element: Option<String>,
}

impl Tools {
    /// Une requête qui change la session ; `Changed` si elle aboutit.
    async fn change(&self, req: Request) -> CallToolResult {
        let done = self.with_session(move |s| s.handle(req).map(|_| ())).await;
        let ok = done.is_ok();
        let result = self.reply(done, |()| {
            self.hub.emit(McpEvent::Changed);
            self.json_result(&json!({ "ok": true }))
        });
        if ok {
            self.prune_surfaces().await;
        }
        result
    }

    /// Élague et rafraîchit les surfaces du hub après un changement du deck.
    ///
    /// `SurfaceHub::prune` verrouille la session : il tourne hors du fil async.
    pub(crate) async fn prune_surfaces(&self) {
        let hub = self.hub.clone();
        if let Err(e) = tokio::task::spawn_blocking(move || hub.prune()).await {
            tracing::warn!(error = %e, "surfaces non élaguées");
        }
    }

    pub(crate) async fn apply(&self, args: Value) -> CallToolResult {
        let ApplyArgs { ops, label } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let label = label.unwrap_or_else(|| self.localizer.current().tr("agent-undo-apply"));
        self.change(Request::Apply { label, ops, merge: None }).await
    }

    pub(crate) async fn chart_kind(&self, args: Value) -> CallToolResult {
        let ChartKindArgs { slide, element, kind } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let label = self.localizer.current().tr("agent-undo-apply");
        self.change(Request::ChartKind { label, slide, element, kind }).await
    }

    pub(crate) async fn show_as(&self, args: Value) -> CallToolResult {
        let ShowAsArgs { slide, element, view } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let label = self.localizer.current().tr("agent-undo-apply");
        self.change(Request::ShowAs { label, slide, element, view }).await
    }

    /// Opens a pack for editing: a library pack in its own tab, a deck-only one in this tab's workshop.
    ///
    /// `turn` is the turn the call arrived in. The agent's gate lets at most one new pack tab open
    /// per turn, and no tab open or take the focus outside a turn.
    pub(crate) async fn edit_pack(&self, turn: Option<u32>, args: Value) -> CallToolResult {
        let args = match self.parse::<EditPackArgs>(args) {
            Ok(args) => args,
            Err(r) => return r,
        };
        let pack = match args.pack {
            Some(pack) => PackIdArgs { pack }.id(),
            None => self.default_pack().await,
        };
        let pack = match pack {
            Ok(pack) => pack,
            Err(e) => return self.fail_with(&e),
        };
        // The gate, its verdict and the hub it asks are built here, in the blocking task: the open
        // holds the pack's opening lock, so the hub lock it takes is a leaf.
        let (tab, hub) = (self.session.clone(), Arc::clone(&self.hub));
        let opened = tokio::task::spawn_blocking(move || {
            let mut verdict = Verdict::default();
            let result = tab.open_pack(pack, None, &mut |gate| verdict.ask(&hub, turn, pack, gate));
            (result, verdict)
        })
        .await;
        let (result, verdict) = match opened {
            Ok(done) => done,
            Err(e) => return self.fail_with(&TaskSnafu.into_error(e)),
        };
        // `Some(tab)`: the pack opened in that other tab; `None`: here, in this tab.
        let elsewhere = match result {
            Ok(PackOpen::Refused) => return self.fail_with(&verdict.refusal()),
            Ok(PackOpen::Here) => None,
            Ok(PackOpen::Tab(tab)) => Some(tab),
            Err(e) => {
                if let (true, Some(turn)) = (verdict.claimed, turn) {
                    self.hub.release_pack_tab(turn, pack);
                }
                return self.fail_with(&SessionSnafu.into_error(e));
            }
        };
        if let (Some(_), true, Some(turn)) = (&elsewhere, verdict.focused, turn) {
            self.hub.note_elsewhere(turn);
        }
        self.hub.emit(McpEvent::Changed);
        self.prune_surfaces().await;
        match elsewhere {
            Some(tab) => {
                let note =
                    self.localizer.current().tr_with("agent-pack-opened-in-tab", &[("tab", tab.as_str().into())]);
                self.json_result(&json!({ "ok": true, "tab": tab, "note": note }))
            }
            None => self.json_result(&json!({ "ok": true })),
        }
    }

    /// The pack `edit_pack` opens without an argument.
    ///
    /// In a pack's own tab, the pack that tab edits; otherwise the deck's main pack. Both are read
    /// from one snapshot, in one session access.
    async fn default_pack(&self) -> Result<uuid::Uuid, ToolError> {
        let found = self
            .with_session(|s| {
                let snap = s.snapshot();
                Ok(match &snap.editing {
                    Some(editing) if editing.host == PackHost::Tab => Some(editing.pack),
                    _ => main_pack(snap.widgets, snap.layouts),
                })
            })
            .await?;
        found.ok_or_else(|| NoPackSnafu.build())
    }

    /// Ajoute au deck la version courante d'un pack de la bibliothèque (mode deck).
    pub(crate) async fn add_pack(&self, args: Value) -> CallToolResult {
        let PackRefArgs { kind, namespace } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        self.change_packs(move |s| s.add_library_pack(kind, &namespace)).await
    }

    /// Retire un pack du deck (mode deck) ; un refus dit pourquoi.
    pub(crate) async fn remove_pack(&self, args: Value) -> CallToolResult {
        let PackRefArgs { kind, namespace } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        self.change_packs(move |s| s.remove_deck_pack(kind, &namespace)).await
    }

    /// Un changement des packs du deck ; `Changed` et élagage s'il aboutit.
    ///
    /// Un refus du cœur se dit par l'op refusée elle-même (`PackChange`), sans le rang de la transaction.
    async fn change_packs(
        &self,
        f: impl FnOnce(&mut oku_slides_session::Session) -> oku_slides_session::Result<()> + Send + 'static,
    ) -> CallToolResult {
        let session = self.session.clone();
        let done = match tokio::task::spawn_blocking(move || session.with(f).and_then(|r| r)).await {
            Ok(done) => done.map_err(|e| PackChangeSnafu.into_error(e)),
            Err(e) => Err(crate::tool_error::TaskSnafu.into_error(e)),
        };
        let ok = done.is_ok();
        let result = self.reply(done, |()| {
            self.hub.emit(McpEvent::Changed);
            self.json_result(&json!({ "ok": true }))
        });
        if ok {
            self.prune_surfaces().await;
        }
        result
    }

    /// Les parents du thème édité (mode pack), en une étape d'annulation.
    pub(crate) async fn set_theme_extends(&self, args: Value) -> CallToolResult {
        let ThemeExtendsArgs { extends } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        self.change(Request::SetEditedThemeExtends { extends }).await
    }

    pub(crate) async fn close_pack(&self) -> CallToolResult {
        self.change(Request::ClosePack).await
    }

    pub(crate) fn focus(&self, args: Value) -> CallToolResult {
        match self.parse::<FocusArgs>(args) {
            Ok(FocusArgs { slide, element }) => {
                self.hub.emit(McpEvent::Focus { slide, element });
                self.json_result(&json!({ "ok": true }))
            }
            Err(r) => r,
        }
    }
}

/// The pack to edit when the agent names none: the deck's main pack, in the interface's order.
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

/// Why the agent's gate refused a pack open.
#[derive(Debug, Clone, Copy)]
enum Refusal {
    /// No open turn: the call arrived outside one, or its turn is over.
    NoTurn,
    /// The turn already claimed another pack tab.
    Limit,
}

/// What the agent's gate answered while a pack opened.
#[derive(Debug, Default)]
struct Verdict {
    /// Why the gate refused, if it did.
    refusal: Option<Refusal>,
    /// A claim on a new pack tab was granted (and must be given back if the open fails).
    claimed: bool,
    /// Taking the focus of another tab was allowed.
    focused: bool,
}

impl Verdict {
    /// The gate: answers `gate` for `pack` in the call's `turn`, and records the answer.
    ///
    /// It never blocks; the hub lock it takes is a leaf.
    fn ask(&mut self, hub: &SurfaceHub, turn: Option<u32>, pack: uuid::Uuid, gate: PackGate) -> bool {
        let Some(turn) = turn else {
            self.refusal = Some(Refusal::NoTurn);
            return false;
        };
        match gate {
            PackGate::Focus => {
                let open = hub.turn_is_open(turn);
                self.focused |= open;
                if !open {
                    self.refusal = Some(Refusal::NoTurn);
                }
                open
            }
            PackGate::NewTab => match hub.reserve_pack_tab(turn, pack) {
                Reserve::Granted => {
                    self.claimed = true;
                    true
                }
                Reserve::NotThisTurn => {
                    self.refusal = Some(Refusal::NoTurn);
                    false
                }
                Reserve::Taken => {
                    self.refusal = Some(Refusal::Limit);
                    false
                }
            },
        }
    }

    /// The error that tells the agent why the gate refused.
    fn refusal(&self) -> ToolError {
        match self.refusal {
            Some(Refusal::Limit) => PackTabLimitSnafu.build(),
            Some(Refusal::NoTurn) => PackTabNoTurnSnafu.build(),
            None => {
                tracing::error!("pack open refused without a gate verdict");
                PackTabNoTurnSnafu.build()
            }
        }
    }
}

#[cfg(test)]
#[path = "edit_tools_tests.rs"]
mod edit_tools_tests;
