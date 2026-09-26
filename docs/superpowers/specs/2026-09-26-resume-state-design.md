# Reprise de l'état de l'application — design

Date : 2026-09-26

## Objectif

Quitter puis relancer l'app ramène exactement au même état : deck ouvert
(même non enregistré), historique annuler / rétablir, slide courante, thème
(porté par le deck), mode édition de modèles, préférence de grille, taille et
position de la fenêtre. L'état est enregistré à chaque modification.

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
- `oku-slides-core` : `History` et `Transaction` dérivent `Serialize` /
  `Deserialize` (`Op` le fait déjà).

## Fichiers

Sous `<data>/resume/` (`<data>` = `settings.paths.data()`), écrits via
`Storage::write_atomic` :

```
resume/
  state.json        { version, path, dirty, saved_hash, notice?, mode, ui: { slide, grid } }
  deck/             le document du deck (ou le deck mis de côté pendant l'édition des modèles)
    deck.json
    history.json    { undo, redo } — la limite vient des réglages
    assets/<hex>    écrit s'il n'existe pas ; orphelins supprimés
    fonts/<nom>     idem
  templates/        présent seulement en édition de modèles : même forme
```

- `mode` : `"deck"` ou `{ "templates": { set, name, builtin } }`.
- `saved_hash` : sha256 du fichier `path` au moment de l'ouverture ou du
  dernier enregistrement.
- `version` : `1`. Une version inconnue est traitée comme un état illisible.

## Restauration (démarrage de l'app)

1. Lire `state.json`, puis `deck/` (et `templates/` si le mode l'indique).
2. Document non modifié, `path` présent et hash du fichier différent de
   `saved_hash` (ou fichier absent de la copie de travail) : rouvrir depuis le
   disque, historique vide.
3. Sinon : deck + historique + `dirty` + `path` depuis la copie de travail.
4. Historique illisible : deck gardé, historique vide. État ou deck
   illisible : journalisé, deck vierge.
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

- Core : aller-retour serde de `History` ; undo / redo identiques ensuite.
- `resume_tests` : aller-retour complet (deck, historique, dirty, chemin,
  slide, grille) ; mode édition de modèles ; fichier modifié ailleurs →
  relu depuis le disque ; état corrompu → deck vierge ; historique corrompu →
  deck gardé ; assets orphelins supprimés ; argument ignoré si travail non
  enregistré.
- Intégration `tests/session.rs` : session A modifie puis est détruite ;
  session B sur le même dossier retrouve le même snapshot, et `undo`
  fonctionne.
- UI (`bun test`) : initialisation depuis `snap.ui`.
