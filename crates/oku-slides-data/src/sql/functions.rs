//! The function whitelist of phase 2 (spec, "Phase 2 spike", Functions), with each arity.

/// What a whitelisted function is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    /// A function of a row.
    Scalar,
    /// An aggregate; `window` when it may also run `OVER` a window.
    Aggregate { window: bool },
    /// A window-only function: it needs `OVER`.
    Window,
}

/// A whitelisted function: its kind and how many arguments it takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Function {
    pub kind: Kind,
    pub min: usize,
    pub max: usize,
}

/// Most arguments of a variadic function.
const VARIADIC: usize = 255;

const SCALAR: Kind = Kind::Scalar;
const AGGREGATE: Kind = Kind::Aggregate { window: false };
const WINDOW_AGGREGATE: Kind = Kind::Aggregate { window: true };
const WINDOW: Kind = Kind::Window;

/// The whitelist, sorted by name for a binary search.
const FUNCTIONS: [(&str, Kind, usize, usize); 89] = [
    ("abs", SCALAR, 1, 1),
    ("avg", WINDOW_AGGREGATE, 1, 1),
    ("cbrt", SCALAR, 1, 1),
    ("ceiling", SCALAR, 1, 1),
    ("char_length", SCALAR, 1, 1),
    ("coalesce", SCALAR, 1, VARIADIC),
    ("concat", SCALAR, 1, VARIADIC),
    ("concat_ws", SCALAR, 2, VARIADIC),
    ("contains", SCALAR, 2, 2),
    ("cos", SCALAR, 1, 1),
    ("count", WINDOW_AGGREGATE, 1, 1),
    ("covar_pop", AGGREGATE, 2, 2),
    ("covar_samp", AGGREGATE, 2, 2),
    ("date", SCALAR, 1, 1),
    ("date_add", SCALAR, 2, 2),
    ("date_diff", SCALAR, 3, 3),
    ("date_part", SCALAR, 2, 2),
    ("date_trunc", SCALAR, 2, 2),
    ("datediff", SCALAR, 3, 3),
    ("datepart", SCALAR, 2, 2),
    ("day", SCALAR, 1, 1),
    ("dayname", SCALAR, 1, 1),
    ("dayofmonth", SCALAR, 1, 1),
    ("dayofweek", SCALAR, 1, 1),
    ("dayofyear", SCALAR, 1, 1),
    ("degrees", SCALAR, 1, 1),
    ("ends_with", SCALAR, 2, 2),
    ("exp", SCALAR, 1, 1),
    ("first_value", WINDOW, 1, 1),
    ("greatest", SCALAR, 1, VARIADIC),
    ("if", SCALAR, 3, 3),
    ("ifnull", SCALAR, 2, 2),
    ("isodow", SCALAR, 1, 1),
    ("isoyear", SCALAR, 1, 1),
    ("lag", WINDOW, 1, 2),
    ("last_day", SCALAR, 1, 1),
    ("lcase", SCALAR, 1, 1),
    ("lead", WINDOW, 1, 2),
    ("least", SCALAR, 1, VARIADIC),
    ("left", SCALAR, 2, 2),
    ("length", SCALAR, 1, 1),
    ("ln", SCALAR, 1, 1),
    ("log", SCALAR, 1, 1),
    ("log10", SCALAR, 1, 1),
    ("log2", SCALAR, 1, 1),
    ("lower", SCALAR, 1, 1),
    ("ltrim", SCALAR, 1, 1),
    ("make_date", SCALAR, 3, 3),
    ("max", WINDOW_AGGREGATE, 1, 1),
    ("mean", WINDOW_AGGREGATE, 1, 1),
    ("median", AGGREGATE, 1, 1),
    ("min", WINDOW_AGGREGATE, 1, 1),
    ("mod", SCALAR, 2, 2),
    ("month", SCALAR, 1, 1),
    ("monthname", SCALAR, 1, 1),
    ("nullif", SCALAR, 2, 2),
    ("pi", SCALAR, 0, 0),
    ("pow", SCALAR, 2, 2),
    ("power", SCALAR, 2, 2),
    ("prefix", SCALAR, 2, 2),
    ("quantile_cont", AGGREGATE, 2, 2),
    ("quantile_disc", AGGREGATE, 2, 2),
    ("quarter", SCALAR, 1, 1),
    ("radians", SCALAR, 1, 1),
    ("regexp_matches", SCALAR, 2, 2),
    ("replace", SCALAR, 3, 3),
    ("reverse", SCALAR, 1, 1),
    ("right", SCALAR, 2, 2),
    ("round", SCALAR, 1, 2),
    ("row_number", WINDOW, 0, 0),
    ("rtrim", SCALAR, 1, 1),
    ("sign", SCALAR, 1, 1),
    ("sin", SCALAR, 1, 1),
    ("split_part", SCALAR, 3, 3),
    ("sqrt", SCALAR, 1, 1),
    ("starts_with", SCALAR, 2, 2),
    ("stddev", AGGREGATE, 1, 1),
    ("stddev_samp", AGGREGATE, 1, 1),
    ("strftime", SCALAR, 2, 2),
    ("strpos", SCALAR, 2, 2),
    ("strptime", SCALAR, 2, 2),
    ("suffix", SCALAR, 2, 2),
    ("sum", WINDOW_AGGREGATE, 1, 1),
    ("ucase", SCALAR, 1, 1),
    ("upper", SCALAR, 1, 1),
    ("var_samp", AGGREGATE, 1, 1),
    ("variance", AGGREGATE, 1, 1),
    ("week", SCALAR, 1, 1),
    ("year", SCALAR, 1, 1),
];

/// Calls only the rewrites write, with their arity there: `make_date` becomes
/// `date(text, format)`, `regexp_matches` becomes `regexp_like`.
const ENGINE_FUNCTIONS: [(&str, Kind, usize, usize); 2] = [("date", SCALAR, 2, 2), ("regexp_like", SCALAR, 2, 2)];

/// The rewrite-only function named `name` (lowercase) taking `arity` arguments.
pub(super) fn lookup_engine(name: &str, arity: usize) -> Option<Function> {
    ENGINE_FUNCTIONS
        .iter()
        .find(|&&(n, _, min, max)| n == name && (min..=max).contains(&arity))
        .map(|&(_, kind, min, max)| Function { kind, min, max })
}

/// The whitelisted function named `name` (any case).
pub(super) fn lookup(name: &str) -> Option<Function> {
    let name = name.to_ascii_lowercase();
    let at = FUNCTIONS.binary_search_by(|(n, ..)| (*n).cmp(name.as_str())).ok()?;
    FUNCTIONS.get(at).map(|&(_, kind, min, max)| Function { kind, min, max })
}

/// The names of the whitelisted functions, sorted.
pub fn functions() -> impl Iterator<Item = &'static str> {
    FUNCTIONS.iter().map(|(name, ..)| *name)
}

/// Whether `name` is a whitelisted aggregate (any case).
pub(super) fn is_aggregate(name: &str) -> bool {
    matches!(lookup(name), Some(Function { kind: Kind::Aggregate { .. }, .. }))
}

#[cfg(test)]
#[path = "functions_tests.rs"]
mod functions_tests;
