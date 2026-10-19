# Typed packs, lot 2 — the built-in `base` widget pack — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The seven element widgets leave the Typst core for a built-in widget pack `base` (kinds `base/text` … `base/stack`), `canvas` is deleted, and nothing in the core library, the host or the UI chooses behavior by a widget name any more.

**Architecture:** The widget schema gains two declared attributes, `grow` (`false | true | "fit"`: how a flow block takes the height) and `typography` (the element's style styles its text; its box reports that typography), checked in Typst and in Rust. The core library (`data.typ`, `widgets.typ`, the generated `main.typ`) reads them instead of testing `kind`. The element widgets move to `typst/oku-slides/packs/base/` (a `widgets.json` manifest, one file per widget, `_commun.typ`), embedded once by `oku-slides-render`, which also offers the "built-in registry" (core + `base`) for every place that needs element widgets before a deck's own packs are resolved. The session lists `base` as a built-in library pack, Essentiel and Grilles require it, new decks and imports pin it, and the per-pack fallback never blames a built-in pack version. Rust code that writes a base kind takes it from one constants module (`oku_slides_core::kinds::base`); code that needs a chart or table param finds it by host type, in the UI through a new snapshot map `host_params`. The core keeps the six slide widgets with bare names until lot 3.

**Tech Stack:** Typst (embedded), Rust (Snafu, serde, speculoos, rstest, insta), Fluent (20 locales), React/TS (Bun), Playwright e2e.

**Spec:** `docs/superpowers/specs/2026-10-18-packs-layouts-widgets-themes-design.md` (§1, §2, §5, §6, §9 item 2). Lot 1 plan, for what exists: `docs/superpowers/plans/2026-10-18-packs-lot1-widget-packs.md`.

**Task:** parent t-1008; sub-tasks t-1035 … t-1043 (Task 1 … Task 9).

## Global Constraints

- Namespace of the pack: `base`. Its widgets: `text`, `card`, `image`, `shape`, `chart`, `table`, `stack` → kinds `base/text`, `base/card`, `base/image`, `base/shape`, `base/chart`, `base/table`, `base/stack`. `canvas` is deleted everywhere (Typst, fixtures, tests, docs); no replacement widget.
- Slide widgets (`title`, `toc`, `chapter`, `content`, `end`, `blank`) stay in the core with bare names (lot 3 moves them).
- `base` manifest: `id: "3b1f6c2e-5a4d-4e8b-9c71-0d2e4f6a8b13"`, `name: "Base"`, `namespace: "base"`, `requires: []`, `parent: null`, `created: "2026-10-18T00:00:00Z"`, library scope, `templates: []`.
- `grow`: `false` (default: its own height), `true` (a share of the remaining height), `"fit"` (its height if it fits a share, else a share). Declared: `image` and `chart` `true`, `table` `"fit"`, the others `false`. `typography: true` only on `text`. A slide widget declares neither (Typst assert and Rust `SchemaFault::SlideSizing`).
- Format: `DECK_VERSION` 6 → 7 (every deck JSON fixture, help deck and test literal follows; the "newer version" tests move from 7 to 8). `REPLICA_FORMAT`, `SOURCES_VERSION` and the worker `PROTOCOL_VERSION` are unchanged: the replica and the sources already refuse a deck of another version (`core/src/replica/read.rs:367-375`, `replica/mod.rs:392`). No migration.
- Golden render test: every page pixel-identical (`crates/oku-slides-render/tests/snapshots/golden__reference_pages.snap` unchanged). Any diff is a bug, never a re-record (`typst/oku-slides/AGENTS.md`).
- Component tokens (`component.card`, `component.table`, …) stay in `default-tokens.json` and the themes (lot 4 moves them; lot 4 must bump `DECK_VERSION` if an embedded `base` would stop finding them).
- A base kind written in Rust comes only from `oku_slides_core::kinds::base`; the namespace `base` only from `kinds::base::NAMESPACE` / `kinds::base::namespace()`. A qualified kind is otherwise built only by `qualify` and read only by `split_kind`.
- `oku-slides-import` never depends on `oku-slides-render` (`crates/oku-slides-import/AGENTS.md`): the session hands it the base pin and the built-in registry.
- No `unwrap`/`expect` outside tests; no silent fallback; Snafu errors with a backtrace per variant; every new or renamed Fluent key in all 20 locales (`crates/oku-slides-i18n/src/locales/*/`), same position, translated.
- Rust unit tests in sibling `x_tests.rs`, speculoos, names `unit_scenario_result`, rstest for tables; regressions in `tests/regressions/issue_NNNN_topic.rs`, declared in that crate's `tests/regressions/main.rs` (NNNN = the sub-task number). A regression is first seen failing for its stated reason.
- Every pub item documented, first sentence under 15 words; code comments in French; docs in English. Source files under 500 lines when possible, never above 1000. Each `ARCHITECTURE.md` and `AGENTS.md` change lands in the task that changes the code it describes; `ARCHITECTURE.md` stays ≤ 50 lines (core, render and session are at 50: cut as much as is added).
- Commits: `type(scope) : description (t-N)`, message in a scratchpad file passed with `git commit -F`; **no `Co-Authored-By` line** (user instruction). The squash commit carries a `BREAKING CHANGE:` footer naming deck version 7, the refusal of version-6 decks and of unsaved version-6 tabs on resume, and that lot-1 library versions of Essentiel or Grilles (unreleased builds only) keep bare kinds.
- Work deferred to lots 3–5 is tracked under t-1008 (its body lists the lots); a code comment that defers names `t-1008`.
- Runs outside `just e2e` set `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS` to a fresh temp dir.

## Deviations from the spec (approved with this plan; spec §2, §8 and §9 updated in the same commit)

1. **No `citation` widget.** No widget called `citation` exists: Grilles ships templates only, and `citation` is an Essentiel *template* (a `content` slide, layout `quote`, one text block). `base` holds seven widgets; nothing is invented. The lot 1 plan's "Grilles widget `grilles/citation`" was wrong too.
2. **A second schema attribute, `typography`.** `data.typ:165` reads `kind == "text"` to probe the box typography; `grow` cannot express it.
3. **`(ctx.load)` takes `flow:`.** `world.rs:108` tests `kind == "image"` to size a flow image at its proportions. The image widget now asks: `(ctx.load)(asset:, fit:, flow: not ctx.framed)`. `ctx.framed` is `"frame" in e` (`data.typ:146`), so rendering is unchanged.
4. **Element box height follows `grow`.** `data.typ:160` measured `chart` and `table` boxes in the available height; it now does so for every widget with `grow != false`, which adds `image`. A flow image taller than its row reported a box taller than the drawn image. Pixels are unchanged; `ElementBox.h` of such an image becomes the drawn height (regression `issue_1036`).
5. **The `card()` library function stays in `slides.typ`.** It is public library API (`lib.typ`, `examples/pro.typ`); `base/card` calls it. Only widget registrations leave the core.
6. **`DECK_VERSION` 7, nothing else bumped.** A version-6 deck (bare element kinds) is refused instead of rendering "unknown element" boxes. A deck JSON without `"version"` still reads as current (existing rule, `model.rs:481`).
7. **The receipt needs no change.** `session/src/agent/receipt.rs:62-64` names an A2UI component `card`, never an element kind.
8. **Panels by host type use a snapshot map.** The UI receives no param types today. `Snapshot.host_params` (`kind → host type → param name`, element widgets of the deck registry with a host param) lets the UI pick `ChartPanel` / `TablePanel` by host type and write to the declared param. `show_as` still creates `base/chart` / `base/table`.
9. **The add grid is unchanged until lot 5.** The five native entries keep their starters (renamed `ADD_NAMES`: `text`, `card`, `shape`, `chart`, `table`, mapped to `base/*` kinds); `packOnlyWidgets` drops **every** `base/*` widget from the pack list, so `base/image` and `base/stack` do not appear as new cards. The starters keep writing `params.option` / `params.data` of `base/chart` / `base/table` (they are base starters, not a dispatch). The "no text on a blank slide" rule stays a UI rule on the native `text` until lot 5.
10. **Base widgets are openable like any pack widget.** `pack_widgets` includes `base/*`, so the stage shows the "open widget" tag on base elements (its label uses the localized kind name); editing one makes a new `base` version, as for any built-in pack in lot 1.
11. **Localized widget and param names exist only for core and `base` kinds.** `session-widget-kind-<kind>` / `session-param-<kind>-<param>` are looked up for a bare kind or a kind in `kinds::base::ALL`, as `…-base-card…`; `session-widget-kind-base-chart` and `-base-table` are added (they had no key). A pack widget of another namespace uses its schema label, else its kind (never a borrowed base label). This localizes one pack inside the session until packs carry their own localized labels.
12. **The registry fixture stays one file.** Spec §8 asks for one snapshot per built-in pack. `fixtures/widgets/base-registry.json` becomes `{"base": [the six slide schemas], "packs": [{"namespace": "base", …}]}` (core + pack `base`), still enforced against the live registry; lot 3 splits it when Essentiel gets widgets. The probe key `"base"` keeps meaning "core widgets" (renamed with lot 3).
13. **Shipped `base` versions are trusted, and the current one is resolved in-process.** With every element in `base`, lot 1's control render without packs no longer reproduces a content-caused timeout (elements become cheap "unknown" boxes), so bisection would disable `base` and, through `requires`, Essentiel. A pack is *trusted* when its namespace is `base` and its version is one the app has shipped, listed in `typst/oku-slides/packs/base/versions.txt` (one hex per line; a test requires the current version to be listed, so every change of a `base` file appends its new version): an older built-in `base` embedded in a deck stays trusted after an update; an edited `base` (deviation 10) or a received pack claiming `base` is not. One predicate, `oku_slides_render::is_trusted_pack`, serves the worker and the session. A trusted pack is never disabled — neither by a probe failure nor by a bounded render failure: the error is reported as the deck's own. The control render keeps the trusted packs; only the others are bisected (even a single one: lot 1 follow-up "M5"). The worker's `blame` marks a slot's pack only when it is the slot's only pack and is untrusted: a slot holding a trusted pack blames nobody, since the deck's own `base` content may be the cause (amended after review: blaming the only untrusted pack beside `base` disabled an innocent user pack when heavy `base` content timed out). The session decides instead: its control render with the trusted packs, then its bisection, whose single-candidate trial beside them marks the culprit through `RenderWorker::mark_faulty` (which never marks a trusted pack). A failure outside the deck render (template previews, variants) is bisected on the deck that failed. A pack of another namespace is probed beside the built-in `base` (`builtin_probe`), so its slots may accept `base/*` kinds. The pin of the current built-in version is resolved in-process (embedded, trusted code, like the core registry today), so a deck on built-ins alone never starts the worker (t-342 kept). This amends the settled rule of `crates/oku-slides-render-worker/AGENTS.md` ("the render client only flags faulty pack fingerprints"): the client never flags a shipped `base` version and blames nothing in a slot holding one.
14. **Open never repairs a deck's pins.** `adopt_default_packs` still touches only a deck without pins (now pinning `essentiel` and, through its `requires`, `base`). Producers pin what they emit, before probing: hi-fi import pins the built-in `base` and the `deck` pack (which requires it), retrofit adds `base` to its target's closure, help pins the built-in `base` and `essentiel`, templates mode keeps `base` in its working deck and saves `requires` covering the namespaces its templates use. A received deck is never completed from the library.
15. **Built-in namespaces are already reserved.** `PackLibrary::add` (`owner_of`) and `library_namespace` read `entries()`, built-ins first (`library.rs:514-534`, `namespace_slug.rs:62-64`), so adding `base` to the built-ins makes another id claiming `base` fail with `NamespaceTaken` and new slugs skip it. No new code: two tests guard it. A lot-1 user library pack already named `base` stays readable by id and cannot get a new version; no migration.
16. **`base` cannot be removed from a deck.** `Op::RemovePack { base }` is refused (`OpErrorKind::BaseRequired`, `op-base-required`): the native add buttons, the agent and paste all write `base/*` kinds, so every deck keeps `base` once pinned.
17. **Template-set pickers list only packs with templates.** `Request::TemplateSets` leaves out packs without templates (`base` today), so the slide picker, the import dialog and the atelier menu do not offer it; a base widget is still opened from its element (deviation 10).
18. **Intermediate commits.** Tasks 2–7 go bottom-up one layer at a time; each task's gate is the crates it touches (Tasks 2 and 3 exclude the render tests that run the importer, `tests/import_*.rs`, fixed in Task 4). The workspace is green again from Task 7; `just ci` and `just e2e` run in Task 9. The branch is squash-merged.

---

### Task 1: Widget schema `grow` and `typography`, host params, base kind constants

Additive: the workspace stays green.

**Files:**
- Modify: `crates/oku-slides-core/src/widget.rs:18-60,137-160,246-262` (`Grow`, schema fields, `HostType` order, `host_params`, slide check), `src/widget_tests.rs`, `src/error/mod.rs:524-590` (`SchemaFault::SlideSizing`), `src/namespace.rs` (+ `namespace_tests.rs`), `src/lib.rs` (module, re-exports, crate doc list)
- Create: `crates/oku-slides-core/src/kinds.rs`, `src/kinds_tests.rs`
- Modify (struct literals gain `grow`, `typography`): `crates/oku-slides-import/src/hifi/widget.rs:216`, `crates/oku-slides-core/src/ops/list_path_tests.rs:9`, `crates/oku-slides-session/src/widget_preview_tests.rs:26`, `crates/oku-slides-session/src/import/hifi_tests.rs:7` (check `rg -n 'WidgetSchema \{' crates`; struct-update forms need nothing)
- Modify i18n: `crates/oku-slides-i18n/src/locales/*/core.ftl` (`widget-fault-slide-sizing`, after `widget-fault-list-default-below-min`)
- Modify: `crates/oku-slides-core/AGENTS.md` (rule amended), `crates/oku-slides-core/ARCHITECTURE.md` (`kinds.rs`, `Grow`; cut to stay ≤ 50 lines)

**Interfaces:**
- Produces:
  - `pub enum Grow { Fixed, Fill, Fit }` (`Default` = `Fixed`; serde `false` / `true` / `"fit"`; `is_fixed()`), re-exported.
  - `WidgetSchema.grow: Grow`, `WidgetSchema.typography: bool` (both `#[serde(default)]`, omitted at their default on output).
  - `HostType` derives `PartialOrd, Ord`.
  - `WidgetRegistry::host_params(&self) -> BTreeMap<String, BTreeMap<HostType, String>>`: for each element widget with a host param, host type → `host_param(h)` name.
  - `SchemaFault::SlideSizing` (`widget-fault-slide-sizing`): a slide widget declaring `grow` or `typography`.
  - `pub mod kinds` → `pub mod base { NAMESPACE, TEXT, CARD, IMAGE, SHAPE, CHART, TABLE, STACK, ALL: [&str; 7], fn namespace() -> Namespace }`.

- [ ] **Step 1: Write the failing tests**

`widget_tests.rs` (add `use rstest::rstest;` if absent):

```rust
#[rstest]
#[case(json!(false), Grow::Fixed)]
#[case(json!(true), Grow::Fill)]
#[case(json!("fit"), Grow::Fit)]
fn grow_from_json_reads_the_three_forms(#[case] v: Value, #[case] expected: Grow) {
    assert_that!(serde_json::from_value::<Grow>(v).unwrap()).is_equal_to(expected);
}

#[rstest]
#[case(json!("fill"))]
#[case(json!(1))]
#[case(json!(null))]
fn grow_from_json_other_value_is_refused(#[case] v: Value) {
    assert_that!(serde_json::from_value::<Grow>(v)).is_err();
}

#[test]
fn widget_schema_without_grow_is_fixed_and_without_typography() {
    let w: WidgetSchema = serde_json::from_value(json!({"name": "x", "category": "element", "params": []})).unwrap();
    assert_that!((w.grow, w.typography)).is_equal_to((Grow::Fixed, false));
}

#[test]
fn widget_schema_fit_and_typography_round_trip() {
    // `label` s'écrit toujours (`null` absent), `description` jamais quand elle manque.
    let src = json!({"name": "x", "label": null, "category": "element", "grow": "fit", "typography": true, "params": []});
    let w: WidgetSchema = serde_json::from_value(src.clone()).unwrap();
    assert_that!(serde_json::to_value(&w).unwrap()).is_equal_to(src);
}

#[test]
fn widget_schema_defaults_are_not_serialized() {
    let w: WidgetSchema = serde_json::from_value(json!({"name": "x", "category": "element", "params": []})).unwrap();
    let out = serde_json::to_value(&w).unwrap();
    assert_that!((out.get("grow").is_none(), out.get("typography").is_none())).is_equal_to((true, true));
}

#[rstest]
#[case(json!({"grow": true}))]
#[case(json!({"typography": true}))]
fn registry_from_probe_slide_declaring_sizing_is_refused(#[case] extra: Value) {
    let mut slide = json!({"name": "s", "category": "slide", "params": []});
    slide.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    let err = WidgetRegistry::from_probe(json!({"base": [slide], "packs": []})).unwrap_err();
    assert_that!(err.kind()).is_equal_to(WidgetErrorKind::BadSchema(SchemaFault::SlideSizing));
}

#[test]
fn host_params_maps_each_host_of_element_widgets_only() {
    let probe = json!({"base": [
        {"name": "s", "category": "slide", "params": [{"name": "o", "type": "host", "host": "chart"}]},
    ], "packs": [{"namespace": "acme", "widgets": [
        {"name": "plot", "category": "element", "params": [
            {"name": "series", "type": "host", "host": "chart"},
            {"name": "other", "type": "host", "host": "chart"},
            {"name": "grid", "type": "host", "host": "table"},
        ]},
        {"name": "note", "category": "element", "params": [{"name": "body", "type": "rich"}]},
    ]}]});
    let hosts = WidgetRegistry::from_probe(probe).unwrap().host_params();
    let plot = BTreeMap::from([(HostType::Chart, "series".to_owned()), (HostType::Table, "grid".to_owned())]);
    assert_that!(hosts).is_equal_to(BTreeMap::from([("acme/plot".to_owned(), plot)]));
}
```


`kinds_tests.rs`:

```rust
use speculoos::prelude::*;

use super::base;
use crate::{Namespace, qualify, split_kind};

#[test]
fn base_kinds_are_qualified_by_the_base_namespace() {
    let names = ["text", "card", "image", "shape", "chart", "table", "stack"];
    let built: Vec<String> = names.iter().map(|n| qualify(&base::namespace(), n)).collect();
    assert_that!(base::ALL.map(str::to_owned).to_vec()).is_equal_to(built);
}

#[test]
fn base_kinds_split_into_the_base_namespace() {
    let namespaces: Vec<Option<&str>> = base::ALL.iter().map(|k| split_kind(k).map(|(ns, _)| ns)).collect();
    assert_that!(namespaces).is_equal_to(vec![Some(base::NAMESPACE); 7]);
}

#[test]
fn base_namespace_parses_like_its_constant() {
    assert_that!(Namespace::parse(base::NAMESPACE)).is_equal_to(Some(base::namespace()));
}
```

- [ ] **Step 2: Run, expect FAIL** — `cargo nextest run -p oku-slides-core grow widget_schema registry_from_probe_slide host_params base_` → does not compile.

- [ ] **Step 3: Implement**

`widget.rs`, after `HostType` (whose derives gain `PartialOrd, Ord`):

```rust
/// Comment un bloc du flux d'une slide prend la hauteur (attribut `grow` du schéma).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(from = "GrowRepr", into = "GrowRepr")]
pub enum Grow {
    /// Sa hauteur propre (`false`).
    #[default]
    Fixed,
    /// Une part de la hauteur qui reste (`true`).
    Fill,
    /// Sa hauteur si elle tient dans une part, sinon une part (`"fit"`).
    Fit,
}

impl Grow {
    /// Vrai pour la valeur par défaut, `false`.
    pub fn is_fixed(&self) -> bool {
        *self == Grow::Fixed
    }
}

/// Forme JSON de [`Grow`] : un booléen ou la chaîne `"fit"` ; toute autre valeur est refusée.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum GrowRepr {
    Flag(bool),
    Word(GrowWord),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum GrowWord {
    Fit,
}

impl From<GrowRepr> for Grow {
    fn from(r: GrowRepr) -> Self {
        match r {
            GrowRepr::Flag(false) => Grow::Fixed,
            GrowRepr::Flag(true) => Grow::Fill,
            GrowRepr::Word(GrowWord::Fit) => Grow::Fit,
        }
    }
}

impl From<Grow> for GrowRepr {
    fn from(g: Grow) -> Self {
        match g {
            Grow::Fixed => GrowRepr::Flag(false),
            Grow::Fill => GrowRepr::Flag(true),
            Grow::Fit => GrowRepr::Word(GrowWord::Fit),
        }
    }
}
```

`WidgetSchema`, after `category`:

```rust
    /// Hauteur d'un bloc du flux : la sienne, une part, ou la sienne si elle tient.
    #[serde(default, skip_serializing_if = "Grow::is_fixed")]
    pub grow: Grow,
    /// Le style de l'élément met en forme son texte ; sa boîte en mesure la typographie.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub typography: bool,
```

`WidgetRegistry`:

```rust
    /// Pour chaque widget d'élément à paramètre d'hôte : hôte → premier paramètre de cet hôte.
    pub fn host_params(&self) -> BTreeMap<String, BTreeMap<HostType, String>> {
        let mut out = BTreeMap::new();
        for (kind, w) in self.widgets.iter().filter(|(_, w)| w.category == Category::Element) {
            let mut hosts = BTreeMap::new();
            // Le premier paramètre d'un hôte gagne, comme dans `host_param`.
            for p in &w.params {
                if let Some(h) = p.ty.host() {
                    hosts.entry(h).or_insert_with(|| p.name.clone());
                }
            }
            if !hosts.is_empty() {
                out.insert(kind.clone(), hosts);
            }
        }
        out
    }
```

`check_widget`, first check:

```rust
    if w.category == Category::Slide && (!w.grow.is_fixed() || w.typography) {
        return Err(fault(w, SchemaFault::SlideSizing));
    }
```

`error/mod.rs`: `SchemaFault::SlideSizing` (doc "Une slide déclare `grow` ou `typography`, réservés aux éléments."), display `"slide declares grow or typography"`, id `"widget-fault-slide-sizing"`. Fluent (en): `widget-fault-slide-sizing = Widget { $widget } is a slide: only element widgets declare grow or typography`, then the 19 other locales, translated.

`kinds.rs`:

```rust
//! Les kinds du pack de widgets intégré `base`, que le code Rust écrit ou reconnaît.
//!
//! Seul endroit où un kind de `base` s'écrit en clair ; `kinds_tests`
//! vérifie que chacun vaut `qualify(base, nom)`.

/// Le pack de widgets intégré `base` : texte, carte, image, forme, graphique, tableau, pile.
pub mod base {
    use crate::Namespace;

    /// L'espace de noms du pack.
    pub const NAMESPACE: &str = "base";
    /// Un texte riche.
    pub const TEXT: &str = "base/text";
    /// Une carte : surtitre, titre, corps.
    pub const CARD: &str = "base/card";
    /// Une image d'un asset du deck.
    pub const IMAGE: &str = "base/image";
    /// Une forme : rectangle, ellipse, ligne ou chemin.
    pub const SHAPE: &str = "base/shape";
    /// Un graphique dessiné par l'hôte.
    pub const CHART: &str = "base/chart";
    /// Un tableau de données.
    pub const TABLE: &str = "base/table";
    /// Une pile d'éléments en lignes, colonnes ou grille.
    pub const STACK: &str = "base/stack";
    /// Tous les widgets du pack, dans l'ordre de déclaration.
    pub const ALL: [&str; 7] = [TEXT, CARD, IMAGE, SHAPE, CHART, TABLE, STACK];

    /// L'espace de noms `base`.
    pub fn namespace() -> Namespace {
        Namespace::builtin(NAMESPACE)
    }
}

#[cfg(test)]
#[path = "kinds_tests.rs"]
mod kinds_tests;
```

`namespace.rs`, beside `deck()`:

```rust
    /// Un espace de noms intégré, écrit dans le code ; `kinds_tests` vérifie sa syntaxe.
    pub(crate) fn builtin(s: &'static str) -> Self {
        Self(s.to_owned())
    }
```

`lib.rs`: `pub mod kinds;`, `pub use widget::Grow;`, and add `kinds`, `Grow`, `WidgetRegistry::host_params` to the crate doc's list of key items.

`crates/oku-slides-core/AGENTS.md`: replace "Build a qualified kind only with qualify and read it only with split_kind; a bare kind is a core widget." by "Build a qualified kind only with qualify or take a base kind from kinds::base; read one only with split_kind. (plan:2026-10-18-packs-lot1-widget-packs, plan:2026-10-18-packs-lot2-base)".

- [ ] **Step 4: Run, expect PASS** — `cargo nextest run --workspace --exclude oku-slides-app`, `cargo nextest run -p oku-slides-i18n` and `just check`.

- [ ] **Step 5: Commit** — `feat(core) : attributs grow et typography du schéma, paramètres d'hôte par widget, kinds du pack base (t-1035)`.

---

### Task 2: Typst core without element widgets, the `base` pack, name-free sizing

The render and render-worker crates are green at the end, except `crates/oku-slides-render/tests/import_*.rs` (the importer still emits bare kinds until Task 4); core and the crates above break on bare kinds until Tasks 3–7 (deviation 18).

**Files:**
- Create: `typst/oku-slides/packs/base/widgets.json`, `typst/oku-slides/packs/base/widgets/{_commun,text,card,image,shape,chart,table,stack}.typ`
- Modify: `typst/oku-slides/src/widget.typ:1-42`, `src/widgets.typ` (slide widgets only, `canvas` deleted), `src/data.typ:1-205`, `lib.typ`, `examples/{from-json.typ,deck.json}`, `README.md` (registry, `base` pack, `grow`, `typography`, slot entries, `load-image` `flow:`, compile root, "a bare kind is a core widget" now means a slide kind, `packs/base` needs the app's virtual root), `typst/oku-slides/AGENTS.md`
- Modify: `crates/oku-slides-render/src/world.rs:65-125` (`flow:`), `src/registry.rs:229-256,308-315` (`core_registry`, `builtin_registry`), `src/lib.rs` (`Renderer::with_builtins`, re-exports, crate doc list), `src/error.rs` (`RegistryInner::BasePack`; `RenderInner::BaseRegistry` display "the built-in widget registry could not be read"), `src/wire.rs:55-66` (`BasePack` → the existing `WireError::Setup { message }`: no protocol change), `src/world_tests.rs:55-69`, `src/registry_tests.rs`, `src/{tables_tests,charts_tests,palette_tests}.rs` (bare kinds), `ARCHITECTURE.md`
- Create: `crates/oku-slides-render/src/base_pack.rs`, `src/base_pack_tests.rs`, `typst/oku-slides/packs/base/versions.txt` (the shipped `base` versions, one hex per line: the current one), `tests/regressions/issue_1036_image_box_follows_its_row.rs` (declared in `tests/regressions/main.rs`)
- Modify: `justfile:313-314` (`just typst` compiles with `--root typst`: pack files import `/oku-slides/lib.typ`), root `README.md:64,184` (element kinds, "falls back to the base widgets")
- Modify render tests: `tests/common/mod.rs` (`renderer()`, `compile_with_packs`, `DECK_VERSION`), `tests/golden.rs`, `tests/golden/widgets.json`, `tests/widgets.rs`, `tests/snapshots/widgets__base_registry.snap` (regenerated, reviewed), and every test calling `Renderer::new` on a deck with elements: `tests/{render,fields,tables,text_indent,png,anchor_path,widgets}.rs`, `tests/regressions/issue_{0001,0002,0003,0176,0325,0544,0554,0609,0645,0841}_*.rs`
- Modify: `crates/oku-slides-core/src/model.rs:445-450` (`Element::text` uses `kinds::base::TEXT`)
- Modify fixtures: `fixtures/widgets/base-registry.json` (regenerated), `fixtures/roadmap.json` (element kinds qualified), `fixtures/AGENTS.md` (rule)
- Modify i18n: `*/document.ftl` (`registry-base-pack-broken`, after `registry-bad-pack-id`)
- Modify render-worker: `src/pack_probe.rs:44,101,116` (`core_probe`), and the tests that render elements or use `base_registry`: `src/{client_tests,client_fonts_tests,pack_set_tests,serve_tests}.rs`, `tests/process.rs` (`core_registry` / `builtin_registry` and a base set)

**Interfaces:**
- Consumes: `Grow`, `typography`, `SchemaFault::SlideSizing`, `kinds::base` (Task 1).
- Produces:
  - Typst: `widget(name, label:, description:, category:, grow: false, typography: false, params:, render)`; schema keys `grow`, `typography`; flow slot entries `(kind, id, grow, body)`; `load-image(e, asset:, fit:, chart:, flow: false)`.
  - render: `pub fn base_pack_files() -> Result<PackFiles, RegistryError>`; `pub fn base_pack_widgets() -> Result<PackWidgets, RegistryError>`; `pub fn core_registry(fonts) -> Result<Arc<WidgetRegistry>, RegistryError>` (renamed from `base_registry`: the six slide widgets); `pub fn builtin_registry(fonts) -> Result<Arc<WidgetRegistry>, RegistryError>` (core + built-in `base`, memoized per process); `Renderer::with_builtins(fonts) -> Result<Renderer, RenderError>` (`new` + `set_widgets` of the built-in set); `RegistryErrorKind::BasePack`; `pub fn is_trusted_pack(p: &PackWidgets) -> bool` (namespace `base` and id — the version hex — listed in `versions.txt`; deviation 13), used by Tasks 5 and 6.
  - `fixtures/widgets/base-registry.json` = core + pack `base` (deviation 12).

- [ ] **Step 1: Write the failing regression first, against today's code**

`tests/regressions/issue_1036_image_box_follows_its_row.rs`, written with today's bare `image` kind and `Renderer::new`, run, and **seen failing on the box height** before any other change:

```rust
//! Une image du flux plus haute que sa rangée annonçait une boîte plus haute que l'image dessinée.

use std::collections::BTreeMap;

use oku_slides_core::Deck;
use speculoos::prelude::*;

use crate::common::{IMAGE_KIND, TEXT_KIND, renderer};

/// Une image SVG de 100 × 2000 : bien plus haute que la place qui lui reste.
const TALL_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="2000"><rect width="100" height="2000"/></svg>"#;

#[test]
fn issue_1036_flow_image_box_is_the_drawn_height() {
    let deck = Deck::from_json(&serde_json::json!({
        "version": oku_slides_core::DECK_VERSION, "params": {"title": "T"},
        "slides": [{"id": "s", "kind": "content", "params": {"title": "Titre", "blocks": [
            {"id": "t", "kind": TEXT_KIND, "params": {"body": [{"runs": [{"t": "Avant"}]}]}},
            {"id": "i", "kind": IMAGE_KIND, "params": {"asset": "sha256:1036", "fit": "contain"}},
        ]}}]
    }).to_string()).unwrap();
    let assets = BTreeMap::from([("sha256:1036".to_owned(), TALL_SVG.to_vec())]);
    let compiled = renderer().render(&deck, &assets).unwrap();
    let layout = compiled.layout();
    let zone = layout.zone_on(1).unwrap();
    let image = layout.boxes_on(1).iter().find(|b| b.id == "i").unwrap();
    assert_that!(image.h).is_greater_than(0.0);
    assert_that!(image.y + image.h).is_less_than_or_equal_to(zone.y + zone.h + 0.5);
}
```

Written first in its today form: `const IMAGE_KIND: &str = "image"; const TEXT_KIND: &str = "text";` and `fn renderer() -> Renderer { Renderer::new(Fonts::default()).unwrap() }` added to `tests/common/mod.rs`, `"version": 6`; run it and record the failing value (the box is measured unbounded at the image's proportions, `world.rs:108-112`, `data.typ:160-161`). Step 5 switches those three helpers to `kinds::base::IMAGE` / `TEXT`, `Renderer::with_builtins` and `DECK_VERSION`, without touching the test. The image takes the `1fr` row, so also assert `(zone.y + zone.h) - (image.y + image.h) < 1.0`. If the content slide has no `ZoneBox` when it has blocks, compare with the body bottom (`page.height - margin-bottom`, from the theme tokens).

- [ ] **Step 2: Write the other failing tests**

`base_pack_tests.rs`:

```rust
use std::collections::BTreeSet;

use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::{Fonts, PackSet, load_registry};
use oku_slides_core::{Category, Grow, Namespace, PackManifest, WidgetRegistry, kinds::base};

fn base_registry_of_pack() -> WidgetRegistry {
    load_registry(&Fonts::default(), &PackSet::new(vec![base_pack_widgets().unwrap()]).unwrap()).unwrap()
}

#[test]
fn base_pack_files_read_as_the_base_manifest() {
    let m = PackManifest::from_files(&base_pack_files().unwrap()).unwrap();
    assert_that!((m.namespace, m.requires.is_empty())).is_equal_to((base::namespace(), true));
}

#[test]
fn base_pack_registers_exactly_the_seven_qualified_widgets() {
    let r = base_registry_of_pack();
    let names: BTreeSet<&str> = r.iter().filter(|w| w.name.starts_with("base/")).map(|w| w.name.as_str()).collect();
    assert_that!(names).is_equal_to(base::ALL.into_iter().collect::<BTreeSet<_>>());
}

#[test]
fn core_registry_holds_the_six_slide_widgets_only() {
    let r = core_registry(&Fonts::default()).unwrap();
    let names: BTreeSet<&str> = r.iter().map(|w| w.name.as_str()).collect();
    let slides = BTreeSet::from(["title", "toc", "chapter", "content", "end", "blank"]);
    assert_that!((names, r.iter().all(|w| w.category == Category::Slide))).is_equal_to((slides, true));
}

#[test]
fn base_widgets_declare_their_grow() {
    let r = base_registry_of_pack();
    let grows: Vec<(&str, Grow)> = base::ALL.iter().map(|k| (*k, r.get(k).unwrap().grow)).collect();
    let expected = vec![
        (base::TEXT, Grow::Fixed), (base::CARD, Grow::Fixed), (base::IMAGE, Grow::Fill), (base::SHAPE, Grow::Fixed),
        (base::CHART, Grow::Fill), (base::TABLE, Grow::Fit), (base::STACK, Grow::Fixed),
    ];
    assert_that!(grows).is_equal_to(expected);
}

#[test]
fn only_base_text_declares_typography() {
    let r = base_registry_of_pack();
    let typo: Vec<&str> = base::ALL.into_iter().filter(|k| r.get(k).is_some_and(|w| w.typography)).collect();
    assert_that!(typo).is_equal_to(vec![base::TEXT]);
}

#[test]
fn base_pack_embeds_every_file_of_its_directory() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../typst/oku-slides/packs/base/widgets");
    let on_disk: BTreeSet<String> =
        std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    let embedded: BTreeSet<String> = BASE_WIDGETS.iter().map(|(n, _)| format!("{n}.typ")).collect();
    assert_that!(embedded).is_equal_to(on_disk);
}

#[test]
fn builtin_registry_is_core_plus_base() {
    let r = builtin_registry(&Fonts::default()).unwrap();
    assert_that!((r.get("content").is_some(), r.get(base::TABLE).is_some())).is_equal_to((true, true));
}

#[test]
fn base_versions_list_the_current_base_version() {
    let current = base_pack_files().unwrap().version().hex().to_owned();
    assert_that!(shipped_base_versions().any(|v| v == current)).is_true();
}

#[test]
fn is_trusted_pack_of_the_current_base_is_true() {
    assert_that!(is_trusted_pack(&base_pack_widgets().unwrap())).is_true();
}

#[test]
fn is_trusted_pack_of_an_edited_base_is_false() {
    let mut files = base_pack_files().unwrap().widgets().clone();
    files.insert(WidgetFile::parse("text").unwrap(), "// modifié".to_owned());
    let edited = PackFiles::new(base_pack_files().unwrap().manifest().clone(), files.clone());
    let pack = PackWidgets::new(edited.version().hex(), base::namespace(), files).unwrap();
    assert_that!(is_trusted_pack(&pack)).is_false();
}

#[test]
fn is_trusted_pack_of_another_namespace_with_the_base_id_is_false() {
    let base = base_pack_widgets().unwrap();
    let other = PackWidgets::new(base.id(), Namespace::parse("acme").unwrap(), base.files().clone()).unwrap();
    assert_that!(is_trusted_pack(&other)).is_false();
}
```

`tests/widgets.rs` — Typst-level `grow` / `typography`, with a test pack `test` (`common::pack`) beside `base` (`common::compile_with_packs(&[pack], blocks)` renders a `content` slide with the base set plus `pack`):

```rust
/// Un widget de test qui remplit sa hauteur : un rectangle de 100 %.
const TALL: &str = r#"#import "/oku-slides/lib.typ": widget
#let widget = widget("tall", grow: true, (args, ctx) => rect(width: 100%, height: 100%))"#;
/// Le même contenu de 40 pt, avec ou sans `grow`.
const FLAT: &str = r#"#import "/oku-slides/lib.typ": widget
#let widget = widget("flat", (args, ctx) => block(width: 100%, height: 40pt))"#;
/// Un widget qui veut `n` pt mais se réduit à la place reçue (comme `base/table`), en `"fit"`.
const FIT: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("fit", grow: "fit", params: (n: p.number()),
  (args, ctx) => layout(sz => block(width: 100%, height: calc.min(args.n * 1pt, sz.height))))"#;
/// Un libellé dont l'élément porte la typographie de son style.
const LABEL: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p, _text-args
#let widget = widget("label", typography: true, params: (x: p.plain()), (args, ctx) => {
  set text(.._text-args(ctx.tokens, ctx.style))
  field(ctx, "x", args.x)
})"#;
const LABEL_PLAIN: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("plainlabel", params: (x: p.plain()), (args, ctx) => field(ctx, "x", args.x))"#;

fn text_block(id: &str) -> serde_json::Value {
    json!({"id": id, "kind": base::TEXT, "params": {"body": [{"runs": [{"t": "Avant"}]}]}})
}

fn bottom_gap(layout: &Layout, id: &str) -> f64 {
    let zone_bottom = layout.zone_on(1).map(|z| z.y + z.h).unwrap();
    let b = layout.boxes_on(1).iter().find(|b| b.id == id).unwrap();
    zone_bottom - (b.y + b.h)
}

#[test]
fn widget_declaring_grow_fill_takes_the_remaining_height() {
    let pack = common::pack("tall", &[("tall", TALL)]);
    let layout = common::render_with_packs(&[pack], json!([text_block("t"), {"id": "g", "kind": "test/tall"}]));
    assert_that!(bottom_gap(&layout, "g")).is_less_than(1.0);
}

#[test]
fn widget_without_grow_keeps_its_natural_height() {
    let pack = common::pack("flat", &[("flat", FLAT)]);
    let layout = common::render_with_packs(&[pack], json!([text_block("t"), {"id": "g", "kind": "test/flat"}]));
    let g = layout.boxes_on(1).iter().find(|b| b.id == "g").unwrap();
    assert_that!(((g.h - 40.0).abs() < 1.0, bottom_gap(&layout, "g") > 50.0)).is_equal_to((true, true));
}

/// `"fit"` qui tient garde sa hauteur : le bloc suivant suit juste dessous.
#[test]
fn widget_declaring_grow_fit_that_fits_keeps_its_height() {
    let pack = common::pack("fit", &[("fit", FIT)]);
    let blocks = json!([text_block("t"), {"id": "f", "kind": "test/fit", "params": {"n": 40}}, text_block("u")]);
    let layout = common::render_with_packs(&[pack], blocks);
    let (f, u) = (common::box_of(&layout, "f"), common::box_of(&layout, "u"));
    assert_that!(((f.h - 40.0).abs() < 1.0, u.y - (f.y + f.h) < 30.0, bottom_gap(&layout, "u") > 50.0))
        .is_equal_to((true, true, true));
}

/// `"fit"` trop haut prend une part : il se réduit, et le bloc suivant reste dans la zone.
#[test]
fn widget_declaring_grow_fit_too_tall_takes_a_share() {
    let pack = common::pack("fit", &[("fit", FIT)]);
    let blocks = json!([text_block("t"), {"id": "f", "kind": "test/fit", "params": {"n": 2000}}, text_block("u")]);
    let layout = common::render_with_packs(&[pack], blocks);
    let f = common::box_of(&layout, "f");
    assert_that!((f.h < 1000.0, bottom_gap(&layout, "u") > -0.5)).is_equal_to((true, true));
}

#[rstest]
#[case("test/label", true)]
#[case("test/plainlabel", false)]
fn element_box_reports_the_style_size_only_with_typography(#[case] kind: &str, #[case] styled: bool) {
    // Un seul pack : deux packs du même espace de noms seraient refusés (`DuplicatePack`).
    let packs = [common::pack("label", &[("label", LABEL), ("plainlabel", LABEL_PLAIN)])];
    let blocks = json!([{"id": "l", "kind": kind, "style": {"size": "title"}, "params": {"x": "Libellé"}}]);
    let layout = common::render_with_packs(&packs, blocks);
    let size = layout.boxes_on(1).iter().find(|b| b.id == "l").unwrap().style.size;
    let title = common::token_size("title");
    assert_that!((size - title).abs() < 0.5).is_equal_to(styled);
}

#[rstest]
#[case(r#"widget("s", category: "slide", grow: true, (a, c) => [])"#)]
#[case(r#"widget("s", category: "slide", typography: true, (a, c) => [])"#)]
#[case(r#"widget("s", grow: "fill", (a, c) => [])"#)]
#[case(r#"widget("s", grow: 1, (a, c) => [])"#)]
#[case(r#"widget("s", typography: "yes", (a, c) => [])"#)]
fn widget_with_invalid_sizing_is_refused_by_the_probe(#[case] call: &str) {
    let src = format!("#import \"/oku-slides/lib.typ\": widget\n#let widget = {call}\n");
    let probe = oku_slides_render::probe_file(&Fonts::default(), &WidgetFile::parse("s").unwrap(), &src, None);
    assert_that!(probe.is_valid()).is_false();
}
```

`common::token_size(name)` reads `size.<name>` from `typst/oku-slides/src/default-tokens.json` in pt (add it beside `theme_tokens`); `common::box_of(layout, id) -> &ElementBox` finds a box on page 1 (panics with the id if absent). Replace the bare-name tests at `tests/widgets.rs:24-30,117-145` by qualified ones (`r.get(base::CARD)`, element names `base::ALL`, slide names unchanged) and delete the `canvas` test (`:332`, `framed("canvas", …)`).

A framed image keeps its frame (deviation 3, not covered by golden), in `tests/widgets.rs`:

```rust
/// Une image large et pleine (600 × 100, noire) : dans le flux, à ses proportions, elle ferait 50 pt de haut.
const WIDE_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="600" height="100"><rect width="600" height="100"/></svg>"#;

#[test]
fn framed_image_is_stretched_over_its_whole_frame() {
    let deck = common::blank_with_element(json!({"id": "i", "kind": base::IMAGE, "frame": {"x": 100, "y": 100, "w": 300, "h": 120},
        "params": {"asset": "sha256:wide", "fit": "stretch"}}));
    let assets = BTreeMap::from([("sha256:wide".to_owned(), WIDE_SVG.to_vec())]);
    let compiled = common::renderer().render(&deck, &assets).unwrap();
    // PNG à 1 px/pt : le bas du cadre (y = 100 + 120 - 5) est noir si l'image remplit le cadre,
    // clair si elle était dessinée à ses proportions (50 pt de haut).
    let png = image::load_from_memory(&compiled.png(0, 1.0).unwrap()).unwrap().to_rgba8();
    let zone = compiled.layout().zone_on(1).unwrap();
    let px = png.get_pixel((zone.x + 250.0) as u32, (zone.y + 215.0) as u32);
    assert_that!(px.0[0] < 40).is_true();
}
```

(`common::blank_with_element(element)`: a deck with one `blank` slide whose `elements` slot holds `element`, `"version": DECK_VERSION`. Frames are relative to the body zone, hence the `zone` offset. If the render crate's dev-deps have no PNG decoder, use the existing `png` dev-dependency's `Decoder` the way `tests/png.rs` reads its output.)

Golden layout guard (deviation 4 must be the only box change), in `tests/golden.rs`:

```rust
/// Chaque boîte d'élément des decks de référence : deck, page, id, x, y, w, h au dixième de pt.
fn reference_boxes() -> Vec<String> {
    let mut r = common::renderer();
    let mut lines = Vec::new();
    for (name, path) in REFERENCES {
        let deck = deck(path);
        r.set_theme(common::theme_tokens(reference_theme(name, &deck))).unwrap();
        let compiled = r.render(&deck, &BTreeMap::new()).unwrap();
        for page in 1..=compiled.page_count() {
            for b in compiled.layout().boxes_on(page) {
                lines.push(format!("{name} p{page} {} {:.1} {:.1} {:.1} {:.1}", b.id, b.x, b.y, b.w, b.h));
            }
        }
    }
    lines
}

#[test]
fn reference_decks_element_boxes_are_unchanged() {
    insta::assert_snapshot!("reference_boxes", reference_boxes().join("\n"));
}
```

Record `golden__reference_boxes.snap` in Step 1, with today's code (`common::renderer()` is still `Renderer::new` then). After the move, review the diff with `cargo insta review`: the only accepted changes are the `e-libre` line removed (its children `el1`, `el2` keep their numbers: they were already absolute) and, for deviation 4, the height of `i1` if and only if Step 1 recorded it taller than its row (write that value in the task report before the move). Anything else is a bug.

- [ ] **Step 3: Run, expect FAIL** — `cargo nextest run -p oku-slides-render` (compile errors on the new API, failing `issue_1036`).

- [ ] **Step 4: Implement the Typst side**

`widget.typ` — signature `widget(name, label: none, description: none, category: "element", grow: false, typography: false, params: (:), render)`; after the description asserts:

```typst
  assert(grow in (false, true, "fit"),
    message: "oku-slides : widget « " + name + " » : grow vaut false, true ou \"fit\"")
  assert(type(typography) == bool,
    message: "oku-slides : widget « " + name + " » : typography est un booléen")
  assert(category == "element" or (grow == false and not typography),
    message: "oku-slides : widget « " + name + " » : une slide ne déclare ni grow ni typography")
```

schema `(name: name, label: label, description: description, category: category, grow: grow, typography: typography, params: list)`. Header comment: flow slot entries are `(kind, id, grow, body)`; `(ctx.load)(asset: …, fit: …, flow: …)` for an image.

`data.typ`:
- keep `#import "widgets.typ": registry` (the core registry, slide widgets); `_base` comment becomes "Widgets du noyau (les slides) : ceux du rendu quand `widgets` vaut none."
- move `_widget` above `_flow`, then add (explicit comparisons: a hand-built schema dict with an odd value counts as `false` instead of breaking the deck):

```typst
// `grow` du widget d'élément `kind` : `true`, `"fit"`, sinon false (sans widget aussi).
#let _grow(widgets, kind) = {
  let w = _widget(widgets, kind, "element")
  let g = if w == none { false } else { w.schema.at("grow", default: false) }
  if g == true or g == "fit" { g } else { false }
}
```

  and in `_flow` the entry `grow: _grow(widgets, b.at("kind", default: none))` between `id` and `body`.
- `render-element`, right after `let w = _widget(widgets, kind, "element")`:

```typst
  // Lus avant `let (w, h)`, qui masque `w`.
  let grows = _grow(widgets, kind) != false
  let typo = w != none and w.schema.at("typography", default: false) == true
```

  then `kind in ("chart", "table")` → `grows`, and `kind == "text"` → `typo`.
- header comment: `"version": 7`, element kinds `base/text`.

`widgets.typ`: delete `_text`, `_card`, `_image`, `_chart`, `_table`, `_shape`, `_stack`, `_canvas` and every helper only they use; imports reduced to what the slide widgets use. `_content`:

```typst
      // Un bloc prend la hauteur selon le `grow` de son widget (voir `_layout`).
      grow: if free { () } else { args.blocks.map(b => b.grow) },
```

`#let registry = (_title, _toc, _chapter, _content, _end, _blank)`. Header: "Widgets de slide du noyau (le lot 3 les déplace) ; les widgets d'élément sont dans le pack `base` (`packs/base`)."

`lib.typ` — exactly these additions (what the seven files import):

```typst
#import "src/tokens.typ": default-tokens, default-params, merge, color-of, size-of, space-of, font-of, weight-of,
  radius-of, stroke-of, leading-of, list-marker, styled, text-style
#import "src/numbers.typ": format-number, deck-lang
```

(Of these **additions**, drop any that no pack file imports after the move; every existing export stays: hi-fi generated packs and the examples use them.) Comment above: "Ce qu'un widget de pack peut importer ; le pack `base` s'en sert."

Pack files — each moves today's code **verbatim** (same expressions, so rendering is identical), declares `#let widget = widget("<name>", …)` and imports only `"/oku-slides/lib.typ"` (with `params as p`) and `"_commun.typ"`:
- `_commun.typ`: `#let placeholder(t, label, height: 100%) = rect(width: 100%, height: height, fill: color-of(t, "surface"), radius: radius-of(t, "sm"), align(center + horizon, text(size: size-of(t, "caption"), fill: color-of(t, "ink-muted"), label)))`, used by image (`"image : " + …`), chart (`"chart"`) and table (`height: if framed { 100% } else { 6em }`, `"table"`).
- `text.typ`: `_inset-max`, `_spacing-max`, `_indent-max` (same names; `world_tests` reads them), `_inset`, `_spacing`, `_indent`, `_indented`, `_anchored`; `widget("text", typography: true, params: (body: p.rich()), …)`.
- `card.typ`: `widget("card", params: (kicker: p.plain(), title: p.plain(), body: p.rich(), highlight: p.flag()), (args, ctx) => card(…))`.
- `image.typ`: `widget("image", grow: true, …)`, loader call `(ctx.load)(asset: args.asset, fit: args.fit, flow: not ctx.framed)`.
- `chart.typ`: `widget("chart", grow: true, params: (option: p.host("chart")), …)`.
- `table.typ`: `widget("table", grow: "fit", params: (data: p.host("table")), …)` with its helpers and bounds (`_table-max-*`, kept as Typst guards; `render/src/tables.rs` stays the authority, see Step 5).
- `shape.typ`: path helpers (`_path-*`) and `widget("shape", …)`.
- `stack.typ`: `widget("stack", …)`.

`widgets.json`: the manifest of Global Constraints, with the key set of `templates/essentiel/pack.json` (`id`, `name`, `namespace`, `requires`, `description: "Les widgets de base : texte, carte, image, forme, graphique, tableau, pile."`, `parent`, `created`, `templates: []`).

`examples/from-json.typ`:

```typst
#import "../lib.typ": *
#import "../src/widgets.typ": registry as core
#import "../packs/base/widgets/text.typ": widget as text-w
#import "../packs/base/widgets/card.typ": widget as card-w
#import "../packs/base/widgets/image.typ": widget as image-w
#import "../packs/base/widgets/shape.typ": widget as shape-w
#import "../packs/base/widgets/chart.typ": widget as chart-w
#import "../packs/base/widgets/table.typ": widget as table-w
#import "../packs/base/widgets/stack.typ": widget as stack-w
// Comme l'assemblage généré par l'app : chaque nom préfixé par `base/`.
#let base = (text-w, card-w, image-w, shape-w, chart-w, table-w, stack-w).map(w => w + (name: "base/" + w.name))
#render-deck(json("deck.json"), theme: papier, widgets: by-name(core + base))
```

`deck.json`: `base/*` element kinds (version bumped in Task 3). README "Compiling the examples": `typst compile --root ../.. from-json.typ` (pack files import `/oku-slides/lib.typ`); run both commands and commit regenerated PDFs only if they change.

`typst/oku-slides/AGENTS.md`: add "Declare zone sizing and typography in the widget schema (grow, typography); never test a kind name in the core library. (plan:2026-10-18-packs-lot2-base)" and "Append the new base version to packs/base/versions.txt whenever a file of packs/base changes. (plan:2026-10-18-packs-lot2-base)".

- [ ] **Step 5: Implement the Rust side**

`world.rs` `main_source`: `#let load-image(e, asset: none, fit: "cover", chart: false, flow: false) = {{` and `if flow {{` in place of `if e.at("kind", default: none) == "image" and "frame" not in e {{` (comment: "le widget le demande : dans le flux, à ses proportions").

`base_pack.rs`:

```rust
//! Le pack de widgets intégré `base`, embarqué : manifeste et fichiers de widgets.
//!
//! Seule copie des sources de `typst/oku-slides/packs/base` dans le binaire :
//! le rendu le rend seul (tests, `Renderer::with_builtins`), la session en fait
//! un pack intégré de la bibliothèque, à côté d'Essentiel et Grilles.

use snafu::ResultExt;

use oku_slides_core::{PackFiles, WidgetFile, kinds::base};

use crate::PackWidgets;
use crate::error::{BasePackSnafu, RegistryError};

macro_rules! base_widgets {
    ($($name:literal),* $(,)?) => {
        &[$(($name, include_bytes!(concat!("../../../typst/oku-slides/packs/base/widgets/", $name, ".typ")))),*]
    };
}

/// Le manifeste du pack `base`.
pub(crate) const BASE_MANIFEST: &[u8] = include_bytes!("../../../typst/oku-slides/packs/base/widgets.json");

/// Les fichiers de widgets du pack `base`, par nom (`_commun` compris).
pub(crate) const BASE_WIDGETS: &[(&str, &[u8])] =
    base_widgets!("_commun", "text", "card", "image", "shape", "chart", "table", "stack");

/// Le pack `base` en fichiers de bibliothèque.
pub fn base_pack_files() -> Result<PackFiles, RegistryError> {
    let widgets = BASE_WIDGETS
        .iter()
        .map(|(name, bytes)| WidgetFile::parse(name).map(|f| (f, *bytes)).ok_or(*name))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|file| BasePackSnafu { file }.build())?;
    PackFiles::from_bytes(BASE_MANIFEST, widgets).ok_or_else(|| BasePackSnafu { file: "widgets.json" }.build().into())
}

/// Le pack `base`, prêt à rendre (id = empreinte de sa version).
pub fn base_pack_widgets() -> Result<PackWidgets, RegistryError> {
    let files = base_pack_files()?;
    PackWidgets::new(files.version().hex(), base::namespace(), files.widgets().clone())
}

/// Les versions de `base` livrées par l'app, une empreinte hex par ligne.
const BASE_VERSIONS: &str = include_str!("../../../typst/oku-slides/packs/base/versions.txt");

/// Les versions livrées de `base` (lignes non vides de `versions.txt`).
pub(crate) fn shipped_base_versions() -> impl Iterator<Item = &'static str> {
    BASE_VERSIONS.lines().map(str::trim).filter(|l| !l.is_empty())
}

/// Vrai pour une version livrée du pack `base` : le repli ne l'écarte jamais.
///
/// L'id d'un pack rendu est l'empreinte de sa version : un `base` modifié,
/// ou un autre pack qui prendrait cet id, n'est pas de confiance.
pub fn is_trusted_pack(p: &PackWidgets) -> bool {
    p.namespace().as_str() == base::NAMESPACE && shipped_base_versions().any(|v| v == p.id())
}

#[cfg(test)]
#[path = "base_pack_tests.rs"]
mod base_pack_tests;
```

(Adjust the `.build()` / `.into()` calls to how `RegistryError` wraps `RegistryInner` — read `error.rs:200-300`, as `BadPackIdSnafu { id }.build().into()` is used in `registry.rs:48`.) `error.rs`: `RegistryInner::BasePack { file: String, backtrace: Backtrace }` (display "built-in base pack unreadable: {file}"), `RegistryErrorKind::BasePack`, Fluent `registry-base-pack-broken = The built-in widget pack is damaged ({ $file }); reinstall the app.` in `document.ftl` (20 locales).

`registry.rs`: rename `base_registry` → `core_registry` and `base_probe` → `core_probe` (every caller in the workspace: `rg -n 'base_registry|base_probe' crates`), doc "Le registre des widgets du noyau (les slides), …". Add:

```rust
/// Registre intégré, lu une fois par processus : noyau et pack `base` intégré.
static BUILTIN: Mutex<Option<Arc<WidgetRegistry>>> = Mutex::new(None);

/// Le registre intégré : les slides du noyau et les widgets du pack `base` intégré.
///
/// Pour tout ce qui écrit ou valide des éléments avant que les packs du deck
/// soient résolus (premier replica, import, aperçus).
pub fn builtin_registry(fonts: &Fonts) -> Result<Arc<WidgetRegistry>, RegistryError> {
    let mut builtin = BUILTIN.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(r) = builtin.as_ref() {
        return Ok(r.clone());
    }
    let r = Arc::new(load_registry(fonts, &PackSet::new(vec![base_pack_widgets()?])?)?);
    *builtin = Some(r.clone());
    Ok(r)
}
```

`lib.rs`:

```rust
    /// Un rendu aux widgets intégrés : le noyau et le pack `base` intégré.
    pub fn with_builtins(fonts: Fonts) -> Result<Self, RenderError> {
        let registry = registry::builtin_registry(&fonts).map_err(Box::new).context(BaseRegistrySnafu)?;
        let pack = base_pack_widgets().map_err(Box::new).context(BaseRegistrySnafu)?;
        let set = PackSet::new(vec![pack]).map_err(Box::new).context(BaseRegistrySnafu)?;
        let mut r = Self::new(fonts)?;
        r.set_widgets(&set, registry)?;
        Ok(r)
    }
```

`RenderInner::BaseRegistry`'s display becomes "the built-in widget registry could not be read" (its Fluent message, if any, likewise in 20 locales). Re-export `base_pack_files`, `base_pack_widgets`, `is_trusted_pack`, `core_registry`, `builtin_registry`, and name them with `Renderer::with_builtins` in the crate doc list (`lib.rs:1-30`). `builtin_registry`'s doc says, like `base_registry`'s, that the first caller's fonts win (schemas do not depend on them). `wire.rs` `from_registry`: `RegistryInner::BasePack { .. } => Self::Setup { message: first_line(…) }` (existing variant: `PROTOCOL_VERSION` unchanged). `versions.txt`: the hex of `base_pack_files()?.version()` (print it once with a test-time `dbg!`, then remove the `dbg!`). `justfile` `typst` recipe: `typst compile --root typst --font-path {{fonts}} typst/oku-slides/examples/{{example}}.typ`.

`world_tests.rs:55-69`: read `_inset-max` / `_spacing-max` from `BASE_WIDGETS` entry `"text"` (`std::str::from_utf8(bytes)`), `_tracking-max` still from `LIBRARY`'s `src/content.typ`.

`core/src/model.rs` `Element::text`: `Self::new(id, crate::kinds::base::TEXT)`; doc "Un élément `base/text` dont le corps (`body`) est `paragraphs`."

`tests/common/mod.rs`: `pub fn renderer() -> Renderer` (= `Renderer::with_builtins(Fonts::default())`), `renderer_with(set)` keeps building `load_registry` + `set_widgets` but over `base_pack_widgets()` + `set`'s packs, `compile_with_packs(packs: &[PackWidgets], blocks)` / `render_with_packs` (the old single-pack helpers call them), every deck literal uses `oku_slides_core::DECK_VERSION`, `IMAGE_KIND`/`TEXT_KIND` = the base constants, `TALL_SVG`, `token_size`. Every render test listed under Files replaces `Renderer::new(Fonts::default())` by `common::renderer()` and its bare element kinds by `kinds::base::*`.

The table, style and path bounds stay enforced in Rust whatever pack renders the element (`render/src/tables.rs` `parse_table` keyed on `HostType::Table`, `core` style and `ShapePath` checks keyed on host types); add in `tests/tables.rs` `table_of_a_pack_widget_is_sanitized_before_typst`: a pack widget `test/grid` with `p.host("table")` and 500 rows renders the guarded form (rows capped as for `base/table`).

`crates/oku-slides-render/ARCHITECTURE.md`: `base_pack.rs` (render owns the `base` sources, the session lists them), `core_registry` / `builtin_registry` / `Renderer::with_builtins`, `flow:` in `load-image`; replace "make the session fall back to the base widgets" by "make the session disable the faulty pack"; stay ≤ 50 lines.

- [ ] **Step 6: Fixtures and golden**

- `tests/golden/widgets.json`, `fixtures/roadmap.json`: element kinds → `base/<name>` (`rg -n '"kind": "(text|card|image|shape|chart|table|stack)"'`); slide kinds unchanged; `"version": 6` kept (Task 3).
- The golden `canvas` element `e-libre` (`tests/golden/widgets.json:435`): replace it by its children, in its place in the parent list, each child's frame shifted by the canvas frame (`x + 800`, `y + 0`). `_canvas` (`widgets.typ:332-337`) and `free-area` (`zone.typ:43-46`) both place with `place(top + left, dx, dy, …)`, so the page must stay pixel-identical. If it does not, stop and report (G9) — never re-record.
- `golden.rs` (and `dump_reference_pages`): `common::renderer()` instead of `Renderer::new`.
- Regenerate `fixtures/widgets/base-registry.json` from `probe_registry(&Fonts::default(), &PackSet::new(vec![base_pack_widgets()?])?)` (pretty JSON, key order of the current file); `base_registry_fixture_matches_the_live_registry` compares with that set. `cargo insta review` on `widgets__base_registry.snap`: accept only qualified names, `grow` / `typography` keys and the removed `canvas`.
- `registry_tests.rs:171-176` becomes `load_registry_pack_card_and_base_card_coexist` (`r.get("acme/card")` and `r.get(base::CARD)` both some); `:207-210` (empty set) asserts `core_registry`.
- `fixtures/AGENTS.md`: "Keep fixtures/widgets/base-registry.json equal to the live core registry plus the base pack; a render test enforces it. (plan:2026-10-18-packs-lot2-base)".

- [ ] **Step 7: Run, expect PASS** — `cargo nextest run -p oku-slides-render -E 'not binary_id(oku-slides-render::import_retrofit) and not binary_id(oku-slides-render::import_decor) and not binary_id(oku-slides-render::import_pack)'` and `cargo nextest run -p oku-slides-render-worker -p oku-slides-i18n`: `golden` with an **unchanged** pixel snapshot, the boxes snapshot reviewed, `widget_files` (README ↔ kpi verbatim), `issue_1036`. Compile both examples with the README commands.

- [ ] **Step 8: Commit** — `feat(render)! : pack de widgets intégré base, noyau Typst sans widget d'élément, tailles lues du schéma (t-1036)`.

---

### Task 3: Core — deck version 7, base kinds in code and tests, `canvas` gone

**Files:**
- Modify: `crates/oku-slides-core/src/model.rs:20` (`DECK_VERSION = 7`), `src/model_tests.rs:10-31`, `src/replica/read_tests.rs:220-228`, `src/ops/packs.rs` + `packs_tests.rs` (`base` cannot be removed, deviation 16), `src/error/mod.rs` (`OpInner::BaseRequired`, `OpErrorKind::BaseRequired`), `src/test_registry.rs`, `tests/common/mod.rs`, and every core test with a bare element kind or `canvas` (map below)
- Modify i18n: `*/core.ftl` (`op-base-required`, after `op-pack-requirement-missing`, 20 locales)
- Modify (deck version 6 → 7 **and** element kinds `base/*`): `fixtures/roadmap.json`, `crates/oku-slides-render/tests/golden/widgets.json`, `typst/oku-slides/examples/deck.json`, `docs/aide/{en,fr}.json` (`card` ×19, `text` ×30 each; prose at line 1718 lists the widgets: drop `canvas`, say element widgets come from the `base` pack; keep the s-format-loro strings intact), `templates/{essentiel,grilles}/pack.json` slide literals if they carry a version
- Modify (version only): every test literal `"version": 6` that is a **deck** version (`rg -n '"version": ?6' crates fixtures docs/aide templates ui src-tauri`), not `core/src/sources_tests.rs:98` (sources manifest) nor the version-history cases of `model_tests.rs`
- Modify: `docs/aide` help-slide tests if they quote a changed string (`crates/oku-slides-session/src/help_tests.rs`)

Core test files (survey counts): `tests/ops.rs` (45; `canvas_chain` 510-564, 640-644), `src/validate_tests.rs` (44; canvas 116, 207, 279-351), `src/replica/mod_tests.rs` (18; `SetKind … "canvas"` 610), `src/locate_tests.rs` (12; 102-116), `src/replica/tree_tests.rs` (11; 493-519), `src/sources_tests.rs` (8; 409-433), `tests/tables.rs` (8), `src/canonical_tests.rs` (7; 159, 352), `src/replica/undo_tests.rs`, `src/replica/tree/deck_tree_tests.rs` (556-577), `src/replica/read_tests.rs` (179, 263), `src/params_tests.rs`, `tests/move_element.rs`, `tests/charts.rs`, `src/fields_tests.rs`, `src/widget_tests.rs`, `src/replica/{host,soft_delete}_tests.rs`, `tests/{tree_convergence,convergence,grid}.rs`, `tests/loro_*_probe.rs`, `tests/regressions/issue_*.rs` (canvas in `issue_0652_undo_remove_vs_peer_move.rs:246`). `src/namespace_tests.rs` and `src/ops/packs_tests.rs` use bare `text` as "a core widget": switch those to `blank`; keep `base/…` where a pack widget is meant.

**Interfaces:**
- Consumes: `fixtures/widgets/base-registry.json` with the `base` pack (Task 2), `kinds::base` (Task 1).
- Produces: `DECK_VERSION = 7`; `Op::RemovePack { namespace: base }` refused with `OpErrorKind::BaseRequired` (`op-base-required = Every deck keeps the base widget pack.`); a test-only free-slot widget `test/frame-box` (`children: p.slot(mode: "free")`, category element) in the probe's `packs` (namespace `test`) of `test_registry.rs` and `tests/common/mod.rs`, replacing `canvas` wherever a test needs a free slot below an element; `base/stack` where a flow slot suffices.

- [ ] **Step 1: Write the failing tests** (`model_tests.rs`, replacing `version_5_is_refused`)

```rust
#[rstest]
#[case(1)]
#[case(5)]
#[case(6)]
fn deck_from_json_older_version_is_refused(#[case] version: u32) {
    let err = Deck::from_json(&format!(r#"{{"version": {version}, "slides": []}}"#)).unwrap_err();
    assert_that!(err.to_string()).contains(format!("unsupported deck version {version}"));
}

#[test]
fn deck_from_json_current_version_is_read() {
    assert_that!(Deck::from_json(r#"{"version": 7, "slides": []}"#).ok().map(|d| d.version)).is_equal_to(Some(7));
}

#[test]
fn deck_empty_is_version_7() {
    assert_that!(Deck::empty().version).is_equal_to(7);
}
```

(`Deck::from_json`'s version error is a serde message, not a Fluent one, so the text assertion follows `model_tests.rs:10-12`; if deck errors gain Fluent keys, assert on the key.) `from_json_newer_version_returns_an_error` (`model_tests.rs:16-17`) and `read_deck_of_a_newer_version_returns_unsupported_version` (`replica/read_tests.rs:220-221`) use `DECK_VERSION + 1` instead of `7`. In `replica/read_tests.rs`, beside it, `read_deck_of_version_6_returns_unsupported_version` following that test's setup verbatim.

`ops/packs_tests.rs`:

```rust
#[test]
fn remove_pack_base_is_refused_even_unused() {
    let mut deck = Deck::empty();
    deck.widgets.insert(kinds::base::namespace(), pin(&[]));
    let err = apply_all(&mut deck, &registry(), vec![Op::RemovePack { namespace: kinds::base::namespace() }]).unwrap_err();
    assert_that!(err.op_kind()).is_equal_to(Some(OpErrorKind::BaseRequired));
}

#[test]
fn remove_pack_base_message_is_localized_in_french() {
    let mut deck = Deck::empty();
    deck.widgets.insert(kinds::base::namespace(), pin(&[]));
    let err = apply_all(&mut deck, &registry(), vec![Op::RemovePack { namespace: kinds::base::namespace() }]).unwrap_err();
    assert_that!(err.localize(&Localizer::new(Some("fr")))).is_not_equal_to(err.localize(oku_slides_i18n::english()));
}
```

(Same accessors as the lot 1 tests of this file: `op_kind()` and the `Localize` impl.) `packs::remove` checks first: `ensure!(ns.as_str() != kinds::base::NAMESPACE, BaseRequiredSnafu)`.

- [ ] **Step 2: Run, expect FAIL** — `cargo nextest run -p oku-slides-core` (version tests, and bare kinds against the new fixture).

- [ ] **Step 3: Implement** — `DECK_VERSION = 7`; the fixture and help-deck rewrites under Files; in each mapped test: element kind literals → `kinds::base::*` in Rust, `"base/<name>"` in JSON; `canvas` chains → `kinds::base::STACK` when only nesting matters, `test/frame-box` when a free slot is needed (frames, `move_element` between free slots, `SetKind` to a free-slot widget).

- [ ] **Step 4: Run, expect PASS** — `cargo nextest run -p oku-slides-core -p oku-slides-i18n -p oku-slides-render -p oku-slides-render-worker` with the Task 2 exclusion of `import_*` (the golden pixel and boxes snapshots unchanged: only `"version"` moved).

- [ ] **Step 5: Commit** — `feat(core)! : decks en version 7, kinds du pack base dans le cœur, ses tests et l'aide, base jamais retiré, canvas supprimé (t-1037)`.

---

### Task 4: Import — kinds from the constants, `base` pinned by every import

**Files:**
- Modify: `crates/oku-slides-import/src/retrofit/{content.rs:162-305,place.rs:23,84,252,270,measure.rs:223-243,313-315,339,group.rs:27,143,148,190}`, `src/hifi/{element.rs:129-385,element_text.rs:182,pack.rs:204-240,mod.rs:95-180,slide.rs:271}`, `src/coverage.rs:97`, `src/plan.rs:38` (doc of `WidgetRequest.fallback`), `src/hifi/mod.rs:61-76` (`HifiTarget.base`), `src/test_support.rs:119-136`, `tests/common/mod.rs:77-86`, every `HifiTarget` literal (`tests/hifi.rs:31-41`, `tests/regressions/issue_0753_output_budget.rs`, `src/hifi/mod_tests.rs:63-90`), and every import test with a bare element kind (`hifi/element_tests.rs` 39, `retrofit/content_tests.rs` 13, `coverage_tests.rs` 10, `retrofit/slide_tests.rs` 9, `retrofit/group_tests.rs` 8, `measure_tests.rs` 6, `zone_tests.rs` 5, `plan_tests.rs` 5, `tests/retrofit.rs` 5, the 2–3-hit files) — never source item kinds (`pattern.rs`, `pptx/*`, `ItemKind`), token paths (`component.card`) or loss reasons
- Modify: `templates/essentiel/pack.json`, `templates/grilles/pack.json` (`"requires": ["base"]`, element kinds `base/*`)
- Modify render tests that run the importer: `crates/oku-slides-render/tests/{import_retrofit,import_decor,import_pack}.rs` (sets include `base_pack_widgets()`; their `HifiTarget`, if any, gains `base`)
- Snapshots: `tests/snapshots/{hifi__*,retrofit__*}.snap`, `src/hifi/snapshots/*element_tests*.snap` — regenerated with `cargo insta review`
- Modify: `crates/oku-slides-import/ARCHITECTURE.md` (kinds from `kinds::base`; `HifiTarget.base`; the session pins `base` for a retrofit)

**Interfaces:**
- Consumes: `kinds::base` (Task 1); the fixture registry with `base` (Task 2).
- Produces:
  - `HifiTarget.base: PackPin` (doc "L'épinglage de `base` que le deck importé prend ; la session le remplit.", the built-in version, Task 5): the hi-fi plan's ops start with `Op::AddPack { namespace: base, pin: target.base }`, then the `deck` pack's `AddPack` whose manifest and pin have `requires: [base]`; the coverage scratch deck applies them in that order. `RetrofitTarget` is unchanged: the session pins `base` before projecting a retrofit (Task 5), and `target.registry` already holds it.
  - `group::STACK` deleted; retrofit and hifi emit and match only `kinds::base::*` (the fallback of a widget request becomes `base/stack`).
  - `generated_registry(base, pack)` unchanged: it already keeps every widget of `base` (qualified names included); the caller passes a registry that holds the `base` pack.

- [ ] **Step 1: Write the failing tests**

```rust
// crates/oku-slides-import/src/hifi/pack_tests.rs
#[test]
fn pack_manifest_requires_base() {
    let m = pack_manifest(&generated_pack(), Uuid::nil(), "Import", "2026-10-18T00:00:00Z", &BTreeMap::new());
    assert_that!(m.requires).is_equal_to(vec![kinds::base::namespace()]);
}

// crates/oku-slides-import/tests/hifi.rs (the file's `plan(fixture)` and `import(fixture)`; `HifiTarget` gains
// `base: common::base_pin()`, the pin of the `base` pack of `fixtures/widgets/base-registry.json`)
#[test]
fn hifi_plan_pins_base_before_the_deck_pack() {
    let ops = plan("minimal").ops;
    let pins: Vec<&str> =
        ops.iter().filter_map(|o| match o { Op::AddPack { namespace, .. } => Some(namespace.as_str()), _ => None }).collect();
    assert_that!(pins).is_equal_to(vec!["base", "deck"]);
}

#[test]
fn hifi_deck_kinds_are_all_base_or_deck() {
    let (_, deck) = import("kitchen");
    let kinds = deck_kinds(&deck);
    let stray: Vec<&String> = kinds.iter().filter(|k| !k.starts_with("deck/") && !kinds::base::ALL.contains(&k.as_str())).collect();
    assert_that!((kinds.is_empty(), stray.is_empty())).is_equal_to((false, true));
}

// crates/oku-slides-import/tests/retrofit.rs (the file's `import(...)` and `deck_kinds`, :65-73)
#[test]
fn retrofit_deck_element_kinds_are_all_base() {
    let (_, deck) = import("kitchen", "essentiel", "papier");
    let kinds = deck_kinds(&deck);
    let stray: Vec<&String> = kinds.iter().filter(|k| !kinds::base::ALL.contains(&k.as_str())).collect();
    assert_that!((stray.is_empty(), kinds.contains(&kinds::base::TEXT.to_owned()), kinds.contains(&kinds::base::STACK.to_owned())))
        .is_equal_to((true, true, true));
}
```

`hifi_applies_without_issue_and_covers_everything` (existing) already requires the coverage to be measured and complete: it fails while the scratch deck lacks `base`. `common::base_pin()`: `PackPin { id: <the fixture's base id>, version: <any valid PackVersion>, requires: vec![] }`, next to `common::registry()`; the `deck_kinds` helper of `tests/retrofit.rs` moves to `tests/common/mod.rs` so `tests/hifi.rs` uses it too. Adapt `import("kitchen", "essentiel", "papier")` to the real arguments of `tests/retrofit.rs`'s `import`. Fold the old `measure.rs` bare-kind assumptions into `measure_tests.rs`: `measure_base_card_measures_its_content` (> 0) next to `measure_unknown_kind_is_zero` (`"acme/card"` → 0.0).

- [ ] **Step 2: Run, expect FAIL** — `cargo nextest run -p oku-slides-import`.

- [ ] **Step 3: Implement** — each literal at the listed lines → the constant (`"card"` → `base::CARD`; `matches!(e.kind.as_str(), "text" | "card")` → `[base::TEXT, base::CARD].contains(&e.kind.as_str())`; `match e.kind.as_str() { "text" => … }` → `match e.kind.as_str() { base::TEXT => … }` (`&str` consts are patterns), same arms, same measures); the hifi manifest `requires: vec![kinds::base::namespace()]` and the deck pin's `requires` equal; the `base` `AddPack` first in the hi-fi ops and in the coverage scratch deck; built-in `pack.json` files get `"requires": ["base"]` and qualified element kinds; the three render `import_*` tests build their sets with `base_pack_widgets()`.

- [ ] **Step 4: Review snapshots** — `cargo insta review`: accept only element kinds becoming `base/*`, the `requires` of the hifi pack, the new `add_pack base` op and the fallback `base/stack` (the deck version moved in Task 3). Anything else is a bug.

- [ ] **Step 5: Run, expect PASS** — `cargo nextest run -p oku-slides-import -p oku-slides-render` (no exclusion now).

- [ ] **Step 6: Commit** — `feat(import)! : kinds du pack base depuis les constantes, base épinglé par chaque import (t-1038)`.

---

### Task 5: Session — built-in `base`, decks and producers pin it, registries, labels, host params

**Files:**
- Modify: `crates/oku-slides-session/src/library_kind.rs:60-90` (`BUILTIN_PACKS`: Essentiel, Grilles, then `base`), `src/pack.rs` (`BASE_PACK`), `src/library.rs:591` (`template_sets`: packs with templates only), `src/deck_pack.rs:24-48` (`adopt_default_packs`), `src/help.rs:61-80`, `src/import/{mod.rs:191,hifi.rs:68-87,retrofit.rs:72-87}`, `src/templates_mode.rs:125-140,215-225` (working deck keeps `base`; saved `requires`), `src/deck_widgets.rs:130-190` and `src/probed_widgets.rs:180-240` (the current built-in `base` resolved in-process), `src/tables.rs:11-28`, `src/param_fields.rs:17-31`, `src/target_params.rs:5` (doc of the key format), `src/views.rs:53` + `src/lib.rs:618-650` (`host_params`), the replica / validation sites that ran on the core-only registry before the deck's packs resolve: `src/lib.rs:360-363` (blank `SourceBundle`), `src/help.rs:69`, `src/workspace_same_deck.rs:50` (if it builds a `SourceBundle` or validates; a slot registry stays core), `src/import/mod.rs:191`; and `src/widget_preview.rs:293,356-360` (preview set = built-in `base` + the edited pack, unless it is `base`)
- Keep `core_registry` (a slot that renders without the deck's set, or a fallback): `src/lib.rs:298` (`new_renderer`), `src/deck_widgets.rs:183` (empty set), `src/probed_widgets.rs:226-235` (no usable pack); `src/history.rs:126`, `src/resume.rs:635,638`, `src/deck_file.rs:45,105` keep the registry they are given (`self.registry` or the caller's).
- Modify tests: `library_tests.rs:16-23` (builtin ids, per-pack `created`), `deck_pack_tests.rs`, `help_tests.rs`, `pack_notice_tests.rs:31` (`user_pack_id` skips built-ins), `import/mod_tests.rs:241` (`["base", "deck"]`), `deck_widgets_fallback_tests.rs:396` (pin count), `tests/session.rs:839-885` (`add_widgets` now lists `base/*` first: assert on the non-base entries), `render_fonts_tests.rs:13-21` and `workspace_tests.rs:863-873` (must stay green unchanged: built-ins start no worker), and the bare kinds of `tests/tables.rs`, `tests/errors.rs`, `src/agent/forms_tests.rs`, `charts_tests.rs`, `namespace_slug_tests.rs`, `fields_tests.rs`, `history_tests.rs`, `import/hifi_tests.rs`, `lib_tests.rs`, `resume_tests.rs`, `tests/history.rs` (→ `kinds::base`)
- Modify i18n: `crates/oku-slides-i18n/src/locales/*/session.ftl:158-183` — `session-widget-kind-{card,image,shape,stack,text}` → `session-widget-kind-base-{…}`, `session-param-{card,image,shape,stack,text}-*` → `session-param-base-{…}-*`, plus new `session-widget-kind-base-chart` / `-base-table` after `-base-card` (20 locales, translated; slide keys unchanged); `*/ui.ftl:152-159` `ui-element-kind` gains `[chart]`, `[table]`, `[stack]` (and drops `[group]` if no kind uses it — `rg -n '"group"' ui/src crates`)
- Create: `tests/regressions/issue_1039_new_deck_pins_base.rs`, `tests/regressions/issue_1039_replicas_know_base_hosts.rs` (declared in `tests/regressions/main.rs`)
- Modify: `crates/oku-slides-session/ARCHITECTURE.md` (built-in `base`, who pins it, in-process built-in, `host_params`, `builtin_registry` sites; ≤ 50 lines), `crates/oku-slides-session/AGENTS.md` (rules)

**Interfaces:**
- Consumes: `base_pack_files`, `base_pack_widgets`, `builtin_registry`, `core_registry` (Task 2), `kinds::base`, `WidgetRegistry::host_params` (Task 1), `HifiTarget.base` (Task 4), `OpErrorKind::BaseRequired` (Task 3).
- Produces:
  - `pub const BASE_PACK: Uuid` (doc "L'id du pack de widgets intégré `base`.").
  - `adopt_default_packs`: unchanged for a deck with pins; a deck without pins gets the current `base` and `essentiel` (and Essentiel's closure). `base` is asked for explicitly: a lot-1 user version of Essentiel may lack `requires: [base]`.
  - `open_help`, hi-fi import and retrofit pin the **built-in** `base` version (`builtin_versions()`), inserted with its files into `deck.widgets` / `packs` **before** `sources_with_packs`; retrofit adds it only when the target's closure lacks it.
  - Templates mode: the working deck also pins the user deck's `base` (or the built-in one); a saved manifest's `requires` is its previous `requires` plus every namespace (other than its own) of the qualified kinds its templates and examples use.
  - `template_sets` lists only packs with at least one template.
  - A pin whose version is the current built-in `base` (`base_pack_files()?.version()`) resolves in-process: `PackWidgets` from `base_pack_widgets()`, no worker probe; a set holding only that pack takes `builtin_registry` and starts no worker.
  - `Snapshot.host_params: BTreeMap<String, BTreeMap<HostType, String>>` (doc "Paramètres d'hôte des widgets d'élément du deck, par kind et type d'hôte."), computed in `snapshot()` from `self.registry.host_params()`.
  - `kind_name` / `param_name` look a key up only for a bare kind or a kind in `kinds::base::ALL` (`widget_key`, below).

- [ ] **Step 1: Write the failing tests**

```rust
// src/library_tests.rs (replaces library_builtins_have_their_namespaces)
#[test]
fn library_builtins_have_their_namespaces() {
    let names: Vec<_> = builtin_versions().iter().map(|b| b.manifest.namespace.to_string()).collect();
    assert_that!(names).is_equal_to(vec!["essentiel".to_owned(), "grilles".to_owned(), "base".to_owned()]);
}

#[test]
fn library_builtin_base_has_its_id_and_seven_widget_files() {
    let base = builtin_versions().iter().find(|b| b.manifest.namespace == kinds::base::namespace()).unwrap();
    let files = base.files.widgets().keys().filter(|f| !f.is_common()).count();
    assert_that!((base.manifest.id, files)).is_equal_to((BASE_PACK, 7));
}

#[rstest]
#[case("base")]
#[case("essentiel")]
#[case("grilles")]
fn library_add_another_pack_claiming_a_builtin_namespace_is_refused(#[case] namespace: &str) {
    let lib = lib();
    let err = lib.add_version(Uuid::new_v4(), &files_of(&manifest_ns(Uuid::new_v4(), namespace, &[]))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NamespaceTaken);
}

#[test]
fn library_namespace_of_a_pack_named_base_skips_the_builtin() {
    let lib = lib();
    assert_that!(library_namespace(&lib, "Base").unwrap().as_str()).is_not_equal_to("base");
}

#[test]
fn library_template_sets_leave_out_packs_without_templates() {
    let ids: Vec<Uuid> = lib().template_sets().iter().map(|p| p.id).collect();
    assert_that!((ids.contains(&DEFAULT_PACK), ids.contains(&BASE_PACK))).is_equal_to((true, false));
}
```

(The two namespace tests pass already — deviation 15 —; they guard it. Use the real names of `template_sets` / `PackInfo.id`.)

```rust
// src/deck_pack_tests.rs
#[test]
fn adopt_default_packs_empty_deck_pins_base_and_essentiel() {
    let (lib, mut sources) = (library_with(&[]), blank_sources());
    adopt_default_packs(&lib, &mut sources).unwrap();
    let pinned: Vec<String> = sources.deck().widgets.keys().map(ToString::to_string).collect();
    assert_that!(pinned).is_equal_to(vec!["base".to_owned(), "essentiel".to_owned()]);
}

#[test]
fn adopt_default_packs_twice_adds_nothing() {
    let (lib, mut sources) = (library_with(&[]), blank_sources());
    adopt_default_packs(&lib, &mut sources).unwrap();
    let once = sources.deck().widgets.clone();
    adopt_default_packs(&lib, &mut sources).unwrap();
    assert_that!(sources.deck().widgets.clone()).is_equal_to(once);
}

#[test]
fn adopt_default_packs_deck_pinning_essentiel_without_base_is_not_repaired() {
    let (lib, mut sources) = (library_with(&[]), blank_sources());
    let mut deck = sources.deck().clone();
    deck.widgets.insert(ns("essentiel"), builtin_pin());
    sources.apply_silent(deck).unwrap();
    adopt_default_packs(&lib, &mut sources).unwrap();
    assert_that!(sources.deck().widgets.contains_key(&kinds::base::namespace())).is_false();
}
```

(`builtin_pin()` (`deck_pack_tests.rs:19-21`) looks Essentiel up by `DEFAULT_PACK`, not by index `[0]`; the same for every `builtin_versions()[0]` in the session tests — `library_tests.rs:29,79,89,100,120,314`, `templates_mode_tests.rs:11,50,109,158`, `resume_tests.rs:155,619`, `help_tests.rs:214`, `library_cache_tests.rs:108`, `tests/regressions/issue_0328_*.rs:78` — even though Essentiel stays first.)

```rust
// src/param_fields_tests.rs
#[rstest]
#[case(kinds::base::CARD, "Carte")]
#[case(kinds::base::TEXT, "Texte")]
#[case(kinds::base::CHART, "Graphique")]
#[case(kinds::base::TABLE, "Tableau")]
#[case("content", "Contenu")]
#[case("acme/card", "acme/card")]
#[case("base-x/card", "base-x/card")]
fn kind_name_in_french(#[case] kind: &str, #[case] expected: &str) {
    assert_that!(kind_name(kind, None, &Localizer::new(Some("fr")))).is_equal_to(expected.to_owned());
}

#[test]
fn param_name_of_base_card_title_is_localized_in_french() {
    let p = ParamSchema { name: "title".into(), label: None, ty: ParamType::Plain, default: None, bind: None };
    assert_that!(param_name(kinds::base::CARD, &p, &Localizer::new(Some("fr")))).is_equal_to("Titre".to_owned());
}
```

(Locale parity of the renamed keys is enforced by `every_locale_has_exactly_the_reference_keys`, `oku-slides-i18n/src/lib_tests.rs:43`. Take the expected French words from `fr/session.ftl`.)

```rust
// tests/tables.rs (the file's `with_chart`, `show_as`, `element`, `slide_json`)
#[test]
fn show_as_table_makes_a_base_table_with_the_same_dataset() {
    let mut s = session();
    let slide = with_chart(&mut s);
    let before = element(&mut s, &slide)["params"]["option"]["dataset"]["source"].clone();
    ok(show_as(&mut s, &slide, "table"));
    let after = element(&mut s, &slide);
    assert_that!((after["kind"].as_str(), after["params"]["data"]["dataset"]["source"].clone()))
        .is_equal_to((Some(kinds::base::TABLE), before));
}

// src/lib_tests.rs
#[test]
fn snapshot_host_params_maps_every_base_host() {
    let s = session_with_fixture();
    let expected = BTreeMap::from([
        (kinds::base::CHART.to_owned(), BTreeMap::from([(HostType::Chart, "option".to_owned())])),
        (kinds::base::SHAPE.to_owned(), BTreeMap::from([(HostType::Path, "path".to_owned())])),
        (kinds::base::TABLE.to_owned(), BTreeMap::from([(HostType::Table, "data".to_owned())])),
    ]);
    assert_that!(s.snapshot().host_params).is_equal_to(expected);
}

// src/help_tests.rs
#[test]
fn help_deck_pins_the_builtin_base_and_essentiel() {
    let mut s = session();
    s.open_help("fr", None).unwrap();
    let builtin = |id| builtin_versions().iter().find(|b| b.manifest.id == id).unwrap().version.clone();
    let pins: Vec<(String, PackVersion)> = s.deck().widgets.iter().map(|(n, p)| (n.to_string(), p.version.clone())).collect();
    assert_that!(pins).is_equal_to(vec![("base".into(), builtin(BASE_PACK)), ("essentiel".into(), builtin(DEFAULT_PACK))]);
}

// src/import/mod_tests.rs (the file's hi-fi and retrofit import helpers, :230-260)
#[test]
fn hifi_import_pins_the_builtin_base_before_the_deck_pack() {
    let s = imported_hifi("minimal");
    let pins: Vec<String> = s.deck().widgets.keys().map(ToString::to_string).collect();
    assert_that!((pins, s.registry.get(kinds::base::SHAPE).is_some())).is_equal_to((vec!["base".into(), "deck".into()], true));
}

#[test]
fn retrofit_onto_a_library_pack_without_base_pins_base() {
    // A library pack `acme` (requires: [], one template copied from Essentiel's `texte`), then a retrofit import onto it.
    let s = imported_retrofit_on_new_pack("minimal", "acme");
    assert_that!(s.deck().widgets.contains_key(&kinds::base::namespace())).is_true();
    assert_that!(validate(s.deck(), &s.registry, None).is_empty()).is_true();
}

// src/templates_mode_tests.rs
#[test]
fn edit_templates_of_a_pack_without_base_can_add_a_base_text() {
    let mut s = session_on_pack_without_base("acme");
    s.edit_templates(pack_id(&s, "acme")).unwrap();
    let r = s.apply("Ajouter", vec![add_text_to_first_template(&s)], None);
    assert_that!(r.is_ok()).is_true();
}

#[test]
fn saved_pack_requires_the_namespaces_its_templates_use() {
    let mut s = session_on_pack_without_base("acme");
    s.edit_templates(pack_id(&s, "acme")).unwrap();
    s.apply("Ajouter", vec![add_text_to_first_template(&s)], None).unwrap();
    s.close_templates().unwrap();
    assert_that!(library_manifest(&s, "acme").requires).is_equal_to(vec![kinds::base::namespace()]);
}
```

Name the helpers after the real ones of each file (`imported_hifi`, `imported_retrofit_on_new_pack`, `session_on_pack_without_base`, `pack_id`, `add_text_to_first_template`, `library_manifest`, `close_templates`): read `import/mod_tests.rs`, `templates_mode_tests.rs` and `test_support.rs` first; add what is missing as small helpers over the public API. `validate(…)`'s real signature is in `oku_slides_core::validate`.

Regressions, each first seen failing for its stated reason (after Tasks 2–4, before Step 3):

```rust
// tests/regressions/issue_1039_new_deck_pins_base.rs
//! Un deck neuf écrivait son texte `base/text` sans épingler `base` : élément inconnu.
#[test]
fn issue_1039_new_deck_knows_its_base_text() {
    let (platform, _dir) = common::platform();
    let s = Session::new(common::app_fonts(), platform).unwrap();
    let text = s.registry().get(kinds::base::TEXT).map(|w| w.typography);
    assert_that!((s.deck().widgets.contains_key(&kinds::base::namespace()), text)).is_equal_to((true, Some(true)));
}

// tests/regressions/issue_1039_replicas_know_base_hosts.rs
//! Un replica écrit avec le registre du noyau ne connaissait pas `base/table` : tableau en valeur plate, fusion
//! cellule par cellule perdue (ADR-0004).
#[rstest]
#[case::new_deck(Open::New)]
#[case::opened_fixture(Open::Fixture)]
#[case::help(Open::Help)]
#[case::hifi_import(Open::Hifi)]
fn issue_1039_every_first_replica_knows_the_base_table_host(#[case] open: Open) {
    let s = open.session();
    let table = s.sources().registry().get(kinds::base::TABLE).and_then(|w| w.host_param(HostType::Table)).map(|p| p.name.clone());
    assert_that!(table).is_equal_to(Some("data".to_owned()));
}
```

(`Open::session()`: a small enum in the file building each case with the integration `tests/common` helpers — `Session::new`, `open(&fixture_path())`, `open_help("fr", None)`, a hi-fi import of `fixtures/import/minimal.pptx`; `Session::sources()` is the read accessor of `doc.sources` — use the existing one, or the snapshot path if none is public. `SourceBundle::registry()` is `core/src/sources.rs:184`.)

- [ ] **Step 2: Run, expect FAIL** — `cargo nextest run -p oku-slides-session -p oku-slides-i18n`.

- [ ] **Step 3: Implement**
  - `library_kind.rs`: append `oku_slides_render::base_pack_files()` to `BUILTIN_PACKS` after Essentiel and Grilles (same `StoredVersion::read`; an unreadable built-in is logged with its `RegistryError` and skipped, like the others); `library_tests.rs:16-23` checks `[DEFAULT_PACK, GRID_PACK, BASE_PACK]` and each built-in's own `created`.
  - `library.rs` `template_sets`: `.filter(|p| !p.templates.is_empty())` (or the manifest's), doc "Les packs qui ont des modèles : l'ajout de slide et l'import n'offrent qu'eux."
  - `deck_pack.rs`, empty-deck branch: `wanted = [AddPack base (current BASE_PACK), AddPack essentiel (current DEFAULT_PACK)]`, then the existing `closure_ops` loop, skipping an op whose namespace is already pinned; doc: "Un deck sans pack épingle `base` et le pack par défaut, et leurs besoins ; un deck qui épingle déjà un pack ne change pas."
  - Built-in pins (help, hi-fi, retrofit): one helper `fn builtin_pin(id: Uuid) -> Result<(Namespace, PackPin, PackFiles)>` in `deck_pack.rs` (from `builtin_versions()`, `.context(UnknownPackSnafu { id })` when absent).
    - `help.rs`: for `id in [BASE_PACK, DEFAULT_PACK]`, `builtin_pin(id)?`, `sources.add_pack(files)`, insert the pin; the help `SourceBundle` is built with `builtin_registry`.
    - `import/mod.rs:191`: the hi-fi target registry is `oku_slides_render::builtin_registry(fonts)`, and `HifiTarget.base` the built-in `base` pin.
    - `import/hifi.rs:73-87`: insert the built-in `base` pin and files into `deck.widgets` / `packs`, then the `deck` pin with `requires` = its manifest's, then `sources_with_packs` (the importer's own `AddPack base` op then finds it pinned: same pin, idempotent).
    - `import/retrofit.rs:72-87`: after `closure_with_files`, if `deck.widgets` lacks `base`, insert the built-in `base` pin and files, before `sources_with_packs`.
  - Templates mode: `edit_templates` (`:131-136`) inserts the user deck's `base` pin (or the built-in one) when the closure lacks it; the manifest save (`:221`) computes `requires` as above (`split_kind` over every kind of templates and examples, minus its own namespace, sorted, plus the previous `requires`).
  - In-process built-in: in `deck_widgets.rs` / `probed_widgets.rs`, a pin whose version equals the current built-in `base` version takes `base_pack_widgets()` and skips `worker.probe_pack`; when every pack of the set is that one, the registry is `builtin_registry` and no worker call is made. Any other set goes to the worker as today (it probes `base` then like any pack).
  - Replica / validation sites listed under Files → `builtin_registry`; the preview set of a non-`base` pack becomes `[base_pack_widgets(), pack]` and its registry `worker.registry_of(&set)`.
  - `tables.rs`: delete `CHART` / `TABLE`; targets `kinds::base::CHART` / `kinds::base::TABLE`; doc "le widget de base sous lequel on montre des données".
  - `param_fields.rs`:

```rust
/// La clé Fluent d'un widget du noyau ou du pack `base` ; `None` pour tout autre pack.
fn widget_key(kind: &str) -> Option<String> {
    match split_kind(kind) {
        None => Some(kind.to_owned()),
        Some((ns, name)) if kinds::base::ALL.contains(&kind) => Some(format!("{ns}-{name}")),
        Some(_) => None,
    }
}
```

    `kind_name`: its `label`, else `widget_key(kind).and_then(|k| l.try_tr(&format!("session-widget-kind-{k}")))`, else `kind`; `param_name` the same with `session-param-{k}-{param}`. `target_params.rs:5` doc names the new key format.
  - `views.rs` / `lib.rs`: `host_params` owned in `Snapshot`, from `self.registry.host_params()`.
  - Fluent renames: `for f in crates/oku-slides-i18n/src/locales/*/session.ftl; do sed -E -i '' 's/^session-(widget-kind|param)-(card|image|shape|stack|text)([- ])/session-\1-base-\2\3/' "$f"; done`, then add the chart/table keys and the `ui-element-kind` variants by hand, then `cargo nextest run -p oku-slides-i18n` (same keys, same order).
  - `crates/oku-slides-session/AGENTS.md`: "Pin base where element kinds are produced (new deck, import, help, templates mode), before probing; never complete a deck's pins on open. (plan:2026-10-18-packs-lot2-base)" and "Build a first replica or validate elements before the deck's packs resolve with builtin_registry; give a slot without a set core_registry. (plan:2026-10-18-packs-lot2-base)".

- [ ] **Step 4: Run, expect PASS** — `cargo nextest run -p oku-slides-session -p oku-slides-i18n`, both regressions and the two lazy-worker tests included.

- [ ] **Step 5: Commit** — `feat(session)! : pack intégré base épinglé à la création, à l'import, dans l'aide et l'atelier, registre intégré, noms traduits par kind qualifié (t-1039)`.

---

### Task 6: Shipped `base` versions are trusted by the fallback

**Files:**
- Modify: `crates/oku-slides-render-worker/src/client.rs:133-143` (`blame`), `src/client_tests.rs`, `crates/oku-slides-render-worker/ARCHITECTURE.md`, `crates/oku-slides-render-worker/AGENTS.md` (amended rule)
- Modify: `crates/oku-slides-session/src/deck_fallback.rs:55-125` (control render with trusted packs, bisection of the others), `src/deck_widgets.rs:130-160` (a trusted pack is never disabled on a probe failure), `src/deck_widgets.rs:61-70` (`Failpoints.control_render` doc), `src/deck_widgets_fallback_tests.rs` (adapted, below), `crates/oku-slides-session/AGENTS.md:32` (amended rule)
- Create: `crates/oku-slides-session/tests/regressions/issue_1040_heavy_base_content_disables_no_pack.rs` (declared in `main.rs`)

**Interfaces:**
- Consumes: `oku_slides_render::is_trusted_pack` (Task 2); in-process built-in resolution (Task 5).
- Produces: `blame(link, packs, e)` marks the slot's only untrusted pack when exactly one remains after removing the trusted ones; `Session::trusted` is not added (the session calls `is_trusted_pack` directly); `disable_faulty` bisects only untrusted packs, its control render keeps the trusted ones.

- [ ] **Step 1: Write the failing tests**

```rust
// crates/oku-slides-render-worker/src/client_tests.rs
#[test]
fn set_widgets_that_loops_beside_trusted_base_marks_the_looping_pack_faulty() {
    let w = worker(300);
    let base = oku_slides_render::base_pack_widgets().unwrap();
    let looping = PackWidgets::new("boucle", ns("boucle"), files(&[("boucle", LOOP_ON_IMPORT)])).unwrap();
    let mut s = slot(&w);
    let set = PackSet::new(vec![base.clone(), looping.clone()]).unwrap();
    let err = s.set_widgets(&set, core_registry(&Fonts::default()).unwrap()).unwrap_err();
    assert_that!((err.kind(), w.is_faulty(&looping), w.is_faulty(&base))).is_equal_to((RenderErrorKind::Timeout, true, false));
}

#[test]
fn a_bounded_failure_with_trusted_base_alone_marks_nothing_faulty() {
    let w = worker(1);
    let base = oku_slides_render::base_pack_widgets().unwrap();
    let mut s = slot(&w);
    let set = PackSet::new(vec![base.clone()]).unwrap();
    // 1 ms : même le pack `base` dépasse le délai ; il n'est pas accusé pour autant.
    let _ = s.set_widgets(&set, core_registry(&Fonts::default()).unwrap());
    assert_that!(w.is_faulty(&base)).is_false();
}
```

```rust
// crates/oku-slides-session/src/deck_widgets_fallback_tests.rs
#[test]
fn a_single_looping_user_pack_beside_the_builtins_is_disabled_and_base_kept() {
    let data = data_dir();
    let mut s = session_with_timeout(&data.0, LOOP_TIMEOUT_MS);
    let looping = user_pack(&s, "Boucle", &[("boucle", LOOP_ON_RENDER)]);
    s.apply("Pack", vec![pin_pack(&looping), add_loop(&looping)], None).unwrap();
    let disabled: Vec<String> = s.widgets_disabled.keys().map(ToString::to_string).collect();
    assert_that!(disabled).is_equal_to(vec![namespace_of_pack(&looping)]);
    assert_that!(s.registry.get(kinds::base::TEXT).is_some()).is_true();
}

#[test]
fn an_edited_base_that_loops_is_not_trusted_and_is_disabled() {
    let data = data_dir();
    let mut s = session_with_timeout(&data.0, LOOP_TIMEOUT_MS);
    // Une version modifiée de `base` (même id, un widget qui boucle) épinglée sous `base`.
    let edited = edited_base_pack(&s, LOOP_ON_RENDER);
    s.apply("Pack", vec![pin_pack(&edited), add_base_text()], None).unwrap();
    assert_that!(s.widgets_disabled.contains_key(&kinds::base::namespace())).is_true();
}
```

(Before Step 3 the first test passes only with two packs or more; with `base` resolved in-process (Task 5) and `[base, boucle]` slots, it fails because `blame` accuses nobody — the reason to fix. `namespace_of_pack`, `edited_base_pack`, `add_base_text`: small helpers beside `user_pack` / `pin_pack` in `deck_widgets_tests.rs`. Existing tests of this file assume a slot holding the looping pack alone: `assert_fell_back` (`:101-106`) becomes "only the trusted packs remain and the registry has no `boucle/*`" (`s.packs.packs().iter().all(is_trusted_pack)`), and `fault_pack_outside_the_deck` builds its set with `base` + the looping pack; adapt each test that compares `s.registry` with the core registry the same way.)

Regression `tests/regressions/issue_1040_heavy_base_content_disables_no_pack.rs`, first seen failing on today's control render (without packs, the elements become cheap "unknown" boxes, the control succeeds, bisection blames `base`):

```rust
//! Un deck dont le contenu dépassait le délai faisait écarter `base` et Essentiel : tout devenait inconnu.
#[test]
fn issue_1040_deck_too_heavy_with_the_builtins_alone_disables_no_pack() {
    let (platform, _dir) = common::platform_with(|s| s.render.timeout_ms = 100);
    let mut s = Session::new(common::app_fonts(), platform).unwrap();
    // 40 slides, chacune un tableau `base/table` de 100 × 20 cellules : bien plus de 100 ms.
    let err = s.apply("Lourd", heavy_table_slides(40, 100, 20), None).and_then(|_| s.render_current());
    assert_that!(err.err().and_then(|e| e.render_error().map(|r| r.kind().is_bounded()))).is_equal_to(Some(true));
    assert_that!(s.widgets_disabled().is_empty()).is_true();
}
```

(`heavy_table_slides`: `AddSlide` ops of `content` slides with one `base/table` block each; `render_current` / `widgets_disabled()`: the session's real render entry point and accessor for the integration tests — read `tests/common/mod.rs` and `lib.rs`; if the bounded error is reported as a notice rather than returned, assert the notice key instead. If 40 × 100 × 20 renders under 100 ms on the machine, raise the counts until the today-form fails.)

- [ ] **Step 2: Run, expect FAIL** — `cargo nextest run -p oku-slides-render-worker -p oku-slides-session deck_widgets_fallback issue_1040`.

- [ ] **Step 3: Implement**
  - `client.rs`:

```rust
/// Marque fautif l'unique pack non intégré de l'emplacement si `e` est un échec borné.
///
/// Une version livrée de `base` n'est jamais accusée (`is_trusted_pack`) ; avec
/// plusieurs autres packs, aucun ne l'est : la session isole le coupable.
pub(crate) fn blame(link: &mut Link, packs: &[WirePack], e: &RenderError) {
    if !e.kind().is_bounded() {
        return;
    }
    let mut untrusted = packs.iter().filter(|p| !is_trusted_wire(p));
    if let (Some(pack), None) = (untrusted.next(), untrusted.next()) {
        link.faulty_packs.insert(fingerprint_hex(pack));
    }
}
```

    (`is_trusted_wire`: `is_trusted_pack` of the wire pack's namespace and id, without rebuilding a `PackWidgets`; or rebuild it with `pack_of` if that is how `client.rs` already handles a `WirePack`.)
  - `deck_fallback.rs` `disable_faulty`: `let (trusted, others): (Vec<_>, Vec<_>) = self.packs.packs().iter().cloned().partition(is_trusted_pack);` — if `others` is empty, `Ok(false)` (the error is the deck's); if no pack is faulty yet, the control render is `fails_with(&trusted, &[], deck)`: true → `Ok(false)`; else `isolate_faulty(&trusted, others)`, each trial rendering `trusted` + candidates, the single-candidate check running even when `others.len() == 1`. `fails_with(base, candidates, deck)`; the failpoint fires when `candidates` is empty. Module doc: trusted packs are never blamed nor disabled.
  - `deck_widgets.rs` resolution: a probe failure of a trusted pack is returned as the error, never recorded in `disabled`.
  - `crates/oku-slides-render-worker/AGENTS.md`: the settled rule becomes "Keep widget fallback policy in the session; the render client only flags faulty pack fingerprints, never a shipped base version. (plan:2026-10-18-packs-lot2-base)".
  - `crates/oku-slides-session/AGENTS.md:32`: the bisection rule becomes "Bisect only untrusted packs, the control render keeping the shipped base; never disable a trusted pack. (plan:2026-10-18-packs-lot2-base)".
  - `crates/oku-slides-render-worker/ARCHITECTURE.md`: `blame` and trusted packs; ≤ 50 lines.

- [ ] **Step 4: Run, expect PASS** — `cargo nextest run -p oku-slides-render-worker -p oku-slides-session`.

- [ ] **Step 5: Commit** — `fix(session) : le repli par pack n'accuse jamais une version livrée de base, témoin rendu avec elle (t-1040)`.

---

### Task 7: CLI, MCP and agent instructions on qualified base kinds — workspace green

**Files:**
- Modify: `crates/oku-slides-mcp/src/tools.rs` (`widgets` description: "the base widgets are `base/<name>`; a deck's other widgets come from its packs, `namespace/name`"), `tests/surfaces.rs:651-659`, `tests/edit.rs:104`, other MCP tests with bare element kinds
- Modify: `crates/oku-slides-agent/src/instructions/widgets.md` (`add_element` kind = the qualified widget name, e.g. `base/text`), agent and local-agent tests with bare element kinds
- Modify: `crates/oku-slides-cli/src/bench.rs:36,71` (`Renderer::with_builtins`), `src/pack_render.rs:89-95` (the `local` branch stays as is: a deck without packs renders with the core registry, exactly as the session's empty set does — `adopt_default_packs` makes it unreachable for decks opened through `sources_with_packs`; say so in its doc), `tests/{cli.rs,import.rs}` and CLI tests with bare kinds or version-6 decks, `src/library_tests.rs` (builtins include `base`)
- Modify: `src-tauri` tests with bare element kinds, if any (`rg -n '"kind": "(text|card|image|shape|chart|table|stack)"' src-tauri`)

**Interfaces:**
- Consumes: everything above.
- Produces: the whole Rust workspace green.

- [ ] **Step 1: Write the failing tests**

```rust
// crates/oku-slides-mcp/tests/surfaces.rs (replaces widgets_tool_lists_card_with_its_params)
#[tokio::test(flavor = "multi_thread")]
async fn widgets_tool_lists_base_card_with_its_params() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let widgets = json_of(&call(&c, "widgets", json!({})).await);
    let card = &widgets["widgets"][kinds::base::CARD];
    let params: Vec<&str> = card["params"].as_array().unwrap().iter().map(|p| p["name"].as_str().unwrap()).collect();
    assert_that!(params).is_equal_to(vec!["kicker", "title", "body", "highlight"]);
}

// crates/oku-slides-cli/tests/cli.rs (with the file's `run`)
#[test]
fn open_out_writes_the_base_pack_beside_the_deck() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("deck");
    run(&["open", common::roadmap_path().to_str().unwrap(), "--out", out.to_str().unwrap()]).success();
    let namespaces: Vec<String> = std::fs::read_dir(out.join("widgets")).unwrap()
        .map(|e| std::fs::read_to_string(e.unwrap().path().join("widgets.json")).unwrap())
        .map(|j| serde_json::from_str::<serde_json::Value>(&j).unwrap()["namespace"].as_str().unwrap().to_owned())
        .collect();
    assert_that!(namespaces).contains("base".to_owned());
}
```

(Adapt `run(...)` and the `open --out` argument shape to `cli.rs`'s existing open tests.) `bench_measures_every_step_and_round_trips` (`cli/tests/cli.rs:197`) must pass again after `bench.rs` switches to `Renderer::with_builtins`.

- [ ] **Step 2: Run, expect FAIL** — `cargo nextest run -p oku-slides-mcp -p oku-slides-cli -p oku-slides-agent -p oku-slides-local-agent`.

- [ ] **Step 3: Implement** the updates listed under Files.

- [ ] **Step 4: Run, expect PASS** — `cargo nextest run --workspace --exclude oku-slides-app` and `just check` (UI may still fail on `api.ts`, fixed next).

- [ ] **Step 5: Commit** — `feat(mcp) : outils, CLI et consignes de l'agent sur les kinds du pack base (t-1041)`.

---

### Task 8: UI — qualified base kinds, panels chosen by host type

**Files:**
- Create: `ui/src/baseKinds.ts` (+ `baseKinds.test.ts`), `ui/src/hostParams.ts` (+ `hostParams.test.ts`)
- Delete: `ui/src/ownPanel.ts`, `ui/src/ownPanel.test.ts` (folded into `hostParams.ts`)
- Modify: `ui/src/api.ts` (`HostType`, `HostParams`, `Snapshot.host_params`), `ui/src/slideTools.ts:6-20`, `ui/src/newElement.ts`, `ui/src/useSlideOps.ts:47-56`, `ui/src/useClipboard.ts:58,93,113-118`, `ui/src/widgetChoices.ts:18-37`, `ui/src/WidgetPicker.tsx:151`, `ui/src/icons.tsx:17` (comment), `ui/src/stageFields.ts:41-45`, `ui/src/Stage.tsx:390,406-418` (open-widget tag label), `ui/src/TargetPanel.tsx:21-37,77-99` (`hostParams` prop), `ui/src/App.tsx:219,427,497,509`, `ui/src/useTargetSettings.ts:27-40`, `ui/src/packOps.ts:26-29` (`firstPackId` skips `base`), `ui/src/chart/{chartDeck.ts,ChartPanel.tsx:46,67,75,SeriesColors.tsx}`, `ui/src/table/{tableDeck.ts:35-42,TablePanel.tsx:36,70}`
- Modify tests: `deck.test.ts` (31 kind hits; `canvas` at 101, 311, 319, 365 → `base/stack`: `deck.ts` reads slot shapes, not kinds), `newElement.test.ts`, `slideTools.test.ts`, `widgetChoices.test.ts`, `packOps.test.ts`, `useClipboard` tests if any, `TargetPanel.render.test.tsx`, `useTargetSettings.render.test.tsx` (`snapshot()` gains `host_params`), `Stage.render.test.tsx` (`:217`: a selected base element now shows `button.tag-open`; keep `:494` for a non-pack element's `span.tag`), `WidgetPicker.render.test.tsx`, `chart/chartDeck.test.ts` (`:26-30` becomes "missing or non-object param → null", with an explicit `param`), `chart/ChartPanel.render.test.tsx` (`mount` gains `kind`, `param` and an `applied` collector, like `TablePanel.render.test.tsx:30-37`), `table/tableDeck.test.ts` (`:13` likewise), `table/TablePanel.render.test.tsx`, `agent/useAgentFocus.render.test.tsx`, `stageFields.test.ts`, `agent/ui.test.ts`, `editorGeometry.render.test.tsx`, `HostSettings.render.test.tsx` — not tests where `chart` / `card` is a pack-widget *file* name or a list-column kind
- Modify e2e: `ui/e2e/010-editor.cjs:125` (`kind: 'base/table'`), `ui/e2e/150-target-panel.cjs:46` (`'deck/frise'`), `ui/e2e/220-import.cjs:190-206` (read the `deck` pin by namespace, not `Object.values(...)[0]`); `020` labels stay (`base/image` and `base/stack` filtered)
- Modify: `ui/AGENTS.md` (rules)

**Interfaces:**
- Consumes: `Snapshot.host_params` (Task 5); `template_sets` without `base` (Task 5).
- Produces:
  - `api.ts`: `export type HostType = "chart" | "table" | "path"; export type HostParams = Record<string, Partial<Record<HostType, string>>>;` `Snapshot.host_params: HostParams` (one definition, imported by `hostParams.ts`).
  - `baseKinds.ts`: `BASE_NAMESPACE = "base"`, `BASE` (name → kind), `type BaseName`, `baseName(kind: string): BaseName | null`.
  - `hostParams.ts`: `export type OwnPanel = { panel: Exclude<HostType, "path">; param: string }; export function ownPanel(params: HostParams, kind: string | undefined): OwnPanel | null` (chart first, then table; `path` has no panel; own properties only). `hostParam` stays module-private (tested through `ownPanel`).
  - `slideTools.ts`: `ADD_NAMES = ["text", "card", "shape", "chart", "table"] as const satisfies readonly BaseName[]`, `type AddName = (typeof ADD_NAMES)[number]` (replaces `ADD_KINDS` / `AddKind`); `packOnlyWidgets` drops every widget with `baseName(w.name) !== null`; `canAdd(name: AddName | "widget", slide)` keeps today's rule on `"text"`.
  - `newElement(name: AddName, id)` returns `kind: BASE[name]`; ids `newId(name[0])`; Fluent selects and `Icon[name]` take the `AddName`; `kindLabel(kind)` passes `baseName(kind) ?? kind`.
  - Chart helpers take `(el: Element, param: string)` (`chartSource`, `chartKind`, `checkboxColumns`, `stackable`, `isStacked`, `stackPatch`, `seriesNames`, `seriesColor`, `colorPatch`; `optionOf(el, param)` without a kind test); `tableOption(el, param)` keeps its shape check, without a kind test; `ChartPanel` / `SeriesColors` / `TablePanel` take `param: string` and use it in every op.

- [ ] **Step 1: Write the failing tests** (French `describe` / `test` names, like the neighbouring files)

```ts
// ui/src/baseKinds.test.ts
import registry from "../../fixtures/widgets/base-registry.json";

describe("baseName", () => {
  test("d'un kind de base rend son nom", () => expect(baseName("base/card")).toBe("card"));
  test("d'un autre espace de noms rend null", () => expect(baseName("acme/card")).toBeNull());
  test("d'un kind nu rend null", () => expect(baseName("text")).toBeNull());
  test("d'un nom inconnu de base rend null", () => expect(baseName("base/")).toBeNull());
});

test("BASE a les widgets du pack base du registre figé", () => {
  const pack = registry.packs.find((p) => p.namespace === BASE_NAMESPACE);
  const names = (pack?.widgets ?? []).map((w) => `${BASE_NAMESPACE}/${w.name}`).sort();
  expect(names).toEqual(Object.values(BASE).sort());
});

// ui/src/hostParams.test.ts
const params: HostParams = { "base/chart": { chart: "option" }, "acme/plot": { chart: "series", table: "grid" }, "base/shape": { path: "path" }, "acme/grid": { table: "grid" } };

describe("ownPanel", () => {
  test("d'un graphique de pack ouvre le panneau graphique sur son paramètre", () =>
    expect(ownPanel(params, "acme/plot")).toEqual({ panel: "chart", param: "series" }));
  test("d'un tableau de pack ouvre le panneau tableau sur son paramètre", () =>
    expect(ownPanel(params, "acme/grid")).toEqual({ panel: "table", param: "grid" }));
  test("d'une forme n'ouvre aucun panneau", () => expect(ownPanel(params, "base/shape")).toBeNull());
  test("d'un kind absent n'ouvre aucun panneau", () => expect(ownPanel(params, "base/text")).toBeNull());
  test("sans élément n'ouvre aucun panneau", () => expect(ownPanel(params, undefined)).toBeNull());
  test.each(["constructor", "__proto__", "toString"])("de %s n'ouvre aucun panneau", (k) => expect(ownPanel(params, k)).toBeNull());
});

// ui/src/chart/chartDeck.test.ts
test("chartSource d'un graphique de pack lit son paramètre", () => {
  const el = { id: "c", kind: "acme/plot", params: { series: { dataset: { source: [["x"], [1]] } } } };
  expect(chartSource(el, "series")).toEqual([["x"], [1]]);
});
test.each(["__proto__", "constructor"])("chartSource d'un paramètre %s absent rend null", (param) => {
  expect(chartSource({ id: "c", kind: "acme/plot", params: {} }, param)).toBeNull();
});

// ui/src/chart/ChartPanel.render.test.tsx — `mount(series, onShowAs, dataset, { kind, param })` now returns
// `{ host, applied }` and passes `param` to the panel and `element.params[param]` as the option.
test("le panneau d'un graphique de pack écrit dans son paramètre", async () => {
  const { host, applied } = await mount([{ type: "bar" }, { type: "bar" }], undefined, undefined, { kind: "acme/plot", param: "series" });
  await act(async () => (host.querySelector(".chart-stack button") as HTMLButtonElement).click());
  expect(applied[0].ops[0]).toMatchObject({ op: "set_chart_option", param: "series" });
});

// ui/src/table/TablePanel.render.test.tsx — `mount(data, onShowAs, numberLang, { kind, param })`
test("le panneau d'un tableau de pack écrit dans son paramètre", async () => {
  const { host, applied } = await mount(undefined, undefined, undefined, { kind: "acme/grid", param: "grid" });
  await act(async () => options(host)[2].click());
  expect(applied[0].ops[0]).toEqual({ op: "set_table_option", target, param: "grid", patch: { striped: true } });
});

// ui/src/TargetPanel.render.test.tsx — `mount` passes `hostParams` (default: base chart and table, plus acme/plot)
const plot = { id: "p1", kind: "acme/plot", params: { series: { dataset: { source: [["C", "S"], ["1", 3]] }, series: [{ type: "bar" }] } } } as unknown as Element;
const text = { id: "x1", kind: "base/text", params: {} } as unknown as Element;

test("TargetPanel élément de pack à hôte graphique montre ChartPanel", async () => {
  const host = await mount({ selected: plot });
  expect(host.querySelector(".chart-panel")).not.toBeNull();
});

test("TargetPanel texte de base montre les réglages, sans panneau propre", async () => {
  const host = await mount({ selected: text });
  expect((host.querySelector(".chart-panel") ?? host.querySelector(".table-panel"))).toBeNull();
  expect(host.querySelector(".host-settings")).not.toBeNull();
});

// ui/src/useTargetSettings.render.test.tsx — the hook's settings are off exactly when TargetPanel shows its own panel
// (`snapshot()` gains `host_params: { "acme/plot": { chart: "series" }, "base/chart": { chart: "option" }, "base/table": { table: "data" } }`;
// the existing "chart/table selected sends no request" tests pass `kind: "base/chart"` / `"base/table"`.)
test.each([["acme/plot", 0], ["base/text", 1]])("useTargetSettings %s selected asks %i surface", async (kind, n) => {
  const { fake, asked } = asking();
  await mountTarget(fake, { snap: snapshot(), current: "s-ch1", element: "x1", kind });
  expect(asked).toHaveLength(n);
});

// ui/src/slideTools.test.ts
test("packOnlyWidgets écarte tous les widgets de base", () => {
  const w = (name: string): AddWidget => ({ name, label: name, description: "" });
  expect(packOnlyWidgets([w("base/card"), w("base/image"), w("base/stack"), w("acme/card")]).map((x) => x.name)).toEqual(["acme/card"]);
});

// ui/src/useClipboard tests (or slideTools.test.ts if paste has none): the paste rule
test("un texte de base copié ne se colle pas sur une slide vide", () => {
  expect(canAdd(baseName("base/text") === "text" ? "text" : "widget", "blank")).toBe(false);
});

// ui/src/packOps.test.ts
test("firstPackId saute base pour essentiel", () => expect(firstPackId(snapWith({ base: pin("b"), essentiel: pin("e") }))).toBe("e"));
test("firstPackId saute base pour le pack du deck", () => expect(firstPackId(snapWith({ base: pin("b"), deck: pin("d") }))).toBe("d"));

// ui/src/widgetChoices.test.ts
test("le filtre « base » ne retient aucune carte native", () => {
  expect(filterChoices(widgetChoices([]), "base")).toEqual([]);
});

// ui/src/newElement.test.ts
test("newElement d'un texte est un texte de base", () => expect(newElement("text", "t1").kind).toBe("base/text"));
```

`snapWith` / `pin` in `packOps.test.ts` beside the existing snapshot builder; adapt `filterChoices` to the real search function of `widgetChoices.ts:51`. The JSON import works with Bun's `resolveJsonModule`; if `tsconfig` refuses it, read the file with `Bun.file(...).json()` in the test.

- [ ] **Step 2: Run, expect FAIL** — `cd ui && bun test --path-ignore-patterns='**/*.render.test.tsx' baseKinds hostParams chartDeck slideTools newElement packOps widgetChoices && bun test ChartPanel.render TablePanel.render TargetPanel.render useTargetSettings.render` (the second process runs the happy-dom render tests, as `ui/package.json` does).

- [ ] **Step 3: Implement**
  - `hostParams.ts`:

```ts
// Paramètres d'hôte des widgets d'élément : quel panneau ouvrir, sur quel paramètre.

import type { HostParams, HostType } from "./api";

/** Le paramètre d'hôte `host` du widget `kind`, ou null (propriétés propres seulement). */
function hostParam(params: HostParams, kind: string, host: HostType): string | null {
  if (!Object.hasOwn(params, kind)) return null;
  const hosts = params[kind];
  return Object.hasOwn(hosts, host) ? (hosts[host] ?? null) : null;
}

/** Un panneau propre et le paramètre qu'il lit et écrit. */
export type OwnPanel = { panel: Exclude<HostType, "path">; param: string };

/** Le panneau propre d'un élément : graphique, sinon tableau ; null pour les autres. */
export function ownPanel(params: HostParams, kind: string | undefined): OwnPanel | null {
  if (kind === undefined) return null;
  const chart = hostParam(params, kind, "chart");
  if (chart !== null) return { panel: "chart", param: chart };
  const table = hostParam(params, kind, "table");
  return table !== null ? { panel: "table", param: table } : null;
}
```

  - `TargetPanel.tsx` gets `hostParams: HostParams` (from `App.tsx:427`, `snap.host_params`); `cardOf`: `const own = ownPanel(hostParams, selected?.kind);` then `own?.panel === "chart"` → `<ChartPanel … param={own.param} />`, `"table"` → `<TablePanel … param={own.param} />`; delete the "must stay in sync" comment. `useTargetSettings.ts:40`: `ownPanel(snap?.host_params ?? {}, element?.kind) === null` (`snap` may be null before the first snapshot; that is the only fallback).
  - `chartDeck.ts`:

```ts
const optionOf = (el: Element, param: string): Option | null => {
  const params = el.params ?? {};
  const option = Object.hasOwn(params, param) ? params[param] : null;
  return typeof option === "object" && option !== null ? (option as Option) : null;
};
```

    and every exported helper gains `param: string` after `el`; `tableOption(el, param)` likewise (`Object.hasOwn`, shape check kept). `ChartPanel` / `SeriesColors` / `TablePanel` pass their `param` prop to the helpers and to every op (the `param: "option"` / `param: "data"` literals go).
  - `baseKinds.ts`:

```ts
// Kinds du pack de widgets intégré `base` ; mêmes valeurs que `oku_slides_core::kinds::base`.

/** L'espace de noms du pack `base`. */
export const BASE_NAMESPACE = "base";
const PREFIX = `${BASE_NAMESPACE}/`;

/** Les widgets du pack `base`, par nom. */
export const BASE = {
  text: "base/text", card: "base/card", image: "base/image", shape: "base/shape",
  chart: "base/chart", table: "base/table", stack: "base/stack",
} as const;

/** Le nom d'un widget de `base`. */
export type BaseName = keyof typeof BASE;

/** Le nom de base d'un kind `base/<nom>`, ou null. */
export function baseName(kind: string): BaseName | null {
  const name = kind.startsWith(PREFIX) ? kind.slice(PREFIX.length) : "";
  return Object.hasOwn(BASE, name) ? (name as BaseName) : null;
}
```

  - `slideTools.ts`, `newElement.ts`, `useSlideOps.ts`, `widgetChoices.ts` (only the choice `key` becomes `BASE[name]`; its searchable `name` stays the bare `AddName`), `WidgetPicker.tsx`, `App.tsx:219,509`: the `AddName` interfaces above; `useClipboard.ts:93`: `canAdd(baseName(element.kind) === "text" ? "text" : "widget", view.slide.kind)`, `:113-118`: `canAdd("text", …)` / `addElement("text", …)` unchanged (`AddName`).
  - `packOps.ts` `firstPackId`: `pins.find(([ns]) => ns !== "deck" && ns !== BASE_NAMESPACE) ?? pins.find(([ns]) => ns !== BASE_NAMESPACE) ?? pins[0]`.
  - `Stage.tsx:406-418`: the open-widget tag's `aria-label` / `title` reuse the loop's `label` (`kindLabel(tg.kind)`, `:390`).
  - `ui/AGENTS.md`: "Choose an element's own panel and its param from snapshot.host_params, never from its kind. (plan:2026-10-18-packs-lot2-base)" and "Change oku_slides_core::kinds::base and ui/src/baseKinds.ts together; a test compares BASE with the registry fixture. (plan:2026-10-18-packs-lot2-base)".

- [ ] **Step 4: Run, expect PASS** — `just test` (Rust + UI) and `cd ui && bunx tsc --noEmit`.

- [ ] **Step 5: Commit** — `feat(ui)! : kinds qualifiés du pack base, panneaux et paramètres choisis par type d'hôte (t-1042)`.

---

### Task 9: Full gate and consolidation

**Files:**
- Modify only what the gate or the consolidation turns up (docs and rules already landed with their code).

- [ ] **Step 1:** `just ci` → green (fmt, clippy, tsc, fonts-lfs, agents-lint, tests, audit).
- [ ] **Step 2:** `just e2e 4999` (or another free port) → green; fix a spec reading a bare kind or the first pin with the smallest change under `ui/e2e/`.
- [ ] **Step 3:** Consolidation (G10):
  - `rg -n '"(text|card|image|shape|chart|table|stack)"' crates ui/src typst --glob '!*tests*' --glob '!*.test.*'` and `rg -n 'canvas' crates ui/src typst docs/aide fixtures` → only non-kind uses (A2UI component ids, list-column kinds, source item kinds, token groups, ECharts keys, the HTML canvas) may remain; list them in the report.
  - `rg -n 'kind ?(==|in) ?\(?"' typst/oku-slides/src crates/oku-slides-render/src` → no element-kind comparison (the `content.typ` list-kind and `core.typ` hits are not element kinds).
  - `rg -n 'base_registry|base_probe' crates` → none; `rg -n 'param: "(option|data)"' ui/src --glob '!*.test.*'` → none; `rg -n 'ADD_KINDS|AddKind|OWN_PANEL' ui/src` → none.
  - `typst/oku-slides/packs/base/versions.txt` lists the final `base` version (the test of Task 2 enforces it after any late change to a `base` file).
  - Every `ARCHITECTURE.md` touched ≤ 50 lines; `just agents-lint` green.
- [ ] **Step 4: Commit** (if anything changed) — `chore(packs) : passe de consolidation du lot 2 (t-1043)`.
