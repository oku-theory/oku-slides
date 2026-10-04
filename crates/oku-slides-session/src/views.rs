//! Vues envoyées à l'interface. Elles empruntent la session : sérialisées
//! aussitôt, elles ne copient ni le deck ni le rendu.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize, Serializer};

use oku_slides_core::{PackRef, PackVersion, Slide, SlideChanges, Version, WidgetFile};
use oku_slides_render::{ElementBox, ZoneBox};

use crate::appearance::Appearance;
use crate::fields::FieldView;
use crate::pack::{PackInfo, Template};

/// Empreinte d'une page (clé de cache de son SVG), écrite en hex ; vide si
/// la page manque.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageHash(pub Option<u128>);

impl Serialize for PageHash {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Some(h) => s.collect_str(&format_args!("{h:032x}")),
            None => s.serialize_str(""),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SlideSummary<'a> {
    pub id: &'a str,
    /// Widget de la slide (`title`, `content`…).
    pub kind: &'a str,
    /// Son paramètre `title`, sinon (paramètre lié) le titre du deck ;
    /// variables remplacées.
    pub title: Option<String>,
    /// Nom de la slide quand elle sert de modèle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<&'a str>,
    /// Page dans le document, à partir de 1.
    pub page: usize,
    pub hash: PageHash,
}

#[derive(Debug, Serialize)]
pub struct Snapshot<'a> {
    pub path: Option<&'a Path>,
    pub name: String,
    pub dirty: bool,
    pub theme: Option<&'a str>,
    pub themes: &'static [&'static str],
    pub title: Option<String>,
    /// Titre tel que saisi (avec ses {{variables}}), pour l'éditer.
    pub title_source: Option<&'a str>,
    pub slides: Vec<SlideSummary<'a>>,
    pub can_undo: bool,
    pub undo_label: Option<String>,
    pub can_redo: bool,
    pub redo_label: Option<String>,
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
    /// Pack de modèles du deck (id et version embarquée).
    pub templates: Option<&'a PackRef>,
    /// Mode d'édition des modèles : le pack en cours de modification.
    pub editing: Option<&'a TemplateMode>,
    /// Bandeau du pack : le deck embarque une autre version que ma version courante.
    pub pack_notice: Option<crate::PackNoticeView>,
    /// Widgets du pack en cours (fichiers, sans `_commun`), sans sonde.
    pub pack_widgets: Vec<&'a str>,
    /// Widgets d'élément du pack au registre : ceux de la barre d'ajout.
    pub add_widgets: Vec<AddWidget<'a>>,
    /// Révision des essais de widgets : un changement relance les aperçus.
    pub widget_trials: u64,
    /// Slide courante et grille, gardées pour la reprise.
    pub ui: &'a UiState,
    /// Historique ouvert : la version montrée, en lecture seule.
    pub history: Option<HistoryView<'a>>,
}

/// Un widget de pack qu'on ajoute à une slide : son nom (`kind`) et son label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AddWidget<'a> {
    pub name: &'a str,
    pub label: Option<&'a str>,
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
/// sous-onglets de widgets.
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
    /// Sous-onglets de widgets ouverts en mode modèles, dans l'ordre d'ouverture.
    #[serde(default)]
    pub widgets: Vec<String>,
    /// Sous-onglet actif ; `None` : « Modèles ».
    #[serde(default)]
    pub widget: Option<String>,
}

fn shown() -> bool {
    true
}

impl Default for UiState {
    fn default() -> Self {
        Self { slide: None, grid: true, ghosts: false, pile: None, notes: None, widgets: Vec::new(), widget: None }
    }
}

/// Nombre maximal de sous-onglets gardés.
pub const MAX_WIDGET_TABS: usize = 64;

impl UiState {
    /// L'état admis : sous-onglets vidés hors du mode modèles, sinon noms
    /// valides, sans doublon, au plus `MAX_WIDGET_TABS`, actif parmi eux.
    pub fn sanitized(mut self, templates: bool) -> Self {
        if !templates {
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
    /// Travail non enregistré : le deck, ou l'ensemble de modèles en cours.
    pub dirty: bool,
    /// L'onglet édite un ensemble de modèles.
    pub editing_templates: bool,
    /// Fichier du deck, absolu et normalisé : un fichier ouvert une fois.
    pub path: Option<PathBuf>,
    /// Deck neuf jamais touché : ouvrir un fichier le remplace.
    pub pristine: bool,
    /// Identité du deck (son replica) : une autre copie du même deck se
    /// reconnaît sans réveiller l'onglet. Absente d'un `workspace.json` plus ancien.
    #[serde(default)]
    pub deck_uid: Option<String>,
}

/// Édition d'un pack de modèles : chaque slide est un modèle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMode {
    pub set: uuid::Uuid,
    pub name: String,
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
}

/// Un modèle avec son aperçu (SVG) dans le thème courant.
#[derive(Debug, Clone, Serialize)]
pub struct TemplateView {
    #[serde(flatten)]
    pub template: Template,
    pub svg: Arc<str>,
}

/// Aperçus d'un pack de modèles, et la liste des packs.
#[derive(Debug, Serialize)]
pub struct TemplatesView {
    pub set: PackInfo,
    pub sets: Vec<PackInfo>,
    pub templates: Arc<[TemplateView]>,
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
    /// Zone des éléments libres (pt, page), avec la grille résolue.
    pub zone: Option<&'a ZoneBox>,
}
