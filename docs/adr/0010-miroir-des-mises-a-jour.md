# ADR-0010: Update mirror on updates.okutheory.com (Scaleway)

- **Status:** accepted
- **Date:** 2026-10-25
- **Tasks:** t-1199, t-1209
- **Spec:** [`2026-10-25-update-mirror-design.md`](../superpowers/specs/2026-10-25-update-mirror-design.md)

## Context

The updater read `latest.json` and bundles from GitHub releases only. GitHub serves release assets
through a redirect to `*.githubusercontent.com`, which corporate proxies often block: behind one,
every update check fails ("Connection reset by peer"). A DNS alias or a redirect from our domain
cannot help, since the proxy judges the host that serves the bytes. GitHub Pages limits files to
100 MB and would put binaries in a repository.

## Decision

- **Mirror the update files under our own domain.** After the GitHub release is published, the
  `Update mirror` workflow copies every bundle named in `latest.json` to a Scaleway Object
  Storage bucket (`fr-par`) behind Scaleway Edge Services (managed Let's Encrypt certificate),
  served at `https://updates.okutheory.com/v<version>/`, then uploads `latest.json` with its URLs
  rewritten to the mirror, last. Only the latest published release is mirrored. GitHub releases
  stay the source of truth and the signed artifacts are unchanged.
- **Mirror first, GitHub second.** `plugins.updater.endpoints` lists the mirror, then GitHub;
  `download_source` accepts both download prefixes.
- **DNS stays at Gandi.** A CNAME record points `updates.okutheory.com` to the Edge Services
  endpoint; the zone (with its ProtonMail records) does not move. Cloudflare R2 was set aside
  because its custom domains need the whole zone on Cloudflare.
- **Least privilege.** The Scaleway API key lives in its own environment `update-mirror` (branch `main`),
  scoped to the bucket, apart from the signing keys of `release`.

## Consequences

- The mirror host appears in three places that must agree: `tauri.conf.json`,
  `MIRROR_DOWNLOAD_PREFIX` in `oku-slides-platform` and `MIRROR_URL` in `update-mirror.yml`.
- The updater takes the first valid answer: a stale mirror hides newer GitHub releases, so a
  failed mirror job must be repaired (manual `Update mirror` run), not ignored.
- A proxy answering 200 with an HTML block page fails the check without falling back to GitHub.
- A new, uncategorized domain may itself be blocked by corporate filters: reachability is checked
  by hand from such a network, and asking the proxy owner to allow it remains the user's lever.
- A proxy that filters downloads by file type defeats the mirror as well: one seen on corporate
  Linux let `latest.json` through but answered 403 to the AppImage on both hosts, and to a
  `.tar.gz` (t-1212). Nothing on our side was found to get past it; an allowlist entry is the
  lever. The host logs the bundle URL and endpoint of a failed download.
- Scaleway becomes a dependency of updates (Edge Services Starter plan, about €1 a month, plus
  storage); GitHub still serves them if the mirror is down. Objects are public-read, as Edge
  Services requires; they are public on GitHub anyway.
- **Downgrade risk.** `latest.json` is unsigned and the minisign signature does not cover the
  version: whoever can write the bucket can announce a fake version pointing to an older signed
  bundle kept in the bucket, and every app installs it. This widens who can do so from GitHub
  maintainers to holders of the Scaleway key or account, or of the Gandi DNS zone. Binding the version to the
  signature in the host is t-1210; until then, guard the token and the account.

## Setup runbook

1. Scaleway console → Object Storage → create bucket `oku-updates`, region `fr-par`,
   visibility private (objects are uploaded public-read one by one).
2. Edge Services → create a pipeline for that bucket (Starter plan). Endpoint → Configure domain:
   `updates.okutheory.com`, with a free Let's Encrypt certificate managed by Scaleway.
3. Gandi → `okutheory.com` → DNS records: add the CNAME `updates` → the target Scaleway shows in
   step 2 (do not touch the other records). Wait until the certificate is issued.
4. Scaleway → IAM → an application `oku-slides-updates-ci` with a policy granting
   `ObjectStorageObjectsWrite` and `ObjectStorageObjectsRead` (and `ObjectStorageBucketsRead`) on
   the project holding the bucket only, then an API key for it (preferred project: that one).
5. GitHub → Settings → Environments → new environment `update-mirror`, deployment branches:
   "Selected branches" with `main` only (this rule alone keeps other branches from the secrets);
   add required reviewers like `release` if it has them. Secrets: `SCW_ACCESS_KEY`,
   `SCW_SECRET_KEY`, `SCW_BUCKET` (`oku-updates`).
6. Actions → `Update mirror` → run on `main` with an empty version: mirrors the latest release.
7. From the corporate network, with its proxy: `curl -sS https://updates.okutheory.com/latest.json`
   and `curl -sSI <one url from it>` (expect 200).
