//! Choix du profil de layout d'une slide : ce qu'offre chaque profil des packs
//! cibles, forme de la slide source, score.
//!
//! Un profil déclare ses indices (rôle, agencement, nombre de blocs, titre,
//! surtitre) ; la grille d'un profil `grid` se résout sur une slide de son
//! layout portant ses paramètres, et la présence d'un slot vient du schéma
//! du layout. La forme d'une slide source vient du type de son layout, de ses
//! titres (`heading`) et de ses unités de contenu (cartes regroupées,
//! `cluster`). Le score (voir [`score`]) est déterministe ; à égalité, l'ordre
//! des packs cibles, puis celui de leur manifeste.

use oku_slides_core::{
    Arrangement, Grid, LayoutProfile, Namespace, ParamType, ParamValue, Params, Slide, SlideRole, WidgetRegistry,
    qualify,
};
use serde_json::Value;

use super::cluster::{Unit, bounds};
use super::heading::Headings;
use crate::source::{Frame, Item, ItemKind, Layout, LayoutKind, Role, SourceSlide};

/// Ce qu'offre un profil de layout.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Profile<'p> {
    /// Le profil et l'espace de noms de son pack.
    pub source: &'p (Namespace, LayoutProfile),
    /// Rôle de slide déclaré.
    pub role: SlideRole,
    /// Agencement déclaré.
    pub arrangement: Arrangement,
    /// Grille, pour l'agencement `Grid`.
    pub grid: Option<Grid>,
    /// Nombre de blocs (ou d'éléments libres) déclaré.
    pub blocks: usize,
    /// Le profil porte un titre.
    pub has_title: bool,
    /// Le profil porte un surtitre.
    pub has_kicker: bool,
    /// Le layout du profil déclare au moins un slot.
    pub has_slot: bool,
}

impl Profile<'_> {
    /// Le kind de la slide du profil : `<namespace>/<layout>`.
    pub(crate) fn kind(&self) -> String {
        qualify(&self.source.0, self.source.1.layout.as_str())
    }

    /// Le nom qualifié du profil : `<namespace>/<id>`.
    pub(crate) fn name(&self) -> String {
        qualify(&self.source.0, &self.source.1.id)
    }
}

/// Les paramètres que le profil `p` pose sur sa slide.
///
/// Une valeur se lit comme le cœur lit celle d'une op ; une valeur que ce
/// typage refuse reste brute (`ParamValue::Value`) : le cœur la contrôle
/// contre le schéma du layout à l'application, rien n'est caché.
pub(crate) fn slide_params(p: &LayoutProfile) -> Params {
    let typed = |v: &Value| serde_json::from_value(v.clone()).unwrap_or_else(|_| ParamValue::Value(v.clone()));
    Params(p.params.iter().map(|(k, v)| (k.clone(), typed(v))).collect())
}

/// Le profil `source`, son slot lu dans le schéma de son layout.
pub(crate) fn profile<'p>(source: &'p (Namespace, LayoutProfile), registry: &WidgetRegistry) -> Profile<'p> {
    let (ns, p) = source;
    let kind = qualify(ns, p.layout.as_str());
    let has_slot = registry.get(&kind).is_some_and(|w| w.params.iter().any(|q| matches!(q.ty, ParamType::Slot { .. })));
    let grid = (p.arrangement == Arrangement::Grid).then(|| {
        let mut slide = Slide::new(p.id.as_str(), &kind);
        slide.params = slide_params(p);
        slide.resolved_grid(registry)
    });
    Profile {
        source,
        role: p.role,
        arrangement: p.arrangement,
        grid,
        blocks: usize::from(p.blocks),
        has_title: p.title,
        has_kicker: p.kicker,
        has_slot,
    }
}

/// Les profils de `list` (ceux des packs cibles, dans l'ordre), tels quels.
pub(crate) fn profiles<'p>(list: &'p [(Namespace, LayoutProfile)], registry: &WidgetRegistry) -> Vec<Profile<'p>> {
    list.iter().map(|s| profile(s, registry)).collect()
}

/// Sorte d'un contenu de slide source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContentKind {
    /// Texte (boîte de texte ou forme avec texte).
    Text,
    /// Image (ou intraduisible avec image de repli).
    Picture,
    /// Tableau.
    Table,
    /// Graphique.
    Chart,
    /// Groupe ou SmartArt dessiné.
    Group,
    /// Forme sans texte.
    Shape,
    /// Carte : forme remplie et textes posés dessus.
    Card,
}

/// La forme d'une slide source.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Shape {
    /// Type du layout.
    pub layout_kind: Option<LayoutKind>,
    /// Un titre non vide.
    pub title: bool,
    /// Un sous-titre non vide.
    pub subtitle: bool,
    /// Les unités de contenu (hors titres, date, pied, numéro), dans l'ordre de dessin.
    pub content: Vec<ContentKind>,
    /// Unités côte à côte : leur nombre si elles forment une rangée de colonnes
    /// (seules, ou entre des unités pleine largeur au-dessus et au-dessous), sinon 1 (0 sans contenu).
    pub columns: usize,
    /// Les rangs (dans `content`) des unités de la rangée de colonnes ; vide sans rangée.
    pub row: Vec<usize>,
}

/// La sorte de contenu d'un item, `None` s'il ne donne rien.
fn content_kind(item: &Item) -> Option<ContentKind> {
    match &item.kind {
        ItemKind::Text(_) => item.has_text().then_some(ContentKind::Text),
        ItemKind::Shape(_) if item.has_text() => Some(ContentKind::Text),
        ItemKind::Shape(_) => Some(ContentKind::Shape),
        ItemKind::Picture(_) => Some(ContentKind::Picture),
        ItemKind::Table(_) => Some(ContentKind::Table),
        ItemKind::Chart(_) => Some(ContentKind::Chart),
        ItemKind::Group { .. } | ItemKind::SmartArt(_) => Some(ContentKind::Group),
        ItemKind::Opaque(o) => o.fallback.as_ref().map(|_| ContentKind::Picture),
    }
}

/// La sorte de contenu d'une unité.
fn unit_kind(unit: &Unit<'_>) -> ContentKind {
    match unit {
        Unit::Card { .. } => ContentKind::Card,
        Unit::Text(_) => ContentKind::Text,
        Unit::Item(item) => content_kind(item).unwrap_or(ContentKind::Shape),
    }
}

/// Les items de contenu parmi `items` : ni titres (`heads`), ni placeholders de
/// titre, sous-titre, date, pied ou numéro de slide, et porteurs de quelque chose.
pub(crate) fn content_items<'a>(items: &[&'a Item], heads: &Headings<'_>) -> Vec<&'a Item> {
    items
        .iter()
        .copied()
        .filter(|i| !matches!(i.role, Some(Role::Title | Role::Subtitle | Role::Date | Role::Footer | Role::SlideNum)))
        .filter(|i| !heads.holds(i) && content_kind(i).is_some())
        .collect()
}

/// Les boîtes forment-elles des colonnes ? Triées par x : aucune ne chevauche
/// la suivante en largeur (à 5 % de la plus étroite près), hauteurs à 25 %
/// près, et chaque paire se recouvre en hauteur d'au moins la moitié de la plus basse.
fn side_by_side(frames: &[Frame]) -> bool {
    let mut frames = frames.to_vec();
    frames.sort_by(|a, b| a.x.total_cmp(&b.x));
    let narrowest = frames.iter().map(|f| f.w).fold(f64::INFINITY, f64::min);
    let (low, high) = frames.iter().fold((f64::INFINITY, 0.0_f64), |(lo, hi), f| (lo.min(f.h), hi.max(f.h)));
    let heights = high > 0.0 && low / high >= 0.75;
    heights
        && frames.windows(2).all(|w| {
            let overlap = (w[0].y + w[0].h).min(w[1].y + w[1].h) - w[0].y.max(w[1].y);
            w[1].x >= w[0].x + w[0].w - 0.05 * narrowest && overlap >= 0.5 * w[0].h.min(w[1].h)
        })
}

/// Part minimale de la largeur de la rangée qu'occupe une unité pleine largeur.
const WIDE: f64 = 0.6;

/// La rangée de colonnes de `frames` : la plus grande suite (au moins deux)
/// de boîtes côte à côte, prise parmi celles qui recouvrent en hauteur une
/// boîte donnée et dont la hauteur atteint 75 % de la plus haute d'entre
/// elles ; les autres boîtes doivent être entièrement au-dessus ou au-dessous
/// de la rangée et au moins aussi larges que 60 % d'elle. Vide sinon.
fn row(frames: &[Frame]) -> Vec<usize> {
    let mut best: Vec<usize> = Vec::new();
    for f in frames {
        let band: Vec<usize> = (0..frames.len())
            .filter(|k| {
                let g = &frames[*k];
                (f.y + f.h).min(g.y + g.h) - f.y.max(g.y) >= 0.5 * f.h.min(g.h)
            })
            .collect();
        let tallest = band.iter().map(|k| frames[*k].h).fold(0.0, f64::max);
        let tall: Vec<usize> = band.into_iter().filter(|k| frames[*k].h >= 0.75 * tallest).collect();
        let boxes: Vec<Frame> = tall.iter().map(|k| frames[*k]).collect();
        if tall.len() >= 2 && tall.len() > best.len() && side_by_side(&boxes) {
            best = tall;
        }
    }
    let span = bounds(best.iter().map(|k| &frames[*k]));
    let fits = (0..frames.len()).filter(|k| !best.contains(k)).all(|k| {
        let g = &frames[k];
        let apart = g.y + g.h <= span.y + 1.0 || g.y >= span.y + span.h - 1.0;
        apart && g.w >= WIDE * span.w
    });
    if fits { best } else { Vec::new() }
}

/// La forme de `slide` (layout cherché dans `layouts`), de titres `heads` et d'unités `units`.
pub(crate) fn shape(slide: &SourceSlide, layouts: &[Layout], heads: &Headings<'_>, units: &[Unit<'_>]) -> Shape {
    let layout_kind = slide.layout.as_ref().and_then(|id| layouts.iter().find(|l| &l.id == id)).map(|l| l.kind.clone());
    let frames: Vec<Frame> = units.iter().map(Unit::frame).collect();
    let row = row(&frames);
    let columns = match units.len() {
        0 => 0,
        _ if row.len() >= 2 => row.len(),
        _ => 1,
    };
    Shape {
        layout_kind,
        title: heads.title.is_some(),
        subtitle: heads.subtitle.is_some(),
        content: units.iter().map(unit_kind).collect(),
        columns,
        row,
    }
}

/// Ce que la slide source demande.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Want {
    /// Couverture.
    Title,
    /// Chapitre.
    Chapter,
    /// Titre seul.
    Empty,
    /// Un contenu.
    Single,
    /// `n` contenus en colonnes.
    Columns(usize),
    /// Plusieurs contenus disposés librement.
    Many,
}

fn want(shape: &Shape) -> Want {
    match shape.layout_kind {
        Some(LayoutKind::Title) => return Want::Title,
        Some(LayoutKind::SectionHeader) => return Want::Chapter,
        _ => {}
    }
    match shape.content.len() {
        0 if shape.subtitle => Want::Title,
        0 => Want::Empty,
        1 => Want::Single,
        _ if shape.columns >= 2 => Want::Columns(shape.columns),
        _ => Want::Many,
    }
}

/// Score de base d'un agencement de contenu pour `want` (table de [`score`]).
#[expect(clippy::match_same_arms, reason = "score table, one line per pair: pattern order decides there")]
fn base(want: Want, layout: Arrangement) -> i32 {
    use Arrangement as L;
    match (want, layout) {
        (Want::Title | Want::Chapter, _) => 10,
        (Want::Empty, L::Grid) => 100,
        (Want::Empty, L::Default) => 80,
        (Want::Empty, L::Free) => 40,
        (Want::Single, L::Default) => 100,
        (Want::Single, L::Grid) => 60,
        (Want::Single, L::Free) => 50,
        (Want::Single, L::Center) => 40,
        (Want::Single, L::Quote) => 30,
        (Want::Single, L::Split | L::Columns) => 20,
        (Want::Columns(2), L::Split) => 100,
        (Want::Columns(2), L::Columns) => 90,
        (Want::Columns(_), L::Columns) => 100,
        (Want::Columns(_), L::Grid) => 60,
        (Want::Columns(_), L::Free) => 50,
        (Want::Columns(_), L::Default) => 40,
        (Want::Columns(_), L::Split) => 20,
        (Want::Many, L::Grid) => 100,
        (Want::Many, L::Free) => 80,
        (Want::Many, L::Default) => 30,
        (Want::Many, L::Columns | L::Split) => 20,
        _ => 0,
    }
}

/// Le score de `profile` pour `shape`.
///
/// | la slide source demande | profil | score |
/// |---|---|---|
/// | couverture (layout `title`, ou sous-titre sans contenu) | `Title` / `Chapter` / contenu | 100 / 50 / 10 |
/// | chapitre (layout `secHead`) | `Chapter` / contenu | 100 / 10 |
/// | titre seul | `Grid` / `Default` / `Free` | 100 / 80 / 40 |
/// | un contenu | `Default` / `Grid` / `Free` / `Center` / `Quote` / `Split`, `Columns` | 100 / 60 / 50 / 40 / 30 / 20 |
/// | deux contenus en colonnes | `Split` / `Columns` / `Grid` / `Free` / `Default` | 100 / 90 / 60 / 50 / 40 |
/// | `n ≥ 3` contenus en colonnes | `Columns` / `Grid` / `Free` / `Default` / `Split` | 100 / 60 / 50 / 40 / 20 |
/// | plusieurs contenus disposés librement | `Grid` / `Free` / `Default` / `Columns`, `Split` | 100 / 80 / 30 / 20 |
///
/// Un profil vierge compte comme `Grid` ; `Toc` et `End` valent 0. Pour un
/// profil de contenu (ou vierge) : −15 si la présence d'un
/// titre diffère, puis − l'écart (au plus 9) entre son nombre de blocs et le
/// nombre de contenus de la slide.
pub(crate) fn score(shape: &Shape, profile: &Profile<'_>) -> i32 {
    let want = want(shape);
    let layout = match profile.role {
        SlideRole::Content => profile.arrangement,
        SlideRole::Blank => Arrangement::Grid,
        SlideRole::Title => return if want == Want::Title { 100 } else { 0 },
        SlideRole::Chapter => {
            return match want {
                Want::Chapter => 100,
                Want::Title => 50,
                _ => 0,
            };
        }
        SlideRole::Toc | SlideRole::End => return 0,
    };
    let titled = if profile.has_title == shape.title { 0 } else { -15 };
    // Capped at 9; a gap outside i32 exceeds 9 anyway.
    let gap = i32::try_from(profile.blocks.abs_diff(shape.content.len())).map_or(9, |g| g.min(9));
    base(want, layout) + titled - gap
}

/// Le profil le mieux noté pour `shape` ; à égalité, le premier dans l'ordre
/// des packs cibles puis de leur manifeste.
///
/// Si aucun n'a un score positif : le premier profil de contenu, sinon le
/// premier profil. `None` seulement si `profiles` est vide.
pub(crate) fn choose<'a, 'p>(shape: &Shape, profiles: &'a [Profile<'p>]) -> Option<&'a Profile<'p>> {
    let mut best: Option<(&Profile<'p>, i32)> = None;
    for p in profiles {
        let s = score(shape, p);
        if best.is_none_or(|(_, b)| s > b) {
            best = Some((p, s));
        }
    }
    match best {
        Some((p, s)) if s > 0 => Some(p),
        _ => profiles.iter().find(|p| p.role == SlideRole::Content).or_else(|| profiles.first()),
    }
}

#[cfg(test)]
#[path = "profile_tests.rs"]
mod profile_tests;
