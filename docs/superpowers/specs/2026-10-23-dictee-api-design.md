# Dictation through an OpenAI-compatible API — Design

## Goal

Offer a third kind of dictation engine next to Apple and Whisper: a remote speech-to-text
API speaking the OpenAI `audio/transcriptions` protocol (OpenAI `gpt-4o-transcribe`,
`whisper-1`, Groq, Mistral Voxtral, a local whisper server…). The text is transcribed as
spoken; translation is out of scope.

## Decisions

- **Protocol:** OpenAI-compatible HTTP, one `POST {url}/audio/transcriptions` per closed
  phrase (multipart: `file` = 16 kHz mono 16-bit WAV, `model`, `language` when chosen,
  `prompt`, `response_format=json`). The realtime WebSocket protocol is not used.
- **Pipeline:** the Whisper phrase pipeline is reused unchanged (earshot cuts phrases,
  `transcribe_loop` transcribes them in order, with `context_of` as the prompt).
- **No preview:** the API engine sends no preview of the phrase in progress (each one
  would be a paid request); `listen_loop` skips `Job::Preview` for it.
- **Key:** read from an environment variable whose name is in the config
  (`OPENAI_API_KEY` by default), when a dictation starts; never written to disk, never
  logged, never sent to the UI.
- **No silent fallback:** without the variable, the API model stays selectable but its
  badge names the missing variable, and starting it reports `ApiKeyMissing`; an HTTP
  failure reports an API error for that phrase and the dictation goes on.

## Components

- `Model::Api` (`models.rs`): `size` 0, never installed on disk, `download` refuses it
  (`NotDownloadable`). Its label shows the configured model: `API ({ $model })`.
- `ApiConfig` in the dictation `Config` (`config.json`, edited from the UI):
  `url` (default `https://api.openai.com/v1`), `model` (default `gpt-4o-transcribe`),
  `key_env` (default `OPENAI_API_KEY`). `#[serde(default)]`: older files load unchanged.
- `PhraseEngine` (crate-internal trait, `phrase_engine.rs`):
  `transcribe(audio, language, context, quick) -> Result<Transcript, SttError>`,
  implemented by `Transcriber` (Whisper) and `ApiTranscriber` (`api.rs`).
  `Stt::engine()` returns `Arc<dyn PhraseEngine>`; the cache key is `EngineKey`
  (`Local(Model)` or `Api(ApiConfig)`), so editing the URL or model rebuilds the client.
- `Model::previews()`: false for `Api`; `listen_loop` sends `Job::Preview` only when true.
- `ApiTranscriber` (`api.rs`): ureq agent (system proxy, timeout
  `stt.api_timeout_secs`, 30 s by default, in `SttSettings`), WAV encoder, hand-built
  multipart body, JSON response `{ "text": … }` → `Transcript { text, language: None,
  no_speech: 0.0 }`.
- `ModelInfo` for `Api`: `available` is always true (the model stays selectable to show
  its settings); `installed` is true when the variable is set (non-empty). The UI badge
  reads "Key found" or "{ $var } not set"; the API model is never offered for download.
- Errors (`error.rs`): `ApiKeyMissing { var }` (kind `Model`) → `stt-api-key-missing`;
  `ApiHttp { source: Box<ureq::Error> }` → `stt-api-unreachable`;
  `ApiStatus { status }` → `stt-api-status` (only the HTTP status is logged, never the body, because an API's 401 body can echo a fragment of the key);
  `ApiResponse { source: serde_json::Error }` → `stt-transcription` (kind `Transcription`).
- UI (`Dictation.tsx`, `api.ts`): `SttModel` gains `"api"`, `SttConfig` gains `api`;
  when the API model is chosen, three fields (URL, model, key variable) under the model
  list, saved on blur, plus a caption explaining the variable and the Finder caveat.

## Data flow

Start → `start()` checks the variable (else `ApiKeyMissing`) → `begin_recording` →
`Sink::Whisper` → `send_final` → `transcribe_loop` → `ApiTranscriber::transcribe` →
`phrases[index]`. Stop → `Job::Done` → `complete()`.

## Caveat

A macOS app opened from the Finder does not inherit shell variables: the user sets the key
with `launchctl setenv OPENAI_API_KEY …` or starts the app from a terminal. The dictation
help (`docs/aide`) says so.

## Testing

- `api_tests.rs`: WAV header and samples; multipart fields; a `tiny_http` fake server
  receives `model`, `language`, `prompt`, the bearer key, and its text comes back; a 401
  gives `ApiStatus` with status 401; a missing variable gives `ApiKeyMissing`.
- `lib_tests.rs`: `status()` lists `api` not installed without the variable; `start()`
  with the API model and no variable reports the error and stays idle; `transcribe_loop`
  fed one `Job::Final` then `Job::Done` with the API model commits the fake server's
  text; `Model::Api.previews()` is false.
- `error_tests.rs`: both new variants localized in en and one other locale (key check).
- `Dictation.render.test.tsx`: choosing API shows the three fields; editing one calls
  `configure` with the new `api` value.
