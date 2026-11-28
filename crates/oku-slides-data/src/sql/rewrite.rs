//! The schema-free rewrites of phase 2 (spec, "Phase 2 spike"): polars runs the result as `DuckDB` would.
//!
//! One pass: query and select rules run before the visitor descends, expression rules after a
//! node's children, so a rule only ever matches what the user wrote and never its own output.
//! Rules marked (s) in the spec need input schemas; the worker applies them.

use std::ops::ControlFlow;

use sqlparser::ast::{
    BinaryOperator, CaseWhen, DataType, DateTimeField, ExactNumberInfo, Expr, ExtractSyntax, Function, Interval,
    OrderByKind, PivotValueSource, Query, TableFactor, Value, VisitMut, VisitorMut,
};

use super::ast::{
    args, binary, call, call_over, cast, for_each_select, function_name, integer_literal, number, string,
};
use super::rewrite_select;
use super::whitelist::refuse;
use super::whitelist_expr::interval_unit;
use crate::DataError;

/// Largest decimal exponent a literal may have: past it, a DOUBLE overflows.
const MAX_EXPONENT: i64 = 308;

/// `query` with the schema-free rewrites applied.
pub(super) fn engine_query(query: &Query) -> Result<Query, DataError> {
    let mut query = query.clone();
    let mut rewriter = Rewriter::default();
    match query.visit(&mut rewriter) {
        ControlFlow::Break(error) => Err(error),
        ControlFlow::Continue(()) => Ok(query),
    }
}

/// `expr`, written by the worker's rewrites, with the schema-free rewrites applied.
#[cfg(feature = "engine")]
pub(super) fn engine_expr(expr: &mut Expr) -> Result<(), DataError> {
    match expr.visit(&mut Rewriter::default()) {
        ControlFlow::Break(error) => Err(error),
        ControlFlow::Continue(()) => Ok(()),
    }
}

/// Numbers of the generated names, unique in the whole query.
#[derive(Debug, Default)]
pub(super) struct Counters {
    next: usize,
}

impl Counters {
    /// A fresh name `prefix` followed by a number.
    pub fn name(&mut self, prefix: &str) -> String {
        self.next += 1;
        format!("{prefix}{}", self.next)
    }
}

/// A PIVOT's parts the worker rewrites, set aside while the visitor walks the rest.
type PivotParts = (Vec<sqlparser::ast::ExprWithAlias>, Vec<Expr>, PivotValueSource);

#[derive(Default)]
struct Rewriter {
    counters: Counters,
    pivots: Vec<PivotParts>,
}

impl VisitorMut for Rewriter {
    type Break = DataError;

    fn pre_visit_query(&mut self, query: &mut Query) -> ControlFlow<DataError> {
        if let Some(order_by) = &mut query.order_by {
            match &mut order_by.kind {
                OrderByKind::All(options) => {
                    options.nulls_first.get_or_insert(false);
                }
                OrderByKind::Expressions(keys) => {
                    for key in keys {
                        key.options.nulls_first.get_or_insert(false);
                    }
                }
            }
        }
        let counters = &mut self.counters;
        match for_each_select(&mut query.body, &mut |select| rewrite_select::apply(select, counters)) {
            Ok(()) => ControlFlow::Continue(()),
            Err(error) => ControlFlow::Break(error),
        }
    }

    // The worker's PIVOT rewrite needs the aggregate as written: rules skip it and its values.
    fn pre_visit_table_factor(&mut self, factor: &mut TableFactor) -> ControlFlow<DataError> {
        if let TableFactor::Pivot { aggregate_functions, value_column, value_source, .. } = factor {
            self.pivots.push((
                std::mem::take(aggregate_functions),
                std::mem::take(value_column),
                std::mem::replace(value_source, PivotValueSource::List(vec![])),
            ));
        }
        ControlFlow::Continue(())
    }

    fn post_visit_table_factor(&mut self, factor: &mut TableFactor) -> ControlFlow<DataError> {
        if let TableFactor::Pivot { aggregate_functions, value_column, value_source, .. } = factor
            && let Some((aggregates, columns, source)) = self.pivots.pop()
        {
            *aggregate_functions = aggregates;
            *value_column = columns;
            *value_source = source;
        }
        ControlFlow::Continue(())
    }

    fn post_visit_expr(&mut self, expr: &mut Expr) -> ControlFlow<DataError> {
        match rewrite_expr(expr) {
            Ok(Some(new)) => *expr = new,
            Ok(None) => {}
            Err(error) => return ControlFlow::Break(error),
        }
        ControlFlow::Continue(())
    }
}

/// Whether `expr` gives a DOUBLE without a cast: a rewritten division or a cast to DOUBLE.
fn is_double(expr: &Expr) -> bool {
    match expr {
        Expr::Nested(inner) => is_double(inner),
        Expr::BinaryOp { op: BinaryOperator::Divide, .. } => true,
        Expr::Cast { data_type, .. } => *data_type == DataType::Double(ExactNumberInfo::None),
        _ => false,
    }
}

/// Whether `expr` is a number literal other than zero: a divisor that needs no `nullif`.
fn is_nonzero_number(expr: &Expr) -> bool {
    match expr {
        Expr::Value(value) => match &value.value {
            Value::Number(text, _) => text.parse::<f64>().is_ok_and(|n| n != 0.0),
            _ => false,
        },
        _ => false,
    }
}

/// The rewrite of one user-written expression whose children are already rewritten.
fn rewrite_expr(expr: &mut Expr) -> Result<Option<Expr>, DataError> {
    Ok(match expr {
        Expr::BinaryOp { left, op: BinaryOperator::Divide, right } => {
            // A numerator already DOUBLE stays as is: recasting `a / b` in `a / b / c` would nest
            // a bracket-free chain one CAST deeper per division.
            if !is_double(left) {
                let numerator = std::mem::replace(&mut **left, Expr::value(Value::Null));
                **left = cast(numerator, DataType::Double(ExactNumberInfo::None));
            }
            // R20: by zero is NULL, as `%` and `//` are, never an infinity data cannot hold.
            if !is_nonzero_number(right) {
                let divisor = std::mem::replace(&mut **right, Expr::value(Value::Null));
                **right = call("nullif", vec![divisor, number("0")]);
            }
            None
        }
        Expr::Interval(interval) => interval_text(interval).map(|text| {
            Expr::Interval(Interval {
                value: Box::new(string(&text)),
                leading_field: None,
                leading_precision: None,
                last_field: None,
                fractional_seconds_precision: None,
            })
        }),
        Expr::Substring { expr: text, substring_from: Some(from), substring_for: None, .. } => {
            match integer_literal(from) {
                Some(start) if start < 0 => Some(call("right", vec![(**text).clone(), number(&(-start).to_string())])),
                _ => None,
            }
        }
        Expr::Value(value) => match &value.value {
            Value::Number(digits, long) if digits.contains(['e', 'E']) => {
                let decimal = decimal(digits).ok_or_else(|| refuse(digits.clone()))?;
                Some(Expr::value(Value::Number(decimal, *long)))
            }
            _ => None,
        },
        Expr::Function(function) => rewrite_call(function),
        _ => None,
    })
}

/// `INTERVAL n UNIT` as the string polars reads, `'n unit[s]'`.
fn interval_text(interval: &Interval) -> Option<String> {
    let unit = interval_unit(interval.leading_field.as_ref()?)?;
    let count = integer_literal(&interval.value)?;
    Some(if count == 1 { format!("1 {unit}") } else { format!("{count} {unit}s") })
}

/// The plain decimal a literal in exponent notation is, with a fraction so it stays a DOUBLE.
fn decimal(text: &str) -> Option<String> {
    let (mantissa, exponent) = text.split_once(['e', 'E'])?;
    let exponent: i64 = exponent.parse().ok()?;
    if exponent.abs() > MAX_EXPONENT {
        return None;
    }
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = format!("{whole}{fraction}");
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let point = i64::try_from(whole.len()).ok()? + exponent;
    let (whole, fraction) = if point <= 0 {
        ("0".to_owned(), format!("{}{digits}", "0".repeat(usize::try_from(-point).ok()?)))
    } else {
        let point = usize::try_from(point).ok()?;
        if point >= digits.len() {
            (format!("{digits}{}", "0".repeat(point - digits.len())), String::new())
        } else {
            (digits[..point].to_owned(), digits[point..].to_owned())
        }
    };
    let whole = whole.trim_start_matches('0');
    let fraction = fraction.trim_end_matches('0');
    let whole = if whole.is_empty() { "0" } else { whole };
    let fraction = if fraction.is_empty() { "0" } else { fraction };
    Some(format!("{whole}.{fraction}"))
}

/// `EXTRACT(field FROM expr)`.
fn extract(field: DateTimeField, expr: Expr) -> Expr {
    Expr::Extract { field, syntax: ExtractSyntax::From, expr: Box::new(expr) }
}

/// The `EXTRACT` field a date function or a `datepart` field name reads.
fn date_field(name: &str) -> Option<DateTimeField> {
    Some(match name {
        "year" => DateTimeField::Year,
        "month" => DateTimeField::Month,
        "day" | "dayofmonth" => DateTimeField::Day,
        "quarter" => DateTimeField::Quarter,
        "week" => DateTimeField::Week(None),
        "dow" | "dayofweek" => DateTimeField::Dow,
        "isodow" => DateTimeField::Isodow,
        "doy" | "dayofyear" => DateTimeField::Doy,
        "isoyear" => DateTimeField::Isoyear,
        _ => return None,
    })
}

/// `CASE WHEN condition THEN result [ELSE otherwise] END`.
fn case_when(condition: Expr, result: Expr, otherwise: Option<Expr>) -> Expr {
    Expr::Case {
        case_token: sqlparser::ast::helpers::attached_token::AttachedToken::empty(),
        end_token: sqlparser::ast::helpers::attached_token::AttachedToken::empty(),
        operand: None,
        conditions: vec![CaseWhen { condition, result }],
        else_result: otherwise.map(Box::new),
    }
}

/// `(expr)`.
fn nested(expr: Expr) -> Expr {
    Expr::Nested(Box::new(expr))
}

/// Moves an aggregate's `FILTER (WHERE c)` into its arguments: `agg(CASE WHEN c THEN x END)`.
///
/// Every argument is wrapped, literals too (`COUNT(1) FILTER (…)` counts the matching rows),
/// except the constant fraction of `quantile_cont` and `quantile_disc`.
fn unfilter(function: &mut Function, name: &str) {
    let Some(condition) = function.filter.take() else { return };
    let constant = matches!(name, "quantile_cont" | "quantile_disc").then_some(1);
    if let sqlparser::ast::FunctionArguments::List(list) = &mut function.args {
        for (at, arg) in list.args.iter_mut().enumerate() {
            // The whitelist lets only unnamed arguments through, and `*` or `t.*` only in `count(*)`.
            let sqlparser::ast::FunctionArg::Unnamed(value) = arg else { continue };
            if constant == Some(at) {
                continue;
            }
            let filtered = match value {
                sqlparser::ast::FunctionArgExpr::Expr(expr) => expr.clone(),
                sqlparser::ast::FunctionArgExpr::Wildcard | sqlparser::ast::FunctionArgExpr::QualifiedWildcard(_) => {
                    number("1")
                }
            };
            *value = sqlparser::ast::FunctionArgExpr::Expr(case_when((*condition).clone(), filtered, None));
        }
    }
}

/// Renames a call in place.
fn rename_call(function: &mut Function, name: &str) {
    function.name = sqlparser::ast::ObjectName::from(vec![super::ast::ident(name)]);
}

/// The rewrite of a whitelisted call, after its `FILTER` moved into its arguments.
fn rewrite_call(function: &mut Function) -> Option<Expr> {
    let name = function_name(function)?;
    unfilter(function, &name);
    let over = function.over.clone();
    let running = matches!(&over, Some(sqlparser::ast::WindowType::WindowSpec(spec)) if !spec.order_by.is_empty());
    let a: Vec<Expr> = args(function).into_iter().cloned().collect();
    let arg = |at: usize| a.get(at).cloned();
    let text = |at: usize| a.get(at).and_then(super::ast::string_literal).map(str::to_ascii_lowercase);
    match name.as_str() {
        "sum" => {
            let x = arg(0)?;
            let summed = if running { call("coalesce", vec![x.clone(), number("0")]) } else { x.clone() };
            let count = call_over("count", vec![x], over.clone());
            Some(case_when(
                binary(count, BinaryOperator::Eq, number("0")),
                Expr::value(Value::Null),
                Some(call_over("sum", vec![summed], over)),
            ))
        }
        "avg" | "mean" if running => {
            let x = arg(0)?;
            let sum = call_over("sum", vec![call("coalesce", vec![x.clone(), number("0")])], over.clone());
            let count = call_over("count", vec![x], over);
            Some(nested(binary(
                cast(sum, DataType::Double(ExactNumberInfo::None)),
                BinaryOperator::Divide,
                call("nullif", vec![count, number("0")]),
            )))
        }
        "mean" => rename(function, "avg"),
        "lcase" => rename(function, "lower"),
        "ucase" => rename(function, "upper"),
        "prefix" => rename(function, "starts_with"),
        "suffix" => rename(function, "ends_with"),
        "regexp_matches" => rename(function, "regexp_like"),
        "log" => rename(function, "log10"),
        "contains" => Some(nested(binary(call("strpos", vec![arg(0)?, arg(1)?]), BinaryOperator::Gt, number("0")))),
        "datepart" => Some(extract(date_field(&text(0)?)?, arg(1)?)),
        "dayname" => Some(call("strftime", vec![arg(0)?, string("%A")])),
        "monthname" => Some(call("strftime", vec![arg(0)?, string("%B")])),
        "date_trunc" => {
            let format = if text(0)? == "year" { "%Y-01-01" } else { "%Y-%m-01" };
            Some(call("date", vec![call("strftime", vec![arg(1)?, string(format)])]))
        }
        "date_add" => Some(nested(binary(arg(0)?, BinaryOperator::Plus, arg(1)?))),
        "date_diff" | "datediff" => {
            let (start, end) = (arg(1)?, arg(2)?);
            let part = |field: DateTimeField, of: &Expr| extract(field, of.clone());
            let years =
                binary(part(DateTimeField::Year, &end), BinaryOperator::Minus, part(DateTimeField::Year, &start));
            Some(nested(match text(0)?.as_str() {
                "day" => binary(
                    cast(end, DataType::Integer(None)),
                    BinaryOperator::Minus,
                    cast(start, DataType::Integer(None)),
                ),
                "month" => binary(
                    binary(nested(years), BinaryOperator::Multiply, number("12")),
                    BinaryOperator::Plus,
                    nested(binary(
                        part(DateTimeField::Month, &end),
                        BinaryOperator::Minus,
                        part(DateTimeField::Month, &start),
                    )),
                ),
                _ => years,
            }))
        }
        "last_day" => {
            let first = call("date", vec![call("strftime", vec![arg(0)?, string("%Y-%m-01")])]);
            let month = Expr::Interval(interval("1 month"));
            let day = Expr::Interval(interval("1 day"));
            Some(nested(binary(binary(first, BinaryOperator::Plus, month), BinaryOperator::Minus, day)))
        }
        "make_date" => {
            let text = call("concat", vec![arg(0)?, string("-"), arg(1)?, string("-"), arg(2)?]);
            Some(call("date", vec![text, string("%Y-%m-%d")]))
        }
        other => Some(extract(date_field(other)?, arg(0)?)),
    }
}

/// Renames the call and keeps it: the result is the call itself.
fn rename(function: &mut Function, name: &str) -> Option<Expr> {
    rename_call(function, name);
    None
}

/// `INTERVAL 'text'`.
fn interval(text: &str) -> Interval {
    Interval {
        value: Box::new(string(text)),
        leading_field: None,
        leading_precision: None,
        last_field: None,
        fractional_seconds_precision: None,
    }
}

#[cfg(test)]
#[path = "rewrite_tests.rs"]
mod rewrite_tests;
