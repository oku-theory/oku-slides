//! Oku Slides charts and tables: ECharts options to SVG, tables to Typst.
//!
//! A chart's path:
//! 1. the LLM writes ECharts options, checked by [`parse_option`];
//! 2. [`canonicalize`] moves inline data into `dataset`, which `prepare_ops` lifts into deck data;
//! 3. option edits go through [`apply_option_patch`] (`set_chart_option`), which returns its inverse;
//! 4. a [`ChartRenderer`] draws the SVG put into the Typst.
//!
//! Views name deck data (`data`); `dataset` is only the render input the core resolves from it.
//! A table ([`TableOption`]) shares `dataset` with charts;
//! [`chart_to_table`] / [`table_to_chart`] switch between them.
//!
//! This crate depends on neither Typst nor Tauri.

mod bar_gap;
mod canonical;
mod convert;
mod error;
mod header;
mod ids;
mod labels;
mod model;
mod numbers;
mod option;
mod patch;
mod refs;
mod render;
mod scale;
mod schema;
mod table;
mod table_convert;
mod theme;
mod used;

pub use canonical::{canonicalize, insert_column_series, remove_column_series, sync_series_with_columns};
pub use convert::convert;
pub use error::{ChartError, ChartErrorKind, Result};
pub use labels::ChartLabels;
pub use model::{AxesShown, ChartKind, ChartModel, normalize, parse_option, require_data_in_chart};
pub use numbers::NumberFormat;
pub use option::*;
pub use patch::{apply_option_patch, reverse_merge_patch};
pub use refs::{check_ref_and_inline, check_ref_null};
pub use render::{ChartRenderer, ChartsRsRenderer, Size, register_fonts};
pub use schema::option_schema;
pub use table::{
    ColumnAlign, MAX_TABLE_CELL_CHARS, MAX_TABLE_COLUMNS, MAX_TABLE_ROWS, MAX_TABLE_WEIGHT, TableOption, parse_table,
    patch_table, require_data_in_table, valid_weights,
};
pub use table_convert::{chart_to_table, table_to_chart};
pub use theme::ChartTheme;
