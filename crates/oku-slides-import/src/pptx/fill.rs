//! Remplissages et traits, lus non résolus (`FillSpec`, `LineSpec`) et résolus par la part qui les affiche.

use roxmltree::Node;

use crate::pptx::color::{ColorCtx, ColorSpec, color_spec_in};
use crate::pptx::units::{angle, emu, pct};
use crate::pptx::xml::{attr_i64, attr_r, attr_str, child, children, elements};
use crate::source::{Dash, Fill, GradientStop, Stroke};

/// Arrêts gardés par dégradé.
pub(crate) const MAX_GRADIENT_STOPS: usize = 64;
const FILLS: [&str; 6] = ["noFill", "solidFill", "gradFill", "blipFill", "pattFill", "grpFill"];
/// Largeur d'un trait sans `w` : 9525 EMU.
const DEFAULT_LINE_PT: f64 = 0.75;

/// Remplissage non résolu.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FillSpec {
    None,
    Solid(ColorSpec),
    Gradient { angle: f64, stops: Vec<(f64, ColorSpec)> },
    Picture(String),
}

impl FillSpec {
    /// `phClr` remplacé par `ph` partout.
    pub(crate) fn with_placeholder(&self, ph: Option<&ColorSpec>) -> FillSpec {
        let sub = |c: &ColorSpec| c.with_placeholder(ph).unwrap_or_else(|| c.clone());
        match self {
            FillSpec::Solid(c) => FillSpec::Solid(sub(c)),
            FillSpec::Gradient { angle, stops } => {
                FillSpec::Gradient { angle: *angle, stops: stops.iter().map(|(p, c)| (*p, sub(c))).collect() }
            }
            other => other.clone(),
        }
    }

    /// Remplissage résolu dans `ctx` (couleur irrésoluble : rien n'est peint, ou l'arrêt est omis).
    pub(crate) fn resolve(&self, ctx: &ColorCtx) -> Fill {
        match self {
            FillSpec::None => Fill::None,
            FillSpec::Solid(c) => c.resolve(ctx).map_or(Fill::None, |color| Fill::Solid { color }),
            FillSpec::Gradient { angle, stops } => Fill::Gradient {
                angle: *angle,
                stops: stops
                    .iter()
                    .filter_map(|(pos, c)| Some(GradientStop { pos: *pos, color: c.resolve(ctx)? }))
                    .collect(),
            },
            FillSpec::Picture(asset) => Fill::Picture { asset: asset.clone() },
        }
    }
}

/// Premier remplissage enfant de `parent` ; `None` = hériter.
pub(crate) fn parse_fill(parent: Node<'_, '_>, blip: &mut dyn FnMut(&str) -> Option<String>) -> Option<FillSpec> {
    let f = elements(parent).find(|n| FILLS.contains(&n.tag_name().name()))?;
    parse_fill_element(f, blip)
}

/// Un élément qui *est* un remplissage ; `None` pour `grpFill` (hériter du groupe).
pub(crate) fn parse_fill_element(f: Node<'_, '_>, blip: &mut dyn FnMut(&str) -> Option<String>) -> Option<FillSpec> {
    match f.tag_name().name() {
        "noFill" => Some(FillSpec::None),
        "solidFill" => Some(color_spec_in(f).map_or(FillSpec::None, FillSpec::Solid)),
        "gradFill" => {
            let stops = child(f, "gsLst")
                .into_iter()
                .flat_map(|l| children(l, "gs"))
                .filter_map(|gs| Some((pct(attr_i64(gs, "pos")?), color_spec_in(gs)?)))
                .take(MAX_GRADIENT_STOPS)
                .collect();
            let ang = child(f, "lin").and_then(|l| attr_i64(l, "ang")).map_or(0.0, angle);
            Some(FillSpec::Gradient { angle: ang, stops })
        }
        "blipFill" => {
            let id = child(f, "blip").and_then(|b| attr_r(b, "embed"))?;
            Some(blip(id).map_or(FillSpec::None, FillSpec::Picture))
        }
        // Motif : approché par sa couleur de premier plan (signalé par `approximation`).
        "pattFill" => Some(child(f, "fgClr").and_then(color_spec_in).map_or(FillSpec::None, FillSpec::Solid)),
        _ => None,
    }
}

/// Rendu approché du remplissage de `parent`, à signaler.
pub(crate) fn approximation(parent: Node<'_, '_>) -> Option<&'static str> {
    let f = elements(parent).find(|n| FILLS.contains(&n.tag_name().name()))?;
    match f.tag_name().name() {
        "pattFill" => Some("pattFill"),
        "gradFill" if child(f, "path").is_some() => Some("gradFill path"),
        "gradFill" if child(f, "gsLst").is_some_and(|l| children(l, "gs").count() > MAX_GRADIENT_STOPS) => {
            Some("gradFill stops")
        }
        _ => None,
    }
}

/// Trait non résolu, partiellement spécifié ; se combine à celui hérité et à celui du style.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct LineSpec {
    pub width: Option<f64>,
    /// `Some(None)` : `noFill`.
    pub color: Option<Option<ColorSpec>>,
    pub dash: Option<Dash>,
}

impl LineSpec {
    /// `self` l'emporte sur `base`, champ par champ.
    pub(crate) fn over(self, base: &LineSpec) -> LineSpec {
        LineSpec {
            width: self.width.or(base.width),
            color: self.color.or_else(|| base.color.clone()),
            dash: self.dash.or(base.dash),
        }
    }

    /// `phClr` remplacé par `ph`.
    pub(crate) fn with_placeholder(&self, ph: Option<&ColorSpec>) -> LineSpec {
        let color = self.color.clone().map(|c| c.map(|c| c.with_placeholder(ph).unwrap_or(c)));
        LineSpec { color, ..self.clone() }
    }

    /// Le trait final dans `ctx` ; `None` s'il n'est pas peint.
    pub(crate) fn finish(&self, ctx: &ColorCtx) -> Option<Stroke> {
        let color = self.color.clone().flatten()?.resolve(ctx)?;
        Some(Stroke { width: self.width.unwrap_or(DEFAULT_LINE_PT), color, dash: self.dash.unwrap_or(Dash::Solid) })
    }
}

/// Lit un `a:ln` (et `lnL`/`lnR`/`lnT`/`lnB` de cellule).
pub(crate) fn parse_line(ln: Node<'_, '_>) -> LineSpec {
    let color = match elements(ln).find(|n| matches!(n.tag_name().name(), "noFill" | "solidFill" | "gradFill")) {
        Some(n) if n.tag_name().name() == "noFill" => Some(None),
        Some(n) if n.tag_name().name() == "solidFill" => color_spec_in(n).map(Some),
        Some(n) => child(n, "gsLst").and_then(|l| children(l, "gs").next()).and_then(color_spec_in).map(Some),
        None => None,
    };
    let dash = child(ln, "prstDash").and_then(|d| attr_str(d, "val")).map(|v| match v {
        "dash" | "sysDash" => Dash::Dash,
        "dot" | "sysDot" => Dash::Dot,
        "dashDot" | "sysDashDot" | "lgDashDot" | "lgDashDotDot" | "sysDashDotDot" => Dash::DashDot,
        "lgDash" => Dash::LongDash,
        _ => Dash::Solid,
    });
    LineSpec { width: attr_i64(ln, "w").map(emu).filter(|w| *w >= 0.0 && *w <= 1e4), color, dash }
}

#[cfg(test)]
#[path = "fill_tests.rs"]
mod fill_tests;
