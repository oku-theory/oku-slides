# Reprise de l'état de l'application — design

Date : 2026-09-26

## Objectif

Quitter puis relancer l'app ramène exactement au même état : deck ouvert
(même non enregistré), slide courante, thème (porté par le deck), mode
édition de modèles, préférence de grille, taille et position de la fenêtre.
L'état est enregistré à chaque modification.

> Mise à jour (t-188, [ADR-0002](../../adr/0002-sources-du-deck-en-crdt-loro.md)) :
> la pile d'annulation ne fait plus partie de ce qui revient au relancement.
> Le deck redevient le replica Loro (`deck.loro`) ; son `UndoManager` est
> reconstruit vide à chaque ouverture, comme à l'ouverture d'un PDF. Les
> sections ci-dessous décrivent encore l'historique JSON d'origine (v1) là
> où le reste n'a pas changé ; le format des fichiers de `deck/` est celui
> du replica (voir ci-dessous).

Hors périmètre : préférences de dictée (déjà dans `<models>/config.json`),
langue (`config.toml`, lecture seule).

## Architecture

- Nouveau module `crates/oku-slides-session/src/resume.rs` (tests :
  `resume_tests.rs`). La persistance est optionnelle : seule l'app Tauri
  l'active, via `Session::resume_from(dir)`. Le serveur de développement et
  les tests existants ne changent pas.
- `Session::handle` enregistre l'état après chaque commande réussie
  (`New`, `Open`, `Save`, `Apply`, `ChartKind`, `Undo`, `Redo`,
  `EditTemplates`, `SaveTemplates`, `CloseTemplates`, `Ui`). Une écriture
  ratée est journalisée (`warn`) ; la requête ne l'est pas.
- ~~`oku-slides-core` : `History` et `Transaction` dérivent `Serialize` /
  `Deserialize`~~ — obsolète depuis t-188 : `History` a disparu, remplacé
  par le replica Loro (`DeckReplica`), sérialisé en snapshot binaire
  (`deck.loro`), pas en JSON.

## Fichiers

Sous `<data>/resume/` (`<data>` = `settings.paths.data()`), écrits via
`Storage::write_atomic` :

```
resume/
  state.json        { version, mode, ui: { slide, grid } }
  deck/             le document du deck (ou le deck mis de côté pendant l'édition des modèles)
    deck.loro       snapshot complet du replica (oku-slides-core::DeckReplica), écriture atomique
    document.json   { path, dirty, saved_hash, blank } — écrit après deck.loro
    assets/<hex>    écrit s'il n'existe pas ; orphelins supprimés
    fonts/<nom>     idem
  templates/        même forme ; lu seulement quand `mode` est l'édition de modèles
```

> Mise à jour (t-188) : `deck/` ne contient plus le deck ni son historique
> en JSON dans `document.json` ; le deck redevient une vue du replica lu
> depuis `deck.loro`. L'ordre d'écriture (`deck.loro` puis `document.json`)
> laisse une fenêtre de crash : un `deck.loro` neuf peut se retrouver à côté
> d'un `document.json` d'avant (ancien `dirty` / `saved_hash`), l'ancien
> format n'écrivant qu'un seul fichier. Conséquence mineure : au pire, un
> `dirty` ou un `saved_hash` périmé d'une écriture ; le deck lui-même reste
> cohérent (`deck.loro` est toujours écrit en entier).

- `mode` : `"deck"` ou `{ "templates": { set, name, builtin } }`.
- `saved_hash` : sha256 (`asset_id`) du fichier `path` au moment de
  l'ouverture ou du dernier enregistrement.
- `version` : `1`. Une version inconnue est traitée comme un état illisible.
- Les avis (`notice`) ne sont pas persistés : ils ne s'affichent qu'une fois.
- Écriture : documents d'abord (`deck.loro` puis `document.json`),
  `state.json` ensuite. Une requête `Ui` n'écrit que `state.json`.

## Restauration (démarrage de l'app)

1. Lire `state.json`, puis `deck/` (et `templates/` si le mode l'indique).
2. Document non modifié, `path` présent et hash du fichier différent de
   `saved_hash` : rouvrir depuis le disque. Fichier absent ou illisible : la
   copie de travail est gardée.
3. Sinon : deck (replica relu depuis `deck.loro`) + `dirty` + `path` depuis
   la copie de travail. La pile d'annulation ne fait pas partie de la copie
   de travail : elle repart toujours de zéro (t-188, ADR-0002).
4. Document des modèles illisible : retour au deck. État, `deck.loro` ou
   `document.json` illisible : journalisé, deck vierge.
5. Fichier passé en argument (« ouvrir avec… ») : il est ouvert, **sauf** si
   l'état restauré contient des modifications non enregistrées. Dans ce cas le
   travail restauré est gardé et un avis (`notice`) le signale.

## Interface ↔ session

- Nouvelle requête `Request::Ui { slide: Option<String>, grid: bool }` :
  l'UI l'envoie quand la slide courante ou la grille change. Pas
  d'historique, pas de rendu ; renvoie le snapshot.
- `Snapshot` gagne `ui: { slide, grid }`. Au montage, `App.tsx` initialise
  `current` et `gridAlways` depuis ce champ.
- `localStorage["oku-slides.grid"]` est supprimé (pas de migration).
- Sans reprise (serveur de dev), `Ui` garde l'état en mémoire seulement.

## Fenêtre

`tauri-plugin-window-state` (taille, position, maximisé). Enregistrement sur
`Moved` / `Resized` en plus de la fermeture.

## Tests (TDD)

- Core : aller-retour du replica (`deck.loro`), voir
  `crates/oku-slides-core/src/replica/`. ~~aller-retour serde de
  `History` ; undo / redo identiques ensuite~~ (obsolète, `History` a
  disparu en t-188).
- `resume_tests` : aller-retour complet (deck, dirty, chemin, slide,
  grille) ; mode édition de modèles ; fichier modifié ailleurs → relu
  depuis le disque ; état corrompu → deck vierge ; `deck.loro` corrompu →
  deck vierge ; assets orphelins supprimés ; argument ignoré si travail non
  enregistré.
- Intégration `tests/session.rs` : session A modifie puis est détruite ;
  session B sur le même dossier retrouve le même snapshot ; `undo` n'y
  trouve rien à défaire (pile reconstruite vide, t-188).
- UI (`bun test`) : initialisation depuis `snap.ui`.
