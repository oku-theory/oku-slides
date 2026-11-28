//! Deck-level data in the replica: `deck.data/<id>`, written by a keyed diff.
//!
//! Layout: `data/<id>` map (`name`, `label`, `kind`, `inline`); a query data adds its `sql`
//! atom and has no table, a manual one holds `table`, a child map per
//! generation (`generation`, `columns`, `rows`) → `layer`, a child map per epoch (`epoch`,
//! `base`, `cells`). Columns are a movable list of `{id, name, type}` maps, rows a movable list
//! of ids, cells a map keyed `<row id>/<column id>`. A new generation is a new `table`
//! container, a compaction a new `layer` container: concurrent writers resolve
//! last-writer-wins, and edits to the losing container are lost ([`lost`](super::data_lost)).
//! A removed data keeps its map with this peer's deletion mark (`@deleted/<peer>`), hidden like
//! a marked tree node: undoing the removal lifts the mark and keeps a peer's concurrent edits.
//! Unlike tree nodes, `compact` never purges it: the removed table stays in the doc.
//!
//! Reading is in `data_read`; it repairs keys the way [`keyed`] does here.

use std::collections::BTreeMap;
use std::sync::Arc;

use loro::{Container, Frontiers, LoroMap, LoroMovableList, LoroValue, ValueOrContainer};
use oku_slides_data::{CellKey, Column, Data, DataCell, DataId, IdRepair, Source, Table, valid_id};
use snafu::ResultExt;

use super::schema::{CELLS, COLUMNS, ROWS};
use super::tree::is_marked;
use super::write::{Res, Writer, item_id, json_to_loro, list_at, map_at, set_value};
use crate::error::{DataKindSwitchSnafu, DeckJsonSnafu, LoroSnafu};
use crate::model::Deck;

/// Key of the deck map that holds the data.
pub(super) const DATA: &str = "data";
/// Key of a data's map that holds its kind: `manual` or `query`.
pub(super) const KIND: &str = "kind";
/// Key of a query data's map that holds its SQL, last-writer-wins.
pub(super) const SQL: &str = "sql";
/// Key of a manual data's map that holds its current generation.
pub(super) const TABLE: &str = "table";
/// Key of a table's map that holds its base and its overlay.
pub(super) const LAYER: &str = "layer";
/// Key of a layer that holds the base reference.
pub(super) const BASE: &str = "base";
/// Key of a table's map that holds its generation id.
pub(super) const GENERATION: &str = "generation";
/// Key of a layer that holds its epoch id: empty for a generation's first layer.
pub(super) const EPOCH: &str = "epoch";

/// What the last application wrote of each data, valid while the doc state is unchanged.
#[derive(Debug)]
pub(super) struct Written {
    /// State of the doc right after that application.
    pub(super) frontiers: Frontiers,
    /// The data it wrote.
    pub(super) data: BTreeMap<DataId, Arc<Data>>,
}

/// The data keys of `map` not marked removed, each with the id it reads as, sorted by id.
///
/// Valid keys keep their id; invalid ones are repaired from `x`, after every valid key, in
/// key order ([`IdRepair`]): the same on every peer.
pub(super) fn keyed(map: &LoroMap) -> Vec<(DataId, String)> {
    let mut keys: Vec<String> = map.keys().map(|k| k.to_string()).filter(|k| !removed(map, k)).collect();
    keys.sort_unstable();
    let (valid, invalid): (Vec<String>, Vec<String>) = keys.into_iter().partition(|k| valid_id(k));
    let mut repair = IdRepair::new();
    let mut out: Vec<(DataId, String)> = valid
        .into_iter()
        .chain(invalid)
        .filter_map(|key| DataId::try_from(repair.repair_id(&key)).ok().map(|id| (id, key)))
        .collect();
    out.sort_unstable();
    out
}

/// Whether the data at `key` carries a deletion mark.
fn removed(map: &LoroMap, key: &str) -> bool {
    matches!(map.get(key), Some(ValueOrContainer::Container(Container::Map(m))) if is_marked(&m))
}

/// A table's generation id; empty when missing or not a string.
pub(super) fn generation_of(table: &LoroMap) -> String {
    match table.get(GENERATION) {
        Some(ValueOrContainer::Value(LoroValue::String(s))) => s.to_string(),
        _ => String::new(),
    }
}

/// A layer's epoch id; empty when missing or not a string (a generation's first layer).
pub(super) fn epoch_of(layer: &LoroMap) -> String {
    match layer.get(EPOCH) {
        Some(ValueOrContainer::Value(LoroValue::String(s))) => s.to_string(),
        _ => String::new(),
    }
}

/// A cell as stored: numbers as doubles.
fn cell_value(cell: &DataCell) -> LoroValue {
    match cell {
        DataCell::Number(n) => LoroValue::Double(*n),
        DataCell::Text(s) => LoroValue::from(s.as_str()),
        DataCell::Bool(b) => LoroValue::Bool(*b),
        DataCell::Null => LoroValue::Null,
    }
}

fn cell_key(key: &CellKey) -> String {
    format!("{}/{}", key.row, key.column)
}

/// Refuses `deck` when one of its data has another kind than the live map of the same id.
///
/// A data never changes kind in place: no op switches it, and a whole-deck write that would
/// (`apply`, `restore`) is refused before anything is written. Switching would delete one
/// side's container, and a peer's concurrent edits to it would be lost without a report
/// ([`lost`](super::data_lost) only compares tables).
pub(super) fn check_kinds(root: &LoroMap, deck: &Deck) -> Res {
    let Some(ValueOrContainer::Container(Container::Map(map))) = root.get(DATA) else { return Ok(()) };
    for (id, data) in &deck.data {
        if deck.truncated.contains(id) || removed(&map, id.as_str()) {
            continue;
        }
        let Some(ValueOrContainer::Container(Container::Map(m))) = map.get(id.as_str()) else { continue };
        if let Some(ValueOrContainer::Value(LoroValue::String(kind))) = m.get(KIND) {
            snafu::ensure!(kind.as_str() == data.kind().as_str(), DataKindSwitchSnafu { id: id.as_str() });
        }
    }
    Ok(())
}

/// The deletion mark a removal sets: this peer's key and the Unix time (s).
pub(super) struct Mark<'a> {
    pub(super) key: &'a str,
    pub(super) now: i64,
}

/// Brings `deck.data` of the deck map `root` to the state of `deck`.
///
/// A data in `deck.truncated` is never written, and its key never removed. A removed data
/// gets `mark`; a key read under another id (repaired) is deleted, and so is a marked key
/// whose id is added again: it starts a fresh map. `written` holds what the last application
/// wrote, when the doc has not changed since: a data still sharing its `Arc` is skipped, and
/// a table of the same generation is diffed against it instead of the doc.
pub(super) fn sync(root: &LoroMap, deck: &Deck, written: Option<&BTreeMap<DataId, Arc<Data>>>, mark: &Mark<'_>) -> Res {
    let map = map_at(root, DATA)?;
    for (id, key) in keyed(&map) {
        if deck.truncated.contains(&id) || (key == id.as_str() && deck.data.contains_key(&id)) {
            continue;
        }
        match map.get(&key) {
            Some(ValueOrContainer::Container(Container::Map(m))) if key == id.as_str() => {
                set_value(&m, mark.key, LoroValue::I64(mark.now))?;
            }
            _ => map.delete(&key).context(LoroSnafu)?,
        }
    }
    for (id, data) in &deck.data {
        if deck.truncated.contains(id) {
            continue;
        }
        if removed(&map, id.as_str()) {
            map.delete(id.as_str()).context(LoroSnafu)?;
        }
        let old = written.and_then(|w| w.get(id));
        if old.is_some_and(|old| Arc::ptr_eq(old, data)) {
            continue;
        }
        sync_data(&map_at(&map, id.as_str())?, data, old.map(Arc::as_ref))?;
    }
    Ok(())
}

/// Brings the data map `map` to `data`; `old` is what was last written there, if known.
fn sync_data(map: &LoroMap, data: &Data, old: Option<&Data>) -> Res {
    set_value(map, "name", LoroValue::from(data.name.as_str()))?;
    match &data.label {
        Some(label) => set_value(map, "label", LoroValue::from(label.as_str()))?,
        None if map.get("label").is_some() => map.delete("label").context(LoroSnafu)?,
        None => {}
    }
    set_value(map, KIND, LoroValue::from(data.kind().as_str()))?;
    set_value(map, "inline", LoroValue::Bool(data.inline))?;
    match &data.source {
        Source::Manual(table) => {
            if map.get(SQL).is_some() {
                map.delete(SQL).context(LoroSnafu)?;
            }
            sync_manual(map, table, old.and_then(Data::table))
        }
        Source::Query { sql } => {
            // A kind switch is only reachable through a whole-deck write, which `check_kinds`
            // refuses up front; a table here belongs to a map whose kind was unreadable.
            if map.get(TABLE).is_some() {
                map.delete(TABLE).context(LoroSnafu)?;
            }
            set_value(map, SQL, LoroValue::from(sql.as_str()))
        }
    }
}

/// Brings the `table` of the data map `map` to `table`; `old` is what was last written there.
fn sync_manual(map: &LoroMap, table: &Table, old: Option<&Table>) -> Res {
    let current = match map.get(TABLE) {
        Some(ValueOrContainer::Container(Container::Map(t))) if generation_of(&t) == table.generation => Some(t),
        _ => None,
    };
    let (container, old) = match current {
        Some(t) => (t, old.filter(|o| o.generation == table.generation)),
        None => (new_generation(map, table)?, None),
    };
    sync_table(&container, table, old)
}

/// A new `table` container for `generation`, its lists and maps created at once: two peers
/// editing it never create the same child twice.
fn new_generation(map: &LoroMap, of: &Table) -> Result<LoroMap, crate::error::ReplicaInner> {
    let table = map.insert_container(TABLE, LoroMap::new()).context(LoroSnafu)?;
    table.insert(GENERATION, of.generation.as_str()).context(LoroSnafu)?;
    list_at(&table, COLUMNS)?;
    list_at(&table, ROWS)?;
    new_layer(&table, &of.epoch)?;
    Ok(table)
}

/// Brings the table container `map` to `table`; `old`, of the same generation, was last
/// written there.
fn sync_table(map: &LoroMap, table: &Table, old: Option<&Table>) -> Res {
    if old.is_none_or(|o| o.columns != table.columns) {
        sync_columns(&list_at(map, COLUMNS)?, &table.columns)?;
    }
    if old.is_none_or(|o| !o.rows.ptr_eq(&table.rows)) {
        sync_rows(&list_at(map, ROWS)?, table)?;
    }
    let current = match map.get(LAYER) {
        Some(ValueOrContainer::Container(Container::Map(l))) if epoch_of(&l) == table.epoch => Some(l),
        _ => None,
    };
    let (layer, old) = match current {
        Some(l) => (l, old.filter(|o| o.epoch == table.epoch)),
        None => (new_layer(map, &table.epoch)?, None),
    };
    match &table.base {
        Some(base) => {
            let json = serde_json::to_value(base).context(DeckJsonSnafu)?;
            set_value(&layer, BASE, json_to_loro(&json))?;
        }
        None if layer.get(BASE).is_some() => layer.delete(BASE).context(LoroSnafu)?,
        None => {}
    }
    let cells = map_at(&layer, CELLS)?;
    match old {
        Some(old) => diff_cells(&cells, old, table),
        None => sync_cells(&cells, table),
    }
}

/// A new `layer` container for `epoch`, its cells created at once.
fn new_layer(table: &LoroMap, epoch: &str) -> Result<LoroMap, crate::error::ReplicaInner> {
    let layer = table.insert_container(LAYER, LoroMap::new()).context(LoroSnafu)?;
    if !epoch.is_empty() {
        layer.insert(EPOCH, epoch).context(LoroSnafu)?;
    }
    map_at(&layer, CELLS)?;
    Ok(layer)
}

/// Brings the column list to `columns`; repaired ids are renamed in place, as in [`sync_rows`].
fn sync_columns(list: &LoroMovableList, columns: &[Column]) -> Res {
    let wanted: Vec<String> = columns.iter().map(|c| c.id.clone()).collect();
    let mut repair = IdRepair::new();
    let mut current = Vec::with_capacity(list.len());
    for i in 0..list.len() {
        let Some(ValueOrContainer::Container(Container::Map(m))) = list.get(i) else {
            current.push(String::new());
            continue;
        };
        let raw = item_id(list, i);
        let id = repair.repair_id(&raw);
        if id != raw {
            set_value(&m, "id", LoroValue::from(id.as_str()))?;
        }
        current.push(id);
    }
    let insert = |at: usize, _: &str| list.insert_container(at, LoroMap::new()).map(drop).context(LoroSnafu);
    let mut visit = |at: usize| match (list.get(at), columns.get(at)) {
        (Some(ValueOrContainer::Container(Container::Map(m))), Some(column)) => {
            set_value(&m, "id", LoroValue::from(column.id.as_str()))?;
            set_value(&m, "name", LoroValue::from(column.name.as_str()))?;
            set_value(&m, "type", LoroValue::from(column.ty.as_str()))
        }
        _ => Ok(()),
    };
    Writer::reorder(list, &current, &wanted, &insert, &mut visit)
}

/// Brings the row list to `table.rows`.
///
/// Ids the read repaired (a duplicate left by concurrent edits) are first renamed in place, as
/// the read named them: two peers renaming the same entry converge, where a remove and an
/// insert would each leave a copy.
fn sync_rows(list: &LoroMovableList, table: &Table) -> Res {
    let wanted: Vec<String> = table.rows.iter().map(str::to_owned).collect();
    let mut repair = IdRepair::new();
    let mut current = Vec::with_capacity(list.len());
    for i in 0..list.len() {
        let raw = match list.get(i) {
            Some(ValueOrContainer::Value(LoroValue::String(s))) => s.to_string(),
            _ => String::new(),
        };
        let id = repair.repair_id(&raw);
        if id != raw {
            list.set(i, id.as_str()).context(LoroSnafu)?;
        }
        current.push(id);
    }
    let insert = |at: usize, id: &str| list.insert(at, id).context(LoroSnafu);
    Writer::reorder(list, &current, &wanted, &insert, &mut |_| Ok(()))
}

/// Writes the overlay entries that differ from `old`.
fn diff_cells(map: &LoroMap, old: &Table, table: &Table) -> Res {
    for item in old.cells.diff(&table.cells) {
        match item {
            imbl::ordmap::DiffItem::Add(key, cell) | imbl::ordmap::DiffItem::Update { new: (key, cell), .. } => {
                map.insert(&cell_key(key), cell_value(cell)).context(LoroSnafu)?;
            }
            imbl::ordmap::DiffItem::Remove(key, _) => map.delete(&cell_key(key)).context(LoroSnafu)?,
        }
    }
    Ok(())
}

/// Brings the overlay map `map` to `table.cells`, comparing with the doc.
fn sync_cells(map: &LoroMap, table: &Table) -> Res {
    let wanted: BTreeMap<String, &DataCell> = table.cells.iter().map(|(k, v)| (cell_key(k), v)).collect();
    let stale: Vec<String> = map.keys().map(|k| k.to_string()).filter(|k| !wanted.contains_key(k)).collect();
    for key in stale {
        map.delete(&key).context(LoroSnafu)?;
    }
    for (key, cell) in wanted {
        set_value(map, &key, cell_value(cell))?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "data_tests.rs"]
mod data_tests;
