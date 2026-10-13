# Import d'une présentation PowerPoint (puis PDF)

Tâche t-692. Aujourd'hui Oku ne sait rien ouvrir d'autre que ses propres
decks (`*.oku-slides.pdf`, ou JSON en développement). Cette spec ajoute
l'**import d'un `.pptx`** en deck Oku, selon deux modes :

- **hi-fidelity** : copie la plus fidèle possible ; crée un **nouveau thème,
  un nouveau pack (templates + widgets)** et le deck ;
- **retrofit** : projette le contenu sur un **pack et un thème existants**,
  et ne crée un widget (dans un pack enfant) que si rien ne convient.

Le pipeline est **aussi déterministe que possible** : extraction, résolution
des styles, squelette et gros du travail sont du code pur ; un agent ne fait
que le **fine-tuning** à la fin. L'import PDF est une phase 2 (spec à part)
qui réutilise la même représentation intermédiaire.

S'appuie sur `2026-10-01-packs-portables-design.md` (packs embarqués,
`parent`), `2026-10-05-themes-packs-editeur-design.md` (thèmes),
`2026-10-02-creation-widgets-design.md` (widgets Typst, sonde),
`2026-09-28-champs-editables-design.md` (champs `{slide, owner, field}`) et
`2026-10-08-widget-tableau-design.md` (tableaux).

## Décisions (choisies le 2026-10-09)

1. **PPTX d'abord, PDF ensuite.** La v1 couvre le PPTX de bout en bout dans
   les deux modes. La représentation intermédiaire (`SourceDeck`) est
   conçue dès maintenant pour accueillir un lecteur PDF.
2. **Hi-fi : éditable d'abord, fidèle au mieux.** Les textes restent
   toujours des champs éditables (`plain`/`rich`) ; formes → `shape`,
   tableaux → `table`, graphiques → `chart` avec leurs données. Seul
   l'intraduisible est figé en image (ou SVG), et le rapport le liste.
3. **Un cœur commun, deux points d'entrée.** Une crate
   `oku-slides-import` sans UI. La **CLI** fait le déterministe pur
   (scriptable, testable) ; l'**app** fait l'import complet avec l'agent.
4. **Retrofit : contenu + géométrie relative, style du thème cible.** On
   garde la structure (titres, corps, listes, tableaux, graphiques, images)
   et l'emphase sémantique (gras, italique). La disposition relative sert à
   choisir le template et l'agencement de grille. Les couleurs sont
   projetées sur les tokens cibles ou abandonnées ; aucune surcharge de
   tokens n'est injectée.
5. **Retrofit : motif sans widget → widget dédié.** Un repli déterministe
   existe toujours et ne bloque jamais. Quand ce repli perd de
   l'information (relations, données, ordre — pas le seul style), l'agent
   **crée un widget dédié**, sans question, dans un **pack enfant** du pack
   cible (le pack d'origine n'est jamais modifié), **un widget par motif**.
6. **Hi-fi : un widget de slide par layout PPTX utilisé.** Le regroupement
   de slides similaires en widgets d'élément est laissé à l'agent pendant le
   fine-tuning (il propose, l'utilisateur confirme).
7. **Fine-tuning : boucle autonome bornée, puis revue finale.** L'agent
   traite les slides de la pire à la meilleure jusqu'à un seuil ou un
   budget d'itérations, puis présente un récapitulatif. Tout est annulable.
8. **Rendu de référence par LibreOffice, optionnel.** `soffice --headless
   --convert-to pdf`, rasterisé par `hayro` (déjà dans le lockfile via
   Typst 0.15). Sans LibreOffice : pas de diff visuel, l'import continue.
9. **Approche : représentation intermédiaire + deux projecteurs** (plutôt
   que deux convertisseurs directs ou un agent qui fait tout). La partie
   difficile — cascade des styles et géométrie — est isolée dans le
   lecteur ; les projecteurs sont des fonctions pures testées en snapshots.

Décisions du plan du lot 2 (2026-10-10) :

- **Modèles dans le cœur.** `Template` et `PackManifest` sont dans
  `oku-slides-core` (`pack.rs`) ; la session les ré-exporte. La crate
  d'import lit les modèles d'un pack sans dépendre de la session.
- **Pas de pack enfant au lot 2.** Le retrofit émet les `WidgetRequest`
  (au plan et au rapport) ; le pack enfant est créé au lot 5, avec le
  premier widget créé par l'agent. `ImportPlan` n'a pas de champ `pack`
  au lot 2.
- **Polices au lot 3.** Le retrofit prend le style du thème cible :
  aucune police n'est cherchée ni substituée. `fonts.rs` (recherche
  système, substitution) arrive avec le hi-fi.
- **Couverture du contenu au lot 2.** `coverage.rs` mesure sur le deck
  obtenu le rappel des runs, la présence des images et l'égalité des
  données des tableaux et graphiques ; le lot 4 garde la référence
  LibreOffice et l'écart visuel (`metrics.rs`).

Décisions du plan du lot 3 (2026-10-11) :

1. **Base étendue** (cœur et Typst). Le widget `shape` gagne `shape:
   "path"` et le paramètre hôte `path` (`{ d }`, coordonnées 0..1 étirées
   au cadre, comme une image) ; `Style.radius` numérique ; `Style` gagne
   `valign` (`top`/`middle`/`bottom`), `inset` (4 marges en pt) et
   `tracking` (pt), lus par le widget `text`. Une forme qui porte du texte
   donne deux éléments (la forme, puis le texte) sur le même cadre.
   Rotation, police par run, soulignement et barré des éléments : pertes
   signalées, sans extension.
2. **Page.** Le thème généré garde 1920 × 1080 ; une source d'un autre
   ratio est mise à l'échelle uniforme (`min(1920 / w, 1080 / h)`) et
   centrée, signalé (`PageFitted`). Marges de page du thème généré nulles :
   la zone de corps est la page, les cadres sont en pt page.
3. **Polices** (`fonts/`, pur) : embarquée dans le PPTX → catalogue (polices
   de l'app, puis du système, énumérées par la CLI avec `fontdb`) →
   substitut aux métriques compatibles (Calibri → Carlito, Cambria →
   Caladea, Arial/Helvetica → Liberation Sans, Times New Roman → Liberation
   Serif, Courier New → Liberation Mono ; livrés dans `assets/fonts`) →
   police du corps, signalé. `fsType` « restricted » jamais embarqué.
4. **Couleurs.** `Origin::Scheme` sans modificateur → référence de jeton
   (`lt1 → bg`, `dk1 → ink`, `dk2 → ink-muted`, `lt2 → surface`, `accent1 →
   accent`, `accent2..6 → accent-2..accent-6`, `hlink → link`, `folHlink →
   link-visited`) ; avec modificateurs ou `Explicit` → `#rrggbb` résolu
   (`#rrggbbaa` avec alpha).
5. **Presets.** Un interpréteur des guides ECMA-376 évalue
   `presetShapeDefinitions.xml` embarqué (tous les presets, `adj` compris) ;
   `rect`, `roundRect`, `ellipse`, `line` restent natifs.
6. **Pack et thème générés vivent dans le deck seulement** (embarqués) ; la
   bibliothèque n'est pas touchée.
7. **Sonde.** Le projecteur déclare les schémas des widgets générés (types
   du cœur) en plus du Typst ; la CLI sonde le pack par le processus de
   rendu et refuse tout écart (un bogue du générateur).
8. **Pack.** `_commun.typ` (décor de chaque master), un widget de slide par
   layout utilisé (`[a-z][a-z0-9-]*`), placeholders → `field(ctx, …)` à
   leur position (`rich` ; `plain` pour date, pied, numéro), slot `free`
   `extras` ; un modèle par layout, la première slide source qui l'utilise
   en exemple.
9. **Effets.** `LossKind::Effects` (t-817) est émis par le hi-fi pour les
   ombres, reflets et 3D ignorés.
10. **Bornes** (revue du plan). Le pack généré respecte les limites de la
    sonde (`[render]` de la config : 64 fichiers, 256 Kio par fichier) : au
    plus `max_files − 1` widgets de layout, pour les layouts les plus
    employés ; les autres slides prennent `blank`, fond et décor en
    éléments (`LayoutDropped`) ; un fichier trop gros perd ses derniers
    items de décor (`DecorTruncated`). La CLI relance la sonde tant que le
    budget laisse des fichiers non sondés et distingue une borne atteinte
    (`cli-import-probe-limit`) d'un écart de schéma. Polices embarquées :
    32 Mio par fichier, 64 Mio en tout. Police du corps absente : première
    police de repli livrée et chargée (Liberation Sans, Carlito, Inter),
    sinon le rapport le dit ; le message nomme le repli retenu.
11. **Interligne** (revue du plan). Le widget `text` rend `leading` (un
    `set` enfermé dans un `if` ne s'appliquait pas). Première ligne de base
    au modèle CSS (police centrée dans le pas de ligne), commune aux
    éléments, placeholders et décor ; l'espace entre paragraphes et items
    de liste d'un élément passe par `Style.spacing` (pt).
12. **Tableaux** (revue du plan). Le style se lit sur les cellules ; le
    thème généré règle `component.table` (en-tête, bandes, `body-bg`
    nullable, taille, filet) d'après le premier tableau ; chaque tableau
    pose `header-row` et `striped` ; le reste (largeurs de colonnes,
    couleurs par cellule, bordures) donne une perte `TableStyle` ; un en-tête ou un corps de style
    hétérogène (couleur, gras, italique, police) en est une aussi.

Précisions prises en chemin :

- Les polices de l'app (substituts compris) ne sont pas embarquées : elles
  sont partout où le deck s'ouvre ; seules les faces du PPTX ou du système
  vont dans `fonts/` du deck.
- Le paramètre `numero` vaut `{n}` (le numéro de la page) par défaut.
- Un paramètre `decor` (`flag`, défaut vrai) existe quand une slide du
  layout masque le décor (`showMasterSp="0"`).
- Le fond propre d'une slide (autre que celui de son layout) est le premier
  élément d'`extras`, sur toute la page.
- `ThemeManifest` descend dans le cœur (`oku_slides_core::pack`), ré-exporté
  par la session, construit par l'import.
- CLI : `--no-system-fonts` ; ids du pack et du thème dérivés du SHA-256 du
  `.pptx` (UUID v8, deux moitiés) : deux imports du même fichier donnent le
  même pack.
- `component.list.marker` prend la puce du corps du master ;
  `marker-color` est nullable (`null` : la couleur du texte).

Défauts retenus sans question :

- **Taille de page.** Hi-fi : 1920 × 1080, la source mise à l'échelle et
  centrée (décision 2 du lot 3). Retrofit : la page du thème cible, la
  projection sur la grille absorbe l'écart de ratio.
- **Polices** (hi-fi, lot 3). Ordre de recherche : polices embarquées dans le PPTX →
  polices de l'app et du système → substitut métriquement compatible (Carlito ↔ Calibri,
  Caladea ↔ Cambria, Liberation ↔ Arial/Times/Courier), signalé au rapport.
  La police retenue est embarquée dans `fonts/` du deck, sauf celles de l'app.

## Architecture

```
.pptx ──▶ pptx::read ──▶ SourceDeck ──┬─▶ hifi::project ─────────┐
                                     └─▶ retrofit::project ─────┤
                         (target pack, registry, theme) ────────┘
                                                                 ▼
                                                            ImportPlan
                                     ┌───────────────────────────┤
                                     ▼                           ▼
                        CLI: session neuve, apply,     App: session neuve, apply,
                        save + report.json             puis agent de fine-tuning
```

Crate `crates/oku-slides-import` :

| Module | Rôle |
|---|---|
| `source/` | `SourceDeck` et ses types (format neutre) |
| `pptx/` | lecteur OOXML : rels, presentation, theme, master, layout, slide, chart, table |
| `collect.rs` | sorties neutres de toute lecture : assets dédupliqués, avertissements bornés, erreur fatale différée, assainissement des chaînes |
| `pptx/cascade.rs` | résolution des styles texte (une seule fonction) |
| `pptx/context.rs` | contexte d'une part (relations, couleurs, parts liées) |
| `pptx/placeholder.rs` | lien et héritage des placeholders |
| `pptx/frame.rs` | cadres graphiques : tableau, graphique, SmartArt, OLE |
| `pptx/units.rs` | EMU → pt, angles, pourcentages |
| `pptx/eot.rs` | polices embarquées EOT (`.fntdata`) |
| `pptx/geometry.rs` | lecture des géométries (`prstGeom`, `custGeom`) ; la conversion en chemin est dans `hifi/geometry/` |
| `pptx/fonts.rs` | inventaire des familles utilisées, polices embarquées (lot 1) |
| `fonts/` | résolution des polices : catalogue (app, système), substituts, faces à embarquer (lot 3) |
| `hifi/mod.rs` | projecteur hi-fi : page, polices, thème, pack, slides, ops, couverture (lot 3) |
| `hifi/page.rs`, `hifi/color.rs` | page source sur 1920 × 1080 ; couleur en jeton ou en hexadécimal |
| `hifi/theme.rs` | jetons et `ThemeManifest` du thème généré |
| `hifi/geometry/` | presets ECMA-376 et `custGeom` en chemins normalisés au cadre (guides, crayon) |
| `hifi/typst/` | générateur Typst du décor (formes, chemins, texte, images) et des placeholders |
| `hifi/widget.rs`, `hifi/pack.rs` | widget de slide d'un layout ; pack généré, schémas déclarés, `pack.json` |
| `hifi/slide.rs`, `hifi/element*.rs` | une slide : placeholders en paramètres, le reste en éléments d'`extras` |
| `retrofit/` | projecteur retrofit : score des templates, grille, couleurs, replis |
| `pattern.rs` | signature de motif (partagée retrofit / regroupement hi-fi) |
| `plan.rs` | `ImportPlan`, `WidgetRequest` |
| `reference.rs` | appel `soffice`, rasterisation `hayro` |
| `coverage.rs` | couverture du contenu, mesurée sur le deck obtenu (lot 2) |
| `metrics.rs` | écart visuel (hi-fi, lot 4) |
| `report.rs` | `ImportReport` |

Dépendances : `zip` (existant), `roxmltree` (nouveau, DOM lecture seule),
`hayro` (déjà transitif, devient direct pour `reference.rs`). La crate ne
dépend ni de la session ni de l'UI : elle prend en entrée le registre de
widgets, les modèles du pack (`oku_slides_core::Template`) et les jetons du
thème cibles, et rend un plan.

## Représentation intermédiaire : `SourceDeck`

**Résolue** (plus aucun héritage à calculer), mais chaque valeur stylée
garde **sa provenance**. La définition faisant foi est le code :
`crates/oku-slides-import/src/source/mod.rs` (et ses sous-modules), avec les
« Décisions de la revue » du plan du lot 1
(`docs/superpowers/plans/2026-10-09-import-pptx-lot1.md`) qui priment sur
tout croquis antérieur : un thème par master, `Origin::Scheme { name, slot,
mods }`, `PlaceholderLink`, styles par niveau typés, avertissements typés.

- `frame` est en **pt source** ; la mise à l'échelle vers 1920×1080 est du
  ressort du projecteur.
- Masters et layouts gardent leurs items de décor (non-placeholders) et leurs
  placeholders (rôle, `idx`, frame) : le hi-fi en a besoin pour générer les
  widgets, le retrofit pour le `type` du layout.

## Lecteur PPTX

- Ordre : `[Content_Types].xml` → `presentation.xml` (+ rels) → `theme` →
  masters → layouts → slides → parts liées (images, charts, diagrams,
  notes).
- **EMU → pt** : 12 700 EMU par pt.
- **Cascade des styles texte**, une seule fonction :
  `master txStyles → layout placeholder → slide placeholder → lstStyle →
  pPr → rPr`, par niveau de paragraphe. Testée en table.
- **Placeholders** : un placeholder de slide hérite frame et style du
  placeholder de layout de même `type`/`idx`, lui-même de celui du master.
- **SmartArt** : importé depuis sa version déjà dessinée
  (`ppt/diagrams/drawingN.xml`) en `Group` de formes éditables.
- **`mc:AlternateContent`** (décision 11 du plan du lot 1) : la branche est
  choisie d'après `Requires` avant d'être lue (`Choice` si chaque préfixe
  désigne un espace de noms connu, sinon `Fallback`) ; seule l'équation
  (`oMath`) devient `Opaque { Math, fallback }`.
- **Graphiques** : `ppt/charts/chartN.xml`, valeurs lues dans
  `c:numCache`/`c:strCache` (le classeur embarqué n'est pas ouvert).
- **Tableaux** : `a:tbl`, fusions (`gridSpan`, `rowSpan`, `hMerge`,
  `vMerge`), style de tableau résolu (`tableStyles.xml`).
- **Hors v1** : animations, transitions ; vidéo/audio → `Opaque` avec
  l'image d'affiche ; effets (ombre, reflet, 3D) ignorés et signalés.

## `ImportPlan`

```rust
ImportPlan {
    theme: Option<PackFiles>,      // hi-fi : thème généré (`theme.json`)
    pack: Option<PackFiles>,       // hi-fi : pack généré (lot 3) ; retrofit : aucun au lot 2, pack enfant au lot 5
    schemas: Vec<WidgetSchema>,    // hi-fi : schémas déclarés, que la sonde doit retrouver
    assets, fonts,                 // fonts : fichiers de `fonts/` du deck
    ops: Vec<Op>,                  // ops `apply` existantes
    widget_requests: Vec<WidgetRequest>,
    report: ImportReport,
}
WidgetRequest { signature, slides: Vec<SlideId>, sample: Vec<Item>, fallback_applied }
```

L'appelant ouvre un deck neuf, installe thème/pack/assets/polices, puis
applique `ops` en **une seule transaction** : l'import passe par les mêmes
validations que l'agent et s'annule d'un coup.

## Projecteur hi-fi

Livré au lot 3 (décisions du plan du lot 3 ci-dessus).

**Thème généré** (`parent` : aucun ; celui du master de la première slide) :

| Source | Tokens |
|---|---|
| `clrScheme` | `color` : `lt1→bg`, `dk1→ink`, `dk2→ink-muted`, `lt2→surface`, `accent1→accent`, `accent2..6→accent-2..accent-6`, `hlink→link`, `folHlink→link-visited` ; `on-accent` (`bg` ou `ink` selon le contraste), `rule` ; `chart.palette` = `accent1..6` |
| `fontScheme`, polices | `font.display = major`, `font.body = minor`, par la famille retenue (`FontPlan::used`) ; défaut gardé si absente |
| placeholders du master et des layouts, sinon `txStyles` | `size.title`, `size.subtitle`, `size.body`, `size.small` (× échelle) ; `weight.body` 400, `weight.strong` 700 |
| puce du premier niveau du corps | `component.list.marker`, `marker-color` (sa couleur ou `null`) |
| premier tableau du deck (cellules résolues) | `component.table` : `header-bg`, `header-color`, `header-weight`, `body-bg` (nullable, ajouté à la base), `stripe-bg`, `size`, `rule-color`, `rule-width` ; chaque tableau pose `header-row`/`striped` lus sur ses cellules, perte `TableStyle` pour ce que le widget ne rend pas |
| `sldSz` | `page` : 1920 × 1080, marges nulles ; échelle et centrage portés par les cadres |

**Pack généré** :

- `_commun.typ` : `champ` et le décor de chaque master.
- **Un widget de slide par layout utilisé** (`l<rang>-<slug>`) : fond, décor
  du master (si le layout le montre) et du layout ; chaque placeholder de
  texte → `field(ctx, …)` à sa position exacte (`titre`, `sous-titre`,
  `corps`… en `rich` ; `date`, `pied`, `numero` en `plain`, `numero` à
  `{n}`) ; rien pour les placeholders d'image, graphique, tableau, média ;
  `grid` ; un slot `free` nommé `extras` sur toute la page ; `decor` (`flag`)
  si une slide masque le décor.
- Un template par layout dans `pack.json`, avec la première slide source
  qui l'utilise comme exemple.

**Générateur Typst** (déterministe) : un `Item` de décor → `place(dx, dy,
rect | ellipse | line | curve | image | text)` en pt page. `rect`,
`roundRect`, `ellipse`, `line` natifs ; tout autre preset (et `custGeom`)
converti en `curve` par les formules de géométrie de l'ECMA-376
(`presetShapeDefinitions.xml`, interprété). Rotation et miroirs dessinés ;
tableau, graphique, opaque sans repli → perte `Opaque`. Tout texte source
passe par un littéral de chaîne échappé.

**Deck** : chaque slide → `add_slide` avec `kind = <widget du layout>`
(`blank` sans layout lisible), placeholders de texte restés à leur boîte →
`params`, autres items → éléments (`text`, `shape` natif ou `path`,
`image`, `table`, `chart`) dans `extras`, cadres mis à l'échelle et
centrés ; fond propre d'abord. Couleur `Scheme` sans modificateur →
**référence de token** ; sinon hexadécimal. Slide masquée → perte
`HiddenSlide`.

## Projecteur retrofit

Entrées : `SourceDeck`, pack cible (templates), `WidgetRegistry` (sonde),
thème cible.

1. **Choix du template par score déterministe.** Pour chaque template :
   champs requis remplissables, slots compatibles avec les types de
   contenu, concordance du `type` de layout source (`title`, `secHead`,
   `obj`, `twoObj`, `titleOnly`, `blank`…). Égalité → ordre du pack.
   Avant le choix (lot 2) : le décor répété (en-tête, pied, logo dans les
   bandes haute et basse de la page, sur au moins la moitié des slides) et
   les placeholders date, pied, numéro sont omis, dits une fois au rapport ;
   sans placeholder de titre, titre, surtitre et sous-titre se déduisent de
   la taille et de la position des textes, et une couverture d'un titre
   d'au moins 40 pt ; une forme remplie qui porte des textes devient une
   carte (titre, surtitre, corps), et les textes voisins dont les cellules
   se chevauchent fusionnent. Une rangée de cartes fait des colonnes.
2. **Disposition.** Une slide « titre + un corps » ou une rangée de
   colonnes va en blocs `flow` (layout `columns`/`split` du modèle ; une
   rangée entre des textes pleine largeur devient une pile en colonnes).
   Sinon la boîte englobante des contenus est ajustée dans la zone de corps
   (échelle uniforme, centrée) : cadres sur un modèle libre, cellules de la
   grille du modèle si elles ne se recouvrent pas, sinon cadres.
3. **Couleurs.** `Scheme(accentN)` → table fixe vers les tokens cibles ;
   les couleurs saturées du deck sans jeton proche, rangées par usage,
   prennent `accent` puis les autres `accent-*` du thème ; `Explicit` →
   token le plus proche en **ΔE OKLab** sous un seuil, sinon abandonnée.
   Une couleur de texte n'est émise que lisible (contraste WCAG ≥ 3:1 sur
   son fond, page ou carte ; sur fond sombre, un jeton de texte clair du
   thème). Une carte sombre devient une carte en relief. Gras et italique
   conservés ; les capitales sont écrites en majuscules.
4. **Motifs sans widget.** Le repli (p. ex. `stack` de `card` pour un
   SmartArt) est appliqué tout de suite : le deck est **complet et
   rendable**. Si le repli perd de l'information (formes omises, motif
   régulier d'au moins trois boîtes, SmartArt), une `WidgetRequest` est
   émise, groupée par **signature de motif** (multiset des types d'items +
   disposition relative normalisée, `pattern.rs`). L'agent la traite au
   fine-tuning (décision 5), dans un pack enfant créé au lot 5.

**Limite connue du lot 2 (t-816).** En flux, un contenu plus haut que sa zone
déborde de la page sans que l'import le voie : un thème cible au corps plus
grand (Papier, slides 6 et 7 d'alto), ou, avec le thème par défaut, un
graphique ou une image qui remplit une colonne en flux `columns` (kitchen,
slide 2, sur Essentiel et Pro : « Gauche » et « Droite » sont coupés sous le
graphique). La couverture du contenu mesure le modèle de deck, pas le rendu :
elle ne le voit pas ; seul le rendu de référence (lot 4) le mesurera.

## Rendu de référence et métriques

- **Référence** (`reference.rs`) : `soffice --headless --convert-to pdf`,
  chemin et timeout en config ; PDF rasterisé par `hayro`. Absent ou en
  échec → avertissement au rapport, pas d'erreur.
- **Hi-fi — écart visuel** : rendu Oku (`Compiled::png`) et référence à la
  même résolution (960 px de large), ΔE OKLab moyen après léger flou ; plus
  un **score par région** à partir des `frame` des items, pour désigner
  l'élément fautif.
- **Retrofit — couverture du contenu** : rappel du texte (chaque run source
  retrouvé), présence des images, égalité des données de tableaux et
  graphiques. Cible 100 % ; tout manque est listé.

## Rapport d'import

`ImportReport` (JSON) : par slide, score (écart ou couverture), replis
appliqués, `Opaque`, effets ignorés ; globalement, polices substituées
(`fonts` : une décision par famille, lot 3), référence disponible ou non,
`widget_requests`. Messages via Fluent
(`oku-slides-i18n`).

## Fine-tuning par agent (app)

- **Outils MCP nouveaux** (`oku-slides-mcp`) : `import_report` (le
  rapport courant) et `import_diff(slide)` (PNG de référence, PNG Oku,
  scores par région).
- **Boucle autonome bornée** : slides de la pire à la meilleure ;
  corrections par `apply` / `set_widget` ; arrêt sous le seuil ou après N
  itérations par slide (seuil et N en config).
- **Retrofit** : chaque `WidgetRequest` → widget créé dans le pack enfant
  (`set_widget`, `preview_widget`), puis repli remplacé sur toutes les
  slides du motif, sans question.
- **Hi-fi** : l'agent propose les regroupements (« N slides partagent ce
  motif → widget d'élément ? ») par `ask_confirm`.
- **Fin** : récapitulatif par `report` (changé, encore au-dessus du seuil,
  figé). Tout passe par la session, donc annulable.
- Les **consignes de fine-tuning** sont un texte versionné livré avec
  l'app, pas codé en dur.

## Points d'entrée

- **CLI** (`oku-slides-cli`), purement déterministe :
  `oku-slides import deck.pptx --mode hifi|retrofit [--pack <id|chemin|nom>]
  [--theme <id|nom>] [--reference auto|none] -o out.oku-slides.pdf
  [--report r.json] [--no-system-fonts]`. `--mode` vaut `retrofit` par
  défaut ; `--pack` y est requis. `--mode hifi` est livré au lot 3 :
  `--pack` et `--theme` y sont refusés (le pack et le thème sont générés,
  embarqués dans le deck seulement) ; les polices de la présentation sont
  cherchées parmi celles de l'app (`--fonts`) et, sauf `--no-system-fonts`,
  du système. `--reference` arrive au lot 4.
- **App** : « Ouvrir » accepte `.pptx` ; un dialogue demande le mode, et en
  retrofit le pack et le thème cibles. L'app crée un deck neuf à partir du
  plan, puis lance l'agent avec les consignes de fine-tuning.

## Erreurs

`ImportError` Snafu opaque, **seulement pour le fatal** : zip invalide,
`presentation.xml` absent, fichier chiffré (ou `.ppt` 97-2003), limites de
lecture dépassées (`ReadLimits` : protection contre les bombes zip ;
dépasser `max_xml_nodes` est `TooLarge`, fatal, jamais une part omise), pack
cible introuvable. Un `TooLarge` levé en lisant une part quelconque (slide, notes,
graphique, SmartArt, polices, styles de tableau) est propagé comme erreur
fatale (`PartCtx::fail` / `soft`), jamais dégradé en part omise avec
`UnreadablePart`. À l'inverse, une part XML imbriquée sur plus de 256
niveaux n'est pas une limite de `ReadLimits` : elle est refusée comme
illisible (`Unreadable`) et traitée comme toute part illisible — omise avec
`UnreadablePart` (item lié : `Opaque` + `UnreadableItem`), fatale seulement
pour une part de structure (`[Content_Types].xml`, relations racine,
`presentation.xml`) ou faute de master utilisable. Un item
illisible ne fait jamais échouer l'import : il devient `Opaque`, avec une
entrée au rapport.

## Tests

- **Fixtures** : OOXML décompressé écrit à la main
  (`fixtures/import/<nom>/`, zippé à la volée par les tests — lisible et
  relu en diff), plus `libreoffice.pptx`, le même deck réécrit par
  LibreOffice (verbeux, réaliste). *(Précisé au plan du lot 1 :
  `python-pptx` n'est pas disponible et des XML écrits à la main se
  relisent mieux.)*
- **Vrai deck** : `fixtures/import/alto-wealth.pptx` (généré par PptxGenJS)
  et son PDF de référence `alto-wealth.pdf`, cible visuelle des lots 4 et 6.
- **Snapshots `insta`** de `SourceDeck` et d'`ImportPlan` par fixture.
- **Cascade des styles** testée en table (`rstest`).
- **Or hi-fi** : PDF de référence **versionnés** (pas de LibreOffice en
  CI) ; écart visuel sous le seuil.
- **Retrofit** : couverture du contenu à 100 % sur les fixtures.
- **CLI** : test d'intégration bout en bout (pptx → `.oku-slides.pdf`
  rouvrable).

## Lots

Un plan et ses sous-tâches par lot, dans cet ordre :

1. *(livré, t-692)* `SourceDeck` + lecteur PPTX (cascade des styles comprise) — plan :
   `docs/superpowers/plans/2026-10-09-import-pptx-lot1.md`.
2. *(livré, t-692)* Projecteur retrofit + CLI + couverture du contenu —
   valide la représentation de bout en bout sans générateur ni référence ;
   plan : `docs/superpowers/plans/2026-10-10-import-pptx-lot2.md`.
3. *(livré, t-692)* Projecteur hi-fi : thème, générateur Typst, pack,
   polices (`fonts/`), base étendue (chemins, ancrage du texte) ; plan :
   `docs/superpowers/plans/2026-10-11-import-pptx-lot3.md`.
4. Rendu de référence + écart visuel.
5. App + fine-tuning par agent (outils MCP, consignes, pack enfant des
   widgets créés pour les `WidgetRequest`).
6. *(Phase 2, spec séparée)* Lecteur PDF → `SourceDeck` par
   `hayro-interpret`, avec inférence des indices sémantiques (rôles,
   master par éléments répétés, paragraphes).
