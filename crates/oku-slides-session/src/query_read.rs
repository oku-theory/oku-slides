//! A query data read by the agent: its ready result, or a query previewed without storing it.
//!
//! A query data reads as its ready result for the current inputs ([`Session::ready_result`]);
//! pending or failed, it is refused (`QueryReadNoResult`). A preview ([`Session::query_preview`])
//! parses a SQL text and resolves what it reads, a query input as its ready result; run with no
//! lock held ([`QueryPreview::run`]), it computes in the query worker, reads the first rows
//! through the read worker and drops the result's lease: neither the deck nor the session's
//! results change.

use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::Arc;

use oku_slides_core::OpError;
use oku_slides_data::sql::{EngineQuery, parse_query};
use oku_slides_data::{Data, DataError, DataId, Dataset, Source};
use oku_slides_platform::bases::BaseStore;
use oku_slides_render_worker::DataWorker;
use snafu::ResultExt;

use crate::derived::{QueryStatus, query_status, release, result_data, run_query};
use crate::error::{PrepareSnafu, QueryInputPendingSnafu, QueryReadNoResultSnafu, RenderSnafu};
use crate::{Result, Session};

/// A query to preview, taken from the session to run with no lock held.
#[derive(Debug)]
pub struct QueryPreview {
    query: EngineQuery,
    inputs: Vec<(String, Arc<Data>)>,
    columns: usize,
    rows: Range<usize>,
    max_chars: usize,
    /// The query worker, which computes the result.
    runner: Arc<DataWorker>,
    /// The read worker, which reads the result's rows.
    reader: Arc<DataWorker>,
    store: BaseStore,
}

impl QueryPreview {
    /// The result as a data entered by hand (its columns and row count) and its rows read.
    ///
    /// Blocking: call it with no session lock held. The result's base is not kept.
    ///
    /// # Errors
    ///
    /// The query's refusal or failure, or the read's.
    pub fn run(self) -> Result<(Arc<Data>, Dataset)> {
        let out = run_query(&self.runner, &self.query, &self.inputs).map_err(OpError::from).context(PrepareSnafu)?;
        let data =
            Arc::new(Data { name: String::new(), label: None, inline: false, source: Source::Manual(out.table()) });
        let columns = out.columns.iter().take(self.columns).map(|c| c.id.clone()).collect();
        let read = self.reader.read(&data, Some(columns), self.rows, self.max_chars);
        release(&self.store, &out.base.id);
        let read = read.context(RenderSnafu)?.map_err(OpError::from).context(PrepareSnafu)?;
        Ok((data, read.dataset))
    }
}

impl Session {
    /// The preview of `sql`: rows `rows` of its result's first `columns` columns, cells cut at
    /// `max_chars`. Run it with no session lock held.
    ///
    /// # Errors
    ///
    /// A query that does not parse, reads a name no data or several data have, or reads a query
    /// data whose result failed (`data-query-input-failed`) or is pending (`QueryInputPending`).
    pub fn query_preview(
        &self,
        sql: &str,
        columns: usize,
        rows: Range<usize>,
        max_chars: usize,
    ) -> Result<QueryPreview> {
        let parsed = parse_query(sql).map_err(OpError::from).context(PrepareSnafu)?;
        let deck = self.deck();
        let statuses = self.query_statuses();
        let mut inputs = Vec::new();
        for name in parsed.inputs() {
            let named: Vec<_> = deck.data.iter().filter(|(_, d)| d.name.eq_ignore_ascii_case(&name)).collect();
            let input = match named.as_slice() {
                [(id, data)] => input(id, data, &statuses)?,
                [] => return Err(OpError::data_unknown_input(&name)).context(PrepareSnafu)?,
                _ => return Err(OpError::data_name_ambiguous(&name)).context(PrepareSnafu)?,
            };
            inputs.push((name, input));
        }
        let query = parsed.for_engine().map_err(OpError::from).context(PrepareSnafu)?;
        let link = &self.data_link;
        let store = self.bases.store().clone();
        Ok(QueryPreview { query, inputs, columns, rows, max_chars, runner: link.query(), reader: link.worker(), store })
    }

    /// The query data `data` (the deck's data `id`) as its ready result for the current inputs.
    ///
    /// # Errors
    ///
    /// `QueryReadNoResult` while its result is pending or failed.
    pub(crate) fn ready_result(&self, id: &DataId, data: &Data) -> Result<Arc<Data>> {
        match query_status(&self.query_statuses(), id, data) {
            Some(QueryStatus::Ready(table)) => Ok(result_data(data, table)),
            _ => Err(QueryReadNoResultSnafu { data: data.name.clone() }.build().into()),
        }
    }
}

/// The input `data` (data `id`) of a preview: as it is when entered by hand, else its ready result.
fn input(id: &DataId, data: &Arc<Data>, statuses: &BTreeMap<DataId, QueryStatus<'_>>) -> Result<Arc<Data>> {
    match query_status(statuses, id, data) {
        None => Ok(Arc::clone(data)),
        Some(QueryStatus::Ready(table)) => Ok(result_data(data, table)),
        Some(QueryStatus::Failed(_)) => Err(OpError::from(DataError::query_input_failed(data.name.as_str())))
            .context(PrepareSnafu)
            .map_err(Into::into),
        Some(QueryStatus::Pending) => Err(QueryInputPendingSnafu { name: data.name.clone() }.build().into()),
    }
}

#[cfg(test)]
#[path = "query_read_tests.rs"]
mod query_read_tests;
