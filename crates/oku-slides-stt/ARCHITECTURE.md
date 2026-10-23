# oku-slides-stt

Oku Slides dictation: microphone -> voice detection -> engine -> text. Polled
(`Stt::status`), by the Tauri app and by the dev server alike.

## Engines

- **Apple** (`Model::Apple`, macOS 26+, setting `stt.apple`): continuous `SpeechAnalyzer`
  through a static Swift bridge (`swift/`, `build.rs`, `apple.rs`).
  Language resolved by `locale.rs`, installed by the system.
  Building on macOS needs Xcode with the macOS 26 SDK or later; the app
  runs from macOS 12 (Whisper only before macOS 26).
- **Whisper** (Turbo, Large, Base): CTranslate2 (`whisper.rs`), models
  downloaded from Hugging Face by `oku_slides_platform::models` (`models.rs` lists the files),
  sentence by sentence (`dictation_whisper.rs`).
- **API** (`Model::Api`): an OpenAI-compatible transcription API (`api.rs`), phrase by
  phrase like Whisper but without preview; the key is read from the environment
  variable named in `ApiConfig`, never stored.

## Flow

`listen_loop` (lib.rs): microphone (`audio.rs`) -> 16 kHz -> earshot (`vad.rs`,
automatic stop) -> `Sink`: Whisper jobs, or the `Stream` of a continuous engine
whose events `dictation_stream.rs` applies (`results.rs`).

In automatic-language mode the API engine sends no language hint: its answer
carries no language, so `detected` stays empty.

## Extension points

- `stream::StreamEngine`: plug in another continuous engine.
- `phrase_engine::PhraseEngine`: plug in another phrase-by-phrase engine (Whisper, API).

## Dependencies

`oku-slides-platform` (settings, storage), `oku-slides-i18n` (messages).
