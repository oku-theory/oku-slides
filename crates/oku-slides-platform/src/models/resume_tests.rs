use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
#[case("bytes 10-35/36", 10, Some((35, Some(36))))]
#[case("bytes 10-35/*", 10, Some((35, None)))]
#[case("bytes 10-10/11", 10, Some((10, Some(11))))]
#[case("bytes 9-35/36", 10, None)]
#[case("bytes 5-4/10", 5, None)]
#[case("bytes 10-35/40", 10, Some((35, Some(40))))]
#[case("bytes 10-35/35", 10, None)]
#[case("bytes 0-18446744073709551615/*", 0, Some((u64::MAX, None)))]
#[case("bytes 0-18446744073709551615/1", 0, None)]
#[case("bytes */36", 10, None)]
#[case("items 10-35/36", 10, None)]
#[case("bytes 10-x/36", 10, None)]
#[case("", 10, None)]
fn parse_content_range_by_header(
    #[case] header: &str,
    #[case] start: u64,
    #[case] expected: Option<(u64, Option<u64>)>,
) {
    assert_that!(parse_content_range(header, start)).is_equal_to(expected);
}

#[rstest]
#[case(10, 206, Some(26), Some("bytes 10-35/36"), Answer::Append { total: Some(36) })]
#[case(10, 206, None, Some("bytes 10-35/*"), Answer::Append { total: Some(36) })]
#[case(10, 206, Some(5), Some("bytes 10-35/36"), Answer::Refuse)]
#[case(10, 206, Some(26), Some("bytes 0-25/36"), Answer::Refuse)]
#[case(10, 206, Some(26), None, Answer::Refuse)]
#[case(0, 206, Some(36), Some("bytes 0-35/36"), Answer::Refuse)]
#[case(10, 206, Some(26), Some("bytes 10-35/40"), Answer::Append { total: Some(40) })]
#[case(10, 206, None, Some("bytes 10-18446744073709551615/*"), Answer::Refuse)]
#[case(10, 200, Some(36), None, Answer::Fresh { total: Some(36) })]
#[case(0, 200, None, None, Answer::Fresh { total: None })]
fn answer_by_status_and_range(
    #[case] from: u64,
    #[case] status: u16,
    #[case] length: Option<u64>,
    #[case] range: Option<&str>,
    #[case] expected: Answer,
) {
    assert_that!(answer(from, status, length, range)).is_equal_to(expected);
}

#[rstest]
#[case(ureq::Error::StatusCode(429), true)]
#[case(ureq::Error::StatusCode(500), true)]
#[case(ureq::Error::StatusCode(502), true)]
#[case(ureq::Error::StatusCode(503), true)]
#[case(ureq::Error::StatusCode(504), true)]
#[case(ureq::Error::StatusCode(404), false)]
#[case(ureq::Error::StatusCode(416), false)]
#[case(ureq::Error::Io(std::io::Error::from(std::io::ErrorKind::ConnectionReset)), true)]
#[case(ureq::Error::Timeout(ureq::Timeout::RecvResponse), true)]
#[case(ureq::Error::HostNotFound, false)]
fn retryable_by_error(#[case] error: ureq::Error, #[case] expected: bool) {
    assert_that!(retryable(&error)).is_equal_to(expected);
}
