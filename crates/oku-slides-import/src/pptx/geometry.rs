//! Géométrie d'une forme : prédéfinie (`prstGeom`) ou libre (`custGeom`, bornée).
//!
//! Les guides d'un tracé libre (`avLst` puis `gdLst`, chacun pouvant nommer
//! les précédents) sont évalués par [`crate::formula`] sur la taille de la
//! forme (`xfrm`, EMU) ; un point, un rayon ou un angle peut nommer un guide.
//! Un nom que rien ne définit, ou un guide illisible, vaut 0.

use std::collections::BTreeMap;

use roxmltree::Node;

use crate::collect::clip_to;
use crate::formula::{Env, Formula, Operand};
use crate::pptx::xml::{attr_bool, attr_f64, attr_str, child, children, elements};
use crate::source::{Geometry, PathCmd, SubPath};

/// Sous-tracés gardés par géométrie.
pub(crate) const MAX_PATHS: usize = 64;
/// Commandes gardées par géométrie (tous sous-tracés confondus).
pub(crate) const MAX_PATH_CMDS: usize = 10_000;
const MAX_ADJ: usize = 16;
/// Guides évalués au plus par tracé libre, `avLst` et `gdLst` confondus.
pub(crate) const MAX_GUIDES: usize = 256;
/// Unités d'angle DrawingML par degré.
const ANGLE_UNITS: f64 = 60_000.0;
/// Borne des coordonnées et rayons normalisés, en tailles de boîte.
pub(crate) const MAX_PATH_SPAN: f64 = 1000.0;

/// Géométrie déclarée dans `sp_pr` ; `None` si aucune.
pub(crate) fn parse_geometry(sp_pr: Node<'_, '_>) -> Option<Geometry> {
    if let Some(p) = child(sp_pr, "prstGeom") {
        let name = clip_to(attr_str(p, "prst").unwrap_or("rect"), 32);
        let adj: BTreeMap<String, f64> = child(p, "avLst")
            .into_iter()
            .flat_map(|l| children(l, "gd"))
            .filter_map(|gd| {
                let v: f64 = attr_str(gd, "fmla")?.strip_prefix("val ")?.trim().parse().ok()?;
                v.is_finite().then(|| (clip_to(attr_str(gd, "name").unwrap_or_default(), 16), v))
            })
            .take(MAX_ADJ)
            .collect();
        return Some(Geometry::Preset { name, adj });
    }
    let cust = child(sp_pr, "custGeom")?;
    let env = guides(cust);
    let mut budget = MAX_PATH_CMDS;
    let paths = child(cust, "pathLst")
        .into_iter()
        .flat_map(|l| children(l, "path"))
        .take(MAX_PATHS)
        .map(|p| {
            let cmds = path(p, budget, &env);
            budget -= cmds.len();
            SubPath { cmds, fill: attr_str(p, "fill") != Some("none"), stroke: attr_bool(p, "stroke").unwrap_or(true) }
        })
        .collect();
    Some(Geometry::Path { paths })
}

/// Taille de la forme d'un `custGeom` (`spPr/xfrm/ext`, EMU), sinon la boîte de son premier tracé.
fn shape_size(cust: Node<'_, '_>) -> (f64, f64) {
    let ext = cust.parent_element().and_then(|sp| child(sp, "xfrm")).and_then(|x| child(x, "ext"));
    let first = child(cust, "pathLst").and_then(|l| children(l, "path").next());
    let side = |a: &str, b: &str| {
        ext.and_then(|e| attr_f64(e, a))
            .or_else(|| first.and_then(|p| attr_f64(p, b)))
            .filter(|v| *v >= 0.0)
            .unwrap_or(1.0)
    };
    (side("cx", "w"), side("cy", "h"))
}

/// Les guides de `cust` évalués dans l'ordre, `avLst` puis `gdLst`, [`MAX_GUIDES`] au plus ;
/// un guide illisible ou non fini est omis.
fn guides<'a>(cust: Node<'a, '_>) -> Env<'a> {
    let (w, h) = shape_size(cust);
    let mut env = Env::new(w, h);
    let lists = ["avLst", "gdLst"].into_iter().filter_map(|l| child(cust, l));
    for gd in lists.flat_map(|l| children(l, "gd")).take(MAX_GUIDES) {
        let (Some(name), Some(f)) = (attr_str(gd, "name"), attr_str(gd, "fmla").and_then(Formula::parse)) else {
            continue;
        };
        if let Some(v) = f.eval(&env) {
            env.set(name, v);
        }
    }
    env
}

/// Valeur de l'attribut `a` de `n` : un nombre, ou le guide qu'il nomme ; `None` sans attribut,
/// 0 pour un nom que rien ne définit.
fn value(n: Node<'_, '_>, a: &str, env: &Env<'_>) -> Option<f64> {
    attr_str(n, a).map(|v| env.get(&Operand::parse(v.trim())).unwrap_or(0.0))
}

/// Valeur ramenée à la boîte `size`, bornée à `[lo, MAX_PATH_SPAN]` (toujours finie).
fn normalized(v: Option<f64>, size: f64, lo: f64) -> f64 {
    (v.unwrap_or(0.0) / size).clamp(lo, MAX_PATH_SPAN)
}

/// Boîte `(w, h)` d'un `path` : `w`/`h` sont des entiers EMU, une boîte de moins d'une unité
/// est dégénérée.
fn box_of(p: Node<'_, '_>) -> (f64, f64) {
    let side = |a| attr_f64(p, a).filter(|v| *v >= 1.0).unwrap_or(1.0);
    (side("w"), side("h"))
}

/// Vrai si [`normalized`] ramène une coordonnée ou un rayon du `path` `p` (guides évalués) dans sa borne.
pub(crate) fn path_clamped(p: Node<'_, '_>) -> bool {
    let (w, h) = box_of(p);
    let cust = p.parent_element().and_then(|l| l.parent_element()).filter(|c| c.tag_name().name() == "custGeom");
    let env = cust.map_or_else(|| Env::new(w, h), guides);
    let off = |n: Node<'_, '_>, a: &str, size: f64, lo: f64| {
        value(n, a, &env).is_some_and(|v| v / size != normalized(Some(v), size, lo))
    };
    p.descendants().filter(Node::is_element).any(|n| match n.tag_name().name() {
        "pt" => off(n, "x", w, -MAX_PATH_SPAN) || off(n, "y", h, -MAX_PATH_SPAN),
        "arcTo" => off(n, "wR", w, 0.0) || off(n, "hR", h, 0.0),
        _ => false,
    })
}

fn path(p: Node<'_, '_>, budget: usize, env: &Env<'_>) -> Vec<PathCmd> {
    let (w, h) = box_of(p);
    let at = |n: Node<'_, '_>, a: &str| value(n, a, env);
    let pts = |n: Node<'_, '_>| -> Vec<(f64, f64)> {
        children(n, "pt")
            .take(3)
            .map(|pt| (normalized(at(pt, "x"), w, -MAX_PATH_SPAN), normalized(at(pt, "y"), h, -MAX_PATH_SPAN)))
            .collect()
    };
    elements(p)
        .filter_map(|c| match c.tag_name().name() {
            "moveTo" => pts(c).first().map(|&(x, y)| PathCmd::MoveTo { x, y }),
            "lnTo" => pts(c).first().map(|&(x, y)| PathCmd::LineTo { x, y }),
            "cubicBezTo" => match pts(c)[..] {
                [(x1, y1), (x2, y2), (x, y)] => Some(PathCmd::CubicTo { x1, y1, x2, y2, x, y }),
                _ => None,
            },
            "quadBezTo" => match pts(c)[..] {
                [(x1, y1), (x, y)] => Some(PathCmd::QuadTo { x1, y1, x, y }),
                _ => None,
            },
            "arcTo" => Some(PathCmd::ArcTo {
                wr: normalized(at(c, "wR"), w, 0.0),
                hr: normalized(at(c, "hR"), h, 0.0),
                start: at(c, "stAng").map_or(0.0, |v| v / ANGLE_UNITS),
                swing: at(c, "swAng").map_or(0.0, |v| v / ANGLE_UNITS),
            }),
            "close" => Some(PathCmd::Close),
            _ => None,
        })
        .take(budget)
        .collect()
}

#[cfg(test)]
#[path = "geometry_tests.rs"]
mod geometry_tests;
