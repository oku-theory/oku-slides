# Duplicate and reorder items in a pack tab's pile

Task t-1249 (parent t-1152). Deferred from lot 6 (`plan:2026-10-28-onglets-pack-theme-lot6-ui`,
decision 6). A pack tab's pile offers add, rename and delete; this adds **duplicate** and
**reorder**, for the user (pile gestures) and for the pack tab's agent (MCP tools).

Today a version holds its files in a `BTreeMap<WidgetFile, String>`: items are listed in
alphabetical order in the pile (after a reopen), the layout and widget pickers and the
`layouts` tool. A new layout lands at the end of the pile only until the tab reopens.

## Decisions (made on 2026-10-31)

1. **Order everywhere (user).** The order is stored in the version and followed by every
   list of a pack's items: the pile, `LayoutPicker`, `WidgetPicker` and the `layouts` tool.
   Both kinds, widgets and layouts. The widget statuses (diagnostics) keep name order: the
   UI re-sorts them (`widgetList.ts` `ordered`) and no list shown in a pack tab reads them
   for order (amended while planning, 2026-10-31).
2. **Auto name, then select (user).** Duplicating `carte` creates `carte-2` at once (the
   first free of `-2`, `-3`, …), right after the original, selected and focused; F2 renames
   it like any item.
3. **MCP tools too (user).** The pack tab's agent gets `duplicate_widget` and
   `reorder_widgets`.
4. **`order` in the manifest.** `PackManifest.order: Vec<WidgetFile>`, `#[serde(default,
   skip_serializing_if = "Vec::is_empty")]`: a version without it keeps its digest.
   `_commun` is never listed. Rejected: ordering the working deck's slides only (lost on
   reopen, invisible to pickers); an ordered map of files (the files are separate `.typ`
   entries of the version).
5. **One resolution.** One core function gives the items of a version in order: those of
   `order` first, as listed, then the others in name order. Older versions, hand-added
   files and imports need no migration.
6. **Strict on read, normalised on write.** A version whose `order` lists a name twice,
   `_commun`, or a file it does not have is rejected (as duplicate A2UI option ids are, never
   deduplicated). `PackManifest::to_files`, the one place every pack write goes through,
   always writes a normalised `order`: missing names dropped, the resolved order otherwise.
   A rename keeps the place, a delete drops the name, a duplicate goes right after its
   original; a new item takes its place among the unlisted ones (by name, after the listed
   ones). A version whose resolved order is name order writes `order` empty, so that
   untouched packs keep their digest.
7. **Duplicate.** `Session::duplicate_widget(pack, from, to: Option<&str>) ->
   Result<(WidgetFile, Vec<WidgetStatus>)>`: the Typst source with its declared name
   rewritten (`renamed_source`), the examples and the style copied; `to` defaults to the
   first free `<from>-N` that fits `MAX_WIDGET_NAME`, else an error. `_commun` and a taken
   `to` are refused. One `commit_manifest`: one version, one undo step. No `ensure_unused`:
   nothing uses a new item.
8. **Reorder.** `Session::reorder_widgets(pack, order: Vec<String>) -> Result<Vec<WidgetStatus>>`
   takes the whole list of items; anything but a permutation of the current items (`_commun`
   excluded) is refused with the missing and extra names. One `commit_manifest`, one undo
   step; the same order is a no-op (no version). The UI derives the list from a drag or a
   key; the agent sends it whole.
9. **Pile gestures.** Duplicate: the item's context menu ("Dupliquer"); not on `_commun`.
   No ⌘D: it stays the window's dictation shortcut, which a focused thumbnail must not
   shadow (amended after the e2e, 2026-10-31). Reorder: drag (`SlidePile`'s `onMove`, as for slides) and
   ⌥↑ / ⌥↓ on the focused thumbnail, the focus following the item. `_commun` stays first and
   cannot move.
10. **`useWidgetFiles` queues them.** `duplicate(from)` and `reorder(order)` run in the same
    queue as `rename` and `remove`, after the pending typing is flushed; a refused request
    leaves the pile as it was and reports through `on.error`.

## Components

- **core (`pack.rs`, `pack_order.rs` new):** the `order` field, its validation in the
  version check, the resolution function, the normalisation used by `commit_manifest`.
- **session (`widgets_edit.rs`, a new `widgets_order.rs` if `widgets_edit.rs` would pass
  500 lines):** `duplicate_widget`, `reorder_widgets`, `Request::DuplicateWidget` /
  `Request::ReorderWidgets` classified like `RenameWidget`; `realign_layouts`,
  `layout_slides`, the widget preview slides (`pack_mode_preview`) and the add grid
  (`add_grid.rs`) follow the resolved order, `realign_layouts` reordering the working deck's
  slides.
- **mcp:** `duplicate_widget { from, to? }`, `reorder_widgets { order }` beside
  `rename_widget`, in the pack tab's tool family; local-agent routing; no bound of their own: the session
  checks the names and that `order` is exactly the items.
- **ui:** `api.ts` routes, `useWidgetFiles.duplicate` / `.reorder`, `PackPile` (menu entry,
  `onMove`, ⌥↑ / ⌥↓), Fluent keys in every locale.
- **docs:** help (`docs/aide`) "Éditer un pack"; `ARCHITECTURE.md` of core, session, mcp
  and ui where they list the pile or pack tools; the lot 6 plan's decision 6 points here.

## Error handling

Snafu variants with backtraces and Fluent messages: `WidgetOrderInvalid` (read: duplicate,
`_commun` or unknown name), `WidgetOrderMismatch { missing, extra }` (reorder),
`NoFreeCopyName { from }` (duplicate). Existing variants cover `_commun`, taken and invalid
names and leased packs.

## Testing

- core: resolution and validation as rstest tables (empty, partial, full, duplicate,
  `_commun`, unknown); normalisation after rename, delete, add, duplicate; an untouched pack
  keeps its digest.
- session: duplicate (auto name, `-3` when `-2` is taken, explicit `to`, `_commun` refused,
  copy placed after the original, one undo step restores the previous version); reorder
  (permutation required, no-op, one undo step); rename and delete keep the order; a layout
  pack's working deck follows the order after a reorder and after a reopen.
- mcp: both tools in a pack tab, refused in a deck tab, errors in a non-English locale.
- ui: `PackPile` render tests (menu entry, ⌘D left to dictation, drag, ⌥↑ / ⌥↓, focus after each);
  `useWidgetFiles` queue tests.
- e2e: one spec duplicates and reorders a layout, then checks `LayoutPicker` in a deck.
