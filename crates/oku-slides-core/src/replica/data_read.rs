//! Reads `deck.data` under bounds: lengths checked before iterating, ids repaired, cuts reported.
//!
//! Data are read in id order ([`keyed`]): the first [`MAX_DATA`] are kept, and each one gets
//! what is left of the deck bounds ([`DeckLoad`]). A data cut by a bound keeps the exact
//! prefix that fits, is reported (`issue-data-truncated`) and lands in `Deck.truncated`,
//! so it is never written back. A stored base that does not parse is reported missing
//! (`issue-data-base-missing`); its data is kept read-only the same way.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use loro::{Container, LoroMap, LoroMovableList, LoroValue, ValueOrContainer};
use oku_slides_data::limits::{MAX_COLUMNS, MAX_DATA, MAX_NAME_CHARS, MAX_OVERLAY, MAX_ROWS, MAX_TEXT_BYTES};
use oku_slides_data::{
    BaseRef, CellKey, Column, ColumnType, Data, DataCell, DataId, DataKind, IdRepair, Rows, Table, clean_label,
};
use oku_slides_i18n::Message;
use serde_json::Value;

use super::data::{BASE, DATA, LAYER, TABLE, epoch_of, generation_of, keyed};
use super::read::loro_to_json;
use super::schema::{CELLS, COLUMNS, ROWS};
use crate::data_bounds::DeckLoad;
use crate::validate::Issue;
use crate::validate_data::{base_missing, cell_type, data_issue, too_many, truncated};

/// The data read from a replica.
#[derive(Debug, Default)]
pub(super) struct DataRead {
    /// Data kept, by id.
    pub(super) data: BTreeMap<DataId, Arc<Data>>,
    /// Data cut, kept read-only, or not read at all.
    pub(super) truncated: BTreeSet<DataId>,
}

/// Reads `deck.data` of the deck map `root`, reporting into `issues`.
pub(super) fn read(root: &LoroMap, issues: &mut Vec<Issue>) -> DataRead {
    let mut out = DataRead::default();
    let Some(ValueOrContainer::Container(Container::Map(map))) = root.get(DATA) else { return out };
    let keyed = keyed(&map);
    let count = keyed.len();
    let mut load = DeckLoad::default();
    let mut dropped = Vec::new();
    for (index, (id, key)) in keyed.into_iter().enumerate() {
        if index >= MAX_DATA {
            dropped.push(id);
            continue;
        }
        let Some(ValueOrContainer::Container(Container::Map(m))) = map.get(&key) else {
            let message = Message::new("issue-unreadable-field").arg("field", format!("{DATA}/{}", id.as_str()));
            issues.push(data_issue(message));
            continue;
        };
        let mut reader = DataReader { left: load.left(), cut: false, base: false, issues: Vec::new() };
        let data = reader.data(&m, &id);
        load = load.plus(DeckLoad::of(&data));
        if reader.cut {
            issues.push(truncated(&data.name));
        }
        issues.append(&mut reader.issues);
        if reader.cut || reader.base {
            out.truncated.insert(id.clone());
        }
        out.data.insert(id, Arc::new(data));
    }
    if !dropped.is_empty() {
        issues.push(data_issue(too_many(count, &dropped)));
        out.truncated.extend(dropped);
    }
    out
}

/// Reads one data within what is left of the deck bounds.
struct DataReader {
    /// What is left of the deck bounds for this data.
    left: DeckLoad,
    /// A bound cut something.
    cut: bool,
    /// A stored base does not parse.
    base: bool,
    /// Issues other than the cut.
    issues: Vec<Issue>,
}

fn string_at(map: &LoroMap, key: &str) -> Option<String> {
    match map.get(key) {
        Some(ValueOrContainer::Value(LoroValue::String(s))) => Some(s.to_string()),
        _ => None,
    }
}

fn map_in(map: &LoroMap, key: &str) -> Option<LoroMap> {
    match map.get(key) {
        Some(ValueOrContainer::Container(Container::Map(m))) => Some(m),
        _ => None,
    }
}

fn list_in(map: &LoroMap, key: &str) -> Option<LoroMovableList> {
    match map.get(key) {
        Some(ValueOrContainer::Container(Container::MovableList(l))) => Some(l),
        _ => None,
    }
}

/// `text` cut to at most `max` bytes, on a character boundary.
fn cut_bytes(text: &str, max: usize) -> &str {
    if text.len() <= max {
        return text;
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

impl DataReader {
    fn data(&mut self, map: &LoroMap, id: &DataId) -> Data {
        let raw = string_at(map, "name").unwrap_or_else(|| id.as_str().to_owned());
        let name: String = clean_label(&raw).chars().take(MAX_NAME_CHARS).collect();
        let label = string_at(map, "label").map(|l| clean_label(&l));
        let inline = matches!(map.get("inline"), Some(ValueOrContainer::Value(LoroValue::Bool(true))));
        // Name and label are kept whole: past the text bound, the data is cut all the same.
        let named = name.len() + label.as_ref().map_or(0, String::len);
        if named > self.left.text {
            self.cut = true;
        }
        self.left.text = self.left.text.saturating_sub(named);
        let table = if let Some(t) = map_in(map, TABLE) {
            self.table(&t, &name)
        } else {
            // No table to read: kept empty, reported and read-only, so it is never written back.
            self.cut = true;
            Table {
                generation: String::new(),
                epoch: String::new(),
                columns: Vec::new(),
                rows: Rows::default(),
                base: None,
                cells: imbl::OrdMap::new(),
            }
        };
        Data { name, label, inline, kind: DataKind::Manual, table }
    }

    fn table(&mut self, map: &LoroMap, name: &str) -> Table {
        let generation = generation_of(map);
        let columns = list_in(map, COLUMNS).map(|l| self.columns(&l, name)).unwrap_or_default();
        let rows = list_in(map, ROWS).map(|l| self.rows(&l)).unwrap_or_default();
        let layer = map_in(map, LAYER);
        let base = layer.as_ref().and_then(|l| self.base(l, name));
        let epoch = layer.as_ref().map(epoch_of).unwrap_or_default();
        let mut table = Table { generation, epoch, columns, rows, base, cells: imbl::OrdMap::new() };
        if let Some(cells) = layer.as_ref().and_then(|l| map_in(l, CELLS)) {
            table.cells = self.cells(&cells, &table);
        }
        table
    }

    /// Columns, ids repaired, names cleaned, within the column and text bounds.
    fn columns(&mut self, list: &LoroMovableList, data: &str) -> Vec<Column> {
        let len = list.len();
        if len > MAX_COLUMNS {
            self.cut = true;
        }
        let mut repair = IdRepair::new();
        let mut out = Vec::with_capacity(len.min(MAX_COLUMNS));
        for i in 0..len.min(MAX_COLUMNS) {
            let Some(ValueOrContainer::Container(Container::Map(m))) = list.get(i) else {
                self.cut = true;
                continue;
            };
            let name = clean_label(&string_at(&m, "name").unwrap_or_default());
            if name.len() > self.left.text {
                self.cut = true;
                break;
            }
            self.left.text -= name.len();
            let raw = string_at(&m, "type").unwrap_or_default();
            let ty = serde_json::from_value::<ColumnType>(Value::String(raw.clone())).unwrap_or_else(|_| {
                self.issues.push(cell_type(data, &name, &clean_label(&raw)));
                ColumnType::Text
            });
            let id = repair.repair_id(&string_at(&m, "id").unwrap_or_default());
            out.push(Column { id, name, ty });
        }
        out
    }

    /// Row ids, repaired, within the row bounds.
    fn rows(&mut self, list: &LoroMovableList) -> Rows {
        let max = MAX_ROWS.min(self.left.row_ids);
        let len = list.len();
        if len > max {
            self.cut = true;
        }
        let ids = (0..len.min(max)).map(|i| match list.get(i) {
            Some(ValueOrContainer::Value(LoroValue::String(s))) => s.to_string(),
            _ => String::new(),
        });
        Rows::repaired(ids.collect::<Vec<_>>())
    }

    /// The stored base reference; one that does not parse is reported missing and keeps the
    /// data read-only. Whether the base itself is there is the bundle's to say (`bases`).
    fn base(&mut self, layer: &LoroMap, data: &str) -> Option<BaseRef> {
        let ValueOrContainer::Value(value) = layer.get(BASE)? else { return None };
        let parsed = serde_json::from_value(loro_to_json(&value)).ok();
        if parsed.is_none() {
            self.base = true;
            self.issues.push(base_missing(data));
        }
        parsed
    }

    /// The overlay, in key order, within the overlay and text bounds; entries of unknown
    /// rows or columns, empty ones without a base, and unreadable ones (a container, a
    /// non-finite or integer number), dropped: they read as empty cells.
    fn cells(&mut self, map: &LoroMap, table: &Table) -> imbl::OrdMap<CellKey, DataCell> {
        let max = MAX_OVERLAY.min(self.left.overlay);
        let mut keys: Vec<String> = map.keys().map(|k| k.to_string()).collect();
        keys.sort_unstable();
        if keys.len() > max {
            self.cut = true;
            keys.truncate(max);
        }
        let columns: BTreeSet<&str> = table.columns.iter().map(|c| c.id.as_str()).collect();
        let mut out = imbl::OrdMap::new();
        for raw in keys {
            let Some(ValueOrContainer::Value(value)) = map.get(&raw) else { continue };
            let Ok(key) = CellKey::try_from(raw) else { continue };
            if !table.rows.contains(&key.row) || !columns.contains(key.column.as_str()) {
                continue;
            }
            let cell = match value {
                LoroValue::Double(n) if n.is_finite() => DataCell::Number(n),
                LoroValue::String(s) => {
                    let kept = cut_bytes(&s, MAX_TEXT_BYTES.min(self.left.text));
                    if kept.len() < s.len() {
                        self.cut = true;
                    }
                    self.left.text -= kept.len();
                    if kept.len() < s.len() && kept.is_empty() {
                        break;
                    }
                    DataCell::Text(kept.to_owned())
                }
                LoroValue::Bool(b) => DataCell::Bool(b),
                LoroValue::Null if table.base.is_some() => DataCell::Null,
                _ => continue,
            };
            out.insert(key, cell);
        }
        out
    }
}
