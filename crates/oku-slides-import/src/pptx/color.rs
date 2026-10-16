//! Couleurs DrawingML : sRGB, thème (via `clrMap`), système, prédéfinies, `phClr`, modificateurs.
//!
//! Une couleur se lit en [`ColorSpec`] (non résolue) et se résout avec le
//! [`ColorCtx`] de la part qui l'affiche : le `clrMapOvr` d'une slide atteint
//! ainsi tout ce qu'elle hérite. `phClr` est remplacé syntaxiquement.

use std::collections::BTreeMap;
use std::rc::Rc;

use roxmltree::Node;

use crate::collect::clip_to;
use crate::pptx::units::pct;
use crate::pptx::xml::{attr_i64, attr_str, elements};
use crate::source::{Color, ColorMod, Origin, Rgba, Styled};

/// Modificateurs gardés par couleur.
pub(crate) const MAX_COLOR_MODS: usize = 16;

/// Éléments couleur lus ; leurs enfants sont des modificateurs.
pub(crate) const COLOR_ELEMENTS: [&str; 5] = ["srgbClr", "sysClr", "prstClr", "scrgbClr", "schemeClr"];

/// Modificateurs appliqués ; tout autre est ignoré (et signalé par `bounds::clamped`).
pub(crate) const KNOWN_MODS: [&str; 6] = ["alpha", "lumMod", "lumOff", "satMod", "tint", "shade"];

/// Emplacements de couleur d'un thème.
pub(crate) const SLOTS: [&str; 12] =
    ["dk1", "lt1", "dk2", "lt2", "accent1", "accent2", "accent3", "accent4", "accent5", "accent6", "hlink", "folHlink"];

const CLR_MAP: [(&str, &str); 12] = [
    ("bg1", "lt1"),
    ("tx1", "dk1"),
    ("bg2", "lt2"),
    ("tx2", "dk2"),
    ("accent1", "accent1"),
    ("accent2", "accent2"),
    ("accent3", "accent3"),
    ("accent4", "accent4"),
    ("accent5", "accent5"),
    ("accent6", "accent6"),
    ("hlink", "hlink"),
    ("folHlink", "folHlink"),
];

/// `clrMap` par défaut.
pub(crate) fn default_map() -> BTreeMap<String, String> {
    CLR_MAP.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect()
}

/// `clrMap` lu dans des attributs, par-dessus `base` : seules les 12 clés et les 12 emplacements sont admis.
pub(crate) fn clean_map<'a>(
    attrs: impl Iterator<Item = (&'a str, &'a str)>,
    base: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut map = base.clone();
    for (k, v) in attrs {
        if CLR_MAP.iter().any(|(key, _)| *key == k) && SLOTS.contains(&v) {
            map.insert(k.to_owned(), v.to_owned());
        }
    }
    map
}

/// Thème et `clrMap` en vigueur dans une part (clones bon marché).
#[derive(Debug, Clone)]
pub(crate) struct ColorCtx {
    pub scheme: Rc<BTreeMap<String, Rgba>>,
    pub map: Rc<BTreeMap<String, String>>,
}

impl ColorCtx {
    pub(crate) fn new(scheme: BTreeMap<String, Rgba>, map: BTreeMap<String, String>) -> Self {
        Self { scheme: Rc::new(scheme), map: Rc::new(map) }
    }

    /// Emplacement de thème et valeur pour un nom (`tx1`, `accent1`, `dk2`…).
    pub(crate) fn slot(&self, name: &str) -> Option<(String, Rgba)> {
        let slot = self.map.get(name).map_or(name, String::as_str);
        self.scheme.get(slot).map(|c| (slot.to_owned(), *c))
    }
}

/// Base d'une couleur non résolue.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ColorBase {
    Fixed(Rgba),
    /// Nom du thème tel qu'écrit (`tx1`, `accent1`…), avant `clrMap`.
    Scheme(String),
    /// `phClr` : la couleur fournie par la référence de style.
    Placeholder,
}

/// Couleur non résolue : base et modificateurs (16 au plus).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ColorSpec {
    pub base: ColorBase,
    pub mods: Vec<ColorMod>,
}

impl ColorSpec {
    /// `phClr` remplacé par `ph` (ses modificateurs, puis les nôtres) ; `None` si `phClr` reste sans valeur.
    pub(crate) fn with_placeholder(&self, ph: Option<&ColorSpec>) -> Option<ColorSpec> {
        match (&self.base, ph) {
            (ColorBase::Placeholder, Some(p)) => {
                let mods = p.mods.iter().chain(&self.mods).copied().take(MAX_COLOR_MODS).collect();
                Some(ColorSpec { base: p.base.clone(), mods })
            }
            (ColorBase::Placeholder, None) => None,
            _ => Some(self.clone()),
        }
    }

    /// Couleur résolue dans `ctx`.
    pub(crate) fn resolve(&self, ctx: &ColorCtx) -> Option<Color> {
        match &self.base {
            ColorBase::Fixed(c) => Some(Styled::explicit(apply(*c, &self.mods))),
            ColorBase::Scheme(name) => {
                let (slot, v) = ctx.slot(name)?;
                Some(Styled {
                    value: apply(v, &self.mods),
                    origin: Origin::Scheme { name: name.clone(), slot, mods: self.mods.clone() },
                })
            }
            ColorBase::Placeholder => None,
        }
    }
}

/// Lit un élément couleur (`srgbClr`, `schemeClr`, `sysClr`, `prstClr`, `scrgbClr`).
pub(crate) fn color_spec(c: Node<'_, '_>) -> Option<ColorSpec> {
    let base = match c.tag_name().name() {
        "srgbClr" => ColorBase::Fixed(hex(attr_str(c, "val")?)?),
        "sysClr" => ColorBase::Fixed(match attr_str(c, "lastClr") {
            Some(last) => hex(last)?,
            None => sys_default(attr_str(c, "val")?),
        }),
        "prstClr" => ColorBase::Fixed(preset(attr_str(c, "val")?)?),
        "scrgbClr" => ColorBase::Fixed(Rgba::new(
            lin_to_srgb(pct(attr_i64(c, "r")?)),
            lin_to_srgb(pct(attr_i64(c, "g")?)),
            lin_to_srgb(pct(attr_i64(c, "b")?)),
        )),
        "schemeClr" => match attr_str(c, "val")? {
            "phClr" => ColorBase::Placeholder,
            name => ColorBase::Scheme(clip_to(name, 16)),
        },
        _ => return None,
    };
    let mods = elements(c)
        .filter_map(|m| {
            let v = pct(attr_i64(m, "val")?);
            Some(match m.tag_name().name() {
                "alpha" => ColorMod::Alpha(v),
                "lumMod" => ColorMod::LumMod(v),
                "lumOff" => ColorMod::LumOff(v),
                "satMod" => ColorMod::SatMod(v),
                "tint" => ColorMod::Tint(v),
                "shade" => ColorMod::Shade(v),
                _ => return None,
            })
        })
        .take(MAX_COLOR_MODS)
        .collect();
    Some(ColorSpec { base, mods })
}

/// Couleur du premier enfant élément de `container` (`solidFill`, `buClr`…), non résolue.
pub(crate) fn color_spec_in(container: Node<'_, '_>) -> Option<ColorSpec> {
    elements(container).next().and_then(color_spec)
}

/// Couleur du premier enfant de `container`, résolue (sans `phClr`).
pub(crate) fn parse_color(container: Node<'_, '_>, ctx: &ColorCtx) -> Option<Color> {
    color_spec_in(container)?.resolve(ctx)
}

fn hex(s: &str) -> Option<Rgba> {
    let v = u32::from_str_radix(s.get(..6)?, 16).ok()?;
    Some(Rgba::new((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

fn sys_default(name: &str) -> Rgba {
    match name {
        "window" | "highlightText" => Rgba::new(255, 255, 255),
        _ => Rgba::new(0, 0, 0),
    }
}

fn preset(name: &str) -> Option<Rgba> {
    Some(match name {
        "black" => Rgba::new(0, 0, 0),
        "white" => Rgba::new(255, 255, 255),
        "red" => Rgba::new(255, 0, 0),
        "green" => Rgba::new(0, 128, 0),
        "blue" => Rgba::new(0, 0, 255),
        "yellow" => Rgba::new(255, 255, 0),
        "gray" | "grey" => Rgba::new(128, 128, 128),
        "darkGray" | "dkGray" => Rgba::new(169, 169, 169),
        "lightGray" | "ltGray" => Rgba::new(211, 211, 211),
        "orange" => Rgba::new(255, 165, 0),
        _ => return None,
    })
}

fn srgb_to_lin(c: u8) -> f64 {
    let c = f64::from(c) / 255.0;
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

fn lin_to_srgb(c: f64) -> u8 {
    let c = c.clamp(0.0, 1.0);
    let s = if c <= 0.003_130_8 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 };
    (s * 255.0).round() as u8
}

fn to_hsl(c: Rgba) -> (f64, f64, f64) {
    let (r, g, b) = (f64::from(c.r) / 255.0, f64::from(c.g) / 255.0, f64::from(c.b) / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let l = (max + min) / 2.0;
    let d = max - min;
    if d.abs() < f64::EPSILON {
        return (0.0, 0.0, l);
    }
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if (max - r).abs() < f64::EPSILON {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < f64::EPSILON {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h / 6.0, s, l)
}

fn hue(p: f64, q: f64, t: f64) -> f64 {
    let t = t.rem_euclid(1.0);
    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 0.5 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

fn from_hsl(h: f64, s: f64, l: f64, a: u8) -> Rgba {
    let byte = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let l = l.clamp(0.0, 1.0);
    if s <= 0.0 {
        let v = byte(l);
        return Rgba { r: v, g: v, b: v, a };
    }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    Rgba { r: byte(hue(p, q, h + 1.0 / 3.0)), g: byte(hue(p, q, h)), b: byte(hue(p, q, h - 1.0 / 3.0)), a }
}

/// Applique les modificateurs dans l'ordre.
fn apply(mut c: Rgba, mods: &[ColorMod]) -> Rgba {
    for m in mods {
        c = match *m {
            ColorMod::Alpha(v) => Rgba { a: (v.clamp(0.0, 1.0) * 255.0).round() as u8, ..c },
            ColorMod::LumMod(v) => {
                let (h, s, l) = to_hsl(c);
                from_hsl(h, s, l * v, c.a)
            }
            ColorMod::LumOff(v) => {
                let (h, s, l) = to_hsl(c);
                from_hsl(h, s, l + v, c.a)
            }
            ColorMod::SatMod(v) => {
                let (h, s, l) = to_hsl(c);
                from_hsl(h, (s * v).clamp(0.0, 1.0), l, c.a)
            }
            ColorMod::Shade(v) => {
                let f = |x: u8| lin_to_srgb(srgb_to_lin(x) * v);
                Rgba { r: f(c.r), g: f(c.g), b: f(c.b), a: c.a }
            }
            ColorMod::Tint(v) => {
                let f = |x: u8| lin_to_srgb(srgb_to_lin(x) * v + (1.0 - v));
                Rgba { r: f(c.r), g: f(c.g), b: f(c.b), a: c.a }
            }
        };
    }
    c
}

#[cfg(test)]
#[path = "color_tests.rs"]
mod color_tests;
