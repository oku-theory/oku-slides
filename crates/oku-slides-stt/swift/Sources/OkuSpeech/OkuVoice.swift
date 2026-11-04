// OkuVoice: the macOS OS voice (AVSpeechSynthesizer) for Rust.
//
// C ABI (@_cdecl), read by crates/oku-slides-stt/src/speak/apple_ffi.rs. A speech accepted by
// `oku_voice_speak` sends one DONE, `done(ctx, generation)`, then calls `free(ctx)` once; Rust
// frees the context in `free`, Swift never does. A refused speech (nil handle) never calls back.
// Plain text only (`AVSpeechUtterance(string:)`), never SSML. No `try!` and no `!`: a crash here
// would bring the app down. AVFoundation delivers the delegate's finish and cancel on the main queue:
// without a running main run loop (a CLI host), DONE comes only from `oku_voice_release`.
//
// One synthesizer serves the whole process: one per speech kept about 60 KiB each for good
// (measured over 1200 speeches, task 14 fix round 1). The router maps each utterance to its speech.
//
// Threading (ruling R42): the router speaks one at a time, the last speech into the bridge wins:
// `speak` stops the current speech first, as Rust's engine does. Its lock guards only the router
// state (`speeches`, `current`); the synthesizer (`speak`, `stopSpeaking`) and `close` are always
// called outside it, so a delegate call delivered on the calling thread cannot deadlock. Rust calls
// come from worker threads, the delegate from the main queue. Callers serialize `speak`: racing
// calls still give each speech one DONE, but may lose or overlap audio.

import AVFoundation
import Foundation

public typealias OkuVoiceListCallback = @convention(c) (
    UnsafeMutableRawPointer?, UnsafePointer<CChar>?, UnsafePointer<CChar>?, UnsafePointer<CChar>?
) -> Void
public typealias OkuVoiceDone = @convention(c) (UnsafeMutableRawPointer?, UInt64) -> Void

/// One speech: its utterance, and its single DONE then `free`.
final class VoiceSpeech: @unchecked Sendable {
    let utterance: AVSpeechUtterance
    private let lock = NSLock()
    private var ctx: UnsafeMutableRawPointer?
    private let generation: UInt64
    private let done: OkuVoiceDone
    private let free: OkuFree
    private var closed = false

    init(
        _ utterance: AVSpeechUtterance,
        _ ctx: UnsafeMutableRawPointer?,
        _ generation: UInt64,
        _ done: @escaping OkuVoiceDone,
        _ free: @escaping OkuFree
    ) {
        self.utterance = utterance
        self.ctx = ctx
        self.generation = generation
        self.done = done
        self.free = free
    }

    /// DONE then `free`, once; called outside every lock, so DONE may start the next speech.
    func close() {
        lock.lock()
        if closed {
            lock.unlock()
            return
        }
        closed = true
        let lent = ctx
        ctx = nil
        lock.unlock()
        done(lent, generation)
        free(lent)
    }
}

/// The process's synthesizer and its delegate: finish or cancel of an utterance closes its speech.
final class VoiceRouter: NSObject, AVSpeechSynthesizerDelegate, @unchecked Sendable {
    static let shared = VoiceRouter()
    private let synthesizer = AVSpeechSynthesizer()
    private let lock = NSLock()
    /// Speeches not closed yet, by utterance.
    private var speeches: [ObjectIdentifier: VoiceSpeech] = [:]
    /// The utterance spoken last: only its speech may stop the synthesizer.
    private var current: ObjectIdentifier?

    private override init() {
        super.init()
        synthesizer.delegate = self
    }

    /// Speaks `speech`, after stopping the one speaking: one at a time, the last one wins.
    func speak(_ speech: VoiceSpeech) {
        let key = ObjectIdentifier(speech.utterance)
        lock.lock()
        speeches[key] = speech
        let previous = current
        current = key
        lock.unlock()
        if previous != nil {
            synthesizer.stopSpeaking(at: .immediate)
        }
        synthesizer.speak(speech.utterance)
    }

    /// Stops the synthesizer if `speech` is the one it speaks; an older speech stops nothing.
    func stop(_ speech: VoiceSpeech) {
        lock.lock()
        let speaking = current == ObjectIdentifier(speech.utterance)
        if speaking {
            current = nil
        }
        lock.unlock()
        if speaking {
            synthesizer.stopSpeaking(at: .immediate)
        }
    }

    /// Forgets `speech` and closes it (its DONE if still due), outside the lock.
    func finish(_ utterance: AVSpeechUtterance) {
        let key = ObjectIdentifier(utterance)
        lock.lock()
        let speech = speeches.removeValue(forKey: key)
        if current == key {
            current = nil
        }
        lock.unlock()
        speech?.close()
    }

    func speechSynthesizer(_ synthesizer: AVSpeechSynthesizer, didFinish utterance: AVSpeechUtterance) {
        finish(utterance)
    }

    func speechSynthesizer(_ synthesizer: AVSpeechSynthesizer, didCancel utterance: AVSpeechUtterance) {
        finish(utterance)
    }
}

/// Every voice of the system, one `cb(ctx, identifier, name, language)` each, synchronously.
@_cdecl("oku_voice_list")
public func okuVoiceList(_ ctx: UnsafeMutableRawPointer?, _ cb: OkuVoiceListCallback) {
    // Every entry point drains its own pool: a Rust thread has none.
    autoreleasepool {
        for voice in AVSpeechSynthesisVoice.speechVoices() {
            voice.identifier.withCString { id in
                voice.name.withCString { name in
                    voice.language.withCString { lang in cb(ctx, id, name, lang) }
                }
            }
        }
    }
}

/// Speaks `text` with the voice `voice` at `rate` times the default (Rust always names a voice;
/// nil would be the default voice).
///
/// Returns a retained handle, or nil for an unknown voice: then neither `done` nor `free` is called.
@_cdecl("oku_voice_speak")
public func okuVoiceSpeak(
    _ text: UnsafePointer<CChar>?,
    _ voice: UnsafePointer<CChar>?,
    _ rate: Float,
    _ generation: UInt64,
    _ ctx: UnsafeMutableRawPointer?,
    _ done: @escaping OkuVoiceDone,
    _ free: @escaping OkuFree
) -> UnsafeMutableRawPointer? {
    autoreleasepool { () -> UnsafeMutableRawPointer? in
        guard let text else { return nil }
        let utterance = AVSpeechUtterance(string: String(cString: text))
        if let voice {
            guard let chosen = AVSpeechSynthesisVoice(identifier: String(cString: voice)) else { return nil }
            utterance.voice = chosen
        }
        let scaled = AVSpeechUtteranceDefaultSpeechRate * rate
        utterance.rate = min(max(scaled, AVSpeechUtteranceMinimumSpeechRate), AVSpeechUtteranceMaximumSpeechRate)
        let speech = VoiceSpeech(utterance, ctx, generation, done, free)
        VoiceRouter.shared.speak(speech)
        return Unmanaged.passRetained(speech).toOpaque()
    }
}

/// Cuts the speech if it is the one speaking; DONE follows from the delegate or the release.
@_cdecl("oku_voice_stop")
public func okuVoiceStop(_ handle: UnsafeMutableRawPointer?) {
    guard let handle else { return }
    autoreleasepool {
        let speech = Unmanaged<VoiceSpeech>.fromOpaque(handle).takeUnretainedValue()
        VoiceRouter.shared.stop(speech)
    }
}

/// Releases the handle `oku_voice_speak` returned, cutting the speech and sending its DONE if due.
@_cdecl("oku_voice_release")
public func okuVoiceRelease(_ handle: UnsafeMutableRawPointer?) {
    guard let handle else { return }
    autoreleasepool {
        let speech = Unmanaged<VoiceSpeech>.fromOpaque(handle).takeRetainedValue()
        VoiceRouter.shared.stop(speech)
        VoiceRouter.shared.finish(speech.utterance)
        // Never registered or already finished: `close` is idempotent.
        speech.close()
    }
}
