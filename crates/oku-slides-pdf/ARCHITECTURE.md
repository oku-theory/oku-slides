# oku-slides-pdf

Le blob des sources Oku Slides (zip, contenant le replica Loro du deck)
rangé dans un PDF, dans son dictionnaire de pièce de page
(`/PieceInfo/OkuSlides/Private`, ISO 32000 §14.5). N'apparaît pas dans le
panneau des pièces jointes des lecteurs PDF.

## API

- `embed_sources(pdf, zip)` : écrit ou remplace le blob (`/Format`,
  `/Version 2`, `/SHA256`, `/LastModified`).
- `extract_sources(pdf) -> Option<Sources>` : lit le blob. `None` si absent
  **ou** d'une version antérieure à 2 (une enveloppe `/Version 1`,
  `deck.json`, est ignorée comme si elle n'existait pas — pas de
  conversion). `PdfErrorKind::UnsupportedVersion` pour une version plus
  récente que celle que ce binaire connaît.
- `strip_sources(pdf)` : retire le blob (export « propre » d'un PDF sans
  historique).
- `Sources::modified_elsewhere()` : le PDF sur disque diffère du hash connu
  à l'ouverture (déclenche la fusion à l'enregistrement, côté session).

## Format du zip (version 9)

```
manifest.json          { format: "oku-slides-sources", version: 9 }
deck.loro              snapshot complet du replica (oku-slides-core::DeckReplica)
assets/<hex>           tels quels
fonts/<nom>            tels quels
packs/<hex>/pack.json  version de pack de modèles (hex = sha-256 du contenu canonique)
packs/<hex>/widgets/<nom>.typ  un par fichier de widget (_commun compris)
```

Le contenu du zip n'est pas interprété ici : `oku-slides-core::SourceBundle`
le construit et le relit. Ce crate ne voit qu'un flux d'octets à poser dans
le PDF ou à en extraire.

## Points d'extension

- Un nouveau format de blob : incrémenter `/Version`, garder l'ancienne
  branche de lecture (`Ok(None)` ou `UnsupportedVersion` selon le sens),
  jamais de migration en place.

## Dépendances

`lopdf` (structure PDF bas niveau), `oku-slides-i18n` (messages Fluent),
`sha2`/`hex` (intégrité), `time` (horodatage).
