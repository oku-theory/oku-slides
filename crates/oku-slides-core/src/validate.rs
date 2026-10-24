//! Validation sémantique d'un deck (au-delà de la désérialisation), par le
//! registre des widgets : chaque slide et chaque élément est contrôlé contre
//! le schéma de son `kind`.

use std::collections::{BTreeSet, HashMap, HashSet};

use oku_slides_i18n::{Localize, Localizer, Message};

use crate::locate::MAX_SLOT_DEPTH;
use crate::model::{Deck, DeckParams, Grid, Paragraph};
use crate::namespace::Namespace;
use crate::ops::ListSpec;
use crate::pack::PackVersion;
use crate::params::{ListItem, ParamValue, Params};
use crate::widget::{Category, ParamType, SlotMode, WidgetRegistry, WidgetSchema};

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

impl Issue {
    /// Une version de pack écartée : son contenu ne donne pas son empreinte.
    pub fn pack_version_mismatch(hex: &str) -> Self {
        Self { slide: None, element: None, message: Message::new("issue-pack-version-mismatch").arg("version", hex) }
    }

    /// La version de pack du deck manque dans ses sources.
    pub fn pack_version_missing(version: &PackVersion) -> Self {
        let message = Message::new("issue-pack-version-missing").arg("version", version.as_str());
        Self { slide: None, element: None, message }
    }

    /// Le manifeste embarqué d'un pack ne correspond pas à son épinglage dans le deck.
    pub fn pack_pin_mismatch(ns: &Namespace) -> Self {
        Self { slide: None, element: None, message: Message::new("issue-pack-pin-mismatch").arg("pack", ns.as_str()) }
    }

    /// Une entrée `tokens` du manifeste embarqué d'un pack est refusée ; `reason` dit laquelle.
    pub fn pack_tokens_invalid(ns: &Namespace, reason: &str) -> Self {
        let message = Message::new("issue-pack-tokens-invalid").arg("pack", ns.as_str()).arg("reason", reason);
        Self { slide: None, element: None, message }
    }

    /// Les fichiers embarqués d'un épinglage sont d'une autre sorte que sa carte.
    pub fn pack_kind_mismatch(ns: &Namespace) -> Self {
        Self { slide: None, element: None, message: Message::new("issue-pack-kind-mismatch").arg("pack", ns.as_str()) }
    }

    /// Un espace de noms épinglé à la fois comme pack de widgets et de layouts.
    pub fn pack_kind_conflict(ns: &Namespace) -> Self {
        Self { slide: None, element: None, message: Message::new("issue-pack-kind-conflict").arg("pack", ns.as_str()) }
    }

    /// Un pack du deck a besoin d'un espace de noms que le deck n'épingle pas.
    pub fn pack_requirement_missing(ns: &Namespace, required: &Namespace) -> Self {
        let message =
            Message::new("issue-pack-requirement-missing").arg("pack", ns.as_str()).arg("required", required.as_str());
        Self { slide: None, element: None, message }
    }

    /// Une version de thème écartée : son contenu ne donne pas son empreinte.
    pub fn theme_version_mismatch(hex: &str) -> Self {
        Self { slide: None, element: None, message: Message::new("issue-theme-version-mismatch").arg("version", hex) }
    }

    /// Le `theme.json` embarqué d'un thème ne correspond pas à son épinglage dans le deck.
    pub fn theme_pin_mismatch(ns: &Namespace) -> Self {
        let message = Message::new("issue-theme-pin-mismatch").arg("namespace", ns.as_str());
        Self { slide: None, element: None, message }
    }

    /// Les thèmes épinglés ne sont pas exactement la fermeture du thème du deck.
    ///
    /// `ns` : le thème manquant ou de trop, ou le thème du deck pour un cycle.
    pub fn theme_closure(ns: &Namespace) -> Self {
        Self { slide: None, element: None, message: Message::new("issue-theme-closure").arg("namespace", ns.as_str()) }
    }

    /// La version d'un thème épinglé par le deck manque dans ses sources.
    pub fn theme_version_missing(version: &PackVersion) -> Self {
        let message = Message::new("issue-theme-version-missing").arg("version", version.as_str());
        Self { slide: None, element: None, message }
    }
}

/// Les valeurs des paramètres `asset` de `params` selon `schema`, listes comprises.
fn param_assets<'p>(schema: &WidgetSchema, params: &'p Params) -> Vec<&'p str> {
    let mut out = Vec::new();
    for p in &schema.params {
        match &p.ty {
            ParamType::Asset => out.extend(params.text(&p.name)),
            ParamType::List { item, .. } => {
                for it in params.list(&p.name).unwrap_or_default() {
                    for c in item {
                        match &c.ty {
                            ParamType::Asset => out.extend(it.values.text(&c.name)),
                            // Liste imbriquée : ses colonnes `asset`, item par item.
                            ParamType::List { item: sub, .. } => {
                                for nested in it.values.list(&c.name).unwrap_or_default() {
                                    let assets = sub.iter().filter(|s| s.ty == ParamType::Asset);
                                    out.extend(assets.filter_map(|s| nested.values.text(&s.name)));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Les assets que cite le deck : paramètres `asset` de ses slides et éléments.
pub fn asset_refs(deck: &Deck, registry: &WidgetRegistry) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for s in &deck.slides {
        if let Some(schema) = registry.get(&s.kind) {
            out.extend(param_assets(schema, &s.params).into_iter().map(str::to_owned));
        }
        for l in s.walk() {
            if let Some(schema) = registry.get(&l.element.kind) {
                out.extend(param_assets(schema, &l.element.params).into_iter().map(str::to_owned));
            }
        }
    }
    out
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
        let grid = s.resolved_grid(registry);
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
            if let Some((field, reason)) = e.style.invalid() {
                v.push(at, Message::new("issue-invalid-style").arg("field", field).arg("reason", reason.name()));
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
            if let (Some(spec), Some(items)) = (ListSpec::of(name, &p.ty), params.list(name)) {
                self.list(at, name, &spec, items);
            }
        }
    }

    /// Ids et bornes des items `items` de la liste au chemin `path`, puis de chaque liste
    /// imbriquée de ses items (chemin `path/item/colonne`).
    fn list(&mut self, at: Place<'_>, path: &str, spec: &ListSpec<'_>, items: &[ListItem]) {
        let mut seen = HashSet::new();
        for it in items {
            if it.id.is_empty() {
                self.push(at, Message::new("issue-empty-item-id").arg("param", path));
            } else if it.id.contains('/') {
                // Un `/` couperait le chemin `liste/item/colonne` : refusé par les ops.
                self.push(at, Message::new("issue-invalid-item-id").arg("param", path));
            } else if !seen.insert(it.id.as_str()) {
                self.push(at, Message::new("issue-duplicate-item-id").arg("param", path));
            }
        }
        let total_over = spec.nested_total(items) > crate::NESTED_ITEMS_MAX;
        if !items.is_empty() && (items.len() < spec.min || items.len() > spec.max_items() || total_over) {
            self.push(at, Message::new("issue-list-bounds").arg("param", path));
        }
        for c in spec.columns {
            let Some(sub) = ListSpec::of(&c.name, &c.ty) else { continue };
            for it in items {
                let nested = it.values.list(&c.name).unwrap_or_default();
                if !nested.is_empty() {
                    self.list(at, &format!("{path}/{}/{}", it.id, c.name), &sub, nested);
                }
            }
        }
    }

    /// Les assets des paramètres `asset` sont dans les sources.
    fn assets(&mut self, at: Place<'_>, schema: Option<&WidgetSchema>, params: &Params) {
        let (Some(schema), Some(assets)) = (schema, self.assets) else { return };
        let absent: Vec<String> =
            param_assets(schema, params).into_iter().filter(|a| !assets.contains(*a)).map(str::to_owned).collect();
        for asset in absent {
            self.push(at, Message::new("issue-missing-asset").arg("asset", asset));
        }
    }

    /// Liens et pastilles de tous les textes riches, y compris les colonnes rich des items de liste
    /// (listes imbriquées comprises).
    fn rich(&mut self, at: Place<'_>, params: &Params) {
        let items = || params.lists().flat_map(|(_, items)| items);
        // Items des listes imbriquées (un seul niveau).
        let nested = || items().flat_map(|it| it.values.lists()).flat_map(|(_, sub)| sub);
        let cells = || items().chain(nested()).flat_map(|it| it.values.riches());
        let runs = || params.riches().chain(cells()).flat_map(|(_, ps)| ps).flat_map(|p: &Paragraph| &p.runs);
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
        (ParamType::List { item, .. }, ParamValue::List(items)) => items.iter().all(|it| {
            it.values
                .iter()
                .all(|(k, v)| item.iter().find(|c| c.name == k).is_some_and(|c| matches!(fits(&c.ty, v), Fit::Ok)))
        }),
        (ParamType::List { .. }, ParamValue::Rich(p)) => p.is_empty(),
        (ParamType::List { .. }, ParamValue::Slot(s)) => s.is_empty(),
        (ParamType::Grid, ParamValue::Value(v)) => v.is_object() && serde_json::from_value::<Grid>(v.clone()).is_ok(),
        (ParamType::Host { host }, ParamValue::Value(v)) => crate::ops::host_option(*host, "", v).is_ok(),
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
