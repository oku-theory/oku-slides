//! Les requêtes de l'interface (même forme pour Tauri et HTTP), et leur
//! traitement par la session (`Session::handle`).

use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::{Namespace, Op, PackKind};

use crate::error::{BuiltinPackSnafu, EditingPackSnafu, InHistorySnafu, ReadOnlySnafu, ResponseSnafu};
use crate::theme_extends::ExtendsEdit;
use crate::views::UiState;
use crate::{Mode, PackLibrary, PreviewFormat, Result, Session};

/// Widget sous lequel montrer des données : graphique ou tableau.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum View {
    /// Un graphique.
    Chart,
    /// Un tableau.
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
    pub page: String,
    pub seq: u64,
}

/// Une requête de l'interface (même forme pour Tauri et HTTP).
#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Request {
    Snapshot,
    /// Enregistre ; sans `overwrite`, un fichier changé sur disque est
    /// d'abord fusionné (même deck) ou laissé tel quel (autre deck, signalé
    /// par `disk_replaced` dans le snapshot).
    Save {
        path: Option<PathBuf>,
        #[serde(default)]
        overwrite: bool,
    },
    Slide {
        id: String,
    },
    /// Les slides où `query` apparaît (tableau de `SearchHit`), titres d'abord.
    Search {
        query: String,
    },
    Svg {
        id: String,
    },
    Apply {
        label: String,
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
        theme: String,
    },
    Undo,
    Redo,
    /// Annule le tour `turn` de l'agent depuis son reçu, s'il est encore au sommet de la pile d'annulation.
    UndoTurn {
        turn: u32,
    },
    Schema,
    /// Registre des widgets du deck ouvert (base, surchargée par son pack).
    Widgets,
    /// Réglages de l'hôte en mode pack (surface ou `null`) : ceux du widget `widget`.
    HostSettings {
        #[serde(default)]
        widget: Option<String>,
    },
    /// Mode pack : la surface des jetons du brouillon du thème (`null`
    /// ailleurs) ; `filter` garde les jetons dont le libellé ou le chemin le contient.
    HostTheme {
        #[serde(default)]
        filter: String,
    },
    /// Mode pack : agit sur le jeton `path` du brouillon (une étape
    /// d'annulation) ; `value` pour `set` seulement.
    HostToken {
        path: Vec<String>,
        action: crate::TokenAction,
        #[serde(default)]
        value: Option<serde_json::Value>,
    },
    /// Surface `host-data` (ou `null`) : les données du graphique ou du tableau `element`.
    DataPanel {
        slide: String,
        element: String,
    },
    /// Applique une modification de la surface `host-data` à `element` (annulable).
    DataEdit {
        slide: String,
        element: String,
        edit: crate::DataEdit,
    },
    /// Change le type d'un graphique ; ses données sont converties. Annulable.
    ChartKind {
        label: String,
        slide: String,
        element: String,
        kind: oku_slides_chart::SeriesType,
    },
    /// Affiche un graphique en tableau, ou l'inverse, en gardant ses données. Annulable.
    ShowAs {
        label: String,
        slide: String,
        element: String,
        view: View,
    },
    /// Layouts des packs de layouts épinglés par le deck, rendus avec son thème (`LayoutsView`).
    Layouts,
    /// Packs de la bibliothèque de sorte `kind`, dans leur version courante.
    Packs {
        kind: PackKind,
    },
    /// Passe en édition de la version courante d'un pack (le deck est mis de côté).
    EditPack {
        pack: Uuid,
    },
    /// Enregistre le pack édité dans la version de cette session d'édition.
    ///
    /// `as_new` : une copie (nouvel id).
    SavePack {
        #[serde(default)]
        as_new: bool,
    },
    /// Quitte l'édition du pack et retrouve le deck.
    ClosePack,
    /// Mode pack : le brouillon du thème part du thème `id` (version
    /// courante), ses jetons remplacés en une étape annulable.
    SetEditedTheme {
        id: Uuid,
    },
    /// Mode pack : renomme le thème édité (sans étape d'annulation).
    RenameEditedTheme {
        name: String,
    },
    /// Mode pack : le brouillon du thème étend `extends`, dans cet ordre (une étape annulable).
    SetEditedThemeExtends {
        extends: Vec<Namespace>,
    },
    /// Mode pack : change la liste `extends` du brouillon par un delta (une étape annulable).
    ///
    /// À plat : `{cmd: "edit_theme_extends", edit: "add", theme: "<espace de noms>"}`.
    EditThemeExtends {
        #[serde(flatten)]
        edit: ExtendsEdit,
    },
    /// Mode deck : nouvelle version du thème `id` (ops de jetons, nom), que
    /// le deck adopte s'il l'utilise. Répond `EditedTheme`.
    EditTheme {
        id: Uuid,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        ops: Vec<crate::TokenOp>,
    },
    /// Supprime un pack de l'utilisateur, toutes versions comprises.
    ///
    /// Refusé sous l'id d'un pack intégré.
    DeletePack {
        pack: Uuid,
    },
    /// Slide courante, grille et fantômes de l'interface (gardés pour la reprise).
    ///
    /// Sans `order` (agent, tests) : toujours appliquée.
    Ui {
        #[serde(flatten)]
        ui: UiState,
        #[serde(default)]
        order: Option<UiOrder>,
    },
    /// Fusionne un autre `.oku-slides.pdf` du même deck. Non annulable.
    Merge {
        path: PathBuf,
    },
    /// Ouvre l'historique sur la dernière version (le deck est mis de côté).
    HistoryOpen,
    /// Montre une version de l'historique (bornée à la dernière).
    HistorySeek {
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
        path: PathBuf,
    },
    /// Écrit une copie à `path` ; le deck garde son fichier.
    SaveCopy {
        path: PathBuf,
    },
    /// Statut des fichiers de widgets du pack en cours (tableau de `WidgetStatus`).
    ///
    /// `pack` : en mode deck, l'espace de noms du pack épinglé ; absent, le pack `deck`.
    WidgetStatus {
        #[serde(default)]
        pack: Option<Namespace>,
    },
    /// Source d'un fichier de widget du pack en cours (`pack` : voir `WidgetStatus`).
    WidgetSource {
        #[serde(default)]
        pack: Option<Namespace>,
        name: String,
    },
    /// Crée ou remplace un fichier de widget ; une source vide le supprime (`WidgetChange`).
    ///
    /// `pack` : voir `WidgetStatus` ; le pack `deck` est créé à la première écriture.
    SetWidget {
        #[serde(default)]
        pack: Option<Namespace>,
        name: String,
        source: String,
    },
    /// Supprime un fichier de widget ; refusé s'il est utilisé par un modèle ou le deck.
    RemoveWidget {
        #[serde(default)]
        pack: Option<Namespace>,
        name: String,
    },
    /// Renomme un fichier de widget dans la version éditée.
    RenameWidget {
        #[serde(default)]
        pack: Option<Namespace>,
        from: String,
        to: String,
    },
    /// Aperçu SVG d'un widget seul (`WidgetPreview`), aux valeurs d'exemple
    /// du widget ; `params` les complète (`pack` : voir `WidgetStatus`).
    PreviewWidget {
        #[serde(default)]
        pack: Option<Namespace>,
        name: String,
        #[serde(default)]
        params: serde_json::Map<String, serde_json::Value>,
    },
    /// Règle une valeur d'exemple du widget `name` ; `null` ou la valeur neutre
    /// l'effacent, un texte vidé reste vide (mode pack, dans le pack).
    SetExampleValue {
        name: String,
        param: String,
        value: serde_json::Value,
    },
    /// Change la liste `param` des valeurs d'exemple ; sans valeur gardée, elle
    /// part des items par défaut du schéma (mode pack).
    EditExampleList {
        name: String,
        param: String,
        change: oku_slides_core::ListOp,
    },
}

impl Request {
    /// Vrai si la requête peut changer le deck (slides ajoutées ou retirées).
    ///
    /// `Save` aussi : un fichier changé sur disque est d'abord fusionné.
    /// L'historique (sauf la liste des versions) : le deck montré change.
    /// `SavePack` aussi : la bibliothèque change, le panneau des packs se relit (`prune`).
    /// Exhaustif à dessein : une requête ajoutée doit dire si elle change le deck.
    pub fn changes_deck(&self) -> bool {
        match self {
            Self::Save { .. }
            | Self::Apply { .. }
            | Self::SetTheme { .. }
            | Self::EditTheme { .. }
            | Self::Undo
            | Self::Redo
            | Self::UndoTurn { .. }
            | Self::ChartKind { .. }
            | Self::ShowAs { .. }
            | Self::EditPack { .. }
            | Self::ClosePack
            | Self::SetEditedTheme { .. }
            | Self::SetEditedThemeExtends { .. }
            | Self::EditThemeExtends { .. }
            | Self::SavePack { .. }
            | Self::Merge { .. }
            | Self::HistoryOpen
            | Self::HistorySeek { .. }
            | Self::HistoryRestore
            | Self::HistoryClose
            | Self::Move { .. }
            | Self::SetWidget { .. }
            | Self::RemoveWidget { .. }
            | Self::RenameWidget { .. }
            | Self::SetExampleValue { .. }
            | Self::EditExampleList { .. }
            | Self::HostToken { .. }
            | Self::DataEdit { .. } => true,
            Self::Snapshot
            | Self::Slide { .. }
            | Self::Search { .. }
            | Self::Svg { .. }
            | Self::Schema
            | Self::Widgets
            | Self::HostSettings { .. }
            | Self::HostTheme { .. }
            | Self::DataPanel { .. }
            | Self::Layouts
            | Self::Packs { .. }
            | Self::RenameEditedTheme { .. }
            | Self::DeletePack { .. }
            | Self::SaveCopy { .. }
            | Self::Ui { .. }
            | Self::HistoryVersions
            | Self::WidgetStatus { .. }
            | Self::WidgetSource { .. }
            | Self::PreviewWidget { .. } => false,
        }
    }

    /// Requêtes refusées pendant l'historique : elles modifieraient le deck,
    /// l'enregistreraient, ou passeraient en mode pack.
    pub(crate) fn edits(&self) -> bool {
        matches!(
            self,
            Self::Apply { .. }
                | Self::SetTheme { .. }
                | Self::EditTheme { .. }
                | Self::Undo
                | Self::Redo
                | Self::UndoTurn { .. }
                | Self::Save { .. }
                | Self::Merge { .. }
                | Self::Move { .. }
                | Self::SaveCopy { .. }
                | Self::ChartKind { .. }
                | Self::ShowAs { .. }
                | Self::EditPack { .. }
                | Self::SavePack { .. }
                | Self::ClosePack
                | Self::SetEditedTheme { .. }
                | Self::SetEditedThemeExtends { .. }
                | Self::EditThemeExtends { .. }
                | Self::RenameEditedTheme { .. }
                | Self::DeletePack { .. }
                | Self::SetWidget { .. }
                | Self::RemoveWidget { .. }
                | Self::RenameWidget { .. }
                | Self::HostToken { .. }
                | Self::DataEdit { .. }
                | Self::SetExampleValue { .. }
                | Self::EditExampleList { .. }
        )
    }

    /// Vrai si la requête termine la séance des widgets du mode deck.
    ///
    /// Toute requête qui change le deck, sauf une modification de widget et
    /// `Save` : l'enregistrement ne la termine que s'il fusionne un fichier
    /// changé sur disque (voir `Session::absorb`). L'interface enregistre
    /// après chaque écriture : sinon chacune ferait une version du pack.
    pub(crate) fn ends_widget_session(&self) -> bool {
        self.changes_deck() && !matches!(self, Self::Save { .. }) && !self.is_widget_edit()
    }

    /// Vrai pour une modification de widget (valeurs d'exemple comprises) : elle ne
    /// termine pas la séance des widgets.
    pub(crate) fn is_widget_edit(&self) -> bool {
        matches!(
            self,
            Self::SetWidget { .. }
                | Self::RemoveWidget { .. }
                | Self::RenameWidget { .. }
                | Self::SetExampleValue { .. }
                | Self::EditExampleList { .. }
        )
    }
}

impl Session {
    /// Traite une requête de l'interface et renvoie la réponse, en JSON.
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
            Request::Snapshot => {}
            Request::Slide { id } => return json(&self.slide(&id)?),
            Request::Search { query } => return json(&self.search(&query)),
            Request::Svg { id } => return json(&*self.svg(&id)?),
            Request::HostSettings { widget } => return json(&self.host_settings(widget.as_deref())?),
            Request::HostTheme { filter } => return json(&self.host_theme(&filter)?),
            Request::DataPanel { slide, element } => return json(&self.data_panel(&slide, &element)?),
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
            Request::DeletePack { pack } => {
                ensure!(!PackLibrary::is_builtin(pack), BuiltinPackSnafu { id: pack.to_string() });
                let kind = self.library.kind_of(pack);
                self.library.delete(pack)?;
                self.pack_cache.clear();
                self.seed_cache.clear();
                self.refresh_pack_updates();
                return json(&self.library.packs(kind));
            }
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
                self.set_theme(&ns)?
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
                self.set_edited_theme(&ns.context(crate::error::UnknownThemeSnafu { theme: id.to_string() })?)?
            }
            Request::RenameEditedTheme { name } => self.rename_edited_theme(name)?,
            Request::SetEditedThemeExtends { extends } => self.set_edited_theme_extends(extends)?,
            Request::EditThemeExtends { edit } => self.edit_theme_extends(edit)?,
            Request::Ui { ui, order } => self.keep_ui(ui, order),
            Request::HistoryOpen => self.history_open()?,
            Request::HistorySeek { index } => self.history_seek(index)?,
            Request::HistoryRestore => self.history_restore()?,
            Request::HistoryClose => self.history_close()?,
            Request::HistoryVersions => return json(self.history_versions()?),
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
