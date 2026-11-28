//! `RunQuery`: the input data, read from the JSON array of the payload, run through the engine;
//! each base path checked as `ReadData` checks it, `out` as `CompactData` checks it.

use std::path::Path;

use oku_slides_data::engine::{self, QueryInput};
use oku_slides_data::sql::EngineQuery;
use oku_slides_data::{Data, DataError, Source};
use oku_slides_render::WireError;

use super::{Reply, Worker, setup};
use crate::base_files::{check_base_path, check_out_path, open_base};
use crate::protocol::{QueryInputRef, Response};

impl Worker {
    pub(super) fn run_query(
        &self,
        json: &[u8],
        query: &EngineQuery,
        inputs: &[QueryInputRef],
        out: &Path,
    ) -> Result<Reply, WireError> {
        let data: Vec<Data> =
            serde_json::from_slice(json).map_err(|e| WireError::Frame { message: format!("query inputs: {e}") })?;
        if data.len() != inputs.len() {
            return Err(WireError::Frame { message: format!("{} query inputs, {} data", inputs.len(), data.len()) });
        }
        let mut tables = Vec::with_capacity(data.len());
        for (input, data) in inputs.iter().zip(&data) {
            let Source::Manual(table) = &data.source else {
                return Err(WireError::Frame { message: format!("query input {}: a query data", input.name) });
            };
            let base = input.base.as_deref().map(|b| check_base_path(self.bases_dir.as_deref(), b)).transpose()?;
            tables.push((input.name.as_str(), table, base));
        }
        let out = check_out_path(&[self.bases_dir.as_deref(), self.work.as_deref()], out)?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&out)
            .map_err(|e| setup(format!("query output {}: {e}", out.display())))?;
        let ran = tables
            .into_iter()
            .map(|(name, table, base)| {
                Ok(QueryInput { name, table, base: base.as_deref().map(open_base).transpose()? })
            })
            .collect::<Result<Vec<_>, DataError>>()
            .and_then(|inputs| engine::run_query(query, inputs, &mut file))
            .and_then(|done| file.sync_all().map(|()| done).map_err(|e| DataError::base_invalid(e.to_string())));
        Ok((ran.map_or_else(|error| Response::DataRefused { error }, |out| Response::QueryDone { out }), Vec::new()))
    }
}
