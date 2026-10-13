# oku-slides-import

Import d'une présentation PowerPoint (`.pptx`) : lecture en représentation
intermédiaire, `source::SourceDeck` (neutre vis-à-vis du format, résolue,
chaque valeur stylée avec sa provenance), puis projection en un plan d'ops
Oku. Spec : `docs/superpowers/specs/2026-10-09-import-pptx-design.md`.

## API

- `read_pptx(bytes, &ReadLimits) -> Result<SourceDeck, ImportError>` : fatal seulement pour un paquet
  illisible, pas une présentation, protégé, sans master utilisable, ou une limite dépassée. `source` : les types.
- `retrofit::project(&SourceDeck, &RetrofitTarget) -> ImportPlan` : le contenu sur un pack et un thème
  existants (modèles `oku_slides_core::Template`, registre, `ColorTokens`, `PageTokens` du thème).
- `hifi::project(&SourceDeck, &HifiTarget) -> ImportPlan` : la source reproduite sur un thème et un pack
  générés (`HifiTarget` : registre de base, `FontCatalog`, ids, nom, date) ; `hifi::sought_fonts` : les
  familles qu'il cherchera (l'appelant n'ajoute au catalogue que celles-là du système).
- `ImportPlan { ops, assets, widget_requests, report, theme, pack, schemas, fonts }` : en hi-fi, les
  `PackFiles` du thème et du pack (au bundle avant les ops), les schémas déclarés (la sonde doit les
  retrouver), le dossier `fonts/` ; `report_json(l)` est la forme de `--report`.
- `report` : `ImportReport` (`slides`, `fonts`, `coverage`…), `Loss`/`LossKind` (`import-loss-*`),
  avertissements (`import-warning-*`), `Coverage`. `coverage::coverage(…, omitted)` mesure sur le deck obtenu
  ce qui reste de la source (runs, images, tableaux, graphiques ; `None` : plan refusé par le cœur, un bogue).
- `fonts` : `FontCatalog` (`add(FontOrigin::App | System, octets)`), `resolve(&[SourceFont], wanted,
  &FontCatalog) -> FontPlan` (`choices`, `files`, `used(famille)`), `sought`, `SUBSTITUTES`.
- Ré-exports : `ImportPlan`, `ImportReport`, `RetrofitTarget`, `HifiTarget`, `ImportError`,
  `ImportErrorKind`, `ReadLimits`, `read_pptx`.

## Couches

- `source/` ne dépend de rien ; `error`, `limits`, `collect` (assets, avertissements, fatal différé) sont
  neutres. `pptx/` en dépend seul ; `shapes` → `frame` → `smartart` → `shapes` est borné par `GroupCtx.depth`.
- Projecteurs `retrofit/`, `hifi/` : `source/`, `plan`, `report`, `coverage`, `fonts`, `oku-slides-core`,
  `oku-slides-chart` ; jamais `pptx/`. Purs (ni aléa, ni horloge, ni fichier, ni session) : ids et date
  viennent de la cible. La crate ne dépend ni de la session, ni du rendu, ni de la CLI.
- `pptx/` : `package` (zip borné), `xml` (nœuds bornés, `TooLarge` fatal), `context`, `placeholder`,
  `cascade` (seule résolution des styles texte), `master`, `slide`, `shapes`, `frame`, `table`, `chart`,
  `smartart`, `fonts` + `eot`. Aides de test : `pptx/test_support`, `src/test_support.rs`, `tests/common`.

## Retrofit (`retrofit/`)

`mod.rs` (accents déduits, décor omis, une slide après l'autre, demandes groupées par `pattern`) ; `decor`
(prédicat d'omission, partagé avec `coverage`) ; `slide` (placement `flow` ou libre) ; `heading` (titres
déduits) ; `cluster` (cartes, textes fusionnés) ; `template` (profil, score, choix) ; `content` et `group`
(éléments, repli `stack`) ; `text`, `table`, `chart`, `color` (OKLab, ΔE, WCAG), `zone` (zone de corps, `snap`).

## Hi-fi (`hifi/`)

- `mod.rs` : page, polices, thème, pack, slides ; ops `set_theme`, `set_templates`, puis `add_slide`,
  `set_notes` ; pertes du pack sur la première slide de leur layout ; couverture sans omission.
- `page` : la page source sur 1920 × 1080 (échelle `min`, centrée, `PageFitted`). `color` : jeton du schéma
  sans modificateur (`SLOT_TOKENS`), sinon `#rrggbb[aa]`. `theme` : jetons du thème généré (couleurs,
  polices de `FontPlan::used`, tailles des placeholders ou `txStyles`, puce `component.list.marker`,
  marges de page nulles) et `ThemeManifest`.
- `geometry/` : `outline` — un preset ou un `custGeom` en `PathCommand` normalisés au cadre ; `presets`
  (`presetShapeDefinitions.xml` d'ECMA-376 embarqué), `formula` (guides), `pen` (arcs en cubiques).
  `presetShapeDefinitions.xml` : définitions de formes de l'ECMA-376 (Office Open XML, partie 1,
  annexe D), copie de LibreOffice core `oox/source/drawingml/customshapes/presetShapeDefinitions.xml` ;
  jeu de données tiers, exempté de la limite des 500 lignes (aucune mention de licence dans le fichier ;
  le dépôt LibreOffice core porte la MPL 2.0, `COPYING.MPL`).
- `typst/` : le décor d'un master ou d'un layout en Typst (`place` par dessin, `shape`, `text` ; tout
  texte source en littéral échappé ; une police du thème par son rôle, `font-of(t, "display" | "body")`,
  comme les éléments : `GenCtx::font_name`) ; `placeholder` : fond et `champ(…)` d'un placeholder.
- `widget` : un widget de slide par layout (`l<rang>-<slug>`, paramètres `titre`, `corps`…, `date`, `pied`,
  `numero` à `{n}`, `grid`, slot `free` `extras`, `decor` si masqué) ; `pack` : `_commun.typ`, registre
  déclaré, `pack.json` (un modèle par widget, sa première slide en exemple). Le pack tient dans les bornes
  de la sonde (`PackLimits`, `[render]` lu par la CLI) : `max_files − 1` widgets (les layouts les plus
  employés ; les autres sur `blank`, fond et décor copiés en éléments, perte `LayoutDropped`), chaque
  fichier sous `max_file_bytes` (décor coupé, perte `DecorTruncated`).
- `slide` : placeholders de texte en paramètres, le reste en éléments d'`extras` (fond propre d'abord) ;
  `element`, `element_text` : un item en `text`, `shape` (natif ou `path`), `image`, `table`, `chart`, une
  forme qui porte du texte en deux éléments ; pertes par item. `table` : le style d'un tableau lu sur ses
  cellules (`TableLook` : en-tête, bandes, jetons `component.table` posés par le thème d'après le premier
  tableau du deck) ; fonds par cellule, bordures, largeurs inégales, autre style → perte `TableStyle`.
- Le rendu compile et sonde les instantanés : `oku-slides-render/tests/import_{decor,pack}.rs`.

## Polices (`fonts/`)

Embarquées dans le PPTX → catalogue (app, puis système) → substitut métrique (Carlito, Caladea,
Liberation, livrés dans `assets/fonts`) → `Missing` (police du corps ; si elle manque aussi, la première
police de l'app sans empattement de `FALLBACKS`, sinon celle du moteur, dite au rapport). Le deck embarque
les faces du PPTX et du système (une par style), jamais celles de l'app ; `fsType` restreint jamais, ni un
fichier de plus de 32 Mio ou au-delà de 64 Mio en tout (la recherche passe à l'étape suivante). Le
catalogue garde les octets qu'on lui donne sans copie (ceux du rendu pour les polices de l'app).
`face.rs` : famille lue comme Typst la lit.

## Limites connues

- t-816 (retrofit) : un contenu plus haut que sa zone de flux sort de la page ; la couverture mesure le
  `Deck`, pas le rendu.
- Hi-fi : dans les éléments d'une slide, rotation, police ou espacement par run, soulignement, barré,
  tirets, dégradé (sa première couleur), image hors du contour sont des pertes (le décor, en Typst libre,
  les rend) ; ligne de texte : la police (ascendante, descendante `hhea`) centrée dans le pas PowerPoint
  (`typst::LineBox`), première ligne de base recalée par les marges `inset` d'un élément, par les bords de
  ligne Typst au décor et aux placeholders — règle mesurée sur la référence d'alto, pas sur PowerPoint ; un
  élément n'a qu'un espace entre paragraphes (`Style.spacing` : pas de ligne, espaces après et avant
  dominants), un autre est une perte `ParagraphStyle`, et un paragraphe vide est omis avec sa ligne ; un
  placeholder de texte passe sous les `extras` ; le texte du décor n'est pas éditable.

## Points d'extension

- Nouveau format (PDF, lot 6) : un frère de `pptx/` qui rend la même `SourceDeck` (`collect::Sink`).
- Nouveau projecteur : un frère de `retrofit/` et `hifi/` qui rend un `ImportPlan` ; `Mode` (CLI) gagne
  sa variante.
- Nouvelle perte : une variante de `LossKind` et sa clé `import-loss-*` dans les 20 `document.ftl`.

## Dépendances

`zip`, `roxmltree`, `ttf-parser`, `uuid`, `oku-slides-core`, `oku-slides-chart`, `oku-slides-i18n`,
`serde`, `serde_json`, `snafu`.
