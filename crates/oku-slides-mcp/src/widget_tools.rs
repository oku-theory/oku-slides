//! Outils des widgets du pack : statut, source, écriture, suppression et
//! aperçu. Disponibles en mode deck comme en mode modèles.
//!
//! Les erreurs reviennent dans la langue de l'utilisateur (Fluent, via
//! [`Localize`]) : nom invalide, widget introuvable ou utilisé par un
//! modèle, limites de taille et de nombre, caractère nul.

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_session::{PreviewFormat, Request, Session};
use rust_mcp_sdk::schema::{CallToolResult, TextContent};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::McpEvent;
use crate::tools::{Tools, fail, parse, png, text};

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
    /// Valeurs d'essai par paramètre ; `null`, un texte vide ou `[]` efface un essai.
    pub values: Map<String, Value>,
}

/// La tâche de la session a échoué (panique, annulation) : journalisé, puis
/// dit dans la langue de l'utilisateur.
fn task_failed(l: &Localizer, e: &tokio::task::JoinError) -> String {
    tracing::error!(error = %e, "tâche de la session d'un outil de widgets");
    l.tr("agent-tool-failed")
}

impl Tools {
    /// [`Tools::with_session`], l'erreur de la session (onglet fermé compris)
    /// dite dans la langue de l'utilisateur.
    async fn with_widgets<T: Send + 'static>(
        &self,
        f: impl FnOnce(&mut Session) -> oku_slides_session::Result<T> + Send + 'static,
    ) -> Result<T, String> {
        let l = self.localizer.clone();
        let session = self.session.clone();
        tokio::task::spawn_blocking(move || session.with(f).and_then(|r| r).map_err(|e| e.localize(&l)))
            .await
            .map_err(|e| task_failed(&self.localizer, &e))?
    }

    /// Une requête de widgets, sa réponse JSON telle quelle.
    async fn widget_request(&self, req: Request) -> CallToolResult {
        match self.with_widgets(move |s| s.handle(req)).await {
            Ok(json) => text(json),
            Err(e) => fail(e),
        }
    }

    pub(crate) async fn widget_sources(&self) -> CallToolResult {
        self.widget_request(Request::WidgetStatus).await
    }

    pub(crate) async fn widget_source(&self, args: Value) -> CallToolResult {
        match parse::<WidgetName>(args) {
            Ok(WidgetName { name }) => match self.with_widgets(move |s| s.widget_source(&name)).await {
                Ok(source) => text(source),
                Err(e) => fail(e),
            },
            Err(r) => r,
        }
    }

    /// Le résultat JSON d'une valeur, ou l'erreur de réponse de la session.
    fn json_text<T: serde::Serialize>(&self, value: &T) -> CallToolResult {
        match serde_json::to_string(value) {
            Ok(json) => text(json),
            Err(e) => {
                tracing::warn!(error = %e, "réponse d'outil de widget non sérialisable");
                fail(self.localizer.tr("session-response"))
            }
        }
    }

    pub(crate) async fn set_widget(&self, args: Value) -> CallToolResult {
        let SetWidgetArgs { name, source } = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let removing = source.is_empty();
        let done = {
            let name = name.clone();
            self.with_widgets(move |s| s.set_widget_by_agent(&name, source).map(|c| (c, s.in_templates()))).await
        };
        let (change, templates) = match done {
            Ok(d) => d,
            Err(e) => return fail(e),
        };
        // Le texte de l'utilisateur est mis en file avant que l'instantané soit relu.
        if templates && change.saved {
            self.hub.emit(McpEvent::WidgetTouched { name, removed: removing });
        }
        self.hub.emit(McpEvent::Changed);
        self.json_text(&change)
    }

    pub(crate) async fn remove_widget(&self, args: Value) -> CallToolResult {
        let WidgetName { name } = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let done = {
            let name = name.clone();
            // Le mode est lu sous le même verrou que la suppression.
            self.with_widgets(move |s| {
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
            Err(e) => fail(e),
        }
    }

    /// Valeurs d'essai d'un widget (mode modèles) ; l'utilisateur les voit dans ses réglages.
    pub(crate) async fn set_widget_trial(&self, args: Value) -> CallToolResult {
        let SetWidgetTrialArgs { name, values } = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        match self.with_widgets(move |s| s.set_widget_trials(&name, values)).await {
            Ok(trials) => {
                self.hub.emit(McpEvent::Changed);
                self.json_text(&serde_json::json!({ "trials": trials }))
            }
            Err(e) => fail(e),
        }
    }

    /// L'image PNG du widget, suivie de ses diagnostics s'il en a.
    pub(crate) async fn preview_widget(&self, args: Value) -> CallToolResult {
        let PreviewWidgetArgs { name, params } = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let preview = self.with_widgets(move |s| s.preview_widget(&name, &params, PreviewFormat::Png)).await;
        let preview = match preview {
            Ok(p) => p,
            Err(e) => return fail(e),
        };
        let diagnostics = match serde_json::to_string(&preview.diagnostics) {
            Ok(json) => json,
            Err(e) => {
                tracing::warn!(error = %e, "diagnostics de l'aperçu non sérialisables");
                return fail(self.localizer.tr("session-response"));
            }
        };
        let Some(bytes) = &preview.png else { return text(diagnostics) };
        let mut result = png(bytes);
        if !preview.diagnostics.is_empty() {
            result.content.push(TextContent::new(diagnostics, None, None).into());
        }
        result
    }
}

#[cfg(test)]
#[path = "widget_tools_tests.rs"]
mod widget_tools_tests;
