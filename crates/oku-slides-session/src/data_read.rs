//! Reading a data's rows, or checking a column type change, through the data worker.
//!
//! A read is taken under the session lock ([`Session::rows_read`]) and run with none held
//! ([`RowsRead::run`]): the worker call never blocks the tab. A `set_column_type` on a data
//! with a base, and a view conversion reading one, wait for the same: the call asks the check
//! ([`Session::ensure_retypes_checked`]) or the read ([`Session::rows_read_off_lock`]),
//! `data_compact::folding` runs it off the lock and calls once more.

use std::ops::Range;
use std::sync::Arc;

use oku_slides_core::{Op, OpError};
use oku_slides_data::{ColumnType, Data, DataError, DataId, Dataset};
use oku_slides_render_worker::DataWorker;
use snafu::ResultExt;

use crate::error::{BaseStaleSnafu, PrepareSnafu, RenderSnafu};
use crate::{Result, Session};

/// A read of a data's rows, taken from the session, to run with no lock held.
#[derive(Debug)]
pub struct RowsRead {
    data: Arc<Data>,
    columns: Option<Vec<String>>,
    rows: Range<usize>,
    max_chars: usize,
    /// The data worker, for a data with a base; `None` reads the cells.
    worker: Option<Arc<DataWorker>>,
}

impl RowsRead {
    /// The rows read. Blocking for a based data; call it with no session lock held.
    ///
    /// # Errors
    ///
    /// The data's refusal (unknown column), or the worker's failure.
    pub fn run(self) -> Result<Dataset> {
        let refused = |e: DataError| OpError::from(e);
        let Some(worker) = self.worker else {
            let table = &self.data.table;
            let read = table.to_dataset(self.columns.as_deref(), self.rows, self.max_chars);
            return Ok(read.map_err(refused).context(PrepareSnafu)?);
        };
        let read = worker.read(&self.data, self.columns, self.rows, self.max_chars).context(RenderSnafu)?;
        Ok(read.map_err(refused).context(PrepareSnafu)?.dataset)
    }
}

impl Session {
    /// The read of rows `rows` of the first `columns` columns of the deck's data `id`, cells
    /// cut at `max_chars`, with that data.
    ///
    /// `None` when the deck has no data `id`. Run the read with no session lock held.
    ///
    /// # Errors
    ///
    /// As [`Session::rows_read`].
    pub fn data_rows(
        &self,
        id: &str,
        columns: usize,
        rows: Range<usize>,
        max_chars: usize,
    ) -> Option<Result<(Arc<Data>, RowsRead)>> {
        let id = DataId::try_from(id.to_owned()).ok()?;
        let data = self.deck().data.get(&id)?;
        let first: Vec<String> = data.table.columns.iter().take(columns).map(|c| c.id.clone()).collect();
        Some(self.rows_read(data, Some(&first), rows, max_chars).map(|read| (Arc::clone(data), read)))
    }

    /// The read of rows `rows` of `columns` (all when `None`) of `data`, cells cut at `max_chars`.
    ///
    /// A data without base is read from its cells; one with a base the deck knows, by the
    /// data worker (the base and the cells written over it).
    ///
    /// # Errors
    ///
    /// A base the deck does not know.
    pub(crate) fn rows_read(
        &self,
        data: &Arc<Data>,
        columns: Option<&[String]>,
        rows: Range<usize>,
        max_chars: usize,
    ) -> Result<RowsRead> {
        let worker = match &data.table.base {
            None => None,
            Some(base) if !self.doc.sources.bases.contains(&base.id) => {
                let e = OpError::from(DataError::base_invalid("the deck does not know this base"));
                return Err(e).context(PrepareSnafu)?;
            }
            Some(_) => Some(self.data_link.worker()),
        };
        let columns = columns.map(<[String]>::to_vec);
        Ok(RowsRead { data: Arc::clone(data), columns, rows, max_chars, worker })
    }
}

/// The base work applies ask (column checks, view reads), and its outcomes, between two applies.
#[derive(Debug, Default)]
pub(crate) struct BaseReads {
    /// Work an apply asked, to run with no lock held.
    wanted: BaseWork,
    /// Outcomes for the next apply, each valid for the table it read.
    done: BaseDone,
}

/// Base work taken from the session, to run with no lock held ([`BaseWork::run`]).
#[derive(Debug, Default)]
pub(crate) struct BaseWork {
    checks: Vec<RetypeCheck>,
    views: Vec<ViewRead>,
}

/// The outcomes of [`BaseWork`] run.
#[derive(Debug, Default)]
pub(crate) struct BaseDone {
    checks: Vec<Retyped>,
    views: Vec<ViewDone>,
}

impl BaseWork {
    /// No work asked.
    pub(crate) fn is_empty(&self) -> bool {
        self.checks.is_empty() && self.views.is_empty()
    }

    /// Runs every check and read. Blocking: call it with no lock held.
    pub(crate) fn run(self) -> BaseDone {
        BaseDone {
            checks: self.checks.into_iter().map(RetypeCheck::run).collect(),
            views: self.views.into_iter().map(ViewRead::run).collect(),
        }
    }
}

impl BaseDone {
    /// Nothing was run.
    pub(crate) fn is_empty(&self) -> bool {
        self.checks.is_empty() && self.views.is_empty()
    }
}

/// A base check of a column type change, taken from the session.
#[derive(Debug)]
struct RetypeCheck {
    change: Retype,
    worker: Arc<DataWorker>,
}

/// What a check is about: a column of a data, as the data stood when the check was taken.
#[derive(Debug)]
struct Retype {
    data: DataId,
    column: String,
    ty: ColumnType,
    snapshot: Arc<Data>,
}

/// A check run: its change and the worker's verdict.
#[derive(Debug)]
struct Retyped {
    change: Retype,
    outcome: Result<()>,
}

impl Retype {
    fn is(&self, data: &DataId, column: &str, ty: ColumnType) -> bool {
        self.data == *data && self.column == column && self.ty == ty
    }
}

impl RetypeCheck {
    /// Asks the worker whether every base cell converts. Blocking: call it with no lock held.
    fn run(self) -> Retyped {
        let change = self.change;
        let checked = self.worker.check_column_type(&change.snapshot, &change.column, change.ty);
        let outcome = match checked.context(RenderSnafu) {
            Ok(verdict) => verdict.map_err(OpError::from).context(PrepareSnafu).map_err(Into::into),
            Err(e) => Err(e.into()),
        };
        Retyped { change, outcome }
    }
}

/// What a view read is about: the columns, rows and cut of a data.
#[derive(Debug, PartialEq)]
struct ViewSpan {
    data: DataId,
    columns: Option<Vec<String>>,
    rows: Range<usize>,
    max_chars: usize,
}

/// A read of a based data for a view conversion, taken from the session.
#[derive(Debug)]
struct ViewRead {
    span: ViewSpan,
    read: RowsRead,
}

/// A view read run: its span, the data as it was read, and the rows.
#[derive(Debug)]
struct ViewDone {
    span: ViewSpan,
    snapshot: Arc<Data>,
    outcome: Result<Dataset>,
}

impl ViewRead {
    /// Reads the rows through the worker. Blocking: call it with no lock held.
    fn run(self) -> ViewDone {
        let snapshot = Arc::clone(&self.read.data);
        ViewDone { span: self.span, snapshot, outcome: self.read.run() }
    }
}

impl Session {
    /// Refuses a batch retyping a column of a based data its base has not been checked for.
    ///
    /// A checked outcome is used once, and only for the table it checked: a refused check
    /// refuses the batch; a missing one, or one the data outlived, is asked for (run by
    /// `data_compact::folding`) and the batch refused meanwhile.
    ///
    /// # Errors
    ///
    /// The check's refusal or failure; `BaseStale` while a check is due; an unknown base.
    pub(crate) fn ensure_retypes_checked(&mut self, ops: &[Op]) -> Result<()> {
        let mut stale = None;
        for op in ops {
            let Op::SetColumnType { data, column, ty } = op else { continue };
            let Some(current) = self.doc.sources.deck().data.get(data).map(Arc::clone) else { continue };
            let Some(base) = &current.table.base else { continue };
            let found = self.base_reads.done.checks.iter().position(|c| c.change.is(data, column, *ty));
            if let Some(found) = found {
                let checked = self.base_reads.done.checks.swap_remove(found);
                if checked.change.snapshot.table == current.table {
                    checked.outcome?;
                    continue;
                }
            }
            if !self.doc.sources.bases.contains(&base.id) {
                let e = OpError::from(DataError::base_invalid("the deck does not know this base"));
                return Err(e).context(PrepareSnafu)?;
            }
            let worker = self.data_link.worker();
            stale.get_or_insert_with(|| current.name.clone());
            let change = Retype { data: data.clone(), column: column.clone(), ty: *ty, snapshot: current };
            self.base_reads.wanted.checks.push(RetypeCheck { change, worker });
        }
        match stale {
            Some(data) => BaseStaleSnafu { data }.fail()?,
            None => Ok(()),
        }
    }

    /// Rows `rows` of `columns` (all when `None`) of the deck's data `id`, cells cut at
    /// `max_chars`, never read through the worker under the lock.
    ///
    /// A data without base is read from its cells. A based one takes the outcome of a read run
    /// for its current table, used once; else the read is asked for (run by
    /// `data_compact::folding`) and the call refused meanwhile.
    ///
    /// # Errors
    ///
    /// `op-data-missing`; the read's refusal or failure; `BaseStale` while a read is due; an
    /// unknown base.
    pub(crate) fn rows_read_off_lock(
        &mut self,
        id: &DataId,
        columns: Option<Vec<String>>,
        rows: Range<usize>,
        max_chars: usize,
    ) -> Result<Dataset> {
        let data = self.doc.sources.deck().data.get(id).map(Arc::clone);
        let data = data.ok_or_else(|| crate::data_panel::data_missing(id.as_str()))?;
        if data.table.base.is_none() {
            return self.rows_read(&data, columns.as_deref(), rows, max_chars)?.run();
        }
        let span = ViewSpan { data: id.clone(), columns, rows, max_chars };
        if let Some(found) = self.base_reads.done.views.iter().position(|v| v.span == span) {
            let done = self.base_reads.done.views.swap_remove(found);
            if done.snapshot.table == data.table {
                return done.outcome;
            }
        }
        let read = self.rows_read(&data, span.columns.as_deref(), span.rows.clone(), max_chars)?;
        self.base_reads.wanted.views.push(ViewRead { span, read });
        BaseStaleSnafu { data: data.name.clone() }.fail()?
    }

    /// Takes the base work the last apply asked, forgetting older outcomes.
    pub(crate) fn take_base_work(&mut self) -> BaseWork {
        self.base_reads.done = BaseDone::default();
        std::mem::take(&mut self.base_reads.wanted)
    }

    /// Records the outcomes of base work run, for the next apply.
    pub(crate) fn record_base_reads(&mut self, done: BaseDone) {
        self.base_reads.done = done;
    }

    /// Forgets every base work asked or run.
    pub(crate) fn clear_base_reads(&mut self) {
        self.base_reads = BaseReads::default();
    }
}

#[cfg(test)]
#[path = "data_read_tests.rs"]
mod data_read_tests;
