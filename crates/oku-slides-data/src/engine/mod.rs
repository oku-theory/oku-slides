//! The Parquet engine of data bases, built only with the `engine` feature (render worker).
//!
//! A base is untrusted bytes: each call takes a file the caller opened, checks its footer before
//! decoding ([`check_footer`]) and runs under `catch_unwind`, so a malformed file or a panic in
//! the decoder is a `BaseInvalid` error, never a crash of the process.
//!
//! A base holds a `__row_id` column then one column per column id, in Parquet's own types
//! (text `Utf8`, number `Float64`, integer `Int64`, bool `Boolean`, date `Date32`).

mod cells;
mod footer;
mod query;
mod read;
mod write;

use std::panic::{AssertUnwindSafe, catch_unwind};

pub use footer::check_footer;
pub use query::{QueryInput, QueryOut, run_query};
pub use read::{ViewRead, check_column_type, read_view};
pub use write::write_base;

use crate::DataError;

/// The row id column of a base.
pub(crate) const ROW_ID: &str = "__row_id";

/// Runs `f`, a panic becoming `BaseInvalid`.
fn guarded<T>(f: impl FnOnce() -> Result<T, DataError>) -> Result<T, DataError> {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or_else(|_| Err(DataError::base_invalid("engine panic")))
}

/// A Parquet or I/O failure, as `BaseInvalid`.
fn invalid(e: impl std::fmt::Display) -> DataError {
    DataError::base_invalid(e.to_string())
}
