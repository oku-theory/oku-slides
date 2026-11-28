use rstest::rstest;
use speculoos::prelude::*;

use super::{Schemas, Ty, typed_sql};
use crate::{DataError, DataErrorKind};

/// A planner giving every probe one column of type `ty`, or refusing when `ty` is `None`.
struct Fake {
    ty: Option<Ty>,
    probes: usize,
}

impl Schemas for Fake {
    fn columns(&mut self, _sql: &str) -> Result<Vec<(String, Ty)>, DataError> {
        self.probes += 1;
        match self.ty {
            Some(ty) => Ok(vec![("__t".to_owned(), ty)]),
            None => Err(DataError::query_timeout(1)),
        }
    }
}

fn typed(sql: &str, ty: Option<Ty>) -> (Result<String, DataError>, usize) {
    let mut fake = Fake { ty, probes: 0 };
    let out = typed_sql(sql, &mut fake);
    (out, fake.probes)
}

#[test]
fn typed_sql_integer_mod_becomes_truncating_case() {
    let (out, _) = typed("SELECT a % 2 AS m FROM t", Some(Ty::Int));
    assert_that!(out.unwrap()).contains("CASE WHEN");
}

#[rstest]
#[case::modulo("SELECT a % 2 AS m FROM t")]
#[case::div("SELECT a // 2 AS m FROM t")]
#[case::round_digits("SELECT round(a, 2) AS m FROM t")]
fn typed_sql_float_operand_is_refused(#[case] sql: &str) {
    let (out, _) = typed(sql, Some(Ty::Float));
    assert_that!(out.unwrap_err().kind()).is_equal_to(DataErrorKind::QueryFloatOperand);
}

#[test]
fn typed_sql_operand_of_unknown_type_gives_the_probe_refusal() {
    // The fake's probe refuses with a timeout: that error, not a construct refusal, comes back.
    let (out, _) = typed("SELECT a % 2 AS m FROM t", None);
    assert_that!(out.unwrap_err().kind()).is_equal_to(DataErrorKind::QueryTimeout);
}

#[test]
fn typed_sql_decimal_literal_cast_rounds_half_away_without_probe() {
    let (out, probes) = typed("SELECT CAST(-2.5 AS INTEGER) AS m", None);
    assert_that!(out.unwrap()).contains("CAST(-3 AS INTEGER)");
    assert_that!(probes).is_equal_to(0);
}

#[test]
fn typed_sql_query_without_rules_is_unchanged_and_never_probed() {
    let (out, probes) = typed("SELECT a, b FROM t WHERE a > 1", None);
    assert_that!(out.unwrap()).is_equal_to("SELECT a, b FROM t WHERE a > 1".to_owned());
    assert_that!(probes).is_equal_to(0);
}
