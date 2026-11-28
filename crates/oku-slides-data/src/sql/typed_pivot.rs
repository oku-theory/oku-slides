//! `PIVOT` and `UNPIVOT` as the derived tables polars runs: one conditional aggregate per value
//! grouped by the other columns, and one `UNION ALL` branch per column.

use std::collections::BTreeSet;

use sqlparser::ast::{
    BinaryOperator, CaseWhen, DataType, ExactNumberInfo, Expr, ExprWithAlias, FunctionArg, FunctionArgExpr,
    FunctionArguments, GroupByExpr, Ident, NullInclusion, PivotValueSource, SelectItem, SetExpr, SetOperator,
    SetQuantifier, TableFactor, Value,
};

use super::ast::{alias, binary, cast, each, number, query_of, select, string};
use super::typed::{Scope, Ty, Walk, bounded, quoted};
use super::whitelist::{refuse, relation_ident};
use crate::DataError;

/// The derived table a `PIVOT` or `UNPIVOT` factor is, its source already walked.
pub(super) fn derived(walk: &mut Walk<'_>, factor: &TableFactor) -> Result<TableFactor, DataError> {
    walk.pivots += 1;
    let fallback = format!("__p{}", walk.pivots);
    let n = walk.pivots;
    let source = |table: &TableFactor| named_source(table, n);
    match factor {
        TableFactor::Pivot { table, aggregate_functions, value_column, value_source, default_on_null, alias } => {
            if default_on_null.is_some() || alias.as_ref().is_some_and(|a| !a.columns.is_empty()) {
                return Err(refuse("PIVOT"));
            }
            let table = source(table);
            let body = pivot(walk, &table, aggregate_functions, value_column, value_source)?;
            Ok(named(body, alias.as_ref().map(|a| a.name.clone()), &table, &fallback))
        }
        TableFactor::Unpivot { table, value, name, columns, null_inclusion, alias } => {
            if alias.as_ref().is_some_and(|a| !a.columns.is_empty()) {
                return Err(refuse("UNPIVOT"));
            }
            let nulls = matches!(null_inclusion, Some(NullInclusion::IncludeNulls));
            let table = source(table);
            let body = unpivot(walk, &table, value, name, columns, nulls)?;
            Ok(named(body, alias.as_ref().map(|a| a.name.clone()), &table, &fallback))
        }
        _ => Err(refuse(factor.to_string())),
    }
}

/// The source of a `PIVOT` or `UNPIVOT`, a subquery named `__s<n>` when unnamed, as polars
/// requires.
fn named_source(table: &TableFactor, n: usize) -> TableFactor {
    match table {
        TableFactor::Derived { lateral, subquery, alias: None } => TableFactor::Derived {
            lateral: *lateral,
            subquery: subquery.clone(),
            alias: Some(alias(Ident::new(format!("__s{n}")), &[])),
        },
        other => other.clone(),
    }
}

/// `(body) AS name`: the factor's alias, else its source's table name, else a fresh name.
fn named(body: SetExpr, given: Option<Ident>, source: &TableFactor, fallback: &str) -> TableFactor {
    let plain = match source {
        TableFactor::Table { alias: None, .. } => relation_ident(source).cloned(),
        _ => None,
    };
    let name = given.or(plain).unwrap_or_else(|| Ident::new(fallback));
    super::ast::derived(query_of(body), alias(name, &[]))
}

fn single_column(expr: &Expr, construct: &str) -> Result<String, DataError> {
    match expr {
        Expr::Identifier(name) => Ok(name.value.clone()),
        _ => Err(refuse(construct)),
    }
}

fn pivot(
    walk: &mut Walk<'_>,
    source: &TableFactor,
    aggregates: &[ExprWithAlias],
    value_column: &[Expr],
    values: &PivotValueSource,
) -> Result<SetExpr, DataError> {
    let ([column], PivotValueSource::List(values)) = (value_column, values) else { return Err(refuse("PIVOT")) };
    let column = single_column(column, "PIVOT")?;
    if aggregates.len() > 1 && aggregates.iter().any(|a| a.alias.is_none()) {
        return Err(refuse("PIVOT"));
    }
    let mut read: BTreeSet<String> = BTreeSet::from([column.to_ascii_lowercase()]);
    for aggregate in aggregates {
        each(&aggregate.expr, &mut |expr| {
            if let Expr::Identifier(name) = expr {
                read.insert(name.value.to_ascii_lowercase());
            }
        });
    }
    let others: Vec<Expr> = walk
        .factor_columns(source)?
        .into_iter()
        .filter(|(name, _)| !read.contains(&name.to_ascii_lowercase()))
        .map(|(name, _)| Expr::Identifier(quoted(&name)))
        .collect();
    let mut projection: Vec<SelectItem> = others.iter().cloned().map(SelectItem::UnnamedExpr).collect();
    let scope = Scope {
        from: vec![sqlparser::ast::TableWithJoins { relation: source.clone(), joins: vec![] }],
        group_by: None,
        output: None,
    };
    for value in values {
        let label = match &value.alias {
            Some(alias) => alias.value.clone(),
            None => literal_text(&value.expr).ok_or_else(|| refuse("PIVOT"))?,
        };
        let matches = binary(Expr::Identifier(Ident::new(&column)), BinaryOperator::Eq, value.expr.clone());
        for aggregate in aggregates {
            let name = match &aggregate.alias {
                Some(alias) => format!("{label}_{}", alias.value),
                None => label.clone(),
            };
            let mut expr = conditional(&aggregate.expr, &matches)?;
            super::rewrite::engine_expr(&mut expr)?;
            walk.expr(&mut expr, &scope)?;
            projection.push(SelectItem::ExprWithAlias { expr: bounded("PIVOT", expr)?, alias: quoted(&name) });
        }
    }
    let mut body = select(projection, source.clone());
    if !others.is_empty() {
        body.group_by = GroupByExpr::Expressions(others, vec![]);
    }
    Ok(SetExpr::Select(Box::new(body)))
}

/// The text a `PIVOT` value names its column with.
fn literal_text(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Value(value) => match &value.value {
            Value::SingleQuotedString(text) | Value::Number(text, _) => Some(text.clone()),
            Value::Boolean(b) => Some(b.to_string()),
            _ => None,
        },
        _ => None,
    }
}

/// `CASE WHEN matches THEN x END`.
fn when(matches: &Expr, x: Expr) -> Expr {
    Expr::Case {
        case_token: sqlparser::ast::helpers::attached_token::AttachedToken::empty(),
        end_token: sqlparser::ast::helpers::attached_token::AttachedToken::empty(),
        operand: None,
        conditions: vec![CaseWhen { condition: matches.clone(), result: x }],
        else_result: None,
    }
}

/// The aggregate over the rows `matches` keeps: a count is 0 and others NULL when it keeps none.
fn conditional(aggregate: &Expr, matches: &Expr) -> Result<Expr, DataError> {
    let Expr::Function(function) = aggregate else { return Err(refuse("PIVOT")) };
    let mut function = function.clone();
    if function.over.is_some() || function.filter.is_some() {
        return Err(refuse("PIVOT"));
    }
    let FunctionArguments::List(list) = &mut function.args else { return Err(refuse("PIVOT")) };
    for arg in &mut list.args {
        let FunctionArg::Unnamed(value) = arg else { return Err(refuse("PIVOT")) };
        let x = match value {
            FunctionArgExpr::Expr(expr) => expr.clone(),
            FunctionArgExpr::Wildcard | FunctionArgExpr::QualifiedWildcard(_) => number("1"),
        };
        *value = FunctionArgExpr::Expr(when(matches, x));
    }
    Ok(Expr::Function(function))
}

fn unpivot(
    walk: &mut Walk<'_>,
    source: &TableFactor,
    value: &Expr,
    name: &Ident,
    columns: &[ExprWithAlias],
    nulls: bool,
) -> Result<SetExpr, DataError> {
    let value = single_column(value, "UNPIVOT")?;
    let all = walk.factor_columns(source)?;
    let mut unpivoted = Vec::new();
    for column in columns {
        let written = single_column(&column.expr, "UNPIVOT")?;
        let (actual, ty) =
            all.iter().find(|(c, _)| c.eq_ignore_ascii_case(&written)).cloned().ok_or_else(|| refuse("UNPIVOT"))?;
        let label = column.alias.as_ref().map_or_else(|| actual.clone(), |alias| alias.value.clone());
        unpivoted.push((actual, ty, label));
    }
    let types: BTreeSet<_> = unpivoted.iter().map(|(_, ty, _)| *ty).filter(|ty| *ty != Ty::Null).collect();
    let widen = types == BTreeSet::from([Ty::Int, Ty::Float]);
    if types.len() > 1 && !widen {
        return Err(refuse("UNPIVOT"));
    }
    let read: BTreeSet<String> = unpivoted.iter().map(|(c, _, _)| c.to_ascii_lowercase()).collect();
    let others: Vec<SelectItem> = all
        .iter()
        .filter(|(c, _)| !read.contains(&c.to_ascii_lowercase()))
        .map(|(c, _)| SelectItem::UnnamedExpr(Expr::Identifier(quoted(c))))
        .collect();
    let mut body: Option<SetExpr> = None;
    for (column, ty, label) in unpivoted {
        let read = Expr::Identifier(quoted(&column));
        let cell = if widen && ty != Ty::Float {
            cast(read.clone(), DataType::Double(ExactNumberInfo::None))
        } else {
            read.clone()
        };
        let mut projection = others.clone();
        projection.push(SelectItem::ExprWithAlias { expr: string(&label), alias: quoted(&name.value) });
        projection.push(SelectItem::ExprWithAlias { expr: cell, alias: quoted(&value) });
        let mut branch = select(projection, source.clone());
        if !nulls {
            branch.selection = Some(Expr::IsNotNull(Box::new(read)));
        }
        let branch = SetExpr::Select(Box::new(branch));
        body = Some(match body {
            None => branch,
            Some(left) => SetExpr::SetOperation {
                op: SetOperator::Union,
                set_quantifier: SetQuantifier::All,
                left: Box::new(left),
                right: Box::new(branch),
            },
        });
    }
    let body = body.ok_or_else(|| refuse("UNPIVOT"))?;
    if body.to_string().len() > super::typed::MAX_RULE_BYTES {
        return Err(refuse("UNPIVOT"));
    }
    Ok(body)
}
