use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::DataErrorKind;
use crate::error::DataInner;
use crate::limits::{MAX_QUERY_DEPTH, MAX_SQL_BYTES, MAX_SQL_NESTING};

/// The construct a refusal names, or the error's kind when it is not a construct refusal.
fn construct(sql: &str) -> String {
    match parse_query(sql) {
        Ok(_) => "accepted".to_owned(),
        Err(e) => match &e.0 {
            DataInner::QueryConstruct { construct, .. } => construct.clone(),
            _ => format!("{:?}", e.kind()),
        },
    }
}

/// Runs `f` on a thread with the 2 MiB stack of a test or worker thread.
fn on_small_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new().stack_size(2 << 20).spawn(f).unwrap().join().unwrap()
}

#[rstest]
#[case::select_where("SELECT id, qty FROM sales WHERE qty > 2 AND NOT region = 'north'")]
#[case::distinct("SELECT DISTINCT region FROM sales")]
#[case::distinct_on("SELECT DISTINCT ON (region) region, qty FROM sales ORDER BY region, qty DESC")]
#[case::limit_offset("SELECT id FROM sales ORDER BY id LIMIT 3 OFFSET 1")]
#[case::fetch_first("SELECT id FROM sales ORDER BY id FETCH FIRST 2 ROWS ONLY")]
#[case::group_having("SELECT region, SUM(qty) AS t FROM sales GROUP BY region HAVING SUM(qty) > 3")]
#[case::case_when(
    "SELECT CASE WHEN qty > 2 THEN 'big' ELSE 'small' END AS s, CASE region WHEN 'n' THEN 1 END FROM sales"
)]
#[case::in_list("SELECT id FROM sales WHERE region IN ('north', 'south') OR qty NOT IN (1, 2)")]
#[case::between_like("SELECT id FROM sales WHERE qty BETWEEN 1 AND 3 AND region LIKE 'n%' AND product ILIKE 'A%'")]
#[case::is_distinct(
    "SELECT id FROM sales WHERE qty IS DISTINCT FROM 2 OR qty IS NOT DISTINCT FROM NULL OR qty IS NULL"
)]
#[case::values("SELECT n, s FROM (VALUES (1, 'a'), (2, 'b')) AS v(n, s)")]
#[case::concat("SELECT region || '-' || product AS k FROM sales")]
#[case::sum_aggregate("SELECT SUM(qty) AS t FROM sales")]
#[case::sum_window("SELECT id, SUM(qty) OVER (PARTITION BY region) AS t FROM sales")]
#[case::division("SELECT qty / 2 AS h FROM sales")]
#[case::no_from("SELECT 1 + 1 AS two")]
#[case::exponent("SELECT 1e3 AS k")]
#[case::in_subquery("SELECT id FROM sales WHERE product IN (SELECT product FROM products)")]
#[case::scalar_where("SELECT id FROM sales WHERE price > (SELECT AVG(price) FROM sales)")]
#[case::scalar_select("SELECT id, price - (SELECT AVG(price) FROM sales) AS d FROM sales")]
#[case::scalar_having(
    "SELECT region, AVG(price) AS a FROM sales GROUP BY region HAVING AVG(price) > (SELECT AVG(price) FROM sales)"
)]
#[case::derived("SELECT t.n FROM (SELECT qty AS n FROM sales) AS t")]
#[case::with_chained("WITH a AS (SELECT * FROM sales), b(x) AS (SELECT qty FROM a) SELECT x FROM b")]
#[case::inner_join("SELECT s.id, p.category FROM sales s JOIN products p ON s.product = p.product")]
#[case::left_join("SELECT s.id, p.category FROM sales AS s LEFT JOIN products AS p ON s.product = p.product")]
#[case::right_join("SELECT s.id FROM sales s RIGHT JOIN products p ON s.product = p.product")]
#[case::full_join("SELECT s.id FROM sales s FULL OUTER JOIN products p ON s.product = p.product")]
#[case::cross_join("SELECT s.id FROM sales s CROSS JOIN products p")]
#[case::natural_join("SELECT id FROM sales NATURAL JOIN products")]
#[case::using_inner("SELECT product, id FROM sales JOIN products USING (product)")]
#[case::using_left("SELECT product, id FROM sales LEFT JOIN products USING (product)")]
#[case::using_full("SELECT product, s.id FROM sales s FULL JOIN products p USING (product)")]
#[case::comma_join("SELECT s.id FROM sales s, products p WHERE s.product = p.product")]
#[case::inner_non_equality("SELECT s.id FROM sales s JOIN products p ON s.product = p.product AND s.qty > 2")]
#[case::union("SELECT id FROM sales UNION SELECT id FROM products")]
#[case::union_all_order_limit("SELECT id FROM sales UNION ALL SELECT id FROM products ORDER BY id LIMIT 3")]
#[case::row_number("SELECT id, ROW_NUMBER() OVER (PARTITION BY region ORDER BY qty DESC) AS rn FROM sales")]
#[case::lag_lead("SELECT LAG(qty) OVER (ORDER BY id), LEAD(qty, 2) OVER (ORDER BY id) FROM sales")]
#[case::first_value("SELECT FIRST_VALUE(id) OVER (PARTITION BY region ORDER BY qty) AS f FROM sales")]
#[case::running_sum(
    "SELECT SUM(qty) OVER (ORDER BY id ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) AS r FROM sales"
)]
#[case::running_avg("SELECT AVG(qty) OVER (ORDER BY id ROWS UNBOUNDED PRECEDING) AS r FROM sales")]
#[case::named_window("SELECT SUM(qty) OVER w AS t FROM sales WINDOW w AS (PARTITION BY region)")]
#[case::group_by_all("SELECT region, product, SUM(qty) AS q FROM sales GROUP BY ALL ORDER BY ALL")]
#[case::group_by_position_alias("SELECT upper(region) AS r, COUNT(*) AS n FROM sales GROUP BY 1 ORDER BY r")]
#[case::group_by_alias("SELECT upper(region) AS r, COUNT(*) AS n FROM sales GROUP BY r HAVING n > 1")]
#[case::group_by_expression(
    "SELECT EXTRACT(month FROM day) AS m, COUNT(*) AS n FROM sales GROUP BY EXTRACT(month FROM day)"
)]
#[case::star_exclude_replace("SELECT * EXCLUDE (id) REPLACE (qty * 2 AS qty) FROM sales")]
#[case::columns_regex("SELECT COLUMNS('^q') FROM sales")]
#[case::qualify("SELECT id, region FROM sales QUALIFY ROW_NUMBER() OVER (PARTITION BY region ORDER BY price DESC) = 1")]
#[case::filter("SELECT SUM(qty) FILTER (WHERE price >= 2) AS s, COUNT(*) FILTER (WHERE qty > 3) AS n FROM sales")]
#[case::pivot(
    "SELECT * FROM (SELECT product, region, qty FROM sales) AS s PIVOT (SUM(qty) FOR region IN ('north', 'south'))"
)]
#[case::unpivot("SELECT * FROM wide UNPIVOT (amount FOR quarter IN (q1, q2, q3))")]
#[case::text_functions(
    "SELECT upper(region), lower(region), length(region), char_length(region), substr(region, 2, 3), \
     substring(region FROM 2 FOR 3), left(region, 2), right(region, 2), replace(region, 'n', 'm'), trim(region), \
     ltrim(region), rtrim(region), concat(region, product), concat_ws('-', region, product), \
     starts_with(region, 'n'), ends_with(region, 'h'), strpos(region, 'o'), position('o' IN region), \
     reverse(region), split_part(region, 'r', 1), contains(region, 'or'), lcase(region), ucase(region), \
     prefix(region, 'n'), suffix(region, 'h'), regexp_matches(region, '^n') FROM sales"
)]
#[case::number_functions(
    "SELECT abs(qty), floor(price), ceil(price), ceiling(price), sqrt(qty), power(qty, 2), pow(qty, 2), \
     exp(qty), ln(qty), log10(qty), log2(qty), sign(qty), greatest(qty, 1), least(qty, 1), pi(), cbrt(qty), \
     degrees(qty), radians(qty), sin(qty), cos(qty), log(qty) FROM sales"
)]
#[case::condition_functions(
    "SELECT coalesce(qty, 0), nullif(qty, 0), ifnull(qty, 0), if(qty > 1, 'a', 'b') FROM sales"
)]
#[case::casts(
    "SELECT CAST(qty AS DOUBLE), CAST(qty AS REAL), CAST(qty AS BIGINT), qty::VARCHAR, TRY_CAST(region AS TEXT), \
     CAST(1 AS BOOLEAN), CAST('2026-01-01' AS DATE) FROM sales"
)]
#[case::date_functions(
    "SELECT EXTRACT(year FROM day), date_part('month', day), strftime(day, '%Y'), strptime('2026-01-01', '%Y-%m-%d'), \
     DATE '2026-01-01', date('2026-01-01'), year(day), month(day), day(day), quarter(day), week(day), \
     dayofweek(day), dayofmonth(day), dayofyear(day), isodow(day), isoyear(day), datepart('year', day), \
     dayname(day), monthname(day), date_trunc('month', day), date_trunc('year', day), \
     date_add(day, INTERVAL 1 MONTH), date_diff('day', day, day), datediff('month', day, day), \
     date_diff('year', day, day), last_day(day), make_date(2026, 1, 31), day + INTERVAL '2 days' FROM sales"
)]
#[case::aggregates(
    "SELECT count(*), count(qty), count(DISTINCT qty), sum(qty), avg(qty), min(qty), max(qty), median(qty), \
     stddev(qty), stddev_samp(qty), variance(qty), var_samp(qty), quantile_cont(qty, 0.5), \
     quantile_disc(qty, 0.5), covar_samp(qty, price), covar_pop(qty, price), mean(qty) FROM sales"
)]
#[case::schema_integer_modulo("SELECT qty % 3, mod(qty, 3), qty // 2 FROM sales")]
#[case::schema_round("SELECT round(price), round(price, 2) FROM sales")]
#[case::schema_cast_integer("SELECT CAST(price AS INTEGER) FROM sales")]
#[case::schema_star_join("SELECT * FROM sales s JOIN products p ON s.product = p.product")]
#[case::schema_where_alias("SELECT qty * 2 AS d FROM sales WHERE d > 2")]
#[case::schema_date_arithmetic("SELECT day + 1, day - day FROM sales")]
#[case::trailing_semicolon("SELECT id FROM sales;")]
fn parse_accepts_whitelisted(#[case] sql: &str) {
    assert_that!(construct(sql)).is_equal_to("accepted".to_owned());
}

#[rstest]
#[case::table_function("SELECT * FROM read_csv('x')", "read_csv")]
#[case::file_path("SELECT * FROM 'x.parquet'", "'x.parquet'")]
#[case::attach("ATTACH 'x' AS y", "ATTACH")]
#[case::insert("INSERT INTO sales VALUES (1)", "INSERT")]
#[case::create("CREATE TABLE t (a INTEGER)", "CREATE")]
#[case::sample("SELECT * FROM sales USING SAMPLE 10", "SAMPLE")]
#[case::tablesample("SELECT * FROM sales TABLESAMPLE 10", "SAMPLE")]
#[case::asof_join("SELECT * FROM sales ASOF JOIN products ON sales.day >= products.day", "ASOF JOIN")]
#[case::from_first("FROM sales SELECT *", "FROM … SELECT")]
#[case::list("SELECT [1, 2]", "[…]")]
#[case::structure("SELECT {'a': 1}", "{…}")]
#[case::lambda("SELECT list_transform(qty, x -> x + 1) FROM sales", "->")]
#[case::dot_call("SELECT sales.region.upper() FROM sales", "sales.region.upper()")]
#[case::at_version("SELECT * FROM sales AT (VERSION => 'x')", "AT (VERSION => …)")]
#[case::two_statements("SELECT 1; SELECT 2", ";")]
#[case::positional("SELECT * FROM sales POSITIONAL JOIN products", "POSITIONAL JOIN")]
#[case::describe("DESCRIBE sales", "DESCRIBE")]
#[case::summarize("SUMMARIZE sales", "SUMMARIZE")]
#[case::with_recursive("WITH RECURSIVE r AS (SELECT 1 AS n) SELECT n FROM r", "WITH RECURSIVE")]
#[case::except("SELECT id FROM sales EXCEPT SELECT id FROM products", "EXCEPT")]
#[case::intersect("SELECT id FROM sales INTERSECT SELECT id FROM products", "INTERSECT")]
#[case::union_by_name("SELECT id FROM sales UNION BY NAME SELECT id FROM products", "UNION BY NAME")]
#[case::exists("SELECT id FROM sales WHERE EXISTS (SELECT 1 FROM products)", "EXISTS")]
#[case::not_in_subquery("SELECT id FROM sales WHERE id NOT IN (SELECT id FROM products)", "NOT IN (SELECT …)")]
#[case::in_null("SELECT id FROM sales WHERE qty IN (1, NULL)", "IN (… NULL …)")]
#[case::in_cast_null(
    "SELECT id FROM sales WHERE qty NOT IN (1, CAST(NULL AS INTEGER))",
    "IN (… CAST(NULL AS INTEGER) …)"
)]
#[case::in_nested_null("SELECT id FROM sales WHERE qty NOT IN (1, (NULL))", "IN (… (NULL) …)")]
#[case::in_colon_null("SELECT id FROM sales WHERE qty NOT IN (1, NULL::INT)", "IN (… NULL::INT …)")]
#[case::in_column("SELECT id FROM sales WHERE qty NOT IN (1, price)", "IN (… price …)")]
#[case::replace_subquery("SELECT * REPLACE ((SELECT MAX(qty) FROM sales) AS qty) FROM sales", "(SELECT …)")]
#[case::typed_alias_column("SELECT a FROM (SELECT 1) AS t (a INTEGER)", "AS … (a INTEGER)")]
#[case::semi_join("SELECT id FROM sales SEMI JOIN products USING (product)", "SEMI JOIN")]
#[case::anti_join("SELECT id FROM sales ANTI JOIN products USING (product)", "ANTI JOIN")]
#[case::outer_non_equality(
    "SELECT s.id FROM sales s LEFT JOIN products p ON s.qty > p.qty",
    "LEFT JOIN … ON s.qty > p.qty"
)]
#[case::mixed_directions(
    "SELECT ROW_NUMBER() OVER (ORDER BY qty, id DESC) FROM sales",
    "OVER (ORDER BY … ASC, … DESC)"
)]
#[case::running_min("SELECT MIN(qty) OVER (ORDER BY id) FROM sales", "MIN(…) OVER (ORDER BY …)")]
#[case::range_frame(
    "SELECT SUM(qty) OVER (ORDER BY id) FROM sales",
    "RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW"
)]
#[case::other_frame(
    "SELECT SUM(qty) OVER (ORDER BY id ROWS BETWEEN 1 PRECEDING AND CURRENT ROW) FROM sales",
    "ROWS BETWEEN 1 PRECEDING AND CURRENT ROW"
)]
#[case::star_rename("SELECT * RENAME (id AS n) FROM sales", "* RENAME")]
#[case::columns_in_where("SELECT id FROM sales WHERE COLUMNS('q') > 1", "COLUMNS(…)")]
#[case::sum_distinct("SELECT SUM(DISTINCT qty) FROM sales", "SUM(DISTINCT …)")]
#[case::round_negative("SELECT round(qty, -1) FROM sales", "round(…, -1)")]
#[case::correlated(
    "SELECT id FROM sales s WHERE qty > (SELECT AVG(qty) FROM sales t WHERE t.region = s.region)",
    "s.region"
)]
#[case::grouped_scalar(
    "SELECT id FROM sales WHERE qty > (SELECT AVG(qty) FROM sales GROUP BY region)",
    "(SELECT … GROUP BY …)"
)]
#[case::non_aggregate_scalar("SELECT id FROM sales WHERE qty > (SELECT qty FROM sales)", "(SELECT …)")]
#[case::duckdb_pivot("PIVOT sales ON region USING SUM(qty)", "PIVOT")]
#[case::multi_part_table("SELECT * FROM main.sales", "main.sales")]
#[case::lag_default("SELECT LAG(qty, 1, 0) OVER (ORDER BY id) FROM sales", "LAG(…, …, …)")]
#[case::reserved_identifier("SELECT __x FROM sales", "__x")]
#[case::interval_hour("SELECT day + INTERVAL 1 HOUR FROM sales", "INTERVAL 1 HOUR")]
#[case::cast_decimal("SELECT CAST(qty AS DECIMAL(10,2)) FROM sales", "DECIMAL(10,2)")]
#[case::is_true("SELECT id FROM sales WHERE (qty > 1) IS TRUE", "IS TRUE")]
#[case::log_base("SELECT log(2, qty) FROM sales", "log(…, …)")]
#[case::date_trunc_week("SELECT date_trunc('week', day) FROM sales", "date_trunc('week', …)")]
fn parse_refuses_construct_named(#[case] sql: &str, #[case] named: &str) {
    assert_that!(construct(sql)).is_equal_to(named.to_owned());
}

#[rstest]
#[case::string_agg("SELECT string_agg(region, ',') FROM sales", "string_agg")]
#[case::rank("SELECT rank() OVER (ORDER BY qty) FROM sales", "rank")]
#[case::min_by("SELECT min_by(id, qty) FROM sales", "min_by")]
#[case::corr("SELECT corr(qty, price) FROM sales", "corr")]
#[case::keeps_case("SELECT Read_Blob('x')", "Read_Blob")]
fn parse_refuses_unknown_function_named(#[case] sql: &str, #[case] function: &str) {
    let error = parse_query(sql).unwrap_err();
    let l = Localizer::new(Some("fr"));
    let fr = error.localize(&l);
    assert_that!(error.kind()).is_equal_to(DataErrorKind::QueryUnknownFunction);
    assert_that!(fr)
        .is_equal_to(l.tr_with("data-query-unknown-function", &[("function", crate::quoted(function).into())]));
    assert_that!(fr).contains(function);
}

#[test]
fn parse_refuses_nesting_past_limit() {
    let sql = format!("SELECT * FROM {}sales{}", "(SELECT * FROM ".repeat(65), ") AS t".repeat(65));
    let error = on_small_stack(move || parse_query(&sql).map(|_| ()).unwrap_err().kind());
    assert_that!(error).is_equal_to(DataErrorKind::QueryTooDeep);
}

#[rstest]
#[case::not("NOT (", ")")]
#[case::try_cast("TRY_CAST(", " AS INTEGER)")]
#[case::substring("substring(", " FROM 1)")]
#[case::interval("INTERVAL (", ") DAY")]
fn parse_refuses_brackets_past_max_nesting_before_the_parser_backtracks(#[case] open: &str, #[case] close: &str) {
    // Unbounded, sqlparser 0.60 backtracks in time doubling per level on a syntax error inside:
    // 40 levels would not return. The error sits at the bottom so the parser would backtrack.
    let depth = 40;
    let sql = format!("SELECT {}1 +{} FROM sales", open.repeat(depth), close.repeat(depth));
    let kind = on_small_stack(move || parse_query(&sql).map(|_| ()).unwrap_err().kind());
    assert_that!(kind).is_equal_to(DataErrorKind::QueryTooDeep);
}

#[rstest]
#[case::subquery("SELECT * FROM {open}sales{close}", "(SELECT * FROM ", ") AS t")]
#[case::not("SELECT {open}1{close} FROM sales", "NOT (", ")")]
fn parse_accepts_brackets_at_max_nesting_on_a_small_stack(
    #[case] shape: &str,
    #[case] open: &str,
    #[case] close: &str,
) {
    let sql = shape.replace("{open}", &open.repeat(MAX_SQL_NESTING)).replace("{close}", &close.repeat(MAX_SQL_NESTING));
    let engine = on_small_stack(move || parse_query(&sql).and_then(|q| q.for_engine()).map(|e| e.sql.len()));
    assert_that!(engine).is_ok();
}

#[test]
fn parse_refuses_a_chain_past_max_depth_on_a_small_stack() {
    // The parser builds `1 + 1 + …` in a loop: only the explicit depth walk sees it.
    let sql = format!("SELECT {}1 FROM sales", "1 + ".repeat(MAX_QUERY_DEPTH));
    let kind = on_small_stack(move || parse_query(&sql).map(|_| ()).unwrap_err().kind());
    assert_that!(kind).is_equal_to(DataErrorKind::QueryTooDeep);
}

#[test]
fn parse_refuses_the_deepest_chain_under_16_kib_on_a_small_stack() {
    let terms = (MAX_SQL_BYTES - "SELECT 1".len()) / "+1".len();
    let sql = format!("SELECT 1{}", "+1".repeat(terms));
    assert_that!(sql.len()).is_less_than_or_equal_to(MAX_SQL_BYTES);
    let kind = on_small_stack(move || parse_query(&sql).map(|_| ()).unwrap_err().kind());
    assert_that!(kind).is_equal_to(DataErrorKind::QueryTooDeep);
}

#[test]
fn parse_accepts_a_chain_just_under_max_depth_and_rewrites_it_on_a_small_stack() {
    let sql = format!("SELECT {}qty FROM sales", "qty / ".repeat(MAX_QUERY_DEPTH - 12));
    let engine = on_small_stack(move || parse_query(&sql).and_then(|q| q.for_engine()).map(|e| e.sql.len()));
    assert_that!(engine).is_ok();
}

#[test]
fn parse_refuses_sql_over_16_kib() {
    let sql = format!("SELECT id FROM sales WHERE region = '{}'", "x".repeat(MAX_SQL_BYTES));
    let error = parse_query(&sql).unwrap_err();
    assert_that!(error.kind()).is_equal_to(DataErrorKind::QueryTooLong);
    let l = Localizer::new(Some("fr"));
    assert_that!(error.localize(&l)).is_equal_to(l.tr_with("data-query-too-long", &[("max", MAX_SQL_BYTES.into())]));
}

#[test]
fn parse_error_names_line_and_column_not_text_fr() {
    let error = parse_query("SELECT secret_column\nFROM sales WHERE qty > > 2").unwrap_err();
    let l = Localizer::new(Some("fr"));
    let fr = error.localize(&l);
    assert_that!(error.kind()).is_equal_to(DataErrorKind::QueryParse);
    let DataInner::QueryParse { line, column, .. } = &error.0 else { panic!("{error:?}") };
    assert_that!(*line).is_equal_to(2);
    assert_that!(*column).is_greater_than(20);
    let at = [("line", "2".into()), ("column", column.to_string().into())];
    assert_that!(fr).is_equal_to(l.tr_with("data-query-parse", &at));
    assert_that!(fr.contains("secret_column") || fr.contains("sales") || fr.contains('>')).is_false();
}

#[test]
fn parse_error_at_end_of_input_names_the_last_position() {
    let error = parse_query("SELECT id\nFROM sales WHERE").unwrap_err();
    let DataInner::QueryParse { line, column, .. } = &error.0 else { panic!("{error:?}") };
    assert_that!((*line, *column)).is_equal_to((2, 17));
}

#[test]
fn parse_refuses_an_empty_query_as_a_parse_error() {
    assert_that!(parse_query("  ;").unwrap_err().kind()).is_equal_to(DataErrorKind::QueryParse);
}

#[test]
fn to_sql_is_the_parsed_text_normalized() {
    let query = parse_query("select id from Sales where qty>2").unwrap();
    assert_that!(query.to_sql()).is_equal_to("SELECT id FROM Sales WHERE qty > 2".to_owned());
}

#[test]
fn engine_query_round_trips_through_json() {
    let engine = parse_query("SELECT id FROM sales").unwrap().for_engine().unwrap();
    let json = serde_json::to_string(&engine).unwrap();
    assert_that!(serde_json::from_str::<EngineQuery>(&json).unwrap()).is_equal_to(engine);
}

#[test]
fn parse_reports_sample_and_asof_only_at_the_error_position() {
    // Here they are column names: the failure is the missing condition at the end.
    let error = parse_query("SELECT sample, asof FROM sales WHERE").unwrap_err();
    assert_that!(error.kind()).is_equal_to(DataErrorKind::QueryParse);
}

#[test]
fn refusals_cut_what_they_quote() {
    let long = format!("coalesce(qty{})", ", qty".repeat(300));
    let in_list = construct(&format!("SELECT id FROM sales WHERE qty IN (1, {long})"));
    assert_that!(in_list.starts_with("IN (… coalesce(")).is_true();
    assert_that!(in_list.chars().count()).is_less_than_or_equal_to(80);
    let name = "f".repeat(500);
    let error = parse_query(&format!("SELECT {name}(1)")).unwrap_err();
    let DataInner::QueryUnknownFunction { function, .. } = &error.0 else { panic!("{error:?}") };
    assert_that!(function.chars().count()).is_less_than_or_equal_to(80);
}

#[rstest]
#[case::case_40(40)]
#[case::case_60(60)]
#[case::case_64(64)]
fn parse_of_brackets_at_15_and_deep_case_returns_on_a_small_stack(#[case] cases: usize) {
    // Brackets just under MAX_SQL_NESTING plus CASE near the parser's recursion limit: the parse
    // ends with a result or a depth refusal, never a stack overflow.
    let sql = format!(
        "SELECT {open}{cases}qty{ends}{close} FROM sales",
        open = "(".repeat(MAX_SQL_NESTING - 1),
        cases = "CASE WHEN qty > 1 THEN ".repeat(cases),
        ends = " END".repeat(cases),
        close = ")".repeat(MAX_SQL_NESTING - 1),
    );
    let result =
        on_small_stack(move || parse_query(&sql).and_then(|q| q.for_engine()).map(|_| ()).map_err(|e| e.kind()));
    if let Err(kind) = result {
        assert_that!([DataErrorKind::QueryTooDeep, DataErrorKind::QueryParse].contains(&kind)).is_true();
    }
}

#[test]
fn engine_sql_holds_only_for_one_whitelisted_query_reading_the_same_inputs() {
    let inputs: BTreeSet<String> = ["sales".to_owned()].into();
    assert_that!(engine_sql_holds("SELECT __n1 FROM sales", &inputs)).is_true();
    assert_that!(engine_sql_holds("SELECT * FROM sales CROSS JOIN read_csv('/etc/passwd') AS z", &inputs)).is_false();
    assert_that!(engine_sql_holds("SELECT * FROM products", &inputs)).is_false();
    assert_that!(engine_sql_holds("SELECT * FROM sales; SELECT 1", &inputs)).is_false();
    assert_that!(engine_sql_holds("SELECT * FROM", &inputs)).is_false();
}

#[rstest]
#[case::at_the_bound(ENGINE_NESTING, true)]
#[case::past_the_bound(ENGINE_NESTING + 1, false)]
fn engine_sql_holds_returns_promptly_on_deep_rewritten_sql_on_a_small_stack(#[case] casts: usize, #[case] holds: bool) {
    // Bracket nesting at the bound plus CASE as deep as a user query reaches: well inside the
    // engine recursion limit, past which sqlparser takes exponential time.
    let sql = format!(
        "SELECT {}{}qty{}{} FROM sales",
        "CAST(".repeat(casts),
        "CASE WHEN qty > 1 THEN ".repeat(PARSER_RECURSION_LIMIT),
        " END".repeat(PARSER_RECURSION_LIMIT),
        " AS DOUBLE) / qty".repeat(casts),
    );
    let inputs: BTreeSet<String> = ["sales".to_owned()].into();
    let started = std::time::Instant::now();
    assert_that!(on_small_stack(move || engine_sql_holds(&sql, &inputs))).is_equal_to(holds);
    assert_that!(started.elapsed()).is_less_than(std::time::Duration::from_secs(5));
}

#[test]
fn for_engine_keeps_a_division_chain_flat() {
    let sql = format!("SELECT {}qty FROM sales", "qty / ".repeat(3));
    assert_that!(parse_query(&sql).unwrap().for_engine().unwrap().sql).is_equal_to(
        "SELECT CAST(qty AS DOUBLE) / nullif(qty, 0) / nullif(qty, 0) / nullif(qty, 0) FROM sales".to_owned(),
    );
}
