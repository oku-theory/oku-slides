//! Requêtes de structure, leurs résultats, et la vue de l'espace de travail.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{Clip, ImportPick};
use crate::views::TabInfo;

/// Une requête de la barre d'onglets ou de l'hôte.
#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum WorkspaceRequest {
    /// La vue, sans rien changer.
    View,
    /// Langue de l'utilisateur et messages Fluent de l'interface.
    Messages,
    /// Choisit la langue de l'interface (`None` : celle du système), gardée
    /// dans `config.toml` ; répond comme `Messages`. Les messages du cœur
    /// (erreurs, noms par défaut) suivent au prochain lancement.
    SetLocale { locale: Option<String> },
    /// Schéma de l'interface choisi : `{ "scheme": "light" | "dark" | null }`
    /// (`null` : celui du système).
    Scheme,
    /// Choisit le schéma de l'interface (`None` ou inconnu : celui du
    /// système), gardé dans `config.toml` ; répond comme `Scheme`.
    SetScheme { scheme: Option<String> },
    /// Un onglet vierge, à la fin de `window`.
    NewTab {
        /// Label de la fenêtre.
        window: String,
    },
    /// Ouvre un fichier : son onglet s'il est déjà ouvert, sinon un nouvel onglet.
    OpenTab {
        /// Fenêtre d'arrivée d'un nouvel onglet.
        window: String,
        /// Le fichier (normalisé à la réception).
        path: PathBuf,
    },
    /// Importe une présentation PowerPoint dans un nouvel onglet.
    ///
    /// Un deck sans fichier, que l'autosauvegarde range dans le dossier des decks comme un deck
    /// nouveau, et qui garde l'état de son import.
    ///
    /// Comme `OpenTab`, il remplace l'onglet actif s'il est vierge ; deux
    /// imports du même fichier font deux onglets (deux decks neufs). La
    /// cible est à plat : `"mode": "retrofit"` avec `layouts` (ids ou noms de
    /// packs de layouts de la bibliothèque, jamais un dossier ; absent : `essentiel`)
    /// et `theme` facultatif, ou `"mode": "hifi"`.
    ImportTab {
        /// Fenêtre d'arrivée du nouvel onglet.
        window: String,
        /// Le `.pptx` (normalisé à la réception).
        path: PathBuf,
        /// Le mode et sa cible.
        #[serde(flatten)]
        target: crate::import::ImportTarget,
        /// L'agent doit fine-tuner l'onglet importé : la réponse le dit à l'hôte ([`WorkspaceOutcome::Tune`]).
        #[serde(default)]
        tune: bool,
    },
    /// L'aide de l'app, en lecture seule, dans la langue de l'interface :
    /// son onglet s'il est déjà ouvert, sinon un nouvel onglet.
    OpenHelp {
        /// Fenêtre d'arrivée d'un nouvel onglet.
        window: String,
    },
    /// L'onglet qui tient déjà le deck de `path` sous un autre fichier :
    /// `{ "tab": "t-3" | null, "title": … | null }` ; ne change rien.
    SameDeck {
        /// Le fichier (normalisé à la réception).
        path: PathBuf,
    },
    /// Apporte à `to` les assets d'un élément copié dans `from`.
    ///
    /// Avant le collage. `to` les garde en mémoire à la place des précédents, et ne
    /// les range dans son bundle que si le collage les cite : `{ "carried": n }` ;
    /// ne change pas la vue.
    CarryAssets {
        /// Onglet où l'élément a été copié.
        from: String,
        /// Onglet où il est collé.
        to: String,
        /// L'élément copié, tel que le presse-papiers le garde.
        element: Box<oku_slides_core::Element>,
    },
    /// Le dernier élément copié et son numéro : `{ "clip": … | null, "seq": n }`.
    Clip,
    /// Retient l'élément copié pour toutes les fenêtres ; répond comme `Clip`.
    SetClip {
        /// L'élément, son onglet, son texte et la fenêtre qui l'envoie.
        clip: Box<Clip>,
    },
    /// L'onglet devient actif dans sa fenêtre.
    Activate {
        /// Identifiant de l'onglet.
        tab: String,
    },
    /// Ferme un onglet.
    Close {
        /// Identifiant de l'onglet.
        tab: String,
        /// L'utilisateur abandonne le travail non enregistré.
        #[serde(default)]
        discard: bool,
    },
    /// Place l'onglet à `index` dans sa barre.
    Reorder {
        /// Identifiant de l'onglet.
        tab: String,
        /// Place finale (bornée à la barre).
        index: usize,
    },
    /// Vers une autre fenêtre, ou une nouvelle.
    MoveTab {
        /// Identifiant de l'onglet.
        tab: String,
        /// Fenêtre d'arrivée ; `None` : une nouvelle fenêtre.
        #[serde(default)]
        window: Option<String>,
        /// Place dans la barre d'arrivée ; `None` : à la fin.
        #[serde(default)]
        index: Option<usize>,
    },
    /// Une fenêtre avec un onglet vierge.
    NewWindow,
    /// Ferme une fenêtre et ses onglets.
    CloseWindow {
        /// Label de la fenêtre.
        window: String,
        /// Onglets dont l'utilisateur abandonne le travail non enregistré.
        #[serde(default)]
        discard: Vec<String>,
    },
    /// L'agent passe dans sa propre fenêtre.
    DetachAgent,
    /// L'agent revient dans la colonne de droite.
    AttachAgent,
    /// La page a ouvert le dialogue d'import du choix `id` : la vue l'oublie.
    ///
    /// Un choix plus récent (autre `id`) reste.
    ImportPicked {
        /// Numéro du choix traité ([`ImportPick::id`]).
        id: u64,
    },
}

/// Un onglet de la vue.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TabView {
    /// Identifiant `t-<n>`.
    pub id: String,
    /// Titre, travail non enregistré, fichier.
    #[serde(flatten)]
    pub info: TabInfo,
}

/// Une fenêtre de la vue : ses onglets et l'onglet actif.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WindowView {
    /// Label `w-<n>`, aussi celui de la fenêtre Tauri.
    pub label: String,
    /// Identifiant de l'onglet actif.
    pub active: String,
    /// Onglets, dans l'ordre de la barre.
    pub tabs: Vec<TabView>,
}

/// Ce que l'interface voit : les fenêtres et leurs onglets.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkspaceView {
    /// Numéro de la vue, croissant : une vue plus petite que celle déjà
    /// montrée est plus ancienne (réponse doublée par un événement) et écartée.
    pub generation: u64,
    /// Dernière fenêtre active.
    pub focused: String,
    /// Fenêtres, dans l'ordre de création.
    pub windows: Vec<WindowView>,
    /// L'agent est dans sa propre fenêtre : la colonne de droite ne le montre plus.
    pub agent_detached: bool,
    /// Le dernier `.pptx` reçu de l'OS, dont sa fenêtre ouvre le dialogue d'import.
    pub import_pick: Option<ImportPick>,
}

/// Ce que [`Workspace::apply`] a fait : la vue à jour, en JSON, dans les deux cas.
#[derive(Debug)]
pub enum WorkspaceOutcome {
    /// La requête a été appliquée.
    Applied(String),
    /// Fermeture de la seule fenêtre refusée, sans y toucher : voir
    /// [`Workspace::is_only_window`], utilisé aussi par l'hôte (fermeture
    /// native) pour le même refus, en aperçu hors verrou.
    LastWindow(String),
    /// Un import appliqué (`ImportTab` avec `tune`).
    ///
    /// L'hôte lance le fine-tuning de l'agent sur l'onglet `tab`, prêt.
    Tune {
        /// La vue à jour.
        json: String,
        /// L'onglet importé.
        tab: String,
    },
}

impl WorkspaceOutcome {
    /// La vue à jour, qu'elle ait changé ou non.
    pub fn into_json(self) -> String {
        match self {
            Self::Applied(json) | Self::LastWindow(json) | Self::Tune { json, .. } => json,
        }
    }

    /// L'onglet importé dont l'agent doit faire le fine-tuning, s'il y en a un.
    pub fn tune_tab(&self) -> Option<&str> {
        match self {
            Self::Tune { tab, .. } => Some(tab),
            Self::Applied(_) | Self::LastWindow(_) => None,
        }
    }

    /// La fenêtre visée était la seule de l'espace de travail : rien n'a été fermé.
    pub fn is_last_window(&self) -> bool {
        matches!(self, Self::LastWindow(_))
    }
}
