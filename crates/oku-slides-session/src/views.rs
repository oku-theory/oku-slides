//! Vues envoyées à l'interface. Elles empruntent la session : sérialisées
//! aussitôt, elles ne copient ni le deck ni le rendu.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize, Serializer};

use oku_slides_core::{Namespace, PackKind, PackPin, PackVersion, Slide, SlideChanges, Version, WidgetFile};
use oku_slides_render::{ElementBox, ZoneBox};

use crate::appearance::Appearance;
use crate::fields::FieldView;
use crate::items::{ItemView, ListView};

/// Empreinte d'une page (clé de cache de son SVG), écrite en hex ; vide si
/// la page manque.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageHash(pub Option<u128>);

impl PageHash {
    /// L'empreinte écrite en 32 chiffres hexadécimaux ; vide si la page manque.
    pub fn hex(self) -> String {
        self.0.map(|h| format!("{h:032x}")).unwrap_or_default()
    }
}

impl Serialize for PageHash {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.hex())
    }
}

#[derive(Debug, Serialize)]
pub struct SlideSummary<'a> {
    pub id: &'a str,
    /// Widget de la slide (`essentiel/title`, `essentiel/content`…).
    pub kind: &'a str,
    /// Son paramètre `title`, sinon (paramètre lié) le titre du deck ;
    /// variables remplacées.
    pub title: Option<String>,
    /// Page dans le document, à partir de 1.
    pub page: usize,
    pub hash: PageHash,
}

#[derive(Debug, Serialize)]
pub struct Snapshot<'a> {
    pub path: Option<&'a Path>,
    pub name: String,
    pub dirty: bool,
    /// Thème du deck : son espace de noms.
    pub theme: Option<&'a oku_slides_core::Namespace>,
    /// Les thèmes épinglés par le deck : son thème et ceux qu'il étend, versions embarquées.
    pub theme_pins: &'a std::collections::BTreeMap<oku_slides_core::Namespace, oku_slides_core::ThemePin>,
    /// Nom du thème tel qu'embarqué par le deck (absent de la bibliothèque s'il vient d'un autre fichier).
    pub theme_name: Option<String>,
    /// Thèmes de la bibliothèque dans leur version courante : intégrés, puis ceux de l'utilisateur.
    pub themes: &'a [crate::ThemeInfo],
    pub title: Option<String>,
    /// Titre tel que saisi (avec ses {{variables}}), pour l'éditer.
    pub title_source: Option<&'a str>,
    /// Langue du deck (`params.lang`) : les grilles écrivent les nombres comme les slides.
    pub lang: Option<&'a str>,
    pub slides: Vec<SlideSummary<'a>>,
    pub can_undo: bool,
    pub undo_label: Option<String>,
    pub can_redo: bool,
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
    /// Mode pack : le pack en cours de modification et le thème édité.
    pub editing: Option<EditingView<'a>>,
    /// Le deck épingle un pack qui n'appartient qu'à lui (import, pack `deck`).
    ///
    /// L'atelier l'édite depuis sa version embarquée ; ses versions restent dans le deck.
    pub deck_pack: bool,
    /// Bandeau du pack, en surface A2UI : le deck embarque une autre version que ma version courante.
    pub pack_notice: Option<crate::agent::Surface>,
    /// Widgets des packs en cours, qualifiés (`<namespace>/<nom>`, sans `_commun`), sans sonde.
    ///
    /// Ceux du pack édité en mode pack, sinon ceux de chaque pack épinglé.
    pub pack_widgets: Vec<String>,
    /// Widgets d'élément du pack au registre : ceux de la barre d'ajout.
    pub add_widgets: Vec<AddWidget<'a>>,
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

/// Un widget de pack à ajouter : nom (`kind`), label et description.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AddWidget<'a> {
    pub name: &'a str,
    pub label: Option<&'a str>,
    /// Ce que montre le widget, en une phrase.
    pub description: Option<&'a str>,
}

/// La version que montre l'historique, et ce qui a changé depuis.
#[derive(Debug, Serialize)]
pub struct HistoryView<'a> {
    /// Cran montré, de `0` à `count - 1` (la version la plus récente).
    pub index: usize,
    /// Nombre de versions (la liste complète : `history_versions`).
    pub count: usize,
    pub version: &'a Version,
    /// Slides de cette version comparées au deck courant.
    pub changes: &'a SlideChanges,
}

/// Ce que l'interface fait garder à l'hôte : slide courante, grille, fantômes, largeurs des colonnes,
/// sous-onglets de widgets, code source des widgets déplié et sa largeur, hauteur de la bande sous la
/// scène, sections repliées de la pile.
///
/// Les champs inconnus d'un `state.json` plus ancien (`board`, `code_place`,
/// `code_height`, retirés par t-526 ; `case`, retiré par t-541) sont ignorés à la lecture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiState {
    #[serde(default)]
    pub slide: Option<String>,
    /// Grille de la slide toujours visible.
    #[serde(default = "shown")]
    pub grid: bool,
    /// Fantômes visibles : zone, widgets et champs annotés.
    #[serde(default)]
    pub ghosts: bool,
    /// Largeur de la pile en px, choisie au séparateur ; `None` : largeur par défaut.
    #[serde(default)]
    pub pile: Option<u32>,
    /// Largeur du fil de l'agent en px, choisie au séparateur ; `None` : largeur par défaut.
    #[serde(default)]
    pub notes: Option<u32>,
    /// Sous-onglets de widgets ouverts en mode pack, dans l'ordre d'ouverture.
    #[serde(default)]
    pub widgets: Vec<String>,
    /// Sous-onglet actif ; `None` : « Layouts ».
    #[serde(default)]
    pub widget: Option<String>,
    /// Code source des widgets déplié à droite de l'aperçu ; replié par défaut.
    #[serde(default)]
    pub code: bool,
    /// Largeur du code en px, choisie au séparateur ; `None` : proportion par défaut.
    #[serde(default)]
    pub code_width: Option<u32>,
    /// Sections repliées (miniatures, widgets) ; toutes ouvertes par défaut.
    #[serde(default)]
    pub folded: Vec<PileSection>,
}

/// Section repliable de l'interface : les miniatures ou les widgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PileSection {
    /// Les miniatures des slides, hors mode pack.
    Slides,
    /// Les miniatures des layouts, en mode pack.
    Layouts,
    /// Les fichiers de widgets du pack.
    Widgets,
}

/// L'état d'interface d'un `state.json` gardé : une section repliée retirée depuis de l'interface
/// (la carte des paramètres) est oubliée au lieu de rendre l'état illisible. Une requête `ui`,
/// elle, refuse toute section inconnue.
pub(crate) fn kept_ui<'de, D: serde::Deserializer<'de>>(d: D) -> Result<UiState, D::Error> {
    let mut value = serde_json::Value::deserialize(d)?;
    if let Some(folded) = value.get_mut("folded").and_then(serde_json::Value::as_array_mut) {
        folded.retain(|s| serde_json::from_value::<PileSection>(s.clone()).is_ok());
    }
    serde_json::from_value(value).map_err(serde::de::Error::custom)
}

fn shown() -> bool {
    true
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            slide: None,
            grid: true,
            ghosts: false,
            pile: None,
            notes: None,
            widgets: Vec::new(),
            widget: None,
            code: false,
            code_width: None,
            folded: Vec::new(),
        }
    }
}

/// Nombre maximal de sous-onglets gardés.
pub const MAX_WIDGET_TABS: usize = 64;

impl UiState {
    /// L'état admis : sections repliées sans doublon ; sous-onglets vidés hors
    /// du mode pack, sinon noms valides, sans doublon, au plus
    /// `MAX_WIDGET_TABS`, actif parmi eux.
    pub fn sanitized(mut self, pack_mode: bool) -> Self {
        let mut folded = std::collections::BTreeSet::new();
        self.folded.retain(|s| folded.insert(*s));
        if !pack_mode {
            self.widgets.clear();
            self.widget = None;
            return self;
        }
        let mut seen = std::collections::BTreeSet::new();
        self.widgets.retain(|n| WidgetFile::parse(n).is_some() && seen.insert(n.clone()));
        self.widgets.truncate(MAX_WIDGET_TABS);
        if self.widget.as_ref().is_some_and(|w| !self.widgets.contains(w)) {
            self.widget = None;
        }
        self
    }
}

#[cfg(test)]
#[path = "views_tests.rs"]
mod views_tests;

/// Un onglet vu de sa barre : titre, travail non enregistré, mode.
///
/// Aussi gardé dans `workspace.json` : la barre se montre au lancement sans
/// relire les sessions (relu, il n'est pas digne de confiance).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabInfo {
    /// Nom du fichier du deck, sinon son titre.
    pub title: String,
    /// Travail non enregistré : le deck, ou le pack en cours d'édition.
    pub dirty: bool,
    /// L'onglet édite un pack (mode pack).
    #[serde(default)]
    pub editing_pack: bool,
    /// Fichier du deck, absolu et normalisé : un fichier ouvert une fois.
    pub path: Option<PathBuf>,
    /// Deck neuf jamais touché : ouvrir un fichier le remplace.
    pub pristine: bool,
    /// Identité du deck (son replica) : une autre copie du même deck se
    /// reconnaît sans réveiller l'onglet. Absente d'un `workspace.json` plus ancien.
    #[serde(default)]
    pub deck_uid: Option<String>,
    /// Langue de l'aide que montre l'onglet ; `None` : un deck.
    #[serde(default)]
    pub help: Option<String>,
    /// Id du thème du deck : l'aide ouverte depuis cet onglet le prend.
    #[serde(default)]
    pub theme: Option<uuid::Uuid>,
}

/// Édition d'un pack : un pack de layouts y a une slide par layout, à sa valeur d'exemple.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackMode {
    /// Id du pack édité.
    pub pack: uuid::Uuid,
    /// Nom du pack édité, lu dans son manifeste.
    pub name: String,
    /// Sorte du pack édité : widgets ou layouts.
    pub kind: PackKind,
    /// Pack intégré : l'enregistrer crée la version de l'utilisateur sous son id.
    pub builtin: bool,
    /// Version d'où viennent widgets et description : la dernière enregistrée.
    pub base: PackVersion,
    /// Parente des versions de la session : la courante à son début.
    ///
    /// `None` pour une copie.
    pub parent: Option<PackVersion>,
    /// La version de cette session, une fois enregistrée.
    ///
    /// Les enregistrements suivants la remplacent.
    pub session_version: Option<PackVersion>,
    /// Pack du deck seul, celui d'un import.
    ///
    /// Ses versions restent dans le deck, jamais dans la bibliothèque.
    pub deck_only: bool,
}

/// Le mode pack vu par l'interface : le pack édité et le brouillon du thème.
#[derive(Debug, Serialize)]
pub struct EditingView<'a> {
    #[serde(flatten)]
    pub mode: &'a PackMode,
    pub theme: crate::ThemeEditView,
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
    pub id: &'a str,
    /// Widget de l'élément : `text`, `card`, `image`…
    pub kind: &'a str,
    /// Placé librement (déplaçable, redimensionnable).
    pub free: bool,
    pub x: f64,
    pub y: f64,
    pub w: f64,
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

#[derive(Debug, Serialize)]
pub struct SlideView<'a> {
    pub slide: &'a Slide,
    pub page: usize,
    pub hash: PageHash,
    pub width: f64,
    pub height: f64,
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
