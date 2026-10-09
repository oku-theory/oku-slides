# oku-slides-session

Session d'édition d'un deck, et espace de travail : fenêtres, onglets, une session par onglet, reprise.
L'app Tauri et le serveur de développement (`oku-slides-cli`, `serve`) exposent la même API.

## Types clés

- `Session` + `Request` (`lib.rs`) : un deck, son rendu Typst, son replica Loro (annulation par
  `UndoManager`, non persistée), ses modèles, sa reprise (`resume.rs`, fil `resume_writer.rs`).
  `Mode::{Templates, History}` mettent le deck de côté (`stash`) : `templates_mode.rs`, `history.rs`
  (`HistoryOpen/Seek/Restore/Close/Versions` ; restaurer = un commit annulable ; refus `HistoryNotOpen` / `InHistory`).
  `help.rs` : l'aide (`docs/aide/<langue>.json`, pack par défaut livré) en lecture seule ; chaque
  méthode qui modifie se garde (`ensure_editable` : `InHistory` / `ReadOnly`) ; `workspace_help.rs` : `OpenHelp`.
- `deck_widgets.rs` : la base surchargée par les widgets **valides** de la version embarquée
  (`SourceBundle.packs[deck.templates.version]`, jamais la bibliothèque ; sonde par fichier, avis
  `session-widgets-broken`) ; absente : base + `session-pack-missing`. `deck_pack.rs` : deck sans
  pack → version courante du pack par défaut ; `SetTemplates` embarque depuis la bibliothèque.
  `library.rs` : `Library<K>` versionnée, générique sur la sorte (`library_kind.rs` : `LibraryKind`, `Templates`) ; `<data>/templates/<uuid>/{current, versions/<hex>/pack.json + widgets/}`. `theme.rs` : sorte `Themes` (`<data>/themes/<uuid>/{current, versions/<hex>/theme.json}`, sans widgets), `ThemeManifest`, `ThemeLibrary`, thèmes intégrés Papier et Pro (`PAPIER_THEME`, `PRO_THEME`, `DEFAULT_THEME`) ; `Session.themes` (chemin `paths.themes()`).
- Thème du deck (`deck_theme.rs`) : la version embarquée (`SourceBundle.themes[deck.theme.version]`)
  fait foi, jamais la bibliothèque ; deck sans thème → version courante du thème par défaut,
  embarquée au chargement. `set_theme` (op `SetTheme`, une étape d'annulation) embarque d'abord la
  version voulue ; le rendu reçoit les jetons résolus (`Renderer::set_theme`), et `appearance.rs`
  en tire l'apparence de l'interface. `theme_schema.rs` (`theme_schema.json`, défauts lus dans
  `typst/oku-slides/src/default-tokens.json`) : contrôle, bornes et section de chaque jeton ; il
  vérifie `SetToken`/`SetTokens` (`session-invalid-token-value`, `session-token-in-use`).
- Éditeur de thème (mode modèles) : `theme_edit.rs` : le brouillon (`ThemeEdit` : base, nom, état)
  vit dans les jetons du deck de travail, sans thème ; ses changements sont des étapes du document
  des modèles. `save_templates` l'enregistre (`save_theme_edit`) : nouvelle version d'un thème de
  l'utilisateur, copie « <nom> (copie) » d'un thème intégré, rien s'il est inchangé ; le deck mis
  de côté y passe. `theme_panel.rs` + `theme_controls.rs` : surface `host-theme` (`host_theme`,
  filtrée ; `host_token` : `set`, `reset`, `remove`, `clear`). Le chemin du jeton part sous la clé
  `token` du contexte d'action, jamais `path` (que @a2ui prend pour une liaison de données).
- `widgets_edit.rs`, `widget_preview.rs` : fichiers de widgets, statuts, aperçu ; `widget_examples.rs`, `widget_params.rs` : valeurs d'exemple des widgets (un jeu par widget), dans le pack (`edit_example_list` pour une liste) ; `param_fields.rs` : champs de réglage d'un paramètre (atelier et cible), `coerce`, `schema_default` ; `items.rs` : `ItemView`, les items d'une liste. `widgets_deck.rs` (`SetTemplates`, une étape d'annulation par séance). En mode modèles, une écriture de widget est aussi une étape du document des modèles (`adopt_session_version`, clé de séance) ; annuler/rétablir réaligne l'édition (`realign_templates`).
- `panel.rs` : réglages de l'hôte au dock (modèle, widget), adressés par clé.
- `target_params.rs` : réglages de la cible (surface `host-target`) : élément sélectionné, sinon widget de la slide, lus dans le deck ; `target_param` / `target_list` annulables.
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
- `agent::turn_view` (surface d'un tour : consigne, timeline, note, issue), `receipt` (`report` et reçu de repli), `render` (composition libre validée) : constructeurs purs, formes A2UI de `oku-slides-a2ui`.
- Miniature d'un reçu (t-450) : le reçu ne porte jamais le rendu de la slide, seulement `SlideRef { id, hash }`
  (empreinte de sa page au moment du `report`, écrite comme `SlideSummary.hash`) ; sa taille ne dépend pas du
  contenu de la slide, ni dans le hub, ni dans l'état de l'interface, ni à chaque resynchronisation.
  L'interface (`ui/src/a2ui/slideThumbs.ts`) résout la référence avec son cache de SVG indexé par empreinte
  (celui de la pile, commande `svg` : Tauri et serveur de développement). **Choix** : l'état de la slide au
  moment du reçu tant que l'image de cette empreinte est connue (en cache, ou slide inchangée depuis) ; sinon
  (slide modifiée depuis et cache perdu, par exemple après un rechargement) son état **actuel**, estompé et
  marqué « modifiée depuis » : la session ne garde que le rendu des pages présentes (`svg_cache`), l'ancien
  n'est plus demandable. Slide supprimée : pas de miniature (la couleur `swatch`, s'il y en a une).
  **Mode historique** : `svg` et les empreintes du snapshot (`SlideSummary.hash`) sont alors ceux de la version
  montrée (le deck attend de côté). Un reçu dont l'image est en cache la garde ; sinon il montre la slide telle
  qu'elle est **dans la version montrée**, marquée « modifiée depuis » si l'empreinte diffère (plus ancienne ou
  plus récente que le reçu : la marque dit seulement « pas l'état du reçu ») ; slide absente de cette version :
  pas de miniature. En refermant l'historique, les empreintes du deck reviennent et la miniature avec.
  **États transitoires** (côté interface) : image pas encore reçue → pas de miniature (la couleur `swatch`
  en attendant, s'il y en a une), puis l'image dès sa réception ; slide en cours de re-rendu → la dernière
  image reçue de la slide reste montrée (`useSvgCache`), marquée d'après l'empreinte actuelle, jusqu'à la
  suivante ; snapshot pas encore chargé → pas de miniature.
- `agent::forms::Surface` : `place`, `turn` (rempli par le hub, au tour de l'appel), `live`, et `question` (attend la
  réponse de l'utilisateur : `ask_*`, confirmation, permission ; marqué par leurs constructeurs, jamais déduit de l'identifiant).

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

Conversations de l'agent par portée dans `agent/<portée>.json` (`agent/conversation.rs`, `resume_agent.rs`).
