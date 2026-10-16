//! Le style d'un tableau source dans les termes du widget `table` de base :
//! drapeaux de son option (`header-row`, `striped`) et jetons du composant
//! `component.table` du thème.
//!
//! Les cellules du lecteur sont résolues (style de tableau appliqué, ou
//! remplissages explicites comme LibreOffice les écrit, `tblPr` vide) : le
//! style se lit sur elles, les drapeaux `tblPr` ne sont que des indices.
//!
//! | Source | Widget |
//! |---|---|
//! | 1re ligne d'un fond uni autre que la 2e (ou `firstRow`) | `header-row` ; `header-bg`, `header-color`, `header-weight` (texte dominant de la ligne) |
//! | lignes du corps d'un même fond | `body-bg` (`null` : sans fond) |
//! | lignes du corps alternant deux fonds | `striped` ; `body-bg` (1re, 3e…), `stripe-bg` (2e, 4e…) |
//! | taille du texte des cellules | `size` (pt page) |
//! | bordure basse commune des cellules | `rule-color`, `rule-width` (0 sans bordure) |
//!
//! Le thème généré pose les jetons du premier tableau du deck
//! ([`first_table`]) : ils valent pour tous. Est irrégulier
//! ([`TableLook::irregular`]) ce que le widget ne rend pas : fonds par
//! cellule, dégradés, bordures latérales ou différentes, largeurs de
//! colonnes inégales (le widget les calcule sur le contenu), des lignes plus
//! hautes que leur texte (`a:tr h`, une hauteur minimale : le widget donne à
//! chaque ligne celle de son contenu, [`stretched`]), plusieurs tailles de
//! texte, un texte du corps d'une autre couleur que l'encre. Un tableau
//! s'accorde au thème du deck sur les seuls jetons qu'il emploie : sans
//! en-tête, ceux de l'en-tête du premier tableau ne le concernent pas.

use serde_json::{Map, Value, json};

use super::color::ThemeColors;
use super::element_text::dominant;
use super::page::{Page, round2};
use super::typst::DEFAULT_SIZE;
use crate::source::{Cell, Dash, Fill, Item, ItemKind, Run, SourceDeck, Stroke, Table};

/// Écart relatif au-delà duquel deux largeurs de colonnes sont inégales.
const WIDTH_TOLERANCE: f64 = 0.1;
/// Écart relatif au-delà duquel une ligne est plus haute que son texte
/// (estimé : la largeur réelle des caractères n'est pas connue ici).
const HEIGHT_TOLERANCE: f64 = 0.25;
/// Avance moyenne d'un caractère, en em : de quoi estimer les lignes d'une cellule.
const CHAR_ADVANCE: f64 = 0.5;
/// Ligne simple de PowerPoint, en fraction de la taille de police.
const LINE: f64 = 1.2;

/// Le style d'un tableau source.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct TableLook {
    /// La première ligne est un en-tête.
    pub(crate) header_row: bool,
    /// Les lignes du corps alternent deux fonds.
    pub(crate) striped: bool,
    /// Les jetons `component.table` qui le rendent.
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

        let (min, max) = t.col_widths.iter().fold((f64::INFINITY, 0.0f64), |(lo, hi), w| (lo.min(*w), hi.max(*w)));
        look.irregular |= t.col_widths.len() > 1 && max > min * (1.0 + WIDTH_TOLERANCE);
        look.irregular |= stretched(t);
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

/// Une ligne de `t` est-elle nettement plus haute que le texte de ses
/// cellules (une cellule fusionnée sur plusieurs lignes n'est pas comptée) ?
fn stretched(t: &Table) -> bool {
    t.rows.iter().zip(&t.row_heights).any(|(row, h)| {
        // Une cellule par colonne de la grille, couvertes comprises.
        let need =
            row.iter().enumerate().filter(|(_, c)| !c.covered && c.row_span <= 1).fold(0.0f64, |need, (x, c)| {
                let span = usize::try_from(c.col_span.max(1)).unwrap_or(1);
                need.max(text_height(c, t.col_widths.iter().skip(x).take(span).sum()))
            });
        need > 0.0 && *h > need * (1.0 + HEIGHT_TOLERANCE)
    })
}

/// Hauteur estimée du texte de la cellule `c` large de `width` pt, marges
/// comprises : chaque paragraphe sur autant de lignes simples que ses
/// caractères en demandent.
fn text_height(c: &Cell, width: f64) -> f64 {
    let tb = &c.text;
    let room = (width - tb.insets.l - tb.insets.r).max(1.0);
    let lines: f64 = tb
        .paragraphs
        .iter()
        .map(|p| {
            let size = p.runs.iter().map(|r| r.style.size).fold(0.0, f64::max);
            let size = if size > 0.0 { size } else { DEFAULT_SIZE };
            let chars: f64 = p.runs.iter().flat_map(|r| r.text.chars()).map(|_| 1.0).sum();
            (chars * CHAR_ADVANCE * size / room).ceil().max(1.0) * LINE * size
        })
        .sum();
    lines + tb.insets.t + tb.insets.b
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
