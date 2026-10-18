//! Écart visuel d'une slide hi-fi : son rendu Oku contre le rendu de
//! référence d'un témoin, à la même taille (spec, « Rendu de référence et
//! métriques »).
//!
//! Les deux images passent en OKLab et sont adoucies par un flou gaussien
//! ([`BLUR_SIGMA`] pixels : l'anticrénelage et le pixel de décalage d'un
//! glyphe comptent peu ; à σ = 1, les titres d'alto, alignés à 1 px près,
//! passaient devant les vrais écarts) ; l'écart d'un pixel
//! est le ΔE OKLab (×100, celui du module `oklab`). Le score d'une slide est
//! l'écart moyen sur la page.
//!
//! Le score d'une région désigne l'élément fautif ; il est calibré sur alto
//! (p. 7 : la ligne coupée de l'encadré ; p. 8 : le retrait des puces) :
//! - chaque pixel appartient à l'item le plus haut qui le couvre (ordre de
//!   dessin de [`regions`]) : une carte ne porte pas l'écart de son texte ;
//! - la région est recalée sur la référence (le décalage entier, au plus
//!   [`MAX_SHIFT`] pixels, qui minimise son écart moyen ; il est rapporté) :
//!   un décalage de quelques pixels, qui se corrige seul, ne masque pas une
//!   coupure ou un retrait ;
//! - son score est l'écart moyen de sa pire tuile de [`TILE`] pixels : un mot
//!   déplacé dans un grand cadre presque vide compte pleinement ;
//! - une région de moins de [`MIN_PIXELS`] pixels à elle n'a pas de score
//!   (ses pixels sont surtout des bords anticrénelés) ; un cadre de taille
//!   négative (le lecteur la garde) n'en a aucun ;
//! - au plus [`MAX_REGIONS`] régions comptent, les plus hautes : le coût reste
//!   borné quel que soit le nombre d'items d'une slide.
//!
//! Une région est suspecte ([`WitnessLimit`]) quand son item est de ceux que
//! le témoin ([`Witness`]) dessine faux, ses limites connues lues dans la
//! source : sous office2pdf, texte qu'il ajuste lui-même à sa boîte, texte
//! interlettré, graphique, SmartArt, police des seuls défauts ; sous
//! LibreOffice, police qu'il ne sait pas remplacer, texte des SmartArt. Son
//! écart est surtout celui du témoin, pas celui d'Oku. Sans témoin, aucune.
//!
//! Une slide est signalée par sa pire région non suspecte
//! ([`SlideScore::worst`]) : la moyenne de la page, dominée par les fonds
//! identiques, ne distingue pas une slide fidèle d'une slide abîmée.
//!
//! [`score`] classe toutes les régions notées : les non suspectes, les pires
//! d'abord, puis les suspectes, les pires d'abord ; le rapport écrit les
//! [`REPORTED_REGIONS`] premières. Pur : les images viennent de l'appelant.

use std::collections::HashMap;

use serde::{Serialize, Serializer};

use crate::oklab::{Oklab, delta_e, oklab};
use crate::source::{Item, ItemKind, Rgba, SourceSlide};

mod witness;

use witness::witness_limit;
pub use witness::{Witness, WitnessLimit};

/// Écart-type du flou gaussien, en pixels (à 960 px de large).
pub const BLUR_SIGMA: f64 = 2.0;

/// Décalage maximal, en pixels et par axe, du recalage d'une région.
pub const MAX_SHIFT: i64 = 3;

/// Un pixel propre sur `ALIGN_STEP` sert à chercher le recalage d'une région.
const ALIGN_STEP: usize = 4;

/// Côté d'une tuile, en pixels.
pub const TILE: usize = 16;

/// Pixels propres en deçà desquels une région n'a pas de score.
pub const MIN_PIXELS: usize = 100;

/// Régions comptées par slide, au plus : les dernières dessinées, au-dessus.
pub const MAX_REGIONS: usize = 256;

/// Nombre de régions écrites au rapport par slide, dans l'ordre du classement.
pub const REPORTED_REGIONS: usize = 3;

/// Une image RGBA 8 bits empruntée, ligne à ligne depuis le haut ; l'opacité est ignorée.
#[derive(Debug, Clone, Copy)]
pub struct Image<'a> {
    /// Largeur, en pixels.
    pub width: u32,
    /// Hauteur, en pixels.
    pub height: u32,
    /// Octets RGBA (`width × height × 4`).
    pub rgba: &'a [u8],
}

/// Un rectangle de l'image, en pixels, attaché à un item source.
#[derive(Debug, Clone, PartialEq)]
pub struct Region {
    /// Id de l'item source.
    pub item: String,
    /// Bord gauche.
    pub x: f64,
    /// Bord haut.
    pub y: f64,
    /// Largeur.
    pub w: f64,
    /// Hauteur.
    pub h: f64,
    /// La limite du témoin sur cet item ; `None` : son écart est celui d'Oku.
    pub suspect: Option<WitnessLimit>,
}

/// L'écart d'une région.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RegionScore {
    /// Id de l'item source.
    pub item: String,
    /// ΔE OKLab moyen de sa pire tuile, une fois recalée, arrondi au centième.
    pub score: f64,
    /// Le recalage retenu, en pixels (`[dx, dy]` : où le rendu Oku la dessine
    /// par rapport à la référence).
    pub shift: [i64; 2],
    /// La limite du témoin sur son item : la région ne signale pas la slide.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suspect: Option<WitnessLimit>,
}

/// L'écart visuel d'une slide.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SlideScore {
    /// ΔE OKLab moyen sur la page, arrondi au centième.
    pub score: f64,
    /// Toutes les régions notées, classées (non suspectes puis suspectes, les pires d'abord).
    ///
    /// Seules les [`REPORTED_REGIONS`] premières s'écrivent.
    #[serde(serialize_with = "reported")]
    pub regions: Vec<RegionScore>,
}

impl SlideScore {
    /// Le score de sa pire région non suspecte ; `None` sans elle.
    pub fn worst(&self) -> Option<f64> {
        self.regions.first().filter(|r| r.suspect.is_none()).map(|r| r.score)
    }
}

/// Les [`REPORTED_REGIONS`] premières régions, la forme écrite du rapport.
fn reported<S: Serializer>(regions: &[RegionScore], s: S) -> Result<S::Ok, S::Error> {
    s.collect_seq(regions.iter().take(REPORTED_REGIONS))
}

/// Le cadre de chaque item de `slide`, groupes ouverts, à `scale` px par pt.
///
/// Seuls les enfants d'un groupe comptent ; l'ordre est celui du dessin (la
/// dernière région est au-dessus). Chaque région porte la limite de `witness`
/// sur son item ; les enfants d'un SmartArt dessiné héritent de la sienne.
/// Sans témoin (`None`), aucune n'est suspecte.
pub fn regions(slide: &SourceSlide, scale: f64, witness: Option<Witness>) -> Vec<Region> {
    fn walk(items: &[Item], scale: f64, witness: Option<Witness>, in_smartart: bool, out: &mut Vec<Region>) {
        for item in items {
            match &item.kind {
                ItemKind::Group { items, smartart } => {
                    walk(items, scale, witness, in_smartart || smartart.is_some(), out);
                }
                _ => {
                    let f = item.frame;
                    out.push(Region {
                        item: item.id.clone(),
                        x: f.x * scale,
                        y: f.y * scale,
                        w: f.w * scale,
                        h: f.h * scale,
                        suspect: witness.and_then(|w| witness_limit(item, in_smartart, w)),
                    });
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(&slide.items, scale, witness, false, &mut out);
    out
}

/// L'écart de `oku` à `reference`, et toutes ses régions notées, classées.
///
/// `None` si les images n'ont pas la même taille, sont vides ou que leurs
/// octets ne la font pas.
pub fn score(reference: Image<'_>, oku: Image<'_>, regions: &[Region]) -> Option<SlideScore> {
    let (w, h) = (reference.width as usize, reference.height as usize);
    let valid = |i: Image<'_>| i.rgba.len() == w * h * 4;
    let same = (oku.width, oku.height) == (reference.width, reference.height);
    if !same || w == 0 || h == 0 || !valid(reference) || !valid(oku) {
        return None;
    }
    let pair = Pair { a: blurred(reference), b: blurred(oku), w, h };
    let mean = (0..w * h).map(|i| delta_e(pair.a[i], pair.b[i])).sum::<f64>() / (w * h) as f64;
    let regions = &regions[regions.len().saturating_sub(MAX_REGIONS)..];
    let mut scored: Vec<RegionScore> = ownership(regions, w, h)
        .iter()
        .zip(regions)
        .filter_map(|(pixels, r)| {
            let (score, shift) = pair.region(pixels, bounds(r, w, h))?;
            Some(RegionScore { item: r.item.clone(), score: round2(score), shift, suspect: r.suspect })
        })
        .collect();
    // Les suspectes après les autres ; à rang égal, la pire, puis l'id.
    scored.sort_by(|x, y| {
        (x.suspect.is_some().cmp(&y.suspect.is_some()))
            .then_with(|| y.score.total_cmp(&x.score))
            .then_with(|| x.item.cmp(&y.item))
    });
    Some(SlideScore { score: round2(mean), regions: scored })
}

/// Les pixels propres de chaque région : ceux de son cadre (borné à l'image)
/// qu'aucune région suivante ne recouvre.
fn ownership(regions: &[Region], w: usize, h: usize) -> Vec<Vec<usize>> {
    let mut owner = vec![usize::MAX; w * h];
    for (i, r) in regions.iter().enumerate() {
        let (x0, y0, x1, y1) = bounds(r, w, h);
        for y in y0..y1 {
            owner[y * w + x0..y * w + x1].fill(i);
        }
    }
    let mut pixels = vec![Vec::new(); regions.len()];
    for (p, &i) in owner.iter().enumerate() {
        if let Some(v) = pixels.get_mut(i) {
            v.push(p);
        }
    }
    pixels
}

/// Le rectangle `r` en pixels entiers, borné à l'image : `x0, y0, x1, y1`.
///
/// Un cadre de taille négative ou non finie est vide (`x1 = x0` ou `y1 = y0`).
fn bounds(r: &Region, w: usize, h: usize) -> (usize, usize, usize, usize) {
    let clamp = |v: f64, max: usize| (v.max(0.0) as usize).min(max);
    let (x0, y0) = (clamp(r.x.floor(), w), clamp(r.y.floor(), h));
    let (x1, y1) = (clamp((r.x + r.w).ceil(), w), clamp((r.y + r.h).ceil(), h));
    (x0, y0, x1.max(x0), y1.max(y0))
}

/// La référence et le rendu Oku en OKLab adouci, de même taille.
struct Pair {
    a: Vec<Oklab>,
    b: Vec<Oklab>,
    w: usize,
    h: usize,
}

impl Pair {
    /// L'écart au pixel `p` de la référence, le rendu Oku décalé de `dx, dy` (bords prolongés).
    fn at(&self, p: usize, dx: i64, dy: i64) -> f64 {
        let (x, y) = ((p % self.w) as i64, (p / self.w) as i64);
        let sx = (x + dx).clamp(0, self.w as i64 - 1) as usize;
        let sy = (y + dy).clamp(0, self.h as i64 - 1) as usize;
        delta_e(self.a[p], self.b[sy * self.w + sx])
    }

    /// Le score d'une région de pixels propres `pixels`, dans le cadre
    /// `x0, y0, x1, y1`, et son recalage ; `None` sous [`MIN_PIXELS`].
    fn region(&self, pixels: &[usize], (x0, y0, x1, y1): (usize, usize, usize, usize)) -> Option<(f64, [i64; 2])> {
        if pixels.len() < MIN_PIXELS {
            return None;
        }
        let shifts = (-MAX_SHIFT..=MAX_SHIFT).flat_map(|dy| (-MAX_SHIFT..=MAX_SHIFT).map(move |dx| (dx, dy)));
        // Le recalage se cherche sur un pixel sur ALIGN_STEP : le décalage d'un
        // dessin se voit sur tous ses pixels, et le coût de la recherche (49
        // décalages) reste celui d'un seul passage.
        let total = |(dx, dy): (i64, i64)| pixels.iter().step_by(ALIGN_STEP).map(|&p| self.at(p, dx, dy)).sum::<f64>();
        // À écart égal, le plus petit décalage (|dx| + |dy|), puis l'ordre de parcours.
        let (dx, dy) = shifts
            .map(|s| (total(s), s))
            .min_by(|x, y| {
                x.0.total_cmp(&y.0).then_with(|| (x.1.0.abs() + x.1.1.abs()).cmp(&(y.1.0.abs() + y.1.1.abs())))
            })
            .map_or((0, 0), |(_, s)| s);
        // Tuiles de TILE pixels au pas de TILE / 2 : un écart de la taille d'une
        // tuile tombe entier dans l'une d'elles, où qu'il soit.
        // Grille dense sur le cadre de la région : une case par tuile.
        let half = TILE / 2;
        let cols = (x1 - x0) / half + 1;
        let mut tiles = vec![(0.0, 0usize); cols * ((y1 - y0) / half + 1)];
        for &p in pixels {
            let (x, y) = ((p % self.w - x0) / half, (p / self.w - y0) / half);
            let d = self.at(p, dx, dy);
            for tx in x.saturating_sub(1)..=x {
                for ty in y.saturating_sub(1)..=y {
                    let t = &mut tiles[ty * cols + tx];
                    t.0 += d;
                    t.1 += 1;
                }
            }
        }
        // Une tuile au bord de la région, presque vide, ne décide pas seule ;
        // une région plus étroite qu'une tuile garde son écart moyen.
        let full = |n: usize| n * 4 >= TILE * TILE;
        let mean = || pixels.iter().map(|&p| self.at(p, dx, dy)).sum::<f64>() / pixels.len() as f64;
        let peak =
            tiles.iter().filter(|(_, n)| full(*n)).map(|(s, n)| s / *n as f64).reduce(f64::max).unwrap_or_else(mean);
        Some((peak, [dx, dy]))
    }
}

/// L'image en OKLab, adoucie par un flou gaussien séparable de [`BLUR_SIGMA`].
fn blurred(image: Image<'_>) -> Vec<Oklab> {
    let (w, h) = (image.width as usize, image.height as usize);
    // Une page n'a que peu de couleurs distinctes : chacune n'est convertie qu'une fois.
    let mut known: HashMap<[u8; 3], [f64; 3]> = HashMap::new();
    let lab: Vec<[f64; 3]> = image
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&[r, g, b, _]| {
            *known.entry([r, g, b]).or_insert_with(|| {
                let c = oklab(Rgba::new(r, g, b));
                [c.l, c.a, c.b]
            })
        })
        .collect();
    let kernel = gaussian(BLUR_SIGMA);
    let rows = convolve(&lab, w, h, &kernel, true);
    convolve(&rows, w, h, &kernel, false).into_iter().map(|[l, a, b]| Oklab { l, a, b }).collect()
}

/// Noyau gaussien normalisé de rayon `⌈3σ⌉`.
fn gaussian(sigma: f64) -> Vec<f64> {
    let radius = (3.0 * sigma).ceil() as i64;
    let k: Vec<f64> = (-radius..=radius).map(|i| (-((i * i) as f64) / (2.0 * sigma * sigma)).exp()).collect();
    let total: f64 = k.iter().sum();
    k.into_iter().map(|v| v / total).collect()
}

/// Convolution de `src` par `kernel`, en ligne (`horizontal`) ou en colonne ;
/// les bords sont prolongés.
fn convolve(src: &[[f64; 3]], w: usize, h: usize, kernel: &[f64], horizontal: bool) -> Vec<[f64; 3]> {
    let radius = (kernel.len() / 2) as i64;
    let mut out = vec![[0.0; 3]; src.len()];
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0.0; 3];
            for (k, weight) in kernel.iter().enumerate() {
                let d = k as i64 - radius;
                let (sx, sy) = if horizontal {
                    ((x as i64 + d).clamp(0, w as i64 - 1) as usize, y)
                } else {
                    (x, (y as i64 + d).clamp(0, h as i64 - 1) as usize)
                };
                let p = src[sy * w + sx];
                for c in 0..3 {
                    acc[c] += weight * p[c];
                }
            }
            out[y * w + x] = acc;
        }
    }
    out
}

/// Arrondi au centième.
fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
#[path = "metrics_tests.rs"]
mod metrics_tests;
