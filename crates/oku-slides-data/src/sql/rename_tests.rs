use speculoos::prelude::*;

use super::super::parse_query;

fn inputs(sql: &str) -> Vec<String> {
    parse_query(sql).unwrap().inputs().into_iter().collect()
}

fn renamed(sql: &str, old: &str, new: &str) -> String {
    parse_query(sql).unwrap().renamed(old, new).to_sql()
}

#[test]
fn inputs_are_lowercased_table_names_not_cte_names() {
    let sql = "WITH a AS (SELECT * FROM Sales), b AS (SELECT * FROM a JOIN PRODUCTS p ON a.id = p.id) \
               SELECT * FROM b WHERE id IN (SELECT id FROM returns)";
    assert_that!(inputs(sql)).is_equal_to(vec!["products".to_owned(), "returns".to_owned(), "sales".to_owned()]);
}

#[test]
fn inputs_count_a_table_named_like_a_later_cte_inside_an_earlier_one() {
    // `a` reads the data `b`: the WITH table `b` is defined after it.
    let sql = "WITH a AS (SELECT * FROM b), b AS (SELECT * FROM a) SELECT * FROM b";
    assert_that!(inputs(sql)).is_equal_to(vec!["b".to_owned()]);
}

#[test]
fn inputs_of_a_cte_shadowed_in_a_subquery_are_not_inputs() {
    let sql = "SELECT * FROM (WITH sales AS (SELECT 1 AS id) SELECT id FROM sales) AS t";
    assert_that!(inputs(sql)).is_empty();
}

#[test]
fn renamed_rewrites_tables_only_not_columns_or_ctes_of_same_name() {
    let sql = "WITH sales_cte AS (SELECT sales FROM sales) \
               SELECT sales.sales, x.sales FROM sales JOIN sales_cte AS x ON sales.sales = x.sales";
    assert_that!(renamed(sql, "Sales", "orders")).is_equal_to(
        "WITH sales_cte AS (SELECT sales FROM orders) \
         SELECT orders.sales, x.sales FROM orders JOIN sales_cte AS x ON orders.sales = x.sales"
            .to_owned(),
    );
}

#[test]
fn renamed_leaves_a_cte_of_the_old_name_and_its_readers() {
    let sql = "WITH sales AS (SELECT * FROM sales) SELECT sales.id FROM sales";
    assert_that!(renamed(sql, "sales", "orders"))
        .is_equal_to("WITH sales AS (SELECT * FROM orders) SELECT sales.id FROM sales".to_owned());
}

#[test]
fn renamed_keeps_an_alias_and_renames_a_qualified_wildcard() {
    assert_that!(renamed("SELECT s.id FROM sales AS s", "sales", "orders"))
        .is_equal_to("SELECT s.id FROM orders AS s".to_owned());
    assert_that!(renamed("SELECT sales.* FROM sales", "sales", "orders"))
        .is_equal_to("SELECT orders.* FROM orders".to_owned());
}

#[test]
fn renamed_quotes_a_keyword_or_non_word_name() {
    assert_that!(renamed("SELECT id FROM sales", "sales", "select"))
        .is_equal_to("SELECT id FROM \"select\"".to_owned());
    assert_that!(renamed("SELECT id FROM sales", "sales", "my sales"))
        .is_equal_to("SELECT id FROM \"my sales\"".to_owned());
}

#[test]
fn to_sql_of_renamed_parses_back_equal() {
    let sql = "WITH a AS (SELECT region, SUM(qty) AS t FROM sales GROUP BY region) \
               SELECT a.region, sales.qty FROM a JOIN sales ON a.region = sales.region \
               WHERE sales.qty > (SELECT AVG(qty) FROM sales) ORDER BY 1";
    for new in ["orders", "order", "my orders"] {
        let renamed = parse_query(sql).unwrap().renamed("sales", new);
        let reparsed = parse_query(&renamed.to_sql()).unwrap();
        assert_that!(reparsed).is_equal_to(&renamed);
        assert_that!(reparsed.inputs().contains(new)).is_true();
    }
}

#[test]
fn renamed_qualifiers_per_select_not_per_set_operation() {
    // The first branch's `sales` is an alias of `other`: only the second branch reads the data.
    let sql = "SELECT sales.x FROM other AS sales UNION SELECT sales.x FROM sales";
    assert_that!(renamed(sql, "sales", "sold"))
        .is_equal_to("SELECT sales.x FROM other AS sales UNION SELECT sold.x FROM sold".to_owned());
}

#[test]
fn renamed_qualifiers_in_the_order_by_of_a_single_select() {
    assert_that!(renamed("SELECT sales.x FROM sales ORDER BY sales.x", "sales", "sold"))
        .is_equal_to("SELECT sold.x FROM sold ORDER BY sold.x".to_owned());
}

#[test]
fn bound_names_are_with_tables_and_relation_aliases_lowercased() {
    let sql = "WITH Ventes AS (SELECT 1 AS x) SELECT * FROM sales AS S JOIN ventes ON s.id = ventes.x \
               WHERE id IN (SELECT id FROM (SELECT id FROM returns) AS Inner_t)";
    let names: Vec<String> = parse_query(sql).unwrap().bound_names().into_iter().collect();
    assert_that!(names).is_equal_to(vec!["inner_t".to_owned(), "s".to_owned(), "ventes".to_owned()]);
}

#[test]
fn bound_names_of_a_query_without_aliases_are_empty() {
    assert_that!(parse_query("SELECT * FROM sales").unwrap().bound_names().is_empty()).is_true();
}
