//! Choix du modèle d'une slide : profil de chaque modèle du pack, forme de la
//! slide source, score.
//!
//! Un modèle ne déclare ni champ requis ni type de mise en page : son profil
//! se déduit de sa slide d'exemple et du schéma de son widget (sorte de
//! slide, slot `flow` rempli ou non, paramètre `grid`, choix `layout`). La
//! forme d'une slide source vient du type de son layout, de ses titres
//! (`heading`) et de ses unités de contenu (cartes regroupées, `cluster`). Le score (voir [`score`])
//! est déterministe ; à égalité, l'ordre du pack.

use oku_slides_core::{Grid, ParamType, SlotMode, Template, WidgetRegistry};

use super::cluster::{Unit, bounds};
use super::heading::Headings;
use crate::source::{Frame, Item, ItemKind, Layout, LayoutKind, Role, SourceSlide};

/// Sorte d'une slide de modèle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlideKind {
    /// Couverture (`title`) : ses champs sont ceux du deck.
    Title,
    /// Chapitre.
    Chapter,
    /// Sommaire.
    Toc,
    /// Contenu.
    Content,
    /// Fin.
    End,
    /// Vierge (`blank`) : sans titre, éléments libres sur la grille.
    Blank,
    /// Widget de slide d'un pack, sans slot.
    Other,
}

/// Agencement du contenu d'un modèle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContentLayout {
    /// Blocs empilés.
    Default,
    /// Une colonne par bloc.
    Columns,
    /// Deux colonnes.
    Split,
    /// Blocs centrés.
    Center,
    /// Citation.
    Quote,
    /// Éléments libres sans grille déclarée.
    Free,
    /// Éléments libres sur une grille.
    Grid,
}

/// Ce qu'offre un modèle.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Profile<'t> {
    /// Le modèle.
    pub template: &'t Template,
    /// Sorte de slide.
    pub kind: SlideKind,
    /// Agencement.
    pub layout: ContentLayout,
    /// Grille, pour l'agencement `Grid`.
    pub grid: Option<Grid>,
    /// Nombre de blocs (ou d'éléments libres) de l'exemple.
    pub blocks: usize,
    /// L'exemple a un titre (une couverture en a toujours un).
    pub has_title: bool,
    /// L'exemple a un surtitre.
    pub has_kicker: bool,
    /// Le widget du modèle déclare au moins un slot.
    pub has_slot: bool,
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

/// La sorte d'une slide de widget `kind`.
fn slide_kind(kind: &str, has_slot: bool) -> SlideKind {
    match kind {
        "title" => SlideKind::Title,
        "chapter" => SlideKind::Chapter,
        "toc" => SlideKind::Toc,
        "content" => SlideKind::Content,
        "end" => SlideKind::End,
        "blank" => SlideKind::Blank,
        _ if has_slot => SlideKind::Content,
        _ => SlideKind::Other,
    }
}

/// Le profil de `template`, d'après sa slide d'exemple et le schéma de son widget.
pub(crate) fn profile<'t>(template: &'t Template, registry: &WidgetRegistry) -> Profile<'t> {
    let slide = &template.slide;
    let params = registry.get(&slide.kind).map(|w| w.params.as_slice()).unwrap_or_default();
    let filled = |mode: SlotMode| -> usize {
        params
            .iter()
            .filter(|p| matches!(&p.ty, ParamType::Slot { mode: m, .. } if *m == mode))
            .map(|p| slide.params.slot(&p.name).map_or(0, <[_]>::len))
            .sum()
    };
    let (flow, free) = (filled(SlotMode::Flow), filled(SlotMode::Free));
    let has_slot = params.iter().any(|p| matches!(p.ty, ParamType::Slot { .. }));
    let kind = slide_kind(&slide.kind, has_slot);
    let grid_set = params.iter().any(|p| matches!(p.ty, ParamType::Grid) && slide.params.value(&p.name).is_some());
    let layout = if kind == SlideKind::Blank || (grid_set && flow == 0) {
        ContentLayout::Grid
    } else if free > 0 && flow == 0 {
        ContentLayout::Free
    } else {
        match slide.params.text("layout") {
            Some("columns") => ContentLayout::Columns,
            Some("split") => ContentLayout::Split,
            Some("center") => ContentLayout::Center,
            Some("quote") => ContentLayout::Quote,
            _ => ContentLayout::Default,
        }
    };
    let non_empty = |name: &str| slide.params.text(name).is_some_and(|t| !t.trim().is_empty());
    Profile {
        template,
        kind,
        layout,
        grid: (layout == ContentLayout::Grid).then(|| slide.resolved_grid(registry)),
        blocks: if flow > 0 { flow } else { free },
        has_title: kind == SlideKind::Title || non_empty("title"),
        has_kicker: non_empty("kicker"),
        has_slot,
    }
}

/// Les profils de `templates`, dans l'ordre du pack.
pub(crate) fn profiles<'t>(templates: &'t [Template], registry: &WidgetRegistry) -> Vec<Profile<'t>> {
    templates.iter().map(|t| profile(t, registry)).collect()
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
fn base(want: Want, layout: ContentLayout) -> i32 {
    use ContentLayout as L;
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
/// | la slide source demande | modèle | score |
/// |---|---|---|
/// | couverture (layout `title`, ou sous-titre sans contenu) | `Title` / `Chapter` / contenu | 100 / 50 / 10 |
/// | chapitre (layout `secHead`) | `Chapter` / contenu | 100 / 10 |
/// | titre seul | `Grid` / `Default` / `Free` | 100 / 80 / 40 |
/// | un contenu | `Default` / `Grid` / `Free` / `Center` / `Quote` / `Split`, `Columns` | 100 / 60 / 50 / 40 / 30 / 20 |
/// | deux contenus en colonnes | `Split` / `Columns` / `Grid` / `Free` / `Default` | 100 / 90 / 60 / 50 / 40 |
/// | `n ≥ 3` contenus en colonnes | `Columns` / `Grid` / `Free` / `Default` / `Split` | 100 / 60 / 50 / 40 / 20 |
/// | plusieurs contenus disposés librement | `Grid` / `Free` / `Default` / `Columns`, `Split` | 100 / 80 / 30 / 20 |
///
/// Un modèle vierge compte comme `Grid` ; `Toc`, `End` et les autres sortes
/// valent 0. Pour un modèle de contenu (ou vierge) : −15 si la présence d'un
/// titre diffère, puis − l'écart (au plus 9) entre son nombre de blocs et le
/// nombre de contenus de la slide.
pub(crate) fn score(shape: &Shape, profile: &Profile<'_>) -> i32 {
    let want = want(shape);
    let layout = match profile.kind {
        SlideKind::Content => profile.layout,
        SlideKind::Blank => ContentLayout::Grid,
        SlideKind::Title => return if want == Want::Title { 100 } else { 0 },
        SlideKind::Chapter => {
            return match want {
                Want::Chapter => 100,
                Want::Title => 50,
                _ => 0,
            };
        }
        SlideKind::Toc | SlideKind::End | SlideKind::Other => return 0,
    };
    let titled = if profile.has_title == shape.title { 0 } else { -15 };
    let gap = profile.blocks.abs_diff(shape.content.len()).min(9);
    // Borné à 9 : la conversion ne déborde pas.
    base(want, layout) + titled - gap as i32
}

/// Le profil le mieux noté pour `shape` ; à égalité, le premier dans l'ordre du pack.
///
/// Si aucun n'a un score positif : le premier modèle de contenu, sinon le
/// premier modèle. `None` seulement si `profiles` est vide.
pub(crate) fn choose<'p, 't>(shape: &Shape, profiles: &'p [Profile<'t>]) -> Option<&'p Profile<'t>> {
    let mut best: Option<(&Profile<'t>, i32)> = None;
    for p in profiles {
        let s = score(shape, p);
        if best.is_none_or(|(_, b)| s > b) {
            best = Some((p, s));
        }
    }
    match best {
        Some((p, s)) if s > 0 => Some(p),
        _ => profiles.iter().find(|p| p.kind == SlideKind::Content).or_else(|| profiles.first()),
    }
}

#[cfg(test)]
#[path = "template_tests.rs"]
mod template_tests;
