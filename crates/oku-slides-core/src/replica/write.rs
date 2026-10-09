//! Réconciliation : amène les conteneurs Loro à l'état d'un `Deck` JSON, par diff.

use std::collections::{HashMap, HashSet};

use loro::{Container, LoroMap, LoroMovableList, LoroText, LoroValue, UpdateOptions, ValueOrContainer};
use serde_json::{Map, Value};
use snafu::ResultExt;

use super::schema::{Ctx, ELEMENTS, Kind, LIST, RICH, column_kind, columns, declared, param_kind};
use super::tree::{elements, slots_of, tree_at, wanted_index};
use crate::error::{CollectionInParamsSnafu, ContainerOutsideParamsSnafu, LoroSnafu, ReplicaInner, TextUpdateSnafu};
use crate::widget::{ParamSchema, WidgetRegistry};

pub(super) type Res = Result<(), ReplicaInner>;

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

pub(super) fn list_at(map: &LoroMap, key: &str) -> Result<LoroMovableList, ReplicaInner> {
    match map.get(key) {
        Some(ValueOrContainer::Container(Container::MovableList(l))) => Ok(l),
        _ => map.insert_container(key, LoroMovableList::new()).context(LoroSnafu),
    }
}

pub(super) fn map_at(map: &LoroMap, key: &str) -> Result<LoroMap, ReplicaInner> {
    match map.get(key) {
        Some(ValueOrContainer::Container(Container::Map(m))) => Ok(m),
        _ => map.insert_container(key, LoroMap::new()).context(LoroSnafu),
    }
}

pub(super) fn set_value(map: &LoroMap, key: &str, v: LoroValue) -> Res {
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

/// La liste des items de la carte `{list: LoroMovableList}` d'un paramètre
/// `list` : réutilisée si elle ne porte rien d'autre, remplacée sinon.
fn items_at(map: &LoroMap, key: &str) -> Result<LoroMovableList, ReplicaInner> {
    let m = match map.get(key) {
        Some(ValueOrContainer::Container(Container::Map(m))) if m.keys().all(|k| &*k == LIST) => m,
        _ => map.insert_container(key, LoroMap::new()).context(LoroSnafu)?,
    };
    list_at(&m, LIST)
}

/// Vide le conteneur `entry` de `map[key]` ; une valeur, ou une carte
/// ordinaire, est retirée.
fn empty_or_delete(map: &LoroMap, key: &str, entry: ValueOrContainer) -> Res {
    match entry {
        ValueOrContainer::Container(Container::Text(t)) => sync_text(&t, ""),
        ValueOrContainer::Container(Container::Map(m)) if m.len() == 1 && m.get(RICH).is_some() => {
            super::rich::sync(&text_at(&m, RICH)?, &Value::Array(Vec::new()))
        }
        ValueOrContainer::Container(Container::Map(m)) if m.len() == 1 && m.get(LIST).is_some() => {
            clear(&list_at(&m, LIST)?)
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
        let owner = matches!(ctx, Ctx::Slide | Ctx::Element);
        let schema = obj.get("kind").and_then(Value::as_str).and_then(|k| self.registry.get(k));
        let schema = schema.map(|w| w.params.as_slice());
        let empty = Map::new();
        let params = obj.get("params").and_then(Value::as_object).unwrap_or(&empty);
        // Avant toute écriture : `apply` ne revient pas en arrière, un deck
        // trop profond ne doit pas laisser une slide à moitié écrite.
        let wanted = match ctx {
            Ctx::Slide => Some(wanted_index(self.registry, &slots_of(Some(params), schema))?),
            _ => None,
        };
        // Structure toujours présente : jamais de création concurrente d'une même clé.
        for key in ctx.texts() {
            text_at(map, key)?;
        }
        for key in ctx.lists() {
            list_at(map, key)?;
        }
        for key in ctx.trees() {
            tree_at(map, key)?;
        }
        let reserved = ctx.reserved();
        let stale: Vec<String> = map
            .keys()
            .map(|k| k.to_string())
            .filter(|k| !obj.contains_key(k) && !reserved.contains(&k.as_str()))
            .collect();
        for key in stale {
            match ctx.kind(&key) {
                // `params` d'une slide ou d'un élément : réconcilié plus bas.
                Kind::Nested(Ctx::Params) if owner => {}
                // Absent = vide : on vide, la clé reste.
                Kind::Prose => sync_text(&text_at(map, &key)?, "")?,
                Kind::Rich => super::rich::sync(&rich_at(map, &key)?, &Value::Array(Vec::new()))?,
                Kind::Collection => clear(&list_at(map, &key)?)?,
                // Jamais rendus par `Ctx::kind` : un slot, une liste ou un hôte n'existe que dans les paramètres.
                Kind::Slot | Kind::List | Kind::Host => return ContainerOutsideParamsSnafu.fail(),
                Kind::Atom | Kind::Nested(_) => map.delete(&key).context(LoroSnafu)?,
            }
        }
        for (key, value) in obj {
            match ctx.kind(key) {
                Kind::Nested(Ctx::Params) if owner => {}
                Kind::Prose => sync_text(&text_at(map, key)?, value.as_str().unwrap_or_default())?,
                Kind::Rich => super::rich::sync(&rich_at(map, key)?, value)?,
                Kind::Collection => {
                    self.sync_list(&list_at(map, key)?, elements(value), &|m, o| self.sync_map(m, o, Ctx::Slide))?
                }
                // Jamais rendus par `Ctx::kind` : un slot, une liste ou un hôte n'existe que dans les paramètres.
                Kind::Slot | Kind::List | Kind::Host => return ContainerOutsideParamsSnafu.fail(),
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
            self.sync_params(&map_at(map, "params")?, params, schema)?;
            // Les éléments d'une slide sont dans son arbre ; ceux d'un
            // élément, ses enfants dans ce même arbre.
            if let Some(wanted) = wanted {
                self.sync_tree(&tree_at(map, ELEMENTS)?, wanted)?;
            }
        }
        Ok(())
    }

    /// Amène les paramètres `map` d'un widget aux paramètres déclarés
    /// `params` (s'il est connu) à l'état de `obj`.
    fn sync_params(&self, map: &LoroMap, obj: &Map<String, Value>, params: Option<&[ParamSchema]>) -> Res {
        self.sync_fields(map, obj, params, param_kind)
    }

    /// Amène l'item `map` d'une liste aux colonnes `columns` à l'état de
    /// `obj` : son `id` en valeur (clé de réconciliation, voir [`item_id`]),
    /// ses colonnes comme des paramètres.
    fn sync_item(&self, map: &LoroMap, obj: &Map<String, Value>, columns: Option<&[ParamSchema]>) -> Res {
        self.sync_fields(map, obj, columns, column_kind)
    }

    /// Amène les champs `map` (paramètres d'un widget ou colonnes d'un
    /// item), déclarés par `fields`, à l'état de `obj` ; `kind` donne la
    /// catégorie de chacun.
    ///
    /// Un champ absent de `obj` est vidé s'il a un conteneur (texte, texte
    /// riche, liste), retiré sinon ; chaque champ `plain`, `rich` ou `list`
    /// déclaré est présent, vide au besoin (parade t-188). Un slot n'est pas
    /// dans `map` : ses éléments sont dans l'arbre de la slide (`tree.rs`).
    fn sync_fields(
        &self,
        map: &LoroMap,
        obj: &Map<String, Value>,
        fields: Option<&[ParamSchema]>,
        kind: fn(Option<&[ParamSchema]>, &str, &Value) -> Kind,
    ) -> Res {
        let stale: Vec<String> = map.keys().map(|k| k.to_string()).filter(|k| !obj.contains_key(k)).collect();
        for key in stale {
            if let Some(entry) = map.get(&key) {
                empty_or_delete(map, &key, entry)?;
            }
        }
        for (name, kind) in declared(fields).filter(|(name, _)| !obj.contains_key(*name)) {
            match kind {
                Kind::Prose => drop(text_at(map, name)?),
                Kind::Rich => drop(rich_at(map, name)?),
                Kind::List => drop(items_at(map, name)?),
                // Pas créé d'avance : la forme du dataset est une donnée ; un
                // slot vit dans l'arbre de la slide, créé avec elle.
                Kind::Host | Kind::Slot | Kind::Collection | Kind::Atom | Kind::Nested(_) => {}
            }
        }
        for (key, value) in obj {
            match kind(fields, key, value) {
                Kind::Prose => sync_text(&text_at(map, key)?, value.as_str().unwrap_or_default())?,
                Kind::Rich => super::rich::sync(&rich_at(map, key)?, value)?,
                // Écrit dans l'arbre de la slide (`tree.rs`) : une valeur
                // restée sous ce nom est retirée.
                Kind::Slot => {
                    if map.get(key).is_some() {
                        map.delete(key).context(LoroSnafu)?
                    }
                }
                // Jamais rendu par `param_kind` ni `column_kind` : les slides seules.
                Kind::Collection => return CollectionInParamsSnafu.fail(),
                Kind::List => {
                    let columns = columns(fields, key);
                    self.sync_list(&items_at(map, key)?, elements(value), &|m, o| self.sync_item(m, o, columns))?
                }
                Kind::Host => match value {
                    Value::Object(o) => self.sync_host(&map_at(map, key)?, o)?,
                    v => set_value(map, key, json_to_loro(v))?,
                },
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
pub(super) fn keyed(ids: &[String]) -> Vec<(String, usize)> {
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
    /// Réconcilie une collection à `id` : suppressions, puis insertions et
    /// déplacements ([`Self::reorder`]) ; `each` amène chaque objet à son état.
    pub(super) fn sync_list(
        &self,
        list: &LoroMovableList,
        items: &[Value],
        each: &dyn Fn(&LoroMap, &Map<String, Value>) -> Res,
    ) -> Res {
        let id_of = |v: &Value| v.get("id").and_then(Value::as_str).unwrap_or_default().to_owned();
        let wanted: Vec<String> = items.iter().map(id_of).collect();
        let current: Vec<String> = (0..list.len()).map(|i| item_id(list, i)).collect();
        let insert = |at: usize, _: &str| list.insert_container(at, LoroMap::new()).map(drop).context(LoroSnafu);
        let mut visit = |at: usize| match (list.get(at), items.get(at).and_then(Value::as_object)) {
            (Some(ValueOrContainer::Container(Container::Map(m))), Some(obj)) => each(&m, obj),
            _ => Ok(()),
        };
        self.reorder(list, &current, &wanted, &insert, &mut visit)
    }

    /// Amène l'ordre de `list`, d'identifiants `current`, à `wanted` :
    /// suppressions, puis insertions (`insert`) et déplacements ; `visit`
    /// voit chaque position une fois en place. Sert aux slides et aux items
    /// de liste ; les éléments sont dans un arbre, voir `tree.rs`.
    ///
    /// Appariement par occurrence ([`keyed`]) et positions indexées : linéaire
    /// quand rien ne change, un passage de la liste par insertion ou déplacement.
    ///
    /// Chaque `mov` part dans son propre commit (`split` juste avant) : Loro
    /// 1.16.2 défait mal un item d'annulation qui en porte plusieurs sur une
    /// liste de conteneurs (élément perdu, voire panique ; t-263).
    pub(super) fn reorder(
        &self,
        list: &LoroMovableList,
        current: &[String],
        wanted: &[String],
        insert: &dyn Fn(usize, &str) -> Res,
        visit: &mut dyn FnMut(usize) -> Res,
    ) -> Res {
        let wanted_keys = keyed(wanted);
        let wanted_set: HashSet<&(String, usize)> = wanted_keys.iter().collect();
        let all = keyed(current);
        for i in (0..all.len()).rev() {
            if !wanted_set.contains(&all[i]) {
                list.delete(i, 1).context(LoroSnafu)?;
            }
        }
        let mut keys: Vec<(String, usize)> = all.into_iter().filter(|k| wanted_set.contains(k)).collect();
        let mut pos: HashMap<(String, usize), usize> = keys.iter().cloned().enumerate().map(|(i, k)| (k, i)).collect();
        for (target, key) in wanted_keys.iter().enumerate() {
            if keys.get(target) != Some(key) {
                match pos.get(key).copied() {
                    // Les positions avant `target` sont déjà en place : `from > target`.
                    Some(from) => {
                        (self.split)();
                        list.mov(from, target).context(LoroSnafu)?;
                        let moved = keys.remove(from);
                        keys.insert(target, moved);
                        reindex(&keys, &mut pos, target, from + 1);
                    }
                    None => {
                        insert(target, &key.0)?;
                        keys.insert(target, key.clone());
                        reindex(&keys, &mut pos, target, keys.len());
                    }
                }
            }
            visit(target)?;
        }
        // Défense en profondeur : l'appariement par occurrence ne devrait
        // jamais laisser de position au-delà de `wanted`, mais des ids vides
        // ou dupliqués le rendent fragile ; on s'en assure explicitement.
        if list.len() > wanted.len() {
            list.delete(wanted.len(), list.len() - wanted.len()).context(LoroSnafu)?;
        }
        Ok(())
    }
}

/// Met à jour les positions de `keys[from..to]`.
fn reindex(keys: &[(String, usize)], pos: &mut HashMap<(String, usize), usize>, from: usize, to: usize) {
    for (i, k) in keys.iter().enumerate().take(to).skip(from) {
        pos.insert(k.clone(), i);
    }
}
