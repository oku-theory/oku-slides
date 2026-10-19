//! Registre des widgets : le schéma des paramètres de chaque composant.
//!
//! Les widgets sont déclarés en Typst (`typst/oku-slides/src/widgets.typ`,
//! et les fichiers `widgets/<nom>.typ` d'un pack, avec `_commun`). Le rendu
//! compile une sonde qui émet leurs schémas en JSON ; [`WidgetRegistry::from_probe`] les lit, les contrôle et
//! range ceux de chaque pack sous leur nom qualifié ([`qualify`]). Le cœur ne voit que ce JSON.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use snafu::ResultExt;

use crate::error::{BadSchemaSnafu, SchemaFault, UnreadableSnafu, WidgetError};
use crate::namespace::{Namespace, qualify};
use crate::ops::{DefaultFault, default_fault};

/// Ce qu'un widget rend : une slide entière, ou un élément de slide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Slide,
    Element,
}

/// Placement des éléments d'un slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SlotMode {
    /// Agencés par le widget (pile, colonnes, grille).
    Flow,
    /// Placés sur leur cadre ou leur cellule de grille.
    Free,
}

/// Types de paramètres fournis et validés par l'hôte Rust.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HostType {
    Chart,
    Table,
    /// Chemin d'une forme ([`ShapePath`](crate::ShapePath)).
    Path,
}

/// Comment un bloc du flux d'une slide prend la hauteur (attribut `grow` du schéma).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(from = "GrowRepr", into = "GrowRepr")]
pub enum Grow {
    /// Sa hauteur propre (`false`).
    #[default]
    Fixed,
    /// Une part de la hauteur qui reste (`true`).
    Fill,
    /// Sa hauteur si elle tient dans une part, sinon une part (`"fit"`).
    Fit,
}

impl Grow {
    /// Vrai pour la valeur par défaut, `false`.
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
    pub fn host(&self) -> Option<HostType> {
        match self {
            ParamType::Host { host } => Some(*host),
            _ => None,
        }
    }
}

/// Type d'un paramètre de widget.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ParamType {
    /// Texte sur une ligne, sans mise en forme.
    Plain,
    /// Paragraphes à runs.
    Rich,
    Flag,
    Number {
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    Choice {
        values: Vec<String>,
    },
    /// Identifiant d'asset `sha256:…`.
    Asset,
    Slot {
        mode: SlotMode,
        /// Widgets d'élément admis ; `None` : tous.
        #[serde(default)]
        accepts: Option<Vec<String>>,
    },
    Grid,
    Host {
        host: HostType,
    },
    /// Liste d'items aux colonnes typées (`item`), saisie en tableau ;
    /// items adressés par leur `id`.
    List {
        item: Vec<ParamSchema>,
        #[serde(default, rename = "item-label")]
        item_label: Option<String>,
        #[serde(default)]
        min: usize,
        #[serde(default)]
        max: Option<usize>,
    },
}

/// Noms interdits à une colonne de `list` : clés de l'item et d'un élément.
pub(crate) const RESERVED_COLUMNS: [&str; 2] = ["id", "kind"];

/// Un paramètre ou une colonne de texte (`plain` ou `rich`) ?
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

/// Un paramètre déclaré par un widget.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamSchema {
    pub name: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(flatten)]
    pub ty: ParamType,
    #[serde(default)]
    pub default: Option<Value>,
    /// Clé de `deck.params` où la valeur est lue et écrite.
    #[serde(default)]
    pub bind: Option<String>,
}

/// Un widget : son nom, son label, sa description, sa catégorie et ses paramètres déclarés.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WidgetSchema {
    pub name: String,
    #[serde(default)]
    pub label: Option<String>,
    /// Ce que montre le widget, en une phrase ; la grille d'ajout la filtre.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub category: Category,
    /// Hauteur d'un bloc du flux : la sienne, une part, ou la sienne si elle tient.
    #[serde(default, skip_serializing_if = "Grow::is_fixed")]
    pub grow: Grow,
    /// Le style de l'élément met en forme son texte ; sa boîte en mesure la typographie.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub typography: bool,
    pub params: Vec<ParamSchema>,
}

impl WidgetSchema {
    /// Le premier paramètre fourni par l'hôte `host`, s'il est déclaré.
    pub fn host_param(&self, host: HostType) -> Option<&ParamSchema> {
        self.params.iter().find(|p| p.ty.host() == Some(host))
    }

    /// Le paramètre `name`, s'il est déclaré.
    pub fn param(&self, name: &str) -> Option<&ParamSchema> {
        self.params.iter().find(|p| p.name == name)
    }
}

/// Les widgets disponibles : ceux du noyau, et ceux de chaque pack sous leur nom qualifié.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WidgetRegistry {
    widgets: BTreeMap<String, WidgetSchema>,
}

/// Les widgets d'un pack, tels que la sonde les émet.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProbedPack {
    namespace: Namespace,
    widgets: Vec<WidgetSchema>,
}

/// La sonde entière ; une clé inconnue (l'ancienne forme `pack`) la rend illisible.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Probe {
    base: Vec<WidgetSchema>,
    #[serde(default)]
    packs: Vec<ProbedPack>,
}

impl WidgetRegistry {
    /// Lit la sonde `{"base": [..], "packs": [{namespace, widgets}]}` émise par Typst.
    ///
    /// Un widget de pack est rangé sous son nom qualifié (`acme/card`), qui
    /// devient aussi son `name` : il ne remplace jamais un widget du noyau.
    pub fn from_probe(probe: Value) -> Result<Self, WidgetError> {
        let probe: Probe = serde_json::from_value(probe).context(UnreadableSnafu)?;
        let mut widgets = collect(probe.base)?;
        for pack in probe.packs {
            for (name, mut w) in collect(pack.widgets)? {
                let qualified = qualify(&pack.namespace, &name);
                w.name = qualified.clone();
                widgets.insert(qualified, w);
            }
        }
        let registry = Self { widgets };
        registry.check_slots()?;
        Ok(registry)
    }

    /// Pour chaque widget d'élément à paramètre d'hôte : hôte → premier paramètre de cet hôte.
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

    pub fn get(&self, name: &str) -> Option<&WidgetSchema> {
        self.widgets.get(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = &WidgetSchema> {
        self.widgets.values()
    }

    pub fn len(&self) -> usize {
        self.widgets.len()
    }

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
