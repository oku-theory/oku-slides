# oku-slides-render

Rendu des decks avec Typst embarqué : le deck est exposé en JSON à la
bibliothèque `typst/oku-slides` (aucun texte du deck n'est du code Typst) ;
le même document compilé donne SVG, PNG et PDF.

## Types clés

- `Renderer` (`lib.rs`) : garde un monde Typst vivant entre deux rendus (compilation
  incrémentale). `render`/`set_deck` compilent le deck ; `set_widgets` charge les widgets de
  base surchargés par un `PackWidgets` ; `set_language` change la langue des cadres de repli.
  Les graphiques sont redessinés avant Typst si la palette du thème compilé diffère de celle
  utilisée pour les dessiner (un second passage).
- `Compiled` (`lib.rs`) : un document compilé — `svg`/`png`/`pdf`, `page_hash` (clé de cache),
  `layout()` (lu une fois dans le document, `OnceCell`).
- `Layout` (`lib.rs`) : ce que le rendu dit du document — `SlidePage` (page de chaque slide),
  `ElementBox` (boîte réelle), `FieldBox` (champ éditable), `ZoneBox`/`GridBox` (zone libre et
  sa grille), `TextStyle` (typographie mesurée) — rangés par page.
- Champs (`typst/oku-slides`) : `field()` émet une boîte par champ, même vide ; `fields-stack`
  colle le marqueur d'un champ vide à l'item voisin, sans occuper de place ni recevoir
  d'espacement (`FieldBox.empty`, `side`).
- `load_registry` + `Registries` (`registry.rs`) : le `WidgetRegistry` (`oku-slides-core`) lu
  par une sonde Typst qui importe `widgets.typ`, base et pack (`PackWidgets`) ; `Registries`
  mémoïse par pack pour éviter de recompiler la sonde à chaque rendu.
- `OkuSlidesWorld` (`world.rs`, privé) : système de fichiers virtuel (bibliothèque embarquée,
  deck, assets, pack) et polices (`Fonts`, `fonts.rs`) sans police système.
- `charts` (privé) : élément à `host("chart")` → SVG rendu par `oku-slides-chart`, servi sous
  `/charts/<empreinte>.svg` ; l'empreinte couvre options, taille, police, palette, libellés.
- `diagnostic` : messages Typst (erreurs, avertissements) localisés.
- `palette` : résolution d'un nom de couleur en hex, palette des graphiques du thème.

## Points d'extension

- Un nouveau format de sortie d'une page : une méthode sur `Compiled` (voir `svg`/`png`/`pdf`).
- Une nouvelle métadonnée lue dans le document : une étiquette Typst (`oku-slides-*`) côté
  bibliothèque, un champ de `Layout` et une entrée dans `Layout::layout()` côté Rust.

## Dépendances

`oku-slides-core` (`Deck`, `WidgetRegistry`), `oku-slides-i18n`, `oku-slides-chart`, `typst` et
sa famille (`typst-pdf`, `typst-svg`, `typst-render`, `typst-layout`, `typst-assets`) ; dev :
`oku-slides-pdf`, `speculoos`, `insta`.
