//! Tableaux (`a:tbl`) : grille, fusions, style de tableau, propriétés de cellule.

use roxmltree::Node;

use crate::collect::clip;
use crate::pptx::cascade::{TextChain, resolve_textbox};
use crate::pptx::color::{color_spec, color_spec_in};
use crate::pptx::context::PartCtx;
use crate::pptx::fill::{FillSpec, LineSpec, parse_fill, parse_line};
use crate::pptx::text::{LevelProps, RunProps, anchor, font_ref, parse_txbody};
use crate::pptx::units::emu;
use crate::pptx::xml::{attr_bool, attr_i64, attr_str, attr_u32, child, children, elements};
use crate::source::{Borders, Cell, Fill, Stroke, Table, TableFlags, TextBox, WarningKind};

const COLORS: [&str; 5] = ["srgbClr", "schemeClr", "sysClr", "prstClr", "scrgbClr"];

/// Une partie de style de tableau, lue (non résolue).
#[derive(Debug, Clone, Default)]
struct Part {
    fill: Option<FillSpec>,
    left: Option<LineSpec>,
    right: Option<LineSpec>,
    top: Option<LineSpec>,
    bottom: Option<LineSpec>,
    inside_h: Option<LineSpec>,
    inside_v: Option<LineSpec>,
    text: RunProps,
}

fn border(bdr: Node<'_, '_>, name: &str, cx: &PartCtx<'_>) -> Option<LineSpec> {
    let side = child(bdr, name)?;
    if let Some(ln) = child(side, "ln") {
        return Some(parse_line(ln));
    }
    let r = child(side, "lnRef")?;
    Some(cx.theme.line_style(attr_u32(r, "idx").unwrap_or(0), color_spec_in(r).as_ref()))
}

fn read_part(p: Node<'_, '_>, cx: &PartCtx<'_>) -> Part {
    let mut part = Part::default();
    if let Some(tc) = child(p, "tcStyle") {
        part.fill = child(tc, "fill").and_then(|f| parse_fill(f, &mut |r| cx.asset(r))).or_else(|| {
            let r = child(tc, "fillRef")?;
            cx.theme.fill_style(attr_u32(r, "idx").unwrap_or(0), color_spec_in(r).as_ref(), &mut |id| cx.asset(id))
        });
        if let Some(b) = child(tc, "tcBdr") {
            part.left = border(b, "left", cx);
            part.right = border(b, "right", cx);
            part.top = border(b, "top", cx);
            part.bottom = border(b, "bottom", cx);
            part.inside_h = border(b, "insideH", cx);
            part.inside_v = border(b, "insideV", cx);
        }
    }
    if let Some(tx) = child(p, "tcTxStyle") {
        part.text = child(tx, "fontRef").map(font_ref).unwrap_or_default();
        part.text.bold = attr_bool(tx, "b");
        part.text.italic = attr_bool(tx, "i");
        part.text.color = elements(tx).find(|c| COLORS.contains(&c.tag_name().name())).and_then(color_spec);
    }
    part
}

fn overlay(acc: &mut Part, top: &Part) {
    if top.fill.is_some() {
        acc.fill.clone_from(&top.fill);
    }
    for (a, t) in [
        (&mut acc.left, &top.left),
        (&mut acc.right, &top.right),
        (&mut acc.top, &top.top),
        (&mut acc.bottom, &top.bottom),
        (&mut acc.inside_h, &top.inside_h),
        (&mut acc.inside_v, &top.inside_v),
    ] {
        if t.is_some() {
            a.clone_from(t);
        }
    }
    acc.text.overlay(&top.text);
}

/// Parties du style `id` (`wholeTbl`, `band1H`…), ou `None` s'il n'existe pas.
fn style_parts(id: &str, cx: &PartCtx<'_>) -> Option<Vec<(String, Part)>> {
    let doc = cx.shared.table_styles.as_ref()?;
    let style = elements(doc.root_element())
        .find(|n| n.tag_name().name() == "tblStyle" && attr_str(*n, "styleId") == Some(id))?;
    Some(elements(style).take(16).map(|p| (p.tag_name().name().to_owned(), read_part(p, cx))).collect())
}

fn flags(pr: Option<Node<'_, '_>>) -> TableFlags {
    let f = |name: &str| pr.and_then(|p| attr_bool(p, name)).unwrap_or(false);
    TableFlags {
        first_row: f("firstRow"),
        last_row: f("lastRow"),
        first_col: f("firstCol"),
        last_col: f("lastCol"),
        band_row: f("bandRow"),
        band_col: f("bandCol"),
    }
}

/// Place d'une cellule : son ancre, ses fusions (bornées à la grille), et si elle touche le bord droit ou le bas.
#[derive(Debug, Clone, Copy)]
struct At {
    r: usize,
    c: usize,
    row_span: u32,
    col_span: u32,
    last_row: bool,
    last_col: bool,
}

impl At {
    /// Place de `tc` ancrée en (r, c) dans une grille nr × nc : les bords se lisent à la fin de la fusion.
    fn of(tc: Node<'_, '_>, (r, c): (usize, usize), (nr, nc): (usize, usize)) -> Self {
        let span = |a: &str, room: usize| {
            attr_u32(tc, a).unwrap_or(1).clamp(1, u32::try_from(room.max(1)).unwrap_or(u32::MAX))
        };
        let row_span = span("rowSpan", nr.saturating_sub(r));
        let col_span = span("gridSpan", nc.saturating_sub(c));
        let end = |at: usize, n: u32| at.saturating_add(usize::try_from(n).unwrap_or(usize::MAX));
        Self { r, c, row_span, col_span, last_row: end(r, row_span) >= nr, last_col: end(c, col_span) >= nc }
    }
}

/// Parties qui s'appliquent à la cellule, du plus faible au plus fort.
fn parts_for(fl: TableFlags, at: At) -> Vec<&'static str> {
    let (r, c) = (at.r, at.c);
    let mut out = vec!["wholeTbl"];
    if fl.band_col && !(fl.first_col && c == 0) {
        let k = c - usize::from(fl.first_col);
        out.push(if k % 2 == 0 { "band1V" } else { "band2V" });
    }
    if fl.band_row && !(fl.first_row && r == 0) {
        let k = r - usize::from(fl.first_row);
        out.push(if k % 2 == 0 { "band1H" } else { "band2H" });
    }
    if fl.first_col && c == 0 {
        out.push("firstCol");
    }
    if fl.last_col && at.last_col {
        out.push("lastCol");
    }
    if fl.first_row && r == 0 {
        out.push("firstRow");
    }
    if fl.last_row && at.last_row {
        out.push("lastRow");
    }
    out
}

/// Lit un `a:tbl`.
pub(crate) fn parse_table(tbl: Node<'_, '_>, cx: &PartCtx<'_>) -> Table {
    let pr = child(tbl, "tblPr");
    let fl = flags(pr);
    let style_id = pr.and_then(|p| child(p, "tableStyleId")).and_then(|n| n.text()).map(str::trim).unwrap_or_default();
    let parts = if style_id.is_empty() {
        Vec::new()
    } else {
        style_parts(style_id, cx).unwrap_or_else(|| {
            cx.warn(None, WarningKind::UnknownTableStyle(clip(style_id)));
            Vec::new()
        })
    };
    let col_widths: Vec<f64> = child(tbl, "tblGrid")
        .into_iter()
        .flat_map(|g| children(g, "gridCol"))
        .map(|c| emu(attr_i64(c, "w").unwrap_or(0)))
        .collect();
    let trs: Vec<Node<'_, '_>> = children(tbl, "tr").collect();
    let row_heights = trs.iter().map(|r| emu(attr_i64(*r, "h").unwrap_or(0))).collect();
    let (nr, nc) = (trs.len(), col_widths.len().max(1));
    let levels = cx.lookup(None).levels;
    let rows = trs
        .iter()
        .enumerate()
        .map(|(r, tr)| {
            children(*tr, "tc")
                .enumerate()
                .map(|(c, tc)| {
                    let at = At::of(tc, (r, c), (nr, nc));
                    let mut acc = Part::default();
                    for name in parts_for(fl, at) {
                        if let Some((_, p)) = parts.iter().find(|(n, _)| n == name) {
                            overlay(&mut acc, p);
                        }
                    }
                    cell(tc, &acc, at, cx, &levels)
                })
                .collect()
        })
        .collect();
    Table { col_widths, row_heights, flags: fl, rows }
}

fn cell(tc: Node<'_, '_>, style: &Part, at: At, cx: &PartCtx<'_>, levels: &[LevelProps]) -> Cell {
    let tc_pr = child(tc, "tcPr");
    let covered = attr_bool(tc, "hMerge") == Some(true) || attr_bool(tc, "vMerge") == Some(true);
    let fill = tc_pr
        .and_then(|p| parse_fill(p, &mut |id| cx.asset(id)))
        .or_else(|| style.fill.clone())
        .map_or(Fill::None, |f| f.resolve(&cx.colors));
    let pick = |edge: Option<&LineSpec>, own: &str| -> Option<Stroke> {
        let own = tc_pr.and_then(|p| child(p, own)).map(parse_line);
        own.or_else(|| edge.cloned()).and_then(|l| l.finish(&cx.colors))
    };
    let borders = Borders {
        l: pick(if at.c == 0 { style.left.as_ref() } else { style.inside_v.as_ref() }, "lnL"),
        r: pick(if at.last_col { style.right.as_ref() } else { style.inside_v.as_ref() }, "lnR"),
        t: pick(if at.r == 0 { style.top.as_ref() } else { style.inside_h.as_ref() }, "lnT"),
        b: pick(if at.last_row { style.bottom.as_ref() } else { style.inside_h.as_ref() }, "lnB"),
    };
    let text = if covered {
        TextBox::default()
    } else {
        let mut tb = child(tc, "txBody").map_or_else(TextBox::default, |tx| {
            let raw = parse_txbody(tx, &mut |id| cx.link(id));
            let chain = TextChain { levels: levels.iter().collect(), bodies: Vec::new() };
            resolve_textbox(&raw, &chain, &cx.env(style.text.clone()))
        });
        if let Some(p) = tc_pr {
            let m = |a: &str, d: f64| attr_i64(p, a).map_or(d, emu);
            tb.insets.l = m("marL", 7.2);
            tb.insets.r = m("marR", 7.2);
            tb.insets.t = m("marT", 3.6);
            tb.insets.b = m("marB", 3.6);
            if let Some(a) = attr_str(p, "anchor") {
                tb.anchor = anchor(a);
            }
        }
        tb
    };
    Cell { text, fill, borders, row_span: at.row_span, col_span: at.col_span, covered }
}

#[cfg(test)]
#[path = "table_tests.rs"]
mod table_tests;
