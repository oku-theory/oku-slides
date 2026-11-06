# oku-slides-stt

Oku Slides speech: dictation and OS voice. Dictation: microphone -> voice detection -> engine
-> text, polled (`Stt::status`) by the Tauri app and by the dev server alike.

## Engines

- **Apple** (`Model::Apple`, macOS 26+, setting `stt.apple`): continuous `SpeechAnalyzer`
  through a static Swift bridge (`swift/`, `build.rs`, `apple.rs`). Language resolved by `locale.rs`, installed by the system. Building needs the macOS 26 SDK;
  the app runs from macOS 12 (Whisper only before 26). Off macOS, `#[cfg]` leaves out what only
  the bridge produces (`stream::raw`, the result events, `Availability::Ready`, `SpeechDenied`).
- **Whisper** (Turbo, Large, Base): CTranslate2 (`whisper.rs`), models downloaded from Hugging
  Face by `oku_slides_platform::models` (`models.rs`), sentence by sentence (`dictation_whisper.rs`).
- **API** (`Model::Api`): OpenAI-compatible or Azure OpenAI transcription through async-openai
  (`api.rs`: `OpenAIConfig`, or `AzureConfig` when `api_version` is set), phrase by phrase
  like Whisper but without preview; the key is read from the environment variable named in
  `ApiConfig`, never stored or copied (`key_present` only tests it). `api_check.rs` holds the
  settings rules (`normalized_url`, `is_loopback`, `valid_key_env`, `valid_api_model`,
  `valid_api_version`); `api_extra.rs` the user-only headers, `api_access_problem` (7.0.0
  `query` refused, not migrated) and redacted errors; `api_http.rs` the transport: one attempt,
  1 MiB reply, status kept. Its reqwest client follows no redirect, skips the proxy for loopback,
  trusts system certificates; a current-thread Tokio runtime runs it. `Model::id`/`from_id`: stable names.

## Flow

`saved_config.rs` reads `config.json` once, at launch; one that does not parse is renamed `config.json.invalid-<secs>[-<n>]`; one it cannot read or rename blocks saving.
`listen_loop` (`listen.rs`): microphone (`audio.rs`) -> 16 kHz -> earshot (`vad.rs`,
automatic stop) -> `Sink`: Whisper jobs, or the `Stream` of a continuous engine
whose events `dictation_stream.rs` applies (`results.rs`). `microphone.rs`: `Stt::microphone` (bridge `oku_speech_microphone`, never asks; `Unknown` off macOS), `Stt::model_files` (missing, partial, complete) for diagnosis.
Lossy numeric casts (sample counts, seconds, positions) live in `num.rs` (named like `oku_slides_core::num`), each with its reason.
Hands-free listening (`State::Listening`, `utterance.rs`): no auto-stop; after each silence of `config.silence` the phrases since the last close become one `Utterance` (Whisper: `Job::Flush`),
timed by sample offsets, sent on the single channel `Stt::utterances` hands out once. `SuspendGuard`s drop audio and the utterance in progress; `epoch` drops their late results. `capture.rs`: `start`,
`listen`, launch and download. `Config.hands_free` is saved, never started by itself at launch. Feature `e2e-fakes` (`just e2e` only, no release build): `fake_source.rs` listens to the lines appended to `stt.fake_utterances`.
In automatic-language mode the API engine sends no language hint, so `detected` stays empty.

## OS voice (`speak/`)

`SpeechEngine` (`status`, `speak`, `stop`; one DONE per `SpeechId`), from `system_speech_engine`:
`apple.rs` (`AVSpeechSynthesizer`, `swift/.../OkuVoice.swift`, a `bridged::BridgedVoice`; every `unsafe` in `apple_ffi.rs`,
contexts freed by Rust only), `windows.rs` (WinRT, own thread), `linux.rs` (Speech Dispatcher, one
SSIP connection per speech, `ssip.rs`; cut by `CANCEL self`; autospawn as libspeechd),
`fake.rs` (`e2e-fakes`: a log through `Storage`). No voice for the language: `Unavailable`.
`speaker.rs`: `Speaker`, one per process (`for_platform`), one thread: a speech waits while the user speaks (`SPEECH_HOLD`), cuts the last, holds a `SuspendGuard` once spoken, a watchdog (chars per second per script) and `SPEECH_GUARD` after it (`hears`); `echo.rs`: the last 3 muted speeches folded (never one that `asks`, which forgets them), in memory, never logged, so `echoes` drops an utterance repeating one within `ECHO_HORIZON`; `speech_config.rs`: `read_aloud`, `SpeechConfig` (a `barge_in` change restarts listening).
Duplex (`duplex.rs`, macOS, `OkuDuplex.swift`, `unsafe` in `duplex_ffi.rs`): one `AVAudioEngine` with voice processing (echo cancellation) taps the microphone (mono, a `Tap` feeding `Recorder::from(duplex)`) and plays the speech that `write` renders (`Duplex::voice`, a `BridgedVoice`), so the canceller removes it; buffers come on the main queue: no main run loop, no start; a buffer is scheduled only while its speech is current. Barge-in (`barge_in.rs`): hands-free listening with `barge_in` on opens the duplex, its voice kept in `Inner.barge` (`barge_voice`); the speaker then plays through it with no guard nor deaf window and cuts on `user_speaking` (`BARGE_POLL`), telling the speech it ended at its start; a failed start listens through cpal, muted, its reason in `barge_in_unavailable`. Hardware: `tests/duplex.rs` (own `main`; also: Oku's voice never opens a phrase). Known gap (t-1490): the WinRT engine stays unverified until `voice-check.yml` runs.

## Extension points and dependencies

`stream::StreamEngine`, `phrase_engine::PhraseEngine` (Whisper, API): other dictation engines.
`oku-slides-platform` (settings, storage), `oku-slides-i18n` (messages), `windows` (Windows voice).
