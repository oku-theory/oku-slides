//! Pointes des extrémités d'un trait (`headEnd`, `tailEnd`) en petits chemins.
//!
//! Ni Typst ni le cœur n'ont de marqueurs : chaque pointe est un chemin à
//! part, plein (triangle, flèche échancrée, losange, ovale) ou tracé (flèche
//! ouverte). Taille relative à l'épaisseur du trait comme PowerPoint : 2, 3
//! ou 5 fois (`sm`, `med`, `lg`) ; le trait n'est pas raccourci sous la pointe.
//! Un tracé fermé n'a pas de pointes, comme dans PowerPoint.

use oku_slides_core::PathCommand;

use crate::source::{Arrow, ArrowKind, ArrowSize, Stroke};

/// Écart sous lequel deux points sont confondus, en pt.
const EPS: f64 = 1e-6;
/// Épaisseur minimale prise pour dimensionner une pointe, en pt : un filet garde une pointe visible.
const MIN_WIDTH: f64 = 1.0;
/// Constante des quarts d'ellipse en Béziers cubiques.
const KAPPA: f64 = 0.552_284_75;
/// Recul de l'échancrure d'une flèche `stealth`, en fraction de sa longueur.
const STEALTH_NOTCH: f64 = 0.5;

/// Une pointe : son chemin en pt du cadre, plein ou tracé à l'épaisseur du trait.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Head {
    /// Commandes, en pt depuis le coin haut gauche du cadre.
    pub(crate) cmds: Vec<PathCommand>,
    /// Pleine (couleur du trait), sinon tracée.
    pub(crate) filled: bool,
}

/// Les pointes de `stroke` aux extrémités du tracé `cmds` (fractions d'un
/// cadre de `w` × `h` pt), le trait épais de `width` pt.
pub(crate) fn heads(cmds: &[PathCommand], w: f64, h: f64, width: f64, stroke: &Stroke) -> Vec<Head> {
    if cmds.iter().any(|c| matches!(c, PathCommand::Close)) {
        return Vec::new();
    }
    let points: Vec<(f64, f64)> = cmds.iter().flat_map(|c| points(*c)).map(|(x, y)| (x * w, y * h)).collect();
    let width = width.max(MIN_WIDTH);
    let head = stroke.head.zip(tip(points.iter().copied()));
    let tail = stroke.tail.zip(tip(points.iter().rev().copied()));
    [head, tail].into_iter().flatten().map(|(arrow, (at, dir))| shape(arrow, at, dir, width)).collect()
}

/// Les points d'une commande, contrôles compris, dans l'ordre du tracé.
pub(crate) fn points(c: PathCommand) -> Vec<(f64, f64)> {
    match c {
        PathCommand::Move(x, y) | PathCommand::Line(x, y) => vec![(x, y)],
        PathCommand::Cubic(x1, y1, x2, y2, x, y) => vec![(x1, y1), (x2, y2), (x, y)],
        PathCommand::Quad(x1, y1, x, y) => vec![(x1, y1), (x, y)],
        PathCommand::Close => Vec::new(),
    }
}

/// L'extrémité (premier point de `pts`) et la direction unitaire qui en sort :
/// du premier point distinct qui la suit vers elle ; `None` sans tel point.
fn tip(mut pts: impl Iterator<Item = (f64, f64)>) -> Option<((f64, f64), (f64, f64))> {
    let at = pts.next()?;
    let from = pts.find(|p| (p.0 - at.0).hypot(p.1 - at.1) > EPS)?;
    let len = (at.0 - from.0).hypot(at.1 - from.1);
    Some((at, ((at.0 - from.0) / len, (at.1 - from.1) / len)))
}

/// Multiple de l'épaisseur du trait.
fn factor(s: ArrowSize) -> f64 {
    match s {
        ArrowSize::Small => 2.0,
        ArrowSize::Medium => 3.0,
        ArrowSize::Large => 5.0,
    }
}

/// La pointe `arrow` en `at`, tournée vers `dir`.
#[expect(clippy::many_single_char_names, reason = "frame u, v and coordinates x, y of a geometric formula")]
fn shape(arrow: Arrow, at: (f64, f64), dir: (f64, f64), width: f64) -> Head {
    let (len, half) = (factor(arrow.length) * width, factor(arrow.width) * width / 2.0);
    // Repère de la pointe : `u` le long du trait (vers l'extérieur), `v` en travers.
    let p = |u: f64, v: f64| (at.0 + u * dir.0 - v * dir.1, at.1 + u * dir.1 + v * dir.0);
    let poly = |pts: &[(f64, f64)]| -> Vec<PathCommand> {
        let at =
            |(i, &(x, y)): (usize, &(f64, f64))| if i == 0 { PathCommand::Move(x, y) } else { PathCommand::Line(x, y) };
        pts.iter().enumerate().map(at).collect()
    };
    let closed = |pts: &[(f64, f64)]| {
        let mut cmds = poly(pts);
        cmds.push(PathCommand::Close);
        Head { cmds, filled: true }
    };
    match arrow.kind {
        ArrowKind::Triangle => closed(&[p(0.0, 0.0), p(-len, half), p(-len, -half)]),
        ArrowKind::Stealth => closed(&[p(0.0, 0.0), p(-len, half), p(-len * STEALTH_NOTCH, 0.0), p(-len, -half)]),
        ArrowKind::Diamond => closed(&[p(len / 2.0, 0.0), p(0.0, half), p(-len / 2.0, 0.0), p(0.0, -half)]),
        ArrowKind::Open => Head { cmds: poly(&[p(-len, half), p(0.0, 0.0), p(-len, -half)]), filled: false },
        ArrowKind::Oval => {
            let (a, b) = (len / 2.0, half);
            let (ka, kb) = (a * KAPPA, b * KAPPA);
            let cubic = |c1: (f64, f64), c2: (f64, f64), e: (f64, f64)| {
                let ((x1, y1), (x2, y2), (x, y)) = (p(c1.0, c1.1), p(c2.0, c2.1), p(e.0, e.1));
                PathCommand::Cubic(x1, y1, x2, y2, x, y)
            };
            let (sx, sy) = p(a, 0.0);
            let cmds = vec![
                PathCommand::Move(sx, sy),
                cubic((a, kb), (ka, b), (0.0, b)),
                cubic((-ka, b), (-a, kb), (-a, 0.0)),
                cubic((-a, -kb), (-ka, -b), (0.0, -b)),
                cubic((ka, -b), (a, -kb), (a, 0.0)),
                PathCommand::Close,
            ];
            Head { cmds, filled: true }
        }
    }
}

#[cfg(test)]
#[path = "arrow_tests.rs"]
mod arrow_tests;
