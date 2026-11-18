# Local dictation on Parakeet TDT 0.6B v3 (t-1663)

Status: approved by the user on 2026-11-18.

## Intent

Local dictation must be faster. Replace the local Whisper engine (Turbo, Large, Base on
CTranslate2 through `ct2rs`) by [nvidia/parakeet-tdt-0.6b-v3](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3),
and remove `ct2rs` from the workspace entirely (user, 2026-11-18: "vitesse, fais-le et supprime
complètement ct2rs").

## Goals

- One local model, `Model::Parakeet`, replaces `Turbo`, `Large` and `Base`; it is the default.
- `ct2rs`, `mel_spec` and the `accelerate` feature disappear, with the CTranslate2 build notes
  (release.yml, voice-check.yml) and the `mel_spec` audit exceptions (deny.toml, .cargo/audit.toml).
- Apple (macOS 26+) and the API engines are untouched.
- Phrase-by-phrase flow (`dictation_whisper.rs`, `listen.rs`, previews, hands-free utterances)
  keeps working as today; only the engine behind `PhraseEngine` changes.

## Non-goals

- No streaming engine (Nemotron streaming, EOU): Parakeet stays phrase by phrase behind the VAD.
- No CUDA, TensorRT or DirectML execution provider in this change.
- No other Whisper-sized multilingual fallback for languages Parakeet lacks.

## Anti-goals

- A silent fallback to another engine or language when Parakeet cannot serve (invariant).
- Overwriting a user's `config.json` that still says `turbo`: it is kept aside (t-1216 path), and
  the reset to defaults is accepted (user, 2026-11-18: "no exception").

## Constraints and facts found

- Runtime: [parakeet-rs](https://crates.io/crates/parakeet-rs) `=0.4.4` (MIT OR Apache-2.0),
  `default-features = false, features = ["cpu", "ort-defaults", "api-28"]` (its defaults, written
  out). It runs on `ort` 2.0.0-rc.15, which downloads a prebuilt ONNX Runtime at build time.
  Deps it adds: `ort`, `realfft`, `eyre`, `hound` (already a dev-dep), `tokenizers` 0.23 with
  `onig`, `ndarray` 0.17. The workspace moves to `ndarray` 0.17 and `tokenizers` 0.23 (or drops
  them where nothing else uses them), so each exists once in the lock file.
- Build-time download: ort-sys fetches ONNX Runtime during `cargo build`. The spike records the
  host, the version and whether a hash is checked; `ORT_LIB_LOCATION` is the documented offline
  path (stt AGENTS.md). `openssl-sys` must not come with it.
- Model files: the ONNX export [istupakov/parakeet-tdt-0.6b-v3-onnx](https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx)
  (CC-BY-4.0, like NVIDIA's), pinned to revision `8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce`:
  `encoder-model.int8.onnx` (652 183 999 B, sha256 `6139d2fa…aff09`),
  `decoder_joint-model.int8.onnx` (18 202 004 B, sha256 `eea7483e…67a70`), `vocab.txt`
  (93 939 B, sha256 `d5854467…e3c35d`, hashed from the pinned revision). Total 670 479 942 B (Turbo was 817 887 676).
  parakeet-rs picks the `.int8.` files when the fp32 ones are absent (`model_tdt.rs::find_encoder`).
  NVIDIA's own repo has only `.nemo`, fp32 safetensors and a GGUF for NeMo-Speech.cpp; neither
  is readable by parakeet-rs.
- `ParakeetTDT::transcribe_samples(&mut self, Vec<f32>, 16000, 1, mode)` returns text and
  timestamps. No language hint (only parakeet-rs's "unified" model has `set_target_lang`), no
  detected language, no prompt or context, greedy decoding only.
- parakeet-rs opens files by path with `std::fs`/ONNX Runtime, not through `Storage`; the
  `.onnx` must sit on disk. Downloads still go through `oku_slides_platform::models` and
  `Storage`; only loading reads the path. Recorded deviation from the OpenDAL rule.
- `PhraseEngine::transcribe` takes `&self` and is `Send + Sync`; parakeet-rs needs `&mut self`:
  hold the model in a `Mutex`, locked with `unwrap_or_else(PoisonError::into_inner)`.
- ONNX Runtime allows one environment per process; nothing else here uses `ort`.
- Languages (25): bg, cs, da, de, el, en, es, et, fi, fr, hr, hu, it, lt, lv, mt, nl, pl, pt,
  ro, ru, sk, sl, sv, uk. The app's UI locales also include ar, bn, hi, ja, ko, th, tr, vi,
  zh-Hans, zh-Hant: Parakeet cannot dictate those.
- Release platforms: `ort` 2.0.0-rc.15 ships prebuilt ONNX Runtime 1.31 for aarch64-apple-darwin,
  x86_64-unknown-linux-gnu and x86_64-pc-windows-msvc, none for x86_64-apple-darwin
  (`ort-sys` `build/download/dist.tsv`). The user dropped Intel Mac support (2026-11-18: "on ne
  supporte plus le mac intel"), so the release goes from four platforms to three.
- macOS minimum: raised from 12.0 to 13.4 (user, 2026-11-18), because the prebuilt ONNX Runtime
  objects are built for 13.4 (spike results below). Tauri, `.cargo/config.toml`, both Swift
  packages and the weak-link test move together; macOS 12 installs receive no further updates.

## Approaches

- **A. candle reading NVIDIA's q8_0 GGUF (pure Rust).** 714 MB, no C++, Metal possible; about
  800 lines of our own FastConformer + TDT + mel code, plus a parity test. Rejected: much more
  code to own for the same model.
- **B. NeMo-Speech.cpp through FFI.** NVIDIA's official ggml runtime, fastest; needs CMake,
  Ninja and SentencePiece in CI on four platforms, hand-written bindings, and its ggml would
  share the binary with llama.cpp's (symbol clash, or dylibs to bundle). Rejected: build risk.
- **C. parakeet-rs on ONNX Runtime, int8 ONNX export (chosen by the user).** About 100 lines of
  adapter, a maintained crate, proven CPU speed; costs a build-time ONNX Runtime download, about
  30 MB of binary and a third-party export instead of NVIDIA's own files.

## Design

### Model (`models.rs`)

- `Model` becomes `Apple, Parakeet, Api` (`ALL` in that order); `Parakeet` is `#[default]`, id
  `parakeet`. No alias for `turbo`, `large`, `base` (user, 2026-11-18: "no exception"): an old
  `config.json` fails to parse and is kept aside. `Model::from_id` accepts only the current ids.
- `files()` lists the three pinned files above (revision and sha256 set; `vocab.txt` with
  `SMALL_FILE_MAX`), `size()` their total, `repo()` the export repo.
- Fluent: `stt-model-parakeet` and `voice-model-parakeet` replace the `turbo/large/base` keys
  in every locale of `stt.ftl` and `session.ftl`; every other text naming Whisper is reworded
  (help JSON, docs, MCP tool descriptions). The label carries no size: the chip shows it.
- README credits the model (NVIDIA Parakeet TDT 0.6B v3, ONNX export by istupakov, CC-BY-4.0).

### Engine (`parakeet.rs`, replacing `whisper.rs`)

- Crate-private `Parakeet::load(dir)` → `ParakeetTDT::from_pretrained(dir, Some(config))`, where
  `config` carries the one ONNX Runtime environment of the process (parakeet-rs 0.4.4 refuses a
  session without it) and intra-op threads = the current `threads()` value; `parakeet_rs::Error`
  mapped to `ModelLoadSnafu`. `Transcriber` (Whisper's struct) and its export disappear;
  `Transcript` moves to `phrase_engine.rs` and stays exported.
- Audio under 0.1 s returns an empty transcript without calling the model; a panic inside the
  model is caught and becomes `SttError::Transcribe`.
- `transcribe(audio, language, context, quick)`: `language`, `context` and `quick` are ignored
  by this engine (documented on the impl); returns `Transcript { text: trimmed, language: None,
  no_speech: 0.0 }`. Empty or blank text is what drops a silent phrase (`keep`).
- `PhraseEngine` keeps its signature (the API engine still uses `language` and `context`).
- `dictation_whisper.rs` becomes `dictation_phrase.rs`, `Sink::Whisper` becomes `Sink::Phrase`,
  `EngineKey::Local(Model)` unchanged. `whisper.rs`, `whisper_tests.rs` and the mel/
  preprocessor code and errors only it used are deleted (no dead code).

### Hardware acceleration (user, 2026-11-18: "coreml si possible et accélération matériel sous linux si possible amd / intel")

- "If possible" is decided by measurement and correctness, not assumed. parakeet-rs itself warns
  that CoreML "currently runs slower than CPU" on Parakeet (dynamic input shapes) and that
  "WebGPU may produce incorrect results" (`execution.rs`).
- macOS arm64: build with parakeet-rs `coreml` (ort has a prebuilt `coreml` ONNX Runtime).
  The spike measures CPU against CoreML (`CpuAndNeuralEngine`, `All`, `CpuAndGpu`, with the
  compiled-model cache under the models dir) on the same WAV, and compares their text.
  CoreML ships only if faster and its text matches CPU's; otherwise CPU, and the reason is
  written in ARCHITECTURE.md.
- Linux x86_64: candidates are WebGPU (AMD and Intel through Vulkan, prebuilt), OpenVINO (Intel,
  needs the OpenVINO runtime installed) and MIGraphX (AMD, needs ROCm). ort links one prebuilt
  feature set per build, so one is chosen. WebGPU is the only one needing no extra system
  install. It ships only if verified on a real Linux machine with an AMD or Intel GPU (CI
  runners have none): faster than CPU and the same text. Without such a check, Linux stays on
  CPU and a follow-up Oku task tracks it.
- parakeet-rs registers an accelerated provider with CPU behind it, so ONNX Runtime would fall
  back silently. If an accelerated provider ships, we register it ourselves through
  `ModelConfig::configure` with `error_on_failure`, log one `warn` naming it, and load on CPU.
  The CoreML cache lives in the model folder, keyed by encoder hash and ORT version, mode 0700.
- Linux and Windows ship on CPU; t-1670 tracks WebGPU on a real AMD/Intel GPU.

#### Spike results (t-1664, 2026-11-18, Apple Silicon Mac, macOS 27, 8 intra-op threads)

Clips (16 kHz mono): `a` = first 3 s of `fixtures/stt/dictee-fr.wav`, `b` = the whole fixture
(5.7 s), `c` (8.2 s) and `d` (28.1 s) = French text read by macOS `say -v Thomas`. The text given
to `say` is the reference of `c` and `d`; `a` and `b` cannot be listened to here, so their
reference is the consensus of Turbo and Parakeet, which give the same text. WER: words lowercased,
punctuation and apostrophes as separators. Turbo runs the app's path (`Stt::transcribe_samples`:
VAD phrases, language `fr`); Parakeet transcribes each clip in one call. Times are medians of 3
after one warm-up, in the same process ("warm"); "cold" is the whole `just transcribe` process.

| Clip | Turbo cold | Turbo warm | Parakeet CPU warm | CoreML ANE / All / GPU warm | Peak RSS Turbo / Parakeet CPU | WER Turbo / Parakeet |
|---|---|---|---|---|---|---|
| a 3 s | 2.77 s | 1.94 s | 0.081 s | 0.136 / 0.149 / 0.156 s | 4.61 / 1.05 GB | (consensus) |
| b 5.7 s | 2.94 s | 2.11 s | 0.124 s | 0.215 / 0.235 / 0.242 s | 4.61 / 1.09 GB | (consensus) |
| c 8.2 s | 3.05 s | 2.13 s | 0.164 s | 0.303 / 0.336 / 0.335 s | 4.62 / 1.22 GB | 0 / 0 % (0/25) |
| d 28.1 s | 5.83 s | 4.95 s | 0.495 s | 1.084 / 1.116 / 1.153 s | 4.76 / 1.41 GB | 1.2 / 1.2 % (1/85) |

- Load: Turbo 0.47 s, Parakeet CPU 1.14 s, CoreML 2.5 to 3.1 s with a warm compiled-model cache
  (3.6 to 5.1 s on the first load). CoreML peak RSS on `d`: 6.4 to 7.8 GB.
- Text: CPU gives the same text on every run. CoreML gives CPU's text on `a`, `b`, `c` but not on
  `d` (`essayée` where CPU writes `essayé`); the one error of both Turbo and Parakeet CPU on `d` is
  that word.
- Preview latency (CPU, growing prefixes of `c`): 1 s 0.049, 2 s 0.064, 3 s 0.081, 4 s 0.095,
  5 s 0.113, 6 s 0.128, 7 s 0.139, 8 s 0.156 s (Turbo: 1.88 to 2.30 s per call).
- Gate passed: Parakeet CPU is 17 times faster than Turbo warm on `b` and 10 times on `d`, with
  the same WER, and uses about a quarter of Turbo's memory.
- **Decisions.** macOS provider: **CPU**. Every CoreML variant is about twice slower than CPU,
  uses more memory, and changes the text on `d`, as parakeet-rs warns (dynamic shapes); the
  `coreml` feature is therefore not added, nor the CoreML cache. **Previews kept**: an 8 s prefix
  takes 0.16 s on CPU, far under 1 s.
- ONNX Runtime: 1.31.0 (ort / ort-sys 2.0.0-rc.15), downloaded by ort-sys's build script from
  `cdn.pyke.io` (`https://cdn.pyke.io/0/pyke:ort-rs/ms@1.31.0/<target>[+features].tar.lzma2`),
  cached under the user cache dir (`ort.pyke.io/dfbin/<target>/<sha256>`). ort-sys checks the
  archive's SHA-256 against `build/download/dist.tsv` (`hmac-sha256`); the aarch64-apple-darwin
  `coreml` archive is 9 687 276 B, sha256 `4033efca0e6c1606bd2ec3870d4cb19d0dba0ad0160dbfb616e66cae969ed68c`
  (recomputed here: it matches). dist.tsv has no plain CPU build for aarch64-apple-darwin nor for
  x86_64-pc-windows-msvc: without features, ort-sys takes the first row of the target, the
  `coreml` build on macOS and the `directml` build on Windows (Linux has a `none` build).
- macOS minimum: the linked binary says `minos 12.0`, but every object of the prebuilt
  `libonnxruntime.a` (1 325 of them) is built for `minos 13.4`; the linker prints no warning.
  Decided by the user (2026-11-18): the app minimum rises to 13.4.
- `openssl-sys`: `cargo tree -i openssl-sys` is empty for the macOS, Linux and Windows targets,
  but `ort-defaults` turns on `ort/default`, which has `tls-native`: ort-sys's build script
  downloads through ureq with native-tls, which is OpenSSL on a Linux build host (build
  dependencies follow the host, which `--target` does not show). Using `tls-rustls` instead of
  `ort-defaults` (ort as a direct dependency with `download-binaries`, `copy-dylibs`,
  `tls-rustls`) avoids it.
- Duplicates: `ndarray` 0.16 and 0.17, `tokenizers` 0.22 and 0.23 while ct2rs and mel_spec are
  in the tree (they hold the old ones); gone once they are removed.
- ct2rs and ONNX Runtime cannot share a binary: both embed `cpuinfo` (about 255 common symbols);
  with both linked, the first ONNX Runtime session crashes (SIGSEGV). Parakeet was measured from a
  standalone crate; the swap stays a single commit.
- CI (voice-check.yml on a temporary branch with the spike dependencies): passed on both runners
  (run 44384439467). Windows (`crt-static`): `cargo check -p oku-slides-stt` passes, ort-sys
  downloads its archive. Linux (ubuntu-24.04): check and `speak::` tests pass and the test
  binaries link with ct2rs and ONNX Runtime together; the build compiles `openssl-sys` 0.9.118
  through `native-tls`, as predicted. The workflow could not carry a full Parakeet build and run
  (the local token cannot push workflow files), so linking the Windows binary with the static CRT
  and running the model on Linux and Windows remain to check in Task 3's CI.

### Language availability

- Language detection is automatic only (user, 2026-11-18: "ok pour la détection auto"); a chosen
  language never reaches Parakeet. It only decides availability:

- `ModelInfo.available` for Parakeet is true when the wanted language (chosen, else UI locale)
  is one of the 25; otherwise greyed out, like Apple. `migrate`: an `Apple` model on a machine
  without Apple's engine becomes `Parakeet` (was `Turbo`).
- Automatic-language mode with Parakeet leaves `detected` empty, as the API already does.
- Starting dictation with Parakeet in an unsupported language (a saved `ja`, or the UI locale)
  refuses with the localized `LocaleUnsupported` error; nothing is transcribed and nothing
  switches engine. The API stays offered (user, 2026-11-18 review).
- Deck vocabulary: Whisper got the slide words as a prompt; Parakeet takes none. The prompt is
  kept for the API engine only (`api_prompt`); local dictation loses that hint (accepted by the
  user, 2026-11-18 review).

### Old Whisper files

- The app does not delete the old model folders; the user deletes them by hand (2026-11-18),
  and the release notes name them.

### Intel Mac dropped

- release.yml: remove the `macos_intel` input, its matrix entry, the Intel mention in the macOS
  ad hoc signing comment, `darwin-x86_64` from the `latest.json` platform filter, and "four
  platforms" wording. `.github/scripts/release-gate.sh` `PLATFORMS` and `release-sign-test.sh`
  fixtures follow. Root AGENTS.md pitfall: "short of three platforms". README: Apple Silicon only.
- Intel users already installed keep their version; the updater offers them nothing new, which
  the release notes say.

### Build and docs

- Workspace: remove `ct2rs`, `mel_spec`, the stt `accelerate` feature and its uses
  (`src-tauri/Cargo.toml`, `justfile` `stt_features`); add `parakeet-rs`; drop the release.yml
  LNK2038 note and the voice-check CMake mention; remove the four `mel_spec`/`zip` advisories
  (RUSTSEC-2026-0367/0368/0369/0429) from deny.toml and audit.toml.
- `examples/transcribe.rs` and `just transcribe` take `parakeet|apple|api` (default parakeet).
- Docs: ARCHITECTURE.md and AGENTS.md of the stt crate (the "Whisper" invariants and settled
  decisions are reworded to Parakeet), README, docs/usage.md, docs/configuration.md,
  docs/development.md, config.default.toml comments, ui `SttModel` type and the A2UI fixture.

### Tests

- Unit: model ids (old ids rejected), `files()`/`size()`, language
  availability (fr available, ja greyed), `migrate` to Parakeet.
- The existing dictation integration test (`tests/dictation.rs`, real model, ignored or gated
  as today) runs on Parakeet; `just transcribe` on a French WAV gives a timing to compare with
  Turbo (measured before removal, same machine).
- `just ci` and `just e2e` green; CI builds on the three release platforms.

### Flow details settled

- Phrase length: the VAD already caps a phrase at the configured maximum (30 s default,
  `config.default.toml`); parakeet-rs takes up to about 4 minutes, so no chunking. Reword the
  "Whisper reads 30 s at a time" comment.
- Previews: `listen.rs` sends a preview only when the transcription thread is free (`busy`),
  one engine thread, so the `Mutex` is never contended by a preview against a final phrase.
- Unsupported languages: the existing greyed-model display (as for Apple) with no new field;
  the API stays offered.

### Order of work

1. Build spike: add parakeet-rs, build and run on macOS arm64 locally, Linux and Windows in CI
   (voice-check.yml or a branch CI run); check the macOS minimum; measure CoreML against CPU.
2. Measure Turbo then Parakeet on 3 s, 8 s and full clips of the same French WAV, same machine,
   before removing anything: load time, transcription time, peak memory, word error rate against
   a reference, preview latency. If Parakeet is not faster or is clearly less accurate, stop and
   report to the user; previews stay only if they keep up.
3. Drop Intel Mac from the release.
4. Swap Whisper for Parakeet in one commit (user, 2026-11-18: "remove completely whisper"): no
   stage where both engines coexist, no Whisper name left in code or user docs.

## Left to the builder

- Exact parakeet-rs `ExecutionConfig` thread setter; whether `TimestampMode` matters (text only).
- Wording of the new Fluent strings in the 21 locales.

## Builder's calls (to check)

- int8 export from istupakov rather than parakeet-rs's own fp32 folder (2.5 GB).
- CoreML or WebGPU only after a measured win with identical text.
