//! Hauteur estimée d'un contenu en flux, sans rendu (t-816).
//!
//! Le projecteur est pur : il ne mesure pas le rendu Typst, il l'estime à
//! partir des jetons du thème cible ([`TextTokens`]) et du modèle de ligne de
//! Typst. Une ligne de texte occupe la hauteur de capitale de sa taille
//! ([`CAP`] em : Typst borne une ligne du haut des capitales à la ligne de
//! base), deux lignes d'un paragraphe sont séparées par l'interligne
//! (`leading`), deux paragraphes par `space.md` (`list.spacing` entre deux
//! items de liste). Les mots passent à la ligne quand la largeur de la boîte
//! est remplie, à [`GLYPH`] em par caractère : une moyenne prudente des
//! polices latines du rendu.
//!
//! | élément | hauteur |
//! |---|---|
//! | `text` | ses paragraphes à la largeur de la boîte |
//! | `card` | marges `padding-y`, surtitre, titre (`card.title-size`) et corps à la largeur moins `padding-x`, séparés par `card.gap` |
//! | `stack` | `columns` ou `grid` : la plus haute colonne par rangée ; `rows` : la somme ; gouttière `space.lg` |
//! | `table` | une ligne de `table.size` et ses marges `table.padding-y` par ligne de données |
//! | `chart` | la largeur × [`CHART_ASPECT`] (le rendu en flux) |
//! | `image` | la largeur × le rapport hauteur / largeur de sa source ([`IMAGE_ASPECT`] sans source) |
//!
//! Un média (graphique ou image) posé seul comme bloc de la mise en page
//! `default` partage la hauteur qui reste sous les autres blocs : il compte
//! au plus sa hauteur naturelle, au moins [`MIN_MEDIA`]. Ailleurs, sa hauteur
//! naturelle est bornée à la place disponible, comme au rendu.

use std::collections::BTreeMap;

use oku_slides_core::kinds::base;
use oku_slides_core::{Element, Paragraph, ParamValue, Run, TokenValue};
use serde_json::{Map, Value};

use super::zone::{number_or_default, token_or_default};

/// Hauteur d'une ligne de texte, en em : la hauteur de capitale.
pub(crate) const CAP: f64 = 0.72;
/// Largeur moyenne d'un caractère, en em.
pub(crate) const GLYPH: f64 = 0.48;
/// Rapport hauteur / largeur d'un graphique en flux (le rendu le pose en 800 × 450).
pub(crate) const CHART_ASPECT: f64 = 0.5625;
/// Rapport hauteur / largeur d'une image dont la source ne dit pas les proportions.
pub(crate) const IMAGE_ASPECT: f64 = 0.75;
/// Hauteur la plus petite d'un média qui partage la place restante, en pt.
pub(crate) const MIN_MEDIA: f64 = 160.0;
/// Largeur d'une puce de liste, en em.
const MARKER: f64 = 0.6;

/// Jetons d'une carte, en pt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardTokens {
    /// Marge horizontale (`component.card.padding-x`).
    pub padding_x: f64,
    /// Marge verticale (`component.card.padding-y`).
    pub padding_y: f64,
    /// Taille du titre (`component.card.title-size`).
    pub title: f64,
    /// Espace entre surtitre, titre et corps (`component.card.gap`).
    pub gap: f64,
}

/// Typographie et espacements du corps d'une slide, en pt (interlignes en em).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextTokens {
    /// Taille du texte courant (`size.body`).
    pub body: f64,
    /// Interligne du texte courant, en em (`leading.normal`).
    pub leading: f64,
    /// Espace entre deux paragraphes et entre deux blocs empilés (`space.md`).
    pub paragraph: f64,
    /// Espace entre deux items de liste (`component.list.spacing`).
    pub list_spacing: f64,
    /// Retrait d'un item de liste (`component.list.indent`).
    pub list_indent: f64,
    /// Taille du titre de slide (`component.title.size`).
    pub title: f64,
    /// Interligne du titre de slide, en em (`component.title.leading`).
    pub title_leading: f64,
    /// Taille du surtitre (`component.kicker.size`).
    pub kicker: f64,
    /// Espace entre le surtitre et le titre (`space.sm`).
    pub kicker_gap: f64,
    /// Gouttière des mises en page `columns` et `split` (`component.columns.gap`).
    pub columns_gap: f64,
    /// Gouttière d'une pile (`space.lg`, celle du widget `stack` par défaut).
    pub stack_gap: f64,
    /// Jetons des cartes.
    pub card: CardTokens,
    /// Taille du texte d'un tableau (`component.table.size`).
    pub table: f64,
    /// Marge verticale d'une cellule (`component.table.padding-y`).
    pub table_padding: f64,
    /// Tailles plus petites que le corps qu'offre le thème, de la plus grande
    /// à la plus petite : la réduction d'un flux qui déborde n'en prend pas d'autre.
    pub smaller: [(&'static str, f64); 2],
}

impl TextTokens {
    /// Les jetons de texte des jetons effectifs `tokens` d'un thème ; un jeton
    /// absent prend la valeur de `default-tokens.json`.
    pub fn from_tokens(tokens: &Map<String, Value>) -> Self {
        let num = |path: &[&str]| number_or_default(tokens, path);
        let tok = |family: &str, path: &[&str]| token_or_default(tokens, family, path);
        let leading = |path: &[&str]| tok("leading", path);
        Self {
            body: num(&["size", "body"]),
            leading: num(&["leading", "normal"]),
            paragraph: num(&["space", "md"]),
            list_spacing: tok("space", &["component", "list", "spacing"]),
            list_indent: tok("space", &["component", "list", "indent"]),
            title: tok("size", &["component", "title", "size"]),
            title_leading: leading(&["component", "title", "leading"]),
            kicker: tok("size", &["component", "kicker", "size"]),
            kicker_gap: num(&["space", "sm"]),
            columns_gap: tok("space", &["component", "columns", "gap"]),
            stack_gap: num(&["space", "lg"]),
            card: CardTokens {
                padding_x: tok("space", &["component", "card", "padding-x"]),
                padding_y: tok("space", &["component", "card", "padding-y"]),
                title: tok("size", &["component", "card", "title-size"]),
                gap: tok("space", &["component", "card", "gap"]),
            },
            table: tok("size", &["component", "table", "size"]),
            table_padding: tok("space", &["component", "table", "padding-y"]),
            smaller: [("small", num(&["size", "small"])), ("caption", num(&["size", "caption"]))],
        }
    }

    /// La taille d'un run : son jeton parmi [`TextTokens::smaller`], sinon `default`.
    fn run_size(&self, run: &Run, default: f64) -> f64 {
        match &run.size {
            Some(TokenValue::Number(n)) => *n,
            Some(TokenValue::Name(name)) => {
                self.smaller.iter().find(|(n, _)| n == name).map_or(default, |(_, size)| *size)
            }
            None => default,
        }
    }
}

/// Nombre de lignes de `text` en `size` pt dans `width` pt : passage à la
/// ligne par mots ; un mot plus long qu'une ligne en prend plusieurs.
pub(crate) fn lines(text: &str, width: f64, size: f64) -> usize {
    let per_line = (width / (size * GLYPH)).floor().max(1.0);
    // Borné par la longueur du texte (bornée par le cœur) : la conversion ne déborde pas.
    let per_line = per_line.min(usize::MAX as f64) as usize;
    let mut count = 0;
    for hard in text.split('\n') {
        let mut used = 0usize;
        let mut here = 1usize;
        for word in hard.split_whitespace() {
            let len = word.chars().count();
            if used == 0 {
                here += len.saturating_sub(1) / per_line;
                used = len % per_line;
                if used == 0 && len > 0 {
                    used = per_line;
                }
            } else if used + 1 + len <= per_line {
                used += 1 + len;
            } else {
                here += 1 + len.saturating_sub(1) / per_line;
                used = len % per_line;
                if used == 0 {
                    used = per_line;
                }
            }
        }
        count += here;
    }
    count.max(1)
}

/// Hauteur de `n` lignes en `size` pt, d'interligne `leading` em.
fn block(n: usize, size: f64, leading: f64) -> f64 {
    // Borné par la longueur du texte : la conversion reste exacte.
    let n = n as f64;
    n * CAP * size + (n - 1.0).max(0.0) * leading * size
}

/// Ce qui guide une estimation : jetons et proportions des images de la slide.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Measure<'a> {
    /// Jetons de texte du thème.
    pub tokens: &'a TextTokens,
    /// Rapport hauteur / largeur de chaque image, par asset.
    pub aspects: &'a BTreeMap<String, f64>,
}

impl Measure<'_> {
    /// Hauteur de paragraphes riches dans `width` pt, au corps `base` pt.
    ///
    /// Une ligne prend la hauteur de capitale de son plus grand run ;
    /// l'interligne reste celui du corps (Typst le résout sur le paragraphe).
    pub(crate) fn paragraphs(&self, paras: &[Paragraph], width: f64, base: f64) -> f64 {
        let t = self.tokens;
        let mut total = 0.0;
        let mut previous: Option<bool> = None;
        for p in paras {
            let list = p.list.is_some();
            let size =
                if p.runs.is_empty() { base } else { p.runs.iter().map(|r| t.run_size(r, base)).fold(0.0, f64::max) };
            let width = if list { width - t.list_indent - MARKER * size } else { width };
            let text: String = p.runs.iter().map(|r| r.t.as_str()).collect();
            if let Some(was_list) = previous {
                total += if was_list && list { t.list_spacing } else { t.paragraph };
            }
            // Borné par la longueur du texte : la conversion reste exacte.
            let n = lines(&text, width.max(1.0), size) as f64;
            total += n * CAP * size + (n - 1.0) * t.leading * base;
            previous = Some(list);
        }
        total
    }

    /// Hauteur d'un texte plain (titre ou surtitre de carte) en `size` pt.
    fn plain(&self, text: &str, width: f64, size: f64, leading: f64) -> f64 {
        block(lines(text, width.max(1.0), size), size, leading)
    }

    /// Hauteur de l'élément `e` dans `width` pt, une place de `room` pt au plus
    /// pour un média ; un média `framed` remplit son cadre et ne compte pas.
    pub(crate) fn element(&self, e: &Element, width: f64, room: f64, framed: bool) -> f64 {
        let t = self.tokens;
        match e.kind.as_str() {
            base::TEXT => {
                let size = match &e.style.size {
                    Some(TokenValue::Number(n)) => *n,
                    Some(TokenValue::Name(n)) => t.smaller.iter().find(|(k, _)| k == n).map_or(t.body, |(_, s)| *s),
                    None => t.body,
                };
                self.paragraphs(e.params.rich("body").unwrap_or_default(), width, size)
            }
            base::CARD => self.card(e, width),
            base::STACK => self.stack(e, width, room),
            base::TABLE => {
                let rows = e.params.value("data").and_then(|d| d.pointer("/dataset/source")).and_then(Value::as_array);
                // Au plus 100 lignes (`MAX_TABLE_ROWS`) : la conversion est exacte.
                let rows = rows.map_or(1, Vec::len) as f64;
                rows * (CAP * t.table + 2.0 * t.table_padding)
            }
            base::CHART if !framed => (width * CHART_ASPECT).min(room),
            base::IMAGE if !framed => (width * self.aspect(e)).min(room),
            _ => 0.0,
        }
    }

    /// Le rapport hauteur / largeur de l'image `e`.
    fn aspect(&self, e: &Element) -> f64 {
        e.params.text("asset").and_then(|a| self.aspects.get(a)).copied().unwrap_or(IMAGE_ASPECT)
    }

    fn card(&self, e: &Element, width: f64) -> f64 {
        let (t, c) = (self.tokens, &self.tokens.card);
        let inner = width - 2.0 * c.padding_x;
        let mut parts = Vec::new();
        if let Some(k) = e.params.text("kicker").filter(|k| !k.trim().is_empty()) {
            parts.push(self.plain(k, inner, t.kicker, t.leading));
        }
        if let Some(title) = e.params.text("title").filter(|k| !k.trim().is_empty()) {
            parts.push(self.plain(title, inner, c.title, t.leading));
        }
        if let Some(body) = e.params.rich("body").filter(|b| !b.is_empty()) {
            parts.push(self.paragraphs(body, inner, t.body));
        }
        // Au plus trois parts : la conversion est exacte.
        let gaps = parts.len().saturating_sub(1) as f64 * c.gap;
        2.0 * c.padding_y + parts.iter().sum::<f64>() + gaps
    }

    fn stack(&self, e: &Element, width: f64, room: f64) -> f64 {
        let gap = self.tokens.stack_gap;
        let children = match e.params.get("children") {
            Some(ParamValue::Slot(c)) => c.as_slice(),
            _ => &[],
        };
        if children.is_empty() {
            return 0.0;
        }
        let across = matches!(e.params.text("layout"), Some("columns" | "grid"));
        let n = if across {
            let count = e.params.value("count").and_then(Value::as_u64).and_then(|n| usize::try_from(n).ok());
            count.filter(|n| *n >= 1).unwrap_or(children.len())
        } else {
            1
        };
        // `n` est au plus le nombre d'enfants ou un compte du cœur : la conversion est exacte.
        let cw = (width - (n as f64 - 1.0) * gap) / n as f64;
        let rows: Vec<f64> = children
            .chunks(n.max(1))
            .map(|row| row.iter().map(|c| self.element(c, cw, room, false)).fold(0.0, f64::max))
            .collect();
        rows.iter().sum::<f64>() + (rows.len() as f64 - 1.0) * gap
    }

    /// Hauteur que demandent les blocs `blocks` d'une slide en mise en page
    /// `layout`, dans une zone de `width` × `height` pt : les médias seuls en
    /// bloc de la mise en page `default` comptent pour [`MIN_MEDIA`] au plus.
    pub(crate) fn flow(&self, blocks: &[Element], layout: &str, width: f64, height: f64) -> f64 {
        let t = self.tokens;
        let across = |n: usize, list: &[Element]| {
            // Au plus le nombre de blocs : la conversion est exacte.
            let cw = (width - (n as f64 - 1.0) * t.columns_gap) / n as f64;
            list.chunks(n.max(1))
                .map(|row| row.iter().map(|b| self.element(b, cw, height, false)).fold(0.0, f64::max))
                .sum::<f64>()
        };
        match layout {
            "columns" => across(blocks.len().max(1), blocks),
            "split" => across(2, blocks),
            _ => {
                let gaps = blocks.len().saturating_sub(1) as f64 * t.paragraph;
                let need: f64 = blocks
                    .iter()
                    .map(|b| match b.kind.as_str() {
                        base::CHART | base::IMAGE => self.element(b, width, height, false).min(MIN_MEDIA),
                        _ => self.element(b, width, height, false),
                    })
                    .sum();
                need + gaps
            }
        }
    }
}

/// La hauteur du titre de slide `title` (et de son surtitre `kicker`) dans
/// `width` pt, avec sa gouttière : ce que la zone de corps perd en haut.
pub(crate) fn title_band(t: &TextTokens, title: Option<&str>, kicker: Option<&str>, width: f64, gap: f64) -> f64 {
    let Some(title) = title.filter(|s| !s.trim().is_empty()) else { return 0.0 };
    let mut band = block(lines(title, width, t.title), t.title, t.title_leading) + gap;
    if kicker.is_some_and(|k| !k.trim().is_empty()) {
        band += CAP * t.kicker + t.kicker_gap;
    }
    band
}

/// Les runs de corps des éléments `list` (textes et cartes, piles comprises)
/// passés à la taille `size` (un jeton du thème).
pub(crate) fn resize(list: &mut [Element], size: &str) {
    for e in list.iter_mut() {
        if [base::TEXT, base::CARD].contains(&e.kind.as_str())
            && let Some(ParamValue::Rich(paras)) = e.params.0.get_mut("body")
        {
            for r in paras.iter_mut().flat_map(|p| p.runs.iter_mut()) {
                r.size = Some(TokenValue::Name(size.to_owned()));
            }
        }
        if let Some(ParamValue::Slot(children)) = e.params.0.get_mut("children") {
            resize(children, size);
        }
    }
}

#[cfg(test)]
#[path = "measure_tests.rs"]
mod measure_tests;
