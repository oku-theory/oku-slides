# oku-slides-render-worker

Typst rendering in a worker process, bounded in time and memory: a render that loops or uses
too much memory is killed without taking the app down.

## Key types

- `RenderWorker` (`client.rs`): the shared link (`Arc<Mutex<Link>>`); `is_faulty`, `slot`. A
  pack that makes rendering fail (timeout, memory, crash) becomes faulty and is not run again.
- `probe_pack` (`pack_probe.rs`): per-file probe, never blaming (a timeout is a diagnostic),
  keyed by `probe_key`, total budget `timeout_ms` per call; `trial_slot`: a preview slot
  that blames no pack.
- `RenderSlot` (`slot.rs`): a slot (`set_widgets`, `set_language`, `render`); its `Drop`
  queues `CloseSlot`.
- `RemoteCompiled` (`compiled.rs`): a generation; `layout`, `page_hash` local, `svg`/`png`/`pdf`
  by request; its `Drop` queues `Release`. The queue is sent before the next request: a
  `Drop` never takes the link lock.
- `RemoteReference` (`reference.rs`): `RenderWorker::reference(pptx, fonts, slides)` has the
  witness (`oku_slides_reference::Office2Pdf`) render the presentation, or a span of its slides, in
  the worker, under its bounds; `witness`, `page_count` local, `png(page, width)` by request; `Drop` queues
  `ReleaseReference`. No pack is blamed; a reference from another epoch answers `Stale`.
- `Link` (`link.rs`, internal): transport (child process or thread), timeout, memory sampled
  every 100 ms (`rss.rs`: `phys_footprint` on macOS, `footprint.rs`, resident memory elsewhere),
  restart and replay of `ReplayState`. The child gets its bounds (`--timeout-ms`,
  `--memory-bytes`) and `OKU_SLIDES_RENDER_WORKER=1`: `Platform::load` picks `InProcess` there,
  so a missing `exit_if_worker` does not respawn in a chain.
- `serve` (`serve.rs`): the worker loop; `exit_if_worker` calls it when the binary runs with
  `--render-worker`, under the watchdog (`watchdog.rs`): request > 2 × timeout, memory over the
  limit for 1 s, parent gone → exit 3. Frames go to a copy of stdout; stdout points to stderr.
- Each connection starts with `Hello`: a worker of another `PROTOCOL_VERSION` is refused
  (`Protocol`, `render-protocol`). Before each request, fonts no open slot uses are dropped
  from `ReplayState` and from the worker (`DropFonts`).
- `Request`/`Response` (`protocol.rs`), frames (`frame.rs`): JSON header and raw payload,
  bounded on both sides by `FrameLimits` from `max_frame_mib`: a header at most an eighth, an
  SVG a quarter, a PNG (page or reference), PDF, font, asset or `.pptx` the whole size.

## Extension point

A new output of a page: a request in `protocol.rs`, its branch in `serve.rs`, a method on
`RemoteCompiled`.

## Dependencies

- `oku-slides-render`: `Renderer`, serializable errors, registry probe; `oku-slides-reference`:
  the witness and the rasterization of its pages.
- `oku-slides-core` (`Deck`, `WidgetRegistry`), `oku-slides-i18n`, `oku-slides-platform`
  (`RenderLaunch`, `[render]` settings, logs).
- `serde`, `serde_json` (`float_roundtrip`), `sha2`, `hex`, `snafu`, `tracing`, `sysinfo`,
  `libc` (unix: `dup2` of stdout, `proc_pid_rusage`).
