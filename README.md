# Slider

Slides créées et modifiées à la voix. Le document est un PDF (`.slider.pdf`)
qui contient ses propres sources : n'importe quel lecteur l'affiche, Slider le
rouvre pour continuer l'édition.

Plan d'implémentation : [Slider — plan d'implémentation](https://claude.ai/code/artifact/fca9973c-7aff-454e-b40b-9a900dedaf57).

## Contenu du dépôt

| Chemin | Rôle |
| --- | --- |
| `crates/slider-core` | Modèle du deck, opérations réversibles, historique, validation, zip des sources |
| `crates/slider-render` | Typst embarqué : SVG par slide, PDF, boîtes des éléments |
| `crates/slider-pdf` | Blob des sources dans le PDF (`/PieceInfo`) : écriture, lecture, retrait |
| `crates/slider-session` | Session d'édition (deck ouvert, historique, rendu, fichier), partagée par l'app et le serveur de dev |
| `crates/slider-cli` | Outil `slider` : rendu, enregistrement, relecture, mesures, serveur de dev |
| `src-tauri` | Application de bureau Tauri 2 |
| `ui` | Interface React + TypeScript (design system Papier) |
| `assets/fonts` | Polices livrées (Newsreader, Instrument Sans, Inter) |
| `typst/slider` | Bibliothèque Typst de slides (structure, jetons, thèmes `papier` et `pro`) |
| `fixtures/` | Decks d'exemple |

## Utilisation

```sh
cargo build --release
B=target/release/slider

$B check fixtures/roadmap.json                       # valider un deck
$B render fixtures/roadmap.json --out out --theme pro # SVG + layout.json
$B save fixtures/roadmap.json deck.slider.pdf --assets ./images
$B open deck.slider.pdf --out extrait/                # deck.json + assets
$B strip deck.slider.pdf copie.pdf                    # copie sans sources
$B bench fixtures/roadmap.json                        # latences
$B schema                                             # schéma JSON des opérations
```

`--fonts <dossier>` ajoute des polices (TTF/OTF) ; sans elles, Typst utilise
ses polices intégrées (Libertinus Serif, New Computer Modern, DejaVu Sans Mono).

## Lancer l'application

Prérequis : Rust, Node 20+, et la CLI Tauri (`cargo install tauri-cli --version "^2"`).

```sh
cd ui && npm install && cd ..
cargo tauri icon src-tauri/icons/icon.png   # une fois : génère icon.icns pour macOS
cargo tauri dev                             # lance Vite et l'app
cargo tauri build                           # paquet .app / .dmg
```

Sans Tauri, dans un navigateur :

```sh
cd ui && npm run build && cd ..
cargo run -p slider-cli -- --fonts assets/fonts serve fixtures/roadmap.json
# puis http://127.0.0.1:4870 (ou `npm run dev` dans ui/ pour le rechargement à chaud, port 5173)
```

### Édition à la main

| Geste | Effet |
| --- | --- |
| Clic | Sélectionne un élément ou un champ (titre, surtitre…) |
| Glisser, poignées | Déplace ou redimensionne un élément libre |
| Double-clic ou Entrée | Modifie le texte (une ligne par paragraphe, `- ` pour une puce) |
| Flèches (Maj : ×10) | Déplace l'élément libre sélectionné de 1 pt |
| Suppr | Supprime l'élément sélectionné |
| ⌘Z, ⇧⌘Z | Annuler, rétablir (un geste = une annulation) |
| ⌘S, ⇧⌘S, ⌘O | Enregistrer, enregistrer sous, ouvrir |

## Tests

```sh
cargo test --workspace
```
