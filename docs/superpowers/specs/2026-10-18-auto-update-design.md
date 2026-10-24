# Automatic updates from GitHub releases — design

Task: t-967. Date: 2026-10-18.
Status: approved 2026-10-18; revised the same day after the plan review (15 reviewers), see
"Revision" at the end. Plan: `docs/superpowers/plans/2026-10-18-auto-update.md`.
The user's answers: check and offer (never install without consent), AppImage updates itself
while .deb/.rpm only get a notice, a config switch (no UI checkbox), the host drives the updater
(approach A).

## Problem

Oku Slides ships through `.github/workflows/release.yml`: `tauri-action` builds macOS (Apple
Silicon, Intel), Linux and Windows packages into a draft `v<version>` release, published once
every chosen platform is green. An installed app never learns that a newer release exists:
users must watch GitHub and reinstall by hand. The repo (`oku-theory/oku-slides`) is public, so
the app can read release assets without a token.

## Goals

- The app checks for a newer release at launch and every few hours, and offers it.
- One click downloads, verifies, installs and restarts, without losing open tabs.
- Installs the updater cannot replace (.deb, .rpm, an app run from its disk image) still learn
  that a release exists.
- A manual "Check for updates…" entry on every OS.
- No update traffic from debug builds, from the web dev server (`just web`, `just e2e`), or when
  the user turns checks off.

Non-goals: silent installs, release channels (beta), delta updates, downgrades, release notes in
the app, Apple Developer ID signing (t-968).

## 1. Signing and publishing

- A minisign key pair from `cargo tauri signer generate`, made once by the user and backed up in
  the password manager (losing it strands every installed app; rotation: t-978).
- The private key and its password are secrets of a GitHub **environment** `release`, restricted
  to `main`; the `build` job names it. No repo-level secret.
- `src-tauri/tauri.conf.json`: `plugins.updater.pubkey` and `plugins.updater.endpoints`:
  `https://github.com/oku-theory/oku-slides/releases/latest/download/latest.json`.
  `bundle.createUpdaterArtifacts` is **not** set there: the workflow passes
  `--config '{"bundle":{"createUpdaterArtifacts":true}}'`, so `just build` needs no key.
- `release.yml`: every action pinned by commit SHA (the build step holds the key). `tauri-action`
  gets both secrets, `includeUpdaterJson: true` and `updaterJsonPreferNsis: true`. Each matrix job
  uploads its signed bundle and adds its key to `latest.json`: `darwin-aarch64` and
  `darwin-x86_64` (`.app.tar.gz`), `linux-x86_64` (AppImage), `windows-x86_64` (NSIS `setup.exe`).
- The `publish` job gates `latest.json`: every built platform must be there with a URL and a
  signature, or the run fails. The repository has immutable releases on, so a published release
  takes no more assets: a release with fewer than four platforms stays a draft (with a warning),
  and installed apps keep seeing the previous complete one. A later `aucune` run that adds the
  missing platforms merges into the same draft's `latest.json` and, at four, publishes it as
  latest (checked in Task 5 of the plan). `aucune` fails early when that version is already
  published; v3.0.1, published partial before this rule, stays macOS Apple Silicon only.
- `releases/latest/` only resolves to a published release marked latest.

Known limits:
- macOS stays ad hoc signed. TCC ties the microphone grant (and possibly other grants) to the
  code signature, so after an update macOS may ask again. t-968 tracks Developer ID signing.
- The signing key shares its step with the build (npm, `build.rs`); signing in a separate job is
  t-977.

## 2. Host and config

### Config (`oku-slides-platform`)

```toml
[updates]
# Look for a newer release at launch, then every `check_interval_hours`.
check_automatically = true
check_interval_hours = 6
```

`UpdateSettings` joins `Settings`; `check_interval_hours` is non-zero. `auto_check(debug_build)`
is false in debug builds or when off. `just e2e` and `just web` drive the CLI dev server, never
the Tauri host, so they need no switch.

### `platform::updates` (pure, covered by `just test`)

`src-tauri` is excluded from `just test`; what can be decided without Tauri lives here, for the
host only:

- `InstallMode::detect(&Host)` → `Install` or `NotifyOnly`. `NotifyOnly` on Linux without
  `APPIMAGE`, on macOS when run from App Translocation or `/Volumes/` (the bundle cannot be
  replaced), outside a `.app` bundle on macOS (the plugin would swap the executable's folder),
  in a debug build, and when an install of that same version already failed.
- `UpdateOffer { version, mode }`, `UpdateStatus { offer, installing }`,
  `UpdateProgress { received, total }`: the wire types the UI mirrors.
- `Progress::advance` → report every 1 % (or 1 MiB if the size is unknown), once at the total, and
  `TooLarge` past 512 MiB (checked before the signature, which needs the whole body).
- `trusted_download(url)`: only `https://github.com/oku-theory/oku-slides/releases/download/…`
  (`latest.json` is unsigned; only bundles are).
- `UpdateStage` (check, download, open, unavailable, refused) → Fluent key. A post-restart failure notice is
  ticket t-985.

### `src-tauri/src/updates.rs`

- `tauri-plugin-updater` 2.14 without default features (`rustls-tls`, `system-proxy`): its `zip`
  feature pulls zip 4, under advisories the repo ignores only because nothing calls `extract`;
  v2 bundles do not need it. `tauri-plugin-opener` for the releases page. Both are called from
  Rust only: no capability grants `updater:*` or `opener:*`.
- A check task: at launch, then every interval, until the app is closing. Each check (30 s
  timeout) stores the result, offer or none, in `Updates` and emits `update-changed` without a
  payload; windows re-read `update_status`, which carries `installing`.
- Commands, all async, all refusing the agent window (it shows agent and imported content):
  `update_status`, `check_update` (manual), `install_update`, `open_release_page` (a constant URL).
- `install_update`: refuse while closing, without an `Install` offer, for an untrusted URL, or
  while another install runs. Mark `installing` (signalled), download with progress events and the
  size cap, re-check closing, then in **one blocking job** `state::settle` and `update.install`,
  then `request_restart` (which passes through `RunEvent::Exit`). A failed install records its
  version under the data dir (that version is then `NotifyOnly`) and restarts the current version;
  a failed download clears `installing`. On Windows the NSIS installer ends the process itself.
- `state::shutdown` splits into `settle` (mark closing, save window state, stop agents,
  `Workspace::settle`) and `exit`; both `settle` steps must be safe to run twice.
- macOS app menu: "Check for updates…" under About, emitting `update-check-requested` to the
  focused workspace window.
- Errors: `UpdateError`, opaque Snafu type, a backtrace per variant, each variant mapped
  exhaustively to an `UpdateStage`. A failed automatic check logs at `warn` only; a failed manual
  check or install is shown in the window.

## 3. UI

- `updateApi.ts` (re-exported by name from `api.ts`): the four commands and three events, `null`
  outside Tauri, so every update control is hidden in the web dev server. Event payloads are not
  trusted: `update-changed` is a signal, progress numbers are validated.
- `useUpdate` (one per workspace window): subscribes, then reads the status; re-reads on every
  signal, keeping only the latest reply. "Later" hides that version in this window until the next
  launch (host-side: t-979), but an install in progress always shows. Install and release-page
  errors go to the window's error line.
- `UpdateBar` in workspace windows only, in normal flow under the titlebar, sharing the notice
  surface with `PackNoticeBar` (`.notice-bar`). Text "Oku Slides <version> is available":
  - `Install`: "Install and restart" (primary), "Later";
  - `NotifyOnly`: "See the release", "Later";
  - installing: a progress bar (indeterminate when the size is unknown), buttons disabled.
- "Check for updates…" in the tab menu (`ActionMenu`), before Help, through `useUpdateCheck` in
  `App`: its toast says "Oku Slides is up to date" or the error; a found update shows the bar. The
  macOS menu request runs the same check.
- Strings go through Fluent (English reference, every locale translated).

## 4. Testing

- `platform`: rstest tables for `InstallMode::detect`, `trusted_download`, `UpdateStage` keys in
  English and French; `Progress` steps, total crossing, zero total and both cap cases;
  `UpdateSettings` defaults, env overrides and the zero interval; `UpdateStatus` JSON shape.
- `src-tauri`: `stage_of` table (`cargo nextest run -p oku-slides-app`).
- UI: `useUpdate` (single subscription, StrictMode, unmount, re-reads, reply order, Later,
  installing, progress, install and release-page failures, menu request), `useUpdateCheck`,
  `UpdateBar` (both modes, every button, progress and indeterminate).
- Manual, after merge: a partial release left as a draft then an `aucune` completion (gate, merge
  and publication), then a real
  update on macOS (from `/Applications` and from the `.dmg`), Windows and a Linux AppImage and
  `.deb`: the app relaunches with its tabs, single-instance on.

## Rules added

- `src-tauri/AGENTS.md` invariants: install only after `state::settle` in the same blocking job,
  then `request_restart`, never from the webview; no updater or opener permission in any
  capability, and update commands refuse the agent window.
- `src-tauri/AGENTS.md` pitfall: the updater installs ring as the global rustls provider.

## Revision (2026-10-18, after the plan review)

Changed from the first approved version: `createUpdaterArtifacts` set by CI only; release
environment, pinned actions and a publish gate; no e2e switch (e2e never runs the host); `notes`
dropped; `update-changed` signal instead of an offer payload; `installing` shared by all windows;
timeout, size cap and trusted URL; translocation, disk image and failed-version cases as
`NotifyOnly`; `request_restart` instead of `restart`; updater without its `zip` feature; the
up-to-date message through the tab's toast (one toast per window). Deferred: t-976 (checks across
sleep), t-977 (separate signing job), t-978 (key rotation ADR), t-979 (Later on the host), t-980
(per-command `AppManifest`).
</content>
