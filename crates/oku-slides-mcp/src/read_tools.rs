//! Outils de lecture : ils ne changent rien.

use oku_slides_session::Request;
use rust_mcp_sdk::schema::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::tools::{SlideId, Tools, png, text};

/// Le thème lu par `theme_tokens`.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ThemeArgs {
    /// Id du thème (UUID, voir `deck_outline`).
    #[schemars(with = "String")]
    pub theme: uuid::Uuid,
}

impl Tools {
    pub(crate) async fn deck_outline(&self) -> CallToolResult {
        let r = self
            .with_session(|s| {
                let snap = s.snapshot();
                let deck = s.deck();
                let slides: Vec<Value> = snap
                    .slides
                    .iter()
                    .map(|sl| {
                        let elements = deck.slide(sl.id).map(|sl| sl.element_ids().count()).unwrap_or(0);
                        json!({
                            "id": sl.id,
                            "kind": sl.kind,
                            "title": sl.title,
                            "elements": elements,
                        })
                    })
                    .collect();
                Ok(json!({
                    "mode": match (&snap.history, &snap.editing) {
                        (Some(_), _) => "history",
                        (None, Some(_)) => "templates",
                        (None, None) => "deck",
                    },
                    "theme": snap.theme.map(|r| json!({ "id": r.id, "name": snap.theme_name })),
                    // En mode modèles : le thème dont part le brouillon (`set_theme` le change).
                    "edited_theme": snap.editing.as_ref().map(|e| json!({
                        "id": e.theme.base.id,
                        "version": e.theme.base.version,
                        "name": e.theme.name,
                        "dirty": e.theme.dirty,
                    })),
                    // Version courante de chaque thème : ce que pose l'op `set_theme`. Puis ceux
                    // du deck seul embarqués (l'import), dans la version que reprendrait le deck.
                    "themes": snap
                        .themes
                        .iter()
                        .chain(&snap.deck_themes)
                        .map(|t| json!({ "id": t.id, "name": t.name, "version": t.version, "deck_only": t.deck_only }))
                        .collect::<Vec<_>>(),
                    "widget_packs": snap.widgets,
                    "dirty": snap.dirty,
                    "slides": slides,
                }))
            })
            .await;
        self.reply(r, |v| self.json_result(&v))
    }

    /// Les jetons propres d'un thème, dans la version d'où partirait `edit_theme`.
    pub(crate) async fn theme_tokens(&self, args: Value) -> CallToolResult {
        let ThemeArgs { theme } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let r = self
            .with_session(move |s| {
                let info = s.theme_info(theme)?;
                let tokens = s.theme_tokens_of(theme)?;
                Ok(json!({
                    "id": info.id,
                    "version": info.version,
                    "name": info.name,
                    "deck_only": info.deck_only,
                    "tokens": tokens,
                }))
            })
            .await;
        self.reply(r, |v| self.json_result(&v))
    }

    pub(crate) async fn slide(&self, args: Value) -> CallToolResult {
        let SlideId { id } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        self.reply(self.with_session(move |s| s.handle(Request::Slide { id })).await, text)
    }

    pub(crate) async fn render_slide(&self, args: Value) -> CallToolResult {
        let SlideId { id } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        self.reply(self.with_session(move |s| s.png(&id)).await, |bytes| png(&bytes))
    }

    pub(crate) async fn widgets(&self) -> CallToolResult {
        self.reply(self.with_session(|s| s.handle(Request::Widgets)).await, text)
    }

    pub(crate) async fn templates(&self) -> CallToolResult {
        self.reply(self.with_session(|s| s.handle(Request::TemplateSets)).await, text)
    }

    pub(crate) async fn settings(&self) -> CallToolResult {
        self.json_result(&self.settings_read().await)
    }
}
