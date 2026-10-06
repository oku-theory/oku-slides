//! `edit_list` : insérer, retirer, déplacer un item d'une liste (`list`), ou
//! modifier un de ses champs. Les items sont adressés par leur id. Aussi :
//! `set_param` d'une liste entière, et amorçage des listes d'un objet créé.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::value::param_value;
use super::{Target, declared, target_mut, target_ref, widget};
use crate::error::{
    DuplicateIdSnafu, IndexOutOfRangeSnafu, InvalidValueSnafu, ItemNotFoundSnafu, ListBound, ListBoundSnafu, OpError,
    ParamTypeSnafu, UnknownParamSnafu,
};
use crate::{Deck, Element, ListItem, ParamSchema, ParamType, ParamValue, Params, WidgetRegistry};

/// Longueur d'une cellule `plain`, en octets.
pub const PLAIN_MAX: usize = 4096;
/// Paragraphes d'une cellule `rich`.
pub const RICH_PARAGRAPHS_MAX: usize = 64;
/// Taille JSON d'une cellule `rich`, en octets.
pub const RICH_BYTES_MAX: usize = 16384;
/// Longueur d'un id d'item, en caractères.
pub const ITEM_ID_MAX: usize = 64;
/// Items d'une liste dont le schéma ne donne pas de `max`.
pub const LIST_ITEMS_MAX: usize = 256;

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

    /// Items au plus : le `max` du schéma, sinon [`LIST_ITEMS_MAX`].
    pub fn max_items(&self) -> usize {
        self.max.unwrap_or(LIST_ITEMS_MAX)
    }

    fn refuse(&self, bound: ListBound, column: &str) -> OpError {
        ListBoundSnafu { list: self.name, bound, column }.build().into()
    }

    fn full(&self, len: usize) -> bool {
        len >= self.max_items()
    }

    fn too_many(&self, len: usize) -> Result<(), OpError> {
        if len > self.max_items() {
            return Err(self.refuse(ListBound::TooManyItems { max: self.max_items() }, ""));
        }
        Ok(())
    }

    /// La valeur de la colonne `c`, convertie et dans ses bornes de taille.
    fn cell(&self, c: &ParamSchema, raw: Value) -> Result<Option<ParamValue>, OpError> {
        let v = param_value(&c.ty, &c.name, raw)?;
        if let Some(v) = &v {
            self.cell_fits(c, v)?;
        }
        Ok(v)
    }

    /// Refuse une cellule `plain` ou `rich` trop longue.
    fn cell_fits(&self, c: &ParamSchema, v: &ParamValue) -> Result<(), OpError> {
        let too_long = match (&c.ty, v) {
            (ParamType::Plain, ParamValue::Text(s)) => s.len() > PLAIN_MAX,
            (ParamType::Rich, ParamValue::Rich(ps)) => {
                ps.len() > RICH_PARAGRAPHS_MAX || serde_json::to_vec(ps).map_or(true, |b| b.len() > RICH_BYTES_MAX)
            }
            _ => false,
        };
        if too_long {
            return Err(self.refuse(ListBound::CellTooLong, &c.name));
        }
        Ok(())
    }

    /// Bornes de taille d'items déjà convertis (sous-arbre inséré) : nombre
    /// d'items, longueur des ids et des cellules.
    pub(crate) fn sizes_fit(&self, items: &[ListItem]) -> Result<(), OpError> {
        self.too_many(items.len())?;
        for it in items {
            id_fits(self, &it.id)?;
            for (name, v) in it.values.iter() {
                if let Some(c) = self.columns.iter().find(|c| c.name == name) {
                    self.cell_fits(c, v)?;
                }
            }
        }
        Ok(())
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
        if let Some(v) = spec.cell(c, raw)? {
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
    id_fits(spec, id)?;
    if items.iter().any(|i| i.id == id) {
        return Err(DuplicateIdSnafu { id }.build().into());
    }
    Ok(())
}

fn id_fits(spec: &ListSpec<'_>, id: &str) -> Result<(), OpError> {
    if id.chars().count() > ITEM_ID_MAX {
        return Err(spec.refuse(ListBound::IdTooLong, ""));
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
                    return Err(spec.refuse(ListBound::Full { max: spec.max_items() }, ""));
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
                    return Err(spec.refuse(ListBound::Minimum { min: spec.min }, ""));
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
                match spec.cell(c, value)? {
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
    spec.too_many(rows.len())?;
    rows.iter()
        .enumerate()
        .map(|(i, row)| {
            let empty = Map::new();
            let values = item_values(&spec, row.as_object().unwrap_or(&empty))?;
            Ok(ListItem { id: format!("d{i}"), values })
        })
        .collect()
}

/// Faute du défaut d'une liste, pour le contrôle du schéma.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DefaultFault {
    /// Une valeur n'a pas le type de sa colonne (ou une ligne n'est pas un objet).
    WrongType,
    /// Les valeurs ont le bon type mais dépassent les bornes de taille (items, cellules).
    TooLarge,
}

/// La faute du défaut de la liste `p`, types contrôlés avant les tailles ;
/// `None` pour un défaut absent, qui n'est pas un tableau, ou valable.
pub(crate) fn default_fault(p: &ParamSchema) -> Option<DefaultFault> {
    let spec = ListSpec::of(&p.name, &p.ty)?;
    let Some(Value::Array(rows)) = &p.default else { return None };
    let mut too_large = spec.too_many(rows.len()).is_err();
    for row in rows {
        let Some(obj) = row.as_object() else { return Some(DefaultFault::WrongType) };
        if obj.keys().any(|k| !spec.columns.iter().any(|c| &c.name == k)) {
            return Some(DefaultFault::WrongType);
        }
        for c in spec.columns {
            let raw = obj.get(&c.name).cloned().or_else(|| c.default.clone()).unwrap_or(Value::Null);
            match param_value(&c.ty, &c.name, raw) {
                Err(_) => return Some(DefaultFault::WrongType),
                Ok(Some(v)) => too_large |= spec.cell_fits(c, &v).is_err(),
                Ok(None) => {}
            }
        }
    }
    too_large.then_some(DefaultFault::TooLarge)
}

/// `set_param` d'une liste entière : ids uniques, colonnes converties,
/// bornes respectées ; `null` vide la liste.
pub(crate) fn list_value(spec: &ListSpec<'_>, value: Value) -> Result<Option<ParamValue>, OpError> {
    let rows = match value {
        Value::Null => return Ok(None),
        Value::Array(rows) => rows,
        _ => return Err(ParamTypeSnafu { param: spec.name, expected: "a list of items or null" }.build().into()),
    };
    spec.too_many(rows.len())?;
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
    if items.len() < spec.min {
        return Err(spec.refuse(ListBound::Minimum { min: spec.min }, ""));
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
