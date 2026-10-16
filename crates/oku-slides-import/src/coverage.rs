//! Couverture du contenu : ce que le deck Oku obtenu garde de la source.
//!
//! Mesurée sur le deck, pas sur le plan : un contenu compte s'il s'y
//! retrouve. La slide source `n` est comparée à la slide Oku `s<n>` (ids du
//! projecteur), plus les paramètres du deck (`title`, `subtitle`) que remplit
//! la couverture.
//!
//! | contenu source | retrouvé si la slide Oku contient |
//! |---|---|
//! | run de texte | son texte (espaces normalisées), à ses frontières de mots, dans un texte de l'élément projeté depuis son item (sinon de la slide) |
//! | run de moins de 3 caractères | son paragraphe (ou sa cellule) entier, à l'identique, dans l'élément (sinon la slide) |
//! | image (image, forme remplie d'une image, repli d'un intraduisible) | un élément `image` du même asset |
//! | tableau | un tableau dont `dataset.source` est la grille source (cellules couvertes `null`) |
//! | graphique | un graphique dont une colonne de `dataset.source` porte les valeurs de chaque série |
//!
//! Frontières de mots : un run qui commence (ou finit) par une lettre ou un
//! chiffre séparé de son voisin dans la source (espace, ponctuation, bord du
//! paragraphe) doit l'être aussi dans le texte trouvé ; un run collé à un
//! voisin au milieu d'un mot, ou bordé de ponctuation, peut l'être dans le deck. Un run
//! en capitales (`Caps::All`) se compare sans la casse ; un run sans lettre ni
//! chiffre ne compte pas. L'élément projeté depuis un item vient de
//! `origins` (id d'item → id d'élément, enfants compris) ; un titre posé en
//! paramètre n'y est pas et se cherche dans la slide.
//!
//! Les items que le projecteur omet par règle sont hors du compte (`omitted`) :
//! la projection et la couverture partagent ce prédicat. Le retrofit omet le
//! décor des slides ([`crate::retrofit::decor::Decor::skips`] : date, pied,
//! numéro, en-tête et pied répétés) ; le hi-fi n'omet rien, le décor des
//! masters et des layouts (que son pack dessine) n'étant pas dans les items
//! des slides. Dans les deux modes, un champ numéro de slide (`slidenum`)
//! est hors du compte : le deck le recalcule, son texte en cache n'est pas
//! un contenu.

use std::collections::BTreeMap;

use oku_slides_core::{Deck, Element, HostType, ParamType, ParamValue, Params, Slide, WidgetRegistry};
use serde_json::Value;

use crate::report::{Coverage, Missing, Ratio};
use crate::retrofit::text::plain;
use crate::source::{self, Caps, Fill, Item, ItemKind, SLIDE_NUMBER, SourceDeck, TextBox};

/// Longueur (en caractères) sous laquelle un run se cherche par son paragraphe entier.
const SHORT_RUN: usize = 3;

/// `s` aux espaces normalisées.
fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Le texte de paragraphes riches, un paragraphe par ligne.
fn rich_text(paragraphs: &[oku_slides_core::Paragraph]) -> impl Iterator<Item = String> + '_ {
    paragraphs.iter().map(|p| p.runs.iter().map(|r| r.t.as_str()).collect())
}

/// Ce qu'une slide Oku montre : textes (de la slide, et de chaque élément avec
/// ses descendants), assets d'images, données de tableaux et de graphiques.
#[derive(Debug, Default)]
struct Seen<'d> {
    texts: Vec<String>,
    by_element: BTreeMap<&'d str, Vec<String>>,
    images: Vec<&'d str>,
    tables: Vec<&'d Value>,
    charts: Vec<&'d Value>,
}

impl<'d> Seen<'d> {
    /// Ajoute le texte `t` à la slide et aux éléments `owners`.
    fn text(&mut self, t: &str, owners: &[&'d str]) {
        let t = norm(t);
        for o in owners {
            self.by_element.entry(o).or_default().push(t.clone());
        }
        self.texts.push(t);
    }

    /// Les cellules textuelles d'un `dataset.source`.
    fn data_texts(&mut self, option: &'d Value, owners: &[&'d str]) {
        let rows = option.pointer("/dataset/source").and_then(Value::as_array).into_iter().flatten();
        for row in rows {
            for cell in row.as_array().into_iter().flatten() {
                if let Some(s) = cell.as_str() {
                    self.text(s, owners);
                }
            }
        }
    }

    /// Les paramètres `params` d'un widget `kind`, dans les éléments `owners`.
    fn params(&mut self, kind: &str, params: &'d Params, registry: &WidgetRegistry, owners: &[&'d str]) {
        let schema = registry.get(kind);
        for (name, value) in params.iter() {
            let ty = schema.and_then(|w| w.param(name)).map(|p| &p.ty);
            match (value, ty) {
                (ParamValue::Text(t), Some(ParamType::Plain) | None) => self.text(t, owners),
                (ParamValue::Text(a), Some(ParamType::Asset)) if kind == "image" => self.images.push(a),
                (ParamValue::Rich(ps), _) => {
                    for t in rich_text(ps) {
                        self.text(&t, owners);
                    }
                }
                (ParamValue::Slot(children), _) => self.elements(children, registry, owners),
                (ParamValue::Value(v), Some(t)) => {
                    match t.host() {
                        Some(HostType::Table) => self.tables.push(v),
                        Some(HostType::Chart) => self.charts.push(v),
                        Some(HostType::Path) | None => continue,
                    }
                    self.data_texts(v, owners);
                }
                _ => {}
            }
        }
    }

    fn elements(&mut self, list: &'d [Element], registry: &WidgetRegistry, owners: &[&'d str]) {
        for e in list {
            let mut inner = owners.to_vec();
            inner.push(&e.id);
            self.params(&e.kind, &e.params, registry, &inner);
        }
    }

    /// Ce que montre `slide`, plus le titre et le sous-titre du deck.
    fn of(deck: &'d Deck, slide: Option<&'d Slide>, with_deck: bool, registry: &WidgetRegistry) -> Self {
        let mut seen = Seen::default();
        if with_deck {
            for t in [&deck.params.title, &deck.params.subtitle].into_iter().flatten() {
                seen.text(t, &[]);
            }
        }
        if let Some(s) = slide {
            seen.params(&s.kind, &s.params, registry, &[]);
        }
        seen
    }

    /// Les textes où chercher le contenu de l'élément `element` ; sans élément d'origine, ceux de la slide.
    fn scope(&self, element: Option<&str>) -> &[String] {
        match element {
            Some(e) => self.by_element.get(e).map_or(&[], Vec::as_slice),
            None => &self.texts,
        }
    }
}

/// Un run à chercher : son texte, s'il est collé à ses voisins, son paragraphe.
#[derive(Debug, Clone, PartialEq)]
struct Probe {
    /// Texte, espaces normalisées (en minuscules pour un run en capitales).
    text: String,
    /// Texte tel que relevé au rapport.
    shown: String,
    /// Le run commence un mot : le texte trouvé doit en commencer un aussi.
    word_start: bool,
    /// Le run finit un mot : le texte trouvé doit en finir un aussi.
    word_end: bool,
    /// Son paragraphe (ou sa cellule) entier, espaces normalisées.
    paragraph: String,
    /// Comparaison sans la casse.
    fold: bool,
}

/// Les runs à chercher de `tb` ; `whole` remplace le paragraphe (texte d'une cellule).
fn probes(tb: &TextBox, whole: Option<&str>) -> Vec<Probe> {
    let mut out = Vec::new();
    for p in &tb.paragraphs {
        let raw: Vec<&str> = p.runs.iter().map(|r| r.text.as_str()).collect();
        let paragraph = whole.map_or_else(|| norm(&raw.concat()), norm);
        for (k, r) in p.runs.iter().enumerate() {
            let text = norm(&r.text);
            // Le numéro de slide est recalculé par le deck : hors du compte.
            if !text.chars().any(char::is_alphanumeric) || r.field.as_deref() == Some(SLIDE_NUMBER) {
                continue;
            }
            let before = raw[..k].concat().chars().last();
            let after = raw[k + 1..].concat().chars().next();
            let alnum = |c: Option<char>| c.is_some_and(char::is_alphanumeric);
            let fold = r.style.caps == Caps::All;
            let word_start = alnum(text.chars().next()) && !(alnum(before) && alnum(r.text.chars().next()));
            let word_end = alnum(text.chars().last()) && !(alnum(after) && alnum(r.text.chars().last()));
            out.push(Probe {
                text: if fold { text.to_lowercase() } else { text.clone() },
                shown: text,
                word_start,
                word_end,
                paragraph: paragraph.clone(),
                fold,
            });
        }
    }
    out
}

/// `probe` se trouve-t-il dans `hay`, à ses frontières de mots ?
fn within(hay: &str, probe: &Probe) -> bool {
    let hay = if probe.fold { hay.to_lowercase() } else { hay.to_owned() };
    hay.match_indices(probe.text.as_str()).any(|(at, m)| {
        let before = hay[..at].chars().last();
        let after = hay[at + m.len()..].chars().next();
        let open = |c: Option<char>| c.is_none_or(|c| !c.is_alphanumeric());
        (!probe.word_start || open(before)) && (!probe.word_end || open(after))
    })
}

/// `probe` est-il montré par l'un de `texts` ?
fn shows(texts: &[String], probe: &Probe) -> bool {
    if probe.text.chars().count() < SHORT_RUN {
        let same = |t: &String| {
            if probe.fold { t.to_lowercase() == probe.paragraph.to_lowercase() } else { *t == probe.paragraph }
        };
        return texts.iter().any(same);
    }
    texts.iter().any(|t| within(t, probe))
}
/// Le texte d'une cellule tel que le tableau du cœur le montre.
fn cell_text(c: &source::Cell) -> String {
    norm(&plain(&c.text).0)
}

/// La grille attendue d'un tableau source : texte plain des cellules, cellules couvertes `null`,
/// lignes complétées à la plus large.
fn table_grid(t: &source::Table) -> Value {
    let width = t.rows.iter().map(Vec::len).max().unwrap_or(0);
    let rows = t.rows.iter().map(|row| {
        let mut cells: Vec<Value> =
            row.iter().map(|c| if c.covered { Value::Null } else { Value::String(cell_text(c)) }).collect();
        cells.resize(width, Value::Null);
        Value::Array(cells)
    });
    Value::Array(rows.collect())
}

/// Les données de `option` portent-elles les catégories de `c` (colonne 0) et, pour chaque
/// série, une colonne distincte de mêmes valeurs ?
fn chart_matches(c: &source::Chart, option: &Value) -> bool {
    let Some(rows) = option.pointer("/dataset/source").and_then(Value::as_array) else { return false };
    let body = rows.get(1..).unwrap_or_default();
    let width = rows.first().and_then(Value::as_array).map_or(0, Vec::len);
    let cell = |r: &Value, j: usize| r.as_array().and_then(|r| r.get(j)).cloned();
    // Un nuage de points n'a pas de colonne de catégories : ses colonnes sont des paires (x, y).
    let scatter = c.kind == source::ChartKind::Scatter;
    if !scatter {
        let shown = |r: &Value| cell(r, 0).and_then(|v| v.as_str().map(norm)).unwrap_or_default();
        let want = |i: usize| c.categories.get(i).map_or_else(|| (i + 1).to_string(), |t| norm(t));
        if !body.iter().enumerate().all(|(i, r)| shown(r) == want(i)) {
            return false;
        }
    }
    let column = |j: usize| -> Vec<Option<f64>> { body.iter().map(|r| cell(r, j).and_then(|v| v.as_f64())).collect() };
    let mut free: Vec<usize> = (usize::from(!scatter)..width).collect();
    c.series.iter().all(|s| {
        let mut want: Vec<Option<f64>> = s.values.iter().map(|v| v.filter(|x| x.is_finite())).collect();
        want.resize(body.len(), None);
        let at = free.iter().position(|&j| column(j) == want);
        at.is_some_and(|k| {
            free.remove(k);
            true
        })
    })
}

/// L'asset d'image d'un item : image, forme remplie d'une image, repli d'un intraduisible.
fn image_asset(item: &Item) -> Option<&str> {
    match &item.kind {
        ItemKind::Picture(p) => Some(&p.asset),
        ItemKind::Shape(s) => match &s.fill {
            Fill::Picture { asset } => Some(asset),
            _ => None,
        },
        ItemKind::Opaque(o) => o.fallback.as_deref(),
        _ => None,
    }
}

/// Les comptes en cours.
#[derive(Debug, Default)]
struct Tally {
    c: Coverage,
}

fn count(r: &mut Ratio, found: bool) {
    r.total += 1;
    r.found += u32::from(found);
}

impl Tally {
    /// Compte les runs `probes` de la slide `slide` dans `texts`.
    fn runs(&mut self, slide: u32, probes: Vec<Probe>, texts: &[String]) {
        for p in probes {
            let found = shows(texts, &p);
            count(&mut self.c.runs, found);
            if !found {
                self.c.missing.push(Missing::Run { slide, text: p.shown });
            }
        }
    }

    /// Compte `item` (et ses enfants) de la slide source `slide` contre `seen` ;
    /// `element` : l'élément projeté depuis un parent.
    fn item(&mut self, slide: u32, item: &Item, seen: &Seen<'_>, cx: &Cx<'_>, element: Option<&str>) {
        if (cx.omitted)(item) {
            return;
        }
        let element = cx.origins.get(&item.id).map(String::as_str).or(element);
        let texts = seen.scope(element);
        if let Some(tb) = item.text() {
            self.runs(slide, probes(tb, None), texts);
        }
        if let Some(asset) = image_asset(item) {
            let found = seen.images.contains(&asset);
            count(&mut self.c.images, found);
            if !found {
                self.c.missing.push(Missing::Image { slide, item: item.id.clone() });
            }
        }
        match &item.kind {
            ItemKind::Group { items, .. } => {
                for child in items {
                    self.item(slide, child, seen, cx, element);
                }
            }
            ItemKind::Table(t) => {
                for c in t.rows.iter().flatten().filter(|c| !c.covered) {
                    self.runs(slide, probes(&c.text, Some(&cell_text(c))), texts);
                }
                let grid = table_grid(t);
                let found = seen.tables.iter().any(|v| v.pointer("/dataset/source") == Some(&grid));
                count(&mut self.c.tables, found);
                if !found {
                    self.c.missing.push(Missing::Table { slide, item: item.id.clone() });
                }
            }
            ItemKind::SmartArt(sa) => {
                for n in &sa.nodes {
                    self.runs(slide, probes(&n.text, None), texts);
                }
            }
            ItemKind::Chart(c) => {
                let found = seen.charts.iter().any(|v| chart_matches(c, v));
                count(&mut self.c.charts, found);
                if !found {
                    self.c.missing.push(Missing::Chart { slide, item: item.id.clone() });
                }
            }
            _ => {}
        }
    }
}

/// Ce que le compte partage : les items omis, l'origine des éléments.
struct Cx<'a> {
    omitted: &'a dyn Fn(&Item) -> bool,
    origins: &'a BTreeMap<String, String>,
}

/// La couverture du contenu de `source` dans `deck` (registre `registry` du
/// pack) ; `origins` : l'id de l'élément projeté depuis chaque item source.
/// `deck_slide` : le numéro de la slide source dont le titre et le sous-titre
/// remplissent ceux du deck. `omitted` : les items que le projecteur omet par
/// règle, hors du compte (avec leurs enfants).
pub fn coverage(
    source: &SourceDeck,
    deck: &Deck,
    registry: &WidgetRegistry,
    origins: &BTreeMap<String, String>,
    deck_slide: Option<u32>,
    omitted: &dyn Fn(&Item) -> bool,
) -> Coverage {
    let cx = Cx { omitted, origins };
    let mut tally = Tally::default();
    for s in &source.slides {
        let seen = Seen::of(deck, deck.slide(&format!("s{}", s.number)), deck_slide == Some(s.number), registry);
        for item in &s.items {
            tally.item(s.number, item, &seen, &cx, None);
        }
    }
    tally.c
}

#[cfg(test)]
#[path = "coverage_tests.rs"]
mod coverage_tests;
