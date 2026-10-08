//! Graphiques et tableaux d'Oku Slides : options ECharts → SVG, tableaux → Typst.
//!
//! Chemin d'un graphique :
//! 1. le LLM écrit des options ECharts, validées par [`parse_option`] ;
//! 2. [`canonicalize`] range les données dans `dataset` avant stockage ;
//! 3. les éditions passent par [`apply_option_patch`] (`set_chart_option`)
//!    et [`DataOp`] (`set_data`), qui renvoient leur inverse ;
//! 4. un [`ChartRenderer`] produit le SVG inséré dans le Typst.
//!
//! Un tableau ([`TableOption`]) partage `dataset` et [`DataOp`] avec les graphiques ;
//! [`chart_to_table`] / [`table_to_chart`] passent de l'un à l'autre.
//!
//! Ce crate ne dépend ni de Typst ni de Tauri.

mod canonical;
mod convert;
mod data_ops;
mod error;
mod header;
mod labels;
mod model;
mod option;
mod patch;
mod render;
mod schema;
mod table;
mod table_convert;
mod theme;

pub use canonical::{canonicalize, sync_series_with_columns};
pub use convert::convert;
pub use data_ops::{DataOp, chart_data};
pub use error::{ChartError, ChartErrorKind, Result};
pub use labels::ChartLabels;
pub use model::{ChartKind, ChartModel, normalize, parse_option};
pub use option::*;
pub use patch::{apply_option_patch, reverse_merge_patch};
pub use render::{ChartRenderer, ChartsRsRenderer, Size, register_fonts};
pub use schema::{data_op_schema, option_schema};
pub use table::{
    ColumnAlign, MAX_TABLE_CELL_CHARS, MAX_TABLE_COLUMNS, MAX_TABLE_ROWS, TableOption, parse_table, patch_table,
    table_data,
};
pub use table_convert::{chart_to_table, table_to_chart};
pub use theme::ChartTheme;
