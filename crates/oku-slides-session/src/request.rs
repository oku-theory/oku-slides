//! Les requêtes de l'interface (même forme pour Tauri et HTTP), et leur
//! traitement par la session (`Session::handle`).

use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ResultExt};
use uuid::Uuid;

use oku_slides_core::{Namespace, Op, PackKind};

use crate::error::{EditingPackSnafu, InHistorySnafu, ReadOnlySnafu, ResponseSnafu};
use crate::theme_extends::ExtendsEdit;
use crate::views::UiState;
use crate::{Mode, PreviewFormat, Result, Session};

/// Widget to show data under: chart or table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum View {
    /// A chart.
    Chart,
    /// A table.
    Table,
}

/// Ordre d'une sauvegarde de l'état d'interface : chacune part tout de suite, sans attendre la
/// précédente, et peut donc arriver après une plus récente. `page` est tiré au chargement de la page,
/// `seq` croît pour chaque sauvegarde de cette page. Ni montré, ni gardé pour la reprise.
///
/// Limite : une autre `page` est toujours acceptée (rechargement), donc une requête tardive de
/// l'ancienne page, arrivée après une de la nouvelle, passe ; la sauvegarde suivante la corrige.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct UiOrder {
    /// Drawn when the page loads: tells a reload apart.
    pub page: String,
    /// Grows with each save from this page.
    pub seq: u64,
}

#[path = "request_class.rs"]
mod request_class;

/// Une requête de l'interface (même forme pour Tauri et HTTP).
#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Request {
    /// The state of the open deck (`Snapshot`), without changing anything.
    Snapshot,
    /// Enregistre ; sans `overwrite`, un fichier changé sur disque est
    /// d'abord fusionné (même deck) ou laissé tel quel (autre deck, signalé
    /// par `disk_replaced` dans le snapshot).
    Save {
        /// New file of the deck; when absent, its current file.
        path: Option<PathBuf>,
        /// Overwrites the file without merging what changed on disk.
        #[serde(default)]
        overwrite: bool,
    },
    /// The view of slide `id` for the stage (`SlideView`).
    Slide {
        /// Id of the slide.
        id: String,
    },
    /// Les slides où `query` apparaît (tableau de `SearchHit`), titres d'abord.
    Search {
        /// Text searched for in the slides.
        query: String,
    },
    /// The SVG of the page of slide `id`.
    Svg {
        /// Id of the slide.
        id: String,
    },
    /// Applies ops to the deck in one undo step named `label`.
    Apply {
        /// Label of the undo step, in the user's language.
        label: String,
        /// Ops applied together, in order.
        ops: Vec<Op>,
        /// Clé de session de saisie : les `Apply` consécutifs de même clé
        /// forment une étape d'annulation.
        #[serde(default)]
        merge: Option<String>,
    },
    /// Le deck prend le thème `theme` et sa fermeture, versions courantes embarquées. Annulable.
    ///
    /// `theme` : son espace de noms, son id ou son nom (sans casse), comme [`crate::embed_theme`].
    SetTheme {
        /// Targeted theme: namespace, id or name.
        theme: String,
    },
    /// Undoes the deck's last step.
    Undo,
    /// Redoes the last undone step.
    Redo,
    /// Annule le tour `turn` de l'agent depuis son reçu, s'il est encore au sommet de la pile d'annulation.
    UndoTurn {
        /// Number of the agent turn to undo.
        turn: u32,
    },
    /// The JSON schema of the deck ops.
    Schema,
    /// Registre des widgets du deck ouvert (base, surchargée par son pack).
    Widgets,
    /// Réglages de l'hôte en mode pack (surface ou `null`) : ceux du widget `widget`.
    HostSettings {
        /// Widget being set up; when absent, no settings shown.
        #[serde(default)]
        widget: Option<String>,
    },
    /// Mode pack : la surface des jetons du brouillon du thème (`null`
    /// ailleurs) ; `filter` garde les jetons dont le libellé ou le chemin le contient.
    HostTheme {
        /// Text searched for in labels and paths; empty, all tokens.
        #[serde(default)]
        filter: String,
    },
    /// Mode pack : agit sur le jeton `path` du brouillon (une étape
    /// d'annulation) ; `value` pour `set` seulement.
    HostToken {
        /// Path of the token in the theme draft.
        path: Vec<String>,
        /// Action on the token: set, restore, remove or clear.
        action: crate::TokenAction,
        /// Value set by `set`; ignored by the other actions.
        #[serde(default)]
        value: Option<serde_json::Value>,
    },
    /// Surface `host-data` (ou `null`) : les données du graphique ou du tableau `element`.
    DataPanel {
        /// Id of the slide that holds the element.
        slide: String,
        /// Id of the chart or table.
        element: String,
    },
    /// Applique une modification de la surface `host-data` à `element` (annulable).
    DataEdit {
        /// Id of the slide that holds the element.
        slide: String,
        /// Id of the modified chart or table.
        element: String,
        /// Change sent by the `host-data` surface.
        edit: crate::DataEdit,
    },
    /// Change le type d'un graphique ; ses données sont converties. Annulable.
    ChartKind {
        /// Label of the undo step.
        label: String,
        /// Id of the slide that holds the chart.
        slide: String,
        /// Id of the chart.
        element: String,
        /// New series type of the chart.
        kind: oku_slides_chart::SeriesType,
    },
    /// Affiche un graphique en tableau, ou l'inverse, en gardant ses données. Annulable.
    ShowAs {
        /// Label of the undo step.
        label: String,
        /// Id of the slide that holds the element.
        slide: String,
        /// Id of the chart or table.
        element: String,
        /// Widget wanted for this data.
        view: View,
    },
    /// Layouts des packs de layouts épinglés par le deck, rendus avec son thème (`LayoutsView`).
    Layouts,
    /// Packs de la bibliothèque de sorte `kind`, dans leur version courante.
    Packs {
        /// Kind of packs listed: widgets or layouts.
        kind: PackKind,
    },
    /// Passe en édition de la version courante d'un pack (le deck est mis de côté).
    EditPack {
        /// Id of the pack to edit.
        pack: Uuid,
    },
    /// Enregistre le pack édité dans la version de cette session d'édition.
    ///
    /// `as_new` : une copie (nouvel id).
    SavePack {
        /// Saves a copy under a new id.
        #[serde(default)]
        as_new: bool,
    },
    /// Quitte l'édition du pack et retrouve le deck.
    ClosePack,
    /// Mode pack : le brouillon du thème part du thème `id` (version
    /// courante), ses jetons remplacés en une étape annulable.
    SetEditedTheme {
        /// Id of the starting theme.
        id: Uuid,
    },
    /// Mode pack : renomme le thème édité (sans étape d'annulation).
    RenameEditedTheme {
        /// New name of the edited theme.
        name: String,
    },
    /// Mode pack : le brouillon du thème étend `extends`, dans cet ordre (une étape annulable).
    SetEditedThemeExtends {
        /// Namespaces of the extended themes, in order.
        extends: Vec<Namespace>,
    },
    /// Mode pack : change la liste `extends` du brouillon par un delta (une étape annulable).
    ///
    /// À plat : `{cmd: "edit_theme_extends", edit: "add", theme: "<espace de noms>"}`.
    EditThemeExtends {
        /// The delta: adding, removing or reordering a theme.
        #[serde(flatten)]
        edit: ExtendsEdit,
    },
    /// Mode deck : nouvelle version du thème `id` (ops de jetons, nom), que
    /// le deck adopte s'il l'utilise. Répond `EditedTheme`.
    EditTheme {
        /// Id of the modified theme.
        id: Uuid,
        /// New name; when absent, the name stays.
        #[serde(default)]
        name: Option<String>,
        /// Token ops applied to the new version.
        #[serde(default)]
        ops: Vec<crate::TokenOp>,
    },
    /// Supprime un pack de l'utilisateur, toutes versions comprises.
    ///
    /// Refusé sous l'id d'un pack intégré.
    DeletePack {
        /// Id of the deleted pack.
        pack: Uuid,
    },
    /// Slide courante, grille et fantômes de l'interface (gardés pour la reprise).
    ///
    /// Sans `order` (agent, tests) : toujours appliquée.
    Ui {
        /// The interface state to keep, its fields flattened.
        #[serde(flatten)]
        ui: UiState,
        /// Order of the save; one older than a received one is ignored.
        #[serde(default)]
        order: Option<UiOrder>,
    },
    /// Fusionne un autre `.oku-slides.pdf` du même deck. Non annulable.
    Merge {
        /// Deck file to merge.
        path: PathBuf,
    },
    /// Ouvre l'historique sur la dernière version (le deck est mis de côté).
    HistoryOpen,
    /// Montre une version de l'historique (bornée à la dernière).
    HistorySeek {
        /// Step of the version, from `0` to the most recent.
        index: usize,
    },
    /// Ramène le deck à la version montrée (annulable) et ferme l'historique.
    HistoryRestore,
    /// Ferme l'historique et retrouve le deck.
    HistoryClose,
    /// Les versions du deck, dans l'ordre (tableau de `Version`).
    HistoryVersions,
    /// Déplace le fichier du deck : écrit à `path`, supprime l'ancien.
    Move {
        /// New file of the deck.
        path: PathBuf,
    },
    /// Écrit une copie à `path` ; le deck garde son fichier.
    SaveCopy {
        /// File of the copy.
        path: PathBuf,
    },
    /// Statut des fichiers de widgets du pack en cours (tableau de `WidgetStatus`).
    ///
    /// `pack` : en mode deck, l'espace de noms du pack épinglé ; absent, le pack `deck`.
    /// En mode pack, ignoré : c'est toujours le pack édité.
    WidgetStatus {
        /// Namespace of the pinned pack; when absent, the `deck` pack.
        #[serde(default)]
        pack: Option<Namespace>,
    },
    /// Source d'un fichier de widget du pack en cours (`pack` : voir `WidgetStatus`).
    WidgetSource {
        /// Namespace of the pack (see `WidgetStatus`).
        #[serde(default)]
        pack: Option<Namespace>,
        /// Name of the widget file.
        name: String,
    },
    /// Crée ou remplace un fichier de widget ; une source vide le supprime (`WidgetChange`).
    ///
    /// `pack` : voir `WidgetStatus` ; le pack `deck` est créé à la première écriture.
    SetWidget {
        /// Namespace of the pack (see `WidgetStatus`).
        #[serde(default)]
        pack: Option<Namespace>,
        /// Name of the widget file.
        name: String,
        /// New Typst source; when empty, the file is deleted.
        source: String,
    },
    /// Supprime un fichier de widget ; refusé s'il est utilisé par un modèle ou le deck.
    RemoveWidget {
        /// Namespace of the pack (see `WidgetStatus`).
        #[serde(default)]
        pack: Option<Namespace>,
        /// Name of the deleted widget file.
        name: String,
    },
    /// Renomme un fichier de widget dans la version éditée.
    RenameWidget {
        /// Namespace of the pack (see `WidgetStatus`).
        #[serde(default)]
        pack: Option<Namespace>,
        /// Current name of the widget file.
        from: String,
        /// New name of the widget file.
        to: String,
    },
    /// Aperçu SVG d'un widget seul (`WidgetPreview`), aux valeurs d'exemple
    /// du widget ; `params` les complète (`pack` : voir `WidgetStatus`).
    PreviewWidget {
        /// Namespace of the pack (see `WidgetStatus`).
        #[serde(default)]
        pack: Option<Namespace>,
        /// Name of the shown widget.
        name: String,
        /// Values that replace the example ones, by parameter.
        #[serde(default)]
        params: serde_json::Map<String, serde_json::Value>,
    },
    /// Règle une valeur d'exemple du widget `name` ; `null` ou la valeur neutre
    /// l'effacent, un texte vidé reste vide (mode pack, dans le pack).
    SetExampleValue {
        /// Name of the widget.
        name: String,
        /// Name of the parameter being set.
        param: String,
        /// New example value; `null` clears it.
        value: serde_json::Value,
    },
    /// Change la liste `param` des valeurs d'exemple ; sans valeur gardée, elle
    /// part des items par défaut du schéma (mode pack).
    EditExampleList {
        /// Name of the widget.
        name: String,
        /// Name of the list parameter.
        param: String,
        /// Change to the list: inserting, removing or moving an item.
        change: oku_slides_core::ListOp,
    },
    /// Pins `version` of `pack` if it is current and newer than the deck's.
    UsePackVersion {
        /// The pack the deck pins.
        pack: Uuid,
        /// The version offered when the pack tab closed.
        version: oku_slides_core::PackVersion,
        /// The deck the offer named; another deck in this tab refuses.
        deck_uid: String,
    },
}

impl Session {
    /// Handles a request from the interface and returns the response, as JSON.
    ///
    /// # Errors
    ///
    /// Fails if the request is rejected (mode, validation, file, rendering) or if the response does
    /// not serialize (`Response`).
    #[expect(clippy::too_many_lines, reason = "dispatch: one arm per Request variant")]
    pub fn handle(&mut self, req: Request) -> Result<String> {
        fn json(v: &(impl Serialize + ?Sized)) -> Result<String> {
            Ok(serde_json::to_string(v).context(ResponseSnafu)?)
        }
        if matches!(self.mode, Mode::Pack { .. })
            && matches!(
                req,
                Request::Save { .. }
                    | Request::EditTheme { .. }
                    | Request::Merge { .. }
                    | Request::Move { .. }
                    | Request::SaveCopy { .. }
            )
        {
            return EditingPackSnafu.fail()?;
        }
        if matches!(self.mode, Mode::History { .. }) && req.edits() {
            return InHistorySnafu.fail()?;
        }
        // L'aide ne change pas le deck et n'ouvre pas d'historique.
        if self.help().is_some() && (req.edits() || matches!(req, Request::HistoryOpen)) {
            return ReadOnlySnafu.fail()?;
        }
        // An editor without its pack's lease takes it before writing, or is refused.
        if matches!(self.mode, Mode::Pack { .. }) && req.writes_edited_pack() {
            self.ensure_pack_lease()?;
        }
        // Les commandes modifient la session et renvoient le nouvel état ; les
        // requêtes renvoient ce qu'elles lisent.
        // Tout sauf la lecture du snapshot change l'état ; `ui` ne touche pas aux
        // documents ; `apply` écrit déjà son propre état de reprise (l'agent
        // l'appelle aussi directement, hors de `handle`), pas la peine de le refaire.
        // L'historique ne change pas le deck mis de côté, sauf `HistoryRestore`.
        let (changes, documents) = (
            !matches!(
                req,
                Request::Snapshot
                    | Request::Apply { .. }
                    | Request::SetTheme { .. }
                    | Request::SetEditedTheme { .. }
                    | Request::SetEditedThemeExtends { .. }
                    | Request::EditThemeExtends { .. }
                    | Request::HostToken { .. }
                    | Request::DataEdit { .. }
                    | Request::UsePackVersion { .. }
            ),
            !matches!(
                req,
                Request::Snapshot
                    | Request::Ui { .. }
                    | Request::HistoryOpen
                    | Request::HistorySeek { .. }
                    | Request::HistoryClose
                    | Request::RenameEditedTheme { .. }
            ),
        );
        // La séance des widgets du mode deck finit à la première autre modification.
        if req.ends_widget_session() {
            self.end_widget_session();
        }
        // Un thème a pu être enregistré ailleurs (autre onglet) : relu au snapshot demandé.
        if matches!(req, Request::Snapshot | Request::SetTheme { .. }) {
            self.theme_list = self.themes.themes();
        }
        match req {
            // Another tab may have written the library: the deck's badge recounts, without adopting.
            Request::Snapshot if matches!(self.mode, Mode::Deck) => self.refresh_pack_updates_if_pins_changed(),
            Request::Snapshot => {}
            Request::Slide { id } => return json(&self.slide(&id)?),
            Request::Search { query } => return json(&self.search(&query)),
            Request::Svg { id } => return json(&*self.svg(&id)?),
            Request::HostSettings { widget } => return json(&self.host_settings(widget.as_deref())),
            Request::HostTheme { filter } => return json(&self.host_theme(&filter)?),
            Request::DataPanel { slide, element } => return json(&self.data_panel(&slide, &element)),
            Request::Schema => return json(&oku_slides_core::ops_schema().context(ResponseSnafu)?),
            Request::Widgets => return json(&*self.registry),
            Request::Layouts => return json(&self.layouts_view()?),
            Request::Packs { kind } => return json(&self.library.packs(kind)),
            Request::WidgetStatus { pack } => return json(&self.widget_status(pack.as_ref())?),
            Request::WidgetSource { pack, name } => return json(&self.widget_source(pack.as_ref(), &name)?),
            Request::SetWidget { pack, name, source } => {
                let change = self.set_widget(pack.as_ref(), &name, source)?;
                self.noted_user_write(&name, change.saved);
                self.persist(true);
                return json(&change);
            }
            Request::RemoveWidget { pack, name } => {
                let statuses = self.remove_widget(pack.as_ref(), &name)?;
                self.persist(true);
                return json(&statuses);
            }
            Request::RenameWidget { pack, from, to } => {
                let statuses = self.rename_widget(pack.as_ref(), &from, &to)?;
                self.persist(true);
                return json(&statuses);
            }
            // `apply` (adoption par le deck) écrit son état de reprise.
            Request::EditTheme { id, name, ops } => return json(&self.edit_theme(id, name, ops)?),
            Request::PreviewWidget { pack, name, params } => {
                return json(&self.preview_widget(pack.as_ref(), &name, &params, PreviewFormat::Svg)?);
            }
            Request::DeletePack { pack } => return json(&self.delete_pack(pack)?),
            Request::Save { path, overwrite } => {
                self.save_checked(path.as_deref(), overwrite)?;
            }
            Request::Move { path } => self.move_to(&path)?,
            Request::SaveCopy { path } => self.save_copy(&path)?,
            Request::Merge { path } => {
                self.merge_file(&path)?;
            }
            Request::Apply { label, ops, merge } => self.apply(&label, ops, merge.as_deref())?,
            Request::SetTheme { theme } => {
                let ns = crate::library_lookup::find_theme(&self.themes, &theme)
                    .context(crate::error::UnknownThemeSnafu { theme: theme.as_str() })?;
                self.set_theme(&ns)?;
            }
            Request::HostToken { path, action, value } => self.host_token(path, action, value)?,
            Request::SetExampleValue { name, param, value } => {
                self.set_example_values(&name, serde_json::Map::from_iter([(param, value)]))?;
            }
            Request::EditExampleList { name, param, change } => {
                self.edit_example_list(&name, &param, change)?;
            }
            Request::DataEdit { slide, element, edit } => self.data_edit(slide, &element, edit)?,
            Request::ChartKind { label, slide, element, kind } => self.set_chart_kind(&label, slide, &element, kind)?,
            Request::ShowAs { label, slide, element, view } => self.show_as(&label, slide, &element, view)?,
            Request::Undo => self.undo()?,
            Request::Redo => self.redo()?,
            Request::UndoTurn { turn } => self.undo_agent_turn(turn)?,
            Request::EditPack { pack } => self.edit_pack(pack)?,
            Request::SavePack { as_new } => {
                self.save_pack(as_new)?;
            }
            Request::ClosePack => self.close_pack()?,
            Request::SetEditedTheme { id } => {
                let ns = self.themes.current_version(id).map(|s| s.manifest.namespace);
                self.set_edited_theme(&ns.context(crate::error::UnknownThemeSnafu { theme: id.to_string() })?)?;
            }
            Request::RenameEditedTheme { name } => self.rename_edited_theme(&name)?,
            Request::SetEditedThemeExtends { extends } => self.set_edited_theme_extends(extends)?,
            Request::EditThemeExtends { edit } => self.edit_theme_extends(edit)?,
            Request::Ui { ui, order } => self.keep_ui(ui, order),
            Request::HistoryOpen => self.history_open()?,
            Request::HistorySeek { index } => self.history_seek(index)?,
            Request::HistoryRestore => self.history_restore()?,
            Request::HistoryClose => self.history_close()?,
            Request::HistoryVersions => return json(self.history_versions()?),
            Request::UsePackVersion { pack, version, deck_uid } => self.use_pack_version(pack, &version, &deck_uid)?,
        }
        if changes {
            self.persist(documents);
        }
        let out = json(&self.snapshot())?;
        // Les avis ne s'affichent qu'une fois : celui du document affiché
        // vient d'être envoyé, il ne le sera plus. (Celui du deck mis de
        // côté, s'il y en a un, n'est pas encore montré : il reste, pour
        // apparaître à son tour quand `close_pack` le rendra visible.)
        self.doc.notice = None;
        self.doc.disk_replaced = false;
        Ok(out)
    }

    /// Garde l'état d'interface reçu, sauf s'il est plus ancien (même page, `seq` inférieur ou égal)
    /// que le dernier gardé : une sauvegarde partie avant peut arriver après (t-674).
    fn keep_ui(&mut self, ui: UiState, order: Option<UiOrder>) {
        if let (Some(new), Some(last)) = (&order, &self.ui_order)
            && new.page == last.page
            && new.seq <= last.seq
        {
            return;
        }
        if order.is_some() {
            self.ui_order = order;
        }
        self.ui = ui.sanitized(matches!(self.mode, Mode::Pack { .. }));
    }
}

#[cfg(test)]
#[path = "request_tests.rs"]
mod request_tests;
