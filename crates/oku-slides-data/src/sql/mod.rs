//! SQL front end of query data: parse, whitelist, rename and rewrite; it never executes SQL.
//!
//! [`parse_query`] accepts one `SELECT` of the phase 2 `DuckDB` subset and refuses anything else
//! with a Fluent-localized error naming the construct. [`Query::for_engine`] applies the
//! rewrites that need no input schema; the worker applies the schema-dependent ones (spec,
//! "Phase 2 spike", rows marked (s)).

mod ast;
mod diagnose;
mod functions;
mod rename;
mod rewrite;
mod rewrite_select;
#[cfg(feature = "engine")]
mod typed;
#[cfg(feature = "engine")]
mod typed_expr;
#[cfg(feature = "engine")]
mod typed_pivot;
#[cfg(feature = "engine")]
mod typed_star;
mod whitelist;
mod whitelist_expr;

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use snafu::ensure;
use sqlparser::ast::Statement;
use sqlparser::dialect::DuckDbDialect;
use sqlparser::parser::Parser;

use crate::DataError;
use crate::error::{QueryConstructSnafu, QueryParseSnafu, QueryRewriteSnafu, QueryTooDeepSnafu, QueryTooLongSnafu};
use crate::limits::{MAX_SQL_BYTES, MAX_SQL_NESTING};
pub use functions::functions;
#[cfg(feature = "engine")]
pub(crate) use typed::{Schemas, Ty, typed_sql};
use whitelist::Names;

/// Nesting the parser accepts before it refuses the query as too deep.
const PARSER_RECURSION_LIMIT: usize = 64;

/// Brackets the rewritten SQL may nest: the rewrites add at most a few per user bracket.
const ENGINE_NESTING: usize = 4 * MAX_SQL_NESTING;

/// The parser's recursion limit on the rewritten SQL, kept out of reach: sqlparser backtracks
/// at every level once a limit is met, which takes exponential time near it.
const ENGINE_RECURSION_LIMIT: usize = 4 * PARSER_RECURSION_LIMIT;

/// Version of the whitelist and its rewrites: a change to what a query means bumps it, and with
/// it the key of every query result.
pub const WHITELIST_VERSION: u32 = 1;

/// A parsed query that passed the whitelist; its text is [`Query::to_sql`].
#[derive(Debug, Clone, PartialEq)]
pub struct Query(sqlparser::ast::Query);

/// What the worker runs: the SQL with every schema-free rewrite applied.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct EngineQuery {
    /// The rewritten SQL; the worker still applies the rewrites that need input schemas.
    pub sql: String,
}

/// Parses `sql` and checks it against the whitelist and the rewrites.
///
/// # Errors
///
/// `QueryTooLong` past [`MAX_SQL_BYTES`], `QueryTooDeep` past [`MAX_SQL_NESTING`] brackets or
/// the parser's or the whitelist's depth, `QueryParse` naming the line and column (never the
/// text), `QueryConstruct` naming a refused construct, and
/// `QueryUnknownFunction` naming a function outside the whitelist.
pub fn parse_query(sql: &str) -> Result<Query, DataError> {
    ensure!(sql.len() <= MAX_SQL_BYTES, QueryTooLongSnafu { max: MAX_SQL_BYTES });
    // A text that does not tokenize gets the parser's error, which names its position.
    let nesting = diagnose::bracket_nesting(sql).unwrap_or_default();
    ensure!(nesting <= MAX_SQL_NESTING, QueryTooDeepSnafu);
    let dialect = DuckDbDialect {};
    let parsed = Parser::new(&dialect)
        .with_recursion_limit(PARSER_RECURSION_LIMIT)
        .try_with_sql(sql)
        .and_then(|mut parser| parser.parse_statements());
    let mut statements = parsed.map_err(|error| diagnose::parse_failure(sql, &error))?;
    ensure!(statements.len() < 2, QueryConstructSnafu { construct: ";" });
    let Some(statement) = statements.pop() else {
        return Err(QueryParseSnafu { line: 1_u64, column: 1_u64 }.build().into());
    };
    let Statement::Query(query) = statement else {
        return Err(QueryConstructSnafu { construct: diagnose::first_word(sql) }.build().into());
    };
    whitelist::check(&query, Names::User)?;
    let query = Query(*query);
    // Rewriting at parse time surfaces the forms the rewriter refuses before the data is stored.
    query.for_engine()?;
    Ok(query)
}

impl Query {
    /// The data names the query reads, lowercased; names of its `WITH` tables are not inputs.
    #[must_use]
    pub fn inputs(&self) -> BTreeSet<String> {
        rename::inputs(&self.0)
    }

    /// The lowercased names the query gives its own relations: `WITH` tables and aliases.
    ///
    /// A rename to one of them would be captured: the query would read its own relation.
    #[must_use]
    pub fn bound_names(&self) -> BTreeSet<String> {
        rename::bound_names(&self.0)
    }

    /// The query reading data `new` wherever it read data `old`; columns and `WITH` names keep theirs.
    #[must_use]
    pub fn renamed(&self, old: &str, new: &str) -> Query {
        Query(rename::renamed(&self.0, old, new))
    }

    /// The query's SQL text, normalized by the parser's printer.
    #[must_use]
    pub fn to_sql(&self) -> String {
        self.0.to_string()
    }

    /// The query for the worker, with the rewrites that need no input schema applied.
    ///
    /// # Errors
    ///
    /// `QueryConstruct` when a form the rewrites cannot express is met, and `QueryRewrite`
    /// when the rewritten SQL fails its own check (both also met by [`parse_query`], so a
    /// parsed query never fails here).
    pub fn for_engine(&self) -> Result<EngineQuery, DataError> {
        let sql = rewrite::engine_query(&self.0)?.to_string();
        ensure!(engine_sql_holds(&sql, &self.inputs()), QueryRewriteSnafu);
        Ok(EngineQuery { sql })
    }
}

/// Whether the rewritten `sql` parses back to one query that passes the whitelist and reads
/// exactly `inputs`: the last guard before SQL reaches the engine.
fn engine_sql_holds(sql: &str, inputs: &BTreeSet<String>) -> bool {
    if diagnose::bracket_nesting(sql).is_none_or(|nesting| nesting > ENGINE_NESTING) {
        return false;
    }
    let dialect = DuckDbDialect {};
    let parsed = Parser::new(&dialect)
        .with_recursion_limit(ENGINE_RECURSION_LIMIT)
        .try_with_sql(sql)
        .and_then(|mut parser| parser.parse_statements());
    match parsed.as_deref() {
        Ok([Statement::Query(query)]) => {
            whitelist::check(query, Names::Engine).is_ok() && rename::inputs(query) == *inputs
        }
        _ => false,
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
