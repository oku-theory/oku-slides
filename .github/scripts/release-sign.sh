#!/usr/bin/env bash
# Signs a draft release's update bundles with the real updater key (release.yml sign job, t-977).
#
# Usage: release-sign.sh LATEST_JSON BUNDLES_DIR VERSION REPO OUT_DIR
#   LATEST_JSON  latest.json of the draft, as tauri-action wrote it (throwaway signatures)
#   BUNDLES_DIR  directory holding every bundle named by its URLs, under its asset name
#   VERSION      release version without `v` (from the version job, not from latest.json)
#   REPO         owner/name, e.g. oku-theory/oku-slides
#   OUT_DIR      receives `<bundle>.sig` for each distinct bundle and the new latest.json
# Environment: TAURI_SIGNING_PRIVATE_KEY (base64 of the minisign secret key file, as Tauri
# stores it) and TAURI_SIGNING_PRIVATE_KEY_PASSWORD (may be empty). Never pass them as arguments.
#
# Refuses a latest.json whose version is not VERSION (or vVERSION), or any platform URL that is
# not https://github.com/REPO/releases/download/vVERSION/ plus a plain asset name (the rule of
# update-mirror.yml, in release-lib.sh). Each distinct bundle is signed once with the trusted comment
# `timestamp:<unix>\tfile:<name>\tversion:<VERSION>`, which requireSignedVersion checks in the app
# (t-1210); every platform key gets the signature of its URL. Prints the `.sig` names produced.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=release-lib.sh
. "$here/release-lib.sh"

die() { echo "::error::release-sign: $*" >&2; exit 1; }

[ "$#" -eq 5 ] || die "usage: release-sign.sh LATEST_JSON BUNDLES_DIR VERSION REPO OUT_DIR"
latest=$1 bundles=$2 version=$3 repo=$4 out=$5

[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "unreadable version: $version"
[[ "$repo" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]] || die "unreadable repo: $repo"
[ -f "$latest" ] || die "$latest not found"
[ -d "$bundles" ] || die "$bundles is not a directory"
[ -n "${TAURI_SIGNING_PRIVATE_KEY:-}" ] || die "TAURI_SIGNING_PRIVATE_KEY is not set"
[ -n "${TAURI_SIGNING_PRIVATE_KEY_PASSWORD+set}" ] || die "TAURI_SIGNING_PRIVATE_KEY_PASSWORD is not set"
command -v minisign > /dev/null || die "minisign not found"
command -v jq > /dev/null || die "jq not found"

prefix="https://github.com/$repo/releases/download/v$version/"


listed=$(jq -r '.version // ""' "$latest") || die "$latest is not valid JSON"
[ "$listed" = "$version" ] || [ "$listed" = "v$version" ] \
  || die "latest.json announces version '$listed', $version expected"
jq -e '.platforms | type == "object" and length > 0' "$latest" > /dev/null \
  || die "latest.json lists no platform"

tmp=$(mktemp -d "${TMPDIR:-/tmp}/release-sign.XXXXXX")
trap 'rm -rf "$tmp"' EXIT
chmod 700 "$tmp"

jq -r '.platforms | to_entries[] | "\(.key)\t\(.value.url // "")"' "$latest" > "$tmp/keys"
: > "$tmp/names"
while IFS=$'\t' read -r key url; do
  name=$(bundle_name "$prefix" "$url") || die "$key: URL refused: '$url'"
  grep -qxF "$name" "$tmp/names" || printf '%s\n' "$name" >> "$tmp/names"
done < "$tmp/keys"

# The secret key only ever sits in this 0600 file, removed on exit with $tmp.
(umask 077 && printf '%s' "$TAURI_SIGNING_PRIVATE_KEY" | base64 -d > "$tmp/secret.key") \
  || die "TAURI_SIGNING_PRIVATE_KEY is not base64"

mkdir -p "$out"
sigs='{}'
while read -r name; do
  file="$bundles/$name"
  [ -f "$file" ] && [ ! -L "$file" ] && [ -s "$file" ] || die "$name: bundle missing or empty in $bundles"
  comment=$(printf 'timestamp:%s\tfile:%s\tversion:%s' "$(date +%s)" "$name" "$version")
  printf '%s\n' "$TAURI_SIGNING_PRIVATE_KEY_PASSWORD" \
    | minisign -S -s "$tmp/secret.key" -m "$file" -x "$tmp/$name.minisig" -t "$comment" > "$tmp/minisign.log" 2>&1 \
    || { cat "$tmp/minisign.log" >&2; die "$name: minisign could not sign (wrong key or password?)"; }
  sig=$(base64 < "$tmp/$name.minisig" | tr -d '\n')
  printf '%s' "$sig" > "$out/$name.sig"
  sigs=$(jq -c --arg n "$name" --arg s "$sig" '. + {($n): $s}' <<< "$sigs")
done < "$tmp/names"

# Every key, `-nsis`/`-app`/`-deb`… included, takes the signature of its URL; urls, notes and
# pub_date are kept as tauri-action wrote them.
jq --arg v "$version" --arg prefix "$prefix" --argjson sigs "$sigs" '
  .version = $v
  | .platforms |= map_values(.signature = $sigs[.url | ltrimstr($prefix)])
' "$latest" > "$tmp/latest.json"
mv "$tmp/latest.json" "$out/latest.json"

sed 's/$/.sig/' "$tmp/names"
