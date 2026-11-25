//! Data edits a merge loses: those made to a `table` (a generation) or a `layer` (an epoch,
//! replaced by a compaction) container that is no longer current.
//!
//! Before the import, each side's table and layer containers per data key are recorded with
//! the containers under them ([`Tables`]). After it, a data's edits are lost when its table,
//! or within the same table its layer, on either side is not the winner, and changes the
//! winner's creator never saw touched it: ours, or theirs we import. Both peers of a merge
//! report the same loss.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use loro::{Container, ContainerID, ContainerTrait, Frontiers, ID, LoroDoc, ValueOrContainer, VersionVector};
use oku_slides_data::DataId;
use snafu::ResultExt;

use super::ROOT;
use super::data::{DATA, LAYER, TABLE, keyed};
use crate::error::{LoroSnafu, ReplicaInner};

/// Levels walked under a table container: table, columns, column maps, then values.
const TABLE_DEPTH: u32 = 3;

/// Levels walked under a layer container: layer, cells, then values.
const LAYER_DEPTH: u32 = 1;

/// A container and the containers under it.
type Under = (ContainerID, HashSet<ContainerID>);

/// Each data key's table container and its current layer container, with what is under each.
pub(super) type Tables = BTreeMap<String, (Under, Option<Under>)>;

/// A container's id and what is under it, down `depth` levels.
fn under(map: &loro::LoroMap, depth: u32) -> Under {
    let mut out = HashSet::new();
    walk(&Container::Map(map.clone()), depth, &mut out);
    (map.id(), out)
}

/// The current layer of `table`.
fn layer_of(table: &loro::LoroMap) -> Option<loro::LoroMap> {
    match table.get(LAYER) {
        Some(ValueOrContainer::Container(Container::Map(l))) => Some(l),
        _ => None,
    }
}

/// The current table container of each data key of `doc`.
fn current(doc: &LoroDoc) -> BTreeMap<String, loro::LoroMap> {
    let Some(ValueOrContainer::Container(Container::Map(data))) = doc.get_map(ROOT).get(DATA) else {
        return BTreeMap::new();
    };
    data.keys()
        .filter_map(|key| {
            let Some(ValueOrContainer::Container(Container::Map(m))) = data.get(&key) else { return None };
            let Some(ValueOrContainer::Container(Container::Map(t))) = m.get(TABLE) else { return None };
            Some((key.to_string(), t))
        })
        .collect()
}

/// The table containers of `doc`, with what is under each.
pub(super) fn tables(doc: &LoroDoc) -> Tables {
    current(doc)
        .into_iter()
        .map(|(key, table)| {
            let layer = layer_of(&table).map(|l| under(&l, LAYER_DEPTH));
            (key, (under(&table, TABLE_DEPTH), layer))
        })
        .collect()
}

/// Adds `container` and its child containers, down `depth` levels, to `out`.
fn walk(container: &Container, depth: u32, out: &mut HashSet<ContainerID>) {
    out.insert(container.id());
    if depth == 0 {
        return;
    }
    let children: Vec<Container> = match container {
        Container::Map(m) => m
            .keys()
            .filter_map(|k| match m.get(&k) {
                Some(ValueOrContainer::Container(c)) => Some(c),
                _ => None,
            })
            .collect(),
        Container::MovableList(l) => (0..l.len())
            .filter_map(|i| match l.get(i) {
                Some(ValueOrContainer::Container(c)) => Some(c),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    for child in &children {
        walk(child, depth - 1, out);
    }
}

/// Whether changes of `from` absent from `without` touched one of `under`.
fn touched(
    doc: &LoroDoc,
    from: &VersionVector,
    without: &VersionVector,
    under: &HashSet<ContainerID>,
) -> Result<bool, ReplicaInner> {
    for span in from.sub_iter(without) {
        // `sub_iter` yields spans with `start <= end`; one that is not is skipped, not guessed.
        let Ok(len) = usize::try_from(span.counter.end - span.counter.start) else {
            tracing::warn!(
                peer = span.peer,
                start = span.counter.start,
                end = span.counter.end,
                "inverted span skipped"
            );
            continue;
        };
        let ids = doc.try_get_changed_containers_in(ID::new(span.peer, span.counter.start), len).context(LoroSnafu)?;
        if ids.iter().any(|id| under.contains(id)) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// What is under the container of `key` in `tables` that `winner` replaced: its table when
/// that is not `winner.0`, else its layer when that is not `winner.1`. With it, the winning
/// container to compare against.
fn switched<'a>(
    tables: &'a Tables,
    key: &str,
    winner: &(ContainerID, Option<ContainerID>),
) -> Option<(&'a HashSet<ContainerID>, ContainerID)> {
    let ((table, under), layer) = tables.get(key)?;
    if *table != winner.0 {
        return Some((under, winner.0.clone()));
    }
    match (layer, &winner.1) {
        (Some((ours, under)), Some(won)) if ours != won => Some((under, won.clone())),
        _ => None,
    }
}

/// The version `container` was created at: the changes its creator had seen, and its own.
///
/// `None` when the history lacks its creation (shallow snapshots are refused on entry, so
/// never in practice).
fn creation(doc: &LoroDoc, container: &ContainerID) -> Option<VersionVector> {
    match container {
        ContainerID::Normal { peer, counter, .. } => doc.frontiers_to_vv(&Frontiers::from(ID::new(*peer, *counter))),
        ContainerID::Root { .. } => Some(VersionVector::default()),
    }
}

/// The data of `doc`, after a merge, whose edits were lost on either side.
///
/// `ours` and `theirs` are the tables before the merge, `before` and `their_vv` the
/// versions of both sides. A side's edits are lost when its table is not the winner and
/// changes the winner's creator never saw touched it: ours since `before`'s past, theirs
/// among those imported.
pub(super) fn lost(
    doc: &LoroDoc,
    ours: &Tables,
    theirs: &Tables,
    before: &VersionVector,
    their_vv: &VersionVector,
) -> Result<Vec<DataId>, ReplicaInner> {
    let mut keys = BTreeSet::new();
    for (key, table) in current(doc) {
        let winner = (table.id(), layer_of(&table).map(|l| l.id()));
        let (our, their) = (switched(ours, &key, &winner), switched(theirs, &key, &winner));
        let mut lost = false;
        // Ours: changes since `before`'s past; theirs: only the imported ones (`before` known).
        for (side, from, imported) in [(our, before, false), (their, their_vv, true)] {
            let Some((under, won)) = side else { continue };
            // Without the winner's creation, every change would look unseen: no report beats a false one.
            let Some(mut known) = creation(doc, &won) else {
                tracing::warn!(data = %key, container = %won, "container creation not in history, loss check skipped");
                continue;
            };
            if imported {
                known.merge(before);
            }
            lost = lost || touched(doc, from, &known, under)?;
        }
        if lost {
            keys.insert(key);
        }
    }
    if keys.is_empty() {
        return Ok(Vec::new());
    }
    let Some(ValueOrContainer::Container(Container::Map(data))) = doc.get_map(ROOT).get(DATA) else {
        return Ok(Vec::new());
    };
    Ok(keyed(&data).into_iter().filter(|(_, key)| keys.contains(key)).map(|(id, _)| id).collect())
}
