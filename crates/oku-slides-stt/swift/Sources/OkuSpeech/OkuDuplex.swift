// OkuDuplex: one AVAudioEngine with voice-processing I/O for Rust: the echo-cancelled microphone
// and Oku's synthesized speech on the same unit, so the canceller has the speech as its reference
// and removes it from what the microphone hears.
//
// C ABI (@_cdecl), read by crates/oku-slides-stt/src/duplex_ffi.rs; failure codes shared with
// `raw` in duplex.rs. `oku_duplex_start` lends a capture context: `frames(ctx, samples, count,
// rate)` from the input tap, mono Float32 at the device rate, then `free(ctx)` once when the engine
// is released; on failure it returns nil with the code written and never calls back. A speech
// accepted by `oku_duplex_speak` keeps OkuVoice's contract (`VoiceSpeech`): one DONE then
// `free(ctx)`, sent when its last buffer has played or when it is cut. Rust frees every context,
// Swift never does. No `try!` and no `!`: a crash here would bring the app down.
//
// Speech: `AVSpeechSynthesizer.write` renders the utterance into buffers (it plays nothing
// itself), delivered on the main queue only: the engine starts only where that queue is drained
// (code 4 elsewhere). Each buffer is converted to the player's format (mono Float32 at the
// mixer's rate) and scheduled on an `AVAudioPlayerNode` of the main mixer, so it goes out through
// the voice-processing unit. One speech at a time, the last one in wins, as in OkuVoice.
//
// Threading: Rust calls from worker threads, buffers come on the main queue, completions from the
// player's thread, frames from the tap's. The engine's lock guards only the speech state; every
// DONE, and every call to the engine, player or synthesizer, runs outside it.
//
// Availability (macOS SDK headers): `setVoiceProcessingEnabled` and `write(_:toBufferCallback:)`
// 10.15, `AVAudioConverter` 10.11, `scheduleBuffer(_:completionCallbackType:)` 10.13: all within the
// 12.0 minimum. The other-audio ducking setting is macOS 14+, set only there.

import AVFoundation
import Foundation

public typealias OkuDuplexFrames = @convention(c) (UnsafeMutableRawPointer?, UnsafePointer<Float>?, Int32, UInt32) -> Void

// Failure codes of `oku_duplex_start`, the same as `raw` in duplex.rs (3 and 4: its other codes).
private let duplexEchoCancel: Int32 = 1
private let duplexNoInput: Int32 = 2
private let duplexEngineStart: Int32 = 3
private let duplexNoMainLoop: Int32 = 4

/// Whether the main queue is drained: the synthesizer delivers its buffers only there.
///
/// A host without a main run loop (the CLI dev server, a test harness keeping the main thread)
/// would play nothing: the engine refuses to start there rather than stay silent.
private func mainQueueRuns() -> Bool {
    if Thread.isMainThread { return true }
    let ran = DispatchSemaphore(value: 0)
    DispatchQueue.main.async { ran.signal() }
    return ran.wait(timeout: .now() + .milliseconds(500)) == .success
}

/// Why the engine did not start: one of the codes above.
struct DuplexFailure: Error {
    let code: Int32
}

/// The capture context lent by Rust: frames while open, then `free` once.
final class DuplexCapture: @unchecked Sendable {
    private let lock = NSLock()
    private var ctx: UnsafeMutableRawPointer?
    private let frames: OkuDuplexFrames
    private let free: OkuFree
    private var closed = false

    init(_ ctx: UnsafeMutableRawPointer?, _ frames: @escaping OkuDuplexFrames, _ free: @escaping OkuFree) {
        self.ctx = ctx
        self.frames = frames
        self.free = free
    }

    /// Sends `count` mono samples at `rate` hertz; nothing once closed.
    func send(_ samples: UnsafePointer<Float>, _ count: AVAudioFrameCount, _ rate: UInt32) {
        lock.lock()
        defer { lock.unlock() }
        if closed || count == 0 { return }
        frames(ctx, samples, Int32(clamping: count), rate)
    }

    /// Stops the frames, then calls `free` once if `owned`: never for an engine that did not start.
    func close(owned: Bool) {
        lock.lock()
        if closed {
            lock.unlock()
            return
        }
        closed = true
        let lent = ctx
        ctx = nil
        lock.unlock()
        if owned { free(lent) }
    }
}

/// Mono Float32 at the device rate from the input tap's buffers, whatever their layout.
final class MonoTap: @unchecked Sendable {
    private let mono: AVAudioFormat
    /// nil when the input is already deinterleaved Float32: its first channel is taken as is.
    private let converter: AVAudioConverter?
    private let rate: UInt32
    private let capture: DuplexCapture

    /// nil for an input with no channel or no usable rate: no microphone.
    init?(_ input: AVAudioFormat, _ capture: DuplexCapture) {
        guard input.channelCount > 0, input.sampleRate > 0,
              let rate = UInt32(exactly: input.sampleRate.rounded()),
              let mono = AVAudioFormat(
                  commonFormat: .pcmFormatFloat32, sampleRate: input.sampleRate, channels: 1, interleaved: false)
        else { return nil }
        if input.commonFormat == .pcmFormatFloat32 && !input.isInterleaved {
            converter = nil
        } else {
            guard let converter = AVAudioConverter(from: input, to: mono) else { return nil }
            self.converter = converter
        }
        self.mono = mono
        self.rate = rate
        self.capture = capture
    }

    /// The voice-processed signal of `buffer`: its first channel, converted when needed.
    ///
    /// With voice processing on, macOS reports several input channels (9 on a MacBook Pro), each
    /// carrying the same processed signal: the first one is the whole of it.
    func deliver(_ buffer: AVAudioPCMBuffer) {
        guard let converter else {
            if let data = buffer.floatChannelData { capture.send(data[0], buffer.frameLength, rate) }
            return
        }
        guard let out = AVAudioPCMBuffer(pcmFormat: mono, frameCapacity: buffer.frameLength),
              (try? converter.convert(to: out, from: buffer)) != nil,
              let data = out.floatChannelData
        else { return }
        capture.send(data[0], out.frameLength, rate)
    }
}

/// One speech of the duplex engine: its `VoiceSpeech`, and its buffers' progress.
final class DuplexSpeech: @unchecked Sendable {
    let speech: VoiceSpeech
    let engine: DuplexEngine
    // Guarded by the engine's lock.
    /// Buffers scheduled on the player and not played back yet.
    var pending = 0
    /// The synthesizer has delivered its last buffer (or failed): DONE once `pending` is 0.
    var written = false
    /// From the voice's format to the player's, made on the first buffer.
    var converter: AVAudioConverter?

    init(_ speech: VoiceSpeech, _ engine: DuplexEngine) {
        self.speech = speech
        self.engine = engine
    }
}

/// The engine: voice-processing input tapped for Rust, a player node for Oku's speech.
final class DuplexEngine: @unchecked Sendable {
    private let engine: AVAudioEngine
    private let player: AVAudioPlayerNode
    private let synthesizer = AVSpeechSynthesizer()
    private let capture: DuplexCapture
    /// The player's format: mono Float32 at the main mixer's rate.
    private let format: AVAudioFormat
    private let lock = NSLock()
    /// Orders scheduling a buffer after the check that its speech is current, against `halt`.
    ///
    /// Taken before `lock`, never inside it: a stopped player may call back `played`, which
    /// takes `lock`, from `halt`. Without it, a buffer of a speech cut between that check and
    /// `scheduleBuffer` would play at the start of the next speech.
    private let playerLock = NSLock()
    /// The speech playing, until it ends or is cut.
    private var current: DuplexSpeech?
    private var closed = false

    private init(_ engine: AVAudioEngine, _ player: AVAudioPlayerNode, _ capture: DuplexCapture, _ format: AVAudioFormat) {
        self.engine = engine
        self.player = player
        self.capture = capture
        self.format = format
    }

    /// Builds and starts the engine with voice processing on, its frames going to `capture`.
    static func start(_ capture: DuplexCapture) -> Result<DuplexEngine, DuplexFailure> {
        guard mainQueueRuns() else { return .failure(DuplexFailure(code: duplexNoMainLoop)) }
        let engine = AVAudioEngine()
        let input = engine.inputNode
        do {
            // Before reading any format: it changes them (often to several channels).
            try input.setVoiceProcessingEnabled(true)
        } catch {
            return .failure(DuplexFailure(code: duplexEchoCancel))
        }
        if #available(macOS 14.0, *) {
            // The least ducking of other apps' audio while the voice processing runs.
            input.voiceProcessingOtherAudioDuckingConfiguration =
                AVAudioVoiceProcessingOtherAudioDuckingConfiguration(enableAdvancedDucking: false, duckingLevel: .min)
        }
        let inputFormat = input.outputFormat(forBus: 0)
        guard let tap = MonoTap(inputFormat, capture) else {
            return .failure(DuplexFailure(code: duplexNoInput))
        }
        let mixerRate = engine.mainMixerNode.outputFormat(forBus: 0).sampleRate
        guard mixerRate > 0, let format = AVAudioFormat(standardFormatWithSampleRate: mixerRate, channels: 1) else {
            return .failure(DuplexFailure(code: duplexEngineStart))
        }
        let player = AVAudioPlayerNode()
        engine.attach(player)
        engine.connect(player, to: engine.mainMixerNode, format: format)
        input.installTap(onBus: 0, bufferSize: 4096, format: inputFormat) { buffer, _ in tap.deliver(buffer) }
        engine.prepare()
        do {
            try engine.start()
        } catch {
            input.removeTap(onBus: 0)
            return .failure(DuplexFailure(code: duplexEngineStart))
        }
        player.play()
        return .success(DuplexEngine(engine, player, capture, format))
    }

    /// Whether the engine runs, restarting it if the system stopped it (a device change).
    private func running() -> Bool {
        if engine.isRunning { return true }
        do {
            try engine.start()
        } catch {
            return false
        }
        return true
    }

    /// Speaks `s`, after cutting the one speaking; false (nothing called back) if it cannot play.
    func speak(_ s: DuplexSpeech) -> Bool {
        // A closed engine never restarts: its voice processing would hold the microphone.
        lock.lock()
        let wasClosed = closed
        lock.unlock()
        guard !wasClosed, running() else { return false }
        lock.lock()
        if closed {
            lock.unlock()
            return false
        }
        let previous = current
        current = s
        lock.unlock()
        if let previous {
            halt()
            previous.speech.close()
        }
        if !player.isPlaying { player.play() }
        synthesizer.write(s.speech.utterance) { [weak self] buffer in self?.receive(buffer, for: s) }
        return true
    }

    /// Cuts `s` if it is the one speaking, then sends its DONE if still due.
    func stop(_ s: DuplexSpeech) {
        lock.lock()
        let speaking = current === s
        if speaking { current = nil }
        lock.unlock()
        if speaking { halt() }
        s.speech.close()
    }

    /// Cuts the speech, stops the engine and frees the capture context: the engine's last call.
    func close() {
        lock.lock()
        if closed {
            lock.unlock()
            return
        }
        closed = true
        let speaking = current
        current = nil
        lock.unlock()
        halt()
        speaking?.speech.close()
        engine.inputNode.removeTap(onBus: 0)
        engine.stop()
        capture.close(owned: true)
    }

    /// Silences the synthesizer and the player; the cut speech's late buffers are ignored.
    ///
    /// Called once `current` no longer names the cut speech, so no buffer of it is scheduled
    /// after the player stops (`schedule` checks `current` under `playerLock`).
    private func halt() {
        synthesizer.stopSpeaking(at: .immediate)
        playerLock.lock()
        player.stop()
        playerLock.unlock()
    }

    /// Schedules `out` on the player if `s` is still the speech playing; false if it was cut.
    private func schedule(_ out: AVAudioPCMBuffer, for s: DuplexSpeech) -> Bool {
        playerLock.lock()
        defer { playerLock.unlock() }
        lock.lock()
        let speaking = current === s
        if speaking { s.pending += 1 }
        lock.unlock()
        guard speaking else { return false }
        player.scheduleBuffer(out, completionCallbackType: .dataPlayedBack) { [weak self] _ in
            self?.played(s)
        }
        return true
    }

    /// One buffer of `s` from the synthesizer: converted and scheduled, or its end.
    private func receive(_ buffer: AVAudioBuffer, for s: DuplexSpeech) {
        guard let pcm = buffer as? AVAudioPCMBuffer else { return }
        lock.lock()
        guard current === s, !s.written else {
            lock.unlock()
            return
        }
        // An empty buffer is the synthesizer's last word.
        let end = pcm.frameLength == 0
        if s.converter == nil && !end {
            s.converter = AVAudioConverter(from: pcm.format, to: format)
        }
        let converter = s.converter
        lock.unlock()
        // The converter keeps a few frames back until the end of the stream drains them.
        let out = converter.flatMap { convert(end ? nil : pcm, with: $0) }
        if let out, out.frameLength > 0, !schedule(out, for: s) {
            // Cut while converting: its buffers never reach the player.
            return
        }
        if end || out == nil {
            // The end, or a buffer that cannot be converted: the speech ends with what is scheduled.
            lock.lock()
            s.written = true
            lock.unlock()
            settle(s)
        }
    }

    /// A buffer of `s` has played back.
    private func played(_ s: DuplexSpeech) {
        lock.lock()
        s.pending -= 1
        lock.unlock()
        settle(s)
    }

    /// Sends the DONE of `s` once it is written and fully played, if it was not cut meanwhile.
    private func settle(_ s: DuplexSpeech) {
        lock.lock()
        let finished = current === s && s.written && s.pending == 0
        if finished { current = nil }
        lock.unlock()
        if finished { s.speech.close() }
    }

    /// `input` in the player's format; nil `input` drains the converter (end of stream).
    private func convert(_ input: AVAudioPCMBuffer?, with converter: AVAudioConverter) -> AVAudioPCMBuffer? {
        let ratio = format.sampleRate / converter.inputFormat.sampleRate
        // Room for the frames the converter held back from earlier buffers.
        let frames = (Double(input?.frameLength ?? 0) * ratio).rounded(.up) + 4096
        guard let capacity = AVAudioFrameCount(exactly: frames),
              let out = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: capacity)
        else { return nil }
        var given = false
        var error: NSError?
        let status = converter.convert(to: out, error: &error) { _, inputStatus in
            guard let input else {
                inputStatus.pointee = .endOfStream
                return nil
            }
            if given {
                inputStatus.pointee = .noDataNow
                return nil
            }
            given = true
            inputStatus.pointee = .haveData
            return input
        }
        return status == .error ? nil : out
    }
}

/// Starts the duplex engine, its frames going to `frames(ctx, …)` until `free(ctx)`.
///
/// Returns a retained engine handle, or nil with `code` written: then neither `frames` nor `free`
/// is called.
@_cdecl("oku_duplex_start")
public func okuDuplexStart(
    _ ctx: UnsafeMutableRawPointer?,
    _ frames: @escaping OkuDuplexFrames,
    _ free: @escaping OkuFree,
    _ code: UnsafeMutablePointer<Int32>?
) -> UnsafeMutableRawPointer? {
    autoreleasepool { () -> UnsafeMutableRawPointer? in
        let capture = DuplexCapture(ctx, frames, free)
        switch DuplexEngine.start(capture) {
        case .success(let engine):
            return Unmanaged.passRetained(engine).toOpaque()
        case .failure(let failure):
            capture.close(owned: false)
            code?.pointee = failure.code
            return nil
        }
    }
}

/// Releases the engine `oku_duplex_start` returned: cuts its speech, stops it, frees the capture.
@_cdecl("oku_duplex_release")
public func okuDuplexRelease(_ handle: UnsafeMutableRawPointer?) {
    guard let handle else { return }
    autoreleasepool {
        Unmanaged<DuplexEngine>.fromOpaque(handle).takeRetainedValue().close()
    }
}

/// Speaks `text` with the voice `voice` at `rate` times the default, through the engine's player.
///
/// Returns a retained speech handle, or nil (unknown voice, engine stopped or released): then
/// neither `done` nor `free` is called.
@_cdecl("oku_duplex_speak")
public func okuDuplexSpeak(
    _ engine: UnsafeMutableRawPointer?,
    _ text: UnsafePointer<CChar>?,
    _ voice: UnsafePointer<CChar>?,
    _ rate: Float,
    _ generation: UInt64,
    _ ctx: UnsafeMutableRawPointer?,
    _ done: @escaping OkuVoiceDone,
    _ free: @escaping OkuFree
) -> UnsafeMutableRawPointer? {
    guard let engine else { return nil }
    return autoreleasepool { () -> UnsafeMutableRawPointer? in
        guard let utterance = voiceUtterance(text, voice, rate) else { return nil }
        let owner = Unmanaged<DuplexEngine>.fromOpaque(engine).takeUnretainedValue()
        let speech = DuplexSpeech(VoiceSpeech(utterance, ctx, generation, done, free), owner)
        guard owner.speak(speech) else { return nil }
        return Unmanaged.passRetained(speech).toOpaque()
    }
}

/// Cuts the speech (player and synthesizer) if it is the one speaking, and sends its DONE if due.
@_cdecl("oku_duplex_stop")
public func okuDuplexStop(_ handle: UnsafeMutableRawPointer?) {
    guard let handle else { return }
    autoreleasepool {
        let speech = Unmanaged<DuplexSpeech>.fromOpaque(handle).takeUnretainedValue()
        speech.engine.stop(speech)
    }
}

/// Releases the handle `oku_duplex_speak` returned, cutting the speech and sending its DONE if due.
@_cdecl("oku_duplex_speech_release")
public func okuDuplexSpeechRelease(_ handle: UnsafeMutableRawPointer?) {
    guard let handle else { return }
    autoreleasepool {
        let speech = Unmanaged<DuplexSpeech>.fromOpaque(handle).takeRetainedValue()
        speech.engine.stop(speech)
    }
}
