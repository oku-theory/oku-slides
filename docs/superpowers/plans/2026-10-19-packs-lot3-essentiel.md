# Typed packs, lot 3 — layout packs and the built-in `essentiel` pack — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The six slide widgets leave the Typst core for a built-in *layout* pack `essentiel` (kinds `essentiel/title` … `essentiel/blank`) pinned in a new `deck.layouts`. Templates and Grilles are deleted. The atelier becomes a pack mode that edits a layout's example, and the retrofit import projects onto the profiles that each chosen layout pack declares.

**Architecture:** A pack has a kind, `PackKind::{Widgets, Layouts}`, carried by `PackFiles`. Each kind has its own manifest file (`widgets.json` / `layouts.json`), files directory, zip directory and library directory. A pack holds only widgets of its kind's category: a file of the other category is dropped at the probe. The core registers no widget, so the probe loses its `base` key. `oku-slides-render` embeds `base` and `essentiel` the same way, and the built-in registry is `base` + `essentiel`. The trust rule of lot 2 (shipped versions, `versions.txt`) covers both built-ins. `Deck.layouts: BTreeMap<Namespace, PackPin>` mirrors `Deck.widgets`. `Op::AddPack` and `Op::RemovePack` take a `kind`. A layout manifest declares `profiles`, named presets of one of its layouts plus fit hints, and the retrofit scores them across every target layout pack. `Template`, `PackManifest.templates`, `Slide.name/description` and `Op::SetSlideField` are deleted. The vocabulary moves from "templates" to "pack(s)" end to end: requests, MCP tools, snapshot, UI, i18n. Decks move to version 8. The replica format, the sources format and the worker protocol are bumped.

**Tech Stack:** Typst (embedded), Rust (Snafu, serde, speculoos, rstest, insta), Fluent (20 locales), React/TS (Bun), Playwright e2e.

**Spec:** `docs/superpowers/specs/2026-10-18-packs-layouts-widgets-themes-design.md` (§1, §2, §3, §5, §6, §8, §9 item 3). Previous plans, for what exists and their deviations: `docs/superpowers/plans/2026-10-18-packs-lot1-widget-packs.md`, `docs/superpowers/plans/2026-10-18-packs-lot2-base.md`.

**Task:** parent t-1008; sub-tasks t-1059 … t-1068 (Task 1 … Task 10).

## Global Constraints

- **`essentiel`**
  - Namespace `essentiel`, kind `layouts`. Layouts `title`, `toc`, `chapter`, `content`, `end`, `blank` → kinds `essentiel/title`, `essentiel/toc`, `essentiel/chapter`, `essentiel/content`, `essentiel/end`, `essentiel/blank`.
  - Manifest `typst/oku-slides/packs/essentiel/layouts.json`: `id: "d5eb8a57-07b4-47f7-a818-dde5347160cf"` (a **new** id: the old Essentiel id `6f3c1e2a-…` belongs to a template pack that user libraries may still hold), `name: "Essentiel"`, `namespace: "essentiel"`, `requires: ["base"]`, `parent: null`, `created: "2026-10-19T00:00:00Z"`, library scope, `examples` and `profiles` as specified in Task 3.
- **`base`**: unchanged, files and version alike. `typst/oku-slides/packs/base/versions.txt` keeps its single line, so t-1055 stays open.
- **Pack kinds**

  | Kind | Manifest | Files dir | Zip dir | Library dir | Category |
  |---|---|---|---|---|---|
  | `widgets` | `widgets.json` | `widgets/` | `widgets/<hex>/` | `<data>/widgets` | `element` |
  | `layouts` | `layouts.json` | `layouts/` | `layouts/<hex>/` | `<data>/layouts` | `slide` |

  - The canonical content (and so the version) of a `widgets` pack is unchanged.
  - The canonical content of a `layouts` pack starts with `layouts\0`.
- **Namespaces**
  - A namespace is unique across both kinds: in the library (`NamespaceTaken`) and in a deck (`PackKindConflict`).
  - Reserved: `deck` (the deck's own widget pack, lot 1) and the new `deck-layouts` (the deck's own layout pack).
- **Format bumps**
  - `DECK_VERSION` 7 → 8, `REPLICA_FORMAT` 5 → 6, `SOURCES_VERSION` 11 → 12, worker `PROTOCOL_VERSION` 4 → 5, resume state `VERSION` 1 → 2.
  - No migration (MVP rule, precedent t-532). The "newer version" tests move from 8 to 9.
  - Every deck JSON fixture, help deck and test literal follows: version 8, qualified slide kinds.
- **Golden render test**: every page is pixel-identical (`crates/oku-slides-render/tests/snapshots/golden__reference_pages.snap` unchanged). Any diff is a bug, never a re-record. `golden__reference_boxes.snap` changes only where a box names a slide kind.
- **Component tokens** (`component.title-slide`, `component.chapter`, `component.toc`, …) stay in `default-tokens.json` and the themes until lot 4.
- **Kind literals**
  - An essentiel kind written in Rust comes only from `oku_slides_core::kinds::essentiel`, and in TS only from `ui/src/builtinKinds.ts`.
  - A base kind comes from `kinds::base` / `builtinKinds.ts` as in lot 2.
  - A qualified kind is otherwise built only by `qualify` and read only by `split_kind`.
- **Errors and code rules**
  - No `unwrap`/`expect` outside tests, and no silent fallback.
  - Snafu errors with a backtrace per variant.
  - Every new, renamed or deleted Fluent key changes in all 20 locales (`crates/oku-slides-i18n/src/locales/*/`), at the same position, translated.
- **Tests**
  - Rust unit tests go in a sibling `x_tests.rs`, use speculoos, are named `unit_scenario_result`, and use rstest for tables.
  - Regressions go in `tests/regressions/issue_NNNN_topic.rs`, declared in that crate's `tests/regressions/main.rs` (NNNN = the sub-task number). A regression is first seen failing for its stated reason.
- **Docs**
  - Every pub item is documented, first sentence under 15 words. Code comments are in French; docs in English.
  - Source files stay under 500 lines when possible, never above 1000. `templates_mode.rs` is split by concept when it becomes `pack_mode*.rs`.
  - Each `ARCHITECTURE.md` and `AGENTS.md` change lands in the task that changes the code it describes. `ARCHITECTURE.md` stays ≤ 50 lines.
- **Commits**
  - Format `type(scope) : description (t-N)`, message in a scratchpad file passed with `git commit -F`, **no `Co-Authored-By` line**.
  - SDD protocol: `git config --worktree oku.review deferred` during execution, removed at the end. Implementers only stage and write `<workspace>/task-N-commit.txt`. The controller commits, with the trailer `Review-waived: revue SDD par tâche et revue finale (oku.review=deferred, t-1008)`.
  - The squash commit carries a `BREAKING CHANGE:` footer naming:
    - deck version 8, replica format 6, sources 12, worker protocol 5;
    - the refusal of version-7 decks and the setting aside of version-7 tabs (with a notice);
    - templates and Grilles deleted;
    - the renamed MCP tools and requests;
    - retrofit `--pack` taking layout pack namespaces;
    - user library template packs (no widgets) listed as empty widget packs.
- Work deferred to lots 4–5 is tracked under t-1008 (its body lists the lots). A code comment that defers names `t-1008`.
- Runs outside `just e2e` set `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS` to a fresh temp dir.

## Deviations from the spec (approved with this plan; spec §2, §3, §6, §8 and §9 updated in the same commit)

1. **`deck.layouts` is a map of `PackPin`s**, like `deck.widgets` (lot 1, deviation 2), not a `Vec<PackRef>`. `requires` of either kind names **widget** namespaces only. `AddPack` checks them against `deck.widgets`.
2. **`Op::AddPack { kind, namespace, pin }` / `Op::RemovePack { kind, namespace }`.**
   - The JSON op gains a required `kind` (`"widgets"` | `"layouts"`).
   - A namespace already pinned under the other kind is refused (`OpErrorKind::PackKindConflict`, `op-pack-kind-conflict`).
   - `essentiel` can be removed when unused (`PackInUse` guards it). Only `base` keeps `BaseRequired`.
3. **A pack's kind fixes its widgets' category.** A `layouts` file declaring `category: "element"`, or a `widgets` file declaring `"slide"`, is dropped at assembly with `SchemaFault::PackCategory` (`widget-fault-pack-category`). It appears in the pack's diagnostics like any broken file.
4. **The core registers nothing, so the probe loses `base`.**
   - Deviation 12 of lot 2 is resolved by **deleting** the probe key, not renaming it: the probe is `{"packs": [{namespace, kind, widgets}]}`.
   - `core_probe` / `core_registry` are deleted. A registry without packs is `WidgetRegistry::default()`.
   - Lot 2's `builtin_probe` (core + `base`) becomes `base_probe`: `base` alone, beside which every other pack is probed, as before.
   - `builtin_registry` is `base` + `essentiel`.
5. **One fixture, one snapshot per built-in pack.**
   - `fixtures/widgets/base-registry.json` becomes `fixtures/widgets/builtin-registry.json`, the probe of `base` + `essentiel`, enforced against the live registry.
   - The render snapshots are split into `widgets__pack_base.snap` and `widgets__pack_essentiel.snap` (spec §8, after lot 2 deviation 12).
6. **Templates are replaced by layout examples and, for import, by layout profiles.**
   - `PackManifest.examples` (it already exists) gives each layout's starting values.
   - The slide picker lists every layout of every embedded layout pack, seeded from its example.
   - A layout manifest also declares `profiles`, named presets of one of its layouts with fit hints (`role`, `arrangement`, `blocks`, `title`, `kicker`). The retrofit scores the profiles of **every target layout pack**, in the order given (user decision). The picker does not show profiles (YAGNI until lot 5).
   - `essentiel`'s ten profiles reproduce today's ten templates, so its retrofit choices do not change.
7. **Pack mode replaces templates mode** (user decision).
   - `Mode::Templates` → `Mode::Pack`.
   - For a layout pack, the working deck has one slide per layout (id `layout-<name>`), seeded from its example. Each op on a layout slide writes `examples[<name>]` at once, as the session version of the pack (one undo step, kept for resume), and re-seeds the slide; saving only saves the manifest (name, theme draft) (amended in execution, ruling R14).
   - Structural slide ops (`add_slide`, `remove_slide`, `move_slide`, `set_kind` on a slide) are refused in pack mode (`SessionErrorKind::PackModeSlides`, `session-pack-mode-slides`).
   - Writing or removing a layout file realigns the working deck.
   - A widget pack's working deck has no slides.
   - The `host-template` settings surface (template name, description, grid) is deleted. The slide's own settings panel edits its example.
8. **Renamed vocabulary** (user decision).

   | Area | Before | After |
   |---|---|---|
   | Requests | `Templates`, `TemplateSets`, `EditTemplates`, `SaveTemplates`, `CloseTemplates`, `DeleteTemplateSet` | `Layouts`, `Packs`, `EditPack`, `SavePack`, `ClosePack`, `DeletePack` |
   | MCP tools | `templates`, `edit_templates`, `save_templates`, `close_templates`, `delete_template_set` | `packs`, `edit_pack`, `save_pack`, `close_pack`, `delete_pack` |
   | Session types | `TemplateMode`, `TemplatesView` | `PackMode`, `LayoutsView` |
   | Workspace | `TabInfo.editing_templates` | `TabInfo.editing_pack` (`#[serde(default)]`) |
   | Library kind | `Templates` | `Widgets`, plus a new `Layouts` kind |

   - The UI files whose role changes are renamed with them (Task 9).
9. **The retrofit targets layout packs, and imports have no child pack.**
   - `ImportTarget::Retrofit { pack, theme }` → `{ layouts: Vec<String>, theme }`, each a layout pack id, folder or name. Empty means `[essentiel]`. The CLI's repeatable `--pack` does the same.
   - The import dialog offers a multi-select of the library's layout packs that have profiles.
   - The retrofit child pack (`import/child_pack.rs`, a deck-only copy of the target template pack) is deleted. Widget writes in an imported deck follow the deck-mode rule.
   - `SlideReport.template` → `SlideReport.profile` (`"<namespace>/<profile id>"`).
10. **Deck-own packs by kind.**
    - In deck mode, an element widget write goes to `deck` (widgets) and a slide widget write to `deck-layouts` (layouts, scope `deck`), each created on first write (lot 1, deviation 5).
    - The hi-fi import puts its generated slide widgets in a deck-only `deck-layouts` pack that requires `base`, and pins `base`, `essentiel` (for its fallback `essentiel/blank` slides) and `deck-layouts`.
11. **Trust covers both built-ins.**
    - `is_trusted_pack` = a built-in namespace (`base`, `essentiel`) and a version listed in that pack's `versions.txt`.
    - The current built-in version of either pack resolves in-process (no worker).
    - `deck_pack.rs`'s `builtin_base_version` / `only_builtin_base` become `builtin_version(id)` / `only_builtins`.
    - An edited built-in remains untrusted: t-1053 stays open.
12. **Folded review tickets.**
    - **t-1054**: `templates_requires` becomes `examples_requires` and gets its table test.
    - **t-1056**: the MCP `widgets` description is tested against `kinds::base::ALL` and `kinds::essentiel::ALL`.
    - **t-1057**:
      - (1) a widget preview uses the packs pinned in the working deck, not the current built-in `base`;
      - (3) an unsaved tab set aside on resume raises a notice;
      - (4) the comment's item: an assembly failure of a set holding a trusted pack is returned as an error, never silently reduced.
      - Item (2) stays open, so the ticket stays open with a comment.
13. **Intermediate commits.**
    - Tasks 1–6 go bottom-up one layer at a time. Each task's gate is the crates it touches: Tasks 1–2 core and i18n, Task 3 render and worker without `tests/import_*.rs`, Task 4 import and render.
    - The workspace is green again from Task 7. `just ci` and `just e2e` run in Task 10.
    - The branch is squash-merged.

---

### Task 1: Core registry — packs have a kind, the probe has no core widgets, `essentiel` kinds

**Files:**
- Create: `crates/oku-slides-core/src/pack_kind.rs` + `pack_kind_tests.rs`
- Modify core:
  - `src/kinds.rs` + `kinds_tests.rs` (`pub mod essentiel`, `is_builtin_kind`)
  - `src/widget.rs:225-262` (`Probe` without `base`, `ProbedPack.kind`, category check) + `widget_tests.rs`
  - `src/error/mod.rs` (`SchemaFault::PackCategory`)
  - `src/lib.rs` (modules, re-exports, crate doc list)
  - `src/test_registry.rs` (the `test` pack replaces pushes into `base[]`)
  - `tests/common/mod.rs`
- Move: `fixtures/widgets/base-registry.json` → `fixtures/widgets/builtin-registry.json`, rewritten as `{"packs": [{"namespace": "base", "kind": "widgets", "widgets": [the seven element schemas, unchanged]}, {"namespace": "essentiel", "kind": "layouts", "widgets": [the six slide schemas, unchanged, bare names]}]}`
- Modify fixture loaders (path only, plus a `test` pack where they pushed into `base[]`):
  - `crates/oku-slides-core/src/{test_registry.rs,validate_tests.rs}`, `tests/{grid.rs,common/mod.rs}`, `tests/regressions/issue_0591*`
  - `crates/oku-slides-import/{src/test_support.rs,tests/common/mod.rs}`
  - `crates/oku-slides-session/{src/test_support.rs,tests/common/mod.rs}`
  - `crates/oku-slides-cli/tests/{import.rs,cli.rs}`
  - `crates/oku-slides-render/{src/tables_tests.rs,src/charts_tests.rs,tests/widgets.rs}` (`rg -l base-registry crates`)
- Modify every core test that names a slide kind (survey: `"content"` ~97, `"blank"` ~20, `"title"` ~4 as a kind, `"end"` 3, `"toc"` 1). Heaviest files:
  - `src/replica/tree/deck_tree_tests.rs`, `tests/ops.rs`, `src/replica/mod_tests.rs`, `src/replica/read_tests.rs`, `src/replica/tree_tests.rs`, `src/validate_tests.rs`, `src/widget_tests.rs`
  - Rust → `kinds::essentiel::*`; JSON literals → `"essentiel/<name>"`.
- Modify i18n: `*/core.ftl` (`widget-fault-pack-category`, after `widget-fault-slide-sizing`, 20 locales)
- Modify: `crates/oku-slides-core/ARCHITECTURE.md` (pack kinds, probe shape; ≤ 50 lines), `crates/oku-slides-core/AGENTS.md` (rule on the probe shape if one names `base`)

**Interfaces:**
- Produces:
  - `pub enum PackKind { Widgets, Layouts }` (serde `"widgets"` / `"layouts"`, `Ord`, `Copy`), with:
    - `manifest_file() -> &'static str` (`widgets.json` / `layouts.json`)
    - `files_dir() -> &'static str` (`widgets` / `layouts`)
    - `category() -> Category` (`Element` / `Slide`)
    - `ALL: [PackKind; 2]`
  - `Probe { packs: Vec<ProbedPack> }`, `ProbedPack { namespace, kind: PackKind, widgets }`, both `deny_unknown_fields`. A probe still carrying `base` is unreadable (`WidgetErrorKind::Unreadable`).
  - `SchemaFault::PackCategory`: a widget whose `category != kind.category()`. The error names the qualified widget, so `assemble_set` drops it (Task 3).
  - `pub mod kinds::essentiel { NAMESPACE, TITLE, TOC, CHAPTER, CONTENT, END, BLANK, ALL: [&str; 6], fn namespace() -> Namespace }` and `pub fn kinds::is_builtin_kind(kind: &str) -> bool` (in `base::ALL` or `essentiel::ALL`).
  - `test_registry::registry()` reads `builtin-registry.json`. `with_list()` adds its widgets in a pack `{"namespace": "test", "kind": …}` matching each widget's category: the `chrono` slide widget becomes `test/chrono` in a `layouts` pack, element test widgets go in a `widgets` pack named `test-el`.

- [ ] **Step 1: Write the failing tests**

`pack_kind_tests.rs`:

```rust
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use crate::{Category, PackKind};

#[rstest]
#[case(PackKind::Widgets, "widgets.json", "widgets", Category::Element)]
#[case(PackKind::Layouts, "layouts.json", "layouts", Category::Slide)]
fn pack_kind_names_its_files_and_category(
    #[case] kind: PackKind,
    #[case] manifest: &str,
    #[case] dir: &str,
    #[case] category: Category,
) {
    assert_that!((kind.manifest_file(), kind.files_dir(), kind.category())).is_equal_to((manifest, dir, category));
}

#[rstest]
#[case(json!("widgets"), PackKind::Widgets)]
#[case(json!("layouts"), PackKind::Layouts)]
fn pack_kind_from_json_reads_both(#[case] v: serde_json::Value, #[case] expected: PackKind) {
    assert_that!(serde_json::from_value::<PackKind>(v).unwrap()).is_equal_to(expected);
}

#[test]
fn pack_kind_from_json_other_value_is_refused() {
    assert_that!(serde_json::from_value::<PackKind>(json!("themes"))).is_err();
}
```

`widget_tests.rs` (replace the tests that build a probe with `base`; every `from_probe(json!({"base": …}))` in this file moves its slide widgets into a `{"namespace": "t", "kind": "layouts"}` pack and its element widgets into `{"namespace": "u", "kind": "widgets"}`):

```rust
#[test]
fn registry_from_probe_with_a_base_key_is_unreadable() {
    let err = WidgetRegistry::from_probe(json!({"base": [], "packs": []})).unwrap_err();
    assert_that!(err.kind()).is_equal_to(WidgetErrorKind::Unreadable);
}

#[test]
fn registry_from_probe_without_packs_is_empty() {
    assert_that!(WidgetRegistry::from_probe(json!({"packs": []})).unwrap()).is_equal_to(WidgetRegistry::default());
}

#[rstest]
#[case("layouts", "element")]
#[case("widgets", "slide")]
fn registry_from_probe_widget_of_the_other_category_is_refused(#[case] kind: &str, #[case] category: &str) {
    let probe = json!({"packs": [{"namespace": "acme", "kind": kind,
        "widgets": [{"name": "w", "category": category, "params": []}]}]});
    let err = WidgetRegistry::from_probe(probe).unwrap_err();
    assert_that!((err.kind(), err.widget().map(str::to_owned)))
        .is_equal_to((WidgetErrorKind::BadSchema(SchemaFault::PackCategory), Some("acme/w".to_owned())));
}

#[test]
fn registry_from_probe_pack_without_kind_is_unreadable() {
    let probe = json!({"packs": [{"namespace": "acme", "widgets": []}]});
    assert_that!(WidgetRegistry::from_probe(probe).unwrap_err().kind()).is_equal_to(WidgetErrorKind::Unreadable);
}

#[test]
fn builtin_fixture_slide_widgets_are_essentiel_kinds() {
    let slides: Vec<String> = crate::test_registry::registry()
        .iter()
        .filter(|w| w.category == Category::Slide)
        .map(|w| w.name.clone())
        .collect();
    let mut expected: Vec<String> = kinds::essentiel::ALL.map(str::to_owned).to_vec();
    expected.sort();
    assert_that!(slides).is_equal_to(expected);
}
```

(Use the registry's existing iteration accessor; if none is public, use `kinds::essentiel::ALL.iter().all(|k| registry.get(k).is_some_and(|w| w.category == Category::Slide))` and assert no bare slide kind resolves: `registry.get("content").is_none()`.)

`kinds_tests.rs`:

```rust
#[test]
fn essentiel_kinds_are_qualified_by_the_essentiel_namespace() {
    let names = ["title", "toc", "chapter", "content", "end", "blank"];
    let built: Vec<String> = names.iter().map(|n| qualify(&essentiel::namespace(), n)).collect();
    assert_that!(essentiel::ALL.map(str::to_owned).to_vec()).is_equal_to(built);
}

#[rstest]
#[case(base::CARD, true)]
#[case(essentiel::CONTENT, true)]
#[case("essentiel/kpi", false)]
#[case("content", false)]
#[case("acme/card", false)]
fn is_builtin_kind_knows_only_the_shipped_kinds(#[case] kind: &str, #[case] expected: bool) {
    assert_that!(is_builtin_kind(kind)).is_equal_to(expected);
}
```

- [ ] **Step 2: Run, expect FAIL**: `cargo nextest run -p oku-slides-core` fails to compile (`PackKind`, `kinds::essentiel`).

- [ ] **Step 3: Implement**
  - `pack_kind.rs` with the doc "La sorte d'un pack : ses fichiers et la catégorie de ses widgets.", re-exported from `lib.rs`.
  - `widget.rs`:
    - `Probe` and `ProbedPack` as under Interfaces.
    - In `from_probe`, for each pack, a widget whose `category != pack.kind.category()` fails with `PackCategory` naming `qualify(ns, name)`.
    - `from_probe`'s doc becomes "Lit la sonde `{"packs": [{namespace, kind, widgets}]}` émise par Typst."
    - The module doc no longer points at `src/widgets.typ`.
  - Fixture rewrite: move the six objects of `"base"` verbatim into the `essentiel` pack entry (still bare names: assembly qualifies). Delete the `"base"` key. Add `"kind"` to both entries.
  - Every slide-kind literal in core tests uses the constants.
  - The two survey heavyweights (`deck_tree_tests.rs`, `tests/ops.rs`) use a local `use oku_slides_core::kinds::essentiel::{CONTENT, BLANK};` to keep lines short.

- [ ] **Step 4: Run, expect PASS**: `cargo nextest run -p oku-slides-core -p oku-slides-i18n`. Other crates do not compile until their tasks; that is expected (deviation 13).

- [ ] **Step 5: Commit**: `feat(core)! : packs typés (widgets, layouts), sonde sans widget du noyau, kinds essentiel, registre intégré en fixture (t-1059)`.

---

### Task 2: Core deck — `deck.layouts`, kinded pack ops, templates deleted, formats bumped

**Files:**
- Modify the pack files and manifest in `crates/oku-slides-core/src/pack.rs` (+ `pack_tests.rs`):
  - `PackFiles { kind, manifest, widgets }`; `new`, `from_bytes` and `verified` take `kind` first; `canonical` prefix; `kind()`.
  - `Template` deleted; `PackManifest.templates` deleted; `PackManifest.profiles`.
  - `PackManifest::to_files(kind, widgets)`.
- Create: `crates/oku-slides-core/src/layout_profile.rs` + `layout_profile_tests.rs` (`LayoutProfile`, `SlideRole`, `Arrangement`)
- Modify the model and ops:
  - `src/model.rs`: `DECK_VERSION = 8`; `Deck.layouts`; `Deck::empty`; `Slide.name/description` deleted.
  - `src/model_tests.rs`, `src/canonical.rs:46` + `canonical_tests.rs:424`.
  - `src/ops/mod.rs`: `SlideField` and `Op::SetSlideField` deleted; `AddPack`/`RemovePack` with `kind`.
  - `src/ops/packs.rs` + `packs_tests.rs`.
  - `src/error/mod.rs`: `OpInner::PackKindConflict`, `OpErrorKind::PackKindConflict`.
- Modify the replica: `src/replica/schema.rs:27-29,112-144` (`(Deck,"layouts") => Nested(PackPins)`, `maps()` gains `"layouts"`, slide texts lose `name`/`description`), `src/replica/mod.rs:66-69` (`REPLICA_FORMAT = 6`, doc), `src/replica/ARCHITECTURE.md:50`, and the replica tests that use `SetSlideField` or slide `name`.
- Modify the sources:
  - `src/sources.rs:46` (`SOURCES_VERSION = 12`), `:144-163` (`embedded_issues` loops over both maps and checks the files' kind), `:166` (`add_pack` keeps the kind).
  - `src/sources_packs.rs:21-125` (directory and manifest by kind: `widgets/<hex>/widgets.json`, `layouts/<hex>/layouts.json`, files under `<hex>/<files_dir>/`).
  - `src/sources_packs_tests.rs`, `src/sources_tests.rs` (version 11 joins the refused table).
- Modify validation: `src/validate.rs:43-77` (pin issues for `layouts`, `issue-pack-kind-mismatch` when the embedded files of a pin are of the other kind) + `validate_tests.rs`.
- Modify: `src/lib.rs` (re-exports: drop `Template`, `SlideField`; add `LayoutProfile`, `SlideRole`, `Arrangement`), every `"version": 7` deck literal in core tests (11 sites), the "newer version" tests (`DECK_VERSION + 1`).
- Modify i18n: `*/core.ftl` (`op-pack-kind-conflict` after `op-base-required`; `issue-pack-kind-mismatch` after `issue-pack-pin-mismatch`; 20 locales)
- Modify: `crates/oku-slides-core/ARCHITECTURE.md`, `crates/oku-slides-core/src/replica/AGENTS.md` / `ARCHITECTURE.md` (format 6)

**Interfaces:**
- Consumes: `PackKind` (Task 1).
- Produces:
  - `PackFiles::new(kind: PackKind, manifest, widgets)`, `PackFiles::from_bytes(kind, pack_json, widgets)`, `PackFiles::verified(kind, hex, manifest, widgets)`, `PackFiles::kind() -> PackKind`. `pack_json()` keeps its name (it writes the kind's manifest file).
  - `PackManifest::to_files(&self, kind: PackKind, widgets) -> serde_json::Result<PackFiles>`.
  - `PackManifest.profiles: Vec<LayoutProfile>`, `#[serde(default, skip_serializing_if = "Vec::is_empty")]`, doc "Préréglages d'un layout que l'import retrofit sait choisir (pack de layouts seulement).".
  - `LayoutProfile { id: String, name: String, layout: WidgetFile, role: SlideRole, arrangement: Arrangement, blocks: u8, title: bool, kicker: bool, params: Map<String, Value> }`. `params` and the three hints have serde defaults, and `params` is skipped when empty.
  - `SlideRole { Title, Chapter, Toc, Content, End, Blank }` and `Arrangement { Default, Columns, Split, Center, Quote, Free, Grid }`, both snake_case serde.
  - `Deck.layouts: BTreeMap<Namespace, PackPin>`, skipped when empty, placed after `widgets`.
  - `Op::AddPack { kind: PackKind, namespace, pin }`, `Op::RemovePack { kind: PackKind, namespace }`.
  - `ops::packs::{add, remove}(deck, kind, …)`: work on the map of `kind`. `PackKindConflict` is raised when `namespace` is a key of the other map. Requirements are checked against `deck.widgets` whatever the kind. `remove` keeps its order of checks, with `BaseRequired` only for `(Widgets, base)`. `pack_users` is unchanged (slides and elements already).
  - `Deck::pin(&self, ns) -> Option<(PackKind, &PackPin)>`: the pin of `ns` in either map.
  - `REPLICA_FORMAT = 6`, `SOURCES_VERSION = 12`, `DECK_VERSION = 8`.

- [ ] **Step 1: Write the failing tests**

`ops/packs_tests.rs` (beside the lot 1/2 tests, which gain `kind: PackKind::Widgets`):

```rust
#[test]
fn add_pack_layouts_pins_into_deck_layouts() {
    let mut deck = Deck::empty();
    deck.widgets.insert(kinds::base::namespace(), pin(&[]));
    let op = Op::AddPack { kind: PackKind::Layouts, namespace: kinds::essentiel::namespace(), pin: pin(&["base"]) };
    apply_all(&mut deck, &registry(), vec![op]).unwrap();
    assert_that!((deck.layouts.contains_key(&kinds::essentiel::namespace()), deck.widgets.len())).is_equal_to((true, 1));
}

#[test]
fn add_pack_layouts_requiring_a_missing_widget_pack_is_refused() {
    let mut deck = Deck::empty();
    let op = Op::AddPack { kind: PackKind::Layouts, namespace: kinds::essentiel::namespace(), pin: pin(&["base"]) };
    let err = apply_all(&mut deck, &registry(), vec![op]).unwrap_err();
    assert_that!(err.op_kind()).is_equal_to(Some(OpErrorKind::PackRequirementMissing));
}

#[test]
fn add_pack_layouts_requirement_met_only_by_a_layout_pack_is_refused() {
    let mut deck = Deck::empty();
    deck.layouts.insert(ns("grid"), pin(&[]));
    let op = Op::AddPack { kind: PackKind::Layouts, namespace: ns("acme"), pin: pin(&["grid"]) };
    let err = apply_all(&mut deck, &registry(), vec![op]).unwrap_err();
    assert_that!(err.op_kind()).is_equal_to(Some(OpErrorKind::PackRequirementMissing));
}

#[rstest]
#[case(PackKind::Widgets, PackKind::Layouts)]
#[case(PackKind::Layouts, PackKind::Widgets)]
fn add_pack_namespace_pinned_under_the_other_kind_is_refused(#[case] first: PackKind, #[case] second: PackKind) {
    let mut deck = Deck::empty();
    apply_all(&mut deck, &registry(), vec![Op::AddPack { kind: first, namespace: ns("acme"), pin: pin(&[]) }]).unwrap();
    let err = apply_all(&mut deck, &registry(), vec![Op::AddPack { kind: second, namespace: ns("acme"), pin: pin(&[]) }])
        .unwrap_err();
    assert_that!(err.op_kind()).is_equal_to(Some(OpErrorKind::PackKindConflict));
}

#[test]
fn remove_pack_essentiel_used_by_a_slide_is_refused_in_use() {
    let mut deck = deck_with_essentiel();
    deck.slides.push(Slide::new("s", kinds::essentiel::CONTENT));
    let op = Op::RemovePack { kind: PackKind::Layouts, namespace: kinds::essentiel::namespace() };
    let err = apply_all(&mut deck, &registry(), vec![op]).unwrap_err();
    assert_that!(err.op_kind()).is_equal_to(Some(OpErrorKind::PackInUse));
}

#[test]
fn remove_pack_essentiel_unused_is_removed() {
    let mut deck = deck_with_essentiel();
    let op = Op::RemovePack { kind: PackKind::Layouts, namespace: kinds::essentiel::namespace() };
    apply_all(&mut deck, &registry(), vec![op]).unwrap();
    assert_that!(deck.layouts.is_empty()).is_true();
}

#[test]
fn remove_pack_under_the_wrong_kind_is_unknown_namespace() {
    let mut deck = deck_with_essentiel();
    let op = Op::RemovePack { kind: PackKind::Widgets, namespace: kinds::essentiel::namespace() };
    let err = apply_all(&mut deck, &registry(), vec![op]).unwrap_err();
    assert_that!(err.op_kind()).is_equal_to(Some(OpErrorKind::UnknownNamespace));
}

#[test]
fn pack_kind_conflict_message_is_localized_in_french() {
    let mut deck = deck_with_essentiel();
    let op = Op::AddPack { kind: PackKind::Widgets, namespace: kinds::essentiel::namespace(), pin: pin(&[]) };
    let err = apply_all(&mut deck, &registry(), vec![op]).unwrap_err();
    assert_that!(err.localize(&Localizer::new(Some("fr")))).is_not_equal_to(err.localize(oku_slides_i18n::english()));
}
```

(`deck_with_essentiel()`: `base` in `widgets`, `essentiel` requiring `base` in `layouts`. `ns`, `pin` and `registry` are this file's helpers.)

`pack_tests.rs`:

```rust
#[test]
fn pack_files_layouts_and_widgets_of_equal_content_have_different_versions() {
    let m = manifest_map();
    let w = PackFiles::new(PackKind::Widgets, m.clone(), BTreeMap::new());
    let l = PackFiles::new(PackKind::Layouts, m, BTreeMap::new());
    assert_that!(w.version()).is_not_equal_to(l.version());
}

#[test]
fn pack_files_widgets_canonical_is_unchanged_by_kinds() {
    // Le contenu canonique d'un pack de widgets ne gagne pas de préfixe : `base` garde sa version.
    let files = PackFiles::new(PackKind::Widgets, manifest_map(), BTreeMap::new());
    assert_that!(files.canonical().starts_with(b"{")).is_true();
}

#[test]
fn pack_manifest_with_templates_key_reads_without_templates() {
    let mut m = manifest_map();
    m.insert("templates".into(), json!([{"id": "t", "name": "T", "slide": {"id": "s", "kind": "content"}}]));
    let read = PackManifest::from_files(&PackFiles::new(PackKind::Widgets, m, BTreeMap::new())).unwrap();
    assert_that!(serde_json::to_value(&read).unwrap().get("templates").is_none()).is_true();
}
```

`layout_profile_tests.rs`:

```rust
#[test]
fn layout_profile_round_trips_with_hints_and_params() {
    let src = json!({"id": "colonnes", "name": "Colonnes", "layout": "content", "role": "content",
        "arrangement": "columns", "blocks": 3, "title": true, "kicker": true, "params": {"layout": "columns"}});
    let p: LayoutProfile = serde_json::from_value(src.clone()).unwrap();
    assert_that!(serde_json::to_value(&p).unwrap()).is_equal_to(src);
}

#[test]
fn layout_profile_minimal_defaults_hints() {
    let p: LayoutProfile = serde_json::from_value(
        json!({"id": "fin", "name": "Fin", "layout": "end", "role": "end", "arrangement": "default"})).unwrap();
    assert_that!((p.blocks, p.title, p.kicker, p.params.is_empty())).is_equal_to((0, false, false, true));
}

#[rstest]
#[case(json!("cover"))]
#[case(json!(1))]
fn layout_profile_unknown_role_is_refused(#[case] role: Value) {
    let src = json!({"id": "x", "name": "X", "layout": "end", "role": role, "arrangement": "default"});
    assert_that!(serde_json::from_value::<LayoutProfile>(src)).is_err();
}
```

`model_tests.rs`: `deck_from_json_older_version_is_refused` gains `#[case(7)]`; `deck_from_json_current_version_is_read` and `deck_empty_is_version_7` use 8 (renamed `…_is_version_8`). Add:

```rust
#[test]
fn deck_layouts_round_trip_and_are_omitted_when_empty() {
    let mut deck = Deck::empty();
    let empty = serde_json::to_value(&deck).unwrap();
    deck.layouts.insert(kinds::essentiel::namespace(), pin_for_tests());
    let back = Deck::from_json(&serde_json::to_string(&deck).unwrap()).unwrap();
    assert_that!((empty.get("layouts").is_none(), back.layouts == deck.layouts)).is_equal_to((true, true));
}
```

`replica/read_tests.rs`: `read_deck_of_version_7_returns_unsupported_version`, beside the lot 2 version-6 test with its setup verbatim. `replica/mod_tests.rs`:

```rust
#[test]
fn replica_layouts_pins_of_two_peers_merge() {
    // Deux pairs ajoutent chacun un pack de layouts : les deux restent (une feuille LWW par espace de noms).
    let (a, b) = two_peers_on(deck_with_base());
    a.apply(vec![Op::AddPack { kind: PackKind::Layouts, namespace: ns("un"), pin: pin(&[]) }]).unwrap();
    b.apply(vec![Op::AddPack { kind: PackKind::Layouts, namespace: ns("deux"), pin: pin(&[]) }]).unwrap();
    let merged = merge(&a, &b);
    assert_that!(merged.layouts.keys().map(Namespace::as_str).collect::<Vec<_>>()).is_equal_to(vec!["deux", "un"]);
}
```

(Use the replica test helpers this file already has for two peers; the names above are placeholders for them, not new API.) `replica/mod_tests.rs`: a replica whose `meta.format` is 5 is refused with `replica-unsupported-version` (the existing format test's table gains 5).

`sources_packs_tests.rs`:

```rust
#[test]
fn sources_layouts_pack_round_trips_under_layouts_dir() {
    let bundle = bundle_with(PackKind::Layouts, "essentiel", &[("content", "#let widget = none")]);
    let zip = bundle.to_zip().unwrap();
    let names = zip_entry_names(&zip);
    let back = SourceBundle::from_zip(&zip).unwrap();
    assert_that!(names.iter().any(|n| n.starts_with("layouts/") && n.ends_with("/layouts.json"))).is_true();
    assert_that!(back.packs.values().next().map(PackFiles::kind)).is_equal_to(Some(PackKind::Layouts));
}

#[test]
fn sources_pin_of_layouts_on_widget_files_is_an_issue() {
    let mut bundle = bundle_with(PackKind::Widgets, "acme", &[]);
    let pin = bundle.deck.widgets.remove(&ns("acme")).unwrap();
    bundle.deck.layouts.insert(ns("acme"), pin);
    let keys: Vec<String> = bundle.embedded_issues().iter().map(|i| i.key().to_owned()).collect();
    assert_that!(keys).contains("issue-pack-kind-mismatch".to_owned());
}
```

(`bundle_with`, `zip_entry_names`: helpers beside the lot 1 ones in this file. Read their real names before writing.)

- [ ] **Step 2: Run, expect FAIL**: `cargo nextest run -p oku-slides-core`.

- [ ] **Step 3: Implement**
  - `PackFiles` changes:
    - Add the `kind` field.
    - `canonical()` pushes `b"layouts\0"` first for `Layouts` only.
    - `PackVersion::of` reads `canonical()`, so nothing else changes.
    - `scope()` is unchanged.
  - Every `PackFiles::new` / `from_bytes` call in core passes the kind: the sources reader passes the kind of the directory it reads from.
  - Delete `Template` and the `templates` field. A manifest with a `templates` key still parses (no `deny_unknown_fields`). Its version is unchanged, since the manifest map is kept as received, but the key is never written by `to_files`.
  - `Slide`: delete `name`, `description`, `SlideField`, `SetSlideField` and its apply arm. `canonical.rs:46` and `replica/schema.rs:120,132` keep only `notes`.
  - Packs and replica:
    - `ops/packs.rs`: parametrize by kind through a private `fn map_mut(deck, kind) -> &mut BTreeMap<Namespace, PackPin>`. The conflict check comes first in `add`; `remove` is unchanged except `BaseRequired` only under `Widgets`.
    - Replica schema: add `(Deck, "layouts") => Kind::Nested(Ctx::PackPins)` and `"layouts"` to `maps()` for `Ctx::Deck`.
    - Set `REPLICA_FORMAT = 6`. Its doc line becomes "6 : `layouts` ; plus de nom ni de description de slide.", and the "change `SOURCES_VERSION` with it" note is kept.
  - Sources: `PACKS_DIR`, `PACK_JSON` and `WIDGETS_DIR` give way to `PackKind::ALL` loops.
    - A zip entry under `layouts/<hex>/` is read as `Layouts`.
    - `embedded_issues` checks each map's pins against their files: same id, namespace and requires as today, plus `files.kind() == map kind`, else `issue-pack-kind-mismatch` (args `pack`).
    - Requirement closure is checked against `deck.widgets` for both maps.
  - `DECK_VERSION = 8`. Every core deck literal `"version": 7` → 8.

- [ ] **Step 4: Run, expect PASS**: `cargo nextest run -p oku-slides-core -p oku-slides-i18n`.

- [ ] **Step 5: Commit**: `feat(core)! : deck.layouts, ops de packs par sorte, gabarits et métadonnées de slide modèle supprimés, deck 8, replica 6, sources 12 (t-1060)`.

---

### Task 3: Typst core without slide widgets, the `essentiel` pack, render and worker by pack kind

**Files:**
- Typst library (`typst/oku-slides`):
  - Delete: `src/widgets.typ`.
  - Create: `packs/essentiel/layouts.json`, `packs/essentiel/layouts/{title,toc,chapter,content,end,blank}.typ`, `packs/essentiel/_commun.typ` only if two layouts share a helper (today `_fields(ctx)`: put it there), `packs/essentiel/versions.txt`.
  - Modify: `src/data.typ:25-38,151,188-207` (no default registry; `widgets` required; unknown slide fallback kept), `examples/from-json.typ` (imports the two packs' files and qualifies by hand), `examples/deck.json` (version 8, `essentiel/*`), `README.md` (slide widgets come from `essentiel`; core registers none), `typst/oku-slides/AGENTS.md`, `typst/oku-slides/ARCHITECTURE.md` if present.
- Rename in render: `crates/oku-slides-render/src/base_pack.rs` → `src/builtin_packs.rs` (+ tests).
- Modify render:
  - `src/registry.rs` (`PackWidgets.kind`, `assembly`/`path`/`sources` by kind, `fingerprint` includes kind, `probe_source` without `base`, `core_*` deleted, `base_probe`, `builtin_registry`)
  - `src/widget_probe.rs` (`probe_file` and `probe_key` take the kind; `assemble_set(fixed: &Json, …)` reads only `packs`)
  - `src/world.rs:33-125` (no `widgets.typ` in `LIBRARY`; `main_source` imports only pack assemblies) + `world_tests.rs`
  - `src/lib.rs:205-219` (`Renderer::new` = no packs; `with_builtins` = `base` + `essentiel`), `src/error.rs:228`, `src/wire.rs:85` ("invalid pack identifier")
  - `ARCHITECTURE.md`, `AGENTS.md`
- Modify render tests:
  - `tests/widgets.rs:146-170` (fixture enforced against `builtin_probe`; snapshots per pack), `tests/snapshots/widgets__base_registry.snap` → `widgets__pack_base.snap` + `widgets__pack_essentiel.snap`, `tests/golden.rs` and `tests/golden/widgets.json` (version 8, `essentiel/*`), `fixtures/roadmap.json` (version 8, `essentiel/*`)
  - every render test with a bare slide kind: `tests/{render,fields,tables,anchor_path,text_indent,widget_files,widgets}.rs`, `tests/regressions/*`, `src/{charts,tables}_tests.rs`
  - `tests/common/mod.rs:56-61`
- Modify worker:
  - `crates/oku-slides-render-worker/src/protocol.rs:10-21` (`PROTOCOL_VERSION = 5`, `WirePack.kind`), `src/pack_probe.rs:1-133` (`base` on an empty probe, others beside `base_probe`), `src/client.rs:48,141-153,268-272` (`is_trusted_pack` from `builtin_packs`), `src/serve.rs`, `src/link.rs`
  - `src/test_packs.rs:46-48` and bare slide kinds in `client_tests.rs`, `pack_set_tests.rs`
  - `ARCHITECTURE.md`, `AGENTS.md` (the trusted-pack rule names both built-ins)

**Interfaces:**
- Consumes: `PackKind`, probe shape, `kinds::essentiel` (Task 1); `PackFiles::kind` (Task 2).
- Produces:
  - `PackWidgets::new(id, namespace, kind: PackKind, files)` and `PackWidgets::kind()`:
    - `path()` is `/packs/<id>/<kind.files_dir()>.typ`;
    - sources sit under `/packs/<id>/<kind.files_dir()>/<f>`;
    - `fingerprint` hashes the kind after the namespace.
  - `pub enum BuiltinPack { Base, Essentiel }` in `builtin_packs.rs`, with:
    - `ALL`;
    - `namespace()`;
    - `kind()`;
    - `files() -> Result<PackFiles, RegistryError>`;
    - `widgets() -> Result<PackWidgets, RegistryError>`;
    - `shipped_versions() -> impl Iterator<Item = &'static str>`.
  - Lot 2's `base_pack_files()` / `base_pack_widgets()` are replaced by `BuiltinPack::Base.files()` / `.widgets()` (callers updated). `pub fn builtin_pack_set() -> Result<PackSet, RegistryError>` returns `base` + `essentiel`.
  - `pub fn is_trusted_pack(p: &PackWidgets) -> bool`: some `BuiltinPack` has `p.namespace()` and its `shipped_versions()` contains `p.id()`.
  - `pub fn base_probe(fonts) -> Result<Json, RegistryError>`: the probe of `base` alone, cached.
  - `pub fn builtin_registry(fonts) -> Result<WidgetRegistry, RegistryError>`: `base` + `essentiel`, cached.
  - `core_probe` and `core_registry` are deleted. Callers use `WidgetRegistry::default()` for "no packs".
  - `probe_file(fonts, kind: PackKind, name, source, common)` and `probe_key(kind, name, source, common)`.
  - Worker: `WirePack { id, namespace, kind, files }`. A pack whose namespace is `base` is probed on `{"packs": []}`, any other beside `base_probe`.

- [ ] **Step 1: Write the failing tests**

```rust
// crates/oku-slides-render/src/builtin_packs_tests.rs
#[rstest]
#[case(BuiltinPack::Base, "base", PackKind::Widgets, 7)]
#[case(BuiltinPack::Essentiel, "essentiel", PackKind::Layouts, 6)]
fn builtin_pack_files_have_their_namespace_kind_and_widgets(
    #[case] pack: BuiltinPack,
    #[case] namespace: &str,
    #[case] kind: PackKind,
    #[case] widgets: usize,
) {
    let files = pack.files().unwrap();
    let manifest = PackManifest::from_files(&files).unwrap();
    let count = files.widgets().keys().filter(|f| !f.is_common()).count();
    assert_that!((manifest.namespace.as_str(), files.kind(), count)).is_equal_to((namespace, kind, widgets));
}

#[rstest]
#[case(BuiltinPack::Base)]
#[case(BuiltinPack::Essentiel)]
fn builtin_pack_current_version_is_listed_as_shipped(#[case] pack: BuiltinPack) {
    let hex = pack.files().unwrap().version().to_hex();
    assert_that!(pack.shipped_versions().any(|v| v == hex)).is_true();
}

#[test]
fn essentiel_manifest_has_the_lot_3_id_and_requires_base() {
    let m = PackManifest::from_files(&BuiltinPack::Essentiel.files().unwrap()).unwrap();
    assert_that!((m.id.to_string(), m.requires.clone()))
        .is_equal_to(("d5eb8a57-07b4-47f7-a818-dde5347160cf".to_owned(), vec![kinds::base::namespace()]));
}

#[test]
fn essentiel_profiles_are_the_ten_former_templates() {
    let m = PackManifest::from_files(&BuiltinPack::Essentiel.files().unwrap()).unwrap();
    let ids: Vec<&str> = m.profiles.iter().map(|p| p.id.as_str()).collect();
    assert_that!(ids).is_equal_to(vec![
        "couverture", "sommaire", "chapitre", "texte", "cartes", "colonnes", "citation", "message", "libre", "fin",
    ]);
}

#[test]
fn is_trusted_pack_an_edited_essentiel_is_not_trusted() {
    let shipped = BuiltinPack::Essentiel.widgets().unwrap();
    let edited = PackWidgets::new("abc123", shipped.namespace().clone(), PackKind::Layouts, shipped.files().clone()).unwrap();
    assert_that!((is_trusted_pack(&shipped), is_trusted_pack(&edited))).is_equal_to((true, false));
}

#[test]
fn is_trusted_pack_another_namespace_with_a_shipped_id_is_not_trusted() {
    let shipped = BuiltinPack::Base.widgets().unwrap();
    let claim = PackWidgets::new(shipped.id(), ns("acme"), PackKind::Widgets, shipped.files().clone()).unwrap();
    assert_that!(is_trusted_pack(&claim)).is_false();
}
```

(Keep the order of `essentiel_profiles_are_the_ten_former_templates` equal to the template order of `templates/essentiel/pack.json` at the start of this task. Read it, and fix the expected list if the file differs.)

```rust
// crates/oku-slides-render/tests/widgets.rs
#[test]
fn builtin_registry_fixture_matches_the_live_registry() {
    let live = oku_slides_render::builtin_probe_for_tests(&fonts()).unwrap();
    let fixture: Value = serde_json::from_str(include_str!("../../../fixtures/widgets/builtin-registry.json")).unwrap();
    assert_that!(live).is_equal_to(fixture);
}

#[rstest]
#[case(BuiltinPack::Base, "pack_base")]
#[case(BuiltinPack::Essentiel, "pack_essentiel")]
fn builtin_pack_registry_snapshot(#[case] pack: BuiltinPack, #[case] name: &str) {
    let registry = load_registry(&PackSet::new(vec![pack.widgets().unwrap()]).unwrap()).unwrap();
    insta::assert_json_snapshot!(name, registry);
}

#[test]
fn a_layouts_pack_widget_declaring_element_is_dropped_with_a_diagnostic() {
    let pack = pack_of(PackKind::Layouts, "acme", &[("w", "#import \"/oku-slides/lib.typ\": *\n#let widget = widget(\"w\", category: \"element\", params: (), (ctx, args) => [])")]);
    let probed = probe_pack(&pack);
    assert_that!(probed.diagnostics_of("w").iter().any(|d| d.message.contains("category"))).is_true();
    assert_that!(probed.registry.get("acme/w")).is_none();
}

#[test]
fn render_with_no_packs_draws_the_unknown_slide_frame() {
    let deck = json!({"version": 8, "slides": [{"id": "s", "kind": "essentiel/title", "params": {}}]});
    let svg = Renderer::new(&fonts()).unwrap().render_svg(&deck, 0).unwrap();
    assert_that!(svg.contains("essentiel/title")).is_true();
}
```

(`builtin_probe_for_tests`: the raw probe JSON of `base` + `essentiel` exposed under `#[cfg(any(test, feature = "test-support"))]`, or the function this file already uses to compare the lot 2 fixture. Reuse it. `pack_of` / `probe_pack` / `diagnostics_of` / `render_svg`: this file's or `tests/common`'s existing helpers. Read them and adapt the names; the asserted behavior is fixed.)

```rust
// crates/oku-slides-render/src/world_tests.rs (replacing the assertions of :32,39)
#[test]
fn main_source_imports_no_core_widgets() {
    let src = main_source(&PackSet::new(vec![BuiltinPack::Essentiel.widgets().unwrap()]).unwrap());
    assert_that!((src.contains("src/widgets.typ"), src.contains("/layouts.typ"))).is_equal_to((false, true));
}
```

Worker (`client_tests.rs`): `set_widgets_that_loops_beside_trusted_base_marks_the_looping_pack_faulty` uses `[base, essentiel, boucle]`. Add `a_bounded_failure_with_both_builtins_alone_marks_nothing_faulty` (lot 2 test with `base` + `essentiel`). `protocol` test: a `Hello` of version 4 is refused.

- [ ] **Step 2: Run, expect FAIL**: `cargo nextest run -p oku-slides-render -p oku-slides-render-worker -E 'not binary(/^import_/)'`.

- [ ] **Step 3: Implement**
  - **Typst.**
    - Move each `_title`, `_toc`, `_chapter`, `_content`, `_end`, `_blank` of `widgets.typ` verbatim into `packs/essentiel/layouts/<name>.typ` as `#let widget = widget("<name>", category: "slide", …)`, importing what it uses from `/oku-slides/lib.typ`. Add to `lib.typ`'s exports any helper the layouts need that is not exported yet (`zone-marker`, `free-area`, `default-grid` and the slide functions already are).
    - `_fields` goes to `_commun.typ` (imported `#import "_commun.typ": fields`).
    - `data.typ`: delete the `widgets.typ` import and `_base`. `render-deck` / `render-slide` take `widgets` without default (`widgets: none` is an `assert` with "render-deck: widgets is required"). The unknown slide frame is unchanged.
  - **Manifest `packs/essentiel/layouts.json`.** Global Constraints fields, plus:
    - **`examples`**: `title` = params of the template `couverture`, `toc` = `sommaire`, `chapter` = `chapitre`, `content` = `texte`, `end` = `fin`. `blank` has no entry. Copy each template's `slide.params` verbatim (element ids inside slots kept: the picker reissues ids, Task 8).
    - **`profiles`**: one per template, in the pack's template order. For each template:
      - `id` and `name` are the template's;
      - `layout` is its slide kind;
      - `role` comes from the `slide_kind` match of `retrofit/template.rs:114-125`;
      - `arrangement` is what `profile()` (`template.rs:128-166`) derives;
      - `blocks` is the filled slot count;
      - `title` / `kicker` mean the example text is non-empty;
      - `params` = the template slide's `layout` and `grid` params only, when set.

      Compute them by running `profile()` on each template **before** `templates/` is deleted: a throwaway test printing JSON, not committed. Commit only the resulting `layouts.json`.
  - **`versions.txt`**: the version hex of `BuiltinPack::Essentiel.files()`, one line (the test prints it on mismatch).
  - **`builtin_packs.rs`.**
    - Two `include_bytes!` tables (lot 2's `base_widgets!` macro made generic over a directory) and one `include_str!` per manifest and per `versions.txt`.
    - `files()` is `PackFiles::from_bytes(self.kind(), …)`. `widgets()` has id = version hex, namespace from the manifest, and its kind.
  - **Registry.**
    - `probe_source` drops `base:` and emits `kind:` per pack.
    - `assemble_set` reads `fixed.get("packs")` and builds `{"packs": …}` with `"kind"` per entry. A `PackCategory` error names a qualified widget, so the existing culprit search drops the file (deviation 3).
    - `Registries::get` returns `WidgetRegistry::default()` for an empty set.
  - **`world.rs`.** Drop `src/widgets.typ` from `LIBRARY`. `main_source` builds `by-name(pack-registry-0 + …)`, or `by-name(())` without packs.
  - **Golden.** Rewrite kinds and version in `tests/golden/widgets.json` and `fixtures/roadmap.json`. Run `cargo nextest run -p oku-slides-render golden`: pages unchanged. Boxes change only where a box names a slide kind; accept those after checking with `cargo insta review`, nothing else.

- [ ] **Step 4: Run, expect PASS**: `cargo nextest run -p oku-slides-render -p oku-slides-render-worker -E 'not binary(/^import_/)'`.

- [ ] **Step 5: Commit**: `feat(render)! : pack de layouts intégré essentiel, noyau Typst sans widget, packs et protocole 5 par sorte, confiance des deux packs livrés (t-1061)`.

---

### Task 4: Import — retrofit on layout profiles across packs, hi-fi on `deck-layouts`, kinds from the constants

**Files:**
- Rename: `crates/oku-slides-import/src/retrofit/template.rs` → `src/retrofit/profile.rs` (+ `template_tests.rs` → `profile_tests.rs`)
- Modify the import crate:
  - `src/retrofit/mod.rs:47-105` (`RetrofitTarget.profiles`; `bare_template` deleted), `src/retrofit/slide.rs:119-127,273-339,399`, `src/retrofit/{place,measure}.rs:15` (import path)
  - `src/report.rs:41,373-374` (`SlideReport.profile`) + `report_tests.rs`
  - `src/hifi/{pack.rs:203-246,mod.rs:133-219,slide.rs:35-132,widget.rs:11,39}`: manifest without templates, kind `Layouts`, namespace `deck-layouts`, `essentiel/blank` fallback, `HifiTarget.essentiel`
  - `src/test_support.rs:13,119-174`, `tests/common/mod.rs:77-120`, `tests/{retrofit,hifi}.rs`, `tests/regressions/issue_0753_output_budget.rs`, `src/hifi/{mod,pack,element}_tests.rs`, `src/retrofit/{mod,slide}_tests.rs`
- Snapshots: `tests/snapshots/retrofit__*.snap`, `tests/snapshots/hifi__*.snap`, `src/hifi/snapshots/*`, regenerated with `cargo insta review`
- Modify render tests that run the importer: `crates/oku-slides-render/tests/{import_retrofit,import_decor,import_pack}.rs`
- Modify: `crates/oku-slides-import/ARCHITECTURE.md`, `crates/oku-slides-import/AGENTS.md`

**Interfaces:**
- Consumes: `LayoutProfile`, `SlideRole`, `Arrangement`, `PackKind`, `kinds::essentiel` (Tasks 1–2); `BuiltinPack` registry fixture (Task 1).
- Produces:
  - `RetrofitTarget { profiles: &'a [(Namespace, LayoutProfile)], registry, colors, page }`: the profiles of every target layout pack, in target order, then manifest order. An empty list fails with `ImportErrorKind::NoProfiles` (`import-no-profiles`, "The chosen layout packs propose no profile.").
  - `profile::Profile<'p> { source: &'p (Namespace, LayoutProfile), role: SlideRole, arrangement: Arrangement, grid: Option<Grid>, blocks: usize, has_title: bool, has_kicker: bool, has_slot: bool }`:
    - `role`, `arrangement`, `blocks`, `title` and `kicker` are read from the declared hints;
    - `grid` is resolved from a slide `Slide { kind: qualify(ns, layout), params: profile.params }` with `resolved_grid(registry)` when `arrangement == Grid`;
    - `has_slot` comes from the layout's schema.
  - Import's private `SlideKind` and `ContentLayout` are deleted, replaced by `SlideRole` / `Arrangement`. `score`, `base`, `want` and `choose` are unchanged apart from these names.
  - A retrofitted slide has kind `qualify(ns, layout)`, and every key of `profile.params` is copied onto it (today: `grid` and `layout`).
  - `SlideReport.profile: String` = `"<namespace>/<id>"`, doc "Le profil de layout choisi, qualifié par son pack.".
  - `HifiTarget.essentiel: PackPin` (doc "L'épinglage d'`essentiel` que le deck importé prend ; la session le remplit."). The hi-fi ops start with:
    1. `AddPack { Widgets, base }`;
    2. `AddPack { Layouts, essentiel }`;
    3. `AddPack { Layouts, deck-layouts }`, whose manifest (scope `Deck`, `requires: [base]`, `examples`, no templates) is built by `PackManifest::to_files(PackKind::Layouts, …)`.
  - `hifi::slide::BLANK` is deleted and the fallback is `kinds::essentiel::BLANK`.
  - `Namespace::deck_layouts()` and `is_deck_layouts()` live in core `namespace.rs` (reserved like `deck`; added in this task with a test in `namespace_tests.rs`).

- [ ] **Step 1: Write the failing tests**

```rust
// crates/oku-slides-import/src/retrofit/profile_tests.rs
#[test]
fn profiles_of_two_packs_keep_target_order_on_ties() {
    let a = (ns("un"), profile_json("p", "content", "content", "default", 1));
    let b = (ns("deux"), profile_json("p", "content", "content", "default", 1));
    let list = vec![a, b];
    let registry = registry_with_layout_packs(&["un", "deux"]);
    let profiles = profiles(&list, &registry);
    let chosen = choose(&shape_single_text(), &profiles).unwrap();
    assert_that!(chosen.source.0.as_str()).is_equal_to("un");
}

#[test]
fn profile_grid_comes_from_its_params() {
    let mut p = profile_json("libre", "blank", "blank", "grid", 0);
    p.params.insert("grid".into(), json!({"columns": 4, "rows": 2}));
    let list = vec![(kinds::essentiel::namespace(), p)];
    let profiles = profiles(&list, &crate::test_support::registry());
    assert_that!(profiles[0].grid.map(|g| (g.columns, g.rows))).is_equal_to(Some((4, 2)));
}

#[rstest]
#[case(SlideRole::Title, Want::Title, 100)]
#[case(SlideRole::Toc, Want::Single, 0)]
#[case(SlideRole::End, Want::Single, 0)]
fn score_by_role_matches_the_former_slide_kinds(#[case] role: SlideRole, #[case] want: Want, #[case] expected: i32) {
    let p = profile_with_role(role);
    assert_that!(score(&shape_wanting(want), &p)).is_equal_to(expected);
}
```

(`profile_json(id, layout, role, arrangement, blocks) -> LayoutProfile`, `registry_with_layout_packs`, `shape_single_text`, `shape_wanting` and `profile_with_role` are test helpers in this file or in `test_support.rs`. The existing `template_tests.rs` cases are rewritten onto the essentiel profiles, read from the fixture manifest `typst/oku-slides/packs/essentiel/layouts.json` through `test_support::essentiel_profiles()`.)

```rust
// crates/oku-slides-import/tests/retrofit.rs (replacing the `#[values("essentiel","grilles")]` cases; Grilles cases deleted)
#[test]
fn retrofit_slide_kinds_are_all_essentiel() {
    let (_, deck) = import("kitchen", &["essentiel"]);
    let stray: Vec<String> = deck.slides.iter().map(|s| s.kind.clone())
        .filter(|k| !kinds::essentiel::ALL.contains(&k.as_str())).collect();
    assert_that!(stray).is_empty();
}

#[test]
fn retrofit_on_two_layout_packs_may_use_the_second() {
    // Un pack `acme` dont le seul profil (chapitre) gagne là où essentiel n'en propose pas de meilleur.
    let (report, _) = import_with_extra_pack("alto", &["essentiel", "acme"], acme_chapter_pack());
    assert_that!(report.slides.iter().any(|s| s.profile.starts_with("acme/"))).is_true();
}

#[test]
fn retrofit_without_profiles_is_refused() {
    let err = import_err("minimal", &["acme-empty"]);
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::NoProfiles);
}
```

(`acme_chapter_pack()`: a layout pack `acme` with one layout `chapter` (copy of `essentiel/chapter`) and one profile `{role: chapter, arrangement: default, title: true}`, placed **before** essentiel's chapter profile in the scoring so it wins on ties. If alto has no chapter slide, use the fixture that has one: `rg -l secHead fixtures/import`. `import` and `import_err` take the list of target namespaces.)

```rust
// crates/oku-slides-import/tests/hifi.rs
#[test]
fn hifi_plan_pins_base_essentiel_then_deck_layouts() {
    let ops = plan("minimal").ops;
    let pins: Vec<(PackKind, &str)> = ops.iter().filter_map(|o| match o {
        Op::AddPack { kind, namespace, .. } => Some((*kind, namespace.as_str())), _ => None }).collect();
    assert_that!(pins).is_equal_to(vec![
        (PackKind::Widgets, "base"), (PackKind::Layouts, "essentiel"), (PackKind::Layouts, "deck-layouts")]);
}

#[test]
fn hifi_slide_kinds_are_deck_layouts_or_essentiel_blank() {
    let (_, deck) = import("kitchen");
    let stray: Vec<&str> = deck.slides.iter().map(|s| s.kind.as_str())
        .filter(|k| !k.starts_with("deck-layouts/") && *k != kinds::essentiel::BLANK).collect();
    assert_that!(stray).is_empty();
}
```

`hifi/pack_tests.rs`: `pack_manifest_has_no_templates_and_layouts_files` asserts `files.kind() == PackKind::Layouts` and that `to_value(manifest).get("templates")` is `None`. The test at `:245-266` (one template per widget) is deleted. `tests/hifi.rs:79` (`manifest.templates.len()`) becomes `files.widgets().len() - common == plan.schemas.len()`.

- [ ] **Step 2: Run, expect FAIL**: `cargo nextest run -p oku-slides-import`.

- [ ] **Step 3: Implement**: as under Interfaces. `slide.rs:273-287` builds `Slide::new(id, &qualify(&profile.source.0, profile.source.1.layout.as_str()))` and copies `profile.source.1.params`. `pick()` (`:119-127`) prefers profiles with `has_slot` as before. The render `import_*` tests read `typst/oku-slides/packs/essentiel/layouts.json`'s profiles instead of `templates/essentiel/pack.json`.

- [ ] **Step 4: Review snapshots**: `cargo insta review`. For the retrofit, accept only:
  - slide kinds becoming `essentiel/*`;
  - `"template": "<id>"` becoming `"profile": "essentiel/<id>"` with the **same** id;
  - the deck version.

  For the hi-fi, accept only:
  - `deck/` slide kinds becoming `deck-layouts/`;
  - `blank` becoming `essentiel/blank`;
  - the new `AddPack` ops and their `kind`;
  - the manifest losing `templates`.

  Anything else, such as a different profile chosen for any slide, is a bug.

- [ ] **Step 5: Run, expect PASS**: `cargo nextest run -p oku-slides-import -p oku-slides-render`.

- [ ] **Step 6: Commit**: `feat(import)! : retrofit sur les profils des packs de layouts choisis, hi-fi sur deck-layouts, kinds essentiel (t-1062)`.

---

### Task 5: Session — built-in `essentiel`, layout library, decks and producers pin it, trust, labels

**Files:**
- Modify the library:
  - `crates/oku-slides-session/src/library_kind.rs:55-96`: `Templates` → `Widgets` (`BUILTIN_WIDGETS` = `base`), new `Layouts` kind (`DIR = "layouts"`, `MANIFEST_FILE = "layouts.json"`, `BUILTIN_LAYOUTS` = `essentiel`), `LibraryKind::PACK_KIND: Option<PackKind>`
  - `src/library.rs:17,157-163,386,514-534,591-601`: `template_sets` deleted, `packs(kind)`; namespace uniqueness across both kinds; `deck-layouts` reserved
  - `src/pack.rs:14-19,81-83`: `DEFAULT_PACK` and `GRID_PACK` deleted, `ESSENTIEL_PACK`; `PackInfo.count` → `PackInfo.widgets` and `PackInfo.kind`
  - `src/lib.rs:108-111` (re-exports)
- Modify pins and resolution:
  - `src/deck_pack.rs:35-89`: `adopt_default_packs` pins `base` (widgets) and `essentiel` (layouts); `builtin_version(id)`, `in_process_pack`, `only_builtins`
  - `src/deck_closure.rs:112-142`: `templates_requires` → `examples_requires` + `deck_closure_tests.rs`
  - `src/deck_widgets.rs:130-202` and `src/probed_widgets.rs:180-300`: the set built from both maps; in-process built-ins; assembly failure with a trusted pack returned (t-1057 #4)
  - `src/deck_fallback.rs:55-125` (trusted partition unchanged, now covering `essentiel`)
- Modify producers:
  - `src/lib.rs:309-331` (blank deck: `essentiel::TITLE`, `essentiel::CONTENT`), `:360-368`
  - `src/help.rs:60-81` (pins built-in `base` and `essentiel`)
  - `src/import/{mod.rs:84-200,hifi.rs:68-95,retrofit.rs:61-110,queue.rs:155}`; `src/import/child_pack.rs` deleted (and its tests)
  - `src/widgets_deck.rs:69-200` (slide widgets → `deck-layouts`), `src/widget_preview.rs:293-360` (deck pins, t-1057 #1; preview slide `essentiel::BLANK`)
- Modify labels: `src/param_fields.rs:15-45` (`widget_key` for `kinds::is_builtin_kind`)
- Modify resume: `src/resume.rs:35,558-646` (`VERSION = 2`; notice when an unsaved tab is set aside, t-1057 #3), `src/workspace_resume.rs:185-215`
- Modify i18n:
  - `*/session.ftl:158-185`: `session-widget-kind-{content,chapter,end,toc}` → `session-widget-kind-essentiel-{…}`, plus new `-essentiel-title` and `-essentiel-blank`; `session-param-{content,chapter,end,toc,title,blank}-*` → `session-param-essentiel-{…}-*`; 20 locales
  - `*/notice.ftl` (or the file holding `notice-tab-resume-failed`): `notice-tab-set-aside` after it, 20 locales
- Modify tests:
  - `library_tests.rs`, `library_namespace_tests.rs`, `library_lookup_tests.rs`, `deck_pack_tests.rs`, `help_tests.rs`, `import/mod_tests.rs`, `import/diff_tests.rs:176`, `workspace_import_tests.rs`, `deck_widgets_*tests.rs`, `widgets_deck_tests.rs`, `widget_preview_tests.rs`, `resume_tests.rs`
  - every session test with a bare slide kind or a version-7 deck (→ `kinds::essentiel`, 8)
- Create (declared in `tests/regressions/main.rs`): `tests/regressions/issue_1063_new_deck_pins_essentiel_layouts.rs`, `tests/regressions/issue_1063_old_tab_set_aside_notice.rs`
- Modify: `crates/oku-slides-session/ARCHITECTURE.md`, `crates/oku-slides-session/AGENTS.md`

Pack mode (`templates_mode.rs` and friends) is Task 6. This task leaves it compiling with the minimum change: it reads examples where it read templates, and leaves no dead code. If the minimum is not small, `edit_templates` returns `SessionErrorKind::PackMode` until Task 6, and Task 6 removes that.

**Interfaces:**
- Consumes: Tasks 1–4.
- Produces:
  - `pub const ESSENTIEL_PACK: Uuid` (doc "L'id du pack de layouts intégré `essentiel`."), equal to the manifest id.
  - `pub struct Widgets;` and `pub struct Layouts;` (`LibraryKind`). `Library::packs(kind: PackKind) -> Vec<PackInfo>`. `PackInfo { kind, namespace, requires, default, received, widgets: usize, profiles: usize }`, where `default` is true for `base` and `essentiel`.
  - `adopt_default_packs`: a deck with **no pin in either map** gets current `base` + `essentiel` (and closures). A deck with any pin is untouched (lot 2, deviation 14).
  - `examples_requires(namespace: &Namespace, manifest: &PackManifest) -> Vec<Namespace>`: the namespaces, in first-seen order and deduplicated, of every qualified kind in the examples' slot values (nested slots walked), excluding `namespace` and kept after the manifest's previous `requires`.
  - `Session::resolve_widgets` builds its `PackSet` from `deck.widgets` then `deck.layouts`, each `PackWidgets` with its kind. When the set holds only current built-ins it takes `builtin_registry` (no worker).
  - Import: `ImportTarget::Retrofit { layouts: Vec<String>, theme: Option<String> }`; empty resolves to `[ESSENTIEL_PACK]`. Each entry resolves (id, folder or name) to a library **layout** pack, else `ImportErrorKind::NotLayoutPack` (`import-not-layout-pack`, arg `pack`). The session pins `base`, every target and their closures, then calls the import crate with the collected profiles.
  - `Session::deck_widget_target(pack, category)`: with no explicit pack, `Element` → `deck`, `Slide` → `deck-layouts` (layouts, scope deck), created on first write. An explicit pack of the wrong kind fails with `WidgetErrorKind::BadSchema(SchemaFault::PackCategory)` (reused). `notice-tab-set-aside` (arg `name`) is raised once per tab set aside on resume.

- [ ] **Step 1: Write the failing tests**

```rust
// src/library_tests.rs (replacing library_builtins_have_their_namespaces)
#[rstest]
#[case(PackKind::Widgets, vec!["base"])]
#[case(PackKind::Layouts, vec!["essentiel"])]
fn library_builtins_by_kind(#[case] kind: PackKind, #[case] expected: Vec<&str>) {
    let lib = library();
    let names: Vec<String> = lib.packs(kind).iter().filter(|p| p.default).map(|p| p.namespace.to_string()).collect();
    assert_that!(names).is_equal_to(expected.into_iter().map(str::to_owned).collect::<Vec<_>>());
}

#[test]
fn library_has_no_grilles() {
    let lib = library();
    let all: Vec<String> = PackKind::ALL.iter().flat_map(|k| lib.packs(*k)).map(|p| p.namespace.to_string()).collect();
    assert_that!(all.contains(&"grilles".to_owned())).is_false();
}

#[rstest]
#[case(PackKind::Widgets, "essentiel")]
#[case(PackKind::Layouts, "base")]
#[case(PackKind::Layouts, "deck-layouts")]
fn library_add_namespace_taken_by_any_kind_is_refused(#[case] kind: PackKind, #[case] namespace: &str) {
    let lib = library();
    let err = lib.add(manifest_of(kind, namespace), BTreeMap::new()).unwrap_err();
    assert_that!(matches!(err.kind(), SessionErrorKind::NamespaceTaken | SessionErrorKind::NamespaceReserved)).is_true();
}
```

(`library()`, `manifest_of(kind, ns)` and the `add` signature: this file's existing helpers. Adapt to them; the asserted kinds are fixed.)

```rust
// src/deck_pack_tests.rs
#[test]
fn adopt_default_packs_pins_base_widgets_and_essentiel_layouts() {
    let mut deck = Deck::empty();
    adopt_default_packs(&library(), &mut deck).unwrap();
    assert_that!((deck.widgets.keys().map(Namespace::as_str).collect::<Vec<_>>(),
                  deck.layouts.keys().map(Namespace::as_str).collect::<Vec<_>>()))
        .is_equal_to((vec!["base"], vec!["essentiel"]));
}

#[test]
fn adopt_default_packs_leaves_a_deck_with_only_layout_pins() {
    let mut deck = Deck::empty();
    deck.layouts.insert(ns("acme"), pin(&[]));
    adopt_default_packs(&library(), &mut deck).unwrap();
    assert_that!(deck.widgets.is_empty()).is_true();
}
```

```rust
// src/deck_closure_tests.rs (t-1054)
#[rstest]
#[case::own_namespace_excluded(&[], &["acme/a"], &[])]
#[case::previous_requires_first(&["zeta"], &["base/text"], &["zeta", "base"])]
#[case::duplicates_ignored(&[], &["base/text", "base/card", "base/text"], &["base"])]
#[case::nested_slot_walked(&[], &["@nested:base/stack>essentiel-x/w"], &["base", "essentiel-x"])]
fn examples_requires_table(#[case] previous: &[&str], #[case] kinds: &[&str], #[case] expected: &[&str]) {
    let manifest = manifest_with_examples("acme", previous, kinds);
    let got: Vec<String> = examples_requires(&ns("acme"), &manifest).iter().map(ToString::to_string).collect();
    assert_that!(got).is_equal_to(expected.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>());
}
```

(`manifest_with_examples(ns, previous, kinds)`: a test helper building one example per kind as a slot value `{"blocks": [{"id", "kind"}]}`. The `@nested:outer>inner` form places `inner` inside a slot of an `outer` element. Write it beside the test.)

```rust
// src/param_fields_tests.rs
#[rstest]
#[case(kinds::essentiel::CONTENT, Some("session-widget-kind-essentiel-content"))]
#[case(kinds::base::CARD, Some("session-widget-kind-base-card"))]
#[case("acme/card", None)]
#[case("content", None)]
fn widget_key_only_for_builtin_kinds(#[case] kind: &str, #[case] expected: Option<&str>) {
    assert_that!(kind_key(kind).as_deref()).is_equal_to(expected);
}

#[test]
fn kind_name_of_essentiel_title_is_localized_in_french() {
    let fr = kind_name(&Localizer::new(Some("fr")), kinds::essentiel::TITLE, None);
    assert_that!(fr).is_not_equal_to(kinds::essentiel::TITLE.to_owned());
}
```

(`kind_key` exposes the key `widget_key` builds, `pub(crate)`. Adapt to the real `kind_name` signature.)

```rust
// src/import/mod_tests.rs
#[test]
fn retrofit_without_layouts_targets_essentiel() {
    let imported = import_retrofit("minimal", &[], None);
    assert_that!(imported.deck.layouts.keys().map(Namespace::as_str).collect::<Vec<_>>()).is_equal_to(vec!["essentiel"]);
}

#[test]
fn retrofit_on_a_widget_pack_is_refused() {
    let err = import_retrofit_err("minimal", &["base"]);
    assert_that!(err.import_kind()).is_equal_to(Some(ImportErrorKind::NotLayoutPack));
}
```

```rust
// src/widgets_deck_tests.rs
#[test]
fn deck_widget_write_of_a_slide_widget_creates_deck_layouts() {
    let mut s = session();
    s.write_widget(None, "cover", SLIDE_WIDGET_SRC).unwrap();
    assert_that!((s.deck().layouts.contains_key(&Namespace::deck_layouts()), s.deck().widgets.contains_key(&Namespace::deck())))
        .is_equal_to((true, false));
}
```

Regressions:

```rust
//! Un nouveau deck n'épinglait aucun pack de layouts : ses slides n'avaient pas de widget.
#[test]
fn issue_1063_new_deck_pins_essentiel_layouts() {
    let (platform, _dir) = common::platform();
    let s = Session::new(common::app_fonts(), platform).unwrap();
    let deck = s.deck();
    assert_that!(deck.layouts.contains_key(&kinds::essentiel::namespace())).is_true();
    assert_that!(deck.slides.iter().all(|sl| kinds::essentiel::ALL.contains(&sl.kind.as_str()))).is_true();
}
```

```rust
//! Un onglet non enregistré d'une version de deck refusée était mis de côté sans avis.
#[test]
fn issue_1063_old_tab_set_aside_raises_a_notice() {
    let (platform, dir) = common::platform();
    common::write_resume_state_of_version_7(&dir, "Ancien");
    let s = Session::resume_from(common::app_fonts(), platform).unwrap();
    assert_that!(s.notices().iter().any(|n| n.key() == "notice-tab-set-aside")).is_true();
}
```

(`write_resume_state_of_version_7`: write the resume files of a version-7 deck the way `resume_tests.rs` builds a state, with the deck JSON's `version` set to 7. `resume_from` / `notices`: the real resume entry point and notice accessor. Read `tests/common/mod.rs`, `resume.rs:558-589` and `workspace_slot.rs:26-36` first. If the notice belongs to the workspace, not the session, assert it there.)

- [ ] **Step 2: Run, expect FAIL**: `cargo nextest run -p oku-slides-session -E 'not test(/templates_mode|pack_mode/)'`.

- [ ] **Step 3: Implement**
  - **Built-ins and library.**
    - `BUILTIN_WIDGETS` reads `BuiltinPack::Base.files()`, `BUILTIN_LAYOUTS` `BuiltinPack::Essentiel.files()`. The `include_str!` of `templates/*` goes.
    - Delete `templates/` (both files and the directory).
    - `Library` holds one store per kind. `owner_of` / `library_namespace` read the entries of both kinds. `deck-layouts` is reserved as `deck` is (`library.rs:386`).
  - **Resolution.**
    - `resolve_widgets` iterates `deck.widgets.iter().map(|p| (PackKind::Widgets, p)).chain(deck.layouts…)`.
    - `in_process_pack(&pin.version)` returns the current `BuiltinPack` of that version if any.
    - `registry_for`: an empty set gives `WidgetRegistry::default()`; only current built-ins give `builtin_registry`; otherwise the worker.
    - `probed_widgets.rs`: when `assemble_each` / `worker.registry_of` fails for a set holding a trusted pack, return the error (as `resolve_widgets` does) instead of logging and falling back (t-1057 #4).
    - Every site lot 2 listed as using `builtin_registry` for a first replica keeps it (now `base` + `essentiel`). The sites that used `core_registry` take `WidgetRegistry::default()`.
  - **Producers.**
    - `help.rs` pins the built-in `base` (widgets) and `essentiel` (layouts) before `sources_with_packs`.
    - The blank deck uses `kinds::essentiel::{TITLE, CONTENT}`.
    - Hi-fi fills `HifiTarget.essentiel` with `builtin_pin(ESSENTIEL_PACK)`.
    - Retrofit resolves its layout packs, inserts their files, collects `(namespace, profile)` in order, and drops the child pack: `ImportState` / `Prepared.child` and `child_pack.rs` go, with every reference (`rg -n child_pack crates`).
    - `queue.rs:155` `deck_key` hashes `m.profiles` instead of `m.templates`.
  - **Widget writes and preview.**
    - `widgets_deck.rs`: `new_deck_pack(kind)` creates `deck` or `deck-layouts`. The category comes from the written file's probe schema; a file that does not probe goes to the pack named by the caller, else `deck`.
    - `widget_preview.rs`: the preview set is the working deck's pinned packs (deck mode: the deck's; pack mode: the working deck's). An element preview is hosted on an `essentiel/blank` slide; a slide widget preview is the slide itself.
  - **Resume.** `resume.rs` `VERSION = 2`. When restoring a tab fails because of the deck version (`UnsupportedDeckVersion`) and the tab had unsaved changes, push `notice-tab-set-aside` with the tab name.

- [ ] **Step 4: Run, expect PASS**: `cargo nextest run -p oku-slides-session -E 'not test(/templates_mode|pack_mode/)'` and `cargo nextest run -p oku-slides-session regressions`.

- [ ] **Step 5: Commit**: `feat(session)! : essentiel intégré en pack de layouts, bibliothèque par sorte, decks et imports l'épinglent, confiance des deux packs livrés (t-1063)`.

---

### Task 6: Session — pack mode (examples instead of templates), renamed requests

**Files:**
- Rename and split `crates/oku-slides-session/src/templates_mode.rs` (537 lines):
  - `src/pack_mode.rs` (enter, close, mode state, realign);
  - `src/pack_mode_save.rs` (edited manifest, save, session version);
  - `src/pack_mode_previews.rs` (layout previews and their cache, used by `Request::Layouts`);
  - tests `pack_mode_tests.rs`, `pack_mode_save_tests.rs`.
- Other renames: `src/templates_library.rs` → `src/pack_library.rs`, `src/template_scratch.rs` → `src/pack_scratch.rs` (+ tests).
- Modify mode and views:
  - `src/lib.rs:179-233,535,575` (`Mode::Pack`, `pack_renderer`, `pack_cache`)
  - `src/views.rs:188,257,276-330`: `PileSection::Templates` → `Layouts`; `TabInfo.editing_pack`; `PackMode`; `LayoutsView`, `LayoutView`, `LayoutPackView`
  - `src/request.rs:140-200,418-431,491-553` (renamed requests, `Layouts`, `Packs { kind }`)
- Modify the pack-mode helpers:
  - `src/panel.rs:16-143`: `host-template` surface and template undo labels deleted
  - `src/widget_examples.rs:46-150`, `src/widget_params.rs:25`
  - `src/widgets_edit.rs:187,385,403-419,459`: `ensure_unused` over examples; realign on write and remove
  - `src/theme_panel.rs:228,278`
- Modify agent wiring: `src/agent/{context.rs:115-151,conversation.rs:162}` (mode string `pack(id=…)`)
- Modify state and switches: `src/resume.rs:68,720` (`SavedMode::Pack`), `src/workspace_same_deck.rs:67`, `src/render_fonts.rs:81-83`, `src/deck_widgets.rs:262-264`, `src/pack_notice.rs:181`
- Modify the session `apply` guard: `src/lib.rs` (or `src/apply.rs` if that is where ops enter): structural slide ops refused in pack mode
- Modify `src/error.rs`: `SessionErrorKind::PackModeSlides`; the template-set errors are renamed to pack errors (`rg -n -i template crates/oku-slides-session/src/error.rs`)
- Modify i18n: `*/session.ftl` (template keys renamed to pack keys, `session-pack-mode-slides` added, keys of the deleted `host-template` panel removed), `*/agent.ftl` (`agent-confirm-save-templates` → `agent-confirm-save-pack`), 20 locales
- Modify tests: every session test of templates mode, renamed, plus `tests/session.rs:429,497`, `tests/errors.rs:134`, `src/deck_theme_edit_tests.rs:116`
- Modify: `crates/oku-slides-session/ARCHITECTURE.md`, `crates/oku-slides-session/AGENTS.md`

**Interfaces:**
- Consumes: Task 5.
- Produces:
  - `Request::Layouts` → `Reply::Layouts(LayoutsView)`, where:
    - `LayoutsView { packs: Vec<LayoutPackView> }` is one entry per embedded layout pack, in namespace order;
    - `LayoutPackView { namespace, name, layouts: Vec<LayoutView> }`;
    - `LayoutView { kind, label, slide: Slide, svg }`: `slide` is kind `kind`, id `layout-<name>`, params = the layout's example (else empty); `svg` is rendered with the deck's theme; `label` is `kind_name`.
  - `Request::Packs { kind: PackKind }` → `Vec<PackInfo>`.
  - `Request::EditPack { pack: Uuid }`, `SavePack { as_new }`, `ClosePack`, `DeletePack { pack }` (refused for built-ins as before).
  - `Session::edit_pack(id)`, `save_pack(as_new)`, `close_pack()`: renamed from the `*_templates` methods with the same behavior, except for the working deck below.
  - Pack mode working deck:
    - Layout pack: one slide per layout file in `WidgetFile` order, id `layout-<name>`, params = example.
    - Widget pack: no slides.
    - Save: for a layout pack, `examples[<name>]` = the params of slide `layout-<name>` minus neutral values (`widget_examples.rs`'s existing neutral filter); `requires` = `examples_requires`.
    - Writing a layout file appends its slide; removing one removes it (`realign_layouts`). `ensure_unused` refuses removing a widget while an example of the pack uses it, and names the examples.
  - In `Mode::Pack`, an `apply` containing `AddSlide`, `RemoveSlide`, `MoveSlide`, or `SetKind` on a slide fails with `SessionErrorKind::PackModeSlides` before any op runs.
  - `Snapshot.editing: Option<PackMode>`, where `PackMode { pack, name, kind, builtin, base, parent, session_version, deck_only, theme }` is `TemplateMode` renamed plus `kind`. `Snapshot.layouts: BTreeMap<Namespace, PackPin>`.

- [ ] **Step 1: Write the failing tests**

```rust
// src/pack_mode_tests.rs
#[test]
fn edit_pack_essentiel_working_deck_has_one_slide_per_layout() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    let ids: Vec<&str> = s.deck().slides.iter().map(|sl| sl.id.as_str()).collect();
    assert_that!(ids).is_equal_to(vec![
        "layout-blank", "layout-chapter", "layout-content", "layout-end", "layout-title", "layout-toc"]);
}

#[test]
fn edit_pack_base_working_deck_has_no_slides() {
    let mut s = session();
    s.edit_pack(BASE_PACK).unwrap();
    assert_that!(s.deck().slides.is_empty()).is_true();
}

#[test]
fn pack_mode_content_slide_starts_from_its_example() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    let slide = s.deck().slides.iter().find(|sl| sl.id == "layout-content").unwrap();
    assert_that!(slide.params.text("title")).is_equal_to(Some("Titre de la slide"));
}

#[rstest]
#[case(Op::RemoveSlide { id: "layout-end".into() })]
#[case(Op::MoveSlide { id: "layout-end".into(), index: 0 })]
#[case(Op::SetKind { id: "layout-end".into(), kind: kinds::essentiel::BLANK.into() })]
fn pack_mode_structural_slide_op_is_refused(#[case] op: Op) {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    let err = s.apply("x", vec![op], None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::PackModeSlides);
}

#[test]
fn pack_mode_slides_message_is_localized_in_french() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    let err = s.apply("x", vec![Op::RemoveSlide { id: "layout-end".into() }], None).unwrap_err();
    assert_that!(err.localize(&Localizer::new(Some("fr")))).is_not_equal_to(err.localize(oku_slides_i18n::english()));
}

#[test]
fn pack_mode_writing_a_layout_appends_its_slide() {
    let mut s = session_with_user_layout_pack("acme", &["one"]);
    s.write_widget(Some(&ns("acme")), "two", SLIDE_WIDGET_SRC).unwrap();
    assert_that!(s.deck().slides.iter().any(|sl| sl.id == "layout-two")).is_true();
}
```

(The exact `Op` field names follow `ops/mod.rs`; adapt them. `session_with_user_layout_pack` creates a library layout pack and enters pack mode on it.)

```rust
// src/pack_mode_save_tests.rs
#[test]
fn save_pack_writes_the_edited_title_into_the_layout_example() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.apply("t", vec![set_text("layout-content", "title", "Nouveau")], None).unwrap();
    let saved = s.save_pack(true).unwrap();
    let m = PackManifest::from_files(&s.library().version_files(saved).unwrap()).unwrap();
    assert_that!(m.examples[&WidgetFile::parse("content").unwrap()].get("title")).is_equal_to(Some(&json!("Nouveau")));
}

#[test]
fn save_pack_requires_the_namespaces_its_examples_use() {
    let mut s = session();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    let saved = s.save_pack(true).unwrap();
    let m = PackManifest::from_files(&s.library().version_files(saved).unwrap()).unwrap();
    assert_that!(m.requires).is_equal_to(vec![kinds::base::namespace()]);
}
```

(`set_text` and `version_files` are helpers or accessors these test files already use for templates. Read `templates_mode_tests.rs` before renaming.)

```rust
// src/request_tests.rs (or the file that tests `Request` dispatch)
#[test]
fn request_layouts_lists_essentiel_layouts_with_previews() {
    let mut s = session();
    let Reply::Layouts(view) = s.request(Request::Layouts).unwrap() else { panic!("réponse inattendue") };
    let kinds: Vec<&str> = view.packs[0].layouts.iter().map(|l| l.kind.as_str()).collect();
    assert_that!((view.packs.len(), kinds.len(), view.packs[0].layouts.iter().all(|l| l.svg.starts_with("<svg"))))
        .is_equal_to((1, 6, true));
}
```

(`panic!` is allowed in tests only. Use the reply-matching idiom of the existing request tests if there is one.)

- [ ] **Step 2: Run, expect FAIL**: `cargo nextest run -p oku-slides-session`.

- [ ] **Step 3: Implement**
  - **Renames.** Rename and split as under Files; every behavior not named in Interfaces is unchanged (stash, scratch, theme draft, switches, session version, deck-only packs edited from the embedded version).
  - **Working deck.** `pack_mode.rs::working_deck(manifest, files, kind)`:
    - builds the slides described in Interfaces;
    - pins the edited pack (in its kind's map), its closure, and `base` (lot 2 rule, kept);
    - for a widget pack, pins `essentiel` too, so widget previews have a host slide.
  - **Save.** `pack_mode_save.rs::edited_manifest` replaces the template rebuild with the example writeback.
  - **Guard.** In `Session::apply`, when `matches!(self.mode, Mode::Pack { .. })` and an op is structural, `ensure!(false, PackModeSlidesSnafu)` before the replica.
  - **Panel.** Delete the `host-template` surface (`panel.rs`) and its undo labels.
  - **Previews.** `Request::Layouts` renders with `pack_mode_previews.rs` (former `render_previews` + cache, keyed by layout pack version and theme).

- [ ] **Step 4: Run, expect PASS**: `cargo nextest run -p oku-slides-session`.

- [ ] **Step 5: Commit**: `feat(session)! : mode pack, exemples de layouts à la place des modèles, requêtes renommées, aperçus de layouts (t-1064)`.

---

### Task 7: CLI, MCP, agent and local agent on layout packs — workspace green

**Files:**
- Modify CLI:
  - `crates/oku-slides-cli/src/{main.rs:85,147,import.rs:5-128,library.rs,library_tests.rs:2-45,pack_render.rs:69-95,bench.rs}`: `--pack` repeatable, layout namespaces; `DEFAULT_PACK` / `GRID_PACK` gone
  - `tests/{import.rs,import_reference.rs:191,cli.rs,pack_broken.rs,regressions/issue_0355_*.rs}`: Grilles cases deleted, version 8, `essentiel/*`, `templates: Vec::new()` removed
- Modify MCP:
  - `crates/oku-slides-mcp/src/{handler.rs:55-82,read_tools.rs:41,111-113,edit_tools.rs:41-111,guarded_tools.rs:35,191-220,tool_error.rs:25,error.rs:32,tools.rs:236-281,widget_tools.rs:101-134}`
  - `tests/{server.rs:21,edit.rs:107,167-237,widgets.rs:63-78,guarded.rs:136-175,import.rs:194,296-352,surfaces.rs}`
  - `ARCHITECTURE.md`
- Modify agent and local agent:
  - `crates/oku-slides-agent/src/instructions/{agent.md,widgets.md,import.md}`, `src/instructions_tests.rs:199-200`, `tests/{scopes.rs:32,stack.rs:331}`
  - `crates/oku-slides-local-agent/src/families.rs:13-76` (`Family::Templates` → `Family::Packs`), `ARCHITECTURE.md:31`
- Modify `src-tauri` if any test or command names a template request (survey: none; check with `rg -n -i template src-tauri`)
- Modify i18n: `*/cli.ftl` (`cli-arg-pack`: "Layout pack for retrofit (namespace, id or folder); repeatable, default essentiel"; `cli-import-pack-required` deleted), `*/agent.ftl` if the MCP strings live there, 20 locales

**Interfaces:**
- Consumes: Tasks 5–6.
- Produces:
  - **CLI.** `oku-slides import --pack <p> [--pack <q>]…`: each value names a layout pack; none means `essentiel`. `--pack` is no longer required.
  - **MCP tools.**
    - `packs` ({kind?} → library packs by kind, with `widgets` and `profiles` counts).
    - `edit_pack` ({pack}), `close_pack`, `save_pack` ({as_new}, confirmed), `delete_pack` ({pack}, confirmed).
    - `layouts` (→ `Request::Layouts` without SVG: `{namespace, name, layouts: [{kind, label}]}`).
    - `deck_outline` reports `widget_packs` and `layout_packs`, and the mode string `pack`.
    - The `apply` description documents `add_pack` / `remove_pack` with `kind`.
    - The `widgets` description lists `kinds::base::ALL` and `kinds::essentiel::ALL`; its text says a deck's other kinds come from its packs, `namespace/name` (t-1056).
    - `import` takes `layouts: [string]` for retrofit.
  - **Agent instructions** name `essentiel/content` for slides, `deck-layouts` for the deck's own layouts, and pack mode instead of templates mode.

- [ ] **Step 1: Write the failing tests**

```rust
// crates/oku-slides-mcp/tests/server.rs
#[test]
fn tools_list_has_pack_tools_and_no_template_tool() {
    let names = tool_names();
    for t in ["packs", "edit_pack", "save_pack", "close_pack", "delete_pack", "layouts"] {
        assert_that!(names.contains(&t.to_owned())).is_true();
    }
    assert_that!(names.iter().any(|n| n.contains("template"))).is_false();
}

// crates/oku-slides-mcp/src/tools_tests.rs (t-1056)
#[test]
fn widgets_description_names_every_builtin_kind() {
    let d = tool_description("widgets");
    let missing: Vec<&str> = kinds::base::ALL.iter().chain(kinds::essentiel::ALL.iter())
        .copied().filter(|k| !d.contains(k)).collect();
    assert_that!(missing).is_empty();
}

#[test]
fn no_tool_description_mentions_templates() {
    let offenders: Vec<String> = all_tools().into_iter()
        .filter(|t| t.description.to_lowercase().contains("template")).map(|t| t.name).collect();
    assert_that!(offenders).is_empty();
}
```

```rust
// crates/oku-slides-cli/tests/import.rs
#[test]
fn import_retrofit_without_pack_uses_essentiel() {
    let out = run(&["import", fixture("minimal"), "--mode", "retrofit", "-o", tmp_out()]);
    let deck = read_deck(&out);
    assert_that!(deck.layouts.keys().map(Namespace::as_str).collect::<Vec<_>>()).is_equal_to(vec!["essentiel"]);
}

#[test]
fn import_retrofit_on_a_widget_pack_fails() {
    let out = run_err(&["import", fixture("minimal"), "--mode", "retrofit", "--pack", "base", "-o", tmp_out()]);
    assert_that!(out.stderr).contains("base");
}
```

(Adapt the CLI argument spelling to `main.rs` and the helpers to `tests/import.rs`.) `crates/oku-slides-agent/src/instructions_tests.rs`: the instructions mention neither `edit_templates` nor `templates mode`, and `widgets.md` mentions `deck-layouts`. The local agent's `families_tests.rs`: `families_of("edit_pack") == [Family::Packs]`, and no tool name contains `template`.

- [ ] **Step 2: Run, expect FAIL**: `cargo nextest run -p oku-slides-cli -p oku-slides-mcp -p oku-slides-agent -p oku-slides-local-agent`.

- [ ] **Step 3: Implement**: as under Interfaces. In the `widgets` tool description, build the base and essentiel kind lists from the constants: `format!` with `kinds::base::ALL.join(", ")`, so the test above locks them in.

- [ ] **Step 4: Run, expect PASS**: `cargo nextest run --workspace --exclude oku-slides-app` (the whole Rust workspace is green from here).

- [ ] **Step 5: Commit**: `feat(mcp)! : outils de packs à la place des modèles, layouts listés, retrofit sur packs de layouts dans la CLI et l'agent (t-1065)`.

---

### Task 8: UI — layouts in the snapshot, the layout picker, kinded pack ops, import dialog

**Files:**
- Rename: `ui/src/baseKinds.ts` → `ui/src/builtinKinds.ts` (+ test)
- Delete: `ui/src/TemplatePicker.tsx`; create `ui/src/LayoutPicker.tsx` (+ `LayoutPicker.render.test.tsx`)
- Modify:
  - `ui/src/api.ts:20-261,343-345,464-470,586-591,645,798-803`: `layouts` pin map; `LayoutsView`; `Packs`/`EditPack`/…; `Slide.name/description` deleted; `TemplateView` → `LayoutView`; `PileSectionName` `"layouts"`
  - `ui/src/packOps.ts`: `addPackOp(kind, …)`; `firstPackId` deleted if unused
  - `ui/src/useTemplatesMode.ts` → `ui/src/useLayoutInsert.ts` (insert only; `packWidgets` and `addChoices` move to `ui/src/usePackWidgets.ts`), `ui/src/deck.ts:106-114` (`instantiate(slide)`)
  - `ui/src/slideTools.ts:7-21` (`canAdd` on `ESSENTIEL` constants), `ui/src/App.tsx:212-222,496-511`
  - `ui/src/ImportDialog.tsx:17-134` (multi-select of layout packs with profiles) + `ImportDialog.render.test.tsx`, `ui/src/useImportPick.render.test.tsx`
  - `ui/src/styles.css:494-504` (`.template*` → `.layout-card*`; `.template-name` used by `WidgetPicker` → `.card-name`, `styles.test.ts`, e2e `020`)
  - every UI test fixture with a bare slide kind: `Stage.render.test.tsx`, `deck.test.ts`, `WidgetTab.render.test.tsx:433`, `widgetPreview.test.ts:33`, `agent/useAgentFocus.render.test.tsx:20`, `slideTools.test.ts`
- Modify i18n: `*/ui.ftl` (`ui-picker-set` deleted, `ui-picker-new-slide` kept, `ui-import-layout-packs` added; `ui-picker-new-template`, `ui-template-variant-name` and `ui-undo-new-template` deleted unless still used, check with `rg`), `ui/src/i18n-keys.test.ts`, 20 locales
- Modify: `ui/ARCHITECTURE.md`, `ui/AGENTS.md`

**Interfaces:**
- Consumes: Tasks 6–7 (`Request::Layouts`, `Packs { kind }`, `Snapshot.layouts`, kinded `add_pack`).
- Produces:
  - `builtinKinds.ts`: `BASE` (lot 2) and `ESSENTIEL = { title: "essentiel/title", …, blank: "essentiel/blank" } as const`, mirroring `kinds::essentiel`, plus `builtinName(kind): string | null`.
  - `LayoutPicker({ onPick, onClose })`: calls `api.layouts()` and shows one section per layout pack (its name) with one card per layout (SVG + label). `onPick(layout: LayoutView)`.
  - `useLayoutInsert().insertLayout(layout)`: `add_slide` with `instantiate(layout.slide)` (fresh slide and element ids) after the current slide.
  - `addPackOp(kind: "widgets" | "layouts", namespace, pin)`.
  - `canAdd(slideKind, name)`: `slideKind === ESSENTIEL.content || (slideKind === ESSENTIEL.blank && name !== "text")`. This is the lot 2 rule on the constants (lot 5 replaces it with schema slots).
  - `ImportDialog` retrofit target: `{ layouts: string[], theme }`, defaulting to `["essentiel"]`. The list comes from `api.packs("layouts")` filtered on `profiles > 0`.

- [ ] **Step 1: Write the failing tests**

```tsx
// ui/src/LayoutPicker.render.test.tsx
test("LayoutPicker lists each layout pack with its layouts", async () => {
  const api = fakeApi({ layouts: { packs: [{ namespace: "essentiel", name: "Essentiel", layouts: [
    { kind: "essentiel/title", label: "Couverture", slide: slideOf("essentiel/title"), svg: "<svg/>" },
    { kind: "essentiel/content", label: "Contenu", slide: slideOf("essentiel/content"), svg: "<svg/>" },
  ] }] } });
  const view = render(<LayoutPicker api={api} onPick={() => {}} onClose={() => {}} />);
  await view.findByText("Essentiel");
  expect(view.getAllByRole("button", { name: /Couverture|Contenu/ }).length).toBe(2);
});

test("LayoutPicker picking a card returns its layout", async () => {
  const picked: string[] = [];
  const api = fakeApi({ layouts: oneLayout("essentiel/end", "Fin") });
  const view = render(<LayoutPicker api={api} onPick={(l) => picked.push(l.kind)} onClose={() => {}} />);
  (await view.findByRole("button", { name: "Fin" })).click();
  expect(picked).toEqual(["essentiel/end"]);
});
```

```ts
// ui/src/builtinKinds.test.ts
test("ESSENTIEL kinds are qualified by essentiel", () => {
  expect(Object.values(ESSENTIEL).every((k) => k.startsWith("essentiel/"))).toBe(true);
});

// ui/src/slideTools.test.ts (cases rewritten)
test("canAdd on essentiel content accepts any native widget", () => {
  expect(canAdd(ESSENTIEL.content, "text")).toBe(true);
});
test("canAdd on a bare content kind refuses", () => {
  expect(canAdd("content", "card")).toBe(false);
});

// ui/src/deck.test.ts
test("instantiate gives fresh slide and element ids", () => {
  const s = instantiate(slideWithBlock("layout-content", "b"));
  expect([s.id === "layout-content", s.params.blocks[0].id === "b"]).toEqual([false, false]);
});

// ui/src/packOps.test.ts
test("addPackOp carries the pack kind", () => {
  expect(addPackOp("layouts", "acme", pin()).kind).toBe("layouts");
});
```

`ImportDialog.render.test.tsx`: the dialog lists the layout packs that have profiles, `Essentiel` is checked by default, and submitting sends `{ layouts: ["essentiel"] }`.

- [ ] **Step 2: Run, expect FAIL**: `cd ui && bun test`.

- [ ] **Step 3: Implement**: as under Interfaces. `App.tsx` wires `LayoutPicker` where `TemplatePicker` was (no `initialSet`, no `onSetChange`, `adoptSet` gone).

- [ ] **Step 4: Run, expect PASS**: `cd ui && bun test && bunx tsc --noEmit`.

- [ ] **Step 5: Commit**: `feat(ui)! : sélecteur de layouts à la place des modèles, kinds essentiel, ops de packs par sorte, import sur packs de layouts (t-1066)`.

---

### Task 9: UI — pack mode, the atelier without templates, e2e

**Files:**
- Rename:
  - `ui/src/TemplateTabs.tsx` → `ui/src/PackTabs.tsx` (the "Modèles" sub-tab deleted)
  - `ui/src/TemplatesBrand.tsx` → `ui/src/PackBrand.tsx`
  - `ui/src/useTemplateWidgets.ts` → `ui/src/usePackWidgetEditor.ts`
  - plus their `*.render.test.tsx`
- Modify:
  - `ui/src/useHostSettings.ts:16,51` + `useHostSettings.render.test.tsx` (`HOST_TEMPLATE` deleted)
  - `ui/src/App.tsx` (lines from the survey: 11, 40-61, 247-251, 289-312, 331-438), `ui/src/AppMenu.tsx:80-102`, `ui/src/close.ts:18-20` + `close.test.ts`
  - `ui/src/useFileActions.ts:55-56`, `ui/src/useSlideOps.ts:75`, `ui/src/useAppKeys.ts:73,99,108`, `ui/src/TabBar.tsx:217`, `ui/src/workspaceApi.ts:11`
  - `ui/src/Stage.tsx:98,167,439` (`templates` prop → `packMode`), `ui/src/ThemePanel.tsx:24`, `ui/src/useThemeSettings.ts:37-40`, `ui/src/deck.ts:351-365`
  - `ui/src/agent/{scope.ts:7,21,useAgentScope.ts:14}`, `ui/src/useAgentWiring.ts:66-71`, `ui/src/useTargetSettings.ts:46`
  - the tests named in the UI survey (`TargetPanel`, `WidgetPicker`, `WidgetPane`, `useKeptUi`, `agent/*`, `TabBar`, `styles.test.ts`)
- Modify i18n: `*/ui.ftl` (template captions, menu and pile labels renamed to pack ones: `ui-caption-templates` → `ui-caption-pack`, `ui-pile-templates` → `ui-pile-layouts`, etc.; keys of the deleted host-template surface removed), 20 locales
- Modify e2e:
  - `ui/e2e/030-panel.cjs` (rewritten: in pack mode on Essentiel, change the grid of `layout-blank` through the slide panel, save, reopen, see the grid kept)
  - `200-theme-editor.cjs:30-55` (first template "couverture" → the `layout-title` slide)
  - `070-agent-scopes.cjs:38-82`, `160-widgets.cjs:20`, `170-code-layout.cjs:40-45`, `180-widget-values.cjs:16-21`, `190-lists-dock.cjs:19-24` (the atelier menu label)
  - `220-import.cjs:188-242` (`packs` API, no `template_sets`), `020-toolbar.cjs:22-36` (`.card-name`, layout picker), `050-help.cjs:37`
  - `ui/e2e/lib/*` helpers that open the atelier
- Modify: `ui/ARCHITECTURE.md`, `ui/AGENTS.md`

**Interfaces:**
- Consumes: Task 6 (`Snapshot.editing: PackMode | null`, renamed requests), Task 8.
- Produces:
  - `api.editPack(id)`, `api.savePack(asNew)`, `api.closePack()`, `api.deletePack(id)`, `api.packs(kind)`, `api.layouts()`.
  - Pack mode UI: the brand shows the pack name and kind; pile section `layouts` (the working deck's slides) for a layout pack, none for a widget pack; `PackTabs` = widget tabs + theme tab.
  - In pack mode the add-slide button is hidden and slide drag, delete and duplicate are disabled (the host refuses them anyway; the UI does not offer them).

- [ ] **Step 1: Write the failing tests**

```tsx
// ui/src/PackTabs.render.test.tsx
test("PackTabs has no templates sub-tab", () => {
  const view = render(<PackTabs {...packTabsProps({ kind: "layouts" })} />);
  expect(view.queryByText(/Modèles|Templates/)).toBeNull();
});

// ui/src/App.render.test.tsx (or the test that renders App in editing mode)
test("pack mode hides the add-slide button", async () => {
  const view = renderApp(snapshotInPackMode("layouts"));
  expect(view.queryByRole("button", { name: /Nouvelle slide|New slide/ })).toBeNull();
});

// ui/src/useHostSettings.render.test.tsx
test("no host-template surface exists in pack mode", () => {
  const { result } = renderHook(() => useHostSettings(snapshotInPackMode("layouts")));
  expect(result.current.surfaces.some((s) => s.id === "host-template")).toBe(false);
});

// ui/src/close.test.ts
test("closing a tab in pack mode with changes asks to save the pack", async () => {
  const calls: string[] = [];
  await closeTab(fakeTabApi({ editing: dirtyPackMode(), savePack: () => calls.push("save") }), "save");
  expect(calls).toEqual(["save"]);
});
```

(Adapt the helper names to the existing tests these replace. The behaviors are fixed.) e2e `030-panel.cjs`, its new body:
1. Open the atelier on Essentiel (menu).
2. Select the slide `layout-blank` in the pile.
3. Change its grid to 4 columns through the target panel.
4. Save as new.
5. Close.
6. Reopen the new pack.
7. Assert that `layout-blank`'s grid is 4 columns.

- [ ] **Step 2: Run, expect FAIL**: `cd ui && bun test`.

- [ ] **Step 3: Implement**: renames and removals as under Files. Wire `snap.editing.kind` into the pile and the add-slide button.

- [ ] **Step 4: Run, expect PASS**: `cd ui && bun test && bunx tsc --noEmit`, then `just e2e 4999`.

- [ ] **Step 5: Commit**: `feat(ui)! : mode pack sans modèles dans l'atelier, onglets de pack, e2e (t-1067)`.

---

### Task 10: Docs, rules, consolidation and full gate

**Files:**
- Modify docs:
  - `docs/aide/{en,fr}.json`: version 8, slide kinds `essentiel/*`; line 261 "template set / mode modèles" → layout packs / pack mode; any prose listing widgets says slide layouts come from the `essentiel` pack. Keep the strings other tests quote intact, or update `crates/oku-slides-session/src/help_tests.rs`.
  - `README.md` (crate map, packs), `typst/oku-slides/README.md` (if Task 3 left anything), `docs/superpowers/specs/2026-10-18-packs-layouts-widgets-themes-design.md` (§2, §3, §6, §8, §9 per the deviations, if Task 1's commit did not already carry them; see Step 1)
- Modify the rules: every `AGENTS.md` naming templates, Grilles, `DEFAULT_PACK`, the core slide widgets or `base-registry.json` (`rg -n -i 'template|gabarit|grilles|base-registry|DEFAULT_PACK' --glob '**/AGENTS.md' --glob '**/ARCHITECTURE.md'`)
- Modify: the t-1008 body (lot 3 done, lots 4–5 remaining), sub-task statuses

**Interfaces:** none.

- [ ] **Step 1: Spec update check.** The spec changes listed under "Deviations" are committed with this plan (the plan commit). Here, verify with `git log -p -- docs/superpowers/specs/2026-10-18-packs-layouts-widgets-themes-design.md` that they landed; add anything missing.
- [ ] **Step 2: Consolidation pass (G10).** Then run `just agents-lint`.
  - `rg -n -i 'template|gabarit' crates ui/src src-tauri typst --glob '!**/snapshots/**'` returns only unrelated hits: Typst `fill-template` and `template-fields` (core, header and footer), and `cargo-generate`. Each remaining hit is justified in the task report.
  - `rg -n '"(title|toc|chapter|content|end|blank)"' crates ui/src --glob '!**/*_tests.rs' --glob '!**/*.test.ts*'` returns only param names.
  - `rg -n 'core_registry|core_probe|base_pack_files|base_pack_widgets|GRID_PACK|DEFAULT_PACK|child_pack' crates` returns nothing.
  - No duplicate helpers between `builtinKinds.ts` and the Rust constants beyond the mirrored values.
- [ ] **Step 3: Full gate.**
  - Run `just ci` and `just e2e 4999`. All green.
  - The golden pages snapshot is unchanged (`git diff main -- crates/oku-slides-render/tests/snapshots/golden__reference_pages.snap` is empty).
- [ ] **Step 4: Commit**: `docs(packs) : aide, README et règles du lot 3, consolidation (t-1068)`.
- [ ] **Step 5: Finish.**
  1. Run `git config --worktree --unset oku.review`.
  2. Squash into `main` with the `BREAKING CHANGE:` footer of the Global Constraints.
  3. Push.
  4. Remove the worktree and the branch.
  5. Mark t-1068 (Task 10) done.
  6. Comment on t-1057 (items 1, 3, 4 done, item 2 open), close t-1054 and t-1056, and leave t-1053 and t-1055 open with a comment naming why.
