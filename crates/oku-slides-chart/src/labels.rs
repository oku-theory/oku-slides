//! Libellés par défaut d'un graphique (série sans nom, en-têtes de la grille),
//! dans la langue de l'utilisateur.

use std::sync::LazyLock;

use oku_slides_i18n::{Localizer, english};

/// Marque le numéro dans le modèle « Série { $n } » : le message est traduit
/// une fois, le numéro inséré à chaque série.
const SLOT: &str = "\u{1}";

/// Libellés par défaut, résolus une fois depuis Fluent (`chart-default-*`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChartLabels {
    series: String,
    category: String,
    value: String,
    name: String,
    axis: String,
    total: String,
    x_of: String,
}

static ENGLISH: LazyLock<ChartLabels> = LazyLock::new(|| ChartLabels::new(english()));

impl ChartLabels {
    /// Labels in the language of `l`.
    pub fn new(l: &Localizer) -> Self {
        Self {
            series: l.tr_with("chart-default-series", &[("n", SLOT.into())]),
            category: l.tr("chart-default-category"),
            value: l.tr("chart-default-value"),
            name: l.tr("chart-default-name"),
            axis: l.tr("chart-default-axis"),
            total: l.tr("chart-default-total"),
            x_of: l.tr_with("chart-default-x", &[("name", SLOT.into())]),
        }
    }

    /// Les libellés anglais, pour valider sans langue d'affichage.
    #[must_use]
    pub fn english() -> &'static Self {
        &ENGLISH
    }

    /// Nom de la série `index` (à partir de 0) quand elle n'en a pas.
    #[must_use]
    pub fn series(&self, index: usize) -> String {
        self.series.replace(SLOT, &(index + 1).to_string())
    }

    /// En-tête de la colonne des catégories (barres, lignes).
    #[must_use]
    pub fn category(&self) -> &str {
        &self.category
    }

    /// En-tête de la colonne des valeurs (secteurs).
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// En-tête de la colonne des noms (secteurs).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// En-tête de la colonne des axes (radar).
    #[must_use]
    pub fn axis(&self) -> &str {
        &self.axis
    }

    /// En-tête de la colonne qui coche les totaux (cascade).
    #[must_use]
    pub fn total(&self) -> &str {
        &self.total
    }

    /// En-tête de la colonne des abscisses d'une série de points.
    #[must_use]
    pub fn x_of(&self, series: &str) -> String {
        self.x_of.replace(SLOT, series)
    }
}

impl Default for ChartLabels {
    fn default() -> Self {
        Self::english().clone()
    }
}

#[cfg(test)]
#[path = "labels_tests.rs"]
mod labels_tests;
