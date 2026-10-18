# oku-slides-core

Deck model, edit operations, Loro replica (CRDT), validation and the sources zip. No dependency
on Typst or Tauri: the session, the render crates, the CLI and the import all build on it.

## Data flow

`apply_all` applies `Op`s atomically to a `Deck`, checked against the deck's `WidgetRegistry`,
and leaves it canonical (`canonicalize`); `SourceBundle::apply` commits it into the `DeckReplica`.
The replica is authoritative; `SourceBundle.deck` is only a view of it, used for rendering,
`validate` and `locate`. Merging another copy goes through `SourceBundle::merge`.

## Modules

- `model.rs`: `Deck` (slides, generic elements `{id, kind, params}`, theme `ThemeRef`, `vars`,
  `widgets`: a `PackPin` per `Namespace`), `Style`, JSON schema via `schemars`. `params.rs`: `Params`,
  `ParamValue`, `ListItem`. `shape_path.rs`: `ShapePath`, the `path` host param of `shape`.
- `namespace.rs`: `Namespace` (`deck` reserved), `qualify` / `split_kind`: a pack widget's kind is `ns/name`, a bare kind is core.
- `widget.rs`: `WidgetRegistry` and `WidgetSchema` / `ParamSchema` / `ParamType` / `Category` /
  `HostType` / `SlotMode`, declared in Typst, pack widgets under `ns/name`; ops and `validate` ask it.
- `ops/`: `Op`, `apply_all`, `ops_schema` (`mod.rs`); `check.rs` types inserted subtrees;
  `value.rs` converts `set_param` values; `list.rs` (`ListOp`, items by id); `list_path.rs`
  (`ListPath`: `param` or `param/item/column`, an item's nested list, one level); `grid.rs`
  (`set_grid` on `Slide::resolved_grid`); `host.rs` (`host(chart|table)` params, `set_data`);
  `moving.rs` (`move_element`, `apply_all_moving`); `packs.rs` (`add_pack`, `remove_pack`, `pack_users`: no removal while used).
- `replica/`: `DeckReplica`, the deck in a `LoroDoc`: merge, history, undo; see
  [`src/replica/ARCHITECTURE.md`](src/replica/ARCHITECTURE.md).
- `pack.rs`: content-addressed `PackVersion` (sha-256 of the canonical manifest (`widgets.json`) and widget files),
  `PackPin` (id, version, `requires`), `PackFiles`, `WidgetFile`, `PackManifest`, `Template`; a theme version is a
  `PackFiles` without widgets whose manifest is `theme.json` (`ThemeManifest`, `ThemeRef`).
  `PackScope` (`library`, default, unwritten; `deck`: an import's, never in the library; `PackFiles::scope`).
- `tokens.rs`: a theme's effective tokens (defaults, then theme) and `merge`.
- `sources.rs`: `SourceBundle` (`SOURCES_VERSION`); its zip in `sources_zip.rs`: Loro snapshot,
  assets, fonts, `widgets/<hex>/{widgets.json, widgets/<name>.typ}` (`sources_packs.rs`),
  `themes/<hex>/theme.json` (`sources_themes.rs`), all add-only, verified on read (a forged
  version is reported once) and unioned on merge; reads bounded by `SourceLimits`.
- `validate.rs`: deck consistency against the registry, as `Issue`s; `canonical.rs`: `canonicalize`
  (sorted keys, empty slots dropped, duplicate ids repaired).
- `fields.rs`, `locate.rs` (`Located`, `MAX_SLOT_DEPTH`), `color.rs`: text fields, element lookup, colors.
- `error/`: `OpError`, `ReplicaError`, `SourcesError`, `TransactionError`… (Snafu, Fluent); `reason.rs`: refusal reasons.

## Extension points

- A model field: `model.rs`, classified in `replica/schema.rs`, covered by the replica read tests.
- An operation: an `Op` variant (`ops/mod.rs`) + `ops_schema`, applied to `Deck`, never the replica.
- A host type: `HostType` + `ParamType::host` + `ops/host.rs`.

## Dependencies

`oku-slides-i18n`, `oku-slides-chart`, `loro`, `zip`, `sha2` / `hex`, `uuid`, `schemars`.
