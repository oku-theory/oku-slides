//! Réconciliation : amène les conteneurs Loro à l'état d'un `Deck` JSON, par diff.

use std::collections::HashMap;

use loro::{Container, LoroMap, LoroMovableList, LoroText, LoroValue, UpdateOptions, ValueOrContainer};
use serde_json::{Map, Value};
use snafu::ResultExt;

use super::schema::{Ctx, Kind, RICH, declared, param_kind};
use crate::error::{LoroSnafu, ReplicaInner, TextUpdateSnafu};
use crate::widget::{WidgetRegistry, WidgetSchema};

type Res = Result<(), ReplicaInner>;

pub(super) fn json_to_loro(v: &Value) -> LoroValue {
    match v {
        Value::Null => LoroValue::Null,
        Value::Bool(b) => LoroValue::Bool(*b),
        Value::Number(n) => match n.as_i64() {
            Some(i) => LoroValue::I64(i),
            None => LoroValue::Double(n.as_f64().unwrap_or_default()),
        },
        Value::String(s) => LoroValue::from(s.as_str()),
        Value::Array(a) => LoroValue::from(a.iter().map(json_to_loro).collect::<Vec<_>>()),
        Value::Object(o) => {
            LoroValue::from(o.iter().map(|(k, v)| (k.clone(), json_to_loro(v))).collect::<HashMap<_, _>>())
        }
    }
}

pub(super) fn text_at(map: &LoroMap, key: &str) -> Result<LoroText, ReplicaInner> {
    match map.get(key) {
        Some(ValueOrContainer::Container(Container::Text(t))) => Ok(t),
        _ => map.insert_container(key, LoroText::new()).context(LoroSnafu),
    }
}

fn list_at(map: &LoroMap, key: &str) -> Result<LoroMovableList, ReplicaInner> {
    match map.get(key) {
        Some(ValueOrContainer::Container(Container::MovableList(l))) => Ok(l),
        _ => map.insert_container(key, LoroMovableList::new()).context(LoroSnafu),
    }
}

fn map_at(map: &LoroMap, key: &str) -> Result<LoroMap, ReplicaInner> {
    match map.get(key) {
        Some(ValueOrContainer::Container(Container::Map(m))) => Ok(m),
        _ => map.insert_container(key, LoroMap::new()).context(LoroSnafu),
    }
}

fn set_value(map: &LoroMap, key: &str, v: LoroValue) -> Res {
    if matches!(map.get(key), Some(ValueOrContainer::Value(old)) if old == v) {
        return Ok(());
    }
    map.insert(key, v).context(LoroSnafu)
}

pub(super) fn sync_text(t: &LoroText, s: &str) -> Res {
    if t.to_string() != s {
        t.update(s, UpdateOptions::default()).context(TextUpdateSnafu)?;
    }
    Ok(())
}

/// Committe les changements en cours ; appelé avant chaque `mov` (voir [`Writer::sync_list`]).
pub(super) type Split<'a> = &'a dyn Fn();

/// La carte `{rich: LoroText}` d'un paramètre `rich` : réutilisée si elle
/// ne porte rien d'autre, remplacée sinon.
fn rich_at(map: &LoroMap, key: &str) -> Result<LoroText, ReplicaInner> {
    let m = match map.get(key) {
        Some(ValueOrContainer::Container(Container::Map(m))) if m.keys().all(|k| &*k == RICH) => m,
        _ => map.insert_container(key, LoroMap::new()).context(LoroSnafu)?,
    };
    text_at(&m, RICH)
}

/// Vide le conteneur `entry` de `map[key]` ; une valeur, ou une carte
/// ordinaire, est retirée.
fn empty_or_delete(map: &LoroMap, key: &str, entry: ValueOrContainer) -> Res {
    match entry {
        ValueOrContainer::Container(Container::Text(t)) => sync_text(&t, ""),
        ValueOrContainer::Container(Container::MovableList(l)) => clear(&l),
        ValueOrContainer::Container(Container::Map(m)) if m.len() == 1 && m.get(RICH).is_some() => {
            super::rich::sync(&text_at(&m, RICH)?, &Value::Array(Vec::new()))
        }
        _ => map.delete(key).context(LoroSnafu),
    }
}

fn clear(l: &LoroMovableList) -> Res {
    if !l.is_empty() {
        l.delete(0, l.len()).context(LoroSnafu)?;
    }
    Ok(())
}

/// Réconciliation d'un deck : le registre donne le type des paramètres.
pub(super) struct Writer<'a> {
    pub(super) registry: &'a WidgetRegistry,
    pub(super) split: Split<'a>,
}

impl Writer<'_> {
    /// Amène `map` à l'état de `obj`.
    pub(super) fn sync_map(&self, map: &LoroMap, obj: &Map<String, Value>, ctx: Ctx) -> Res {
        // Structure toujours présente : jamais de création concurrente d'une même clé.
        for key in ctx.texts() {
            text_at(map, key)?;
        }
        for key in ctx.lists() {
            list_at(map, key)?;
        }
        let owner = matches!(ctx, Ctx::Slide | Ctx::Element);
        let stale: Vec<String> = map.keys().map(|k| k.to_string()).filter(|k| !obj.contains_key(k)).collect();
        for key in stale {
            match ctx.kind(&key) {
                // `params` d'une slide ou d'un élément : réconcilié plus bas.
                Kind::Nested(Ctx::Params) if owner => {}
                // Absent = vide : on vide, la clé reste.
                Kind::Prose => sync_text(&text_at(map, &key)?, "")?,
                Kind::Rich => super::rich::sync(&rich_at(map, &key)?, &Value::Array(Vec::new()))?,
                Kind::Collection(_) => clear(&list_at(map, &key)?)?,
                Kind::Atom | Kind::Nested(_) => map.delete(&key).context(LoroSnafu)?,
            }
        }
        for (key, value) in obj {
            match ctx.kind(key) {
                Kind::Nested(Ctx::Params) if owner => {}
                Kind::Prose => sync_text(&text_at(map, key)?, value.as_str().unwrap_or_default())?,
                Kind::Rich => super::rich::sync(&rich_at(map, key)?, value)?,
                Kind::Collection(child) => {
                    self.sync_list(&list_at(map, key)?, value.as_array().map(Vec::as_slice).unwrap_or_default(), child)?
                }
                Kind::Atom => set_value(map, key, json_to_loro(value))?,
                Kind::Nested(child) => match value {
                    Value::Object(o) => self.sync_map(&map_at(map, key)?, o, child)?,
                    v => set_value(map, key, json_to_loro(v))?,
                },
            }
        }
        if owner {
            // Toujours créée, même vide : deux premiers paramètres posés en
            // concurrence ne créent pas chacun leur carte (parade t-188).
            let schema = obj.get("kind").and_then(Value::as_str).and_then(|k| self.registry.get(k));
            let empty = Map::new();
            let params = obj.get("params").and_then(Value::as_object).unwrap_or(&empty);
            self.sync_params(&map_at(map, "params")?, params, schema)?;
        }
        Ok(())
    }

    /// Amène les paramètres `map` d'un widget de schéma `schema` (s'il est
    /// connu) à l'état de `obj`.
    ///
    /// Un paramètre absent de `obj` est vidé s'il a un conteneur (texte,
    /// texte riche, slot), retiré sinon ; chaque paramètre `plain`, `rich` ou
    /// `slot` déclaré est présent, vide au besoin (parade t-188).
    fn sync_params(&self, map: &LoroMap, obj: &Map<String, Value>, schema: Option<&WidgetSchema>) -> Res {
        let stale: Vec<String> = map.keys().map(|k| k.to_string()).filter(|k| !obj.contains_key(k)).collect();
        for key in stale {
            if let Some(entry) = map.get(&key) {
                empty_or_delete(map, &key, entry)?;
            }
        }
        for (name, kind) in declared(schema).filter(|(name, _)| !obj.contains_key(*name)) {
            match kind {
                Kind::Prose => drop(text_at(map, name)?),
                Kind::Rich => drop(rich_at(map, name)?),
                Kind::Collection(_) => drop(list_at(map, name)?),
                Kind::Atom | Kind::Nested(_) => {}
            }
        }
        for (key, value) in obj {
            match param_kind(schema, key, value) {
                Kind::Prose => sync_text(&text_at(map, key)?, value.as_str().unwrap_or_default())?,
                Kind::Rich => super::rich::sync(&rich_at(map, key)?, value)?,
                Kind::Collection(child) => {
                    self.sync_list(&list_at(map, key)?, value.as_array().map(Vec::as_slice).unwrap_or_default(), child)?
                }
                Kind::Atom | Kind::Nested(_) => set_value(map, key, json_to_loro(value))?,
            }
        }
        Ok(())
    }
}

fn item_id(list: &LoroMovableList, i: usize) -> String {
    match list.get(i) {
        Some(ValueOrContainer::Container(Container::Map(m))) => match m.get("id") {
            Some(ValueOrContainer::Value(LoroValue::String(s))) => s.to_string(),
            _ => String::new(),
        },
        _ => String::new(),
    }
}

/// Clé stable d'une position, qui distingue les occurrences d'un même id.
///
/// `validate` signale les ids vides ou dupliqués (`issue-empty-element-id`,
/// `issue-duplicate-element-id`), mais ne les interdit pas : la réconciliation
/// doit rester correcte face à eux (ni échec, ni divergence), d'où
/// l'appariement par occurrence plutôt que par id nu.
fn keyed(ids: &[String]) -> Vec<(String, usize)> {
    let mut seen: HashMap<&str, usize> = HashMap::new();
    ids.iter()
        .map(|id| {
            let n = seen.entry(id.as_str()).or_insert(0);
            let key = (id.clone(), *n);
            *n += 1;
            key
        })
        .collect()
}

impl Writer<'_> {
    /// Réconcilie une collection à `id` : suppressions, puis insertions et déplacements.
    ///
    /// Chaque `mov` part dans son propre commit (`split` juste avant) : Loro
    /// 1.16.2 défait mal un item d'annulation qui en porte plusieurs sur une
    /// liste de conteneurs (élément perdu, voire panique ; t-263).
    fn sync_list(&self, list: &LoroMovableList, items: &[Value], ctx: Ctx) -> Res {
        let id_of = |v: &Value| v.get("id").and_then(Value::as_str).unwrap_or_default().to_owned();
        let wanted: Vec<String> = items.iter().map(id_of).collect();
        let wanted_keys = keyed(&wanted);
        let mut keys = keyed(&(0..list.len()).map(|i| item_id(list, i)).collect::<Vec<_>>());
        for i in (0..keys.len()).rev() {
            if !wanted_keys.contains(&keys[i]) {
                list.delete(i, 1).context(LoroSnafu)?;
                keys.remove(i);
            }
        }
        for (target, item) in items.iter().enumerate() {
            let key = &wanted_keys[target];
            match keys.iter().position(|k| k == key) {
                Some(pos) if pos == target => {}
                Some(pos) => {
                    (self.split)();
                    list.mov(pos, target).context(LoroSnafu)?;
                    let moved = keys.remove(pos);
                    keys.insert(target, moved);
                }
                None => {
                    list.insert_container(target, LoroMap::new()).context(LoroSnafu)?;
                    keys.insert(target, key.clone());
                }
            }
            if let (Some(ValueOrContainer::Container(Container::Map(m))), Some(obj)) =
                (list.get(target), item.as_object())
            {
                self.sync_map(&m, obj, ctx)?;
            }
        }
        // Défense en profondeur : l'appariement par occurrence ci-dessus ne
        // devrait jamais laisser de position au-delà de `items.len()`, mais des
        // ids vides ou dupliqués le rendent fragile ; on s'en assure explicitement.
        if list.len() > items.len() {
            list.delete(items.len(), list.len() - items.len()).context(LoroSnafu)?;
        }
        Ok(())
    }
}
