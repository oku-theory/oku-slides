//! The trampolines, called directly as the bridge would: any host, and under Miri.

use std::ffi::{CString, c_void};
use std::ptr::NonNull;
use std::sync::{Arc, Mutex, PoisonError};

use speculoos::prelude::*;

use super::*;

/// The ids a [`SpeechDone`] was called with, and a witness of the callback still alive.
#[derive(Debug, Default)]
struct Record {
    ids: Mutex<Vec<SpeechId>>,
}

impl Record {
    fn ids(&self) -> Vec<SpeechId> {
        self.ids.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }
}

fn recording(record: &Arc<Record>) -> SpeechDone {
    let record = Arc::clone(record);
    SpeechDone::new(move |id| record.ids.lock().unwrap_or_else(PoisonError::into_inner).push(id))
}

/// A handle the bridge would return: never dereferenced by these tests.
fn fake_handle() -> *mut c_void {
    NonNull::<u8>::dangling().as_ptr().cast::<c_void>()
}

/// Starts speech `id` on a fake bridge and returns the context it was lent.
fn lend(id: SpeechId, record: &Arc<Record>) -> *mut c_void {
    let mut lent = std::ptr::null_mut();
    let handle = start(id, recording(record), |ctx, generation| {
        assert_that!(generation).is_equal_to(id.get());
        lent = ctx;
        fake_handle()
    })
    .unwrap_or_else(|| panic!("the fake bridge accepted the speech"));
    assert_that!(handle.as_ptr()).is_equal_to(fake_handle());
    lent
}

#[test]
fn apple_callbacks_called_twice_free_once() {
    let record = Arc::new(Record::default());
    let ctx = lend(SpeechId::new(7), &record);
    on_done(ctx, 7);
    on_done(ctx, 7);
    assert_that!(record.ids()).is_equal_to(vec![SpeechId::new(7)]);
    free_ctx(ctx);
    // The slot, and the callback with it, is gone: only the test's reference is left.
    assert_that!(Arc::strong_count(&record)).is_equal_to(1);
}

#[test]
fn apple_done_for_older_generation_is_ignored() {
    let record = Arc::new(Record::default());
    let ctx = lend(SpeechId::new(5), &record);
    on_done(ctx, 4);
    assert_that!(record.ids()).is_empty();
    on_done(ctx, 5);
    assert_that!(record.ids()).is_equal_to(vec![SpeechId::new(5)]);
    free_ctx(ctx);
}

#[test]
fn apple_free_without_done_drops_callback_uncalled() {
    let record = Arc::new(Record::default());
    let ctx = lend(SpeechId::new(3), &record);
    free_ctx(ctx);
    assert_that!(record.ids()).is_empty();
    assert_that!(Arc::strong_count(&record)).is_equal_to(1);
}

#[test]
fn apple_refused_speech_frees_context_uncalled() {
    let record = Arc::new(Record::default());
    let handle = start(SpeechId::new(2), recording(&record), |_, _| std::ptr::null_mut());
    assert_that!(handle.is_none()).is_true();
    assert_that!(record.ids()).is_empty();
    assert_that!(Arc::strong_count(&record)).is_equal_to(1);
}

#[test]
fn done_before_speak_returns_is_matched() {
    let record = Arc::new(Record::default());
    // The bridge finishes at once, inside the speak call, then frees the context.
    let handle = start(SpeechId::new(9), recording(&record), |ctx, generation| {
        on_done(ctx, generation);
        free_ctx(ctx);
        fake_handle()
    });
    assert_that!(handle.is_some()).is_true();
    assert_that!(record.ids()).is_equal_to(vec![SpeechId::new(9)]);
    assert_that!(Arc::strong_count(&record)).is_equal_to(1);
}

#[test]
fn apple_done_callback_panic_is_caught() {
    let mut lent = std::ptr::null_mut();
    start(SpeechId::new(1), SpeechDone::new(|_| panic!("callback panics")), |ctx, _| {
        lent = ctx;
        fake_handle()
    })
    .unwrap_or_else(|| panic!("the fake bridge accepted the speech"));
    on_done(lent, 1);
    on_done(lent, 1);
    free_ctx(lent);
}

#[test]
fn apple_voice_callback_appends_voice() {
    let mut out: Vec<VoiceInfo> = Vec::new();
    let (id, name, lang) = (CString::new("com.apple.voice.Thomas"), CString::new("Thomas"), CString::new("fr-FR"));
    let (id, name, lang) = (
        id.unwrap_or_else(|e| panic!("{e}")),
        name.unwrap_or_else(|e| panic!("{e}")),
        lang.unwrap_or_else(|e| panic!("{e}")),
    );
    let ctx = (&raw mut out).cast::<c_void>();
    on_voice(ctx, id.as_ptr(), name.as_ptr(), lang.as_ptr());
    on_voice(ctx, std::ptr::null(), name.as_ptr(), lang.as_ptr());
    assert_that!(out).is_equal_to(vec![VoiceInfo {
        id: "com.apple.voice.Thomas".into(),
        name: "Thomas".into(),
        lang: "fr-FR".into(),
    }]);
}
