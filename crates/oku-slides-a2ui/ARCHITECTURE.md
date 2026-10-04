# oku-slides-a2ui

Les messages du protocole A2UI v0.9 que la session envoie à l'interface, et le catalogue
`urn:oku-slides:a2ui:catalog:v3` (`CATALOG_ID`). Le crate ne fait que décrire des données : ni état,
ni rendu, ni accès au disque.

## Types clés

- `ServerMessage` (`message.rs`) : `createSurface`, `updateComponents`, `updateDataModel`,
  `deleteSurface` ; sérialisation conforme à la v0.9, schéma JSON par `schemars`.
- `Component { id, kind }` et `ComponentKind` : le catalogue (mise en page, texte, carte, boutons,
  `Receipt`, `Timeline`, `Prompt`, `DiffText`, `SlideThumb`…) ; constructeurs `card`, `column`, `row`,
  `text`, `button`. `Dynamic<T>` : une valeur littérale ou une liaison `{path}` au modèle de données.
- `SlideRef { id, hash }` : la slide d'un `Receipt`, désignée par l'empreinte de sa page, jamais par son rendu
  (l'interface résout la miniature ; choix « état au reçu ou actuel » décrit dans `oku-slides-session`).
- Aides sur `ComponentKind` : `children` (identifiants enfants), `actions` (actions portées),
  `without_action` (le composant privé de son action, ou rien), `agent_allowed` (permis à `render` ;
  `Prompt`, `Timeline`, `Receipt`, `SlideThumb` sont réservés à l'hôte).

## Point d'extension

Un nouveau composant s'ajoute ici (variante de `ComponentKind`, avec ses aides) **et** dans
`ui/src/a2ui/catalog.tsx`. Le test de contrat de `oku-slides-session` (`forms_tests.rs`,
`every_form_matches_the_ui_fixture`) compare les formes émises à la fixture de l'interface : il échoue si les deux
côtés divergent.

## Dépendances

`serde`, `serde_json`, `schemars` ; aucune dépendance interne.

## Utilisateurs

`oku-slides-session` (formes, tours, reçus, `render`) et `oku-slides-mcp` (validation des surfaces
de l'agent).
