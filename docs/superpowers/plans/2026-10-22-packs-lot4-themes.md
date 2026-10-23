# Typed packs, lot 4 — themes: `extends`, component tokens owned by packs — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Themes get a `namespace` and an ordered `extends` list. A deck pins its theme and that theme's whole `extends` closure. The component tokens, and the Typst code that reads them, move from the core into the pack that owns the component. The theme editor is composed of a global section plus one section per embedded pack. The hi-fi import writes `deck.tokens` on a new built-in theme `neutre`, and deck-only themes disappear.

**Architecture:**
- **Typst.** The functions that read component tokens (`slide`, `title-slide`, `chapter`, `toc`, `end-slide`, `card` and their helpers) leave `src/slides.typ` for `packs/essentiel/layouts/_commun.typ` and `packs/base/widgets/card.typ`. They read `ctx.component`, the tokens of their own pack's namespace, so no pack file names its namespace.
- **Tokens.** The four groups shared by several owners (`header`, `footer`, `list`, `kicker`) become global token families of the core. The other nine groups are declared in the manifests as `tokens` entries (`{path, default, control, …}`) and exposed as `component.<namespace>.<group>`.
- **Rust.** Rust composes the theme layer and sends it as `/theme.json`:
  1. the component defaults of every embedded pack;
  2. then the deck's theme chain, linearized.

  Typst's merge (`defaults ← theme ← deck.tokens`) is unchanged, so the worker protocol is unchanged.
- **Deck.** `Deck.theme: Option<Namespace>` names the theme. `Deck.themes: BTreeMap<Namespace, ThemePin>` holds its materialized closure, the same shape as `deck.widgets` and `deck.layouts`.
- **Formats.** Decks move to version 9, the replica format to 7, the sources to 13.

**Tech Stack:** Typst (embedded), Rust (Snafu, serde, speculoos, rstest, insta), Fluent (20 locales), React/TS (Bun), Playwright e2e.

**Spec:** `docs/superpowers/specs/2026-10-18-packs-layouts-widgets-themes-design.md` (§1, §2, §3, §4, §6, §7, §9 item 4). Previous plans, for what exists and their deviations:
- `docs/superpowers/plans/2026-10-18-packs-lot1-widget-packs.md`
- `docs/superpowers/plans/2026-10-18-packs-lot2-base.md`
- `docs/superpowers/plans/2026-10-19-packs-lot3-essentiel.md`

**Task:** parent t-1008; sub-tasks t-1095 … t-1102 (Task 1 … Task 8).

## Global Constraints

- **Where each component token group goes** (from `default-tokens.json` `component`, and from the themes):

  | Group | Before | After | Reader after |
  |---|---|---|---|
  | `header`, `footer` | `component.header`, `component.footer` | global families `header`, `footer` | core `chrome` (`src/core.typ`) |
  | `list` | `component.list` | global family `list` | core `deck()` `set list`, `base/card`, `base/text` |
  | `kicker` | `component.kicker` | global family `kicker` | core `kicker()`, `base/card`, essentiel layouts |
  | `card`, `table` | `component.card`, `component.table` | `component.base.card`, `component.base.table` | `base` pack (`ctx.component`) |
  | `title`, `title-slide`, `chapter`, `toc`, `end-slide`, `quote`, `columns` | `component.<group>` | `component.essentiel.<group>` | `essentiel` pack (`ctx.component`) |

  - After this lot, `default-tokens.json` has no `component` key.
  - Every value moves verbatim: the same value, under its new path, in the defaults, the pack manifests, `pro.json` and `papier.json`.
- **Golden render test**
  - Every page is pixel-identical: `crates/oku-slides-render/tests/snapshots/golden__reference_pages.snap` is unchanged.
  - `golden__reference_boxes.snap` is unchanged too.
  - Any diff is a bug, never a re-record.
- **Pack manifest `tokens`.** A list of entries, in both `widgets.json` and `layouts.json`:

  ```json
  {"path": ["card", "bg"], "default": "bg", "control": "color"}
  ```

  - `path` is relative to `component.<namespace>`.
  - `control` and its parameters take the format of the entries of `theme_schema.json`, without `section`.
  - `nullable` and `optional` work as in `theme_schema.json`.
  - `default` is required unless `optional` is true. A literal `default: null` is a value, not an absence.
  - `label` is optional. It is the editor label for a pack that is not built in. A built-in pack's entries are labelled through Fluent (`theme-token-component-<ns>-<path joined with ->`) and carry no `label`.
- **Themes**
  - `theme.json` gains `namespace` (required) and `extends: [namespace…]` (ordered, omitted when empty). It loses `scope`.
  - A namespace is unique across all three kinds in the library (`NamespaceTaken`). `deck` and `deck-layouts` stay reserved.
  - Built-in themes, in library order:

    | Name | Namespace | Id | Notes |
    |---|---|---|---|
    | Papier | `papier` | `2a7d4c91-5e3b-4f08-b6a1-8c9e0d1f2a31` | `DEFAULT_THEME`, unchanged |
    | Pro | `pro` | `7e1b9c3d-2f4a-4d6e-a8b0-5c7d9e1f3b42` | |
    | Neutre | `neutre` | `9c4e2b7a-1d3f-4a6b-8e5c-2f7a0b9d4c61` | new, `tokens: {}`, file `typst/oku-slides/themes/neutre.json`, `created: "2026-10-22T00:00:00Z"` |

    None of them extends another.
  - Linearization (spec §4):
    - depth-first post-order;
    - each theme is applied once, at its first position;
    - later parents win;
    - the merge is deep by path;
    - a list replaces.

    A cycle is `ThemeCycle { chain }`.
- **Rendering layers**: `/theme.json` = `component.<ns>` defaults of every embedded layout and widget pack, then each theme of the linearized chain (the deck's theme last), merged with `oku_slides_core::tokens::merge`. Typst then merges `default-tokens.json ← /theme.json ← deck.tokens`, as today.
- **Format bumps**
  - `DECK_VERSION` 8 → 9, `REPLICA_FORMAT` 6 → 7, `SOURCES_VERSION` 12 → 13.
  - Worker `PROTOCOL_VERSION` stays 5, and the resume state `VERSION` stays 2.
  - No migration (MVP rule, precedent t-532). The "newer version" tests move from 9 to 10.
  - Every deck JSON fixture, help deck and test literal follows: version 9, `theme` a namespace, `themes` pins.
- **Built-in pack versions**
  - `base` and `essentiel` both change: each gets a second line in its `versions.txt` (append only).
  - `base/widgets.json` loses `"templates": []`.
- **Kind and namespace literals**
  - A theme namespace in Rust comes only from `oku_slides_session::theme::{PAPIER_NAMESPACE, PRO_NAMESPACE, NEUTRE_NAMESPACE}`.
  - In TS, the UI never hard-codes one.
  - The rules of lots 2 and 3 for `kinds::base` and `kinds::essentiel` still hold.
- **Errors and code rules**
  - No `unwrap`/`expect` outside tests, and no silent fallback.
  - Snafu errors with a backtrace per variant.
  - Every new, renamed or deleted Fluent key changes in all 20 locales (`crates/oku-slides-i18n/src/locales/*/`), at the same position, translated.
- **Tests**
  - Rust unit tests go in a sibling `x_tests.rs`, use speculoos, are named `unit_scenario_result`, and use rstest for tables.
  - Regressions go in `tests/regressions/issue_NNNN_topic.rs`, declared in that crate's `tests/regressions/main.rs` (NNNN = the sub-task number). A regression is first seen failing for its stated reason.
- **Docs**
  - Every pub item is documented, first sentence under 15 words. Code comments are in French; docs in English.
  - Source files stay under 500 lines when possible, never above 1000.
  - Each `ARCHITECTURE.md` and `AGENTS.md` change lands in the task that changes the code it describes. `ARCHITECTURE.md` stays ≤ 50 lines.
- **Commits**
  - Format `type(scope) : description (t-N)`, message in a scratchpad file passed with `git commit -F`, **no `Co-Authored-By` line**.
  - SDD protocol: `git config --worktree oku.review deferred` during execution, removed at the end. Implementers only stage and write `<workspace>/task-N-commit.txt`. The controller commits, with the trailer `Review-waived: revue SDD par tâche et revue finale (oku.review=deferred, t-1008)`.
  - The squash commit carries a `BREAKING CHANGE:` footer naming:
    - deck version 9, replica format 7, sources 13;
    - the refusal of version-8 decks and the setting aside of version-8 tabs (with a notice);
    - `deck.theme` is now a namespace, plus `deck.themes` pins; the `set_theme` op takes a namespace;
    - component token paths moved (`header`, `footer`, `list`, `kicker` global; `component.base.*`, `component.essentiel.*`);
    - deck-only themes deleted: the hi-fi import writes `deck.tokens` on `neutre`;
    - user library themes without a namespace are ignored, with a notice;
    - saving an edited built-in theme creates a theme that extends it;
    - the MCP outline and `theme_tokens` shapes;
    - the slide functions (`slide`, `title-slide`, `chapter`, `toc`, `end-slide`, `card`) removed from `lib.typ`.
- Work deferred to lot 5 is tracked under t-1008. A code comment that defers names `t-1008`.
- Runs outside `just e2e` set `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS` to a fresh temp dir.

## Deviations from the spec (approved with this plan; spec §1, §2, §3, §4, §6, §7 and §9 updated in the same commit)

1. **The code that reads component tokens moves with them** (user decision).
   - `slide`, `_slide-title`, `_layout`, `title-slide`, `chapter`, `toc`, `end-slide` go to `packs/essentiel/layouts/_commun.typ`, and `card` goes to `packs/base/widgets/card.typ`.
   - They read `ctx.component`. `render-element` and `render-slide` set it to `t.component.at(<namespace of the kind>, default: (:))`.
   - The core keeps `deck`, `blank-slide` (the fallback frame), `kicker`, `chrome` and the helpers. `lib.typ` stops exporting the moved functions and exports the helpers they need (`styled`, `text-style`, `case-of`, `_heading`, `_mark`, `chapter-number`, `chapter-counter`, `fields-stack`, `config`).
2. **Shared groups become global families** (user decision). `header`, `footer`, `list` and `kicker` are top-level token families of the core, read where they were read before. No pack declares them, and no component name can clash with a namespace.
3. **A manifest's `tokens` is a list of typed entries** (user decision), not a dictionary of values. Each entry is one source for three things: the default (`component.<ns>` defaults), the editor control, and the validation. `Control` moves from the session to `oku-slides-core` (`token_spec.rs`) so that `PackManifest` can carry it.
4. **The deck pins a namespace and a closure map** (user decision).
   - `Deck.theme: Option<Namespace>` and `Deck.themes: BTreeMap<Namespace, ThemePin { id, version, extends }>`, instead of `theme: ThemeRef` plus ancestors.
   - `Op::SetTheme { theme: Option<Namespace>, themes: BTreeMap<Namespace, ThemePin> }`.
   - The core refuses a cycle with `ThemeCycle { chain }`, and with the new `ThemeClosure { namespace }` a map that is not exactly the closure of `theme` (missing, extra, or an `extends` that differs).
   - `None` keeps its meaning: the default theme (`papier`), which the session adopts explicitly.
5. **The theme layer is composed in Rust.** `tokens::theme_layer` merges the pack component defaults and the linearized chain. The session, the CLI and the import send that layer as `/theme.json`, so Typst and the worker protocol do not change. For standalone Typst use, `tokens.typ` gains `pack-tokens(namespace, manifest)`; `examples/from-json.typ` and the README use it.
6. **The built-in theme `neutre`** (user decision).
   - The hi-fi import pins `neutre` and writes its generated tokens with `SetTokens`, so its render matches today's deck-only theme over the defaults.
   - `ImportPlan.theme`, `PackScope::Deck` on themes, `deck_themes.rs`, `Snapshot.deck_themes` and `ThemeInfo.deck_only` are deleted.
   - The import rule "never emit token overrides" (`crates/oku-slides-import/AGENTS.md`) now applies to the retrofit only.
7. **User library themes without a namespace are ignored** (user decision, no migration). They are unreadable as a `ThemeManifest`. The theme library lists them as skipped, and the session raises the notice `notice-themes-skipped` once per process, with their count.
8. **Saving an edited built-in theme creates a child** (user decision).
   - The child extends the built-in: `extends: [<built-in namespace>]`. It holds only the edited tokens, and its namespace comes from its name (`library_namespace`).
   - Editing a built-in starts from an empty draft over a layer that includes the built-in. Editing a user theme edits its own tokens over its parents.
   - The editor shows `extends` read-only, plus the stashed deck's overrides. Editing the `extends` list goes to lot 5, as a ticket under t-1008 opened by Task 8.
   - The MCP op and the manifest accept any `extends`.
9. **Widget packs get preview slides** (user decision).
   - In pack mode, a widget pack's working deck has one host slide per element widget, id `widget-<name>`, kind `essentiel/content`, holding one block of that widget seeded from its example.
   - Every op on such a slide or its elements is refused with `SessionErrorKind::PackModePreview` (`session-pack-mode-preview`).
   - Writing or removing a widget file realigns them.
   - The UI shows them only on the Theme tab.
10. **Theme files keep their canonical form.**
    - `PackFiles::theme(manifest)`, `PackFiles::theme_from_bytes(json)` and `PackFiles::verified_theme(hex, json)` replace every theme call site that passed `PackKind::Widgets`.
    - Internally a theme is still prefix-free, so a theme's version is a pure function of its `theme.json`, as before.
11. **The retrofit keeps measuring with the built-ins' tokens.** `measure.rs`, `zone.rs` and `color.rs` read `component.essentiel.title` and `component.base.{card,table}` whatever layout pack they target, as they read the unqualified groups before. They read `kicker` and `list` as global families.
12. **Folded tickets**
    - **t-1055**: `base` changes, so its `versions.txt` has two lines. A fixture copy of the previous shipped `base` (it imports `card` from `lib.typ`, which no longer exports it) fails its probe. The session returns the error instead of disabling a trusted pack.
    - **t-1094**: `bun audit` clean (markdown-it, vite).
    - `"templates": []` leaves `base/widgets.json`.
13. **Intermediate commits.**
    - Tasks 1–5 go bottom-up one layer at a time. Each task's gate is the crates it touches: Task 1 core and i18n, Task 2 render and worker, Task 3 import, Tasks 4–5 session.
    - The workspace is green again from Task 6. `just ci` and `just e2e` run in Task 8.
    - The branch is squash-merged.

---

### Task 1: Core — themes with a namespace and extends, ThemePin closure, component tokens in pack manifests, formats bumped

Sub-task: t-1095.

**Files:**
- Create in `crates/oku-slides-core/src/`:
  - `token_spec.rs` + `token_spec_tests.rs` (`Control`, moved from session; `ComponentToken`)
  - `theme_chain.rs` + `theme_chain_tests.rs` (`ThemePin`, `linearize`, `check_closure`)
- Modify core:
  - `src/pack.rs:194-203,473-507`:
    - `ThemeRef` deleted;
    - `ThemeManifest { namespace, extends }` without `scope`;
    - `PackManifest.tokens: Vec<ComponentToken>` and `PackManifest::component_defaults()`;
    - `PackFiles::{theme, theme_from_bytes, verified_theme}`.
  - Tests: `pack_tests.rs`.
  - `src/model.rs:21-48`: `DECK_VERSION = 9`, `theme: Option<Namespace>`, `themes: BTreeMap<Namespace, ThemePin>`. Tests: `model_tests.rs`.
  - `src/ops/mod.rs:212-215,663-670`: `SetTheme { theme, themes }` applied through `check_closure`. Tests: `tests/ops.rs`.
  - `src/error/mod.rs`, `src/error/tests.rs`: `OpErrorKind::{ThemeCycle, ThemeClosure}`.
  - `src/tokens.rs`: `theme_layer`. Tests: `tokens_tests.rs`.
  - `src/sources.rs:48`: `SOURCES_VERSION = 13`.
  - `src/sources_themes.rs`: `verify_themes` via `PackFiles::verified_theme`; `theme_issues` checks every pin. Tests: `sources_themes_tests.rs`.
  - `src/validate.rs:77-82`: new issue `theme_pin_mismatch`.
  - `src/replica/mod.rs:70`: `REPLICA_FORMAT = 7`.
  - `src/replica/schema.rs:112-142`: `"themes"` nested like the pack pins.
  - Tests: `replica/*_tests.rs`, `tests/convergence.rs`.
  - `src/lib.rs`: re-exports.
  - `ARCHITECTURE.md`, `AGENTS.md`.
- Modify i18n (`crates/oku-slides-i18n/src/locales/*/core.ftl`, 20 locales): add `op-theme-cycle` (`{ $chain }`), `op-theme-closure` (`{ $namespace }`), `issue-theme-pin-mismatch` (`{ $namespace }`).
- Modify every core fixture and test literal for version 9: `rg -l '"version": ?8' crates/oku-slides-core fixtures`.

**Interfaces:**
- Consumes: `Namespace`, `PackPin`, `PackFiles`, `PackKind`, `tokens::merge` (lots 1–3).
- Produces:
  - `pub enum Control` (unchanged variants: `Color`, `Pt`, `Em`, `Weight`, `Fonts`, `Choice`, `Bool`, `Text`, `Palette`, `Number`, `Rule`), now `Serialize + Deserialize`, in `oku_slides_core::token_spec`.
  - `pub struct ComponentToken`:

    ```rust
    pub struct ComponentToken {
        pub path: Vec<String>,
        #[serde(flatten)]
        pub control: Control,
        pub default: Option<Value>,
        pub nullable: bool,
        pub optional: bool,
        pub label: Option<String>,
    }
    ```

    - `default: Some(Value::Null)` for a literal `null`.
    - `pub fn check(&self) -> Result<(), TokenEntryError>`: a missing `default` without `optional`, or an empty `path`.
  - `PackManifest.tokens: Vec<ComponentToken>` (`skip_serializing_if = "Vec::is_empty"`, so the versions of packs without tokens are unchanged), and `pub fn component_defaults(&self) -> Map<String, Value>`, the nested defaults dictionary, without the entries that have no default.
  - `ThemeManifest { id, name, namespace: Namespace, extends: Vec<Namespace>, description, parent, created, tokens }`.
  - `PackFiles::theme(manifest: Map<String, Value>) -> PackFiles`, `PackFiles::theme_from_bytes(json: &[u8]) -> Option<PackFiles>`, `PackFiles::verified_theme(hex: &str, json: &[u8]) -> Option<PackFiles>`.
  - `pub struct ThemePin { pub id: Uuid, pub version: PackVersion, pub extends: Vec<Namespace> }` (`extends` skipped when empty), `JsonSchema`.
  - `pub fn linearize(theme: &Namespace, pins: &BTreeMap<Namespace, ThemePin>) -> Result<Vec<Namespace>, ThemeChainError>`, with variants:
    - `Cycle { chain: Vec<Namespace> }`
    - `Missing { namespace }`
  - `pub fn check_closure(theme: Option<&Namespace>, pins: &BTreeMap<Namespace, ThemePin>) -> Result<(), ThemeChainError>`. It adds the variant `Extra { namespace }`. `None` requires an empty map.
  - `pub fn theme_layer<'a>(components: impl IntoIterator<Item = (&'a Namespace, Map<String, Value>)>, chain: impl IntoIterator<Item = &'a Map<String, Value>>) -> Map<String, Value>`.
  - `Deck { theme: Option<Namespace>, themes: BTreeMap<Namespace, ThemePin>, … }`.
  - `Op::SetTheme { theme: Option<Namespace>, #[serde(default)] themes: BTreeMap<Namespace, ThemePin> }`.
  - `OpErrorKind::ThemeCycle` and `OpErrorKind::ThemeClosure`.
  - `Issue::theme_pin_mismatch(namespace)`.

- [ ] **Step 1: Write the failing tests**

```rust
// crates/oku-slides-core/src/theme_chain_tests.rs
fn pin(extends: &[&str]) -> ThemePin {
    ThemePin { id: Uuid::nil(), version: PackVersion::of(b"x"), extends: extends.iter().map(|n| ns(n)).collect() }
}

#[rstest]
#[case::single(&[("marque", &[][..])], "marque", &["marque"])]
#[case::multiple(&[("marque", &["papier", "sombre"][..]), ("papier", &[]), ("sombre", &[])], "marque", &["papier", "sombre", "marque"])]
#[case::diamond(&[("marque", &["papier", "sombre"][..]), ("papier", &["neutre"]), ("sombre", &["neutre"]), ("neutre", &[])], "marque", &["neutre", "papier", "sombre", "marque"])]
fn linearize_closure_gives_post_order_each_once(#[case] pins: &[(&str, &[&str])], #[case] theme: &str, #[case] want: &[&str]) {
    let map = pins.iter().map(|(n, e)| (ns(n), pin(e))).collect();
    assert_that!(linearize(&ns(theme), &map).map(|c| c.iter().map(|n| n.as_str().to_owned()).collect::<Vec<_>>()))
        .is_ok_containing(want.iter().map(|s| s.to_string()).collect::<Vec<_>>());
}

#[test]
fn linearize_cycle_names_the_chain() {
    let map = [(ns("a"), pin(&["b"])), (ns("b"), pin(&["a"]))].into_iter().collect();
    let err = linearize(&ns("a"), &map).unwrap_err();
    assert_that!(err).is_equal_to(ThemeChainError::Cycle { chain: vec![ns("a"), ns("b"), ns("a")] });
}

#[rstest]
#[case::missing_parent(&[("a", &["b"][..])], Some("a"))]
#[case::extra_pin(&[("a", &[][..]), ("b", &[])], Some("a"))]
#[case::pins_without_theme(&[("a", &[][..])], None)]
fn check_closure_not_exact_is_refused(#[case] pins: &[(&str, &[&str])], #[case] theme: Option<&str>) {
    let map = pins.iter().map(|(n, e)| (ns(n), pin(e))).collect();
    assert_that!(check_closure(theme.map(ns).as_ref(), &map)).is_err();
}
```

```rust
// crates/oku-slides-core/src/tokens_tests.rs (additions)
#[test]
fn theme_layer_later_parent_wins_and_list_replaces() {
    let components = [(ns("base"), json!({"card": {"bg": "bg", "gap": "sm"}}).as_object().cloned().unwrap_or_default())];
    let papier = json!({"color": {"accent": "#111"}, "font": {"body": ["A", "B"]}});
    let sombre = json!({"color": {"accent": "#222"}, "font": {"body": ["C"]}, "component": {"base": {"card": {"bg": "ink"}}}});
    let layer = theme_layer(components.iter().map(|(n, m)| (n, m.clone())), [papier.as_object().unwrap(), sombre.as_object().unwrap()]);
    assert_that!(Value::Object(layer)).is_equal_to(json!({
        "component": {"base": {"card": {"bg": "ink", "gap": "sm"}}},
        "color": {"accent": "#222"}, "font": {"body": ["C"]},
    }));
}
```

```rust
// crates/oku-slides-core/src/token_spec_tests.rs
#[rstest]
#[case(json!({"path": ["card", "bg"], "default": "bg", "control": "color"}), true)]
#[case(json!({"path": ["chapter", "number-color"], "control": "color", "optional": true}), true)]
#[case(json!({"path": ["title", "rule"], "default": null, "control": "rule", "nullable": true}), true)]
#[case(json!({"path": ["card", "bg"], "control": "color"}), false)]
#[case(json!({"path": [], "default": 1, "control": "bool"}), false)]
fn component_token_check_requires_default_unless_optional(#[case] entry: Value, #[case] ok: bool) {
    let token: ComponentToken = serde_json::from_value(entry).unwrap();
    assert_that!(token.check().is_ok()).is_equal_to(ok);
}

#[test]
fn component_token_literal_null_default_is_kept() {
    let token: ComponentToken = serde_json::from_value(json!({"path": ["t", "r"], "default": null, "control": "rule", "nullable": true})).unwrap();
    assert_that!(token.default).is_equal_to(Some(Value::Null));
}
```

```rust
// crates/oku-slides-core/src/pack_tests.rs (additions)
#[test]
fn pack_manifest_without_tokens_keeps_its_version() {
    // Un manifeste sans `tokens` s'écrit comme avant : même empreinte.
    let files = sample_pack_files();
    let again = PackManifest::from_files(&files).unwrap().to_files(files.kind(), files.widgets().clone()).unwrap();
    assert_that!(again.version()).is_equal_to(files.version());
}

#[test]
fn component_defaults_nest_paths_and_skip_optional() {
    let m = manifest_with_tokens(json!([
        {"path": ["card", "bg"], "default": "bg", "control": "color"},
        {"path": ["chapter", "number-color"], "control": "color", "optional": true},
    ]));
    assert_that!(Value::Object(m.component_defaults())).is_equal_to(json!({"card": {"bg": "bg"}}));
}

#[test]
fn theme_files_hash_like_a_bare_theme_json() {
    let json = br#"{"id":"2a7d4c91-5e3b-4f08-b6a1-8c9e0d1f2a31","name":"T","namespace":"t","created":"2026-10-22T00:00:00Z","tokens":{}}"#;
    let files = PackFiles::theme_from_bytes(json).unwrap();
    assert_that!(PackFiles::verified_theme(files.version().hex(), json)).is_some();
}
```

```rust
// crates/oku-slides-core/tests/ops.rs (additions)
#[test]
fn set_theme_with_its_closure_pins_theme_and_parents() { /* SetTheme marque + {marque, papier} → deck.theme, deck.themes */ }

#[rstest]
#[case::cycle(OpErrorKind::ThemeCycle)]
#[case::closure(OpErrorKind::ThemeClosure)]
fn set_theme_bad_closure_is_refused(#[case] kind: OpErrorKind) { /* one map per case */ }

#[test]
fn set_theme_none_clears_the_pins() {}

#[test]
fn deck_version_nine_reads_and_ten_is_refused() {}
```

Also, in `sources_themes_tests.rs`:
- every pin of `deck.themes` without its version in the sources gives `theme_version_missing`;
- a pin whose manifest namespace, id or `extends` differs gives `theme_pin_mismatch`.

In `error/tests.rs`, both new kinds are localized in `en` and `fr` (assert on the key).

- [ ] **Step 2: Run them to see them fail**

Run: `cargo nextest run -p oku-slides-core`
Expected: compile errors on `ThemePin`, `linearize`, `ComponentToken`, `theme_layer`, `PackFiles::theme`.

- [ ] **Step 3: Implement**
  1. Move `Control` from `crates/oku-slides-session/src/theme_schema.rs:22-62` into `token_spec.rs`, adding `Serialize` with the same tags. Keep `pub use oku_slides_core::token_spec::Control;` in the session so that crate keeps compiling.
  2. `ComponentToken.default` uses a small `deserialize_with` that wraps any present value in `Some`, so a literal `null` is kept.
  3. `linearize`: recursive post-order with a stack for the cycle chain and a `seen` set; a parent not in `pins` is `Missing`.
  4. `check_closure`: `linearize`, then compare the set of reached namespaces with the keys (an extra key is `Extra`).
  5. Then check that each pin's `extends` is what was walked (it is, by construction: `linearize` walks `pin.extends`).
  6. `Op::SetTheme`: map `ThemeChainError::Cycle` to `OpInner::ThemeCycle { chain }` (chain joined with ` → `) and the others to `ThemeClosure { namespace }`.
  7. `sources_themes::theme_issues`: for every `(ns, pin)`, check that the version is present, then that the manifest has `namespace == ns`, `id == pin.id` and `extends == pin.extends`.
  8. Replica: `(Deck, "themes")` is a nested map of pins like `widgets`/`layouts`, and `(Deck, "theme")` stays an atom (now a string).
  9. Bump `DECK_VERSION`, `REPLICA_FORMAT` and `SOURCES_VERSION`. Rewrite the core deck fixtures.
  10. Delete `ThemeRef` and fix the core call sites. Session and later crates are fixed in their own tasks.

- [ ] **Step 4: Run the gate**

Run: `cargo nextest run -p oku-slides-core -p oku-slides-i18n && cargo clippy -p oku-slides-core -p oku-slides-i18n --all-targets -- -D warnings`
Expected: PASS. Other crates may not compile yet (deviation 13).

- [ ] **Step 5: Docs.** Update `crates/oku-slides-core/ARCHITECTURE.md` (theme manifest, `ThemePin`, `theme_chain`, `token_spec`; `PackScope` now packs only) and `crates/oku-slides-core/AGENTS.md`. Add the rule: "Resolve a theme's tokens only through theme_chain::linearize and tokens::theme_layer; never merge a theme chain by hand. (plan:2026-10-22-packs-lot4-themes)". Run `just agents-lint`.

- [ ] **Step 6: Stage and write `task-1-commit.txt`**: `feat(core)! : thèmes à espace de noms et extends, épinglage ThemePin, jetons de composant dans les manifestes, deck 9 (t-1095)`.

---

### Task 2: Typst and render — component tokens and their code in the packs, global frame families, Neutre, golden identical

Sub-task: t-1096.

**Files:**
- Typst library (`typst/oku-slides`):
  - `src/default-tokens.json`: `component` removed, and `header`, `footer`, `list`, `kicker` become top-level families.
  - `src/slides.typ`:
    - keeps `deck`, `_mark`, `_heading`, `kicker` and `blank-slide`;
    - `deck()` reads `t.list`;
    - `kicker()` reads `t.kicker`;
    - `slide`, `_slide-title`, `_layout`, `title-slide`, `chapter`, `toc`, `end-slide` and `card` are moved out.
  - `src/core.typ:193-216` (`chrome` reads `t.at(kind)`; `default-header` reads `t.header`, `default-footer` reads `t.footer`).
  - `src/data.typ:151,198` (`ctx.component`).
  - `src/tokens.typ`: `pack-tokens(namespace, manifest)`.
  - `lib.typ` (exports; `neutre`).
  - `packs/essentiel/layouts/_commun.typ`: receives the slide functions.
  - `packs/essentiel/layouts/{title,toc,chapter,content,end}.typ`: call them with `ctx`.
  - `packs/essentiel/layouts.json`: `tokens` entries for `title`, `title-slide`, `chapter`, `toc`, `end-slide`, `quote`, `columns`.
  - `packs/essentiel/versions.txt`: second line.
  - `packs/base/widgets/card.typ`: receives `card`.
  - `packs/base/widgets/text.typ:41` (`t.list`) and `packs/base/widgets/table.typ:46` (`ctx.component.table`).
  - `packs/base/widgets.json`: `tokens` for `card` and `table`, `templates` removed.
  - `packs/base/versions.txt`: second line.
  - `themes/pro.json`, `themes/papier.json` (namespace; groups moved per the Global Constraints table); `themes/neutre.json` (new).
  - `examples/from-json.typ`, `examples/deck.json` (version 9).
  - `README.md` (Tokens and Bundled themes sections), `AGENTS.md:21` ("component tokens live in the owning pack's manifest; a theme sets them under component.<ns>"), `ARCHITECTURE.md` if present.
- Render (`crates/oku-slides-render`):
  - `src/world.rs:33-52` (`LIBRARY_DATA` serves `themes/neutre.json`)
  - `src/builtin_packs.rs:20-25` (no new file: `card` was already listed). Tests: `builtin_packs_tests.rs`.
  - `tests/common/mod.rs:120-157`: `theme_json(name)` returns `theme_layer(builtin pack defaults, [theme tokens])`; `reference_theme` "none" sends the packs-only layer; `theme_name` matches by namespace.
  - `tests/golden.rs` and `tests/golden/widgets.json` (version 9), `fixtures/roadmap.json` (version 9).
  - Any render test that sets `component.*` tokens or a theme: `tests/{render,tables}.rs`, `tests/regressions/issue_0554_police_graphiques_du_theme.rs`, `tests/regressions/issue_0841_encre_carte_mise_en_avant.rs`, `src/{palette,charts,world}_tests.rs`.
  - `ARCHITECTURE.md`, `AGENTS.md`.
- Render worker: only test literals using version 8 or a theme (`src/*_tests.rs`, `tests/process.rs`).

**Interfaces:**
- Consumes: `PackManifest::component_defaults`, `tokens::theme_layer`, `ThemeManifest.namespace` (Task 1).
- Produces:
  - Typst `ctx.component`: the dictionary `t.component.<namespace of the widget's kind>`, empty when the pack declares none.
  - Typst `pack-tokens(namespace, manifest)`: `(component: ((namespace): <defaults from manifest.tokens>))`.
  - Rust `BuiltinPack::component_defaults(self) -> Result<Map<String, Value>, RegistryError>` (from its manifest), and `pub fn builtin_component_defaults() -> Result<Vec<(Namespace, Map<String, Value>)>, RegistryError>`, used by tests, the CLI and the import.

- [ ] **Step 1: Write the failing tests**

```rust
// crates/oku-slides-render/src/builtin_packs_tests.rs (additions)
#[rstest]
#[case(BuiltinPack::Base, &["card", "table"])]
#[case(BuiltinPack::Essentiel, &["chapter", "columns", "end-slide", "quote", "title", "title-slide", "toc"])]
fn builtin_pack_declares_its_component_groups(#[case] pack: BuiltinPack, #[case] groups: &[&str]) {
    let defaults = pack.component_defaults().unwrap();
    assert_that!(defaults.keys().map(String::as_str).collect::<Vec<_>>()).is_equal_to(groups.to_vec());
}

#[test]
fn default_tokens_have_no_component_and_the_four_frame_families() {
    let defaults: Map<String, Value> = serde_json::from_slice(include_bytes!("../../../typst/oku-slides/src/default-tokens.json")).unwrap();
    assert_that!(defaults.contains_key("component")).is_false();
    for family in ["header", "footer", "list", "kicker"] {
        assert_that!(defaults.contains_key(family)).is_true();
    }
}

#[rstest]
#[case(BuiltinPack::Base)]
#[case(BuiltinPack::Essentiel)]
fn builtin_pack_has_two_shipped_versions(#[case] pack: BuiltinPack) {
    assert_that!(pack.shipped_versions().count()).is_equal_to(2);
}
```

Also a render test (`tests/render.rs`) checks that a layer setting `component.base.card.bg` changes the card fill. Setting the old `component.card.bg` must change nothing, which proves the old path is dead.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo nextest run -p oku-slides-render builtin_pack default_tokens`
Expected: FAIL (`component_defaults` missing; `component` still in the defaults).

- [ ] **Step 3: Move the tokens.**
  1. Write the manifest `tokens` entries from `default-tokens.json` and `crates/oku-slides-session/src/theme_schema.json`.
     - Each entry's `control` (and parameters, `nullable`, `optional`) is copied from the schema entry of the same leaf. Its `default` comes from `default-tokens.json`. Its `path` drops the `component` prefix.
     - `chapter.number-color` is `optional` with no default.
  2. Rewrite `pro.json` and `papier.json` with the group moves of the Global Constraints table, values verbatim, and add `namespace`.
  3. Create `neutre.json`.
  4. Append each built-in pack's new version to its `versions.txt`. `builtin_pack_current_version_is_listed_as_shipped` prints the hex.

- [ ] **Step 4: Move the code.**
  1. Move the functions listed in deviation 1 into the packs, verbatim apart from the token lookups:
     - `config().tokens.component.<g>` → `ctx.component.<g>` for a pack group;
     - `t.component.<g>` → `t.<g>` for a global family.
  2. Their signatures gain `ctx` as the first argument where they had none.
  3. `render-element` and `render-slide` set `ctx.component`.
  4. `lib.typ` exports per deviation 1, plus `neutre`.
  5. The essentiel layouts import the functions from `_commun.typ`, and `card.typ` defines `card` locally.

- [ ] **Step 5: Golden.**
  - Update `tests/common/mod.rs` so every render receives the layer (built-in pack defaults + theme).
  - Run `cargo nextest run -p oku-slides-render --test golden`. Expected: PASS with both snapshots unchanged (`git diff --stat -- crates/oku-slides-render/tests/snapshots/golden__*` empty).
  - On a diff, find the token or code difference (G13–G15), never re-record.

- [ ] **Step 6: Run the gate**

Run: `cargo nextest run -p oku-slides-render -p oku-slides-render-worker && cargo clippy -p oku-slides-render -p oku-slides-render-worker --all-targets -- -D warnings`
Expected: PASS. `widgets__pack_base.snap`, `widgets__pack_essentiel.snap` and `fixtures/widgets/builtin-registry.json` do not change (the probe reads schemas, not tokens); any change is justified in the report.

- [ ] **Step 7: Docs.**
  - Update `typst/oku-slides/README.md`: the layering line becomes defaults ← packs' component defaults ← theme chain ← deck tokens; the family table gains `header`, `footer`, `list`, `kicker` and `component.<namespace>.*`; the manifest shape gains `namespace` and `extends`; Neutre is listed.
  - Update `typst/oku-slides/AGENTS.md:21`, the render `ARCHITECTURE.md` and `AGENTS.md`.
  - Run `just agents-lint`.

- [ ] **Step 8: Stage and write `task-2-commit.txt`**: `feat(typst)! : jetons de composant et leur code dans base et essentiel, familles header/footer/list/kicker, thème Neutre (t-1096)`.

---

### Task 3: Import — hi-fi tokens in deck.tokens on Neutre, retrofit on the new token paths

Sub-task: t-1097.

**Files:**
- Modify `crates/oku-slides-import/src/`:
  - `hifi/theme.rs:15-148`:
    - `theme_manifest` deleted;
    - `theme_tokens` writes `list` (top-level) and `component.base.table`;
    - module doc.
  - `hifi/mod.rs:63-82,153-190`:
    - `HifiTarget.theme_id` removed;
    - ops `SetTheme { theme: Some(neutre), themes: {} }` (the session fills the closure) then `SetTokens { tokens }`;
    - `plan.theme` gone.
  - `plan.rs:78-81,115,164,194-195`: `ImportPlan.theme` and `take_theme` deleted.
  - `retrofit/measure.rs:51-123`: `list`, `kicker` global; `component.essentiel.{title,columns}`; `component.base.{card,table}`.
  - `retrofit/zone.rs:113-120`: `component.essentiel.title`.
  - `retrofit/color.rs:92-107`: `component.base.card`.
  - `test_support.rs:27`: defaults plus the built-in packs' component defaults (`builtin_component_defaults`).
  - Tests and snapshots: `hifi/theme_tests.rs`, `hifi/mod_tests.rs`, `retrofit/{color,measure,zone}_tests.rs`, `hifi/snapshots/*theme*.snap`, `tests/hifi.rs:41,72-77`, `tests/snapshots/hifi__*_ops.snap`, `tests/retrofit.rs`, `tests/common/mod.rs:164`, `tests/regressions/issue_0753_output_budget.rs`, `tests/regressions/issue_0755_shared_theme.rs`.
- Modify `crates/oku-slides-import/ARCHITECTURE.md:10-14,34,39-40` and `AGENTS.md:44`: "The retrofit never emits raw hex colors or token overrides; the hi-fi import writes its generated tokens into deck.tokens on the Neutre theme. (plan:2026-10-22-packs-lot4-themes)".

**Interfaces:**
- Consumes: `Op::SetTheme { theme, themes }`, `Op::SetTokens` (Task 1); `builtin_component_defaults` (Task 2). The namespace literal `neutre` is passed by the session in `HifiTarget.theme: Namespace`. The import crate does not depend on the session.
- Produces: `HifiTarget { …, theme: Namespace }` (the theme to pin); `ImportPlan` without `theme`. The plan's first theme op is `SetTheme { theme: Some(target.theme), themes: BTreeMap::new() }`, and the session completes the closure.

- [ ] **Step 1: Write the failing tests**

```rust
// crates/oku-slides-import/src/hifi/mod_tests.rs (replacing the theme-file assertions)
#[test]
fn hifi_plan_pins_the_target_theme_then_sets_its_tokens() {
    let plan = minimal_plan();
    let ops: Vec<&Op> = plan.ops.iter().filter(|o| matches!(o, Op::SetTheme { .. } | Op::SetTokens { .. })).collect();
    assert_that!(ops.len()).is_equal_to(2);
    assert_that!(matches!(ops[0], Op::SetTheme { theme: Some(t), .. } if t.as_str() == "neutre")).is_true();
    assert_that!(matches!(ops[1], Op::SetTokens { tokens } if tokens["weight"]["strong"] == 700)).is_true();
}

// crates/oku-slides-import/src/hifi/theme_tests.rs
#[test]
fn theme_tokens_put_list_global_and_table_under_base() {
    let tokens = kitchen_tokens();
    assert_that!(tokens.contains_key("list")).is_true();
    assert_that!(tokens["component"]["base"].get("table")).is_some();
    assert_that!(tokens["component"].get("list")).is_none();
}

// crates/oku-slides-import/src/retrofit/zone_tests.rs
#[test]
fn page_tokens_read_the_essentiel_title() { /* tokens with component.essentiel.title.size = 77 → title zone uses 77 */ }
```

- [ ] **Step 2: Run them to see them fail.** Run: `cargo nextest run -p oku-slides-import`. Expected: FAIL to compile (`theme_manifest`, `ImportPlan.theme`).

- [ ] **Step 3: Implement** per the file list. Regenerate the hi-fi snapshots with `cargo insta test -p oku-slides-import --review`. Review them: only the theme op, the new `set_tokens` op, the token paths and the absent `theme.json` change.

- [ ] **Step 4: Run the gate**

Run: `cargo nextest run -p oku-slides-import && cargo clippy -p oku-slides-import --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 5: Stage and write `task-3-commit.txt`**: `feat(import)! : l'import hi-fi écrit deck.tokens sur Neutre, retrofit sur les nouveaux chemins de jetons (t-1097)`.

---

### Task 4: Session — theme library and deck theme closure, render layer, deck-only themes deleted

Sub-task: t-1098.

**Files:**
- Modify `crates/oku-slides-session/src/`:
  - `theme.rs`:
    - `PAPIER_NAMESPACE`, `PRO_NAMESPACE`, `NEUTRE_NAMESPACE`, `NEUTRE_THEME`;
    - built-ins Papier, Pro, Neutre via `PackFiles::theme_from_bytes`;
    - `VersionManifest::namespace`;
    - `ThemeInfo { namespace, extends }` without `deck_only`;
    - `ThemeLibrary::skipped() -> usize`.
  - `library.rs:264,394-407`:
    - a theme version is read through `PackFiles::theme_from_bytes`;
    - an unreadable user theme is counted as skipped, not listed.
  - `library_packs.rs`: `PackLibrary` holds a clone of the `ThemeLibrary` for namespace checks.
    - `check_kinds` refuses a namespace owned by a theme.
    - `namespaces()` includes themes.
    - New `pub fn check_theme(&self, id, ns) -> Result<()>`.
  - `namespace_slug.rs:62`: `library_namespace` covers themes (through `namespaces()`).
  - `deck_theme.rs`:
    - `adopt_default_theme` pins the `papier` closure;
    - `theme_tokens` → `theme_layer(&self, deck) -> Map`;
    - `render_themed` sends it;
    - `embed_op_themes` fills `SetTheme.themes` from the library (`theme_closure`) and embeds every version;
    - `set_theme(ns)`.
  - Create `theme_closure.rs` + tests: `pub(crate) fn theme_closure(themes: &ThemeLibrary, ns: &Namespace) -> Result<(BTreeMap<Namespace, ThemePin>, Vec<PackFiles>)>`, with `UnknownTheme` when a namespace is absent, plus the `ThemeCycle` check.
  - Delete `deck_themes.rs` and `deck_themes_tests.rs`. Remove `Snapshot.deck_themes` (`lib.rs:662`, `views.rs`).
  - `deck_theme_edit.rs:12-14,82-114,156-163`: deck-only branches deleted (Task 5 finishes the editor).
  - `theme_picker.rs:94,147-191`: cards by namespace, deck-only themes gone.
  - `library_lookup.rs:119-126`: `embed_theme(themes, wanted, bundle)` matches a namespace, id or name and embeds the closure.
  - `resume.rs:537-548`: `PackFiles::verified_theme`.
  - `import/hifi.rs:72-135`: no theme identity half (the pack id stays); pins `NEUTRE_NAMESPACE`.
  - `import/retrofit.rs:50-126`: effective tokens = `merge(defaults, theme_layer)`.
  - `import/queue.rs:148-157`: the key hashes the layer plus `deck.tokens`.
  - `import/render.rs:35-37`: sends the layer.
  - `import/error.rs`: `ThemeMissing` deleted.
  - `deck_widgets.rs:~151` and `deck_widgets_tests.rs`: the t-1055 test.
  - `pack_notice.rs` or the notice module: `notice-themes-skipped`.
  - `error.rs`: `ThemeCycle` and `ThemeClosure` mapped from the op error.
  - `test_support.rs:285,328-340`: `adopt_deck_only_theme` deleted.
- Create the fixture `fixtures/packs/base-fe1495fc/` (`widgets.json` + `widgets/*.typ`, copied from `git show 47e3da97:typst/oku-slides/packs/base/…`) with a `README.md` line naming its origin. Read `fixtures/AGENTS.md` first.
- Tests:
  - `theme_tests.rs`, `theme_picker_tests.rs`, `deck_theme_tests.rs`, `library_lookup_tests.rs`, `library_namespace_tests.rs`, `resume_tests.rs`, `import/{mod,hifi,retrofit,queue,diff}_tests.rs`, `merge_tests.rs`;
  - `tests/{session,workspace,merge,history}.rs`, `tests/common/hifi_import.rs`;
  - `tests/regressions/issue_0903_repick_deck_only_theme.rs` deleted (its behavior no longer exists), and its line in `tests/regressions/main.rs` removed.
- i18n `session.ftl` (20 locales):
  - add `notice-themes-skipped` (`{ $count }`), `session-theme-cycle` (`{ $chain }`), `session-theme-closure` (`{ $namespace }`);
  - delete `session-import-theme-missing`.
- `crates/oku-slides-session/ARCHITECTURE.md:11,16-20,26` and `AGENTS.md`; `crates/AGENTS.md:14` ("Never write a deck-only pack (PackScope::Deck) to the library"; themes have no scope).

**Interfaces:**
- Consumes: Tasks 1–3.
- Produces:
  - `pub const PAPIER_NAMESPACE: &str = "papier"`, `PRO_NAMESPACE = "pro"`, `NEUTRE_NAMESPACE = "neutre"`, `pub const NEUTRE_THEME: Uuid`.
  - `ThemeInfo { id, version, name, namespace: Namespace, extends: Vec<Namespace>, builtin, default }`.
  - `Session::theme_layer(&self, deck: &Deck) -> Result<Map<String, Value>>`. In pack mode, Task 5 extends it.
  - `Snapshot.theme: Option<Namespace>` and `Snapshot.theme_pins: BTreeMap<Namespace, ThemePin>`; `deck_themes` removed.
  - `Request::SetTheme { theme: String }` (namespace, id or name, as `embed_theme`).

- [ ] **Step 1: Write the failing tests**

```rust
// crates/oku-slides-session/src/theme_closure_tests.rs
#[test]
fn theme_closure_of_a_child_pins_it_and_its_parent() {
    let (themes, _dir) = library_with_child("marque", &["papier"]);
    let (pins, files) = theme_closure(&themes, &ns("marque")).unwrap();
    assert_that!(pins.keys().map(Namespace::as_str).collect::<Vec<_>>()).is_equal_to(vec!["marque", "papier"]);
    assert_that!(files.len()).is_equal_to(2);
}

#[test]
fn theme_closure_with_an_unknown_parent_is_refused() {
    let (themes, _dir) = library_with_child("marque", &["absent"]);
    assert_that!(theme_closure(&themes, &ns("marque")).unwrap_err().kind()).is_equal_to(SessionErrorKind::UnknownTheme);
}

// crates/oku-slides-session/src/theme_tests.rs
#[test]
fn builtin_themes_are_papier_pro_neutre_with_namespaces() {
    let ns: Vec<_> = ThemeLibrary::builtin_only().themes().into_iter().map(|t| t.namespace.as_str().to_owned()).collect();
    assert_that!(ns).is_equal_to(vec!["papier".to_owned(), "pro".to_owned(), "neutre".to_owned()]);
}

#[test]
fn user_theme_without_namespace_is_skipped_and_counted() {
    let (themes, _dir) = library_with_raw_theme(r#"{"id":"…","name":"Old","created":"2026-10-01T00:00:00Z","tokens":{}}"#);
    assert_that!(themes.themes().iter().any(|t| t.name == "Old")).is_false();
    assert_that!(themes.skipped()).is_equal_to(1);
}

// crates/oku-slides-session/src/library_namespace_tests.rs
#[rstest]
#[case::pack_over_theme("papier")]
fn pack_with_a_theme_namespace_is_refused(#[case] taken: &str) { /* add_version of a widget pack named `taken` → NamespaceTaken */ }

#[test]
fn library_namespace_skips_theme_namespaces() { /* name "Papier" → "papier-2" */ }

// crates/oku-slides-session/src/deck_theme_tests.rs
#[test]
fn theme_layer_holds_pack_defaults_under_the_theme() {
    let s = session_with_theme(PRO_NAMESPACE);
    let layer = s.theme_layer(s.deck()).unwrap();
    assert_that!(layer["component"]["base"]["card"]["title-weight"]).is_equal_to(json!(800)); // Pro
    assert_that!(layer["component"]["base"]["card"]["height"]).is_equal_to(json!("auto"));   // base default
}

#[test]
fn set_theme_op_without_pins_gets_the_library_closure() {}

#[test]
fn new_deck_pins_papier() {}
```

```rust
// crates/oku-slides-session/tests/regressions/issue_1098_theme_layer_of_a_child.rs
// Un enfant de Papier rend avec les jetons de Papier sous les siens : le
// calque envoyé au rendu suit la chaîne linéarisée.
```

```rust
// crates/oku-slides-session/src/deck_widgets_tests.rs (t-1055)
#[test]
fn trusted_old_shipped_base_failing_its_probe_is_an_error() {
    let old = fixture_pack("packs/base-fe1495fc");
    assert_that!(oku_slides_render::is_trusted_pack(&old.widgets())).is_true();
    let err = resolve_with(old).unwrap_err();
    assert_that!(err.to_string()).contains("card");
}
```

Also: `hifi_imports_pin_neutre_with_deck_tokens` (`import/mod_tests.rs`, replacing `hifi_generates_a_deck_only_pack_and_theme`), and a resume test reading a version-8 tab, which is set aside with its notice.

- [ ] **Step 2: Run them to see them fail.** Run: `cargo nextest run -p oku-slides-session`. Expected: compile errors (constants, `theme_closure`, `theme_layer`).

- [ ] **Step 3: Implement** per the file list:
  1. `PackLibrary` gets its `ThemeLibrary` at construction. Update the constructors and the session's `Libraries` wiring.
  2. The skipped count is computed once per scan.
  3. The notice is raised once per process (same pattern as the existing library notices in `pack_notice.rs`).

- [ ] **Step 4: Run the gate**

Run: `cargo nextest run -p oku-slides-session && cargo clippy -p oku-slides-session --all-targets -- -D warnings`
Expected: PASS, except the theme-editor tests that Task 5 rewrites. List them in the report and mark them `#[ignore = "t-1099"]` only if they cannot compile otherwise. Task 5 removes every such `ignore`.

- [ ] **Step 5: Docs** (see the file list), then `just agents-lint`.

- [ ] **Step 6: Stage and write `task-4-commit.txt`**: `feat(session)! : bibliothèque de thèmes à espaces de noms, fermeture extends épinglée, calque de rendu, thèmes du deck seul supprimés (t-1098, t-1055)`.

---

### Task 5: Session — composed theme editor, child copies, widget-pack preview slides

Sub-task: t-1099.

**Files:**
- Modify `crates/oku-slides-session/src/`:
  - `theme_schema.json`: every `component.*` entry is deleted, and the `header`, `footer`, `list`, `kicker` entries move to top-level paths with `"section": "frame"`.
  - `theme_schema.rs`:
    - `Section::Components` → `Section::Frame`, plus `Section::Pack(Namespace)` (`#[serde(skip)]`);
    - `pub fn composed(packs: &[(Namespace, String, Vec<ComponentToken>)]) -> Vec<TokenSpec>`;
    - `spec`, `validate` and `users` take the composed schema.
  - `theme_panel.rs:36-47,84-87,201-275`:
    - sections = the global ones, then one per embedded pack (label = pack name);
    - a token label is the Fluent key for a built-in pack, otherwise the entry's `label`, otherwise the path joined with `.`;
    - a read-only "extends" row (`theme-extends`) and a read-only list of the stashed deck's `deck.tokens` paths (`theme-deck-overrides`).
  - `theme_controls.rs`: no change of behavior; it takes `TokenSpec` from the composed schema.
  - `theme_edit.rs:42-380`:
    - `ThemeEdit { base, name, child: bool }`;
    - `initial_theme_edit`: a built-in base gives `child: true` and an empty draft;
    - `save_theme_edit`: a child creates `{ namespace: library_namespace(name), extends: [base ns], tokens: draft }`, a user theme a new version (same namespace and `extends`), and `PackScope::Deck` branches are deleted;
    - `check_token_ops` uses the composed schema.
  - `pack_mode.rs:1-17,161-176,214-291`:
    - the draft's layer is the linearized chain of the base's parents, or of the base itself for a child;
    - a widget pack's working deck gets its preview slides;
    - `realign_widget_previews`.
  - Create `pack_mode_preview.rs` + `pack_mode_preview_tests.rs`: `preview_slides(pack: &PackFiles, ns: &Namespace) -> Vec<Slide>` and the refusal of ops targeting `widget-*` slides.
  - `deck_theme_edit.rs:126-186`: `edit_theme` on a built-in creates a child (as `save_theme_edit`), and on a user theme a new version.
  - `agent/context.rs:123-135`: "theme: <name> (extends <names>)".
  - `error.rs`: `PackModePreview`.
- i18n `session.ftl` (20 locales):
  - rename `theme-token-component-{header,footer,list,kicker}-*` → `theme-token-{header,footer,list,kicker}-*`;
  - rename `theme-token-component-{card,table}-*` → `theme-token-component-base-{card,table}-*`;
  - rename `theme-token-component-{title,title-slide,chapter,toc,end-slide,quote,columns}-*` → `theme-token-component-essentiel-…`;
  - `theme-section-components` → `theme-section-frame`;
  - add `theme-extends` (`{ $names }`), `theme-deck-overrides`, `session-pack-mode-preview`;
  - update `theme-copy-name` only if its wording says "copy" of a snapshot (it is now a child: keep the key, wording "{ $name } (variante)" in fr, "{ $name } (variant)" in en, translated elsewhere).
- Tests:
  - `theme_schema_tests.rs:149-363` (cases on new paths; `schema_accepts_every_default_value` over defaults + built-in pack defaults; `schema_accepts_every_builtin_theme_value` over the three themes);
  - `theme_panel_tests.rs`, `theme_edit_tests.rs`, `deck_theme_edit_tests.rs`;
  - `tests/regressions/issue_0554_action_jeton_sans_cle_path.rs`, `issue_0902_deck_only_workshop.rs` (deleted if it only tests deck-only themes; otherwise rewritten on a deck-only pack);
  - `tests/snapshots/agent__agent_context_*.snap`.

**Interfaces:**
- Consumes: Task 4 (`theme_layer`, `theme_closure`, `ThemeInfo`), Task 1 (`ComponentToken`).
- Produces:
  - `ThemeEditView { base, name, child, extends: Vec<String> /* names */ }`.
  - The `host-theme` surface with sections `colors, typography, scripts, spacing, shapes, page, charts, frame, pack:<ns>…, app-light, app-dark` (pack sections in namespace order).
  - Preview slide ids `widget-<name>`.

- [ ] **Step 1: Write the failing tests**

```rust
// crates/oku-slides-session/src/theme_schema_tests.rs (additions)
#[test]
fn composed_schema_adds_one_section_per_pack() {
    let packs = vec![(ns("base"), "Base".to_owned(), base_tokens())];
    let schema = composed(&packs);
    assert_that!(schema.iter().any(|s| s.section == Section::Pack(ns("base")) && s.path == ["component", "base", "card", "bg"])).is_true();
}

#[rstest]
#[case(&["header", "left"], Section::Frame)]
#[case(&["kicker", "case"], Section::Frame)]
#[case(&["list", "marker"], Section::Frame)]
fn frame_families_are_global(#[case] path: &[&str], #[case] section: Section) {
    assert_that!(spec(&composed(&[]), &strings(path)).map(|s| s.section.clone())).is_equal_to(Some(section));
}

// crates/oku-slides-session/src/theme_edit_tests.rs (additions)
#[test]
fn saving_an_edited_builtin_creates_a_child_with_only_the_edits() {
    let mut s = pack_session_with_theme(PAPIER_NAMESPACE);
    s.apply_token(&["color", "accent"], json!("#123456")).unwrap();
    let r = s.save_theme_edit().unwrap().unwrap();
    let m = s.theme_manifest(&r).unwrap();
    assert_that!(m.extends).is_equal_to(vec![ns("papier")]);
    assert_that!(Value::Object(m.tokens)).is_equal_to(json!({"color": {"accent": "#123456"}}));
}

#[test]
fn editing_a_user_theme_keeps_its_namespace_and_extends() {}

#[test]
fn pack_mode_layer_of_a_child_draft_includes_its_builtin() {}

// crates/oku-slides-session/src/pack_mode_preview_tests.rs
#[test]
fn widget_pack_working_deck_has_one_preview_slide_per_widget() {
    let deck = working_deck_of(BuiltinPack::Base);
    assert_that!(deck.slides.iter().map(|s| s.id.as_str()).collect::<Vec<_>>())
        .is_equal_to(vec!["widget-card", "widget-chart", "widget-image", "widget-shape", "widget-stack", "widget-table", "widget-text"]);
}

#[rstest]
#[case(Op::SetParam { id: "widget-card".into(), key: "title".into(), value: json!("x") })]
#[case(Op::RemoveSlide { id: "widget-card".into() })]
fn op_on_a_preview_slide_is_refused(#[case] op: Op) {
    assert_that!(apply_in_widget_pack(op).unwrap_err().kind()).is_equal_to(SessionErrorKind::PackModePreview);
}

#[test]
fn writing_a_widget_file_adds_its_preview_slide() {}
```

(`Op::SetParam` stands for whatever op the core uses to set a slide param. The implementer uses its real name and shape from `crates/oku-slides-core/src/ops/mod.rs`.)

Also: `theme_panel_tests.rs` checks the section order, the read-only `theme-extends` row and the overrides list. The session's i18n test checks that every built-in pack entry has its Fluent key in `en` and `fr`.

- [ ] **Step 2: Run them to see them fail.** Run: `cargo nextest run -p oku-slides-session theme pack_mode_preview`. Expected: FAIL.

- [ ] **Step 3: Implement** per the file list.
  - The composed schema for a deck comes from the manifests of its pinned packs: `deck.widgets` + `deck.layouts`, read from the sources or the built-ins.
  - Remove every `#[ignore = "t-1099"]` left by Task 4.

- [ ] **Step 4: Run the gate**

Run: `cargo nextest run -p oku-slides-session -p oku-slides-i18n && cargo clippy -p oku-slides-session --all-targets -- -D warnings`
Expected: PASS, with no `t-1099` ignore left (`rg -n 't-1099' crates` → nothing).

- [ ] **Step 5: Docs.** Update the session `ARCHITECTURE.md` (theme editor composed, child copies, preview slides) and `AGENTS.md` (`SetToken` validation uses the composed schema). Run `just agents-lint`.

- [ ] **Step 6: Stage and write `task-5-commit.txt`**: `feat(session) : éditeur de thème composé par pack, variantes des thèmes intégrés, aperçu des packs de widgets (t-1099)`.

---

### Task 6: CLI, MCP, agent and local agent on theme namespaces — workspace green

Sub-task: t-1100.

**Files:**
- CLI (`crates/oku-slides-cli/src/`):
  - `main.rs:54-88,340-385,459-466`: `--theme` takes a namespace, id or name; render, save and bench send the layer.
  - `pack_render.rs:84-98`, `bench.rs:21,37`, `import.rs:83-129`.
  - Tests: `tests/cli.rs`, `tests/import.rs`.
  - i18n `cli.ftl` (`cli-arg-theme`, `cli-arg-import-theme` wording: "namespace, id or name").
- MCP (`crates/oku-slides-mcp/src/`):
  - `tools.rs:254-305` descriptions:
    - `deck_outline` themes `[{namespace,name,extends,builtin}]`, no `deck_only`;
    - `theme_tokens` returns `{namespace, extends, tokens}` (the theme's own tokens) plus `deck_tokens`;
    - `edit_theme`: a built-in is saved as a variant extending it, after confirmation; for a change to this deck only, use `set_token` ops;
    - `apply`: `set_theme` takes `{"theme": "<namespace>"}`.
  - `read_tools.rs:14-97`, `guarded_tools.rs:42-51,295-315` (the deck-only skip removed: every built-in edit asks), `surface_tools.rs:215-235`, `tool_error.rs:71-113`, `hub/themes.rs`.
  - Tests: `tests/{themes,guarded,import,edit,surfaces,turns,server}.rs`.
  - `ARCHITECTURE.md:13-27`, `AGENTS.md:50,54`.
- Agent: `crates/oku-slides-agent/src/instructions/{agent,import,widgets}.md`, `tests/{stack,turns}.rs`, `ARCHITECTURE.md:11`, `AGENTS.md:7`; i18n `agent.ftl` (`agent-confirm-copy-theme` wording "creates a variant of…").
- Local agent: `crates/oku-slides-local-agent/src/families.rs:46-67`.
- The rest of the workspace: every remaining compile error and test literal (`rg -n 'ThemeRef|deck_themes|deck_only|theme_id' crates src-tauri --glob '!**/snapshots/**'`).

**Interfaces:**
- Consumes: Tasks 1–5.
- Produces:
  - the MCP JSON shapes above;
  - the agent instruction text using namespaces (`set_theme` with `"pro"`) and the component paths (`component.base.card…`, `kicker`, `list`).

- [ ] **Step 1: Write the failing tests**

```rust
// crates/oku-slides-mcp/tests/themes.rs (additions)
#[tokio::test]
async fn deck_outline_lists_themes_by_namespace_with_extends() {
    let out = outline(&hub_with_child_theme("marque", &["papier"])).await;
    assert_that!(out["themes"].as_array().unwrap().iter().any(|t| t["namespace"] == "marque" && t["extends"] == json!(["papier"]))).is_true();
}

#[tokio::test]
async fn edit_theme_of_a_builtin_asks_and_creates_a_variant() {}

#[tokio::test]
async fn apply_set_theme_by_namespace_pins_the_closure() {}

// crates/oku-slides-cli/tests/cli.rs (additions)
#[test]
fn render_with_theme_namespace_uses_that_theme() {}
```

- [ ] **Step 2: Run them to see them fail.** Run: `cargo nextest run -p oku-slides-mcp -p oku-slides-cli -p oku-slides-agent -p oku-slides-local-agent`. Expected: FAIL.

- [ ] **Step 3: Implement** per the file list.

- [ ] **Step 4: Workspace gate**

Run: `just test` (nextest without the app + UI unit tests) and `cargo clippy --workspace --all-targets -- -D warnings`.
Expected: Rust PASS. UI unit tests that need Task 7 are listed in the report, not fixed here.

- [ ] **Step 5: Stage and write `task-6-commit.txt`**: `feat(agent) : CLI, MCP et agent sur les espaces de noms de thème, variantes et deck.tokens (t-1100)`.

---

### Task 7: UI — theme namespaces, extends and deck overrides, widget-pack preview, e2e

Sub-task: t-1101.

**Files:**
- Modify `ui/src/`:
  - `api.ts:64-72,194-245,573,609-618,840-844`:
    - `ThemeRef` removed;
    - `Snapshot.theme: string | null`, `Snapshot.theme_pins: Record<string, ThemePin>`;
    - `ThemeInfo { namespace, extends }` without `deck_only`;
    - `deck_themes` removed;
    - `ThemeEditView { child, extends }`;
    - op `set_theme { theme: string | null }`.
  - `deck.ts:351-365`: the look key uses `theme_pins` (versions) and `editing.theme`.
  - `ThemeMenu.tsx`: the "This presentation" group is deleted; the item for a theme pinned but absent from the library stays (`ui-toolbar-theme-embedded`). `ThemePanel.tsx`: base by namespace.
  - `ImportDialog.tsx:20-152`: filter `!th.default` only.
  - `App.tsx:321-323,404-407`: on a widget pack, the Theme tab shows the stage with the preview slides; the other tabs keep `ui-stage-widget-pack`.
  - `a2ui/ThemeChoice.tsx` if it reads `deck_only` or `ThemeRef`.
  - Tests:
    - `ThemeMenu.render.test.tsx`, `ThemePanel.render.test.tsx`, `ImportDialog.render.test.tsx`, `a2ui/ThemeChoice.render.test.tsx`;
    - `a2ui/fixtures/agent-surfaces.json`;
    - a new `App` render test for the widget-pack Theme tab, if `App` has render tests (otherwise e2e only).
- i18n `ui.ftl` (20 locales): delete `ui-toolbar-theme-deck-group`.
- e2e:
  - `ui/e2e/200-theme-editor.cjs`:
    - saving an edited Papier gives a variant that extends Papier (the panel shows "Étend : Papier");
    - the stashed deck's overrides list shows a token set on the deck.
  - `ui/e2e/220-import.cjs:109-118,190-201`: a hi-fi import pins `neutre`, with non-empty `deck.tokens`; the repick test becomes "pick Papier, then back to Neutre keeps the deck tokens".
  - `ui/e2e/030-panel.cjs:52-69`: the section names.
  - A new step in `200-theme-editor.cjs`: the Theme tab of a widget pack (`base`, edited in pack mode) shows one preview slide per widget.
- `ui/ARCHITECTURE.md` / `ui/AGENTS.md` if they name deck-only themes.

**Interfaces:**
- Consumes: the Task 4–5 snapshot and views.
- Produces: none for later tasks.

- [ ] **Step 1: Write the failing tests**

```tsx
// ui/src/ThemeMenu.render.test.tsx (replacing the deck-group case)
test("ThemeMenu without deck-only themes lists library themes by namespace", () => {
  const view = renderMenu({ themes: [theme("papier"), theme("pro"), theme("neutre")], value: "pro" });
  expect(view.getAllByRole("option").map((o) => o.getAttribute("value"))).toEqual(["papier", "pro", "neutre"]);
});

// ui/src/ImportDialog.render.test.tsx
test("ImportDialog offers every non-default library theme", () => { /* neutre and pro offered, papier is the default entry */ });
```

- [ ] **Step 2: Run them to see them fail.** Run: `cd ui && bun test`. Expected: FAIL (types and props).

- [ ] **Step 3: Implement** per the file list. Then run `bun x tsc --noEmit` from `ui/`.

- [ ] **Step 4: e2e**

Run: `just e2e 4999`
Expected: PASS. `just e2e` sets its own data dirs.

- [ ] **Step 5: Stage and write `task-7-commit.txt`**: `feat(ui) : thèmes par espace de noms, variante et surcharges du deck, aperçu des packs de widgets (t-1101)`.

---

### Task 8: Docs, rules, bun audit, consolidation and full gate

Sub-task: t-1102.

**Files:**
- Modify docs:
  - `docs/aide/{en,fr}.json`:
    - version 9;
    - `s-grille` picker text (`:257-261`);
    - `s-import` `:525,573` (the hi-fi import uses Neutre with its own tokens on the deck);
    - `s-typst-jetons` `:1608-1680` (layering, families, `component.<namespace>`, `extends`, three shipped themes);
    - `:2333` (sources theme pins).

    Keep the strings `crates/oku-slides-session/src/help_tests.rs` quotes, or update that test.
  - `README.md` (crate map line on themes, if any).
  - The spec, if Step 1 finds a gap.
- Modify the rules:
  - every `AGENTS.md` and `ARCHITECTURE.md` that names deck-only themes, `component.card`-style paths, `ThemeRef` or `deck_themes`;
  - find them with `rg -n -i 'deck-only theme|deck_only|ThemeRef|deck_themes|component\.(card|table|title|chapter|toc|kicker|list|header|footer)' --glob '**/AGENTS.md' --glob '**/ARCHITECTURE.md'`.
- `ui/package.json`, `ui/bun.lock` (t-1094): update markdown-it and vite to versions without advisories; `bun audit` reports none.
- Oku:
  - the t-1008 body: lot 4 done, lot 5 left;
  - a new ticket under t-1008: "Éditer la liste extends d'un thème dans l'éditeur (lot 5)";
  - sub-task statuses.

**Interfaces:** none.

- [ ] **Step 1: Spec update check.** The spec changes listed under "Deviations" are committed with this plan (the plan commit). Verify with `git log -p -- docs/superpowers/specs/2026-10-18-packs-layouts-widgets-themes-design.md` that they landed; add anything missing.
- [ ] **Step 2: Consolidation pass (G10).**
  - `rg -n 'component\.(header|footer|list|kicker|card|table|title|title-slide|chapter|toc|end-slide|quote|columns)\b' crates typst ui/src docs --glob '!**/snapshots/**'` returns only the new qualified forms.
  - `rg -n 'ThemeRef|deck_themes|deck_only|theme_manifest|take_theme|PackScope::Deck' crates ui/src` returns only pack (not theme) uses of `PackScope::Deck` and `deck_only`.
  - `rg -n 'PackKind::Widgets' crates/oku-slides-*/src | rg -i theme` returns nothing.
  - No duplicate merge or linearization code outside `oku_slides_core::{tokens, theme_chain}`.
  - Then run `just agents-lint`.
- [ ] **Step 3: bun audit (t-1094).** From `ui/`, update the two packages with Bun, then run `bun audit` (clean), `bun test` and `bun x tsc --noEmit`.
- [ ] **Step 4: Full gate.**
  - Run `just ci` and `just e2e 4999`. All green.
  - The golden snapshots are unchanged: `git diff main -- crates/oku-slides-render/tests/snapshots/golden__reference_pages.snap crates/oku-slides-render/tests/snapshots/golden__reference_boxes.snap` is empty.
- [ ] **Step 5: Commit**: `docs(packs) : aide, README et règles du lot 4, bun audit, consolidation (t-1102, t-1094)`.
- [ ] **Step 6: Finish.**
  1. Run `git config --worktree --unset oku.review`.
  2. Merge `main` into the branch, then rerun the gate if `main` moved.
  3. Squash into `main` with the `BREAKING CHANGE:` footer of the Global Constraints.
  4. Push.
  5. Remove the worktree and the branch.
  6. Mark t-1102 done.
  7. Close t-1055 and t-1094.
  8. Comment on t-1053 and t-1057 (still open, the reason unchanged).
