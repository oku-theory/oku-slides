//! Zone de corps et grille : où vont les éléments libres d'une slide.
//!
//! Le cœur place un élément libre relativement à la zone de corps, connue
//! seulement au rendu (marqueur `<oku-slides-zone>`). Le projecteur l'approche
//! par les jetons `page`, `space` et `component.essentiel.title` du thème cible
//! ([`PageTokens`]). La géométrie de la grille est celle de `grid-frame`
//! (`typst/oku-slides/src/zone.typ`) et de `ui/src/grid.ts` ; [`snap`] porte
//! `snapRect`. Les items d'une slide entrent dans la zone ensemble ([`Fit`]) :
//! leur boîte englobante y est ajustée à échelle uniforme et centrée.
//!
//! Un jeton absent des jetons reçus prend sa valeur dans `default-tokens.json`
//! (lu ici, jamais recopié) : les jetons du thème restent la seule source.

use std::sync::LazyLock;

use oku_slides_core::tokens::{merge, theme_layer};
use oku_slides_core::{Cell, Frame, Grid, Namespace, PackFiles, PackKind, PackManifest, TokenValue};
use serde_json::{Map, Value};

use super::cluster::bounds;
use super::measure::TextTokens;
use crate::error::{ImportError, structure};
use crate::source;

/// Les manifestes des packs intégrés `base` (widgets) puis `essentiel` (layouts), source de leurs jetons de composant.
const BUILTIN_MANIFESTS: [(PackKind, &str); 2] = [
    (PackKind::Widgets, include_str!("../../../../typst/oku-slides/packs/base/widgets.json")),
    (PackKind::Layouts, include_str!("../../../../typst/oku-slides/packs/essentiel/layouts.json")),
];

/// Les défauts de composant d'un pack, avec son espace de noms.
pub(crate) type PackDefaults = (Namespace, Map<String, Value>);

/// Un jeton par défaut livré (`default-tokens.json` ou un manifeste intégré) est illisible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DefaultsFault;

/// Les défauts de composant des packs intégrés, par espace de noms (tests) ; erreur si un manifeste livré est illisible.
///
/// L'import ne dépend pas du rendu : il lit lui-même les manifestes que le rendu embarque.
#[cfg(test)]
pub(crate) fn builtin_component_defaults() -> Result<Vec<PackDefaults>, DefaultsFault> {
    parse_component_defaults(BUILTIN_MANIFESTS)
}

/// Les défauts de composant des manifestes `manifests`, par espace de noms.
fn parse_component_defaults(
    manifests: impl IntoIterator<Item = (PackKind, &'static str)>,
) -> Result<Vec<PackDefaults>, DefaultsFault> {
    manifests
        .into_iter()
        .map(|(kind, json)| {
            let files = PackFiles::from_bytes(kind, json.as_bytes(), []).ok_or(DefaultsFault)?;
            let manifest = PackManifest::from_files(&files).map_err(|_| DefaultsFault)?;
            Ok((manifest.namespace.clone(), manifest.component_defaults().map_err(|_| DefaultsFault)?))
        })
        .collect()
}

/// Les jetons de `default-tokens.json`, sans les jetons de composant des packs.
#[cfg(test)]
pub(crate) fn render_defaults() -> Result<Map<String, Value>, DefaultsFault> {
    parse_render_defaults(include_str!("../../../../typst/oku-slides/src/default-tokens.json"))
}

/// Les jetons du JSON `json`, un objet.
fn parse_render_defaults(json: &str) -> Result<Map<String, Value>, DefaultsFault> {
    match serde_json::from_str(json) {
        Ok(Value::Object(map)) => Ok(map),
        _ => Err(DefaultsFault),
    }
}

/// Les jetons effectifs sans thème : les défauts du rendu `tokens`, puis les jetons de composant de `manifests`.
fn parse_defaults(
    tokens: &str,
    manifests: impl IntoIterator<Item = (PackKind, &'static str)>,
) -> Result<Map<String, Value>, DefaultsFault> {
    let mut defaults = parse_render_defaults(tokens)?;
    let packs = parse_component_defaults(manifests)?;
    merge(&mut defaults, &theme_layer(packs.iter().map(|(ns, d)| (ns, d.clone())), []));
    Ok(defaults)
}

/// Les défauts livrés, lus une fois.
static DEFAULTS: LazyLock<Result<Map<String, Value>, DefaultsFault>> = LazyLock::new(|| {
    parse_defaults(include_str!("../../../../typst/oku-slides/src/default-tokens.json"), BUILTIN_MANIFESTS)
});

/// Vérifie que les défauts livrés se lisent : le retrofit échoue ici plutôt que de mesurer avec des zéros.
pub(crate) fn check_defaults() -> Result<(), ImportError> {
    DEFAULTS.as_ref().map(|_| ()).map_err(|_| structure("built-in token defaults"))
}

/// Les jetons par défaut du rendu : `default-tokens.json` et les défauts de composant des packs intégrés.
///
/// Vide si la lecture a échoué : [`check_defaults`], appelée par le retrofit avant toute mesure, l'aura refusé.
pub(crate) fn defaults() -> &'static Map<String, Value> {
    static EMPTY: LazyLock<Map<String, Value>> = LazyLock::new(Map::new);
    DEFAULTS.as_ref().unwrap_or(&EMPTY)
}

/// Plus petit côté d'un cadre projeté, en pt : le cœur refuse un cadre vide.
const MIN_SIDE: f64 = 1.0;

/// Les jetons de page du thème cible, en pt sur la page 1920 × 1080.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageTokens {
    /// Largeur de la page (`page.width`).
    pub width: f64,
    /// Hauteur de la page (`page.height`).
    pub height: f64,
    /// Marge gauche et droite (`page.margin-x`).
    pub margin_x: f64,
    /// Marge haute (`page.margin-top`).
    pub margin_top: f64,
    /// Marge basse (`page.margin-bottom`).
    pub margin_bottom: f64,
    /// Hauteur approchée de la ligne de titre et de sa gouttière.
    pub title_band: f64,
    /// Gouttière de grille par défaut (`space.lg`).
    pub gap: f64,
    /// Typographie et espacements du corps, pour estimer la hauteur d'un flux.
    pub text: TextTokens,
}

impl Default for PageTokens {
    /// Les jetons de `default-tokens.json` : titre 60 pt, gouttière `lg` = 40 pt.
    fn default() -> Self {
        Self::from_tokens(defaults())
    }
}

/// La valeur `path` de `tokens` (`["page", "width"]`), si c'est un nombre.
pub(crate) fn number(tokens: &Map<String, Value>, path: &[&str]) -> Option<f64> {
    let (last, groups) = path.split_last()?;
    let mut map = tokens;
    for g in groups {
        map = map.get(*g)?.as_object()?;
    }
    map.get(*last)?.as_f64()
}

/// Une valeur de jeton : un nombre, ou le nom d'un jeton de la famille `family`.
pub(crate) fn token(tokens: &Map<String, Value>, family: &str, v: Option<&Value>) -> Option<f64> {
    match v? {
        Value::Number(n) => n.as_f64(),
        Value::String(name) => number(tokens, &[family, name]),
        _ => None,
    }
}

/// La valeur `path` de `tokens`, sinon des jetons par défaut ; 0 si elle
/// manque aux deux (jamais : `default_tokens_hold_every_read_path`).
pub(crate) fn number_or_default(tokens: &Map<String, Value>, path: &[&str]) -> f64 {
    number(tokens, path).or_else(|| number(defaults(), path)).unwrap_or(0.0)
}

/// La valeur `path` de `tokens` (un nombre ou le nom d'un jeton de `family`),
/// sinon celle des jetons par défaut ; 0 si elle manque aux deux.
pub(crate) fn token_or_default(tokens: &Map<String, Value>, family: &str, path: &[&str]) -> f64 {
    let at = |t: &Map<String, Value>| {
        let (last, groups) = path.split_last()?;
        let mut map = t;
        for g in groups {
            map = map.get(*g)?.as_object()?;
        }
        token(t, family, map.get(*last))
    };
    at(tokens).or_else(|| at(defaults())).unwrap_or(0.0)
}

impl PageTokens {
    /// Les jetons de page des jetons effectifs `tokens` d'un thème (défauts
    /// fusionnés avec ceux du thème) ; un jeton absent ou illisible garde la
    /// valeur de [`PageTokens::default`].
    ///
    /// `title_band` approche la ligne de titre d'une slide de contenu : une
    /// ligne de `component.essentiel.title.size` (une hauteur d'em, un peu plus que la
    /// ligne réelle de Typst, pour ne pas déborder), le filet
    /// `component.essentiel.title.rule` s'il existe (sa hauteur et l'espace `sm` qui le
    /// sépare du titre), puis la gouttière `component.essentiel.title.gap`. Le surtitre
    /// n'est pas compté : le retrofit n'en émet pas.
    pub fn from_tokens(tokens: &Map<String, Value>) -> Self {
        let get = |path: &[&str]| number_or_default(tokens, path);
        let title = tokens
            .get("component")
            .and_then(|c| c.get("essentiel"))
            .and_then(|e| e.get("title"))
            .and_then(Value::as_object);
        let field = |name: &str| title.and_then(|t| t.get(name));
        let size = token(tokens, "size", field("size")).unwrap_or_else(|| get(&["size", "title"]));
        let gap = get(&["space", "lg"]);
        let title_gap = token(tokens, "space", field("gap")).unwrap_or(gap);
        let rule = match field("rule") {
            Some(Value::Object(r)) => r.get("height").and_then(Value::as_f64).unwrap_or(0.0) + get(&["space", "sm"]),
            _ => 0.0,
        };
        Self {
            width: get(&["page", "width"]),
            height: get(&["page", "height"]),
            margin_x: get(&["page", "margin-x"]),
            margin_top: get(&["page", "margin-top"]),
            margin_bottom: get(&["page", "margin-bottom"]),
            title_band: size + rule + title_gap,
            gap,
            text: TextTokens::from_tokens(tokens),
        }
    }
}

/// Un rectangle en pt sur la page 1920 × 1080.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Zone {
    /// Bord gauche.
    pub x: f64,
    /// Bord haut.
    pub y: f64,
    /// Largeur.
    pub w: f64,
    /// Hauteur.
    pub h: f64,
}

/// La zone de corps d'une slide de contenu, en coordonnées de page : la page
/// moins ses marges, moins la ligne de titre si la slide en a une.
pub(crate) fn body_zone(page: &PageTokens, titled: bool) -> Zone {
    let band = if titled { page.title_band } else { 0.0 };
    Zone {
        x: page.margin_x,
        y: page.margin_top + band,
        w: (page.width - 2.0 * page.margin_x).max(MIN_SIDE),
        h: (page.height - page.margin_top - page.margin_bottom - band).max(MIN_SIDE),
    }
}

/// La gouttière de `grid` en pt : un nombre tel quel, sinon `page.gap`.
///
/// Un nom de jeton autre que `lg` est approché par `page.gap` : aucun profil
/// intégré n'en déclare, et le projecteur n'émet jamais de gouttière.
pub(crate) fn grid_gap(grid: &Grid, page: &PageTokens) -> f64 {
    match &grid.gap {
        Some(TokenValue::Number(n)) => *n,
        Some(TokenValue::Name(_)) | None => page.gap,
    }
}

/// Largeur d'une colonne et hauteur d'une ligne de `grid` dans `zone`.
pub(crate) fn track_size(zone: &Zone, grid: &Grid, gap: f64) -> (f64, f64) {
    let (columns, rows) = (f64::from(grid.columns.max(1)), f64::from(grid.rows.max(1)));
    ((zone.w - (columns - 1.0) * gap) / columns, (zone.h - (rows - 1.0) * gap) / rows)
}

/// Le cadre de `cell`, relatif à `zone` (même calcul que `grid-frame`).
///
/// Sert à juger si une grille tient, et d'oracle aux tests (une cellule
/// accrochée retombe sur les pistes de la grille rendue).
pub(crate) fn cell_rect(zone: &Zone, grid: &Grid, gap: f64, cell: &Cell) -> Frame {
    let (cw, rh) = track_size(zone, grid, gap);
    let (cols, rows) = (f64::from(cell.cols), f64::from(cell.rows));
    Frame {
        x: f64::from(cell.col) * (cw + gap),
        y: f64::from(cell.row) * (rh + gap),
        w: cols * cw + (cols - 1.0) * gap,
        h: rows * rh + (rows - 1.0) * gap,
    }
}

/// L'indice du bord de `edges` le plus proche de `v` ; à égalité, le premier.
fn nearest(edges: impl Iterator<Item = f64>, v: f64) -> u32 {
    let mut best = (0u32, f64::INFINITY);
    for (i, e) in (0u32..).zip(edges) {
        if (e - v).abs() < best.1 {
            best = (i, (e - v).abs());
        }
    }
    best.0
}

/// La cellule de `grid` la plus proche du cadre `r` (relatif à `zone`) :
/// colonne et ligne au début de piste le plus proche, étendue à la fin de
/// piste la plus proche, une case au moins. Portage de `snapRect`.
pub(crate) fn snap(zone: &Zone, grid: &Grid, gap: f64, r: Frame) -> Cell {
    let (cw, rh) = track_size(zone, grid, gap);
    let starts = |n: u32, size: f64| (0..n.max(1)).map(move |i| f64::from(i) * (size + gap));
    let col = nearest(starts(grid.columns, cw), r.x);
    let row = nearest(starts(grid.rows, rh), r.y);
    let end_col = nearest(starts(grid.columns, cw).map(|s| s + cw), r.x + r.w).max(col);
    let end_row = nearest(starts(grid.rows, rh).map(|s| s + rh), r.y + r.h).max(row);
    Cell { col, row, cols: end_col - col + 1, rows: end_row - row + 1 }
}

/// Passage des pt source aux pt relatifs à la zone : `x × scale + dx`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Fit {
    /// Échelle uniforme.
    pub scale: f64,
    /// Décalage horizontal.
    pub dx: f64,
    /// Décalage vertical.
    pub dy: f64,
}

impl Fit {
    /// La boîte englobante de `frames` (pt source) ajustée dans `zone` : à
    /// l'échelle de page `scale` si elle y tient, sinon réduite pour y tenir
    /// (échelle uniforme), et centrée. Sans cadre : l'échelle de page, la page
    /// source posée sur la page Oku.
    pub(crate) fn new<'f>(frames: impl IntoIterator<Item = &'f source::Frame>, scale: f64, zone: &Zone) -> Self {
        let mut frames = frames.into_iter().peekable();
        if frames.peek().is_none() {
            return Self { scale, dx: -zone.x, dy: -zone.y };
        }
        let source::Frame { x: x0, y: y0, w, h, .. } = bounds(frames);
        let mut s = scale;
        if w * s > zone.w && w > 0.0 {
            s = zone.w / w;
        }
        if h * s > zone.h && h > 0.0 {
            s = zone.h / h;
        }
        Self { scale: s, dx: (zone.w - w * s) / 2.0 - x0 * s, dy: (zone.h - h * s) / 2.0 - y0 * s }
    }
}

/// `[a, b]` ramené dans `[0, len]`, d'au moins [`MIN_SIDE`] de long.
fn clamp_span(a: f64, b: f64, len: f64) -> (f64, f64) {
    let lo = a.clamp(0.0, len);
    let hi = b.clamp(0.0, len);
    if hi - lo >= MIN_SIDE {
        return (lo, hi);
    }
    let hi = (lo + MIN_SIDE).min(len.max(MIN_SIDE));
    (hi - MIN_SIDE, hi)
}

/// `v` arrondi au centième de pt : des cadres lisibles, sans écart visible.
fn hundredth(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// Le cadre source `item` (pt de page source) relatif à `zone` par `fit`, et
/// ramené dedans (filet de sûreté : un cadre ajusté y est déjà), au centième de pt.
pub(crate) fn to_zone(item: &source::Frame, fit: &Fit, zone: &Zone) -> Frame {
    let (x0, x1) = clamp_span(item.x * fit.scale + fit.dx, (item.x + item.w) * fit.scale + fit.dx, zone.w);
    let (y0, y1) = clamp_span(item.y * fit.scale + fit.dy, (item.y + item.h) * fit.scale + fit.dy, zone.h);
    let (x0, x1, y0, y1) = (hundredth(x0), hundredth(x1), hundredth(y0), hundredth(y1));
    Frame { x: x0, y: y0, w: hundredth(x1 - x0), h: hundredth(y1 - y0) }
}

#[cfg(test)]
#[path = "zone_tests.rs"]
mod zone_tests;
