//! The C ABI of the Swift bridge (`swift/`), and its codes.
//!
//! The only `unsafe` code of the crate. `oku_model_respond` sends one event to [`on_event`]
//! from a Swift task, then calls [`free_ctx`] once: the context (a boxed `Sender<Outcome>`)
//! lives until then. Off macOS no bridge exists: the model is reported unavailable.
#![cfg_attr(target_os = "macos", expect(unsafe_code, reason = "FFI bridge to Swift: the crate's only unsafe file"))]

use crate::{AppleModel, ErrorKind};
#[cfg(not(target_os = "macos"))]
use crate::{AppleModelError, Request};
#[cfg(not(target_os = "macos"))]
use tokio_util::sync::CancellationToken;

/// Availability codes of `oku_model_availability`, as in `OkuModel.swift`.
#[cfg(any(test, target_os = "macos"))]
pub(crate) const AVAILABILITY: [(i32, AppleModel); 6] = [
    (0, AppleModel::Ready),
    (1, AppleModel::DeviceNotEligible),
    (2, AppleModel::IntelligenceOff),
    (3, AppleModel::ModelNotReady),
    (4, AppleModel::MacOsTooOld),
    (5, AppleModel::Unknown),
];

/// Error codes of an error event, as in `OkuModel.swift`.
#[cfg(any(test, target_os = "macos"))]
pub(crate) const ERRORS: [(i32, ErrorKind); 4] =
    [(1, ErrorKind::ContextOverflow), (2, ErrorKind::Refused), (3, ErrorKind::Failed), (4, ErrorKind::Cancelled)];

/// The state for an availability code; an unknown code is `Unknown`.
#[cfg(any(test, target_os = "macos"))]
pub(crate) fn model_of(code: i32) -> AppleModel {
    AVAILABILITY.iter().find(|(c, _)| *c == code).map_or(AppleModel::Unknown, |(_, m)| *m)
}

/// The error kind for an error code; an unknown code is `Failed`.
#[cfg(any(test, target_os = "macos"))]
pub(crate) fn kind_of(code: i32) -> ErrorKind {
    ERRORS.iter().find(|(c, _)| *c == code).map_or(ErrorKind::Failed, |(_, k)| *k)
}

/// `value` as a C string, each interior NUL replaced by U+FFFD so it does not fail the turn.
///
/// A NUL can come from any text the turn quotes (an imported file, a tool reply). The error
/// stays for the type's sake: no NUL is left to refuse.
#[cfg(any(test, target_os = "macos"))]
pub(crate) fn c_text(value: &str) -> Result<std::ffi::CString, std::ffi::NulError> {
    std::ffi::CString::new(value.replace('\0', "\u{FFFD}"))
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn availability() -> AppleModel {
    AppleModel::MacOsTooOld
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn respond(_request: &Request<'_>, _cancel: &CancellationToken) -> Result<String, AppleModelError> {
    Err(AppleModelError::of(ErrorKind::Failed, "Apple's on-device model exists only on macOS".to_owned()))
}

#[cfg(target_os = "macos")]
pub(crate) use mac::{availability, respond};

#[cfg(target_os = "macos")]
mod mac {
    use std::ffi::{CStr, CString, c_char, c_void};
    use std::sync::mpsc;
    use std::time::Duration;

    use super::{c_text, kind_of, model_of};
    use crate::{AppleModel, AppleModelError, ErrorKind, Request};
    use tokio_util::sync::CancellationToken;

    /// How often a waiting reply checks its cancel token.
    const POLL: Duration = Duration::from_millis(25);

    /// The event kind of a reply (any other kind is an error).
    const KIND_REPLY: i32 = 0;

    /// The one event of a reply: the generated JSON, or an error kind and message.
    type Outcome = Result<String, (ErrorKind, String)>;

    type Callback = extern "C" fn(*mut c_void, i32, i32, *const c_char);
    type Free = extern "C" fn(*mut c_void);

    unsafe extern "C" {
        fn oku_model_availability() -> i32;
        fn oku_model_respond(
            instructions: *const c_char,
            prompt: *const c_char,
            schema: *const c_char,
            temperature: f64,
            max_tokens: i32,
            ctx: *mut c_void,
            cb: Callback,
            free: Free,
        ) -> *mut c_void;
        fn oku_model_cancel(handle: *mut c_void);
        fn oku_model_release(handle: *mut c_void);
    }

    /// The bridge's event, sent to the waiting reply.
    extern "C" fn on_event(ctx: *mut c_void, kind: i32, code: i32, text: *const c_char) {
        if ctx.is_null() {
            return;
        }
        // SAFETY: `ctx` comes from `Box::into_raw` of a `Sender<Outcome>` (`respond`); the
        // bridge calls `on_event` at most once, under its lock, before `free_ctx`.
        let tx = unsafe { &*ctx.cast::<mpsc::Sender<Outcome>>() };
        let text = if text.is_null() {
            String::new()
        } else {
            // SAFETY: a NUL-terminated C string, valid during the call (`withCString`).
            unsafe { CStr::from_ptr(text) }.to_string_lossy().into_owned()
        };
        let outcome = if kind == KIND_REPLY { Ok(text) } else { Err((kind_of(code), text)) };
        // A receiver gone: the reply was cancelled and nobody waits any more.
        tx.send(outcome).ok();
    }

    /// The bridge is done with this context.
    extern "C" fn free_ctx(ctx: *mut c_void) {
        if ctx.is_null() {
            return;
        }
        // SAFETY: called once by the bridge, after its last `on_event`.
        drop(unsafe { Box::from_raw(ctx.cast::<mpsc::Sender<Outcome>>()) });
    }

    pub(crate) fn availability() -> AppleModel {
        // SAFETY: no argument and no shared state; the bridge checks the macOS version itself.
        model_of(unsafe { oku_model_availability() })
    }

    /// A reply in flight on the bridge, released on drop.
    struct Handle(*mut c_void);

    impl Handle {
        fn cancel(&self) {
            if !self.0.is_null() {
                // SAFETY: the handle stays valid until `oku_model_release` (in `drop`).
                unsafe { oku_model_cancel(self.0) };
            }
        }
    }

    impl Drop for Handle {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: releases the reference `oku_model_respond` took, once.
                unsafe { oku_model_release(self.0) };
            }
        }
    }

    fn text(field: &str, value: &str) -> Result<CString, AppleModelError> {
        c_text(value).map_err(|_| AppleModelError::of(ErrorKind::Failed, format!("{field} contains a NUL byte")))
    }

    pub(crate) fn respond(request: &Request<'_>, cancel: &CancellationToken) -> Result<String, AppleModelError> {
        let instructions = text("instructions", request.instructions)?;
        let prompt = text("prompt", request.prompt)?;
        let schema = text("schema", request.schema)?;
        // The bridge takes an `Int32`; a larger cap means "no cap" in practice, so clamping
        // keeps the caller's intent (the context is 4,096 tokens anyway).
        let max_tokens = i32::try_from(request.max_tokens).unwrap_or(i32::MAX);
        let (tx, rx) = mpsc::channel::<Outcome>();
        let ctx = Box::into_raw(Box::new(tx)).cast::<c_void>();
        // SAFETY: the strings live during the call and the bridge copies them; it keeps `ctx`
        // until `free_ctx`. A null handle (macOS too old): the error comes through `ctx`.
        let handle = Handle(unsafe {
            oku_model_respond(
                instructions.as_ptr(),
                prompt.as_ptr(),
                schema.as_ptr(),
                request.temperature,
                max_tokens,
                ctx,
                on_event,
                free_ctx,
            )
        });
        loop {
            match rx.recv_timeout(POLL) {
                Ok(Ok(json)) => return Ok(json),
                Ok(Err((kind, message))) => return Err(AppleModelError::of(kind, message)),
                Err(mpsc::RecvTimeoutError::Timeout) if cancel.is_cancelled() => {
                    // The Swift task ends on its own; its late event finds no receiver.
                    handle.cancel();
                    return Err(AppleModelError::of(ErrorKind::Cancelled, String::new()));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(AppleModelError::of(ErrorKind::Failed, "the bridge ended without a reply".to_owned()));
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "bridge_tests.rs"]
mod bridge_tests;
