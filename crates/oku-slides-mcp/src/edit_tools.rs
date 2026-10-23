//! Outils qui modifient le deck ; tout s'annule (dans le groupe du tour s'il est ouvert).

use oku_slides_chart::SeriesType;
use std::collections::BTreeMap;

use oku_slides_core::{Namespace, Op, PackKind, PackPin};
use oku_slides_session::Request;
use rust_mcp_sdk::schema::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use snafu::{IntoError, ResultExt};

use crate::McpEvent;
use crate::tool_error::{NoPackSnafu, PackChangeSnafu, PackIdSnafu, ToolError};
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
pub(crate) struct PackIdArgs {
    /// Id d'un pack (UUID, voir `packs`).
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
    /// Id d'un pack (UUID, voir `packs`) ; absent : le pack principal du deck (son pack de layouts
    /// propre, sinon son pack de widgets propre, sinon un pack non intégré, sinon Essentiel).
    #[serde(default)]
    pub pack: Option<String>,
}

/// `add_pack` et `remove_pack` : le pack visé.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PackRefArgs {
    /// `widgets` (éléments) ou `layouts` (slides).
    pub kind: PackKind,
    /// Espace de noms du pack (voir `packs` et `deck_outline`).
    pub namespace: Namespace,
}

/// `set_theme_extends` : les parents du thème édité.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ThemeExtendsArgs {
    /// Espaces de noms des thèmes de la bibliothèque, dans l'ordre (les derniers l'emportent).
    pub extends: Vec<Namespace>,
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

    pub(crate) async fn edit_pack(&self, args: Value) -> CallToolResult {
        let pack = match self.parse::<EditPackArgs>(args) {
            Ok(EditPackArgs { pack: Some(pack) }) => PackIdArgs { pack }.id(),
            // Le pack du deck, choisi comme l'interface (`firstPackId`).
            Ok(EditPackArgs { pack: None }) => {
                match self.with_session(|s| Ok(main_pack(s.snapshot().widgets, s.snapshot().layouts))).await {
                    Ok(Some(pack)) => Ok(pack),
                    Ok(None) => Err(NoPackSnafu.build()),
                    Err(e) => Err(e),
                }
            }
            Err(r) => return r,
        };
        match pack {
            Ok(pack) => self.change(Request::EditPack { pack }).await,
            Err(e) => self.fail_with(e),
        }
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

/// Le pack à éditer quand l'agent n'en nomme pas, choisi comme `firstPackId` côté interface.
///
/// Le pack de layouts propre du deck, sinon son pack de widgets propre, sinon le premier pack non
/// intégré (layouts d'abord), sinon Essentiel, enfin `base`.
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
#[path = "edit_tools_tests.rs"]
mod edit_tools_tests;
