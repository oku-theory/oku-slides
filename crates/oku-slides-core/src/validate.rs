//! Validation sémantique d'un deck (au-delà de la désérialisation), par le
//! registre des widgets : chaque slide et chaque élément est contrôlé contre
//! le schéma de son `kind`.

use std::collections::{HashMap, HashSet};

use oku_slides_i18n::{Localize, Localizer, Message};

use crate::locate::MAX_SLOT_DEPTH;
use crate::model::{Deck, DeckParams, Grid, Paragraph};
use crate::params::{ParamValue, Params};
use crate::widget::{Category, HostType, ParamType, SlotMode, WidgetRegistry, WidgetSchema};

/// Un problème du deck : où, et quoi.
#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub slide: Option<String>,
    pub element: Option<String>,
    pub message: Message,
}

impl Localize for Issue {
    fn localize(&self, l: &Localizer) -> String {
        let message = self.message.localize(l);
        match (&self.slide, &self.element) {
            (Some(s), Some(e)) => format!("[{s}/{e}] {message}"),
            (Some(s), None) => format!("[{s}] {message}"),
            _ => message,
        }
    }
}

impl std::fmt::Display for Issue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.localize(oku_slides_i18n::english()))
    }
}

/// Vérifie le deck contre `registry`. `assets` : identifiants disponibles,
/// si on veut les contrôler.
pub fn validate(deck: &Deck, registry: &WidgetRegistry, assets: Option<&HashSet<String>>) -> Vec<Issue> {
    let mut v = Validator { deck, registry, assets, out: Vec::new(), binds_checked: HashSet::new() };
    let mut slide_ids = HashSet::new();
    for s in &deck.slides {
        let at = Place { slide: &s.id, element: None };
        if s.id.is_empty() {
            v.push(at, Message::new("issue-empty-slide-id"));
        }
        if !slide_ids.insert(s.id.as_str()) {
            v.push(at, Message::new("issue-duplicate-slide-id"));
        }
        let slide_schema = v.widget(at, &s.kind, Category::Slide);
        v.params(at, slide_schema, &s.params);
        v.assets(at, slide_schema, &s.params);
        v.rich(at, &s.params);
        v.binds(at, slide_schema);
        let grid = s.grid();
        // Genre de chaque propriétaire de slot déjà vu (parents avant enfants).
        let mut kinds: HashMap<&str, &str> = HashMap::new();
        let mut seen = HashSet::new();
        for l in s.walk() {
            let e = l.element;
            let at = Place { slide: &s.id, element: Some(&e.id) };
            let schema = v.widget(at, &e.kind, Category::Element);
            v.params(at, schema, &e.params);
            let owner_kind = match l.owner {
                None => Some(s.kind.as_str()),
                Some(id) => kinds.get(id).copied(),
            };
            let slot =
                owner_kind.and_then(|k| registry.get(k)).and_then(|w| w.param(l.slot)).and_then(|p| match &p.ty {
                    ParamType::Slot { mode, accepts } => Some((*mode, accepts.as_deref())),
                    _ => None,
                });
            if let Some((_, Some(accepts))) = slot
                && !accepts.contains(&e.kind)
            {
                v.push(at, Message::new("issue-slot-rejects").arg("kind", e.kind.as_str()));
            }
            if e.id.is_empty() {
                v.push(at, Message::new("issue-empty-element-id"));
            }
            if !seen.insert(e.id.as_str()) {
                v.push(at, Message::new("issue-duplicate-element-id"));
            }
            match slot.map(|(mode, _)| mode) {
                Some(SlotMode::Free) if !l.is_free() => v.push(at, Message::new("issue-free-without-frame")),
                Some(SlotMode::Flow) if l.is_free() => v.push(at, Message::new("issue-placed-in-flow")),
                _ => {}
            }
            if let Some(f) = e.frame
                && !(f.w > 0.0 && f.h > 0.0)
            {
                v.push(at, Message::new("issue-empty-frame"));
            }
            // Seuls les éléments des slots libres de la slide s'accrochent à sa grille.
            if let Some(c) = e.cell
                && l.owner.is_none()
                && matches!(slot, Some((SlotMode::Free, _)))
                && !c.fits(&grid)
            {
                v.push(at, Message::new("issue-cell-outside-grid"));
            }
            v.assets(at, schema, &e.params);
            v.rich(at, &e.params);
            v.binds(at, schema);
            // Même borne que les ops : un élément à la profondeur `depth` est
            // dans `depth + 1` slots, au plus `MAX_SLOT_DEPTH`.
            if l.depth >= MAX_SLOT_DEPTH {
                v.push(at, Message::new("issue-slot-too-deep"));
            }
            kinds.insert(e.id.as_str(), e.kind.as_str());
        }
    }
    v.out
}

/// Où se trouve un problème.
#[derive(Clone, Copy)]
struct Place<'a> {
    slide: &'a str,
    element: Option<&'a str>,
}

struct Validator<'a> {
    deck: &'a Deck,
    registry: &'a WidgetRegistry,
    assets: Option<&'a HashSet<String>>,
    out: Vec<Issue>,
    /// Widgets dont les `bind` ont déjà été contrôlés.
    binds_checked: HashSet<&'a str>,
}

impl<'a> Validator<'a> {
    fn push(&mut self, at: Place<'_>, message: Message) {
        self.out.push(Issue { slide: Some(at.slide.to_owned()), element: at.element.map(str::to_owned), message });
    }

    /// Le schéma de `kind`, s'il est connu ; signale un widget inconnu ou
    /// d'une autre catégorie.
    fn widget(&mut self, at: Place<'_>, kind: &str, category: Category) -> Option<&'a WidgetSchema> {
        let Some(schema) = self.registry.get(kind) else {
            self.push(at, Message::new("issue-unknown-widget").arg("kind", kind));
            return None;
        };
        if schema.category != category {
            self.push(at, Message::new("issue-wrong-category").arg("kind", kind));
        }
        Some(schema)
    }

    /// Chaque paramètre est déclaré et de la forme de son type. Les
    /// paramètres d'un widget inconnu ne sont pas contrôlés.
    fn params(&mut self, at: Place<'_>, schema: Option<&WidgetSchema>, params: &Params) {
        let Some(schema) = schema else { return };
        for (name, value) in params.iter() {
            let Some(p) = schema.param(name) else {
                self.push(at, Message::new("issue-unknown-param").arg("param", name));
                continue;
            };
            match fits(&p.ty, value) {
                Fit::Ok => {}
                Fit::WrongType => self.push(at, Message::new("issue-param-type").arg("param", name)),
                Fit::OutOfChoice => self.push(at, Message::new("issue-choice-out-of-range").arg("param", name)),
            }
        }
    }

    /// Les assets des paramètres `asset` sont dans les sources.
    fn assets(&mut self, at: Place<'_>, schema: Option<&WidgetSchema>, params: &Params) {
        let (Some(schema), Some(assets)) = (schema, self.assets) else { return };
        for p in schema.params.iter().filter(|p| p.ty == ParamType::Asset) {
            if let Some(asset) = params.text(&p.name)
                && !assets.contains(asset)
            {
                self.push(at, Message::new("issue-missing-asset").arg("asset", asset));
            }
        }
    }

    /// Liens et pastilles de tous les textes riches.
    fn rich(&mut self, at: Place<'_>, params: &Params) {
        let runs = || params.riches().flat_map(|(_, ps)| ps).flat_map(|p: &Paragraph| &p.runs);
        for link in runs().filter_map(|r| r.link.as_deref()).filter(|l| !is_safe_link(l)) {
            self.push(at, Message::new("issue-unsupported-link").arg("link", link));
        }
        if runs().any(|r| r.highlight == Some(0)) {
            self.push(at, Message::new("issue-highlight-zero"));
        }
    }

    /// Chaque `bind` du widget vise un paramètre du deck ; une fois par widget.
    fn binds(&mut self, at: Place<'_>, schema: Option<&'a WidgetSchema>) {
        let Some(schema) = schema else { return };
        if !self.binds_checked.insert(schema.name.as_str()) {
            return;
        }
        for p in &schema.params {
            if let Some(key) = &p.bind
                && !is_deck_param(&self.deck.params, key)
            {
                self.push(at, Message::new("issue-unknown-bind").arg("param", p.name.as_str()));
            }
        }
    }
}

/// Un paramètre lié vise un champ nommé de [`DeckParams`] ou une clé de `extra`.
fn is_deck_param(params: &DeckParams, key: &str) -> bool {
    DeckParams::is_field(key) || params.extra.contains_key(key)
}

/// Accord d'une valeur avec le type de son paramètre.
pub(crate) enum Fit {
    Ok,
    WrongType,
    OutOfChoice,
}

/// La valeur a-t-elle la forme de son type ? Partagé avec les ops, qui
/// contrôlent ainsi les sous-arbres insérés.
pub(crate) fn fits(ty: &ParamType, value: &ParamValue) -> Fit {
    let ok = match (ty, value) {
        (ParamType::Choice { values }, ParamValue::Text(s)) => {
            return if values.iter().any(|v| v == s) { Fit::Ok } else { Fit::OutOfChoice };
        }
        (ParamType::Plain | ParamType::Asset, ParamValue::Text(_)) => true,
        (ParamType::Rich, ParamValue::Rich(_)) | (ParamType::Slot { .. }, ParamValue::Slot(_)) => true,
        (ParamType::Rich, ParamValue::Slot(s)) => s.is_empty(),
        (ParamType::Slot { .. }, ParamValue::Rich(p)) => p.is_empty(),
        (ParamType::Flag, ParamValue::Value(v)) => v.is_boolean(),
        (ParamType::Number { min, max }, ParamValue::Value(v)) => {
            v.as_f64().is_some_and(|n| !min.is_some_and(|m| n < m) && !max.is_some_and(|m| n > m))
        }
        (ParamType::Grid, ParamValue::Value(v)) => v.is_object() && serde_json::from_value::<Grid>(v.clone()).is_ok(),
        (ParamType::Host { host: HostType::Chart }, ParamValue::Value(v)) => {
            serde_json::from_value::<oku_slides_chart::ChartOption>(v.clone())
                .is_ok_and(|o| oku_slides_chart::normalize(&o, oku_slides_chart::ChartLabels::english()).is_ok())
        }
        _ => false,
    };
    if ok { Fit::Ok } else { Fit::WrongType }
}

/// Schémas de lien acceptés : web et courriel, rien qui s'exécute.
fn is_safe_link(link: &str) -> bool {
    let lower = link.to_ascii_lowercase();
    ["http://", "https://", "mailto:"].iter().any(|scheme| lower.starts_with(scheme))
}

#[cfg(test)]
#[path = "validate_tests.rs"]
mod validate_tests;
