# Lot A: updates and model downloads (t-1052, t-976, t-979, t-985, t-986, t-984, t-977, t-978, t-1210)

**Parent task:** t-1355. **Steps:** one task per ticket, then review and gates (sub-tasks
t-1356 to t-1366). User decision (2026-10-31): t-1210 uses the updater plugin's
`requireSignedVersion`. v2 after a 15-reviewer plan review (findings folded in below).

**Goal:** resumable model downloads, a sturdier update flow whose state lives in tested platform
code, and an update signing pipeline where the key is isolated and each signature is bound to
its version.

## Task 1: Model downloads resume a cut file (t-1052)

- `Storage::appender` (OpenDAL `append`) and `Storage::reader` → `StreamReader::read_chunk`, in a
  new `storage/stream.rs` with `StreamWriter` (storage.rs stays under 500 lines). `Stream` errors
  carry an `action` (read/write). Tests in `storage_tests.rs`: append creates and extends, chunked
  read to EOF.
- Pure decisions in `models/resume.rs` (rstest tables in `models/resume_tests.rs`):
  `parse_content_range(header, start)` accepts only `bytes L-E/T` with start `L`, `E >= L`,
  `T` = `*` or above `E` (a capped slice, RFC 9110), checked arithmetic; `retryable(&ureq::Error)` = status 429/500/502/503/504,
  `Io`, `Timeout`.
- `fetch(agent, url, from: Option<u64>)` sends `Accept-Encoding: identity`, and `Range: bytes=L-`
  when `from` is set, on its own thread (`models/download/transport.rs`). `Head { status, length,
  content_range }`, classified by `resume::answer`. Writer opened after the head.
- Per file: unpinned files (no SHA-256) drop any `.part` and never resume (their revision moves).
  A pinned file with a `.part` of length `L > 0` re-hashes it chunk by chunk (calling `progress`, so
  cancel works; an unreadable `.part` is dropped and fetched again) and asks from `L`. 206 with a
  valid `Content-Range` from `L` appends; 200 truncates; a 206 without a valid range restarts from
  zero at once, without spending a retry (once per file, error `Range` after); 416 verifies the
  `.part` by its hash and renames it, a mismatch restarting once like any stale prefix. The
  hasher state is kept in memory across attempts of one run (no second re-hash).
- Byte counts are whole-file: `got` starts at `L`, the expected total is `L + Content-Length`
  (or the `Content-Range` total); progress reports `done + got`. A body that ends early (ureq's
  `UnexpectedEof` read error, or a clean end below the total) is a short body: retried, and once
  retries run out it reports `Incomplete { got, expected }` (whole-file), the `.part` kept. A
  clean end that brought new bytes (a capped slice) is continued at once, no retry spent. A body
  longer than its expected total is refused before writing past it.
- Retries: retryable errors, `HttpRead`, `Stalled` (before or during the body) and a short body,
  with the linear backoff. The `.part` is kept (closed) on these, on write errors and when
  retries run out. The counter resets only when the `.part` passes its high-water mark by at
  least 1 MiB, so a cutting, range-stripping proxy cannot loop for ever; a file spends at most
  `RETRY_BUDGET` (8) × `download_retries` failures in all, which bounds request threads left
  blocked on dead connections. A cancel deletes the `.part` (re-hash included: cancel = forget).
  A SHA mismatch deletes it; if the attempt had resumed, one restart from zero follows (a stale
  `.part` from an older pin), else `Integrity`.
- Tests in `models_resume_tests.rs` (raw-TCP scripted hub): cut then resumed with `Range`, cut
  without retries keeps the `.part`, resume on the next run, 200 to a range restarts, 416 on a
  longer `.part` restarts, stale prefix restarts once and succeeds, pinned and unpinned mixed,
  headers then stalled body, half-open socket after 5 MiB, short body → `Incomplete`, write error
  keeps the `.part`, range-stripping cutting proxy fails after the retries, redirect keeps `Range`,
  cancel during a resume deletes the `.part`; after review: complete `.part` + 416, refused range
  with zero retries, capped slices, over-long body, progress resets the count, retry budget, 503
  then 200, 404 fatal, cancel during re-hash, unreadable `.part`. Tests live in
  `models/download_tests.rs` (hub helpers in `download_hub_tests.rs`). `oku-slides-stt`:
  `Incomplete` on its kind and in French.
- Size cap per file: out of scope (`PinnedFile` has no size; backlog ticket).
- Docs: `download` doc, `models.rs`, `download_retries` comment in `config.default.toml` and
  `config.rs`, platform `ARCHITECTURE.md` and `AGENTS.md` (resume rule).

## Task 2: Periodic update check survives sleep (t-976)

- Platform `updates`: `CHECK_TICK` (1 h) and `check_due(last, now, interval)` using
  `now.duration_since(last)` (a clock gone backwards is due; no `SystemTime + Duration` overflow).
- Host `spawn_checks`: check at launch, then every `CHECK_TICK` check when due (the interval
  counts whole hours, so never shorter).
  `Updates.last_check: Mutex<Option<SystemTime>>` set after each successful check, manual ones
  included. The "accepted, t-976" doc goes; the doc says a wake may wait up to one tick.
- Tests: `check_due` rstest table (none, just under, exactly, past, clock backwards, `u64::MAX`
  hours); regression `tests/regressions/issue_0976_check_after_sleep.rs`.

## Task 3: "Later" kept by the host, per version (t-979)

- Platform `updates::UpdateMarks` (storage + data dir, blocking calls): `failed_version`,
  `record_failure(version)`, `take_notice`, `later`, `set_later`, `clear_later`; files under
  `updates/`, read with `read_limited` (64 bytes) and accepted only as `major.minor.patch`.
  `UpdateStatus::new(offer, installing, later)` sets `dismissed: bool`. Module `updates/marks.rs`,
  tests in `updates/marks_tests.rs`; the mark file constants are `pub(crate)`.
- Host: `later_update(version)` stores it only if it is the current offer's version (decided
  under the offer's lock), returns the status and emits `update-changed`; `check_update` (the
  command, not the shared `check`) clears it first and emits at once, so a failed check still
  shows the offer again. All mark I/O in `spawn_blocking`. Commands registered in `main.rs`, `workspace_window` first.
- UI: `UpdateStatus.dismissed`, `UpdateApi.later(version)`; `useUpdate` drops its local state,
  applies the returned status, reports a rejection. Render tests: later calls the host,
  `dismissed` hides unless installing, a rejection reports.

## Task 4: Say an install failed after the restart (t-985)

- `record_failure` writes `failed-version` and `failed-notice` (the version, never text).
  `Updates::new` reads the notice into memory (`UpdateMarks::notice`, a malformed one removed);
  `take_update_notice` (after `workspace_window`) removes it from disk (`clear_notice`) and
  hands the first window the text of `update-error-install` with `$version` (20 locales),
  localized at take time through `updates::InstallFailure`, then `null`.
- UI: `useUpdate` asks once per `api` through a shared promise (StrictMode-safe; a failed
  request is forgotten so the next mount asks again) and exposes
  `notice` + `dismissNotice`; Workspace shows it as a lasting `UpdateNoticeBar` (close button
  `ui-update-notice-close`, 20 locales, `role="alert"`). Render tests: shown,
  `null` shows nothing, StrictMode asks once.
- Scope: macOS and AppImage; the Windows installer exits the process itself.

## Task 5: Distinct message for "installation already in progress" (t-986)

- `Inner::AlreadyInstalling` (backtrace) replaces `Refused { why: "already installing" }`, maps to
  `UpdateStage::Busy` (`update-error-busy`, 20 locales). Platform key test table and the host
  `updates_tests.rs` table updated.

## Task 6: Dependabot for the pinned actions (t-984)

- `.github/dependabot.yml`: `github-actions`, weekly, grouped, ignoring `dtolnay/rust-toolchain`
  (its ref is the Rust version). PR titles get a `t-N` at squash. audit.yml floating tags: out of scope.

## Task 7: Sign update bundles in a separate job (t-977)

- `build`: leaves the `release` environment; signs Tauri's updater artifacts with a throwaway key
  (`cargo tauri signer generate --ci`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ""` explicit, `shell: bash`) so tauri-action still builds them.
- `sign` job (ubuntu, `release` env, needs `version` and `build`, `contents: write`): apt
  `minisign` only, no Rust tooling. `.github/scripts/release-sign.sh` reads the draft's
  `latest.json`, refuses any platform key whose `url` is not the release prefix plus a plain
  name, signs each distinct bundle URL once (every key, `-nsis`/`-app`/`-deb`… included, gets the
  signature of its URL) with `minisign -S -t "timestamp:…\tfile:…\tversion:<v>"` (`<v>`
  from the `version` job, key via env, password on stdin) and writes `latest.json` from these
  trusted inputs. The workflow uploads with `--clobber` and deletes the other `.sig` assets.
- After review (security): the sign and publish jobs take their scripts and the pubkey from the
  run's own commit on main (`github.sha`), never from the tag; the version job refuses a tag whose
  commit is not on main (build jobs hold a write token and could plant one). The URL rule lives
  once in `release-lib.sh`; `release-fetch.sh` downloads `latest.json` and its bundles. During a
  key rotation the gate verifies against the `UPDATER_ROTATION_PUBKEY` repository variable.
  Read-only build jobs with per-platform hashes: follow-up (ADR-0014).
- `publish` gate, `.github/scripts/release-gate.sh`: `.version == <v>`, every key's URL prefixed
  and plain, each signature (from `latest.json`) verified with `minisign -V` against `tauri.conf.json`'s
  pubkey on main, trusted comment whose tab-split fields include exactly `version:<v>`; the four platform keys present.
- `.github/scripts/release-sign-test.sh` (in `just ci`): fake release, throwaway and real keys;
  asserts sign then gate pass, and the gate refuses a throwaway signature, another version,
  `version:18.0.0`, and a foreign URL.
- **Not checkable before a real release:** the workflow wiring itself (environment secrets,
  `gh release upload --clobber` on a draft, tauri-action accepting the throwaway key). A
  one-platform dispatch would test it but needs an unpublished tag, which only the next release
  PR creates.

## Task 8: ADR-0014 on backing up and rotating the updater signing key (t-978)

- Where the key and password live and their offline backups; the lost-key and leaked-key
  procedures (rotation release signed with the old key carrying the new pubkey, a transition
  period, apps left behind); what isolating the key buys (no lasting theft) and does not (a
  compromised build can still get its upload signed); `requireSignedVersion` gives nothing once
  the key leaks. `src-tauri/AGENTS.md` pitfall points to ADR-0014; ADR index.

## Task 9: Bind the update signature to the offered version (t-1210)

- `tauri.conf.json`: `plugins.updater.requireSignedVersion: true` (published 8.0.0 signatures
  carry `version:8.0.0`). The existing `mirror_prefix_matches_tauri_endpoint_and_workflow` test
  also checks the flag. Cargo.toml notes the plugin must stay at 2.14 or later.
- ADR-0010 downgrade paragraph: closed for apps from this release on; residuals (`file:` not
  bound, a stale mirror can freeze updates). `src-tauri/AGENTS.md` rule.

## Task 10: Lot A review and gates

- Multi-angle review of the diff; `just ci`; `cargo nextest run -p oku-slides-app`; `just e2e`;
  close the nine tickets and the sub-tasks; one squash commit citing every ticket.
