//! Géométrie d'une forme en chemin du cœur, normalisé au cadre (0..1).
//!
//! Un preset (`prstGeom`) est évalué sur ses définitions ECMA-376
//! (`presets.rs`) : ajustements de la forme à la place des défauts, guides
//! (`crate::formula`), puis sous-chemins tracés (`pen.rs`, arcs en cubiques). Un
//! tracé libre (`custGeom`) passe par le même crayon. Les formules dépendent
//! du rapport largeur/hauteur : la taille réelle de la forme (pt) est requise.
//!
//! Nombre de commandes borné : un preset est fini, un tracé libre en a au
//! plus `MAX_PATH_CMDS` (lecteur), chaque arc en devient au plus quatre.

pub(crate) mod arrow;
mod pen;
mod presets;

use std::collections::BTreeMap;

use oku_slides_core::PathCommand;

use self::pen::Pen;
use self::presets::{Command, Preset};
use crate::formula::{ANGLE, Env, Operand};
use crate::source::{Geometry, PathCmd, SubPath};

/// Contour d'une forme, coordonnées relatives au cadre (certains presets
/// débordent : bulles, flèches courbes).
///
/// Le plus souvent `filled == stroked` : un seul élément `path` rempli et
/// tracé. Sinon (traits seuls des légendes, `fill="none"` ; faces ombrées
/// sans contour, `stroke="false"`), le projecteur dessine les deux.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Outline {
    /// Sous-chemins remplis, à la suite.
    pub(crate) filled: Vec<PathCommand>,
    /// Sous-chemins tracés, à la suite.
    pub(crate) stroked: Vec<PathCommand>,
    /// Zone de texte du preset (gauche, haut, droite, bas) ; le cadre entier
    /// pour un tracé libre.
    pub(crate) text_rect: [f64; 4],
}

/// Contour de `geometry` pour une forme de `w` × `h` pt ; `None` pour un
/// preset inconnu, une taille non finie ou négative, ou un résultat non fini :
/// l'appelant dessine le cadre et perd la géométrie
/// ([`crate::report::ShapeEffect::Geometry`]). Des définitions embarquées
/// illisibles (un bogue, gardé par un test) arrêtent un build de debug et
/// donnent la même perte sur chaque preset en release.
pub(crate) fn outline(geometry: &Geometry, w: f64, h: f64) -> Option<Outline> {
    if !(w.is_finite() && h.is_finite() && w >= 0.0 && h >= 0.0) {
        return None;
    }
    let outline = match geometry {
        Geometry::Preset { name, adj } => {
            let preset = presets::preset(name).unwrap_or_else(|e| {
                debug_assert!(false, "presetShapeDefinitions.xml illisible : {e}");
                None
            });
            preset_outline(preset?, adj, w, h)?
        }
        Geometry::Path { paths } => custom_outline(paths, w, h),
    };
    let finite = outline.filled.iter().chain(&outline.stroked).all(|c| numbers(*c).iter().all(|v| v.is_finite()));
    (finite && outline.text_rect.iter().all(|v| v.is_finite())).then_some(outline)
}

/// `1 / side`, ou 0 pour un côté nul (tout le contour y est sur le bord).
fn inverse(side: f64) -> f64 {
    if side > 0.0 { 1.0 / side } else { 0.0 }
}

fn preset_outline(p: &'static Preset, adj: &BTreeMap<String, f64>, w: f64, h: f64) -> Option<Outline> {
    let mut env = Env::new(w, h);
    for (name, f) in &p.adjust {
        let v = match adj.get(name) {
            Some(v) => *v,
            None => f.eval(&env)?,
        };
        env.set(name, v);
    }
    for (name, f) in &p.guides {
        let v = f.eval(&env)?;
        env.set(name, v);
    }
    let mut outline = Outline { filled: Vec::new(), stroked: Vec::new(), text_rect: [0.0, 0.0, 1.0, 1.0] };
    if let Some(r) = &p.text_rect {
        let [l, t, r, b] = r.each_ref().map(|o| env.get(o));
        outline.text_rect = [l? * inverse(w), t? * inverse(h), r? * inverse(w), b? * inverse(h)];
    }
    for path in &p.paths {
        // Un chemin à espace propre (`w`, `h`) est étiré au cadre.
        let (sx, sy) = path.size.map_or((inverse(w), inverse(h)), |(pw, ph)| (1.0 / pw, 1.0 / ph));
        let mut pen = Pen::new(sx, sy);
        let get = |o: &Operand| env.get(o);
        for c in &path.commands {
            match c {
                Command::Move([x, y]) => pen.move_to((get(x)?, get(y)?)),
                Command::Line([x, y]) => pen.line_to((get(x)?, get(y)?)),
                Command::Cubic([x1, y1, x2, y2, x, y]) => {
                    pen.cubic_to((get(x1)?, get(y1)?), (get(x2)?, get(y2)?), (get(x)?, get(y)?));
                }
                Command::Quad([x1, y1, x, y]) => pen.quad_to((get(x1)?, get(y1)?), (get(x)?, get(y)?)),
                Command::Arc([wr, hr, st, sw]) => pen.arc_to(get(wr)?, get(hr)?, get(st)? * ANGLE, get(sw)? * ANGLE),
                Command::Close => pen.close(),
            }
        }
        push(&mut outline, pen.finish(), path.fill, path.stroke);
    }
    Some(outline)
}

/// Les fractions du lecteur repassent en pt le temps du tracé : les angles
/// d'un arc sont visuels, ils dépendent du rapport des côtés.
fn custom_outline(paths: &[SubPath], w: f64, h: f64) -> Outline {
    let (w, h) = (if w > 0.0 { w } else { 1.0 }, if h > 0.0 { h } else { 1.0 });
    let mut outline = Outline { filled: Vec::new(), stroked: Vec::new(), text_rect: [0.0, 0.0, 1.0, 1.0] };
    for sub in paths {
        let mut pen = Pen::new(1.0 / w, 1.0 / h);
        for c in &sub.cmds {
            match *c {
                PathCmd::MoveTo { x, y } => pen.move_to((x * w, y * h)),
                PathCmd::LineTo { x, y } => pen.line_to((x * w, y * h)),
                PathCmd::CubicTo { x1, y1, x2, y2, x, y } => {
                    pen.cubic_to((x1 * w, y1 * h), (x2 * w, y2 * h), (x * w, y * h))
                }
                PathCmd::QuadTo { x1, y1, x, y } => pen.quad_to((x1 * w, y1 * h), (x * w, y * h)),
                PathCmd::ArcTo { wr, hr, start, swing } => {
                    pen.arc_to(wr * w, hr * h, start.to_radians(), swing.to_radians())
                }
                PathCmd::Close => pen.close(),
            }
        }
        push(&mut outline, pen.finish(), sub.fill, sub.stroke);
    }
    outline
}

fn push(outline: &mut Outline, commands: Vec<PathCommand>, fill: bool, stroke: bool) {
    if fill {
        outline.filled.extend_from_slice(&commands);
    }
    if stroke {
        outline.stroked.extend(commands);
    }
}

fn numbers(c: PathCommand) -> Vec<f64> {
    match c {
        PathCommand::Move(x, y) | PathCommand::Line(x, y) => vec![x, y],
        PathCommand::Cubic(a, b, c, d, e, f) => vec![a, b, c, d, e, f],
        PathCommand::Quad(a, b, c, d) => vec![a, b, c, d],
        PathCommand::Close => Vec::new(),
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
