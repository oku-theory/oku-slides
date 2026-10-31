# oku-slides-platform

Oku Slides foundation: logging, settings, file storage and language, shared by the app,
the CLI, the session, the agent and dictation.

## Modules

- `lib.rs`: `Platform` bundles `Settings`, `Storage`, the launch `Localizer`, the
  `LiveLocalizer` (language switched hot, shared by clones) and a `RenderLaunch`
  (`InProcess` or `Process`). `Platform::load` builds it once at startup; it clones cheaply.
  `RENDER_WORKER_ENV` marks a render worker process, which never launches another one.
  `Platform.program` is this executable (`None` in tests); `LOCAL_AGENT_FLAG` runs the local agent;
  `program_path` (`ProgramPath`, `login_path.rs`: the app's `with_login_path` reads the login shell's `PATH` on a thread).
- `telemetry`: `init` installs the `tracing` subscriber, first; `NETWORK_DEBUG_ENV` adds HTTP/TLS logs, `ACP_DEBUG_ENV` ACP messages (`ACP_TARGET`). `errors`: `one_line`.
- `config`: `Settings`, its sections (`PathSettings`, `RenderSettings`, `SttSettings`, `UsageSettings`, …), `APP_ID`.
  `config` (file `config_agent.rs`): `AgentSettings`, named `AgentPreset`s (a `command` xor a
  `Builtin`), the active one valid by construction (`new`, `select`, `insert_preset`, and on load).
- `updates`: updater decisions without Tauri: `InstallMode`, `UpdateStatus` (with "Later"), `Progress`
  (1 % steps, 512 MiB cap), `download_source` (trusted `UpdateSource`), `UpdateStage`, `InstallFailure`,
  `check_due` on an hourly `CHECK_TICK`, `UpdateMarks` (failed version, notice, "Later" on disk).
- `config_write`: `write_value` sets one `config.toml` key, keeping comments. `program`: `find_program`.
- `storage`: `Storage`, a synchronous OpenDAL file API on its own small Tokio runtime
  (`read_limited`, `write_atomic`, `delete_dir`, `writer`, `appender`, `reader`, `read_fonts`, …); call it outside
  async contexts only. `StorageError::kind` gives a `StorageErrorKind`.
- `models`: `PinnedFile`, `is_complete`, (feature `download`) `download`: hashed `.part`s renamed
  once verified, `.complete` last, a pinned `.part` resumed by `Range` (`resume.rs` decides,
  `download/transport.rs` fetches); retries capped per file; waits cancellable; `DownloadError::kind`.
- `http` (feature `download`): `system_tls`, TLS on the system certificate store, for every ureq agent.
- `local_agent`: pinned Qwen3 `AGENT_MODEL`, `Target`, `resolve`, `local_engine` → `ReadyEngine` or `Unavailable`.
- `listen` (feature `listen`): `serve_connections`, bounded by `Limits`, replaces `axum::serve`.

## Startup order

`telemetry::init` → `Platform::load` (settings → storage → `Localizer` from `i18n.locale`, else the system language) → app state.

## Config sources and on-disk layout

Three layers, the last wins: embedded `src/config.default.toml`, then `config.toml` in
the data dir, then `OKU_SLIDES_<SECTION>__<KEY>` env vars (`OKU_SLIDES_STT__VAD__PAUSE`).
`OKU_SLIDES_PATHS__DATA` also moves where `config.toml` is read. `PathSettings` puts the data dir at `~/Library/Application Support/me.dartiguenave.oku-slides` on macOS (XDG or
`APPDATA` elsewhere), with `models/`, `widgets/`, `layouts/`, `themes/`, `config.toml`; decks in `~/Documents/Oku Slides`.

## Extension points

- A setting: a field in a `Settings` section and its default in `config.default.toml`.
- An agent preset: `[agent.presets.<id>]` in `config.default.toml`, exactly one of `command`, `builtin`.

## Dependencies

`oku-slides-i18n` (the `Localizer`); `ureq`, `sha2`, `hex` for model downloads (feature `download`).
