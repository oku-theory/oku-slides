//! The footer check of a base, made before any page is decoded.

use std::fs::File;

use polars::io::SerReader;
use polars::io::parquet::read::ParquetReader;
use polars::prelude::ArrowDataType;

use super::{ROW_ID, guarded, invalid};
use crate::limits::{MAX_BASE_BYTES, MAX_BASE_ROW_GROUPS, MAX_BASE_UNCOMPRESSED, MAX_COLUMNS, MAX_ROWS};
use crate::{BaseRef, ColumnType, DataError};

/// What a base's footer declares.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Footer {
    /// Size of the file.
    pub(crate) bytes: u64,
    pub(crate) rows: usize,
    pub(crate) row_groups: usize,
    /// Uncompressed bytes over all row groups.
    pub(crate) uncompressed: u64,
    /// Name and type of each column, `None` for a type no column holds.
    pub(crate) fields: Vec<(String, Option<ColumnType>)>,
}

/// Checks `file`'s footer against `base` before decoding: its size and rows are the recorded
/// ones, at most [`MAX_ROWS`] rows, [`MAX_COLUMNS`] columns plus the row id, [`MAX_BASE_ROW_GROUPS`]
/// row groups and [`MAX_BASE_UNCOMPRESSED`] bytes uncompressed, and its schema is the row id
/// then the recorded columns.
///
/// # Errors
///
/// `BaseInvalid` when the footer does not parse or any check fails.
pub fn check_footer(file: &File, base: &BaseRef) -> Result<(), DataError> {
    guarded(|| check(&footer(file)?, base))
}

/// What `file`'s footer declares; only the footer is read.
fn footer(file: &File) -> Result<Footer, DataError> {
    let bytes = file.metadata().map_err(invalid)?.len();
    if bytes > MAX_BASE_BYTES {
        return Err(DataError::base_invalid(format!("{bytes} bytes")));
    }
    let mut reader = ParquetReader::new(file.try_clone().map_err(invalid)?);
    let metadata = reader.get_metadata().map_err(invalid)?.clone();
    let schema = reader.schema().map_err(invalid)?;
    let uncompressed = metadata
        .row_groups
        .iter()
        .try_fold(0_u64, |sum, g| u64::try_from(g.total_byte_size()).ok().and_then(|size| sum.checked_add(size)));
    Ok(Footer {
        bytes,
        rows: metadata.num_rows,
        row_groups: metadata.row_groups.len(),
        uncompressed: uncompressed.unwrap_or(u64::MAX),
        fields: schema.iter().map(|(name, field)| (name.to_string(), column_type(field.dtype()))).collect(),
    })
}

/// The column type a Parquet column of `dtype` holds.
fn column_type(dtype: &ArrowDataType) -> Option<ColumnType> {
    match dtype {
        ArrowDataType::Utf8 | ArrowDataType::LargeUtf8 | ArrowDataType::Utf8View => Some(ColumnType::Text),
        ArrowDataType::Float64 => Some(ColumnType::Number),
        ArrowDataType::Int64 => Some(ColumnType::Integer),
        ArrowDataType::Boolean => Some(ColumnType::Bool),
        ArrowDataType::Date32 => Some(ColumnType::Date),
        _ => None,
    }
}

/// Checks a declared footer against `base`.
pub(crate) fn check(footer: &Footer, base: &BaseRef) -> Result<(), DataError> {
    let refuse = |reason: String| Err(DataError::base_invalid(reason));
    if footer.bytes != base.bytes {
        return refuse(format!("{} bytes, {} recorded", footer.bytes, base.bytes));
    }
    if footer.rows > MAX_ROWS || footer.rows != base.rows as usize {
        return refuse(format!("{} rows, {} recorded", footer.rows, base.rows));
    }
    if footer.fields.len() > MAX_COLUMNS + 1 {
        return refuse(format!("{} columns", footer.fields.len()));
    }
    if footer.row_groups > MAX_BASE_ROW_GROUPS {
        return refuse(format!("{} row groups", footer.row_groups));
    }
    if footer.uncompressed > MAX_BASE_UNCOMPRESSED {
        return refuse(format!("{} bytes uncompressed", footer.uncompressed));
    }
    let expected =
        std::iter::once((ROW_ID, ColumnType::Text)).chain(base.columns.iter().map(|c| (c.id.as_str(), c.ty)));
    let matches = footer.fields.len() == base.columns.len() + 1
        && footer.fields.iter().zip(expected).all(|((name, ty), (id, want))| name == id && *ty == Some(want));
    if !matches {
        return refuse("schema differs from the recorded columns".to_owned());
    }
    Ok(())
}

#[cfg(test)]
#[path = "footer_tests.rs"]
mod footer_tests;
