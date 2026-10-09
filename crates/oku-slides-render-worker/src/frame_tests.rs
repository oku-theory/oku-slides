use speculoos::prelude::*;

use super::*;
use crate::protocol::Request;

#[test]
fn frame_round_trip_keeps_header_and_payload() {
    let mut buf = Vec::new();
    write_frame(&mut buf, &serde_json::json!({"op": "x"}), b"abc", FrameLimits::uniform(usize::MAX)).unwrap();
    let f = read_frame(&mut buf.as_slice(), FrameLimits::uniform(1024)).unwrap().unwrap();
    assert_that!(serde_json::from_slice::<serde_json::Value>(&f.header).unwrap())
        .is_equal_to(serde_json::json!({"op": "x"}));
    assert_that!(f.payload).is_equal_to(b"abc".to_vec());
}

#[test]
fn read_frame_at_clean_end_returns_none() {
    assert_that!(read_frame(&mut [].as_slice(), FrameLimits::uniform(1024)).unwrap()).is_none();
}

#[test]
fn read_frame_truncated_returns_truncated() {
    let mut buf = Vec::new();
    write_frame(&mut buf, &serde_json::json!({}), b"abcdef", FrameLimits::uniform(usize::MAX)).unwrap();
    buf.truncate(buf.len() - 2);
    assert_that!(matches!(
        read_frame(&mut buf.as_slice(), FrameLimits::uniform(1024)),
        Err(FrameError::Truncated { .. })
    ))
    .is_true();
}

#[test]
fn read_frame_over_max_returns_too_large() {
    let mut buf = Vec::new();
    write_frame(&mut buf, &serde_json::json!({}), &[0u8; 64], FrameLimits::uniform(usize::MAX)).unwrap();
    assert_that!(matches!(read_frame(&mut buf.as_slice(), FrameLimits::uniform(16)), Err(FrameError::TooLarge { .. })))
        .is_true();
}

/// Un lecteur qui répond `Interrupted` à sa première lecture.
struct InterruptedOnce<'a> {
    interrupted: bool,
    inner: &'a [u8],
}

impl std::io::Read for InterruptedOnce<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if !self.interrupted {
            self.interrupted = true;
            return Err(std::io::ErrorKind::Interrupted.into());
        }
        self.inner.read(buf)
    }
}

#[test]
fn read_frame_interrupted_before_first_byte_retries() {
    let mut buf = Vec::new();
    write_frame(&mut buf, &serde_json::json!({"op": "x"}), b"abc", FrameLimits::uniform(usize::MAX)).unwrap();
    let mut r = InterruptedOnce { interrupted: false, inner: &buf };
    let f = read_frame(&mut r, FrameLimits::uniform(1024)).unwrap().unwrap();
    assert_that!(f.payload).is_equal_to(b"abc".to_vec());
}

#[test]
fn write_frame_over_max_returns_too_large_and_writes_nothing() {
    let mut buf = Vec::new();
    let err = write_frame(&mut buf, &serde_json::json!({}), &[0u8; 64], FrameLimits::uniform(16)).unwrap_err();
    assert_that!(matches!(err, FrameError::TooLarge { .. })).is_true();
    assert_that!(buf).is_empty();
}

fn truncated(bytes: &[u8]) -> bool {
    matches!(read_frame(&mut &bytes[..], FrameLimits::uniform(1024)), Err(FrameError::Truncated { .. }))
}

#[test]
fn read_frame_eof_inside_header_length_returns_truncated() {
    assert_that!(truncated(&[0, 0])).is_true();
}

#[test]
fn read_frame_header_shorter_than_its_length_returns_truncated() {
    assert_that!(truncated(&[0, 0, 0, 10, b'{', b'}'])).is_true();
}

#[test]
fn read_frame_eof_inside_payload_length_returns_truncated() {
    assert_that!(truncated(&[0, 0, 0, 2, b'{', b'}', 0])).is_true();
}

#[test]
fn read_frame_header_over_its_own_limit_returns_too_large() {
    let mut buf = Vec::new();
    write_frame(&mut buf, &serde_json::json!({"op": "0123456789"}), b"", FrameLimits::uniform(usize::MAX)).unwrap();
    let limits = FrameLimits { header: 8, payload: 1024 };
    assert_that!(matches!(read_frame(&mut buf.as_slice(), limits), Err(FrameError::TooLarge { .. }))).is_true();
}

#[test]
fn write_frame_header_over_its_own_limit_returns_too_large_and_writes_nothing() {
    let mut buf = Vec::new();
    let limits = FrameLimits { header: 8, payload: 1024 };
    let err = write_frame(&mut buf, &serde_json::json!({"op": "0123456789"}), b"", limits).unwrap_err();
    assert_that!(matches!(err, FrameError::TooLarge { .. })).is_true();
    assert_that!(buf).is_empty();
}

#[test]
fn request_limits_bound_the_header_below_the_payload() {
    assert_that!(FrameLimits::requests(1024)).is_equal_to(FrameLimits { header: 128, payload: 1024 });
}

#[test]
fn reply_limits_bound_svg_below_pdf_and_other_replies_to_no_payload() {
    let svg = Request::Svg { slot: 1, generation: 1, page: 0 };
    let png = Request::Png { slot: 1, generation: 1, page: 0, pixel_per_pt: 1.0 };
    let pdf = Request::Pdf { slot: 1, generation: 1 };
    let open = Request::OpenSlot { slot: 1, fonts: vec![] };
    let payloads = [&svg, &png, &pdf, &open].map(|r| FrameLimits::reply_to(r, 1024).payload);
    assert_that!(payloads).is_equal_to([256, 1024, 1024, 0]);
    assert_that!(FrameLimits::reply_to(&pdf, 1024).header).is_equal_to(128);
}

#[test]
fn read_frame_with_reads_its_limits_after_the_first_byte() {
    let mut buf = Vec::new();
    write_frame(&mut buf, &serde_json::json!({}), &[0u8; 64], FrameLimits::uniform(usize::MAX)).unwrap();
    let read = std::cell::Cell::new(false);
    let mut r = InterruptedOnce { interrupted: false, inner: &buf };
    let limits = || {
        read.set(true);
        FrameLimits { header: 16, payload: 16 }
    };
    assert_that!(matches!(read_frame_with(&mut r, limits), Err(FrameError::TooLarge { .. }))).is_true();
    assert_that!(read.get()).is_true();
}
