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

## Outil en ligne de commande

```sh
just slider check fixtures/roadmap.json                       # valider un deck
just slider render fixtures/roadmap.json --out out --theme pro # SVG + layout.json
just save fixtures/roadmap.json deck.slider.pdf                # enregistrer en .slider.pdf
just slider open deck.slider.pdf --out extrait/                # deck.json + assets
just slider strip deck.slider.pdf copie.pdf                    # copie sans sources
just bench                                                     # latences
just slider schema                                             # schéma JSON des opérations
```

`--fonts <dossier>` ajoute des polices (TTF/OTF) ; sans elles, Typst utilise
ses polices intégrées (Libertinus Serif, New Computer Modern, DejaVu Sans Mono).

## Lancer l'application

Tout se lance depuis la racine du dépôt avec [just](https://just.systems).
Prérequis : Rust, [Bun](https://bun.sh), just, et la CLI Tauri
(`cargo install tauri-cli --version "^2"`). `just doctor` vérifie tout.

```sh
just dev      # app de bureau en développement (installe les dépendances de l'interface)
just build    # paquet .app / .dmg
just web      # interface dans le navigateur avec rechargement à chaud (localhost:5173)
just serve    # interface compilée dans le navigateur (127.0.0.1:4870)
just test     # tests Rust
just check    # format, clippy, types TypeScript
just          # toutes les commandes
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

