//! Outils de l'app hors du deck : historique, présentation et interface.
//!
//! Le menu principal ne garde que Présenter, Ouvrir et Aide : le reste se demande à l'agent
//! (t-988). Présenter, l'interface et l'ouverture de l'historique vivent dans la page : l'outil
//! envoie un évènement que l'onglet applique par le même chemin que ses propres commandes
//! (saisie en cours validée, enregistrement en attente parti d'abord). Seul l'onglet affiché
//! de sa fenêtre les reçoit : un autre est refusé.

use oku_slides_session::Request;
use rust_mcp_sdk::schema::CallToolResult;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use snafu::{OptionExt, ResultExt, ensure};

use crate::McpEvent;
use crate::tool_error::{
    EmptyInterfaceSnafu, HistoryIndexSnafu, HistoryNotOpenedSnafu, NotShownSnafu, NothingToPresentSnafu, TaskSnafu,
    ToolError, UnknownLocaleSnafu,
};
use crate::tools::{Tools, text};

/// Délai laissé à l'onglet pour ouvrir l'historique (saisie validée, enregistrement parti).
const HISTORY_OPEN_WAIT: std::time::Duration = std::time::Duration::from_secs(5);
/// Pas de la relecture de la session pendant cette attente.
const HISTORY_OPEN_POLL: std::time::Duration = std::time::Duration::from_millis(50);

/// Où se montre la présentation (contrat de `McpEvent::Present`, repris par `ui/src/api.ts`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PresentMode {
    /// Tout l'écran.
    #[default]
    Screen,
    /// Toute la fenêtre.
    Window,
}

/// Schéma de couleurs de l'interface (contrat de `McpEvent::Interface`) : les schémas que
/// l'espace de travail sait imposer (`light`, `dark`), plus `system` (aucun choix).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SchemeChoice {
    /// Clair.
    Light,
    /// Sombre.
    Dark,
    /// Celui du système.
    System,
}

/// Ce que l'agent change dans l'interface ; absent : inchangé.
///
/// Contrat de `McpEvent::Interface`, repris par `ui/src/api.ts` : un champ absent part en `null`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InterfaceChange {
    /// Schéma de couleurs.
    #[serde(default)]
    pub scheme: Option<SchemeChoice>,
    /// Langue : un code livré (`en`, `fr`…) ou `system`.
    #[serde(default)]
    pub locale: Option<String>,
    /// Grille de la slide montrée.
    #[serde(default)]
    pub grid: Option<bool>,
    /// Repères des champs vides.
    #[serde(default)]
    pub guides: Option<bool>,
}

impl InterfaceChange {
    /// Aucun changement demandé.
    fn is_empty(&self) -> bool {
        self.scheme.is_none() && self.locale.is_none() && self.grid.is_none() && self.guides.is_none()
    }
}

/// Arguments de `present`.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PresentArgs {
    /// `screen` (défaut) ou `window`.
    #[serde(default)]
    pub mode: PresentMode,
}

/// Ce que fait l'outil `history`.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HistoryAction {
    /// Ouvre l'historique dans l'onglet (sur la dernière version).
    Open,
    /// Les versions, de la plus ancienne à la plus récente ; historique ouvert.
    List,
    /// Montre la version `index`.
    Seek,
    /// Ramène le deck à la version montrée, et ferme l'historique.
    Restore,
    /// Ferme l'historique, deck inchangé.
    Close,
}

/// Arguments de `history`.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct HistoryArgs {
    /// Ce que fait l'outil.
    pub action: HistoryAction,
    /// Pour `seek` : rang de la version (0 : la plus ancienne).
    #[serde(default)]
    pub index: Option<usize>,
}

impl Tools {
    /// Refusé si l'onglet n'est pas l'affiché de sa fenêtre : il n'applique pas les évènements
    /// de la page.
    async fn ensure_shown(&self) -> Result<(), ToolError> {
        let tab = self.session.clone();
        let shown = tokio::task::spawn_blocking(move || tab.shown()).await.context(TaskSnafu)?;
        ensure!(shown, NotShownSnafu);
        Ok(())
    }

    /// Ouvre (par l'onglet), liste, parcourt, restaure ou ferme l'historique.
    pub(crate) async fn history(&self, args: Value) -> CallToolResult {
        let HistoryArgs { action, index } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let req = match action {
            HistoryAction::Open => return self.history_open().await,
            HistoryAction::List => {
                return self.reply(self.with_session(|s| s.handle(Request::HistoryVersions)).await, text);
            }
            HistoryAction::Seek => match index.context(HistoryIndexSnafu) {
                Ok(index) => Request::HistorySeek { index },
                Err(e) => return self.fail_with(e),
            },
            HistoryAction::Restore => Request::HistoryRestore,
            HistoryAction::Close => Request::HistoryClose,
        };
        // Après `seek`, la version montrée (le rang demandé est borné à la dernière).
        let done = self
            .with_session(move |s| {
                s.handle(req)?;
                Ok(s.snapshot().history.map(|h| json!({ "index": h.index, "count": h.count })))
            })
            .await;
        self.reply(done, |shown| {
            self.hub.emit(McpEvent::Changed);
            self.json_result(&json!({ "ok": true, "shown": shown }))
        })
    }

    /// Demande à l'onglet d'ouvrir l'historique (sa saisie validée et son enregistrement parti
    /// d'abord, comme par ses propres commandes), attend qu'il soit ouvert et rend les versions.
    /// Refusé là où l'historique ne s'ouvre pas ; pas ouvert à temps : dit à l'agent.
    async fn history_open(&self) -> CallToolResult {
        if let Err(e) = self.ensure_shown().await {
            return self.fail_with(e);
        }
        if let Err(e) = self.with_session(|s| s.ensure_deck_mode()).await {
            return self.fail_with(e);
        }
        self.hub.emit(McpEvent::OpenHistory);
        let deadline = tokio::time::Instant::now() + HISTORY_OPEN_WAIT;
        loop {
            match self.with_session(|s| Ok(s.snapshot().history.is_some())).await {
                Ok(true) => return self.reply(self.with_session(|s| s.handle(Request::HistoryVersions)).await, text),
                Ok(false) if tokio::time::Instant::now() < deadline => tokio::time::sleep(HISTORY_OPEN_POLL).await,
                Ok(false) => return self.fail_with(HistoryNotOpenedSnafu.build()),
                Err(e) => return self.fail_with(e),
            }
        }
    }

    /// Lance la présentation dans l'onglet ; refusée là où la page ne présente pas.
    pub(crate) async fn present(&self, args: Value) -> CallToolResult {
        let PresentArgs { mode } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        if let Err(e) = self.ensure_shown().await {
            return self.fail_with(e);
        }
        let can = self
            .with_session(|s| {
                let snap = s.snapshot();
                Ok(!snap.slides.is_empty() && snap.editing.is_none() && snap.history.is_none() && !snap.read_only)
            })
            .await;
        match can {
            Ok(true) => {
                self.hub.emit(McpEvent::Present { mode });
                self.json_result(&json!({ "ok": true }))
            }
            Ok(false) => self.fail_with(NothingToPresentSnafu.build()),
            Err(e) => self.fail_with(e),
        }
    }

    /// Change le schéma, la langue, la grille ou les repères de l'interface.
    pub(crate) async fn set_interface(&self, args: Value) -> CallToolResult {
        let mut change: InterfaceChange = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        if change.is_empty() {
            return self.fail_with(EmptyInterfaceSnafu.build());
        }
        // L'interface ne reçoit qu'un code livré : `FR-ca` part en `fr`.
        if let Some(locale) = change.locale.as_deref() {
            change.locale = match oku_slides_i18n::shipped(locale) {
                _ if locale.eq_ignore_ascii_case("system") => Some("system".to_owned()),
                Some(code) => Some(code.to_owned()),
                None => return self.fail_with(UnknownLocaleSnafu.build()),
            };
        }
        if let Err(e) = self.ensure_shown().await {
            return self.fail_with(e);
        }
        self.hub.emit(McpEvent::Interface { change });
        self.json_result(&json!({ "ok": true }))
    }
}
