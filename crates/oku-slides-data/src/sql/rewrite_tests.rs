use rstest::rstest;
use speculoos::prelude::*;

use super::super::parse_query;
use crate::error::DataInner;

/// The SQL the worker gets for `sql`.
fn engine(sql: &str) -> String {
    parse_query(sql).unwrap().for_engine().unwrap().sql
}

#[rstest]
#[case::order_by_nulls_last(
    "SELECT id FROM sales ORDER BY qty DESC, id",
    "SELECT id FROM sales ORDER BY qty DESC NULLS LAST, id NULLS LAST"
)]
#[case::order_by_all("SELECT id FROM sales ORDER BY ALL", "SELECT id FROM sales ORDER BY ALL NULLS LAST")]
#[case::order_by_kept("SELECT id FROM sales ORDER BY id NULLS FIRST", "SELECT id FROM sales ORDER BY id NULLS FIRST")]
#[case::sum(
    "SELECT SUM(qty) AS t FROM sales",
    "SELECT CASE WHEN count(qty) = 0 THEN NULL ELSE sum(qty) END AS t FROM sales"
)]
#[case::sum_window(
    "SELECT SUM(qty) OVER (PARTITION BY region) FROM sales",
    "SELECT CASE WHEN count(qty) OVER (PARTITION BY region) = 0 THEN NULL ELSE sum(qty) OVER (PARTITION BY region) END FROM sales"
)]
#[case::division("SELECT qty / 2 AS h FROM sales", "SELECT CAST(qty AS DOUBLE) / 2 AS h FROM sales")]
#[case::division_by_column(
    "SELECT price / qty AS r FROM sales",
    "SELECT CAST(price AS DOUBLE) / nullif(qty, 0) AS r FROM sales"
)]
#[case::division_by_zero("SELECT qty / 0 AS h FROM sales", "SELECT CAST(qty AS DOUBLE) / nullif(0, 0) AS h FROM sales")]
#[case::no_from("SELECT 1 + 1 AS two", "SELECT 1 + 1 AS two FROM (VALUES (1)) AS __one (__x)")]
#[case::exponent("SELECT 1e3 AS k, 1.5E-3 AS m", "SELECT 1000.0 AS k, 0.0015 AS m FROM (VALUES (1)) AS __one (__x)")]
#[case::scalar_where(
    "SELECT id FROM sales WHERE price > (SELECT AVG(price) FROM sales)",
    "SELECT id FROM sales CROSS JOIN (SELECT AVG(price) AS __v2 FROM sales) AS __s1 WHERE price > __s1.__v2"
)]
#[case::scalar_having(
    "SELECT region FROM sales GROUP BY region HAVING AVG(price) > (SELECT AVG(price) FROM sales)",
    "SELECT region FROM sales CROSS JOIN (SELECT AVG(price) AS __v2 FROM sales) AS __s1 GROUP BY region \
     HAVING AVG(price) > max(__s1.__v2)"
)]
#[case::scalar_select_star(
    "SELECT *, (SELECT MAX(id) FROM sales) AS m FROM sales",
    "SELECT * EXCLUDE (__v2), __s1.__v2 AS m FROM sales CROSS JOIN (SELECT MAX(id) AS __v2 FROM sales) AS __s1"
)]
#[case::full_using(
    "SELECT product, s.id FROM sales s FULL JOIN products p USING (product)",
    "SELECT coalesce(s.product, p.product) AS product, s.id FROM sales s FULL JOIN products p \
     ON s.product = p.product"
)]
#[case::comma_join(
    "SELECT s.id FROM sales s, products p WHERE s.product = p.product",
    "SELECT s.id FROM sales s CROSS JOIN products p WHERE s.product = p.product"
)]
#[case::inner_non_equality(
    "SELECT s.id FROM sales s JOIN products p ON s.product = p.product AND (s.qty > 2 OR p.qty < 1) WHERE s.id > 0",
    "SELECT s.id FROM sales s JOIN products p ON s.product = p.product WHERE s.id > 0 AND (s.qty > 2 OR p.qty < 1)"
)]
#[case::inner_only_non_equality(
    "SELECT s.id FROM sales s JOIN products p ON s.qty > p.qty",
    "SELECT s.id FROM sales s CROSS JOIN products p WHERE s.qty > p.qty"
)]
#[case::window_null_flag(
    "SELECT id, ROW_NUMBER() OVER (ORDER BY qty DESC) AS rn FROM sales",
    "SELECT id, ROW_NUMBER() OVER (ORDER BY __n1 DESC, qty DESC) AS rn \
     FROM (SELECT *, qty IS NOT NULL AS __n1 FROM sales) AS sales"
)]
#[case::window_null_flag_star(
    "SELECT *, LAG(qty) OVER (ORDER BY id NULLS FIRST) AS p FROM sales AS s",
    "SELECT * EXCLUDE (__n1), LAG(qty) OVER (ORDER BY __n1, id) AS p \
     FROM (SELECT *, id IS NOT NULL AS __n1 FROM sales AS s) AS s"
)]
#[case::running_sum(
    "SELECT SUM(qty) OVER (ORDER BY id ROWS UNBOUNDED PRECEDING) AS r FROM sales",
    "SELECT CASE WHEN count(qty) OVER (ORDER BY __n1, id ROWS UNBOUNDED PRECEDING) = 0 THEN NULL \
     ELSE sum(coalesce(qty, 0)) OVER (ORDER BY __n1, id ROWS UNBOUNDED PRECEDING) END AS r \
     FROM (SELECT *, id IS NULL AS __n1 FROM sales) AS sales"
)]
#[case::running_avg(
    "SELECT AVG(qty) OVER (ORDER BY id ROWS UNBOUNDED PRECEDING) AS r FROM sales",
    "SELECT (CAST(sum(coalesce(qty, 0)) OVER (ORDER BY __n1, id ROWS UNBOUNDED PRECEDING) AS DOUBLE) \
     / nullif(count(qty) OVER (ORDER BY __n1, id ROWS UNBOUNDED PRECEDING), 0)) AS r \
     FROM (SELECT *, id IS NULL AS __n1 FROM sales) AS sales"
)]
#[case::named_window(
    "SELECT MAX(qty) OVER w AS t FROM sales WINDOW w AS (PARTITION BY region)",
    "SELECT MAX(qty) OVER (PARTITION BY region) AS t FROM sales"
)]
#[case::group_by_repeated(
    "SELECT EXTRACT(month FROM day) AS m, COUNT(*) AS n FROM sales GROUP BY EXTRACT(month FROM day)",
    "SELECT EXTRACT(MONTH FROM day) AS m, COUNT(*) AS n FROM sales GROUP BY 1"
)]
#[case::qualify(
    "SELECT id FROM sales QUALIFY ROW_NUMBER() OVER (PARTITION BY region) = 1",
    "SELECT * EXCLUDE (__q1) FROM (SELECT id, ROW_NUMBER() OVER (PARTITION BY region) AS __q1 FROM sales) AS __q \
     WHERE __q1 = 1"
)]
#[case::filter(
    "SELECT MAX(qty) FILTER (WHERE price >= 2) AS s, COUNT(*) FILTER (WHERE qty > 3) AS n FROM sales",
    "SELECT MAX(CASE WHEN price >= 2 THEN qty END) AS s, COUNT(CASE WHEN qty > 3 THEN 1 END) AS n FROM sales"
)]
#[case::filter_count_literal(
    "SELECT COUNT(1) FILTER (WHERE region = 'n') AS c FROM sales",
    "SELECT COUNT(CASE WHEN region = 'n' THEN 1 END) AS c FROM sales"
)]
#[case::filter_sum_literal(
    "SELECT SUM(1) FILTER (WHERE region = 'n') AS s FROM sales",
    "SELECT CASE WHEN count(CASE WHEN region = 'n' THEN 1 END) = 0 THEN NULL \
     ELSE sum(CASE WHEN region = 'n' THEN 1 END) END AS s FROM sales"
)]
#[case::filter_quantile_keeps_fraction(
    "SELECT quantile_cont(qty, 0.5) FILTER (WHERE qty > 1) AS q FROM sales",
    "SELECT quantile_cont(CASE WHEN qty > 1 THEN qty END, 0.5) AS q FROM sales"
)]
#[case::contains("SELECT contains(region, 'or') FROM sales", "SELECT (strpos(region, 'or') > 0) FROM sales")]
#[case::renamed_calls(
    "SELECT lcase(r), ucase(r), prefix(r, 'a'), suffix(r, 'b'), regexp_matches(r, 'c'), log(q), mean(q) FROM t",
    "SELECT lower(r), upper(r), starts_with(r, 'a'), ends_with(r, 'b'), regexp_like(r, 'c'), log10(q), avg(q) FROM t"
)]
#[case::date_parts(
    "SELECT year(d), dayofweek(d), dayofmonth(d), dayofyear(d), isoyear(d), datepart('quarter', d), date_part('month', d) FROM t",
    "SELECT EXTRACT(YEAR FROM d), EXTRACT(DOW FROM d), EXTRACT(DAY FROM d), EXTRACT(DOY FROM d), \
     EXTRACT(ISOYEAR FROM d), EXTRACT(QUARTER FROM d), date_part('month', d) FROM t"
)]
#[case::day_month_names("SELECT dayname(d), monthname(d) FROM t", "SELECT strftime(d, '%A'), strftime(d, '%B') FROM t")]
#[case::date_trunc(
    "SELECT date_trunc('year', d), date_trunc('month', d) FROM t",
    "SELECT date(strftime(d, '%Y-01-01')), date(strftime(d, '%Y-%m-01')) FROM t"
)]
#[case::interval(
    "SELECT d + INTERVAL 1 MONTH, d - INTERVAL 3 DAY FROM t",
    "SELECT d + INTERVAL '1 month', d - INTERVAL '3 days' FROM t"
)]
#[case::date_add("SELECT date_add(d, INTERVAL 2 YEAR) FROM t", "SELECT (d + INTERVAL '2 years') FROM t")]
#[case::date_diff(
    "SELECT date_diff('day', a, b), datediff('month', a, b), date_diff('year', a, b) FROM t",
    "SELECT (CAST(b AS INTEGER) - CAST(a AS INTEGER)), \
     ((EXTRACT(YEAR FROM b) - EXTRACT(YEAR FROM a)) * 12 + (EXTRACT(MONTH FROM b) - EXTRACT(MONTH FROM a))), \
     (EXTRACT(YEAR FROM b) - EXTRACT(YEAR FROM a)) FROM t"
)]
#[case::last_day(
    "SELECT last_day(d) FROM t",
    "SELECT (date(strftime(d, '%Y-%m-01')) + INTERVAL '1 month' - INTERVAL '1 day') FROM t"
)]
#[case::make_date("SELECT make_date(y, m, 1) FROM t", "SELECT date(concat(y, '-', m, '-', 1), '%Y-%m-%d') FROM t")]
#[case::negative_substring("SELECT substr(r, -2) FROM t", "SELECT right(r, 2) FROM t")]
fn for_engine_applies_the_rewrite(#[case] sql: &str, #[case] expected: &str) {
    assert_that!(engine(sql)).is_equal_to(expected.to_owned());
}

#[test]
fn for_engine_rewrites_in_one_pass_never_its_own_output() {
    // The SUM rule copies its argument: the `/` rule ran once on it, before the copy.
    assert_that!(engine("SELECT SUM(qty / 2) AS t FROM sales")).is_equal_to(
        "SELECT CASE WHEN count(CAST(qty AS DOUBLE) / 2) = 0 THEN NULL ELSE sum(CAST(qty AS DOUBLE) / 2) END AS t \
         FROM sales"
            .to_owned(),
    );
    // The running AVG rule emits a `/` and a SUM: neither is rewritten again.
    let running = engine("SELECT AVG(qty) OVER (ORDER BY 1 ROWS UNBOUNDED PRECEDING) FROM sales");
    assert_that!(running.matches("CAST(").count()).is_equal_to(1);
    assert_that!(running.contains("CASE")).is_false();
}

#[test]
fn for_engine_leaves_the_schema_rewrites_to_the_worker() {
    let sql = "SELECT qty % 3, mod(qty, 3), qty // 2, round(price), CAST(price AS INTEGER), d + 1, d - d \
               FROM sales WHERE x > 2";
    assert_that!(engine(sql)).is_equal_to(
        "SELECT qty % 3, mod(qty, 3), qty // 2, round(price), CAST(price AS INTEGER), d + 1, d - d FROM sales WHERE x > 2"
            .to_owned(),
    );
}

#[test]
fn for_engine_leaves_a_pivot_aggregate_to_the_worker() {
    let sql = "SELECT * FROM (SELECT region, qty FROM sales) AS s PIVOT (SUM(qty / 2) FOR region IN ('n', 's'))";
    assert_that!(engine(sql)).is_equal_to(
        "SELECT * FROM (SELECT region, qty FROM sales) AS s PIVOT(SUM(qty / 2) FOR region IN ('n', 's'))".to_owned(),
    );
}

#[test]
fn for_engine_rewrites_inside_nested_queries() {
    assert_that!(engine("SELECT t.h FROM (SELECT qty / 2 AS h FROM sales) AS t"))
        .is_equal_to("SELECT t.h FROM (SELECT CAST(qty AS DOUBLE) / 2 AS h FROM sales) AS t".to_owned());
}

#[rstest]
#[case::exponent_overflow("SELECT 1e400", "1e400")]
#[case::right_join_after_comma("SELECT 1 FROM a, b RIGHT JOIN c ON b.x = c.x", ", … RIGHT JOIN")]
#[case::moved_term_before_full("SELECT 1 FROM a JOIN b ON a.x > b.x FULL JOIN c ON a.y = c.y", "JOIN … ON a.x > b.x")]
#[case::full_using_star("SELECT * FROM a FULL JOIN b USING (k)", "* … USING")]
#[case::full_using_unnamed("SELECT k FROM (SELECT 1 AS k) FULL JOIN b USING (k)", "FULL JOIN … USING")]
#[case::grouped_window_order(
    "SELECT region, ROW_NUMBER() OVER (ORDER BY region) FROM sales GROUP BY region",
    "GROUP BY … OVER (ORDER BY …)"
)]
#[case::join_unqualified_window_key(
    "SELECT ROW_NUMBER() OVER (ORDER BY qty) FROM sales s JOIN products p ON s.id = p.id",
    "OVER (ORDER BY qty)"
)]
fn parse_refuses_what_the_rewrites_cannot_express(#[case] sql: &str, #[case] named: &str) {
    let error = parse_query(sql).unwrap_err();
    let DataInner::QueryConstruct { construct, .. } = &error.0 else { panic!("{error:?}") };
    assert_that!(construct.as_str()).is_equal_to(named);
}

#[rstest]
#[case::flag_injection(
    "SELECT ROW_NUMBER() OVER (ORDER BY qty) AS r FROM sales AS \"s CROSS JOIN read_csv('/etc/passwd') AS z\"",
    "SELECT ROW_NUMBER() OVER (ORDER BY __n1, qty) AS r FROM (SELECT *, qty IS NULL AS __n1 \
     FROM sales AS \"s CROSS JOIN read_csv('/etc/passwd') AS z\") AS \"s CROSS JOIN read_csv('/etc/passwd') AS z\""
)]
#[case::flag_mixed_case(
    "SELECT ROW_NUMBER() OVER (ORDER BY \"S\".qty) AS r FROM sales AS \"S\" JOIN products AS p ON \"S\".id = p.id",
    "SELECT ROW_NUMBER() OVER (ORDER BY __n1, \"S\".qty) AS r FROM (SELECT *, \"S\".qty IS NULL AS __n1 \
     FROM sales AS \"S\") AS \"S\" JOIN products AS p ON \"S\".id = p.id"
)]
#[case::flag_keyword(
    "SELECT ROW_NUMBER() OVER (ORDER BY qty) AS r FROM sales AS \"select\"",
    "SELECT ROW_NUMBER() OVER (ORDER BY __n1, qty) AS r FROM (SELECT *, qty IS NULL AS __n1 \
     FROM sales AS \"select\") AS \"select\""
)]
#[case::flag_quote(
    "SELECT ROW_NUMBER() OVER (ORDER BY qty) AS r FROM sales AS \"a\"\"b\"",
    "SELECT ROW_NUMBER() OVER (ORDER BY __n1, qty) AS r FROM (SELECT *, qty IS NULL AS __n1 \
     FROM sales AS \"a\"\"b\") AS \"a\"\"b\""
)]
#[case::using_space_and_keyword(
    "SELECT product FROM sales AS \"s s\" FULL JOIN products AS \"select\" USING (product)",
    "SELECT coalesce(\"s s\".product, \"select\".product) AS product FROM sales AS \"s s\" \
     FULL JOIN products AS \"select\" ON \"s s\".product = \"select\".product"
)]
#[case::using_injection(
    "SELECT product FROM sales AS \"x FULL JOIN read_csv('f') AS y\" FULL JOIN products AS P USING (product)",
    "SELECT coalesce(\"x FULL JOIN read_csv('f') AS y\".product, P.product) AS product \
     FROM sales AS \"x FULL JOIN read_csv('f') AS y\" FULL JOIN products AS P \
     ON \"x FULL JOIN read_csv('f') AS y\".product = P.product"
)]
fn for_engine_keeps_quoted_aliases_quoted(#[case] sql: &str, #[case] expected: &str) {
    // The alias text stays one quoted identifier: for_engine's own check would refuse a read_csv.
    assert_that!(engine(sql)).is_equal_to(expected.to_owned());
}
