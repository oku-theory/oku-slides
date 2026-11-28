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
- `RemoteCompiled` (`compiled.rs`): a generation; `svg`/`png`/`pdf` by request; its `Drop` queues
  `Release`, sent before the next request: a `Drop` never takes the link lock.
- `RemoteReference` (`reference.rs`): `RenderWorker::reference(witness, pptx, fonts, slides)` has the
  `ReferenceWitness` (`Office2Pdf`, or `Soffice` under its own timeout) render a span; `png(page,
  width)` by request; `Drop` queues `ReleaseReference`. No pack blamed; another epoch: `Stale`.
- `extract_document` (`extract.rs`, t-1755): `oku_slides_extract` in its own worker, no blame. `Extracted`:
  Markdown then images (past frame or `max_image_bytes`: dropped, in the `Images` cut); the host checks
  lengths, bounds, sniffed `RASTERS`. Or `ExtractRefused`. Debug-only `OKU_SLIDES_TEST_EXTRACT_*` hooks.
- `Link` (`link.rs`, internal): transport (child process or thread), timeout, memory sampled
  every 100 ms (`rss.rs`: `phys_footprint` on macOS, `footprint.rs`, resident memory elsewhere),
  restart and replay of `ReplayState` (slots' last assets forgotten). The child gets
  `--timeout-ms`, `--memory-bytes`, `OKU_SLIDES_RENDER_WORKER=1` (then `InProcess`: no respawn chain).
- `Sandbox` (`sandbox.rs`, macOS, t-912): `sandbox-exec -p worker.sb` around the worker: writes only
  in a fresh work dir (`TMPDIR`, `--work-dir`), no IP, execs itself and `with_soffice`'s program
  only. `Hello`'s canary outside that dir must be denied, or the worker is refused (`WorkerLost`).
- `serve` (`serve.rs`): the worker loop, run by `exit_if_worker` on `--render-worker`, under the
  watchdog (`watchdog.rs`): request > 2 × timeout, memory over the limit for 1 s, parent gone (Windows:
  pid free or reused, by start time) → exit 3. Frames go to a copy of stdout; stdout points to stderr.
- `Hello` starts each connection: another `PROTOCOL_VERSION` (10) is refused (`Protocol`). Before each
  request, unused fonts are dropped (`DropFonts`); `registry_of`: a set's registry; `trial_slot` blames none.
- `Request`/`Response` (`protocol.rs`), frames (`frame.rs`): JSON header and raw payload, bounded by `FrameLimits`
  (`max_frame_mib`): header an eighth, SVG a quarter, extraction its text plus image bounds, others the whole.
- Data (t-1690): `RenderSlot::render` sends `PutData`/`DropData` for data whose `Arc` or base path changed, `PatchData` (a `CellPatch`) when only cells did (`MissingData`: resent whole once); only bases `set_known_bases` names get a path; `set_results`: a ready query result as `PutResult` (a table over its base), a pending one as `DropData`.
  `base_files.rs`: paths inside `Hello`'s `bases` dir only, opened `O_NOFOLLOW`, read by the slots' `BaseRead`.
  `DataWorker` (`data_worker.rs`): own link at `data_timeout_ms`/`data_memory_mib` (`DataWorker::query`: `query_*`); `read`, `check_column_type`, `compact`, `run_query` (`RunQuery`, `serve_query.rs`; timeout → `QueryTimeout`; `engine_version`) write `.tmp-*` in the work dir, else the store; adopted.

## Extension point

A new output of a page: a request in `protocol.rs`, its branch in `serve.rs`, a method on `RemoteCompiled`.

## Dependencies

- `oku-slides-render` (`Renderer`, errors, registry probe), `oku-slides-reference` (witness, rasterization),
  `oku-slides-extract` (documents, `RASTERS`), `oku-slides-import` (`ReadLimits`), `oku-slides-data` (`engine`: polars, whose
  never-called polars-io `cloud` brings `reqwest`, `hyper`, `h2`, `object_store`: the only HTTP crates `just render-offline-deps` lets in, through polars-io only; `(deny network*)` walls them off),
  `oku-slides-core`, `oku-slides-i18n`, `oku-slides-platform` (`RenderLaunch`); `serde`, `serde_json` (`float_roundtrip`),
  `sha2`, `hex`, `snafu`, `tracing`, `sysinfo`, `tempfile`, `libc` (`dup2`, `proc_pid_rusage`, `O_NOFOLLOW`).
