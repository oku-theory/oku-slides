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
- `PackWidgets` : fichiers `widgets/<nom>.typ` (`_commun` partagé) servis sous `/packs/<id>/`,
  module d'assemblage généré `/packs/<id>/widgets.typ`. `probe_file` (`widget_probe.rs`) : un
  fichier seul → `FileProbe` (schéma ou diagnostics à ligne et colonne), nom déclaré = nom de
  fichier ; `assemble` : registre des seuls valides. `Registries` (`registry.rs`) mémoïse par
  pack et par `probe_key` ; `base_registry` donne la base sans pack.
- Erreurs bornées (`error.rs`) : `RenderErrorKind` compte `Timeout`, `OutOfMemory`, `WorkerLost`,
  `Frame`, `Stale`, `PackDisabled` en plus des échecs Typst ; `is_bounded` dit lesquels
  (hors `Frame`, `Stale`) font replier la session sur les widgets de base.
- `wire` (`wire.rs`) : `WireError` : forme sérialisable (sans backtrace) de
  `RenderError`/`RegistryError`, pour le tube du travailleur ; `Layout` est aussi sérialisable ; `Fonts::files` expose
  les octets des polices pour les empreintes et le rejeu.
- `OkuSlidesWorld` (`world.rs`, privé) : système de fichiers virtuel (bibliothèque embarquée,
  deck, assets, pack) et polices (`Fonts`, `fonts.rs`) sans police système.
- `charts` (privé) : élément à `host("chart")` → SVG rendu par `oku-slides-chart`, servi sous
  `/charts/<empreinte>.svg` ; l'empreinte couvre options, taille, police, palette, libellés.
- `diagnostic` : messages Typst localisés ; `palette` : couleurs nommées, palette des graphiques.

## Points d'extension

- Nouveau format de sortie d'une page : une méthode sur `Compiled`.
- Une nouvelle métadonnée lue dans le document : une étiquette Typst (`oku-slides-*`) côté
  bibliothèque, un champ de `Layout` et une entrée dans `Layout::layout()` côté Rust.

## Dépendances

`oku-slides-core` (`Deck`, `WidgetRegistry`), `oku-slides-i18n`, `oku-slides-chart`, `typst` et
sa famille (`typst-pdf`, `typst-svg`, `typst-render`, `typst-layout`, `typst-assets`) ; dev :
`oku-slides-pdf`, `speculoos`, `insta`.
