//! Géométrie d'une forme : prédéfinie (`prstGeom`) ou libre (`custGeom`, bornée).
//!
//! Les formules de guide (`gd fmla="*/ w 1 2"`) ne sont pas évaluées : seules
//! les constantes `val N` sont lues ; un point qui nomme un guide vaut 0 (t-727).

use std::collections::BTreeMap;

use roxmltree::Node;

use crate::collect::clip_to;
use crate::pptx::units::angle;
use crate::pptx::xml::{attr_f64, attr_i64, attr_str, child, children, elements};
use crate::source::{Geometry, PathCmd};

/// Sous-tracés gardés par géométrie.
pub(crate) const MAX_PATHS: usize = 64;
/// Commandes gardées par géométrie (tous sous-tracés confondus).
pub(crate) const MAX_PATH_CMDS: usize = 10_000;
const MAX_ADJ: usize = 16;

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
    let mut budget = MAX_PATH_CMDS;
    let paths = child(cust, "pathLst")
        .into_iter()
        .flat_map(|l| children(l, "path"))
        .take(MAX_PATHS)
        .map(|p| {
            let cmds = path(p, budget);
            budget -= cmds.len();
            cmds
        })
        .collect();
    Some(Geometry::Path { paths })
}

fn path(p: Node<'_, '_>, budget: usize) -> Vec<PathCmd> {
    let w = attr_f64(p, "w").filter(|v| *v > 0.0).unwrap_or(1.0);
    let h = attr_f64(p, "h").filter(|v| *v > 0.0).unwrap_or(1.0);
    let pts = |n: Node<'_, '_>| -> Vec<(f64, f64)> {
        children(n, "pt")
            .take(3)
            .map(|pt| (attr_f64(pt, "x").unwrap_or(0.0) / w, attr_f64(pt, "y").unwrap_or(0.0) / h))
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
                wr: attr_f64(c, "wR").unwrap_or(0.0) / w,
                hr: attr_f64(c, "hR").unwrap_or(0.0) / h,
                start: attr_i64(c, "stAng").map_or(0.0, angle),
                swing: attr_i64(c, "swAng").map_or(0.0, angle),
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
