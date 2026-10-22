//! Préréglages d'un layout, que l'import retrofit choisit par affinité.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::pack::WidgetFile;

/// Un préréglage nommé d'un layout, avec ses indices d'affinité.
///
/// Le retrofit compare chaque slide importée aux indices (`role`,
/// `arrangement`, `blocks`, `title`, `kicker`) et pose `params` sur la slide
/// du layout choisi.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutProfile {
    /// Identifiant du préréglage, unique dans son pack.
    pub id: String,
    /// Nom affiché.
    pub name: String,
    /// Le layout du pack que ce préréglage utilise.
    pub layout: WidgetFile,
    /// Le rôle de slide que ce préréglage sert.
    pub role: SlideRole,
    /// La disposition du contenu qu'il attend.
    pub arrangement: Arrangement,
    /// Nombre de blocs de contenu du préréglage.
    #[serde(default)]
    pub blocks: u8,
    /// Vrai si le préréglage porte un titre.
    #[serde(default)]
    pub title: bool,
    /// Vrai si le préréglage porte un surtitre.
    #[serde(default)]
    pub kicker: bool,
    /// Paramètres posés sur la slide du layout.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub params: Map<String, Value>,
}

/// Le rôle d'une slide dans le déroulé d'un deck.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlideRole {
    /// Couverture.
    Title,
    /// Ouverture de chapitre.
    Chapter,
    /// Sommaire.
    Toc,
    /// Slide de contenu.
    Content,
    /// Slide de fin.
    End,
    /// Slide vide.
    Blank,
}

/// La disposition du contenu d'une slide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Arrangement {
    /// Disposition par défaut du layout.
    Default,
    /// Blocs en colonnes.
    Columns,
    /// Deux moitiés.
    Split,
    /// Contenu centré.
    Center,
    /// Une citation.
    Quote,
    /// Éléments placés librement.
    Free,
    /// Une grille de cartes.
    Grid,
}

#[cfg(test)]
#[path = "layout_profile_tests.rs"]
mod layout_profile_tests;
