#!/usr/bin/env bash
# Downloads a draft release's latest.json and every bundle it names (release.yml, t-977).
#
# Usage: release-fetch.sh VERSION REPO DIR
#   Writes DIR/latest.json and DIR/bundles/<asset name>. Any URL outside the release (the
#   bundle_name rule of release-lib.sh) stops it before anything else is downloaded.
# Needs `gh` with GH_TOKEN.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=release-lib.sh
. "$here/release-lib.sh"

die() { echo "::error::release-fetch: $*" >&2; exit 1; }

[ "$#" -eq 3 ] || die "usage: release-fetch.sh VERSION REPO DIR"
version=$1 repo=$2 dir=$3
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "unreadable version: $version"
[[ "$repo" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]] || die "unreadable repo: $repo"
tag="v$version"
prefix="https://github.com/$repo/releases/download/$tag/"

mkdir -p "$dir/bundles"
gh release download "$tag" --repo "$repo" -p latest.json -O "$dir/latest.json" --clobber
names=()
while read -r url; do
  name=$(bundle_name "$prefix" "$url") || die "URL refused: '$url'"
  names+=("$name")
done < <(jq -r '[.platforms[].url // ""] | unique[]' "$dir/latest.json")
[ "${#names[@]}" -gt 0 ] || die "latest.json lists no bundle"
for name in "${names[@]}"; do
  gh release download "$tag" --repo "$repo" -p "$name" -D "$dir/bundles" --clobber
done
