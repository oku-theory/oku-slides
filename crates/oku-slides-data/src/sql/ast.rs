//! AST helpers: node builders for the rewrites and the children of a whitelisted expression.

use sqlparser::ast::helpers::attached_token::AttachedToken;
use sqlparser::ast::{
    BinaryOperator, CastKind, DataType, Expr, Function, FunctionArg, FunctionArgExpr, FunctionArgumentList,
    FunctionArguments, GroupByExpr, Ident, ObjectName, Query, Select, SelectFlavor, SelectItem, SetExpr, TableAlias,
    TableAliasColumnDef, TableFactor, TableWithJoins, Value, WindowType,
};

/// An unquoted identifier.
pub(super) fn ident(name: &str) -> Ident {
    Ident::new(name)
}

/// The column reference `qualifier.column`.
pub(super) fn column_of(qualifier: &Ident, column: &Ident) -> Expr {
    Expr::CompoundIdentifier(vec![qualifier.clone(), column.clone()])
}

/// A numeric literal written `digits`.
pub(super) fn number(digits: &str) -> Expr {
    Expr::value(Value::Number(digits.to_owned(), false))
}

/// A string literal.
pub(super) fn string(text: &str) -> Expr {
    Expr::value(Value::SingleQuotedString(text.to_owned()))
}

/// `left op right`.
pub(super) fn binary(left: Expr, op: BinaryOperator, right: Expr) -> Expr {
    Expr::BinaryOp { left: Box::new(left), op, right: Box::new(right) }
}

/// `CAST(expr AS data_type)`.
pub(super) fn cast(expr: Expr, data_type: DataType) -> Expr {
    Expr::Cast { kind: CastKind::Cast, expr: Box::new(expr), data_type, format: None }
}

/// The call `name(args…)`.
pub(super) fn call(name: &str, args: Vec<Expr>) -> Expr {
    Expr::Function(Function {
        name: ObjectName::from(vec![ident(name)]),
        uses_odbc_syntax: false,
        parameters: FunctionArguments::None,
        args: FunctionArguments::List(FunctionArgumentList {
            duplicate_treatment: None,
            args: args.into_iter().map(|arg| FunctionArg::Unnamed(FunctionArgExpr::Expr(arg))).collect(),
            clauses: vec![],
        }),
        filter: None,
        null_treatment: None,
        over: None,
        within_group: vec![],
    })
}

/// The call `name(args…) OVER window`.
pub(super) fn call_over(name: &str, args: Vec<Expr>, over: Option<WindowType>) -> Expr {
    let mut expr = call(name, args);
    if let Expr::Function(function) = &mut expr {
        function.over = over;
    }
    expr
}

/// The plain argument expressions of a call; wildcard and named arguments are skipped.
pub(super) fn args(function: &Function) -> Vec<&Expr> {
    match &function.args {
        FunctionArguments::List(list) => list
            .args
            .iter()
            .filter_map(|arg| match arg {
                FunctionArg::Unnamed(FunctionArgExpr::Expr(expr)) => Some(expr),
                _ => None,
            })
            .collect(),
        FunctionArguments::None | FunctionArguments::Subquery(_) => vec![],
    }
}

/// The function's single-part name, lowercased; `None` for a qualified name.
pub(super) fn function_name(function: &Function) -> Option<String> {
    match function.name.0.as_slice() {
        [part] => part.as_ident().map(|name| name.value.to_ascii_lowercase()),
        _ => None,
    }
}

/// A table alias, with column names when `columns` is not empty.
pub(super) fn alias(name: Ident, columns: &[&str]) -> TableAlias {
    TableAlias {
        explicit: true,
        name,
        columns: columns.iter().map(|column| TableAliasColumnDef { name: ident(column), data_type: None }).collect(),
    }
}

/// The derived table `(query) AS alias`.
pub(super) fn derived(query: Query, alias: TableAlias) -> TableFactor {
    TableFactor::Derived { lateral: false, subquery: Box::new(query), alias: Some(alias) }
}

/// A query of one set expression and nothing else.
pub(super) fn query_of(body: SetExpr) -> Query {
    Query {
        with: None,
        body: Box::new(body),
        order_by: None,
        limit_clause: None,
        fetch: None,
        locks: vec![],
        for_clause: None,
        settings: None,
        format_clause: None,
        pipe_operators: vec![],
    }
}

/// `SELECT projection FROM relation`.
pub(super) fn select(projection: Vec<SelectItem>, relation: TableFactor) -> Select {
    select_from(projection, vec![TableWithJoins { relation, joins: vec![] }])
}

/// `SELECT projection FROM from…`, with no `FROM` when `from` is empty.
pub(super) fn select_from(projection: Vec<SelectItem>, from: Vec<TableWithJoins>) -> Select {
    Select {
        select_token: AttachedToken::empty(),
        distinct: None,
        top: None,
        top_before_distinct: false,
        projection,
        exclude: None,
        into: None,
        from,
        lateral_views: vec![],
        prewhere: None,
        selection: None,
        group_by: GroupByExpr::Expressions(vec![], vec![]),
        cluster_by: vec![],
        distribute_by: vec![],
        sort_by: vec![],
        having: None,
        named_window: vec![],
        qualify: None,
        window_before_qualify: false,
        value_table_mode: None,
        connect_by: None,
        flavor: SelectFlavor::Standard,
    }
}

/// The direct sub-expressions of a whitelisted expression, subqueries excluded.
pub(super) fn children(expr: &Expr) -> Vec<&Expr> {
    match expr {
        Expr::IsNull(e)
        | Expr::IsNotNull(e)
        | Expr::Nested(e)
        | Expr::UnaryOp { expr: e, .. }
        | Expr::Cast { expr: e, .. }
        | Expr::Extract { expr: e, .. }
        | Expr::Ceil { expr: e, .. }
        | Expr::Floor { expr: e, .. }
        | Expr::Trim { expr: e, .. }
        | Expr::InSubquery { expr: e, .. } => vec![e],
        Expr::Interval(interval) => vec![&interval.value],
        Expr::IsDistinctFrom(a, b)
        | Expr::IsNotDistinctFrom(a, b)
        | Expr::BinaryOp { left: a, right: b, .. }
        | Expr::Like { expr: a, pattern: b, .. }
        | Expr::ILike { expr: a, pattern: b, .. }
        | Expr::Position { expr: a, r#in: b } => vec![a, b],
        Expr::Between { expr, low, high, .. } => vec![expr, low, high],
        Expr::InList { expr, list, .. } => std::iter::once(&**expr).chain(list).collect(),
        Expr::Substring { expr, substring_from, substring_for, .. } => {
            std::iter::once(&**expr).chain(substring_from.as_deref()).chain(substring_for.as_deref()).collect()
        }
        Expr::Case { operand, conditions, else_result, .. } => operand
            .as_deref()
            .into_iter()
            .chain(conditions.iter().flat_map(|when| [&when.condition, &when.result]))
            .chain(else_result.as_deref())
            .collect(),
        Expr::Function(function) => {
            let over = match &function.over {
                Some(WindowType::WindowSpec(spec)) => {
                    spec.partition_by.iter().chain(spec.order_by.iter().map(|key| &key.expr)).collect()
                }
                _ => vec![],
            };
            args(function).into_iter().chain(function.filter.as_deref()).chain(over).collect()
        }
        _ => vec![],
    }
}

/// The direct sub-expressions of a whitelisted expression, mutably, subqueries excluded.
pub(super) fn children_mut(expr: &mut Expr) -> Vec<&mut Expr> {
    match expr {
        Expr::IsNull(e)
        | Expr::IsNotNull(e)
        | Expr::Nested(e)
        | Expr::UnaryOp { expr: e, .. }
        | Expr::Cast { expr: e, .. }
        | Expr::Extract { expr: e, .. }
        | Expr::Ceil { expr: e, .. }
        | Expr::Floor { expr: e, .. }
        | Expr::Trim { expr: e, .. }
        | Expr::InSubquery { expr: e, .. } => vec![e],
        Expr::Interval(interval) => vec![&mut interval.value],
        Expr::IsDistinctFrom(a, b)
        | Expr::IsNotDistinctFrom(a, b)
        | Expr::BinaryOp { left: a, right: b, .. }
        | Expr::Like { expr: a, pattern: b, .. }
        | Expr::ILike { expr: a, pattern: b, .. }
        | Expr::Position { expr: a, r#in: b } => vec![a, b],
        Expr::Between { expr, low, high, .. } => vec![expr, low, high],
        Expr::InList { expr, list, .. } => std::iter::once(&mut **expr).chain(list).collect(),
        Expr::Substring { expr, substring_from, substring_for, .. } => std::iter::once(&mut **expr)
            .chain(substring_from.as_deref_mut())
            .chain(substring_for.as_deref_mut())
            .collect(),
        Expr::Case { operand, conditions, else_result, .. } => operand
            .as_deref_mut()
            .into_iter()
            .chain(conditions.iter_mut().flat_map(|when| [&mut when.condition, &mut when.result]))
            .chain(else_result.as_deref_mut())
            .collect(),
        Expr::Function(function) => {
            let filter = function.filter.as_deref_mut();
            let over: Vec<&mut Expr> = match &mut function.over {
                Some(WindowType::WindowSpec(spec)) => {
                    spec.partition_by.iter_mut().chain(spec.order_by.iter_mut().map(|key| &mut key.expr)).collect()
                }
                _ => vec![],
            };
            let args = match &mut function.args {
                FunctionArguments::List(list) => list
                    .args
                    .iter_mut()
                    .filter_map(|arg| match arg {
                        FunctionArg::Unnamed(FunctionArgExpr::Expr(expr)) => Some(expr),
                        _ => None,
                    })
                    .collect(),
                FunctionArguments::None | FunctionArguments::Subquery(_) => vec![],
            };
            args.into_iter().chain(filter).chain(over).collect()
        }
        _ => vec![],
    }
}

/// Calls `visit` on `expr` and its sub-expressions, pre-order, outside subqueries; a `false`
/// return skips the node's children.
pub(super) fn each_mut(expr: &mut Expr, visit: &mut dyn FnMut(&mut Expr) -> bool) {
    if visit(expr) {
        for child in children_mut(expr) {
            each_mut(child, visit);
        }
    }
}

/// Calls `visit` on `expr` and its sub-expressions, pre-order, outside subqueries.
pub(super) fn each(expr: &Expr, visit: &mut dyn FnMut(&Expr)) {
    visit(expr);
    for child in children(expr) {
        each(child, visit);
    }
}

/// How many arguments a call has, wildcards and named ones included.
pub(super) fn arg_count(function: &Function) -> usize {
    match &function.args {
        FunctionArguments::List(list) => list.args.len(),
        FunctionArguments::None | FunctionArguments::Subquery(_) => 0,
    }
}

/// The integer a literal is, sign included.
pub(super) fn integer_literal(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::Value(value) => match &value.value {
            Value::Number(digits, false) => digits.parse().ok(),
            _ => None,
        },
        Expr::UnaryOp { op: sqlparser::ast::UnaryOperator::Minus, expr } => integer_literal(expr).map(|n| -n),
        _ => None,
    }
}

/// The text of a string literal.
pub(super) fn string_literal(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::Value(value) => match &value.value {
            Value::SingleQuotedString(text) => Some(text),
            _ => None,
        },
        _ => None,
    }
}

/// Calls `visit` on each select of a set expression, outside nested queries.
pub(super) fn for_each_select<E>(
    body: &mut SetExpr,
    visit: &mut dyn FnMut(&mut Select) -> Result<(), E>,
) -> Result<(), E> {
    match body {
        SetExpr::Select(select) => visit(select),
        SetExpr::SetOperation { left, right, .. } => {
            for_each_select(left, visit)?;
            for_each_select(right, visit)
        }
        _ => Ok(()),
    }
}
