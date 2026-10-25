#!/usr/bin/env bash
# Rust code metrics (rust-code-analysis): fails when a file exceeds 1000 physical lines (AGENTS.md
# invariant; loc.sloc, blank and comment lines included, not ploc), and reports without failing
# the functions whose cognitive complexity exceeds clippy.toml's `cognitive-complexity-threshold`.
# Writes the Markdown report to stdout and the raw JSON metrics to <dir>.
# Usage: code-metrics.sh <dir>
set -euo pipefail

out=${1:?usage: code-metrics.sh <dir>}
max_lines=1000
threshold=$(awk -F' *= *' '$1 == "cognitive-complexity-threshold" { print $2 }' clippy.toml)
[ -n "$threshold" ] || { echo "cognitive-complexity-threshold absent de clippy.toml" >&2; exit 1; }

mkdir -p "$out"
rust-code-analysis-cli --metrics --output-format json --paths crates --paths src-tauri/src \
  --include '*.rs' --output "$out"

# An empty output dir (paths changed, silent tool) would pass for flawless code.
files=$(find "$out" -name '*.rs.json' | wc -l | tr -d ' ')
expected=$(git ls-files 'crates/*.rs' 'src-tauri/src/*.rs' | wc -l | tr -d ' ')
[ "$files" -eq "$expected" ] || { echo "$files fichiers analysés sur $expected" >&2; exit 1; }

long=$(find "$out" -name '*.rs.json' -print0 | xargs -0 jq -r --argjson max "$max_lines" '
  select(.metrics.loc.sloc > $max) | "| `\(.name)` | \(.metrics.loc.sloc | floor) |"' | sort)
complex=$(find "$out" -name '*.rs.json' -print0 | xargs -0 jq -r --argjson t "$threshold" '
  .name as $f | .. | objects | select(.kind == "function" and .metrics.cognitive.sum > $t)
  | "\(.metrics.cognitive.sum | floor)\t| `\($f):\(.start_line)` | `\(.name)` | \(.metrics.cognitive.sum | floor) |"' \
  | sort -rn | cut -f2-)

echo "## Métriques du code Rust"
echo
echo "$files fichiers analysés."
echo
if [ -n "$long" ]; then
  echo "### Fichiers de plus de $max_lines lignes (bloquant)"
  echo
  echo "| Fichier | Lignes |"
  echo "|---|---|"
  echo "$long"
else
  echo "Aucun fichier de plus de $max_lines lignes."
fi
echo
if [ -n "$complex" ]; then
  echo "### Complexité cognitive au-dessus de $threshold (signalée)"
  echo
  echo "| Emplacement | Fonction | Complexité |"
  echo "|---|---|---|"
  echo "$complex"
else
  echo "Aucune fonction au-dessus de $threshold en complexité cognitive."
fi

[ -z "$long" ]
