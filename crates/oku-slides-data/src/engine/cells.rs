//! Cells of a base column: read as [`DataCell`]s, written from them.
//!
//! A value that its column type cannot hold as a cell (an integer past 2^53, a date outside
//! years 0001 to 9999) reads as its text form, which does not fit the type: the reader reports it.

use polars::prelude::{Column, DataType, PlSmallStr};
use time::Date;
use time::macros::format_description;

use super::invalid;
use crate::{ColumnType, DataCell, DataError};

/// Julian day of 1970-01-01, day 0 of `Date32`.
const UNIX_JULIAN_DAY: i32 = 2_440_588;
/// Largest whole number an `f64` holds exactly: 2^53.
const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_992;

/// The cells of `column`, a base column of type `ty`.
pub(super) fn read_cells(column: &Column, ty: ColumnType) -> Result<Vec<DataCell>, DataError> {
    let null = DataCell::Null;
    Ok(match ty {
        ColumnType::Text => column
            .str()
            .map_err(invalid)?
            .into_iter()
            .map(|v| v.map_or(null.clone(), |s| DataCell::Text(s.into())))
            .collect(),
        ColumnType::Number => {
            column.f64().map_err(invalid)?.into_iter().map(|v| v.map_or(null.clone(), DataCell::Number)).collect()
        }
        ColumnType::Integer => {
            column.i64().map_err(invalid)?.into_iter().map(|v| v.map_or(null.clone(), integer_cell)).collect()
        }
        ColumnType::Bool => {
            column.bool().map_err(invalid)?.into_iter().map(|v| v.map_or(null.clone(), DataCell::Bool)).collect()
        }
        ColumnType::Date => {
            let days = column.date().map_err(invalid)?.physical();
            days.into_iter().map(|v| v.map_or(null.clone(), date_cell)).collect()
        }
    })
}

/// A column named `id` of type `ty` holding `cells`, each fitting `ty`.
pub(super) fn write_column(id: &str, ty: ColumnType, cells: &[DataCell]) -> Result<Column, DataError> {
    let name = PlSmallStr::from(id);
    let unfit = || DataError::base_invalid(format!("a cell of column {id} does not fit {ty}"));
    Ok(match ty {
        ColumnType::Text => {
            let values: Vec<Option<&str>> =
                cells.iter().map(|c| text(c).ok_or_else(unfit)).collect::<Result<_, _>>()?;
            Column::new(name, values)
        }
        ColumnType::Number => {
            let values: Vec<Option<f64>> =
                cells.iter().map(|c| number(c).ok_or_else(unfit)).collect::<Result<_, _>>()?;
            Column::new(name, values)
        }
        ColumnType::Integer => {
            let values: Vec<Option<i64>> =
                cells.iter().map(|c| number(c).map(|n| n.map(to_i64)).ok_or_else(unfit)).collect::<Result<_, _>>()?;
            Column::new(name, values)
        }
        ColumnType::Bool => {
            let values: Vec<Option<bool>> = cells
                .iter()
                .map(|c| match c {
                    DataCell::Null => Some(None),
                    DataCell::Bool(b) => Some(Some(*b)),
                    _ => None,
                })
                .map(|v| v.ok_or_else(unfit))
                .collect::<Result<_, _>>()?;
            Column::new(name, values)
        }
        ColumnType::Date => {
            let values: Vec<Option<i32>> = cells
                .iter()
                .map(|c| match c {
                    DataCell::Null => Some(None),
                    DataCell::Text(s) => day_of(s).map(Some),
                    _ => None,
                })
                .map(|v| v.ok_or_else(unfit))
                .collect::<Result<_, _>>()?;
            Column::new(name, values).cast(&DataType::Date).map_err(invalid)?
        }
    })
}

#[expect(clippy::option_option, reason = "outer None: the cell does not fit the column; inner None: a null")]
fn text(cell: &DataCell) -> Option<Option<&str>> {
    match cell {
        DataCell::Null => Some(None),
        DataCell::Text(s) => Some(Some(s)),
        _ => None,
    }
}

#[expect(clippy::option_option, reason = "outer None: the cell does not fit the column; inner None: a null")]
fn number(cell: &DataCell) -> Option<Option<f64>> {
    match cell {
        DataCell::Null => Some(None),
        DataCell::Number(n) => Some(Some(*n)),
        _ => None,
    }
}

/// `n`, a whole number of at most 2^53 in absolute value, as an `i64`.
#[expect(clippy::cast_possible_truncation, reason = "the integer type admits only whole numbers up to 2^53")]
fn to_i64(n: f64) -> i64 {
    n as i64
}

/// An `Int64` value: a number up to 2^53, else its text form.
#[expect(clippy::cast_precision_loss, reason = "only values up to 2^53 are converted, which f64 holds exactly")]
fn integer_cell(n: i64) -> DataCell {
    if n.abs() <= MAX_SAFE_INTEGER { DataCell::Number(n as f64) } else { DataCell::Text(n.to_string()) }
}

/// A `Date32` value: `YYYY-MM-DD` for years 0001 to 9999, else the day number as text.
fn date_cell(days: i32) -> DataCell {
    let format = format_description!("[year]-[month]-[day]");
    let date = days
        .checked_add(UNIX_JULIAN_DAY)
        .and_then(|julian| Date::from_julian_day(julian).ok())
        .filter(|d| (1..=9999).contains(&d.year()))
        .and_then(|d| d.format(&format).ok());
    DataCell::Text(date.unwrap_or_else(|| days.to_string()))
}

/// The `Date32` day of `YYYY-MM-DD`.
fn day_of(text: &str) -> Option<i32> {
    let format = format_description!("[year]-[month]-[day]");
    Date::parse(text, &format).ok().map(|d| d.to_julian_day() - UNIX_JULIAN_DAY)
}
