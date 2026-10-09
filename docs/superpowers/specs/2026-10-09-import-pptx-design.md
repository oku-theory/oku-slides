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

Défauts retenus sans question :

- **Taille de page.** Hi-fi : les tokens `page` du thème généré prennent le
  ratio source (4:3, perso…). Retrofit : la page du thème cible, la
  projection sur la grille absorbe l'écart de ratio.
- **Polices.** Ordre de recherche : polices embarquées dans le PPTX →
  polices système → substitut métriquement compatible (Carlito ↔ Calibri,
  Caladea ↔ Cambria, Liberation ↔ Arial/Times/Courier), signalé au rapport.
  La police retenue est embarquée dans `fonts/` du deck.

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
| `pptx/cascade.rs` | résolution des styles texte (une seule fonction) |
| `pptx/geometry.rs` | presets `prstGeom` → path, EMU → pt |
| `pptx/fonts.rs` | inventaire des familles utilisées, polices embarquées (lot 1) |
| `fonts.rs` | recherche système et substitution des polices (lot 2) |
| `hifi/` | projecteur hi-fi : thème, générateur Typst, pack, ops |
| `retrofit/` | projecteur retrofit : score des templates, grille, couleurs, replis |
| `pattern.rs` | signature de motif (partagée retrofit / regroupement hi-fi) |
| `plan.rs` | `ImportPlan`, `WidgetRequest` |
| `reference.rs` | appel `soffice`, rasterisation `hayro` |
| `metrics.rs` | écart visuel (hi-fi), couverture du contenu (retrofit) |
| `report.rs` | `ImportReport` |

Dépendances : `zip` (existant), `roxmltree` (nouveau, DOM lecture seule),
`hayro` (déjà transitif, devient direct pour `reference.rs`). La crate ne
dépend ni de la session ni de l'UI : elle prend en entrée le registre de
widgets, le pack et le thème cibles, et rend un plan.

## Représentation intermédiaire : `SourceDeck`

**Résolue** (plus aucun héritage à calculer), mais chaque valeur stylée
garde **sa provenance**.

```rust
SourceDeck { size, theme: SourceTheme, masters, layouts, slides, assets, fonts }
SourceTheme { colors: {dk1, lt1, dk2, lt2, accent1..6, hlink, folHlink}, fonts: {major, minor} }
SourceSlide { id, layout: Option<LayoutId>, background: Fill, items: Vec<Item>, notes }
Item { id, frame: {x, y, w, h, rot}, z, role: Option<Role>, kind: ItemKind }
Role = Title | Subtitle | Body | Picture | Footer | SlideNum | Date
ItemKind =
    Text(TextBox { paragraphs, insets, anchor, autofit })
  | Shape { geom: Preset(name, adj) | Path, fill, stroke, text: Option<TextBox> }
  | Picture { asset, crop }
  | Table { col_widths, rows, cells (fusions comprises), styles résolus }
  | Chart { kind, categories, series }
  | Group(Vec<Item>)
  | Opaque { reason, fallback: Option<AssetId> }
Styled<T> { value: T, origin: Scheme(name) | Explicit | Default }
```

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
- **`mc:AlternateContent`** : `mc:Choice` si on sait le lire, sinon
  `mc:Fallback` (image) → `Opaque { fallback }`.
- **Graphiques** : `ppt/charts/chartN.xml`, valeurs lues dans
  `c:numCache`/`c:strCache` (le classeur embarqué n'est pas ouvert).
- **Tableaux** : `a:tbl`, fusions (`gridSpan`, `rowSpan`, `hMerge`,
  `vMerge`), style de tableau résolu (`tableStyles.xml`).
- **Hors v1** : animations, transitions ; vidéo/audio → `Opaque` avec
  l'image d'affiche ; effets (ombre, reflet, 3D) ignorés et signalés.

## `ImportPlan`

```rust
ImportPlan {
    theme: Option<Theme>,          // hi-fi : thème généré
    pack: Option<Pack>,            // hi-fi : pack généré ; retrofit : pack enfant vide si requests
    assets, fonts,
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

**Thème généré** (`parent` : aucun) :

| Source | Tokens |
|---|---|
| `clrScheme` | `color` : `lt1→bg`, `dk1→ink`, `dk2→ink-muted`, `lt2→surface`, `accent1→accent`, `accent2..6` et `hlink` en clés perso ; `chart.palette` = `accent1..6` |
| `fontScheme` | `font.display = major`, `font.body = minor` |
| `txStyles` du master | `size`/`weight`/`leading` : titre → `size.title`, niveaux de corps → `size.body`… |
| `sldSz` | `page` (ratio source) |

**Pack généré** :

- `_commun.typ` dessine le décor du master.
- **Un widget de slide par layout utilisé** : décor du master + du layout ;
  chaque placeholder → `field(ctx, …)` à sa position exacte (`rich`, ou
  `plain` pour le pied de page) ; un slot `free` nommé `extras` pour les
  éléments propres à la slide.
- Un template par layout dans `pack.json`, avec la première slide source
  qui l'utilise comme exemple.

**Générateur Typst** (déterministe) : un `Item` de décor → `place(dx, dy,
rect | ellipse | path | image | text)`. Table de correspondance pour la
vingtaine de presets courants ; les autres convertis en `path` par les
formules de géométrie de l'ECMA-376 (`presetShapeDefinitions.xml`).

**Deck** : chaque slide → `add_slide` avec `kind = <widget du layout>`,
placeholders → `params`, autres items → `add_element` (`text`, `shape`,
`image`, `table`, `chart`) dans `extras`, `frame` ×(1920 / largeur
source). Couleur `origin = Scheme` → **référence de token** ; `Explicit` →
hexadécimal.

## Projecteur retrofit

Entrées : `SourceDeck`, pack cible (templates), `WidgetRegistry` (sonde),
thème cible.

1. **Choix du template par score déterministe.** Pour chaque template :
   champs requis remplissables, slots compatibles avec les types de
   contenu, concordance du `type` de layout source (`title`, `secHead`,
   `obj`, `twoObj`, `titleOnly`, `blank`…). Égalité → ordre du pack.
2. **Disposition.** Boîtes source projetées sur la grille 12×6 →
   cellules, ou layout `columns`/`rows` de `content`.
3. **Couleurs.** `Scheme(accentN)` → table fixe vers les tokens cibles ;
   `Explicit` → token le plus proche en **ΔE OKLab** sous un seuil, sinon
   abandonnée. Gras et italique conservés.
4. **Motifs sans widget.** Le repli (p. ex. `stack` de `card` pour un
   SmartArt) est appliqué tout de suite : le deck est **complet et
   rendable**. Si le repli perd de l'information, une `WidgetRequest` est
   émise, groupée par **signature de motif** (multiset des types d'items +
   disposition relative normalisée, `pattern.rs`). L'agent la traite au
   fine-tuning (décision 5).

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
appliqués, `Opaque`, effets ignorés ; globalement, polices substituées,
référence disponible ou non, `widget_requests`. Messages via Fluent
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
  `oku-slides import deck.pptx --mode hifi|retrofit [--pack <id|chemin>]
  [--theme <id>] [--reference auto|none] -o out.oku-slides.pdf
  [--report r.json]`. `--pack` est requis en retrofit.
- **App** : « Ouvrir » accepte `.pptx` ; un dialogue demande le mode, et en
  retrofit le pack et le thème cibles. L'app crée un deck neuf à partir du
  plan, puis lance l'agent avec les consignes de fine-tuning.

## Erreurs

`ImportError` Snafu opaque, **seulement pour le fatal** : zip invalide,
`presentation.xml` absent, fichier chiffré (ou `.ppt` 97-2003), limites de
lecture dépassées (`ReadLimits` : protection contre les bombes zip), pack
cible introuvable. Un item
illisible ne fait jamais échouer l'import : il devient `Opaque`, avec une
entrée au rapport.

## Tests

- **Fixtures** : OOXML décompressé écrit à la main
  (`fixtures/import/<nom>/`, zippé à la volée par les tests — lisible et
  relu en diff), plus `libreoffice.pptx`, le même deck réécrit par
  LibreOffice (verbeux, réaliste). *(Précisé au plan du lot 1 :
  `python-pptx` n'est pas disponible et des XML écrits à la main se
  relisent mieux.)*
- **Snapshots `insta`** de `SourceDeck` et d'`ImportPlan` par fixture.
- **Cascade des styles** testée en table (`rstest`).
- **Or hi-fi** : PDF de référence **versionnés** (pas de LibreOffice en
  CI) ; écart visuel sous le seuil.
- **Retrofit** : couverture du contenu à 100 % sur les fixtures.
- **CLI** : test d'intégration bout en bout (pptx → `.oku-slides.pdf`
  rouvrable).

## Lots

Un plan et ses sous-tâches par lot, dans cet ordre :

1. `SourceDeck` + lecteur PPTX (cascade des styles comprise) — plan :
   `docs/superpowers/plans/2026-10-09-import-pptx-lot1.md`.
2. Projecteur retrofit + CLI — valide la représentation de bout en bout
   sans générateur ni référence.
3. Projecteur hi-fi : thème, générateur Typst, pack.
4. Rendu de référence + métriques.
5. App + fine-tuning par agent (outils MCP, consignes).
6. *(Phase 2, spec séparée)* Lecteur PDF → `SourceDeck` par
   `hayro-interpret`, avec inférence des indices sémantiques (rôles,
   master par éléments répétés, paragraphes).
