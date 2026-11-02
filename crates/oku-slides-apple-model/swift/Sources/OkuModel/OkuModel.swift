// OkuModel: Apple's on-device language model (FoundationModels, macOS 26+) for Rust.
//
// C ABI (@_cdecl) read by crates/oku-slides-apple-model/src/bridge.rs; codes shared with
// bridge.rs. A reply sends exactly one event through `cb(ctx, kind, code, text)`, then calls
// `free(ctx)` once. No `try!` and no `!`: a crash here would bring the app down.
// FoundationModels is weak-linked (deployment target 12.0): every use sits behind
// `#available(macOS 26.0, *)`.

import Foundation
import FoundationModels

public typealias OkuModelCallback = @convention(c) (UnsafeMutableRawPointer?, Int32, Int32, UnsafePointer<CChar>?) -> Void
public typealias OkuModelFree = @convention(c) (UnsafeMutableRawPointer?) -> Void

private let kindReply: Int32 = 0
private let kindError: Int32 = 1

private let errContextOverflow: Int32 = 1
private let errRefused: Int32 = 2
private let errFailed: Int32 = 3
private let errCancelled: Int32 = 4

private let availReady: Int32 = 0
private let availDeviceNotEligible: Int32 = 1
private let availIntelligenceOff: Int32 = 2
private let availModelNotReady: Int32 = 3
private let availMacOsTooOld: Int32 = 4
private let availUnknown: Int32 = 5

/// Back to Rust: one event, then `free` once.
final class Sink: @unchecked Sendable {
    private let ctx: UnsafeMutableRawPointer?
    private let cb: OkuModelCallback
    private let free: OkuModelFree
    private let lock = NSLock()
    private var closed = false

    init(_ ctx: UnsafeMutableRawPointer?, _ cb: OkuModelCallback, _ free: OkuModelFree) {
        self.ctx = ctx
        self.cb = cb
        self.free = free
    }

    /// The last event, then `free`; later calls do nothing.
    func close(_ kind: Int32, _ code: Int32, _ text: String) {
        lock.lock()
        defer { lock.unlock() }
        if closed { return }
        closed = true
        text.withCString { cb(ctx, kind, code, $0) }
        free(ctx)
    }
}

/// One reply in flight: its task, cancellable from any thread.
final class Job: @unchecked Sendable {
    private let lock = NSLock()
    private var task: Task<Void, Never>?
    private var cancelled = false

    func start(_ task: Task<Void, Never>) {
        lock.lock()
        defer { lock.unlock() }
        self.task = task
        if cancelled { task.cancel() }
    }

    func cancel() {
        lock.lock()
        defer { lock.unlock() }
        cancelled = true
        task?.cancel()
    }
}

@_cdecl("oku_model_availability")
public func okuModelAvailability() -> Int32 {
    guard #available(macOS 26.0, *) else { return availMacOsTooOld }
    switch SystemLanguageModel.default.availability {
    case .available:
        return availReady
    case .unavailable(let reason):
        switch reason {
        case .deviceNotEligible: return availDeviceNotEligible
        case .appleIntelligenceNotEnabled: return availIntelligenceOff
        case .modelNotReady: return availModelNotReady
        @unknown default: return availUnknown
        }
    }
}

/// The error code and an English message for a failed reply.
@available(macOS 26.0, *)
private func classify(_ error: Error) -> (Int32, String) {
    let message = String(describing: error)
    if error is CancellationError { return (errCancelled, message) }
    if #available(macOS 27.0, *), let e = error as? LanguageModelError {
        switch e {
        case .contextSizeExceeded: return (errContextOverflow, message)
        case .guardrailViolation, .refusal: return (errRefused, message)
        default: return (errFailed, message)
        }
    }
    if let e = error as? LanguageModelSession.GenerationError {
        switch e {
        case .exceededContextWindowSize: return (errContextOverflow, message)
        case .guardrailViolation, .refusal: return (errRefused, message)
        default: return (errFailed, message)
        }
    }
    return (errFailed, message)
}

/// One guided reply: a fresh session with `instructions`, then `prompt` under `schema`.
@available(macOS 26.0, *)
private func reply(
    instructions: String, prompt: String, schema: String, temperature: Double, maxTokens: Int
) async -> (Int32, Int32, String) {
    let decoded: GenerationSchema
    do {
        decoded = try JSONDecoder().decode(GenerationSchema.self, from: Data(schema.utf8))
    } catch {
        return (kindError, errFailed, "invalid generation schema: \(error)")
    }
    let session = LanguageModelSession(instructions: instructions)
    let options = GenerationOptions(temperature: temperature, maximumResponseTokens: maxTokens > 0 ? maxTokens : nil)
    do {
        let response = try await session.respond(
            to: prompt, schema: decoded, includeSchemaInPrompt: true, options: options)
        if Task.isCancelled { return (kindError, errCancelled, "cancelled") }
        return (kindReply, 0, response.content.jsonString)
    } catch {
        let (code, message) = classify(error)
        return (kindError, Task.isCancelled ? errCancelled : code, message)
    }
}

@_cdecl("oku_model_respond")
public func okuModelRespond(
    _ instructions: UnsafePointer<CChar>, _ prompt: UnsafePointer<CChar>, _ schema: UnsafePointer<CChar>,
    _ temperature: Double, _ maxTokens: Int32,
    _ ctx: UnsafeMutableRawPointer?, _ cb: OkuModelCallback, _ free: OkuModelFree
) -> UnsafeMutableRawPointer? {
    let sink = Sink(ctx, cb, free)
    guard #available(macOS 26.0, *) else {
        sink.close(kindError, errFailed, "the on-device model needs macOS 26 or later")
        return nil
    }
    let (instructions, prompt, schema) = (String(cString: instructions), String(cString: prompt), String(cString: schema))
    let tokens = Int(maxTokens)
    let job = Job()
    job.start(Task.detached {
        let (kind, code, text) = await reply(
            instructions: instructions, prompt: prompt, schema: schema, temperature: temperature, maxTokens: tokens)
        sink.close(kind, code, text)
    })
    return Unmanaged.passRetained(job).toOpaque()
}

@_cdecl("oku_model_cancel")
public func okuModelCancel(_ handle: UnsafeMutableRawPointer?) {
    guard let handle else { return }
    Unmanaged<Job>.fromOpaque(handle).takeUnretainedValue().cancel()
}

@_cdecl("oku_model_release")
public func okuModelRelease(_ handle: UnsafeMutableRawPointer?) {
    guard let handle else { return }
    Unmanaged<Job>.fromOpaque(handle).release()
}
