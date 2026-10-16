//! Le thème généré : les jetons d'un thème Oku tirés du thème source d'un
//! master, des polices retenues et des styles de texte de ce master.
//!
//! Un seul thème par deck : celui du master de la première slide
//! ([`theme_master`]). Les couleurs d'un autre master au thème différent
//! s'écrivent en hexadécimal ([`super::color::ThemeColors::write`]).
//!
//! | Source | Jetons |
//! |---|---|
//! | `clrScheme` | `color` : les 12 emplacements ([`SLOT_TOKENS`]), `on-accent` et `rule` dérivés |
//! | accents | `chart.palette` = `accent`, `accent-2`… `accent-6` (références) |
//! | `fontScheme` | `font.display` = majeure retenue, `font.body` = mineure retenue ; introuvable : le repli (`FontPlan::fallback`) ; puis leurs polices `ea` et `cs` retenues ([`role_families`]) |
//! | placeholders du master et de ses layouts, `txStyles` | `size.title`, `size.subtitle`, `size.body`, `size.small` |
//! | gras d'un run, texte maigre | `weight.strong` = 700, `weight.body` = 400 : les graisses de PowerPoint |
//! | puce (caractère) du premier niveau du corps du master | `component.list.marker`, `component.list.marker-color` (`null` : celle du texte) |
//! | premier tableau du deck | `component.table` : fonds d'en-tête, du corps et des bandes, couleur et graisse de l'en-tête, taille, filet ([`super::table`]) |
//! | page | `page.margin-*` nulles : la zone de corps est la page |
//!
//! Ce qui manque à la source (emplacement absent, police introuvable,
//! placeholder sans style) n'est pas écrit : le jeton par défaut du rendu
//! reste. `page.width` et `page.height` restent ceux du rendu (1920 × 1080,
//! voir [`super::page`]) ; les autres jetons `component.*` ne servent pas
//! aux widgets générés et restent aussi. Une puce numérotée ou absente
//! laisse la puce par défaut ; le retrait de la puce source n'est pas
//! repris : `component.list.indent` est l'écart de la puce au texte, hors
//! largeur de la puce, que la source ne donne pas.

use serde_json::{Map, Value, json};
use uuid::Uuid;

use oku_slides_core::ThemeManifest;

use super::color::{SLOT_TOKENS, ThemeColors};
use super::page::Page;
use super::table::{TableLook, first_table};
use crate::fonts::FontPlan;
use crate::retrofit::color::contrast;
use crate::source::{Bullet, Master, Placeholder, Rgba, Role, Script, SourceDeck, ThemeFonts};

/// Part de l'encre dans le filet (`rule`) : le « Arrière-plan 1, plus sombre
/// 15 % » de PowerPoint, le gris des séparateurs d'un thème Office (`#d9d9d9`
/// sur blanc), discret sur le fond et dans sa teinte.
const RULE_INK: f64 = 0.15;

/// Graisse d'un texte maigre de PowerPoint.
const WEIGHT_BODY: u16 = 400;
/// Graisse d'un texte gras de PowerPoint.
const WEIGHT_BOLD: u16 = 700;

/// Plus grande taille de police d'un jeton `size.*` (borne du schéma des thèmes), en pt.
const SIZE_MAX: f64 = 400.0;

/// Le master qui donne le thème : celui du layout de la première slide qui
/// en a un ; à défaut, le premier master.
pub(crate) fn theme_master(deck: &SourceDeck) -> Option<&Master> {
    let of_slide = deck.slides.iter().find_map(|s| {
        let layout = deck.layouts.iter().find(|l| Some(&l.id) == s.layout.as_ref())?;
        deck.masters.iter().find(|m| m.id == layout.master)
    });
    of_slide.or_else(|| deck.masters.first())
}

/// Les couleurs du thème généré pour `master` (vides si son thème manque).
pub(crate) fn theme_colors(deck: &SourceDeck, master: &Master) -> ThemeColors {
    deck.themes.get(master.theme).map(ThemeColors::new).unwrap_or_default()
}

/// Les jetons du thème généré pour `master`, aux polices de `fonts`, sur la page `page`.
pub(crate) fn theme_tokens(deck: &SourceDeck, master: &Master, fonts: &FontPlan, page: &Page) -> Map<String, Value> {
    let mut tokens = Map::new();
    let colors = theme_colors(deck, master);
    tokens.insert("color".to_owned(), Value::Object(color_tokens(&colors)));
    let palette = colors.palette();
    if !palette.is_empty() {
        tokens.insert("chart".to_owned(), json!({ "palette": palette }));
    }
    let font = font_tokens(deck.themes.get(master.theme).map(|t| &t.fonts), fonts);
    if !font.is_empty() {
        tokens.insert("font".to_owned(), Value::Object(font));
    }
    let size = size_tokens(deck, master, page);
    if !size.is_empty() {
        tokens.insert("size".to_owned(), Value::Object(size));
    }
    let mut component = Map::new();
    if let Some(list) = list_tokens(master, &colors) {
        component.insert("list".to_owned(), list);
    }
    if let Some(look) = first_table(deck).map(|t| TableLook::of(t, &colors, page)) {
        component.insert("table".to_owned(), Value::Object(look.tokens));
    }
    if !component.is_empty() {
        tokens.insert("component".to_owned(), Value::Object(component));
    }
    tokens.extend(bare_tokens());
    tokens
}

/// `font` : les familles des titres et du corps du thème `theme` ; sans
/// thème, le repli des familles introuvables ([`FontPlan::fallback`]) pour
/// les deux, que le texte sans famille prenne la même police que le reste.
fn font_tokens(theme: Option<&ThemeFonts>, fonts: &FontPlan) -> Map<String, Value> {
    let mut font = Map::new();
    for (role, major) in [("display", true), ("body", false)] {
        let families = match theme {
            Some(tf) => role_families(tf, major, fonts),
            None => fonts.fallback.as_deref().into_iter().collect(),
        };
        if !families.is_empty() {
            font.insert(role.to_owned(), json!(families));
        }
    }
    font
}

/// Les jetons d'un thème généré sans master : ceux de [`bare_tokens`] et les
/// polices de repli ([`font_tokens`]).
pub(crate) fn masterless_tokens(fonts: &FontPlan) -> Map<String, Value> {
    let mut tokens = bare_tokens();
    let font = font_tokens(None, fonts);
    if !font.is_empty() {
        tokens.insert("font".to_owned(), Value::Object(font));
    }
    tokens
}

/// Les jetons que tout thème généré pose, même sans master : graisses de
/// PowerPoint, marges de page nulles.
fn bare_tokens() -> Map<String, Value> {
    let mut tokens = Map::new();
    // Un run gras du texte riche prend `weight.strong` : la face grasse (700) de la source.
    tokens.insert("weight".to_owned(), json!({ "body": WEIGHT_BODY, "strong": WEIGHT_BOLD }));
    tokens.insert("page".to_owned(), json!({ "margin-x": 0, "margin-top": 0, "margin-bottom": 0 }));
    tokens
}

/// Le `theme.json` du thème généré (première version, sans parent).
pub(crate) fn theme_manifest(tokens: Map<String, Value>, id: Uuid, name: &str, created: &str) -> ThemeManifest {
    ThemeManifest {
        id,
        name: name.to_owned(),
        description: String::new(),
        parent: None,
        created: created.to_owned(),
        tokens,
    }
}

/// Les familles retenues des titres (`major`) ou du corps du thème `tf` : la
/// latine (introuvable : le repli, le corps sinon le repli final), puis celles
/// d'Asie de l'Est et des écritures complexes que le thème nomme et que
/// `fonts` retient, sans doublon. Typst prend chaque caractère dans la
/// première qui l'a. Vide sans police latine retenue ni repli.
pub(crate) fn role_families<'p>(tf: &ThemeFonts, major: bool, fonts: &'p FontPlan) -> Vec<&'p str> {
    let latin = tf.family(major, Script::Latin).and_then(|f| fonts.used(f)).or(fonts.fallback.as_deref());
    let Some(latin) = latin else { return Vec::new() };
    let mut out = vec![latin];
    for script in [Script::EastAsian, Script::Complex] {
        if let Some(used) = tf.family(major, script).and_then(|f| fonts.used(f))
            && !out.contains(&used)
        {
            out.push(used);
        }
    }
    out
}

/// `color` : un jeton par emplacement présent, puis `on-accent` et `rule`.
fn color_tokens(colors: &ThemeColors) -> Map<String, Value> {
    let mut out = Map::new();
    for (_, token) in SLOT_TOKENS {
        if let Some(c) = colors.get(token) {
            out.insert(token.to_owned(), Value::String(c.to_hex()));
        }
    }
    let (Some(bg), Some(ink)) = (colors.get("bg"), colors.get("ink")) else { return out };
    if let Some(accent) = colors.get("accent") {
        // Le texte sur l'accent : le fond ou l'encre, celui qui contraste le plus.
        let on = if contrast(bg, accent) >= contrast(ink, accent) { "bg" } else { "ink" };
        out.insert("on-accent".to_owned(), Value::String(on.to_owned()));
    }
    out.insert("rule".to_owned(), Value::String(mix(bg, ink, RULE_INK).to_hex()));
    out
}

/// `a` mêlée de `b` à la part `t` (sRGB, opaque).
fn mix(a: Rgba, b: Rgba, t: f64) -> Rgba {
    let channel = |x: u8, y: u8| {
        let v = f64::from(x) + (f64::from(y) - f64::from(x)) * t;
        // `as` sature : le mélange reste dans 0..=255.
        v.round() as u8
    };
    Rgba::new(channel(a.r, b.r), channel(a.g, b.g), channel(a.b, b.b))
}

/// `size.*`, en pt page : le premier niveau des placeholders du master
/// (titre, corps, pied) et du sous-titre de ses layouts ; sans placeholder,
/// les `txStyles` du master pour le titre et le corps.
fn size_tokens(deck: &SourceDeck, master: &Master, page: &Page) -> Map<String, Value> {
    let subtitles = deck.layouts.iter().filter(|l| l.master == master.id).flat_map(|l| &l.placeholders);
    let styles = &master.text_styles;
    let sources = [
        ("title", first_level(&master.placeholders, &[Role::Title]).or(Some(styles.title.run.size))),
        ("subtitle", first_level(subtitles, &[Role::Subtitle])),
        ("body", first_level(&master.placeholders, &[Role::Body, Role::Object]).or(Some(styles.body.run.size))),
        ("small", first_level(&master.placeholders, &[Role::Footer, Role::Date, Role::SlideNum])),
    ];
    sources
        .into_iter()
        .filter_map(|(name, pt)| {
            let pt = pt.filter(|v| v.is_finite() && *v > 0.0)?;
            Some((name.to_owned(), json!(page.len(pt).min(SIZE_MAX))))
        })
        .collect()
}

/// `component.list` : la puce du premier niveau du corps du master (son
/// placeholder de corps, sinon ses `txStyles`), si c'est un caractère ; sa
/// couleur propre, sinon `null` (la couleur du texte).
fn list_tokens(master: &Master, colors: &ThemeColors) -> Option<Value> {
    let body = master.placeholders.iter().find(|p| matches!(p.role, Role::Body | Role::Object));
    let level = body.and_then(|p| p.levels.first()).unwrap_or(&master.text_styles.body);
    let Bullet::Char { char, color, .. } = &level.para.bullet else { return None };
    let marker_color = color.as_ref().map(|c| colors.write(c));
    Some(json!({ "marker": char, "marker-color": marker_color }))
}

/// La taille du premier niveau du premier placeholder de `phs` d'un des rôles `roles`, en pt source.
fn first_level<'a>(phs: impl IntoIterator<Item = &'a Placeholder>, roles: &[Role]) -> Option<f64> {
    phs.into_iter().find(|p| roles.contains(&p.role)).and_then(|p| p.levels.first()).map(|l| l.run.size)
}

#[cfg(test)]
#[path = "theme_tests.rs"]
mod theme_tests;
