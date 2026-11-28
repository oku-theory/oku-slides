//! Errors of a failed parse: the construct when one is recognizable, else the position.
//!
//! A parse error never quotes the SQL: the message names a line and a column only.

use sqlparser::dialect::DuckDbDialect;
use sqlparser::keywords::Keyword;
use sqlparser::parser::ParserError;
use sqlparser::tokenizer::{Token, Tokenizer};

use crate::DataError;
use crate::error::{QueryConstructSnafu, QueryParseSnafu, QueryTooDeepSnafu, quoted};

/// Statement words a query may start with; another keyword first names a refused statement.
const QUERY_STARTS: [Keyword; 3] = [Keyword::SELECT, Keyword::WITH, Keyword::VALUES];

/// `DuckDB` statements the parser does not know as keywords.
const DUCKDB_STATEMENTS: [&str; 1] = ["SUMMARIZE"];

/// The error for a parse failure of `sql`.
pub(super) fn parse_failure(sql: &str, error: &ParserError) -> DataError {
    let message = match error {
        ParserError::RecursionLimitExceeded => return QueryTooDeepSnafu.build().into(),
        ParserError::TokenizerError(message) | ParserError::ParserError(message) => message,
    };
    let (line, column) = position(message).unwrap_or_else(|| end_of(sql));
    if let Some(construct) = refused_construct(sql, (line, column)) {
        return QueryConstructSnafu { construct: quoted(&construct) }.build().into();
    }
    QueryParseSnafu { line, column }.build().into()
}

/// The first word of `sql`, uppercased: the statement a non-query names.
pub(super) fn first_word(sql: &str) -> String {
    match significant_tokens(sql).as_deref() {
        Some([Token::Word(word), ..]) => quoted(&word.value.to_ascii_uppercase()),
        _ => String::new(),
    }
}

/// The construct a text the parser refuses uses, when it is one the whitelist names.
///
/// `SAMPLE` names the failure only next to the error position `error_at`, and `ASOF JOIN` only
/// at or before it: elsewhere they are column or table names, and the failure is another one.
fn refused_construct(sql: &str, error_at: (u64, u64)) -> Option<String> {
    let spanned = Tokenizer::new(&DuckDbDialect {}, sql).tokenize_with_location().ok()?;
    let spanned: Vec<_> = spanned.into_iter().filter(|token| !matches!(token.token, Token::Whitespace(_))).collect();
    let failed = spanned
        .iter()
        .position(|token| (token.span.start.line, token.span.start.column) >= error_at)
        .unwrap_or(spanned.len());
    let tokens: Vec<Token> = spanned.into_iter().map(|token| token.token).collect();
    if let Some(Token::Word(word)) = tokens.first()
        && word.quote_style.is_none()
        && word.keyword != Keyword::NoKeyword
        && word.keyword != Keyword::FROM
        && !QUERY_STARTS.contains(&word.keyword)
    {
        return Some(word.value.to_ascii_uppercase());
    }
    if let Some(Token::Word(word)) = tokens.first()
        && word.quote_style.is_none()
        && DUCKDB_STATEMENTS.contains(&word.value.to_ascii_uppercase().as_str())
    {
        return Some(word.value.to_ascii_uppercase());
    }
    tokens.iter().enumerate().find_map(|(at, token)| {
        let word = |offset: usize| match tokens.get(at + offset) {
            Some(Token::Word(w)) if w.quote_style.is_none() => Some(w.value.to_ascii_uppercase()),
            _ => None,
        };
        match token {
            Token::Mul if word(1).as_deref() == Some("RENAME") => Some("* RENAME".to_owned()),
            Token::Word(w) if w.quote_style.is_none() => match w.value.to_ascii_uppercase().as_str() {
                "SAMPLE" if near(at, failed) || (at == failed + 1 && tokens.get(failed).is_some_and(using)) => {
                    Some("SAMPLE".to_owned())
                }
                // sqlparser reads `ASOF JOIN` and fails later, on the missing `MATCH_CONDITION`.
                "ASOF" if at <= failed && word(1).as_deref() == Some("JOIN") => Some("ASOF JOIN".to_owned()),
                "AT" if tokens.get(at + 1) == Some(&Token::LParen) => match word(2).as_deref() {
                    Some(kind @ ("VERSION" | "TIMESTAMP")) => Some(format!("AT ({kind} => …)")),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    })
}

/// Whether token `at` is the failed one or the one just before it.
fn near(at: usize, failed: usize) -> bool {
    at == failed || at + 1 == failed
}

/// Whether `token` is the unquoted keyword `USING` (`USING SAMPLE` fails on `USING`).
fn using(token: &Token) -> bool {
    matches!(token, Token::Word(word) if word.quote_style.is_none() && word.keyword == Keyword::USING)
}

/// The deepest bracket nesting of `sql`, or `None` when it does not tokenize.
pub(super) fn bracket_nesting(sql: &str) -> Option<usize> {
    let mut depth = 0_usize;
    let mut deepest = 0;
    for token in significant_tokens(sql)? {
        match token {
            Token::LParen | Token::LBracket | Token::LBrace => {
                depth = depth.saturating_add(1);
                deepest = deepest.max(depth);
            }
            Token::RParen | Token::RBracket | Token::RBrace => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Some(deepest)
}

/// The tokens of `sql` without whitespace and comments, or `None` when it does not tokenize.
fn significant_tokens(sql: &str) -> Option<Vec<Token>> {
    let tokens = Tokenizer::new(&DuckDbDialect {}, sql).tokenize().ok()?;
    Some(tokens.into_iter().filter(|token| !matches!(token, Token::Whitespace(_))).collect())
}

/// The `Line: X, Column: Y` a parser message ends with.
fn position(message: &str) -> Option<(u64, u64)> {
    let tail = &message[message.rfind("Line: ")?..];
    let (line, column) = tail.strip_prefix("Line: ")?.split_once(", Column: ")?;
    Some((line.trim().parse().ok()?, column.trim().parse().ok()?))
}

/// The position just past the end of `sql`, where a parse error without one happened.
fn end_of(sql: &str) -> (u64, u64) {
    let lines = sql.split('\n').count();
    let last = sql.rsplit('\n').next().unwrap_or_default().chars().count();
    (u64::try_from(lines).unwrap_or(u64::MAX), u64::try_from(last).unwrap_or(u64::MAX).saturating_add(1))
}
