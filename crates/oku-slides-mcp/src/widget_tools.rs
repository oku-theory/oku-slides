//! Outils des widgets du pack : statut, source, écriture, suppression et
//! aperçu. Disponibles en mode deck comme en mode modèles.
//!
//! Les erreurs reviennent dans la langue de l'utilisateur, comme celles de tous les outils
//! ([`Tools::fail_with`]) : nom invalide, widget introuvable ou utilisé par un modèle, limites
//! de taille et de nombre, caractère nul.

use oku_slides_session::{PreviewFormat, Request};
use rust_mcp_sdk::schema::{CallToolResult, TextContent};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value};

use snafu::ResultExt;

use crate::McpEvent;
use crate::tool_error::ResponseSnafu;
use crate::tools::{Tools, png, text};

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct WidgetName {
    /// Nom du fichier : `[a-z][a-z0-9-]*` (le nom du widget), ou `_commun` pour le code partagé.
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SetWidgetArgs {
    /// Nom du fichier : `[a-z][a-z0-9-]*` (le nom du widget), ou `_commun`.
    pub name: String,
    /// Source Typst complète du fichier ; vide : le fichier est supprimé.
    pub source: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PreviewWidgetArgs {
    /// Nom du widget à montrer.
    pub name: String,
    /// Valeurs de paramètres ; les autres prennent leur défaut ou un exemple.
    #[serde(default)]
    pub params: Map<String, Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SetWidgetTrialArgs {
    /// Nom du widget.
    pub name: String,
    /// Valeurs d'exemple par paramètre ; `null` efface une valeur (retour à
    /// celle de l'aperçu) ; un texte vide ou `[]` (texte riche ou liste) la
    /// gardent vide.
    pub values: Map<String, Value>,
}

impl Tools {
    /// Une requête de widgets, sa réponse JSON telle quelle.
    async fn widget_request(&self, req: Request) -> CallToolResult {
        self.reply(self.with_session(move |s| s.handle(req)).await, text)
    }

    pub(crate) async fn widget_sources(&self) -> CallToolResult {
        self.widget_request(Request::WidgetStatus).await
    }

    pub(crate) async fn widget_source(&self, args: Value) -> CallToolResult {
        match self.parse::<WidgetName>(args) {
            Ok(WidgetName { name }) => self.reply(self.with_session(move |s| s.widget_source(&name)).await, text),
            Err(r) => r,
        }
    }

    pub(crate) async fn set_widget(&self, args: Value) -> CallToolResult {
        let SetWidgetArgs { name, source } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let removing = source.is_empty();
        let done = {
            let name = name.clone();
            self.with_session(move |s| s.set_widget_by_agent(&name, source).map(|c| (c, s.in_templates()))).await
        };
        let (change, templates) = match done {
            Ok(d) => d,
            Err(e) => return self.fail_with(e),
        };
        // Le texte de l'utilisateur est mis en file avant que l'instantané soit relu.
        if templates && change.saved {
            self.hub.emit(McpEvent::WidgetTouched { name, removed: removing });
        }
        self.hub.emit(McpEvent::Changed);
        self.json_result(&change)
    }

    pub(crate) async fn remove_widget(&self, args: Value) -> CallToolResult {
        let WidgetName { name } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let done = {
            let name = name.clone();
            // Le mode est lu sous le même verrou que la suppression.
            self.with_session(move |s| {
                let templates = s.in_templates();
                s.handle(Request::RemoveWidget { name }).map(|json| (json, templates))
            })
            .await
        };
        match done {
            Ok((json, templates)) => {
                self.hub.emit(McpEvent::Changed);
                if templates {
                    self.hub.emit(McpEvent::WidgetTouched { name, removed: true });
                }
                text(json)
            }
            Err(e) => self.fail_with(e),
        }
    }

    /// Valeurs d'exemple d'un widget (mode modèles, dans le pack) ;
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
        let PreviewWidgetArgs { name, params } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let preview = self.with_session(move |s| s.preview_widget(&name, &params, PreviewFormat::Png)).await;
        let preview = match preview {
            Ok(p) => p,
            Err(e) => return self.fail_with(e),
        };
        let diagnostics = match serde_json::to_string(&preview.diagnostics).context(ResponseSnafu) {
            Ok(json) => json,
            Err(e) => return self.fail_with(e),
        };
        let Some(bytes) = &preview.png else { return text(diagnostics) };
        let mut result = png(bytes);
        if !preview.diagnostics.is_empty() {
            result.content.push(TextContent::new(diagnostics, None, None).into());
        }
        result
    }
}
