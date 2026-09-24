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
| `crates/slider-cli` | Outil `slider` : rendu, enregistrement, relecture, mesures |
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

## Tests

```sh
cargo test --workspace
```
