//! Les kinds du pack de widgets intégré `base`, que le code Rust écrit ou reconnaît.
//!
//! Seul endroit où un kind de `base` s'écrit en clair ; `kinds_tests`
//! vérifie que chacun vaut `qualify(base, nom)`.

/// Le pack de widgets intégré `base` : texte, carte, image, forme, graphique, tableau, pile.
pub mod base {
    use crate::Namespace;

    /// L'espace de noms du pack.
    pub const NAMESPACE: &str = "base";
    /// Un texte riche.
    pub const TEXT: &str = "base/text";
    /// Une carte : surtitre, titre, corps.
    pub const CARD: &str = "base/card";
    /// Une image d'un asset du deck.
    pub const IMAGE: &str = "base/image";
    /// Une forme : rectangle, ellipse, ligne ou chemin.
    pub const SHAPE: &str = "base/shape";
    /// Un graphique dessiné par l'hôte.
    pub const CHART: &str = "base/chart";
    /// Un tableau de données.
    pub const TABLE: &str = "base/table";
    /// Une pile d'éléments en lignes, colonnes ou grille.
    pub const STACK: &str = "base/stack";
    /// Tous les widgets du pack, dans l'ordre de déclaration.
    pub const ALL: [&str; 7] = [TEXT, CARD, IMAGE, SHAPE, CHART, TABLE, STACK];

    /// L'espace de noms `base`.
    pub fn namespace() -> Namespace {
        Namespace::builtin(NAMESPACE)
    }
}

#[cfg(test)]
#[path = "kinds_tests.rs"]
mod kinds_tests;
