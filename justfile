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

# Tests de bout en bout (Playwright, sous Bun) contre un serveur neuf sur le port 4872
e2e: install
    #!/usr/bin/env bash
    set -euo pipefail
    (cd ui && bun run build && bunx playwright install chromium)
    cargo build -q -p oku-slides-cli -p oku-slides-agent --bin oku-slides --bin oku-slides-fake-agent
    OKU_SLIDES_AGENT__COMMAND="$PWD/target/debug/oku-slides-fake-agent" \
    OKU_SLIDES_AGENT__ARGS="--script $PWD/fixtures/agent/e2e.json" \
      ./target/debug/oku-slides --fonts {{fonts}} serve "{{deck}}" --ui ui/dist --port 4872 &
    trap "kill $! 2>/dev/null" EXIT
    for _ in $(seq 1 50); do curl -sf -o /dev/null http://127.0.0.1:4872/ && break; sleep 0.2; done
    cd ui && E2E_URL=http://127.0.0.1:4872/ bun run e2e

# Vérifications : format, clippy, types TypeScript, polices en LFS
check: install fonts-lfs
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

# Formate le code Rust
fmt:
    cargo fmt --all

# Corrige automatiquement ce que clippy sait corriger
lint-fix:
    cargo clippy --workspace --all-targets --fix --allow-dirty --allow-staged

# Couverture des tests (lignes), crate par crate
coverage:
    cargo llvm-cov nextest --workspace --exclude oku-slides-app --summary-only

# Dépendances : licences, avis de sécurité, sources
audit:
    cargo deny check
    cargo audit

# Relance une commande à chaque modification : just watch test
watch cmd="test":
    cargo watch -s "just {{cmd}}"

# Tout ce que vérifie l'intégration continue, en local
ci: check test audit

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

# Transcrit un fichier WAV avec le modèle de dictée (turbo, large ou base ; langue facultative)
transcribe wav model="turbo" lang="":
    cargo run -q --release -p oku-slides-stt --features "{{stt_features}}" --example transcribe -- {{wav}} {{model}} {{lang}}

# Vérification manuelle avec le vrai agent (réseau, compte) : just agent-smoke fixtures/roadmap.json "Rends le titre de la slide 2 plus percutant"
agent-smoke deck prompt:
    cargo run -p oku-slides-agent --example smoke -- {{deck}} "{{prompt}}"

# Supprime les builds (Rust et interface)
clean:
    cargo clean
    rm -rf ui/dist ui/node_modules
