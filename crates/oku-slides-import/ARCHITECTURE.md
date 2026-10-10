# oku-slides-import

Import d'une présentation PowerPoint (`.pptx`) en représentation
intermédiaire, `source::SourceDeck` : neutre vis-à-vis du format, résolue,
chaque valeur stylée avec sa provenance. Spec :
`docs/superpowers/specs/2026-10-09-import-pptx-design.md`.

## API

- `read_pptx(bytes, &ReadLimits) -> Result<SourceDeck, ImportError>` : fatal
  seulement pour un paquet illisible, pas une présentation, protégé, sans
  master utilisable, ou une limite dépassée.
- `source` : les types de la représentation, sans ré-export à plat.

## Couches

- `source/` ne dépend de rien ; `error.rs`, `limits.rs` (`ReadLimits` :
  octets, parts, nœuds XML, slides, avec `Default`) et `collect.rs` (assets,
  avertissements, fatal différé, assainissement) sont neutres vis-à-vis du format.
- `pptx/` (et demain `pdf/`) dépend de `source/`, `error.rs`, `collect.rs`.
  Dans `pptx/`, `shapes` → `frame` → `smartart` → `shapes` forment un cycle
  borné par `GroupCtx.depth` et `in_drawing`.
- Projecteurs (lots 2-3) : `source/` + `oku-slides-core`, jamais `pptx/`.
  `oku-slides-core` ne dépend jamais de cette crate.

## Modules de `pptx/`

- `package` (zip borné, relations en cache), `xml` (analyse bornée en nœuds
  et en profondeur ; `max_xml_nodes` dépassé = `TooLarge`, fatal).
- `context` (contexte d'une part), `placeholder`, `cascade` (seule résolution
  des styles texte), `color`/`fill`/`geometry`/`units`, `theme`, `text`.
- `master`, `slide`, `shapes`, `frame` (tableau, graphique, SmartArt, OLE) ;
  `table`, `chart`, `smartart`, `fonts` + `eot` (polices embarquées).
- `test_support` : seul endroit des paquets et contextes de test.

## Points d'extension

- Nouveau format source (PDF, lot 6) : un module frère de `pptx/` qui produit
  la même `SourceDeck` et réutilise `collect::Sink`.

## Dépendances

`zip`, `roxmltree`, `oku-slides-core` (`asset_id`), `oku-slides-i18n`.
