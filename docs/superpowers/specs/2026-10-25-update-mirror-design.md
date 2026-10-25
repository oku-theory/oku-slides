# Update mirror on our own domain — design

Task: t-1209 (follows the diagnosis in t-1199). Date: 2026-10-25.
Status: approved 2026-10-25. Decision record: [ADR-0010](../../adr/0010-miroir-des-mises-a-jour.md).
Builds on [`2026-10-18-auto-update-design.md`](2026-10-18-auto-update-design.md).
The user's answers: host `updates.okutheory.com` (first given as `oku-theory.com`, which is not
registered), DNS kept at Gandi, Scaleway Object Storage behind Edge Services as backend (Cloudflare
R2 first, dropped: its custom domains need the zone on Cloudflare), reachability from the
corporate network not tested yet.

## Problem

Installed apps read `https://github.com/oku-theory/oku-slides/releases/latest/download/latest.json`
and download bundles from `…/releases/download/…`. GitHub answers both with a redirect to
`release-assets.githubusercontent.com` (or `objects.githubusercontent.com`). Corporate
TLS-intercepting proxies often block that host: on a Linux AppImage behind such a proxy, with
`HTTPS_PROXY` set, every check fails with "Connection reset by peer", and `curl -L` hangs after the
redirect. It is not a TLS problem (`rustls-platform-verifier` trusts the corporate root), and no DNS
CNAME or HTTP redirect from our domain can help: the bytes must be served under our domain.

## Goals

- `latest.json` and every bundle it names are served from `https://updates.okutheory.com`.
- The app checks the mirror first and falls back to GitHub when the mirror cannot be reached.
- A release updates the mirror in the same workflow run, so the mirror never lags behind GitHub.
- A failed mirror upload can be repaired for an already published (immutable) release.

Non-goals: dropping GitHub releases, a second signing key, delta updates, release channels.
GitHub Pages was rejected (100 MB per file, binaries committed to a repo).

## 1. Mirror layout

Scaleway Object Storage bucket (`fr-par`, objects public-read as Edge Services requires) behind an
Edge Services pipeline with the custom domain `updates.okutheory.com` (managed Let's Encrypt
certificate; a CNAME at Gandi points to the pipeline):

| Key | Content | `Cache-Control` |
|-----|---------|-----------------|
| `v<version>/<asset>` | a bundle named in `latest.json`, same file name as on GitHub | `public, max-age=31536000, immutable` |
| `latest.json` | the release's `latest.json`, URLs rewritten to `https://updates.okutheory.com/v<version>/<asset>` | `public, max-age=300` |

Signatures are embedded in `latest.json` (`platforms.*.signature`), so `.sig` files are not
mirrored. Old versions stay in the bucket (cheap, and an app mid-download keeps a valid URL).

## 2. Workflow `update-mirror.yml`

A reusable workflow, two triggers:

- `workflow_call` with input `version` (called by `release.yml`);
- `workflow_dispatch` with input `version`, empty = the latest published release (repair path:
  `release.yml` refuses a version already published).

Only the latest published release (`gh release view`, never a draft) is mirrored: a given version
must equal it, so a mistaken run cannot roll `latest.json` back and queued runs cannot cancel a
newer one into a stale mirror.

One job `mirror` on `ubuntu-24.04`, environment `update-mirror` (restricted to `main`, holds only
`SCW_ACCESS_KEY`, `SCW_SECRET_KEY`, `SCW_BUCKET`; never the signing keys).
Steps:

1. Resolve the latest release's tag; refuse a `version` that is not it.
2. `gh release download <tag> -p latest.json`; check `.version` equals the tag's version.
3. For each distinct `platforms.*.url`: it must start with
   `https://github.com/oku-theory/oku-slides/releases/download/<tag>/`; the asset name is the rest
   (one path segment, no `..`). Download it with `gh release download -p <name>`.
4. Upload each bundle to `v<version>/<name>` with the immutable cache header (`aws s3 cp`, endpoint
   `https://s3.fr-par.scw.cloud`, region `fr-par`, `--acl public-read`, request/response checksums
   `when_required` because S3-compatible stores reject some default CRC headers of recent AWS CLI
   versions).
5. Rewrite every URL with `jq` to the mirror, upload `latest.json` **last**, so a reader never sees a
   `latest.json` whose bundles are missing.
6. Verify through the public host: `latest.json` (cache-busting query) equals the uploaded one,
   retried for up to 7 minutes since the edge may keep the previous one for its max-age, and a `HEAD`
   on every mirrored URL answers 200, with the expected `Content-Length` when the header is present
   (curl with a timeout and retries, so a slow edge neither hangs the job nor fails it once).

## 3. `release.yml`

- The `publish` job exposes `complete` (the gate's output) as a job output.
- New job `mirror`: `needs: [version, publish]`, `if: needs.publish.outputs.complete == 'true'`,
  `uses: ./.github/workflows/update-mirror.yml` with the version. A failure turns the run red; apps
  keep reading the previous, consistent mirror `latest.json` until a manual `Update mirror` run.

## 4. App

- `src-tauri/tauri.conf.json` `plugins.updater.endpoints`:
  `https://updates.okutheory.com/latest.json`, then the GitHub URL. tauri-plugin-updater 2.14 tries
  the next endpoint on a network error or a non-2xx status; the first valid answer wins, even an
  older version (hence goal 3). The check (`CHECK_TIMEOUT`, 60 s for all endpoints) builds its
  client with a 10 s connect timeout (TCP, proxy tunnel, TLS) and a 20 s read timeout, so a mirror
  that hangs (a filtered domain) still leaves time to try GitHub; the plugin is pinned to `~2.14`
  since `configure_client` exposes its reqwest version. A proxy answering 200 with an HTML block page fails the check
  without fallback (the body is not JSON): this is why reachability is verified by hand.
- `oku-slides-platform::updates`: `MIRROR_DOWNLOAD_PREFIX = "https://updates.okutheory.com/v"`;
  `trusted_download` accepts either prefix; on the mirror only
  `v<major.minor.patch>/<one file name of [A-Za-z0-9._+-]>`. `RELEASES_URL` (the page
  opened for `NotifyOnly`) stays on GitHub.

## 5. Setup (by the user, once)

Runbook in ADR-0010: Scaleway bucket, Edge Services pipeline and custom domain, the CNAME at
Gandi, an IAM application whose API key can only read and write objects in that project, the
`update-mirror` environment (deployment branch `main`) with the three secrets, then
a manual `Update mirror` run for the current release, so installed apps find it before the next
release.

## 6. Testing

- `platform`: rstest cases for `trusted_download` on mirror URLs (accepted, `http`, look-alike host,
  `..`, `latest.json`, `/vendor/…`, bad version, empty or nested name).
- Workflow: `actionlint` if installed; the first real run is the manual `Update mirror` for the
  current release, whose verification step is the end-to-end check.
- Manual, done criterion: from the corporate network,
  `curl -sS https://updates.okutheory.com/latest.json` returns the JSON and a `curl -I` on one
  bundle answers 200; then an AppImage there finds and installs an update.

## 7. Residual risk

`latest.json` is unsigned. Whoever can write the bucket (Scaleway key or account, the Gandi
DNS zone) can announce any version and point it at an older bundle still in the bucket, with its
real signature: a signed downgrade, or a frozen version, for every user, since the mirror is read
first. Before, this needed GitHub write access. Mitigations here: a token scoped to the bucket, its
own environment restricted to `main`. Binding the signature to the offered version in the host is
follow-up t-1210.
