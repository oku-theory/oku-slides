# Dictation through an OpenAI-compatible API — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `Model::Api` dictation engine that sends each closed phrase to an
OpenAI-compatible `POST {url}/audio/transcriptions`, configured from the dictation panel.

**Architecture:** A crate-internal `PhraseEngine` trait lets the existing Whisper phrase
pipeline (`transcribe_loop`) drive either the local `Transcriber` or a new `ApiTranscriber`
(`api.rs`, ureq, hand-built multipart, WAV encoder). The dictation `Config` gains an
`ApiConfig` (url, model, key_env); the key is read from the named environment variable.
The UI shows three fields when the API model is chosen.

**Tech Stack:** Rust (ureq 3, serde, snafu, speculoos, rstest, tiny_http), React + Bun
tests, Fluent (20 locales).

**Spec:** `docs/superpowers/specs/2026-10-23-dictee-api-design.md`

**Task:** t-1137; sub-tasks t-1138 (Task 1), t-1139 (Task 2), t-1140 (Task 3), t-1141 (Task 4).

## Global Constraints

- No `unwrap`/`expect` outside tests; Snafu errors with a backtrace in every variant,
  Fluent-localized messages.
- Never write, log or send to the UI the API key; read it with `std::env::var` only.
- Never call `std::env::set_var` (unsafe in edition 2024, and unsafe code lives only in
  `apple.rs`): tests use `HOME` as a variable that is set and a random name as one that
  is not.
- Rust unit tests in sibling `x_tests.rs` (`#[cfg(test)] #[path = "x_tests.rs"] mod x_tests;`),
  speculoos, names `unit_scenario_result`, one behavior per test.
- Every new Fluent key in all 20 locales (`crates/oku-slides-i18n/src/locales/*/`), same
  key order, translated (never the English text copied); `ui.ftl` keys through `t()`.
- Dependency versions only in the workspace `Cargo.toml`; crates use `.workspace = true`.
- Files under 500 lines target, never over 1000 (`lib.rs` is at 770: keep new logic
  in `api.rs`, `phrase_engine.rs`, `models.rs`).
- Document every new item and module, first sentence under 15 words; code comments in French.
- Commits `type(scope) : description (t-N)`, no Co-Authored-By line; commit with `-F <file>`
  (message file in the scratchpad).

---

### Task 1: API errors, Fluent keys and the timeout setting

**Files:**
- Modify: `crates/oku-slides-stt/src/error.rs`, `crates/oku-slides-stt/src/error_tests.rs`
- Modify: `crates/oku-slides-stt/Cargo.toml` (add `ureq.workspace = true`)
- Modify: `crates/oku-slides-i18n/src/locales/*/stt.ftl` (20 files)
- Modify: `crates/oku-slides-platform/src/config.rs` (`SttSettings`),
  `config.default.toml` (`[stt]`), `config_tests.rs`

**Interfaces — Produces:**
```rust
// SttInner (error.rs)
#[snafu(display("dictation API key missing: environment variable {var} is not set"))]
ApiKeyMissing { var: String, backtrace: Backtrace },          // kind Model
#[snafu(display("dictation API unreachable"))]
ApiHttp { source: Box<ureq::Error>, backtrace: Backtrace },   // kind Transcription
#[snafu(display("dictation API answered HTTP {status}"))]
ApiStatus { status: u16, backtrace: Backtrace },              // kind Transcription
#[snafu(display("unreadable dictation API answer"))]
ApiResponse { source: serde_json::Error, backtrace: Backtrace }, // kind Transcription
// SttSettings (platform)
/// Attente maximale d'une réponse de l'API de dictée (s).
pub api_timeout_secs: f32,   // default 30.0
```
Fluent (`stt.ftl`, after `stt-locale-unsupported`):
```
stt-api-key-missing = Set the environment variable { $var } to use the dictation API
stt-api-unreachable = The dictation API cannot be reached
stt-api-status = The dictation API refused the request (HTTP { $status })
stt-model-api = API ({ $model })
```
(`stt-model-api` goes after `stt-model-apple`.)

- [ ] **Step 1: Failing tests.** In `error_tests.rs`:
```rust
#[test]
fn api_key_missing_kind_is_model() {
    let e: SttError = ApiKeyMissingSnafu { var: "OPENAI_API_KEY" }.build().into();
    assert_that!(e.kind()).is_equal_to(SttErrorKind::Model);
}

#[rstest]
#[case("en", "OPENAI_API_KEY")]
#[case("fr", "OPENAI_API_KEY")]
fn api_key_missing_localized_names_the_variable(#[case] locale: &str, #[case] var: &str) {
    let l = Localizer::new(locale);
    let e: SttError = ApiKeyMissingSnafu { var }.build().into();
    assert_that!(e.localize(&l)).contains(var);
}

#[test]
fn api_status_localized_in_french_shows_the_status() {
    let e: SttError = ApiStatusSnafu { status: 401u16 }.build().into();
    let text = e.localize(&Localizer::new("fr"));
    assert_that!(text).contains("401");
    assert_that!(text).is_not_equal_to(e.localize(&Localizer::new("en")));
}
```
(Match how `error_tests.rs` builds a `Localizer` today; add `rstest` to dev-deps from the
workspace if missing.) In `config_tests.rs`:
```rust
#[test]
fn defaults_stt_api_timeout_is_thirty_seconds() {
    let s = Settings::defaults().unwrap();
    assert_that!(s.stt.api_timeout_secs).is_equal_to(30.0);
}
```
- [ ] **Step 2:** `cargo nextest run -p oku-slides-stt -p oku-slides-platform` → FAIL (missing items).
- [ ] **Step 3: Implement.** Add the variants, `kind()` arms (`ApiKeyMissing` with the `Model`
  group, the three others with `Transcription`), `localize()` arms:
  `ApiKeyMissing { var, .. } => l.tr_with("stt-api-key-missing", &[("var", var.as_str().into())])`,
  `ApiHttp { .. } => l.tr("stt-api-unreachable")`,
  `ApiStatus { status, .. } => l.tr_with("stt-api-status", &[("status", (*status).into())])`
  (use the same numeric-arg form as `stt-download-incomplete`), `ApiResponse { .. } => l.tr("stt-transcription")`.
  Add the four keys to all 20 `stt.ftl` (translate; keep `{ $var }`, `{ $status }`,
  `{ $model }`, `HTTP` and `API` unchanged). Add `api_timeout_secs` to `SttSettings` and
  `api_timeout_secs = 30.0` with a French comment to `[stt]` in `config.default.toml`.
- [ ] **Step 4:** same command → PASS; `cargo nextest run -p oku-slides-i18n` → PASS.
- [ ] **Step 5: Commit** `feat(stt) : erreurs et réglage de l'API de dictée (t-1137, t-1138)`.

### Task 2: `ApiTranscriber` and the `PhraseEngine` trait

**Files:**
- Create: `crates/oku-slides-stt/src/phrase_engine.rs` (trait + impl for `Transcriber`)
- Create: `crates/oku-slides-stt/src/api.rs`, `crates/oku-slides-stt/src/api_tests.rs`
- Modify: `crates/oku-slides-stt/src/lib.rs` (`mod api; mod phrase_engine; pub use api::ApiConfig;`)
- Modify: `crates/oku-slides-stt/src/audio.rs` (`parse_wav` becomes `pub(crate)`)
- Modify: `crates/oku-slides-stt/src/test_support.rs` (`fake_api`)

**Interfaces — Consumes:** Task 1 errors, `SttSettings::api_timeout_secs`.
**Produces:**
```rust
// phrase_engine.rs
/// Un moteur qui transcrit une phrase entière.
///
/// Implémentez ce trait pour brancher un moteur phrase par phrase (Whisper, une API).
pub(crate) trait PhraseEngine: Send + Sync {
    /// Transcrit un audio mono 16 kHz ; `quick` : aperçu (décodage rapide).
    fn transcribe(&self, audio: &[f32], language: Option<&str>, context: Option<&str>, quick: bool)
        -> Result<Transcript, SttError>;
}
impl PhraseEngine for Transcriber { /* délègue à Transcriber::transcribe */ }

// api.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiConfig { pub url: String, pub model: String, pub key_env: String }
// Default: "https://api.openai.com/v1", "gpt-4o-transcribe", "OPENAI_API_KEY"
impl ApiConfig {
    /// La clé, lue dans sa variable ; absente ou vide : `ApiKeyMissing`.
    pub(crate) fn key(&self) -> Result<String, SttError>;
}
pub(crate) fn wav(samples: &[f32]) -> Vec<u8>;                       // 16 kHz mono PCM16
pub(crate) fn multipart(fields: &[(&str, &str)], wav: &[u8]) -> (String /*content-type*/, Vec<u8>);
pub(crate) struct ApiTranscriber { /* config, key, agent */ }       // Debug manuel, sans la clé
impl ApiTranscriber { pub(crate) fn new(config: ApiConfig, timeout: Duration) -> Result<Self, SttError>; }
impl PhraseEngine for ApiTranscriber { /* POST, quick ignoré */ }

// test_support.rs
/// Requête reçue par la fausse API : en-tête Authorization et corps.
pub(crate) struct ApiCall { pub authorization: String, pub body: Vec<u8> }
/// Une fausse API : répond `status` et `body` à chaque requête ; rend son URL racine.
pub(crate) fn fake_api(status: u16, body: &'static str) -> (String, mpsc::Receiver<ApiCall>);
```

`wav`: RIFF header (`RIFF`, 36 + len, `WAVE`, `fmt ` 16, PCM 1, mono 1, `SAMPLE_RATE`,
`SAMPLE_RATE * 2`, block 2, 16 bits, `data`, len) then each sample
`(s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16` little-endian.

`multipart`: boundary `"oku-slides-" + 32 hex chars` from `std::time::SystemTime` nanos
and a process counter; each text field
`--b\r\nContent-Disposition: form-data; name="k"\r\n\r\nv\r\n`, then the file
`--b\r\nContent-Disposition: form-data; name="file"; filename="phrase.wav"\r\nContent-Type: audio/wav\r\n\r\n<bytes>\r\n--b--\r\n`;
content type `multipart/form-data; boundary=b`.

`ApiTranscriber::new`: `config.key()?`; agent
`ureq::Agent::config_builder().proxy(ureq::Proxy::try_from_env()).timeout_global(Some(timeout)).http_status_as_error(false).build().into()`.
`transcribe`: fields `model`, `response_format=json`, `language` if `Some`, `prompt` if the
trimmed context is not empty; `POST {url.trim_end_matches('/')}/audio/transcriptions`
with `Authorization: Bearer <key>`; transport error → `ApiHttp`; non-2xx → log
`tracing::warn!(status, body = %first 500 chars, "API de dictée")` then `ApiStatus`;
body `{"text": …}` (serde struct) → `Transcript { text, language: None, no_speech: 0.0 }`,
parse error → `ApiResponse`.

- [ ] **Step 1: Failing tests** in `api_tests.rs`:
```rust
#[test]
fn wav_one_second_parses_back_at_16k_mono() {
    let (rate, samples) = crate::audio::parse_wav(&wav(&vec![0.25; 16_000])).unwrap();
    assert_that!(rate).is_equal_to(SAMPLE_RATE);
    assert_that!(samples).has_length(16_000);
}

#[test]
fn multipart_fields_and_file_are_in_the_body() {
    let (ct, body) = multipart(&[("model", "whisper-1")], b"RIFFxx");
    let boundary = ct.strip_prefix("multipart/form-data; boundary=").unwrap();
    let text = String::from_utf8_lossy(&body);
    assert_that!(text.as_ref()).contains("name=\"model\"\r\n\r\nwhisper-1\r\n");
    assert_that!(text.as_ref()).contains("filename=\"phrase.wav\"");
    assert_that!(text.as_ref()).ends_with(&format!("--{boundary}--\r\n"));
}

#[test]
fn key_unset_variable_returns_api_key_missing() {
    let config = ApiConfig { key_env: "OKU_SLIDES_TEST_NO_SUCH_KEY_7f3a".into(), ..ApiConfig::default() };
    assert_that!(config.key().unwrap_err().kind()).is_equal_to(SttErrorKind::Model);
}

fn transcriber(url: String) -> ApiTranscriber {
    let config = ApiConfig { url, model: "whisper-1".into(), key_env: "HOME".into() };
    ApiTranscriber::new(config, Duration::from_secs(5)).unwrap()
}

#[test]
fn transcribe_ok_returns_the_api_text() {
    let (url, _calls) = fake_api(200, r#"{"text":"Bonjour à tous."}"#);
    let t = transcriber(url).transcribe(&[0.0; 1600], Some("fr"), Some("Oku"), false).unwrap();
    assert_that!(t.text).is_equal_to("Bonjour à tous.".to_owned());
}

#[test]
fn transcribe_sends_bearer_key_model_language_and_prompt() {
    let (url, calls) = fake_api(200, r#"{"text":"x"}"#);
    transcriber(url).transcribe(&[0.0; 1600], Some("fr"), Some("Oku"), false).unwrap();
    let call = calls.recv_timeout(Duration::from_secs(5)).unwrap();
    let body = String::from_utf8_lossy(&call.body).into_owned();
    assert_that!(call.authorization).is_equal_to(format!("Bearer {}", std::env::var("HOME").unwrap()));
    assert_that!(body.as_str()).contains("whisper-1");
    assert_that!(body.as_str()).contains("name=\"language\"\r\n\r\nfr\r\n");
    assert_that!(body.as_str()).contains("name=\"prompt\"\r\n\r\nOku\r\n");
}

#[test]
fn transcribe_unauthorized_returns_transcription_error() {
    let (url, _calls) = fake_api(401, r#"{"error":"bad key"}"#);
    let e = transcriber(url).transcribe(&[0.0; 1600], None, None, false).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SttErrorKind::Transcription);
}

#[test]
fn debug_never_shows_the_key() {
    let (url, _calls) = fake_api(200, "{}");
    let shown = format!("{:?}", transcriber(url));
    assert_that!(shown.contains(&std::env::var("HOME").unwrap())).is_false();
}
```
`fake_api` (test_support): `tiny_http::Server::http("127.0.0.1:0")`, URL
`format!("http://{}/v1", server.server_addr().to_ip().unwrap())`, a thread that for each
request reads the `Authorization` header and the body, sends an `ApiCall`, and responds
`Response::from_string(body).with_status_code(status)`.
- [ ] **Step 2:** `cargo nextest run -p oku-slides-stt api_tests` → FAIL.
- [ ] **Step 3:** Implement `phrase_engine.rs`, `api.rs`, `fake_api`, the `lib.rs` module lines.
- [ ] **Step 4:** same command → PASS.
- [ ] **Step 5: Commit** `feat(stt) : client de l'API de transcription compatible OpenAI (t-1137, t-1139)`.

### Task 3: `Model::Api` wired into `Stt`

**Files:**
- Modify: `crates/oku-slides-stt/src/models.rs` (variant, `ALL`, `repo`, `files`, `size`,
  `label`, `is_installed`, `previews`, `download`)
- Modify: `crates/oku-slides-stt/src/lib.rs` (`Config.api`, `Inner.engine`, `configure`,
  `start`, `download`, `model_info`, `begin_recording`, `listen_loop`, `Sink::Whisper`)
- Modify: `crates/oku-slides-stt/src/dictation_whisper.rs` (`engine()` returns `Arc<dyn PhraseEngine>`)
- Test: `crates/oku-slides-stt/src/lib_tests.rs`
- Modify: `crates/oku-slides-stt/ARCHITECTURE.md`, `crates/oku-slides-stt/AGENTS.md`

**Interfaces — Consumes:** `ApiConfig`, `ApiTranscriber`, `PhraseEngine`, `fake_api`.
**Produces:**
```rust
pub enum Model { Apple, Turbo, Large, Base, Api }          // serde "api"; ALL has 5 entries
pub const ALL: [Model; 5] = [Model::Apple, Model::Turbo, Model::Large, Model::Base, Model::Api];
pub fn label(self, l: &Localizer, api: &ApiConfig) -> String; // Api: tr_with("stt-model-api", model)
pub(crate) fn previews(self) -> bool;                       // false for Api
pub struct Config { /* … */ pub api: ApiConfig }
/// Ce qui identifie un moteur chargé : un modèle local, ou une API et ses réglages.
#[derive(Debug, Clone, PartialEq)]
enum EngineKey { Local(Model), Api(ApiConfig) }
fn engine_key(config: &Config) -> EngineKey;
// Inner.engine: Option<(EngineKey, Arc<dyn PhraseEngine>)>
// Sink::Whisper { jobs, busy, previews: bool }
```
Behavior:
- `Model::Api`: `repo()` `"api"`, `files()` empty, `size()` 0, `is_installed` false;
  `models::download` refuses it with `NotDownloadable` (like Apple).
- `model_info(Api)`: `available = true`, `installed = g.config.api.key().is_ok()`;
  label via `model.label(&self.localizer, &g.config.api)`.
- `start()`: for `Api`, `match api.key() { Ok(_) => self.begin_recording(), Err(e) => self.report(e) }`.
- `Stt::download(Api, _)`: `self.report(NotDownloadableSnafu.build().into())` and return.
- `engine()`: cache by `engine_key`; `Api(c)` → `ApiTranscriber::new(c, Duration::from_secs_f32(self.settings.api_timeout_secs))`;
  `Local(m)` → `Transcriber::load` as today. `configure()` drops the cached engine when
  the key changes.
- `begin_recording`: `Sink::Whisper { …, previews: model.previews() }`; `listen_loop`
  sends `Job::Preview` only when `previews`.

- [ ] **Step 1: Failing tests** in `lib_tests.rs` (and update
  `stt_default_config_is_turbo_auto_language_auto_stop_and_persists`: `has_length(4)`):
```rust
fn api_config(url: &str, key_env: &str) -> Config {
    Config { model: Model::Api, api: ApiConfig { url: url.into(), model: "whisper-1".into(), key_env: key_env.into() }, ..Config::default() }
}

#[test]
fn status_api_without_key_is_listed_not_installed() {
    let dir = fresh_dir("oku-slides-stt-");
    let stt = Stt::new(&platform(dir.path()));
    stt.configure(api_config("http://127.0.0.1:9/v1", "OKU_SLIDES_TEST_NO_SUCH_KEY_7f3a"));
    let api = stt.status().models.into_iter().find(|m| m.id == Model::Api).unwrap();
    assert_that!(api.installed).is_false();
}

#[test]
fn start_api_without_key_reports_error_and_stays_idle() {
    let dir = fresh_dir("oku-slides-stt-");
    let stt = Stt::new(&platform(dir.path()));
    stt.configure(api_config("http://127.0.0.1:9/v1", "OKU_SLIDES_TEST_NO_SUCH_KEY_7f3a"));
    stt.start(Vec::new());
    let s = stt.status();
    assert_that!(s.state).is_equal_to(State::Idle);
    assert_that!(s.error.unwrap()).contains("OKU_SLIDES_TEST_NO_SUCH_KEY_7f3a");
}

#[test]
fn transcribe_loop_api_commits_the_api_text() {
    let dir = fresh_dir("oku-slides-stt-");
    let stt = Stt::new(&platform(dir.path()));
    let (url, _calls) = crate::test_support::fake_api(200, r#"{"text":"Bonjour."}"#);
    stt.configure(api_config(&url, "HOME"));
    let session = {
        let mut g = stt.state();
        g.phrases.push(None);
        g.state = State::Transcribing;
        g.session
    };
    let (jobs, rx) = mpsc::channel();
    jobs.send(Job::Final { index: 0, audio: vec![0.0; 1600] }).unwrap();
    jobs.send(Job::Done).unwrap();
    stt.transcribe_loop(session, rx, Arc::new(AtomicBool::new(false)));
    assert_that!(stt.status().text).is_equal_to(Some("Bonjour.".to_owned()));
}

#[test]
fn api_model_sends_no_previews() {
    assert_that!(Model::Api.previews()).is_false();
}

#[test]
fn config_without_api_section_loads_default_api() {
    let c: Config = serde_json::from_str(r#"{"model":"turbo"}"#).unwrap();
    assert_that!(c.api).is_equal_to(ApiConfig::default());
}
```
- [ ] **Step 2:** `cargo nextest run -p oku-slides-stt` → the new tests FAIL.
- [ ] **Step 3:** Implement the behavior above.
- [ ] **Step 4:** `cargo nextest run -p oku-slides-stt` → PASS; `cargo clippy -p oku-slides-stt -- -D warnings` clean;
  `wc -l crates/oku-slides-stt/src/lib.rs` under 1000.
- [ ] **Step 5:** `ARCHITECTURE.md`: add an **API** engine bullet (OpenAI-compatible,
  `api.rs`, phrase by phrase, no preview, key from an environment variable) and the
  `PhraseEngine` extension point; stay ≤ 50 lines. `AGENTS.md` (Invariants):
  `- Read the dictation API key only from the environment variable named in ApiConfig; never store, log or send it to the UI. (spec:2026-10-23-dictee-api-design)`.
  Run `just agents-lint`.
- [ ] **Step 6: Commit** `feat(stt) : moteur de dictée Model::Api, phrase par phrase et sans aperçu (t-1137, t-1140)`.

### Task 4: Dictation panel fields, help and UI strings

**Files:**
- Modify: `ui/src/api.ts` (`SttModel`, `SttApiConfig`, `SttConfig.api`)
- Modify: `ui/src/Dictation.tsx` (badge, idle line, API fields, download guards, `modelNote`)
- Modify: `ui/src/Dictation.render.test.tsx`, `ui/src/AgentWindow.render.test.tsx` (fixtures get `api`)
- Modify: `crates/oku-slides-i18n/src/locales/*/ui.ftl` (20 files)
- Modify: `docs/aide/en.json`, `docs/aide/fr.json` (slide `s-dicter`)

**Interfaces — Consumes:** host `Status` with `config.api` and model id `"api"`.
**Produces:**
```ts
export type SttModel = "apple" | "turbo" | "large" | "base" | "api";
export interface SttApiConfig { url: string; model: string; key_env: string }
export interface SttConfig { /* … */ api: SttApiConfig }
```
Fluent `ui.ftl` (next to the other `ui-dictation-*` keys):
```
ui-dictation-note-api = Remote service, OpenAI-compatible
ui-dictation-api-ready = Key found
ui-dictation-api-key-missing = { $var } not set
ui-dictation-api-url = API address
ui-dictation-api-model = Model
ui-dictation-api-key-env = Key variable
ui-dictation-api-note = Each sentence is sent to this service. The key is read from the environment variable when dictation starts; an app opened from the Finder does not see shell variables (use launchctl setenv).
```
Behavior in `Dictation.tsx`:
- Badge for `m.id === "api"`: `installed ? t("ui-dictation-api-ready") : t("ui-dictation-api-key-missing", { var: status.config.api.key_env })`, class `ok` when installed.
- Idle body: `toDownload` excludes `api`; a `keyMissing` line (`model?.id === "api" && !model.installed`) shows `ui-dictation-api-key-missing`.
- Download button hidden for `api`.
- When `status.config.model === "api"`: three `Field variant="row"` text inputs (url,
  model, key_env), local state, `configure({ api: { ...status.config.api, [k]: value } })`
  on blur when changed, then the `ui-dictation-api-note` caption.
- `modelNote("api")` → `t("ui-dictation-note-api")`.

- [ ] **Step 1: Failing render tests** in `Dictation.render.test.tsx` (follow the file's
  existing helpers for rendering, opening the settings and spying on `stt.configure`):
  - choosing settings with `config.model = "api"` shows inputs labelled
    `ui-dictation-api-url`, `-model`, `-key-env` with the config values;
  - changing the model input to `whisper-1` then blurring calls `configure` with
    `api.model === "whisper-1"` and the other two fields unchanged;
  - with `api` not installed, the badge reads the key-missing text with `OPENAI_API_KEY`
    and no download button is shown.
- [ ] **Step 2:** `cd ui && bun test src/Dictation.render.test.tsx` → FAIL.
- [ ] **Step 3:** Implement; add the 7 keys to all 20 `ui.ftl` (translated; keep
  `{ $var }`, `launchctl setenv`, `Finder`, `API`); fixtures gain
  `api: { url: "https://api.openai.com/v1", model: "gpt-4o-transcribe", key_env: "OPENAI_API_KEY" }`.
- [ ] **Step 4:** add a bullet to slide `s-dicter` in `docs/aide/en.json`
  (`"Through an API"` bold + `": pick API in the dictation settings; each sentence goes to an OpenAI-compatible service, with the key read from an environment variable."`)
  and the French equivalent in `fr.json` (`"Par une API"`).
- [ ] **Step 5:** `just test` and `just check` → PASS.
- [ ] **Step 6: Commit** `feat(ui) : réglages de la dictée par API dans le panneau de dictée (t-1137, t-1141)`.
