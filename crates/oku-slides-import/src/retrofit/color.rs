//! Couleurs source projetées sur les jetons `color` du thème cible.
//!
//! Le retrofit garde le style du thème cible : il n'émet jamais de `#rrggbb`,
//! seulement le nom d'une couleur existante du thème (le rendu Typst panique
//! sur un nom inconnu). Une couleur source devient, dans l'ordre :
//!
//! 1. rien, si elle est la valeur par défaut du lecteur (`Origin::Default`) :
//!    le thème cible décide ;
//! 2. le jeton de son emplacement de thème ([`slot_token`]), s'il existe et
//!    qu'aucun modificateur ne l'altère ;
//! 3. l'accent déduit du deck ([`ColorTokens::with_accents`]) ;
//! 4. `ink` ou `bg` pour un noir ou un blanc neutres ([`neutral_token`]) ;
//! 5. le jeton le plus proche en OKLab si `ΔE ≤ MAX_DELTA_E` ;
//! 6. sinon elle est abandonnée : perte [`LossKind::ColorDropped`].
//!
//! L'opacité est ignorée : les jetons sont opaques. La lisibilité d'un texte
//! se mesure par le contraste WCAG ([`contrast`]) : un jeton de texte sous
//! [`MIN_CONTRAST`] contre son fond n'est jamais émis ([`text_color`]).

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::report::LossKind;
use crate::source::{Color, Fill, Item, ItemKind, Origin, Rgba, SourceDeck};

/// Écart OKLab (×100) au-delà duquel une couleur n'a pas de jeton proche.
///
/// Une différence juste perceptible vaut environ 2 ; à 12, une couleur garde
/// un jeton de même famille (un bleu foncé `#1f4e79` → `ink` de Pro, 6,3)
/// mais n'en prend pas un d'une autre teinte (un rouge pur → `ink-muted` de
/// Papier, 27,9 ; un orange `#ed7d31` → `rule`, 23,1).
pub(crate) const MAX_DELTA_E: f64 = 12.0;

/// Chroma OKLab sous laquelle une couleur est neutre (un gris).
const NEUTRAL_CHROMA: f64 = 0.02;

/// Clarté OKLab sous laquelle un neutre est un noir d'encre.
const BLACK_LIGHTNESS: f64 = 0.25;

/// Clarté OKLab au-dessus de laquelle un neutre est un blanc de fond.
const WHITE_LIGHTNESS: f64 = 0.95;

/// Profondeur maximale d'une chaîne d'alias entre jetons de couleur.
const MAX_ALIAS_DEPTH: usize = 16;

/// Contraste WCAG minimal d'un texte émis contre son fond (grand texte, AA).
pub(crate) const MIN_CONTRAST: f64 = 3.0;

/// Chroma OKLab à partir de laquelle une couleur du deck compte comme accent.
const ACCENT_CHROMA: f64 = 0.08;

/// Jetons d'un texte clair sur fond sombre, dans l'ordre de préférence.
const ON_DARK: [&str; 3] = ["on-dark-body", "white", "on-accent"];

/// Une couleur dans l'espace OKLab (Björn Ottosson, 2020).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Oklab {
    /// Clarté, 0 (noir) à 1 (blanc).
    pub l: f64,
    /// Axe vert (négatif) – rouge (positif).
    pub a: f64,
    /// Axe bleu (négatif) – jaune (positif).
    pub b: f64,
}

impl Oklab {
    /// Chroma : distance à l'axe des gris.
    pub(crate) fn chroma(&self) -> f64 {
        self.a.hypot(self.b)
    }
}

/// Composante sRGB 8 bits → linéaire.
fn linear(c: u8) -> f64 {
    let c = f64::from(c) / 255.0;
    if c <= 0.040_45 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

/// `c` en OKLab (opacité ignorée).
pub(crate) fn oklab(c: Rgba) -> Oklab {
    let (r, g, b) = (linear(c.r), linear(c.g), linear(c.b));
    let l = (0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b).cbrt();
    let m = (0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b).cbrt();
    let s = (0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b).cbrt();
    Oklab {
        l: 0.210_454_255_3 * l + 0.793_617_785_0 * m - 0.004_072_046_8 * s,
        a: 1.977_998_495_1 * l - 2.428_592_205_0 * m + 0.450_593_709_9 * s,
        b: 0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766_0 * s,
    }
}

/// Écart entre deux couleurs : distance euclidienne OKLab, ×100.
pub(crate) fn delta_e(x: Oklab, y: Oklab) -> f64 {
    100.0 * ((x.l - y.l).powi(2) + (x.a - y.a).powi(2) + (x.b - y.b).powi(2)).sqrt()
}

/// Luminance relative WCAG de `c` (opacité ignorée).
fn luminance(c: Rgba) -> f64 {
    0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b)
}

/// Contraste WCAG entre deux couleurs : 1 (identiques) à 21 (noir sur blanc).
pub(crate) fn contrast(a: Rgba, b: Rgba) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// Un fond sombre : un texte blanc y contraste plus qu'un texte noir.
pub(crate) fn is_dark(c: Rgba) -> bool {
    contrast(c, Rgba::new(255, 255, 255)) > contrast(c, Rgba::new(0, 0, 0))
}

/// La couleur `name` du groupe `color`, alias résolus ; `None` pour un nom
/// inconnu, une boucle d'alias ou une autre valeur.
fn resolve(color: &Map<String, Value>, name: &str) -> Option<Rgba> {
    let mut v = color.get(name)?;
    for _ in 0..MAX_ALIAS_DEPTH {
        let s = v.as_str()?;
        if s.starts_with('#') {
            return parse_hex(s);
        }
        v = color.get(s)?;
    }
    None
}

/// Les couleurs du thème cible, par nom de jeton, alias résolus ; les fonds
/// d'une carte ; les accents déduits du deck source.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ColorTokens {
    colors: BTreeMap<String, Rgba>,
    /// Fond d'une carte (`component.card.bg`), puis d'une carte en relief (`highlight-bg`).
    cards: [Option<Rgba>; 2],
    /// Couleurs du deck ramenées à un accent, par ordre d'usage.
    accents: Vec<(Rgba, String)>,
}

impl ColorTokens {
    /// Depuis les jetons effectifs du thème (défauts fusionnés) : le groupe
    /// `color`, dont une valeur est `#rgb`, `#rrggbb`, `#rrggbbaa` ou le nom
    /// d'un autre jeton de couleur (un nom inconnu, une boucle d'alias ou une
    /// autre valeur : jeton écarté) ; les fonds `component.card.bg` et
    /// `component.card.highlight-bg`, des noms de couleur.
    pub fn from_tokens(tokens: &Map<String, Value>) -> Self {
        let empty = Map::new();
        let color = tokens.get("color").and_then(Value::as_object).unwrap_or(&empty);
        let card = tokens.get("component").and_then(|c| c.get("card"));
        let card_color =
            |key: &str| card.and_then(|c| c.get(key)).and_then(Value::as_str).and_then(|n| resolve(color, n));
        Self {
            colors: color.keys().filter_map(|k| resolve(color, k).map(|c| (k.clone(), c))).collect(),
            cards: [card_color("bg"), card_color("highlight-bg")],
            accents: Vec::new(),
        }
    }

    /// La couleur du jeton `name`.
    pub fn get(&self, name: &str) -> Option<Rgba> {
        self.colors.get(name).copied()
    }

    /// Vrai si le thème a un jeton `name`.
    pub fn contains(&self, name: &str) -> bool {
        self.colors.contains_key(name)
    }

    /// Le fond d'une carte, en relief (`highlight`) ou non ; repli : `ink`
    /// pour le relief, `bg` sinon (les défauts du rendu), puis blanc.
    pub(crate) fn card(&self, highlight: bool) -> Rgba {
        let (own, fallback) = if highlight { (self.cards[1], "ink") } else { (self.cards[0], "bg") };
        own.or_else(|| self.get(fallback)).unwrap_or(Rgba::new(255, 255, 255))
    }

    /// Le fond de page (`bg`) ; repli : blanc.
    pub(crate) fn page(&self) -> Rgba {
        self.get("bg").unwrap_or(Rgba::new(255, 255, 255))
    }

    /// Le premier jeton de texte clair ([`ON_DARK`]) lisible sur `bg`.
    pub(crate) fn on_dark(&self, bg: Rgba) -> Option<&'static str> {
        ON_DARK.into_iter().find(|t| self.get(t).is_some_and(|c| contrast(c, bg) >= MIN_CONTRAST))
    }

    /// Le jeton le plus proche de `c` et son écart ; à égalité, le premier par nom.
    pub(crate) fn nearest(&self, c: Rgba) -> Option<(&str, f64)> {
        let target = oklab(c);
        let mut best: Option<(&str, f64)> = None;
        for (name, token) in &self.colors {
            let d = delta_e(target, oklab(*token));
            if best.is_none_or(|(_, b)| d < b) {
                best = Some((name, d));
            }
        }
        best
    }

    /// Les accents déduits de `deck` : ses couleurs explicites saturées
    /// (chroma OKLab ≥ 0,08) sans jeton proche (`ΔE > MAX_DELTA_E`), rangées
    /// par usage décroissant ([`usage`] ; à égalité, par valeur hexadécimale),
    /// prennent dans l'ordre `accent` puis les autres clés `accent-*` du thème
    /// (par nom). Déterministe ; seules des clés existantes sont prises.
    pub fn with_accents(mut self, deck: &SourceDeck) -> Self {
        let mut used: BTreeMap<String, (Rgba, f64)> = BTreeMap::new();
        for slide in &deck.slides {
            for item in &slide.items {
                usage(item, &mut |c, weight| {
                    let c = Rgba { a: 255, ..c };
                    let e = used.entry(c.to_hex()).or_insert((c, 0.0));
                    e.1 += weight;
                });
            }
        }
        let mut ranked: Vec<(String, Rgba, f64)> = used
            .into_iter()
            .map(|(hex, (c, w))| (hex, c, w))
            .filter(|(_, c, _)| {
                oklab(*c).chroma() >= ACCENT_CHROMA && self.nearest(*c).is_none_or(|(_, d)| d > MAX_DELTA_E)
            })
            .collect();
        ranked.sort_by(|a, b| b.2.total_cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
        let keys = ["accent"]
            .into_iter()
            .filter(|k| self.contains(k))
            .map(str::to_owned)
            .chain(self.colors.keys().filter(|k| k.starts_with("accent-")).cloned());
        self.accents = ranked.into_iter().zip(keys).map(|((_, c, _), key)| (c, key)).collect();
        self
    }

    /// L'accent déduit pour la couleur `c`.
    fn accent(&self, c: Rgba) -> Option<&str> {
        let c = Rgba { a: 255, ..c };
        self.accents.iter().find(|(a, _)| Rgba { a: 255, ..*a } == c).map(|(_, k)| k.as_str())
    }
}

/// Appelle `f` pour chaque couleur explicite de `item` (et de ses enfants),
/// avec son poids d'usage en pt² : caractères × taille² pour un run de texte,
/// surface pour un fond uni, longueur × épaisseur pour un trait.
fn usage(item: &Item, f: &mut impl FnMut(Rgba, f64)) {
    let explicit = |c: &Color| matches!(c.origin, Origin::Explicit).then_some(c.value);
    if let Some(tb) = item.text() {
        for r in tb.paragraphs.iter().flat_map(|p| &p.runs) {
            if let Some(c) = explicit(&r.style.color) {
                // Le nombre de caractères d'un run est borné par `clip` (lot 1) : la conversion est exacte.
                f(c, r.text.chars().count() as f64 * r.style.size * r.style.size);
            }
        }
    }
    let frame = &item.frame;
    match &item.kind {
        ItemKind::Shape(s) => {
            if let Fill::Solid { color } = &s.fill
                && let Some(c) = explicit(color)
            {
                f(c, frame.w * frame.h);
            }
            if let Some(st) = &s.stroke
                && let Some(c) = explicit(&st.color)
            {
                f(c, 2.0 * (frame.w + frame.h) * st.width);
            }
        }
        ItemKind::Group { items, .. } => {
            for child in items {
                usage(child, f);
            }
        }
        _ => {}
    }
}

/// `#rgb`, `#rrggbb` ou `#rrggbbaa` ; `None` pour toute autre forme.
fn parse_hex(s: &str) -> Option<Rgba> {
    let h = s.strip_prefix('#')?;
    if !h.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
    let nibble = |i: usize| u8::from_str_radix(h.get(i..=i)?, 16).ok().map(|n| n * 17);
    match h.len() {
        3 => Some(Rgba::new(nibble(0)?, nibble(1)?, nibble(2)?)),
        6 => Some(Rgba::new(byte(0)?, byte(2)?, byte(4)?)),
        8 => Some(Rgba { r: byte(0)?, g: byte(2)?, b: byte(4)?, a: byte(6)? }),
        _ => None,
    }
}

/// Le jeton fixe d'un emplacement du thème source ; `None` pour les autres
/// (`accent2..6`, `folHlink` : couleur la plus proche).
pub(crate) fn slot_token(slot: &str) -> Option<&'static str> {
    match slot {
        "dk1" => Some("ink"),
        "lt1" => Some("bg"),
        "dk2" => Some("ink-muted"),
        "lt2" => Some("surface"),
        "accent1" | "hlink" => Some("accent"),
        _ => None,
    }
}

/// `ink` pour un noir neutre, `bg` pour un blanc neutre : un thème clair ne les
/// reprend jamais à l'identique (l'encre de Papier, `#2e2b26`, est à 29 d'un noir pur).
fn neutral_token(c: Oklab) -> Option<&'static str> {
    if c.chroma() >= NEUTRAL_CHROMA {
        return None;
    }
    if c.l < BLACK_LIGHTNESS {
        Some("ink")
    } else if c.l > WHITE_LIGHTNESS {
        Some("bg")
    } else {
        None
    }
}

/// La couleur source `c` en nom de jeton du thème cible.
///
/// `Ok(None)` : valeur par défaut du lecteur, le thème cible décide ;
/// `Ok(Some(nom))` : un jeton existant ; `Err(ColorDropped(hex))` : aucun jeton proche.
pub(crate) fn project_color(c: &Color, tokens: &ColorTokens) -> Result<Option<String>, LossKind> {
    if let Origin::Default = c.origin {
        return Ok(None);
    }
    if let Origin::Scheme { slot, mods, .. } = &c.origin
        && mods.is_empty()
        && let Some(token) = slot_token(slot).filter(|t| tokens.contains(t))
    {
        return Ok(Some(token.to_owned()));
    }
    if let Some(token) = tokens.accent(c.value) {
        return Ok(Some(token.to_owned()));
    }
    let lab = oklab(c.value);
    if let Some(token) = neutral_token(lab).filter(|t| tokens.contains(t)) {
        return Ok(Some(token.to_owned()));
    }
    match tokens.nearest(c.value) {
        Some((name, d)) if d <= MAX_DELTA_E => Ok(Some(name.to_owned())),
        _ => Err(LossKind::ColorDropped(c.value.to_hex())),
    }
}

/// Le jeton d'un texte de couleur `c` posé sur le fond `bg`, s'il y est lisible.
///
/// Le jeton projeté ([`project_color`]) est gardé s'il contraste d'au moins
/// [`MIN_CONTRAST`] avec `bg`. Sinon, un texte plus clair que son fond sombre
/// prend le premier jeton de texte clair lisible ([`ON_DARK`]) ; à défaut, rien
/// (la couleur du thème). Jamais un jeton sous [`MIN_CONTRAST`].
pub(crate) fn text_color(c: &Color, bg: Rgba, tokens: &ColorTokens) -> Result<Option<String>, LossKind> {
    let Some(token) = project_color(c, tokens)? else { return Ok(None) };
    if tokens.get(&token).is_some_and(|t| contrast(t, bg) >= MIN_CONTRAST) {
        return Ok(Some(token));
    }
    let lighter = luminance(c.value) > luminance(bg);
    Ok((lighter && is_dark(bg)).then(|| tokens.on_dark(bg)).flatten().map(str::to_owned))
}

#[cfg(test)]
#[path = "color_tests.rs"]
mod color_tests;
