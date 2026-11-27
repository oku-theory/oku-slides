// OkuSpeech: what the voice and duplex bridges (OkuVoice.swift, OkuDuplex.swift) share, and the
// microphone permission.
//
// C ABI (@_cdecl), read by crates/oku-slides-stt/src/microphone_ffi.rs. No `try!` or `!`: a
// crash here would take the app down.

import AVFoundation

/// Releases a context the Rust side lent to the bridge: called once, when the bridge is done with it.
public typealias OkuFree = @convention(c) (UnsafeMutableRawPointer?) -> Void

/// Microphone permission, never requested here: 0 undetermined, 1 restricted, 2 denied, 3 granted.
///
/// Same code table as `Microphone::from_code` in crates/oku-slides-stt/src/microphone.rs.
@_cdecl("oku_speech_microphone")
public func okuSpeechMicrophone() -> Int32 {
    switch AVCaptureDevice.authorizationStatus(for: .audio) {
    case .notDetermined: return 0
    case .restricted: return 1
    case .denied: return 2
    case .authorized: return 3
    @unknown default: return -1
    }
}
