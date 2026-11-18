# Local dictation on Parakeet Implementation Plan (t-1663)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Whisper/CTranslate2 local dictation engines with Parakeet TDT 0.6B v3 (int8 ONNX through parakeet-rs), remove `ct2rs`, and drop Intel Mac from the release.

**Architecture:** A crate-private `parakeet.rs` implements the existing `PhraseEngine` trait over `parakeet_rs::ParakeetTDT` behind a `Mutex`. Task 3 swaps Whisper for Parakeet in one commit, with no stage where both engines coexist: Whisper, `ct2rs` and every Whisper name leave the code, and `Model` collapses to `{Apple, Parakeet, Api}` with no alias for the old ids. Task 1 measured CPU fastest on Apple Silicon (CoreML ~2× slower), so every platform runs on CPU.

**Tech Stack:** Rust, parakeet-rs =0.4.4 on CPU (ort =2.0.0-rc.15, ONNX Runtime 1.31), Fluent, React/TS, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-11-18-dictee-parakeet-design.md`

**Oku sub-tasks:** Task 1 t-1664, Task 2 t-1668, Task 3 t-1665 (absorbs t-1666), Task 4 t-1669 (t-1667, the folder cleanup, dropped: the user deletes the old folders by hand); Linux GPU follow-up t-1670.

## Global Constraints

- Every commit: Conventional Commits, English, cites `(t-1663)`; message file under `.superpowers/sdd/dictee-parakeet/`, `git commit -F`, trailer `Reviewed-by:` (or `Review-waived: docs-only`). No Co-Authored-By. The squash commit is `feat(stt)!:` with a `BREAKING CHANGE:` footer (enum variants, cargo feature `accelerate` and a release platform removed).
- Model files, istupakov/parakeet-tdt-0.6b-v3-onnx revision `8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce`, all three pinned by size and sha256:
  `encoder-model.int8.onnx` 652183999 B `6139d2fa7e1b086097b277c7149725edbab89cc7c7ae64b23c741be4055aff09`;
  `decoder_joint-model.int8.onnx` 18202004 B `eea7483ee3d1a30375daedc8ed83e3960c91b098812127a0d99d1c8977667a70`;
  `vocab.txt` 93939 B `d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d`. Total 670479942 B.
- The 25 languages: bg cs da de el en es et fi fr hr hu it lt lv mt nl pl pt ro ru sk sl sv uk.
- Model id `"parakeet"`. No alias: `turbo`, `large`, `base` disappear from the code (user, 2026-11-18: "no exception"). A `config.json` naming one fails to parse and takes the existing t-1216 path: kept aside as `config.json.invalid-*`, defaults loaded, logged. `Model::from_id("turbo") == None`.
- The app never deletes the old Whisper model folders; the user deletes them by hand.
- Dependencies: workspace `parakeet-rs = { version = "=0.4.4", default-features = false, features = ["cpu", "api-28"] }` (no `ort-defaults`: it pulls `tls-native`, so OpenSSL on Linux build hosts) plus workspace `ort = { version = "=2.0.0-rc.15", default-features = false, features = ["std", "ndarray", "download-binaries", "copy-dylibs", "tls-rustls", "api-28"] }` (adjust the list to what `ort-defaults` enabled minus `tls-native`; check `parakeet-rs`'s Cargo.toml), both used from the stt crate only with `.workspace = true`. No accelerated provider: Task 1 measured CoreML ~2× slower than CPU, so macOS, Linux and Windows run on CPU. `cargo tree -i openssl-sys` and `cargo tree -i native-tls` empty for all three targets. Workspace `ndarray` → `"0.17"`, `tokenizers` → `"0.23"` (one copy each in `cargo tree -d`).
- Mutex: `lock().unwrap_or_else(PoisonError::into_inner)`. No silent fallback. Errors Fluent-localized. Tests in sibling `x_tests.rs`, speculoos, rstest. `#[expect(…, reason)]`, never `#[allow]`. No thiserror/chrono. No dead code: every "delete if unused" is a `git grep` with zero hits, then a firm deletion. ARCHITECTURE.md ≤ 50 lines, committed with the code it describes. French doc comments on lines being edited are translated to English.
- App runs: fresh temp `OKU_SLIDES_PATHS__DATA` / `OKU_SLIDES_PATHS__DECKS` per run. No `CARGO_*` overrides.
- macOS minimum is 13.4 from Task 2 on (user, 2026-11-18: the prebuilt ONNX Runtime objects target 13.4). Every place that names the minimum moves together.
- Fixture WAV: `fixtures/stt/dictee-fr.wav` (Git LFS).

## Review Focus

1. Config saved with `"model": "turbo"`/`"large"`/`"base"`: the file is kept aside (never overwritten), dictation starts on the Parakeet defaults and the error is logged — Task 3 (regression `issue_1663_old_model_ids.rs`).
2. Parakeet selected with a language outside the 25 (saved `ja`, or UI locale `ja` without a choice): greyed in the picker, and starting dictation refuses with the localized `LocaleUnsupported` error, never transcribes — Task 3 (`start_parakeet_in_japanese_refuses_locale_unsupported`).
3. An upgrading user presses the mic with Parakeet not downloaded: the download starts, no error — Task 3 (`start_parakeet_not_installed_starts_download`).
4. A Linux or Windows build pulls no OpenSSL and the model runs on CPU there — Task 3 Step 9 (`cargo tree -i openssl-sys` per target) and Task 4 CI.
5. Empty, one-sample or silent audio returns empty text, not an error; a panic inside the model becomes `SttError::Transcribe` — Task 3 (`transcribe_empty_audio_returns_empty_text`, `transcribe_panic_is_an_error`).

---

### Task 1: Spike and benchmark gate (t-1664)

No product code is committed; the outcome is a numbers table and a provider decision in the spec's "Hardware acceleration" section.

**Files:**
- Create (throwaway, not committed): `crates/oku-slides-stt/examples/parakeet_spike.rs`
- Modify: `docs/superpowers/specs/2026-11-18-dictee-parakeet-design.md` (results)

**Interfaces:**
- Produces: `macos_provider ∈ {Cpu, CoreML(units)}`; Linux and Windows are CPU (t-1670 tracks Linux GPU).

- [ ] **Step 1: Clips.** From `fixtures/stt/dictee-fr.wav` cut 3 s, 8 s and full-length 16 kHz mono clips into `$TMPDIR/clips/`; write the reference text of each by listening (`$TMPDIR/clips/*.txt`).
- [ ] **Step 2: Turbo baseline** (before any enum change, current binary). Reuse the existing Turbo download: `OKU_SLIDES_PATHS__DATA` = a fresh temp dir whose models dir is a symlink to the real `~/Library/Application Support/me.dartiguenave.oku-slides/models` read-only copy, or point the example at it directly; never download 800 MB again. `just transcribe <clip> turbo fr` ×3 per clip after one warm-up: median wall time, text, peak RSS (`/usr/bin/time -l`).
- [ ] **Step 3: Spike build.** Locally add the workspace dependency from Global Constraints plus, under `[target.'cfg(target_os = "macos")'.dependencies]`, `parakeet-rs = { workspace = true, features = ["coreml"] }`. Build with `--locked` after `cargo update -p parakeet-rs`. Record: ONNX Runtime version, download host and whether ort-sys checks a hash (`build/download/dist.tsv`), the sha256 of the downloaded archive, `otool -l` `minos` of the linked binary (> 12.0 → STOP and ask), `cargo tree -i openssl-sys` (must be empty), `cargo tree -d | grep -E "ndarray|tokenizers"` (one each).
- [ ] **Step 4: Measure** per clip, median of 3 after one warm-up: load time, transcription time, peak RSS, word error rate against the reference, for CPU and CoreML `CpuAndNeuralEngine`, `All`, `CpuAndGpu` (cache dir set). Preview latency: transcribe growing prefixes (1, 2, … 8 s) of the 8 s clip and note the time per call.
- [ ] **Step 5: Gate.** STOP and report the table to the user if CPU Parakeet is slower than Turbo on the full clip, or its WER is worse than Turbo's by more than 2 points. macOS provider: a CoreML variant only if faster than CPU with identical text. Previews: kept only if a 8 s prefix transcribes in under 1 s on CPU; otherwise Task 3 sets `previews()` false for Parakeet.
- [ ] **Step 6: Linux/Windows build.** Push the branch and run `voice-check.yml` on ubuntu and windows with the default features. Expected: both build and the spike runs on CPU.
- [ ] **Step 7: Record** the table, the ORT facts from step 3 and the decisions in the spec; delete the spike example and revert the local Cargo edits; commit the spec (`docs(stt): parakeet spike results (t-1663)`, `Review-waived: docs-only`). `oku cli task done --project slider --id t-1664 --result "…"`.

### Task 2: Drop Intel Mac, raise macOS minimum to 13.4 (t-1668)

Before the engine work: ort has no x86_64-apple-darwin build, so the Intel matrix entry would fail from Task 3 on; its prebuilt objects target macOS 13.4.

**Files:**
- Modify: `.github/workflows/release.yml` (the `macos_intel` input and matrix entry, the Intel line of the ad hoc signing comment near :239, `darwin-x86_64` in the `latest.json` filter near :369, "four platforms" wording), `.github/scripts/release-gate.sh:26` (`PLATFORMS`), `.github/scripts/release-sign-test.sh:63-64,146-168` (Intel fixtures), root `AGENTS.md` pitfall ("short of three platforms"), `README.md` (Apple Silicon only).
- macOS 13.4: `src-tauri/tauri.conf.json:48` (`minimumSystemVersion`), `.cargo/config.toml:6` (`MACOSX_DEPLOYMENT_TARGET`; translate its French comment), `crates/oku-slides-stt/build.rs:9` and `crates/oku-slides-apple-model/build.rs:9` (`SwiftLinker::new("13.4")`), both `swift/Package.swift:7` (`.macOS("13.4")`), `crates/oku-slides-local-agent/tests/weak_link.rs:36` (`minos 13.4`), the 12.0/"macOS 12" mentions in `crates/oku-slides-apple-model/{ARCHITECTURE.md:22-32, src/lib.rs:6, swift/Sources/OkuModel/OkuModel.swift:6}`, root `AGENTS.md:58` pitfall (13.4; source: this plan), `Cargo.toml:138` comment, `README.md` requirements. Release notes: macOS 12 installs receive no further updates.
- Kept on purpose, noted in the commit message: `crates/oku-slides-platform/src/local_agent.rs` `Target::IntelMac`, `Unavailable::IntelMac` and the `agent-local-intel-mac` keys (source builds still exist).

- [ ] **Step 1:** Edit the files above; the release-notes template gains one line: Intel Mac installs receive no further updates.
- [ ] **Step 2:** `git grep -nE "12\.0|\.v12|macOS 12" -- .cargo src-tauri/tauri.conf.json crates/*/build.rs crates/*/swift crates/oku-slides-apple-model crates/oku-slides-local-agent/tests AGENTS.md README.md` → nothing about the OS minimum; `cargo nextest run -p oku-slides-local-agent weak_link` and `-p oku-slides-stt -p oku-slides-apple-model` build → PASS; `bash .github/scripts/release-sign-test.sh` → PASS; `actionlint .github/workflows/release.yml` → no error (install with `brew install actionlint` if missing); `git grep -n "x86_64-apple\|darwin-x86_64\|macos_intel" .github` → nothing.
- [ ] **Step 3: Commit** `ci(release)!: drop Intel Mac, require macOS 13.4 (t-1663)`; mark t-1668 done.

### Task 3: Parakeet replaces Whisper (t-1665)

One task, one commit: the Parakeet engine goes in and Whisper goes out together. Afterwards the word "whisper" and the ids `turbo`, `large`, `base` appear nowhere in code, config, translations, CI or user docs; only dated records (`docs/superpowers`, `RETROFIT_PLAN.md`) keep them.

**Files:**
- Create: `crates/oku-slides-stt/src/parakeet.rs`, `src/parakeet_tests.rs`
- Modify: `src/lib.rs` (module list; `model_info`), `src/models.rs` + `models_tests.rs` (`Model::Parakeet`, `PARAKEET_LANGUAGES`, `fn parakeet_supports`), `src/capture.rs` (start refusal), `src/capture_tests.rs` (or the existing start tests file), `threads()` moves from `src/whisper.rs` into `parakeet.rs`, workspace and stt `Cargo.toml`, `crates/oku-slides-i18n/src/locales/*/stt.ftl` and `*/session.ftl` (`stt-model-parakeet`, `voice-model-parakeet` in the 21 locales), `crates/oku-slides-agent/src/voice_port.rs:72` (`Model::Parakeet => ModelKind::Local`), plus the removal list below.

**Interfaces:**
- Produces:
  - `pub(crate) struct Parakeet { inner: Mutex<ParakeetTDT> }`, `pub(crate) fn load(dir: &Path) -> Result<Parakeet, SttError>`. Module doc records that parakeet-rs reads files by path (OpenDAL deviation, spec).
  - `impl PhraseEngine for Parakeet`: `language`, `context`, `quick` ignored (documented); returns `Transcript { text: trimmed, language: None, no_speech: 0.0 }`.
  - `Model::Parakeet`, id `"parakeet"`, `size() == 670479942`, `files()` the three pinned files; `pub const PARAKEET_LANGUAGES: [&str; 25]` and `pub(crate) fn parakeet_supports(language: &str) -> bool` (primary subtag, case-insensitive) in `models.rs`.
  - Errors: existing `ModelLoad { dir, source }` (kind `SttErrorKind::Model`) and `Transcribe { step: "parakeet", source }`; `LocaleUnsupported { locale }` on start.

- [ ] **Step 1: Failing tests.**
  - `parakeet_tests.rs`:
    - `load_missing_dir_returns_model_load_error`: kind `SttErrorKind::Model`, and `err.localize(&Localizer::new("fr"))` equals the `fr` text of the model-load key.
    - `load_garbage_files_returns_model_load_error`: three files of `b"x"` under the pinned names → kind `Model`.
    - `load_empty_vocab_returns_model_load_error`: real-size-agnostic garbage ONNX plus empty `vocab.txt` → `Err`, no panic.
    - `transcribe_empty_audio_returns_empty_text`, `transcribe_one_sample_returns_empty_text`: the pure guard `fn too_short(audio: &[f32]) -> bool` is true for 0 and 1 samples and false at 1600 (0.1 s); `transcribe` returns `Transcript::default()` when it holds, without touching the model.
    - `transcribe_panic_is_an_error`: the pure wrapper `fn guarded(f: impl FnOnce() -> Result<String, BoxError>) -> Result<String, SttError>` turns a panic into `Transcribe { step: "parakeet" }`.
    - `#[ignore = "needs the real model"] transcribe_french_wav_returns_text`: `fixtures/stt/dictee-fr.wav` → non-empty text, `language` `None`; `transcribe_silence_returns_empty_text` (0.2 s zeros), `transcribe_ignores_bogus_language` (language `Some("xx")` → same text as `None`), `transcribe_overlong_audio_returns_text` (35 s of the WAV repeated).
    - `start_parakeet_not_installed_starts_download` (start tests): Parakeet not installed → state `Downloading`, no error.
  - `models_tests.rs`: `parakeet_files_are_pinned` (names, sizes, sha256 for all three, revision; sum == `size()`); `parakeet_supports` rstest: `fr`, `FR`, `pt-BR`, `uk` true; `ja`, `zh-Hans`, `""` false; `PARAKEET_LANGUAGES` all true.
  - `lib_tests.rs`: `model_info_parakeet_available_for_french`, `model_info_parakeet_greyed_for_japanese` (chosen `Some("ja")`), `model_info_parakeet_uses_ui_locale_when_no_language` (localizer `ja`, language `None` → greyed), `model_info_chosen_language_wins_over_ui_locale` (localizer `ja`, chosen `fr` → available), `model_info_apple_and_api_unchanged`; `auto_language_leaves_detected_empty` (store a final `Transcript` with `language: None` → `detected` stays `None`, no error reported).
  - start tests: `start_parakeet_in_japanese_refuses_locale_unsupported` (Parakeet installed, language `ja` → state `Idle`, error is the localized `LocaleUnsupported` text, no recording thread).
  - session: `every_model_has_a_voice_model_label` in `voice_panel_tests.rs` (for each `Model::ALL` id, the `voice-model-<id>` key exists in `en` and `fr`).
- [ ] **Step 2:** `cargo nextest run -p oku-slides-stt -p oku-slides-session` → the new tests FAIL.
- [ ] **Step 3: Implement `parakeet.rs`.** Create the ONNX Runtime environment once (`static ENV: OnceLock<…>` from `parakeet_rs::ort::init().build()`, error → `ModelLoad`) and pass `ExecutionConfig::new().with_environment(env)` with intra-op threads from `threads()`; `ParakeetTDT::from_pretrained(dir, Some(config))`. Import the library trait as `use parakeet_rs::Transcriber as _;`. `transcribe`: `too_short` guard, then `guarded(|| inner.lock()…transcribe_samples(audio.to_vec(), 16000, 1, None))` through `catch_unwind(AssertUnwindSafe(…))`; poison recovery is kept (invariant) — note in a comment that the model's feature cache may be stale after a panic, and the next phrase rebuilds it. Errors: `parakeet_rs::Error` boxed into `BoxError` (not eyre). The single transcription thread serializes final, flush and preview jobs, so the Mutex is never contended. CPU only (Task 1): no provider code, no CoreML cache; ARCHITECTURE.md records why (CoreML ~2× slower, different text).
- [ ] **Step 4: Wire.** `models.rs`: `Model::Parakeet` per the Interfaces below (`#[default]`), `previews()` per Task 1. `dictation_phrase.rs::engine()`: `Model::Parakeet => Arc::new(Parakeet::load(&model.path(&self.dir))?)`. `model_info`: Parakeet `available = parakeet_supports(&self.wanted_language())`-equivalent computed from `g` (chosen language, else `localizer.locale()`). `capture.rs` start: before `is_installed`, `Model::Parakeet` with an unsupported wanted language → `self.refuse(&LocaleUnsupportedSnafu { locale }.build().into())`. `voice_port.rs:72` arm. Fluent: `stt-model-parakeet = Parakeet v3 (25 European languages)` (no size: the chip shows it) and `voice-model-parakeet` in all 21 locales. `ui/src/voiceApi.ts` type. Docs: stt `ARCHITECTURE.md` describes `parakeet.rs` and the provider choice with its reason; stt `AGENTS.md` gains the settled decision "Load Parakeet model files by path through parakeet-rs; download and delete only through Storage. (spec:2026-11-18-dictee-parakeet-design, t-1663)" and "Set `ORT_LIB_LOCATION` to build offline: ort-sys downloads ONNX Runtime at build time. (t-1663)".
- [ ] **Step 5:** `cargo nextest run -p oku-slides-stt parakeet --run-ignored all` with the model in `$TMPDIR/parakeet` → PASS.

#### Removal (same task, same commit)

**Files** (from `git grep -nE "Turbo|Large|Base|\"turbo\"|\"large\"|\"base\"|Whisper|whisper|ct2rs|mel_spec|accelerate"` across `crates src-tauri ui docs .github justfile deny.toml .cargo Cargo.toml`; every hit is in this list or deliberately kept):
- Delete: `crates/oku-slides-stt/src/whisper.rs`, `whisper_tests.rs`.
- Rename: `src/dictation_whisper.rs` → `src/dictation_phrase.rs`; `Sink::Whisper` → `Sink::Phrase` (`lib.rs:294`, `listen.rs`, `capture.rs:231`); `whisper_prompt` → `api_prompt` (`lib.rs:759-800`, kept for the API engine only, comments reworded).
- stt: `src/lib.rs` (`pub use whisper::{Transcriber, Transcript}` → `pub use phrase_engine::Transcript`; `Transcriber` and its `impl PhraseEngine` deleted; `migrate()`), `src/phrase_engine.rs` (`Transcript` moves here), `src/models.rs` + tests, `src/lib_tests.rs:117-132`, `src/error.rs` (delete `ModelFile`, `MissingModelFile`, `ModelSettings`, `MelFilters`, `Features`, `NoLanguage`, `NotContiguous` and their `ShapeError` sources; keep `ZeroSetting` — API timeout; edit both variant lists at :170 and :238 and both `kind()` matches; reword the :12 doc comment), `src/error_tests.rs:53-54`, `src/{utterance.rs:46, vad.rs:125, listen.rs:96-122, microphone_tests.rs:12, test_support.rs (Whisper Base fixture engine → Parakeet), dictation_stream_tests.rs, saved_config_tests.rs}`, `tests/dictation.rs` (`Model::Base` → `Parakeet`, label assertion), `tests/regressions/issue_1663_old_model_ids.rs` + `main.rs`, `examples/transcribe.rs`, `Cargo.toml` (description, `ct2rs`, `mel_spec`, `accelerate`, `ndarray` if the grep shows no other user in stt).
- Workspace: `Cargo.toml` (remove `ct2rs`, `mel_spec`; `ndarray` 0.17, `tokenizers` 0.23 or removed if unused, with its pin comment), `src-tauri/Cargo.toml:43` (`features = ["accelerate"]`), `justfile:585-589` (`stt_features` removed; `transcribe wav model="parakeet"`, comment lists `parakeet, apple, api`), `deny.toml` (remove `RUSTSEC-2026-0367/0368/0369/0429` and rewrite the comment block naming mel_spec/ct2rs/ndarray-npy), `.cargo/audit.toml` (same four ids), `.github/workflows/release.yml:211` (LNK2038 note), `.github/workflows/voice-check.yml:44` (CMake comment).
- Other crates: `crates/oku-slides-agent/src/{voice_port_tests.rs:359-368,441-458,487, voice_port_hands_free_tests.rs:24 (`"parakeet"`), settings_port_tests.rs:70,78}`, `crates/oku-slides-mcp/src/{voice_tools.rs:35,70, diagnosis.rs:34}` (`apple | parakeet | api`), `crates/oku-slides-mcp/tests/{server.rs:133-136, voice.rs, regressions/issue_1144_set_setting_dictation.rs:10,17}`, `crates/oku-slides-session/src/{voice_view.rs:24,33, voice_view_tests.rs:17,81, voice_fixture.rs:8-19, voice_panel_tests.rs:144}`, `crates/oku-slides-platform/src/config.default.toml:179,202`, `src-tauri/src/commands.rs:265`.
- i18n: delete `stt-model-turbo|large|base` and `voice-model-turbo|large|base` in all 21 locales; reword every Whisper mention in `stt.ftl` and `session.ftl` (`voice-dictation-intro*` included).
- UI: `ui/src/voiceApi.ts` (`SttModel = "apple" | "parakeet" | "api"`), `ui/src/test/sttSpy.ts`, `ui/src/AgentWindow.render.test.tsx:156`, `ui/e2e/210-settings-box.cjs:24,89-90,173-174` (chip `parakeet`, language in the 25), `ui/src/a2ui/fixtures/agent-surfaces.json` (regenerated, never hand-edited).
- Docs: stt `ARCHITECTURE.md` and `AGENTS.md` (every Whisper invariant and settled decision rewritten or deleted; list them in the commit message), `docs/{usage.md:101-109,147, configuration.md:69,83, development.md:22}`, `docs/aide/{en,fr}.json:383,1799`, `README.md:56` (plus a credit line: "Dictation model: NVIDIA Parakeet TDT 0.6B v3, ONNX export by istupakov, CC-BY-4.0"). Kept as history: `RETROFIT_PLAN.md`.

**Interfaces (removal):**
- Produces: `Model::{Apple, Parakeet, Api}` (`ALL` in that order, `Parakeet` `#[default]`); `Model::from_id` current ids only; `pub use phrase_engine::Transcript`; TS `type SttModel = "apple" | "parakeet" | "api"`.

- [ ] **Step 6: Failing tests.**
  - `models_tests.rs`: rstest values `Apple, Parakeet, Api`; `model_all_lists_every_variant_tested_above` → 3; `old_model_ids_do_not_deserialize` (`#[values("turbo","large","base")]` → `Err`); `model_from_id_old_id_is_none` (same values → `None`); `default_model_is_parakeet`.
  - `lib_tests.rs`: `migrate_apple_on_machine_without_it_falls_back_to_parakeet` (rewrites rows 117/122/127: the `migrate(Base)==Base` row becomes `migrate(Parakeet)==Parakeet`); the other migrate tests retargeted.
  - `tests/regressions/issue_1663_old_model_ids.rs`: `#[values("turbo","large","base")]` config file → after `Stt::new` the original bytes are in `config.json.invalid-*`, status model `parakeet`, no `config.json` overwritten before a user change.
  - `mcp/tests/server.rs`: setting `dictation.model` to `"turbo"` is refused; `"parakeet"` reads back `"parakeet"`.
- [ ] **Step 7:** run → FAIL.
- [ ] **Step 8: Implement** the file list. Regenerate the A2UI fixture: `OKU_UPDATE_FIXTURES=1 cargo nextest run -p oku-slides-session every_form_matches_the_ui_fixture`.
- [ ] **Step 9: Verify.**
  - `git grep -nE "\"(turbo|large|base)\"" -- crates ui src-tauri` → only `old_model_ids_do_not_deserialize`, `model_from_id_old_id_is_none`, the MCP refusal test and `issue_1663_old_model_ids.rs`.
  - `git grep -nE "ct2rs|mel_spec|accelerate|Transcriber\b" -- ':!docs/superpowers' ':!RETROFIT_PLAN.md'` → nothing (except `parakeet_rs::Transcriber as _`).
  - `git grep -n -i whisper -- crates src-tauri ui docs justfile Cargo.toml deny.toml .cargo .github ':!docs/superpowers'` → nothing.
  - `cargo tree -i zip@2.4.2` → nothing; `cargo tree -d | grep -E "ndarray|tokenizers"` → one each; `cargo deny check advisories` → no unused-ignore warning.
  - `just test`, `just check`, `just agents-lint` → PASS.
- [ ] **Step 10: Commit** `feat(stt)!: Parakeet replaces Whisper, ct2rs removed (t-1663)`; mark t-1665 and t-1666 done.

### Task 4: Land (t-1669)

- [ ] Baseline: `cargo audit` on `main` lists `quinn-proto` RUSTSEC-2026-0523/0524 and `quinn-udp` RUSTSEC-2026-0538, unrelated to this branch; if `just ci` fails only on them, report it to the user and open an Oku task, never ignore them here.
- [ ] `just ci` and `just e2e 4999` (fresh temp dirs) → green; CI on the branch builds the three release platforms.
- [ ] Whole-branch review; merge `main` in, `git fetch`, squash onto main per root AGENTS.md "Land" with the `feat(stt)!:` message and `BREAKING CHANGE:` footer (a `config.json` naming `turbo`, `large` or `base` is set aside and dictation settings return to defaults; old model folders are not deleted, list their three names for manual removal; a config saved by this version cannot be read by an older one; Intel Mac dropped); push; remove worktree and branch; mark t-1669 and `oku cli task done --project slider --id t-1663 --result "…"`.

## Follow-ups (not in this change)

- Verify downloaded model files beyond the `.complete` marker (hash on load).
- t-1670: WebGPU on a real Linux AMD/Intel GPU.
