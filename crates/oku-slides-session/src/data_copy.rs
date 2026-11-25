//! Copies of a pack's example data into a deck: the data an added example names.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use oku_slides_core::{Deck, Op, OpError};
use oku_slides_data::{Data, DataId, IdGen, NewTable, default_name};

/// One `AddData` per distinct data of `source` that `refs` names, and the old → new id map.
///
/// Each copy draws a new id from `ids` (none of `deck`'s or `reserved`'s, the ids the batch
/// adds itself), a new generation, and a name unique
/// in `deck` and among the copies (`default_name`); it keeps the label, the row and column ids
/// and the cells. A ref `source` does not hold is left out of the map.
///
/// # Errors
///
/// An invalid drawn id.
pub(crate) fn copy_data_ops(
    source: &BTreeMap<DataId, Arc<Data>>,
    refs: &[DataId],
    deck: &Deck,
    reserved: &BTreeSet<String>,
    ids: &mut IdGen<'_>,
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
        let mut table = NewTable::from(&data.table);
        table.generation = Some(ids(&|_| false));
        ops.push(Op::AddData { id: Some(id.clone()), name, label: data.label.clone(), table });
        map.insert(old.clone(), id);
    }
    Ok((ops, map))
}

#[cfg(test)]
#[path = "data_copy_tests.rs"]
mod data_copy_tests;
