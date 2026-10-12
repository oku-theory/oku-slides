#!/usr/bin/env bash
# Lance Oku Slides (serveur web + UI construite) avec des données isolées et le faux agent ACP.
# Usage : serve.sh [port] [scénario du faux agent] [deck]
# Depuis la racine du dépôt ; les binaires (target/debug) et ui/dist doivent être construits.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
port="${1:-4991}"
script="${2:-$here/agent-details.json}"
deck="${3:-fixtures/roadmap.json}"
if curl -s -o /dev/null "http://127.0.0.1:$port/"; then echo "serve : port $port déjà pris" >&2; exit 2; fi
tmp="$(mktemp -d "${TMPDIR:-/tmp}/oku-slides-run.XXXXXX")"
# Jamais les données de l'utilisateur : réglages, onglets et conversations vont dans $tmp.
export OKU_SLIDES_PATHS__DATA="$tmp/data" OKU_SLIDES_PATHS__DECKS="$tmp/decks"
mkdir -p "$tmp/data"
bin="$(cd "${CARGO_TARGET_DIR:-target}" && pwd)/debug"
printf '[agent]\nselected = "faux"\n\n[agent.presets.faux]\nname = "Faux"\ncommand = "%s"\nargs = ["--script", "%s"]\n' \
  "$bin/oku-slides-fake-agent" "$(cd "$(dirname "$script")" && pwd)/$(basename "$script")" > "$tmp/data/config.toml"
echo "serve : données dans $tmp, http://127.0.0.1:$port/"
exec "$bin/oku-slides" --fonts assets/fonts serve "$deck" --ui ui/dist --port "$port"
