//! The session's link for data work: reads, compactions and type checks of based data, in a
//! worker of its own bounded by `data_timeout_ms` and `data_memory_mib` (t-1690); queries in
//! another, bounded by `query_timeout_ms` and `query_memory_mib` (t-1774).

use std::ops::Range;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use oku_slides_data::engine::QueryOut;
use oku_slides_data::limits::MAX_BASE_BYTES;
use oku_slides_data::sql::{EngineQuery, WHITELIST_VERSION};
use oku_slides_data::{BaseId, BaseRef, ColumnType, Data, DataError, Dataset};
use oku_slides_platform::RenderLaunch;
use oku_slides_platform::bases::BaseStore;
use oku_slides_platform::config::RenderSettings;
use oku_slides_render::{RenderError, RenderErrorKind};

use crate::client::unexpected;
use crate::link::Link;
use crate::protocol::{PROTOCOL_VERSION, QueryInputRef, Request, Response};

/// The engine a query result comes from: the polars version, the worker protocol and the SQL
/// whitelist. A change to any of them may change a result, so it is part of every result's key.
#[must_use]
pub fn engine_version() -> String {
    format!("polars-0.53.0/protocol-{PROTOCOL_VERSION}/whitelist-{WHITELIST_VERSION}")
}

/// What a data's view reads from its base.
#[derive(Debug, Clone, PartialEq)]
pub struct DataRead {
    /// The dataset, as `Table::to_dataset` gives it.
    pub dataset: Dataset,
    /// Columns whose base cells did not convert to their type, read as text.
    pub misfit: Vec<String>,
}

/// A data's refusal (`Err` inside), or the link's failure (`Err` outside).
pub type DataResult<T> = Result<Result<T, DataError>, RenderError>;

/// A worker for one session's data, launched at the first request.
pub struct DataWorker {
    link: Mutex<Link>,
    store: BaseStore,
    /// The worker's time bound, told by a query refused past it.
    timeout_ms: u64,
}

impl std::fmt::Debug for DataWorker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataWorker").field("bases", &self.store.dir()).finish_non_exhaustive()
    }
}

impl DataWorker {
    /// A data worker reading bases from `store`, with the data bounds of `settings`.
    #[must_use]
    pub fn new(launch: RenderLaunch, settings: &RenderSettings, store: BaseStore) -> Self {
        let mut settings = settings.clone();
        settings.timeout_ms = settings.data_timeout_ms;
        settings.memory_mib = settings.data_memory_mib;
        Self::bounded(launch, settings, store)
    }

    /// A worker for the session's queries, with the query bounds of `settings`.
    #[must_use]
    pub fn query(launch: RenderLaunch, settings: &RenderSettings, store: BaseStore) -> Self {
        let mut settings = settings.clone();
        settings.timeout_ms = settings.query_timeout_ms;
        settings.memory_mib = settings.query_memory_mib;
        Self::bounded(launch, settings, store)
    }

    fn bounded(launch: RenderLaunch, settings: RenderSettings, store: BaseStore) -> Self {
        let timeout_ms = settings.timeout_ms;
        let mut link = Link::new(launch, settings);
        link.bases = Some(store.dir().to_owned());
        Self { link: Mutex::new(link), store, timeout_ms }
    }

    /// Reads rows `rows` of `columns` (all when `None`) of a based `data`.
    ///
    /// # Errors
    ///
    /// The link's failure; the data's refusal inside.
    pub fn read(
        &self,
        data: &Data,
        columns: Option<Vec<String>>,
        rows: Range<usize>,
        max_cell_chars: usize,
    ) -> DataResult<DataRead> {
        let Some(base) = self.base_path(data) else { return Ok(Err(no_base())) };
        let req = Request::ReadData { base, columns, rows, max_cell_chars };
        Ok(match self.call(&req, data)? {
            Response::Read { dataset, misfit } => Ok(DataRead { dataset, misfit }),
            Response::DataRefused { error } => Err(error),
            other => return Err(unexpected(&other)),
        })
    }

    /// Checks that the base cells of `column` convert to `ty`.
    ///
    /// # Errors
    ///
    /// The link's failure; the data's refusal inside (`ColumnType` with the first rows).
    pub fn check_column_type(&self, data: &Data, column: &str, ty: ColumnType) -> DataResult<()> {
        let Some(base) = self.base_path(data) else { return Ok(Err(no_base())) };
        let req = Request::CheckColumnType { base, column: column.to_owned(), ty };
        Ok(match self.call(&req, data)? {
            Response::Ok => Ok(()),
            Response::DataRefused { error } => Err(error),
            other => return Err(unexpected(&other)),
        })
    }

    /// Writes `data`'s table (base and overlay) as a new base, adopted by the store (and leased).
    ///
    /// # Errors
    ///
    /// The link's failure, or a store failure (as `Frame`); the data's refusal inside.
    pub fn compact(&self, data: &Data, max_bytes: u64) -> DataResult<BaseRef> {
        let base = self.base_path(data);
        let json = to_json(data)?;
        let accept = |reply| match reply {
            Response::Compacted { base } => Ok((base.id.clone(), base)),
            other => Err(other),
        };
        self.write("compact", &json, |out| Request::CompactData { base, out }, accept, max_bytes)
    }

    /// Runs `query` over `inputs` (the name it reads each by, and the data) and writes its result
    /// as a new base, adopted by the store (and leased). Past the worker's timeout, the query
    /// is refused (`QueryTimeout`) and the worker restarted.
    ///
    /// # Errors
    ///
    /// The link's failure, or a store failure (as `Frame`); the query's refusal inside.
    pub fn run_query(&self, query: &EngineQuery, inputs: &[(String, Arc<Data>)]) -> DataResult<QueryOut> {
        let refs = inputs
            .iter()
            .map(|(name, data)| QueryInputRef { name: name.clone(), base: self.base_path(data) })
            .collect();
        let data: Vec<&Data> = inputs.iter().map(|(_, data)| data.as_ref()).collect();
        let json = serde_json::to_vec(&data).map_err(|e| RenderError::frame(format!("query inputs: {e}")))?;
        let request = |out| Request::RunQuery { query: query.clone(), inputs: refs, out };
        let accept = |reply| match reply {
            Response::QueryDone { out } => Ok((out.base.id.clone(), out)),
            other => Err(other),
        };
        match self.write("query", &json, request, accept, MAX_BASE_BYTES) {
            Ok(done) => Ok(done),
            Err(e) if e.kind() == RenderErrorKind::Timeout => Ok(Err(DataError::query_timeout(self.timeout_ms))),
            Err(e) => Err(e),
        }
    }

    /// Sends `request`, which writes a base at a fresh `out` path, and adopts the base of the
    /// reply `accept` takes (its id and value); whatever the worker wrote before refusing or
    /// failing is removed.
    fn write<T>(
        &self,
        what: &str,
        json: &[u8],
        request: impl FnOnce(PathBuf) -> Request,
        accept: impl FnOnce(Response) -> Result<(BaseId, T), Response>,
        max_bytes: u64,
    ) -> DataResult<T> {
        let mut link = self.link.lock().unwrap_or_else(PoisonError::into_inner);
        link.ensure_connected()?;
        // The sandbox folder, else the store's: a `.tmp-` file there is the store's to collect.
        let dir = link.work_dir().unwrap_or_else(|| self.store.dir()).to_owned();
        let name = oku_slides_data::new_id(&|id| dir.join(format!(".tmp-{what}-{id}.parquet")).exists());
        let out = dir.join(format!(".tmp-{what}-{name}.parquet"));
        let reply = link.call(&request(out.clone()), json).map(|(reply, _)| reply);
        drop(link);
        let (expected, written) = match reply.map(accept) {
            Ok(Ok(written)) => written,
            Ok(Err(Response::DataRefused { error })) => return discarded(what, &out, Ok(Err(error))),
            Ok(Err(other)) => return discarded(what, &out, Err(unexpected(&other))),
            Err(e) => return discarded(what, &out, Err(e)),
        };
        let adopted = self.store.adopt(&out, max_bytes);
        let removed = std::fs::remove_file(&out);
        let id = adopted.map_err(|e| RenderError::frame(format!("{what} base: {e}")))?;
        removed.map_err(|e| RenderError::frame(format!("{what} output {}: {e}", out.display())))?;
        if id != expected {
            return Err(RenderError::frame(format!("{what} base hashed {} not {}", id.hex(), expected.hex())));
        }
        Ok(Ok(written))
    }

    fn base_path(&self, data: &Data) -> Option<PathBuf> {
        data.table().and_then(|t| t.base.as_ref()).map(|b| self.store.path(&b.id))
    }

    fn call(&self, req: &Request, data: &Data) -> Result<Response, RenderError> {
        let json = to_json(data)?;
        let mut link = self.link.lock().unwrap_or_else(PoisonError::into_inner);
        Ok(link.call(req, &json)?.0)
    }
}

fn to_json(data: &Data) -> Result<Vec<u8>, RenderError> {
    serde_json::to_vec(data).map_err(|e| RenderError::frame(format!("data: {e}")))
}

/// `refused`, once the output `out` of a refused or failed write request is removed.
fn discarded<T>(what: &str, out: &std::path::Path, refused: DataResult<T>) -> DataResult<T> {
    let removed = match std::fs::remove_file(out) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            Err(RenderError::frame(format!("{what} output {}: {e}", out.display())))
        }
        _ => Ok(()),
    };
    match (removed, refused) {
        // The link's failure is the one told; the file left is logged.
        (Err(e), Err(failed)) => {
            tracing::warn!(error = %e, "{what} output not removed");
            Err(failed)
        }
        (Err(e), Ok(_)) => Err(e),
        (Ok(()), refused) => refused,
    }
}

fn no_base() -> DataError {
    DataError::base_invalid("the data has no base")
}

#[cfg(test)]
#[path = "data_worker_tests.rs"]
mod data_worker_tests;
