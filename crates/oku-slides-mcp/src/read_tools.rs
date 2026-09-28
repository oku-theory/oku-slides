//! Outils de lecture : ils ne changent rien.

use oku_slides_session::Request;
use rust_mcp_sdk::schema::CallToolResult;
use serde_json::{Value, json};

use crate::tools::{SlideId, Tools, fail, json_result, parse, png, text};

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
                    "theme": snap.theme,
                    "themes": snap.themes,
                    "template_set": snap.templates,
                    "dirty": snap.dirty,
                    "slides": slides,
                }))
            })
            .await;
        match r {
            Ok(v) => json_result(&v),
            Err(e) => fail(e),
        }
    }

    pub(crate) async fn slide(&self, args: Value) -> CallToolResult {
        let SlideId { id } = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        match self.with_session(move |s| s.handle(Request::Slide { id }).map_err(|e| e.one_line())).await {
            Ok(json) => text(json),
            Err(e) => fail(e),
        }
    }

    pub(crate) async fn render_slide(&self, args: Value) -> CallToolResult {
        let SlideId { id } = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        match self.with_session(move |s| s.png(&id).map_err(|e| e.one_line())).await {
            Ok(bytes) => png(&bytes),
            Err(e) => fail(e),
        }
    }

    pub(crate) async fn widgets(&self) -> CallToolResult {
        match self.with_session(|s| s.handle(Request::Widgets).map_err(|e| e.one_line())).await {
            Ok(json) => text(json),
            Err(e) => fail(e),
        }
    }

    pub(crate) async fn templates(&self) -> CallToolResult {
        match self.with_session(|s| s.handle(Request::TemplateSets).map_err(|e| e.one_line())).await {
            Ok(json) => text(json),
            Err(e) => fail(e),
        }
    }

    pub(crate) async fn settings(&self) -> CallToolResult {
        json_result(&self.settings_read().await)
    }
}
