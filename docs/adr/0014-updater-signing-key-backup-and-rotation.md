# ADR-0014: Updater signing key, backup and rotation

- **Status:** accepted
- **Date:** 2026-10-31
- **Tasks:** t-978, t-977, t-1210
- **Spec:** [`2026-10-18-auto-update-design.md`](../superpowers/specs/2026-10-18-auto-update-design.md)

## Context

Installed apps accept an update only if its bundle carries a minisign signature from the key
whose public half is `plugins.updater.pubkey` in `tauri.conf.json`. That key is the root of
trust of every update: losing it ends automatic updates, leaking it lets anyone push code to
every installed app. Until now the key reached the build jobs of `release.yml`, which also run
npm, `build.rs` and third-party actions, and nothing said how to back it up or replace it.

## Decision

- **Where it lives.** The secret key (base64 of the minisign key file, as Tauri writes it) and
  its password are the secrets `TAURI_SIGNING_PRIVATE_KEY` and
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` of the GitHub environment `release` (branch `main`).
- **Offline backup, key and password apart.** The maintainer keeps the key file in one place
  (the password manager) and the password in another (an offline copy, e.g. printed or on an
  encrypted drive kept at home). Either alone is useless; both are needed to rebuild the secrets.
- **Only the sign job holds the key (t-977).** Build jobs sign with a throwaway key so
  tauri-action still produces the updater artifacts. The `sign` job of `release.yml` (ubuntu,
  `release` environment, apt `minisign`) re-signs each bundle named in `latest.json` with
  `.github/scripts/release-sign.sh`, after refusing any URL outside the release. Its scripts come
  from the run's own commit on main, never from the tag, and the version job refuses a tag whose
  commit is not on main: build jobs hold a write token and could plant one. The publish gate
  (`release-gate.sh`) verifies every signature against the pubkey of `tauri.conf.json` on main
  before the draft goes out.
- **Signatures bind the version (t-1210).** The trusted comment is
  `timestamp:…\tfile:…\tversion:<v>`, and `plugins.updater.requireSignedVersion` makes the app
  refuse a bundle whose signed version differs from the one `latest.json` offers.
- **Yearly restore drill.** Once a year, rebuild the key from the two backups, decode it
  (`base64 -d`), sign a scratch file with `minisign -S -t "version:0.0.0"`, and verify it with
  `minisign -V -p` against the base64-decoded `plugins.updater.pubkey`. A failed drill is treated
  as a lost key.

## Procedures

**Lost key** (secrets and both backups gone, or password forgotten). Installed apps can no
longer be updated automatically: nothing they accept can be produced. Generate a new key
(`cargo tauri signer generate`), store it as above, put the new pubkey in `tauri.conf.json`, and
release. Users must download and install that release by hand; say so in its release notes and
in the README, since the old apps will simply never see an update.

**Leaked key** (secret and password, or anything that signed with them, exposed). An attacker
can sign any bundle with any version: `requireSignedVersion` gives nothing then, and the mirror
or a hijacked GitHub account suffices to serve it. Rotate:

1. Generate the new key. Ship a **rotation release** whose `tauri.conf.json` carries the new
   pubkey, signed with the **old** key, so installed apps accept it. Set the repository variable
   `UPDATER_ROTATION_PUBKEY` to the old pubkey first: the publish gate then verifies against it
   instead of the new one in `tauri.conf.json`.
2. **Transition period**: keep signing the following releases with the old key. GitHub
   "latest" and the mirror serve only the newest release, so an app that skipped versions must
   still find a bundle it trusts, and from it reach a build with the new pubkey. Last a few
   months, longer if issue reports show old versions still in use.
3. Switch the `release` secrets to the new key and delete `UPDATER_ROTATION_PUBKEY`; from then
   on, apps older than the rotation release cannot update and need a manual install (release
   notes, README).

Revoke the old secrets in every backup once the switch is done. During steps 1 and 2 the
attacker keeps the power to sign: rotation shortens the exposure, it does not end it at once.

## Consequences

- Build code (dependencies, `build.rs`, actions) can no longer read the key or keep a copy, as
  long as main is protected: the sign job runs only scripts from main. A compromised build is a
  one-release problem, not a lasting theft.
- It does not stop a compromised build from uploading a malicious bundle under an expected name:
  build jobs share a repository write token (tauri-action uploads to the draft), so one job can
  replace any platform's bundle, and the sign job signs whatever the draft holds at an allowed
  URL. The URL allow-list (release prefix, plain asset names) only keeps it from signing files
  outside the release. Read-only build jobs handing bundles over as artifacts, with each
  platform's hashes checked before signing, would narrow this (follow-up).
- The sign and publish jobs download every bundle (a few hundred MB) and need `minisign` from
  apt; `just release-sign-test` checks both scripts on a fake release.
- `plugins.updater.pubkey` changes only through the procedures above (`src-tauri/AGENTS.md`).
