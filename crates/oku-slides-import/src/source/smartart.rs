//! SmartArt sémantique : disposition et arbre des nœuds, lus dans le modèle de données.

use serde::Serialize;

use super::text::TextBox;

/// Un SmartArt tel que son modèle de données le décrit, dessin à part.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SmartArt {
    /// Identifiant de disposition (`layoutDef@uniqueId`, sinon `prSet@loTypeId`).
    pub layout: Option<String>,
    /// Catégorie de disposition (`process`, `cycle`, `hierarchy`…).
    pub category: Option<String>,
    /// Nœuds en ordre de lecture : parcours préfixe des liens parent-enfant.
    pub nodes: Vec<SmartArtNode>,
}

/// Un nœud de SmartArt (point `node` ou `asst` du modèle de données).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SmartArtNode {
    /// Son `modelId`, assaini.
    pub id: String,
    /// Profondeur dans l'arbre, 0 pour un nœud de premier niveau.
    pub depth: u8,
    /// Son texte (vide s'il n'en a pas).
    pub text: TextBox,
}

impl SmartArt {
    /// Les nœuds qui portent un texte non vide (autre que des espaces).
    pub fn texted(&self) -> impl Iterator<Item = &SmartArtNode> {
        self.nodes.iter().filter(|n| n.text.paragraphs.iter().flat_map(|p| &p.runs).any(|r| !r.text.trim().is_empty()))
    }
}
