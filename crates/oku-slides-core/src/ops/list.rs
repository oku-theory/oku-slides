//! `edit_list` : insérer, retirer, déplacer un item d'une liste (`list`), ou
//! modifier un de ses champs. Les items sont adressés par leur id. Aussi :
//! `set_param` d'une liste entière, et amorçage des listes d'un objet créé.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::list_path::ListPath;
use super::value::param_value;
use super::{Target, declared, target_mut, target_ref, widget};
use crate::error::{
    DuplicateIdSnafu, IndexOutOfRangeSnafu, InvalidValueSnafu, ItemNotFoundSnafu, ListBound, ListBoundSnafu, OpError,
    ParamTypeSnafu, Shape, UnknownParamSnafu, ValueReason,
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
/// Items d'une liste au plus : le `max` d'un schéma est ramené à cette borne.
pub const LIST_ITEMS_MAX: usize = 256;
/// Items des listes imbriquées de tous les items d'une liste, au plus (t-495).
pub const NESTED_ITEMS_MAX: usize = 2 * LIST_ITEMS_MAX;

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

    pub(super) fn column(&self, field: &str) -> Result<&'a ParamSchema, OpError> {
        self.columns
            .iter()
            .find(|c| c.name == field)
            .ok_or_else(|| UnknownParamSnafu { widget: self.name, param: field }.build().into())
    }

    /// Items au plus : le `max` du schéma borné à [`LIST_ITEMS_MAX`], sinon [`LIST_ITEMS_MAX`].
    pub fn max_items(&self) -> usize {
        self.max.map_or(LIST_ITEMS_MAX, |m| m.min(LIST_ITEMS_MAX))
    }

    /// Items des listes imbriquées de tous les items de `items`.
    pub fn nested_total(&self, items: &[ListItem]) -> usize {
        let nested = || self.columns.iter().filter(|c| matches!(c.ty, ParamType::List { .. }));
        items.iter().map(|it| nested().map(|c| it.values.list(&c.name).map_or(0, <[_]>::len)).sum::<usize>()).sum()
    }

    /// Refuse plus de [`NESTED_ITEMS_MAX`] items imbriqués dans `items`.
    pub(super) fn nested_total_fits(&self, items: &[ListItem]) -> Result<(), OpError> {
        if self.nested_total(items) > NESTED_ITEMS_MAX {
            return Err(self.refuse_total());
        }
        Ok(())
    }

    /// Le refus d'un total d'items imbriqués au-delà de [`NESTED_ITEMS_MAX`].
    pub(super) fn refuse_total(&self) -> OpError {
        self.refuse(ListBound::TooManyItems { max: NESTED_ITEMS_MAX })
    }

    fn refuse(&self, bound: ListBound) -> OpError {
        ListBoundSnafu { list: self.name, bound }.build().into()
    }

    fn full(&self, len: usize) -> bool {
        len >= self.max_items()
    }

    fn too_many(&self, len: usize) -> Result<(), OpError> {
        if len > self.max_items() {
            return Err(self.refuse(ListBound::TooManyItems { max: self.max_items() }));
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
        if !text_fits(&c.ty, v) {
            return Err(self.refuse(ListBound::CellTooLong { column: c.name.clone() }));
        }
        Ok(())
    }

    /// Bornes de taille d'items déjà convertis (sous-arbre inséré) : nombre
    /// d'items (imbriqués compris), ids (sans `/`, longueur) et cellules.
    pub(crate) fn sizes_fit(&self, items: &[ListItem]) -> Result<(), OpError> {
        self.too_many(items.len())?;
        self.nested_total_fits(items)?;
        for it in items {
            if it.id.contains('/') {
                return Err(InvalidValueSnafu { field: self.name, reason: ValueReason::BadItemId }.build().into());
            }
            id_fits(self, &it.id)?;
            for (name, v) in it.values.iter() {
                let Some(c) = self.columns.iter().find(|c| c.name == name) else { continue };
                self.cell_fits(c, v)?;
                if let (Some(sub), ParamValue::List(sub_items)) = (ListSpec::of(&c.name, &c.ty), v) {
                    sub.sizes_fit(sub_items)?;
                }
            }
        }
        Ok(())
    }
}

/// La valeur `v` tient-elle dans les bornes de taille d'un texte de type `ty` ?
/// `plain` : `PLAIN_MAX` octets ; `rich` : `RICH_PARAGRAPHS_MAX` paragraphes et
/// `RICH_BYTES_MAX` octets. Les autres types n'ont pas de borne de texte.
#[must_use]
pub fn text_fits(ty: &ParamType, v: &ParamValue) -> bool {
    match (ty, v) {
        (ParamType::Plain, ParamValue::Text(s)) => s.len() <= PLAIN_MAX,
        (ParamType::Rich, ParamValue::Rich(ps)) => {
            ps.len() <= RICH_PARAGRAPHS_MAX && serde_json::to_vec(ps).is_ok_and(|b| b.len() <= RICH_BYTES_MAX)
        }
        _ => true,
    }
}

/// Comment les valeurs d'un item se complètent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fill {
    /// Item nouveau (`insert`) : une colonne absente prend son défaut, liste imbriquée comprise.
    New,
    /// Ligne d'un défaut de schéma : comme `New`, et les lignes de ses listes imbriquées
    /// reçoivent les ids `d0`, `d1`…
    Default,
    /// Liste remplacée (`set_param`, collage) : une liste imbriquée absente reste vide
    /// (son défaut n'amorce qu'un objet créé) ; les autres colonnes prennent leur défaut.
    Replace,
}

/// Lignes d'une liste imbriquée écrites dans un défaut : ids `d0`, `d1`… à celles qui n'en
/// ont pas (les mêmes que Typst) ; toute autre valeur est rendue telle quelle.
fn seed_ids(c: &ParamSchema, v: Value) -> Value {
    match (&c.ty, v) {
        (ParamType::List { .. }, Value::Array(rows)) => Value::Array(
            rows.into_iter()
                .enumerate()
                .map(|(i, row)| match row {
                    Value::Object(mut o) => {
                        o.entry("id").or_insert_with(|| Value::String(format!("d{i}")));
                        Value::Object(o)
                    }
                    other => other,
                })
                .collect(),
        ),
        (_, v) => v,
    }
}

/// Valeurs d'un item : colonnes déclarées, converties ; absentes : défaut (voir [`Fill`]).
fn item_values(spec: &ListSpec<'_>, values: &Map<String, Value>, fill: Fill) -> Result<Params, OpError> {
    if let Some(k) = values.keys().find(|k| !spec.columns.iter().any(|c| &c.name == *k)) {
        return Err(UnknownParamSnafu { widget: spec.name, param: k.as_str() }.build().into());
    }
    let mut out = Params::default();
    for c in spec.columns {
        let nested = matches!(c.ty, ParamType::List { .. });
        let raw = match values.get(&c.name) {
            Some(v) if fill == Fill::Default => seed_ids(c, v.clone()),
            Some(v) => v.clone(),
            None if nested && fill == Fill::Replace => Value::Null,
            None => c.default.clone().map(|d| seed_ids(c, d)).unwrap_or(Value::Null),
        };
        if let Some(v) = spec.cell(c, raw)? {
            out.set(c.name.clone(), v);
        }
    }
    Ok(out)
}

fn check_id(spec: &ListSpec<'_>, items: &[ListItem], id: &str) -> Result<(), OpError> {
    if id.is_empty() || id.contains('/') {
        return Err(InvalidValueSnafu { field: spec.name, reason: ValueReason::BadItemId }.build().into());
    }
    id_fits(spec, id)?;
    if items.iter().any(|i| i.id == id) {
        return Err(DuplicateIdSnafu { id }.build().into());
    }
    Ok(())
}

fn id_fits(spec: &ListSpec<'_>, id: &str) -> Result<(), OpError> {
    if id.chars().count() > ITEM_ID_MAX {
        return Err(spec.refuse(ListBound::IdTooLong));
    }
    Ok(())
}

pub(super) fn position(spec: &ListSpec<'_>, items: &[ListItem], id: &str) -> Result<usize, OpError> {
    items.iter().position(|i| i.id == id).ok_or_else(|| ItemNotFoundSnafu { list: spec.name, item: id }.build().into())
}

impl ListOp {
    /// Applique le changement à `items` ; en cas d'erreur, `items` est intact.
    pub fn apply(self, items: &mut Vec<ListItem>, spec: &ListSpec<'_>) -> Result<(), OpError> {
        match self {
            ListOp::Insert { index, id, values } => {
                check_id(spec, items, &id)?;
                if spec.full(items.len()) {
                    return Err(spec.refuse(ListBound::Full { max: spec.max_items() }));
                }
                let at = index.unwrap_or(items.len());
                if at > items.len() {
                    return Err(IndexOutOfRangeSnafu { index: at, len: items.len() }.build().into());
                }
                let values = item_values(spec, &values, Fill::New)?;
                items.insert(at, ListItem { id, values });
            }
            ListOp::Remove { item } => {
                let i = position(spec, items, &item)?;
                if items.len() <= spec.min {
                    return Err(spec.refuse(ListBound::Minimum { min: spec.min }));
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
            let values = item_values(&spec, row.as_object().unwrap_or(&empty), Fill::Default)?;
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
    /// Une liste imbriquée par défaut a moins d'items que son `min` : toute insertion
    /// de l'item qui la porte serait refusée.
    BelowMin,
}

/// La faute du défaut de la liste `p`, types contrôlés avant les tailles ;
/// `None` pour un défaut absent, qui n'est pas un tableau, ou valable. `nested` : `p` est la
/// colonne d'une liste, dont le défaut passe par `set_param` (ids, `min`).
pub(crate) fn default_fault(p: &ParamSchema, nested: bool) -> Option<DefaultFault> {
    let spec = ListSpec::of(&p.name, &p.ty)?;
    let Some(Value::Array(rows)) = &p.default else { return None };
    rows_fault(&spec, rows, nested)
}

/// La faute des lignes `rows` d'un défaut de la liste `spec`. Une liste imbriquée
/// (`nested`) se lit comme [`seed_ids`] la prépare : `id` chaîne valable et unique ou
/// absent, au moins `min` lignes ; dans une ligne parente, `null` vaut une liste vide.
fn rows_fault(spec: &ListSpec<'_>, rows: &[Value], nested: bool) -> Option<DefaultFault> {
    if nested && rows.len() < spec.min {
        return Some(DefaultFault::BelowMin);
    }
    let mut too_large = rows.len() > spec.max_items();
    let mut ids = std::collections::HashSet::new();
    for row in rows {
        let Some(obj) = row.as_object() else { return Some(DefaultFault::WrongType) };
        let id_ok = |id: &Value| {
            id.as_str().is_some_and(|s| !s.is_empty() && !s.contains('/') && s.chars().count() <= ITEM_ID_MAX)
        };
        match obj.get("id") {
            Some(id) if nested && id_ok(id) && ids.insert(id.as_str().unwrap_or_default().to_owned()) => {}
            None => {}
            Some(_) => return Some(DefaultFault::WrongType),
        }
        if obj.keys().any(|k| k != "id" && !spec.columns.iter().any(|c| &c.name == k)) {
            return Some(DefaultFault::WrongType);
        }
        for c in spec.columns {
            if let Some(sub) = ListSpec::of(&c.name, &c.ty) {
                let fault = match obj.get(&c.name) {
                    Some(Value::Array(sub_rows)) => rows_fault(&sub, sub_rows, true),
                    // Absente : le défaut de la colonne, contrôlé avec elle ; `null` : vide.
                    None | Some(Value::Null) => None,
                    Some(_) => Some(DefaultFault::WrongType),
                };
                match fault {
                    Some(DefaultFault::TooLarge) => too_large = true,
                    Some(f) => return Some(f),
                    None => {}
                }
                continue;
            }
            let raw = obj.get(&c.name).cloned().or_else(|| c.default.clone()).unwrap_or(Value::Null);
            match param_value(&c.ty, &c.name, raw) {
                Err(_) => return Some(DefaultFault::WrongType),
                Ok(Some(v)) => too_large |= !text_fits(&c.ty, &v),
                Ok(None) => {}
            }
        }
    }
    // Total des listes imbriquées : lignes données, ou défaut de la colonne pour une absente.
    let nested_total: usize = rows
        .iter()
        .filter_map(Value::as_object)
        .flat_map(|obj| {
            spec.columns.iter().filter(|c| matches!(c.ty, ParamType::List { .. })).map(move |c| {
                match obj.get(&c.name) {
                    Some(Value::Array(a)) => a.len(),
                    None => c.default.as_ref().and_then(Value::as_array).map_or(0, Vec::len),
                    Some(_) => 0,
                }
            })
        })
        .sum();
    too_large |= nested_total > NESTED_ITEMS_MAX;
    too_large.then_some(DefaultFault::TooLarge)
}

/// `set_param` d'une liste entière : ids uniques, colonnes converties,
/// bornes respectées ; `null` vide la liste.
pub(crate) fn list_value(spec: &ListSpec<'_>, value: Value) -> Result<Option<ParamValue>, OpError> {
    let rows = match value {
        Value::Null => return Ok(None),
        Value::Array(rows) => rows,
        _ => return Err(ParamTypeSnafu { param: spec.name, expected: Shape::Items.or_null() }.build().into()),
    };
    spec.too_many(rows.len())?;
    let mut items: Vec<ListItem> = Vec::with_capacity(rows.len());
    for row in rows {
        let Value::Object(mut obj) = row else {
            return Err(ParamTypeSnafu { param: spec.name, expected: Shape::Items.or_null() }.build().into());
        };
        let id = match obj.remove("id") {
            Some(Value::String(s)) => s,
            _ => return Err(InvalidValueSnafu { field: spec.name, reason: ValueReason::MissingItemId }.build().into()),
        };
        check_id(spec, &items, &id)?;
        let values = item_values(spec, &obj, Fill::Replace)?;
        items.push(ListItem { id, values });
    }
    if items.len() < spec.min {
        return Err(spec.refuse(ListBound::Minimum { min: spec.min }));
    }
    spec.nested_total_fits(&items)?;
    Ok(if items.is_empty() { None } else { Some(ParamValue::List(items)) })
}

/// `edit_list` : la liste au chemin `param` de la cible (paramètre, ou
/// `param/item/colonne` pour une liste imbriquée), changée par `change`.
pub(super) fn edit_list(
    deck: &mut Deck,
    registry: &WidgetRegistry,
    target: &Target,
    param: &str,
    change: ListOp,
) -> Result<(), OpError> {
    let (kind, _, _) = target_ref(deck, target)?;
    let schema = widget(registry, kind)?;
    let path =
        ListPath::parse(param).ok_or_else(|| OpError::from(UnknownParamSnafu { widget: kind, param }.build()))?;
    let p = declared(schema, path.param)?.clone();
    let spec = ListSpec::of(&p.name, &p.ty)
        .ok_or_else(|| OpError::from(ParamTypeSnafu { param, expected: Shape::List.required() }.build()))?;
    let (_, params) = target_mut(deck, target)?;
    let mut items = params.list(path.param).map(<[_]>::to_vec).unwrap_or_default();
    path.apply(&mut items, &spec, change)?;
    if items.is_empty() {
        params.remove(path.param);
    } else {
        params.set(path.param, ParamValue::List(items));
    }
    Ok(())
}

/// Amorce les listes absentes d'un objet créé avec le défaut du schéma de `kind`.
/// Un défaut que le cœur refuse est une erreur (le contrôle du schéma l'écarte d'avance).
pub(crate) fn seed_lists(registry: &WidgetRegistry, kind: &str, params: &mut Params) -> Result<(), OpError> {
    let Some(schema) = registry.get(kind) else { return Ok(()) };
    for p in &schema.params {
        if params.get(&p.name).is_some() {
            continue;
        }
        let items = default_items(p)?;
        if !items.is_empty() {
            params.set(p.name.clone(), ParamValue::List(items));
        }
    }
    Ok(())
}

/// [`seed_lists`] d'un élément et, récursivement, de ceux de ses slots.
pub(crate) fn seed_element(registry: &WidgetRegistry, element: &mut Element) -> Result<(), OpError> {
    seed_lists(registry, &element.kind, &mut element.params)?;
    seed_slots(registry, &mut element.params)
}

/// [`seed_lists`] de chaque élément des slots de `params`, récursivement.
pub(crate) fn seed_slots(registry: &WidgetRegistry, params: &mut Params) -> Result<(), OpError> {
    for (_, children) in params.slots_mut() {
        for child in children {
            seed_element(registry, child)?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "list_tests.rs"]
mod list_tests;
