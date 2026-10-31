# oku-slides-core

Deck model, edit operations, Loro replica (CRDT), validation and the sources zip. No dependency
on Typst or Tauri: the session, the render crates, the CLI and the import all build on it.

## Data flow

`apply_all` applies `Op`s atomically to a `Deck`, checked against the deck's `WidgetRegistry`,
and leaves it canonical (`canonicalize`); `SourceBundle::apply` commits it into the `DeckReplica`.
The replica is authoritative; `SourceBundle.deck` is only a view of it, used for rendering,
`validate` and `locate`. Merging another copy goes through `SourceBundle::merge`.

## Modules

- `model.rs`: `Deck` (slides, generic elements `{id, kind, params}`, `theme` (a `Namespace`) and `themes` (its `extends`
  closure: a `ThemePin` per namespace), `vars`, `widgets` and `layouts`: a `PackPin` per `Namespace`, unique across both),
  `Style`, JSON schema via `schemars`. `params.rs`: `Params`, `ParamValue`, `ListItem`. `heading_level.rs`: `HeadingLevel`. `shape_path.rs`: `ShapePath`.
- `namespace.rs`: `Namespace` (`deck` reserved), `qualify` / `split_kind` (every kind is `ns/name`). `kinds.rs`: `kinds::base`,
  `kinds::essentiel`, `is_builtin_kind`. `pack_kind.rs`: `PackKind` (`widgets`: elements, `layouts`: slides).
- `widget.rs`: `WidgetRegistry`, `WidgetSchema` and its param types, declared in Typst; ops and `validate` ask it. Probe
  `{"packs": [{namespace, kind, widgets}]}`; a widget whose category is not its pack kind's is refused.
- `ops/`: `Op`, `apply_all`, `ops_schema` (`mod.rs`; `set_theme` checks `check_closure`); `edit.rs` applies the multi-step variants (checks, then the write); `check.rs` types subtrees;
  `value.rs` converts `set_param` values; `list.rs` (`ListOp`, items by id); `list_path.rs` (`ListPath`: `param` or
  `param/item/column`, one level); `grid.rs` (`set_grid` on `Slide::resolved_grid`); `host.rs` (`host(chart|table)`,
  `set_data`); `moving.rs` (`move_element`); `packs.rs` (`add_pack`, `remove_pack` by kind, `pack_users`).
- `replica/`: `DeckReplica`, the deck in a `LoroDoc` (merge, history, undo): [`src/replica/ARCHITECTURE.md`](src/replica/ARCHITECTURE.md).
- `pack.rs`: content-addressed `PackVersion` (sha-256 of `layouts\0` for a layout pack, the canonical manifest and widget files),
  `PackPin` (id, version, `requires`), `PackFiles` (with its `PackKind`), `WidgetFile`, `PackManifest` (`examples`, `styles`, read in
  `pack_examples.rs`; `tokens`: `ComponentToken`s checked on read; `profiles`: `LayoutProfile`). A theme version
  (`ThemeManifest`: `namespace`, ordered `extends`) is a prefix-free `PackFiles` without widgets, manifest `theme.json`
  (`PackFiles::theme`, `verified_theme`). `PackScope`, packs only (`library` default, unwritten; `deck`: an import's).
- `theme_chain.rs`: `ThemePin`, `linearize` (post-order, each theme once, the deck's last), `check_closure`.
  `token_spec.rs`: `Control` (an editor control), `ComponentToken` (a pack manifest `tokens` entry).
- `sources.rs`: `SourceBundle` (`SOURCES_VERSION`); its zip in `sources_zip.rs`: Loro snapshot, assets, fonts, `<kind>/<hex>/
  {<kind>.json, <kind>/<name>.typ}` per `PackKind` (`sources_packs.rs`), `themes/<hex>/theme.json` (`sources_themes.rs`, pins
  checked against manifests and closure), `usage.jsonl` (read last, own cap, never fails), all add-only, verified on read
  (a forged version reported once, as a namespace under both kinds), unioned on merge; reads bounded by `SourceLimits`.
- `tokens.rs`: `merge`, effective tokens, `theme_layer` (pack component defaults, then the chain). `validate.rs`: `Issue`s, cited assets (`asset_refs`, `element_asset_refs`);
  `canonical.rs`: `canonicalize`. `fields.rs`, `locate.rs`, `color.rs`: text fields, element lookup, colors (`color_names`: those a deck uses). `media.rs`: `sniff`, `dimensions`.
  `num.rs`: named float/int casts (import, session). `error/` (Snafu, Fluent): `OpError`, `ReplicaError`…; `reason.rs`: refusal reasons. `usage.rs`: `UsageLedger`; `usage_report.rs`: `aggregate`, `Range`.

## Extension points

- A model field: `model.rs`, classified in `replica/schema.rs`, covered by the replica read tests.
- An operation: an `Op` variant (`ops/mod.rs`) + `ops_schema`, plus its function in `ops/edit.rs` if not trivial; applied to `Deck`, never the replica.
- A host type: `HostType` + `ParamType::host` + `ops/host.rs`.

## Dependencies

`oku-slides-i18n`, `oku-slides-chart`, `loro`, `zip`, `sha2` / `hex`, `uuid`, `schemars`, `time` (usage dates).
