//! Formes, géométries et images.

use std::collections::BTreeMap;

use serde::Serialize;

use super::style::{Fill, Stroke};
use super::text::TextBox;

/// Commande de tracé ; coordonnées en fraction (0..1) de la boîte de l'item.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "cmd")]
pub enum PathCmd {
    /// Déplacement sans tracé.
    MoveTo { x: f64, y: f64 },
    /// Segment.
    LineTo { x: f64, y: f64 },
    /// Courbe de Bézier cubique.
    CubicTo { x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64 },
    /// Courbe de Bézier quadratique.
    QuadTo { x1: f64, y1: f64, x: f64, y: f64 },
    /// Arc OOXML : rayons (fraction de la boîte), départ et balayage en degrés.
    ArcTo { wr: f64, hr: f64, start: f64, swing: f64 },
    /// Fermeture du sous-tracé.
    Close,
}

/// Géométrie d'une forme.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Geometry {
    /// Forme prédéfinie OOXML.
    Preset {
        /// Nom (`rect`, `roundRect`, `line`…).
        name: String,
        /// Ajustements (`adj`, `adj1`…) en 100 000ᵉ.
        adj: BTreeMap<String, f64>,
    },
    /// Tracé libre (`custGeom`).
    Path {
        /// Sous-tracés (64 au plus ; 10 000 commandes au plus au total).
        paths: Vec<SubPath>,
    },
}

/// Sous-tracé d'un tracé libre.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SubPath {
    /// Commandes.
    pub cmds: Vec<PathCmd>,
    /// Rempli (`fill` autre que `none`).
    pub fill: bool,
    /// Contour tracé (`stroke` autre que `false`).
    pub stroke: bool,
}

/// Forme dessinée, éventuellement porteuse de texte.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Shape {
    /// Géométrie.
    pub geom: Geometry,
    /// Remplissage.
    pub fill: Fill,
    /// Contour, s'il est peint.
    pub stroke: Option<Stroke>,
    /// Texte, s'il n'est pas vide.
    pub text: Option<TextBox>,
    /// Formes qu'un connecteur relie (`stCxn`, `endCxn`), s'il en relie une.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<Connection>,
}

/// Point de connexion d'une forme : l'item et l'indice de son site (`idx`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Site {
    /// Id de l'item relié, dans la même part (`<owner>/<cNvPr id>`, sans suffixe d'unicité).
    pub item: String,
    /// Indice du site de connexion de sa géométrie.
    pub idx: u32,
}

/// Extrémités reliées d'un connecteur ; une extrémité libre est `None`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Connection {
    /// Début du tracé (`stCxn`).
    pub start: Option<Site>,
    /// Fin du tracé (`endCxn`).
    pub end: Option<Site>,
}

/// Rognage d'une image, en fraction de chaque bord (`l + r < 1`, `t + b < 1`).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize)]
pub struct Crop {
    /// Gauche.
    pub l: f64,
    /// Haut.
    pub t: f64,
    /// Droite.
    pub r: f64,
    /// Bas.
    pub b: f64,
}

/// Image posée.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Picture {
    /// Identifiant d'asset.
    pub asset: String,
    /// Rognage.
    pub crop: Crop,
}
