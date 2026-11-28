//! The rewrites that need input schemas (spec, "Phase 2 spike", rows marked (s)), run by the
//! worker where the inputs are known: integer `%`, `mod` and `//`, `round`, integer `CAST`,
//! date arithmetic, `*` over a join, a select alias in `WHERE`, `PIVOT` and `UNPIVOT`.
//!
//! One manual walk: `WITH` tables first, then each select's factors, its `*`, its `WHERE`
//! aliases and its expressions, post-order, so a rule never meets its own output. Types come
//! from the engine's planner ([`Schemas`]), one probe per operand a rule needs.

use sqlparser::ast::{
    Cte, Expr, GroupByExpr, Ident, OrderByKind, Query, Select, SelectItem, SetExpr, TableFactor, TableWithJoins, With,
};
use sqlparser::dialect::DuckDbDialect;
use sqlparser::parser::Parser;
use sqlparser::tokenizer::Token;

use super::ast::{alias, children_mut, ident, query_of, select, select_from};
use super::{ENGINE_RECURSION_LIMIT, engine_sql_holds, rename, typed_expr, typed_pivot, typed_star};
use crate::DataError;
use crate::error::QueryRewriteSnafu;
use crate::limits::MAX_SQL_BYTES;

/// Largest SQL a rule may write for one node: past it, the query is refused as too large.
pub(super) const MAX_RULE_BYTES: usize = 8 * MAX_SQL_BYTES;

/// A column's type, as the rules read it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Ty {
    /// A signed or unsigned integer.
    Int,
    /// A float.
    Float,
    /// Text.
    Text,
    /// A boolean.
    Bool,
    /// A date.
    Date,
    /// The type of a bare `NULL`.
    Null,
    /// Any other type.
    Other,
}

/// The engine's planner: the columns a query gives, never running it.
pub(crate) trait Schemas {
    /// The names and types of the columns `sql` gives.
    ///
    /// # Errors
    ///
    /// The engine's refusal of `sql`.
    fn columns(&mut self, sql: &str) -> Result<Vec<(String, Ty)>, DataError>;
}

/// `sql`, an engine query, with the schema rewrites applied.
///
/// # Errors
///
/// `QueryFloatOperand` for a float operand of `%`, `mod`, `//` or `round(x, n)`,
/// `QueryConstruct` for a form the rules cannot write, `QueryRewrite` when the result fails its
/// own check, and the engine's refusal of an operand or a relation whose type or columns a rule
/// needs (a column not found).
pub(crate) fn typed_sql(sql: &str, schemas: &mut dyn Schemas) -> Result<String, DataError> {
    let mut query = parse(sql).ok_or_else(|| QueryRewriteSnafu.build())?;
    let inputs = rename::inputs(&query);
    Walk { schemas, ctes: Vec::new(), pivots: 0 }.query(&mut query)?;
    let out = query.to_string();
    snafu::ensure!(engine_sql_holds(&out, &inputs), QueryRewriteSnafu);
    Ok(out)
}

fn parse(sql: &str) -> Option<Query> {
    let dialect = DuckDbDialect {};
    let mut parser = Parser::new(&dialect).with_recursion_limit(ENGINE_RECURSION_LIMIT).try_with_sql(sql).ok()?;
    let query = parser.parse_query().ok()?;
    (parser.peek_token().token == Token::EOF).then_some(*query)
}

/// Where an expression is evaluated: the relations of its select and its grouping, and for an
/// `ORDER BY` the query's own output, whose names (a select alias, a union's columns) it may read.
#[derive(Debug, Clone, Default)]
pub(super) struct Scope {
    pub from: Vec<TableWithJoins>,
    pub group_by: Option<GroupByExpr>,
    pub output: Option<TableWithJoins>,
}

impl Scope {
    fn of(select: &Select) -> Self {
        Self { from: select.from.clone(), group_by: Some(select.group_by.clone()), output: None }
    }

    /// The scope of an `ORDER BY` over `body`, already rewritten.
    fn order_by(body: &SetExpr) -> Self {
        let mut scope = match body {
            SetExpr::Select(select) => Self::of(select),
            _ => Self::default(),
        };
        let relation = TableFactor::Derived {
            lateral: false,
            subquery: Box::new(query_of(body.clone())),
            alias: Some(alias(ident("__o"), &[])),
        };
        scope.output = Some(TableWithJoins { relation, joins: vec![] });
        scope
    }
}

/// The walk's state: the `WITH` tables in scope, as rewritten, and the PIVOT count.
pub(super) struct Walk<'s> {
    pub schemas: &'s mut dyn Schemas,
    ctes: Vec<Cte>,
    pub pivots: usize,
}

impl Walk<'_> {
    pub(super) fn query(&mut self, query: &mut Query) -> Result<(), DataError> {
        let visible = self.ctes.len();
        if let Some(with) = &mut query.with {
            for cte in &mut with.cte_tables {
                self.query(&mut cte.query)?;
                self.ctes.push(cte.clone());
            }
        }
        let walked = self.body(&mut query.body).and_then(|()| {
            let Some(order_by) = &mut query.order_by else { return Ok(()) };
            let OrderByKind::Expressions(keys) = &mut order_by.kind else { return Ok(()) };
            let scope = Scope::order_by(&query.body);
            keys.iter_mut().try_for_each(|key| self.expr(&mut key.expr, &scope))
        });
        self.ctes.truncate(visible);
        walked
    }

    fn body(&mut self, body: &mut SetExpr) -> Result<(), DataError> {
        match body {
            SetExpr::Select(select) => self.select(select),
            SetExpr::Query(query) => self.query(query),
            SetExpr::SetOperation { left, right, .. } => {
                self.body(left)?;
                self.body(right)
            }
            SetExpr::Values(values) => {
                let scope = Scope::default();
                values.rows.iter_mut().flatten().try_for_each(|expr| self.expr(expr, &scope))
            }
            _ => Ok(()),
        }
    }

    fn select(&mut self, select: &mut Select) -> Result<(), DataError> {
        for table in &mut select.from {
            self.factor(&mut table.relation)?;
            for join in &mut table.joins {
                self.factor(&mut join.relation)?;
            }
        }
        typed_star::expand(self, select)?;
        typed_star::where_aliases(self, select)?;
        let scope = Scope::of(select);
        for item in &mut select.projection {
            if let SelectItem::UnnamedExpr(expr) | SelectItem::ExprWithAlias { expr, .. } = item {
                self.expr(expr, &scope)?;
            }
        }
        let clauses = select.selection.iter_mut().chain(&mut select.having).chain(&mut select.qualify);
        for expr in clauses {
            self.expr(expr, &scope)?;
        }
        if let GroupByExpr::Expressions(exprs, _) = &mut select.group_by {
            exprs.iter_mut().try_for_each(|expr| self.expr(expr, &scope))?;
        }
        for join in select.from.iter_mut().flat_map(|table| &mut table.joins) {
            if let Some(sqlparser::ast::JoinConstraint::On(condition)) = join_constraint(&mut join.join_operator) {
                self.expr(condition, &scope)?;
            }
        }
        Ok(())
    }

    fn factor(&mut self, factor: &mut TableFactor) -> Result<(), DataError> {
        match factor {
            TableFactor::Derived { subquery, .. } => self.query(subquery),
            TableFactor::Pivot { table, .. } | TableFactor::Unpivot { table, .. } => {
                self.factor(table)?;
                let derived = typed_pivot::derived(self, factor)?;
                *factor = derived;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Applies the expression rules to `expr`, post-order; subqueries are walked as queries.
    pub(super) fn expr(&mut self, expr: &mut Expr, scope: &Scope) -> Result<(), DataError> {
        match expr {
            Expr::Subquery(query) | Expr::Exists { subquery: query, .. } => return self.query(query),
            Expr::InSubquery { subquery, .. } => self.query(subquery)?,
            _ => {}
        }
        for child in children_mut(expr) {
            self.expr(child, scope)?;
        }
        if let Some(new) = typed_expr::rule(self, expr, scope)? {
            *expr = new;
        }
        Ok(())
    }

    /// The type of `expr` in `scope`: probed alone, then with the select's grouping, then over
    /// the query's output.
    ///
    /// # Errors
    ///
    /// The engine's refusal of the first probe when none gives a type: a column not found.
    pub(super) fn ty(&mut self, expr: &Expr, scope: &Scope) -> Result<Ty, DataError> {
        if let Some(ty) = typed_expr::literal_ty(expr) {
            return Ok(ty);
        }
        let item = SelectItem::ExprWithAlias { expr: expr.clone(), alias: ident("__t") };
        let mut tries = vec![(scope.from.clone(), None)];
        if scope.group_by.as_ref().is_some_and(|g| !matches!(g, GroupByExpr::Expressions(e, _) if e.is_empty())) {
            tries.push((scope.from.clone(), scope.group_by.clone()));
        }
        tries.extend(scope.output.iter().map(|output| (vec![output.clone()], None)));
        let mut first = None;
        for (from, group_by) in tries {
            let mut probe = select_from(vec![item.clone()], from);
            if let Some(group_by) = group_by {
                probe.group_by = group_by;
            }
            match self.columns(SetExpr::Select(Box::new(probe))) {
                Ok(columns) => match columns.as_slice() {
                    [(_, ty)] => return Ok(*ty),
                    _ => return Err(QueryRewriteSnafu.build().into()),
                },
                Err(error) => {
                    first.get_or_insert(error);
                }
            }
        }
        Err(first.unwrap_or_else(|| QueryRewriteSnafu.build().into()))
    }

    /// The columns of `SELECT * FROM factor`, with the `WITH` tables in scope.
    pub(super) fn factor_columns(&mut self, factor: &TableFactor) -> Result<Vec<(String, Ty)>, DataError> {
        let star = SelectItem::Wildcard(sqlparser::ast::WildcardAdditionalOptions::default());
        self.columns(SetExpr::Select(Box::new(select(vec![star], factor.clone()))))
    }

    /// The columns of `SELECT * FROM from`, with the `WITH` tables in scope.
    pub(super) fn relation_columns(&mut self, from: &[TableWithJoins]) -> Result<Vec<(String, Ty)>, DataError> {
        let star = SelectItem::Wildcard(sqlparser::ast::WildcardAdditionalOptions::default());
        self.columns(SetExpr::Select(Box::new(select_from(vec![star], from.to_vec()))))
    }

    fn columns(&mut self, body: SetExpr) -> Result<Vec<(String, Ty)>, DataError> {
        let mut probe = query_of(body);
        if !self.ctes.is_empty() {
            probe.with = Some(With {
                with_token: sqlparser::ast::helpers::attached_token::AttachedToken::empty(),
                recursive: false,
                cte_tables: self.ctes.clone(),
            });
        }
        self.schemas.columns(&probe.to_string())
    }
}

/// The constraint of a join, when it has one.
fn join_constraint(operator: &mut sqlparser::ast::JoinOperator) -> Option<&mut sqlparser::ast::JoinConstraint> {
    use sqlparser::ast::JoinOperator as J;
    match operator {
        J::Join(c)
        | J::Inner(c)
        | J::Left(c)
        | J::LeftOuter(c)
        | J::Right(c)
        | J::RightOuter(c)
        | J::FullOuter(c)
        | J::CrossJoin(c) => Some(c),
        _ => None,
    }
}

/// `rule` named `construct` when its output is past [`MAX_RULE_BYTES`].
pub(super) fn bounded(construct: &str, expr: Expr) -> Result<Expr, DataError> {
    if expr.to_string().len() > MAX_RULE_BYTES {
        return Err(super::whitelist::refuse(construct));
    }
    Ok(expr)
}

/// A quoted identifier, for a column name as the engine gives it.
pub(super) fn quoted(name: &str) -> Ident {
    Ident::with_quote('"', name)
}

#[cfg(test)]
#[path = "typed_tests.rs"]
mod typed_tests;
