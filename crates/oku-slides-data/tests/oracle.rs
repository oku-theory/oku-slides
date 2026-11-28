//! The `DuckDB` oracle of query results (t-1774): every case of `oracle_cases.json` runs through
//! the whitelist, the rewrites and polars, and through `DuckDB` over the same frames; the names
//! and rows must match. Run by `just test-oracle` (feature `oracle`).
#![expect(clippy::panic, clippy::unwrap_used, reason = "an oracle test: it fails through panic!")]

use std::collections::{BTreeMap, BTreeSet};

use duckdb::Connection;
use duckdb::types::ValueRef;
use oku_slides_data::engine::{QueryInput, QueryOut, read_view, run_query};
use oku_slides_data::sql::parse_query;
use oku_slides_data::test_support::{fold, new_table};
use oku_slides_data::{ColumnType, DataCell, DataError, DataErrorKind, Rows, Table};
use serde::Deserialize;
use speculoos::prelude::*;
use tempfile::NamedTempFile;

/// The `DuckDB` frames, as in the spike: the same values build the deck's tables below.
/// IEEE operations off: `/` by zero errors, as `%` and `//` do, instead of giving an infinity.
const FRAMES: &str = "
SET ieee_floating_point_ops = false;
CREATE TABLE sales (id BIGINT, region VARCHAR, product VARCHAR, day DATE, qty BIGINT, price DOUBLE);
INSERT INTO sales VALUES
 (1,'north','apple',DATE '2026-01-05',3,1.5), (2,'north','pear', DATE '2026-01-20',5,2.0),
 (3,'south','apple',DATE '2026-02-03',2,1.5), (4,'south','plum', DATE '2026-02-14',7,3.0),
 (5,'east', 'pear', DATE '2026-03-01',1,2.0), (6,'north','apple',DATE '2026-03-15',4,1.5),
 (7,'south','pear', DATE '2026-03-28',6,2.0), (8,'east', 'kiwi', DATE '2026-04-02',8,2.5),
 (9,'west', NULL,   DATE '2026-04-10',NULL,1.0);
CREATE TABLE products (product VARCHAR, category VARCHAR);
INSERT INTO products VALUES ('apple','tree'),('pear','tree'),('plum','stone'),('fig','other');
CREATE TABLE wide (product VARCHAR, q1 BIGINT, q2 BIGINT, q3 BIGINT);
INSERT INTO wide VALUES ('apple',1,2,3),('pear',4,5,6),('plum',7,NULL,9);
CREATE TABLE returns (qty BIGINT, region VARCHAR, id BIGINT);
INSERT INTO returns VALUES (2,'north',2),(1,'east',5);
";

/// Cases that run in `DuckDB` and fail at run time in polars: each one a known engine gap.
const RUNTIME_GAPS: &[&str] = &[
    // Polars identifiers are case-sensitive: a name in another case fails, never wrong rows (spec).
    "probe_case_insensitive_table",
    "probe_case_insensitive_col",
    "r3_canon_ident",
    // Polars VALUES takes literals only, not `x::DOUBLE`; the `UNION ALL` twins run the fuzz.
    "t5_round_fuzz_values",
    "t5_round_special_values",
];

/// Cases the worker refuses by design once it knows the input types, with the refusal's kind.
const EXPECTED_REFUSALS: &[(&str, DataErrorKind)] = &[
    // Float `round(x, n > 0)`, `%`, `mod` and `//` differ from DuckDB on binary edges (spec).
    ("fn_math_043", DataErrorKind::QueryFloatOperand),
    ("probe_round_half", DataErrorKind::QueryFloatOperand),
    ("probe_round_half_col", DataErrorKind::QueryFloatOperand),
    ("probe_round_neg_num", DataErrorKind::QueryFloatOperand),
    ("r3_round_half", DataErrorKind::QueryFloatOperand),
    ("r3_round_half_lit", DataErrorKind::QueryFloatOperand),
    ("r4_nofrom_values_round", DataErrorKind::QueryFloatOperand),
    ("f1_round_edges", DataErrorKind::QueryFloatOperand),
    ("f1_round_special", DataErrorKind::QueryFloatOperand),
    ("f1_round_special_native", DataErrorKind::QueryFloatOperand),
    ("f1_round_edges_double", DataErrorKind::QueryFloatOperand),
    ("f1_round_tie_edges", DataErrorKind::QueryFloatOperand),
    ("f1_round_tie_special", DataErrorKind::QueryFloatOperand),
    ("f1_round_tie_cols", DataErrorKind::QueryFloatOperand),
    ("r3_mod_double", DataErrorKind::QueryFloatOperand),
    ("r4_mod_double", DataErrorKind::QueryFloatOperand),
    ("f1_mod_exact", DataErrorKind::QueryFloatOperand),
    ("f4_floordiv_float", DataErrorKind::QueryFloatOperand),
    // Its `VALUES` holds `1e15 + 0.7`, not a literal: the engine refuses it as it types `x`.
    ("f2_fmod_float", DataErrorKind::QueryRun),
    // A result needs one name per column: DuckDB keeps two `x`, data refuses them.
    ("probe_dup_output_names", DataErrorKind::QueryDuplicateColumn),
    // A result column's name is its id: letters, digits, - and _ only.
    ("probe_quoted_ident", DataErrorKind::IdInvalid),
    // Data holds no NaN or infinity: `sqrt(-1)` and `ln(0)` are refused, not stored.
    ("probe_sqrt_neg", DataErrorKind::QueryColumnType),
    ("probe_ln_zero", DataErrorKind::QueryColumnType),
];

#[derive(Deserialize)]
struct Case {
    name: String,
    sql: String,
    ordered: bool,
    /// `DuckDB` errors on `%`, `//` (R11) or `/` (R20, IEEE operations off) by zero where data gives NULL.
    #[serde(default)]
    duck_error_is_null: bool,
}

/// A normalised cell: numbers to ten significant digits, dates as `YYYY-MM-DD` text.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Cell {
    Null,
    Bool(bool),
    Num(String),
    Text(String),
}

type Rowset = (Vec<String>, Vec<Vec<Cell>>);

fn num(x: f64) -> Cell {
    if x.is_nan() {
        return Cell::Num("nan".to_owned());
    }
    if x.is_infinite() {
        return Cell::Num(if x > 0.0 { "inf" } else { "-inf" }.to_owned());
    }
    let ten: f64 = format!("{x:.9e}").parse().unwrap();
    Cell::Num(format!("{}", if ten == 0.0 { 0.0 } else { ten }))
}

fn cell(cell: DataCell) -> Cell {
    match cell {
        DataCell::Null => Cell::Null,
        DataCell::Bool(b) => Cell::Bool(b),
        DataCell::Number(x) => num(x),
        DataCell::Text(s) => Cell::Text(s),
    }
}

fn table(columns: &[(&str, ColumnType)], rows: Vec<Vec<DataCell>>) -> Table {
    let typed: Vec<(&str, Option<ColumnType>)> = columns.iter().map(|(id, ty)| (*id, Some(*ty))).collect();
    let (mut table, _) = new_table(&typed, rows).into_table().unwrap();
    for column in &mut table.columns {
        column.name = column.id.clone();
    }
    table
}

fn t(s: &str) -> DataCell {
    DataCell::Text(s.to_owned())
}

fn n(x: f64) -> DataCell {
    DataCell::Number(x)
}

/// The deck's frames: `sales` folded over a base, the others as overlays only.
fn frames() -> (Vec<(&'static str, Table)>, NamedTempFile) {
    let sale = |id: f64, region: &str, product: Option<&str>, day: &str, qty: Option<f64>, price: f64| {
        vec![n(id), t(region), product.map_or(DataCell::Null, t), t(day), qty.map_or(DataCell::Null, n), n(price)]
    };
    let sales = table(
        &[
            ("id", ColumnType::Integer),
            ("region", ColumnType::Text),
            ("product", ColumnType::Text),
            ("day", ColumnType::Date),
            ("qty", ColumnType::Integer),
            ("price", ColumnType::Number),
        ],
        vec![
            sale(1.0, "north", Some("apple"), "2026-01-05", Some(3.0), 1.5),
            sale(2.0, "north", Some("pear"), "2026-01-20", Some(5.0), 2.0),
            sale(3.0, "south", Some("apple"), "2026-02-03", Some(2.0), 1.5),
            sale(4.0, "south", Some("plum"), "2026-02-14", Some(7.0), 3.0),
            sale(5.0, "east", Some("pear"), "2026-03-01", Some(1.0), 2.0),
            sale(6.0, "north", Some("apple"), "2026-03-15", Some(4.0), 1.5),
            sale(7.0, "south", Some("pear"), "2026-03-28", Some(6.0), 2.0),
            sale(8.0, "east", Some("kiwi"), "2026-04-02", Some(8.0), 2.5),
            sale(9.0, "west", None, "2026-04-10", None, 1.0),
        ],
    );
    let (sales, file) = fold(&sales);
    let products = table(
        &[("product", ColumnType::Text), ("category", ColumnType::Text)],
        [("apple", "tree"), ("pear", "tree"), ("plum", "stone"), ("fig", "other")]
            .iter()
            .map(|(p, c)| vec![t(p), t(c)])
            .collect(),
    );
    let q = |x: Option<f64>| x.map_or(DataCell::Null, n);
    let wide = table(
        &[
            ("product", ColumnType::Text),
            ("q1", ColumnType::Integer),
            ("q2", ColumnType::Integer),
            ("q3", ColumnType::Integer),
        ],
        vec![
            vec![t("apple"), q(Some(1.0)), q(Some(2.0)), q(Some(3.0))],
            vec![t("pear"), q(Some(4.0)), q(Some(5.0)), q(Some(6.0))],
            vec![t("plum"), q(Some(7.0)), q(None), q(Some(9.0))],
        ],
    );
    let returns = table(
        &[("qty", ColumnType::Integer), ("region", ColumnType::Text), ("id", ColumnType::Integer)],
        vec![vec![n(2.0), t("north"), n(2.0)], vec![n(1.0), t("east"), n(5.0)]],
    );
    (vec![("sales", sales), ("products", products), ("wide", wide), ("returns", returns)], file)
}

/// What our side gives: the whitelist's refusal, a run-time failure, or the result.
enum Ours {
    Refused,
    Failed(DataError),
    Rows(Rowset),
}

fn ours(sql: &str, frames: &[(&'static str, Table)], base: &NamedTempFile) -> Ours {
    let Ok(query) = parse_query(sql).and_then(|q| q.for_engine()) else { return Ours::Refused };
    let inputs = frames
        .iter()
        .map(|(name, table)| QueryInput { name, table, base: table.base.as_ref().map(|_| base.reopen().unwrap()) })
        .collect();
    let mut out = NamedTempFile::new().unwrap();
    match run_query(&query, inputs, out.as_file_mut()) {
        Ok(result) => Ours::Rows(read_back(&result, &out)),
        Err(e) => Ours::Failed(e),
    }
}

fn read_back(result: &QueryOut, out: &NamedTempFile) -> Rowset {
    let count = usize::try_from(result.base.rows).unwrap();
    let table = Table {
        generation: "g".to_owned(),
        epoch: String::new(),
        columns: result.columns.clone(),
        rows: Rows::repaired((0..count).map(|i| i.to_string())),
        base: Some(result.base.clone()),
        cells: imbl::OrdMap::new(),
    };
    let ids: Vec<String> = result.columns.iter().map(|c| c.id.clone()).collect();
    let read = read_view(&table, out.reopen().unwrap(), &ids, 0..count, 1_000_000).unwrap();
    let names = result.columns.iter().map(|c| c.name.clone()).collect();
    (names, read.dataset.source.into_iter().skip(1).map(|row| row.into_iter().map(cell).collect()).collect())
}

/// `DuckDB`'s result, every column cast to DOUBLE, BOOLEAN or VARCHAR (TIMESTAMP through DATE).
fn duck(db: &Connection, sql: &str) -> Result<Rowset, String> {
    db.execute_batch(&format!("DROP TABLE IF EXISTS r; CREATE TABLE r AS {sql}")).map_err(|e| e.to_string())?;
    let mut describe = db.prepare("SELECT column_name, column_type FROM (DESCRIBE r)").unwrap();
    let columns: Vec<(String, String)> =
        describe.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap().map(Result::unwrap).collect();
    let select: Vec<String> = columns
        .iter()
        .map(|(name, ty)| {
            let col = format!("\"{}\"", name.replace('"', "\"\""));
            if ty == "BOOLEAN" {
                col
            } else if ty.starts_with("TIMESTAMP") {
                format!("CAST(CAST({col} AS DATE) AS VARCHAR)")
            } else if ["TINYINT", "SMALLINT", "INTEGER", "BIGINT", "HUGEINT", "FLOAT", "DOUBLE"]
                .iter()
                .any(|n| ty.starts_with(n) || ty.starts_with('U'))
                || ty.starts_with("DECIMAL")
            {
                format!("CAST({col} AS DOUBLE)")
            } else {
                format!("CAST({col} AS VARCHAR)")
            }
        })
        .collect();
    let sql = format!("SELECT {} FROM r", if select.is_empty() { "*".to_owned() } else { select.join(", ") });
    let mut stmt = db.prepare(&sql).unwrap();
    let width = columns.len();
    let rows = stmt
        .query_map([], |row| {
            (0..width)
                .map(|i| {
                    Ok(match row.get_ref(i)? {
                        ValueRef::Null => Cell::Null,
                        ValueRef::Boolean(b) => Cell::Bool(b),
                        ValueRef::Double(x) => num(x),
                        ValueRef::Text(s) => Cell::Text(String::from_utf8_lossy(s).into_owned()),
                        other => panic!("unexpected DuckDB value {other:?}"),
                    })
                })
                .collect::<Result<Vec<_>, duckdb::Error>>()
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    Ok((columns.into_iter().map(|(name, _)| name).collect(), rows))
}

fn sorted(mut set: Rowset, ordered: bool) -> Rowset {
    if !ordered {
        set.1.sort();
    }
    set
}

#[test]
fn oracle_whitelist_queries_match_duckdb() {
    let cases: Vec<Case> = serde_json::from_str(include_str!("oracle_cases.json")).unwrap();
    let (frames, base) = frames();
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch(FRAMES).unwrap();
    let gaps: BTreeSet<&str> = RUNTIME_GAPS.iter().copied().collect();
    let expected: BTreeMap<&str, DataErrorKind> = EXPECTED_REFUSALS.iter().copied().collect();
    let (mut matched, mut refused, mut failed, mut both_failed, mut duck_only_errors) = (0, 0, 0, 0, 0);
    let mut problems = Vec::new();
    for case in &cases {
        let ours = ours(&case.sql, &frames, &base);
        let theirs = duck(&db, &case.sql);
        match (ours, theirs) {
            (Ours::Refused, _) => refused += 1,
            (Ours::Failed(e), _) if expected.get(case.name.as_str()) == Some(&e.kind()) => refused += 1,
            (Ours::Failed(_), _) if gaps.contains(case.name.as_str()) => failed += 1,
            (Ours::Failed(_), Err(_)) => both_failed += 1,
            (Ours::Failed(e), _) => problems.push(format!("{}: failed: {e} ({:?})", case.name, e.kind())),
            (Ours::Rows(_), _) if gaps.contains(case.name.as_str()) || expected.contains_key(case.name.as_str()) => {
                problems.push(format!("{}: runs now, drop it from its list", case.name));
            }
            (Ours::Rows(got), Err(e)) if case.duck_error_is_null && e.contains("by zero") => {
                duck_only_errors += 1;
                let nulls = got.1.iter().all(|row| row.iter().skip(1).all(|c| *c == Cell::Null));
                if !nulls {
                    problems.push(format!("{}: by zero is not NULL: {:?}", case.name, got.1));
                }
            }
            (Ours::Rows(got), Err(e)) => problems.push(format!("{}: DuckDB failed: {e}; ours {got:?}", case.name)),
            (Ours::Rows(got), Ok(want)) => {
                let (got, want) = (sorted(got, case.ordered), sorted(want, case.ordered));
                if got == want {
                    matched += 1;
                } else {
                    problems.push(format!("{}: differs\n  ours {got:?}\n  duck {want:?}", case.name));
                }
            }
        }
    }
    println!(
        "{} cases: {matched} match, {refused} refused, {failed} known gaps, {both_failed} fail in both, \
         {duck_only_errors} DuckDB by-zero errors",
        cases.len()
    );
    assert_that!(problems).named(&problems.join("\n")).is_empty();
}
