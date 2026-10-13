//! Unité de contenu ([`Unit`]) → élément Oku, et replis des motifs sans widget.
//!
//! | unité | élément |
//! |---|---|
//! | carte (forme remplie portant des textes) | `card` : le plus grand texte en `title`, un court texte au-dessus en `kicker`, le reste en `body` |
//! | textes fusionnés | `text` (paragraphes mis bout à bout) |
//! | texte | `text` (corps riche) |
//! | forme remplie avec texte | `card` (corps riche) |
//! | forme non remplie avec texte | `text` |
//! | forme sans texte, remplie d'une image | `image` |
//! | filet (forme plus fine qu'un quart de piste) | `shape: line`, trait projeté (son trait, sinon son fond) |
//! | forme sans texte | `shape` (`rect`, `ellipse` ou `line` selon le preset), fond et trait projetés |
//! | image | `image` (`fit: contain`), si le rendu sait la dessiner |
//! | tableau | `table` |
//! | graphique | `chart` |
//! | groupe, SmartArt dessiné | `stack` de `card` (une par enfant avec texte) et d'éléments |
//! | intraduisible avec image de repli | `image`, perte `Opaque` |
//! | intraduisible sans repli | rien, perte `Opaque` |
//!
//! Les textes d'une carte ou des textes fusionnés sont mis bout à bout dans
//! les bornes du texte riche ; leurs pertes vont sur la forme de la carte ou
//! le premier texte. Une carte est en relief (`highlight`) quand son fond se projette sur un
//! jeton qui contraste (≥ 3:1) avec le fond d'une carte du thème et que le
//! thème a un jeton de texte clair pour le relief ; sinon son fond est
//! abandonné. Ses textes sont colorés contre le fond de la carte rendue.
//! Rotation, miroir et rognage sont perdus (`Transform`).

use std::collections::BTreeMap;

use oku_slides_core::{Element, Paragraph, ParamValue};
use serde_json::Value;

use super::chart::chart;
use super::cluster::Unit;
use super::color::{ColorTokens, MIN_CONTRAST, contrast, project_color};
use super::group::group;
use super::heading::{is_kicker_text, max_run_size};
use super::table::table;
use super::text::{note, plain, rich};
use crate::report::{Loss, LossKind};
use crate::source::{self, Asset, Fill, Geometry, Item, ItemKind, TextBox};

/// Types MIME que le rendu sait dessiner.
pub(crate) const DRAWABLE: [&str; 5] = ["image/png", "image/jpeg", "image/gif", "image/webp", "image/svg+xml"];
/// Presets dessinés en ellipse.
const ELLIPSES: [&str; 3] = ["ellipse", "flowChartConnector", "donut"];
/// Part d'une piste de grille sous laquelle une forme est un filet.
const THIN_TRACK: f64 = 0.25;

/// Ce dont la conversion a besoin.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ContentCx<'a> {
    /// Couleurs du thème cible.
    pub tokens: &'a ColorTokens,
    /// Assets de la source, par id.
    pub assets: &'a BTreeMap<String, Asset>,
    /// Largeur d'une colonne et hauteur d'une ligne de la grille de la slide,
    /// en pt source : une forme plus fine qu'un quart de piste est un filet
    /// (`(0, 0)` : jamais).
    pub track: (f64, f64),
}

/// Une unité convertie.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Content {
    /// L'élément ; `None` si l'unité ne donne rien (vide, intraduisible).
    pub element: Option<Element>,
    /// Pertes, chacune une fois par item.
    pub losses: Vec<Loss>,
    /// Sorte du widget de repli employé (`stack`), pour un motif sans widget
    /// dont le repli perd quelque chose (voir [`group`]).
    pub fallback: Option<String>,
}

/// Ajoute la perte `kind` sur `item` si elle n'y est pas.
pub(crate) fn lose(losses: &mut Vec<Loss>, item: &str, kind: LossKind) {
    note(losses, Loss::on(item, kind));
}

/// Le jeton de `color`, ou rien (couleur par défaut, ou abandonnée avec une perte).
fn token(color: &source::Color, cx: &ContentCx<'_>, item: &str, losses: &mut Vec<Loss>) -> Option<String> {
    project_color(color, cx.tokens).unwrap_or_else(|loss| {
        lose(losses, item, loss);
        None
    })
}

/// La couleur unie d'un fond : la sienne, ou le premier arrêt d'un dégradé.
fn fill_color(fill: &Fill) -> Option<&source::Color> {
    match fill {
        Fill::Solid { color } => Some(color),
        Fill::Gradient { stops, .. } => stops.first().map(|s| &s.color),
        Fill::None | Fill::Picture { .. } => None,
    }
}

/// Où se pose un texte : sur la page, ou sur une carte (en relief ou non).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ground {
    /// Le fond de page `bg`.
    Page,
    /// Le fond d'une carte, en relief (`true`) ou non.
    Card(bool),
}

/// Le texte riche de `boxes` (pris dans l'ordre) posé sur `ground`, pertes notées sur `item`.
pub(crate) fn body(
    boxes: &[&TextBox],
    ground: Ground,
    cx: &ContentCx<'_>,
    item: &str,
    losses: &mut Vec<Loss>,
) -> Vec<Paragraph> {
    let bg = match ground {
        Ground::Page => cx.tokens.page(),
        Ground::Card(highlight) => cx.tokens.card(highlight),
    };
    let out = rich(boxes.iter().copied(), cx.tokens, bg);
    for l in out.losses {
        lose(losses, item, l);
    }
    out.paragraphs
}

/// Le texte plain de `tb`, pertes notées sur `item`.
pub(crate) fn plain_of(tb: &TextBox, item: &str, losses: &mut Vec<Loss>) -> String {
    let (text, lost) = plain(tb);
    for l in lost {
        lose(losses, item, l);
    }
    text
}

/// Un élément `text` au corps `paragraphs`, ou rien s'il est vide.
fn text(id: String, paragraphs: Vec<Paragraph>) -> Option<Element> {
    (!paragraphs.is_empty()).then(|| Element::text(id, paragraphs))
}

/// Une carte : surtitre et titre (plain) éventuels, corps riche.
pub(crate) fn card(
    id: String,
    kicker: Option<String>,
    title: Option<String>,
    paragraphs: Vec<Paragraph>,
    highlight: bool,
) -> Element {
    let mut e = Element::new(id, "card");
    for (name, value) in [("kicker", kicker), ("title", title)] {
        if let Some(t) = value.filter(|t| !t.is_empty()) {
            e.params.set(name, ParamValue::Text(t));
        }
    }
    if !paragraphs.is_empty() {
        e.params.set("body", ParamValue::Rich(paragraphs));
    }
    if highlight {
        e.params.set("highlight", ParamValue::Value(Value::Bool(true)));
    }
    e
}

/// Une carte au fond `fill` est-elle en relief ? Son jeton contraste avec le
/// fond d'une carte du thème, et le thème sait écrire clair sur le relief.
fn highlighted(fill: &Fill, cx: &ContentCx<'_>, item: &str, losses: &mut Vec<Loss>) -> bool {
    let Some(t) = fill_color(fill).and_then(|c| token(c, cx, item, losses)) else { return false };
    let tokens = cx.tokens;
    tokens.get(&t).is_some_and(|c| contrast(c, tokens.card(false)) >= MIN_CONTRAST)
        && tokens.on_dark(tokens.card(true)).is_some()
}

/// Un élément `image` de l'asset `asset`, ou rien s'il manque ou ne se dessine pas
/// (perte `UnsupportedImage`).
pub(crate) fn image(
    id: String,
    asset: &str,
    cx: &ContentCx<'_>,
    item: &str,
    losses: &mut Vec<Loss>,
) -> Option<Element> {
    let a = cx.assets.get(asset)?;
    if !DRAWABLE.contains(&a.mime.as_str()) {
        lose(losses, item, LossKind::UnsupportedImage(a.mime.clone()));
        return None;
    }
    let mut e = Element::new(id, "image");
    e.params.set("asset", ParamValue::Text(asset.to_owned()));
    e.params.set("fit", ParamValue::Text("contain".to_owned()));
    Some(e)
}

/// Un élément porteur d'une option d'hôte (`table.data`, `chart.option`).
fn host(
    id: String,
    kind: &str,
    param: &str,
    option: Option<(Value, Vec<LossKind>)>,
    item: &str,
    losses: &mut Vec<Loss>,
) -> Option<Element> {
    let (value, lost) = option?;
    for l in lost {
        lose(losses, item, l);
    }
    let mut e = Element::new(id, kind);
    e.params.set(param, ParamValue::Value(value));
    Some(e)
}

/// La forme du widget `shape` pour une géométrie source.
fn shape_kind(geom: &Geometry) -> &'static str {
    match geom {
        Geometry::Preset { name, .. } if ELLIPSES.contains(&name.as_str()) => "ellipse",
        Geometry::Preset { name, .. } if name == "line" || name.contains("Connector") => "line",
        Geometry::Preset { .. } | Geometry::Path { .. } => "rect",
    }
}

/// Une forme : carte ou texte si elle porte du texte, image si elle est
/// remplie d'une image, filet si elle est plus fine qu'un quart de piste,
/// sinon `shape` ; rien si ni son fond ni son trait n'ont de jeton.
fn shape(id: String, item: &Item, s: &source::Shape, cx: &ContentCx<'_>, losses: &mut Vec<Loss>) -> Option<Element> {
    if let Some(tb) = s.text.as_ref().filter(|_| item.has_text()) {
        if s.fill.is_none() {
            return text(id, body(&[tb], Ground::Page, cx, &item.id, losses));
        }
        let highlight = highlighted(&s.fill, cx, &item.id, losses);
        return Some(card(id, None, None, body(&[tb], Ground::Card(highlight), cx, &item.id, losses), highlight));
    }
    if let Fill::Picture { asset } = &s.fill {
        return image(id, asset, cx, &item.id, losses);
    }
    let thin = is_thin(&item.frame, cx.track);
    let fill = fill_color(&s.fill).and_then(|c| token(c, cx, &item.id, losses));
    let stroke = s.stroke.as_ref().and_then(|st| token(&st.color, cx, &item.id, losses));
    let mut e = Element::new(id, "shape");
    if thin {
        e.params.set("shape", ParamValue::Text("line".to_owned()));
        e.style.stroke = Some(stroke.or(fill)?);
        return Some(e);
    }
    if fill.is_none() && stroke.is_none() {
        return None;
    }
    e.params.set("shape", ParamValue::Text(shape_kind(&s.geom).to_owned()));
    e.style.fill = fill;
    e.style.stroke = stroke;
    Some(e)
}

/// Une boîte plus fine qu'un quart de piste (`track` : colonne et ligne en pt source) : un filet.
pub(crate) fn is_thin(f: &source::Frame, track: (f64, f64)) -> bool {
    f.w < THIN_TRACK * track.0 || f.h < THIN_TRACK * track.1
}

/// La perte `Transform` d'un item tourné ou retourné.
pub(crate) fn transform(item: &Item, losses: &mut Vec<Loss>) {
    let f = &item.frame;
    if f.rot != 0.0 || f.flip_h || f.flip_v {
        lose(losses, &item.id, LossKind::Transform);
    }
}

/// L'élément d'un item qui n'est pas un groupe.
pub(crate) fn leaf(item: &Item, id: String, cx: &ContentCx<'_>, losses: &mut Vec<Loss>) -> Option<Element> {
    transform(item, losses);
    match &item.kind {
        ItemKind::Text(tb) => text(id, body(&[tb], Ground::Page, cx, &item.id, losses)),
        ItemKind::Shape(s) => shape(id, item, s, cx, losses),
        ItemKind::Picture(p) => {
            if p.crop != source::Crop::default() {
                lose(losses, &item.id, LossKind::Transform);
            }
            image(id, &p.asset, cx, &item.id, losses)
        }
        ItemKind::Table(t) => host(id, "table", "data", table(t), &item.id, losses),
        ItemKind::Chart(c) => host(id, "chart", "option", chart(c), &item.id, losses),
        ItemKind::Opaque(o) => {
            lose(losses, &item.id, LossKind::Opaque(o.reason.clone()));
            o.fallback.as_deref().and_then(|a| image(id, a, cx, &item.id, losses))
        }
        ItemKind::Group { .. } => None,
    }
}

/// Le titre d'une carte : le texte le plus grand, s'il y en a plusieurs et
/// qu'il dépasse strictement tous les autres.
fn card_title<'a>(texts: &[(&'a Item, &'a TextBox)]) -> Option<usize> {
    if texts.len() < 2 {
        return None;
    }
    let (best, size) = texts
        .iter()
        .enumerate()
        .map(|(k, (_, tb))| (k, max_run_size(tb)))
        .fold((0, 0.0), |acc, (k, s)| if s > acc.1 { (k, s) } else { acc });
    texts.iter().enumerate().all(|(k, (_, tb))| k == best || max_run_size(tb) < size).then_some(best)
}

/// Le surtitre d'une carte : le texte juste au-dessus du titre, d'un seul
/// paragraphe d'au plus 80 caractères, plus petit que lui.
fn card_kicker(texts: &[(&Item, &TextBox)], title: usize) -> Option<usize> {
    let (t_item, t_box) = texts[title];
    let above = |k: usize| {
        let (item, tb) = texts[k];
        item.frame.y + item.frame.h <= t_item.frame.y + 1.0
            && is_kicker_text(tb)
            && max_run_size(tb) < max_run_size(t_box)
    };
    (0..title).rev().find(|k| above(*k))
}

/// La carte d'une forme remplie et des textes posés dessus.
fn cluster_card(id: String, ground: &Item, texts: &[&Item], cx: &ContentCx<'_>, losses: &mut Vec<Loss>) -> Element {
    transform(ground, losses);
    let fill = match &ground.kind {
        ItemKind::Shape(s) => &s.fill,
        _ => &Fill::None,
    };
    let highlight = highlighted(fill, cx, &ground.id, losses);
    let boxes: Vec<(&Item, &TextBox)> = texts.iter().filter_map(|i| i.text().map(|tb| (*i, tb))).collect();
    for (item, _) in &boxes {
        transform(item, losses);
    }
    let title = card_title(&boxes);
    let kicker = title.and_then(|t| card_kicker(&boxes, t));
    let head = |k: Option<usize>, losses: &mut Vec<Loss>| k.map(|k| plain_of(boxes[k].1, &boxes[k].0.id, losses));
    let (kicker_text, title_text) = (head(kicker, losses), head(title, losses));
    let rest: Vec<&TextBox> = boxes
        .iter()
        .enumerate()
        .filter(|(k, _)| Some(*k) != title && Some(*k) != kicker)
        .map(|(_, (_, tb))| *tb)
        .collect();
    let paragraphs = body(&rest, Ground::Card(highlight), cx, &ground.id, losses);
    card(id, kicker_text, title_text, paragraphs, highlight)
}

/// L'élément d'id `id` pour `unit`, ses pertes et son repli.
///
/// Les enfants d'un repli ont pour id `<id>-1`, `<id>-2`… Des textes fusionnés
/// gardent leurs pertes sur leurs propres items.
pub(crate) fn element(unit: &Unit<'_>, id: String, cx: &ContentCx<'_>) -> Content {
    let mut losses = Vec::new();
    let (element, fallback) = match unit {
        Unit::Card { shape, texts, .. } => (Some(cluster_card(id, shape, texts, cx, &mut losses)), None),
        Unit::Text(items) => {
            for item in items {
                transform(item, &mut losses);
            }
            let boxes: Vec<&TextBox> = items.iter().filter_map(|i| i.text()).collect();
            (text(id, body(&boxes, Ground::Page, cx, &unit.lead().id, &mut losses)), None)
        }
        Unit::Item(item) => match &item.kind {
            ItemKind::Group { items, smartart } => {
                transform(item, &mut losses);
                group(items, *smartart, &id, cx, &mut losses)
            }
            _ => (leaf(item, id, cx, &mut losses), None),
        },
    };
    Content { element, losses, fallback }
}

#[cfg(test)]
#[path = "content_tests.rs"]
mod content_tests;
