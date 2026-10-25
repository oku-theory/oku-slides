//! Les kinds des packs intégrés `base` et `essentiel`, que le code Rust écrit ou reconnaît.
//!
//! Seul endroit où un kind intégré s'écrit en clair ; `kinds_tests`
//! vérifie que chacun vaut `qualify(pack, nom)`.

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
    #[must_use]
    pub fn namespace() -> Namespace {
        Namespace::builtin(NAMESPACE)
    }
}

/// Le pack de layouts intégré `essentiel` : titre, sommaire, chapitre, contenu, fin, vide.
pub mod essentiel {
    use crate::Namespace;

    /// L'espace de noms du pack.
    pub const NAMESPACE: &str = "essentiel";
    /// La slide de titre.
    pub const TITLE: &str = "essentiel/title";
    /// Le sommaire.
    pub const TOC: &str = "essentiel/toc";
    /// La slide de chapitre.
    pub const CHAPTER: &str = "essentiel/chapter";
    /// La slide de contenu.
    pub const CONTENT: &str = "essentiel/content";
    /// La slide de fin.
    pub const END: &str = "essentiel/end";
    /// La slide vide.
    pub const BLANK: &str = "essentiel/blank";
    /// Tous les layouts du pack, dans l'ordre de déclaration.
    pub const ALL: [&str; 6] = [TITLE, TOC, CHAPTER, CONTENT, END, BLANK];

    /// L'espace de noms `essentiel`.
    #[must_use]
    pub fn namespace() -> Namespace {
        Namespace::builtin(NAMESPACE)
    }
}

/// Vrai pour un kind livré par un pack intégré (`base` ou `essentiel`).
#[must_use]
pub fn is_builtin_kind(kind: &str) -> bool {
    base::ALL.contains(&kind) || essentiel::ALL.contains(&kind)
}

#[cfg(test)]
#[path = "kinds_tests.rs"]
mod kinds_tests;
