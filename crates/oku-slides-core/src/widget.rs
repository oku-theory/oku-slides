//! Registre des widgets : le schéma des paramètres de chaque composant.
//!
//! Les widgets sont déclarés en Typst, dans les fichiers `<dossier>/<nom>.typ`
//! d'un pack (avec `_commun`) ; le cœur n'en déclare aucun. Le rendu
//! compile une sonde qui émet leurs schémas en JSON ; [`WidgetRegistry::from_probe`] les lit, les contrôle et
//! range ceux de chaque pack sous leur nom qualifié ([`qualify`]). Le cœur ne voit que ce JSON.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use snafu::ResultExt;

use crate::error::{BadSchemaSnafu, SchemaFault, UnreadableSnafu, WidgetError};
use crate::namespace::{Namespace, qualify};
use crate::ops::{DefaultFault, default_fault};
use crate::pack_kind::PackKind;

/// What a widget renders: a whole slide, or a slide element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    /// A whole slide, placed in the deck.
    Slide,
    /// An element, placed in a slot.
    Element,
}

/// Placement of the elements of a slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SlotMode {
    /// Laid out by the widget (stack, columns, grid).
    Flow,
    /// Placed on their frame or their grid cell.
    Free,
}

/// Parameter types supplied and validated by the Rust host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HostType {
    /// Chart: options and dataset (`dataset`).
    Chart,
    /// Table: display options and dataset.
    Table,
    /// Path of a shape ([`ShapePath`](crate::ShapePath)).
    Path,
}

/// How a flow block of a slide takes its height (schema attribute `grow`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(from = "GrowRepr", into = "GrowRepr")]
pub enum Grow {
    /// Its own height (`false`).
    #[default]
    Fixed,
    /// A share of the remaining height (`true`).
    Fill,
    /// Its own height if it fits in a share, otherwise a share (`"fit"`).
    Fit,
}

impl Grow {
    /// Vrai pour la valeur par défaut, `false`.
    #[must_use]
    pub fn is_fixed(&self) -> bool {
        *self == Grow::Fixed
    }
}

/// Forme JSON de [`Grow`] : un booléen ou la chaîne `"fit"` ; toute autre valeur est refusée.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum GrowRepr {
    Flag(bool),
    Word(GrowWord),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum GrowWord {
    Fit,
}

impl From<GrowRepr> for Grow {
    fn from(r: GrowRepr) -> Self {
        match r {
            GrowRepr::Flag(false) => Grow::Fixed,
            GrowRepr::Flag(true) => Grow::Fill,
            GrowRepr::Word(GrowWord::Fit) => Grow::Fit,
        }
    }
}

impl From<Grow> for GrowRepr {
    fn from(g: Grow) -> Self {
        match g {
            Grow::Fixed => GrowRepr::Flag(false),
            Grow::Fill => GrowRepr::Flag(true),
            Grow::Fit => GrowRepr::Word(GrowWord::Fit),
        }
    }
}

impl ParamType {
    /// L'hôte qui fournit ce paramètre, s'il en a un.
    #[must_use]
    pub fn host(&self) -> Option<HostType> {
        match self {
            ParamType::Host { host } => Some(*host),
            _ => None,
        }
    }
}

/// Type of a widget parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ParamType {
    /// Single-line text, without formatting.
    Plain,
    /// Paragraphs of runs.
    Rich,
    /// Boolean.
    Flag,
    /// Number, bounded by `min` and `max` when given.
    Number {
        /// Smallest allowed value; absent: no bound.
        #[serde(default)]
        min: Option<f64>,
        /// Largest allowed value; absent: no bound.
        #[serde(default)]
        max: Option<f64>,
    },
    /// One value among `values`.
    Choice {
        /// Allowed values, in display order.
        values: Vec<String>,
    },
    /// Asset identifier `sha256:…`.
    Asset,
    /// List of child elements, placed according to `mode`.
    Slot {
        /// Placement of the elements: laid out or free.
        mode: SlotMode,
        /// Accepted element widgets; `None`: all of them.
        #[serde(default)]
        accepts: Option<Vec<String>>,
    },
    /// Layout grid ([`Grid`](crate::Grid)).
    Grid,
    /// Value supplied and validated by the Rust host.
    Host {
        /// Host that supplies the value.
        host: HostType,
    },
    /// List of items with typed columns (`item`), edited as a table.
    ///
    /// Items are addressed by their `id`.
    List {
        /// Columns of an item, in display order.
        item: Vec<ParamSchema>,
        /// Name of an item, for the add buttons.
        #[serde(default, rename = "item-label")]
        item_label: Option<String>,
        /// Minimum number of items.
        #[serde(default)]
        min: usize,
        /// Maximum number of items, capped at [`LIST_ITEMS_MAX`](crate::LIST_ITEMS_MAX).
        #[serde(default)]
        max: Option<usize>,
    },
}

/// Noms interdits à une colonne de `list` : clés de l'item et d'un élément.
pub(crate) const RESERVED_COLUMNS: [&str; 2] = ["id", "kind"];

/// Un paramètre ou une colonne de texte (`plain` ou `rich`) ?
#[must_use]
pub fn is_text(ty: &ParamType) -> bool {
    matches!(ty, ParamType::Plain | ParamType::Rich)
}

/// Types admis pour une colonne de `list`, hors liste imbriquée (`list`, un seul niveau :
/// voir `check_list`).
pub(crate) fn is_column_type(ty: &ParamType) -> bool {
    matches!(
        ty,
        ParamType::Plain
            | ParamType::Rich
            | ParamType::Number { .. }
            | ParamType::Flag
            | ParamType::Choice { .. }
            | ParamType::Asset
    )
}

/// A parameter declared by a widget.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamSchema {
    /// Name of the parameter, without `/`, unique in the widget.
    pub name: String,
    /// Label shown in the editor; absent: the name.
    #[serde(default)]
    pub label: Option<String>,
    /// Type of the parameter and its constraints.
    #[serde(flatten)]
    pub ty: ParamType,
    /// Default value, as written in the Typst schema.
    #[serde(default)]
    pub default: Option<Value>,
    /// Key of `deck.params` where the value is read and written.
    #[serde(default)]
    pub bind: Option<String>,
}

/// A widget: its name, label, description, category and declared parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WidgetSchema {
    /// Qualified name of the widget (`acme/card`).
    pub name: String,
    /// Label shown in the editor; absent: the name.
    #[serde(default)]
    pub label: Option<String>,
    /// What the widget shows, in one sentence; the add grid filters on it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Slide or element; equal to the kind of its pack.
    pub category: Category,
    /// Height of a flow block: its own, a share, or its own if it fits.
    #[serde(default, skip_serializing_if = "Grow::is_fixed")]
    pub grow: Grow,
    /// The element's style formats its text; its box measures the typography.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub typography: bool,
    /// Declared parameters, in schema order.
    pub params: Vec<ParamSchema>,
}

impl WidgetSchema {
    /// Le premier paramètre fourni par l'hôte `host`, s'il est déclaré.
    #[must_use]
    pub fn host_param(&self, host: HostType) -> Option<&ParamSchema> {
        self.params.iter().find(|p| p.ty.host() == Some(host))
    }

    /// Le paramètre `name`, s'il est déclaré.
    #[must_use]
    pub fn param(&self, name: &str) -> Option<&ParamSchema> {
        self.params.iter().find(|p| p.name == name)
    }
}

/// Les widgets disponibles : ceux de chaque pack, sous leur nom qualifié.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct WidgetRegistry {
    widgets: BTreeMap<String, WidgetSchema>,
}

/// Les widgets d'un pack, tels que la sonde les émet.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProbedPack {
    namespace: Namespace,
    kind: PackKind,
    widgets: Vec<WidgetSchema>,
}

/// La sonde entière ; une clé inconnue (l'ancienne clé `base`) la rend illisible.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Probe {
    packs: Vec<ProbedPack>,
}

impl WidgetRegistry {
    /// Reads the probe `{"packs": [{namespace, kind, widgets}]}` emitted by Typst.
    ///
    /// A pack widget is stored under its qualified name (`acme/card`), which
    /// also becomes its `name`. Its category must be that of the pack's kind.
    ///
    /// # Errors
    ///
    /// Unreadable probe, invalid widget schema, category that is not the
    /// pack's, or slot that accepts an unknown element widget.
    pub fn from_probe(probe: Value) -> Result<Self, WidgetError> {
        let probe: Probe = serde_json::from_value(probe).context(UnreadableSnafu)?;
        let mut widgets = BTreeMap::new();
        for pack in probe.packs {
            for (name, mut w) in collect(pack.widgets)? {
                let qualified = qualify(&pack.namespace, &name);
                w.name.clone_from(&qualified);
                if w.category != pack.kind.category() {
                    return Err(fault(&w, SchemaFault::PackCategory));
                }
                widgets.insert(qualified, w);
            }
        }
        let registry = Self { widgets };
        registry.check_slots()?;
        Ok(registry)
    }

    /// Pour chaque widget d'élément à paramètre d'hôte : hôte → premier paramètre de cet hôte.
    #[must_use]
    pub fn host_params(&self) -> BTreeMap<String, BTreeMap<HostType, String>> {
        let mut out = BTreeMap::new();
        for (kind, w) in self.widgets.iter().filter(|(_, w)| w.category == Category::Element) {
            let mut hosts = BTreeMap::new();
            // Le premier paramètre d'un hôte gagne, comme dans `host_param`.
            for p in &w.params {
                if let Some(h) = p.ty.host() {
                    hosts.entry(h).or_insert_with(|| p.name.clone());
                }
            }
            if !hosts.is_empty() {
                out.insert(kind.clone(), hosts);
            }
        }
        out
    }

    /// The widget with qualified name `name`, if it is declared.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&WidgetSchema> {
        self.widgets.get(name)
    }

    /// The widgets, by ascending qualified name.
    pub fn iter(&self) -> impl Iterator<Item = &WidgetSchema> {
        self.widgets.values()
    }

    /// Number of declared widgets.
    #[must_use]
    pub fn len(&self) -> usize {
        self.widgets.len()
    }

    /// Are no widgets declared?
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.widgets.is_empty()
    }

    /// Chaque slot n'accepte que des widgets d'élément déclarés.
    fn check_slots(&self) -> Result<(), WidgetError> {
        for w in self.widgets.values() {
            for p in &w.params {
                let ParamType::Slot { accepts: Some(kinds), .. } = &p.ty else { continue };
                let known = |k: &String| self.get(k).is_some_and(|x| x.category == Category::Element);
                if !kinds.iter().all(known) {
                    return Err(fault(w, SchemaFault::UnknownSlotKind));
                }
            }
        }
        Ok(())
    }
}

fn fault(w: &WidgetSchema, fault: SchemaFault) -> WidgetError {
    BadSchemaSnafu { widget: w.name.clone(), fault }.build().into()
}

/// Contrôle chaque widget d'une liste et les range par nom.
fn collect(list: Vec<WidgetSchema>) -> Result<BTreeMap<String, WidgetSchema>, WidgetError> {
    let mut out = BTreeMap::new();
    for w in list {
        check_widget(&w)?;
        if out.contains_key(&w.name) {
            return Err(fault(&w, SchemaFault::DuplicateWidget));
        }
        out.insert(w.name.clone(), w);
    }
    Ok(out)
}

fn check_widget(w: &WidgetSchema) -> Result<(), WidgetError> {
    if w.category == Category::Slide && (!w.grow.is_fixed() || w.typography) {
        return Err(fault(w, SchemaFault::SlideSizing));
    }
    if w.name.is_empty() || w.params.iter().any(|p| p.name.is_empty()) {
        return Err(fault(w, SchemaFault::EmptyName));
    }
    let mut seen = BTreeSet::new();
    for p in &w.params {
        if !seen.insert(p.name.as_str()) {
            return Err(fault(w, SchemaFault::DuplicateParam));
        }
        check_param(w, p)?;
    }
    Ok(())
}

fn check_param(w: &WidgetSchema, p: &ParamSchema) -> Result<(), WidgetError> {
    if p.name.contains('/') {
        return Err(fault(w, SchemaFault::ReservedName));
    }
    if p.bind.is_some() && p.ty != ParamType::Plain {
        return Err(fault(w, SchemaFault::BindNotPlain));
    }
    match &p.ty {
        ParamType::Choice { values } => {
            if values.is_empty() {
                return Err(fault(w, SchemaFault::EmptyChoice));
            }
            let in_values = |d: &Value| d.as_str().is_some_and(|d| values.iter().any(|v| v == d));
            if p.default.as_ref().is_some_and(|d| !in_values(d)) {
                return Err(fault(w, SchemaFault::DefaultNotInChoice));
            }
        }
        ParamType::Flag if p.default.as_ref().is_some_and(|d| !d.is_boolean()) => {
            return Err(fault(w, SchemaFault::DefaultWrongType));
        }
        ParamType::Number { min, max }
            if p.default.as_ref().is_some_and(|d| {
                !d.as_f64().is_some_and(|n| !min.is_some_and(|m| n < m) && !max.is_some_and(|m| n > m))
            }) =>
        {
            return Err(fault(w, SchemaFault::DefaultWrongType));
        }
        ParamType::Plain if p.default.as_ref().is_some_and(|d| !d.is_string()) => {
            return Err(fault(w, SchemaFault::DefaultWrongType));
        }
        ParamType::Grid if p.default.as_ref().is_some_and(|d| !d.is_object()) => {
            return Err(fault(w, SchemaFault::DefaultWrongType));
        }
        ParamType::List { .. } => check_list(w, p, false)?,
        _ => {}
    }
    Ok(())
}

/// Contrôle la liste `p` ; `nested` : `p` est une colonne d'une liste, qui ne peut porter
/// elle-même de colonne `list` (un seul niveau d'imbrication, t-495).
fn check_list(w: &WidgetSchema, p: &ParamSchema, nested: bool) -> Result<(), WidgetError> {
    let ParamType::List { item, min, max, .. } = &p.ty else { return Ok(()) };
    if item.is_empty() {
        return Err(fault(w, SchemaFault::EmptyList));
    }
    let mut names = BTreeSet::new();
    for c in item {
        let is_list = matches!(c.ty, ParamType::List { .. });
        if is_list && nested {
            return Err(fault(w, SchemaFault::ListDepth));
        }
        if !is_list && !is_column_type(&c.ty) {
            return Err(fault(w, SchemaFault::ColumnType));
        }
        if c.name.is_empty() || RESERVED_COLUMNS.contains(&c.name.as_str()) || c.name.contains('/') {
            return Err(fault(w, SchemaFault::ReservedName));
        }
        if !names.insert(c.name.as_str()) {
            return Err(fault(w, SchemaFault::DuplicateParam));
        }
        if c.bind.is_some() {
            return Err(fault(w, SchemaFault::BindNotPlain));
        }
        if is_list {
            check_list(w, c, true)?;
        } else {
            check_param(w, c)?;
        }
    }
    if max.is_some_and(|m| m < *min) {
        return Err(fault(w, SchemaFault::ListBounds));
    }
    // Types, colonnes connues et tailles : `default_fault` ; ici, seulement « un tableau ».
    let ok = p.default.as_ref().is_none_or(Value::is_array);
    match (ok, default_fault(p, nested)) {
        (false, _) | (_, Some(DefaultFault::WrongType)) => Err(fault(w, SchemaFault::DefaultWrongType)),
        (true, Some(DefaultFault::TooLarge)) => Err(fault(w, SchemaFault::ListDefaultTooLarge)),
        (true, Some(DefaultFault::BelowMin)) => Err(fault(w, SchemaFault::ListDefaultBelowMin)),
        (true, None) => Ok(()),
    }
}

#[cfg(test)]
#[path = "widget_tests.rs"]
mod widget_tests;
