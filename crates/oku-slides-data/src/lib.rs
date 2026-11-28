//! Deck data: datasets, data tables and their edits, number rules, names and identifiers.

mod base_ref;
pub mod bounded;
mod cell_patch;
mod column_type;
mod data;
mod dataset;
#[cfg(feature = "engine")]
pub mod engine;
mod error;
mod error_wire;
mod ids;
pub mod limits;
mod name;
mod new_table;
mod number;
mod rows;
pub mod sql;
mod table;
mod table_op;
mod text;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub use base_ref::{BaseColumn, BaseId, BaseRef};
pub use cell_patch::CellPatch;
pub use column_type::ColumnType;
pub use data::{Data, DataId, DataKind, Source};
pub use dataset::{DataCell, Dataset};
pub use error::{DataError, DataErrorKind, quoted};
pub use ids::{IdGen, IdRepair, new_id, valid_id};
pub use name::{default_name, valid_name};
pub use new_table::{FromDataset, NewColumn, NewRow, NewTable, RowLoad};
pub use number::{format_number, parse_number};
pub use rows::Rows;
pub use table::{CellKey, CellRead, Column, Table};
pub use table_op::{GivenIds, TableOp};
pub use text::{clean_label, strip_controls};
