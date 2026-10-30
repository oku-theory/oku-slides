//! Vues envoyées à l'interface. Elles empruntent la session : sérialisées
//! aussitôt, elles ne copient ni le deck ni le rendu.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize, Serializer};

use oku_slides_core::{Namespace, PackKind, PackPin, PackVersion, Slide, SlideChanges, Version, WidgetFile};
use oku_slides_render::{ElementBox, ZoneBox};

use crate::appearance::Appearance;
use crate::fields::FieldView;
use crate::items::{ItemView, ListView};
use crate::tab_document::TabDocument;

/// Empreinte d'une page (clé de cache de son SVG), écrite en hex ; vide si
/// la page manque.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageHash(pub Option<u128>);

impl PageHash {
    /// L'empreinte écrite en 32 chiffres hexadécimaux ; vide si la page manque.
    #[must_use]
    pub fn hex(self) -> String {
        self.0.map(|h| format!("{h:032x}")).unwrap_or_default()
    }
}

impl Serialize for PageHash {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.hex())
    }
}

/// A slide in the deck list: what it takes to draw its thumbnail.
#[derive(Debug, Serialize)]
pub struct SlideSummary<'a> {
    /// Id of the slide.
    pub id: &'a str,
    /// Widget de la slide (`essentiel/title`, `essentiel/content`…).
    pub kind: &'a str,
    /// Son paramètre `title`, sinon (paramètre lié) le titre du deck ;
    /// variables remplacées.
    pub title: Option<String>,
    /// Page dans le document, à partir de 1.
    pub page: usize,
    /// Hash of its page, cache key of the thumbnail.
    pub hash: PageHash,
}

/// The state of the open deck sent to the interface after each request.
#[derive(Debug, Serialize)]
#[expect(clippy::struct_excessive_bools, reason = "independent flags of the JSON format the interface reads (api.ts)")]
pub struct Snapshot<'a> {
    /// File of the deck; absent until it is saved.
    pub path: Option<&'a Path>,
    /// Displayed name of the deck (tab, window title).
    pub name: String,
    /// Unsaved changes.
    pub dirty: bool,
    /// Thème du deck : son espace de noms.
    pub theme: Option<&'a oku_slides_core::Namespace>,
    /// Les thèmes épinglés par le deck : son thème et ceux qu'il étend, versions embarquées.
    pub theme_pins: &'a std::collections::BTreeMap<oku_slides_core::Namespace, oku_slides_core::ThemePin>,
    /// Nom du thème tel qu'embarqué par le deck (absent de la bibliothèque s'il vient d'un autre fichier).
    pub theme_name: Option<String>,
    /// Thèmes de la bibliothèque dans leur version courante : intégrés, puis ceux de l'utilisateur.
    pub themes: &'a [crate::ThemeInfo],
    /// Title of the deck, with variables replaced.
    pub title: Option<String>,
    /// Titre tel que saisi (avec ses {{variables}}), pour l'éditer.
    pub title_source: Option<&'a str>,
    /// Langue du deck (`params.lang`) : les grilles écrivent les nombres comme les slides.
    pub lang: Option<&'a str>,
    /// The deck's slides, in order.
    pub slides: Vec<SlideSummary<'a>>,
    /// A step can be undone.
    pub can_undo: bool,
    /// Label of the next step to undo, if there is one.
    pub undo_label: Option<String>,
    /// An undone step can be redone.
    pub can_redo: bool,
    /// Label of the next step to redo, if there is one.
    pub redo_label: Option<String>,
    /// Le tour de l'agent dont l'étape est au sommet de la pile d'annulation : son reçu propose de l'annuler.
    pub undo_turn: Option<u32>,
    /// Avertissements du rendu, dans la langue de l'utilisateur.
    pub warnings: Vec<String>,
    /// Signalé à l'ouverture : le PDF a été modifié par un autre outil, ou
    /// des parties du deck étaient illisibles et ont été mises de côté.
    pub notice: Option<&'a str>,
    /// Le dernier `Save` n'a rien écrit : le fichier est devenu un autre deck
    /// (vrai une fois ; l'interface propose « Écraser » ou « Enregistrer sous… »).
    pub disk_replaced: bool,
    /// L'interface prend les couleurs et les polices du thème des slides.
    pub appearance: &'a Appearance,
    /// Packs de widgets épinglés par le deck, par espace de noms (id, version embarquée, besoins).
    pub widgets: &'a BTreeMap<Namespace, PackPin>,
    /// Packs de layouts épinglés par le deck, par espace de noms (id, version embarquée, besoins).
    pub layouts: &'a BTreeMap<Namespace, PackPin>,
    /// Pack mode: the pack being edited.
    pub editing: Option<EditingView<'a>>,
    /// A theme tab: the edited theme's draft and flags.
    pub theming: Option<crate::ThemeTabView>,
    /// Le deck épingle un pack qui n'appartient qu'à lui (import, pack `deck`).
    ///
    /// L'atelier l'édite depuis sa version embarquée ; ses versions restent dans le deck.
    pub deck_pack: bool,
    /// Épinglages (packs et thème) plus anciens que ma version courante : la pastille du bouton des packs.
    pub pack_updates: u32,
    /// Widgets des packs en cours, qualifiés (`<namespace>/<nom>`, sans `_commun`), sans sonde.
    ///
    /// Ceux du pack édité en mode pack, sinon ceux de chaque pack épinglé.
    pub pack_widgets: Vec<String>,
    /// La grille d'ajout : les widgets d'élément de chaque pack de widgets épinglé, avec leur départ.
    pub add_packs: Vec<crate::AddPackView>,
    /// Le premier slot libre de chaque layout épinglé et les widgets qu'il accepte (`None` : tous).
    ///
    /// Un layout sans slot libre est absent : ses slides n'acceptent pas d'élément ajouté.
    pub free_slots: BTreeMap<String, Option<Vec<String>>>,
    /// Révision des valeurs d'exemple des widgets (`widget_examples.rs`) : un changement relance les aperçus.
    pub widget_trials: u64,
    /// Slide courante et grille, gardées pour la reprise.
    pub ui: &'a UiState,
    /// Historique ouvert : la version montrée, en lecture seule.
    pub history: Option<HistoryView<'a>>,
    /// L'aide de l'app : le deck se lit, il ne se modifie pas.
    pub read_only: bool,
    /// Paramètres d'hôte des widgets d'élément du deck, par kind et type d'hôte.
    pub host_params: BTreeMap<String, BTreeMap<oku_slides_core::HostType, String>>,
}

/// La version que montre l'historique, et ce qui a changé depuis.
#[derive(Debug, Serialize)]
pub struct HistoryView<'a> {
    /// Cran montré, de `0` à `count - 1` (la version la plus récente).
    pub index: usize,
    /// Nombre de versions (la liste complète : `history_versions`).
    pub count: usize,
    /// The shown version: author, date and label of the edit.
    pub version: &'a Version,
    /// Slides de cette version comparées au deck courant.
    pub changes: &'a SlideChanges,
}

/// What the interface has the host keep: slide, ghosts, columns, item and code.
///
/// The current slide, ghosts, column widths, a pack tab's selected item, the widget source
/// code shown and its width, and the pile's folded sections. Unknown fields of an older
/// `state.json` (`board`, `code_place`, `code_height`, removed by t-526; `case`, removed by t-541;
/// `widgets`, the widget sub-tabs removed by t-1244; `grid`, shown only while dragging since
/// t-1283) are ignored when read.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UiState {
    /// Id of the current slide; none if the deck is empty.
    #[serde(default)]
    pub slide: Option<String>,
    /// Fantômes visibles : zone, widgets et champs annotés.
    #[serde(default)]
    pub ghosts: bool,
    /// Largeur de la pile en px, choisie au séparateur ; `None` : largeur par défaut.
    #[serde(default)]
    pub pile: Option<u32>,
    /// Largeur du fil de l'agent en px, choisie au séparateur ; `None` : largeur par défaut.
    #[serde(default)]
    pub notes: Option<u32>,
    /// The selected item of the edited pack in pack mode, `_commun` included; `None`: none.
    #[serde(default)]
    pub widget: Option<String>,
    /// Code source des widgets déplié à droite de l'aperçu ; replié par défaut.
    #[serde(default)]
    pub code: bool,
    /// Largeur du code en px, choisie au séparateur ; `None` : proportion par défaut.
    #[serde(default)]
    pub code_width: Option<u32>,
    /// Folded sections of the pile (thumbnails); all open by default.
    #[serde(default)]
    pub folded: Vec<PileSection>,
}

/// A foldable section of the interface: the slide or layout thumbnails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PileSection {
    /// The slide thumbnails, outside pack mode.
    Slides,
    /// The layout thumbnails, in a layout pack or theme tab.
    Layouts,
}

/// The interface state of a kept `state.json`; folded sections removed since are dropped.
///
/// A section the interface no longer has (the parameters card, the widgets section of
/// t-1244) is forgotten instead of making the state unreadable. A `ui` request, for its
/// part, refuses any unknown section.
pub(crate) fn kept_ui<'de, D: serde::Deserializer<'de>>(d: D) -> Result<UiState, D::Error> {
    let mut value = serde_json::Value::deserialize(d)?;
    if let Some(folded) = value.get_mut("folded").and_then(serde_json::Value::as_array_mut) {
        folded.retain(|s| serde_json::from_value::<PileSection>(s.clone()).is_ok());
    }
    serde_json::from_value(value).map_err(serde::de::Error::custom)
}

impl UiState {
    /// The admitted state: folded sections without duplicates, a valid selected item.
    ///
    /// Outside pack mode the selected item is cleared; in pack mode it is kept only when it
    /// is a valid file name, `_commun` included.
    #[must_use]
    pub fn sanitized(mut self, pack_mode: bool) -> Self {
        let mut folded = std::collections::BTreeSet::new();
        self.folded.retain(|s| folded.insert(*s));
        if !pack_mode || self.widget.as_ref().is_some_and(|w| WidgetFile::parse(w).is_none()) {
            self.widget = None;
        }
        self
    }
}

#[cfg(test)]
#[path = "views_tests.rs"]
mod views_tests;

/// A tab as its bar shows it: title, unsaved work, what it edits.
///
/// Also kept in `workspace.json`, so the bar shows at launch without reading the
/// sessions. It is never deserialized from a saved entry directly: `take_info`
/// converts legacy entries and validates the rest (a saved entry is not trusted).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabInfo {
    /// The deck's file name, else its title.
    pub title: String,
    /// Unsaved work: the deck, the edited pack, or a save its owner has not taken.
    pub dirty: bool,
    /// A new deck never touched: opening a file replaces it.
    pub pristine: bool,
    /// Id of the deck's theme: help opened from this tab takes it.
    #[serde(default)]
    pub theme: Option<uuid::Uuid>,
    /// What the tab edits: a deck, a pack or a theme, read from its mode.
    pub document: TabDocument,
    /// The pack or theme this tab created empty; collected when the tab leaves untouched.
    #[serde(default)]
    pub created: Option<uuid::Uuid>,
}

/// A pack being edited: a layout pack has one slide per layout, at its example value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackMode {
    /// Id of the edited pack.
    pub pack: uuid::Uuid,
    /// Name of the edited pack, read from its manifest.
    pub name: String,
    /// Kind of the edited pack: widgets or layouts.
    pub kind: PackKind,
    /// A built-in pack: saving it creates the user's version under its id.
    pub builtin: bool,
    /// Version the widgets and description come from: the last one saved.
    pub base: PackVersion,
    /// Parent of the session's versions: the current one at its start.
    ///
    /// `None` for a copy.
    pub parent: Option<PackVersion>,
    /// This session's version, once saved.
    ///
    /// Later saves replace it.
    pub session_version: Option<PackVersion>,
    /// A deck-only pack, such as an import's.
    ///
    /// Its versions stay in the deck, never in the library.
    pub deck_only: bool,
    /// The pack or theme this tab created empty; collected when the tab leaves untouched.
    #[serde(default)]
    pub created_empty: Option<uuid::Uuid>,
}

/// Pack mode as the interface sees it: the edited pack and its lease flags.
#[derive(Debug, Serialize)]
pub struct EditingView<'a> {
    /// The edited pack, its fields flattened into the view.
    #[serde(flatten)]
    pub mode: &'a PackMode,
    /// True when this tab cannot save its pack now: held elsewhere, or gone.
    pub read_only: bool,
    /// True when this tab may write without first taking the lease.
    pub leased: bool,
    /// The edited pack's name, cleaned for titles and dialogs.
    pub display_name: String,
    /// True while a deck-pack tab holds a save its owner deck has not acknowledged.
    pub unsent: bool,
}

impl std::ops::Deref for EditingView<'_> {
    type Target = PackMode;

    fn deref(&self) -> &PackMode {
        self.mode
    }
}

/// Un layout avec son aperçu (SVG) dans le thème du deck.
#[derive(Debug, Clone, Serialize)]
pub struct LayoutView {
    /// Son kind qualifié (`essentiel/content`).
    pub kind: String,
    /// Son nom dans la langue de l'utilisateur (`param_fields::kind_name`).
    pub label: String,
    /// Sa slide de départ : ce layout, id `layout-<nom>`, à sa valeur d'exemple.
    pub slide: Slide,
    /// L'aperçu de cette slide, en SVG.
    pub svg: Arc<str>,
}

/// Les layouts d'un pack de layouts épinglé, avec leurs aperçus.
#[derive(Debug, Clone, Serialize)]
pub struct LayoutPackView {
    /// Espace de noms du pack dans le deck.
    pub namespace: Namespace,
    /// Nom du pack, lu dans son manifeste.
    pub name: String,
    /// Ses layouts, dans l'ordre des fichiers.
    pub layouts: Vec<LayoutView>,
}

/// Les layouts de chaque pack de layouts épinglé, dans l'ordre des espaces de noms.
#[derive(Debug, Serialize)]
pub struct LayoutsView {
    /// Un pack de layouts épinglé par entrée.
    pub packs: Vec<LayoutPackView>,
}

/// Boîte d'un élément du deck, sélectionnable.
#[derive(Debug, Serialize)]
pub struct Target<'a> {
    /// Id of the element.
    pub id: &'a str,
    /// Widget de l'élément : `text`, `card`, `image`…
    pub kind: &'a str,
    /// Placé librement (déplaçable, redimensionnable).
    pub free: bool,
    /// Left edge of the box, in page pt.
    pub x: f64,
    /// Top edge of the box, in page pt.
    pub y: f64,
    /// Width of the box, in pt.
    pub w: f64,
    /// Height of the box, in pt.
    pub h: f64,
}

impl<'a> Target<'a> {
    /// La cible d'un élément, d'après sa boîte mesurée par le rendu.
    pub(crate) fn element(b: &'a ElementBox, kind: &'a str, free: bool) -> Self {
        Self { id: &b.id, kind, free, x: b.x, y: b.y, w: b.w, h: b.h }
    }
}

/// Largeur d'une slide, en pt : le repère des vues (`SlideView`), slide comme aperçu.
pub(crate) const SLIDE_WIDTH: f64 = 1920.0;
/// Hauteur d'une slide, en pt.
pub(crate) const SLIDE_HEIGHT: f64 = 1080.0;

/// The current slide for the stage: content, rendered page and handles.
#[derive(Debug, Serialize)]
pub struct SlideView<'a> {
    /// The slide, as in the deck.
    pub slide: &'a Slide,
    /// Its page in the document, from 1.
    pub page: usize,
    /// Hash of its page, cache key of its SVG.
    pub hash: PageHash,
    /// Width of the frame, in pt (1920).
    pub width: f64,
    /// Height of the frame, in pt (1080).
    pub height: f64,
    /// Boxes of the selectable elements.
    pub targets: Vec<Target<'a>>,
    /// Champs éditables, dans l'ordre de Tab.
    pub fields: Vec<FieldView<'a>>,
    /// Items de liste de la page (poignées).
    pub items: Vec<ItemView<'a>>,
    /// Listes de la page, même vides (ajout à une liste vide).
    pub lists: Vec<ListView<'a>>,
    /// Zone des éléments libres (pt, page), avec la grille résolue.
    pub zone: Option<&'a ZoneBox>,
}
