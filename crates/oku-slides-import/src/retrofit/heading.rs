//! Titres d'une slide : ceux des placeholders, sinon déduits de la taille et
//! de la position des textes (une slide dessinée à la main n'a pas de
//! placeholder ; [`infer_roles`]).

use super::text::plain;
use crate::source::{Frame, Item, Role, Size, TextBox};

/// Taille minimale d'un titre déduit, en pt.
const TITLE_MIN: f64 = 20.0;
/// Rapport minimal entre un titre déduit et la taille médiane de la slide.
const TITLE_RATIO: f64 = 1.4;
/// Part haute de la page où commence un titre déduit.
const TITLE_ZONE: f64 = 0.4;
/// Taille à partir de laquelle un titre fait une couverture et admet un sous-titre.
const COVER_TITLE: f64 = 40.0;
/// Rapport minimal entre un sous-titre et la taille médiane.
const SUBTITLE_RATIO: f64 = 1.15;
/// Écart vertical maximal entre un titre et son surtitre ou son sous-titre, en part de la hauteur de page.
const NEAR: f64 = 0.1;
/// Longueur maximale d'un surtitre (de slide ou de carte), en caractères.
const KICKER_MAX: usize = 80;

/// Un titre déduit d'une slide sans placeholder de titre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Heading {
    /// Titre.
    Title,
    /// Surtitre.
    Kicker,
    /// Sous-titre.
    Subtitle,
}

/// Les titres d'une slide.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Headings<'a> {
    /// Titre.
    pub title: Option<&'a Item>,
    /// Sous-titre.
    pub subtitle: Option<&'a Item>,
    /// Surtitre (déduit seulement).
    pub kicker: Option<&'a Item>,
    /// Couverture déduite : un titre d'au moins 40 pt, sans autre texte de titre.
    pub cover: bool,
}

impl Headings<'_> {
    /// L'item `item` est-il un des titres ?
    pub(crate) fn holds(&self, item: &Item) -> bool {
        [self.title, self.subtitle, self.kicker].into_iter().flatten().any(|h| h.id == item.id)
    }
}

/// La plus grande taille de caractère d'un texte.
pub(crate) fn max_run_size(tb: &TextBox) -> f64 {
    tb.paragraphs.iter().flat_map(|p| &p.runs).map(|r| r.style.size).fold(0.0, f64::max)
}

/// La plus grande taille de caractère d'un item avec texte.
pub(crate) fn size_of(item: &Item) -> f64 {
    item.text().map_or(0.0, max_run_size)
}

/// Le texte est-il de la forme d'un surtitre : un seul paragraphe non vide
/// d'au plus 80 caractères (la taille et la place se jugent chez l'appelant) ?
pub(crate) fn is_kicker_text(tb: &TextBox) -> bool {
    tb.paragraphs.iter().filter(|p| p.runs.iter().any(|r| !r.text.trim().is_empty())).count() == 1
        && plain(tb).0.chars().count() <= KICKER_MAX
}

/// Les deux boîtes se recouvrent-elles en largeur ?
fn overlaps_x(a: &Frame, b: &Frame) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w
}

/// Les titres déduits de `items` (une slide sans placeholder de titre), sur une page `page`.
///
/// La taille d'un item est sa plus grande taille de caractère ; la médiane est
/// celle des items avec texte (médiane basse pour un nombre pair).
/// - titre : la plus grande taille (à égalité, le premier dessiné) d'un item
///   qui commence dans les 40 % hauts de la page, d'au moins 20 pt et 1,4 × la médiane ;
/// - surtitre : un texte d'un paragraphe d'au plus 80 caractères, plus petit
///   que le titre, qui finit au plus 0,1 × la hauteur de page au-dessus de lui
///   et le recouvre en largeur (le plus proche) ;
/// - sous-titre, si le titre fait au moins 40 pt : le texte juste au-dessous
///   (au plus 0,1 × la hauteur de page), qui le recouvre en largeur, d'au moins 1,15 × la médiane.
pub(crate) fn infer_roles(items: &[&Item], page: Size) -> Vec<(String, Heading)> {
    let texts: Vec<&Item> = items.iter().copied().filter(|i| i.has_text()).collect();
    let mut sizes: Vec<f64> = texts.iter().map(|i| size_of(i)).collect();
    sizes.sort_by(f64::total_cmp);
    let Some(median) = sizes.get(sizes.len().saturating_sub(1) / 2).copied() else { return Vec::new() };
    let title =
        texts.iter().copied().filter(|i| i.frame.y <= TITLE_ZONE * page.h).fold(None, |best: Option<&Item>, i| {
            if best.is_none_or(|b| size_of(i) > size_of(b)) { Some(i) } else { best }
        });
    let Some(title) = title.filter(|t| size_of(t) >= TITLE_MIN && size_of(t) >= TITLE_RATIO * median) else {
        return Vec::new();
    };
    let near = NEAR * page.h;
    let t = title.frame;
    let mut roles = vec![(title.id.clone(), Heading::Title)];
    let kicker = texts
        .iter()
        .filter(|i| i.id != title.id && overlaps_x(&i.frame, &t) && size_of(i) < size_of(title))
        .filter(|i| {
            let bottom = i.frame.y + i.frame.h;
            bottom <= t.y + 1.0 && t.y - bottom <= near
        })
        .filter(|i| i.text().is_some_and(is_kicker_text))
        .max_by(|a, b| (a.frame.y + a.frame.h).total_cmp(&(b.frame.y + b.frame.h)));
    if let Some(k) = kicker {
        roles.push((k.id.clone(), Heading::Kicker));
    }
    if size_of(title) >= COVER_TITLE {
        let below = t.y + t.h;
        let subtitle = texts
            .iter()
            .filter(|i| i.id != title.id && overlaps_x(&i.frame, &t) && size_of(i) >= SUBTITLE_RATIO * median)
            .filter(|i| i.frame.y >= below - 1.0 && i.frame.y - below <= near)
            .min_by(|a, b| a.frame.y.total_cmp(&b.frame.y));
        if let Some(s) = subtitle {
            roles.push((s.id.clone(), Heading::Subtitle));
        }
    }
    roles
}

/// Les titres de `items` : ceux des placeholders s'il y a un titre, sinon
/// ceux que déduit [`infer_roles`] ; une couverture déduite est un titre
/// d'au moins 40 pt sans autre texte d'au moins 20 pt.
pub(crate) fn headings<'a>(items: &[&'a Item], page: Size) -> Headings<'a> {
    let role = |r: Role| items.iter().copied().find(|i| i.role == Some(r) && i.has_text());
    if let Some(title) = role(Role::Title) {
        return Headings { title: Some(title), subtitle: role(Role::Subtitle), kicker: None, cover: false };
    }
    let roles = infer_roles(items, page);
    let find = |h: Heading| {
        roles.iter().find(|(_, r)| *r == h).and_then(|(id, _)| items.iter().copied().find(|i| &i.id == id))
    };
    let mut heads = Headings {
        title: find(Heading::Title),
        subtitle: find(Heading::Subtitle),
        kicker: find(Heading::Kicker),
        cover: false,
    };
    heads.cover = heads.title.is_some_and(|t| size_of(t) >= COVER_TITLE)
        && !items.iter().any(|i| !heads.holds(i) && size_of(i) >= TITLE_MIN);
    heads
}

#[cfg(test)]
#[path = "heading_tests.rs"]
mod heading_tests;
