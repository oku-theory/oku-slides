//! Vues envoyées à l'interface. Elles empruntent la session : sérialisées
//! aussitôt, elles ne copient ni le deck ni le rendu.

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize, Serializer};

use oku_slides_core::{Slide, SlideChanges, Version};
use oku_slides_render::{ElementBox, FieldBox, TextStyle, ZoneBox};

use crate::appearance::Appearance;
use crate::templates::{SetInfo, Template};

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
    /// Ensemble de modèles du deck.
    pub templates: &'a str,
    /// Mode d'édition des modèles : l'ensemble en cours de modification.
    pub editing: Option<&'a TemplateMode>,
    /// Slide courante et grille, gardées pour la reprise.
    pub ui: &'a UiState,
    /// Historique ouvert : la version montrée, en lecture seule.
    pub history: Option<HistoryView<'a>>,
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

/// Ce que l'interface fait garder à l'hôte : slide courante et grille.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiState {
    #[serde(default)]
    pub slide: Option<String>,
    /// Grille de la slide toujours visible.
    #[serde(default = "shown")]
    pub grid: bool,
}

fn shown() -> bool {
    true
}

impl Default for UiState {
    fn default() -> Self {
        Self { slide: None, grid: true }
    }
}

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

/// Édition d'un ensemble de modèles : chaque slide est un modèle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMode {
    pub set: String,
    pub name: String,
    /// Ensemble intégré : l'enregistrer en crée une copie.
    pub builtin: bool,
}

/// Un modèle avec son aperçu (SVG) dans le thème courant.
#[derive(Debug, Clone, Serialize)]
pub struct TemplateView {
    #[serde(flatten)]
    pub template: Template,
    pub svg: Arc<str>,
}

/// Aperçus d'un ensemble de modèles, et la liste des ensembles.
#[derive(Debug, Serialize)]
pub struct TemplatesView {
    pub set: SetInfo,
    pub sets: Vec<SetInfo>,
    pub templates: Arc<[TemplateView]>,
}

/// Ce que désigne une boîte : un élément (et son widget) ou un champ de slide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "target", content = "kind", rename_all = "lowercase")]
pub enum TargetKind<'a> {
    /// Widget de l'élément : `text`, `card`, `image`…
    Element(&'a str),
    /// Paramètre de la slide : `title`, `subtitle`, `kicker`, `text`…
    Field(&'a str),
}

/// Boîte sélectionnable : un élément du deck ou un champ de slide (`@title`…).
#[derive(Debug, Serialize)]
pub struct Target<'a> {
    /// Id de l'élément, ou `@<paramètre>` pour un champ de la slide.
    pub id: Cow<'a, str>,
    #[serde(flatten)]
    pub kind: TargetKind<'a>,
    /// Paramètre édité en place : pour un élément, son premier paramètre
    /// `rich` (ordre du schéma) ; pour un champ, son nom. Absent : la boîte
    /// n'est pas éditable en place (`shape`, `image`…).
    pub param: Option<&'a str>,
    /// Placé librement (déplaçable, redimensionnable).
    pub free: bool,
    /// Texte éditable, un paragraphe par ligne (`- ` en tête pour une puce).
    pub text: Option<Cow<'a, str>>,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// Zone disponible pour le texte : début et largeur (contient la boîte).
    pub ax: f64,
    pub aw: f64,
    /// Typographie du rendu, pour un éditeur calé sur le texte.
    pub style: Option<&'a TextStyle>,
}

impl<'a> Target<'a> {
    /// La cible d'un élément, d'après sa boîte mesurée par le rendu.
    pub(crate) fn element(
        b: &'a ElementBox,
        kind: &'a str,
        param: Option<&'a str>,
        free: bool,
        text: Option<Cow<'a, str>>,
    ) -> Self {
        Self {
            id: Cow::Borrowed(&b.id),
            kind: TargetKind::Element(kind),
            param,
            free,
            text,
            x: b.x,
            y: b.y,
            w: b.w,
            h: b.h,
            ax: b.ax.unwrap_or(b.x).min(b.x),
            aw: b.aw.unwrap_or(b.w).max(b.w),
            style: b.style.as_ref(),
        }
    }

    /// La cible d'un champ de la slide (id `@<paramètre>`), d'après sa boîte.
    pub(crate) fn field(f: &'a FieldBox, text: Cow<'a, str>) -> Self {
        Self {
            id: Cow::Owned(format!("@{}", f.field)),
            kind: TargetKind::Field(&f.field),
            param: Some(&f.field),
            free: false,
            text: Some(text),
            x: f.x,
            y: f.y,
            w: f.w,
            h: f.h,
            ax: f.ax.unwrap_or(f.x).min(f.x),
            aw: f.aw.unwrap_or(f.w).max(f.w),
            style: f.style.as_ref(),
        }
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
    /// Zone des éléments libres (pt, page), avec la grille résolue.
    pub zone: Option<&'a ZoneBox>,
}
