//! Outils des widgets du pack : statut, source, écriture, suppression et
//! aperçu. Disponibles en mode deck comme en mode pack.
//!
//! Les erreurs reviennent dans la langue de l'utilisateur, comme celles de tous les outils
//! ([`Tools::fail_with`]) : nom invalide, widget introuvable ou utilisé par un exemple, limites
//! de taille et de nombre, caractère nul.

use oku_slides_core::Namespace;
use oku_slides_session::{PreviewFormat, Request};
use rust_mcp_sdk::schema::{CallToolResult, TextContent};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value};

use snafu::ResultExt;

use crate::McpEvent;
use crate::tool_error::{ContinueInPackTabSnafu, ResponseSnafu};
use crate::tools::{Tools, png, text};

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PackArgs {
    /// Namespace of a pack pinned by the deck; absent: the `deck` pack.
    #[serde(default)]
    pub pack: Option<Namespace>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct WidgetName {
    /// Namespace of a pack pinned by the deck; absent: the `deck` pack.
    #[serde(default)]
    pub pack: Option<Namespace>,
    /// File name: `[a-z][a-z0-9-]*` (the widget's name), or `_commun` for shared code.
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SetWidgetArgs {
    /// Namespace of a pack pinned by the deck.
    ///
    /// Absent: the `deck` pack, created on the first write.
    #[serde(default)]
    pub pack: Option<Namespace>,
    /// File name: `[a-z][a-z0-9-]*` (the widget's name), or `_commun`.
    pub name: String,
    /// Complete Typst source of the file; empty: the file is deleted.
    pub source: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PreviewWidgetArgs {
    /// Namespace of a pack pinned by the deck; absent: the `deck` pack.
    #[serde(default)]
    pub pack: Option<Namespace>,
    /// Name of the widget to show.
    pub name: String,
    /// Parameter values; the others take their default or an example.
    #[serde(default)]
    pub params: Map<String, Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SetWidgetTrialArgs {
    /// Name of the widget.
    pub name: String,
    /// Example values by parameter.
    ///
    /// `null` clears a value, back to the preview's; an empty text or `[]`
    /// (rich text or list) keeps it empty.
    pub values: Map<String, Value>,
}

impl Tools {
    /// Une requête de widgets, sa réponse JSON telle quelle.
    async fn widget_request(&self, req: Request) -> CallToolResult {
        self.reply(self.with_session(move |s| s.handle(req)).await, text)
    }

    pub(crate) async fn widget_sources(&self, args: Value) -> CallToolResult {
        match self.parse::<PackArgs>(args) {
            Ok(PackArgs { pack }) => self.widget_request(Request::WidgetStatus { pack }).await,
            Err(r) => r,
        }
    }

    pub(crate) async fn widget_source(&self, args: Value) -> CallToolResult {
        match self.parse::<WidgetName>(args) {
            Ok(WidgetName { pack, name }) => {
                self.reply(self.with_session(move |s| s.widget_source(pack.as_ref(), &name)).await, text)
            }
            Err(r) => r,
        }
    }

    /// The refusal of a deck-mode widget write in a turn that went on in a pack tab, if it applies.
    ///
    /// Once `edit_pack` opened or focused a pack's tab in `turn` (its claim counts as soon as it is
    /// granted), this tab leaves its widgets alone: the request continues in the pack's tab.
    fn continues_in_pack_tab(&self, turn: Option<u32>) -> Option<CallToolResult> {
        turn.filter(|turn| self.hub.writes_elsewhere(*turn)).map(|_| self.fail_with(&ContinueInPackTabSnafu.build()))
    }

    pub(crate) async fn set_widget(&self, turn: Option<u32>, args: Value) -> CallToolResult {
        let SetWidgetArgs { pack, name, source } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        if let Some(refused) = self.continues_in_pack_tab(turn) {
            return refused;
        }
        let removing = source.is_empty();
        let done = {
            let name = name.clone();
            self.with_session(move |s| {
                s.set_widget_by_agent(pack.as_ref(), &name, source).map(|c| (c, s.in_pack_mode()))
            })
            .await
        };
        let (change, pack_mode) = match done {
            Ok(d) => d,
            Err(e) => return self.fail_with(&e),
        };
        // Le texte de l'utilisateur est mis en file avant que l'instantané soit relu.
        if pack_mode && change.saved {
            self.hub.emit(McpEvent::WidgetTouched { name, removed: removing });
        }
        self.hub.emit(McpEvent::Changed);
        self.json_result(&change)
    }

    pub(crate) async fn remove_widget(&self, turn: Option<u32>, args: Value) -> CallToolResult {
        let WidgetName { pack, name } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        if let Some(refused) = self.continues_in_pack_tab(turn) {
            return refused;
        }
        let done = {
            let name = name.clone();
            // Le mode est lu sous le même verrou que la suppression.
            self.with_session(move |s| {
                let pack_mode = s.in_pack_mode();
                s.handle(Request::RemoveWidget { pack, name }).map(|json| (json, pack_mode))
            })
            .await
        };
        match done {
            Ok((json, pack_mode)) => {
                self.hub.emit(McpEvent::Changed);
                if pack_mode {
                    self.hub.emit(McpEvent::WidgetTouched { name, removed: true });
                }
                text(json)
            }
            Err(e) => self.fail_with(&e),
        }
    }

    /// Valeurs d'exemple d'un widget (mode pack, dans le pack) ;
    /// l'utilisateur les voit dans ses réglages et dans l'aperçu.
    pub(crate) async fn set_widget_trial(&self, args: Value) -> CallToolResult {
        let SetWidgetTrialArgs { name, values } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let trials = self.with_session(move |s| s.set_example_values_by_agent(&name, values)).await;
        self.reply(trials, |trials| {
            self.hub.emit(McpEvent::Changed);
            self.json_result(&serde_json::json!({ "trials": trials }))
        })
    }

    /// L'image PNG du widget, suivie de ses diagnostics s'il en a.
    pub(crate) async fn preview_widget(&self, args: Value) -> CallToolResult {
        let PreviewWidgetArgs { pack, name, params } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let preview =
            self.with_session(move |s| s.preview_widget(pack.as_ref(), &name, &params, PreviewFormat::Png)).await;
        let preview = match preview {
            Ok(p) => p,
            Err(e) => return self.fail_with(&e),
        };
        let diagnostics = match serde_json::to_string(&preview.diagnostics).context(ResponseSnafu) {
            Ok(json) => json,
            Err(e) => return self.fail_with(&e),
        };
        let Some(bytes) = &preview.png else { return text(diagnostics) };
        let mut result = png(bytes);
        if !preview.diagnostics.is_empty() {
            result.content.push(TextContent::new(diagnostics, None, None).into());
        }
        result
    }
}
