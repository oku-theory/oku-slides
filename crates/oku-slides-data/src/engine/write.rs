//! The write of a new base: a table's cells at its current state, base and overlay folded.

use std::fs::File;
use std::io::Write;

use polars::io::parquet::write::{ParquetCompression, ParquetWriter};
use polars::prelude::{Column, DataFrame, PlSmallStr};
use sha2::{Digest, Sha256};

use super::cells::write_column;
use super::read::BaseReader;
use super::{ROW_ID, guarded, invalid};
use crate::error::ColumnTypeSnafu;
use crate::limits::MAX_BASE_BYTES;
use crate::table::NAMED_ROWS;
use crate::{BaseColumn, BaseId, BaseRef, CellRead, DataCell, DataError, Table};

/// Writes to `out` the base of `table` at its current state: rows in `rows` order, columns in
/// `columns` order, each cell its overlay entry, else its base cell converted to the column's
/// type, else empty. Rows and columns hidden from the table are dropped. Zstd-compressed.
///
/// `base` is the table's current base file, `None` for a table without one.
///
/// # Errors
///
/// `ColumnType` when a cell does not fit its column's type; `BaseInvalid` for a refused base,
/// a write failure, a file over [`MAX_BASE_BYTES`] or a panic of the encoder.
pub fn write_base(table: &Table, base: Option<File>, out: &mut File) -> Result<BaseRef, DataError> {
    guarded(|| {
        let ids: Vec<String> = table.columns.iter().map(|c| c.id.clone()).collect();
        let reader = match (base, &table.base) {
            (Some(file), Some(_)) => Some(BaseReader::open(table, file, &ids)?),
            (None, None) => None,
            _ => return Err(DataError::base_invalid("base file and base reference disagree")),
        };
        let row_ids: Vec<&str> = table.rows.iter().collect();
        let mut frame = vec![Column::new(PlSmallStr::from(ROW_ID), row_ids.clone())];
        let mut columns = Vec::with_capacity(table.columns.len());
        for column in &table.columns {
            let mut cells = Vec::with_capacity(row_ids.len());
            let mut refused = Vec::new();
            let mut more = 0_usize;
            for row in &row_ids {
                let cell = match (table.cell(row, &column.id), &reader) {
                    (CellRead::Overlay(cell), _) if column.ty.fits(cell) => Ok(cell.clone()),
                    (CellRead::Overlay(_), _) => Err(()),
                    (CellRead::Base, Some(reader)) => reader.cell(row, &column.id, column.ty).map_err(drop),
                    _ => Ok(DataCell::Null),
                };
                match cell {
                    Ok(cell) => cells.push(cell),
                    Err(()) if refused.len() < NAMED_ROWS => refused.push((*row).to_owned()),
                    Err(()) => more += 1,
                }
            }
            snafu::ensure!(
                refused.is_empty(),
                ColumnTypeSnafu { column: column.name.as_str(), ty: column.ty, rows: refused, more }
            );
            let text_bytes = cells.iter().map(|c| if let DataCell::Text(s) = c { s.len() as u64 } else { 0 }).sum();
            frame.push(write_column(&column.id, column.ty, &cells)?);
            columns.push(BaseColumn { id: column.id.clone(), ty: column.ty, text_bytes });
        }
        let mut frame = DataFrame::new(row_ids.len(), frame).map_err(invalid)?;
        let mut hashed = Hashed { out, hash: Sha256::new(), bytes: 0 };
        ParquetWriter::new(&mut hashed)
            .with_compression(ParquetCompression::Zstd(None))
            .finish(&mut frame)
            .map_err(invalid)?;
        hashed.flush().map_err(invalid)?;
        if hashed.bytes > MAX_BASE_BYTES {
            return Err(DataError::base_invalid(format!("{} bytes written", hashed.bytes)));
        }
        let id = BaseId::from_sha256(&hashed.hash.finalize().into());
        let rows = u32::try_from(row_ids.len()).map_err(invalid)?;
        Ok(BaseRef { id, bytes: hashed.bytes, rows, columns })
    })
}

/// A writer that hashes and counts what goes through it.
struct Hashed<'a> {
    out: &'a mut File,
    hash: Sha256,
    bytes: u64,
}

impl Write for Hashed<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.out.write(buf)?;
        self.hash.update(&buf[..n]);
        self.bytes += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.out.flush()
    }
}

#[cfg(test)]
#[path = "write_tests.rs"]
mod write_tests;
