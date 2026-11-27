use speculoos::prelude::*;

use super::*;

#[test]
fn column_reads_letters_and_saturates() {
    assert_that!(column("A1")).is_equal_to(Some(0));
    assert_that!(column("b3")).is_equal_to(Some(1));
    assert_that!(column("XFD1048576")).is_equal_to(Some(16_383));
    assert_that!(column(&"Z".repeat(40))).is_equal_to(Some(u64::MAX - 1));
    assert_that!(column("12")).is_none();
}

#[test]
fn is_date_format_id_built_in_dates_only() {
    for id in [14, 22, 27, 36, 45, 47, 50, 58] {
        assert_that!(is_date_format_id(id)).named(&id.to_string()).is_true();
    }
    for id in [0, 1, 13, 23, 37, 44, 49, 59] {
        assert_that!(is_date_format_id(id)).named(&id.to_string()).is_false();
    }
}

#[test]
fn is_date_format_ignores_literals() {
    assert_that!(is_date_format("yyyy-mm-dd")).is_true();
    assert_that!(is_date_format("[h]:mm:ss")).is_true();
    assert_that!(is_date_format("0.00")).is_false();
    assert_that!(is_date_format("General")).is_false();
    assert_that!(is_date_format("#,##0 \"days\"")).is_false();
    assert_that!(is_date_format("[Red]0;\\d0_)")).is_false();
}
