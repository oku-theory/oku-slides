# oku-slides-a2ui

The A2UI v0.9 protocol messages the session sends to the UI, and the catalog
`urn:oku-slides:a2ui:catalog:v3` (`CATALOG_ID`). The crate only describes data: no state, no
rendering, no disk access.

## Key types

- `ServerMessage` (`message.rs`): `createSurface`, `updateComponents`, `updateDataModel`,
  `deleteSurface`; serialization follows v0.9, JSON schema via `schemars`.
- `Component { id, kind }` and `ComponentKind`: the catalog (layout, text, card, buttons, `Receipt`,
  `Timeline`, `Prompt`, `DiffText`, `SlideThumb`, `ThemeChoice`, `Notice`, `ChartData`, `TableData`…); constructors `card`, `column`, `row`,
  `text`, `button`; button builders `danger`, `icon` (`ButtonIcon`). `Dynamic<T>`: a literal value or a `{path}` binding to the data model.
- `SlideRef { id, hash }`: a `Receipt`'s slide, named by its page hash, never by its rendering (the
  UI resolves the thumbnail; the "state at receipt or current" choice is described in
  `oku-slides-session`).
- Helpers on `ComponentKind`: `children` (child ids), `actions` (actions it carries),
  `without_action` (the component without its action, or nothing), `agent_allowed` (allowed in
  `render`; `Prompt`, `Timeline`, `Receipt`, `SlideThumb`, `Notice`, `ThemeChoice`, `ChartData`,
  `TableData` are reserved to the host).
- `SeriesSwatch` (`data.rs`): a colourable series of `ChartData`. `ChartData` / `TableData` carry a
  chart's or table's data; the UI writes one edit at their `edit` path, then sends `action`.

## Extension point

A new component is added here (a `ComponentKind` variant, with its helpers) **and** in
`ui/src/a2ui/catalog.tsx`. The `oku-slides-session` contract test (`forms_tests.rs`,
`every_form_matches_the_ui_fixture`) compares the emitted forms with the UI fixture: it fails when
the two sides diverge.

## Dependencies

`serde`, `serde_json`, `schemars`; no internal dependency.

## Users

`oku-slides-session` (forms, turns, receipts, `render`, the pack banner) and `oku-slides-mcp`
(validation of agent surfaces).
