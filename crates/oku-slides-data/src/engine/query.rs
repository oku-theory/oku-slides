//! A query run on the engine: inputs folded to frames, the schema rewrites applied
//! ([`crate::sql::typed_sql`]), the result checked against the data bounds and written as a base.
//!
//! The engine's error text can quote cell values: only a scrubbed first line of it reaches
//! `QueryRun` ([`scrub`]).

use std::collections::BTreeSet;
use std::fs::File;
use std::panic::{AssertUnwindSafe, catch_unwind};

use polars::prelude::{Column as Series, DataFrame, DataType, IdxSize, IntoLazy, LazyFrame, PlSmallStr, PolarsError};
use polars::sql::SQLContext;
use snafu::ensure;

use super::cells::{read_cells, write_column};
use super::write::{fold, write_cells};
use crate::error::{
    Bound, IdInvalidSnafu, QueryColumnTypeSnafu, QueryDuplicateColumnSnafu, QueryRunSnafu, QueryTooLargeSnafu,
};
use crate::limits::{MAX_COLUMNS, MAX_ROWS, MAX_TEXT_BYTES};
use crate::sql::{EngineQuery, Schemas, Ty, typed_sql};
use crate::{BaseRef, Column, ColumnType, DataCell, DataError, Rows, Table, valid_id};

/// Longest engine reason kept in a `QueryRun` error.
const MAX_REASON_CHARS: usize = 200;

/// One input of a query: the name the query reads it by, its table and the table's base file.
#[derive(Debug)]
pub struct QueryInput<'a> {
    /// The data's name.
    pub name: &'a str,
    /// The table at its current state.
    pub table: &'a Table,
    /// The table's base file, `None` for a table without one.
    pub base: Option<File>,
}

/// The result of a query: its base and its columns, each column's id its output name.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct QueryOut {
    /// The base written.
    pub base: BaseRef,
    /// The result columns, in output order.
    pub columns: Vec<Column>,
}

impl QueryOut {
    /// The result as a table over its base: rows `0`..`n-1`, an empty overlay, and its
    /// generation the base's hash, so one result always gives one table.
    #[must_use]
    pub fn table(&self) -> Table {
        Table {
            generation: self.base.id.hex().to_owned(),
            epoch: String::new(),
            columns: self.columns.clone(),
            rows: Rows::from_unique((0..self.base.rows).map(|i| i.to_string())),
            base: Some(self.base.clone()),
            cells: imbl::OrdMap::new(),
        }
    }
}

/// Runs `query` over `inputs` and writes its result to `out` as a base.
///
/// # Errors
///
/// The refusals of the schema rewrites; `QueryRun` for the engine's refusal or a panic, with a
/// scrubbed reason; `QueryTooLarge` past [`MAX_ROWS`] rows, [`MAX_COLUMNS`] columns or a text
/// cell of [`MAX_TEXT_BYTES`]; `QueryColumnType` for a result column data cannot hold;
/// `QueryDuplicateColumn` for two result columns of one name; the fold's errors for an input.
pub fn run_query(query: &EngineQuery, inputs: Vec<QueryInput<'_>>, out: &mut File) -> Result<QueryOut, DataError> {
    catch_unwind(AssertUnwindSafe(|| run(query, inputs, out)))
        .unwrap_or_else(|_| Err(QueryRunSnafu { reason: "engine panic" }.build().into()))
}

fn run(query: &EngineQuery, inputs: Vec<QueryInput<'_>>, out: &mut File) -> Result<QueryOut, DataError> {
    let mut context = SQLContext::new();
    for input in inputs {
        context.register(&input.name.to_lowercase(), frame(input.table, input.base)?.lazy());
    }
    let sql = typed_sql(&query.sql, &mut Engine(&mut context))?;
    let limit = IdxSize::try_from(MAX_ROWS + 1).map_err(engine)?;
    let result = context.execute(&sql).and_then(|lf| lf.limit(limit).collect()).map_err(refused)?;
    ensure!(result.height() <= MAX_ROWS, QueryTooLargeSnafu { what: Bound::Rows, max: MAX_ROWS });
    ensure!(result.width() <= MAX_COLUMNS, QueryTooLargeSnafu { what: Bound::Columns, max: MAX_COLUMNS });
    let mut seen = BTreeSet::new();
    let mut columns = Vec::with_capacity(result.width());
    for series in result.columns() {
        let name = series.name().to_string();
        // The name is the column's id too: an alias with a space or past 64 characters is refused.
        ensure!(valid_id(&name), IdInvalidSnafu { id: name });
        ensure!(seen.insert(name.clone()), QueryDuplicateColumnSnafu { column: name });
        let (ty, cells) = cells(series, &name)?;
        columns.push((name, ty, cells));
    }
    let row_ids: Vec<String> = (0..result.height()).map(|i| i.to_string()).collect();
    let written: Vec<(&str, ColumnType, Vec<DataCell>)> =
        columns.iter().map(|(name, ty, cells)| (name.as_str(), *ty, cells.clone())).collect();
    let base = write_cells(&row_ids, &written, out)?;
    let columns = columns.into_iter().map(|(name, ty, _)| Column { id: name.clone(), name, ty }).collect();
    Ok(QueryOut { base, columns })
}

/// The frame of `table` at its current state, columns named as the query reads them.
fn frame(table: &Table, base: Option<File>) -> Result<DataFrame, DataError> {
    let (rows, cells) = fold(table, base)?;
    let mut frame = Vec::with_capacity(table.columns.len());
    for (column, cells) in table.columns.iter().zip(&cells) {
        let mut series = write_column(&column.id, column.ty, cells)?;
        series.rename(PlSmallStr::from(column.name.as_str()));
        frame.push(series);
    }
    DataFrame::new(rows.len(), frame).map_err(engine)
}

/// The column type and cells of a result column.
fn cells(series: &Series, name: &str) -> Result<(ColumnType, Vec<DataCell>), DataError> {
    let unfit = || DataError::from(QueryColumnTypeSnafu { column: name }.build());
    let (ty, cast) = match ty_of(series.dtype()) {
        Ty::Text | Ty::Null => (ColumnType::Text, DataType::String),
        Ty::Float => (ColumnType::Number, DataType::Float64),
        Ty::Int => (ColumnType::Integer, DataType::Int64),
        Ty::Bool => (ColumnType::Bool, DataType::Boolean),
        Ty::Date => (ColumnType::Date, DataType::Date),
        Ty::Other => return Err(unfit()),
    };
    let series = series.strict_cast(&cast).map_err(|_| unfit())?;
    let cells = read_cells(&series, ty).map_err(|_| unfit())?;
    for cell in &cells {
        if let DataCell::Text(text) = cell {
            ensure!(text.len() <= MAX_TEXT_BYTES, QueryTooLargeSnafu { what: Bound::Text, max: MAX_TEXT_BYTES });
        }
        if !ty.fits(cell) {
            return Err(unfit());
        }
    }
    Ok((ty, cells))
}

/// How the typed rules read an engine type.
fn ty_of(dtype: &DataType) -> Ty {
    match dtype {
        DataType::String => Ty::Text,
        DataType::Float32 | DataType::Float64 => Ty::Float,
        DataType::Int8
        | DataType::Int16
        | DataType::Int32
        | DataType::Int64
        | DataType::UInt8
        | DataType::UInt16
        | DataType::UInt32
        | DataType::UInt64 => Ty::Int,
        DataType::Boolean => Ty::Bool,
        DataType::Date => Ty::Date,
        DataType::Null => Ty::Null,
        _ => Ty::Other,
    }
}

/// The engine as the typed rules' planner.
struct Engine<'c>(&'c mut SQLContext);

impl Schemas for Engine<'_> {
    fn columns(&mut self, sql: &str) -> Result<Vec<(String, Ty)>, DataError> {
        let schema = self.0.execute(sql).and_then(|mut lf: LazyFrame| lf.collect_schema()).map_err(refused)?;
        Ok(schema.iter().map(|(name, dtype)| (name.to_string(), ty_of(dtype))).collect())
    }
}

/// The engine's refusal of a query: `QueryDuplicateColumn` for two output columns of one name,
/// named by the first quoted name of the message, otherwise [`engine`].
fn refused(e: PolarsError) -> DataError {
    if let PolarsError::Duplicate(message) = &e
        && let Some(column) = message.split('\'').nth(1)
    {
        return QueryDuplicateColumnSnafu { column }.build().into();
    }
    engine(e)
}

/// An engine refusal, as `QueryRun` with a scrubbed reason.
fn engine(e: impl std::fmt::Display) -> DataError {
    QueryRunSnafu { reason: scrub(&e.to_string()) }.build().into()
}

/// The engine's message stripped of what can quote a cell: its first line only, each quoted,
/// backticked or bracketed segment replaced by `…`, at most [`MAX_REASON_CHARS`] characters.
fn scrub(message: &str) -> String {
    let line = message.lines().next().unwrap_or_default();
    let mut out = String::new();
    let mut closing: Vec<char> = Vec::new();
    for c in line.chars() {
        match (closing.last(), c) {
            (Some(&end), _) if c == end => {
                closing.pop();
                if closing.is_empty() {
                    out.push('…');
                }
            }
            (Some('\'' | '"' | '`'), _) => {}
            (_, '\'' | '"' | '`') => closing.push(c),
            (_, '[') => closing.push(']'),
            (_, '(') if !closing.is_empty() => closing.push(')'),
            (Some(_), _) => {}
            (None, _) => out.push(c),
        }
    }
    out.chars().take(MAX_REASON_CHARS).collect()
}

#[cfg(test)]
#[path = "query_tests.rs"]
mod query_tests;
