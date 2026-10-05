//! Les requêtes de l'interface (même forme pour Tauri et HTTP).

use std::path::PathBuf;

use serde::Deserialize;
use uuid::Uuid;

use oku_slides_core::Op;

use crate::views::UiState;

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
    Undo,
    Redo,
    Schema,
    /// Registre des widgets du deck ouvert (base, surchargée par son pack).
    Widgets,
    /// Réglages de l'hôte (surface ou `null`) : le widget s'il en a, sinon le modèle `slide`.
    HostSettings {
        slide: String,
        #[serde(default)]
        widget: Option<String>,
    },
    /// Applique le réglage `key` du modèle `slide` (mode modèles, annulable).
    HostParam {
        slide: String,
        key: String,
        value: serde_json::Value,
    },
    /// Change le type d'un graphique ; ses données sont converties. Annulable.
    ChartKind {
        label: String,
        slide: String,
        element: String,
        kind: oku_slides_chart::SeriesType,
    },
    /// Modèles d'un pack, rendus avec le thème du deck.
    ///
    /// Par défaut, celui du deck, dans sa version embarquée.
    Templates {
        #[serde(default)]
        set: Option<Uuid>,
    },
    /// Liste des packs de modèles, dans leur version courante.
    TemplateSets,
    /// Passe en édition de la version courante d'un pack (le deck est mis de côté).
    EditTemplates {
        set: Uuid,
    },
    /// Enregistre le pack édité dans la version de cette session d'édition.
    ///
    /// `as_new` : une copie (nouvel id).
    SaveTemplates {
        #[serde(default)]
        as_new: bool,
    },
    /// Quitte l'édition des modèles et retrouve le deck.
    CloseTemplates,
    /// Supprime un pack de l'utilisateur, toutes versions comprises.
    ///
    /// Refusé sous l'id d'un pack intégré.
    DeleteTemplateSet {
        set: Uuid,
    },
    /// Slide courante, grille et fantômes de l'interface (gardés pour la reprise).
    Ui(UiState),
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
    /// Bandeau du pack : le deck passe à ma version courante (annulable).
    PackUseMine,
    /// Bandeau du pack : sa version devient ma version courante.
    PackMakeCurrent,
    /// Ferme le bandeau du pack pour la session.
    PackNoticeClose,
    /// Statut des fichiers de widgets du pack en cours (tableau de `WidgetStatus`).
    WidgetStatus,
    /// Source d'un fichier de widget du pack en cours.
    WidgetSource {
        name: String,
    },
    /// Crée ou remplace un fichier de widget ; une source vide le supprime (`WidgetChange`).
    SetWidget {
        name: String,
        source: String,
    },
    /// Supprime un fichier de widget ; refusé s'il est utilisé par un modèle.
    RemoveWidget {
        name: String,
    },
    /// Renomme un fichier de widget dans la version éditée.
    RenameWidget {
        from: String,
        to: String,
    },
    /// Aperçu SVG d'un widget seul (`WidgetPreview`), aux valeurs d'exemple
    /// du widget ; `params` les complète.
    PreviewWidget {
        name: String,
        #[serde(default)]
        params: serde_json::Map<String, serde_json::Value>,
    },
    /// Règle une valeur d'exemple du widget `name` ; `null` ou la valeur neutre
    /// l'effacent, un texte vidé reste vide (mode modèles, dans le pack).
    SetExampleValue {
        name: String,
        param: String,
        value: serde_json::Value,
    },
    /// Change la liste `param` des valeurs d'exemple ; sans valeur gardée, elle
    /// part des items par défaut du schéma (mode modèles).
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
    /// Exhaustif à dessein : une requête ajoutée doit dire si elle change le deck.
    pub fn changes_deck(&self) -> bool {
        match self {
            Self::Save { .. }
            | Self::Apply { .. }
            | Self::Undo
            | Self::Redo
            | Self::ChartKind { .. }
            | Self::EditTemplates { .. }
            | Self::CloseTemplates
            | Self::Merge { .. }
            | Self::HistoryOpen
            | Self::HistorySeek { .. }
            | Self::HistoryRestore
            | Self::HistoryClose
            | Self::Move { .. }
            | Self::PackUseMine
            | Self::SetWidget { .. }
            | Self::RemoveWidget { .. }
            | Self::RenameWidget { .. }
            | Self::SetExampleValue { .. }
            | Self::EditExampleList { .. }
            | Self::HostParam { .. } => true,
            Self::Snapshot
            | Self::Slide { .. }
            | Self::Search { .. }
            | Self::Svg { .. }
            | Self::Schema
            | Self::Widgets
            | Self::HostSettings { .. }
            | Self::Templates { .. }
            | Self::TemplateSets
            | Self::SaveTemplates { .. }
            | Self::DeleteTemplateSet { .. }
            | Self::SaveCopy { .. }
            | Self::Ui(_)
            | Self::HistoryVersions
            | Self::PackMakeCurrent
            | Self::PackNoticeClose
            | Self::WidgetStatus
            | Self::WidgetSource { .. }
            | Self::PreviewWidget { .. } => false,
        }
    }

    /// Requêtes refusées pendant l'historique : elles modifieraient le deck,
    /// l'enregistreraient, ou passeraient aux modèles.
    pub(crate) fn edits(&self) -> bool {
        matches!(
            self,
            Self::Apply { .. }
                | Self::Undo
                | Self::Redo
                | Self::Save { .. }
                | Self::Merge { .. }
                | Self::Move { .. }
                | Self::SaveCopy { .. }
                | Self::ChartKind { .. }
                | Self::EditTemplates { .. }
                | Self::SaveTemplates { .. }
                | Self::CloseTemplates
                | Self::DeleteTemplateSet { .. }
                | Self::PackUseMine
                | Self::SetWidget { .. }
                | Self::RemoveWidget { .. }
                | Self::RenameWidget { .. }
                | Self::HostParam { .. }
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
