//! Numbers typed as text, and their display.

/// Significant digits a typed number may carry, so it reads back exactly as an `f64`.
const MAX_DIGITS: usize = 15;

/// A number typed as text: an optional `-` or `+`, digits, at most one `.` or `,` decimal mark.
///
/// At most 15 significant digits; no grouping, exponent, whitespace, infinity or NaN. A comma
/// after one to three digits followed by exactly three is read as grouping and refused.
#[must_use]
pub fn parse_number(s: &str) -> Option<f64> {
    let unsigned = s.strip_prefix(['-', '+']).unwrap_or(s);
    let (int, frac) = match unsigned.split_once(['.', ',']) {
        Some((int, frac)) => (int, Some(frac)),
        None => (unsigned, None),
    };
    let digits = |part: &str| part.bytes().all(|b| b.is_ascii_digit());
    if !digits(int) || !frac.is_none_or(|f| !f.is_empty() && digits(f)) || (int.is_empty() && frac.is_none()) {
        return None;
    }
    if unsigned.contains(',')
        && (1..=3).contains(&int.len())
        && !int.starts_with('0')
        && frac.is_some_and(|f| f.len() == 3)
    {
        return None;
    }
    let significant = int.bytes().chain(frac.unwrap_or("").bytes()).skip_while(|&b| b == b'0').count();
    if significant > MAX_DIGITS {
        return None;
    }
    let sign = if s.starts_with('-') { "-" } else { "" };
    let int = if int.is_empty() { "0" } else { int };
    format!("{sign}{int}.{}", frac.unwrap_or("0")).parse().ok()
}

/// A number as displayed: Rust's shortest exact form, `-0` shown as `0`.
#[must_use]
pub fn format_number(n: f64) -> String {
    if n == 0.0 { "0".to_owned() } else { format!("{n}") }
}

#[cfg(test)]
#[path = "number_tests.rs"]
mod number_tests;
