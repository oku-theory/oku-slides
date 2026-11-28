//! Copies of a pack's example data into a deck: the data an added example names.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use oku_slides_core::{Deck, Op, OpError};
use oku_slides_data::{Data, DataId, IdGen, NewTable, Source, Table, default_name};

/// What the copy of a query data is.
#[derive(Clone, Copy)]
pub(crate) enum Queries<'a> {
    /// A query data cannot be copied: its inputs are not here.
    Refused,
    /// The copy is the query itself, which reads the deck's own data of the same names.
    Kept,
    /// The copy is manual data holding the query's ready result, by data id; a query with no
    /// result there cannot be copied.
    Values(&'a BTreeMap<DataId, Table>),
}

/// One `AddData` per distinct data of `source` that `refs` names, and the old → new id map.
///
/// Each copy draws a new id from `ids` (none of `deck`'s or `reserved`'s, the ids the batch
/// adds itself), a new generation, and a name unique
/// in `deck` and among the copies (`default_name`); it keeps the label, the row and column ids
/// and the cells. A query data is copied as `queries` says, with its label. A ref `source`
/// does not hold is left out of the map.
///
/// # Errors
///
/// An invalid drawn id; a query data `queries` does not allow, or has no result for.
pub(crate) fn copy_data_ops(
    source: &BTreeMap<DataId, Arc<Data>>,
    refs: &[DataId],
    deck: &Deck,
    reserved: &BTreeSet<String>,
    ids: &mut IdGen<'_>,
    queries: Queries<'_>,
) -> Result<(Vec<Op>, BTreeMap<DataId, DataId>), OpError> {
    let mut names: BTreeSet<String> = deck.data.values().map(|d| d.name.to_lowercase()).collect();
    let mut drawn = BTreeSet::new();
    let mut ops = Vec::new();
    let mut map = BTreeMap::new();
    for old in refs {
        let Some(data) = source.get(old) else { continue };
        if map.contains_key(old) {
            continue;
        }
        let taken = |c: &str| deck.data.contains_key(c) || reserved.contains(c) || drawn.contains(c);
        let id = DataId::try_from(ids(&taken))?;
        drawn.insert(id.as_str().to_owned());
        let (name, _) = default_name(&data.name, 2, |n| names.contains(&n.to_lowercase()));
        names.insert(name.to_lowercase());
        let derived = || OpError::data_derived(old.as_str());
        let (table, sql) = match (&data.source, queries) {
            (Source::Manual(table), _) => (Some(table), None),
            (Source::Query { sql }, Queries::Kept) => (None, Some(sql.clone())),
            (Source::Query { .. }, Queries::Values(results)) => (Some(results.get(old).ok_or_else(derived)?), None),
            (Source::Query { .. }, Queries::Refused) => return Err(derived()),
        };
        let table = table.map(|from| NewTable { generation: Some(ids(&|_| false)), ..NewTable::from(from) });
        ops.push(Op::AddData { id: Some(id.clone()), name, label: data.label.clone(), table, sql });
        map.insert(old.clone(), id);
    }
    Ok((ops, map))
}

#[cfg(test)]
#[path = "data_copy_tests.rs"]
mod data_copy_tests;
