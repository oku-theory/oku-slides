//! Liste des outils, schémas d'entrée et résultats.
//!
//! MCP 1.x ne garde que `properties` et `required` à la racine d'un schéma :
//! [`nest_schema`] déplace les `$defs` sous la première propriété.

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use base64::Engine as _;
use oku_slides_i18n::LiveLocalizer;
use oku_slides_platform::Limits;
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
    /// Langue courante de l'interface, relue à chaque texte.
    pub(crate) localizer: LiveLocalizer,
    /// Bornes des connexions du serveur d'outils (`[serve]`).
    pub(crate) limits: Limits,
    /// La plateforme : de quoi lancer le témoin d'un import (`import_tools`).
    pub(crate) platform: Platform,
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
        let serve = &platform.settings.serve;
        Self {
            session,
            hub,
            settings,
            storage: platform.storage.clone(),
            localizer: platform.live_localizer.clone(),
            limits: Limits::new(serve.header_timeout(), serve.max_connections.get()),
            platform: platform.clone(),
        }
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

/// La description de `import_diff`, avec les bornes réelles des images et des régions.
fn import_diff_description() -> String {
    use oku_slides_session::import::{LISTED, REFERENCE_WIDTH, ZOOM};
    format!(
        "Hi-fi import: compare one slide with its reference (the original PowerPoint rendering). Returns two PNGs, reference then Oku as rendered now (the source page, {REFERENCE_WIDTH} px wide), then JSON: score, worst, excess (sum of what non-suspect regions exceed the threshold by: lower is a gain), threshold, state, tries_left, global_tries_left, counted, charge, regions (top {LISTED}, worst first: item, score, shift [dx, dy] px of Oku vs reference, suspect if the reference itself is wrong there, frame {{x, y, w, h}} in pt of the Oku page: the source item's frame placed on the Oku page, where the original draws it (it does not follow your changes: compare it with the element's own frame), and elements [ids] to fix with apply, or param, the slide parameter it fills), regions_more, suspect (count). The first score of a slide (here or by import_report) is free; a whole-slide diff whose rendering changed since the slide's last score counts one try of the slide (charge slide), or one global try when the change comes from the whole deck alone, the slide itself unchanged (charge global, see import_report); an unchanged rendering is free, and so is a return to the slide's best-scored rendering (charge revert). Refused for a slide with no try left (done, witness, unscored, exhausted, stalled), the zoom too. With region (an item id from regions): both crops of that region, {ZOOM}x with a margin, and its region entry; counts no try."
    )
}

/// Comment un outil de widget sans `pack` trouve un pack propre du deck (`deck` ou `deck-layouts`).
const DECK_OWN_PACKS: &str = "the deck's own element pack `deck` (or its own layout pack `deck-layouts` when the deck has only that one); the deck's own slide widgets are in `deck-layouts`: pass pack `deck-layouts` for them";

/// La description de `widgets`, avec les kinds réels des packs intégrés.
fn widgets_description() -> String {
    use oku_slides_core::kinds::{base, essentiel};
    format!(
        "Widgets available in this deck: every widget with its parameters and their types. A kind is qualified by its pack: `namespace/name`. The base element widgets are {}. The essentiel slide layouts are {}. A deck's other kinds come from its packs, `namespace/name` (the deck's own packs are `deck` for elements and `deck-layouts` for slides). Read it before adding an element or a slide.",
        quoted(&base::ALL),
        quoted(&essentiel::ALL),
    )
}

/// Les kinds entre accents graves, séparés par des virgules.
fn quoted(kinds: &[&str]) -> String {
    kinds.iter().map(|k| format!("`{k}`")).collect::<Vec<_>>().join(", ")
}

/// Noms de tous les outils MCP, dans l'ordre de la liste.
pub fn tool_names() -> Vec<String> {
    list().into_iter().map(|t| t.name).collect()
}

/// Tous les outils, dans l'ordre où l'agent les lit.
pub(crate) fn list() -> Vec<Tool> {
    static LIST: OnceLock<Vec<Tool>> = OnceLock::new();
    LIST.get_or_init(|| {
        vec![
            tool::<NoArgs>(
                "deck_outline",
                "List the slides of the open deck: id, kind, title, element count; theme {namespace, name} of the deck (null in pack mode), the available themes [{namespace, name, extends, builtin}] (extends: the namespaces a theme builds on, in order, the later ones winning; the set_theme op takes {\"theme\": \"<namespace>\"} and the deck then pins that theme and everything it extends), widget_packs and layout_packs {namespace: {id, version, requires?}} (the element and slide layout packs the deck pins; the add_pack / remove_pack tools change them), mode (deck, pack or history). To change or rename a theme, use edit_theme: never enter pack mode for a theme. In pack mode, edited_theme {id, version, namespace, name, child, extends [{namespace, name}], dirty} is the theme being edited (namespace: the theme it starts from; extends: its parents in order, set with set_theme_extends; child: saving it creates a variant extending a built-in theme): a set_theme op alone (no other op in the transaction) makes the draft start from that theme; edit its tokens with set_token.",
            ),
            tool::<NoArgs>("widgets", &widgets_description()),
            tool::<crate::read_tools::ThemeArgs>(
                "theme_tokens",
                "Read a theme (its namespace, as deck_outline lists it) in the version edit_theme starts from: {namespace, extends, name, version, tokens, deck_tokens}. tokens: the theme's own tokens {group: {name: value}} (a token it does not set comes from the themes it extends, then the defaults); deck_tokens: the deck's own overrides on top of the theme (deck.tokens, edited with the set_token op). Read it before edit_theme so you can restore the previous values: set_token back to a value it had, unset_token for a token it did not have.",
            ),
            tool::<SlideId>(
                "slide",
                "Full content of one slide: elements, styles, grid, notes, and their boxes on the page.",
            ),
            tool::<SlideId>(
                "render_slide",
                "PNG image of one slide as rendered now. Look at it after changing the layout.",
            ),
            tool::<crate::read_tools::PacksArgs>("packs", "Packs of the library (built-in and the user's), each in its current version: kind (widgets: elements; layouts: slides), id, version, name, namespace, requires, default, received, widgets and profiles counts. edit_pack and delete_pack take its id."),
            tool::<NoArgs>("layouts", "Slide layouts of the layout packs this deck pins, per pack {namespace, name, layouts [{kind, label}]}: the kinds add_slide and set_kind take for a slide."),
            tool::<NoArgs>("settings", "App settings you can read (dictation, history); change the dictation with set_dictation. The interface language is in each turn context; change it with set_interface."),
            tool::<crate::edit_tools::ApplyArgs>("apply", "Apply deck ops atomically (all or nothing). set_param changes a parameter of a slide or an element (see widgets for its schema); a rich value is a list of paragraphs, never a string: [{\"runs\":[{\"t\":\"Plain, \"},{\"t\":\"bold\",\"bold\":true}]},{\"runs\":[{\"t\":\"A bullet\"}],\"list\":\"bullet\"}] (run: t, bold?, italic?, link?; list?: bullet|number; slide shows existing values in this shape). set_grid changes a slide's grid (columns, rows; placed elements follow); set_deck_param changes a deck parameter (title, author…); set_kind changes the widget of a slide or an element; add_element adds an element to a slot; add_element and add_slide take \"defaults\": true to seed a new widget's list defaults (never when re-inserting or moving). move_element moves an element with its children to a slot of the same or another slide (slide, element, to {owner, param}, index?), keeping its id and any concurrent edit; prefer it to remove_element + add_element. edit_list changes a list parameter item by item, addressed by target and param (the list, or list/<item id>/<column> for an item's nested list), with change insert (index?, id, values?), remove (item), move (item, to) or set_field (item, field, value); items are addressed by id and you supply the id. set_chart_option changes a chart's option; set_data changes the data of a chart or a table (dataset.source, row 0 = column names), addressed by target and param. set_table_option changes a table's display (header-row, header-column, striped, align: auto|start|center|end per column) as a JSON merge patch; table data is the same dataset as a chart's. add_pack {kind, namespace, pin {id, version, requires?}} pins a pack of kind widgets (elements) or layouts (slides), its requires first; remove_pack {kind, namespace} unpins one no slide or element uses (`base` stays). set_theme {\"theme\": \"<namespace>\"} gives the deck a theme (a namespace from deck_outline's themes, for instance \"pro\"); the deck pins that theme and everything it extends. Every change the user can undo. Prefer one apply with several ops over many calls."),
            tool::<crate::edit_tools::ChartKindArgs>("chart_kind", "Change a chart's type; its data is converted."),
            tool::<crate::edit_tools::ShowAsArgs>("show_as", "Show a chart as a table, or a table as a chart, keeping its data; one undo step."),
            tool::<crate::edit_tools::EditPackArgs>("edit_pack", "Switch to pack mode on the current version of a pack; when pack is omitted, the deck's main pack: its own layout pack, else its own widget pack, else a library pack (layouts first), else essentiel (the deck is put aside, nothing is lost). A layout pack shows one slide per layout, at its example values: edit them with set_param (slides cannot be added, removed or moved): each change is written to the layout's example at once, as the session version of the pack (one undo step), and stays in the library after close_pack; save_pack only saves the pack's name and theme draft. Not for themes: use edit_theme. The deck's own deck-only pack (an imported deck's) is edited from the version the deck embeds and its saves stay in the deck, never in the library; a deck-only pack the deck no longer uses is refused."),
            tool::<NoArgs>("close_pack", "Leave pack mode and get the deck back (unsaved pack changes are dropped)."),
            tool::<crate::edit_tools::PackRefArgs>("add_pack", "Add a library pack (kind widgets: elements, layouts: slides; namespace as packs lists it) to the deck in its current version, with the packs it requires; one undo step. Refused when the deck already pins that namespace or the pack is the deck's own. Deck mode only."),
            tool::<crate::edit_tools::PackRefArgs>("remove_pack", "Remove a pack (kind, namespace as deck_outline lists it) from the deck; one undo step. Refused, with the reason, while a slide or an element uses it, another pinned pack requires it, or for base. Deck mode only."),
            tool::<crate::edit_tools::ThemeExtendsArgs>("set_theme_extends", "Pack mode only: set the parents of the edited theme (edited_theme in deck_outline), as library theme namespaces in order, the later ones winning; [] renders over the defaults. One undo step. Refused for an unknown theme, a duplicate, or a cycle."),
            tool::<crate::widget_tools::PackArgs>("widget_sources", &format!("Widget files of one pack (pack: its namespace; absent, {DECK_OWN_PACKS}; in pack mode, the edited pack): name, schema when valid, diagnostics (file, line, column, message). `_commun` holds shared code. Works in deck and pack modes.")),
            tool::<crate::widget_tools::WidgetName>("widget_source", &format!("Typst source of one widget file (pack: its namespace; absent, {DECK_OWN_PACKS}).")),
            tool::<crate::widget_tools::SetWidgetArgs>("set_widget", "Create or replace a widget file (Typst) in a pack (pack: its namespace; a pack of another namespace is pinned with the add_pack op). Absent in deck mode, an element widget goes to the deck's own pack `deck` and a slide widget to its own layout pack `deck-layouts`, each created on first write. Returns its status and the widgets this change broke. In deck mode the deck moves to a new version of its pack; successive widget edits make one undo step."),
            tool::<crate::widget_tools::WidgetName>("remove_widget", &format!("Delete a widget file (pack: its namespace; absent, {DECK_OWN_PACKS}); refused while a slide or a layout example uses it.")),
            tool::<crate::widget_tools::PreviewWidgetArgs>("preview_widget", &format!("PNG of one widget alone on a temporary slide (deck theme and tokens), followed by its diagnostics if any (pack: its namespace; absent, {DECK_OWN_PACKS}). Missing params take the widget's test values (stored in the pack), then their default or a sample value. Look at it after each set_widget.")),
            tool::<crate::widget_tools::SetWidgetTrialArgs>("set_widget_trial", "Pack mode only: set test values of a widget's parameters, saved in the pack, used by its preview and shown in its settings. Rich values are paragraphs. null resets one to its default; an empty text or [] keeps it empty (to test an empty case). All or nothing."),
            tool::<NoArgs>("import_report", "Report of the PowerPoint import of this tab (an error if the deck was not imported here). Always: mode, losses {total, by_kind}, frozen [{slide, item}] (Opaque items kept as drawn, never fixable). Retrofit: widget_requests [{signature, slides, fallback, sample {items [{kind, geometry?, frame [x, y, w, h] in pt, fill?, stroke? as {hex, slot?} (slot: the PowerPoint theme color), paragraphs?, depth? (a group's children follow it)}], omitted?}}] (sample: the pattern's first occurrence, at most 16 items, never its text). Hi-fi: threshold, mean, tries_per_slide, global_tries_left, witness (or reference_unavailable), groups [{signature, occurrences [{slide, item, elements}]}] (a group repeated on 3+ slides, candidate for an element widget), done (count of slides whose non-suspect regions are all under the threshold) and slides, the queue worst first: [{slide, source, score, worst, excess, state, tries_left, stale?}] with excess the sum of what its non-suspect regions exceed the threshold by (the gain measure), state pending (fix it), exhausted (no try left), stalled (no gain over the last tries), witness (unchanged since the import, only regions the reference renders wrong are above the threshold: do not touch; once changed, such a slide counts as done) or unscored (no reference, or its comparison failed in this report). Slides deleted since the import are left out. Every slide whose rendering changed since its last score is scored again, at a cost: one try of that slide, or one global try for a change of the whole deck alone (theme, pack widget, deck tokens or params; a slide edited since its last score always pays its own try), whatever the number of slides it changes, going back to a deck content already scored being free (the same theme tokens, pack widgets, deck tokens and params, even saved as a new version); going back to a slide's best-scored rendering is free too; once global_tries_left is 0, such a change costs each changed slide a try. A changed slide with no try left keeps its old score, is marked stale: true in the queue and listed in stale; stale and unscored slides are left out of mean, mean_before and worse. mean_before (the same slides as imported, rounded like mean) and worse [slide ids whose excess rose above the best they reached] show the effect of your changes (revert a theme change that raised the mean or made a slide worse)."),
            tool::<crate::import_tools::ImportDiffArgs>("import_diff", &import_diff_description()),
            tool::<crate::edit_tools::FocusArgs>("focus", "Show a slide (and an element) to the user."),
            tool::<oku_slides_session::agent::AskChoice>("ask_choice", "Ask the user to choose (2-6 options). Blocks until they answer; returns {choice} or {answer_text} if they answered by voice, or {cancelled}. Use when the decision is theirs. anchor {slide, owner, field?} shows it under that element or text field of the slide (owner: an element, or the slide itself with field); without it, in the thread."),
            tool::<oku_slides_session::agent::AskConfirm>("ask_confirm", "Ask yes/no. Blocks; returns {confirmed} / {answer_text} / {cancelled}."),
            tool::<oku_slides_session::agent::AskRewrite>("ask_rewrite", "Propose 2-3 rewordings of a text field, addressed by {slide, owner, field}: owner is an element of the slide (field optional: its first rich text by default) or the slide itself (field required); field is a text parameter of the owner's widget (see widgets). Blocks; the chosen text is applied by Oku Slides; returns {kept, text}."),
            tool::<oku_slides_session::agent::ShowParams>("show_params", "Show a panel of live settings; each field applies op_template with \"$value\" replaced. Returns at once; later changes are reported in the next context."),
            tool::<oku_slides_session::agent::ShowVariants>("show_variants", "Show 2-4 rendered variants of a slide; the user's pick is applied. Returns at once; the pick is reported in the next context."),
            tool::<crate::surface_tools::ShowThemesArgs>("show_themes", "Show the theme picker in the dock: the slide (default: the first) rendered in each available theme, with its colors. Use it whenever the user should choose or compare the deck's theme, never ask_choice or render for that. The user's click sets the theme (one undo step); the picker stays open after your turn and follows later changes. Returns {shown} at once; the pick is reported in the next context. Deck mode only."),
            tool::<NoArgs>("show_packs", "Show the deck packs panel in the dock: each layout and widget pack the deck pins with its version and how it relates to the library's, the theme, and buttons to add, remove or update them. Use it whenever the user should review or change the deck's packs. The user's clicks apply at once (one undo step each); the panel stays open after your turn and follows later changes. Returns {shown} at once. Deck mode only."),
            tool::<oku_slides_session::agent::Report>("report", "End every turn with this: a receipt card under the user's request. title: one line saying what changed; facts: 1-4 short lines; slide: shows its thumbnail; swatch: a color worth showing; why: your explanation in Markdown, folded by default; actions: 0-4 next steps as buttons (a click sends prompt, or label, as the user's next message). A second call in the same turn replaces the first. Your free text is shown only as one-line progress notes."),
            tool::<oku_slides_session::agent::Render>("render", "Show a card you compose from the A2UI catalog (v0.9 shape: [{id, component, …}], one id \"root\"). Components: Column{children}, Row{children}, Card{child, tone?}, Text{text, variant?: h4|body|caption|note}, Badge{text, tone?}, Markdown{text}, Disclosure{summary, child}, DiffText{segments}, Button{label, action}. All values literal (no {path}). Only action, on a Button whose label is the prompt: {event: {name: \"suggest\", context: {prompt: <the label>}}}. No primary/danger/icon styles. For a turn's conclusion use report (Receipt and slide thumbnails are reserved to it). At most 40 components forming a tree (each id used once), 6 levels, 64 KiB. Use it only when report, ask_* and show_* do not fit. Errors say what to fix."),
            tool::<crate::guarded_tools::SaveArgs>("save", "Save the deck as .oku-slides.pdf (not a file open in another tab). The user confirms first."),
            tool::<crate::guarded_tools::OpenArgs>("open", "Open a .oku-slides.pdf (absolute path) in a new tab, or show the tab that has it; returns {tab}. The user confirms first. You stay in this tab: its deck is unchanged and your tools keep acting on it."),
            tool::<crate::guarded_tools::OpenArgs>("move", "Move the deck's file to another .oku-slides.pdf (absolute path): writes it there and deletes the old file. The user confirms first. Refused when a file already exists there or is open in another tab."),
            tool::<crate::guarded_tools::OpenArgs>("save_copy", "Write a copy of the deck to another .oku-slides.pdf (absolute path); the deck keeps its own file. The user confirms first. Refused when a file already exists there or is open in another tab."),
            tool::<crate::guarded_tools::OpenArgs>("merge", "Merge another .oku-slides.pdf (absolute path) of the same deck into this one: its concurrent edits join this deck's. Cannot be undone; the user confirms first."),
            tool::<crate::app_tools::HistoryArgs>("history", "The deck's version history. open: the tab opens the history on the latest version (the deck is put aside, nothing is lost) and the versions are returned, oldest first, once it is open; list: the versions again; seek {index}: shows that version and returns the shown {index, count}; restore: brings the deck back to the shown version (undoable) and closes the history; close: gives the deck back unchanged. While the history is open, edits are refused. Refused unless this tab is the one shown in its window."),
            tool::<crate::app_tools::PresentArgs>("present", "Start presenting the deck from the slide shown, full screen (mode screen, default) or in the window (mode window). Refused with no slide, in pack mode, in the history, on a read-only tab, or unless this tab is the one shown in its window. The user leaves with Escape."),
            tool::<crate::app_tools::InterfaceChange>("set_interface", "Change the interface right away, as its own commands do; give at least one field, omitted ones stay as they are. scheme: light, dark or system; locale: a shipped language code (en, fr, de…) or system; grid: show the slide grid; guides: show the dashed outlines of empty fields. Refused unless this tab is the one shown in its window."),
            tool::<NoArgs>("new", "Start a new blank deck. The user confirms first."),
            tool::<crate::guarded_tools::SavePackArgs>("save_pack", "Save the pack being edited in pack mode as a new version (as_new: a copy with a new id). The user confirms first."),
            tool::<crate::edit_tools::PackIdArgs>("delete_pack", "Delete one of the user's packs (see packs), all versions included. The user confirms first."),
            tool::<crate::guarded_tools::EditThemeArgs>("edit_theme", "Edit or rename a theme from deck mode (the user stays on the slides); theme: its namespace (as deck_outline lists it), id or name. Saves a new current version of the theme with ops (set_token {path, value}, unset_token {path}, checked like the theme editor) and/or a new name. A built-in theme is not changed: the edit is saved as a variant, a new theme that extends it (new namespace taken from its name, \"(variant)\" unless named) and holds only the edited tokens. A deck using the theme moves to the new version or to the variant (one undo step); deck token overrides stay on top. For a change to this deck only, do not use this tool: use set_token ops on deck.tokens with apply. Returns {id, namespace, version, name, changed, adopted, tokens} (tokens: the theme's own tokens after the edit, as theme_tokens reads them). The user always confirms first."),
            tool::<NoArgs>("show_dictation", "Show the dictation and app agent settings box in the dock, open until the user closes it. It shows the recognition model, spoken language, auto stop, the dictation API, model downloads and the app agent. Returns {shown}."),
            tool::<crate::voice_tools::SetDictationArgs>("set_dictation", "Change dictation settings, except the API address and key variable, which only the user sets in the box. Fields: model, language (a code or \"auto\"), auto_stop, silence (0.5–10 s), api_model; switching to api or to a model to download asks the user. Returns {applied: now | unchanged}."),
            tool::<crate::voice_tools::ModelArg>("download_dictation_model", "Download a dictation model or Apple language after the user confirms. Returns {applied}."),
            tool::<NoArgs>("cancel_dictation_download", "Stop the dictation model download in progress; refused when none runs."),
            tool::<crate::voice_tools::AgentArg>("select_agent", "Switch the app agent to a preset id (see settings) after the user confirms; it takes effect once no reply runs. Returns {applied: now | when_idle | unchanged}."),
            tool::<NoArgs>("download_agent_model", "Download the built-in local agent's model after the user confirms."),
            tool::<NoArgs>("cancel_agent_model_download", "Stop the local agent model download in progress; refused when none runs."),
            tool::<crate::guarded_tools::SetSettingArgs>("set_setting", "Change an app setting (whitelisted keys only; dictation goes through set_dictation). The user confirms first; returns {applied: next_launch}."),
        ]
    })
    .clone()
}

#[cfg(test)]
#[path = "tools_tests.rs"]
mod tools_tests;
