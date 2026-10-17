# oku-slides-platform

Oku Slides foundation: logging, settings, file storage and language, shared by the app,
the CLI, the session, the agent and dictation. With the `listen` feature, it also holds
the bounded accept loop of local HTTP servers (CLI `serve`, MCP tool server).

## Modules

- `lib.rs`: `Platform` bundles `Settings`, `Storage`, the launch `Localizer`, the
  `LiveLocalizer` (interface language switched hot, shared by every clone) and a
  `RenderLaunch` (`InProcess` or `Process` for the render worker). `Platform::load` builds it once at
  startup; it clones cheaply. `RENDER_WORKER_ENV` marks a render worker process, which
  never launches another one.
- `telemetry`: `init` installs the `tracing` subscriber; call it first.
- `errors`: `one_line` prints an error and all its causes on one line.
- `config`: `Settings` and its sections (`PathSettings`, `RenderSettings`,
  `ServeSettings`, `SttSettings`, `AgentSettings` with named `AgentPreset`s,
  `ImportSettings`: threshold, witness timeouts and `soffice` programs, tuning tries, …), plus `APP_ID`.
- `config_write`: `write_value` sets one key in `config.toml` with `toml_edit`, keeping
  comments and order; it takes effect on the next launch.
- `storage`: `Storage`, a synchronous OpenDAL file API on its own small Tokio runtime
  (`read_limited`, `write_atomic`, `delete_dir`, `writer` for streams, `read_fonts`, …).
  Call it outside async contexts only. `StorageError::kind` gives a `StorageErrorKind`.
- `listen` (feature `listen`): `serve_connections` with `Limits` (header timeout,
  connection cap) and a bounded graceful shutdown, replacing `axum::serve`.

## Startup order

`telemetry::init` → `Platform::load` (settings → storage → `Localizer` from
`i18n.locale`, else the system language) → app state.

## Config sources and on-disk layout

Three layers, the last wins: embedded `src/config.default.toml`, then `config.toml` in
the data dir, then `OKU_SLIDES_<SECTION>__<KEY>` env vars (`OKU_SLIDES_STT__VAD__PAUSE`).
`OKU_SLIDES_PATHS__DATA` also moves where `config.toml` is read. `PathSettings` puts the
data dir at `~/Library/Application Support/me.dartiguenave.oku-slides` on macOS (XDG or
`APPDATA` elsewhere), with `models/`, `templates/`, `themes/` and `config.toml` under it;
decks default to `~/Documents/Oku Slides`.

## Extension points

- A setting: a field in a `Settings` section and its default in `config.default.toml`.
- An agent preset: an `[agent.presets.<id>]` table in `config.default.toml`.

## Dependencies

`oku-slides-i18n` (the `Localizer` held by `Platform`).
