//! The session's link for data work: reads, compactions and type checks of based data, in a
//! worker of its own bounded by `data_timeout_ms` and `data_memory_mib` (t-1690).

use std::ops::Range;
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

use oku_slides_data::{BaseRef, ColumnType, Data, DataError, Dataset};
use oku_slides_platform::RenderLaunch;
use oku_slides_platform::bases::BaseStore;
use oku_slides_platform::config::RenderSettings;
use oku_slides_render::RenderError;

use crate::client::unexpected;
use crate::link::Link;
use crate::protocol::{Request, Response};

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
        let mut link = Link::new(launch, settings);
        link.bases = Some(store.dir().to_owned());
        Self { link: Mutex::new(link), store }
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
        let base = data.table.base.as_ref().map(|b| self.store.path(&b.id));
        let mut link = self.link.lock().unwrap_or_else(PoisonError::into_inner);
        link.ensure_connected()?;
        // The sandbox folder, else the store's: a `.tmp-` file there is the store's to collect.
        let dir = link.work_dir().unwrap_or_else(|| self.store.dir()).to_owned();
        let name = oku_slides_data::new_id(&|id| dir.join(format!(".tmp-compact-{id}.parquet")).exists());
        let out = dir.join(format!(".tmp-compact-{name}.parquet"));
        let json = to_json(data)?;
        let reply = link.call(&Request::CompactData { base, out: out.clone() }, &json).map(|(reply, _)| reply);
        drop(link);
        let written = match reply {
            Ok(Response::Compacted { base }) => base,
            refused => {
                // Whatever the worker wrote before refusing or failing is never adopted.
                let removed = discard(&out);
                let refused = match refused {
                    Ok(Response::DataRefused { error }) => Ok(Err(error)),
                    Ok(other) => Err(unexpected(&other)),
                    Err(e) => Err(e),
                };
                return match (removed, refused) {
                    // The link's failure is the one told; the file left is logged.
                    (Err(e), Err(failed)) => {
                        tracing::warn!(error = %e, "compaction output not removed");
                        Err(failed)
                    }
                    (Err(e), Ok(_)) => Err(e),
                    (Ok(()), refused) => refused,
                };
            }
        };
        let adopted = self.store.adopt(&out, max_bytes);
        let removed = std::fs::remove_file(&out);
        let id = adopted.map_err(|e| RenderError::frame(format!("compacted base: {e}")))?;
        removed.map_err(|e| RenderError::frame(format!("compaction output {}: {e}", out.display())))?;
        if id != written.id {
            return Err(RenderError::frame(format!("compacted base hashed {} not {}", id.hex(), written.id.hex())));
        }
        Ok(Ok(written))
    }

    fn base_path(&self, data: &Data) -> Option<PathBuf> {
        data.table.base.as_ref().map(|b| self.store.path(&b.id))
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

/// Removes the compaction output `out`, if the worker wrote it.
fn discard(out: &std::path::Path) -> Result<(), RenderError> {
    match std::fs::remove_file(out) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            Err(RenderError::frame(format!("compaction output {}: {e}", out.display())))
        }
        _ => Ok(()),
    }
}

fn no_base() -> DataError {
    DataError::base_invalid("the data has no base")
}

#[cfg(test)]
#[path = "data_worker_tests.rs"]
mod data_worker_tests;
