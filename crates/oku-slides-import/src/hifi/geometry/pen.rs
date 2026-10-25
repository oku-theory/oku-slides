//! Tracé d'un sous-chemin DrawingML en commandes du cœur : point courant
//! suivi, arcs (`arcTo`) en Béziers cubiques, mise à l'échelle au cadre.

use std::f64::consts::{FRAC_PI_2, TAU};

use oku_slides_core::PathCommand;
use oku_slides_core::num::trunc_u32;

/// Écart sous lequel un balayage est nul ou un tour complet.
const EPS: f64 = 1e-9;

/// Crayon : coordonnées locales (espace du `path`) multipliées par
/// `(sx, sy)` à l'émission.
pub(super) struct Pen {
    out: Vec<PathCommand>,
    sx: f64,
    sy: f64,
    cur: (f64, f64),
    start: (f64, f64),
    /// Un sous-chemin est ouvert (un `M` a été émis depuis le dernier `Z`).
    open: bool,
}

impl Pen {
    pub(super) fn new(sx: f64, sy: f64) -> Self {
        Pen { out: Vec::new(), sx, sy, cur: (0.0, 0.0), start: (0.0, 0.0), open: false }
    }

    pub(super) fn finish(self) -> Vec<PathCommand> {
        self.out
    }

    fn at(&self, (x, y): (f64, f64)) -> (f64, f64) {
        (x * self.sx, y * self.sy)
    }

    /// Un tracé sans `moveTo` (en tête, ou après `close`) part du point courant.
    fn ensure_open(&mut self) {
        if !self.open {
            let (x, y) = self.at(self.cur);
            self.out.push(PathCommand::Move(x, y));
            self.start = self.cur;
            self.open = true;
        }
    }

    pub(super) fn move_to(&mut self, p: (f64, f64)) {
        let (x, y) = self.at(p);
        self.out.push(PathCommand::Move(x, y));
        (self.cur, self.start, self.open) = (p, p, true);
    }

    pub(super) fn line_to(&mut self, p: (f64, f64)) {
        self.ensure_open();
        let (x, y) = self.at(p);
        self.out.push(PathCommand::Line(x, y));
        self.cur = p;
    }

    pub(super) fn cubic_to(&mut self, c1: (f64, f64), c2: (f64, f64), p: (f64, f64)) {
        self.ensure_open();
        let ((x1, y1), (x2, y2), (x, y)) = (self.at(c1), self.at(c2), self.at(p));
        self.out.push(PathCommand::Cubic(x1, y1, x2, y2, x, y));
        self.cur = p;
    }

    pub(super) fn quad_to(&mut self, c: (f64, f64), p: (f64, f64)) {
        self.ensure_open();
        let ((x1, y1), (x, y)) = (self.at(c), self.at(p));
        self.out.push(PathCommand::Quad(x1, y1, x, y));
        self.cur = p;
    }

    pub(super) fn close(&mut self) {
        if self.open {
            self.out.push(PathCommand::Close);
            self.cur = self.start;
            self.open = false;
        }
    }

    /// Arc d'ellipse de rayons `(rx, ry)` qui part du point courant ; `start`
    /// et `sweep` en radians, angles visuels (la direction du rayon depuis le
    /// centre, comme les guides `cat2`/`sat2` des presets), sens horaire
    /// (y vers le bas) pour un balayage positif. Un balayage au-delà d'un tour
    /// est ramené à un tour ; au plus un quart de tour par cubique.
    pub(super) fn arc_to(&mut self, rx: f64, ry: f64, start: f64, sweep: f64) {
        self.ensure_open();
        let t1 = parametric(rx, ry, start);
        let dt = if sweep.abs() >= TAU - EPS {
            TAU.copysign(sweep)
        } else {
            let t2 = parametric(rx, ry, start + sweep);
            let d = (sweep.signum() * (t2 - t1)).rem_euclid(TAU);
            if d > TAU - EPS { 0.0 } else { d * sweep.signum() }
        };
        if dt.abs() < EPS {
            return;
        }
        let (px, py) = self.cur;
        let centre = (px - rx * t1.cos(), py - ry * t1.sin());
        let point = |t: f64| (centre.0 + rx * t.cos(), centre.1 + ry * t.sin());
        let tangent = |t: f64| (-rx * t.sin(), ry * t.cos());
        let n = (dt.abs() / FRAC_PI_2 - EPS).ceil().max(1.0);
        let step = dt / n;
        let k = 4.0 / 3.0 * (step / 4.0).tan();
        let mut a = t1;
        // `n`, already whole (`ceil`), is at most 4 (quarter turns): truncation is exact.
        for _ in 0..trunc_u32(n) {
            let b = a + step;
            let (pa, da, pb, db) = (point(a), tangent(a), point(b), tangent(b));
            self.cubic_to((pa.0 + k * da.0, pa.1 + k * da.1), (pb.0 - k * db.0, pb.1 - k * db.1), pb);
            a = b;
        }
    }
}

/// Paramètre de l'ellipse `(rx cos t, ry sin t)` dont le rayon a la direction
/// visuelle `angle`.
fn parametric(rx: f64, ry: f64, angle: f64) -> f64 {
    (rx * angle.sin()).atan2(ry * angle.cos())
}

#[cfg(test)]
#[path = "pen_tests.rs"]
mod pen_tests;
