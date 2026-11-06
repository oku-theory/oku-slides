//! The C ABI of the Swift duplex bridge (`OkuDuplex.swift`): externs, trampolines, contexts.
//!
//! Every `unsafe` block of the duplex audio is here. Ownership of the capture context:
//! - [`start_with`] boxes an `Arc<Tap>` and lends its pointer to the bridge;
//! - the bridge calls [`on_frames`] from its tap while the engine runs, then [`free_tap`] exactly
//!   once, when the engine is released;
//! - if the bridge refuses to start (null handle), it never calls back and [`start_with`] frees
//!   the context itself.
//!
//! Speech contexts are the voice bridge's (`speak::apple_ffi`): one DONE, then `free` once.
//! Only Rust frees a context; the Swift side never does. The trampolines catch panics: none
//! crosses into Swift. Compiled on every host under `cfg(test)`, so their tests also run under Miri.
#![expect(unsafe_code, reason = "FFI bridge to Swift: the only unsafe file of the duplex audio")]

use std::ffi::c_void;
#[cfg(target_os = "macos")]
use std::ffi::{CString, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr::NonNull;
use std::sync::Arc;

use crate::duplex::Tap;
#[cfg(target_os = "macos")]
use crate::speak::SpeechDone;
#[cfg(target_os = "macos")]
use crate::speak::SpeechId;
#[cfg(target_os = "macos")]
use crate::speak::apple_ffi::{self, DoneFn, FreeFn};

/// Microphone frames: mono `Float32` samples, their count, the device rate in hertz.
pub(crate) type FramesFn = extern "C" fn(*mut c_void, *const f32, i32, u32);
/// The bridge has finished with the capture context.
pub(crate) type FreeTapFn = extern "C" fn(*mut c_void);

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn oku_duplex_start(ctx: *mut c_void, frames: FramesFn, free: FreeTapFn, code: *mut i32) -> *mut c_void;
    fn oku_duplex_release(engine: *mut c_void);
    fn oku_duplex_speak(
        engine: *mut c_void,
        text: *const c_char,
        voice: *const c_char,
        rate: f32,
        generation: u64,
        ctx: *mut c_void,
        done: DoneFn,
        free: FreeFn,
    ) -> *mut c_void;
    fn oku_duplex_stop(speech: *mut c_void);
    fn oku_duplex_speech_release(speech: *mut c_void);
}

/// Frames from the bridge's input tap, pushed to the tap's attached recorder.
pub(crate) extern "C" fn on_frames(ctx: *mut c_void, samples: *const f32, count: i32, rate: u32) {
    if ctx.is_null() || samples.is_null() {
        return;
    }
    let Ok(count) = usize::try_from(count) else { return };
    let pushed = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: `ctx` comes from `Box::into_raw` of an `Arc<Tap>` in `start_with`; the bridge
        // calls `on_frames` only before its single `free_tap`, so the box is alive. Shared access.
        let tap = unsafe { &*ctx.cast::<Arc<Tap>>() };
        // SAFETY: the bridge passes `count` initialized `Float32` samples, valid during the call.
        let frames = unsafe { std::slice::from_raw_parts(samples, count) };
        tap.push(frames, rate);
    }));
    if pushed.is_err() {
        tracing::error!("duplex frames callback panicked");
    }
}

/// The bridge has finished with the capture context: Rust frees it.
pub(crate) extern "C" fn free_tap(ctx: *mut c_void) {
    if ctx.is_null() {
        return;
    }
    // SAFETY: `ctx` comes from `Box::into_raw` of an `Arc<Tap>` in `start_with`; the bridge calls
    // `free_tap` once, after its last `on_frames`, so nothing uses the box afterwards.
    let tap = unsafe { Box::from_raw(ctx.cast::<Arc<Tap>>()) };
    if catch_unwind(AssertUnwindSafe(move || drop(tap))).is_err() {
        tracing::error!("duplex capture context drop panicked");
    }
}

/// Lends a capture context over `tap` to `bridge`, which returns the engine or null.
///
/// `bridge` gets the context and where to write its failure code. On null, the bridge never
/// calls back: the context is freed here and the code comes back.
pub(crate) fn start_with(
    tap: Arc<Tap>,
    bridge: impl FnOnce(*mut c_void, *mut i32) -> *mut c_void,
) -> Result<NonNull<c_void>, i32> {
    let ctx = Box::into_raw(Box::new(tap)).cast::<c_void>();
    let mut code = 0i32;
    let engine = NonNull::new(bridge(ctx, &raw mut code));
    if engine.is_none() {
        // A refusal is the bridge's last word on `ctx`: Rust takes it back.
        free_tap(ctx);
    }
    engine.ok_or(code)
}

/// A running duplex engine; dropping it stops the engine and frees the capture context.
#[cfg(target_os = "macos")]
#[derive(Debug)]
pub(crate) struct EngineHandle(NonNull<c_void>);

// SAFETY: the handle names a Swift `DuplexEngine` whose speech state and capture context are
// guarded by locks; `AVAudioEngine`, `AVAudioPlayerNode` and `AVSpeechSynthesizer` accept these
// calls off the main thread. Rust shares the handle in an `Arc` and releases it once, in `Drop`.
#[cfg(target_os = "macos")]
unsafe impl Send for EngineHandle {}
// SAFETY: shared use is sound because the speaker's thread is the only caller of the bridge's
// speak and stop (Swift's `speak` reads `running()` and writes to the synthesizer outside its
// lock), and the engine is released once, by `Drop`, on the last `Arc`'s drop, after every call.
#[cfg(target_os = "macos")]
unsafe impl Sync for EngineHandle {}

/// Starts the duplex engine, its frames going to `tap`; the bridge's failure code otherwise.
#[cfg(target_os = "macos")]
pub(crate) fn start(tap: Arc<Tap>) -> Result<EngineHandle, i32> {
    start_with(tap, |ctx, code| {
        // SAFETY: `code` points to a live `i32` for the call; `ctx` stays valid until the bridge's
        // single `free_tap`, or is freed by `start_with` on null.
        unsafe { oku_duplex_start(ctx, on_frames, free_tap, code) }
    })
    .map(EngineHandle)
}

#[cfg(target_os = "macos")]
impl Drop for EngineHandle {
    fn drop(&mut self) {
        // SAFETY: drops the reference `oku_duplex_start` returned, once: `Drop` runs once.
        unsafe { oku_duplex_release(self.0.as_ptr()) };
    }
}

/// A speech the duplex engine accepted, until Rust releases it.
#[cfg(target_os = "macos")]
#[derive(Debug)]
pub(crate) struct SpeechHandle(NonNull<c_void>);

// SAFETY: the handle names a Swift `DuplexSpeech` whose state is guarded by its engine's lock;
// Rust gives each handle one owner (`current`, then `release`, which consumes it).
#[cfg(target_os = "macos")]
unsafe impl Send for SpeechHandle {}

/// Speaks `text` with `voice` through `engine`, at `rate` times the default rate.
///
/// `None` when the bridge refuses (an unknown voice, a released engine); `done` is then dropped
/// uncalled.
#[cfg(target_os = "macos")]
pub(crate) fn speak(
    engine: &EngineHandle,
    id: SpeechId,
    text: &str,
    voice: &str,
    rate: f32,
    done: SpeechDone,
) -> Option<SpeechHandle> {
    // A NUL would end the C string early: it becomes a space, so all the text is spoken.
    let text = CString::new(text.replace('\0', " ")).ok()?;
    // A voice id holding a NUL names no voice: refused like an unknown one.
    let voice = CString::new(voice).ok()?;
    apple_ffi::start(id, done, |ctx, generation| {
        // SAFETY: `engine` is alive (borrowed); `text` and `voice` live through the call (the bridge
        // copies them); `ctx` stays valid until the bridge's single `free_ctx`, or is freed by
        // `start` on null.
        unsafe {
            oku_duplex_speak(
                engine.0.as_ptr(),
                text.as_ptr(),
                voice.as_ptr(),
                rate,
                generation,
                ctx,
                apple_ffi::on_done,
                apple_ffi::free_ctx,
            )
        }
    })
    .map(SpeechHandle)
}

#[cfg(target_os = "macos")]
impl SpeechHandle {
    /// Cuts the speech: stops the player and the synthesizer; its DONE runs here if still due.
    pub(crate) fn stop(&self) {
        // SAFETY: the handle is valid until `release`, which consumes it.
        unsafe { oku_duplex_stop(self.0.as_ptr()) };
    }

    /// Releases the speech, cutting it; the bridge sends its DONE now if it has not yet.
    pub(crate) fn release(self) {
        // SAFETY: drops the reference `oku_duplex_speak` returned, once: `self` is consumed.
        unsafe { oku_duplex_speech_release(self.0.as_ptr()) };
    }
}

#[cfg(test)]
#[path = "duplex_ffi_tests.rs"]
mod duplex_ffi_tests;
