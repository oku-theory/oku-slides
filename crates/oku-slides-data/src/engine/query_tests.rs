use rstest::rstest;
use speculoos::prelude::*;
use tempfile::NamedTempFile;

use super::{QueryInput, QueryOut, run_query};
use crate::engine::read_view;
use crate::limits::MAX_ROWS;
use crate::sql::parse_query;
use crate::test_support::{fold, new_table, set_cell};
use oku_slides_i18n::{Localize, Localizer};

use crate::{ColumnType, DataCell, DataError, DataErrorKind, Table};

fn text(s: &str) -> DataCell {
    DataCell::Text(s.to_owned())
}

fn num(n: f64) -> DataCell {
    DataCell::Number(n)
}

/// A stored table whose column names are their ids.
fn table(columns: &[(&str, ColumnType)], rows: Vec<Vec<DataCell>>) -> Table {
    let typed: Vec<(&str, Option<ColumnType>)> = columns.iter().map(|(id, ty)| (*id, Some(*ty))).collect();
    let (mut table, _) = new_table(&typed, rows).into_table().unwrap();
    for column in &mut table.columns {
        column.name = column.id.clone();
    }
    table
}

/// `sales`: id, region, product, qty (integers and texts), price (number), day (date).
fn sales() -> Table {
    let row = |id: f64, region: &str, product: &str, qty: f64, price: f64, day: &str| {
        vec![num(id), text(region), text(product), num(qty), num(price), text(day)]
    };
    table(
        &[
            ("id", ColumnType::Integer),
            ("region", ColumnType::Text),
            ("product", ColumnType::Text),
            ("qty", ColumnType::Integer),
            ("price", ColumnType::Number),
            ("day", ColumnType::Date),
        ],
        vec![
            row(1.0, "north", "apple", 3.0, 1.5, "2026-01-05"),
            row(2.0, "north", "pear", 5.0, 2.0, "2026-01-20"),
            row(3.0, "south", "apple", 2.0, 1.5, "2026-02-03"),
            row(4.0, "south", "plum", -7.0, 2.5, "2026-02-14"),
        ],
    )
}

fn products() -> Table {
    table(
        &[("product", ColumnType::Text), ("category", ColumnType::Text)],
        vec![vec![text("apple"), text("tree")], vec![text("pear"), text("tree")], vec![text("plum"), text("stone")]],
    )
}

/// Runs `sql` over `inputs` (name, table, base file) and reads the result back.
fn run(
    sql: &str,
    inputs: &[(&str, &Table, Option<&NamedTempFile>)],
) -> Result<(QueryOut, Vec<Vec<DataCell>>), DataError> {
    let query = parse_query(sql)?.for_engine()?;
    let inputs = inputs
        .iter()
        .map(|(name, table, base)| QueryInput { name, table, base: base.map(|f| f.reopen().unwrap()) })
        .collect();
    let mut out = NamedTempFile::new().unwrap();
    let result = run_query(&query, inputs, out.as_file_mut())?;
    let rows = read_back(&result, &out);
    Ok((result, rows))
}

/// The rows of a query result, read from its base.
fn read_back(result: &QueryOut, out: &NamedTempFile) -> Vec<Vec<DataCell>> {
    let count = result.base.rows as usize;
    let table = result.table();
    let ids: Vec<String> = result.columns.iter().map(|c| c.id.clone()).collect();
    let read = read_view(&table, out.reopen().unwrap(), &ids, 0..count, 100_000).unwrap();
    read.dataset.source.into_iter().skip(1).collect()
}

fn names(result: &QueryOut) -> Vec<&str> {
    result.columns.iter().map(|c| c.name.as_str()).collect()
}

fn refusal(sql: &str) -> DataError {
    let sales = sales();
    match run(sql, &[("sales", &sales, None)]) {
        Ok((result, _)) => panic!("{sql} ran: {:?}", result.columns),
        Err(error) => error,
    }
}

/// Runs `sql` over `sales` and gives its rows.
fn rows(sql: &str) -> Vec<Vec<DataCell>> {
    let sales = sales();
    run(sql, &[("sales", &sales, None)]).unwrap_or_else(|e| panic!("{sql}: {e}")).1
}

#[test]
fn run_query_joins_two_inputs_overlay_over_base() {
    let (mut based, file) = fold(&sales());
    set_cell(&mut based, "r1", "qty", num(50.0));
    let products = products();

    let (result, rows) = run(
        "SELECT s.id, p.category, s.qty FROM sales s JOIN products p ON s.product = p.product ORDER BY s.id",
        &[("sales", &based, Some(&file)), ("products", &products, None)],
    )
    .unwrap();

    assert_that!(names(&result)).is_equal_to(vec!["id", "category", "qty"]);
    assert_that!(rows).is_equal_to(vec![
        vec![num(1.0), text("tree"), num(3.0)],
        vec![num(2.0), text("tree"), num(50.0)],
        vec![num(3.0), text("tree"), num(2.0)],
        vec![num(4.0), text("stone"), num(-7.0)],
    ]);
    let ids: Vec<&str> = result.columns.iter().map(|c| c.id.as_str()).collect();
    assert_that!(ids).is_equal_to(vec!["id", "category", "qty"]);
}

#[test]
fn run_query_hidden_rows_and_columns_are_not_seen() {
    let (mut based, file) = fold(&sales());
    based.rows.remove(0);
    based.columns.retain(|c| c.id != "price");

    let (result, rows) = run("SELECT * FROM sales ORDER BY id", &[("sales", &based, Some(&file))]).unwrap();
    let hidden = run("SELECT price FROM sales", &[("sales", &based, Some(&file))]).unwrap_err();

    assert_that!(names(&result)).is_equal_to(vec!["id", "region", "product", "qty", "day"]);
    assert_that!(rows.iter().map(|row| row[0].clone()).collect::<Vec<_>>()).is_equal_to(vec![
        num(2.0),
        num(3.0),
        num(4.0),
    ]);
    assert_that!(hidden.kind()).is_equal_to(DataErrorKind::QueryRun);
}

#[rstest]
#[case::text("SELECT region AS x FROM sales", ColumnType::Text)]
#[case::number("SELECT price AS x FROM sales", ColumnType::Number)]
#[case::integer("SELECT qty AS x FROM sales", ColumnType::Integer)]
#[case::int32("SELECT CAST(qty AS INTEGER) AS x FROM sales", ColumnType::Integer)]
#[case::count("SELECT COUNT(*) AS x FROM sales", ColumnType::Integer)]
#[case::bool("SELECT qty > 2 AS x FROM sales", ColumnType::Bool)]
#[case::date("SELECT day AS x FROM sales", ColumnType::Date)]
#[case::null("SELECT NULL AS x FROM sales", ColumnType::Text)]
fn result_dtype_maps_to_column_type(#[case] sql: &str, #[case] ty: ColumnType) {
    let sales = sales();
    let (result, _) = run(sql, &[("sales", &sales, None)]).unwrap();
    assert_that!(result.columns.iter().map(|c| c.ty).collect::<Vec<_>>()).is_equal_to(vec![ty]);
}

#[test]
fn datetime_result_column_is_refused_and_named() {
    let error = refusal("SELECT strptime('2024', '%Y') AS at FROM sales");
    assert_that!(error.kind()).is_equal_to(DataErrorKind::QueryColumnType);
    assert_that!(error.to_string()).is_equal_to("result column at has a type data cannot hold".to_owned());
}

#[test]
fn result_over_max_rows_is_refused() {
    let tall = crate::test_support::sized(MAX_ROWS / 2 + 1, 1);
    let error =
        run("SELECT a.C0 FROM tall a CROSS JOIN (VALUES (1), (2)) AS b(x)", &[("tall", &tall, None)]).unwrap_err();
    assert_that!(error.kind()).is_equal_to(DataErrorKind::QueryTooLarge);
}

#[test]
fn duplicate_output_names_are_refused() {
    let error = refusal("SELECT id AS x, qty AS x FROM sales");
    assert_that!(error.kind()).is_equal_to(DataErrorKind::QueryDuplicateColumn);
    assert_that!(error.to_string()).is_equal_to("two result columns are named x".to_owned());
}

#[test]
fn run_query_error_reason_never_quotes_cell_values() {
    let error = refusal("SELECT CAST(region AS INTEGER) AS x FROM sales");
    assert_that!(error.kind()).is_equal_to(DataErrorKind::QueryRun);
    assert_that!(error.to_string().contains("north")).is_false();
}

#[test]
fn negative_integer_mod_and_div_truncate_like_duckdb() {
    let rows = rows("SELECT qty % 2 AS m, mod(qty, -2) AS n, qty // 2 AS d FROM sales WHERE id = 4");
    assert_that!(rows).is_equal_to(vec![vec![num(-1.0), num(-1.0), num(-3.0)]]);
}

#[test]
fn division_by_zero_is_null_like_mod_and_div() {
    let rows = rows("SELECT qty / 0 AS a, price / (qty - qty) AS b, qty / 2 AS c FROM sales WHERE id = 1");
    assert_that!(rows).is_equal_to(vec![vec![DataCell::Null, DataCell::Null, num(1.5)]]);
}

#[test]
fn pivot_aggregate_division_by_zero_is_null() {
    let sql = "SELECT * FROM (SELECT region, qty FROM sales) AS s PIVOT (SUM(qty / 0) FOR region IN ('north'))";
    assert_that!(rows(sql)).is_equal_to(vec![vec![DataCell::Null]]);
}

#[rstest]
#[case::mod_column("SELECT price % 2 AS m FROM sales")]
#[case::mod_call("SELECT mod(qty, price) AS m FROM sales")]
#[case::div_literal("SELECT qty // 0.5 AS m FROM sales")]
fn float_operand_of_mod_or_div_is_refused(#[case] sql: &str) {
    assert_that!(refusal(sql).kind()).is_equal_to(DataErrorKind::QueryFloatOperand);
}

#[test]
fn round_of_float_ties_goes_away_from_zero_and_cast_to_even() {
    let rows = rows(
        "SELECT round(price) AS r, round(-price) AS n, CAST(price AS BIGINT) AS c, round(qty) AS i \
         FROM sales WHERE id IN (1, 4) ORDER BY id",
    );
    assert_that!(rows).is_equal_to(vec![
        vec![num(2.0), num(-2.0), num(2.0), num(3.0)],
        vec![num(3.0), num(-3.0), num(2.0), num(-7.0)],
    ]);
}

#[test]
fn float_round_to_decimals_is_refused() {
    assert_that!(refusal("SELECT round(price, 1) AS r FROM sales").kind())
        .is_equal_to(DataErrorKind::QueryFloatOperand);
}

#[test]
fn star_over_join_takes_duckdb_names() {
    let (sales, products) = (sales(), products());
    let (result, _) = run(
        "SELECT * FROM sales s JOIN products p ON s.product = p.product",
        &[("sales", &sales, None), ("products", &products, None)],
    )
    .unwrap();
    let (using, _) = run(
        "SELECT * EXCLUDE (day) FROM sales JOIN products USING (product)",
        &[("sales", &sales, None), ("products", &products, None)],
    )
    .unwrap();

    assert_that!(names(&result)).is_equal_to(vec![
        "id",
        "region",
        "product",
        "qty",
        "price",
        "day",
        "product_1",
        "category",
    ]);
    assert_that!(names(&using)).is_equal_to(vec!["id", "region", "product", "qty", "price", "category"]);
}

#[test]
fn select_alias_in_where_is_its_expression() {
    let rows = rows("SELECT id, qty * 2 AS twice FROM sales WHERE twice > 5 ORDER BY id");
    assert_that!(rows).is_equal_to(vec![vec![num(1.0), num(6.0)], vec![num(2.0), num(10.0)]]);
}

#[test]
fn pivot_groups_the_other_columns() {
    let rows = rows(
        "SELECT * FROM (SELECT region, product, qty FROM sales) AS src PIVOT (sum(qty) AS total FOR product IN ('apple' AS a, 'pear')) ORDER BY region",
    );
    assert_that!(rows)
        .is_equal_to(vec![vec![text("north"), num(3.0), num(5.0)], vec![text("south"), num(2.0), DataCell::Null]]);
}

#[test]
fn pivot_names_values_with_their_aggregate_alias() {
    let sales = sales();
    let (result, _) = run(
        "SELECT * FROM (SELECT region, product, qty FROM sales) AS src PIVOT (sum(qty) AS total FOR product IN ('apple' AS a, 'pear'))",
        &[("sales", &sales, None)],
    )
    .unwrap();
    assert_that!(names(&result)).is_equal_to(vec!["region", "a_total", "pear_total"]);
}

#[test]
fn unpivot_skips_nulls_and_widens_to_number() {
    let rows = rows(
        "SELECT id, metric, v FROM (SELECT id, qty, price, CAST(NULL AS DOUBLE) AS gap FROM sales WHERE id = 1) AS src \
         UNPIVOT (v FOR metric IN (qty, price, gap)) ORDER BY metric",
    );
    assert_that!(rows)
        .is_equal_to(vec![vec![num(1.0), text("price"), num(1.5)], vec![num(1.0), text("qty"), num(3.0)]]);
}

#[test]
fn pivot_over_unnamed_subquery_runs() {
    let rows = rows(
        "SELECT * FROM (SELECT region, product, qty FROM sales) PIVOT (sum(qty) FOR product IN ('apple', 'plum')) ORDER BY region",
    );
    assert_that!(rows)
        .is_equal_to(vec![vec![text("north"), num(3.0), DataCell::Null], vec![text("south"), num(2.0), num(-7.0)]]);
}

#[test]
fn pivot_count_of_no_row_is_zero() {
    let rows = rows(
        "SELECT * FROM (SELECT region, product FROM sales) AS s PIVOT (count(*) FOR product IN ('pear', 'plum')) ORDER BY region",
    );
    assert_that!(rows)
        .is_equal_to(vec![vec![text("north"), num(1.0), num(0.0)], vec![text("south"), num(0.0), num(1.0)]]);
}

#[test]
fn date_plus_days_and_date_difference_follow_duckdb() {
    let rows = rows(
        "SELECT day + 1 AS next, 2 + day AS later, day - 5 AS before, day - DATE '2026-01-01' AS since FROM sales WHERE id = 1",
    );
    assert_that!(rows).is_equal_to(vec![vec![text("2026-01-06"), text("2026-01-07"), text("2025-12-31"), num(4.0)]]);
}

#[rstest]
#[case::modulo("SELECT qtty % 2 AS m FROM sales")]
#[case::cast("SELECT CAST(qtty AS INTEGER) AS n FROM sales")]
#[case::round("SELECT round(qtty, 1) AS r FROM sales")]
#[case::order_by_alias("SELECT qty AS q FROM sales ORDER BY qq % 2")]
#[case::union_order_by("SELECT qty FROM sales UNION ALL SELECT qty FROM sales ORDER BY qtty % 2")]
fn mistyped_column_under_a_typed_rule_is_the_engine_refusal(#[case] sql: &str) {
    let error = refusal(sql);
    assert_that!(error.kind()).is_equal_to(DataErrorKind::QueryRun);
    assert_that!(error.message().id()).is_equal_to("data-query-run");
}

#[rstest]
#[case::order_by_alias("SELECT id, qty AS q FROM sales ORDER BY q % 2, id", vec![4.0, 3.0, 1.0, 2.0])]
#[case::union_order_by(
    "SELECT id, qty FROM sales WHERE id < 3 UNION ALL SELECT id, qty FROM sales WHERE id > 2 ORDER BY qty % 2, id",
    vec![4.0, 3.0, 1.0, 2.0]
)]
fn typed_rule_in_order_by_reads_output_columns(#[case] sql: &str, #[case] ids: Vec<f64>) {
    let firsts: Vec<DataCell> = rows(sql).into_iter().map(|row| row[0].clone()).collect();
    assert_that!(firsts).is_equal_to(ids.into_iter().map(num).collect::<Vec<_>>());
}

#[rstest]
#[case::space("SELECT count(*) AS \"a b\" FROM sales", "a b")]
#[case::too_long(&format!("SELECT count(*) AS {} FROM sales", "x".repeat(65)), &"x".repeat(65))]
fn result_column_name_that_is_no_id_is_refused(#[case] sql: &str, #[case] name: &str) {
    let error = refusal(sql);
    assert_that!(error.kind()).is_equal_to(DataErrorKind::IdInvalid);
    let fr = error.localize(&Localizer::new(Some("fr")));
    assert_that!(fr).contains(name);
    assert_that!(fr).is_not_equal_to(error.localize(&Localizer::new(Some("en"))));
}

#[test]
fn date_plus_month_interval_runs_without_probing_the_interval() {
    let rows = rows("SELECT day + INTERVAL 1 MONTH AS v FROM sales WHERE id = 1");
    assert_that!(rows).is_equal_to(vec![vec![text("2026-02-05")]]);
}

#[test]
fn query_out_table_same_result_gives_same_table() {
    let sales = sales();
    let (result, _) = run("SELECT region FROM sales", &[("sales", &sales, None)]).unwrap();
    assert_that!(result.table()).is_equal_to(result.table());
    assert_that!(result.table().generation.as_str()).is_equal_to(result.base.id.hex());
    assert_that!(result.table().rows.len()).is_equal_to(result.base.rows as usize);
}
