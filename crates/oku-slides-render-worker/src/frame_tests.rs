use speculoos::prelude::*;

use super::*;

#[test]
fn frame_round_trip_keeps_header_and_payload() {
    let mut buf = Vec::new();
    write_frame(&mut buf, &serde_json::json!({"op": "x"}), b"abc", usize::MAX).unwrap();
    let f = read_frame(&mut buf.as_slice(), 1024).unwrap().unwrap();
    assert_that!(serde_json::from_slice::<serde_json::Value>(&f.header).unwrap())
        .is_equal_to(serde_json::json!({"op": "x"}));
    assert_that!(f.payload).is_equal_to(b"abc".to_vec());
}

#[test]
fn read_frame_at_clean_end_returns_none() {
    assert_that!(read_frame(&mut [].as_slice(), 1024).unwrap()).is_none();
}

#[test]
fn read_frame_truncated_returns_truncated() {
    let mut buf = Vec::new();
    write_frame(&mut buf, &serde_json::json!({}), b"abcdef", usize::MAX).unwrap();
    buf.truncate(buf.len() - 2);
    assert_that!(matches!(read_frame(&mut buf.as_slice(), 1024), Err(FrameError::Truncated { .. }))).is_true();
}

#[test]
fn read_frame_over_max_returns_too_large() {
    let mut buf = Vec::new();
    write_frame(&mut buf, &serde_json::json!({}), &[0u8; 64], usize::MAX).unwrap();
    assert_that!(matches!(read_frame(&mut buf.as_slice(), 16), Err(FrameError::TooLarge { .. }))).is_true();
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
    write_frame(&mut buf, &serde_json::json!({"op": "x"}), b"abc", usize::MAX).unwrap();
    let mut r = InterruptedOnce { interrupted: false, inner: &buf };
    let f = read_frame(&mut r, 1024).unwrap().unwrap();
    assert_that!(f.payload).is_equal_to(b"abc".to_vec());
}

#[test]
fn write_frame_over_max_returns_too_large_and_writes_nothing() {
    let mut buf = Vec::new();
    let err = write_frame(&mut buf, &serde_json::json!({}), &[0u8; 64], 16).unwrap_err();
    assert_that!(matches!(err, FrameError::TooLarge { .. })).is_true();
    assert_that!(buf).is_empty();
}
