# oku-slides-cli

The `oku-slides` binary: check, render, save, reopen, strip and import decks without the
app, plus benchmarks and `serve`, the dev server that runs the UI and API in a browser
without Tauri. Results go to stdout, logs to stderr, help in the user's language.

## Modules

- `main.rs`: `Cli` and `Cmd` (clap), with help text from Fluent `cli-cmd-…` and
  `cli-arg-…` keys (`command`); `run` dispatches each `Cmd`. Shared helpers load a deck
  from JSON or `.oku-slides.pdf` (`load_bundle`), embed its pack and theme, and write the
  document (`write_document`, `write_rendered`).
- `out.rs`: `Out`, localized command output on stdout; a closed pipe ends the command.
- `library.rs`: `deck_pack_version`, a deck's pack version beside it (`open --out`) or in the
  library, checked against its id.
- `pack_render.rs`: `render` renders a deck in memory, or in the render worker when its pack
  (`session::ProbedWidgets`) has valid widgets.
- `import.rs`: `oku-slides import`, `ImportMode` `retrofit` or `hifi` (into the session's by `From`) gives
  the `ImportTarget`; `session::import` applies and renders it, notes overflows (text fields past their
  page, in retrofit boxes past the bottom margin) and, in hi-fi unless `--reference none`, scores it
  at once (the witness chosen by `session::import`, the app's `ReferenceJob::run`, then `score_deck`;
  `auto` = `[import] witness`, `office2pdf|soffice` override it, `none`; t-905); saved like `save`,
  report optional.
- `bench.rs`: render, export and save latencies.

## Startup

`main`: `exit_if_worker` → `color_eyre::install` → `telemetry::init` → `Platform::load`
→ localized `command` parse → `run`.

## Dev server (`serve.rs`, `guard.rs`, `sse.rs`)

`Shared` holds a `Workspace`, the agent `Stack` and `Stt`; `serve_connections` runs with
`Limits` from `[serve]`. `handle` calls `guard::check` first (ADR-0003; off loopback also
the launch `Token` of the printed URL, by Bearer header or cookie), then routes `POST
/api`, `/api/workspace`, `/api/stt`, `/api/agent`, `GET /api/events` (SSE: `Clients`,
`SseBody`), and static files from `--ui` (default `ui/dist`). Ctrl-C stops it in `GRACE`.

## Extension points

- A command: a `Cmd` variant, its help keys in `command` and `cli.ftl`, an arm in `run`.
- An API route: an entry in `API` and a branch in `Shared::api`.

## Dependencies

`oku-slides-core`, `oku-slides-render`, `oku-slides-render-worker`, `oku-slides-pdf`,
`oku-slides-session`, `oku-slides-import`, `oku-slides-agent` (`oku-slides-reference` in tests),
`oku-slides-stt`, `oku-slides-platform` (feature `listen`), `oku-slides-i18n`.
