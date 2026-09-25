# Écho — toutes les commandes se lancent depuis la racine du dépôt.
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
    cargo tauri dev --additional-watch-folders "{{justfile_directory()}}/typst/slider"

# Construit l'app (.app / .dmg sur macOS)
build: install
    cargo tauri build

# Interface dans le navigateur, sans Tauri : http://127.0.0.1:4870
serve file=deck: install
    cd ui && bun run build
    cargo run -q -p slider-cli -- --fonts {{fonts}} serve "{{file}}"

# Interface avec rechargement à chaud (http://localhost:5173) et API Rust
web file=deck: install
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build -q -p slider-cli
    ./target/debug/slider --fonts {{fonts}} serve "{{file}}" --ui ui/dist &
    trap "kill $! 2>/dev/null" EXIT
    cd ui && bun run dev

# Tests Rust (hors app Tauri)
test:
    cargo test --workspace --exclude slider-app

# Vérifications : format, clippy, types TypeScript
check: install
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cd ui && bun run typecheck

# Formate le code Rust
fmt:
    cargo fmt --all

# Outil en ligne de commande : just slider render fixtures/roadmap.json --out out
slider *args:
    cargo run -q -p slider-cli -- --fonts {{fonts}} {{args}}

# Latences de rendu et d'enregistrement (build optimisé)
bench file=deck:
    cargo run -q --release -p slider-cli -- --fonts {{fonts}} bench "{{file}}"

# Enregistre un deck JSON en .echo.pdf
save file=deck out="deck.echo.pdf":
    cargo run -q -p slider-cli -- --fonts {{fonts}} save "{{file}}" "{{out}}"

# Compile un exemple de la bibliothèque Typst : just typst pro
typst example="demo":
    typst compile --root typst/slider --font-path {{fonts}} typst/slider/examples/{{example}}.typ

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
    [ "$ok" = 1 ]

# Accélération de la dictée selon la machine (Accelerate sur Mac)
stt_features := if os() == "macos" { "accelerate" } else { "" }

# Transcrit un fichier WAV avec le modèle de dictée (turbo, large ou base ; langue facultative)
transcribe wav model="turbo" lang="":
    cargo run -q --release -p slider-stt --features "{{stt_features}}" --example transcribe -- {{wav}} {{model}} {{lang}}

# Supprime les builds (Rust et interface)
clean:
    cargo clean
    rm -rf ui/dist ui/node_modules
