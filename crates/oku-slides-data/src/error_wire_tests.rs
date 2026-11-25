use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use speculoos::prelude::*;

use crate::{DataError, DataErrorKind};

#[rstest]
#[case::base_invalid(DataError::base_invalid("bad footer"), DataErrorKind::BaseInvalid)]
#[case::no_column(DataError::no_column("c1"), DataErrorKind::NoColumn)]
#[case::id_missing(DataError::id_missing(), DataErrorKind::IdMissing)]
fn data_error_round_trips_through_json(#[case] error: DataError, #[case] kind: DataErrorKind) {
    let fr = Localizer::new(Some("fr"));
    let json = serde_json::to_string(&error).unwrap();
    let back: DataError = serde_json::from_str(&json).unwrap();
    assert_that!(back.kind()).is_equal_to(kind);
    assert_that!(back.localize(&fr)).is_equal_to(error.localize(&fr));
}
