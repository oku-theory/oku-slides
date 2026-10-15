//! Bornes des valeurs numériques lues, celles du schéma OOXML (ECMA-376, partie 1, 20.1.10 et
//! 21.1.10), et repérage des valeurs qu'elles ont ramenées.
//!
//! Le lecteur ramène silencieusement une valeur hors plage ([`attr_in`]) ; [`clamped`] repasse
//! une part pour le signaler, une fois par sorte de valeur (`Approximated`).

use roxmltree::Node;

use crate::pptx::context::PartCtx;
use crate::pptx::geometry::path_clamped;
use crate::pptx::xml::attr_i64;
use crate::source::WarningKind;

/// `ST_TextFontSize`, en centièmes de pt.
pub(crate) const FONT_SIZE: (i64, i64) = (100, 400_000);
/// `ST_TextPoint` (`spc`), en centièmes de pt.
pub(crate) const CHAR_SPACING: (i64, i64) = (-400_000, 400_000);
/// `ST_TextSpacingPoint` (`spcPts`), en centièmes de pt.
pub(crate) const SPACING_POINTS: (i64, i64) = (0, 158_400);
/// `ST_TextSpacingPercent` (`spcPct`), en 100 000ᵉ.
pub(crate) const SPACING_PERCENT: (i64, i64) = (0, 13_200_000);
/// `ST_TextBulletSizePercent` (`buSzPct`), en 100 000ᵉ.
pub(crate) const BULLET_SIZE: (i64, i64) = (25_000, 400_000);
/// `ST_TextFontScalePercent` (`fontScale`), en 100 000ᵉ.
pub(crate) const FONT_SCALE: (i64, i64) = (1_000, 100_000);
/// `lnSpcReduction` : une réduction d'au plus 100 %.
pub(crate) const LINE_REDUCTION: (i64, i64) = (0, 100_000);
/// `ST_TextMargin` (`marL`), en EMU.
pub(crate) const MARGIN: (i64, i64) = (0, 51_206_400);
/// `ST_TextIndent` (`indent`), en EMU.
pub(crate) const INDENT: (i64, i64) = (-51_206_400, 51_206_400);

/// Entier `name` de `n`, ramené dans `range`.
pub(crate) fn attr_in(n: Node<'_, '_>, name: &str, range: (i64, i64)) -> Option<i64> {
    attr_i64(n, name).map(|v| v.clamp(range.0, range.1))
}

/// `ST_PositiveFixedPercentage` (`pos` d'un arrêt de dégradé), en 100 000ᵉ.
pub(crate) const GRADIENT_POS: (i64, i64) = (0, 100_000);

/// Un attribut borné : les éléments qui le portent, son nom, sa plage, son libellé.
struct Bound {
    on: fn(&str) -> bool,
    attr: &'static str,
    range: (i64, i64),
    label: &'static str,
}

fn run(e: &str) -> bool {
    matches!(e, "rPr" | "defRPr" | "endParaRPr")
}

fn para(e: &str) -> bool {
    e == "pPr" || e == "defPPr" || (e.starts_with("lvl") && e.ends_with("pPr"))
}

/// Les attributs que le lecteur ramène dans leur plage, où il les lit.
const BOUNDS: [Bound; 10] = [
    Bound { on: run, attr: "sz", range: FONT_SIZE, label: "sz" },
    Bound { on: run, attr: "spc", range: CHAR_SPACING, label: "spc" },
    Bound { on: |e| e == "spcPts", attr: "val", range: SPACING_POINTS, label: "spcPts" },
    Bound { on: |e| e == "spcPct", attr: "val", range: SPACING_PERCENT, label: "spcPct" },
    Bound { on: |e| e == "buSzPct", attr: "val", range: BULLET_SIZE, label: "buSzPct" },
    Bound { on: |e| e == "normAutofit", attr: "fontScale", range: FONT_SCALE, label: "fontScale" },
    Bound { on: |e| e == "normAutofit", attr: "lnSpcReduction", range: LINE_REDUCTION, label: "lnSpcReduction" },
    Bound { on: para, attr: "marL", range: MARGIN, label: "marL" },
    Bound { on: para, attr: "indent", range: INDENT, label: "indent" },
    Bound { on: |e| e == "gs", attr: "pos", range: GRADIENT_POS, label: "gs pos" },
];

/// Libellés des sortes de valeurs de `root` que le lecteur a ramenées dans leur plage, chacune
/// une fois, dans l'ordre de [`BOUNDS`] puis `custGeom`.
pub(crate) fn clamped(root: Node<'_, '_>) -> Vec<&'static str> {
    let mut hit = [false; BOUNDS.len()];
    let mut geometry = false;
    for n in root.descendants().filter(Node::is_element) {
        let name = n.tag_name().name();
        for (seen, b) in hit.iter_mut().zip(&BOUNDS) {
            if !*seen && (b.on)(name) {
                *seen = attr_i64(n, b.attr).is_some_and(|v| v != v.clamp(b.range.0, b.range.1));
            }
        }
        geometry = geometry
            || (name == "path"
                && n.parent_element().is_some_and(|p| p.tag_name().name() == "pathLst")
                && path_clamped(n));
    }
    let mut out: Vec<&'static str> = BOUNDS.iter().zip(hit).filter(|(_, h)| *h).map(|(b, _)| b.label).collect();
    if geometry {
        out.push("custGeom");
    }
    out
}

/// Signale `Approximated` une fois par sorte de valeur de la part `root` ramenée dans sa plage.
pub(crate) fn warn_clamped(cx: &PartCtx<'_>, root: Node<'_, '_>) {
    for label in clamped(root) {
        cx.warn(None, WarningKind::Approximated(label.to_owned()));
    }
}

#[cfg(test)]
#[path = "bounds_tests.rs"]
mod bounds_tests;
