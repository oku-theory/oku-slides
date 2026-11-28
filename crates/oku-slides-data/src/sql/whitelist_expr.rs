//! The expression half of the whitelist: operators, literals, casts, calls and windows.

use sqlparser::ast::{
    BinaryOperator, CastKind, CeilFloorKind, DataType, DateTimeField, DuplicateTreatment, ExactNumberInfo, Expr,
    Function, FunctionArg, FunctionArgExpr, FunctionArguments, Interval, SetExpr, UnaryOperator, Value, WindowFrame,
    WindowFrameBound, WindowFrameUnits, WindowSpec, WindowType,
};

use super::ast::{args, children, function_name, integer_literal, string_literal};
use super::functions::{self, Kind};
use super::whitelist::{Ctx, Names, Node, Walk, check_qualifier, refuse};
use crate::DataError;

/// `EXTRACT` fields phase 2 runs.
const EXTRACT_FIELDS: [&str; 9] = ["year", "month", "day", "quarter", "week", "dow", "isodow", "doy", "isoyear"];

/// Interval units phase 2 runs, singular.
const INTERVAL_UNITS: [&str; 4] = ["day", "week", "month", "year"];

/// Whether an `EXTRACT` field is one phase 2 runs.
fn extract_field(field: &DateTimeField) -> bool {
    matches!(
        field,
        DateTimeField::Year
            | DateTimeField::Month
            | DateTimeField::Day
            | DateTimeField::Quarter
            | DateTimeField::Week(None)
            | DateTimeField::Dow
            | DateTimeField::Isodow
            | DateTimeField::Doy
            | DateTimeField::Isoyear
    )
}

/// The singular unit of an interval field phase 2 runs.
pub(super) fn interval_unit(field: &DateTimeField) -> Option<&'static str> {
    match field {
        DateTimeField::Day | DateTimeField::Days => Some("day"),
        DateTimeField::Week(None) | DateTimeField::Weeks => Some("week"),
        DateTimeField::Month | DateTimeField::Months => Some("month"),
        DateTimeField::Year | DateTimeField::Years => Some("year"),
        _ => None,
    }
}

/// Whether `text` is an interval string phase 2 runs: an integer and a day, week, month or year unit.
fn interval_text(text: &str) -> bool {
    let mut words = text.split_whitespace();
    let (Some(count), Some(unit), None) = (words.next(), words.next(), words.next()) else { return false };
    let count = count.strip_prefix('-').unwrap_or(count);
    let unit = unit.to_ascii_lowercase();
    let unit = unit.strip_suffix('s').unwrap_or(&unit);
    !count.is_empty() && count.bytes().all(|b| b.is_ascii_digit()) && INTERVAL_UNITS.contains(&unit)
}

/// Whether a cast target is one phase 2 runs.
fn cast_type(data_type: &DataType) -> bool {
    matches!(
        data_type,
        DataType::Double(ExactNumberInfo::None)
            | DataType::DoublePrecision
            | DataType::Real
            | DataType::BigInt(None)
            | DataType::Integer(None)
            | DataType::Int(None)
            | DataType::Varchar(None)
            | DataType::Text
            | DataType::Boolean
            | DataType::Bool
            | DataType::Date
    )
}

/// The construct naming a call with `count` arguments, as written.
fn call_shape(name: &str, count: usize) -> String {
    format!("{name}({})", vec!["…"; count].join(", "))
}

impl<'a> Walk<'a> {
    /// Checks one expression and queues its sub-expressions and subqueries.
    pub fn expr(&mut self, expr: &'a Expr, ctx: &Ctx<'a>, depth: usize) -> Result<(), DataError> {
        match expr {
            Expr::Identifier(ident) => self.plain(ident)?,
            Expr::CompoundIdentifier(parts) => match parts.as_slice() {
                [qualifier, column] => {
                    self.plain(column)?;
                    self.plain(qualifier)?;
                    check_qualifier(qualifier, &ctx.facts, &expr.to_string())?;
                }
                _ => return Err(refuse(expr.to_string())),
            },
            Expr::Value(value) => match &value.value {
                Value::Number(..) | Value::SingleQuotedString(_) | Value::Boolean(_) | Value::Null => {}
                other => return Err(refuse(other.to_string())),
            },
            Expr::TypedString(typed) if typed.data_type == DataType::Date => {}
            Expr::TypedString(typed) => return Err(refuse(typed.data_type.to_string())),
            Expr::UnaryOp { op: UnaryOperator::Not | UnaryOperator::Minus | UnaryOperator::Plus, .. }
            | Expr::IsNull(_)
            | Expr::IsNotNull(_)
            | Expr::Nested(_)
            | Expr::IsDistinctFrom(..)
            | Expr::IsNotDistinctFrom(..)
            | Expr::Between { .. }
            | Expr::Position { .. }
            | Expr::Case { .. } => {}
            Expr::UnaryOp { op, .. } => return Err(refuse(op.to_string())),
            Expr::BinaryOp { op, .. } => binary(op)?,
            Expr::Like { any, escape_char, .. } | Expr::ILike { any, escape_char, .. } => {
                if *any {
                    return Err(refuse("LIKE ANY"));
                }
                if escape_char.is_some() {
                    return Err(refuse("ESCAPE"));
                }
            }
            Expr::InList { list, .. } => {
                if let Some(item) = list.iter().find(|item| !in_list_item(item)) {
                    let shown = if matches!(item, Expr::Value(v) if v.value == Value::Null) {
                        "NULL".to_owned()
                    } else {
                        item.to_string()
                    };
                    return Err(refuse(format!("IN (… {shown} …)")));
                }
            }
            Expr::InSubquery { subquery, negated, .. } => {
                if *negated {
                    return Err(refuse("NOT IN (SELECT …)"));
                }
                self.push(Node::Query(subquery), depth);
            }
            Expr::Subquery(query) => {
                scalar_subquery(query, ctx.scalar)?;
                self.push(Node::Query(query), depth);
            }
            Expr::Cast { kind, data_type, format, .. } => {
                if *kind == CastKind::SafeCast {
                    return Err(refuse("SAFE_CAST"));
                }
                if format.is_some() {
                    return Err(refuse("CAST (… FORMAT …)"));
                }
                if !cast_type(data_type) {
                    return Err(refuse(data_type.to_string()));
                }
            }
            Expr::Extract { field, .. } => {
                if !extract_field(field) {
                    return Err(refuse(format!("EXTRACT({field} FROM …)")));
                }
            }
            Expr::Ceil { field, .. } | Expr::Floor { field, .. } => {
                if !matches!(field, CeilFloorKind::DateTimeField(DateTimeField::NoDateTime)) {
                    return Err(refuse(expr.to_string()));
                }
            }
            Expr::Substring { substring_from, substring_for, .. } => {
                let start = substring_from.as_deref().and_then(integer_literal);
                match (start, substring_for) {
                    (Some(start), None) if start != 0 => {}
                    (Some(start), Some(_)) if start > 0 => {}
                    _ => return Err(refuse(expr.to_string())),
                }
            }
            Expr::Trim { trim_where, trim_what, trim_characters, .. } => {
                if trim_where.is_some() || trim_what.is_some() || trim_characters.is_some() {
                    return Err(refuse("TRIM(… FROM …)"));
                }
            }
            Expr::Interval(interval) => check_interval(interval, expr)?,
            Expr::Function(function) => return self.function(function, ctx, depth),
            other => return Err(refuse(named(other))),
        }
        for child in children(expr) {
            self.push_expr(child, ctx, depth);
        }
        Ok(())
    }

    fn function(&mut self, function: &'a Function, ctx: &Ctx<'a>, depth: usize) -> Result<(), DataError> {
        let Some(lower) = function_name(function) else { return Err(refuse(format!("{}()", function.name))) };
        let written = function.name.to_string();
        if lower == "columns" {
            return Err(refuse("COLUMNS(…)"));
        }
        if function.uses_odbc_syntax {
            return Err(refuse("{fn …}"));
        }
        if !matches!(function.parameters, FunctionArguments::None) {
            return Err(refuse(format!("{written}(…)(…)")));
        }
        if !function.within_group.is_empty() {
            return Err(refuse("WITHIN GROUP"));
        }
        if let Some(treatment) = &function.null_treatment {
            return Err(refuse(treatment.to_string()));
        }
        let list = match &function.args {
            FunctionArguments::List(list) => Some(list),
            FunctionArguments::Subquery(_) => return Err(refuse(format!("{written}(SELECT …)"))),
            FunctionArguments::None => None,
        };
        let arguments = list.map(|list| list.args.as_slice()).unwrap_or_default();
        if let Some(clause) = list.and_then(|list| list.clauses.first()) {
            return Err(refuse(format!("{written}(… {clause})")));
        }
        let distinct = matches!(list.and_then(|l| l.duplicate_treatment), Some(DuplicateTreatment::Distinct));
        if distinct && lower != "count" {
            return Err(refuse(format!("{}(DISTINCT …)", written.to_ascii_uppercase())));
        }
        for argument in arguments {
            match argument {
                FunctionArg::Unnamed(FunctionArgExpr::Expr(_)) => {}
                FunctionArg::Unnamed(FunctionArgExpr::Wildcard) if lower == "count" && arguments.len() == 1 => {}
                FunctionArg::Unnamed(FunctionArgExpr::Wildcard) => return Err(refuse(format!("{written}(*)"))),
                FunctionArg::Unnamed(FunctionArgExpr::QualifiedWildcard(name)) => {
                    return Err(refuse(format!("{written}({name}.*)")));
                }
                FunctionArg::Named { operator, .. } | FunctionArg::ExprNamed { operator, .. } => {
                    return Err(refuse(operator.to_string()));
                }
            }
        }
        // The rewrites put windows in scalar calls (`nullif(count(x) OVER (…), 0)`); a user's
        // query may not.
        let inner = Ctx { windows: ctx.windows && self.names == Names::Engine, ..ctx.clone() };
        let plain = Ctx { scalar: false, ..inner.clone() };
        let engine = match self.names {
            Names::Engine => functions::lookup_engine(&lower, arguments.len()),
            Names::User => None,
        };
        let Some(spec) = engine.or_else(|| functions::lookup(&lower)) else {
            self.unknown.get_or_insert(written);
            for arg in args(function) {
                self.push_expr(arg, &inner, depth);
            }
            return Ok(());
        };
        if arguments.len() < spec.min || arguments.len() > spec.max {
            return Err(refuse(call_shape(&written, arguments.len())));
        }
        argument_rules(&lower, &written, &args(function))?;
        match (spec.kind, &function.over) {
            (Kind::Window, None) => return Err(refuse(format!("{written}(…) without OVER"))),
            (Kind::Scalar | Kind::Aggregate { window: false }, Some(_)) => {
                return Err(refuse(format!("{written}(…) OVER (…)")));
            }
            (_, Some(_)) if !ctx.windows => return Err(refuse("OVER")),
            (_, Some(over)) => {
                let window = window_of(over, ctx)?;
                check_over(&lower, &written, spec.kind, window)?;
                for expr in window.partition_by.iter().chain(window.order_by.iter().map(|key| &key.expr)) {
                    self.push_expr(expr, &plain, depth);
                }
            }
            (_, None) => {}
        }
        if let Some(filter) = &function.filter {
            if !matches!(spec.kind, Kind::Aggregate { .. }) || function.over.is_some() {
                return Err(refuse("FILTER (WHERE …)"));
            }
            self.push_expr(filter, &plain, depth);
        }
        for arg in args(function) {
            self.push_expr(arg, &inner, depth);
        }
        Ok(())
    }
}

/// Whether `item` may stand in an `IN` list: a non-NULL literal.
///
/// A NULL item makes `NOT IN` NULL on every row in `DuckDB` and not in the engine; any
/// expression or column may be NULL (`CAST(NULL AS INTEGER)`, `(NULL)`, `NULL::INT`), so only
/// literals that never are pass.
fn in_list_item(item: &Expr) -> bool {
    match item {
        Expr::Value(value) => {
            matches!(value.value, Value::Number(..) | Value::SingleQuotedString(_) | Value::Boolean(_))
        }
        Expr::TypedString(typed) => typed.data_type == DataType::Date,
        Expr::UnaryOp { op: UnaryOperator::Minus | UnaryOperator::Plus, expr } => {
            matches!(expr.as_ref(), Expr::Value(value) if matches!(value.value, Value::Number(..)))
        }
        _ => false,
    }
}

/// Refuses an operator outside the whitelist.
fn binary(op: &BinaryOperator) -> Result<(), DataError> {
    match op {
        BinaryOperator::Plus
        | BinaryOperator::Minus
        | BinaryOperator::Multiply
        | BinaryOperator::Divide
        | BinaryOperator::Modulo
        | BinaryOperator::DuckIntegerDivide
        | BinaryOperator::StringConcat
        | BinaryOperator::Gt
        | BinaryOperator::Lt
        | BinaryOperator::GtEq
        | BinaryOperator::LtEq
        | BinaryOperator::Eq
        | BinaryOperator::NotEq
        | BinaryOperator::And
        | BinaryOperator::Or => Ok(()),
        other => Err(refuse(other.to_string())),
    }
}

/// The window a call runs over: its own spec or a named one of the select.
fn window_of<'a>(over: &'a WindowType, ctx: &Ctx<'a>) -> Result<&'a WindowSpec, DataError> {
    match over {
        WindowType::WindowSpec(spec) => match &spec.window_name {
            Some(name) => Err(refuse(format!("OVER ({name} …)"))),
            None => Ok(spec),
        },
        WindowType::NamedWindow(name) => {
            let lower = name.value.to_ascii_lowercase();
            let found = ctx.facts.windows.iter().find(|(window, _)| *window == lower);
            found.map(|(_, spec)| *spec).ok_or_else(|| refuse(format!("OVER {name}")))
        }
    }
}

/// Refuses a window the engine cannot run as written (mixed directions, frames, running
/// aggregates other than sum, count and avg).
fn check_over(lower: &str, written: &str, kind: Kind, spec: &WindowSpec) -> Result<(), DataError> {
    if spec.order_by.iter().any(|key| key.with_fill.is_some()) {
        return Err(refuse("WITH FILL"));
    }
    let ascending = |key: &sqlparser::ast::OrderByExpr| key.options.asc.unwrap_or(true);
    if let Some(first) = spec.order_by.first()
        && spec.order_by.iter().any(|key| ascending(key) != ascending(first))
    {
        return Err(refuse("OVER (ORDER BY … ASC, … DESC)"));
    }
    let ordered = !spec.order_by.is_empty();
    match (kind, &spec.window_frame) {
        (Kind::Aggregate { .. }, frame) if ordered => match lower {
            "sum" | "count" | "avg" | "mean" => match frame {
                None => return Err(refuse("RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW")),
                Some(frame) if !running(frame) => return Err(refuse(frame_text(frame))),
                Some(_) => {}
            },
            _ => return Err(refuse(format!("{}(…) OVER (ORDER BY …)", written.to_ascii_uppercase()))),
        },
        (Kind::Window | Kind::Aggregate { .. }, Some(frame)) => return Err(refuse(frame_text(frame))),
        _ => {}
    }
    Ok(())
}

/// A frame as written in a window.
fn frame_text(frame: &WindowFrame) -> String {
    match &frame.end_bound {
        Some(end) => format!("{} BETWEEN {} AND {end}", frame.units, frame.start_bound),
        None => format!("{} {}", frame.units, frame.start_bound),
    }
}

/// Whether a frame is `ROWS` from the partition's start to the current row.
fn running(frame: &WindowFrame) -> bool {
    frame.units == WindowFrameUnits::Rows
        && frame.start_bound == WindowFrameBound::Preceding(None)
        && matches!(frame.end_bound, None | Some(WindowFrameBound::CurrentRow))
}

/// Refuses a scalar subquery phase 2 cannot turn into a join: it must be one ungrouped aggregate.
fn scalar_subquery(query: &sqlparser::ast::Query, allowed: bool) -> Result<(), DataError> {
    let SetExpr::Select(select) = &*query.body else { return Err(refuse("(SELECT …)")) };
    let grouped = match &select.group_by {
        sqlparser::ast::GroupByExpr::All(_) => true,
        sqlparser::ast::GroupByExpr::Expressions(exprs, _) => !exprs.is_empty(),
    };
    if grouped && allowed {
        return Err(refuse("(SELECT … GROUP BY …)"));
    }
    let single_aggregate = matches!(select.projection.as_slice(), [item] if match item {
        sqlparser::ast::SelectItem::UnnamedExpr(expr) | sqlparser::ast::SelectItem::ExprWithAlias { expr, .. } => {
            matches!(expr, Expr::Function(function) if function.over.is_none()
                && function_name(function).is_some_and(|name| functions::is_aggregate(&name)))
        }
        _ => false,
    });
    let bare = query.with.is_none()
        && query.order_by.is_none()
        && query.limit_clause.is_none()
        && query.fetch.is_none()
        && select.having.is_none()
        && select.distinct.is_none()
        && select.qualify.is_none();
    if allowed && single_aggregate && bare { Ok(()) } else { Err(refuse("(SELECT …)")) }
}

fn check_interval(interval: &Interval, expr: &Expr) -> Result<(), DataError> {
    let precise = interval.leading_precision.is_some()
        || interval.last_field.is_some()
        || interval.fractional_seconds_precision.is_some();
    let ok = !precise
        && match (&*interval.value, &interval.leading_field) {
            (value, Some(field)) => interval_unit(field).is_some() && integer_literal(value).is_some_and(|n| n >= 0),
            (value, None) => string_literal(value).is_some_and(interval_text),
        };
    if ok { Ok(()) } else { Err(refuse(expr.to_string())) }
}

/// Literal arguments the whitelisted functions require, checked on the call as written.
fn argument_rules(lower: &str, written: &str, args: &[&Expr]) -> Result<(), DataError> {
    let text = |at: usize| args.get(at).and_then(|arg| string_literal(arg)).map(str::to_ascii_lowercase);
    let shown = |at: usize| args.get(at).map(ToString::to_string).unwrap_or_default();
    match lower {
        "round" if args.len() == 2 && integer_literal(args[1]).is_none_or(|n| n < 0) => {
            Err(refuse(format!("{written}(…, {})", shown(1))))
        }
        "replace" if text(1).is_none_or(|pattern| pattern.is_empty()) => {
            Err(refuse(format!("{written}(…, {}, …)", shown(1))))
        }
        "date_trunc" if !matches!(text(0).as_deref(), Some("year" | "month")) => {
            Err(refuse(format!("{written}({}, …)", shown(0))))
        }
        "date_diff" | "datediff" if !matches!(text(0).as_deref(), Some("day" | "month" | "year")) => {
            Err(refuse(format!("{written}({}, …, …)", shown(0))))
        }
        "date_part" | "datepart" if !text(0).is_some_and(|field| EXTRACT_FIELDS.contains(&field.as_str())) => {
            Err(refuse(format!("{written}({}, …)", shown(0))))
        }
        "quantile_cont" | "quantile_disc" if !matches!(args.get(1), Some(Expr::Value(v)) if matches!(v.value, Value::Number(..))) => {
            Err(refuse(format!("{written}(…, {})", shown(1))))
        }
        _ => Ok(()),
    }
}

/// The name a refusal gives an expression form phase 2 does not run.
fn named(expr: &Expr) -> String {
    let name = match expr {
        Expr::IsTrue(_) => "IS TRUE",
        Expr::IsNotTrue(_) => "IS NOT TRUE",
        Expr::IsFalse(_) => "IS FALSE",
        Expr::IsNotFalse(_) => "IS NOT FALSE",
        Expr::IsUnknown(_) => "IS UNKNOWN",
        Expr::IsNotUnknown(_) => "IS NOT UNKNOWN",
        Expr::Exists { .. } => "EXISTS",
        Expr::Array(_) => "[…]",
        Expr::Dictionary(_) | Expr::Map(_) => "{…}",
        Expr::Struct { .. } => "STRUCT",
        Expr::Lambda(_) => "->",
        Expr::Tuple(_) => "(…, …)",
        Expr::GroupingSets(_) => "GROUPING SETS",
        Expr::Cube(_) => "CUBE",
        Expr::Rollup(_) => "ROLLUP",
        Expr::Wildcard(_) => "*",
        Expr::SimilarTo { .. } => "SIMILAR TO",
        Expr::RLike { .. } => "REGEXP",
        Expr::AnyOp { .. } => "ANY",
        Expr::AllOp { .. } => "ALL",
        Expr::Collate { .. } => "COLLATE",
        Expr::AtTimeZone { .. } => "AT TIME ZONE",
        Expr::Convert { .. } => "CONVERT",
        Expr::Overlay { .. } => "OVERLAY",
        Expr::InUnnest { .. } => "UNNEST",
        Expr::IsNormalized { .. } => "IS NORMALIZED",
        Expr::MatchAgainst { .. } => "MATCH",
        Expr::OuterJoin(_) => "(+)",
        Expr::Prior(_) => "PRIOR",
        Expr::MemberOf(_) => "MEMBER OF",
        other => return crate::error::quoted(&other.to_string()),
    };
    name.to_owned()
}
