#!/usr/bin/env bash
# Verifies a draft release's latest.json before it is published (release.yml publish job, t-977).
#
# Usage: release-gate.sh LATEST_JSON BUNDLES_DIR VERSION REPO PUBKEY [REQUIRED_KEYS]
#   LATEST_JSON    latest.json of the draft, as the sign job uploaded it
#   BUNDLES_DIR    directory holding every bundle named by its URLs, under its asset name
#   VERSION        release version without `v` (from the version job)
#   REPO           owner/name, e.g. oku-theory/oku-slides
#   PUBKEY         the pubkey installed apps trust (base64, as Tauri stores it): tauri.conf.json on
#                  main, or UPDATER_ROTATION_PUBKEY during a key rotation (ADR-0014)
#   REQUIRED_KEYS  comma-separated platform keys that must be present; default: the four
#                  platforms of a complete release (PLATFORMS below)
#
# Checks: `.version` is VERSION or vVERSION; every platform key's URL is
# https://github.com/REPO/releases/download/vVERSION/ plus a plain asset name; the required keys
# are present; every key's signature verifies with minisign against PUBKEY over its bundle, and
# its trusted comment has exactly one `version:` field, equal to `version:VERSION` (what
# requireSignedVersion reads in the app, t-1210). It only verifies: release.yml decides whether
# the release is complete. Exits non-zero with a `::error::` line on the first failure.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=release-lib.sh
. "$here/release-lib.sh"

# The platforms a complete release carries; keep in step with the release.yml matrix.
PLATFORMS=darwin-aarch64,darwin-x86_64,linux-x86_64,windows-x86_64

die() { echo "::error::release-gate: $*" >&2; exit 1; }

[ "$#" -eq 5 ] || [ "$#" -eq 6 ] \
  || die "usage: release-gate.sh LATEST_JSON BUNDLES_DIR VERSION REPO PUBKEY [REQUIRED_KEYS]"
latest=$1 bundles=$2 version=$3 repo=$4 pubkey=$5 required=${6:-$PLATFORMS}

[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "unreadable version: $version"
[[ "$repo" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]] || die "unreadable repo: $repo"
[[ "$required" =~ ^[a-z0-9_-]+(,[a-z0-9_-]+)*$ ]] || die "unreadable required keys: $required"
[ -f "$latest" ] || die "$latest not found"
[ -d "$bundles" ] || die "$bundles is not a directory"
command -v minisign > /dev/null || die "minisign not found"
command -v jq > /dev/null || die "jq not found"

prefix="https://github.com/$repo/releases/download/v$version/"


listed=$(jq -r '.version // ""' "$latest") || die "$latest is not valid JSON"
[ "$listed" = "$version" ] || [ "$listed" = "v$version" ] \
  || die "latest.json announces version '$listed', $version expected"
jq -e '.platforms | type == "object"' "$latest" > /dev/null || die "latest.json has no platforms"

tmp=$(mktemp -d "${TMPDIR:-/tmp}/release-gate.XXXXXX")
trap 'rm -rf "$tmp"' EXIT

printf '%s' "$pubkey" | base64 -d > "$tmp/pub.key" 2> /dev/null || die "the public key is not base64"

for key in ${required//,/ }; do
  jq -e --arg k "$key" '.platforms[$k] | type == "object"' "$latest" > /dev/null \
    || die "$key missing from latest.json"
done

jq -r '.platforms | to_entries[] | "\(.key)\t\(.value.url // "")\t\(.value.signature // "")"' \
  "$latest" > "$tmp/keys"
n=0
while IFS=$'\t' read -r key url sig; do
  name=$(bundle_name "$prefix" "$url") || die "$key: URL refused: '$url'"
  file="$bundles/$name"
  [ -f "$file" ] && [ ! -L "$file" ] || die "$key: bundle $name missing from $bundles"
  [ -n "$sig" ] || die "$key: no signature"
  printf '%s' "$sig" | base64 -d > "$tmp/sig" 2> /dev/null || die "$key: signature is not base64"
  comment=$(minisign -V -Q -p "$tmp/pub.key" -m "$file" -x "$tmp/sig" 2> "$tmp/minisign.log") \
    || { cat "$tmp/minisign.log" >&2; die "$key: signature of $name does not verify with the updater pubkey"; }
  # The trusted comment is tab-separated `name:value` fields; exactly one may name the version.
  versions=$(printf '%s\n' "$comment" | tr '\t' '\n' | grep '^version:' || true)
  [ "$versions" = "version:$version" ] \
    || die "$key: trusted comment of $name binds '${versions//$'\n'/,}', version:$version expected"
  n=$((n + 1))
done < "$tmp/keys"

echo "latest.json v$version: $n platform keys verified (required: $required)"
