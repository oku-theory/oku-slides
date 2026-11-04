//! The C ABI of the Swift voice bridge (`OkuVoice.swift`): externs, trampolines, contexts.
//!
//! Every `unsafe` block of the macOS voice is here. Ownership of one speech's context:
//! - [`start`] boxes a [`Slot`] and lends its pointer to the bridge;
//! - the bridge calls [`on_done`] (any number of times), then [`free_ctx`] exactly once;
//! - if the bridge refuses the speech (null handle), it never calls back and [`start`] frees
//!   the slot itself.
//!
//! Only Rust frees the context; the Swift side never does. DONE takes the [`Ctx`] out of the
//! slot's `Mutex<Option<Ctx>>` when the generation matches, so a second call, or a call for an
//! older generation, does nothing. The trampolines catch panics: none crosses into Swift.
//! Compiled on every host under `cfg(test)`, so the trampoline tests also run under Miri.
#![expect(unsafe_code, reason = "FFI bridge to Swift: the only unsafe file of the OS voice")]

#[cfg(target_os = "macos")]
use std::ffi::CString;
use std::ffi::{CStr, c_char, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Mutex, PoisonError};

use super::{SpeechDone, SpeechId, VoiceInfo};

/// DONE: the speech of this generation ended or was cut.
#[cfg(target_os = "macos")]
pub(crate) type DoneFn = extern "C" fn(*mut c_void, u64);
/// The bridge has finished with the context.
#[cfg(target_os = "macos")]
pub(crate) type FreeFn = extern "C" fn(*mut c_void);
/// One voice of the list: identifier, name, language.
#[cfg(target_os = "macos")]
pub(crate) type VoiceFn = extern "C" fn(*mut c_void, *const c_char, *const c_char, *const c_char);

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn oku_voice_list(ctx: *mut c_void, cb: VoiceFn);
    fn oku_voice_speak(
        text: *const c_char,
        voice: *const c_char,
        rate: f32,
        generation: u64,
        ctx: *mut c_void,
        done: DoneFn,
        free: FreeFn,
    ) -> *mut c_void;
    fn oku_voice_stop(handle: *mut c_void);
    fn oku_voice_release(handle: *mut c_void);
}

/// What DONE hands back: the speech's id and its callback.
#[derive(Debug)]
struct Ctx {
    id: SpeechId,
    done: SpeechDone,
}

/// The context lent to the bridge for one speech; DONE empties it once.
#[derive(Debug)]
struct Slot(Mutex<Option<Ctx>>);

/// DONE from the bridge: calls the speech's callback once, for its own generation only.
pub(crate) extern "C" fn on_done(ctx: *mut c_void, generation: u64) {
    if ctx.is_null() {
        return;
    }
    let called = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: `ctx` comes from `Box::into_raw` of a `Slot` in `start`; the bridge calls
        // `on_done` only before its single `free_ctx`, so the slot is alive. Shared access only:
        // the `Mutex` serializes concurrent calls.
        let slot = unsafe { &*ctx.cast::<Slot>() };
        let taken = {
            let mut g = slot.0.lock().unwrap_or_else(PoisonError::into_inner);
            if g.as_ref().is_some_and(|c| c.id.get() == generation) { g.take() } else { None }
        };
        // Called outside the lock: the callback may start the next speech.
        if let Some(Ctx { id, done }) = taken {
            done.call(id);
        }
    }));
    if called.is_err() {
        tracing::error!(generation, "voice DONE callback panicked");
    }
}

/// The bridge has finished with `ctx`: Rust frees it, with a callback DONE never took.
pub(crate) extern "C" fn free_ctx(ctx: *mut c_void) {
    if ctx.is_null() {
        return;
    }
    // SAFETY: `ctx` comes from `Box::into_raw` of a `Slot` in `start`; the bridge calls
    // `free_ctx` once, after its last `on_done`, so nothing uses the slot afterwards.
    let slot = unsafe { Box::from_raw(ctx.cast::<Slot>()) };
    // A callback dropped uncalled may run arbitrary drop code: keep its panic on this side.
    if catch_unwind(AssertUnwindSafe(move || drop(slot))).is_err() {
        tracing::error!("voice context drop panicked");
    }
}

/// One voice listed by the bridge, appended to the `Vec<VoiceInfo>` behind `ctx`.
pub(crate) extern "C" fn on_voice(ctx: *mut c_void, id: *const c_char, name: *const c_char, lang: *const c_char) {
    if ctx.is_null() || id.is_null() || name.is_null() || lang.is_null() {
        return;
    }
    let listed = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: the bridge calls `on_voice` only during `oku_voice_list`, whose `ctx` is the
        // `&mut Vec<VoiceInfo>` that `voices` holds exclusively for the call.
        let out = unsafe { &mut *ctx.cast::<Vec<VoiceInfo>>() };
        // SAFETY: zero-terminated C strings, valid for the call (`withCString` on the Swift side).
        let text = |p: *const c_char| unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned();
        out.push(VoiceInfo { id: text(id), name: text(name), lang: text(lang) });
    }));
    if listed.is_err() {
        tracing::error!("voice list callback panicked");
    }
}

/// A speech the bridge accepted, until Rust releases it.
#[derive(Debug)]
pub(crate) struct VoiceHandle(*mut c_void);

// SAFETY: the handle names a Swift `VoiceSpeech` whose context and closed flag are guarded by a
// lock; the one process synthesizer it reaches is guarded by the router's lock, and only told
// `speak` and `stopSpeaking`, which `AVSpeechSynthesizer` accepts off the main thread. Rust gives
// each handle one owner (`current`, then `release`, which consumes it).
unsafe impl Send for VoiceHandle {}

/// Lends a fresh context for speech `id` to `bridge`, which returns its handle or null.
///
/// `bridge` gets the context and the generation, and may call [`on_done`] and [`free_ctx`]
/// before it returns. On null, the bridge never calls back: the slot is freed here and `done`
/// is dropped uncalled.
pub(crate) fn start(
    id: SpeechId,
    done: SpeechDone,
    bridge: impl FnOnce(*mut c_void, u64) -> *mut c_void,
) -> Option<VoiceHandle> {
    let ctx = Box::into_raw(Box::new(Slot(Mutex::new(Some(Ctx { id, done }))))).cast::<c_void>();
    let handle = bridge(ctx, id.get());
    if handle.is_null() {
        // A refusal is the bridge's last word on `ctx`: Rust takes it back.
        free_ctx(ctx);
        return None;
    }
    Some(VoiceHandle(handle))
}

/// Every voice of the system.
#[cfg(target_os = "macos")]
pub(crate) fn voices() -> Vec<VoiceInfo> {
    let mut out: Vec<VoiceInfo> = Vec::new();
    // SAFETY: synchronous call; `out` outlives it and only `on_voice` touches it meanwhile.
    unsafe { oku_voice_list((&raw mut out).cast::<c_void>(), on_voice) };
    out
}

/// Speaks `text` with `voice`, at `rate` times the default rate.
///
/// `None` when the bridge refuses (an unknown voice); `done` is then dropped uncalled.
#[cfg(target_os = "macos")]
pub(crate) fn speak(id: SpeechId, text: &str, voice: &str, rate: f32, done: SpeechDone) -> Option<VoiceHandle> {
    // A NUL would end the C string early: it becomes a space, so all the text is spoken.
    let text = CString::new(text.replace('\0', " ")).ok()?;
    // A voice id holding a NUL names no voice: refused like an unknown one.
    let voice = CString::new(voice).ok()?;
    start(id, done, |ctx, generation| {
        // SAFETY: `text` and `voice` live through the call (the bridge copies them); `ctx` stays
        // valid until the bridge's single `free_ctx`, or is freed by `start` on null.
        unsafe { oku_voice_speak(text.as_ptr(), voice.as_ptr(), rate, generation, ctx, on_done, free_ctx) }
    })
}

#[cfg(target_os = "macos")]
impl VoiceHandle {
    /// Cuts the speech; DONE follows, from the bridge.
    pub(crate) fn stop(&self) {
        // SAFETY: the handle is valid until `release`, which consumes it.
        unsafe { oku_voice_stop(self.0) };
    }

    /// Releases the speech, cutting it; the bridge sends its DONE now if it has not yet.
    pub(crate) fn release(self) {
        // SAFETY: drops the reference `oku_voice_speak` returned, once: `self` is consumed.
        unsafe { oku_voice_release(self.0) };
    }
}

#[cfg(test)]
#[path = "apple_ffi_tests.rs"]
mod apple_ffi_tests;
