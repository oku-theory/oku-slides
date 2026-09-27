# oku-slides-session

Session d'édition d'un deck, et espace de travail : fenêtres, onglets, une
session par onglet, reprise au relancement. L'app Tauri (`src-tauri`) et le
serveur de développement (`oku-slides-cli`, `serve`) exposent la même API.

## Types clés

- `Session` + `Request` (`lib.rs`) : un deck, son rendu Typst, son historique, son
  mode modèles, sa reprise (`resume.rs`, écrite par un fil dédié : `resume_writer.rs`).
- `Workspace` + `WorkspaceRequest` / `WorkspaceView` (`workspace.rs`) : fenêtres `w-<n>`,
  onglets `t-<n>`, onglet/fenêtre actifs ; `TabInfo` (`views.rs`) : ce que montre la barre.
- `Slot` (`workspace_slot.rs`) : session d'un onglet, `Ready`, `Dormant` ou `Closed`.
- `workspace_resume.rs` : relecture et écriture de `workspace.json`, orphelins, `open_files`.
- `paragraphs_to_text` (`text.rs`, inline dans `inline.rs`) : markdown d'édition
  relu par `@lezer/markdown` ; contrat `fixtures/markdown.json` et `-corpus.json` (`UPDATE_CORPUS=1`).

## Verrous

`Mutex<Workspace>` pour la structure, `Arc<Mutex<Slot>>` par onglet.

- Ordre : **l'espace de travail, puis une session, jamais l'inverse.**
- Jamais d'attente sur une session sous le verrou de l'espace de travail :
  `try_lock` seulement (fermeture, remplacement d'un onglet vierge).
- Travail lent hors du verrou de l'espace de travail : session neuve construite puis
  insérée, onglet endormi relu sous son seul verrou, dossiers des onglets fermés supprimés après.
- Seules `Workspace::apply`, `request`, `open_files` et `settle` combinent
  les deux verrous. Un verrou empoisonné est repris (`relock`).

## Disposition de `<data>/resume/`

```
workspace.json   structure (version 2), compteurs, `kept`, `info` par onglet
tabs/t-<n>/      state.json, deck/, templates/   (écrit par sa session seule)
```

- Création : dossier de l'onglet d'abord, `workspace.json` ensuite ;
  fermeture : l'inverse. Un orphelin est supprimé au lancement suivant.
- `workspace.json` n'est pas digne de confiance : identifiants validés, infos lues
  entrée par entrée, compteurs bornés. Illisible : tout `resume/` mis de côté
  (`resume.failed-…`) ; un onglet illisible : son dossier seul (`t-<n>.failed-…`).
  Au lancement, seuls les `resume.keep_failed` plus récents de chaque sorte restent.
- Restauration paresseuse : la barre vient de `info`, une session n'est
  relue qu'à la première requête de son onglet (ADR-0001).

## Dépendances

`oku-slides-core` (deck, historique), `oku-slides-render` (Typst), `oku-slides-pdf`,
`oku-slides-platform` (stockage, réglages), `oku-slides-i18n`, `oku-slides-chart`, `oku-slides-a2ui`.
