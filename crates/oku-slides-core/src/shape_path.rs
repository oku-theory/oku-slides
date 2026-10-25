//! Chemin d'une forme `shape: "path"` : paramètre d'hôte `host("path")`,
//! valeur `{ "d": "M 0 0 L 1 1 Z" }`.
//!
//! Mini-syntaxe de `d`, coordonnées relatives au cadre (0 à 1 de gauche à
//! droite et de haut en bas, étirées au cadre comme une image) :
//! `M x y` (début d'un sous-chemin), `L x y`, `C x1 y1 x2 y2 x y`
//! (cubique), `Q x1 y1 x y` (quadratique), `Z` (ferme le sous-chemin) ;
//! nombres décimaux (`-0.25`, `1`, `.5`, sans exposant), au plus
//! ±[`SHAPE_PATH_COORD_MAX`] ; lettres et nombres séparés par des espaces ;
//! le chemin commence par `M` et tient en [`SHAPE_PATH_MAX`] octets.
//! Remplissage non nul : un sous-chemin de sens contraire évide (anneau).
//! Le widget Typst (`widgets.typ`, `_path-commands`) lit la même syntaxe et
//! ne dessine rien d'un chemin qu'il refuse.

use serde::{Deserialize, Serialize};

/// Longueur maximale de `d`, en octets.
pub const SHAPE_PATH_MAX: usize = 65_536;

/// Plus grande coordonnée en valeur absolue : un point peut sortir du cadre
/// (bulle, flèche), pas s'en éloigner sans borne.
pub const SHAPE_PATH_COORD_MAX: f64 = 1000.0;

/// Décimales gardées par [`ShapePath::from_commands`] : moins d'un millième
/// de pt sur un cadre de la taille d'une page.
const DECIMALS: usize = 6;

/// Une commande d'un chemin, coordonnées relatives au cadre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathCommand {
    /// Lifts the pen and goes to point `(x, y)`: start of a subpath.
    Move(f64, f64),
    /// Straight segment to point `(x, y)`.
    Line(f64, f64),
    /// Points de contrôle puis point d'arrivée.
    Cubic(f64, f64, f64, f64, f64, f64),
    /// Point de contrôle puis point d'arrivée.
    Quad(f64, f64, f64, f64),
    /// Closes the subpath with a segment back to its starting point.
    Close,
}

impl PathCommand {
    fn letter(self) -> char {
        match self {
            PathCommand::Move(..) => 'M',
            PathCommand::Line(..) => 'L',
            PathCommand::Cubic(..) => 'C',
            PathCommand::Quad(..) => 'Q',
            PathCommand::Close => 'Z',
        }
    }

    fn numbers(self) -> Vec<f64> {
        match self {
            PathCommand::Move(x, y) | PathCommand::Line(x, y) => vec![x, y],
            PathCommand::Cubic(x1, y1, x2, y2, x, y) => vec![x1, y1, x2, y2, x, y],
            PathCommand::Quad(x1, y1, x, y) => vec![x1, y1, x, y],
            PathCommand::Close => Vec::new(),
        }
    }

    /// La commande de lettre `letter` sur `n`, si `n` en a le compte.
    fn of(letter: &str, n: &[f64]) -> Option<Self> {
        Some(match (letter, n) {
            ("M", [x, y]) => PathCommand::Move(*x, *y),
            ("L", [x, y]) => PathCommand::Line(*x, *y),
            ("C", [x1, y1, x2, y2, x, y]) => PathCommand::Cubic(*x1, *y1, *x2, *y2, *x, *y),
            ("Q", [x1, y1, x, y]) => PathCommand::Quad(*x1, *y1, *x, *y),
            ("Z", []) => PathCommand::Close,
            _ => return None,
        })
    }
}

/// Valeur du paramètre d'hôte `path`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShapePath {
    /// Commands of the path, in SVG `d` attribute format.
    ///
    /// The commands are `M`, `L`, `C`, `Q` and `Z`.
    pub d: String,
}

impl ShapePath {
    /// Le chemin de `commands`, nombres arrondis à six décimales ; `None`
    /// si le résultat serait refusé (vide, sans `M` en tête, coordonnée non
    /// finie ou trop grande, trop long).
    #[must_use]
    pub fn from_commands(commands: &[PathCommand]) -> Option<Self> {
        let mut d = String::new();
        for c in commands {
            if !d.is_empty() {
                d.push(' ');
            }
            d.push(c.letter());
            for n in c.numbers() {
                d.push(' ');
                d.push_str(&number(n)?);
            }
        }
        let path = ShapePath { d };
        path.commands().is_some().then_some(path)
    }

    /// Les commandes de `d`, ou `None` s'il est vide ou mal formé.
    #[must_use]
    pub fn commands(&self) -> Option<Vec<PathCommand>> {
        if self.d.len() > SHAPE_PATH_MAX {
            return None;
        }
        let mut out: Vec<PathCommand> = Vec::new();
        let mut words = self.d.split_whitespace().peekable();
        while let Some(letter) = words.next() {
            let mut n = Vec::new();
            while let Some(w) = words.next_if(|w| !is_letter(w)) {
                n.push(coordinate(w)?);
            }
            let c = PathCommand::of(letter, &n)?;
            if out.is_empty() && !matches!(c, PathCommand::Move(..)) {
                return None;
            }
            out.push(c);
        }
        (!out.is_empty()).then_some(out)
    }
}

fn is_letter(w: &str) -> bool {
    matches!(w, "M" | "L" | "C" | "Q" | "Z")
}

/// Un nombre décimal de la syntaxe, dans les bornes.
fn coordinate(w: &str) -> Option<f64> {
    let digits = w.strip_prefix(['-', '+']).unwrap_or(w);
    let (int, frac) = digits.split_once('.').unwrap_or((digits, ""));
    let decimal = !(int.is_empty() && frac.is_empty())
        && int.bytes().all(|b| b.is_ascii_digit())
        && frac.bytes().all(|b| b.is_ascii_digit());
    let n: f64 = if decimal { w.parse().ok()? } else { return None };
    (n.abs() <= SHAPE_PATH_COORD_MAX).then_some(n)
}

/// `n` écrit sans exposant, à six décimales au plus, sans zéros inutiles.
fn number(n: f64) -> Option<String> {
    if !n.is_finite() || n.abs() > SHAPE_PATH_COORD_MAX {
        return None;
    }
    let s = format!("{n:.DECIMALS$}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    Some(if s == "-0" { "0".to_owned() } else { s.to_owned() })
}

#[cfg(test)]
#[path = "shape_path_tests.rs"]
mod shape_path_tests;
