//! A source table's style in the terms of the base `table` widget: flags of
//! its option (`header-row`, `striped`) and tokens of the theme's
//! `component.base.table` component.
//!
//! The reader's cells are resolved (table style applied, or explicit fills as
//! LibreOffice writes them, empty `tblPr`): the style is read on them, the
//! `tblPr` flags are only hints.
//!
//! | Source | Widget |
//! |---|---|
//! | 1st row of a plain fill other than the 2nd's (or `firstRow`) | `header-row`; `header-bg`, `header-color`, `header-weight` (the row's dominant text) |
//! | body rows of one fill | `body-bg` (`null`: no fill) |
//! | body rows alternating two fills | `striped`; `body-bg` (1st, 3rd…), `stripe-bg` (2nd, 4th…) |
//! | text size of the cells | `size` (page pt) |
//! | common bottom border of the cells | `rule-color`, `rule-width` (0 without a border) |
//! | common cell margins (`marL` = `marR`, `marT` = `marB`) | `padding-x`, `padding-y` (page pt) |
//! | column widths, row heights (`a:gridCol`, `a:tr h`) | option `column-widths`, `row-heights` ([`set_geometry`]) |
//!
//! The generated theme sets the tokens of the deck's first table
//! ([`first_table`]): they hold for all. Irregular ([`TableLook::irregular`])
//! is what the widget does not draw: per-cell fills, gradients, side or
//! differing borders, differing cell margins, several text sizes, a body
//! text in another color than the ink. A table agrees with the deck's theme
//! on the tokens it uses only: without a header, the first table's header
//! tokens do not concern it.

use serde_json::{Map, Value, json};

use super::color::ThemeColors;
use super::element_text::dominant;
use super::page::{Page, round2};
use crate::source::{Cell, Dash, Fill, Item, ItemKind, Run, SourceDeck, Stroke, Table};

/// Largest cell padding token, in page pt: `max` of `table.padding-x/y` in `packs/base/widgets.json`.
///
/// A test reads the JSON, so the two cannot drift apart.
const MAX_PADDING: f64 = 400.0;

/// Le style d'un tableau source.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct TableLook {
    /// La première ligne est un en-tête.
    pub(crate) header_row: bool,
    /// Les lignes du corps alternent deux fonds.
    pub(crate) striped: bool,
    /// Les jetons `component.base.table` qui le rendent.
    pub(crate) tokens: Map<String, Value>,
    /// Une part du style n'a pas d'équivalent dans le widget.
    pub(crate) irregular: bool,
}

impl TableLook {
    /// Le style de `t`, couleurs écrites par `colors`, tailles sur `page`.
    pub(crate) fn of(t: &Table, colors: &ThemeColors, page: &Page) -> Self {
        let mut look = Self::default();
        let rows: Vec<Vec<&Cell>> = t.rows.iter().map(|r| r.iter().filter(|c| !c.covered).collect()).collect();
        let fills: Vec<Option<&Fill>> = rows.iter().map(|r| uniform(r.iter().map(|c| &c.fill))).collect();
        let distinct = rows.len() >= 2 && fills.first() != fills.get(1);
        look.header_row = t.flags.first_row || (distinct && matches!(fills.first(), Some(Some(Fill::Solid { .. }))));
        let first = usize::from(look.header_row);
        if look.header_row {
            let header = rows.first().map(Vec::as_slice).unwrap_or_default();
            look.fill("header-bg", fills.first().copied().flatten(), colors, false);
            // Le widget a une couleur, une graisse et une police d'en-tête, sans italique.
            let head: Vec<&Run> = header.iter().flat_map(|c| runs(c)).collect();
            look.irregular |= !is_uniform(head.iter().map(|r| &r.style.color.value))
                || !is_uniform(head.iter().map(|r| r.style.bold))
                || !is_uniform(head.iter().map(|r| &r.style.font.value))
                || head.iter().any(|r| r.style.italic);
            let colors_of = header.iter().flat_map(|c| runs(c)).map(|r| (colors.write(&r.style.color), weight(r)));
            look.set("header-color", dominant(colors_of).map(Value::String));
            let bold = dominant(header.iter().flat_map(|c| runs(c)).map(|r| (r.style.bold, weight(r))));
            look.set("header-weight", bold.map(|b| json!(if b { "strong" } else { "body" })));
        }

        let body = &fills[first.min(fills.len())..];
        let even = uniform(body.iter().step_by(2).copied());
        let odd = uniform(body.iter().skip(1).step_by(2).copied());
        match (even, odd) {
            (None, None) => {}
            (Some(e), None) if body.len() == 1 => look.fill("body-bg", e, colors, true),
            (Some(e), Some(o)) if e == o => look.fill("body-bg", e, colors, true),
            (Some(e), Some(o)) => {
                look.striped = true;
                look.fill("body-bg", e, colors, true);
                look.fill("stripe-bg", o, colors, false);
            }
            _ => look.irregular = true,
        }

        let cells = || rows.iter().flatten();
        let sizes: Vec<(u64, usize)> =
            cells().flat_map(|c| runs(c)).map(|r| (r.style.size.to_bits(), weight(r))).collect();
        look.irregular |= sizes.iter().any(|(s, _)| Some(*s) != sizes.first().map(|(f, _)| *f));
        let size = dominant(sizes).map(f64::from_bits).filter(|s| s.is_finite() && *s > 0.0);
        look.set("size", size.map(|s| json!(round2(page.len(s)))));
        // Le corps s'écrit dans l'encre du thème.
        let body_cells = cells().skip(if look.header_row { rows.first().map_or(0, Vec::len) } else { 0 });
        let body_runs: Vec<&Run> = body_cells.flat_map(|c| runs(c)).collect();
        look.irregular |= body_runs.iter().any(|r| Some(r.style.color.value) != colors.get("ink"));
        // Le corps du widget est maigre, droit et d'une seule police.
        look.irregular |= body_runs.iter().any(|r| r.style.bold || r.style.italic)
            || !is_uniform(body_runs.iter().map(|r| &r.style.font.value));

        let bottom = uniform(cells().map(|c| c.borders.b.as_ref()));
        match bottom {
            Some(Some(s)) if s.dash == Dash::Solid => {
                look.set("rule-color", Some(Value::String(colors.write(&s.color))));
                look.set("rule-width", Some(json!(round2(page.len(s.width)))));
            }
            Some(None) => look.set("rule-width", Some(json!(0))),
            _ => look.irregular = true,
        }
        let rule = bottom.flatten();
        look.irregular |= cells().any(|c| c.borders.l.is_some() || c.borders.r.is_some());
        look.irregular |= rows.first().is_some_and(|r| r.iter().any(|c| c.borders.t.is_some()));
        look.irregular |= rows.iter().skip(1).flatten().any(|c| !same_rule(c.borders.t.as_ref(), rule));

        match uniform(cells().map(|c| c.text.insets)) {
            Some(i) if (i.l - i.r).abs() < f64::EPSILON && (i.t - i.b).abs() < f64::EPSILON => {
                let pad = |v: f64| json!(round2(page.len(v).clamp(0.0, MAX_PADDING)));
                look.set("padding-x", Some(pad(i.l)));
                look.set("padding-y", Some(pad(i.t)));
            }
            None if cells().next().is_none() => {}
            _ => look.irregular = true,
        }
        look
    }

    /// Le widget rend-il `self` tel quel, sous les jetons du thème `deck` ?
    /// Seuls comptent les jetons que `self` emploie : ce qu'il n'a pas (un
    /// en-tête, des bandes) n'a rien à appliquer.
    pub(crate) fn matches(&self, deck: Option<&Self>) -> bool {
        !self.irregular && deck.is_none_or(|d| self.tokens.iter().all(|(k, v)| d.tokens.get(k) == Some(v)))
    }

    /// Pose les drapeaux `header-row` et `striped` dans l'option `option` du widget.
    pub(crate) fn set_flags(&self, option: &mut Value) {
        if let Some(o) = option.as_object_mut() {
            o.insert("header-row".to_owned(), Value::Bool(self.header_row));
            o.insert("striped".to_owned(), Value::Bool(self.striped));
        }
    }

    /// Le jeton de couleur `key` du fond `f` d'une ligne (`null` : sans
    /// fond) ; une ligne sans fond commun (`None`) ou d'un fond que le widget
    /// ne peint pas (dégradé, image) est irrégulière.
    ///
    /// Seul `body-bg` accepte `null` ; une clé non nullable sans fond prend
    /// celui de la page (`bg`).
    fn fill(&mut self, key: &str, f: Option<&Fill>, colors: &ThemeColors, nullable: bool) {
        match f {
            Some(Fill::None) if nullable => self.set(key, Some(Value::Null)),
            Some(Fill::None) => self.set(key, Some(json!("bg"))),
            Some(Fill::Solid { color }) => self.set(key, Some(Value::String(colors.write(color)))),
            _ => self.irregular = true,
        }
    }

    fn set(&mut self, key: &str, value: Option<Value>) {
        if let Some(v) = value {
            self.tokens.insert(key.to_owned(), v);
        }
    }
}

/// Le premier tableau du deck, dans l'ordre des slides et de dessin (groupes compris).
pub(crate) fn first_table(deck: &SourceDeck) -> Option<&Table> {
    fn find(items: &[Item]) -> Option<&Table> {
        items.iter().find_map(|i| match &i.kind {
            ItemKind::Table(t) => Some(t),
            ItemKind::Group { items, .. } => find(items),
            _ => None,
        })
    }
    deck.slides.iter().find_map(|s| find(&s.items))
}

/// Writes the column widths and row heights of `t` as weights in the widget option `option`.
///
/// One weight per column and row the option keeps (at most `MAX_TABLE_COLUMNS`
/// and `MAX_TABLE_ROWS`), and only when they are all finite and positive:
/// otherwise the widget sizes that dimension on the content.
pub(crate) fn set_geometry(option: &mut Value, t: &Table) {
    let source = option.pointer("/dataset/source").and_then(Value::as_array);
    let rows = source.map_or(0, Vec::len);
    let width = source.and_then(|s| s.first()).and_then(Value::as_array).map_or(0, Vec::len);
    let weights = |all: &[f64], count: usize| {
        all.get(..count).filter(|w| !w.is_empty() && oku_slides_chart::valid_weights(w, count)).map(|w| json!(w))
    };
    let (columns, heights) = (weights(&t.col_widths, width), weights(&t.row_heights, rows));
    if let Some(o) = option.as_object_mut() {
        if let Some(w) = columns {
            o.insert("column-widths".to_owned(), w);
        }
        if let Some(h) = heights {
            o.insert("row-heights".to_owned(), h);
        }
    }
}

/// La valeur commune de `values`, `None` si elles diffèrent ou s'il n'y en a pas.
fn uniform<T: PartialEq>(mut values: impl Iterator<Item = T>) -> Option<T> {
    let first = values.next()?;
    values.all(|v| v == first).then_some(first)
}

/// Les valeurs sont-elles toutes égales (vrai sans valeur) ?
fn is_uniform<T: PartialEq>(mut values: impl Iterator<Item = T>) -> bool {
    values.next().is_none_or(|first| values.all(|v| v == first))
}

/// Les runs non vides d'une cellule.
fn runs(c: &Cell) -> impl Iterator<Item = &Run> {
    c.text.paragraphs.iter().flat_map(|p| &p.runs).filter(|r| !r.text.is_empty())
}

/// Poids d'un run : son nombre de caractères.
fn weight(r: &Run) -> usize {
    r.text.chars().count()
}

/// La bordure haute `top` d'une cellule sous une autre est-elle le filet `rule` (ou rien) ?
fn same_rule(top: Option<&Stroke>, rule: Option<&Stroke>) -> bool {
    top.is_none() || top == rule
}

#[cfg(test)]
#[path = "table_tests.rs"]
mod table_tests;
