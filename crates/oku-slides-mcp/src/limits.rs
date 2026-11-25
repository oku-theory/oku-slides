//! The caps on what the data tools give the agent: their replies stay small and bounded.

/// Rows `data_read` reads when the agent gives no `limit`.
pub(crate) const DEFAULT_READ_ROWS: usize = 20;
/// The most rows one `data_read` returns.
pub(crate) const MAX_READ_ROWS: usize = 200;
/// The most cells one `data_read` returns; whole rows past it are dropped.
pub(crate) const MAX_READ_CELLS: usize = 10_000;
/// The longest `data_read` reply, in bytes of JSON; whole rows past it are dropped.
pub(crate) const MAX_READ_BYTES: usize = 256 * 1024;
/// The longest cell text `data_read` returns, in characters.
pub(crate) const MAX_READ_CELL_CHARS: usize = 200;
/// The columns `data_list` and `data_read` show of a data, the first ones.
pub(crate) const MAX_SHOWN_COLUMNS: usize = 100;
/// The views `data_list` names for a data, the first ones; `more` counts the others.
pub(crate) const MAX_SHOWN_USERS: usize = 50;
