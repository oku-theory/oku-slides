//! The Swift bridge's microphone permission (`oku_speech_microphone`, `OkuSpeech.swift`).
#![expect(unsafe_code, reason = "FFI call into the Swift bridge: one read-only function")]

unsafe extern "C" {
    fn oku_speech_microphone() -> i32;
}

/// The bridge's microphone permission code (see `Microphone::from_code`); never asks.
pub(crate) fn microphone_code() -> i32 {
    // SAFETY: no argument, no shared state; `AVCaptureDevice.authorizationStatus` only reads.
    unsafe { oku_speech_microphone() }
}
