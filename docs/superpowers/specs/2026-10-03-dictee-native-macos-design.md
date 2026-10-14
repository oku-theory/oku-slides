# Dictation with Apple's native engine — design

Date: 2026-10-03 · Task: t-393

## Goal

Dictation must return the text as soon as one has finished speaking. Today,
Whisper large-v3-turbo (CTranslate2, CPU + Accelerate, beam 5 for the final pass) makes
the user wait ~4–8 s after the last sentence. On macOS 26+, the system
engine (`SpeechAnalyzer` / `SpeechTranscriber`) transcribes continuously on the
Neural Engine: the final text arrives ~0.1 s after the voice ends.

It becomes the default model on macOS 26+; Whisper remains the engine on
Windows, Linux and macOS < 26, and a possible choice on Mac.

Out of scope: native Windows engine (`Windows.Media.SpeechRecognition`) and
Linux; automatic language detection on the Apple side.

## Spike measurements

23.8 s of French dictation (synthetic voice "Thomas", `say`), M1 Max,
loaded machine (load average 160–178) for all runs.

| Engine | Ready in | Whole file | Wait after speech |
|---|---|---|---|
| Apple `SpeechTranscriber` | 0.26 s | 1.0–1.6 s (15–24× real time) | 0.05–0.1 s |
| Apple `DictationTranscriber` | — | 1.0 s | 0.8 s |
| Whisper turbo | 0.9–1.3 s | 19–31 s (0.8–1.2×) | ≈ 4–8 s (estimated) |
| Whisper base | 0.2–0.4 s | 4.4–21 s | ≈ 1–5 s (estimated) |

Comparable quality: Apple formats numbers ("12 %") and punctuates well,
but confuses homophones (« mes trois puces », « réduit ») and adds neither
quotation marks nor a final period; Whisper turbo is slightly better at
spelling. Without the `.fastResults` option, the first Apple preview arrives
only after 12 s; with it, at 1 s. The French model was already installed
on the system.

## Decisions

- **One more model.** `Model::Apple` (« Apple (système) », "Apple (system)") joins Turbo,
  Large and Base in the existing list; the UI barely changes.
- **"Auto" = interface language.** `SpeechTranscriber` requires a
  locale: "Auto" takes the UI language (localizer), failing that the system
  locale. The UI shows « Auto (français) » ("Auto (French)") via `status.detected`.
- **One-time switch.** `Config` gains `version`; a config without a version,
  on a machine where Apple is `Ready` or `Missing`, switches to `Apple` on
  load, in memory only: the switch is idempotent, and a system
  update triggers it later. The file is only rewritten on the
  next `configure`. After that, the user's choice is respected.
- **Static Swift bridge.** `SpeechAnalyzer` exists only in Swift
  (async/`AsyncSequence`), with no Objective-C API: a small static Swift
  library, with a C ABI (`@_cdecl`), linked into the `oku-slides-stt` crate.
  Rejected: a separate Swift binary (lifecycle, signing, two
  protocols) and `swift-bridge` (too heavy for six functions).
- **Vocabulary as a list.** `start { prompt: string }` becomes
  `start { vocabulary: string[] }` (deck titles). Apple receives them as
  `contextualStrings`; Whisper joins them with "." (truncated to 600
  characters) for its prompt. No backward compatibility.
- **macOS 12 minimum.** The Swift bridge links the Swift Concurrency runtime,
  shipped with the OS since macOS 12: the app, the CLI and the Swift package
  deploy to 12.0 (`tauri.conf.json`, `.cargo/config.toml`, `build.rs`,
  `Package.swift`). Apple's engine stays reserved for macOS 26+.
- **No silent fallback.** A language that the Apple engine does not support
  greys out the Apple entry; we do not quietly switch to Whisper.

## Modules

```
crates/oku-slides-stt/
├── swift/                     Swift package (macOS only), ~200 lines
│   ├── Package.swift
│   └── Sources/OkuSpeech/OkuSpeech.swift
├── build.rs                   compiles swift/ via swift-rs, cfg(target_os = "macos")
└── src/
    ├── models.rs              Model::Apple
    ├── stream.rs              Availability, Event, Stream and StreamEngine traits,
    │                          NoStreamEngine (outside macOS or `stt.apple = false`)
    ├── apple.rs               Apple's StreamEngine, FFI over swift/ (the only `unsafe`)
    ├── locale.rs              "Auto" → UI language → system locale
    ├── results.rs             pure accumulator of streaming results
    ├── dictation_whisper.rs   Whisper transcription loop (moved out of lib.rs, unchanged)
    ├── dictation_stream.rs    streaming results loop, language download
    └── lib.rs                 listen_loop feeds the chosen engine
```

`lib.rs` is 691 lines long: the Whisper loop moves out of it first
(`dictation_whisper.rs`), without changing its behavior.

### `Model::Apple` and availability

```rust
pub enum Availability { Unsupported, Missing, Ready }
```

- `availability(locale)` queries the system (`SpeechTranscriber.
  installedLocales` / `supportedLocales`); outside macOS or without the engine,
  `NoStreamEngine` (`stream.rs`) always returns `Unsupported`. The Whisper
  models keep `is_installed` (`.complete` marker).
- `Model::ALL` lists Apple first. `status()` omits Apple when it is
  `Unsupported` for the platform (outside Mac, macOS < 26).
- `ModelInfo` gains `available: bool`: `false` when Apple does not support the
  chosen language (greyed-out entry, « Langue non prise en charge », "Language not supported").
- `Model::Apple.size()` is 0; the download progress comes
  from `AssetInventory`.

### `StreamEngine`

Internal traits (not `pub`) of the **streaming** engines: `StreamEngine`
(`supported`, `availability`, `install`, `open`) opens a `Stream` (`push`,
`finish`, `cancel`) and returns an `mpsc::Receiver<Event>`. Whisper does not
implement them: it keeps its jobs. `Stt` holds an
`Arc<dyn StreamEngine>`: `AppleEngine` on macOS when the
`stt.apple` setting is `true` (default), otherwise `NoStreamEngine` (always
`Unsupported`). Tests inject a fake engine.

`listen_loop` remains the owner of the microphone, of the
resampling to 16 kHz, of the level, of earshot (`speaking`, `stop_in`,
automatic stop); only the destination of the audio changes:

- Whisper: closed sentences and previews as `Final` / `Preview` jobs to
  `transcribe_loop`, as today;
- Apple: each resampled block goes into `apple::Session::push()`;
  earshot's sentences only serve the automatic stop.

### Swift bridge (`swift/` + `apple.rs`)

C ABI exposed through `@_cdecl`:

| Function | Role |
|---|---|
| `oku_speech_availability(locale) -> i32` | `Unsupported` / `Missing` / `Ready` |
| `oku_speech_install(locale, ctx, cb)` | downloads the language, progress through `cb` |
| `oku_speech_start(locale, vocab, n, ctx, cb) -> handle` | opens a session |
| `oku_speech_push(handle, samples, len)` | f32 mono 16 kHz |
| `oku_speech_finish(handle)` | finalizes, then emits `Done` |
| `oku_speech_cancel(handle)` | abandons (`cancelAndFinishNow`) |

- The Swift side converts the 16 kHz to `bestAvailableAudioFormat` with
  `AVAudioConverter`; Rust sends only one format, Whisper's.
- `SpeechTranscriber` with `reportingOptions: [.volatileResults,
  .fastResults]` and `attributeOptions: [.audioTimeRange]`.
- A single C callback carries all events:
  `Partial(text)`, `Final(text, start, end)`, `Progress(received, total)`,
  `Done`, `Error(code, message)`.
- The callback arrives on a Swift thread: it only does a `send` into an
  `mpsc`. Only the Rust dictation thread writes into `Inner`.
- The Swift code has neither `try!` nor `!`: a Swift crash would bring down the app.
  Every error becomes `Error(code, message)`.
- `apple.rs` wraps the handle in `Session` (`Drop` calls
  `oku_speech_cancel`) and contains all of the crate's `unsafe`, each block
  with its `SAFETY:` comment.

## Flow of an Apple dictation

```
UI ─ start(vocabulary) ─▶ Stt::start
                           │ locale = locale::resolve(config.language, UI language, system)
                           │ availability(locale) ─ Missing ─▶ download(Apple) ─▶ resume
                           ▼ Ready
                   begin_recording
                     ├─ microphone (cpal)                     in parallel
                     └─ apple::Session::start(locale, vocab)  (≈0.3 s; Swift buffers)
                           │
listen_loop (tick): mic → 16 kHz ─┬─▶ earshot: speaking, silence, stop_in, auto stop
                                  └─▶ session.push(samples)
                           │
Swift thread: SpeechTranscriber ─ callback ─▶ mpsc ─▶ Rust "results" thread
                                              ├─ Partial → Inner.partial
                                              └─ Final   → Inner.phrases (clean())
                           │
Stop (button or auto): session.finish() → last Finals, then Done
   └─ results thread: text = committed, seq += 1, state = Idle
Cancel: session.cancel(), session += 1 → late callbacks are ignored
```

- `results::apply(&mut Inner, Event)` is a pure function: `Partial`
  replaces `partial`; `Final` pushes `clean(text)` into `phrases` and clears
  `partial`.
- `detected` receives the resolved locale.
- No `context_of` for Apple: the session is a single continuous stream, the
  engine itself keeps the context of what came before.
- `transcribe_samples` (`transcribe` example, tests) follows the same path:
  push everything, `finish`, gather the `Final`s; the `TimedPhrase`s
  come from `audioTimeRange`.
- Language download: `Stt::download(Model::Apple)` calls
  `oku_speech_install`; same `Downloading` state, same `Download` in
  `Status`, same resumption of the dictation at the end.

## Errors and permissions

New Snafu variants (with `backtrace`) in `error.rs`:

| Variant | `SttErrorKind` | Message |
|---|---|---|
| `SpeechDenied` | `Permission` (new) | `stt-speech-denied` |
| `LocaleUnsupported { locale }` | `Model` | `stt-locale-unsupported` |
| `AssetInstall { reason }` | `Download` | `stt-download` |
| `Analyzer { reason }` | `Transcription` | `stt-transcription` |
| `NotDownloadable` | `Model` | `stt-model` |
| `FinishTimeout` | `Transcription` | `stt-transcription` |

- `apple.rs` translates the bridge's codes into variants; the mapping
  table is tested.
- `src-tauri/Info.plist` gains `NSSpeechRecognitionUsageDescription`.
- `NotDownloadable`: `models::download(Model::Apple)` refuses (Apple's
  language is installed through `AssetInventory`, not through Hugging Face).
- "Speech Recognition" permission: the spike, launched from the
  terminal, did not ask for it. **As soon as the app can dictate with Apple,
  check in the packaged app whether `SpeechAnalyzer` requires it** (one must
  be able to dictate to find out: second-to-last task of the plan). If so, `start` calls
  `SFSpeechRecognizer.requestAuthorization` when the status is
  `notDetermined`, and a refusal gives `SpeechDenied`.
  **Finding (2026-10-03, packaged app, macOS 27): no permission
  request, dictation works.** `SpeechAnalyzer` does not require
  the speech recognition permission: no `requestAuthorization`.
  `SpeechDenied` and the `DENIED` code remain in the bridge's interface,
  with no emitter on the Swift side.
- Availability of a language: based on `SpeechTranscriber.installedLocales`
  / `supportedLocales` (finding: `AssetInventory.status` only reports
  "installed" for the languages reserved by the app). A language without a
  region takes the system's region (`fr` → `fr_FR`; otherwise the system
  picked `fr_BE`).
- If `Done` does not arrive after `finish()`: timeout
  `stt.apple_finish_timeout_secs` (`SttSettings`, default in the embedded
  TOML), then `FinishTimeout` and back to `Idle`; the already
  finalized text is kept.
- Settings (`[stt]`, embedded TOML): `apple = true` (offer Apple's
  engine; `false` removes it, and tests turn it off to stay
  deterministic on a Mac running macOS 26) and `apple_finish_timeout_secs`.
- New Fluent keys in the 20 locales: `stt-speech-denied`,
  `stt-locale-unsupported`, `stt-model-apple` (stt.ftl);
  `ui-dictation-note-apple`, `ui-dictation-model-unavailable`,
  `ui-dictation-language-auto-resolved`, `ui-dictation-to-download-language`,
  `ui-dictation-model-intro-apple` (ui.ftl; the intro that mentions both
  engines is shown only if Apple is offered).

## Tests

1. **Unit, all platforms**: `locale::resolve` (auto, unsupported UI
   language, explicit language); `Config` migration (no version +
   Ready/Missing → Apple, no version + Unsupported → unchanged, with
   version → unchanged); Whisper vocabulary (joining, truncation);
   `results::apply` (order of finals, `partial` cleared, `clean`); bridge
   codes → variants.
2. **Flow with a fake engine**: `ScriptedEngine` (`#[cfg(test)]`, a
   `StreamEngine`) emits scripted events. Cases: start → results → stop →
   `Idle` + `text`; cancel with a late callback ignored; `finish` without
   `Done` → `FinishTimeout` + `Idle`; automatic stop by earshot →
   `finish`. Reuses `fake_mic` and `wait_until` from `tests/dictation.rs`.
3. **Real Apple integration**: `tests/apple.rs`,
   `#[cfg(target_os = "macos")]`, `#[ignore = "macOS 26 + langue fr
   installée"]`, run by `just test-apple`. Fixture
   `fixtures/stt/dictee-fr.wav` (~8 s, `say`, 16 kHz). Checks
   keywords (« diapositive », « trimestre ») and a wait after
   `finish` < 2 s.
4. **UI** (`bun test`): Apple entry greyed out if `available: false`;
   « Auto (français) » from `detected`; `stt.start(vocabulary)`.
5. **Tool**: the `transcribe` example accepts `apple`
   (`just transcribe <wav> apple fr`) to compare the engines.

## Manual validation

In the app on macOS 26: first dictation after an update (switch to
Apple), French dictation of 20–30 s with one's own voice, final text perceived
as immediate; choice of an unsupported language (greyed-out entry); back to
Whisper Turbo from the list.
