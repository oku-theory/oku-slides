# oku-slides-core

Modèle du deck, opérations, replica Loro (CRDT), validation et zip des
sources. Aucune dépendance à Typst ni à Tauri.

## Types clés

- `Deck` (`model.rs`) : slides et éléments génériques `{id, kind, params}`
  (`Params`/`ParamValue`, `params.rs`), thème, `vars`, `templates` (`PackRef` : id et version
  embarquée, atome du replica) ; schéma JSON (`schemars`). `Style` (`Element.style`, en `Box`) :
  typographie et dessin d'un élément, dont `valign` (`STYLE_VALIGNS`, élément à cadre seulement),
  `inset` (4 marges en pt, `STYLE_INSET_MAX`), `tracking` (pt, `STYLE_TRACKING_MAX`) et `spacing` (espace
  entre paragraphes et items de liste, pt, de 0 à `STYLE_SPACING_MAX`) ; `radius` en pt ou jeton.
- `ShapePath` (`shape_path.rs`) : paramètre hôte `path` du widget `shape` (`shape: "path"`), `{ d }`
  en mots séparés (`M`, `L`, `C`, `Q`, `Z`, décimaux sans exposant), coordonnées 0..1 étirées au cadre,
  bornées (`SHAPE_PATH_MAX`, `SHAPE_PATH_COORD_MAX`) ; `from_commands(&[PathCommand])` (arrondi à 6
  décimales) et `commands()`.
- `WidgetRegistry` (`widget.rs`) : le schéma des widgets (`WidgetSchema`, `ParamSchema`,
  `ParamType`, `Category`, `HostType`, `SlotMode`) déclarés côté Typst, lu par le rendu ; les
  ops et `validate` s'y réfèrent pour ce qu'un `kind` accepte.
- `Op` + `apply_all` (`ops/mod.rs`) : opérations d'édition, appliquées atomiquement sur un
  `Deck` cloné avec le `WidgetRegistry` du deck (erreur → rien n'a bougé) ; tout sous-arbre
  inséré est typé contre le registre (`ops/check.rs`). `ops/list.rs` : `ListOp` (`edit_list`, items par id ; `ParamType::List`, conteneur `{list}`). `ops/grid.rs` : `set_grid`, sur la
  grille résolue (`Slide::resolved_grid`). `ops/host.rs` : paramètres `host(chart|table)` (contrôle à l'insertion, `set_chart_option`, `set_table_option`, `set_data`).
- `DeckReplica` (`replica/`) : le deck dans un `LoroDoc`, fusion et historique ; voir [`src/replica/ARCHITECTURE.md`](src/replica/ARCHITECTURE.md).
- `PackVersion`/`PackRef`/`PackFiles`/`WidgetFile` (`pack.rs`) : version adressée par contenu
  (sha-256 de `pack.json` à clés triées, `0`, puis nom, `0`, source, `0` par fichier de widget).
  Le contenu de `pack.json` y vit aussi : `PackManifest` (`from_files`, `to_files`, lecture
  tolérante des `examples`), `Template` (un modèle : sa slide d'exemple) et `now_created` ; la
  session les ré-exporte, l'import les lit sans dépendre d'elle.
- `tokens.rs` : jetons effectifs d'un thème (`effective_tokens` : défauts puis thème) et fusion
  profonde `merge`, partagés par la session (éditeur de thème), la CLI et l'import.
  `ThemeRef` (`Deck.theme`, atome du replica) : une version de thème est un `PackFiles` sans
  widget dont le manifeste est `theme.json` (`ThemeManifest`, `pack.rs` : `from_files`, `to_files` ;
  ré-exporté par la session, construit par l'import hi-fi) ; `SetTheme` change la seule référence (`null` : thème
  par défaut) ; embarquer la version dans les sources revient à l'hôte. Jetons : `SetToken` pose `null` tel quel (« vider »),
  `UnsetToken` retire la clé (objets vides élagués), `SetTokens` remplace tout `tokens`.
- `SourceBundle` (`sources.rs`) : `replica` fait foi, `deck` n'en est qu'une **vue** (rendu,
  `validate`, `locate`) ; `new` refuse un deck trop profond, met sous forme canonique avant et après la normalisation des hôtes, `merge` et `restore` passent `validate`.
  Zip v10 : snapshot Loro, assets, polices, `packs/<hex>/{pack.json, widgets/<nom>.typ}` et
  `themes/<hex>/theme.json` (`sources_themes.rs`) en ajout seul, vérifiés à la lecture
  (`issue-pack-version-mismatch`, `issue-theme-version-mismatch` ; version du deck absente →
  `issue-*-version-missing`), nom invalide → `BadName`, unis à la fusion.
- `validate` (`validate.rs`) : cohérence du deck contre le `WidgetRegistry` ; signale plutôt qu'il n'échoue.
- `Slide::text_fields` (`fields.rs`) : champs texte ; `locate` (`locate.rs`) : chemin d'un élément ;
  `apply(…, merge)` fusionne les étapes d'annulation de même clé.
- `canonicalize` (`canonical.rs`) : forme canonique d'un `Deck` (clés triées, slots vides supprimés) ; répare les ids en double — copie exacte écartée, autre occurrence renommée `{id}-n` — comme `apply_all` et la relecture du replica (t-652).

## Erreurs

`error/mod.rs` : `OpError`, `ReplicaError`, `SourcesError`, `TransactionError` — Snafu, `Localize`, Fluent ; `error/reason.rs` : les raisons de refus (`ValueReason`, `ExpectedType`, `Shape`) ; tests dans `error/tests.rs` et `error/reason_tests.rs`.
Une valeur refusée porte sa raison (`ValueReason`, `OpError::value_reason`) : `op-invalid-value-because` + `op-value-*`.

## Points d'extension

- Un nouveau champ du modèle : `model.rs`, classé dans le replica (`schema.rs`), couvert
  dans `replica/read_tests.rs` / `mod_tests.rs`, sans clé en `@…` (`tree_tests.rs`).
- Une nouvelle opération : `Op` (`ops/mod.rs`) + `ops_schema`, appliquée sur `Deck` (jamais sur le replica).
- Un type d'hôte : `HostType` + `ParamType::host` + `ops/host.rs` (`host_option`, `set_option`, `set_data`).

## Dépendances

`oku-slides-i18n`, `oku-slides-chart`, `loro`, `zip`, `sha2`/`hex` (assets, packs), `uuid` ; dev : `speculoos`, `rstest`, `proptest`.
