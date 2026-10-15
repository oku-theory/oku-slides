//! Formes : natifs Typst (`rect`, `ellipse`, `line`) ou `curve` du contour,
//! remplissage (plein, dégradé, image) et trait.

use std::collections::BTreeMap;

use oku_slides_core::PathCommand;

use super::{Gen, Placement, Typst, load, num, pt, string};
use crate::hifi::geometry::arrow::heads;
use crate::hifi::geometry::{Outline, outline};
use crate::report::{Loss, LossKind};
use crate::source::{Dash, Fill, Geometry, Shape, Stroke};

/// Zone de texte d'une forme sans contour calculé : son cadre entier.
const FULL: [f64; 4] = [0.0, 0.0, 1.0, 1.0];
/// Ajustement par défaut d'un `roundRect` (ECMA-376), en 100 000ᵉ du petit côté.
const ROUND_RECT_ADJ: f64 = 16_667.0;
/// Ajustement maximal d'un `roundRect` : demi-côté.
const ROUND_RECT_ADJ_MAX: f64 = 50_000.0;

impl Gen<'_> {
    /// Dessine la forme `s` de l'item `id` en `at` (son texte à part).
    pub(super) fn shape(&mut self, id: &str, at: &Placement, s: &Shape) {
        let f = at.frame;
        let fill = self.fill(id, &s.fill, f.w, f.h);
        let stroke = self.stroke(s.stroke.as_ref());
        let (w, h) = (pt(f.w), pt(f.h));
        match &s.geom {
            Geometry::Preset { name, adj } if name == "rect" || name == "roundRect" => {
                let radius = if name == "roundRect" {
                    format!(", radius: {}", pt(round_rect_radius(adj, f.w, f.h)))
                } else {
                    String::new()
                };
                let body = format!("rect(width: {w}, height: {h}, fill: {fill}, stroke: {stroke}{radius}),");
                at.emit(&mut self.w, |t| t.line(&body));
            }
            Geometry::Preset { name, .. } if name == "ellipse" => {
                let body = format!("ellipse(width: {w}, height: {h}, fill: {fill}, stroke: {stroke}),");
                at.emit(&mut self.w, |t| t.line(&body));
            }
            Geometry::Preset { name, .. } if name == "line" => {
                if s.stroke.is_none() {
                    return;
                }
                // Le miroir d'une ligne change sa diagonale : il s'écrit dans ses extrémités.
                let (x0, x1) = if at.flip_h { (w.clone(), pt(0.0)) } else { (pt(0.0), w.clone()) };
                let (y0, y1) = if at.flip_v { (h.clone(), pt(0.0)) } else { (pt(0.0), h.clone()) };
                let body = format!("line(start: ({x0}, {y0}), end: ({x1}, {y1}), stroke: {stroke}),");
                let (fx, fy) = (if at.flip_h { 1.0 } else { 0.0 }, if at.flip_v { 1.0 } else { 0.0 });
                let at = Placement { flip_h: false, flip_v: false, ..*at };
                at.emit(&mut self.w, |t| t.line(&body));
                let diagonal = [PathCommand::Move(fx, fy), PathCommand::Line(1.0 - fx, 1.0 - fy)];
                self.arrows(&at, &diagonal, s.stroke.as_ref());
            }
            geom => match outline(geom, f.w, f.h) {
                Some(o) => {
                    self.curves(at, &o, &fill, &stroke);
                    self.arrows(at, &o.stroked, s.stroke.as_ref());
                }
                // Preset inconnu (fichier altéré) : son cadre, plutôt que rien.
                None => {
                    let body = format!("rect(width: {w}, height: {h}, fill: {fill}, stroke: {stroke}),");
                    at.emit(&mut self.w, |t| t.line(&body));
                }
            },
        }
    }

    /// Les `curve` du contour `o` : un seul s'il est rempli et tracé à
    /// l'identique, sinon la part remplie puis la part tracée.
    fn curves(&mut self, at: &Placement, o: &Outline, fill: &str, stroke: &str) {
        let (w, h) = (at.frame.w, at.frame.h);
        if o.filled == o.stroked {
            at.emit(&mut self.w, |t| curve(t, &o.filled, w, h, fill, stroke));
            return;
        }
        if !o.filled.is_empty() && fill != "none" {
            at.emit(&mut self.w, |t| curve(t, &o.filled, w, h, fill, "none"));
        }
        if !o.stroked.is_empty() && stroke != "none" {
            at.emit(&mut self.w, |t| curve(t, &o.stroked, w, h, "none", stroke));
        }
    }

    /// Les pointes de `stroke` au bout du tracé `cmds` (fractions du cadre de
    /// `at`) : des `curve` pleins de la couleur du trait, ou tracés pour une flèche ouverte.
    fn arrows(&mut self, at: &Placement, cmds: &[PathCommand], stroke: Option<&Stroke>) {
        let Some(s) = stroke else { return };
        let width = self.ctx.page.len(s.width);
        let color = self.color(&s.color);
        for head in heads(cmds, at.frame.w, at.frame.h, width, s) {
            let (fill, pen) = if head.filled {
                (color.clone(), "none".to_owned())
            } else {
                ("none".to_owned(), format!("(paint: {color}, thickness: {})", pt(width)))
            };
            at.emit(&mut self.w, |t| curve(t, &head.cmds, 1.0, 1.0, &fill, &pen));
        }
    }

    /// Le remplissage `fill` d'une forme de `w` × `h` pt, en expression Typst.
    pub(super) fn fill(&mut self, id: &str, fill: &Fill, w: f64, h: f64) -> String {
        match fill {
            Fill::None => "none".to_owned(),
            Fill::Solid { color } => self.color(color),
            Fill::Gradient { angle, stops } => {
                let mut stops: Vec<(f64, String)> =
                    stops.iter().map(|s| (s.pos.clamp(0.0, 1.0), self.color(&s.color))).collect();
                stops.sort_by(|a, b| a.0.total_cmp(&b.0));
                let (Some(first), Some(last)) = (stops.first().cloned(), stops.last().cloned()) else {
                    return "none".to_owned();
                };
                if stops.len() == 1 {
                    return first.1;
                }
                // Typst veut un premier arrêt à 0 % et un dernier à 100 %.
                if first.0 > 0.0 {
                    stops.insert(0, (0.0, first.1));
                }
                if last.0 < 1.0 {
                    stops.push((1.0, last.1));
                }
                let stops: Vec<String> = stops.iter().map(|(p, c)| format!("({c}, {}%)", num(p * 100.0))).collect();
                format!("gradient.linear({}, angle: {}deg, space: rgb)", stops.join(", "), num(*angle))
            }
            Fill::Picture { asset } => {
                let Some(a) = self.ctx.assets.get(asset) else { return "none".to_owned() };
                if !a.is_drawable() {
                    self.losses.push(Loss::on(id, LossKind::UnsupportedImage(a.mime.clone())));
                    return "none".to_owned();
                }
                self.assets.insert(asset.clone());
                // Une tuile de la taille du cadre : l'image étirée à la forme, quelle qu'elle soit.
                let (w, h) = (pt(w), pt(h));
                format!("tiling(size: ({w}, {h}), box(width: {w}, height: {h}, {}))", load(asset))
            }
        }
    }

    /// La couleur `c` : `color-of` du jeton ou de l'hexadécimal.
    pub(super) fn color(&self, c: &crate::source::Color) -> String {
        format!("color-of(t, {})", string(&self.ctx.colors.write(c)))
    }

    /// Le trait `stroke`, en expression Typst (`none` sans trait).
    fn stroke(&self, stroke: Option<&Stroke>) -> String {
        let Some(s) = stroke else { return "none".to_owned() };
        let dash = match s.dash {
            Dash::Solid => String::new(),
            Dash::Dash => ", dash: \"dashed\"".to_owned(),
            Dash::Dot => ", dash: \"dotted\"".to_owned(),
            Dash::DashDot => ", dash: \"dash-dotted\"".to_owned(),
            Dash::LongDash => ", dash: \"loosely-dashed\"".to_owned(),
        };
        format!("(paint: {}, thickness: {}{dash})", self.color(&s.color), pt(self.ctx.page.len(s.width)))
    }
}

/// Le rayon des coins d'un `roundRect` d'ajustements `adj`, dans un cadre
/// de `w` × `h` (même unité que le résultat).
pub(crate) fn round_rect_radius(adj: &BTreeMap<String, f64>, w: f64, h: f64) -> f64 {
    let a = adj.get("adj").copied().unwrap_or(ROUND_RECT_ADJ).clamp(0.0, ROUND_RECT_ADJ_MAX);
    a / 100_000.0 * w.min(h)
}

/// Un `curve` des commandes `cmds` (fractions du cadre) étirées à `w` × `h` pt.
fn curve(t: &mut Typst, cmds: &[PathCommand], w: f64, h: f64, fill: &str, stroke: &str) {
    let p = |x: f64, y: f64| format!("({}, {})", pt(x * w), pt(y * h));
    t.open(&format!("curve(fill: {fill}, stroke: {stroke},"));
    for c in cmds {
        let line = match *c {
            PathCommand::Move(x, y) => format!("curve.move({}),", p(x, y)),
            PathCommand::Line(x, y) => format!("curve.line({}),", p(x, y)),
            PathCommand::Cubic(x1, y1, x2, y2, x, y) => {
                format!("curve.cubic({}, {}, {}),", p(x1, y1), p(x2, y2), p(x, y))
            }
            PathCommand::Quad(x1, y1, x, y) => format!("curve.quad({}, {}),", p(x1, y1), p(x, y)),
            PathCommand::Close => "curve.close(mode: \"straight\"),".to_owned(),
        };
        t.line(&line);
    }
    t.close("),");
}

/// Zone de texte de la forme `s` dans un cadre de `w` × `h` pt page
/// (gauche, haut, droite, bas, fractions du cadre).
pub(super) fn text_rect(s: &Shape, w: f64, h: f64) -> [f64; 4] {
    outline(&s.geom, w, h).map_or(FULL, |o| o.text_rect)
}
