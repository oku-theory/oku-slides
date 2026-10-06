//! Schéma des jetons de design : contrôle, bornes et section de chaque chemin.
//!
//! Le schéma (`theme_schema.json`) liste une entrée par feuille de
//! `default-tokens.json` (sauf `page.width` et `page.height`, fixés par la
//! grille 1920 × 1080), plus des entrées facultatives ([`TokenSpec::optional`]) :
//! les couleurs des variantes d'interface `app-dark` / `app-light` et
//! `component.chapter.number-color` (repli Typst : `ink`). Les couleurs propres à un thème (`brand-ink`…)
//! sont des noms libres sous `color` : [`spec`] leur donne un contrôle
//! [`Control::Color`] générique. [`validate`] vérifie une valeur avant de
//! l'écrire dans un brouillon de thème ; [`users`] et [`ensure_unused`]
//! disent quels jetons renvoient à un autre, avant de le retirer.

use std::collections::HashSet;
use std::sync::LazyLock;

use oku_slides_core::is_hex_color;
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::error::{InvalidTokenValueSnafu, Result, TokenInUseSnafu};

/// Contrôle de l'éditeur d'un jeton, et ses contraintes.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "control", rename_all = "kebab-case")]
pub enum Control {
    /// `#rgb`, `#rrggbb`, `#rrggbbaa`, ou nom d'une couleur de `color`.
    Color,
    /// Nombre en pt borné, ou (si `refs`) nom d'un jeton de cette catégorie.
    Pt {
        min: f64,
        max: f64,
        refs: Option<String>,
    },
    /// Nombre en em borné, ou (si `refs`) nom d'un jeton de cette catégorie.
    Em {
        min: f64,
        max: f64,
        refs: Option<String>,
    },
    /// Graisse 100 à 900 par 100, ou nom d'un jeton `weight`.
    Weight,
    /// Liste ordonnée et non vide de familles de police.
    Fonts,
    /// Une des valeurs listées.
    Choice {
        options: Vec<String>,
    },
    Bool,
    /// Chaîne libre (gabarits d'en-tête et de pied, marqueur de liste).
    Text,
    /// Liste non vide de couleurs (hex ou noms de `color`).
    Palette,
    /// Nombre sans unité borné ; `step` est le pas proposé par l'éditeur.
    Number {
        min: f64,
        max: f64,
        step: f64,
    },
    /// Filet de titre : `{width, height, color}` (pt, pt, couleur).
    Rule,
}

/// Entrée du schéma : un chemin de jeton, son contrôle et sa section.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TokenSpec {
    pub path: Vec<String>,
    pub section: Section,
    #[serde(flatten)]
    pub control: Control,
    /// `null` (« vider ») est permis.
    #[serde(default)]
    pub nullable: bool,
    /// Peut manquer dans `default-tokens.json` : variantes d'interface
    /// `app-dark` / `app-light`, champ que Typst lit avec une valeur de repli,
    /// couleurs libres sous `color`.
    #[serde(default)]
    pub optional: bool,
}

/// Section d'affichage d'un jeton dans l'éditeur de thème.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Section {
    Colors,
    Typography,
    Spacing,
    Shapes,
    Page,
    Charts,
    Components,
    AppLight,
    AppDark,
    Scripts,
}

/// Bornes en pt du filet de titre (largeur, hauteur).
pub(crate) const RULE_MAX: f64 = 400.0;

/// La valeur au chemin `path` des jetons `tokens`.
pub(crate) fn lookup<'a>(tokens: &'a Map<String, Value>, path: &[String]) -> Option<&'a Value> {
    let (last, groups) = path.split_last()?;
    let mut map = tokens;
    for g in groups {
        map = map.get(g)?.as_object()?;
    }
    map.get(last)
}

/// Le schéma tel que le dépôt le livre.
fn parse_schema() -> serde_json::Result<Vec<TokenSpec>> {
    serde_json::from_str(include_str!("theme_schema.json"))
}

/// Schéma lu une fois ; illisible (couvert par les tests), il est vide.
static SCHEMA: LazyLock<Vec<TokenSpec>> = LazyLock::new(|| {
    parse_schema().inspect_err(|e| tracing::error!(error = %e, "schéma des jetons illisible")).unwrap_or_default()
});

/// Contrôle des couleurs libres sous `color`.
static FREE_COLOR: LazyLock<TokenSpec> = LazyLock::new(|| TokenSpec {
    path: vec!["color".to_owned(), "*".to_owned()],
    section: Section::Colors,
    control: Control::Color,
    nullable: false,
    optional: true,
});

/// Les jetons par défaut (`default-tokens.json`), lus une fois ; illisibles
/// (couvert par les tests), vides.
static DEFAULT_TOKENS: LazyLock<Map<String, Value>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../typst/oku-slides/src/default-tokens.json"))
        .inspect_err(|e| tracing::error!(error = %e, "jetons par défaut illisibles"))
        .unwrap_or_default()
});

/// Les jetons par défaut, sous tout thème : la couche du bas.
pub fn default_tokens() -> &'static Map<String, Value> {
    &DEFAULT_TOKENS
}

/// Fusion profonde de `over` dans `base`, comme `merge` côté Typst.
pub fn merge(base: &mut Map<String, Value>, over: &Map<String, Value>) {
    for (k, v) in over {
        match (base.get_mut(k), v) {
            (Some(Value::Object(b)), Value::Object(o)) => merge(b, o),
            _ => {
                base.insert(k.clone(), v.clone());
            }
        }
    }
}

/// Toutes les entrées du schéma, dans l'ordre du fichier.
pub fn schema() -> &'static [TokenSpec] {
    &SCHEMA
}

/// Nom libre d'une couleur : `[a-z][a-z0-9-]*`.
fn is_color_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes.next().is_some_and(|b| b.is_ascii_lowercase())
        && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// L'entrée d'un chemin ; `color.<nom libre>` a l'entrée générique (chemin `color.*`).
pub fn spec(path: &[String]) -> Option<&'static TokenSpec> {
    schema().iter().find(|s| s.path == path).or_else(|| match path {
        [group, name] if group == "color" && is_color_name(name) => Some(&*FREE_COLOR),
        _ => None,
    })
}

/// Catégorie de jetons à laquelle une valeur de ce contrôle peut renvoyer par nom.
/// Les graisses de `weight.*` ne renvoient pas l'une à l'autre (Typst ne suit
/// pas de chaîne de graisses).
fn refers_to<'a>(control: &'a Control, path: &[String]) -> Option<&'a str> {
    match control {
        Control::Color | Control::Palette | Control::Rule => Some("color"),
        Control::Pt { refs, .. } | Control::Em { refs, .. } => refs.as_deref(),
        Control::Weight if path.first().is_some_and(|g| g != "weight") => Some("weight"),
        _ => None,
    }
}

/// Vérifie qu'une valeur convient au jeton `path` dans les jetons `tokens`
/// (le brouillon fusionné où elle vivra) : contrôle, bornes, références.
pub fn validate(path: &[String], value: &Value, tokens: &Map<String, Value>) -> Result<()> {
    check(path, value, tokens).map_err(|reason| InvalidTokenValueSnafu { path: path.join("."), reason }.build().into())
}

/// Raison du refus, traduite par `session-token-reason-<raison>`.
type Check = std::result::Result<(), &'static str>;

fn check(path: &[String], value: &Value, tokens: &Map<String, Value>) -> Check {
    let Some(spec) = spec(path) else {
        return Err(if matches!(path, [group, _] if group == "color") { "bad-name" } else { "unknown-token" });
    };
    if value.is_null() {
        return if spec.nullable { Ok(()) } else { Err("not-nullable") };
    }
    match &spec.control {
        Control::Color => check_color(value, tokens)?,
        Control::Pt { min, max, refs } | Control::Em { min, max, refs } => {
            check_number(value, *min, *max, refs.as_deref(), tokens)?
        }
        Control::Number { min, max, .. } => check_number(value, *min, *max, None, tokens)?,
        Control::Weight => check_weight(value, refers_to(&spec.control, path), tokens)?,
        Control::Fonts => {
            let families = value.as_array().filter(|a| !a.is_empty()).ok_or("bad-fonts")?;
            if !families.iter().all(|f| f.as_str().is_some_and(|f| !f.trim().is_empty())) {
                return Err("bad-fonts");
            }
        }
        Control::Choice { options } => {
            let choice = value.as_str().ok_or("wrong-type")?;
            if !options.iter().any(|o| o == choice) {
                return Err("not-a-choice");
            }
        }
        Control::Bool => {
            value.as_bool().ok_or("wrong-type")?;
        }
        Control::Text => {
            value.as_str().ok_or("wrong-type")?;
        }
        Control::Palette => {
            let entries = value.as_array().filter(|a| !a.is_empty()).ok_or("bad-palette")?;
            if !entries.iter().all(|e| check_color(e, tokens).is_ok()) {
                return Err("bad-palette");
            }
        }
        Control::Rule => check_rule(value, tokens).map_err(|_| "bad-rule")?,
    }
    check_loop(path, value, refers_to(&spec.control, path), tokens)
}

/// `name` est une clé de la catégorie `group` des jetons.
fn reference(tokens: &Map<String, Value>, group: &str, name: &str) -> Check {
    match tokens.get(group).and_then(Value::as_object).is_some_and(|m| m.contains_key(name)) {
        true => Ok(()),
        false => Err("unknown-reference"),
    }
}

fn check_color(value: &Value, tokens: &Map<String, Value>) -> Check {
    match value.as_str().ok_or("wrong-type")? {
        hex if hex.starts_with('#') => is_hex_color(hex).then_some(()).ok_or("not-a-color"),
        name => reference(tokens, "color", name),
    }
}

fn check_number(value: &Value, min: f64, max: f64, refs: Option<&str>, tokens: &Map<String, Value>) -> Check {
    match (value, refs) {
        (Value::String(name), Some(group)) => reference(tokens, group, name),
        _ => {
            let n = value.as_f64().ok_or("wrong-type")?;
            (min..=max).contains(&n).then_some(()).ok_or("out-of-range")
        }
    }
}

fn check_weight(value: &Value, refs: Option<&str>, tokens: &Map<String, Value>) -> Check {
    match (value, refs) {
        (Value::String(name), Some(group)) => reference(tokens, group, name),
        (Value::Number(n), _) => {
            // Un entier JSON : `text(weight:)` de Typst refuse un flottant (400.0).
            let w = n.as_u64().ok_or("not-a-weight")?;
            ((100..=900).contains(&w) && w % 100 == 0).then_some(()).ok_or("not-a-weight")
        }
        _ => Err("not-a-weight"),
    }
}

fn check_rule(value: &Value, tokens: &Map<String, Value>) -> Check {
    let rule = value.as_object().ok_or("wrong-type")?;
    for side in ["width", "height"] {
        check_number(rule.get(side).unwrap_or(&Value::Null), 0.0, RULE_MAX, None, tokens)?;
    }
    check_color(rule.get("color").unwrap_or(&Value::Null), tokens)
}

/// Un jeton de catégorie `group` qui renvoie à un autre de la même
/// catégorie (`color.muted` → `ink`, `size.lead` → `subtitle`) ne doit pas
/// boucler, une fois la valeur posée.
fn check_loop(path: &[String], value: &Value, refs: Option<&str>, tokens: &Map<String, Value>) -> Check {
    let [group, name] = path else { return Ok(()) };
    if refs != Some(group.as_str()) {
        return Ok(());
    }
    let Some(siblings) = tokens.get(group).and_then(Value::as_object) else { return Ok(()) };
    let mut seen = HashSet::from([name.as_str()]);
    let mut current = value;
    while let Some(next) = current.as_str().filter(|s| !s.starts_with('#')) {
        if !seen.insert(next) {
            return Err("reference-loop");
        }
        // Un nom inconnu plus loin dans la chaîne est l'affaire de son propre jeton.
        let Some(v) = siblings.get(next) else { return Ok(()) };
        current = v;
    }
    Ok(())
}

/// La valeur renvoie-t-elle au jeton `name` (selon son contrôle) ?
fn mentions(control: &Control, value: &Value, name: &str) -> bool {
    match control {
        Control::Palette => value.as_array().is_some_and(|a| a.iter().any(|e| e.as_str() == Some(name))),
        Control::Rule => value.get("color").and_then(Value::as_str) == Some(name),
        _ => value.as_str() == Some(name),
    }
}

/// Les jetons de `tokens` (chemins pointés, triés) qui renvoient à `path`
/// (`color.x`, `size.x`…) : même nom, et un contrôle qui renvoie à cette catégorie.
pub fn users(path: &[String], tokens: &Map<String, Value>) -> Vec<String> {
    let [group, name] = path else { return Vec::new() };
    let mut found = Vec::new();
    let mut prefix = Vec::new();
    walk(tokens, &mut prefix, &mut |p, value, spec| {
        if p != path && refers_to(&spec.control, p) == Some(group.as_str()) && mentions(&spec.control, value, name) {
            found.push(p.join("."));
        }
    });
    found.sort();
    found
}

/// Parcourt les jetons ; la descente s'arrête au premier chemin du schéma
/// (un filet de titre est une valeur, pas un groupe).
fn walk(
    tokens: &Map<String, Value>,
    prefix: &mut Vec<String>,
    visit: &mut dyn FnMut(&[String], &Value, &'static TokenSpec),
) {
    for (k, v) in tokens {
        prefix.push(k.clone());
        match (spec(prefix), v) {
            (Some(spec), _) => visit(prefix, v, spec),
            (None, Value::Object(m)) => walk(m, prefix, visit),
            (None, _) => {}
        }
        prefix.pop();
    }
}

/// Vérifie les valeurs d'un arbre de jetons `tokens` dans `effective` (où
/// elles vivront). `strict` : chaque feuille, un chemin inconnu refusé (un
/// brouillon de thème) ; sinon, seules les couleurs et palettes (surcharges
/// d'un deck, t-194).
pub fn validate_tree(tokens: &Map<String, Value>, effective: &Map<String, Value>, strict: bool) -> Result<()> {
    fn visit(
        tokens: &Map<String, Value>,
        prefix: &mut Vec<String>,
        effective: &Map<String, Value>,
        strict: bool,
    ) -> Result<()> {
        for (k, v) in tokens {
            prefix.push(k.clone());
            match (spec(prefix), v) {
                (Some(s), _) if strict || matches!(s.control, Control::Color | Control::Palette) => {
                    validate(prefix, v, effective)?
                }
                (Some(_), _) => {}
                (None, Value::Object(m)) => visit(m, prefix, effective, strict)?,
                (None, _) if strict => validate(prefix, v, effective)?,
                (None, _) => {}
            }
            prefix.pop();
        }
        Ok(())
    }
    visit(tokens, &mut Vec::new(), effective, strict)
}

/// Refuse le retrait de `path` si d'autres jetons y renvoient (ils sont nommés).
pub fn ensure_unused(path: &[String], tokens: &Map<String, Value>) -> Result<()> {
    let users = users(path, tokens);
    if users.is_empty() {
        return Ok(());
    }
    Err(TokenInUseSnafu { path: path.join("."), users }.build().into())
}

#[cfg(test)]
#[path = "theme_schema_tests.rs"]
mod theme_schema_tests;
