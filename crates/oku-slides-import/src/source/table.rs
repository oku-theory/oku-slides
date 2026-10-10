//! Tableaux résolus (style de tableau appliqué).

use serde::Serialize;

use super::style::{Fill, Stroke};
use super::text::TextBox;

/// Bordures d'une cellule.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Borders {
    /// Gauche.
    pub l: Option<Stroke>,
    /// Haut.
    pub t: Option<Stroke>,
    /// Droite.
    pub r: Option<Stroke>,
    /// Bas.
    pub b: Option<Stroke>,
}

/// Cellule.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Cell {
    /// Texte (vide si la cellule est couverte).
    pub text: TextBox,
    /// Remplissage.
    pub fill: Fill,
    /// Bordures.
    pub borders: Borders,
    /// Lignes couvertes, 1.. (bornées à la grille).
    pub row_span: u32,
    /// Colonnes couvertes, 1.. (bornées à la grille).
    pub col_span: u32,
    /// Couverte par la fusion d'une autre cellule.
    pub covered: bool,
}

/// Options de mise en forme du tableau (`tblPr`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct TableFlags {
    /// Première ligne mise en valeur (en-tête).
    pub first_row: bool,
    /// Dernière ligne mise en valeur (total).
    pub last_row: bool,
    /// Première colonne mise en valeur.
    pub first_col: bool,
    /// Dernière colonne mise en valeur.
    pub last_col: bool,
    /// Lignes en bandes.
    pub band_row: bool,
    /// Colonnes en bandes.
    pub band_col: bool,
}

/// Tableau.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Table {
    /// Largeur des colonnes, en pt.
    pub col_widths: Vec<f64>,
    /// Hauteur des lignes, en pt.
    pub row_heights: Vec<f64>,
    /// Options de mise en forme.
    pub flags: TableFlags,
    /// Cellules, ligne par ligne.
    pub rows: Vec<Vec<Cell>>,
}
