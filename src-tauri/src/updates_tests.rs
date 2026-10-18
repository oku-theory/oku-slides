use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
#[case(CheckTimeoutSnafu.build(), UpdateStage::Check)]
#[case(TooLargeSnafu.build(), UpdateStage::Download)]
#[case(UntrustedSnafu { url: "x".to_owned() }.build(), UpdateStage::Download)]
#[case(UnavailableSnafu.build(), UpdateStage::Unavailable)]
#[case(RefusedSnafu { why: "agent window" }.build(), UpdateStage::Refused)]
fn stage_of_error_names_its_message(#[case] inner: Inner, #[case] expected: UpdateStage) {
    assert_that!(stage_of(&inner)).is_equal_to(expected);
}
