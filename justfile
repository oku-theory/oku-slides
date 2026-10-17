# Oku Slides — toutes les commandes se lancent depuis la racine du dépôt.
# `just` sans argument affiche la liste.

set shell := ["bash", "-euo", "pipefail", "-c"]

fonts := "assets/fonts"
deck := "fixtures/roadmap.json"

# Affiche les commandes disponibles
default:
    @just --list --unsorted

# Installe les dépendances de l'interface (Bun)
install:
    cd ui && bun install

# Lance l'app de bureau en développement (Vite + Tauri), relancée à chaque modification :
# interface (HMR), crates Rust et bibliothèque Typst embarquée
dev: install
    cargo tauri dev --additional-watch-folders "{{justfile_directory()}}/typst/oku-slides"

# Construit l'app (.app / .dmg sur macOS)
build: install
    cargo tauri build

# Interface dans le navigateur, sans Tauri : http://127.0.0.1:4870
serve file=deck: install
    cd ui && bun run build
    cargo run -q -p oku-slides-cli -- --fonts {{fonts}} serve "{{file}}"

# Interface avec rechargement à chaud (http://localhost:5173) et API Rust
web file=deck: install
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build -q -p oku-slides-cli
    ./target/debug/oku-slides --fonts {{fonts}} serve "{{file}}" --ui ui/dist &
    trap "kill $! 2>/dev/null" EXIT
    cd ui && bun run dev

# Tests Rust (hors app Tauri) et tests de l'interface
test:
    cargo nextest run --workspace --exclude oku-slides-app
    # Tests de rendu (DOM happy-dom) dans leur propre processus : cache de modules à part.
    cd ui && bun run test

# Tests de bout en bout (Playwright, sous Bun) contre un serveur neuf, sur le port
# `port` (4872 par défaut ; `just e2e 4999` ou `just e2e port=4999` quand une autre
# session tient déjà le port). Données et decks dans un dossier temporaire, supprimé
# à la fin ; les binaires viennent de `$CARGO_TARGET_DIR` s'il est défini.
e2e port="4872": install
    #!/usr/bin/env bash
    set -euo pipefail
    port="{{port}}"
    port="${port#port=}"
    [[ "$port" =~ ^[0-9]+$ ]] || { echo "e2e : port invalide « $port »" >&2; exit 2; }
    (cd ui && bun run build && bunx playwright install chromium)
    cargo build -q -p oku-slides-cli -p oku-slides-agent --bin oku-slides --bin oku-slides-fake-agent
    bin="$(cd "${CARGO_TARGET_DIR:-target}" && pwd)/debug"
    # Un serveur d'une autre session sur ce port fausserait tout : refusé.
    if curl -s -o /dev/null "http://127.0.0.1:$port/"; then echo "e2e : port $port déjà pris" >&2; exit 2; fi
    tmp="$(mktemp -d "${TMPDIR:-/tmp}/oku-slides-e2e.XXXXXX")"
    export OKU_SLIDES_PATHS__DATA="$tmp/data" OKU_SLIDES_PATHS__DECKS="$tmp/decks"
    mkdir -p "$tmp/data"
    # Le journal du faux agent (`--log`) : les specs y lisent le résultat des appels MCP (220-import).
    log="$tmp/fake-agent.log"
    printf '[agent]\nselected = "e2e"\n\n[agent.presets.e2e]\nname = "Faux"\ncommand = "%s"\nargs = ["--script", "%s", "--log", "%s"]\n\n[agent.presets.e2e-bis]\nname = "Faux bis"\ncommand = "%s"\nargs = ["--script", "%s"]\n' \
      "$bin/oku-slides-fake-agent" "$PWD/fixtures/agent/e2e.json" "$log" "$bin/oku-slides-fake-agent" "$PWD/fixtures/agent/e2e-bis.json" > "$tmp/data/config.toml"
    "$bin/oku-slides" --fonts {{fonts}} serve "{{deck}}" --ui ui/dist --port "$port" &
    server=$!
    trap 'kill "$server" 2>/dev/null || true; wait "$server" 2>/dev/null || true; rm -rf "$tmp"' EXIT
    for _ in $(seq 1 50); do curl -sf -o /dev/null "http://127.0.0.1:$port/" && break; sleep 0.2; done
    cd ui && E2E_URL="http://127.0.0.1:$port/" E2E_AGENT_LOG="$log" bun run e2e

# Vérifications : format, clippy, types TypeScript, polices en LFS
check: install fonts-lfs agents-lint
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cd ui && bun run typecheck

# Polices de l'index : des pointeurs Git LFS, jamais le binaire (ajout sans git-lfs)
fonts-lfs:
    #!/usr/bin/env bash
    bad=0
    while IFS= read -r f; do
      head=$(git cat-file -p ":$f" | head -c 43 | tr -d '\0')
      if [ "$head" != "version https://git-lfs.github.com/spec/v1" ]; then
        echo "✗ $f : binaire dans l'index au lieu d'un pointeur LFS (git lfs install && git add --renormalize $f)"
        bad=1
      fi
    done < <(git ls-files -- '{{fonts}}/*.ttf' '{{fonts}}/*.otf')
    exit $bad

# Tests de agents-lint : un arbre valide, puis un arbre par cas ; sortie comparée à l'identique
agents-lint-test:
    #!/usr/bin/env bash
    set -euo pipefail
    tmp="$(mktemp -d "${TMPDIR:-/tmp}/agents-lint.XXXXXX")"
    trap 'rm -rf "$tmp"' EXIT
    jf={{quote(justfile())}}
    valid() {
      local d="$tmp/$1"
      mkdir -p "$d/docs/adr" "$d/docs/superpowers/specs" "$d/docs/superpowers/plans" "$d/crates/a"
      git -C "$d" init -q
      echo "# ADR" > "$d/docs/adr/0001-x.md"
      echo "# spec" > "$d/docs/superpowers/specs/2026-01-01-x-design.md"
      echo "# plan" > "$d/docs/superpowers/plans/2026-01-01-x.md"
      printf '# Root\n\nSee ADR-0001.\n\n## Invariants\n\n- root rule (t-2)\n\n## Harness map\n\n- `crates/a/AGENTS.md` — when touching a\n' > "$d/AGENTS.md"
      printf '# a — agent rules\n\n## Invariants\n\n- keep x (ADR-0001, t-1)\n- a wrapped rule that\n  continues here (spec:2026-01-01-x-design)\n\n## Testing\n\n- run y (plan:2026-01-01-x)\n' > "$d/crates/a/AGENTS.md"
    }
    pad() { # pad <fichier> <lignes totales> : complète par des règles sourcées
      local f="$1" want="$2" n; n=$(awk 'END { print NR }' "$f")
      while [ "$n" -lt "$want" ]; do echo "- r (t-1)" >> "$f"; n=$((n + 1)); done
    }
    pass=0; failed=0
    expect() { # expect <nom> <sortie exacte attendue>
      local name="$1" want="$2" out code=0
      out="$(just -f "$jf" agents-lint "$tmp/$name" 2>&1)" || code=$?
      if [ "$want" = "✓ harnais AGENTS.md" ] && [ "$code" = 0 ] && [ "$out" = "$want" ]; then pass=$((pass + 1))
      elif [ "$want" != "✓ harnais AGENTS.md" ] && [ "$code" = 1 ] && [ "$(grep -v '^error: recipe' <<< "$out")" = "$want" ]; then pass=$((pass + 1))
      else failed=$((failed + 1)); printf '✗ %s : attendu « %s », obtenu (%s) :\n%s\n' "$name" "$want" "$code" "$out"; fi
    }
    ok="✓ harnais AGENTS.md"
    A="crates/a/AGENTS.md"
    valid valid; expect valid "$ok"
    mkdir -p "$tmp/empty"; git -C "$tmp/empty" init -q; expect empty "$ok"
    valid claude-root; printf '@AGENTS.md\n' > "$tmp/claude-root/CLAUDE.md"; expect claude-root "✗ CLAUDE.md : désactive la lecture native des AGENTS.md par Claude Code (à supprimer)"
    valid claude-nested; echo "x" > "$tmp/claude-nested/crates/a/CLAUDE.md"; expect claude-nested "✗ crates/a/CLAUDE.md : désactive la lecture native des AGENTS.md par Claude Code (à supprimer)"
    valid claude-local; echo "CLAUDE.local.md" > "$tmp/claude-local/.gitignore"; echo "x" > "$tmp/claude-local/CLAUDE.local.md"; expect claude-local "✗ CLAUDE.local.md : désactive la lecture native des AGENTS.md par Claude Code (à supprimer)"
    valid dot-claude; mkdir -p "$tmp/dot-claude/.claude"; echo "x" > "$tmp/dot-claude/.claude/CLAUDE.md"; expect dot-claude "✗ .claude/CLAUDE.md : désactive la lecture native des AGENTS.md par Claude Code (à supprimer)"
    valid bad-adr; echo "- z (ADR-0099)" >> "$tmp/bad-adr/$A"; expect bad-adr "✗ $A : référence introuvable : ADR-0099"
    valid bad-spec; echo "- z (spec:nope)" >> "$tmp/bad-spec/$A"; expect bad-spec "✗ $A : référence introuvable : spec:nope"
    valid bad-plan; echo "- z (plan:nope)" >> "$tmp/bad-plan/$A"; expect bad-plan "✗ $A : référence introuvable : plan:nope"
    valid root-bad-ref; echo "Also ADR-0042." >> "$tmp/root-bad-ref/AGENTS.md"; expect root-bad-ref "✗ AGENTS.md : référence introuvable : ADR-0042"
    valid word-boundary; echo "- z subplan:nope (t-3)" >> "$tmp/word-boundary/$A"; expect word-boundary "$ok"
    valid no-source; echo "- a rule without source" >> "$tmp/no-source/$A"; expect no-source "✗ $A : règle sans source : - a rule without source"
    valid pitfall; printf '\n## Pitfalls\n\n- p without source\n' >> "$tmp/pitfall/$A"; expect pitfall "✗ $A : règle sans source : - p without source"
    valid testing; printf '\n## Testing\n\n- t without source\n' >> "$tmp/testing/$A"; expect testing "✗ $A : règle sans source : - t without source"
    valid decision; printf '\n## Settled decisions\n\n- d without source\n' >> "$tmp/decision/$A"; expect decision "✗ $A : règle sans source : - d without source"
    valid fake-source; echo "- z (offset-0)" >> "$tmp/fake-source/$A"; expect fake-source "✗ $A : règle sans source : - z (offset-0)"
    valid source-not-last; printf -- '- z (t-4)\n  because reasons\n' >> "$tmp/source-not-last/$A"; expect source-not-last "✗ $A : règle sans source : - z (t-4)   because reasons"
    valid free-section; printf '\n## Notes\n\n- free text\n' >> "$tmp/free-section/$A"; expect free-section "$ok"
    valid fenced; printf '\n## Testing\n\n```\n- not a rule\n```\n' >> "$tmp/fenced/$A"; expect fenced "$ok"
    valid root-no-source; printf '\n## Pitfalls\n\n- root rule\n' >> "$tmp/root-no-source/AGENTS.md"; expect root-no-source "✗ AGENTS.md : règle sans source : - root rule"
    valid budget-60; pad "$tmp/budget-60/$A" 60; expect budget-60 "$ok"
    valid budget-61; pad "$tmp/budget-61/$A" 60; printf -- '- r (t-1)' >> "$tmp/budget-61/$A"; expect budget-61 "✗ $A : 61 lignes > 60"
    valid root-130; pad "$tmp/root-130/AGENTS.md" 130; expect root-130 "$ok"
    valid root-131; pad "$tmp/root-131/AGENTS.md" 131; expect root-131 "✗ AGENTS.md : 131 lignes > 130"
    valid unmapped; mkdir -p "$tmp/unmapped/crates/c"; echo "# c" > "$tmp/unmapped/crates/c/AGENTS.md"; expect unmapped "✗ AGENTS.md : absent de la carte : crates/c/AGENTS.md"
    valid dead-map; echo '- `crates/z/AGENTS.md` — gone' >> "$tmp/dead-map/AGENTS.md"; expect dead-map "✗ AGENTS.md : entrée de carte introuvable : crates/z/AGENTS.md"
    valid escape-map; echo '- `../x/AGENTS.md` — outside' >> "$tmp/escape-map/AGENTS.md"; expect escape-map "✗ AGENTS.md : entrée de carte invalide : ../x/AGENTS.md"
    valid map-section-end; printf '\n## Other\n\n- see `crates/z/AGENTS.md` elsewhere\n' >> "$tmp/map-section-end/AGENTS.md"; expect map-section-end "$ok"
    valid noroot; rm "$tmp/noroot/AGENTS.md"; expect noroot "✗ AGENTS.md : AGENTS.md racine manquant"
    valid pruned; for p in node_modules target .worktrees .claude .git; do mkdir -p "$tmp/pruned/$p/x"; echo "# p" > "$tmp/pruned/$p/x/AGENTS.md"; echo "stray" > "$tmp/pruned/$p/x/CLAUDE.md"; echo "stray" > "$tmp/pruned/$p/x/CLAUDE.local.md"; done; expect pruned "$ok"
    valid see-adr; echo "- z (see ADR-0001)" >> "$tmp/see-adr/$A"; expect see-adr "$ok"
    valid sub-adr; echo "- z subADR-0099 (t-3)" >> "$tmp/sub-adr/$A"; expect sub-adr "$ok"
    valid adr-5; echo "- z (ADR-00012)" >> "$tmp/adr-5/$A"; expect adr-5 "✗ $A : référence invalide : ADR-00012"$'\n'"✗ $A : règle sans source : - z (ADR-00012)"
    valid dup-ref; printf -- '- z (ADR-0099)\n- y (ADR-0099)\n' >> "$tmp/dup-ref/$A"; expect dup-ref "✗ $A : référence introuvable : ADR-0099"
    valid long-rule; echo "- this unsourced rule is deliberately longer than sixty characters in total" >> "$tmp/long-rule/$A"; expect long-rule "✗ $A : règle sans source : - this unsourced rule is deliberately longer than sixty char"
    valid blank-ends; printf -- '- z\n\n  (t-1)\n' >> "$tmp/blank-ends/$A"; expect blank-ends "✗ $A : règle sans source : - z"
    valid abs-map; echo '- `/x/AGENTS.md` — absolute' >> "$tmp/abs-map/AGENTS.md"; expect abs-map "✗ AGENTS.md : entrée de carte invalide : /x/AGENTS.md"
    valid mid-dotdot; echo '- `crates/../../x/AGENTS.md` — escape' >> "$tmp/mid-dotdot/AGENTS.md"; expect mid-dotdot "✗ AGENTS.md : entrée de carte invalide : crates/../../x/AGENTS.md"
    valid nomap; printf '# Root\n\nSee ADR-0001.\n' > "$tmp/nomap/AGENTS.md"; expect nomap "✗ AGENTS.md : absent de la carte : crates/a/AGENTS.md"
    valid ignored; echo "dist/" > "$tmp/ignored/.gitignore"; mkdir -p "$tmp/ignored/dist"; echo "stray" > "$tmp/ignored/dist/CLAUDE.md"; expect ignored "$ok"
    valid adj-adr; echo "- z (ADR-0001 ADR-0099)" >> "$tmp/adj-adr/$A"; expect adj-adr "✗ $A : référence introuvable : ADR-0099"
    valid slash-adr; echo "- z (ADR-0001/ADR-0099)" >> "$tmp/slash-adr/$A"; expect slash-adr "✗ $A : référence introuvable : ADR-0099"
    valid ticket-suffix; echo "- z (t-1x)" >> "$tmp/ticket-suffix/$A"; expect ticket-suffix "✗ $A : règle sans source : - z (t-1x)"
    valid see-ticket-suffix; echo "- z (see t-12abc)" >> "$tmp/see-ticket-suffix/$A"; expect see-ticket-suffix "✗ $A : règle sans source : - z (see t-12abc)"
    valid nested-dot-claude; mkdir -p "$tmp/nested-dot-claude/crates/a/.claude"; echo "x" > "$tmp/nested-dot-claude/crates/a/.claude/CLAUDE.md"; expect nested-dot-claude "✗ crates/a/.claude/CLAUDE.md : désactive la lecture native des AGENTS.md par Claude Code (à supprimer)"
    valid nested-dot-claude-ignored; echo ".claude/" > "$tmp/nested-dot-claude-ignored/.gitignore"; mkdir -p "$tmp/nested-dot-claude-ignored/crates/a/.claude"; echo "x" > "$tmp/nested-dot-claude-ignored/crates/a/.claude/CLAUDE.md"; expect nested-dot-claude-ignored "✗ crates/a/.claude/CLAUDE.md : désactive la lecture native des AGENTS.md par Claude Code (à supprimer)"
    valid claude-worktree; echo ".claude/worktrees/" > "$tmp/claude-worktree/.gitignore"; mkdir -p "$tmp/claude-worktree/.claude/worktrees/w/crates/a/.claude"; echo "x" > "$tmp/claude-worktree/.claude/worktrees/w/crates/a/.claude/CLAUDE.md"; echo "x" > "$tmp/claude-worktree/.claude/worktrees/w/CLAUDE.md"; expect claude-worktree "$ok"
    valid non-ascii; mkdir -p "$tmp/non-ascii/crates/é"; printf '# e\n\n## Invariants\n\n- no source here\n' > "$tmp/non-ascii/crates/é/AGENTS.md"; echo '- `crates/é/AGENTS.md` — when touching e' >> "$tmp/non-ascii/AGENTS.md"; expect non-ascii "✗ crates/é/AGENTS.md : règle sans source : - no source here"
    echo "agents-lint-test : $pass ok, $failed en échec"
    [ "$failed" = 0 ]

# Revue du harnais : ADR, specs, plans et tickets livrés depuis la dernière revue
# (marque « Last review: <commit> » de docs/harness-review.md)
agents-review:
    #!/usr/bin/env bash
    set -euo pipefail
    since=$(sed -n 's/^Last review: \([0-9a-f]\{7,40\}\).*/\1/p' docs/harness-review.md | head -1)
    [ -n "$since" ] || { echo "agents-review : pas de « Last review: <commit> » dans docs/harness-review.md" >&2; exit 2; }
    git cat-file -e "$since^{commit}" 2>/dev/null || { echo "agents-review : commit $since inconnu" >&2; exit 2; }
    echo "Depuis $since ($(git log -1 --format=%ad --date=short "$since")) :"
    echo "— Documents (A ajouté, M modifié, D supprimé, R renommé) :"
    docs=$(git diff --name-status --diff-filter=AMDR "$since" HEAD -- docs/adr docs/superpowers)
    echo "${docs:-(aucun)}" | sed 's/^/  /'
    echo "— Tickets cités dans les messages de commit :"
    tickets=$(git log --format=%B "$since..HEAD" | { grep -o -E '\bt-[0-9]+\b' || true; } | sort -t- -k2 -n -u | tr '\n' ' ')
    echo "  ${tickets:-(aucun)}"

# Harnais AGENTS.md : aucun CLAUDE.md, références, sources, budgets, carte racine
agents-lint root=".":
    #!/usr/bin/env bash
    set -uo pipefail
    root={{quote(root)}}
    root="${root#root=}"
    cd "$root" || { echo "agents-lint : dossier introuvable « $root »" >&2; exit 2; }
    git rev-parse --is-inside-work-tree > /dev/null 2>&1 || { echo "agents-lint : « $root » n'est pas un dépôt git" >&2; exit 2; }
    bad=0
    fail() { echo "✗ $1 : $2"; bad=1; }
    agents=()
    # Chemins bruts (non ASCII compris) ; élagage : node_modules, target, .worktrees, et l'arbre .claude/ racine.
    while IFS= read -r f; do
      [ -f "$f" ] || continue
      case "$(basename "$f")" in
        AGENTS.md) agents+=("$f") ;;
        *) fail "$f" "désactive la lecture native des AGENTS.md par Claude Code (à supprimer)" ;;
      esac
    done < <({ git -c core.quotePath=false ls-files --cached --others --exclude-standard -- '*AGENTS.md' '*CLAUDE.md' '*CLAUDE.local.md' \
                 | grep -E '(^|/)(AGENTS|CLAUDE|CLAUDE\.local)\.md$' \
                 | grep -vE '(^|/)(node_modules|target|\.worktrees)/|^\.claude/'
               # Ignorés par git mais lus par Claude Code : CLAUDE.local.md, .claude/CLAUDE.md (à toute profondeur).
               git -c core.quotePath=false ls-files --others --ignored --exclude-standard -- '*CLAUDE.local.md' '*/.claude/CLAUDE.md' \
                 | grep -vE '(^|/)(node_modules|target|\.worktrees)/|^\.claude/'
               [ -f .claude/CLAUDE.md ] && echo .claude/CLAUDE.md
             } | sort -u)
    for f in ${agents[@]+"${agents[@]}"}; do
      # Références : ADR-NNNN, spec:<nom>, plan:<nom> (mot entier) doivent désigner un fichier.
      # ADR : un mot par ligne (coupé sur tout caractère hors [[:alnum:]_-]), donc références collées comprises.
      adrs="$(tr -c '[:alnum:]_-' '\n' < "$f" | grep -oE '^ADR-[0-9]+' || true)"
      while IFS= read -r ref; do
        compgen -G "docs/adr/${ref#ADR-}-*.md" > /dev/null || fail "$f" "référence introuvable : $ref"
      done < <(grep -xE 'ADR-[0-9]{4}' <<< "$adrs" | sort -u)
      while IFS= read -r ref; do
        fail "$f" "référence invalide : $ref"
      done < <(grep -xE 'ADR-[0-9]{5,}' <<< "$adrs" | sort -u)
      while IFS= read -r ref; do
        kind="${ref%%:*}"; name="${ref#*:}"
        [ -f "docs/superpowers/${kind}s/$name.md" ] || fail "$f" "référence introuvable : $ref"
      done < <(grep -oE '(^|[^[:alnum:]_-])(spec|plan):[[:alnum:]_-]+' "$f" | sed -E 's/^[^sp]//' | sort -u)
      # Budget : 130 lignes à la racine, 60 ailleurs (dernière ligne comptée même sans saut final).
      max=60; [ "$f" = AGENTS.md ] && max=130
      n=$(awk 'END { print NR }' "$f")
      [ "$n" -le "$max" ] || fail "$f" "$n lignes > $max"
      # Règles : chaque puce des quatre sections finit par sa source, mot entier (hors blocs de code).
      awk -v f="$f" '
        function check() {
          if (inb && b !~ /\(([^()]*[^[:alnum:]_-])?(ADR-[0-9][0-9][0-9][0-9]|t-[0-9]+|spec:[[:alnum:]_-]+|plan:[[:alnum:]_-]+)([,[:space:]][^()]*)?\)[[:space:]]*$/) {
            printf "✗ %s : règle sans source : %s\n", f, substr(b, 1, 60); bad = 1
          }
          inb = 0; b = ""
        }
        /^[[:space:]]*```/ { check(); fence = !fence; next }
        fence { next }
        /^## / { check(); sec = ($0 ~ /^## (Invariants|Pitfalls|Testing|Settled decisions)[[:space:]]*$/); next }
        /^- / { check(); if (sec) { inb = 1; b = $0 }; next }
        /^[[:space:]]+[^[:space:]]/ { if (inb) b = b " " $0; next }
        { check() }
        END { check(); exit bad }' "$f" || bad=1
    done
    # Carte racine : chaque AGENTS.md imbriqué y figure, chaque entrée existe dans le dépôt.
    nested=()
    for f in ${agents[@]+"${agents[@]}"}; do [ "$f" = AGENTS.md ] || nested+=("$f"); done
    if [ "${#nested[@]}" -gt 0 ]; then
      if [ ! -f AGENTS.md ]; then
        fail AGENTS.md "AGENTS.md racine manquant"
      else
        map="$(awk '/^## Harness map[[:space:]]*$/ { m = 1; next } /^## / { m = 0 } m' AGENTS.md | grep -oE '`[^`]+/AGENTS\.md`' | tr -d '`' || true)"
        for f in "${nested[@]}"; do grep -qxF "$f" <<< "$map" || fail AGENTS.md "absent de la carte : $f"; done
        while IFS= read -r e; do
          [ -z "$e" ] && continue
          case "$e" in /*|../*|*/../*) fail AGENTS.md "entrée de carte invalide : $e"; continue ;; esac
          [ -f "$e" ] || fail AGENTS.md "entrée de carte introuvable : $e"
        done <<< "$map"
      fi
    fi
    [ "$bad" = 0 ] && echo "✓ harnais AGENTS.md"
    exit "$bad"

# Formate le code Rust
fmt:
    cargo fmt --all

# Corrige automatiquement ce que clippy sait corriger
lint-fix:
    cargo clippy --workspace --all-targets --fix --allow-dirty --allow-staged

# Couverture des tests (lignes), crate par crate
coverage:
    cargo llvm-cov nextest --workspace --exclude oku-slides-app --summary-only

# Dépendances : licences, avis de sécurité, sources. `bun audit` lit ui/bun.lock seul ;
# une exception npm motivée s'y ajoute en `--ignore <GHSA>`, commentée ici.
audit:
    cargo deny check
    cargo audit
    cd ui && bun audit

# Relance une commande à chaque modification : just watch test
watch cmd="test":
    cargo watch -s "just {{cmd}}"

# Tout ce que vérifie l'intégration continue, en local
ci: check agents-lint-test test audit

# Installe les outils de développement
setup:
    cargo install --locked cargo-nextest cargo-deny cargo-audit cargo-llvm-cov cargo-watch tauri-cli

# Outil en ligne de commande : just oku-slides render fixtures/roadmap.json --out out
oku-slides *args:
    cargo run -q -p oku-slides-cli -- --fonts {{fonts}} {{args}}

# Latences de rendu et d'enregistrement (build optimisé)
bench file=deck:
    cargo run -q --release -p oku-slides-cli -- --fonts {{fonts}} bench "{{file}}"

# Enregistre un deck JSON en .oku-slides.pdf
save file=deck out="deck.oku-slides.pdf":
    cargo run -q -p oku-slides-cli -- --fonts {{fonts}} save "{{file}}" "{{out}}"

# Compile un exemple de la bibliothèque Typst : just typst pro
typst example="demo":
    typst compile --root typst/oku-slides --font-path {{fonts}} typst/oku-slides/examples/{{example}}.typ

# Génère toutes les icônes de l'app depuis src-tauri/icons/icon.png
icons:
    cargo tauri icon src-tauri/icons/icon.png

# Vérifie que les outils nécessaires sont installés
doctor:
    #!/usr/bin/env bash
    ok=1
    for tool in cargo bun cmake; do
      if command -v "$tool" >/dev/null; then echo "✓ $tool"; else echo "✗ $tool manquant"; ok=0; fi
    done
    if cargo tauri --version >/dev/null 2>&1; then echo "✓ cargo tauri"; else echo "✗ cargo tauri manquant : cargo install tauri-cli --version '^2'"; ok=0; fi
    command -v cmake >/dev/null || echo "  cmake sert à compiler CTranslate2 (dictée) : brew install cmake"
    # Le pont Swift de la dictée d'Apple exige le SDK de macOS 26 ou plus.
    if [ "$(uname)" = Darwin ]; then
      sdk="$(xcrun --show-sdk-version 2>/dev/null)"
      if [ -n "$sdk" ] && [ "${sdk%%.*}" -ge 26 ] 2>/dev/null; then echo "✓ Xcode, SDK macOS $sdk"; else echo "✗ Xcode avec le SDK macOS 26+ manquant (SDK : ${sdk:-aucun}) : installez Xcode 26 ou plus"; ok=0; fi
    fi
    if command -v typst >/dev/null; then echo "✓ typst (optionnel)"; else echo "· typst absent (optionnel, pour just typst)"; fi
    if command -v git-lfs >/dev/null; then echo "✓ git-lfs"; else echo "✗ git-lfs manquant (polices de assets/fonts) : brew install git-lfs && git lfs install"; ok=0; fi
    # Installé dans CE clone : le filtre (checkout) et le hook pre-push (envoi des polices).
    if [ "$(git config --get filter.lfs.process)" = "git-lfs filter-process" ]; then echo "✓ filtre LFS"; else echo "✗ filtre LFS absent : git lfs install"; ok=0; fi
    hooks="$(git rev-parse --git-dir)/hooks"; global="$(git config --get core.hooksPath)"
    if grep -qs 'git lfs pre-push' "$hooks/pre-push" || { [ -n "$global" ] && grep -qs 'git lfs pre-push' "$global/pre-push"; }; then echo "✓ hook pre-push LFS"; else echo "✗ hook pre-push LFS absent (les polices ne seraient pas envoyées) : git lfs install"; ok=0; fi
    # Sans Git LFS au clonage, les polices ne sont que des pointeurs texte.
    if grep -lq "^version https://git-lfs" {{fonts}}/*.ttf {{fonts}}/*.otf 2>/dev/null; then echo "✗ polices non récupérées (pointeurs LFS) : git lfs pull"; ok=0; else echo "✓ polices"; fi
    [ "$ok" = 1 ]

# Accélération de la dictée selon la machine (Accelerate sur Mac)
stt_features := if os() == "macos" { "accelerate" } else { "" }

# Transcrit un fichier WAV avec le modèle de dictée (turbo, large, base ou apple ; langue facultative)
transcribe wav model="turbo" lang="":
    cargo run -q --release -p oku-slides-stt --features "{{stt_features}}" --example transcribe -- {{wav}} {{model}} {{lang}}

# Dictée par le moteur d'Apple, pour de vrai (macOS 26, français installé).
test-apple:
    cargo nextest run -p oku-slides-stt --run-ignored ignored-only -E 'binary_id(oku-slides-stt::apple) | test(/^apple::/)'

# Vérification manuelle avec le vrai agent (réseau, compte) : just agent-smoke fixtures/roadmap.json "Rends le titre de la slide 2 plus percutant"
agent-smoke deck prompt:
    cargo run -p oku-slides-agent --example smoke -- {{deck}} "{{prompt}}"

# Supprime les builds (Rust et interface)
clean:
    cargo clean
    rm -rf ui/dist ui/node_modules
