//! The capture trampolines, called directly as the bridge would: any host, and under Miri.

use std::ptr::NonNull;

use speculoos::prelude::*;

use super::*;
use crate::audio::{Recorder, Resampler};

/// An engine handle the bridge would return: never dereferenced by these tests.
fn fake_engine() -> *mut c_void {
    NonNull::<u8>::dangling().as_ptr().cast::<c_void>()
}

/// Starts a fake engine on `tap` and returns the capture context it was lent.
fn lend(tap: &Arc<Tap>) -> *mut c_void {
    let mut lent = std::ptr::null_mut();
    let engine = start_with(Arc::clone(tap), |ctx, _| {
        lent = ctx;
        fake_engine()
    });
    assert_that!(engine.map(NonNull::as_ptr)).is_equal_to(Ok(fake_engine()));
    lent
}

#[test]
fn duplex_capture_48k_frames_resample_to_16k_through_the_recorder() {
    let tap = Arc::new(Tap::default());
    let ctx = lend(&tap);
    let (captured, attached) = tap.attach();
    let recorder = Recorder::fed(captured, attached);
    let tenth = vec![0.25f32; 4_800];
    on_frames(ctx, tenth.as_ptr(), 4_800, 48_000);
    let (samples, rate) = recorder.take();
    assert_that!(rate).is_equal_to(48_000);
    let mut out = Vec::new();
    Resampler::new(rate).push(&samples, &mut out);
    assert_that!(out.len().abs_diff(1_600)).is_less_than_or_equal_to(1);
    assert_that!(recorder.level()).is_greater_than(0.0);
    free_tap(ctx);
}

#[test]
fn duplex_frames_after_recorder_finish_are_dropped() {
    let tap = Arc::new(Tap::default());
    let ctx = lend(&tap);
    let (captured, attached) = tap.attach();
    let mut recorder = Recorder::fed(captured, attached);
    recorder.finish();
    on_frames(ctx, [0.5f32; 160].as_ptr(), 160, 16_000);
    assert_that!(recorder.take().0).is_empty();
    free_tap(ctx);
}

#[test]
fn duplex_null_or_negative_frames_are_ignored() {
    let tap = Arc::new(Tap::default());
    let ctx = lend(&tap);
    let (captured, attached) = tap.attach();
    let recorder = Recorder::fed(captured, attached);
    on_frames(ctx, std::ptr::null(), 160, 16_000);
    on_frames(ctx, [0.5f32; 4].as_ptr(), -4, 16_000);
    on_frames(std::ptr::null_mut(), [0.5f32; 4].as_ptr(), 4, 16_000);
    assert_that!(recorder.take().0).is_empty();
    free_tap(ctx);
}

#[test]
fn duplex_refused_start_frees_the_context_and_returns_the_code() {
    let tap = Arc::new(Tap::default());
    let refused = start_with(Arc::clone(&tap), |_, code| {
        // SAFETY (test): `code` points to the `i32` that `start_with` holds for the call.
        unsafe { *code = 1 };
        std::ptr::null_mut()
    });
    assert_that!(refused.map(NonNull::as_ptr)).is_equal_to(Err(1));
    assert_that!(Arc::strong_count(&tap)).is_equal_to(1);
}

#[test]
fn duplex_free_tap_releases_the_tap() {
    let tap = Arc::new(Tap::default());
    let ctx = lend(&tap);
    assert_that!(Arc::strong_count(&tap)).is_equal_to(2);
    free_tap(ctx);
    assert_that!(Arc::strong_count(&tap)).is_equal_to(1);
}
