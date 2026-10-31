# Duplicate and reorder in a pack tab's pile — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Duplicate an item of a pack tab's pile and reorder its items, from the pile and from the tab's agent, with the order stored in the version and followed by the pile and the pickers.

**Architecture:** A new `PackManifest.order` field (core) lists items in their chosen order; `pack_order.rs` resolves it (listed first, the rest by name) and normalises it in `PackManifest::to_files`, the one place every pack write goes through; `from_files` rejects a bad list. The session adds `duplicate_widget` and `reorder_widgets` beside `rename_widget`, and every builder of a pack's item list (layout slides, widget preview slides, add grid) iterates the resolved order. MCP adds two pack-tab tools; the UI adds a menu entry, drag and ⌥↑ / ⌥↓ to `PackPile` (⌘D was dropped after the e2e: it stays dictation, spec decision 9).

**Tech Stack:** Rust (snafu, serde, speculoos, rstest, nextest), Fluent (20 locales), React/TS (Bun, happy-dom render tests), Playwright e2e.

**Spec:** `docs/superpowers/specs/2026-10-31-pile-duplicate-reorder-design.md` (decisions 1-10). Oku: t-1152 › t-1249; Task 1 t-1320, Task 2 t-1321, Task 3 t-1322, Task 4 t-1323, Task 5 t-1324, Task 6 t-1325.

## Global Constraints

- Every rule of `AGENTS.md` (root, `crates/`, each touched crate, `ui/`, `docs/`) applies; read each touched directory's `AGENTS.md` and `ARCHITECTURE.md` first.
- Files aim at 500 lines, never above 1000: `pack.rs` (554) gets no new logic, it goes to `pack_order.rs`; `widgets_edit.rs` (549) gets no new public method, they go to `widgets_order.rs`.
- Errors: Snafu, `backtrace: Backtrace` in every variant, opaque public types, Fluent-localized messages; no `unwrap`/`expect` outside tests.
- Every new Fluent key in all 20 locales (`crates/oku-slides-i18n/src/locales/*/`), same key order, translated (never the English text), `en` first.
- Rust unit tests in a sibling `x_tests.rs` (`#[cfg(test)] #[path = "x_tests.rs"] mod x_tests;`), speculoos, names `unit_scenario_result`, rstest for tables; localized error tests assert the kind or key plus one non-English locale.
- Every run outside `just e2e` sets `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS` to a fresh temp dir; the first `just test` in this worktree builds cold: run it in the background.
- Commits: `type(scope): description (t-N)`, English, each citing its sub-task and t-1249; message file in the scratchpad (`git commit -F`), never `$TMPDIR`; end with the attribution line the session gives; the commit hook wants `Reviewed-by:` or `Review-waived:`.
- Every `pub` item and new module documented, first sentence under 15 words.

---

### Task 1: `order` in the manifest (core)

**Files:**
- Create: `crates/oku-slides-core/src/pack_order.rs`, `crates/oku-slides-core/src/pack_order_tests.rs`
- Modify: `crates/oku-slides-core/src/pack.rs` (field after `tokens`, ~l. 444; `from_files` l. 455; `to_files` l. 489), `crates/oku-slides-core/src/lib.rs` (module + re-export), `crates/oku-slides-core/src/error/token.rs` (new kind and variant), `crates/oku-slides-i18n/src/locales/*/core.ftl` (one key after `pack-manifest-json`), `crates/oku-slides-core/ARCHITECTURE.md`
- Modify (mechanical): every `PackManifest { … }` literal (50 sites, `rg -l "PackManifest \{" crates src-tauri -g '*.rs'`) gains `order: Vec::new(),`
- Test: `crates/oku-slides-core/src/pack_order_tests.rs`, `crates/oku-slides-core/src/pack_tests.rs`

**Interfaces:**
- Produces:
  - `PackManifest.order: Vec<WidgetFile>` — `#[serde(default, skip_serializing_if = "Vec::is_empty")]`, doc: "Items in their chosen order; the unlisted follow by name. Empty: name order."
  - `pub fn item_order<'a>(order: &'a [WidgetFile], files: &'a PackFiles) -> Vec<&'a WidgetFile>` in `pack_order.rs`, re-exported at the crate root: the files of `files` except `_commun`, those of `order` first (as listed, skipping names `files` lacks), then the others in name order.
  - `PackManifestErrorKind::Order` and `PackManifestError::order_name(&self) -> Option<&str>` (the offending name).
  - Fluent key `pack-manifest-order = The pack's item order is invalid at “{ $name }”.` (fr: `L'ordre des éléments du pack est invalide à « { $name } ».`).

- [ ] **Step 1: Write the failing resolution tests** in `pack_order_tests.rs` (module declared at the bottom of `pack_order.rs`). Build files with `PackFiles::new(PackKind::Layouts, Map::new(), widgets)` where `widgets` maps `WidgetFile::parse(n)` to `String::new()`.

```rust
use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::{PackFiles, PackKind, WidgetFile};

fn files(names: &[&str]) -> PackFiles {
    let widgets = names.iter().map(|n| (WidgetFile::parse(n).unwrap(), String::new())).collect();
    PackFiles::new(PackKind::Layouts, serde_json::Map::new(), widgets)
}

fn names(order: &[&str]) -> Vec<WidgetFile> {
    order.iter().map(|n| WidgetFile::parse(n).unwrap()).collect()
}

#[rstest]
#[case::empty_is_name_order(&[], &["a", "b", "c"])]
#[case::partial_then_name_order(&["c"], &["c", "a", "b"])]
#[case::full(&["b", "c", "a"], &["b", "c", "a"])]
#[case::missing_name_skipped(&["z", "b"], &["b", "a", "c"])]
fn item_order_of_three_items_follows_the_list(#[case] order: &[&str], #[case] expected: &[&str]) {
    let f = files(&["_commun", "a", "b", "c"]);
    let order = names(order);
    let got: Vec<&str> = item_order(&order, &f).into_iter().map(WidgetFile::as_str).collect();
    assert_that!(got).is_equal_to(expected.to_vec());
}
```

- [ ] **Step 2: Write the failing normalisation and validation tests** (same file, plus `pack_tests.rs` for `from_files`/`to_files`):

```rust
#[rstest]
#[case::alphabetical_collapses(&["a", "b", "c"], &[])]
#[case::reordered_kept_whole(&["c", "a"], &["c", "a", "b"])]
#[case::missing_dropped(&["c", "gone", "a"], &["c", "a", "b"])]
fn normalized_order_of_three_items(#[case] order: &[&str], #[case] expected: &[&str]) {
    let f = files(&["a", "b", "c"]);
    assert_that!(normalized_order(&names(order), f.widgets().keys())).is_equal_to(names(expected));
}
```

In `pack_tests.rs` (reuse its manifest builder; read the file's top helpers first):
- `to_files_with_an_alphabetical_order_writes_no_order_key` — `manifest.order = names(["a","b"])`, files `a`, `b`: `files.manifest().contains_key("order")` is false.
- `to_files_of_a_manifest_without_order_keeps_its_digest` — the version hash of `to_files` with `order: vec![]` equals the hash of the same manifest serialized before this change (assert that `serde_json::to_value(&manifest)` has no `order` key, which is what keeps the digest).
- rstest `from_files_with_a_bad_order_is_refused(#[case] order: &[&str])` with cases `duplicate(["a","a"])`, `common(["_commun"])`, `unknown(["zz"])`: write the raw manifest JSON with `"order": [...]` into `PackFiles::new` (bypassing `to_files`) and assert `PackManifest::from_files(&f).unwrap_err().kind()` is `PackManifestErrorKind::Order`.
- `from_files_with_a_bad_order_localizes_in_french` — `Localizer` for `fr`: the message contains `« zz »`.

- [ ] **Step 3: Run them, expect compile failures** (`item_order`, `normalized_order`, `order` unknown).

Run: `cargo nextest run -p oku-slides-core pack_order pack_tests`
Expected: FAIL to compile.

- [ ] **Step 4: Implement.**
  - `pack.rs`: the `order` field (after `tokens`). `from_files` adds, after `check_entries`: `check_order(&manifest.order, files).context(ManifestOrderSnafu)?;`. `to_files` serializes a clone whose `order` is `normalized_order(&self.order, &PackFiles-of-widgets)`: build the normalised list from the `widgets` map directly (write `normalized_order` over an iterator of names, see below) so no temporary `PackFiles` is needed.
  - `pack_order.rs` (module doc: "Item order of a pack version: resolution, normalisation, check."):

```rust
/// The items of `files` in order: those of `order` first, then the rest by name.
#[must_use]
pub fn item_order<'a>(order: &'a [WidgetFile], files: &'a PackFiles) -> Vec<&'a WidgetFile> {
    resolve(order, files.widgets().keys())
}

/// `order` ready to write: missing names dropped, unlisted appended; empty when in name order.
pub(crate) fn normalized_order<'a>(order: &[WidgetFile], names: impl Iterator<Item = &'a WidgetFile> + Clone) -> Vec<WidgetFile> {
    let resolved = resolve(order, names.clone());
    let by_name: Vec<&WidgetFile> = names.filter(|n| !n.is_common()).collect();
    if resolved == by_name { Vec::new() } else { resolved.into_iter().cloned().collect() }
}

fn resolve<'a>(order: &'a [WidgetFile], names: impl Iterator<Item = &'a WidgetFile> + Clone) -> Vec<&'a WidgetFile> {
    let mut out: Vec<&WidgetFile> = order.iter().filter(|o| !o.is_common() && names.clone().any(|n| n == *o)).collect();
    out.dedup(); // listed once by `check_order` on read; a caller's list may repeat
    out.extend(names.filter(|n| !n.is_common() && !order.contains(n)));
    out
}

/// Refuses an `order` that repeats a name, lists `_commun` or a file `files` lacks.
pub(crate) fn check_order(order: &[WidgetFile], files: &PackFiles) -> Result<(), OrderError> { … }
```

  `resolve` takes names from `order` in list order (a `BTreeMap` key iterator is already in name order, so the tail is name order). `out.dedup()` only removes adjacent repeats; since `check_order` refuses repeats on read and `reorder_widgets` (Task 2) refuses them on input, that is enough — say so in the comment.
  - `error/token.rs`: `PackManifestErrorKind::Order` (doc: "`order` repeats a name, lists `_commun` or an unknown file."), inner variant `#[snafu(display("invalid item order in the pack manifest at {name}"))] ManifestOrder { name: String, backtrace: Backtrace }` built by `check_order` (make `check_order` return `Result<(), PackManifestError>` directly with `ManifestOrderSnafu { name }.fail()` if that reads simpler than a separate `OrderError`; pick one, no extra type), `kind()`, `order_name()`, and `localize` → `pack-manifest-order` with `name`.
  - `core.ftl` in all 20 locales, right after `pack-manifest-json`.
  - The 50 `PackManifest {` literals: add `order: Vec::new(),` after their `tokens` line (`cargo check --workspace --tests` lists the remaining ones).
  - `ARCHITECTURE.md` (core): one line for `pack_order.rs`.

- [ ] **Step 5: Run the tests.**

Run: `cargo nextest run -p oku-slides-core` then `cargo check --workspace --all-targets`
Expected: PASS; no compile error.

- [ ] **Step 6: Commit** — `feat(core,i18n): item order in pack manifests (t-1249, t-1320)`.

---

### Task 2: Duplicate and reorder in the session

**Files:**
- Create: `crates/oku-slides-session/src/widgets_order.rs`, `crates/oku-slides-session/src/widgets_order_tests.rs`
- Modify: `crates/oku-slides-session/src/widgets_edit.rs` (`rename_widget` l. 422: keep the place; `remove_widget` l. 396 needs nothing, `to_files` drops the name), `crates/oku-slides-session/src/request.rs` (two variants after `RenameWidget`, l. 316, and their arms after l. 475), `crates/oku-slides-session/src/request_class.rs` (add both next to `RenameWidget` in each of the five matches, l. 46, 99, 128, 156, 235), `crates/oku-slides-session/src/error.rs` (two variants near `WidgetExists`, l. 370, 390, 476, 607, 774), `crates/oku-slides-session/src/lib.rs` (module, re-export `WidgetDuplicate`), `crates/oku-slides-i18n/src/locales/*/session.ftl` (two keys after `session-widget-exists`), `crates/oku-slides-session/ARCHITECTURE.md` (the widget-editing line names `widgets_order.rs`)

**Interfaces:**
- Consumes: `PackManifest.order`, `item_order` (Task 1).
- Produces:
  - `pub struct WidgetDuplicate { pub name: String, pub statuses: Vec<WidgetStatus> }` (`Serialize`; doc: "What a duplicate returns: the copy's name and the statuses after it.")
  - `Session::duplicate_widget(&mut self, pack: Option<&Namespace>, from: &str, to: Option<&str>) -> Result<WidgetDuplicate>`
  - `Session::reorder_widgets(&mut self, pack: Option<&Namespace>, order: &[String]) -> Result<Vec<WidgetStatus>>`
  - `Request::DuplicateWidget { pack: Option<Namespace>, from: String, to: Option<String> }` (cmd `duplicate_widget`, `#[serde(default)]` on `pack` and `to`), `Request::ReorderWidgets { pack: Option<Namespace>, order: Vec<String> }` (cmd `reorder_widgets`); both answer JSON and `persist(true)` like `RenameWidget`.
  - Errors: `NoFreeCopyName { from: String }` → kind `InvalidWidget`, key `session-widget-no-copy-name = No free name for a copy of “{ $from }”.`; `WidgetOrderMismatch { missing: Vec<String>, extra: Vec<String> }` → kind `InvalidWidget`, key `session-widget-order-mismatch = The new order must list each item once. Missing: { $missing }. Unexpected: { $extra }.` (names joined with `, `; an empty list shows `—`).

- [ ] **Step 1: Write the failing tests** in `widgets_order_tests.rs`. Reuse the `editing` helper pattern of `widgets_edit_tests.rs` (copy it: a pack tab of `ATELIER`) and `KPI`; for layouts use `test_support::layout_pack_tab(data, "salle", &["a", "b", "c"])`.

```rust
#[test]
fn duplicate_widget_without_a_name_takes_the_first_free_copy_name() {
    let data = crate::test_support::fresh_dir("oku-slides-order-");
    let mut s = editing(data.path());
    s.set_widget(None, "kpi", KPI.into()).unwrap();
    let copy = s.duplicate_widget(None, "kpi", None).unwrap();
    assert_that!(copy.name.as_str()).is_equal_to("kpi-2");
    assert_that!(s.widget_source(None, "kpi-2").unwrap().as_str()).contains("widget(\"kpi-2\"");
}
```

Then, one behaviour each:
- `duplicate_widget_when_the_copy_name_is_taken_takes_the_next` (`kpi-2` exists → `kpi-3`).
- `duplicate_widget_with_a_name_uses_it` (`Some("total")`).
- `duplicate_widget_to_a_taken_name_is_refused` (kind `InvalidWidget`, `WidgetExists`).
- `duplicate_widget_of_the_shared_file_is_refused` (`"_commun"` → `InvalidWidget`).
- `duplicate_widget_of_a_64_char_name_is_refused_with_no_copy_name` (a 64-char name: `"k".repeat(64)` with its declared name; no `-N` fits → error key `session-widget-no-copy-name`, also asserted in `fr`).
- `duplicate_widget_copies_examples_and_style` (set an example value with the existing example-values API used in `widget_examples_tests.rs::rename_widget_carries_its_values`, then compare the manifest examples of `kpi` and `kpi-2`).
- `duplicate_widget_places_the_copy_right_after_its_original` (layout pack `a b c`, duplicate `a` → `item_order` of the edited version is `a a-2 b c`; read it through `PackManifest::from_files` on the session's edited files, using the same accessor the tests of `widgets_edit_tests.rs` use for the edited version).
- `duplicate_widget_then_undo_removes_the_copy` (`s.undo()` → `widget_source(None, "kpi-2")` is `UnknownWidget`).
- `reorder_widgets_stores_the_order` (layouts `a b c`, reorder `["c","a","b"]` → resolved order `c a b`).
- `reorder_widgets_with_a_missing_or_extra_name_is_refused` (rstest: `["a","b"]`, `["a","b","c","d"]`, `["a","a","b","c"]`, `["_commun","a","b","c"]` → `InvalidWidget`; the first also asserted on its `fr` message listing `c`).
- `reorder_widgets_to_the_same_order_makes_no_version` (the edited version is unchanged — compare the version hash before and after).
- `reorder_widgets_then_undo_restores_the_order`.
- `rename_widget_keeps_its_place_in_the_order` (reorder `c a b`, rename `a`→`z` → `c z b`).
- `remove_widget_drops_it_from_the_order` (reorder `c a b`, remove `a` → `c b`).
- `duplicate_and_reorder_requests_answer_json` (`s.handle(Request::DuplicateWidget { … })` and `ReorderWidgets` return `Ok`; the first's JSON has `"name":"kpi-2"`).

- [ ] **Step 2: Run them, expect compile failures.**

Run: `cargo nextest run -p oku-slides-session widgets_order`
Expected: FAIL to compile.

- [ ] **Step 3: Implement** in `widgets_order.rs` (`impl Session`, module doc: "Duplicate and reorder the items of the edited pack."):
  - `duplicate_widget`: `parse_name(from)`; refuse `_commun` (`InvalidWidgetNameSnafu`, as `rename_widget`); `widget_target(pack)`; source or `UnknownWidget`; `to` = given (parsed, `_commun` refused, taken → `WidgetExists`) else `(2..).map(|n| format!("{from}-{n}")).map_while(|c| WidgetFile::parse(&c)).find(|c| t.files.widget(c).is_none())` → `NoFreeCopyName`. Copy with `renamed_source(source, from, to)` (make it `pub(crate)` if it is not). Manifest: clone `examples[from]` and `styles[from]` to `to`; `manifest.order = item_order(&manifest.order, &t.files)` cloned, with `to` inserted right after `from`. `self.commit_manifest(&t, manifest, widgets)` → statuses, `with_render(…, &to)` as `rename_widget` does; `scratch.written(&to)` only if the scratch has such a hook for new files — check `pack_scratch.rs` and mirror what `set_widget` does for a new file.
  - `reorder_widgets`: parse each name (`parse_name`); compute `missing` (items not listed) and `extra` (listed names not items, `_commun`, repeats); non-empty → `WidgetOrderMismatch`. `manifest.order = parsed`; `commit_manifest` (it already makes no version when nothing changed, which `to_files`' normalisation guarantees for the same order). Returns the statuses.
  - `rename_widget` (`widgets_edit.rs`): before `commit_manifest`, `manifest.order = item_order(&manifest.order, &t.files).into_iter().map(|n| if *n == from { to.clone() } else { n.clone() }).collect();` — fill the whole list so the place survives.
  - `request.rs`, `request_class.rs`, `error.rs`, `session.ftl` (20 locales), `lib.rs`, `ARCHITECTURE.md` as listed.

- [ ] **Step 4: Run the tests.**

Run: `cargo nextest run -p oku-slides-session widgets_order widgets_edit request_class`
Expected: PASS.

- [ ] **Step 5: Commit** — `feat(session,i18n): duplicate and reorder pack items (t-1249, t-1321)`.

---

### Task 3: Item lists follow the order

**Files:**
- Modify: `crates/oku-slides-session/src/pack_mode.rs` (`layout_names` l. 82, `realign_layouts` l. 96: reorder the working deck to the resolved order instead of appending), `crates/oku-slides-session/src/pack_mode_preview.rs` (`preview_slides` l. 44, `realign_widget_previews` l. 56), `crates/oku-slides-session/src/add_grid.rs` (`add_pack_view` l. 92, `PackSeeds`: iterate the resolved order), module docs of `pack_mode.rs` ("in file order" → "in the version's item order")
- Test: `crates/oku-slides-session/src/pack_mode_tests.rs`, `crates/oku-slides-session/src/pack_mode_preview_tests.rs`, `crates/oku-slides-session/src/add_grid_tests.rs`, `crates/oku-slides-session/src/deck_widgets_fallback_tests.rs` is not touched

**Interfaces:**
- Consumes: `item_order` (Task 1), `reorder_widgets` (Task 2).
- Produces: no new API. `layout_names(manifest, files)` takes the manifest (callers pass it; all have it). `PackSeeds` gains `order: Vec<WidgetFile>` (the resolved element files), used by `add_pack_view` to emit `widgets` in order.

- [ ] **Step 1: Write the failing tests** (one behaviour each):
  - `pack_mode_tests.rs`: `reorder_widgets_in_a_layout_pack_reorders_the_working_deck` (layout tab `a b c`, reorder `c a b` → working deck slide ids `layout-c layout-a layout-b`); `layout_pack_tab_reopened_after_a_reorder_keeps_the_order` (reorder, then open a new pack tab on the same pack id in the same data dir — mirror the reopen helper the pack tab tests use — and check the ids); `duplicate_widget_in_a_layout_pack_puts_its_slide_after_the_original` (`a b c`, duplicate `a` → `layout-a layout-a-2 layout-b layout-c`).
  - `pack_mode_preview_tests.rs`: `reorder_widgets_in_a_widget_pack_reorders_the_preview_slides` (two widgets, reorder, compare `slide_ids` filtered on `widget-`).
  - `deck`-side: `layouts_view_lists_a_pack_s_layouts_in_its_order` — a deck pinning a layout version whose manifest has `order: [c, a, b]` (use `test_support::add_version_of` with a manifest whose `order` is set, then `Op::AddPack` as in `deck_widgets_fallback_tests.rs:200`) → `view.packs[i].layouts` kinds `ns/c ns/a ns/b`. Put it in `pack_mode_tests.rs` if it fits there, else in a new `layouts_order_tests.rs` declared from `pack_mode_previews.rs`.
  - `add_grid_tests.rs`: `add_packs_lists_a_pack_s_widgets_in_its_order` (a pinned widget pack version with `order` set → `AddPackView.widgets` names in that order).

- [ ] **Step 2: Run them, expect failures** (name order instead of the stored order).

Run: `cargo nextest run -p oku-slides-session pack_mode add_grid`
Expected: FAIL on the order assertions.

- [ ] **Step 3: Implement.**
  - `layout_names(manifest, files)` returns `item_order(&manifest.order, files)` for a layout pack, nothing otherwise; `layout_slides` passes its manifest.
  - `realign_layouts`: build the slide list in `ids` order, taking each existing slide (params refreshed as today) or a new `example_slide`; slides not in `ids` go (as today). Replace `deck.slides` with that list; keep `canonicalize`.
  - `preview_slides` / `realign_widget_previews`: iterate `item_order(&manifest.order, files)` instead of `files.widgets().keys()`; the comment "Elles suivent l'ordre des fichiers" becomes "in the version's item order" (translate the touched comments to English).
  - `add_grid.rs`: `element_files` keeps filtering; `pack_seeds` reads the manifest already: store `order` = `item_order(&manifest.order, files)` filtered by `element_files`' predicate; `add_pack_view` iterates `seeds.order` and looks each seed up in `seeds.seeds`.

- [ ] **Step 4: Run the session suite.**

Run: `cargo nextest run -p oku-slides-session` (background; cold build)
Expected: PASS.

- [ ] **Step 5: Commit** — `feat(session): pack item lists follow the stored order (t-1249, t-1322)`.

---

### Task 4: MCP tools

**Files:**
- Modify: `crates/oku-slides-mcp/src/widget_tools.rs` (args structs and two handlers after `remove_widget`, l. 132), `crates/oku-slides-mcp/src/handler.rs` (l. 73, two arms), `crates/oku-slides-mcp/src/tools.rs` (two entries after `select_item`, l. 362, `Kinds::PACK`), `crates/oku-slides-mcp/src/tools_tests.rs` (`PACK_TOOLS`, sorted), `crates/oku-slides-local-agent/src/families.rs` (l. 72-74: both in `Packs`), `crates/oku-slides-local-agent/src/families_tests.rs` (cases in `families_of_a_pack_tool_is_packs`), `crates/oku-slides-agent/src/instructions/pack.md` (name both tools), `crates/oku-slides-mcp/ARCHITECTURE.md` if it lists widget tools
- Test: `crates/oku-slides-mcp/tests/pack_tab_tools.rs`

**Interfaces:**
- Consumes: `Request::DuplicateWidget`, `Request::ReorderWidgets` (Task 2).
- Produces:
  - `DuplicateWidgetArgs { from: String, to: Option<String> }` (docs: "Name of the item to copy." / "Name of the copy; absent: `<from>-2`, or the next free number.")
  - `ReorderWidgetsArgs { order: Vec<String> }` (doc: "Every item of the pack once, `_commun` excluded, in the new order.")
  - Tool descriptions: `duplicate_widget` — "Copy an item (widget or layout) of this pack, with its test values and style, right after it; returns {name, statuses}. One undo step."; `reorder_widgets` — "Set the order of this pack's items (the pile, the layout and widget pickers): every item once, `_commun` excluded. One undo step."
  - Events: duplicate emits `McpEvent::Changed` then `McpEvent::WidgetTouched { name: <copy>, removed: false }`; reorder emits `McpEvent::Changed`.

- [ ] **Step 1: Write the failing tests** in `pack_tab_tools.rs` (its helpers: `start_in_pack_tab`, `server`, `call`, `text_of`, `ACME_WIDGET`, `unknown_tool`):

```rust
#[tokio::test(flavor = "multi_thread")]
async fn duplicate_widget_in_a_pack_tab_answers_the_copy_name() {
    let (h, _ws, _tab) = start_in_pack_tab().await;
    let c = server(&h).await;
    let r = call(&c, "duplicate_widget", json!({ "from": ACME_WIDGET })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(text_of(&r).as_str()).contains(&format!("\"name\":\"{ACME_WIDGET}-2\""));
}
```

Also: `reorder_widgets_with_a_missing_item_is_refused_in_french` (`order: []` when the pack has items → `is_error`, text equals `h.localizer.tr_with("session-widget-order-mismatch", …)` and contains a French word of that message); `duplicate_widget_outside_a_pack_tab_is_refused_as_an_unknown_tool` (as `save_pack_outside_a_pack_tab…`); `reorder_widgets_in_a_pack_tab_emits_changed` (subscribe, call with the pack's items reversed, `next_event` is `Changed`). Add `"duplicate_widget"` and `"reorder_widgets"` to `PACK_TOOLS` in `tools_tests.rs` (sorted) and to the families test cases.

- [ ] **Step 2: Run them, expect failures** (unknown tool).

Run: `cargo nextest run -p oku-slides-mcp pack_tab_tools tools_tests` and `cargo nextest run -p oku-slides-local-agent families`
Expected: FAIL.

- [ ] **Step 3: Implement** the two handlers on the `remove_widget` model: `parse`, `continues_elsewhere(turn)`, `with_session(|s| s.handle(Request::…))`, emit the events, `text(json)`, else `fail_with`. Register them; `families.rs`; one sentence in `pack.md`: "Copy an item with `duplicate_widget`; `reorder_widgets` sets the items' order."

- [ ] **Step 4: Run the tests.**

Run: same as Step 2, then `cargo nextest run -p oku-slides-agent` (instruction tests)
Expected: PASS.

- [ ] **Step 5: Commit** — `feat(mcp,local-agent,agent): duplicate_widget and reorder_widgets tools (t-1249, t-1323)`.

---

### Task 5: Pile gestures (UI)

**Files:**
- Modify: `ui/src/api.ts` (after `renameWidget`, l. 741; `WidgetDuplicate` type near `WidgetChange`, l. 196), `ui/src/useWidgetFiles.ts` (interface l. 44-48, implementations after `rename`, l. 372), `ui/src/PackPile.tsx`, `ui/src/SlidePile.tsx` (title l. 191: `hint ?? (onMove ? t("ui-pile-drag-hint") : undefined)`), `crates/oku-slides-i18n/src/locales/*/ui.ftl` (keys after `ui-pile-menu-delete`, l. 351)
- Test: `ui/src/PackPile.render.test.tsx`, `ui/src/usePackItem.render.test.tsx` or the `useWidgetFiles` tests file that covers `rename` (find it with `rg -l "renameWidget" ui/src/*.test.tsx`)

**Interfaces:**
- Consumes: commands `duplicate_widget`, `reorder_widgets` (Task 2).
- Produces:
  - `api.duplicateWidget(from: string, pack?: string) => call<WidgetDuplicate>({ cmd: "duplicate_widget", from, pack })`, `api.reorderWidgets(order: string[], pack?: string) => call<WidgetStatus[]>({ cmd: "reorder_widgets", order, pack })`; `interface WidgetDuplicate { name: string; statuses: WidgetStatus[] }`.
  - `WidgetFiles.duplicate(from: string): Promise<string | null>` (the copy's name, `null` if refused) and `WidgetFiles.reorder(order: string[]): Promise<boolean>`; both `flush()` first and run through `enqueue` like `rename`; on success `setStatuses(ordered(list))`, `invalidate()`, `on.changed()`; a refusal goes to `on.error` through the queue's existing error path.
  - `PackPile`: menu entry `ui-pile-menu-duplicate` (not on `_commun`); `onItemKey`: `mod+d` (`e.metaKey || e.ctrlKey`, key `d`) duplicates, `Alt+ArrowUp` / `Alt+ArrowDown` move the item one place (not `_commun`, no move past either end); `SlidePile` gets `onMove={(id, slot) => …}`; after a duplicate `onSelect(copy)` and focus `{ item: copy }`; after a move the focus stays on the moved item (`setRefocus({ item: name })`).
  - Keys (en / fr): `ui-pile-menu-duplicate = Duplicate` / `Dupliquer`; `ui-pack-pile-hint = Drag to move · ⌥↑ ⌥↓ · Double-click or F2 to rename` / `Glisser pour déplacer · ⌥↑ ⌥↓ · Double-clic ou F2 pour renommer` — `PackPile` passes it as `hint` and `ui-widgets-rename-hint` stays only where still used (`rg` it; delete it from all locales if unused).

- [ ] **Step 1: Write the failing render tests** in `PackPile.render.test.tsx`, reading its existing setup (fake `files`, `snap` with `layout-a`, `layout-b`, `layout-c`) first. One behaviour each:
  - `PackPile menu Dupliquer duplicates the item and selects the copy` (fake `files.duplicate` resolves `"a-2"`; `onSelect` called with `"a-2"`).
  - `PackPile ⌘D on a thumbnail duplicates it`.
  - `PackPile shared row menu has no Dupliquer`.
  - `PackPile ⌥↓ on the first item reorders it after the second` (`files.reorder` called with `["b","a","c"]`).
  - `PackPile ⌥↑ on the first item does nothing`.
  - `PackPile drop of an item at slot 3 sends it last` (call the `onMove` the pile passes — drive it through `SlidePile`'s pointer events as `SlidePile`'s own drag tests do, or assert through a `SlidePile` render test if `PackPile` cannot reach it: `reorder` called with `["b","c","a"]` for `a` to slot 3).
  - `PackPile after ⌥↓ keeps the focus on the moved item` (after the re-render with the new `snap` order, `document.activeElement` is the thumbnail named `a`).
  And in the `useWidgetFiles` tests: `useWidgetFiles duplicate sends pending typing first` and `useWidgetFiles reorder refused reports the error and keeps the statuses`.

- [ ] **Step 2: Run them, expect failures.**

Run: `cd ui && bun test src/PackPile.render.test.tsx` (and the `useWidgetFiles` test file)
Expected: FAIL.

- [ ] **Step 3: Implement** as in Interfaces. Moving: `const next = [...order]; next.splice(at, 1); next.splice(slot > at ? slot - 1 : slot, 0, name);` (slot is `SlidePile`'s: 0 = before the first); `⌥↑` is slot `at - 1`, `⌥↓` is slot `at + 2`, as `useAppKeys` does for slides. `ui.ftl` keys in all 20 locales.

- [ ] **Step 4: Run the UI gate.**

Run: `just check` (fmt, clippy, tsc, agents-lint) and `cd ui && bun test`
Expected: PASS.

- [ ] **Step 5: Commit** — `feat(ui,i18n): duplicate and reorder in the pack pile (t-1249, t-1324)`.

---

### Task 6: e2e, help, docs and gates

**Files:**
- Create: `ui/e2e/300-pile-ordre.cjs`
- Modify: `docs/aide/fr.json`, `docs/aide/en.json` (the "Éditer un pack" text, `s-grille`, l. 237 in `fr.json`: "on le duplique (menu ou ⌘D), on le déplace (glisser, ⌥↑ ⌥↓) : l'ordre vaut aussi pour les sélecteurs de layouts et de widgets"; read `docs/AGENTS.md` first), `docs/superpowers/plans/2026-10-28-onglets-pack-theme-lot6-ui.md` (decision 6: "Done in t-1249, `spec:2026-10-31-pile-duplicate-reorder-design`"), `ui/AGENTS.md` / crate `AGENTS.md` only if a task created or changed a rule (e.g. "Write a pack's item order through `PackManifest::to_files`, never by hand." in `crates/oku-slides-core/AGENTS.md`)
- Test: the e2e spec

**Interfaces:**
- Consumes: everything above.

- [ ] **Step 1: Write the e2e spec**, after reading `ui/e2e/250-packs.cjs` and `ui/e2e/lib/` for the helpers that open a layout pack tab and the "Nouvelle slide" picker (labels are French). It: opens a library layout pack in its tab (create one through the menu entry "Nouveau pack de layouts" if no helper opens an existing one, and add two layouts `a` and `b` through "Nouveau layout"), duplicates `a` with ⌘D (the pile shows `a`, `a-2`, `b`), moves `b` first with ⌥↑ twice, closes the tab updating the deck, opens "Nouvelle slide" in the deck and checks the layout cards of that pack read `b`, `a`, `a-2`.

- [ ] **Step 2: Run it.**

Run: `just e2e 4999`
Expected: PASS, every earlier spec still green.

- [ ] **Step 3: Help and docs** as listed; `just agents-lint`.

- [ ] **Step 4: Full gate.**

Run: `just ci` (background; log under `.superpowers/sdd/2026-10-31-pile-duplicate-reorder/`)
Expected: PASS.

- [ ] **Step 5: Commit** — `test(ui,docs): pile duplicate and reorder e2e and help (t-1249, t-1325)`; then close t-1249 (`oku cli task done --project slider --id t-1249 --result "…"`).
