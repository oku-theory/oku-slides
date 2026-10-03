# oku-slides-session

Session d'édition d'un deck, et espace de travail : fenêtres, onglets, une session par onglet, reprise.
L'app Tauri et le serveur de développement (`oku-slides-cli`, `serve`) exposent la même API.

## Types clés

- `Session` + `Request` (`lib.rs`) : un deck, son rendu Typst, son replica Loro (annulation par
  `UndoManager`, non persistée), ses modèles, sa reprise (`resume.rs`, fil `resume_writer.rs`).
  `Mode::{Templates, History}` mettent le deck de côté (`stash`) : `templates_mode.rs`, `history.rs`
  (`HistoryOpen/Seek/Restore/Close/Versions` ; restaurer = un commit annulable ; refus `HistoryNotOpen` / `InHistory`).
- `deck_widgets.rs` : la base surchargée par les widgets **valides** de la version embarquée
  (`SourceBundle.packs[deck.templates.version]`, jamais la bibliothèque ; sonde par fichier, avis
  `session-widgets-broken`) ; absente : base + `session-pack-missing`. `deck_pack.rs` : deck sans
  pack → version courante du pack par défaut ; `SetTemplates` embarque depuis la bibliothèque.
  `library.rs` : `Library` versionnée (`<data>/templates/<uuid>/{current, versions/<hex>/}`).
- `widgets_edit.rs` (statut, `set/remove/rename_widget`, mode modèles par `adopt_session_version`), `widgets_deck.rs` (mode deck : `SetTemplates`, une étape d'annulation par séance), `widget_preview.rs` (`preview_widget`, emplacement d'essai), `request.rs` (`Request`).
- Rendu borné : un `RenderWorker` par `Session`, un `RenderSlot` par rendu (deck, modèles, variantes, polices) ; `resolve_widgets` sonde les packs par lui.
  `fall_back_to_base` : pack fautif (`is_bounded`) → base + avis `session-pack-disabled` et un second essai (deck, aperçus, variantes ; `usable_pack` avant d'ouvrir un emplacement). `with_compiled` : export sur une poignée `Stale` (travailleur relancé) → re-rendu une fois, puis réessai.
- `merge.rs` : fusion d'un autre PDF du même deck (`merge_file`) ; `save_checked` : même deck changé sur disque → fusion puis écriture, autre deck → refusé.
- `Workspace` + `WorkspaceRequest` / `WorkspaceView` (`workspace.rs`) : fenêtres `w-<n>`, onglets `t-<n>` ; `TabInfo` (`views.rs`) : la barre.
- `Slot` (`workspace_slot.rs`) : session d'un onglet (`Ready`, `Dormant`, `Closed`) ; `workspace_resume.rs` : reprise ;
  `workspace_same_deck.rs` : `SameDeck` (onglet tenant déjà le deck) ; `post_notice`.
- `SlideView.fields` (`fields.rs`) : champs éditables (ordre de Tab) ; `targets` : boîtes d'éléments.
- `paragraphs_to_text` (`text.rs`, `inline.rs`) : markdown d'édition relu par `@lezer/markdown` ; contrat `fixtures/markdown*.json`.
- `agent::tabs` : `Sessions` (fourni par l'hôte), `TabSession` ; `workspace_agent.rs` l'implémente
  (`with_session`, `open_elsewhere`, `open_beside`). Un tour = un groupe d'annulation (`begin_turn`).

## Verrous

`Mutex<Workspace>` pour la structure, `Arc<Mutex<Slot>>` par onglet ; ordre **l'espace de travail, puis une session, jamais l'inverse.** Jamais d'attente sur une session sous le verrou de l'espace de travail : `try_lock` seulement (fermeture, onglet vierge).
- Travail lent hors de ce verrou : session neuve construite puis insérée ; onglet endormi relu sous son seul verrou.
- Seules `Workspace::apply`, `request` (par `with_session`, que l'agent emprunte aussi), `open_files`, `settle` et `post_notice` combinent les deux verrous, toujours en séquence, jamais imbriqués. Verrou empoisonné : `relock`.

## Disposition de `<data>/resume/`

```
workspace.json   structure (version 2), compteurs, `kept`, `info` par onglet
tabs/t-<n>/      state.json, deck/, templates/   (écrit par sa session seule)
deck/            deck.loro, document.json { path, dirty, saved_hash, blank }, assets/, fonts/, packs/<hex>.json|<hex>.<nom>.typ
```

- `workspace.json` n'est pas digne de confiance : identifiants validés, infos lues entrée par entrée, compteurs bornés. Illisible : tout `resume/` mis de côté (`resume.failed-…`) ; un onglet illisible : son dossier seul (`t-<n>.failed-…`). Au lancement, seuls les `resume.keep_failed` plus récents restent.
- Création : dossier d'onglet puis `workspace.json`, fermeture : l'inverse. Restauration paresseuse : la barre vient de `info`, session relue à la première requête (ADR-0001).

## Dépendances

`oku-slides-core` (deck, replica Loro), `oku-slides-render` (Typst), `oku-slides-render-worker` (processus de rendu), `oku-slides-pdf`, `oku-slides-platform` (stockage, réglages), `oku-slides-i18n`, `oku-slides-chart`, `oku-slides-a2ui`.
