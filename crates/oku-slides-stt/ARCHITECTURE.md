# oku-slides-stt

Oku Slides dictation: microphone -> voice detection -> engine -> text. Polled
(`Stt::status`), by the Tauri app and by the dev server alike.

## Engines

- **Apple** (`Model::Apple`, macOS 26+, setting `stt.apple`): continuous `SpeechAnalyzer`
  through a static Swift bridge (`swift/`, `build.rs`, `apple.rs`).
  Language resolved by `locale.rs`, installed by the system.
  Building on macOS needs Xcode with the macOS 26 SDK or later; the app
  runs from macOS 12 (Whisper only before macOS 26). Off macOS, `#[cfg]` leaves out
  what only the bridge produces (`stream::raw`, the result events, `Availability::Ready`,
  `SpeechDenied`).
- **Whisper** (Turbo, Large, Base): CTranslate2 (`whisper.rs`), models
  downloaded from Hugging Face by `oku_slides_platform::models` (`models.rs` lists the files),
  sentence by sentence (`dictation_whisper.rs`).
- **API** (`Model::Api`): OpenAI-compatible or Azure OpenAI transcription through async-openai
  (`api.rs`: `OpenAIConfig`, or `AzureConfig` when `api_version` is set), phrase by phrase
  like Whisper but without preview; the key is read from the environment variable named in
  `ApiConfig`, never stored or copied (`key_present` only tests it). `api_check.rs` holds the
  settings rules (`normalized_url`, `is_loopback`, `valid_key_env`, `valid_api_model`,
  `valid_api_version`); `api_extra.rs` the user-only headers, `api_access_problem` (7.0.0
  `query` refused, not migrated) and redacted errors; `api_http.rs` the transport: one attempt,
  1 MiB reply, status kept. Its reqwest client follows no redirect, skips the proxy for loopback,
  trusts system certificates; a current-thread Tokio runtime runs it. `Model::id`/`from_id`: stable names.

## Settings

`saved_config.rs` reads `config.json` in the models folder once, at launch; a file that
does not parse is renamed `config.json.invalid-<secs>[-<n>]`; one it cannot read or rename blocks saving.

## Flow

`listen_loop` (lib.rs): microphone (`audio.rs`) -> 16 kHz -> earshot (`vad.rs`,
automatic stop) -> `Sink`: Whisper jobs, or the `Stream` of a continuous engine
whose events `dictation_stream.rs` applies (`results.rs`).
Lossy numeric casts (sample counts, seconds, positions) live in `num.rs` (named like `oku_slides_core::num`), each with its reason.

In automatic-language mode the API engine sends no language hint: its answer
carries no language, so `detected` stays empty.

## Extension points

- `stream::StreamEngine`: plug in another continuous engine.
- `phrase_engine::PhraseEngine`: plug in another phrase-by-phrase engine (Whisper, API).

## Dependencies

`oku-slides-platform` (settings, storage), `oku-slides-i18n` (messages).
