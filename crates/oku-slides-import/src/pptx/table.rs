//! Tableaux (`a:tbl`) : grille, fusions, style de tableau (du fichier ou
//! intégré, [`builtin`]) et son fond (`tblBg`), propriétés de cellule.

use roxmltree::Node;

use crate::collect::clip;
use crate::pptx::cascade::resolve_textbox;
use crate::pptx::color::{color_spec, color_spec_in};
use crate::pptx::context::PartCtx;
use crate::pptx::fill::{FillSpec, LineSpec, parse_fill, parse_line};
use crate::pptx::placeholder::Inherited;
use crate::pptx::table_styles::builtin;
use crate::pptx::text::{RunProps, anchor, font_ref, parse_txbody};
use crate::pptx::units::emu;
use crate::pptx::xml::{attr_bool, attr_i64, attr_str, attr_u32, child, children, elements, parse};
use crate::source::{Borders, Cell, Fill, Rgba, Stroke, Styled, Table, TableFlags, TextBox, WarningKind};
use oku_slides_core::num::byte;

const COLORS: [&str; 5] = ["srgbClr", "schemeClr", "sysClr", "prstClr", "scrgbClr"];

/// Une partie de style de tableau, lue (non résolue) ; posée sur une cellule
/// ([`overlay`]), `left`…`bottom` sont les bords de la cellule.
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

/// Le remplissage d'un `tcStyle` ou d'un `tblBg` : `fill`, sinon le style `fillRef` du thème.
fn style_fill(n: Node<'_, '_>, cx: &PartCtx<'_>) -> Option<FillSpec> {
    child(n, "fill").and_then(|f| parse_fill(f, &mut |r| cx.asset(r))).or_else(|| {
        let r = child(n, "fillRef")?;
        cx.theme.fill_style(attr_u32(r, "idx").unwrap_or(0), color_spec_in(r).as_ref(), &mut |id| cx.asset(id))
    })
}

fn read_part(p: Node<'_, '_>, cx: &PartCtx<'_>) -> Part {
    let mut part = Part::default();
    if p.tag_name().name() == "tblBg" {
        part.fill = style_fill(p, cx);
        return part;
    }
    if let Some(tc) = child(p, "tcStyle") {
        part.fill = style_fill(tc, cx);
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

/// Les bords (gauche, droite, haut, bas) de la cellule `at` que donne la
/// partie `name` : ses bords extérieurs aux limites de sa zone (le tableau ;
/// sa ligne pour une partie de ligne, sa colonne pour une partie de colonne),
/// ses bords intérieurs ailleurs.
fn cell_sides<'p>(name: &str, p: &'p Part, at: At) -> [&'p Option<LineSpec>; 4] {
    let row_part = matches!(name, "firstRow" | "lastRow" | "band1H" | "band2H");
    let col_part = matches!(name, "firstCol" | "lastCol" | "band1V" | "band2V");
    let edge = |outer: bool, side: &'p Option<LineSpec>, inner: &'p Option<LineSpec>| if outer { side } else { inner };
    [
        edge(col_part || at.c == 0, &p.left, &p.inside_v),
        edge(col_part || at.last_col, &p.right, &p.inside_v),
        edge(row_part || at.r == 0, &p.top, &p.inside_h),
        edge(row_part || at.last_row, &p.bottom, &p.inside_h),
    ]
}

/// Pose la partie `name` (`top`) sur le style `acc` de la cellule `at` : ses
/// bords deviennent ceux de la cellule (`left`, `right`, `top`, `bottom`).
fn overlay(acc: &mut Part, name: &str, top: &Part, at: At) {
    if top.fill.is_some() {
        acc.fill.clone_from(&top.fill);
    }
    let sides = cell_sides(name, top, at);
    for (a, t) in [&mut acc.left, &mut acc.right, &mut acc.top, &mut acc.bottom].into_iter().zip(sides) {
        if t.is_some() {
            a.clone_from(t);
        }
    }
    acc.text.overlay(&top.text);
}

/// Parties d'un `tblStyle` (`tblBg`, `wholeTbl`, `band1H`…).
fn read_style(style: Node<'_, '_>, cx: &PartCtx<'_>) -> Vec<(String, Part)> {
    elements(style).take(16).map(|p| (p.tag_name().name().to_owned(), read_part(p, cx))).collect()
}

/// Parties du style `id` : celui de `tableStyles.xml`, sinon le style intégré
/// de PowerPoint de ce GUID ([`builtin`]) ; `None` s'il n'existe pas.
fn style_parts(id: &str, cx: &PartCtx<'_>) -> Option<Vec<(String, Part)>> {
    let own = cx.shared.table_styles.as_ref().and_then(|doc| {
        elements(doc.root_element()).find(|n| n.tag_name().name() == "tblStyle" && attr_str(*n, "styleId") == Some(id))
    });
    if let Some(style) = own {
        return Some(read_style(style, cx));
    }
    let xml = builtin(id)?;
    match parse(&xml, "builtin table style", cx.nodes()) {
        Ok(doc) => Some(read_style(doc.root_element(), cx)),
        Err(e) => {
            debug_assert!(false, "style intégré illisible, un bogue de sa définition : {e:?}");
            None
        }
    }
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
    let inh = cx.lookup(None);
    let bg = parts.iter().find(|(n, _)| n == "tblBg").and_then(|(_, p)| p.fill.as_ref()).map(|f| f.resolve(&cx.colors));
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
                            overlay(&mut acc, name, p, at);
                        }
                    }
                    cell(tc, &acc, at, cx, (&inh, bg.as_ref()))
                })
                .collect()
        })
        .collect();
    Table { col_widths, row_heights, flags: fl, rows }
}

/// Le remplissage `fill` d'une cellule posé sur le fond du tableau `bg`
/// (`tblBg`) : sans remplissage, la cellule montre le fond ; translucide, elle
/// s'y mêle.
fn on_background(fill: Fill, bg: Option<&Fill>) -> Fill {
    let Some(Fill::Solid { color: under }) = bg else { return fill };
    match fill {
        Fill::None => Fill::Solid { color: under.clone() },
        Fill::Solid { color } if color.value.a < 255 => {
            Fill::Solid { color: Styled::explicit(over(color.value, under.value)) }
        }
        f => f,
    }
}

/// `top` posée sur `bottom` (composition alpha, sRGB).
fn over(top: Rgba, bottom: Rgba) -> Rgba {
    let (ta, ba) = (f64::from(top.a) / 255.0, f64::from(bottom.a) / 255.0);
    let a = ta + ba * (1.0 - ta);
    let mix = |t: u8, b: u8| {
        let v = if a > 0.0 { (f64::from(t) * ta + f64::from(b) * ba * (1.0 - ta)) / a } else { 0.0 };
        byte(v)
    };
    Rgba { r: mix(top.r, bottom.r), g: mix(top.g, bottom.g), b: mix(top.b, bottom.b), a: byte(a * 255.0) }
}

/// La cellule `tc` au style `style`, avec les niveaux de texte et le fond du tableau de `(levels, bg)`.
fn cell(tc: Node<'_, '_>, style: &Part, at: At, cx: &PartCtx<'_>, (inh, bg): (&Inherited, Option<&Fill>)) -> Cell {
    let tc_pr = child(tc, "tcPr");
    let covered = attr_bool(tc, "hMerge") == Some(true) || attr_bool(tc, "vMerge") == Some(true);
    let fill = tc_pr
        .and_then(|p| parse_fill(p, &mut |id| cx.asset(id)))
        .or_else(|| style.fill.clone())
        .map_or(Fill::None, |f| f.resolve(&cx.colors));
    let fill = on_background(fill, bg);
    let pick = |edge: Option<&LineSpec>, own: &str| -> Option<Stroke> {
        let own = tc_pr.and_then(|p| child(p, own)).map(parse_line);
        own.or_else(|| edge.cloned()).and_then(|l| l.finish(&cx.colors))
    };
    let borders = Borders {
        l: pick(style.left.as_ref(), "lnL"),
        r: pick(style.right.as_ref(), "lnR"),
        t: pick(style.top.as_ref(), "lnT"),
        b: pick(style.bottom.as_ref(), "lnB"),
    };
    let text = if covered {
        TextBox::default()
    } else {
        let mut tb = child(tc, "txBody").map_or_else(TextBox::default, |tx| {
            let raw = parse_txbody(tx, &mut |id| cx.link(id));
            resolve_textbox(&raw, &inh.chain(), &cx.env(style.text.clone()))
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
