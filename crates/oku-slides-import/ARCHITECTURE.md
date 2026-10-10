# oku-slides-import

Import d'une présentation PowerPoint (`.pptx`) : lecture en représentation
intermédiaire, `source::SourceDeck` (neutre vis-à-vis du format, résolue,
chaque valeur stylée avec sa provenance), puis projection en un plan d'ops
Oku. Spec : `docs/superpowers/specs/2026-10-09-import-pptx-design.md`.

## API

- `read_pptx(bytes, &ReadLimits) -> Result<SourceDeck, ImportError>` : fatal
  seulement pour un paquet illisible, pas une présentation, protégé, sans
  master utilisable, ou une limite dépassée.
- `source` : les types de la représentation, sans ré-export à plat.
- `retrofit::project(&SourceDeck, &RetrofitTarget) -> ImportPlan` : le
  contenu projeté sur un pack et un thème existants. `RetrofitTarget` :
  modèles du pack (`oku_slides_core::Template`), registre des widgets,
  `ColorTokens` et `PageTokens` tirés des jetons effectifs du thème.
- `plan` : `ImportPlan { ops, assets, widget_requests, report }`,
  `WidgetRequest` ; `ImportPlan::report_json(l)` (forme de `--report`).
- `report` : `ImportReport`, `SlideReport`, `Loss`/`LossKind` (localisés,
  `import-loss-*`), avertissements du lecteur localisés (`import-warning-*`),
  `Coverage`, `Ratio`, `Missing`.
- `coverage::coverage(&SourceDeck, &Deck, &WidgetRegistry, &origins, deck_slide: Option<u32>) -> Coverage` :
  ce que le deck obtenu garde de la source (runs à leurs frontières de mots
  dans l'élément projeté depuis leur item) ; `project` la remplit.
  `ImportReport::coverage == None` : le cœur a refusé le plan (un bogue,
  `debug_assert`), la couverture n'a pas pu être mesurée.
- Ré-exports à la racine : `ImportPlan`, `ImportReport`, `RetrofitTarget`,
  `ImportError`, `ImportErrorKind`, `ReadLimits`, `read_pptx`.

## Couches

- `source/` ne dépend de rien ; `error.rs`, `limits.rs` (`ReadLimits` :
  octets, parts, nœuds XML, slides, avec `Default`) et `collect.rs` (assets,
  avertissements, fatal différé, assainissement) sont neutres vis-à-vis du format.
- `pptx/` (et demain `pdf/`) dépend de `source/`, `error.rs`, `collect.rs`.
  Dans `pptx/`, `shapes` → `frame` → `smartart` → `shapes` forment un cycle
  borné par `GroupCtx.depth` et `in_drawing`.
- Projecteurs (`retrofit/`, au lot 3 `hifi/`) : `source/`, `plan`, `report`,
  `pattern`, `coverage`, `oku-slides-core`, `oku-slides-chart` ; jamais `pptx/`.
  Ils sont purs : ni aléa, ni horloge, ni fichier, ni session. `oku-slides-core`
  ne dépend jamais de cette crate ; elle ne dépend ni de la session, ni du rendu,
  ni de la CLI (qui l'appelle : `oku-slides import`).

## Projecteur retrofit (`retrofit/`)

- `mod.rs` : `project` — accents déduits du deck, décor omis, profils des
  modèles, une slide source après l'autre, assets référencés, demandes de
  widget groupées par signature, couverture mesurée sur les ops appliquées à
  un deck vide.
- `decor.rs` : ce que la règle omet (placeholders date, pied, numéro ;
  en-tête et pied répétés dans les bandes de la page) — un seul prédicat,
  partagé avec `coverage`.
- `slide.rs` : une slide → `AddSlide` (`s<numéro>`, éléments `s<numéro>-<rang>`),
  `SetDeckParam` (première couverture), `SetNotes` ; placement `flow` (blocs,
  agencement du modèle, rangée en pile) ou libre (boîte englobante ajustée
  dans la zone : cadres, ou cellules de la grille si elles ne se recouvrent pas).
- `heading.rs` : titres d'une slide, des placeholders ou déduits (taille,
  position) ; couverture déduite.
- `cluster.rs` : unités de contenu — cartes (forme remplie et textes posés
  dessus), textes fusionnés, items seuls.
- `template.rs` : profil d'un modèle (sa slide d'exemple et le schéma de son
  widget ; `Profile::has_slot`), forme d'une slide source (unités, rangée de colonnes), score et
  choix (à égalité, l'ordre du pack ; `slide::pick` ne prend pas le registre).
- `content.rs` : unité → élément (`text`, `card`, `shape`, filet `line`,
  `image`, `table`, `chart`), pertes par item source ; `group.rs` : repli
  `stack` des groupes et SmartArt dessinés, dit seulement s'il perd quelque chose.
- `text.rs` (riche, plain, bornes du cœur, couleurs lisibles sur leur fond),
  `table.rs`, `chart.rs` (options canoniques, sans couleur), `color.rs`
  (OKLab, ΔE, table du schéma → jetons, accents déduits, contraste WCAG ;
  seules des clés existantes du thème sont émises), `zone.rs` (zone de corps
  approchée par les jetons de page, ajustement `Fit`, `snap` porté de `ui/src/grid.ts`).
- `pattern.rs` (racine) : signature d'un motif, pour grouper les demandes.

Limite connue (t-816) : le débordement en flux. Un contenu plus haut que sa zone
sort de la page (Papier, slides 6 et 7 d'alto ; thème par défaut, graphique ou
image qui remplit une colonne en flux `columns`, kitchen slide 2 sur Essentiel
et Pro). La couverture mesure le `Deck`, pas le rendu : elle ne le voit pas.

## Modules de `pptx/`

- `package` (zip borné, relations en cache), `xml` (analyse bornée en nœuds
  et en profondeur ; `max_xml_nodes` dépassé = `TooLarge`, fatal).
- `context` (contexte d'une part), `placeholder`, `cascade` (seule résolution
  des styles texte), `color`/`fill`/`geometry`/`units`, `theme`, `text`.
- `master`, `slide`, `shapes`, `frame` (tableau, graphique, SmartArt, OLE) ;
  `table`, `chart`, `smartart`, `fonts` + `eot` (polices embarquées).
- `test_support` : seul endroit des paquets et contextes de test du lecteur ;
  `src/test_support.rs` : celles des projecteurs (registre de base, modèles et
  jetons des packs et thèmes intégrés, slides source) ; `tests/common` : celles
  des tests d'intégration.

## Points d'extension

- Nouveau format source (PDF, lot 6) : un module frère de `pptx/` qui produit
  la même `SourceDeck` et réutilise `collect::Sink`.
- Nouveau projecteur (hi-fi, lot 3) : un module frère de `retrofit/` qui rend
  un `ImportPlan` ; `Mode` gagne sa variante.
- Nouvelle perte : une variante de `LossKind`, sa clé `import-loss-*` dans les
  20 `document.ftl` (`message_id`, `Localize for Loss`).

## Dépendances

`zip`, `roxmltree`, `oku-slides-core` (`asset_id`, ops, modèle, `Template`),
`oku-slides-chart` (bornes des tableaux), `oku-slides-i18n`, `serde`,
`serde_json`, `snafu`.
