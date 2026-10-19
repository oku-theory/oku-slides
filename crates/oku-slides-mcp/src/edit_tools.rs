//! Outils qui modifient le deck ; tout s'annule (dans le groupe du tour s'il est ouvert).

use oku_slides_chart::SeriesType;
use std::collections::BTreeMap;

use oku_slides_core::{Namespace, Op, PackPin};
use oku_slides_session::Request;
use rust_mcp_sdk::schema::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use snafu::ResultExt;

use crate::McpEvent;
use crate::tool_error::{NoPackSnafu, PackIdSnafu, ToolError};
use crate::tools::Tools;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ApplyArgs {
    /// Ops du deck, appliquées ensemble (tout ou rien). Voir le schéma de chaque op.
    pub ops: Vec<Op>,
    /// Libellé d'annulation hors d'un tour (dans un tour, celui du tour).
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ShowAsArgs {
    pub slide: String,
    pub element: String,
    /// `table` ou `chart` ; les données sont gardées.
    pub view: oku_slides_session::View,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ChartKindArgs {
    pub slide: String,
    pub element: String,
    /// Nouveau type ; les données sont converties.
    pub kind: SeriesType,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SetArgs {
    /// Id d'un pack de modèles (UUID, voir `templates`).
    pub set: String,
}

impl SetArgs {
    /// L'id du pack ; un texte qui n'est pas un UUID est refusé par un
    /// message qui renvoie l'agent vers `templates`.
    pub(crate) fn pack(&self) -> Result<uuid::Uuid, ToolError> {
        uuid::Uuid::parse_str(&self.set).context(PackIdSnafu)
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct EditTemplatesArgs {
    /// Id d'un pack de modèles (UUID, voir `templates`) ; absent : le pack principal du deck (un
    /// pack de la bibliothèque autre que `base`, sinon son pack propre).
    #[serde(default)]
    pub set: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct FocusArgs {
    pub slide: String,
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

    pub(crate) async fn edit_templates(&self, args: Value) -> CallToolResult {
        let set = match self.parse::<EditTemplatesArgs>(args) {
            Ok(EditTemplatesArgs { set: Some(set) }) => SetArgs { set }.pack(),
            // Le pack du deck, choisi comme l'interface (`firstPackId`) : un pack ni `deck` ni
            // `base`, sinon un autre que `base`, sinon le premier épinglé.
            Ok(EditTemplatesArgs { set: None }) => {
                match self.with_session(|s| Ok(main_pack(s.snapshot().widgets))).await {
                    Ok(Some(set)) => Ok(set),
                    Ok(None) => Err(NoPackSnafu.build()),
                    Err(e) => Err(e),
                }
            }
            Err(r) => return r,
        };
        match set {
            Ok(set) => self.change(Request::EditTemplates { set }).await,
            Err(e) => self.fail_with(e),
        }
    }

    pub(crate) async fn close_templates(&self) -> CallToolResult {
        self.change(Request::CloseTemplates).await
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

/// Le pack à éditer quand l'agent n'en nomme pas, choisi comme `firstPackId` côté interface :
/// un pack ni `deck` ni `base`, sinon un autre que `base`, sinon le premier épinglé.
pub(crate) fn main_pack(pins: &BTreeMap<Namespace, PackPin>) -> Option<uuid::Uuid> {
    let is_base = |ns: &Namespace| ns.as_str() == oku_slides_core::kinds::base::NAMESPACE;
    pins.iter()
        .find(|(ns, _)| !ns.is_deck() && !is_base(ns))
        .or_else(|| pins.iter().find(|(ns, _)| !is_base(ns)))
        .or_else(|| pins.iter().next())
        .map(|(_, pin)| pin.id)
}

#[cfg(test)]
#[path = "edit_tools_tests.rs"]
mod edit_tools_tests;
