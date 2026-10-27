//! Outils de lecture : ils ne changent rien.

use oku_slides_core::PackKind;
use oku_slides_session::Request;
use rust_mcp_sdk::schema::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use snafu::ResultExt;

use crate::tool_error::{ResponseSnafu, ToolError};
use crate::tools::{SlideId, Tools, png, text};

/// The theme read by `theme_tokens`.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ThemeArgs {
    /// Namespace of the theme (see `deck_outline`); its id or its name are understood too.
    pub theme: String,
}

/// The kind of packs read by `packs`.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PacksArgs {
    /// `widgets` (elements) or `layouts` (slides); absent: both, widgets first.
    #[serde(default)]
    pub kind: Option<PackKind>,
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
                        let elements = deck.slide(sl.id).map_or(0, |sl| sl.element_ids().count());
                        json!({
                            "id": sl.id,
                            "kind": sl.kind,
                            "title": sl.title,
                            "elements": elements,
                        })
                    })
                    .collect();
                Ok(json!({
                    "mode": match (&snap.history, &snap.editing, &snap.theming) {
                        (Some(_), _, _) => "history",
                        (None, Some(_), _) => "pack",
                        (None, None, Some(_)) => "theme",
                        (None, None, None) => "deck",
                    },
                    "theme": snap.theme.map(|ns| json!({ "namespace": ns, "name": snap.theme_name })),
                    // In a theme tab: the draft and the theme it starts from.
                    "edited_theme": snap.theming.as_ref().map(|e| json!({
                        "id": e.theme.base.id,
                        "version": e.theme.base.version,
                        "namespace": e.theme.namespace,
                        "name": e.theme.name,
                        "child": e.theme.child,
                        "extends": e.theme.extends,
                        "dirty": e.theme.dirty,
                    })),
                    // Les thèmes de la bibliothèque : ce que désigne l'op `set_theme` par son espace de noms.
                    "themes": snap
                        .themes
                        .iter()
                        .map(|t| json!({ "namespace": t.namespace, "name": t.name, "extends": t.extends, "builtin": t.builtin }))
                        .collect::<Vec<_>>(),
                    "widget_packs": snap.widgets,
                    "layout_packs": snap.layouts,
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
                let info = s.find_theme_info(&theme)?;
                let tokens = s.theme_tokens_of(info.id)?;
                Ok(json!({
                    "namespace": info.namespace,
                    "extends": info.extends,
                    "name": info.name,
                    "version": info.version,
                    "tokens": tokens,
                    "deck_tokens": s.deck().tokens,
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

    /// Les packs de la bibliothèque, dans leur version courante, d'une sorte ou des deux.
    pub(crate) async fn packs(&self, args: Value) -> CallToolResult {
        let PacksArgs { kind } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let kinds = kind.map_or_else(|| PackKind::ALL.to_vec(), |k| vec![k]);
        let listed =
            self.with_session(move |s| kinds.into_iter().map(|kind| s.handle(Request::Packs { kind })).collect()).await;
        // Chaque sorte répond un tableau JSON : mis bout à bout.
        let packs = listed.and_then(|lists: Vec<String>| {
            lists.iter().try_fold(Vec::new(), |mut packs, list| {
                packs.extend(serde_json::from_str::<Vec<Value>>(list).context(ResponseSnafu)?);
                Ok::<_, ToolError>(packs)
            })
        });
        self.reply(packs, |v| self.json_result(&v))
    }

    /// Les layouts des packs de layouts du deck, sans leurs aperçus.
    pub(crate) async fn layouts(&self) -> CallToolResult {
        let r = self
            .with_session(|s| {
                let view = s.layouts_view()?;
                let packs: Vec<Value> = view
                    .packs
                    .iter()
                    .map(|p| {
                        let layouts: Vec<Value> =
                            p.layouts.iter().map(|l| json!({ "kind": l.kind, "label": l.label })).collect();
                        json!({ "namespace": p.namespace, "name": p.name, "layouts": layouts })
                    })
                    .collect();
                Ok(json!({ "packs": packs }))
            })
            .await;
        self.reply(r, |v| self.json_result(&v))
    }

    pub(crate) async fn settings(&self) -> CallToolResult {
        self.json_result(&self.settings_read().await)
    }
}
