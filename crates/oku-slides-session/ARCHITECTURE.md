# oku-slides-session

Session d'édition d'un deck, et espace de travail : fenêtres, onglets, une
session par onglet, reprise au relancement. L'app Tauri (`src-tauri`) et le
serveur de développement (`oku-slides-cli`, `serve`) exposent la même API.

## Types clés

- `Session` + `Request` (`lib.rs`) : un deck, son rendu Typst, son replica Loro (annulation par
  `UndoManager`, non persistée), ses modèles, sa reprise (`resume.rs`, fil `resume_writer.rs`).
- `merge.rs` : fusion d'un autre PDF du même deck (`merge_file`) et enregistrement gardé
  (`save_checked`) : même deck changé sur disque → fusion puis écriture, autre deck → refusé.
- `Workspace` + `WorkspaceRequest` / `WorkspaceView` (`workspace.rs`) : fenêtres `w-<n>`,
  onglets `t-<n>`, onglet/fenêtre actifs ; `TabInfo` (`views.rs`) : ce que montre la barre.
- `Slot` (`workspace_slot.rs`) : session d'un onglet, `Ready`, `Dormant` ou `Closed`.
- `workspace_resume.rs` : relecture et écriture de `workspace.json`, orphelins, `open_files`.
- `workspace_same_deck.rs` : `SameDeck`, l'onglet qui tient déjà une autre copie d'un deck
  (`TabInfo.deck_uid`) ; `post_notice` y pose l'avis d'un fichier ouvert à part par l'OS.
- `paragraphs_to_text` (`text.rs`, inline dans `inline.rs`) : markdown d'édition
  relu par `@lezer/markdown` ; contrat `fixtures/markdown.json` et `-corpus.json` (`UPDATE_CORPUS=1`).

## Verrous

`Mutex<Workspace>` pour la structure, `Arc<Mutex<Slot>>` par onglet.

- Ordre : **l'espace de travail, puis une session, jamais l'inverse.**
- Jamais d'attente sur une session sous le verrou de l'espace de travail : `try_lock` seulement (fermeture, remplacement d'un onglet vierge).
- Travail lent hors du verrou de l'espace de travail : session neuve construite puis insérée, onglet endormi relu sous son seul verrou, dossiers fermés supprimés après.
- Seules `Workspace::apply`, `request`, `open_files`, `settle` et `post_notice` combinent les deux verrous, toujours en séquence, jamais imbriqués. Verrou empoisonné : `relock`.

## Disposition de `<data>/resume/`

```
workspace.json   structure (version 2), compteurs, `kept`, `info` par onglet
tabs/t-<n>/      state.json, deck/, templates/   (écrit par sa session seule)
deck/            deck.loro, document.json { path, dirty, saved_hash, blank }, assets/, fonts/
```

- Création : dossier de l'onglet d'abord, `workspace.json` ensuite ; fermeture : l'inverse. Un orphelin est supprimé au lancement suivant.
- `workspace.json` n'est pas digne de confiance : identifiants validés, infos lues entrée par entrée, compteurs bornés. Illisible : tout `resume/` mis
  de côté (`resume.failed-…`) ; un onglet illisible : son dossier seul (`t-<n>.failed-…`). Au lancement, seuls les `resume.keep_failed` plus récents restent.
- Restauration paresseuse : la barre vient de `info`, une session n'est relue qu'à la première requête de son onglet (ADR-0001).

## Dépendances

`oku-slides-core` (deck, replica Loro), `oku-slides-render` (Typst), `oku-slides-pdf`,
`oku-slides-platform` (stockage, réglages), `oku-slides-i18n`, `oku-slides-chart`, `oku-slides-a2ui`.
