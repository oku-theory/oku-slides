//! Helpers of the data tests: a counting id generator, small tables and, with the engine,
//! tables over a written base. Built for this crate's tests and with `test-support`.
#![expect(
    clippy::panic,
    clippy::must_use_candidate,
    clippy::missing_panics_doc,
    reason = "test fixtures: they fail through panic!, their results are always used"
)]

use crate::{ColumnType, DataCell, GivenIds, IdGen, NewColumn, NewRow, NewTable, Table};

/// An id generator drawing `id1`, `id2`… and skipping taken ones.
pub fn counter() -> impl FnMut(&dyn Fn(&str) -> bool) -> String {
    let mut next = 0_u32;
    move |taken: &dyn Fn(&str) -> bool| loop {
        next += 1;
        let id = format!("id{next}");
        if !taken(&id) {
            return id;
        }
    }
}

/// Fills the ids of `table` with [`counter`].
pub fn fill(table: &mut NewTable) {
    let mut ids = counter();
    let ids: &mut IdGen<'_> = &mut ids;
    table.fill_ids(None, &GivenIds::default(), ids);
}

/// A new table with these column ids and rows (`r0`, `r1`…), typed from their cells.
pub fn new_table(columns: &[(&str, Option<ColumnType>)], rows: Vec<Vec<DataCell>>) -> NewTable {
    NewTable {
        generation: Some("g1".to_owned()),
        columns: columns
            .iter()
            .map(|(id, ty)| NewColumn { id: Some((*id).to_owned()), name: format!("Name {id}"), ty: *ty })
            .collect(),
        rows: rows.into_iter().enumerate().map(|(i, cells)| NewRow { id: Some(format!("r{i}")), cells }).collect(),
        base: None,
        overlay: imbl::OrdMap::new(),
    }
}

/// A stored table: columns `a` (text), `n` (number); rows `r0` ("x", 1), `r1` ("y", 2).
pub fn small() -> Table {
    let table = new_table(
        &[("a", Some(ColumnType::Text)), ("n", Some(ColumnType::Number))],
        vec![
            vec![DataCell::Text("x".into()), DataCell::Number(1.0)],
            vec![DataCell::Text("y".into()), DataCell::Number(2.0)],
        ],
    );
    let Ok((table, _)) = table.into_table() else { panic!("small table refused") };
    table
}

/// A stored table of `rows` rows and `columns` empty number columns, ids `r0`… and `c0`….
pub fn sized(rows: usize, columns: usize) -> Table {
    Table {
        generation: "g1".to_owned(),
        epoch: String::new(),
        columns: (0..columns)
            .map(|j| crate::Column { id: format!("c{j}"), name: format!("C{j}"), ty: ColumnType::Number })
            .collect(),
        rows: crate::Rows::from_unique((0..rows).map(|i| format!("r{i}"))),
        base: None,
        cells: imbl::OrdMap::new(),
    }
}

/// A base reference for tests.
pub fn base() -> crate::BaseRef {
    crate::BaseRef {
        id: crate::BaseId::try_from(format!("sha256:{}", "a".repeat(64))).unwrap_or_else(|_| panic!("base id")),
        bytes: 10,
        rows: 2,
        columns: Vec::new(),
    }
}

/// A table of `rows` rows and `columns` number columns (`c0`…), each cell `i * 1000 + j`, folded
/// into a base written in a temporary file; the overlay is empty. Keep the file alive while the
/// table is read.
#[cfg(feature = "engine")]
#[must_use]
pub fn based_table(rows: usize, columns: usize) -> (Table, tempfile::NamedTempFile) {
    let mut table = sized(rows, columns);
    for i in 0..rows {
        for j in 0..columns {
            #[expect(clippy::cast_precision_loss, reason = "test values stay far below 2^53")]
            let value = (i * 1000 + j) as f64;
            table.cells.insert(crate::table::key(&format!("r{i}"), &format!("c{j}")), DataCell::Number(value));
        }
    }
    fold(&table)
}

/// A stored table of every column type (`t`, `n`, `i`, `b`, `d`), empty cells included.
pub fn typed() -> Table {
    let t = |s: &str| DataCell::Text(s.to_owned());
    let table = new_table(
        &[
            ("t", Some(ColumnType::Text)),
            ("n", Some(ColumnType::Number)),
            ("i", Some(ColumnType::Integer)),
            ("b", Some(ColumnType::Bool)),
            ("d", Some(ColumnType::Date)),
        ],
        vec![
            vec![t("héllo"), DataCell::Number(1.5), DataCell::Number(3.0), DataCell::Bool(true), t("2024-01-31")],
            vec![DataCell::Null, DataCell::Number(-0.25), DataCell::Null, DataCell::Bool(false), t("0001-01-01")],
            vec![t(""), DataCell::Null, DataCell::Number(-42.0), DataCell::Null, DataCell::Null],
        ],
    );
    let Ok((table, _)) = table.into_table() else { panic!("typed table refused") };
    table
}

/// `table` folded into a base written in a temporary file: the based table (empty overlay) and
/// its file, to keep alive while the table is read.
#[cfg(feature = "engine")]
#[must_use]
pub fn fold(table: &Table) -> (Table, tempfile::NamedTempFile) {
    let mut file = tempfile::NamedTempFile::new().unwrap_or_else(|e| panic!("temp file: {e}"));
    let base = crate::engine::write_base(table, None, file.as_file_mut()).unwrap_or_else(|e| panic!("write: {e}"));
    let mut based = table.clone();
    based.base = Some(base);
    based.cells = imbl::OrdMap::new();
    (based, file)
}

/// Writes `cell` in the overlay of `table`, at `row` × `column`.
pub fn set_cell(table: &mut Table, row: &str, column: &str, cell: DataCell) {
    table.cells.insert(crate::table::key(row, column), cell);
}
