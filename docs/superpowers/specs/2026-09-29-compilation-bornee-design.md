# Bounded Typst compilation in a worker process — design

Date: 2026-09-29 · Task: t-327

## Goal

No Typst compilation can freeze or crash the app anymore. Everything
that runs Typst coming from a pack — deck render, registry probe,
charts, export — runs in a **persistent subprocess**, bounded in
time and memory; beyond that, the host kills it, restarts it and falls back to
the base widgets.

"Portable widgets" program, three sub-projects, each with its own spec:

0. **Bounded compilation** (this spec, t-327).
1. Portable packs (t-328): unique identity and version of packs, copy
   of the pack embedded in the file, which is authoritative at render time.
2. Widget creation (t-329): the agent writes and fixes `widgets.typ`,
   advanced Typst editor in template mode.

Sub-project 1 makes the app run the Typst of a received PDF; sub-project 2, Typst
written by trial and error. This sub-project is their prerequisite.

Out of scope: a trust gate ("trust this pack"),
OS-level network and file isolation (the Typst `World` already serves
only its virtual file system), a worker pool.

## Findings

- `typst::compile` has no cancellation API (0.15.1). Its internal guards
  (`while` iterations, call depth, introspection passes) cover
  neither `for _ in range(1e12)` nor allocation: neither time nor
  memory is bounded.
- A pack's code runs on **every render** (the `render` of each
  widget), not only in the probe: validating a pack once is not enough,
  a widget may loop only with certain parameters.
- A Rust thread cannot be killed: abandoning it leaves the CPU consumed
  until the app exits, and memory stays unbounded.
- `Compiled` wraps a `PagedDocument`, which is not serializable. `Layout` and
  its boxes already are (`serde`).
- macOS does not enforce `RLIMIT_AS`/`RLIMIT_DATA`: the memory limit is
  measured from the host.

## Decisions

- **One subprocess per session**, persistent to keep the incremental
  cache (`comemo`) between two renders; one request in flight at a time,
  as today under the session. It hosts several **slots**
  (`slot`), one `Renderer` each: the deck, the template previews, the
  agent's variants. A kill restarts them all.
- **Two transports, one protocol.** `Process` (subprocess, bounds
  enforced) and `InProcess` (`serve` on a thread, in-memory pipes, timeout
  reported but nothing is killed). `Platform::load()` — the binaries — chooses
  `Process(current_exe)`; `Platform::with_settings()` — tests and tools —
  `InProcess`.
- **Self re-execution**: `std::env::current_exe()` launched with
  `--render-worker`. No separate binary: the host and the worker are
  always the same version.
- **Watchdog in the host**: per-request timeout, sampling of the worker's
  resident memory (RSS) every 100 ms; beyond the limit, `kill`. Same
  mechanism on all three OSes.
- **The base registry stays in the host**, loaded once per process
  (`registry.rs`): embedded, trusted code; the session needs it
  to validate without a round trip. Only packs go through the
  worker.
- **A pack's registry is built in the host.** The worker returns
  the raw JSON of the probe; the host derives the `WidgetRegistry` from it
  (`registry_from_probe`) and keeps it by fingerprint: the session compares
  registries with `Arc::ptr_eq`, and `WidgetRegistry` does not need to be
  serialized.
- **The fallback policy lives in the session**, not in the client:
  the invariant of `deck_widgets.rs` (one registry for the session, the
  sources and the render) forbids the client from switching widgets on its own. The
  client reports and marks the faulty fingerprint; the session switches.
- **Resident memory read by `sysinfo`** (without default features,
  `system` only): new dependency, none in the workspace reads the RSS
  of a child process on all three OSes.
- **`Renderer` stays the engine**, public and unchanged in
  `oku-slides-render`; the worker hosts it.

## Architecture

New crate `oku-slides-render-worker` (depends on `oku-slides-render`,
`oku-slides-core`, `oku-slides-platform`, `oku-slides-i18n`):

- `frame`: length-prefixed frames.
- `serve(reader, writer, fonts)`: the worker loop. Reads requests,
  runs them on a `Renderer`, replies. Keeps the last `Compiled` under a
  **generation** number (`u64`, increasing) and the received assets by
  identifier.
- `RenderWorker`: host side, the shared link (`Arc<Mutex<…>>`) with a
  worker: transport, watchdog, restart, faulty fingerprints,
  `probe(pack)`.
- `RenderSlot`: a slot, same surface as `Renderer`:
  `set_language`, `set_widgets`, `render`, `registry`, `chart_palette`.
  `render` returns a `RemoteCompiled`.
- `RemoteCompiled`: handle to a generation. Local: `page_count`,
  `page_hash`, `layout()`, `warnings`. Per request: `svg`, `png`, `pdf`.
- `Limits { timeout, memory }`: read from the configuration.

The entry point of both binaries (CLI `oku-slides`, Tauri app) tests
`--render-worker` at the top of `main`, before any other initialization, and
calls `oku_slides_render_worker::run_from_args()`, which never returns.

## Protocol

Frame: header length (`u32` big-endian), JSON header
(`serde_json`), payload length (`u32`), raw binary payload
(possibly empty). PNG, PDF, assets and fonts travel as is. A
length above `render.max_frame_mib` is rejected (`Frame`), on both
sides.

| Request | Payload | Response |
|---|---|---|
| `SetLanguage { locale }` | — | `Ok` |
| `PutFont { name }` | bytes | `Ok` |
| `PutAsset { id }` | bytes | `Ok` |
| `Probe { pack: { id, source } }` | — | `Probe { json }` (raw probe output) |
| `OpenSlot { slot, fonts: [fingerprint] }` | — | `Ok` |
| `CloseSlot { slot }` | — | `Ok` |
| `SetWidgets { slot, pack: Option<{ id, source }> }` | — | `Ok` |
| `Render { slot, deck, assets: [id] }` | — | `Compiled { generation, page_count, page_hashes, layout, tokens, warnings, chart_palette }` |
| `Svg { slot, generation, page }` | — | `Bytes` (UTF-8) |
| `Png { slot, generation, page, pixel_per_pt }` | — | `Bytes` |
| `Pdf { slot, generation }` | — | `Bytes` |

`PutFont` and `PutAsset` carry a fingerprint (sha-256); `OpenSlot` and
`Render` only cite fingerprints already sent.

Any request may answer `Error { kind, message, diagnostics }`, which
carries a serialized `RenderError` or `RegistryError`. `Svg`, `Png`
and `Pdf` for a generation that is no longer the latest answer `Stale`.

The host sends an asset only if the worker does not have it (set of ids
sent, kept by the client, cleared on restart). `Registries` (memoization
by fingerprint) lives in the worker.

## Configuration

`config.default.toml` of `oku-slides-platform`:

```toml
[render]
# Maximum duration of a Typst compilation or export (ms); beyond it, the
# render process is restarted and the deck renders with the base widgets.
timeout_ms = 10000
# Maximum memory of the render process (MiB).
memory_mib = 2048
# Maximum size of a message exchanged with the render process (MiB).
max_frame_mib = 512
```

Overridable with `OKU_SLIDES_RENDER__TIMEOUT_MS`, etc.

## Failures and fallback

`RenderErrorKind` gains `Timeout`, `OutOfMemory` and `WorkerLost` (unexpected
end of the worker, including a Typst panic, which today
would crash the app), and `Frame` (unreadable or oversized message). Snafu
with backtrace, lowercase messages, localized via Fluent in the 20
languages.

On `Timeout`, `OutOfMemory` or `WorkerLost`:

1. **During `Probe`, `SetWidgets` or a `Render` with a pack**: the
   client marks the fingerprint (sha-256 of the pack source) **faulty for
   the session**; the worker is restarted on the next request, its state
   replayed (language, fonts, slots, widgets). The session, on a
   bounded error with a pack installed, installs the base widgets
   (`install_widgets`), renders again once and shows a notice that
   names the pack and the cause. The deck renders with the
   base widgets: its pack-specific widget elements show
   the "unknown element" box, their parameters are kept.
2. **With the base widgets** (oversized deck): the error is reported as
   is, like a compilation error today; the deck stays
   editable, the last successful render stays displayed. The worker is
   restarted for the next request.
3. **A faulty pack is not retried** in the session. Modifying it changes
   its fingerprint: it is retried.

**Stale handle.** After a new render or a restart, an old
`RemoteCompiled` receives `Stale`; the session re-renders once and
reports the error to the UI only if that re-render fails.

## Consumers

- **Session**: owns a `RenderWorker`; `new_renderer` opens a `RenderSlot`.
  `deck_widgets.rs` gets a pack's registry through `RenderWorker::probe`
  (probe JSON, `registry_from_probe` in the host, kept by
  fingerprint) and installs it through `RenderSlot::set_widgets`.
  Stopping the session closes the worker (closing stdin, then
  `kill` after a short delay).
- **CLI**: `render` and `save` keep the in-memory `Renderer`: they
  load no pack (base widgets, trusted). To be revisited with t-328,
  when the CLI reads the embedded pack. `main` handles `--render-worker`:
  `oku-slides serve` runs a `Workspace`, hence sessions.
- **Workspace**: its `Registries` (base only) becomes `base_registry`.
- **MCP**: goes through the session, nothing to change.
- **Tauri**: flag at the top of `main.rs`.
- The ARCHITECTURE.md files of `render`, `session` and the new crate are
  updated.

## Tests

In build order, with TDD:

1. **Frames**: header and payload round trip; truncated frame;
   length above `max_frame_mib` rejected.
2. **In-memory `serve`** (pipes, no subprocess): `Render` then `Svg`
   of the same generation; `Stale` after a second `Render`; `PutAsset`
   then a deck that uses it.
3. **`RenderWorker` with a real subprocess** (the crate's
   `oku-slides-render-worker` binary, reserved for tests, never
   packaged):
   - render identical byte for byte to the in-memory render for
     `fixtures/roadmap` (SVG of each page, `Layout`);
   - looping pack (`for _ in range(1000000000000) {}` in a widget):
     `Timeout`, fallback to the base, faulty fingerprint, second render without
     a new attempt;
   - pack that allocates endlessly, under a low test `memory_mib`:
     `OutOfMemory`;
   - worker killed from outside: `WorkerLost`, restart, replay, identical
     render.
4. **Regression** (session, `harness = false` test that handles
   `--render-worker` itself): a looping pack no longer freezes the session — the
   render returns in less than `timeout_ms` plus a margin, with the
   base widgets and a notice.
5. **i18n**: keys of the new errors in the 20 languages.
