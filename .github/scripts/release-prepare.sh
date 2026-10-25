#!/usr/bin/env bash
# Prepares the release PR before release-plz: git-cliff (cliff.toml) computes the next version
# and changelog over every commit since the current version's tag, ui/ included. Writes the
# version to Cargo.toml's [workspace.package] (the app version's only source) and adds its
# section at the top of CHANGELOG.md, without committing: release-plz (allow_dirty) takes these
# files, sees a local version above the last tag and opens the PR with it.
# Prints the new version, or nothing when there is nothing to release. Usage: release-prepare.sh
set -euo pipefail

current=$(awk '/^\[workspace.package\]/{f=1} f && /^version = /{gsub(/"/,"",$3); print $3; exit}' Cargo.toml)
[ -n "$current" ] || { echo "version absente de [workspace.package]" >&2; exit 1; }
# main's version is always released: without its tag, the release job failed or was skipped.
git rev-parse -q --verify "refs/tags/v$current" >/dev/null \
  || { echo "tag v$current absent : la version courante n'est pas publiée" >&2; exit 1; }

if [ -z "$(git log --format=%H "v$current..HEAD" | head -1)" ]; then
  exit 0
fi

# Only commits that cliff.toml skips (chore(release)...): git-cliff returns the current version.
next=$(git cliff --bumped-version)
next=${next#v}
[[ "$next" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "version suivante illisible : $next" >&2; exit 1; }
[ "$next" != "$current" ] || exit 0

tmp=$(mktemp)
awk -v old="$current" -v new="$next" '
  /^\[workspace.package\]/ { f = 1 }
  f && $0 == "version = \"" old "\"" { $0 = "version = \"" new "\""; f = 0 }
  { print }' Cargo.toml >"$tmp" && mv "$tmp" Cargo.toml
grep -qx "version = \"$next\"" Cargo.toml || { echo "Cargo.toml non mis à jour" >&2; exit 1; }
# Workspace crates at the new version in Cargo.lock, other pins unchanged: when no crate changed
# since the tag, release-plz looks the app up there and fails otherwise (v5.1.1, t-1209).
cargo metadata --format-version 1 >/dev/null
grep -qx "version = \"$next\"" Cargo.lock || { echo "Cargo.lock non mis à jour" >&2; exit 1; }

if [ -f CHANGELOG.md ]; then
  # `--prepend` would write the section above the header: it goes before the first version.
  section=$(mktemp)
  git cliff --unreleased --tag "v$next" --strip header --output "$section"
  awk -v section="$section" '
    !done && /^## \[/ { while ((getline line < section) > 0) print line; print ""; done = 1 }
    { print }
    END { if (!done) { print ""; while ((getline line < section) > 0) print line } }' \
    CHANGELOG.md >"$tmp" && mv "$tmp" CHANGELOG.md
else
  git cliff --unreleased --tag "v$next" --output CHANGELOG.md
fi
grep -q "^## \[$next\]" CHANGELOG.md || { echo "CHANGELOG.md sans section $next" >&2; exit 1; }

echo "$next"
