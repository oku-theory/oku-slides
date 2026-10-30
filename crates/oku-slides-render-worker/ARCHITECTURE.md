# oku-slides-render-worker

Typst rendering in a worker process, bounded in time and memory: a render that loops or uses
too much memory is killed without taking the app down.

## Key types

- `RenderWorker` (`client.rs`): the shared link; `is_faulty`, `mark_faulty`, `slot`. `blame`: a failure (timeout,
  memory, crash) marks a slot's lone pack unless `is_never_blamed`; a shipped built-in there blames nobody.
- `probe_pack` (`pack_probe.rs`): per-file probe (kind's directory) assembled beside built-in `base`
  (empty probe for `base`), no blame (a timeout is a diagnostic), keyed by `probe_key`, budget per call.
- `RenderSlot` (`slot.rs`): a slot (`set_widgets`, `set_language`, `render`: resends once the
  assets a worker answers `MissingAssets` for, t-1295); its `Drop` queues `CloseSlot`.
- `RemoteCompiled` (`compiled.rs`): a generation; `layout`, `page_hash` local, `svg`/`png`/`pdf`
  by request; its `Drop` queues `Release`. The queue is sent before the next request: a
  `Drop` never takes the link lock.
- `RemoteReference` (`reference.rs`): `RenderWorker::reference(witness, pptx, fonts, slides)`
  has the `ReferenceWitness` (`Office2Pdf`, or `Soffice` under its own timeout, `Error Timeout`
  past it) render the presentation or a span of it; `witness`, `page_count` local, `png(page,
  width)` by request; `Drop` queues `ReleaseReference`. No pack blamed; another epoch: `Stale`.
- `Link` (`link.rs`, internal): transport (child process or thread), timeout, memory sampled
  every 100 ms (`rss.rs`: `phys_footprint` on macOS, `footprint.rs`, resident memory elsewhere),
  restart and replay of `ReplayState` (slots' last assets forgotten). The child gets
  `--timeout-ms`, `--memory-bytes`, `OKU_SLIDES_RENDER_WORKER=1` (then `InProcess`: no respawn chain).
- `Sandbox` (`sandbox.rs`, macOS, t-912): `Link` spawns `sandbox-exec -p worker.sb` around the
  worker. It writes only in a fresh work dir (its `TMPDIR`, `--work-dir`, emptied after each
  reference, deleted with the connection), no IP, exec of itself and the allowed `soffice`
  only (`RenderWorker::with_soffice`). `Hello` carries a canary outside that dir: the worker must
  fail to create it (`Canary::Denied`) or it is refused (`WorkerLost`). No switch; other OSes and
  `InProcess` are unsandboxed.
- `serve` (`serve.rs`): the worker loop, run by `exit_if_worker` on `--render-worker`, under the
  watchdog (`watchdog.rs`): request > 2 × timeout, memory over the limit for 1 s, parent gone →
  exit 3. Frames go to a copy of stdout; stdout points to stderr.
- `Hello` starts each connection: another `PROTOCOL_VERSION` (6) is refused (`Protocol`). Before each
  request, unused fonts are dropped (`DropFonts`); `registry_of`: a set's registry; `trial_slot` blames none.
- `Request`/`Response` (`protocol.rs`), frames (`frame.rs`): JSON header and raw payload, bounded
  by `FrameLimits` (`max_frame_mib`): header an eighth, SVG a quarter, any other payload the whole.

## Extension point

A new output of a page: a request in `protocol.rs`, its branch in `serve.rs`, a method on
`RemoteCompiled`.

## Dependencies

- `oku-slides-render`: `Renderer`, serializable errors, registry probe; `oku-slides-reference`:
  the witness and the rasterization of its pages.
- `oku-slides-core` (`Deck`, `WidgetRegistry`), `oku-slides-i18n`, `oku-slides-platform` (`RenderLaunch`, settings).
- `serde`, `serde_json` (`float_roundtrip`), `sha2`, `hex`, `snafu`, `tracing`, `sysinfo`,
  `libc` (unix: `dup2` of stdout, `proc_pid_rusage`).
