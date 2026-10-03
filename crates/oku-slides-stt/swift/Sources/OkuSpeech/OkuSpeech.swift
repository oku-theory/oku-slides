// OkuSpeech : le moteur de dictée d'Apple (SpeechAnalyzer, macOS 26+) pour Rust.
//
// ABI C (@_cdecl), lue par crates/oku-slides-stt/src/apple.rs ; codes partagés
// avec stream.rs (`raw`). Un appel asynchrone rend ses événements par
// `cb(ctx, …)`, puis appelle `free(ctx)` une seule fois. Ni `try!` ni `!` :
// un plantage ici ferait tomber l'app.

import AVFoundation
import CoreMedia
import Foundation
import Speech

public typealias OkuCallback = @convention(c) (UnsafeMutableRawPointer?, Int32, UnsafePointer<CChar>?, Double, Double) -> Void
public typealias OkuFree = @convention(c) (UnsafeMutableRawPointer?) -> Void

private let kindPartial: Int32 = 0
private let kindFinal: Int32 = 1
private let kindProgress: Int32 = 2
private let kindDone: Int32 = 3
private let kindError: Int32 = 4
private let errAnalyzer = 1.0
private let errLocale = 2.0
private let errAsset = 3.0

/// Retour vers Rust : des événements, puis `free` une seule fois.
final class Sink: @unchecked Sendable {
    private let ctx: UnsafeMutableRawPointer?
    private let cb: OkuCallback
    private let free: OkuFree
    private let lock = NSLock()
    private var closed = false

    init(_ ctx: UnsafeMutableRawPointer?, _ cb: OkuCallback, _ free: OkuFree) {
        self.ctx = ctx
        self.cb = cb
        self.free = free
    }

    func send(_ kind: Int32, _ text: String = "", _ a: Double = 0, _ b: Double = 0) {
        lock.lock()
        defer { lock.unlock() }
        if closed { return }
        text.withCString { cb(ctx, kind, $0, a, b) }
    }

    /// Dernier événement (`Done` ou erreur), puis `free`.
    func close(_ kind: Int32, _ text: String = "", _ code: Double = 0) {
        lock.lock()
        defer { lock.unlock() }
        if closed { return }
        closed = true
        text.withCString { cb(ctx, kind, $0, code, 0) }
        free(ctx)
    }
}

private final class ResultBox<T>: @unchecked Sendable {
    var value: T?
}

/// Attente maximale d'un appel bloquant : un service Speech figé ne bloque pas l'app.
private let blockingTimeout: DispatchTimeInterval = .seconds(5)

/// Attend une tâche asynchrone depuis un fil Rust (jamais depuis le pool de Swift).
///
/// Passé `blockingTimeout`, rend `fallback` ; la tâche finit seule, sans lecteur.
private func blocking<T>(_ fallback: T, _ body: @escaping @Sendable () async -> T) -> T {
    let box = ResultBox<T>()
    let done = DispatchSemaphore(value: 0)
    Task {
        box.value = await body()
        done.signal()
    }
    if done.wait(timeout: .now() + blockingTimeout) == .timedOut {
        return fallback
    }
    return box.value ?? fallback
}

@available(macOS 26.0, *)
private func transcriber(_ locale: Locale) -> SpeechTranscriber {
    SpeechTranscriber(
        locale: locale,
        transcriptionOptions: [],
        // `.fastResults` : sans lui, le premier aperçu arrive au bout de 12 s.
        reportingOptions: [.volatileResults, .fastResults],
        attributeOptions: [.audioTimeRange]
    )
}

@_cdecl("oku_speech_supported")
public func okuSpeechSupported() -> Bool {
    if #available(macOS 26.0, *) {
        return SpeechTranscriber.isAvailable
    }
    return false
}

/// Même langue et même région (pas d'identifiants : fr_FR contre fr-FR).
@available(macOS 26.0, *)
private func same(_ a: Locale, _ b: Locale, region: Locale.Region?) -> Bool {
    a.language.languageCode == b.language.languageCode && a.region == region
}

/// La langue du moteur pour une étiquette ; partagée par disponibilité, installation et dictée.
///
/// Sans région (« fr »), celle du système est préférée, puis la région exacte,
/// sinon l'équivalent proposé par Apple (qui choisit une variante au hasard).
@available(macOS 26.0, *)
private func resolve(_ id: String) async -> Locale? {
    let wanted = Locale(identifier: id)
    let supported = await SpeechTranscriber.supportedLocales
    let region = wanted.region ?? Locale.current.region
    if let found = supported.first(where: { same($0, wanted, region: region) }) {
        return found
    }
    return await SpeechTranscriber.supportedLocale(equivalentTo: wanted)
}

/// 0 : inconnue, 1 : à télécharger, 2 : prête.
///
/// D'après les langues installées du système : `AssetInventory.status` ne voit
/// installées que celles réservées par l'app.
@_cdecl("oku_speech_availability")
public func okuSpeechAvailability(_ locale: UnsafePointer<CChar>) -> Int32 {
    guard #available(macOS 26.0, *) else { return 0 }
    let id = String(cString: locale)
    return blocking(Int32(0)) {
        guard let found = await resolve(id) else { return 0 }
        if await SpeechTranscriber.installedLocales.contains(where: { same($0, found, region: found.region) }) {
            return 2
        }
        if await SpeechTranscriber.supportedLocales.contains(where: { same($0, found, region: found.region) }) {
            return 1
        }
        return 0
    }
}

@_cdecl("oku_speech_install")
public func okuSpeechInstall(
    _ locale: UnsafePointer<CChar>, _ ctx: UnsafeMutableRawPointer?, _ cb: OkuCallback, _ free: OkuFree
) {
    let sink = Sink(ctx, cb, free)
    let id = String(cString: locale)
    guard #available(macOS 26.0, *) else {
        sink.close(kindError, id, errLocale)
        return
    }
    Task {
        guard let found = await resolve(id) else {
            sink.close(kindError, id, errLocale)
            return
        }
        do {
            if let request = try await AssetInventory.assetInstallationRequest(supporting: [transcriber(found)]) {
                let watch = request.progress.observe(\.fractionCompleted) { progress, _ in
                    sink.send(kindProgress, "", Double(progress.completedUnitCount), Double(progress.totalUnitCount))
                }
                try await request.downloadAndInstall()
                watch.invalidate()
            }
            sink.close(kindDone)
        } catch {
            sink.close(kindError, String(describing: error), errAsset)
        }
    }
}

/// Une dictée : l'audio arrive avant que l'analyseur soit prêt, il attend dans `pending`.
@available(macOS 26.0, *)
final class Session: @unchecked Sendable {
    private let sink: Sink
    private let module: SpeechTranscriber
    private let analyzer: SpeechAnalyzer
    private let input: AsyncStream<AnalyzerInput>.Continuation
    private let source: AVAudioFormat?
    private let lock = NSLock()
    private var target: AVAudioFormat?
    private var converter: AVAudioConverter?
    private var pending: [Float] = []
    private var ended = false
    private var setup: Task<Bool, Never>?
    private var results: Task<Void, Never>?

    init(locale: Locale, vocabulary: [String], sink: Sink) {
        self.sink = sink
        module = transcriber(locale)
        analyzer = SpeechAnalyzer(modules: [module])
        let (stream, input) = AsyncStream<AnalyzerInput>.makeStream()
        self.input = input
        source = AVAudioFormat(commonFormat: .pcmFormatFloat32, sampleRate: 16_000, channels: 1, interleaved: false)
        let (module, analyzer) = (self.module, self.analyzer)
        results = Task {
            do {
                for try await result in module.results {
                    let text = String(result.text.characters)
                    if result.isFinal {
                        sink.send(kindFinal, text, result.range.start.seconds, result.range.end.seconds)
                    } else {
                        sink.send(kindPartial, text)
                    }
                }
            } catch {
                sink.close(kindError, String(describing: error), errAnalyzer)
            }
        }
        setup = Task { [self] in
            do {
                guard let format = await SpeechAnalyzer.bestAvailableAudioFormat(compatibleWith: [module]) else {
                    sink.close(kindError, "no audio format", errAnalyzer)
                    return false
                }
                if !vocabulary.isEmpty {
                    let context = AnalysisContext()
                    context.contextualStrings[.general] = vocabulary
                    try await analyzer.setContext(context)
                }
                try await analyzer.start(inputSequence: stream)
                ready(format)
                return true
            } catch {
                sink.close(kindError, String(describing: error), errAnalyzer)
                return false
            }
        }
    }

    /// Format de l'analyseur connu : on convertit et on transmet ce qui attendait.
    private func ready(_ format: AVAudioFormat) {
        lock.lock()
        defer { lock.unlock() }
        target = format
        if let source, source != format {
            converter = AVAudioConverter(from: source, to: format)
        }
        let waiting = pending
        pending = []
        waiting.withUnsafeBufferPointer { yield($0) }
    }

    func push(_ samples: UnsafeBufferPointer<Float>) {
        lock.lock()
        defer { lock.unlock() }
        if ended { return }
        if target == nil {
            pending.append(contentsOf: samples)
            return
        }
        yield(samples)
    }

    /// Convertit au format de l'analyseur et transmet (verrou tenu).
    private func yield(_ samples: UnsafeBufferPointer<Float>) {
        guard !samples.isEmpty, let source, let target, let base = samples.baseAddress,
              let buffer = AVAudioPCMBuffer(pcmFormat: source, frameCapacity: AVAudioFrameCount(samples.count)),
              let channel = buffer.floatChannelData
        else { return }
        buffer.frameLength = AVAudioFrameCount(samples.count)
        channel[0].update(from: base, count: samples.count)
        guard let converter else {
            input.yield(AnalyzerInput(buffer: buffer))
            return
        }
        let capacity = AVAudioFrameCount(Double(samples.count) * target.sampleRate / source.sampleRate) + 32
        guard let out = AVAudioPCMBuffer(pcmFormat: target, frameCapacity: capacity) else { return }
        var fed = false
        var error: NSError?
        _ = converter.convert(to: out, error: &error) { _, status in
            if fed {
                status.pointee = .noDataNow
                return nil
            }
            fed = true
            status.pointee = .haveData
            return buffer
        }
        if error == nil, out.frameLength > 0 {
            input.yield(AnalyzerInput(buffer: out))
        }
    }

    /// Fin de l'audio : derniers textes définitifs, puis `Done`.
    func finish() {
        lock.lock()
        ended = true
        lock.unlock()
        let (setup, results, input, analyzer, sink) = (setup, results, input, analyzer, sink)
        Task {
            // Analyseur jamais démarré : l'erreur est déjà partie.
            guard await setup?.value == true else { return }
            input.finish()
            do {
                try await analyzer.finalizeAndFinishThroughEndOfInput()
            } catch {
                sink.close(kindError, String(describing: error), errAnalyzer)
                return
            }
            await results?.value
            sink.close(kindDone)
        }
    }

    /// Abandon : on arrête tout, sans attendre de texte.
    func cancel() {
        lock.lock()
        ended = true
        lock.unlock()
        input.finish()
        let (analyzer, results, sink) = (analyzer, results, sink)
        Task {
            await analyzer.cancelAndFinishNow()
            results?.cancel()
            sink.close(kindDone)
        }
    }
}

@available(macOS 26.0, *)
private func session(_ handle: UnsafeMutableRawPointer) -> Session {
    Unmanaged<Session>.fromOpaque(handle).takeUnretainedValue()
}

@_cdecl("oku_speech_start")
public func okuSpeechStart(
    _ locale: UnsafePointer<CChar>, _ words: UnsafePointer<UnsafePointer<CChar>?>?, _ count: Int32,
    _ ctx: UnsafeMutableRawPointer?, _ cb: OkuCallback, _ free: OkuFree
) -> UnsafeMutableRawPointer? {
    let sink = Sink(ctx, cb, free)
    let id = String(cString: locale)
    guard #available(macOS 26.0, *) else {
        sink.close(kindError, id, errLocale)
        return nil
    }
    var vocabulary: [String] = []
    if let words {
        for i in 0..<Int(max(count, 0)) {
            if let word = words[i] { vocabulary.append(String(cString: word)) }
        }
    }
    let found = blocking(Locale?.none) { await resolve(id) }
    guard let found else {
        sink.close(kindError, id, errLocale)
        return nil
    }
    return Unmanaged.passRetained(Session(locale: found, vocabulary: vocabulary, sink: sink)).toOpaque()
}

@_cdecl("oku_speech_push")
public func okuSpeechPush(_ handle: UnsafeMutableRawPointer?, _ samples: UnsafePointer<Float>?, _ count: Int32) {
    guard #available(macOS 26.0, *), let handle, let samples, count > 0 else { return }
    session(handle).push(UnsafeBufferPointer(start: samples, count: Int(count)))
}

@_cdecl("oku_speech_finish")
public func okuSpeechFinish(_ handle: UnsafeMutableRawPointer?) {
    guard #available(macOS 26.0, *), let handle else { return }
    session(handle).finish()
}

@_cdecl("oku_speech_cancel")
public func okuSpeechCancel(_ handle: UnsafeMutableRawPointer?) {
    guard #available(macOS 26.0, *), let handle else { return }
    session(handle).cancel()
}

@_cdecl("oku_speech_release")
public func okuSpeechRelease(_ handle: UnsafeMutableRawPointer?) {
    guard #available(macOS 26.0, *), let handle else { return }
    Unmanaged<Session>.fromOpaque(handle).release()
}
