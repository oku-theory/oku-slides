//! Éléments hi-fi d'une slide : un item propre à la slide (hors placeholders
//! de texte, qui remplissent les paramètres du widget de son layout) en
//! éléments du cœur posés en `frame` (pt page) dans le slot libre `extras`,
//! dans l'ordre de dessin, avec leurs pertes.
//!
//! | Item | Éléments |
//! |---|---|
//! | texte | `text` : corps riche et style de la boîte ([`super::element_text`]) |
//! | forme `rect`, `roundRect`, `ellipse` | `shape` natif (`radius` en pt pour `roundRect`) |
//! | ligne horizontale | `shape: line` |
//! | ligne oblique ou verticale, autre géométrie | `shape: path`, contour ([`outline`]) normalisé au cadre |
//! | contour rempli ≠ contour tracé | la part remplie sans trait, puis la part tracée sans fond |
//! | forme remplie d'une image | `image` étirée au cadre, puis le trait |
//! | forme portant du texte | la forme, puis un `text` sur le même cadre (marges : zone de texte de la géométrie et marges source) |
//! | image, repli d'un opaque | `image` (`fit: stretch`) |
//! | tableau, graphique | `table`, `chart` (conversions du retrofit, couleurs des séries gardées) ; d'un tableau, `header-row` et `striped` lus sur ses cellules ([`super::table`]), perte `TableStyle` pour le reste de son style |
//! | groupe, SmartArt dessiné | ses enfants, à leur place absolue |
//! | opaque sans repli | rien, perte `Opaque` |
//!
//! Le remplissage est plein : un dégradé prend la couleur de son premier
//! arrêt (`ShapeStyle(Gradient)`) ; les tirets se perdent (`ShapeStyle(Dash)`),
//! une géométrie inconnue devient un rectangle (`ShapeStyle(Geometry)`).
//! Les miroirs d'une ligne ou d'un chemin s'écrivent dans ses coordonnées ;
//! ceux d'un rectangle ou d'une ellipse n'y changent rien, ceux d'une zone
//! de texte de forme la déplacent. Perte `Transform` : une rotation, un
//! miroir d'image, de tableau ou de graphique, un miroir vertical de texte
//! (PowerPoint le retourne), un rognage. Perte `Effects` : ombres, reflets
//! et 3D signalés par le lecteur ([`effect_items`]).

use std::collections::BTreeSet;

use oku_slides_core::{Element, Frame, ParamValue, PathCommand, STYLE_INSET_MAX, ShapePath, TokenValue};
use serde_json::{Value, json};

use super::element_text::text_element;
use super::geometry::arrow::{heads, points};
use super::geometry::{Outline, outline};
use super::page::round2;
use super::table::TableLook;
use super::typst::{GenCtx, round_rect_radius};
use crate::report::{Loss, LossKind, ShapeEffect};
use crate::retrofit::chart::chart;
use crate::retrofit::table::table;
use crate::retrofit::text::note;
use crate::source::{
    self, ChartKind, Crop, Dash, Fill, Geometry, Insets, Item, ItemKind, Rgba, Shape, Stroke, Warning, WarningKind,
};

/// Effets signalés par le lecteur (`EffectIgnored`) que le hi-fi perd ;
/// rotation et miroir de groupe se lisent dans le cadre (`Transform`).
const EFFECTS: [&str; 3] = ["effectLst", "scene3d", "sp3d"];
/// Côté minimal d'un cadre, en pt page : le cœur refuse un cadre vide, et
/// une ligne droite n'a pas d'épaisseur.
const MIN_SIDE: f64 = 1.0;
/// Zone de texte d'une géométrie sans contour calculé : son cadre entier.
const FULL: [f64; 4] = [0.0, 0.0, 1.0, 1.0];
/// Rognage imperceptible, en fraction du bord (0,1 %) : un `srcRect` dont
/// chaque bord reste sous ce seuil (PowerPoint en écrit de 0,001 % pour un
/// simple recadrage à la souris) n'est pas une perte `Transform`.
const CROP_TOLERANCE: f64 = 0.001;

/// Ce dont la conversion a besoin.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ElementCtx<'a> {
    /// Page, couleurs et polices du thème généré, assets de la source.
    pub(crate) base: GenCtx<'a>,
    /// Items dont le lecteur a ignoré des effets ([`effect_items`]).
    pub(crate) effects: &'a BTreeSet<String>,
    /// Le style des tableaux que pose le thème généré (celui du premier tableau du deck).
    pub(crate) table: Option<&'a TableLook>,
}

/// Les éléments d'un item.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Placed {
    /// Éléments, dans l'ordre de dessin.
    pub(crate) elements: Vec<Element>,
    /// Assets qu'ils dessinent : le deck doit les porter.
    pub(crate) assets: BTreeSet<String>,
    /// Pertes, chacune une fois par item.
    pub(crate) losses: Vec<Loss>,
    /// L'élément `text` de chaque item qui porte du texte (id d'item, id
    /// d'élément) : là où la couverture cherche ses runs.
    pub(crate) origins: Vec<(String, String)>,
}

/// Les items dont le lecteur a ignoré des ombres, reflets ou effets 3D.
pub(crate) fn effect_items(warnings: &[Warning]) -> BTreeSet<String> {
    warnings
        .iter()
        .filter_map(|w| match (&w.kind, &w.item) {
            (WarningKind::EffectIgnored(e), Some(item)) if EFFECTS.contains(&e.as_str()) => Some(item.clone()),
            _ => None,
        })
        .collect()
}

/// Les éléments de `item`, chacun d'id `next_id()` (`s<numéro>-<rang>`).
pub(crate) fn elements(item: &Item, ctx: &ElementCtx<'_>, next_id: &mut impl FnMut() -> String) -> Placed {
    let mut b = Builder { ctx, next_id, out: Placed::default() };
    b.item(item);
    b.out
}

/// Un trait : couleur Oku et épaisseur en pt page.
#[derive(Debug, Clone, PartialEq)]
struct Pen {
    color: String,
    width: f64,
}

/// Le peint d'une forme : fond et trait.
#[derive(Debug, Clone, PartialEq)]
struct Paint {
    fill: Option<String>,
    pen: Option<Pen>,
}

impl Paint {
    fn is_none(&self) -> bool {
        self.fill.is_none() && self.pen.is_none()
    }
}

/// L'état d'une conversion.
struct Builder<'c, 'a, F> {
    ctx: &'c ElementCtx<'a>,
    next_id: &'c mut F,
    out: Placed,
}

impl<F: FnMut() -> String> Builder<'_, '_, F> {
    fn lose(&mut self, item: &Item, kind: LossKind) {
        note(&mut self.out.losses, Loss::on(&item.id, kind));
    }

    /// Un élément `kind` neuf en `frame`, ajouté à la suite.
    fn push(&mut self, kind: &str, frame: Frame) -> &mut Element {
        let mut e = Element::new((self.next_id)(), kind);
        e.frame = Some(frame);
        self.out.elements.push(e);
        let last = self.out.elements.len() - 1;
        &mut self.out.elements[last]
    }

    fn item(&mut self, item: &Item) {
        let f = &item.frame;
        if self.ctx.effects.contains(&item.id) {
            self.lose(item, LossKind::Effects);
        }
        if f.rot != 0.0 {
            self.lose(item, LossKind::Transform);
        }
        let flipped = f.flip_h || f.flip_v;
        let frame = self.ctx.base.page.frame(f);
        match &item.kind {
            ItemKind::Text(tb) => {
                if f.flip_v {
                    self.lose(item, LossKind::Transform);
                }
                self.text(item, frame, inset(FULL, frame, tb.insets, self.ctx));
            }
            ItemKind::Shape(s) => self.shape(item, s, frame),
            ItemKind::Picture(p) => {
                if flipped || visible(p.crop) {
                    self.lose(item, LossKind::Transform);
                }
                self.image(item, &p.asset, frame);
            }
            ItemKind::Table(t) => {
                if flipped {
                    self.lose(item, LossKind::Transform);
                }
                // En-tête et bandes lus sur les cellules ; le reste du style, s'il diffère du thème, est perdu.
                let look = TableLook::of(t, self.ctx.base.colors, self.ctx.base.page);
                if !look.matches(self.ctx.table) {
                    self.lose(item, LossKind::TableStyle);
                }
                let option = table(t).map(|(mut option, lost)| {
                    look.set_flags(&mut option);
                    (option, lost)
                });
                self.host(item, frame, "table", "data", option);
            }
            ItemKind::Chart(c) => {
                if flipped {
                    self.lose(item, LossKind::Transform);
                }
                let option = chart(c).map(|(mut option, lost)| {
                    series_colors(&mut option, c);
                    (option, lost)
                });
                self.host(item, frame, "chart", "option", option);
            }
            ItemKind::Group { items, .. } => {
                if flipped {
                    self.lose(item, LossKind::Transform);
                }
                for child in items {
                    self.item(child);
                }
            }
            ItemKind::Opaque(o) => match &o.fallback {
                Some(asset) => {
                    if flipped {
                        self.lose(item, LossKind::Transform);
                    }
                    self.image(item, asset, frame);
                }
                None => self.lose(item, LossKind::Opaque(o.reason.clone())),
            },
        }
    }

    /// Le texte de `item` en `frame`, écarté des bords de `inset` ; rien s'il est vide.
    fn text(&mut self, item: &Item, frame: Frame, inset: [f64; 4]) {
        let Some(tb) = item.text().filter(|_| item.has_text()) else { return };
        let (element, lost) = text_element((self.next_id)(), tb, widened(frame), inset, self.ctx);
        for l in lost {
            self.lose(item, l);
        }
        if let Some(e) = element {
            self.out.origins.push((item.id.clone(), e.id.clone()));
            self.out.elements.push(e);
        }
    }

    /// L'image `asset` étirée à `frame` ; rien si l'asset manque (le lecteur
    /// l'a dit), perte s'il ne se dessine pas.
    fn image(&mut self, item: &Item, asset: &str, frame: Frame) {
        let Some(a) = self.ctx.base.assets.get(asset) else { return };
        if !a.is_drawable() {
            self.lose(item, LossKind::UnsupportedImage(a.mime.clone()));
            return;
        }
        self.out.assets.insert(asset.to_owned());
        let e = self.push("image", widened(frame));
        e.params.set("asset", ParamValue::Text(asset.to_owned()));
        e.params.set("fit", ParamValue::Text("stretch".to_owned()));
    }

    /// Un élément porteur d'une option d'hôte (`table.data`, `chart.option`).
    fn host(&mut self, item: &Item, frame: Frame, kind: &str, param: &str, option: Option<(Value, Vec<LossKind>)>) {
        let Some((value, lost)) = option else { return };
        for l in lost {
            self.lose(item, l);
        }
        self.push(kind, widened(frame)).params.set(param, ParamValue::Value(value));
    }

    /// Une forme, puis son texte sur le même cadre.
    fn shape(&mut self, item: &Item, s: &Shape, frame: Frame) {
        let f = &item.frame;
        let mut paint = Paint { fill: self.fill(item, &s.fill), pen: self.pen(item, s.stroke.as_ref()) };
        let geometry = outline(&s.geom, frame.w, frame.h);
        if let Fill::Picture { asset } = &s.fill {
            if !matches!(&s.geom, Geometry::Preset { name, .. } if name == "rect") {
                self.lose(item, LossKind::ShapeStyle(ShapeEffect::Picture));
            }
            self.image(item, asset, frame);
            paint.fill = None;
        }
        match &s.geom {
            Geometry::Preset { name, .. } if name == "rect" => self.native(frame, "rect", None, &paint),
            Geometry::Preset { name, adj } if name == "roundRect" => {
                let radius = round2(round_rect_radius(adj, frame.w, frame.h));
                self.native(frame, "rect", Some(radius), &paint);
            }
            Geometry::Preset { name, .. } if name == "ellipse" => self.native(frame, "ellipse", None, &paint),
            Geometry::Preset { name, .. } if name == "line" => self.line(item, frame, paint.pen.clone()),
            _ => match &geometry {
                Some(o) => self.outline(item, frame, o, &paint),
                None => {
                    self.lose(item, LossKind::ShapeStyle(ShapeEffect::Geometry));
                    self.native(frame, "rect", None, &paint);
                }
            },
        }
        if let (Some(stroke), Some(pen)) = (&s.stroke, &paint.pen) {
            let stroked = match &s.geom {
                Geometry::Preset { name, .. } if name == "line" => Some(line_cmds(f.flip_h, f.flip_v)),
                _ => geometry.as_ref().map(|o| flipped(&o.stroked, f.flip_h, f.flip_v)),
            };
            self.arrows(item, frame, &stroked.unwrap_or_default(), stroke, pen);
        }
        if item.has_text() {
            let [l, t, r, b] = geometry.map_or(FULL, |o| o.text_rect);
            let (l, r) = if f.flip_h { (1.0 - r, 1.0 - l) } else { (l, r) };
            let (t, b) = if f.flip_v { (1.0 - b, 1.0 - t) } else { (t, b) };
            if f.flip_v {
                self.lose(item, LossKind::Transform);
            }
            let insets = s.text.as_ref().map_or_else(Insets::default, |tb| tb.insets);
            self.text(item, frame, inset([l, t, r, b], frame, insets, self.ctx));
        }
    }

    /// Une forme native (`rect`, `ellipse`) ; rien sans fond ni trait.
    fn native(&mut self, frame: Frame, shape: &str, radius: Option<f64>, paint: &Paint) {
        if paint.is_none() {
            return;
        }
        let e = self.push("shape", widened(frame));
        e.params.set("shape", ParamValue::Text(shape.to_owned()));
        e.style.radius = radius.map(TokenValue::Number);
        paint.apply(e);
    }

    /// Une ligne : native si elle est horizontale, sinon un chemin d'un coin
    /// à l'autre (selon ses miroirs) ; rien sans trait.
    fn line(&mut self, item: &Item, frame: Frame, pen: Option<Pen>) {
        let Some(pen) = pen else { return };
        let paint = Paint { fill: None, pen: Some(pen) };
        if frame.h == 0.0 && frame.w > 0.0 {
            // Le widget tire la ligne en haut de son cadre : le cadre part de la ligne.
            let e = self.push("shape", Frame { h: MIN_SIDE, ..frame });
            e.params.set("shape", ParamValue::Text("line".to_owned()));
            paint.apply(e);
            return;
        }
        let f = &item.frame;
        self.path(item, frame, line_cmds(f.flip_h, f.flip_v), &paint);
    }

    /// Les pointes de `stroke` au bout du tracé `cmds` (fractions de `frame`) :
    /// chacune un `shape: path` sur son propre cadre, pleine de la couleur du trait ou tracée.
    fn arrows(&mut self, item: &Item, frame: Frame, cmds: &[PathCommand], stroke: &Stroke, pen: &Pen) {
        for head in heads(cmds, frame.w, frame.h, pen.width, stroke) {
            let pts: Vec<(f64, f64)> = head.cmds.iter().flat_map(|c| points(*c)).collect();
            let min = |f: fn(&(f64, f64)) -> f64| pts.iter().map(f).fold(f64::INFINITY, f64::min);
            let max = |f: fn(&(f64, f64)) -> f64| pts.iter().map(f).fold(f64::NEG_INFINITY, f64::max);
            let (x, y) = (min(|p| p.0), min(|p| p.1));
            let (w, h) = (max(|p| p.0) - x, max(|p| p.1) - y);
            let at = Frame { x: frame.x + x, y: frame.y + y, w, h };
            let local = map_points(&head.cmds, |px, py| (fraction(px - x, w), fraction(py - y, h)));
            let paint = if head.filled {
                Paint { fill: Some(pen.color.clone()), pen: None }
            } else {
                Paint { fill: None, pen: Some(pen.clone()) }
            };
            self.path(item, at, local, &paint);
        }
    }

    /// Le contour `o` : un chemin rempli et tracé, ou la part remplie sans
    /// trait puis la part tracée sans fond.
    fn outline(&mut self, item: &Item, frame: Frame, o: &Outline, paint: &Paint) {
        let f = &item.frame;
        let flip = |cmds: &[PathCommand]| flipped(cmds, f.flip_h, f.flip_v);
        if o.filled == o.stroked {
            self.path(item, frame, flip(&o.filled), paint);
            return;
        }
        if !o.filled.is_empty() {
            self.path(item, frame, flip(&o.filled), &Paint { fill: paint.fill.clone(), pen: None });
        }
        if !o.stroked.is_empty() {
            self.path(item, frame, flip(&o.stroked), &Paint { fill: None, pen: paint.pen.clone() });
        }
    }

    /// Un chemin `cmds` (fractions de `frame`) ; rien sans fond ni trait, son
    /// cadre (perte) s'il ne s'écrit pas dans les bornes du cœur.
    fn path(&mut self, item: &Item, frame: Frame, cmds: Vec<PathCommand>, paint: &Paint) {
        if paint.is_none() || cmds.is_empty() {
            return;
        }
        let (frame, cmds) = widen(frame, cmds);
        let Some(path) = ShapePath::from_commands(&cmds) else {
            self.lose(item, LossKind::ShapeStyle(ShapeEffect::Geometry));
            self.native(frame, "rect", None, paint);
            return;
        };
        let e = self.push("shape", frame);
        e.params.set("shape", ParamValue::Text("path".to_owned()));
        e.params.set("path", ParamValue::Value(json!({ "d": path.d })));
        paint.apply(e);
    }

    /// Le fond plein de `fill` : sa couleur, ou le premier arrêt d'un dégradé (perte).
    fn fill(&mut self, item: &Item, fill: &Fill) -> Option<String> {
        match fill {
            Fill::None | Fill::Picture { .. } => None,
            Fill::Solid { color } => Some(self.ctx.base.colors.write(color)),
            Fill::Gradient { stops, .. } => {
                self.lose(item, LossKind::ShapeStyle(ShapeEffect::Gradient));
                let first = stops.iter().min_by(|a, b| a.pos.total_cmp(&b.pos))?;
                Some(self.ctx.base.colors.write(&first.color))
            }
        }
    }

    /// Le trait `stroke`, continu (tirets : perte).
    fn pen(&mut self, item: &Item, stroke: Option<&Stroke>) -> Option<Pen> {
        let s = stroke?;
        if s.dash != Dash::Solid {
            self.lose(item, LossKind::ShapeStyle(ShapeEffect::Dash));
        }
        Some(Pen { color: self.ctx.base.colors.write(&s.color), width: self.ctx.base.page.len(s.width) })
    }
}

impl Paint {
    /// Écrit le fond et le trait dans le style de `e`.
    fn apply(&self, e: &mut Element) {
        e.style.fill.clone_from(&self.fill);
        if let Some(pen) = &self.pen {
            e.style.stroke = Some(pen.color.clone());
            e.style.stroke_width = Some(TokenValue::Number(pen.width));
        }
    }
}

/// Le rognage `c` se voit : un de ses bords atteint [`CROP_TOLERANCE`].
fn visible(c: Crop) -> bool {
    [c.l, c.t, c.r, c.b].iter().any(|v| v.abs() >= CROP_TOLERANCE)
}

/// Les marges d'un texte dans `frame` (pt page) : la zone `[l, t, r, b]`
/// (fractions du cadre) puis les marges source `insets`, dans les bornes du cœur.
fn inset([l, t, r, b]: [f64; 4], frame: Frame, insets: Insets, ctx: &ElementCtx<'_>) -> [f64; 4] {
    let page = ctx.base.page;
    let side = |zone: f64, margin: f64| round2(zone + page.len(margin)).clamp(0.0, STYLE_INSET_MAX);
    [
        side(t * frame.h, insets.t),
        side((1.0 - r) * frame.w, insets.r),
        side((1.0 - b) * frame.h, insets.b),
        side(l * frame.w, insets.l),
    ]
}

/// Les couleurs des séries du graphique `c` dans ses options : celles qui en
/// ont une, pour les sortes dont chaque série est une colonne (pas les
/// secteurs ni le radar, colorés par point).
fn series_colors(option: &mut Value, c: &source::Chart) {
    let per_series =
        matches!(c.kind, ChartKind::Column | ChartKind::Bar | ChartKind::Line | ChartKind::Area | ChartKind::Scatter);
    let Some(series) = option.get_mut("series").and_then(Value::as_array_mut).filter(|_| per_series) else { return };
    for (s, src) in series.iter_mut().zip(&c.series) {
        if let Some(color) = &src.color {
            // Les options n'admettent que `#rrggbb` : l'opacité se perd.
            s["itemStyle"] = json!({ "color": Rgba { a: 255, ..color.value }.to_hex() });
        }
    }
}

/// `cmds` aux points transformés par `f`.
fn map_points(cmds: &[PathCommand], f: impl Fn(f64, f64) -> (f64, f64)) -> Vec<PathCommand> {
    cmds.iter()
        .map(|c| match *c {
            PathCommand::Move(x, y) => {
                let (x, y) = f(x, y);
                PathCommand::Move(x, y)
            }
            PathCommand::Line(x, y) => {
                let (x, y) = f(x, y);
                PathCommand::Line(x, y)
            }
            PathCommand::Cubic(x1, y1, x2, y2, x, y) => {
                let ((x1, y1), (x2, y2), (x, y)) = (f(x1, y1), f(x2, y2), f(x, y));
                PathCommand::Cubic(x1, y1, x2, y2, x, y)
            }
            PathCommand::Quad(x1, y1, x, y) => {
                let ((x1, y1), (x, y)) = (f(x1, y1), f(x, y));
                PathCommand::Quad(x1, y1, x, y)
            }
            PathCommand::Close => PathCommand::Close,
        })
        .collect()
}

/// La diagonale d'une ligne d'un coin à l'autre selon ses miroirs, en fractions du cadre.
fn line_cmds(flip_h: bool, flip_v: bool) -> Vec<PathCommand> {
    let (x0, y0) = (if flip_h { 1.0 } else { 0.0 }, if flip_v { 1.0 } else { 0.0 });
    vec![PathCommand::Move(x0, y0), PathCommand::Line(1.0 - x0, 1.0 - y0)]
}

/// `cmds` (fractions du cadre) retournées selon les miroirs.
fn flipped(cmds: &[PathCommand], flip_h: bool, flip_v: bool) -> Vec<PathCommand> {
    map_points(cmds, |x, y| (if flip_h { 1.0 - x } else { x }, if flip_v { 1.0 - y } else { y }))
}

/// `v` en fraction de `side`, 0 sur un côté nul.
fn fraction(v: f64, side: f64) -> f64 {
    if side > 0.0 { v / side } else { 0.0 }
}

/// `frame` élargi à [`MIN_SIDE`] au moins de chaque côté, autour de son
/// centre, et `cmds` ramenés au cadre élargi (une ligne verticale passe au
/// milieu de son cadre).
fn widen(frame: Frame, cmds: Vec<PathCommand>) -> (Frame, Vec<PathCommand>) {
    let (w, h) = (frame.w.max(MIN_SIDE), frame.h.max(MIN_SIDE));
    if w == frame.w && h == frame.h {
        return (frame, cmds);
    }
    let (px, py) = ((w - frame.w) / 2.0, (h - frame.h) / 2.0);
    let cmds = map_points(&cmds, |x, y| ((x * frame.w + px) / w, (y * frame.h + py) / h));
    (Frame { x: round2(frame.x - px), y: round2(frame.y - py), w, h }, cmds)
}

/// `frame` élargi à [`MIN_SIDE`] au moins de chaque côté (voir [`widen`]).
fn widened(frame: Frame) -> Frame {
    widen(frame, Vec::new()).0
}

#[cfg(test)]
#[path = "element_tests.rs"]
pub(super) mod element_tests;
