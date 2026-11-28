use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use speculoos::prelude::*;

use crate::{DataError, DataErrorKind};

#[rstest]
#[case::base_invalid(DataError::base_invalid("bad footer"), DataErrorKind::BaseInvalid)]
#[case::no_column(DataError::no_column("c1"), DataErrorKind::NoColumn)]
#[case::id_missing(DataError::id_missing(), DataErrorKind::IdMissing)]
#[case::source_mismatch(
    crate::error::SourceMismatchSnafu { kind: crate::DataKind::Query }.build().into(),
    DataErrorKind::SourceMismatch
)]
#[case::inline_query(crate::error::InlineQuerySnafu.build().into(), DataErrorKind::InlineQuery)]
#[case::query_parse(crate::error::QueryParseSnafu { line: 2_u64, column: 14_u64 }.build().into(), DataErrorKind::QueryParse)]
#[case::query_construct(
    crate::error::QueryConstructSnafu { construct: "ATTACH" }.build().into(),
    DataErrorKind::QueryConstruct
)]
#[case::query_too_long(crate::error::QueryTooLongSnafu { max: 16_384_usize }.build().into(), DataErrorKind::QueryTooLong)]
#[case::query_too_deep(crate::error::QueryTooDeepSnafu.build().into(), DataErrorKind::QueryTooDeep)]
#[case::query_rewrite(crate::error::QueryRewriteSnafu.build().into(), DataErrorKind::QueryRewrite)]
#[case::query_unknown_function(
    crate::error::QueryUnknownFunctionSnafu { function: "read_csv" }.build().into(),
    DataErrorKind::QueryUnknownFunction
)]
#[case::query_run(crate::error::QueryRunSnafu { reason: "column not found" }.build().into(), DataErrorKind::QueryRun)]
#[case::query_too_large(
    crate::error::QueryTooLargeSnafu { what: crate::error::Bound::Rows, max: 65_536_usize }.build().into(),
    DataErrorKind::QueryTooLarge
)]
#[case::query_column_type(
    crate::error::QueryColumnTypeSnafu { column: "moment" }.build().into(),
    DataErrorKind::QueryColumnType
)]
#[case::query_duplicate_column(
    crate::error::QueryDuplicateColumnSnafu { column: "product" }.build().into(),
    DataErrorKind::QueryDuplicateColumn
)]
#[case::query_float_operand(
    crate::error::QueryFloatOperandSnafu { function: "%", operand: "price" }.build().into(),
    DataErrorKind::QueryFloatOperand
)]
#[case::query_timeout(DataError::query_timeout(30_000), DataErrorKind::QueryTimeout)]
#[case::query_input_failed(DataError::query_input_failed("Ventes"), DataErrorKind::QueryInputFailed)]
fn data_error_round_trips_through_json(#[case] error: DataError, #[case] kind: DataErrorKind) {
    let fr = Localizer::new(Some("fr"));
    let json = serde_json::to_string(&error).unwrap();
    let back: DataError = serde_json::from_str(&json).unwrap();
    assert_that!(back.kind()).is_equal_to(kind);
    assert_that!(back.localize(&fr)).is_equal_to(error.localize(&fr));
}
