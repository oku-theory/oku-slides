//! `edit_list` : insérer, retirer, déplacer un item d'une liste (`list`), ou
//! modifier un de ses champs. Les items sont adressés par leur id. Aussi :
//! `set_param` d'une liste entière, et amorçage des listes d'un objet créé.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::value::param_value;
use super::{Target, declared, target_mut, target_ref, widget};
use crate::error::{
    DuplicateIdSnafu, IndexOutOfRangeSnafu, InvalidValueSnafu, ItemNotFoundSnafu, OpError, ParamTypeSnafu,
    UnknownParamSnafu,
};
use crate::{Deck, Element, ListItem, ParamSchema, ParamType, ParamValue, Params, WidgetRegistry};

/// Un changement d'une liste ; la liste entière se remplace par `set_param`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum ListOp {
    /// Insère l'item `id` (unique dans la liste, sans `/`) ; `index` absent :
    /// à la fin ; colonnes absentes de `values` : leur défaut.
    Insert {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<usize>,
        id: String,
        #[serde(default)]
        values: Map<String, Value>,
    },
    Remove {
        item: String,
    },
    /// Déplace l'item à l'index `to` de la liste après déplacement.
    Move {
        item: String,
        to: usize,
    },
    /// Valeur d'une colonne, convertie selon son type ; `null` la retire.
    SetField {
        item: String,
        field: String,
        value: Value,
    },
}

/// Colonnes et bornes d'un paramètre `list`.
#[derive(Debug, Clone, Copy)]
pub struct ListSpec<'a> {
    pub name: &'a str,
    pub columns: &'a [ParamSchema],
    pub min: usize,
    pub max: Option<usize>,
}

impl<'a> ListSpec<'a> {
    /// Le paramètre `name` de type `ty`, s'il est une liste.
    pub fn of(name: &'a str, ty: &'a ParamType) -> Option<Self> {
        match ty {
            ParamType::List { item, min, max, .. } => Some(Self { name, columns: item, min: *min, max: *max }),
            _ => None,
        }
    }

    fn column(&self, field: &str) -> Result<&'a ParamSchema, OpError> {
        self.columns
            .iter()
            .find(|c| c.name == field)
            .ok_or_else(|| UnknownParamSnafu { widget: self.name, param: field }.build().into())
    }

    fn full(&self, len: usize) -> bool {
        self.max.is_some_and(|m| len >= m)
    }
}

/// Valeurs d'un item : colonnes déclarées, converties ; absentes : défaut.
fn item_values(spec: &ListSpec<'_>, values: &Map<String, Value>) -> Result<Params, OpError> {
    if let Some(k) = values.keys().find(|k| !spec.columns.iter().any(|c| &c.name == *k)) {
        return Err(UnknownParamSnafu { widget: spec.name, param: k.as_str() }.build().into());
    }
    let mut out = Params::default();
    for c in spec.columns {
        let raw = values.get(&c.name).cloned().or_else(|| c.default.clone()).unwrap_or(Value::Null);
        if let Some(v) = param_value(&c.ty, &c.name, raw)? {
            out.set(c.name.clone(), v);
        }
    }
    Ok(out)
}

fn check_id(spec: &ListSpec<'_>, items: &[ListItem], id: &str) -> Result<(), OpError> {
    if id.is_empty() || id.contains('/') {
        return Err(InvalidValueSnafu { field: spec.name, reason: "an item id is a non-empty string without /" }
            .build()
            .into());
    }
    if items.iter().any(|i| i.id == id) {
        return Err(DuplicateIdSnafu { id }.build().into());
    }
    Ok(())
}

fn position(spec: &ListSpec<'_>, items: &[ListItem], id: &str) -> Result<usize, OpError> {
    items.iter().position(|i| i.id == id).ok_or_else(|| ItemNotFoundSnafu { list: spec.name, item: id }.build().into())
}

impl ListOp {
    /// Applique le changement à `items` ; en cas d'erreur, `items` est intact.
    pub fn apply(self, items: &mut Vec<ListItem>, spec: &ListSpec<'_>) -> Result<(), OpError> {
        match self {
            ListOp::Insert { index, id, values } => {
                check_id(spec, items, &id)?;
                if spec.full(items.len()) {
                    return Err(InvalidValueSnafu { field: spec.name, reason: "the list is full" }.build().into());
                }
                let at = index.unwrap_or(items.len());
                if at > items.len() {
                    return Err(IndexOutOfRangeSnafu { index: at, len: items.len() }.build().into());
                }
                let values = item_values(spec, &values)?;
                items.insert(at, ListItem { id, values });
            }
            ListOp::Remove { item } => {
                let i = position(spec, items, &item)?;
                if items.len() <= spec.min {
                    return Err(InvalidValueSnafu { field: spec.name, reason: "the list has its minimum of items" }
                        .build()
                        .into());
                }
                items.remove(i);
            }
            ListOp::Move { item, to } => {
                let i = position(spec, items, &item)?;
                if to >= items.len() {
                    return Err(IndexOutOfRangeSnafu { index: to, len: items.len() }.build().into());
                }
                let it = items.remove(i);
                items.insert(to, it);
            }
            ListOp::SetField { item, field, value } => {
                let i = position(spec, items, &item)?;
                let c = spec.column(&field)?;
                match param_value(&c.ty, &field, value)? {
                    Some(v) => drop(items[i].values.set(field, v)),
                    None => drop(items[i].values.remove(&field)),
                }
            }
        }
        Ok(())
    }
}

/// Items par défaut du schéma, aux ids `d0`, `d1`… (les mêmes que Typst).
pub fn default_items(p: &ParamSchema) -> Result<Vec<ListItem>, OpError> {
    let Some(spec) = ListSpec::of(&p.name, &p.ty) else { return Ok(Vec::new()) };
    let Some(Value::Array(rows)) = &p.default else { return Ok(Vec::new()) };
    rows.iter()
        .enumerate()
        .map(|(i, row)| {
            let empty = Map::new();
            let values = item_values(&spec, row.as_object().unwrap_or(&empty))?;
            Ok(ListItem { id: format!("d{i}"), values })
        })
        .collect()
}

/// `set_param` d'une liste entière : ids uniques, colonnes converties,
/// bornes respectées ; `null` vide la liste.
pub(crate) fn list_value(spec: &ListSpec<'_>, value: Value) -> Result<Option<ParamValue>, OpError> {
    let rows = match value {
        Value::Null => return Ok(None),
        Value::Array(rows) => rows,
        _ => return Err(ParamTypeSnafu { param: spec.name, expected: "a list of items or null" }.build().into()),
    };
    let mut items: Vec<ListItem> = Vec::with_capacity(rows.len());
    for row in rows {
        let Value::Object(mut obj) = row else {
            return Err(ParamTypeSnafu { param: spec.name, expected: "a list of items or null" }.build().into());
        };
        let id = match obj.remove("id") {
            Some(Value::String(s)) => s,
            _ => return Err(InvalidValueSnafu { field: spec.name, reason: "every item needs an id" }.build().into()),
        };
        check_id(spec, &items, &id)?;
        let values = item_values(spec, &obj)?;
        items.push(ListItem { id, values });
    }
    if items.len() < spec.min || spec.max.is_some_and(|m| items.len() > m) {
        return Err(InvalidValueSnafu { field: spec.name, reason: "the list is outside its bounds" }.build().into());
    }
    Ok(if items.is_empty() { None } else { Some(ParamValue::List(items)) })
}

/// `edit_list` : la liste du paramètre `param` de la cible, changée par `change`.
pub(super) fn edit_list(
    deck: &mut Deck,
    registry: &WidgetRegistry,
    target: &Target,
    param: &str,
    change: ListOp,
) -> Result<(), OpError> {
    let (kind, _, _) = target_ref(deck, target)?;
    let p = declared(widget(registry, kind)?, param)?.clone();
    let spec = ListSpec::of(&p.name, &p.ty)
        .ok_or_else(|| OpError::from(ParamTypeSnafu { param, expected: "a list" }.build()))?;
    let (_, params) = target_mut(deck, target)?;
    let mut items = params.list(param).map(<[_]>::to_vec).unwrap_or_default();
    change.apply(&mut items, &spec)?;
    if items.is_empty() {
        params.remove(param);
    } else {
        params.set(param, ParamValue::List(items));
    }
    Ok(())
}

/// Amorce les listes absentes d'un objet créé avec le défaut du schéma de `kind`.
pub(crate) fn seed_lists(registry: &WidgetRegistry, kind: &str, params: &mut Params) {
    let Some(schema) = registry.get(kind) else { return };
    for p in &schema.params {
        if params.get(&p.name).is_none()
            && let Ok(items) = default_items(p)
            && !items.is_empty()
        {
            params.set(p.name.clone(), ParamValue::List(items));
        }
    }
}

/// [`seed_lists`] d'un élément et, récursivement, de ceux de ses slots.
pub(crate) fn seed_element(registry: &WidgetRegistry, element: &mut Element) {
    seed_lists(registry, &element.kind, &mut element.params);
    seed_slots(registry, &mut element.params);
}

/// [`seed_lists`] de chaque élément des slots de `params`, récursivement.
pub(crate) fn seed_slots(registry: &WidgetRegistry, params: &mut Params) {
    for (_, children) in params.slots_mut() {
        for child in children {
            seed_element(registry, child);
        }
    }
}

#[cfg(test)]
#[path = "list_tests.rs"]
mod list_tests;
