//! The typed expression rules: integer `%`, `mod` and `//` truncate as in `DuckDB`, `round`
//! takes ties away from zero, an integer `CAST` of a float rounds first; float operands of
//! `%`, `mod`, `//` and `round(x, n)` are refused; `date ± n` and `date - date` count days.

use sqlparser::ast::{BinaryOperator, DataType, Expr, UnaryOperator, Value};

use super::ast::{args, binary, call, cast, function_name, integer_literal, number};
use super::typed::{Scope, Ty, Walk, bounded};
use crate::DataError;
use crate::error::QueryFloatOperandSnafu;

/// The rewrite of `expr`, whose children are already rewritten, when a rule matches it.
pub(super) fn rule(walk: &mut Walk<'_>, expr: &Expr, scope: &Scope) -> Result<Option<Expr>, DataError> {
    match expr {
        Expr::BinaryOp { left, op: BinaryOperator::Modulo, right } => integer(walk, "%", left, right, scope),
        Expr::BinaryOp { left, op: BinaryOperator::DuckIntegerDivide, right } => {
            integer(walk, "//", left, right, scope)
        }
        Expr::BinaryOp { left, op: op @ (BinaryOperator::Plus | BinaryOperator::Minus), right } => {
            days(walk, left, op, right, scope)
        }
        Expr::Function(function) if function.over.is_none() => {
            let a = args(function);
            match (function_name(function).as_deref(), a.as_slice()) {
                (Some("mod"), [left, right]) => integer(walk, "mod", left, right, scope),
                (Some("round"), [x]) => round(walk, x, None, scope),
                (Some("round"), [x, n]) => round(walk, x, Some(n), scope),
                _ => Ok(None),
            }
        }
        Expr::Cast { kind, expr: operand, data_type, format: None }
            if matches!(data_type, DataType::BigInt(None) | DataType::Integer(None) | DataType::Int(None)) =>
        {
            if let Some(rounded) = literal_rounded(operand) {
                let literal = Expr::Cast {
                    kind: kind.clone(),
                    expr: Box::new(rounded),
                    data_type: data_type.clone(),
                    format: None,
                };
                return Ok(Some(literal));
            }
            match walk.ty(operand, scope)? {
                Ty::Float => {
                    let round = call("round", vec![(**operand).clone(), number("0")]);
                    let cast = Expr::Cast {
                        kind: kind.clone(),
                        expr: Box::new(round),
                        data_type: data_type.clone(),
                        format: None,
                    };
                    bounded("CAST", cast).map(Some)
                }
                _ => Ok(None),
            }
        }
        _ => Ok(None),
    }
}

/// The type of a literal, without a probe.
pub(super) fn literal_ty(expr: &Expr) -> Option<Ty> {
    match expr {
        Expr::Value(value) => match &value.value {
            Value::Number(digits, _) if digits.bytes().all(|b| b.is_ascii_digit()) => Some(Ty::Int),
            Value::Number(..) => Some(Ty::Float),
            Value::SingleQuotedString(_) => Some(Ty::Text),
            Value::Boolean(_) => Some(Ty::Bool),
            Value::Null => Some(Ty::Null),
            _ => None,
        },
        Expr::UnaryOp { op: UnaryOperator::Minus | UnaryOperator::Plus, expr } => {
            literal_ty(expr).filter(|ty| matches!(ty, Ty::Int | Ty::Float))
        }
        Expr::Nested(inner) => literal_ty(inner),
        // An interval alone fails its probe when it counts months or years.
        Expr::Interval(_) => Some(Ty::Other),
        _ => None,
    }
}

/// An operand as it reads inside a larger expression: bracketed unless atomic.
fn operand(expr: &Expr) -> Expr {
    match expr {
        Expr::Identifier(_)
        | Expr::CompoundIdentifier(_)
        | Expr::Value(_)
        | Expr::Function(_)
        | Expr::Nested(_)
        | Expr::Cast { .. } => expr.clone(),
        other => Expr::Nested(Box::new(other.clone())),
    }
}

/// `CASE WHEN condition THEN result ELSE otherwise END`.
fn case_when(condition: Expr, result: Expr, otherwise: Expr) -> Expr {
    Expr::Case {
        case_token: sqlparser::ast::helpers::attached_token::AttachedToken::empty(),
        end_token: sqlparser::ast::helpers::attached_token::AttachedToken::empty(),
        operand: None,
        conditions: vec![sqlparser::ast::CaseWhen { condition, result }],
        else_result: Some(Box::new(otherwise)),
    }
}

/// `a % b`, `mod(a, b)` or `a // b` truncated toward zero, as `DuckDB` gives them; polars floors.
fn integer(
    walk: &mut Walk<'_>,
    function: &str,
    left: &Expr,
    right: &Expr,
    scope: &Scope,
) -> Result<Option<Expr>, DataError> {
    let mut ints = true;
    for side in [left, right] {
        match walk.ty(side, scope)? {
            Ty::Float => {
                return Err(QueryFloatOperandSnafu { function, operand: side.to_string() }.build().into());
            }
            Ty::Int | Ty::Null => {}
            _ => ints = false,
        }
    }
    if !ints {
        // Text or dates: the engine refuses them as it runs the query.
        return Ok(None);
    }
    let (a, b) = (operand(left), operand(right));
    let modulo = binary(a.clone(), BinaryOperator::Modulo, b.clone());
    let signs_differ = binary(
        Expr::Nested(Box::new(binary(a.clone(), BinaryOperator::Lt, number("0")))),
        BinaryOperator::NotEq,
        Expr::Nested(Box::new(binary(b.clone(), BinaryOperator::Lt, number("0")))),
    );
    let condition =
        binary(binary(modulo.clone(), BinaryOperator::NotEq, number("0")), BinaryOperator::And, signs_differ);
    let (fixed, plain) = if function == "//" {
        let quotient = binary(a, BinaryOperator::DuckIntegerDivide, b);
        (binary(quotient.clone(), BinaryOperator::Plus, number("1")), quotient)
    } else {
        (binary(modulo.clone(), BinaryOperator::Minus, b), modulo)
    };
    bounded(function, Expr::Nested(Box::new(case_when(condition, fixed, plain)))).map(Some)
}

/// `date ± n` as the date `n` days on and `date - date` as the days between, as `DuckDB` gives
/// them; polars refuses the first and gives a duration for the second.
fn days(
    walk: &mut Walk<'_>,
    left: &Expr,
    op: &BinaryOperator,
    right: &Expr,
    scope: &Scope,
) -> Result<Option<Expr>, DataError> {
    let lhs @ (Ty::Date | Ty::Int) = walk.ty(left, scope)? else { return Ok(None) };
    let rhs = walk.ty(right, scope)?;
    let count = |side: &Expr| cast(operand(side), DataType::BigInt(None));
    let date = |days: Expr| cast(days, DataType::Date);
    let expr = match (lhs, op, rhs) {
        (Ty::Date, _, Ty::Int) => date(binary(count(left), op.clone(), operand(right))),
        (Ty::Int, BinaryOperator::Plus, Ty::Date) => date(binary(operand(left), op.clone(), count(right))),
        (Ty::Date, BinaryOperator::Minus, Ty::Date) => {
            Expr::Nested(Box::new(binary(count(left), op.clone(), count(right))))
        }
        _ => return Ok(None),
    };
    bounded("date", expr).map(Some)
}

/// `round(x)` or `round(x, 0)` of a float with ties away from zero (polars takes them to even);
/// `round(x, n)` of a float with another `n` is refused.
fn round(walk: &mut Walk<'_>, x: &Expr, n: Option<&Expr>, scope: &Scope) -> Result<Option<Expr>, DataError> {
    if walk.ty(x, scope)? != Ty::Float {
        return Ok(None);
    }
    if n.is_some_and(|n| integer_literal(n) != Some(0)) {
        return Err(QueryFloatOperandSnafu { function: "round", operand: x.to_string() }.build().into());
    }
    let x = operand(x);
    let abs = call("abs", vec![x.clone()]);
    let whole = call("floor", vec![abs.clone()]);
    let tie = binary(binary(abs, BinaryOperator::Minus, whole.clone()), BinaryOperator::Eq, number("0.5"));
    let up = binary(whole, BinaryOperator::Plus, number("1"));
    let away = case_when(
        binary(x.clone(), BinaryOperator::Lt, number("0")),
        Expr::UnaryOp { op: UnaryOperator::Minus, expr: Box::new(Expr::Nested(Box::new(up.clone()))) },
        up,
    );
    let rounded = case_when(tie, away, call("round", vec![x, number("0")]));
    bounded("round", Expr::Nested(Box::new(rounded))).map(Some)
}

/// A decimal literal rounded half away from zero, as `DuckDB` casts its `DECIMAL` literals.
fn literal_rounded(expr: &Expr) -> Option<Expr> {
    let (negative, digits) = match expr {
        Expr::Value(value) => match &value.value {
            Value::Number(digits, _) => (false, digits),
            _ => return None,
        },
        Expr::UnaryOp { op: UnaryOperator::Minus, expr } => match expr.as_ref() {
            Expr::Value(value) => match &value.value {
                Value::Number(digits, _) => (true, digits),
                _ => return None,
            },
            _ => return None,
        },
        _ => return None,
    };
    let (whole, fraction) = digits.split_once('.')?;
    if !whole.bytes().chain(fraction.bytes()).all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut whole: Vec<u8> = if whole.is_empty() { b"0".to_vec() } else { whole.as_bytes().to_vec() };
    if fraction.as_bytes().first().is_some_and(|d| *d >= b'5') {
        increment(&mut whole);
    }
    let text = String::from_utf8(whole).ok()?;
    let literal = number(if text.trim_start_matches('0').is_empty() { "0" } else { &text });
    Some(if negative && text.bytes().any(|b| b != b'0') {
        Expr::UnaryOp { op: UnaryOperator::Minus, expr: Box::new(literal) }
    } else {
        literal
    })
}

/// Adds one to a decimal number written in ASCII digits.
fn increment(digits: &mut Vec<u8>) {
    for digit in digits.iter_mut().rev() {
        if *digit == b'9' {
            *digit = b'0';
        } else {
            *digit += 1;
            return;
        }
    }
    digits.insert(0, b'1');
}
