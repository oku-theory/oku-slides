//! Prepares a batch of ops before it is applied: bounds and ids.
//!
//! The model may leave out data, row, column and generation ids; `prepare_ops`
//! draws them, against the deck and the ids already given or drawn in the
//! batch, applying each op to a scratch deck so later ops see earlier ones.
//! An op the scratch deck refuses is left as is: the real apply reports it. Inline view data
//! become sealed `PutViewData` ops (see `prepare_view`).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use oku_slides_chart::ChartLabels;
use oku_slides_data::limits::{BIND_DATA_WEIGHT, MAX_BATCH_CELLS, REMOVE_COLUMN_WEIGHT};
use oku_slides_data::{DataId, GivenIds, IdGen, NewTable, TableOp, new_id};

use crate::error::{DataFault, OpError};
use crate::model::Deck;
use crate::ops::Op;
use crate::prepare_view::Lifter;
use crate::validate::Issue;
use crate::widget::WidgetRegistry;

/// Where a batch comes from: external input is bounded, the host's own is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// The agent, the UI or an MCP client: checked by [`check_external`].
    External,
    /// Import or paste, built by the host itself.
    Host,
}

/// What `prepare_ops` reads: the deck, the widgets, chart labels and an id source.
pub struct PrepareCtx<'a, 'g> {
    /// The deck the batch applies to.
    pub deck: &'a Deck,
    /// The widgets the ops are checked against.
    pub registry: &'a WidgetRegistry,
    /// Chart labels, for the headers of inline chart data.
    pub labels: &'a ChartLabels,
    /// The id source: `new_id` in the app, a counter in tests.
    pub ids: &'a mut IdGen<'g>,
    /// Where the batch comes from.
    pub origin: Origin,
}

impl std::fmt::Debug for PrepareCtx<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PrepareCtx").field("origin", &self.origin).finish_non_exhaustive()
    }
}

/// A batch ready to apply, and what preparing it noticed.
#[derive(Debug, Clone, PartialEq)]
pub struct Prepared {
    /// The ops, every id filled in.
    pub ops: Vec<Op>,
    /// Warnings for the caller.
    pub warnings: Vec<Issue>,
}

/// The production id source, boxed for crates that do not depend on `oku-slides-data`.
#[must_use]
pub fn fresh_ids() -> Box<IdGen<'static>> {
    Box::new(new_id)
}

/// Refuses an external batch over [`MAX_BATCH_CELLS`] or giving host-only fields.
///
/// Each op counts its cells, and at least one, so the cap also bounds the op count; a
/// `remove_column` counts [`REMOVE_COLUMN_WEIGHT`], since it scans the whole overlay, a
/// `bind_data` counts [`BIND_DATA_WEIGHT`], since it may walk the whole deck, and a query
/// ([`query_weight`]) one per started KiB of its SQL, which every check parses.
///
/// # Errors
///
/// `op-data-internal` for a table with `base` or `overlay` or a `put_view_data`,
/// `op-data-batch-too-large` past the cap.
pub fn check_external(ops: &[Op]) -> Result<(), OpError> {
    let mut count = 0usize;
    for op in ops {
        let cells = match op {
            Op::AddData { table, sql, .. } => {
                let cells = match table {
                    Some(table) => {
                        internal_fields(table)?;
                        table.cell_count()
                    }
                    None => 0,
                };
                cells.saturating_add(sql.as_deref().map_or(0, query_weight))
            }
            Op::SetQuery { sql, .. } => query_weight(sql),
            Op::EditData { change, .. } => match change {
                TableOp::Replace { table } => {
                    internal_fields(table)?;
                    change.cell_count()
                }
                TableOp::RemoveColumn { .. } => REMOVE_COLUMN_WEIGHT,
                _ => change.cell_count(),
            },
            Op::BindData { .. } => BIND_DATA_WEIGHT,
            Op::PutViewData { .. } => return Err(DataFault::Internal.into()),
            _ => 0,
        };
        count = count.saturating_add(cells.max(1));
    }
    if count > MAX_BATCH_CELLS {
        return Err(DataFault::BatchTooLarge { count, max: MAX_BATCH_CELLS }.into());
    }
    Ok(())
}

/// The weight of a query op: 1, plus one per KiB of its SQL.
fn query_weight(sql: &str) -> usize {
    1 + sql.len() / 1024
}

fn internal_fields(table: &NewTable) -> Result<(), OpError> {
    if table.base.is_some() || !table.overlay.is_empty() {
        return Err(DataFault::Internal.into());
    }
    Ok(())
}

/// Checks an external batch, then fills in every id it leaves out.
///
/// # Errors
///
/// Those of [`check_external`] for an external batch; an invalid drawn id.
pub fn prepare_ops(ops: Vec<Op>, ctx: &mut PrepareCtx<'_, '_>) -> Result<Prepared, OpError> {
    if ctx.origin == Origin::External {
        check_external(&ops)?;
    }
    let given = Given::of(&ops);
    let mut drawn = BTreeSet::new();
    let mut scratch = ctx.deck.clone();
    let mut live = true;
    let mut out = Vec::with_capacity(ops.len());
    let mut names: BTreeSet<String> = scratch.data.values().map(|d| d.name.to_lowercase()).collect();
    let mut suffixes = HashMap::new();
    let mut warnings = Vec::new();
    for mut op in ops {
        if live {
            fill_ids(&mut op, &scratch, &given, &mut drawn, ctx.ids)?;
            let puts = Lifter {
                registry: ctx.registry,
                labels: ctx.labels,
                given: &given.data,
                drawn: &drawn,
                given_tables: &given.tables,
                names: &mut names,
                suffixes: &mut suffixes,
                warnings: &mut warnings,
            }
            .lift(&mut op, &scratch, ctx.ids)?;
            for put in puts {
                if let Op::PutViewData { id, .. } = &put {
                    drawn.insert(id.as_str().to_owned());
                }
                live = live && put.clone().apply(&mut scratch, ctx.registry).is_ok();
                out.push(put);
            }
            let renames = matches!(op, Op::RemoveData { .. } | Op::RenameData { .. });
            if let Op::AddData { name, .. } = &op {
                names.insert(name.to_lowercase());
            }
            live = live && op.clone().apply(&mut scratch, ctx.registry).is_ok();
            // A removed or renamed data frees its name: the names are those of the deck again.
            if live && renames {
                names = scratch.data.values().map(|d| d.name.to_lowercase()).collect();
            }
        }
        out.push(op);
    }
    Ok(Prepared { ops: out, warnings })
}

/// Fills the ids `op` leaves out, as `scratch` stands before it.
fn fill_ids(
    op: &mut Op,
    scratch: &Deck,
    given: &Given,
    drawn: &mut BTreeSet<String>,
    ids: &mut IdGen<'_>,
) -> Result<(), OpError> {
    match op {
        Op::AddData { id, table, .. } => {
            if id.is_none() {
                let taken = |c: &str| given.data.contains(c) || drawn.contains(c) || scratch.data.contains_key(c);
                let fresh = ids(&taken);
                drawn.insert(fresh.clone());
                *id = Some(DataId::try_from(fresh)?);
            }
            let in_batch = id.as_ref().and_then(|id| given.tables.get(id));
            if let Some(table) = table {
                table.fill_ids(None, in_batch.unwrap_or(&GivenIds::default()), ids);
            }
        }
        Op::EditData { data, change } => {
            if let Some(t) = scratch.data.get(data).and_then(|d| d.table()) {
                change.fill_ids(t, given.tables.get(data).unwrap_or(&GivenIds::default()), ids);
            }
        }
        _ => {}
    }
    Ok(())
}

/// The ids a batch gives: data ids, and row and column ids per data.
#[derive(Default)]
struct Given {
    data: BTreeSet<String>,
    tables: BTreeMap<DataId, GivenIds>,
}

impl Given {
    fn of(ops: &[Op]) -> Self {
        let mut given = Given::default();
        for op in ops {
            match op {
                Op::AddData { id: Some(id), table, .. } => {
                    given.data.insert(id.as_str().to_owned());
                    if let Some(table) = table {
                        given.tables.entry(id.clone()).or_default().record_table(table);
                    }
                }
                Op::EditData { data, change } => given.tables.entry(data.clone()).or_default().record(change),
                _ => {}
            }
        }
        given
    }
}

#[cfg(test)]
#[path = "prepare_tests.rs"]
mod prepare_tests;
