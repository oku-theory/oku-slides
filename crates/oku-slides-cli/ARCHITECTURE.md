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
- `library.rs`: packs and themes found by id, folder or name (`resolve_pack`,
  `embed_theme`, `pack_library`, `theme_library`, `beside_deck`).
- `pack_render.rs`: `DeckWidgets::probe` probes a deck's embedded pack in the render
  worker (in up to `PROBE_ROUNDS` calls); `render` renders in memory or in the worker.
- `import.rs`: `oku-slides import`, `ImportMode` `retrofit` (onto an existing pack and
  theme) or `hifi`; one transaction, then saved like `save`, with an optional report.
- `import_overflow.rs`: text fields rendered past their page become `Overflow` losses.
- `import_reference.rs`: hi-fi `--reference auto`: the witness in a render worker of its own, 20
  slides at a time, each Oku page cropped to the source page at 960 px, scored into the report.
- `import_hifi.rs`: hifi preparation: generated pack and theme with an identity from the
  `.pptx` bytes, font catalog, probe check against the declared schemas.
- `system_fonts.rs`: copies the system faces of the families a presentation asks for.
- `bench.rs`: render, export and save latencies.

## Startup

`main`: `exit_if_worker` → `color_eyre::install` → `telemetry::init` → `Platform::load`
→ localized `command` parse → `run`.

## Dev server (`serve.rs`, `guard.rs`, `sse.rs`)

`serve` holds a `Workspace` (session tabs), the agent `Stack` and `Stt` in `Shared`, and
runs through `serve_connections` with `Limits` from `[serve]`. `handle` calls
`guard::check` first (ADR-0003), then routes `POST /api`, `/api/workspace`, `/api/stt`,
`/api/agent`, the `GET /api/events` SSE stream (`Clients`, `SseBody`), and static files
from `--ui` (default `ui/dist`). Ctrl-C stops it within `GRACE`.

## Extension points

- A command: a `Cmd` variant, its help keys in `command` and `cli.ftl`, an arm in `run`.
- An API route: an entry in `API` and a branch in `Shared::api`.

## Dependencies

`oku-slides-core`, `oku-slides-render`, `oku-slides-render-worker`, `oku-slides-pdf`,
`oku-slides-session`, `oku-slides-import`, `oku-slides-agent`, `oku-slides-stt`,
`oku-slides-platform` (feature `listen`), `oku-slides-i18n`.
