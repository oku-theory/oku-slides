//! Bounds on a data and on a deck's data, checked while parsing and by every growing edit.

/// Rows in a data.
pub const MAX_ROWS: usize = 65_536;
/// Columns in a data.
pub const MAX_COLUMNS: usize = 1_000;
/// UTF-8 bytes in one text cell.
pub const MAX_TEXT_BYTES: usize = 16_384;
/// Entries in a data's overlay (its cells stored outside a base).
pub const MAX_OVERLAY: usize = 200_000;
/// UTF-8 bytes of a query data's SQL.
pub const MAX_SQL_BYTES: usize = 16_384;
/// Nesting levels of a query's syntax tree (a chain `a + b + …` nests one level per term),
/// checked before any recursive walk of the tree.
pub const MAX_QUERY_DEPTH: usize = 256;
/// Nested brackets in a query's SQL, checked before parsing: past it the parser backtracks in
/// time exponential in the nesting (sqlparser 0.60 on `NOT (`, `TRY_CAST(`, `substring(`).
pub const MAX_SQL_NESTING: usize = 16;
/// Characters in a row, column, data or generation id.
pub const MAX_ID_CHARS: usize = 64;
/// Hexadecimal characters in a drawn id.
pub const NEW_ID_CHARS: usize = 8;
/// Characters in a data name.
pub const MAX_NAME_CHARS: usize = 64;
/// Characters in a data label or a column name.
pub const MAX_LABEL_CHARS: usize = 200;
/// Data in a deck.
pub const MAX_DATA: usize = 256;
/// Row ids over all the data of a deck.
pub const MAX_DECK_ROW_IDS: usize = 1_048_576;
/// Overlay entries over all the data of a deck.
pub const MAX_DECK_OVERLAY: usize = 300_000;
/// UTF-8 bytes of overlay text, names, labels and column names over all the data of a deck.
pub const MAX_DECK_TEXT_BYTES: usize = 8 * 1024 * 1024;
/// Bytes of a deck's JSON, checked before parsing it.
pub const MAX_DECK_JSON_BYTES: usize = 64 * 1024 * 1024;
/// Cells an external batch of data ops may carry, each op counting at least one.
pub const MAX_BATCH_CELLS: usize = 100_000;
/// What a `remove_column` counts toward [`MAX_BATCH_CELLS`]: it scans the whole overlay, so an
/// external batch holds at most [`MAX_COLUMNS`] of them, enough to empty one data.
pub const REMOVE_COLUMN_WEIGHT: usize = MAX_BATCH_CELLS / MAX_COLUMNS;
/// What a `bind_data` counts toward [`MAX_BATCH_CELLS`]: binding an inline data walks the whole
/// deck, so an external batch holds at most 1,000 binds.
pub const BIND_DATA_WEIGHT: usize = MAX_BATCH_CELLS / 1_000;
/// Overlay entries at which the host folds a data into a new base, when no undo step touches it.
pub const COMPACT_AT: usize = 10_000;
/// Bytes of one base file.
pub const MAX_BASE_BYTES: u64 = 512 * 1024 * 1024;
/// Uncompressed bytes a base file's footer may declare, checked before decoding.
pub const MAX_BASE_UNCOMPRESSED: u64 = 2 * 1024 * 1024 * 1024;
/// Bytes of the bases a deck's current data reference, together.
pub const MAX_DECK_BASE_BYTES: u64 = 512 * 1024 * 1024;
/// Row groups in one base file.
pub const MAX_BASE_ROW_GROUPS: usize = 1_024;
