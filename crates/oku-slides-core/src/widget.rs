//! Registre des widgets : le schéma des paramètres de chaque composant.
//!
//! Les widgets sont déclarés en Typst (`typst/oku-slides/src/widgets.typ`,
//! et les fichiers `widgets/<nom>.typ` d'un pack, avec `_commun`). Le rendu
//! compile une sonde qui émet leurs schémas en JSON ; [`WidgetRegistry::from_probe`] les lit, les contrôle et
//! applique les surcharges du pack. Le cœur ne voit que ce JSON.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use snafu::ResultExt;

use crate::error::{
    BadSchemaSnafu, IncompatibleCategorySnafu, IncompatibleOverrideSnafu, SchemaFault, UnreadableSnafu, WidgetError,
};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HostType {
    Chart,
    Table,
    /// Chemin d'une forme ([`ShapePath`](crate::ShapePath)).
    Path,
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

/// Types admis pour une colonne de `list` ; liste imbriquée : t-495.
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

/// Un widget : son nom, sa catégorie et ses paramètres dans l'ordre déclaré.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WidgetSchema {
    pub name: String,
    #[serde(default)]
    pub label: Option<String>,
    pub category: Category,
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

/// Les widgets disponibles : ceux de la base, surchargés par ceux du pack.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WidgetRegistry {
    widgets: BTreeMap<String, WidgetSchema>,
}

#[derive(Deserialize)]
struct Probe {
    base: Vec<WidgetSchema>,
    #[serde(default)]
    pack: Option<Vec<WidgetSchema>>,
}

impl WidgetRegistry {
    /// Lit la sonde `{"base": [..], "pack": [..] | null}` émise par Typst.
    pub fn from_probe(probe: Value) -> Result<Self, WidgetError> {
        let probe: Probe = serde_json::from_value(probe).context(UnreadableSnafu)?;
        let mut widgets = collect(probe.base)?;
        if let Some(pack) = probe.pack {
            for (name, w) in collect(pack)? {
                if let Some(base) = widgets.get(&name) {
                    check_override(base, &w)?;
                }
                widgets.insert(name, w);
            }
        }
        let registry = Self { widgets };
        registry.check_slots()?;
        Ok(registry)
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
        ParamType::List { item, min, max, .. } => {
            if item.is_empty() {
                return Err(fault(w, SchemaFault::EmptyList));
            }
            let mut names = BTreeSet::new();
            for c in item {
                if !is_column_type(&c.ty) {
                    return Err(fault(w, SchemaFault::ColumnType));
                }
                if c.name.is_empty() || RESERVED_COLUMNS.contains(&c.name.as_str()) {
                    return Err(fault(w, SchemaFault::ReservedName));
                }
                if !names.insert(c.name.as_str()) {
                    return Err(fault(w, SchemaFault::DuplicateParam));
                }
                if c.bind.is_some() {
                    return Err(fault(w, SchemaFault::BindNotPlain));
                }
                check_param(w, c)?;
            }
            if max.is_some_and(|m| m < *min) {
                return Err(fault(w, SchemaFault::ListBounds));
            }
            // Types, colonnes connues et tailles : `default_fault` ; ici, seulement « un tableau ».
            let ok = p.default.as_ref().is_none_or(Value::is_array);
            match (ok, default_fault(p)) {
                (false, _) | (_, Some(DefaultFault::WrongType)) => return Err(fault(w, SchemaFault::DefaultWrongType)),
                (true, Some(DefaultFault::TooLarge)) => return Err(fault(w, SchemaFault::ListDefaultTooLarge)),
                (true, None) => {}
            }
        }
        _ => {}
    }
    Ok(())
}

/// Un widget de pack peut remplacer un widget de base s'il garde sa
/// catégorie et chacun de ses paramètres, avec un type et un `bind`
/// compatibles : toute valeur valide pour le paramètre de base doit rester
/// valide pour celui du pack.
fn check_override(base: &WidgetSchema, pack: &WidgetSchema) -> Result<(), WidgetError> {
    if base.category != pack.category {
        return Err(IncompatibleCategorySnafu { widget: base.name.clone() }.build().into());
    }
    for bp in &base.params {
        let ok = pack.param(&bp.name).is_some_and(|pp| bp.bind == pp.bind && compatible(&bp.ty, &pp.ty));
        if !ok {
            return Err(IncompatibleOverrideSnafu { widget: base.name.clone(), param: bp.name.clone() }.build().into());
        }
    }
    Ok(())
}

/// Une valeur valide pour `base` l'est-elle pour `pack` ?
fn compatible(base: &ParamType, pack: &ParamType) -> bool {
    match (base, pack) {
        (ParamType::Choice { values: b }, ParamType::Choice { values: p }) => b.iter().all(|v| p.contains(v)),
        (ParamType::Number { min: bmin, max: bmax }, ParamType::Number { min: pmin, max: pmax }) => {
            let min_ok = match bmin {
                Some(bm) => pmin.is_none_or(|pm| pm <= *bm),
                None => pmin.is_none(),
            };
            let max_ok = match bmax {
                Some(bx) => pmax.is_none_or(|px| px >= *bx),
                None => pmax.is_none(),
            };
            min_ok && max_ok
        }
        (ParamType::Slot { mode: bmode, accepts: baccepts }, ParamType::Slot { mode: pmode, accepts: paccepts }) => {
            bmode == pmode
                && match baccepts {
                    Some(bset) => paccepts.as_ref().is_none_or(|pset| bset.iter().all(|k| pset.contains(k))),
                    None => paccepts.is_none(),
                }
        }
        (b, p) => b == p,
    }
}

#[cfg(test)]
#[path = "widget_tests.rs"]
mod widget_tests;
