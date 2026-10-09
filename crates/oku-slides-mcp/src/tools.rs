//! Liste des outils, schémas d'entrée et résultats.
//!
//! MCP 1.x ne garde que `properties` et `required` à la racine d'un schéma :
//! [`nest_schema`] déplace les `$defs` sous la première propriété.

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use base64::Engine as _;
use oku_slides_i18n::Localizer;
use oku_slides_platform::storage::Storage;
use oku_slides_session::Platform;
use oku_slides_session::agent::TabSession;
use rust_mcp_sdk::schema::{CallToolResult, ImageContent, TextContent, Tool, ToolInputSchema};
use schemars::JsonSchema;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use snafu::ResultExt;

use crate::tool_error::{ArgumentsSnafu, ResponseSnafu, SessionSnafu, SettingSnafu, TaskSnafu, ToolError};
use crate::{Applied, SettingsPort, SurfaceHub};

/// Les outils de l'agent, servis par [`crate::ToolServer`].
#[derive(Clone)]
pub struct Tools {
    pub(crate) session: TabSession,
    pub(crate) hub: Arc<SurfaceHub>,
    pub(crate) settings: Arc<dyn SettingsPort>,
    pub(crate) storage: Storage,
    pub(crate) localizer: Arc<Localizer>,
}

impl std::fmt::Debug for Tools {
    /// `Storage` (OpenDAL) n'implémente pas `Debug`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tools").finish_non_exhaustive()
    }
}

impl Tools {
    pub fn new(
        session: TabSession,
        hub: Arc<SurfaceHub>,
        settings: Arc<dyn SettingsPort>,
        platform: &Platform,
    ) -> Self {
        Self { session, hub, settings, storage: platform.storage.clone(), localizer: platform.localizer.clone() }
    }

    /// Travail synchrone sur la session de l'onglet, hors du fil async ; le
    /// verrou n'est tenu que le temps de `f`. L'erreur (onglet fermé compris) se dit
    /// par [`Tools::fail_with`].
    pub(crate) async fn with_session<T: Send + 'static>(
        &self,
        f: impl FnOnce(&mut oku_slides_session::Session) -> oku_slides_session::Result<T> + Send + 'static,
    ) -> Result<T, ToolError> {
        let session = self.session.clone();
        tokio::task::spawn_blocking(move || session.with(f).and_then(|r| r))
            .await
            .context(TaskSnafu)?
            .context(SessionSnafu)
    }

    /// Arguments d'un outil, ou la réponse d'erreur à renvoyer.
    #[expect(clippy::result_large_err, reason = "le `CallToolResult` d'erreur part directement en réponse à l'agent")]
    pub(crate) fn parse<T: DeserializeOwned>(&self, args: Value) -> Result<T, CallToolResult> {
        serde_json::from_value(args).context(ArgumentsSnafu).map_err(|e| self.fail_with(e))
    }

    /// Le résultat JSON d'une valeur, ou l'erreur de réponse.
    pub(crate) fn json_result(&self, v: &impl Serialize) -> CallToolResult {
        match serde_json::to_string(v).context(ResponseSnafu) {
            Ok(s) => text(s),
            Err(e) => self.fail_with(e),
        }
    }

    /// `result`, ou sa réponse d'erreur.
    pub(crate) fn reply<T>(
        &self,
        result: Result<T, ToolError>,
        ok: impl FnOnce(T) -> CallToolResult,
    ) -> CallToolResult {
        match result {
            Ok(v) => ok(v),
            Err(e) => self.fail_with(e),
        }
    }

    /// Lecture des réglages, hors du fil async : l'implémentation réelle
    /// (`config.toml` via `Storage`/OpenDAL) bloque comme la session.
    pub(crate) async fn settings_read(&self) -> BTreeMap<String, Value> {
        let settings = self.settings.clone();
        tokio::task::spawn_blocking(move || settings.read()).await.unwrap_or_default()
    }

    /// Écriture d'un réglage déjà autorisé, hors du fil async.
    ///
    /// Le message de l'hôte ne part qu'au journal (il peut nommer un fichier) : l'agent reçoit
    /// la clé refusée.
    pub(crate) async fn settings_write(&self, key: String, value: Value) -> Result<Applied, ToolError> {
        let settings = self.settings.clone();
        let written = {
            let key = key.clone();
            tokio::task::spawn_blocking(move || settings.write(&key, &value)).await.context(TaskSnafu)?
        };
        written.map_err(|message| SettingSnafu { key, message }.build())
    }
}

#[derive(Debug, Clone, Copy, serde::Deserialize, JsonSchema)]
pub(crate) struct NoArgs {}

#[derive(Debug, Clone, serde::Deserialize, JsonSchema)]
pub(crate) struct SlideId {
    /// Identifiant de la slide (voir `deck_outline`).
    pub id: String,
}

pub(crate) fn text(s: impl Into<String>) -> CallToolResult {
    CallToolResult::text_content(vec![TextContent::new(s.into(), None, None)])
}

/// Réponse d'erreur au texte déjà dit : seulement par [`Tools::fail_with`].
pub(crate) fn fail(msg: impl Into<String>) -> CallToolResult {
    let mut r = text(msg);
    r.is_error = Some(true);
    r
}

pub(crate) fn png(bytes: &[u8]) -> CallToolResult {
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    CallToolResult::image_content(vec![ImageContent::new(b64, "image/png".into(), None, None)])
}

/// Déplace les `$defs` de la racine sous la première propriété et réécrit
/// les `$ref` (`#/$defs/X` → `#/properties/<p>/$defs/X`).
pub(crate) fn nest_schema(mut schema: Value) -> Value {
    let Some(defs) = schema.as_object_mut().and_then(|o| o.remove("$defs")) else { return schema };
    let Some(first) = schema.get("properties").and_then(Value::as_object).and_then(|p| p.keys().next().cloned()) else {
        return schema;
    };
    let prefix = format!("#/properties/{first}/$defs/");
    fn rewrite(v: &mut Value, prefix: &str) {
        match v {
            Value::Object(map) => {
                for (k, child) in map.iter_mut() {
                    if k == "$ref" {
                        if let Some(rest) = child.as_str().and_then(|r| r.strip_prefix("#/$defs/")) {
                            *child = Value::String(format!("{prefix}{rest}"));
                        }
                    } else {
                        rewrite(child, prefix);
                    }
                }
            }
            Value::Array(items) => items.iter_mut().for_each(|c| rewrite(c, prefix)),
            _ => {}
        }
    }
    let mut defs = defs;
    rewrite(&mut defs, &prefix);
    rewrite(&mut schema, &prefix);
    if let Some(p) = schema.pointer_mut(&format!("/properties/{first}")).and_then(Value::as_object_mut) {
        p.insert("$defs".into(), defs);
    }
    schema
}

/// Schéma d'entrée MCP 1.x : `properties` et `required` seulement.
fn input_schema(schema: Value) -> ToolInputSchema {
    let schema = nest_schema(schema);
    let properties = schema
        .get("properties")
        .and_then(Value::as_object)
        .map(|p| p.iter().map(|(k, v)| (k.clone(), v.as_object().cloned().unwrap_or_default())).collect());
    let required = schema
        .get("required")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_owned)).collect())
        .unwrap_or_default();
    ToolInputSchema::new(required, properties, None)
}

fn tool<T: JsonSchema>(name: &str, description: &str) -> Tool {
    let schema = serde_json::to_value(schemars::schema_for!(T)).unwrap_or_else(|_| json!({}));
    Tool {
        name: name.into(),
        title: None,
        description: Some(description.into()),
        input_schema: input_schema(schema),
        output_schema: None,
        annotations: None,
        execution: None,
        icons: vec![],
        meta: None,
    }
}

/// Tous les outils, dans l'ordre où l'agent les lit.
pub(crate) fn list() -> Vec<Tool> {
    static LIST: OnceLock<Vec<Tool>> = OnceLock::new();
    LIST.get_or_init(|| {
        vec![
            tool::<NoArgs>(
                "deck_outline",
                "List the slides of the open deck: id, kind, title, element count; theme {id, name} of the deck (null in templates mode), the available themes [{id, name, version}] (set_theme takes {id, version}), template set, mode. In templates mode, edited_theme {id, version, name, dirty} is the theme being edited: a set_theme op alone (no other op in the transaction) makes the draft start from that theme; edit its tokens with set_token.",
            ),
            tool::<NoArgs>(
                "widgets",
                "Widgets available in this deck: every widget with its parameters and their types. Read it before adding an element.",
            ),
            tool::<SlideId>(
                "slide",
                "Full content of one slide: elements, styles, grid, notes, and their boxes on the page.",
            ),
            tool::<SlideId>(
                "render_slide",
                "PNG image of one slide as rendered now. Look at it after changing the layout.",
            ),
            tool::<NoArgs>("templates", "Template packs (built-in and the user's), each in its current version, with their templates."),
            tool::<NoArgs>("settings", "App settings you can read (dictation, language, history)."),
            tool::<crate::edit_tools::ApplyArgs>("apply", "Apply deck ops atomically (all or nothing). set_param changes a parameter of a slide or an element (see widgets for its schema); set_grid changes a slide's grid (columns, rows; placed elements follow); set_deck_param changes a deck parameter (title, author…); set_kind changes the widget of a slide or an element; add_element adds an element to a slot; add_element and add_slide take \"defaults\": true to seed a new widget's list defaults (never when re-inserting or moving). edit_list changes a list parameter item by item, addressed by target and param, with change insert (index?, id, values?), remove (item), move (item, to) or set_field (item, field, value); items are addressed by id and you supply the id. set_chart_option changes a chart's option; set_data changes the data of a chart or a table (dataset.source, row 0 = column names), addressed by target and param. set_table_option changes a table's display (header-row, header-column, striped, align: auto|start|center|end per column) as a JSON merge patch; table data is the same dataset as a chart's. Every change the user can undo. Prefer one apply with several ops over many calls."),
            tool::<crate::edit_tools::ChartKindArgs>("chart_kind", "Change a chart's type; its data is converted."),
            tool::<crate::edit_tools::ShowAsArgs>("show_as", "Show a chart as a table, or a table as a chart, keeping its data; one undo step."),
            tool::<crate::edit_tools::SetArgs>("edit_templates", "Switch to editing the current version of a template pack (the deck is put aside, nothing is lost)."),
            tool::<NoArgs>("close_templates", "Leave template editing and get the deck back (unsaved template changes are dropped)."),
            tool::<NoArgs>("widget_sources", "Widget files of the pack in use (the edited template pack, or the deck's pack): name, schema when valid, diagnostics (file, line, column, message). `_commun` holds shared code. Works in deck and template modes."),
            tool::<crate::widget_tools::WidgetName>("widget_source", "Typst source of one widget file."),
            tool::<crate::widget_tools::SetWidgetArgs>("set_widget", "Create or replace a widget file (Typst). Returns its status and the widgets this change broke. In deck mode the deck moves to a new version of its pack; successive widget edits make one undo step."),
            tool::<crate::widget_tools::WidgetName>("remove_widget", "Delete a widget file; refused while a template of the pack uses it."),
            tool::<crate::widget_tools::PreviewWidgetArgs>("preview_widget", "PNG of one widget alone on a temporary slide (deck theme and tokens), followed by its diagnostics if any. Missing params take the widget's test values (stored in the pack), then their default or a sample value. Look at it after each set_widget."),
            tool::<crate::widget_tools::SetWidgetTrialArgs>("set_widget_trial", "Templates mode only: set test values of a widget's parameters, saved in the pack, used by its preview and shown in its settings. Rich values are paragraphs. null resets one to its default; an empty text or [] keeps it empty (to test an empty case). All or nothing."),
            tool::<crate::edit_tools::FocusArgs>("focus", "Show a slide (and an element) to the user."),
            tool::<oku_slides_session::agent::AskChoice>("ask_choice", "Ask the user to choose (2-6 options). Blocks until they answer; returns {choice} or {answer_text} if they answered by voice, or {cancelled}. Use when the decision is theirs. anchor {slide, owner, field?} shows it under that element or text field of the slide (owner: an element, or the slide itself with field); without it, in the thread."),
            tool::<oku_slides_session::agent::AskConfirm>("ask_confirm", "Ask yes/no. Blocks; returns {confirmed} / {answer_text} / {cancelled}."),
            tool::<oku_slides_session::agent::AskRewrite>("ask_rewrite", "Propose 2-3 rewordings of a text field, addressed by {slide, owner, field}: owner is an element of the slide (field optional: its first rich text by default) or the slide itself (field required); field is a text parameter of the owner's widget (see widgets). Blocks; the chosen text is applied by Oku Slides; returns {kept, text}."),
            tool::<oku_slides_session::agent::ShowParams>("show_params", "Show a panel of live settings; each field applies op_template with \"$value\" replaced. Returns at once; later changes are reported in the next context."),
            tool::<oku_slides_session::agent::ShowVariants>("show_variants", "Show 2-4 rendered variants of a slide; the user's pick is applied. Returns at once; the pick is reported in the next context."),
            tool::<oku_slides_session::agent::Report>("report", "End every turn with this: a receipt card under the user's request. title: one line saying what changed; facts: 1-4 short lines; slide: shows its thumbnail; swatch: a color worth showing; why: your explanation in Markdown, folded by default; actions: 0-4 next steps as buttons (a click sends prompt, or label, as the user's next message). A second call in the same turn replaces the first. Your free text is shown only as one-line progress notes."),
            tool::<oku_slides_session::agent::Render>("render", "Show a card you compose from the A2UI catalog (v0.9 shape: [{id, component, …}], one id \"root\"). Components: Column{children}, Row{children}, Card{child, tone?}, Text{text, variant?: h4|body|caption|note}, Badge{text, tone?}, Markdown{text}, Disclosure{summary, child}, DiffText{segments}, Button{label, action}. All values literal (no {path}). Only action, on a Button whose label is the prompt: {event: {name: \"suggest\", context: {prompt: <the label>}}}. No primary/danger styles. For a turn's conclusion use report (Receipt and slide thumbnails are reserved to it). At most 40 components forming a tree (each id used once), 6 levels, 64 KiB. Use it only when report, ask_* and show_* do not fit. Errors say what to fix."),
            tool::<crate::guarded_tools::SaveArgs>("save", "Save the deck as .oku-slides.pdf (not a file open in another tab). The user confirms first."),
            tool::<crate::guarded_tools::OpenArgs>("open", "Open a .oku-slides.pdf (absolute path) in a new tab, or show the tab that has it; returns {tab}. The user confirms first. You stay in this tab: its deck is unchanged and your tools keep acting on it."),
            tool::<NoArgs>("new", "Start a new blank deck. The user confirms first."),
            tool::<crate::guarded_tools::SaveTemplatesArgs>("save_templates", "Save the template pack being edited as a new version (as_new: a copy with a new id). The user confirms first."),
            tool::<crate::edit_tools::SetArgs>("delete_template_set", "Delete one of the user's template packs, all versions included. The user confirms first."),
            tool::<crate::guarded_tools::SetSettingArgs>("set_setting", "Change an app setting (whitelisted keys only). The user confirms first; returns {applied: now|next_launch}."),
        ]
    })
    .clone()
}

#[cfg(test)]
#[path = "tools_tests.rs"]
mod tools_tests;
