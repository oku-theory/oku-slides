#!/usr/bin/env bash
# Monte la version de l'app (patch, minor ou major) partout où elle est écrite : Cargo.toml
# (workspace), Cargo.lock (crates du workspace), src-tauri/tauri.conf.json et ui/package.json.
# Affiche la nouvelle version. Usage : bump-version.sh patch|minor|major
set -euo pipefail

part=${1:?usage: bump-version.sh patch|minor|major}
old=$(jq -r .version src-tauri/tauri.conf.json)
IFS=. read -r major minor patch <<<"$old"
case "$part" in
  patch) patch=$((patch + 1)) ;;
  minor) minor=$((minor + 1)); patch=0 ;;
  major) major=$((major + 1)); minor=0; patch=0 ;;
  *) echo "partie inconnue : $part" >&2; exit 1 ;;
esac
new="$major.$minor.$patch"

# Les trois sources doivent être d'accord avant le bump : sinon l'une a dérivé à la main.
for v in "$(jq -r .version ui/package.json)" \
         "$(awk '/^\[workspace.package\]/{f=1} f && /^version = /{gsub(/"/,"",$3); print $3; exit}' Cargo.toml)"; do
  [ "$v" = "$old" ] || { echo "versions divergentes : $v ≠ $old (tauri.conf.json)" >&2; exit 1; }
done

tmp=$(mktemp)
jq --arg v "$new" '.version = $v' src-tauri/tauri.conf.json >"$tmp" && mv "$tmp" src-tauri/tauri.conf.json
jq --arg v "$new" '.version = $v' ui/package.json >"$tmp" && mv "$tmp" ui/package.json
awk -v old="$old" -v new="$new" '
  /^\[workspace.package\]/ { f = 1 }
  f && $0 == "version = \"" old "\"" { $0 = "version = \"" new "\""; f = 0 }
  { print }' Cargo.toml >"$tmp" && mv "$tmp" Cargo.toml
# Dans le lock, seules les entrées des crates du workspace (oku-slides*) changent de version.
awk -v old="$old" -v new="$new" '
  /^name = "oku-slides/ { f = 1; print; next }
  f && $0 == "version = \"" old "\"" { $0 = "version = \"" new "\"" }
  { f = 0; print }' Cargo.lock >"$tmp" && mv "$tmp" Cargo.lock

# Un motif awk qui ne trouve rien laisserait l'ancienne version sans erreur : chaque fichier est
# relu, et toutes les crates du lock doivent porter la nouvelle version.
grep -qx "version = \"$new\"" Cargo.toml || { echo "Cargo.toml non mis à jour" >&2; exit 1; }
stale=$(awk -v new="$new" '
  /^name = "oku-slides/ { crate = $3; next }
  crate != "" { if ($0 != "version = \"" new "\"") print crate; crate = "" }' Cargo.lock)
[ -z "$stale" ] || { echo "Cargo.lock : crates restées à l'ancienne version : $stale" >&2; exit 1; }

echo "$new"
