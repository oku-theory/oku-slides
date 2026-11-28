//! Query results the session holds: their keys, their states and their recompute.
//!
//! A query data's result is keyed by what it depends on ([`ResultKey`]): the engine version,
//! its SQL and its inputs' keys in name order; a manual input is keyed by its base id and its
//! overlay, a query input by its own key. After each use of the session (every path that edits it
//! goes through the workspace's acquisition) the workspace takes the jobs of
//! the queries whose key changed ([`Session::query_jobs`]): in compute order, one per data in
//! flight, each once its query inputs are ready. A job runs with no lock held
//! ([`QueryJob::run`]) and lands under the lock ([`Session::finish_query`]), dropped if its key
//! changed meanwhile; a result landing renders the deck again. A job dropped without landing
//! clears its in-flight mark, so the next request takes it again.
//!
//! A ready result is a base the store keeps leased (the worker's adoption) until its state is
//! replaced. A view of a pending or failed query draws its failure frame; a failed one also
//! raises `issue-data-query-failed`. A ready result's views raise the view issues planned over
//! it ([`Session::query_issues`]). A query reading a failed input fails too, naming it
//! (`data-query-input-failed`), and computes again once the input's key changes. A query
//! reading a pending or broken input stays pending: the input says why.
//!
//! A job whose worker was lost, replaced or timed out ([`QueryReply::Lost`]) is not the query's
//! fault: its query stays pending and computes again, at most [`QUERY_ATTEMPTS`] times for one
//! key, then fails with the last error like a refused query (t-1794). A query timeout counts as
//! lost: the host cannot tell a hung worker from a slow query, so a query that truly exceeds the
//! bound fails after [`QUERY_ATTEMPTS`] times the timeout, holding the one query worker.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, Mutex, PoisonError};

use oku_slides_core::{Deck, Issue, ViewRef, plan_views};
use oku_slides_data::sql::{EngineQuery, parse_query};
use oku_slides_data::{BaseId, Data, DataError, DataErrorKind, DataId, Source, Table};
use oku_slides_i18n::{Localize, Message};
use oku_slides_platform::bases::BaseStore;
use oku_slides_render::{RenderErrorKind, Results, with_results};
use oku_slides_render_worker::{DataResult, DataWorker, QueryOut, engine_version};
use sha2::{Digest, Sha256};

use crate::Session;

/// Runs of one query for one key before a lost worker fails it.
pub(crate) const QUERY_ATTEMPTS: u8 = 3;

/// What a query result depends on: sha256 of the engine version, the SQL and the input keys
/// in name order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResultKey(pub [u8; 32]);

/// Where a query data's result stands.
#[derive(Debug)]
pub enum QueryState {
    /// Not computed for its current inputs yet, or waiting for one of them.
    Pending,
    /// Computed for `key`.
    Ready {
        /// The inputs it was computed for.
        key: ResultKey,
        /// The result: its base and columns.
        out: Arc<QueryOut>,
        /// The result as a table over its base, built once from `out`.
        table: Table,
    },
    /// Refused or failed for `key`.
    Failed {
        /// The inputs it failed for.
        key: ResultKey,
        /// Why.
        error: DataError,
    },
}

impl QueryState {
    /// Ready for `key` with `out`, its table built from it.
    pub(crate) fn ready(key: ResultKey, out: QueryOut) -> Self {
        let table = out.table();
        QueryState::Ready { key, out: Arc::new(out), table }
    }

    fn key(&self) -> Option<ResultKey> {
        match self {
            QueryState::Pending => None,
            QueryState::Ready { key, .. } | QueryState::Failed { key, .. } => Some(*key),
        }
    }
}

/// Where a query data's result stands for the deck's current inputs.
#[derive(Debug, Clone)]
pub enum QueryStatus<'a> {
    /// Not computed for the current inputs yet, or waiting for one of them.
    Pending,
    /// Computed for the current inputs: the result as a table over its base.
    Ready(&'a Table),
    /// Refused or failed for the current inputs, or unable to compute (cycle, missing input,
    /// invalid query): why.
    Failed(Message),
}

/// The session's query results.
#[derive(Debug)]
pub(crate) struct Derived {
    pub(crate) states: BTreeMap<DataId, QueryState>,
    /// The data a job is computing, each cleared when its job is dropped.
    in_flight: Arc<Mutex<BTreeSet<DataId>>>,
    /// The runs each data lost to its worker for a key, cleared once it lands otherwise.
    lost: BTreeMap<DataId, (ResultKey, u8)>,
    store: BaseStore,
}

impl Derived {
    /// No result yet; the ready ones will be leased in `store`.
    pub(crate) fn new(store: BaseStore) -> Self {
        Self { states: BTreeMap::new(), in_flight: Arc::default(), lost: BTreeMap::new(), store }
    }

    /// The bases of the ready results, which the render worker reads.
    pub(crate) fn bases(&self) -> impl Iterator<Item = BaseId> + '_ {
        self.states.values().filter_map(|state| match state {
            QueryState::Ready { out, .. } => Some(out.base.id.clone()),
            QueryState::Pending | QueryState::Failed { .. } => None,
        })
    }

    fn set(&mut self, id: DataId, state: QueryState) {
        if let Some(QueryState::Ready { out, .. }) = self.states.insert(id, state) {
            release(&self.store, &out.base.id);
        }
    }

    fn remove(&mut self, id: &DataId) {
        self.lost.remove(id);
        if let Some(QueryState::Ready { out, .. }) = self.states.remove(id) {
            release(&self.store, &out.base.id);
        }
    }

    /// The ready result of `id`, as a table, whatever its key.
    fn ready_table(&self, id: &DataId) -> Option<&Table> {
        match self.states.get(id)? {
            QueryState::Ready { table, .. } => Some(table),
            QueryState::Pending | QueryState::Failed { .. } => None,
        }
    }

    /// The ready result of `id` for `key`, as a table.
    fn table(&self, id: &DataId, key: ResultKey) -> Option<&Table> {
        match self.states.get(id)? {
            QueryState::Ready { key: ready, table, .. } if *ready == key => Some(table),
            _ => None,
        }
    }

    /// Marks `id` in flight until the returned guard drops; `None` when a job already computes it.
    fn take_flight(&self, id: &DataId) -> Option<InFlight> {
        let mut set = self.in_flight.lock().unwrap_or_else(PoisonError::into_inner);
        set.insert(id.clone()).then(|| InFlight { set: Arc::clone(&self.in_flight), data: id.clone() })
    }

    /// The name of the first input of `keyed` whose query failed for its current key.
    fn failed_input<'d>(&self, deck: &'d Deck, keyed: &Keyed) -> Option<&'d str> {
        keyed.inputs.iter().find_map(|(_, id)| match self.states.get(id) {
            Some(QueryState::Failed { .. }) => deck.data.get(id).map(|d| d.name.as_str()),
            _ => None,
        })
    }

    /// The data `keyed` reads, a query input as its ready result; `None` while one is not ready.
    fn inputs(&self, deck: &Deck, keyed: &Keyed) -> Option<Vec<(String, Arc<Data>)>> {
        keyed
            .inputs
            .iter()
            .map(|(name, id)| {
                let data = deck.data.get(id)?;
                if data.table().is_some() {
                    return Some((name.clone(), Arc::clone(data)));
                }
                Some((name.clone(), result_data(data, self.ready_table(id)?)))
            })
            .collect()
    }
}

impl Drop for Derived {
    fn drop(&mut self) {
        for state in self.states.values() {
            if let QueryState::Ready { out, .. } = state {
                release(&self.store, &out.base.id);
            }
        }
    }
}

/// The status of the deck's data `id` (`data`) in `statuses`: `None` for a data entered by hand,
/// else as [`status_of_query`] tells it.
#[must_use]
pub fn query_status<'a>(
    statuses: &BTreeMap<DataId, QueryStatus<'a>>,
    id: &DataId,
    data: &Data,
) -> Option<QueryStatus<'a>> {
    data.sql().map(|_| status_of_query(statuses, id, &data.name))
}

/// The status of the deck's query data `id` (named `name`) in `statuses`.
///
/// `statuses` cover every query data of the deck; a query missing from them is a bug, logged and
/// told as failed (`session-data-query-no-state`), never as pending.
pub(crate) fn status_of_query<'a>(
    statuses: &BTreeMap<DataId, QueryStatus<'a>>,
    id: &DataId,
    name: &str,
) -> QueryStatus<'a> {
    statuses.get(id).cloned().unwrap_or_else(|| {
        tracing::error!(data = %id.as_str(), "a query data without a status");
        QueryStatus::Failed(Message::new("session-data-query-no-state").arg("data", name.to_owned()))
    })
}

/// The query data `data` as its ready result `table`: a data entered by hand, read like one.
pub(crate) fn result_data(data: &Data, table: &Table) -> Arc<Data> {
    Arc::new(Data { source: Source::Manual(table.clone()), ..data.clone() })
}

/// Runs `query` over `inputs` in `worker`: its result, its refusal, or its worker lost.
/// Blocking: call it with no session lock held.
pub(crate) fn run_query(worker: &DataWorker, query: &EngineQuery, inputs: &[(String, Arc<Data>)]) -> QueryReply {
    QueryReply::of(worker.run_query(query, inputs))
}

/// Drops the lease a ready result held on its base.
pub(crate) fn release(store: &BaseStore, id: &BaseId) {
    if let Err(e) = store.release([id]) {
        tracing::warn!(error = %e, "query result lease not released");
    }
}

/// A query of the deck as it computes: its key, and the data it reads by name.
struct Keyed {
    key: ResultKey,
    inputs: Vec<(String, DataId)>,
}

/// The deck's queries that are not broken, in compute order, each keyed; `None` while an input
/// has no key (a broken query, or a name two data share).
fn keyed(deck: &Deck) -> Vec<(DataId, Option<Keyed>)> {
    let mut names: HashMap<String, Vec<&DataId>> = HashMap::new();
    for (id, data) in &deck.data {
        names.entry(data.name.to_ascii_lowercase()).or_default().push(id);
    }
    let engine = engine_version();
    let mut keys: BTreeMap<DataId, ResultKey> = BTreeMap::new();
    let mut out = Vec::new();
    for id in deck.query_order().order {
        let Some(sql) = deck.data.get(&id).and_then(|d| d.sql()) else { continue };
        // In the order, a query parses: `query_order` breaks the others.
        let Ok(query) = parse_query(sql) else { continue };
        let mut inputs = Vec::new();
        let mut pairs: Vec<(String, ResultKey)> = Vec::new();
        for name in query.inputs() {
            let Some([input]) = names.get(&name).map(Vec::as_slice) else { break };
            let key = match deck.data.get(*input).and_then(|d| d.table()) {
                Some(table) => Some(manual_key(table)),
                None => keys.get(*input).copied(),
            };
            let Some(key) = key else { break };
            pairs.push((name.clone(), key));
            inputs.push((name, (*input).clone()));
        }
        let whole = pairs.len() == query.inputs().len();
        let keyed = whole.then(|| {
            let mut fields: Vec<&[u8]> = vec![engine.as_bytes(), sql.as_bytes()];
            for (name, key) in &pairs {
                fields.push(name.as_bytes());
                fields.push(&key.0);
            }
            let key = digest(&fields);
            keys.insert(id.clone(), key);
            Keyed { key, inputs }
        });
        out.push((id, keyed));
    }
    out
}

/// A manual data's key: its base id and its overlay (columns, rows and cells) as JSON.
fn manual_key(table: &Table) -> ResultKey {
    let base = table.base.as_ref().map(|b| b.id.hex()).unwrap_or_default();
    let overlay = Table { generation: String::new(), epoch: String::new(), base: None, ..table.clone() };
    let json = serde_json::to_vec(&overlay).unwrap_or_else(|e| {
        // A table always serializes; the error text still keys it apart from any other.
        tracing::error!(error = %e, "data overlay not serialized for its key");
        e.to_string().into_bytes()
    });
    digest(&[base.as_bytes(), &json])
}

/// sha256 of `fields`, each prefixed by its length.
fn digest(fields: &[&[u8]]) -> ResultKey {
    let mut hasher = Sha256::new();
    for field in fields {
        hasher.update(u64::try_from(field.len()).unwrap_or(u64::MAX).to_le_bytes());
        hasher.update(field);
    }
    ResultKey(hasher.finalize().into())
}

/// `issue-data-query-failed`: the query of the data `name` failed for `error`.
fn failed(name: &str, error: &DataError) -> Issue {
    let message = Message::new("issue-data-query-failed").arg("data", name.to_owned()).arg("reason", error.message());
    Issue { slide: None, element: None, message }
}

/// The views of `id` and of every query reading it, directly or through other queries: those
/// a change of `id` redraws once the results land.
pub(crate) fn dependent_views(deck: &Deck, id: &DataId) -> Vec<ViewRef> {
    let mut seen = BTreeSet::from([id.clone()]);
    let mut stack = vec![id.clone()];
    let mut views = Vec::new();
    while let Some(next) = stack.pop() {
        views.extend(deck.data_users(&next));
        for reader in deck.data_readers(&next) {
            if seen.insert(reader.clone()) {
                stack.push(reader);
            }
        }
    }
    views
}

/// The in-flight mark of a query data, cleared on drop.
#[derive(Debug)]
struct InFlight {
    set: Arc<Mutex<BTreeSet<DataId>>>,
    data: DataId,
}

impl Drop for InFlight {
    fn drop(&mut self) {
        self.set.lock().unwrap_or_else(PoisonError::into_inner).remove(&self.data);
    }
}

/// A query computing, taken from the session to run with no lock held.
///
/// Dropped, landed or not, it clears its data's in-flight mark.
#[derive(Debug)]
pub(crate) struct QueryJob {
    /// The query data.
    pub(crate) data: DataId,
    _flight: InFlight,
    key: ResultKey,
    query: EngineQuery,
    inputs: Vec<(String, Arc<Data>)>,
    worker: Arc<DataWorker>,
    store: BaseStore,
}

/// How a query run ended.
#[derive(Debug)]
pub(crate) enum QueryReply {
    /// The result, its base adopted (and leased) in the store.
    Done(QueryOut),
    /// The query was refused, or its worker failed for it (out of memory, a bad frame).
    Refused(DataError),
    /// Its worker was lost, replaced or timed out: a run again may succeed.
    Lost(DataError),
}

impl QueryReply {
    /// The reply of a worker's `run`: a timeout, a lost or replaced worker is [`QueryReply::Lost`].
    pub(crate) fn of(run: DataResult<QueryOut>) -> Self {
        match run {
            Ok(Ok(out)) => QueryReply::Done(out),
            Ok(Err(error)) if error.kind() == DataErrorKind::QueryTimeout => QueryReply::Lost(error),
            Ok(Err(error)) => QueryReply::Refused(error),
            Err(e) => {
                let reason = oku_slides_platform::errors::one_line(&e);
                tracing::warn!(error = %reason, "query worker failed");
                let error = DataError::query_run(reason);
                match e.kind() {
                    RenderErrorKind::WorkerLost | RenderErrorKind::Stale | RenderErrorKind::Timeout => {
                        QueryReply::Lost(error)
                    }
                    _ => QueryReply::Refused(error),
                }
            }
        }
    }

    /// The result, or why there is none.
    pub(crate) fn into_result(self) -> Result<QueryOut, DataError> {
        match self {
            QueryReply::Done(out) => Ok(out),
            QueryReply::Refused(error) | QueryReply::Lost(error) => Err(error),
        }
    }
}

impl QueryJob {
    /// Runs the query in the data worker. Blocking; call it with no session lock held.
    pub(crate) fn run(&self) -> QueryReply {
        run_query(&self.worker, &self.query, &self.inputs)
    }

    /// Drops a job whose result is not kept: a written base loses its adoption lease.
    pub(crate) fn abandon(self, reply: QueryReply) {
        if let QueryReply::Done(out) = reply {
            release(&self.store, &out.base.id);
        }
    }
}

impl Derived {
    /// `issue-data-query-failed` for each query of `deck` whose result failed for its current
    /// inputs.
    pub(crate) fn failed_issues(&self, deck: &Deck) -> Vec<Issue> {
        keyed(deck)
            .into_iter()
            .filter_map(|(id, keyed)| match self.states.get(&id)? {
                QueryState::Failed { key, error } if Some(*key) == keyed.map(|k| k.key) => {
                    Some(failed(&deck.data.get(&id)?.name, error))
                }
                _ => None,
            })
            .collect()
    }

    /// The status of each query data of `deck` for its current inputs, broken ones failed.
    pub(crate) fn statuses(&self, deck: &Deck) -> BTreeMap<DataId, QueryStatus<'_>> {
        let named = |id: &DataId| deck.data.get(id).map_or("", |d| d.name.as_str()).to_owned();
        let mut out: BTreeMap<DataId, QueryStatus<'_>> = deck
            .query_order()
            .broken
            .into_iter()
            .map(|(id, fault)| {
                let message = fault.message(&named(&id));
                (id, QueryStatus::Failed(message))
            })
            .collect();
        for (id, keyed) in keyed(deck) {
            let current = keyed.map(|k| k.key);
            let status = match self.states.get(&id) {
                Some(QueryState::Ready { key, table, .. }) if Some(*key) == current => QueryStatus::Ready(table),
                Some(QueryState::Failed { key, error }) if Some(*key) == current => {
                    QueryStatus::Failed(error.message())
                }
                _ => QueryStatus::Pending,
            };
            out.insert(id, status);
        }
        out
    }

    /// The results `deck`'s views read: each query data's ready result for its current inputs,
    /// `None` while it is pending or failed.
    pub(crate) fn results(&self, deck: &Deck) -> Results {
        keyed(deck)
            .into_iter()
            .map(|(id, keyed)| {
                let table = keyed.and_then(|k| self.table(&id, k.key)).cloned();
                (id, table)
            })
            .collect()
    }

    /// Marks pending the queries of `deck` whose key changed, and takes a job for each whose
    /// inputs are ready and that no job computes yet, in compute order; `worker` runs them.
    pub(crate) fn jobs(&mut self, deck: &Deck, worker: &dyn Fn() -> Arc<DataWorker>) -> Vec<QueryJob> {
        let keyed = keyed(deck);
        let live: BTreeSet<&DataId> = keyed.iter().map(|(id, _)| id).collect();
        let gone: Vec<DataId> = self.states.keys().filter(|id| !live.contains(id)).cloned().collect();
        for id in &gone {
            self.remove(id);
        }
        let mut jobs = Vec::new();
        for (id, keyed) in keyed {
            let Some(keyed) = keyed else {
                self.set(id, QueryState::Pending);
                continue;
            };
            let state = self.states.get(&id);
            if state.and_then(QueryState::key) == Some(keyed.key) {
                continue;
            }
            // Inputs come first in compute order: their state is already current.
            if let Some(input) = self.failed_input(deck, &keyed) {
                let error = DataError::query_input_failed(input);
                self.set(id, QueryState::Failed { key: keyed.key, error });
                continue;
            }
            if !matches!(state, Some(QueryState::Pending)) {
                self.set(id.clone(), QueryState::Pending);
            }
            let Some(inputs) = self.inputs(deck, &keyed) else { continue };
            let Some(sql) = deck.data.get(&id).and_then(|d| d.sql()) else { continue };
            let query = match parse_query(sql).and_then(|q| q.for_engine()) {
                Ok(query) => query,
                Err(error) => {
                    self.set(id, QueryState::Failed { key: keyed.key, error });
                    continue;
                }
            };
            let Some(flight) = self.take_flight(&id) else { continue };
            let store = self.store.clone();
            jobs.push(QueryJob { data: id, _flight: flight, key: keyed.key, query, inputs, worker: worker(), store });
        }
        jobs
    }

    /// Lands `job`'s `reply`, unless its query's inputs in `deck` changed meanwhile: then the
    /// result is dropped (`None`) and the next [`Derived::jobs`] computes it again.
    pub(crate) fn land(&mut self, deck: &Deck, job: QueryJob, reply: QueryReply) -> Option<Landed> {
        let current = keyed(deck).into_iter().find(|(id, _)| *id == job.data).and_then(|(_, k)| k).map(|k| k.key);
        if current != Some(job.key) {
            job.abandon(reply);
            return None;
        }
        let reply = match reply {
            QueryReply::Lost(error) => {
                let runs = match self.lost.get(&job.data) {
                    Some((key, runs)) if *key == job.key => runs.saturating_add(1),
                    _ => 1,
                };
                if runs < QUERY_ATTEMPTS {
                    self.lost.insert(job.data.clone(), (job.key, runs));
                    // Still pending: the next `jobs` takes it again.
                    return Some(Landed::Retried(error));
                }
                QueryReply::Refused(error)
            }
            reply => reply,
        };
        self.lost.remove(&job.data);
        let (state, landed) = match reply {
            QueryReply::Done(out) => (QueryState::ready(job.key, out), Landed::Ready),
            QueryReply::Lost(error) | QueryReply::Refused(error) => {
                let issue = deck.data.get(&job.data).map(|data| failed(&data.name, &error));
                (QueryState::Failed { key: job.key, error }, Landed::Failed(issue))
            }
        };
        self.set(job.data.clone(), state);
        Some(landed)
    }
}

/// How a landed job left its query.
#[derive(Debug)]
pub(crate) enum Landed {
    /// Ready: the deck renders again.
    Ready,
    /// Failed, raising this issue (none when the data is gone).
    Failed(Option<Issue>),
    /// Its worker was lost for this reason: still pending, it computes again.
    Retried(DataError),
}

impl Session {
    /// The state of the result of query data `id`; `None` for a data that is not a query, or a
    /// query no request has looked at yet.
    #[must_use]
    pub fn query_state(&self, id: &DataId) -> Option<&QueryState> {
        self.derived.states.get(id)
    }

    /// The issues the query results raise: `issue-data-query-failed` for each query whose
    /// result failed for its current inputs, then the view issues the ready results raise
    /// (missing column, truncated, render budget) that the deck alone does not, since a query
    /// view plans as derived without its result.
    #[must_use]
    pub fn query_issues(&self) -> Vec<Issue> {
        let deck = self.doc.sources.deck();
        let mut issues = self.derived.failed_issues(deck);
        if let Cow::Owned(over) = with_results(deck, &self.query_results(deck)) {
            let stored = plan_views(deck, &self.registry).1;
            issues.extend(plan_views(&over, &self.registry).1.into_iter().filter(|i| !stored.contains(i)));
        }
        issues
    }

    /// The results `deck`'s views read: each query data's ready result for its current inputs,
    /// `None` while it is pending or failed.
    pub(crate) fn query_results(&self, deck: &Deck) -> Results {
        self.derived.results(deck)
    }

    /// The status of each query data of the deck for its current inputs; data entered by hand
    /// have none.
    #[must_use]
    pub fn query_statuses(&self) -> BTreeMap<DataId, QueryStatus<'_>> {
        self.derived.statuses(self.doc.sources.deck())
    }

    /// Computes the query results until none is left to compute, as the workspace's thread
    /// does after each request; for tests on a session outside a workspace.
    #[cfg(any(test, feature = "test-support"))]
    pub fn settle_queries(&mut self) {
        loop {
            let jobs = self.query_jobs();
            if jobs.is_empty() {
                return;
            }
            for job in jobs {
                let reply = job.run();
                self.finish_query(job, reply);
            }
        }
    }

    /// Marks pending the queries whose key changed, and takes a job for each whose inputs are
    /// ready and that no job computes yet, in compute order.
    pub(crate) fn query_jobs(&mut self) -> Vec<QueryJob> {
        let link = &self.data_link;
        self.derived.jobs(self.doc.sources.deck(), &|| link.query())
    }

    /// Lands `job`'s `reply`, unless its query's inputs changed meanwhile: then the result is
    /// dropped and the next [`Session::query_jobs`] computes it again. A result renders the deck.
    ///
    /// Returns whether the query's state changed, for the host to push the tab's views again.
    pub(crate) fn finish_query(&mut self, job: QueryJob, reply: QueryReply) -> bool {
        match self.derived.land(self.doc.sources.deck(), job, reply) {
            None => false,
            Some(Landed::Retried(error)) => {
                tracing::warn!(error = %error.localize(&self.localizer), "query worker lost: computed again");
                false
            }
            // A failed query keeps the placeholder its pending one drew.
            Some(Landed::Failed(issue)) => {
                if let Some(issue) = issue {
                    tracing::warn!(issue = %issue.localize(&self.localizer), "query failed");
                }
                true
            }
            Some(Landed::Ready) => {
                self.sync_bases();
                if let Err(e) = self.rerender() {
                    tracing::warn!(error = %e.one_line(), "render after a query result failed");
                }
                true
            }
        }
    }
}

#[path = "derived_cache.rs"]
mod cache;
pub use cache::DeckResults;

#[cfg(test)]
#[path = "derived_tests.rs"]
pub(crate) mod derived_tests;

#[cfg(test)]
#[path = "derived_lost_tests.rs"]
mod derived_lost_tests;
