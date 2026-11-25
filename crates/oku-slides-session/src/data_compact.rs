//! Host-side compaction: a data's overlay folds into a new Parquet base, in a new layer.
//!
//! A job snapshots the data under the session lock ([`crate::Session::compaction_job`]), calls
//! the data worker with no lock held ([`Job::run`]), then commits under the lock again
//! ([`crate::Session::finish_compaction`]); the commit is dropped if the data changed meanwhile.
//! The commit is silent: no undo step, no dirty mark.
//!
//! An edit an overlay cap refuses asks a fold instead ([`Session::apply_folding`]): [`folding`]
//! runs it the same way, between two calls, then retries the edit once.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use oku_slides_core::replica::Dropped;
use oku_slides_core::{Deck, Op, OpErrorKind, TransactionError};
use oku_slides_data::limits::{COMPACT_AT, MAX_BASE_BYTES, MAX_DECK_OVERLAY, MAX_OVERLAY};
use oku_slides_data::{BaseRef, CellRead, Data, DataCell, DataId, Table, TableOp, new_id};
use oku_slides_platform::bases::{BaseStore, BaseStoreErrorKind};
use oku_slides_render_worker::DataWorker;

use crate::Session;
use crate::data_read::BaseWork;

/// The first wait after a failed compaction; it doubles at each new failure.
pub(crate) const FIRST_BACKOFF: Duration = Duration::from_mins(1);
/// The longest wait between two attempts on an unchanged data.
pub(crate) const MAX_BACKOFF: Duration = Duration::from_mins(30);

/// Whether a data is due for compaction, from its overlay and the session's state.
#[must_use]
pub(crate) fn is_due(overlay: usize, undo_touches: bool, in_flight: bool, backing_off: bool) -> bool {
    overlay >= COMPACT_AT && !undo_touches && !in_flight && !backing_off
}

/// What a job's result depends on: a compaction of another key is stale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JobKey {
    generation: String,
    epoch: String,
    touch_seq: u64,
}

/// A failed key, and when it may be tried again.
#[derive(Debug)]
struct Failure {
    key: JobKey,
    retry_at: Instant,
    delay: Duration,
}

/// The session's compaction state.
#[derive(Debug, Default)]
pub(crate) struct Compaction {
    in_flight: BTreeSet<DataId>,
    failed: BTreeMap<DataId, Failure>,
    /// Per data, the table last seen and a counter bumped each time it differs.
    touches: BTreeMap<DataId, (Table, u64)>,
    /// The data an overflowing apply asks to fold, once the session's lock is released.
    fold_wanted: Option<DataId>,
}

/// A compaction taken from the session, to run with no lock held.
#[derive(Debug)]
pub(crate) struct Job {
    data: DataId,
    key: JobKey,
    snapshot: Arc<Data>,
    worker: Arc<DataWorker>,
    store: BaseStore,
}

/// How a job ended.
#[derive(Debug)]
pub(crate) enum Reply {
    /// The worker wrote this base, adopted (and leased) in the store.
    Written(BaseRef),
    /// The base store is over `max_store_mib`: nothing was written.
    StoreFull,
    /// The worker failed.
    Failed,
}

impl Job {
    /// Drops a job whose session went away: a written base loses its adoption lease.
    pub(crate) fn abandon(self, reply: Reply) {
        if let Reply::Written(base) = reply
            && let Err(e) = self.store.release([&base.id])
        {
            tracing::warn!(error = %e, "abandoned base lease not released");
        }
    }

    /// Writes the new base. Blocking; call it with no session lock held.
    pub(crate) fn run(&self) -> Reply {
        let extra = self.snapshot.table.base.as_ref().map_or(0, |b| b.bytes);
        if let Err(e) = self.store.check_room(extra) {
            if e.kind() == BaseStoreErrorKind::Full {
                return Reply::StoreFull;
            }
            tracing::warn!(error = %e, "base store unreadable before compaction");
            return Reply::Failed;
        }
        match self.worker.compact(&self.snapshot, MAX_BASE_BYTES) {
            Ok(Ok(base)) => Reply::Written(base),
            Ok(Err(e)) => {
                tracing::warn!(error = %e, data = ?self.data, "compaction refused");
                Reply::Failed
            }
            Err(e) => {
                tracing::warn!(error = %e, data = ?self.data, "compaction worker failed");
                Reply::Failed
            }
        }
    }
}

/// How a session's overflowing apply ended (`Session::try_apply`).
#[derive(Debug)]
pub(crate) enum Applied {
    /// The ops are applied.
    Done,
    /// An overlay cap refused them; folding `data` makes room. `error` is the refusal.
    NeedsFold { data: DataId, error: crate::SessionError },
}

/// Which overlay cap refused a batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// One data's (`data-too-large { overlay }`).
    Data,
    /// The deck's (`op-data-deck-full { overlay }`).
    Deck,
}

/// The overlay entries one batch may write into a data it edits.
#[derive(Debug, Default)]
struct Writes {
    /// The cells it sets, by row and column.
    set: BTreeSet<(String, String)>,
    /// The non-empty cells its inserted rows and columns hold.
    inserted: usize,
    /// Whether it replaces the whole table: a fold cannot help it.
    replaced: bool,
}

impl Writes {
    /// The entries these writes add to `table`, folded first or as it is. An upper bound:
    /// removals are not counted.
    fn on(&self, table: &Table, folded: bool) -> usize {
        let new = |(row, column): &&(String, String)| {
            folded || matches!(table.cell(row, column), CellRead::Base | CellRead::Null)
        };
        self.inserted + self.set.iter().filter(new).count()
    }
}

/// The overlay entries a batch may write, read before it is applied.
#[derive(Debug, Default)]
pub(crate) struct Growth {
    /// Per data the batch edits.
    edits: BTreeMap<DataId, Writes>,
    /// The cells of the data it adds and of the tables it replaces.
    new: usize,
}

impl Growth {
    /// What `ops` may write.
    pub(crate) fn of(ops: &[Op]) -> Growth {
        let mut growth = Growth::default();
        let filled =
            |cells: &BTreeMap<String, DataCell>| cells.values().filter(|c| !matches!(c, DataCell::Null)).count();
        for op in ops {
            match op {
                Op::AddData { table, .. } => growth.new += table.cell_count(),
                Op::EditData { data, change } => {
                    let writes = growth.edits.entry(data.clone()).or_default();
                    match change {
                        TableOp::SetCell { row, column, .. } => {
                            writes.set.insert((row.clone(), column.clone()));
                        }
                        TableOp::InsertRow { cells, .. } | TableOp::InsertColumn { cells, .. } => {
                            writes.inserted += filled(cells);
                        }
                        TableOp::Replace { table } => {
                            writes.replaced = true;
                            growth.new += table.cell_count();
                        }
                        TableOp::RenameColumn { .. } | TableOp::RemoveRow { .. } | TableOp::RemoveColumn { .. } => {}
                    }
                }
                _ => {}
            }
        }
        growth
    }

    /// The data to fold so that the batch fits after a `refusal`; `None` if no fold can.
    ///
    /// A data's cap folds a data the batch pushes past it. The deck's folds the largest data
    /// the batch edits, else the deck's largest. Either way, the batch must then fit both caps.
    pub(crate) fn fold_target(&self, deck: &Deck, refusal: Refusal) -> Option<DataId> {
        let overlay = |id: &DataId| deck.data.get(id).map_or(0, |d| d.table.overlay_len());
        let writes = |id: &DataId, folded: bool| match (self.edits.get(id), deck.data.get(id)) {
            (Some(w), Some(d)) => w.on(&d.table, folded),
            _ => 0,
        };
        let total: usize = deck.data.values().map(|d| d.table.overlay_len()).sum();
        let unfolded: usize = self.new + self.edits.keys().map(|id| writes(id, false)).sum::<usize>();
        let cures = |id: &DataId| {
            let (own, after) = (writes(id, false), writes(id, true));
            overlay(id) > 0
                && !self.edits.get(id).is_some_and(|w| w.replaced)
                && after <= MAX_OVERLAY
                && (total - overlay(id)) + (unfolded - own) + after <= MAX_DECK_OVERLAY
        };
        let largest = |mut ids: Vec<&DataId>| {
            ids.sort_by_key(|id| std::cmp::Reverse(overlay(id)));
            ids.into_iter().find(|id| cures(id)).cloned()
        };
        let edited = self.edits.keys();
        match refusal {
            Refusal::Data => largest(edited.filter(|id| overlay(id) + writes(id, false) > MAX_OVERLAY).collect()),
            Refusal::Deck => largest(edited.collect()).or_else(|| largest(deck.data.keys().collect())),
        }
    }
}

/// Runs `f` through `with`; a fold, column checks or view reads it asks run with no lock held,
/// then `f` once more.
///
/// `with` calls its callback at most once, under the session's lock. After an overlay refusal
/// [`Session::apply_folding`] can cure, the fold's worker writes the base between two calls;
/// the commit is dropped if the data changed meanwhile. A `set_column_type` on a based data
/// asks its base checked ([`Session::ensure_retypes_checked`]), a view conversion its rows read
/// ([`Session::rows_read_off_lock`]); their outcomes are recorded between the same two calls.
/// Only a committed fold or recorded outcomes run `f` again, its result replacing the refusal.
/// `None`: `with` did not call `f`.
///
/// # Errors
///
/// `with`'s, from any of its calls.
pub(crate) fn folding<R, E>(
    mut with: impl FnMut(&mut dyn FnMut(&mut Session)) -> Result<(), E>,
    mut f: impl FnMut(&mut Session) -> R,
) -> Result<Option<R>, E> {
    let mut out = None;
    let mut job = None;
    let mut work = BaseWork::default();
    with(&mut |s| {
        s.compaction.fold_wanted = None;
        s.clear_base_reads();
        out = Some(f(s));
        job = s.take_fold(Instant::now());
        work = s.take_base_work();
    })?;
    if job.is_none() && work.is_empty() {
        return Ok(out);
    }
    let mut pending = job.map(|job| {
        let reply = job.run();
        (job, reply)
    });
    let mut done = Some(work.run());
    let mut folded = false;
    let mut checked = false;
    let committed = with(&mut |s| {
        if let Some((job, reply)) = pending.take() {
            folded = s.finish_fold(job, reply, Instant::now());
        }
        if let Some(done) = done.take().filter(|d| !d.is_empty()) {
            s.record_base_reads(done);
            checked = true;
        }
    });
    // A tab closed meanwhile: the written base loses its lease.
    if let Some((job, reply)) = pending.take() {
        job.abandon(reply);
    }
    committed?;
    if folded || checked {
        with(&mut |s| {
            out = Some(f(s));
            s.clear_base_reads();
        })?;
    }
    Ok(out)
}

impl Compaction {
    /// The key of `id` whose table is `table`, bumping its touch counter if the table changed.
    fn key_of(&mut self, id: &DataId, table: &Table) -> JobKey {
        let entry = self.touches.entry(id.clone()).or_insert_with(|| (table.clone(), 0));
        if entry.0 != *table {
            *entry = (table.clone(), entry.1 + 1);
        }
        JobKey { generation: table.generation.clone(), epoch: table.epoch.clone(), touch_seq: entry.1 }
    }

    /// Whether `id`'s last failure on `key` still waits at `now`.
    fn backing_off(&self, id: &DataId, key: &JobKey, now: Instant) -> bool {
        self.failed.get(id).is_some_and(|f| f.key == *key && now < f.retry_at)
    }

    /// Records a failure of `key`: the wait doubles on the same key, up to [`MAX_BACKOFF`].
    fn fail(&mut self, id: &DataId, key: JobKey, now: Instant) {
        let delay = match self.failed.get(id) {
            Some(f) if f.key == key => (f.delay * 2).min(MAX_BACKOFF),
            _ => FIRST_BACKOFF,
        };
        self.failed.insert(id.clone(), Failure { key, retry_at: now + delay, delay });
    }
}

impl Session {
    /// The data due for compaction at `now`.
    pub(crate) fn compactions_due(&mut self, now: Instant) -> Vec<DataId> {
        let data: Vec<(DataId, Arc<Data>)> =
            self.deck().data.iter().map(|(id, d)| (id.clone(), Arc::clone(d))).collect();
        data.into_iter()
            .filter(|(id, d)| {
                let key = self.compaction.key_of(id, &d.table);
                is_due(
                    d.table.overlay_len(),
                    self.doc.sources.replica().undo_touches_data(id),
                    self.compaction.in_flight.contains(id),
                    self.compaction.backing_off(id, &key, now),
                )
            })
            .map(|(id, _)| id)
            .collect()
    }

    /// Takes a job for `id` if it is still due at `now` (or unconditionally for a `fold`).
    ///
    /// `None`: not due, or gone.
    pub(crate) fn compaction_job(&mut self, id: &DataId, now: Instant, fold: bool) -> Option<Job> {
        if !fold && !self.compactions_due(now).contains(id) {
            return None;
        }
        let snapshot = Arc::clone(self.deck().data.get(id)?);
        let worker = self.data_link.worker();
        let key = self.compaction.key_of(id, &snapshot.table);
        self.compaction.in_flight.insert(id.clone());
        let store = self.bases.store().clone();
        Some(Job { data: id.clone(), key, snapshot, worker, store })
    }

    /// Commits `job`'s `reply` at `now`; `true` if the data now reads from the new base.
    ///
    /// A job whose data changed meanwhile is dropped, its base unleased for collection. A
    /// failure backs off; a full store also shows `session-data-store-full`.
    pub(crate) fn finish_compaction(&mut self, job: Job, reply: Reply, now: Instant) -> bool {
        let id = job.data;
        self.compaction.in_flight.remove(&id);
        let current = self.deck().data.get(&id).map(|d| d.table.clone());
        let base = match reply {
            Reply::Written(base) => base,
            Reply::StoreFull => {
                let name = job.snapshot.name.clone();
                let text = self.localizer.tr_with("session-data-store-full", &[("data", name.into())]);
                self.set_notice(text);
                self.compaction.fail(&id, job.key, now);
                return false;
            }
            Reply::Failed => {
                self.compaction.fail(&id, job.key, now);
                return false;
            }
        };
        let committed = match current {
            Some(table) if self.compaction.key_of(&id, &table) == job.key => {
                let epoch = new_id(&|e| e == table.epoch);
                match self.doc.sources.compact_data(&id, &job.snapshot.table, epoch, base.clone()) {
                    Ok(done) => done,
                    Err(e) => {
                        tracing::warn!(error = %e, data = ?id, "compaction not committed");
                        self.compaction.fail(&id, job.key, now);
                        false
                    }
                }
            }
            _ => false,
        };
        if committed {
            self.compaction.failed.remove(&id);
            self.sync_bases();
            self.persist(true);
        }
        // The worker's adoption lease: the session's own lease now holds a committed base.
        if let Err(e) = job.store.release([&base.id]) {
            tracing::warn!(error = %e, "compacted base lease not released");
        }
        committed
    }

    /// The fold an overflowing apply asked, as a job to run with no lock held.
    pub(crate) fn take_fold(&mut self, now: Instant) -> Option<Job> {
        let data = self.compaction.fold_wanted.take()?;
        self.compaction_job(&data, now, true)
    }

    /// Commits a fold's `reply` like [`Session::finish_compaction`], then cuts the undo steps
    /// that write into its data; `true` if it folded. A cut shows `session-data-undo-cut`.
    pub(crate) fn finish_fold(&mut self, job: Job, reply: Reply, now: Instant) -> bool {
        let (id, name) = (job.data.clone(), job.snapshot.name.clone());
        if !self.finish_compaction(job, reply, now) {
            return false;
        }
        if self.doc.sources.drop_data_undo(&id) == Dropped::Cut {
            let text = self.localizer.tr_with("session-data-undo-cut", &[("data", name.into())]);
            self.set_notice(text);
        }
        true
    }

    /// `apply`, telling an overlay-cap refusal a fold may cure from other errors.
    ///
    /// # Errors
    ///
    /// As [`Session::apply`], except an overlay-cap refusal with a data to fold.
    pub(crate) fn try_apply(&mut self, label: &str, ops: Vec<Op>, merge: Option<&str>) -> crate::Result<Applied> {
        let growth = Growth::of(&ops);
        let Err(error) = self.apply(label, ops, merge) else { return Ok(Applied::Done) };
        let Some(op_error) = error.transaction().map(TransactionError::op_error).filter(|e| e.is_overlay_full()) else {
            return Err(error);
        };
        let refusal = if op_error.kind() == OpErrorKind::DataDeckFull { Refusal::Deck } else { Refusal::Data };
        match growth.fold_target(self.deck(), refusal) {
            Some(data) => Ok(Applied::NeedsFold { data, error }),
            None => Err(error),
        }
    }

    /// `apply`; an overlay-cap refusal a fold can cure also asks that fold.
    ///
    /// The fold runs with the session's lock released, through [`folding`], which then calls
    /// the caller again: `Request::Apply` and grid edits through [`crate::Workspace::request`],
    /// the agent's `apply_by_agent` and the MCP hub's actions through
    /// [`crate::agent::TabSession::with_folding`]. Under another caller, nothing folds.
    ///
    /// # Errors
    ///
    /// As [`Session::apply`].
    pub fn apply_folding(&mut self, label: &str, ops: Vec<Op>, merge: Option<&str>) -> crate::Result<()> {
        match self.try_apply(label, ops, merge)? {
            Applied::Done => Ok(()),
            Applied::NeedsFold { data, error } => {
                self.compaction.fold_wanted = Some(data);
                Err(error)
            }
        }
    }
}

#[cfg(test)]
#[path = "data_compact_tests.rs"]
mod data_compact_tests;
