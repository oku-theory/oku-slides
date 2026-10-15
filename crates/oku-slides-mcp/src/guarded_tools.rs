//! Outils qui ne s'annulent pas : l'utilisateur confirme d'abord.

use std::path::{Path, PathBuf};

use oku_slides_session::{Request, Session, TokenOp};
use rust_mcp_sdk::schema::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use snafu::{IntoError, ResultExt};

use crate::McpEvent;
use crate::paths::checked;
use crate::settings::writable;
use crate::tool_error::{
    DeniedSnafu, NoFileSnafu, OpenElsewhereSnafu, SessionSnafu, SettingLockedSnafu, TaskSnafu, ToolError,
};
use crate::tools::{Tools, text};

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SaveArgs {
    /// Chemin absolu du `.oku-slides.pdf` ; absent : le fichier actuel.
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct OpenArgs {
    /// Chemin absolu d'un `.oku-slides.pdf`.
    pub path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SaveTemplatesArgs {
    /// Enregistrer comme nouvel ensemble.
    #[serde(default)]
    pub as_new: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct EditThemeArgs {
    /// Id du thème (UUID, voir `deck_outline`), édité dans sa version courante.
    #[schemars(with = "String")]
    pub theme: uuid::Uuid,
    /// Nouveau nom ; absent : le nom actuel (« (copie) » pour un thème intégré).
    #[serde(default)]
    pub name: Option<String>,
    /// Ops de jetons, appliquées dans l'ordre.
    #[serde(default)]
    pub ops: Vec<TokenOp>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SetSettingArgs {
    /// `dictation.model` (apple|turbo|large|base), `dictation.language` (code ou null), `dictation.auto_stop`, `dictation.silence`, `i18n.locale`, `history.limit`, `stt.vad.<clé>`.
    pub key: String,
    pub value: Value,
}

impl Tools {
    /// Demande à l'utilisateur, puis exécute `req` ; `Changed` si elle aboutit.
    async fn guarded(&self, message: String, danger: bool, req: Request) -> CallToolResult {
        self.guarded_with(message, danger, move |s| s.handle(req).map(|_| ())).await
    }

    /// Demande à l'utilisateur, puis appelle `f` sur la session de l'onglet ;
    /// `Changed` si elle aboutit.
    async fn guarded_with(
        &self,
        message: String,
        danger: bool,
        f: impl FnOnce(&mut Session) -> oku_slides_session::Result<()> + Send + 'static,
    ) -> CallToolResult {
        if !self.hub.confirm(message, danger).await {
            return self.denied();
        }
        let done = self.with_session(f).await;
        self.reply(done, |()| {
            self.hub.emit(McpEvent::Changed);
            self.json_result(&json!({ "ok": true }))
        })
    }

    /// L'utilisateur a refusé.
    fn denied(&self) -> CallToolResult {
        self.fail_with(DeniedSnafu.build())
    }

    /// `snapshot().dirty`, lu hors du fil async (le verrou de la session est
    /// synchrone : voir [`Tools::with_session`]).
    async fn dirty(&self) -> bool {
        self.with_session(|s| Ok(s.snapshot().dirty)).await.unwrap_or(false)
    }

    /// Un autre deck dans l'onglet : ses surfaces orphelines se ferment
    /// (`SurfaceHub::prune` verrouille la session : hors du fil async).
    async fn prune_if_done(&self, result: &CallToolResult) {
        if result.is_error == Some(true) {
            return;
        }
        let hub = self.hub.clone();
        if let Err(e) = tokio::task::spawn_blocking(move || hub.prune()).await {
            tracing::warn!(error = %e, "surfaces non élaguées");
        }
    }

    /// `path` est-il déjà ouvert dans un autre onglet ? Lu hors du fil async.
    async fn open_elsewhere(&self, path: &Path) -> bool {
        let (session, path) = (self.session.clone(), path.to_path_buf());
        tokio::task::spawn_blocking(move || session.open_elsewhere(&path)).await.unwrap_or(false)
    }

    /// Chemin du fichier ouvert, lu hors du fil async.
    async fn current_path(&self) -> Option<PathBuf> {
        self.with_session(|s| Ok(s.snapshot().path.map(Path::to_path_buf))).await.ok().flatten()
    }

    /// `Storage` (OpenDAL) bloque : à appeler seulement depuis `spawn_blocking`,
    /// jamais depuis le fil async (cf. `Tools::with_session`).
    async fn exists(&self, path: &Path) -> bool {
        let storage = self.storage.clone();
        let path = path.to_path_buf();
        tokio::task::spawn_blocking(move || storage.exists(&path).unwrap_or(false)).await.unwrap_or(false)
    }

    pub(crate) async fn save(&self, args: Value) -> CallToolResult {
        let SaveArgs { path } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let path = match path.as_deref().map(checked).transpose() {
            Ok(p) => p,
            Err(e) => return self.fail_with(e),
        };
        // Un fichier ne vit que dans un onglet : refusé sans rien demander.
        if let Some(p) = &path
            && self.open_elsewhere(p).await
        {
            return self.fail_with(OpenElsewhereSnafu.build());
        }
        let shown = match &path {
            Some(p) => p.display().to_string(),
            None => match self.current_path().await {
                Some(p) => p.display().to_string(),
                None => return self.fail_with(NoFileSnafu.build()),
            },
        };
        let exists = match &path {
            Some(p) => self.exists(p).await,
            None => false,
        };
        let key = if exists { "agent-confirm-save-over" } else { "agent-confirm-save" };
        let message = self.localizer.current().tr_with(key, &[("path", shown.into())]);
        self.guarded(message, exists, Request::Save { path, overwrite: false }).await
    }

    /// Ouvre le fichier dans un autre onglet (ou montre celui qui l'a) ;
    /// l'onglet de l'agent garde son deck et sa conversation.
    pub(crate) async fn open(&self, args: Value) -> CallToolResult {
        let OpenArgs { path } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let path = match checked(&path) {
            Ok(p) => p,
            Err(e) => return self.fail_with(e),
        };
        let message =
            self.localizer.current().tr_with("agent-confirm-open", &[("path", path.display().to_string().into())]);
        if !self.hub.confirm(message, false).await {
            return self.denied();
        }
        let session = self.session.clone();
        let opened: Result<String, ToolError> = match tokio::task::spawn_blocking(move || session.open_tab(&path)).await
        {
            Ok(r) => r.context(SessionSnafu),
            Err(e) => Err(TaskSnafu.into_error(e)),
        };
        self.reply(opened, |tab| self.json_result(&json!({ "ok": true, "tab": tab })))
    }

    pub(crate) async fn new_deck(&self) -> CallToolResult {
        let dirty = self.dirty().await;
        let message = self.localizer.current().tr(if dirty { "agent-confirm-new-dirty" } else { "agent-confirm-new" });
        let result = self.guarded_with(message, dirty, Session::new_deck).await;
        self.prune_if_done(&result).await;
        result
    }

    pub(crate) async fn save_templates(&self, args: Value) -> CallToolResult {
        let SaveTemplatesArgs { as_new } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let name = self
            .with_session(|s| Ok(s.snapshot().editing.map(|e| e.name.clone())))
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        let message = self.localizer.current().tr_with("agent-confirm-save-templates", &[("name", name.into())]);
        self.guarded(message, false, Request::SaveTemplates { as_new }).await
    }

    pub(crate) async fn delete_template_set(&self, args: Value) -> CallToolResult {
        let crate::edit_tools::SetArgs { set } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        // Le nom du pack dans sa version courante ; inconnu : son id.
        let name = self
            .with_session(move |s| Ok(s.library_pack_name(set)))
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| set.to_string());
        let message = self.localizer.current().tr_with("agent-confirm-delete-set", &[("name", name.into())]);
        self.guarded(message, true, Request::DeleteTemplateSet { set }).await
    }

    /// Nouvelle version d'un thème (copie d'un thème intégré), en mode deck :
    /// l'écran reste sur les slides. Répond le thème obtenu.
    pub(crate) async fn edit_theme(&self, args: Value) -> CallToolResult {
        let EditThemeArgs { theme, name, ops } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        // Un thème inconnu est refusé sans rien demander.
        let info = match self.with_session(move |s| s.theme_info(theme)).await {
            Ok(i) => i,
            Err(e) => return self.fail_with(e),
        };
        let key = if info.builtin { "agent-confirm-copy-theme" } else { "agent-confirm-edit-theme" };
        let message = self.localizer.current().tr_with(key, &[("name", info.name.into())]);
        if !self.hub.confirm(message, false).await {
            return self.denied();
        }
        let done = self.with_session(move |s| s.handle(Request::EditTheme { id: theme, name, ops })).await;
        self.reply(done, |edited| {
            self.hub.emit(McpEvent::Changed);
            text(edited)
        })
    }

    pub(crate) async fn set_setting(&self, args: Value) -> CallToolResult {
        let SetSettingArgs { key, value } = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        if !writable(&key) {
            return self.fail_with(SettingLockedSnafu { key }.build());
        }
        let old = self.settings_read().await.get(&key).cloned().unwrap_or(Value::Null);
        let message = self.localizer.current().tr_with(
            "agent-confirm-setting",
            &[("key", key.as_str().into()), ("old", old.to_string().into()), ("new", value.to_string().into())],
        );
        if !self.hub.confirm(message, false).await {
            return self.denied();
        }
        let written = self.settings_write(key, value).await;
        self.reply(written, |applied| self.json_result(&json!({ "applied": applied })))
    }
}
