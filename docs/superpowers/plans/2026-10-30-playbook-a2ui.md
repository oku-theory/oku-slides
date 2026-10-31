# A2UI playbook Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One Rust source of examples for every A2UI catalog v4 component feeds a dev-only
gallery page, the `render` tool's component list (generated from the catalog's JSON schema)
and a new `a2ui_examples` MCP tool.

**Architecture:** `oku-slides-a2ui` derives `JsonSchema` on the catalog and gains a `playbook`
module: typed examples (`examples()`), an agent reference built from the schema filtered by
what the agent examples use (`agent_reference()`, `render_components()`). A test writes the
examples as v0.9 messages to `ui/src/a2ui/fixtures/playbook.json`; the UI page
`ui/playbook.html` renders them through `SurfaceView`. `oku-slides-mcp` serves the reference
as `a2ui_examples` and injects the list into `render`'s description.

**Tech Stack:** Rust (schemars 1.2, serde_json with `preserve_order`, snafu 0.9, speculoos,
rstest, insta), React 19 + Vite 7 + Bun tests (happy-dom), `@a2ui/web_core` 0.14.

**Spec:** `docs/superpowers/specs/2026-10-30-playbook-a2ui-design.md` (task t-1312). Read it
before any task; decision 10 (agent = `render` accepts it) drives Tasks 2 to 4. Its section
"Planning revisions" lists where this plan refines the first draft (the `a2ui_tools` module,
four `ToolError` variants, the cached reference, `ComponentKind::is_field`…).

## Global Constraints

- Catalog id stays `urn:oku-slides:a2ui:catalog:v4`; no `ComponentKind` change in this plan.
- No `unwrap`/`expect` outside tests, no silent fallback; `std::fs` only in tests.
- Errors: Snafu, a backtrace in every variant, opaque public types.
- Every `pub` item and new module documented, first sentence under 15 words; English only;
  French doc comments met on edited types are translated in the same commit.
- Dependencies: `workspace.dependencies` + `.workspace = true`; never thiserror or chrono.
- Source files target 500 lines, never above 1000; Rust unit tests in a sibling `x_tests.rs`
  included by `#[cfg(test)] #[path = "x_tests.rs"] mod tests;`, asserting with speculoos.
- Test names `unit_scenario_result`, one behavior per test, rstest for table cases.
- Every new Fluent key in all 20 locales, same key order, a real translation (never English
  copied); code tokens such as `render`, `show_params` stay untranslated.
- `ARCHITECTURE.md` files stay at most 50 lines.
- Commits: Conventional Commits, `type(scope): description (t-1312)`, message written to a
  file in the scratchpad and passed with `git commit -F`, files staged with a separate
  `git add`. No `Co-Authored-By` line (user instruction).
- Runs outside `just e2e` set `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS` to a fresh
  temp dir. The first nextest build of this worktree is cold (over 10 min): run it in the
  background and wait for it.

## File map

| File | Task | Responsibility |
|---|---|---|
| `crates/oku-slides-a2ui/Cargo.toml` | 3 | `snafu`, dev `insta`, `rstest` (Task 1 adds nothing: `schemars` is already a dependency) |
| `crates/oku-slides-a2ui/src/{component,props,action,data,list}.rs` | 1 | `JsonSchema` derives (not on `Component`) |
| `crates/oku-slides-a2ui/src/catalog_schema_tests.rs` | 1 | test-only: the schema's component names, shared by the crate's tests |
| `crates/oku-slides-a2ui/src/component.rs` | 2 | `ComponentKind::is_field` |
| `crates/oku-slides-a2ui/src/playbook.rs` | 2 | `Example`, `Group`, `examples()`, `kind_info` |
| `crates/oku-slides-a2ui/src/playbook/entries.rs`, `host_entries.rs` | 2 | the example data (agent-capable kinds, host-only kinds) |
| `crates/oku-slides-a2ui/src/playbook_tests.rs` | 2 | structure tests, UI fixture writer |
| `crates/oku-slides-a2ui/src/playbook/reference.rs` | 3 | schema → `ComponentReference` (cached), `render_components` |
| `crates/oku-slides-a2ui/src/playbook/reference_tests.rs` | 3 | cleaning, faults, exact list, snapshot |
| `crates/oku-slides-session/src/agent/render.rs` (+ `render_tests.rs`) | 2 | `is_field` in `check_available`; `agent == render accepts` |
| `crates/oku-slides-mcp/src/a2ui_tools.rs` (+ `_tests.rs`) | 4 | `a2ui_examples` |
| `crates/oku-slides-mcp/src/{tools,handler,tool_error,lib}.rs` | 4 | listing, dispatch, errors |
| `crates/oku-slides-i18n/src/locales/*/agent.ftl` | 4 | three error keys |
| `crates/oku-slides-local-agent/src/families.rs` (+ `compact_tests.rs`) | 4 | the tool's family; compacted descriptions keep the list |
| `ui/playbook.html`, `ui/src/playbook/*` | 5 | the gallery page and its tests |
| `ui/AGENTS.md`, root `AGENTS.md` | 5 | the English-literals exception; the Commands line |
| `ui/src/a2ui/fixtures/playbook.json` | 2 (generated) | examples as v0.9 messages |

---

### Task 1: Catalog JSON schema

**Files:**
- Modify: `crates/oku-slides-a2ui/src/component.rs` (derive on `ComponentKind` only)
- Modify: `crates/oku-slides-a2ui/src/props.rs`, `action.rs`, `data.rs`, `list.rs` (derives)
- Create: `crates/oku-slides-a2ui/src/catalog_schema_tests.rs` (test-only, declared in `lib.rs`)

`Component` gets no derive: the schema read is `ComponentKind`'s, and `Component` is only
serialized. `schemars` is already a dependency of the crate (`ChoiceOption` derives it): no
`Cargo.toml` change in this task. The test lives in its own file because
`component_tests.rs` is already at 534 lines.

**Interfaces:**
- Produces: `schemars::JsonSchema` implemented by `ComponentKind` and every type its fields
  reach. `schema_for!(ComponentKind)` is a `oneOf` with one branch per variant in declaration
  order, each branch naming its component in `properties.component.const` (schemars may emit
  `enum: ["Name"]` instead: the helper reads both).
- Produces (test-only): `crate::catalog_schema_tests::{CATALOG, catalog()}`: the 25 names, and
  the names read from the schema in order. Task 2's tests use `catalog()`; Task 3 makes it
  call the production `branch_name`.

- [ ] **Step 1: Write the failing test** (`src/catalog_schema_tests.rs`)

```rust
//! The catalog schema's component names, shared by the crate's tests.

use speculoos::prelude::*;

use crate::ComponentKind;

/// The 25 catalog components, in `ComponentKind` declaration order.
pub(crate) const CATALOG: [&str; 25] = [
    "Column", "Card", "Text", "TextField", "RichTextField", "Stepper", "Row", "Button", "ChoiceChips", "Slider",
    "ColorField", "Select", "ListTable", "SlideThumb", "ThemeChoice", "DiffText", "Prompt", "Timeline", "Receipt",
    "Disclosure", "Badge", "Markdown", "Notice", "ChartData", "TableData",
];

/// The component name a `oneOf` branch carries (`const`, or a one-value `enum`).
fn branch_name(branch: &serde_json::Value) -> Option<&str> {
    let tag = branch.get("properties")?.get("component")?;
    tag.get("const").and_then(serde_json::Value::as_str).or_else(|| tag.get("enum")?.get(0)?.as_str())
}

/// The component names of the `ComponentKind` schema, in branch order.
pub(crate) fn catalog() -> Vec<String> {
    let schema = serde_json::to_value(schemars::schema_for!(ComponentKind)).unwrap();
    schema["oneOf"].as_array().unwrap().iter().map(|b| branch_name(b).unwrap().to_owned()).collect()
}

#[test]
fn component_kind_schema_has_one_branch_per_component_in_order() {
    assert_that!(catalog()).is_equal_to(CATALOG.map(str::to_owned).to_vec());
}
```

In `lib.rs`, after the other test modules (or after the `mod` lines if there are none):

```rust
#[cfg(test)]
#[path = "catalog_schema_tests.rs"]
mod catalog_schema_tests;
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo nextest run -p oku-slides-a2ui component_kind_schema` (background on a cold build)
Expected: compile error, `ComponentKind: JsonSchema` is not satisfied.

- [ ] **Step 3: Derive the schema**

Add `schemars::JsonSchema` to the `#[derive(...)]` of: `ComponentKind` (`component.rs`);
`Tone`, `StepStatus`, `PromptImage`, `TimelineStep`, `ToolCallDetail`, `TextVariant`,
`ButtonIcon`, `SlideRef`, `DiffSegment`, `DiffKind` (`props.rs`; `ChoiceOption` already has
it); `Dynamic<T>`, `Action`, `Event` (`action.rs`); `SeriesSwatch` (`data.rs`); `ListColumn`,
`SubList`, `SubListRaw`, `SubColumn`, `ColumnKind` (`list.rs`). Example:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "component")]
pub enum ComponentKind {
```

`SubList` carries `#[serde(try_from = "SubListRaw")]`: schemars 1.2 then reads the schema of
`SubListRaw` (its `try_from` source), so the derive on `SubListRaw` is required, and the
docs the agent would read are `SubListRaw`'s. Copy each field doc comment of `SubList` onto
the same field of `SubListRaw`, and give `SubListRaw` the doc comment
`/// A nested list as it arrives, checked into a [`SubList`].` if it has none.

Translate `data.rs` (its module doc and `SeriesSwatch` docs are French). The whole file
becomes:

```rust
//! Data components of a chart or a table (`ChartData`, `TableData`), host only.
//!
//! The session builds them from the deck; the UI writes a change at the model's `edit` path,
//! then sends the action, which the session applies as one undoable op.

use serde::{Deserialize, Serialize};

/// A series that can be colored: its name (its column header) and its own color.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SeriesSwatch {
    /// The series name, read in its column header.
    pub name: String,
    /// Its own `#rrggbb` color; absent: the theme palette.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[cfg(test)]
#[path = "data_tests.rs"]
mod tests;
```

- [ ] **Step 4: Run the crate's tests**

Run: `cargo nextest run -p oku-slides-a2ui`
Expected: all PASS. If the branch name sits elsewhere than `properties.component`, fix
`branch_name` to match the real output, and fix Task 3's `branch_name` the same way.

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-a2ui/src
git commit -F <scratchpad>/msg-1.txt
# feat(a2ui): the catalog derives its JSON schema (t-1312)
```

---

### Task 2: Playbook examples, UI fixture, render acceptance

**Files:**
- Create: `crates/oku-slides-a2ui/src/playbook.rs`
- Create: `crates/oku-slides-a2ui/src/playbook/entries.rs`, `crates/oku-slides-a2ui/src/playbook/host_entries.rs`
- Create: `crates/oku-slides-a2ui/src/playbook_tests.rs`
- Modify: `crates/oku-slides-a2ui/src/component.rs` (`ComponentKind::is_field`)
- Modify: `crates/oku-slides-a2ui/src/lib.rs` (module, re-exports, crate doc)
- Modify: `crates/oku-slides-session/src/agent/render.rs` (`check_available` calls `is_field`)
- Generated: `ui/src/a2ui/fixtures/playbook.json`
- Test: `crates/oku-slides-session/src/agent/render_tests.rs`

**Interfaces:**
- Consumes: `crate::catalog_schema_tests::catalog()` (Task 1, tests only).
- Produces (re-exported at the crate root):

```rust
pub enum Group { Layout, Atom, Field, HostOnly }        // Serialize: "layout" | "atom" | "field" | "host_only"
pub struct Example {
    pub component: &'static str, pub variant: &'static str, pub note: &'static str,
    pub group: Group, pub agent: bool, pub components: Vec<Component>, pub data: serde_json::Value,
}
pub fn examples() -> Vec<Example>;                       // catalog order, root first in `components`
impl ComponentKind { pub fn is_field(&self) -> bool; }   // the eight data-bound inputs
```

- [ ] **Step 1: `ComponentKind::is_field`, used by `render`**

In `component.rs`, in the `impl ComponentKind` block (next to `children`):

```rust
    /// True for an input bound to the data model, shown by the agent through `show_params`.
    #[must_use]
    pub fn is_field(&self) -> bool {
        matches!(
            self,
            Self::TextField { .. }
                | Self::RichTextField { .. }
                | Self::Stepper { .. }
                | Self::Slider { .. }
                | Self::ColorField { .. }
                | Self::Select { .. }
                | Self::ListTable { .. }
                | Self::ChoiceChips { .. }
        )
    }
```

In `crates/oku-slides-session/src/agent/render.rs`, `check_available` replaces its
`let field = matches!(…);` with `ensure!(!c.kind.is_field(), …)` (same error), so "is a
field" is written once. The existing `render_tests` field cases keep it covered. Translate
the file's French module doc while there, with the catalog version corrected:

```rust
//! `render`: a free composition of catalog v4 components by the agent, checked here.
//! An error goes back to the agent (`AgentForm`), readable, so it can fix it.
```

The doc of `Render.components` also says v3: it becomes "Components of the Oku Slides A2UI
catalog v4; one has id \"root\"."

- [ ] **Step 2: Write `playbook.rs`** (types, `kind_info`, entry point; no examples yet)

```rust
//! The playbook: every catalog component shown in its meaningful variants.
//!
//! [`examples`] feeds the dev gallery (`ui/playbook.html`, through `playbook.json`) and the
//! agent reference (`agent_reference`), so both show the same surfaces.

mod entries;
mod host_entries;

use serde::Serialize;
use serde_json::Value;

use crate::component::{Component, ComponentKind};

/// Where an example sits in the gallery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Group {
    /// Column, Row, Card: they place other components.
    Layout,
    /// A text, a badge, a button, a disclosure: one visible piece.
    Atom,
    /// An input bound to the data model; the agent shows it through `show_params`.
    Field,
    /// A component only the host builds (`Component::agent_allowed` is false).
    HostOnly,
}

/// One playbook example: a component in one variant, as a small surface.
#[derive(Debug, Clone, PartialEq)]
pub struct Example {
    /// The v0.9 component name, such as `Button`.
    pub component: &'static str,
    /// The variant, unique within the component, such as `danger`.
    pub variant: &'static str,
    /// One English sentence: when to use it.
    pub note: &'static str,
    /// Its place in the gallery, from the root's kind.
    pub group: Group,
    /// Whether `render` accepts it (a session test checks it).
    pub agent: bool,
    /// The tree, root first, under the id `root`.
    pub components: Vec<Component>,
    /// The surface's data model; `{}` when every value is literal.
    pub data: Value,
}

/// Every example, in catalog order.
#[must_use]
pub fn examples() -> Vec<Example> {
    entries::all()
}

/// The v0.9 name and the group of a kind; no `_` arm, so a new variant must be named here.
fn kind_info(kind: &ComponentKind) -> (&'static str, Group) {
    use ComponentKind as K;
    use Group::{Atom, Field, HostOnly, Layout};
    match kind {
        K::Column { .. } => ("Column", Layout),
        K::Card { .. } => ("Card", Layout),
        K::Text { .. } => ("Text", Atom),
        K::TextField { .. } => ("TextField", Field),
        K::RichTextField { .. } => ("RichTextField", Field),
        K::Stepper { .. } => ("Stepper", Field),
        K::Row { .. } => ("Row", Layout),
        K::Button { .. } => ("Button", Atom),
        K::ChoiceChips { .. } => ("ChoiceChips", Field),
        K::Slider { .. } => ("Slider", Field),
        K::ColorField { .. } => ("ColorField", Field),
        K::Select { .. } => ("Select", Field),
        K::ListTable { .. } => ("ListTable", Field),
        K::SlideThumb { .. } => ("SlideThumb", HostOnly),
        K::ThemeChoice { .. } => ("ThemeChoice", HostOnly),
        K::DiffText { .. } => ("DiffText", Atom),
        K::Prompt { .. } => ("Prompt", HostOnly),
        K::Timeline { .. } => ("Timeline", HostOnly),
        K::Receipt { .. } => ("Receipt", HostOnly),
        K::Disclosure { .. } => ("Disclosure", Atom),
        K::Badge { .. } => ("Badge", Atom),
        K::Markdown { .. } => ("Markdown", Atom),
        K::Notice { .. } => ("Notice", HostOnly),
        K::ChartData { .. } => ("ChartData", HostOnly),
        K::TableData { .. } => ("TableData", HostOnly),
    }
}

/// An example rooted at `root`, followed by `rest`.
fn example(
    variant: &'static str,
    note: &'static str,
    agent: bool,
    root: ComponentKind,
    rest: Vec<Component>,
    data: Value,
) -> Example {
    let (component, group) = kind_info(&root);
    let mut components = vec![Component::new("root", root)];
    components.extend(rest);
    Example { component, variant, note, group, agent, components, data }
}

#[cfg(test)]
#[path = "playbook_tests.rs"]
mod tests;
```

In `lib.rs`: add `mod playbook;`, `pub use playbook::{Example, Group, examples};`, and a crate
doc bullet: ``//! - [`examples`]: the playbook, every component in its variants (gallery, agent).``

- [ ] **Step 3: Write the failing structure tests** (`playbook_tests.rs`)

```rust
use std::collections::{BTreeSet, HashSet};

use serde_json::{Value, json};
use speculoos::prelude::*;

use super::{Example, Group, examples};
use crate::catalog_schema_tests::catalog;
use crate::{Component, ServerMessage};

#[test]
fn playbook_examples_cover_every_component_in_catalog_order() {
    let mut seen: Vec<String> = Vec::new();
    for e in examples() {
        if seen.last().map(String::as_str) != Some(e.component) {
            seen.push(e.component.to_owned());
        }
    }
    assert_that!(seen).is_equal_to(catalog());
}

#[test]
fn playbook_variants_are_unique_per_component() {
    let all = examples();
    let keys: HashSet<(&str, &str)> = all.iter().map(|e| (e.component, e.variant)).collect();
    assert_that!(keys.len()).is_equal_to(all.len());
}

#[test]
fn playbook_component_name_matches_the_root_serde_tag() {
    for e in examples() {
        let root = serde_json::to_value(&e.components[0]).unwrap();
        asserting(&format!("{}/{}", e.component, e.variant)).that(&root["component"]).is_equal_to(json!(e.component));
    }
}

#[test]
fn playbook_group_is_host_only_exactly_when_not_agent_allowed() {
    for e in examples() {
        let host_only = e.group == Group::HostOnly;
        asserting(&format!("{}/{}", e.component, e.variant))
            .that(&host_only)
            .is_equal_to(!e.components[0].agent_allowed());
    }
}

#[test]
fn playbook_group_is_field_exactly_when_the_root_is_a_field() {
    for e in examples() {
        asserting(&format!("{}/{}", e.component, e.variant))
            .that(&(e.group == Group::Field))
            .is_equal_to(e.components[0].kind.is_field());
    }
}

#[test]
fn playbook_every_layout_and_atom_component_has_an_agent_example() {
    let all = examples();
    let lacking: BTreeSet<&str> = all
        .iter()
        .filter(|e| matches!(e.group, Group::Layout | Group::Atom))
        .map(|e| e.component)
        .filter(|c| !all.iter().any(|e| e.component == *c && e.agent))
        .collect();
    assert_that!(lacking).is_empty();
}

/// Root first under `root`, ids unique, every child present, every component reachable.
fn tree_fault(e: &Example) -> Option<String> {
    if e.components.first().map(|c| c.id.as_str()) != Some("root") {
        return Some("root is not first".into());
    }
    let ids: BTreeSet<&str> = e.components.iter().map(|c| c.id.as_str()).collect();
    if ids.len() != e.components.len() {
        return Some("duplicate id".into());
    }
    let mut reached = BTreeSet::from(["root"]);
    let mut todo = vec!["root"];
    while let Some(id) = todo.pop() {
        let Some(c) = e.components.iter().find(|c| c.id == id) else { return Some(format!("missing {id}")) };
        for child in c.kind.children() {
            if !ids.contains(child) {
                return Some(format!("missing child {child}"));
            }
            if reached.insert(child) {
                todo.push(child);
            }
        }
    }
    (reached != ids).then(|| "unreachable component".into())
}

#[test]
fn playbook_trees_are_valid() {
    let faults: Vec<String> = examples()
        .iter()
        .filter_map(|e| tree_fault(e).map(|f| format!("{}/{}: {f}", e.component, e.variant)))
        .collect();
    assert_that!(faults).is_empty();
}

#[test]
fn playbook_components_survive_a_serde_round_trip() {
    for e in examples() {
        for c in &e.components {
            let back: Component = serde_json::from_value(serde_json::to_value(c).unwrap()).unwrap();
            assert_that!(back).is_equal_to(c.clone());
        }
    }
}

/// An example as the gallery reads it: its surface as v0.9 messages.
fn ui_entry(e: &Example) -> Value {
    let surface = format!("playbook-{}-{}", e.component, e.variant);
    json!({
        "component": e.component,
        "variant": e.variant,
        "note": e.note,
        "group": e.group,
        "agent": e.agent,
        "surface": surface,
        "messages": [
            ServerMessage::create_surface(&surface),
            ServerMessage::update_components(&surface, e.components.clone()),
            ServerMessage::set_data_model(&surface, e.data.clone()),
        ],
    })
}

/// Contract with the gallery: `ui/src/a2ui/fixtures/playbook.json`. `OKU_UPDATE_FIXTURES=1` rewrites it.
#[test]
fn playbook_ui_fixture_is_current() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/src/a2ui/fixtures/playbook.json");
    let entries: Vec<Value> = examples().iter().map(ui_entry).collect();
    let json = serde_json::to_string_pretty(&entries).unwrap() + "\n";
    if std::env::var_os("OKU_UPDATE_FIXTURES").is_some() {
        std::fs::write(path, &json).unwrap();
    }
    let fixture = std::fs::read_to_string(path).unwrap_or_default();
    assert_that!(fixture).named("OKU_UPDATE_FIXTURES=1 to regenerate").is_equal_to(json);
}
```

`Component::agent_allowed` is on `Component` (see `component.rs`), so the group test calls it
on `e.components[0]`.

- [ ] **Step 4: Run to verify they fail**

Run: `cargo nextest run -p oku-slides-a2ui playbook`
Expected: compile error, file not found for module `entries`.

- [ ] **Step 5: Write the examples** (`playbook/entries.rs` and `playbook/host_entries.rs`)

The data is split by concept: `entries.rs` holds the kinds `render` can take (layout, atoms,
fields) and the catalog-order `all()`; `host_entries.rs` holds the eight host-only kinds
(`slide_thumb` to `table_data`). The helpers both files need (`lit`, `palette`) are
`pub(super)` in `entries.rs`; `host_entries.rs` imports them with
`use super::entries::{lit, palette};`, and `entries.rs` imports the eight host functions. Each file stays under 500 lines after
`cargo fmt -p oku-slides-a2ui`.

`playbook/entries.rs`:

```rust
//! The playbook's examples, in catalog order; `agent` is true only where `render` accepts it.

use serde_json::{Value, json};

use super::host_entries::{
    chart_data, notice, prompt, receipt, slide_thumb, table_data, theme_choice, timeline,
};
use super::{Example, example};
use crate::action::{Action, Dynamic};
use crate::component::{Component, ComponentKind as K};
use crate::list::{ColumnKind, ListColumn, SubColumn, SubList};
use crate::props::{ButtonIcon, ChoiceOption, DiffKind, DiffSegment, TextVariant, Tone};

/// The palette offered for highlighting and colors.
const PALETTE: [&str; 3] = ["#1f4e79", "#c2573a", "#2f855a"];

/// A literal text.
pub(super) fn lit(s: &str) -> Dynamic<String> {
    Dynamic::Literal(s.to_owned())
}

/// A text component with a literal value.
fn text(id: &str, s: &str, variant: Option<TextVariant>) -> Component {
    Component::text(id, s, variant)
}

/// The only action `render` lets the agent send: the button's label as the next prompt.
fn suggest(label: &str) -> Action {
    Action::event("suggest", [("prompt", Dynamic::Literal(Value::String(label.to_owned())))])
}

/// A choice option.
fn option(id: &str, label: &str, hint: Option<&str>) -> ChoiceOption {
    ChoiceOption { id: id.to_owned(), label: label.to_owned(), hint: hint.map(str::to_owned) }
}

/// A list column with no options and no nested list.
fn column(key: &str, label: &str, kind: ColumnKind) -> ListColumn {
    ListColumn { key: key.to_owned(), label: label.to_owned(), kind, options: Vec::new(), list: None }
}

/// The palette offered for highlighting and colors, as a prop value.
pub(super) fn palette() -> Vec<String> {
    PALETTE.map(str::to_owned).to_vec()
}

/// Every example, in catalog order: one function per component, in `ComponentKind` order.
pub(super) fn all() -> Vec<Example> {
    [
        column_layout(), card(), text_atom(), text_field(), rich_text_field(), stepper(), row(), button(),
        choice_chips(), slider(), color_field(), select(), list_table(), slide_thumb(), theme_choice(), diff_text(),
        prompt(), timeline(), receipt(), disclosure(), badge(), markdown(), notice(), chart_data(), table_data(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn column_layout() -> Vec<Example> {
    vec![example(
        "default",
        "Stacks children top to bottom: the usual root of a card's body.",
        true,
        K::Column { children: vec!["title".into(), "body".into()] },
        vec![
            text("title", "Three palettes", Some(TextVariant::H4)),
            text("body", "Pick one to apply it to every slide.", None),
        ],
        json!({}),
    )]
}

fn card() -> Vec<Example> {
    let body = || vec![text("t", "A card groups related content.", None)];
    vec![
        example("default", "Frames one child; the neutral card of a thread.", true,
            K::Card { child: "t".into(), tone: None }, body(), json!({})),
        example("accent", "Highlights a conclusion or a suggestion.", true,
            K::Card { child: "t".into(), tone: Some(Tone::Accent) }, body(), json!({})),
        example("danger", "Marks a host warning; render refuses the danger tone.", false,
            K::Card { child: "t".into(), tone: Some(Tone::Danger) }, body(), json!({})),
    ]
}

fn text_atom() -> Vec<Example> {
    let variant = |name: &'static str, note: &'static str, v: TextVariant, s: &str| {
        example(name, note, true, K::Text { text: lit(s), variant: Some(v) }, vec![], json!({}))
    };
    vec![
        variant("h4", "A section heading inside a card.", TextVariant::H4, "Palette"),
        variant("body", "Body text, the default style.", TextVariant::Body, "The title turns white on the dark background."),
        variant("caption", "A smaller line under a heading or a figure.", TextVariant::Caption, "Source: Insee, 2024"),
        variant("note", "A dimmed line, truncated when too long.", TextVariant::Note, "Applied to slides 2 to 9"),
        example("bound", "Reads its text from the data model; render accepts literals only.", false,
            K::Text { text: Dynamic::path("/title"), variant: None }, vec![], json!({ "title": "Quarterly review" })),
    ]
}

fn text_field() -> Vec<Example> {
    vec![example(
        "bound",
        "Edits a model value; the action fires on Enter or blur.",
        false,
        K::TextField { label: lit("Title"), value: Dynamic::path("/title"), action: Some(Action::named("commit")) },
        vec![],
        json!({ "title": "Quarterly review" }),
    )]
}

fn rich_text_field() -> Vec<Example> {
    vec![example(
        "bound",
        "Edits paragraphs as markdown; writes the markdown and the paragraphs, then acts.",
        false,
        K::RichTextField {
            label: lit("Body"),
            value: Dynamic::path("/body"),
            source: Dynamic::path("/body-source"),
            palette: palette(),
            action: Some(Action::named("commit")),
        },
        vec![],
        json!({ "body": "**Bold** start", "body-source": [{ "runs": [{ "t": "Bold", "bold": true }, { "t": " start" }] }] }),
    )]
}

fn stepper() -> Vec<Example> {
    vec![example(
        "bounded",
        "A small integer with − and + buttons, between min and max.",
        false,
        K::Stepper { label: lit("Columns"), value: Dynamic::path("/columns"), min: Some(1), max: Some(4), action: Some(Action::named("step")) },
        vec![],
        json!({ "columns": 2 }),
    )]
}

fn row() -> Vec<Example> {
    vec![example(
        "default",
        "Places children side by side, such as badges or buttons.",
        true,
        K::Row { children: vec!["a".into(), "b".into()] },
        vec![
            Component::new("a", K::Badge { text: lit("12 slides"), tone: None }),
            Component::new("b", K::Badge { text: lit("Saved"), tone: Some(Tone::Accent) }),
        ],
        json!({}),
    )]
}

fn button() -> Vec<Example> {
    let host = |name: &'static str, note: &'static str, b: Component| {
        example(name, note, false, b.kind, vec![], json!({}))
    };
    vec![
        example("suggest", "Sends its label as the user's next prompt: the only button render allows.", true,
            K::Button { label: lit("Make it shorter"), primary: None, danger: None, icon: None, action: suggest("Make it shorter") },
            vec![], json!({})),
        host("primary", "The surface's main action (host only).",
            Component::button("root", "Apply", true, Action::named("apply"))),
        host("danger", "A committing action, not undoable (host only).",
            Component::button("root", "Delete the pack", false, Action::named("delete")).danger(true)),
        host("icon", "A pictogram; the label becomes its accessible name (host only).",
            Component::button("root", "Close", false, Action::named("close")).icon(ButtonIcon::Close)),
    ]
}

fn choice_chips() -> Vec<Example> {
    vec![example(
        "default",
        "A few short choices as chips; the chosen id goes to the model.",
        false,
        K::ChoiceChips {
            options: vec![option("a", "Two columns", None), option("b", "Quote", None), option("c", "Full image", None)],
            value: Dynamic::path("/layout"),
            action: Some(Action::named("choose")),
        },
        vec![],
        json!({ "layout": "b" }),
    )]
}

fn slider() -> Vec<Example> {
    vec![example(
        "stepped",
        "A bounded number; the action fires on release.",
        false,
        K::Slider { label: lit("Opacity"), value: Dynamic::path("/opacity"), min: 0.0, max: 100.0, step: Some(5.0), action: Some(Action::named("slide")) },
        vec![],
        json!({ "opacity": 60 }),
    )]
}

fn color_field() -> Vec<Example> {
    vec![example(
        "palette",
        "A hex color, picked from the palette or typed.",
        false,
        K::ColorField { label: lit("Accent"), value: Dynamic::path("/color"), palette: palette(), action: Some(Action::named("color")) },
        vec![],
        json!({ "color": "#1f4e79" }),
    )]
}

fn select() -> Vec<Example> {
    vec![example(
        "with-hints",
        "A dropdown for longer option lists; a hint shows next to a label.",
        false,
        K::Select {
            label: lit("Theme"),
            value: Dynamic::path("/theme"),
            options: vec![option("papier", "Paper", Some("light")), option("pro", "Pro", Some("dark")), option("neutre", "Neutral", None)],
            action: Some(Action::named("choose")),
        },
        vec![],
        json!({ "theme": "papier" }),
    )]
}

fn list_table() -> Vec<Example> {
    let flat = vec![
        column("name", "Name", ColumnKind::Text),
        column("count", "Count", ColumnKind::Number),
        column("done", "Done", ColumnKind::Flag),
    ];
    let mut tone = column("tone", "Tone", ColumnKind::Select);
    tone.options = vec![option("a", "Two columns", None), option("b", "Quote", Some("sober"))];
    let mut tasks = column("tasks", "Tasks", ColumnKind::List);
    tasks.list = Some(Box::new(SubList {
        columns: vec![
            SubColumn { key: "done".into(), label: "Done".into(), kind: ColumnKind::Flag, options: Vec::new() },
            SubColumn { key: "note".into(), label: "Note".into(), kind: ColumnKind::Rich, options: Vec::new() },
        ],
        min: 0,
        max: Some(8),
        item_label: Some("task".into()),
    }));
    let table = |columns: Vec<ListColumn>| K::ListTable {
        label: lit("Steps"),
        rows: Dynamic::path("/rows"),
        change: Dynamic::path("/change"),
        markdown: Dynamic::path("/markdown"),
        columns,
        min: 1,
        max: Some(10),
        item_label: Some("step".into()),
        palette: palette(),
        action: Some(Action::named("list")),
    };
    vec![
        example("flat", "A list of items edited in place: text, number and checkbox columns.", false,
            table(flat), vec![],
            json!({ "rows": [{ "id": "r0", "name": "Intro", "count": 3, "done": true }, { "id": "r1", "name": "Plan", "count": 5, "done": false }],
                    "change": null, "markdown": {} })),
        example("nested", "Rich, select and a one-level nested list column.", false,
            table(vec![column("date", "Date", ColumnKind::Text), column("text", "Text", ColumnKind::Rich), tone, tasks]), vec![],
            json!({
                "rows": [{ "id": "d0", "date": "2019", "text": [{ "runs": [{ "t": "Founded" }] }], "tone": "a",
                           "tasks": [{ "id": "t1", "done": true, "note": [{ "runs": [{ "t": "Review" }] }] }] }],
                "change": null,
                "markdown": { "d0": { "text": "Founded" }, "d0/tasks/t1": { "note": "Review" } },
            })),
    ]
}

fn diff_text() -> Vec<Example> {
    let seg = |kind: DiffKind, t: &str| DiffSegment { kind, text: t.to_owned() };
    vec![example(
        "default",
        "Shows a proposed rewrite word by word.",
        true,
        K::DiffText { segments: vec![seg(DiffKind::Keep, "Sales "), seg(DiffKind::Del, "rose"), seg(DiffKind::Ins, "grew"), seg(DiffKind::Keep, " by 12%.")] },
        vec![],
        json!({}),
    )]
}

fn disclosure() -> Vec<Example> {
    let detail = || vec![text("d", "Slides 2, 5 and 9 use the old title size.", None)];
    vec![
        example("collapsed", "Hides details behind a summary line.", true,
            K::Disclosure { summary: lit("3 slides changed"), child: "d".into(), open: None }, detail(), json!({})),
        example("open", "Shown expanded on first render.", true,
            K::Disclosure { summary: lit("3 slides changed"), child: "d".into(), open: Some(true) }, detail(), json!({})),
    ]
}

fn badge() -> Vec<Example> {
    let badge = |name: &'static str, note: &'static str, agent: bool, tone: Option<Tone>| {
        example(name, note, agent, K::Badge { text: lit("12 slides"), tone }, vec![], json!({}))
    };
    vec![
        badge("neutral", "A count or a status.", true, None),
        badge("accent", "A status worth noticing.", true, Some(Tone::Accent)),
        badge("danger", "A host alert; render refuses the danger tone.", false, Some(Tone::Danger)),
    ]
}

fn markdown() -> Vec<Example> {
    vec![example(
        "default",
        "Formatted text without raw HTML: bold, italic, lists, links.",
        true,
        K::Markdown { text: lit("**Bold**, *italic* and a list:\n\n- first\n- second") },
        vec![],
        json!({}),
    )]
}

```

`playbook/host_entries.rs`:

```rust
//! The playbook's host-only examples: components the agent never sends (`agent` false).

use serde_json::{Value, json};

use super::entries::{lit, palette};
use super::{Example, example};
use crate::action::{Action, Dynamic};
use crate::component::{Component, ComponentKind as K};
use crate::data::SeriesSwatch;
use crate::props::{PromptImage, SlideRef, StepStatus, TimelineStep, ToolCallDetail};

/// The action a host data component sends after writing `/edit`.
fn data_action() -> Action {
    Action::event("data", [("edit", Dynamic::path("/edit"))])
}

pub(super) fn slide_thumb() -> Vec<Example> {
    vec![example(
        "default",
        "A slide rendered by the host; a click sends the action.",
        false,
        K::SlideThumb {
            svg: r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 160 90"><rect width="160" height="90" fill="#1f4e79"/></svg>"##.into(),
            label: lit("4 · Quarterly review"),
            action: Some(Action::named("slide")),
        },
        vec![],
        json!({}),
    )]
}

pub(super) fn theme_choice() -> Vec<Example> {
    let choice = |svg: Option<String>, active: bool| K::ThemeChoice {
        svg,
        name: "Paper".into(),
        slide: "s-4".into(),
        swatches: vec!["#ffffff".into(), "#2f5fb3".into()],
        active,
        action: Some(Action::event("theme", [("theme", Dynamic::Literal(json!("papier")))])),
    };
    let thumb = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 160 90"><rect width="160" height="90" fill="#ffffff"/><rect x="12" y="12" width="90" height="10" fill="#2f5fb3"/></svg>"##;
    vec![
        example("active", "The deck's current theme, with its thumbnail and swatches.", false,
            choice(Some(thumb.into()), true), vec![], json!({})),
        example("no-thumbnail", "A theme whose render failed: still selectable.", false,
            choice(None, false), vec![], json!({})),
    ]
}

pub(super) fn prompt() -> Vec<Example> {
    vec![example(
        "with-image",
        "The user's prompt opening a turn, with a pasted image chip.",
        false,
        K::Prompt { text: lit("Use this photo as the background of slide four."), images: vec![PromptImage { mime: "image/png".into(), width: 1280, height: 720 }] },
        vec![],
        json!({}),
    )]
}

pub(super) fn timeline() -> Vec<Example> {
    let mut note = TimelineStep::new("Night blue chosen.", StepStatus::Completed);
    note.detail = Some("The title turns white.".into());
    let mut tools = TimelineStep::new("apply", StepStatus::Completed);
    tools.count = Some(2);
    tools.calls = vec![ToolCallDetail {
        label: "apply".into(),
        status: StepStatus::Completed,
        input: Some("{\n  \"slide\": 4\n}".into()),
        output: Some("background set".into()),
    }];
    let steps = vec![note, tools, TimelineStep::new("render_slide", StepStatus::InProgress)];
    // Typed literal steps: the host binds them to its model, the catalog accepts both.
    let kind = |collapsed: bool| K::Timeline {
        steps: Dynamic::Literal(steps.clone()),
        collapsed: Dynamic::Literal(collapsed),
        summary: lit("3 tools · 1 note · 12 s"),
    };
    vec![
        example("expanded", "A turn's steps while it runs.", false, kind(false), vec![], json!({})),
        example("collapsed", "A finished turn folded into its summary line.", false, kind(true), vec![], json!({})),
    ]
}

pub(super) fn receipt() -> Vec<Example> {
    vec![example(
        "with-slide",
        "A turn's conclusion; with no deck its thumbnail shows as missing.",
        false,
        K::Receipt {
            title: lit("Slide 4: night blue background"),
            facts: vec!["title in white".into(), "light blue kicker".into()],
            slide: Some(SlideRef { id: "s-4".into(), hash: "0123456789abcdef0123456789abcdef".into() }),
            swatch: Some("#00154e".into()),
        },
        vec![],
        json!({}),
    )]
}

pub(super) fn notice() -> Vec<Example> {
    vec![example(
        "with-button",
        "A host banner with its buttons and a close cross.",
        false,
        K::Notice { text: lit("A new version is ready."), children: vec!["restart".into()], close: Action::named("dismiss"), close_label: lit("Dismiss") },
        vec![Component::button("restart", "Restart", false, Action::named("restart"))],
        json!({}),
    )]
}

pub(super) fn chart_data() -> Vec<Example> {
    vec![example(
        "bar",
        "A chart's data table, series colors and stacking.",
        false,
        K::ChartData {
            kind: Some("bar".into()),
            source: vec![
                vec![json!("Month"), json!("Sales"), json!("Target")],
                vec![json!("Jan"), json!(3), json!(4)],
                vec![json!("Feb"), json!(5), json!(4)],
            ],
            checkbox_columns: vec![],
            stacked: Some(false),
            series: vec![SeriesSwatch { name: "Sales".into(), color: None }, SeriesSwatch { name: "Target".into(), color: Some("#1f4e79".into()) }],
            palette: palette(),
            edit: Dynamic::path("/edit"),
            action: data_action(),
        },
        vec![],
        json!({ "edit": null }),
    )]
}

pub(super) fn table_data() -> Vec<Example> {
    vec![example(
        "default",
        "A table's cells and display options.",
        false,
        K::TableData {
            source: vec![vec![json!("A"), json!("B")], vec![json!(1), Value::Null]],
            header_row: true,
            header_column: false,
            striped: true,
            align: vec!["auto".into(), "end".into()],
            edit: Dynamic::path("/edit"),
            action: data_action(),
        },
        vec![],
        json!({ "edit": null }),
    )]
}
```

Format with `cargo fmt -p oku-slides-a2ui` and check both files stay under 500 lines
(`wc -l crates/oku-slides-a2ui/src/playbook/*.rs`). `StepStatus::InProgress` and
`Completed` are the serde names `in_progress` / `completed`: check them in `props.rs`.

- [ ] **Step 6: Generate the fixture and run the tests**

```bash
OKU_UPDATE_FIXTURES=1 cargo nextest run -p oku-slides-a2ui playbook_ui_fixture_is_current
cargo nextest run -p oku-slides-a2ui
```
Expected: all PASS; `ui/src/a2ui/fixtures/playbook.json` exists with 40 entries.

- [ ] **Step 7: Write the render acceptance test** (append to `crates/oku-slides-session/src/agent/render_tests.rs`)

```rust
/// What `a2ui_examples` shows, `render` accepts, and nothing else (spec:2026-10-30-playbook-a2ui-design).
#[test]
fn render_accepts_exactly_the_playbook_agent_examples() {
    let wrong: Vec<String> = oku_slides_a2ui::examples()
        .into_iter()
        .filter(|e| {
            let components = e.components.iter().map(|c| serde_json::to_value(c).unwrap()).collect();
            render("render-1", &Render { components }).is_ok() != e.agent
        })
        .map(|e| format!("{}/{} (agent: {})", e.component, e.variant, e.agent))
        .collect();
    assert_that!(wrong).is_empty();
}
```

- [ ] **Step 8: See it fail, then pass**

First prove the test bites: set `agent` to `true` on `Card`/`danger` in `entries.rs`, run
`cargo nextest run -p oku-slides-session render_accepts_exactly`, and expect FAIL listing
`Card/danger (agent: true)`. Restore `false`, rerun: PASS. Any other listed example means its
`agent` flag is wrong: fix the flag (not `render`), regenerate the fixture (Step 6), rerun.

- [ ] **Step 9: Commit**

```bash
git add crates/oku-slides-a2ui/src crates/oku-slides-session/src/agent ui/src/a2ui/fixtures/playbook.json
git commit -F <scratchpad>/msg-2.txt
# feat(a2ui,session): the playbook, every catalog component in its variants (t-1312)
```

---

### Task 3: Agent reference from the schema

**Files:**
- Modify: `crates/oku-slides-a2ui/Cargo.toml` (`snafu.workspace = true`; dev: `insta.workspace = true`, `rstest.workspace = true`), `Cargo.lock`
- Create: `crates/oku-slides-a2ui/src/playbook/reference.rs`, `crates/oku-slides-a2ui/src/playbook/reference_tests.rs`
- Modify: `crates/oku-slides-a2ui/src/playbook.rs` (`pub(crate) mod reference; pub use …`), `src/lib.rs` (re-exports)
- Modify: `crates/oku-slides-a2ui/src/catalog_schema_tests.rs` (use the production `branch_name`)
- Modify: `crates/oku-slides-a2ui/ARCHITECTURE.md`, `crates/oku-slides-a2ui/AGENTS.md`

**Interfaces:**
- Consumes: `examples()`, `Example` (Task 2); the schema (Task 1).
- Produces `crate::playbook::reference::branch_name(&Value) -> Option<&str>` (`pub(crate)`,
  in a `pub(crate) mod reference`): the one reader of a branch's component name.
- Produces (re-exported at the crate root). `agent_reference` is computed once and cached:
  the schema and the examples are constants of the build.

```rust
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PropReference { pub name: String, pub required: bool,
    #[serde(rename = "type")] pub ty: String,
    #[serde(skip_serializing_if = "Option::is_none")] pub values: Option<Vec<String>>,
    pub description: String }
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExampleReference { pub variant: &'static str, pub note: &'static str, pub surface: Vec<Component> }
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComponentReference { pub component: String, pub props: Vec<PropReference>, pub examples: Vec<ExampleReference> }
#[derive(Debug, Snafu)] pub struct PlaybookError(/* opaque */);
pub fn agent_reference() -> Result<&'static [ComponentReference], &'static PlaybookError>;
pub fn render_components(reference: &[ComponentReference]) -> String;
```

- [ ] **Step 1: Write the failing tests** (`reference_tests.rs`)

```rust
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::{Inner, agent_reference, branch_name, clean_doc, reference_from, render_components};
use crate::Example;

#[rstest]
#[case::plain("Child identifiers, top to bottom.", "Child identifiers, top to bottom.")]
#[case::link("Components of the [`CATALOG_ID`](crate::CATALOG_ID) catalog.", "Components of the `CATALOG_ID` catalog.")]
#[case::bare_link("Fate of a text chunk in a [`DiffSegment`].", "Fate of a text chunk in a `DiffSegment`.")]
#[case::first_paragraph("A button.\n\n`primary`: the main action.", "A button.")]
#[case::wrapped("Line one\ncontinues here.", "Line one continues here.")]
fn clean_doc_keeps_the_first_paragraph_without_links(#[case] doc: &str, #[case] expected: &str) {
    assert_that!(clean_doc(doc)).is_equal_to(expected.to_owned());
}

#[test]
fn render_components_lists_exactly_what_the_agent_examples_use() {
    assert_that!(render_components(agent_reference().unwrap())).is_equal_to(
        "Column{children}, Card{child, tone?: accent}, Text{text, variant?: h4|body|caption|note}, \
         Row{children}, Button{label, action}, DiffText{segments}, Disclosure{summary, child, open?}, \
         Badge{text, tone?: accent}, Markdown{text}"
            .to_owned(),
    );
}

#[rstest]
#[case::literal_text("Text", "text", "string")]
#[case::named_array("DiffText", "segments", "[DiffSegment]")]
#[case::named_object("Button", "action", "Action")]
#[case::optional_bool("Disclosure", "open", "boolean")]
#[case::string_array("Column", "children", "[string]")]
fn agent_reference_reads_each_prop_type(#[case] component: &str, #[case] prop: &str, #[case] ty: &str) {
    let reference = agent_reference().unwrap();
    let props = &reference.iter().find(|c| c.component == component).unwrap().props;
    assert_that!(props.iter().find(|p| p.name == prop).unwrap().ty.as_str()).is_equal_to(ty);
}

#[test]
fn agent_reference_button_reads_label_then_action_both_required() {
    let reference = agent_reference().unwrap();
    let button = serde_json::to_value(reference.iter().find(|c| c.component == "Button").unwrap()).unwrap();
    let props: Vec<Value> = button["props"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| json!([p["name"], p["required"], p["type"]]))
        .collect();
    assert_that!(props).is_equal_to(vec![json!(["label", true, "string"]), json!(["action", true, "Action"])]);
}

#[test]
fn agent_reference_matches_its_snapshot() {
    insta::assert_json_snapshot!(agent_reference().unwrap());
}

#[test]
fn agent_reference_descriptions_are_clean_and_present() {
    let faults: Vec<String> = agent_reference()
        .unwrap()
        .iter()
        .flat_map(|c| c.props.iter().map(move |p| (c.component.as_str(), p)))
        .filter(|(_, p)| p.description.is_empty() || p.description.contains("](") || p.description.contains("crate::"))
        .map(|(c, p)| format!("{c}.{}: {:?}", p.name, p.description))
        .collect();
    assert_that!(faults).is_empty();
}

#[rstest]
#[case::const_tag(json!({ "properties": { "component": { "const": "Badge" } } }), Some("Badge"))]
#[case::enum_tag(json!({ "properties": { "component": { "enum": ["Badge"] } } }), Some("Badge"))]
#[case::untagged(json!({ "properties": {} }), None)]
fn branch_name_reads_a_const_or_a_one_value_enum(#[case] branch: Value, #[case] expected: Option<&str>) {
    assert_that!(branch_name(&branch)).is_equal_to(expected);
}

/// The Markdown agent examples alone.
fn markdown_examples() -> Vec<Example> {
    crate::examples().into_iter().filter(|e| e.component == "Markdown").collect()
}

/// A schema with one Markdown branch whose `text` prop is `text`, and `defs`.
fn markdown_schema(text: Value, defs: Value) -> Value {
    json!({
        "$defs": defs,
        "oneOf": [{
            "type": "object",
            "properties": { "component": { "const": "Markdown" }, "text": text },
            "required": ["component", "text"],
        }],
    })
}

#[test]
fn reference_from_a_schema_without_one_of_is_no_branches() {
    let e = reference_from(&json!({}), &crate::examples()).unwrap_err();
    assert_that!(matches!(e, Inner::NoBranches { .. })).is_true();
}

#[test]
fn reference_from_a_schema_missing_a_branch_is_missing_branch() {
    let e = reference_from(&json!({ "oneOf": [] }), &crate::examples()).unwrap_err();
    assert_that!(matches!(e, Inner::MissingBranch { ref component, .. } if component == "Column")).is_true();
}

#[test]
fn reference_from_an_undocumented_prop_is_no_description() {
    let schema = markdown_schema(json!({ "type": "string" }), json!({}));
    let e = reference_from(&schema, &markdown_examples()).unwrap_err();
    assert_that!(matches!(e, Inner::NoDescription { ref prop, .. } if prop == "text")).is_true();
}

#[test]
fn reference_from_an_unresolved_reference_is_unresolved() {
    let schema = markdown_schema(json!({ "$ref": "#/$defs/Nowhere", "description": "The text." }), json!({}));
    let e = reference_from(&schema, &markdown_examples()).unwrap_err();
    assert_that!(matches!(e, Inner::Unresolved { .. })).is_true();
}

#[test]
fn reference_from_a_shape_it_cannot_read_is_unknown_shape() {
    let schema = markdown_schema(json!({ "not": {}, "description": "The text." }), json!({}));
    let e = reference_from(&schema, &markdown_examples()).unwrap_err();
    assert_that!(matches!(e, Inner::UnknownShape { .. })).is_true();
}

#[test]
fn reference_from_a_reference_cycle_is_too_deep() {
    let defs = json!({ "Loop": { "$ref": "#/$defs/Loop" } });
    let schema = markdown_schema(json!({ "$ref": "#/$defs/Loop", "description": "The text." }), defs);
    let e = reference_from(&schema, &markdown_examples()).unwrap_err();
    assert_that!(matches!(e, Inner::TooDeep { .. })).is_true();
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo nextest run -p oku-slides-a2ui reference`
Expected: compile error, module `reference` not found.

- [ ] **Step 3: Write `reference.rs`**

```rust
//! The agent's reference: the catalog schema, cut to what the agent examples use.
//!
//! Only components with an example `render` accepts appear, with their required props and the
//! optional props and enum values those examples set: the agent never reads what `render` refuses.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use schemars::Schema;
use schemars::generate::SchemaSettings;
use schemars::transform::{Transform, transform_subschemas};
use serde::Serialize;
use serde_json::{Map, Value};
use snafu::{Backtrace, OptionExt, ResultExt, Snafu, ensure};

use super::Example;
use crate::component::{Component, ComponentKind};

/// One prop of a component, as the agent reads it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PropReference {
    /// The prop's key.
    pub name: String,
    /// Whether every component must set it.
    pub required: bool,
    /// Its type: `string`, `integer`, `boolean`, `[DiffSegment]`, `Action`, `any`…
    #[serde(rename = "type")]
    pub ty: String,
    /// The string values it takes, cut to those the agent examples use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
    /// The first paragraph of its doc comment, without rustdoc links.
    pub description: String,
}

/// One example of a component, as a ready `render` input.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExampleReference {
    /// The variant shown.
    pub variant: &'static str,
    /// When to use it.
    pub note: &'static str,
    /// The components to pass to `render`, root first.
    pub surface: Vec<Component>,
}

/// A component `render` accepts: its props and its examples.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComponentReference {
    /// The v0.9 component name.
    pub component: String,
    /// Its props, in declaration order.
    pub props: Vec<PropReference>,
    /// Its agent examples.
    pub examples: Vec<ExampleReference>,
}

/// The catalog schema could not be read into the agent reference.
#[derive(Debug, Snafu)]
pub struct PlaybookError(Inner);

#[derive(Debug, Snafu)]
enum Inner {
    #[snafu(display("the catalog schema has no oneOf"))]
    NoBranches { backtrace: Backtrace },
    #[snafu(display("a catalog schema branch names no component"))]
    Unnamed { backtrace: Backtrace },
    #[snafu(display("unresolved schema reference {reference}"))]
    Unresolved { reference: String, backtrace: Backtrace },
    #[snafu(display("{component}.{prop} has no description"))]
    NoDescription { component: String, prop: String, backtrace: Backtrace },
    #[snafu(display("an agent example names {component}, absent from the schema"))]
    MissingBranch { component: String, backtrace: Backtrace },
    #[snafu(display("cannot serialize an example component"))]
    ToJson { source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("a prop schema the reference cannot read: {shape}"))]
    UnknownShape { shape: String, backtrace: Backtrace },
    #[snafu(display("schema references nest deeper than {MAX_DEPTH}"))]
    TooDeep { backtrace: Backtrace },
}

/// How deep `describe` follows `$ref`s and unions; the catalog needs 3.
const MAX_DEPTH: usize = 8;

/// Rewrites every description to its first paragraph, without rustdoc links.
#[derive(Debug, Clone)]
struct CleanDocs;

impl Transform for CleanDocs {
    fn transform(&mut self, schema: &mut Schema) {
        if let Some(Value::String(doc)) = schema.get_mut("description") {
            *doc = clean_doc(doc);
        }
        transform_subschemas(self, schema);
    }
}

/// The first paragraph of `doc` on one line, ``[`X`](path)`` and ``[`X`]`` written `` `X` ``.
fn clean_doc(doc: &str) -> String {
    let first = doc.split("\n\n").next().unwrap_or_default();
    let line = first.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = String::with_capacity(line.len());
    let mut rest = line.as_str();
    while let Some(start) = rest.find("[`") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find("`]") else {
            out.push_str(&rest[start..]);
            return out;
        };
        out.push_str(&after[..=end]);
        rest = &after[end + 2..];
        if let Some(target) = rest.strip_prefix('(')
            && let Some(close) = target.find(')')
        {
            rest = &target[close + 1..];
        }
    }
    out.push_str(rest);
    out
}

/// The catalog schema with clean descriptions.
fn catalog_schema() -> Value {
    SchemaSettings::draft2020_12().with_transform(CleanDocs).into_generator().into_root_schema_for::<ComponentKind>().to_value()
}

/// The reference of every component `render` accepts, in catalog order.
///
/// Built once: the schema and the examples are constants of the build.
///
/// # Errors
///
/// [`PlaybookError`] if the catalog schema cannot be read (a programming fault).
pub fn agent_reference() -> Result<&'static [ComponentReference], &'static PlaybookError> {
    static REFERENCE: OnceLock<Result<Vec<ComponentReference>, PlaybookError>> = OnceLock::new();
    REFERENCE
        .get_or_init(|| Ok(reference_from(&catalog_schema(), &super::examples())?))
        .as_ref()
        .map(Vec::as_slice)
}

/// The reference that `schema` and `examples` give.
fn reference_from(schema: &Value, examples: &[Example]) -> Result<Vec<ComponentReference>, Inner> {
    let empty = Map::new();
    let defs = schema.get("$defs").and_then(Value::as_object).unwrap_or(&empty);
    let branches = schema.get("oneOf").and_then(Value::as_array).context(NoBranchesSnafu)?;
    let mut out = Vec::new();
    let mut named = BTreeSet::new();
    for branch in branches {
        let name = branch_name(branch).context(UnnamedSnafu)?;
        named.insert(name.to_owned());
        let agent: Vec<&Example> = examples.iter().filter(|e| e.agent && e.component == name).collect();
        if agent.is_empty() {
            continue;
        }
        let used = used_values(name, &agent)?;
        let required: BTreeSet<&str> =
            branch.get("required").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).collect();
        let mut props = Vec::new();
        for (prop, sub) in branch.get("properties").and_then(Value::as_object).into_iter().flatten() {
            let is_required = required.contains(prop.as_str());
            if prop == "component" || (!is_required && !used.contains_key(prop)) {
                continue;
            }
            let (ty, all_values) = describe(sub, defs, 0)?;
            let values = all_values.map(|v| match used.get(prop) {
                Some(seen) => v.into_iter().filter(|x| seen.contains(x)).collect(),
                None => v,
            });
            let description = description(sub, defs)?.filter(|d| !d.is_empty()).context(NoDescriptionSnafu {
                component: name.to_owned(),
                prop: prop.clone(),
            })?;
            props.push(PropReference { name: prop.clone(), required: is_required, ty, values, description });
        }
        let examples = agent
            .iter()
            .map(|e| ExampleReference { variant: e.variant, note: e.note, surface: e.components.clone() })
            .collect();
        out.push(ComponentReference { component: name.to_owned(), props, examples });
    }
    if let Some(e) = examples.iter().find(|e| e.agent && !named.contains(e.component)) {
        return MissingBranchSnafu { component: e.component.to_owned() }.fail();
    }
    Ok(out)
}

/// The component name of a `oneOf` branch (`const`, or a one-value `enum`).
pub(crate) fn branch_name(branch: &Value) -> Option<&str> {
    let tag = branch.get("properties")?.get("component")?;
    tag.get("const").and_then(Value::as_str).or_else(|| tag.get("enum")?.get(0)?.as_str())
}

/// Per prop set (non-null) by a `name` component of the agent examples: the string values seen.
fn used_values(name: &str, agent: &[&Example]) -> Result<BTreeMap<String, BTreeSet<String>>, Inner> {
    let mut used: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for e in agent {
        for c in &e.components {
            let v = serde_json::to_value(c).context(ToJsonSnafu)?;
            if v.get("component").and_then(Value::as_str) != Some(name) {
                continue;
            }
            for (k, x) in v.as_object().into_iter().flatten().filter(|(_, x)| !x.is_null()) {
                let seen = used.entry(k.clone()).or_default();
                if let Some(s) = x.as_str() {
                    seen.insert(s.to_owned());
                }
            }
        }
    }
    Ok(used)
}

/// The schema `#/$defs/<name>` points to, and that name.
fn resolve<'a>(reference: &str, defs: &'a Map<String, Value>) -> Result<(&'a str, &'a Value), Inner> {
    let name = reference.strip_prefix("#/$defs/").context(UnresolvedSnafu { reference })?;
    defs.get_key_value(name).map(|(k, v)| (k.as_str(), v)).context(UnresolvedSnafu { reference })
}

/// A prop's own description, else the description of the type it points to.
fn description(sub: &Value, defs: &Map<String, Value>) -> Result<Option<String>, Inner> {
    if let Some(d) = sub.get("description").and_then(Value::as_str) {
        return Ok(Some(d.to_owned()));
    }
    match sub.get("$ref").and_then(Value::as_str) {
        Some(r) => Ok(resolve(r, defs)?.1.get("description").and_then(Value::as_str).map(str::to_owned)),
        None => Ok(None),
    }
}

/// `{path}`: the data-model branch of a `Dynamic`.
fn is_path(s: &Value) -> bool {
    s.get("properties").and_then(Value::as_object).is_some_and(|p| p.len() == 1 && p.contains_key("path"))
}

/// The type a schema reads as, and its string values when it is a string enum.
///
/// `any` only for a schema that accepts any JSON (`true` or `{}`); a shape it cannot read
/// is an error, so a new prop type never reaches the agent as a vague `any`.
fn describe(s: &Value, defs: &Map<String, Value>, depth: usize) -> Result<(String, Option<Vec<String>>), Inner> {
    ensure!(depth < MAX_DEPTH, TooDeepSnafu);
    let unknown = || UnknownShapeSnafu { shape: s.to_string() };
    if s == &Value::Bool(true) || s.as_object().is_some_and(Map::is_empty) {
        return Ok(("any".into(), None));
    }
    if let Some(r) = s.get("$ref").and_then(Value::as_str) {
        let (name, target) = resolve(r, defs)?;
        if target.get("properties").is_some() {
            return Ok((name.to_owned(), None));
        }
        return describe(target, defs, depth + 1);
    }
    if let Some(branches) = s.get("anyOf").or_else(|| s.get("oneOf")).and_then(Value::as_array) {
        let kept: Vec<&Value> = branches
            .iter()
            .filter(|b| b.get("type").and_then(Value::as_str) != Some("null") && !is_path(b))
            .collect();
        let consts: Option<Vec<String>> =
            kept.iter().map(|b| b.get("const").and_then(Value::as_str).map(str::to_owned)).collect();
        if let Some(values) = consts.filter(|v| !v.is_empty()) {
            return Ok(("string".into(), Some(values)));
        }
        return match kept.as_slice() {
            [one] => describe(one, defs, depth + 1),
            _ => unknown().fail(),
        };
    }
    if let Some(values) = s.get("enum").and_then(Value::as_array) {
        return Ok(("string".into(), Some(values.iter().filter_map(Value::as_str).map(str::to_owned).collect())));
    }
    let ty = match s.get("type") {
        Some(Value::String(t)) => t.clone(),
        Some(Value::Array(ts)) => ts.iter().filter_map(Value::as_str).find(|t| *t != "null").context(unknown())?.to_owned(),
        _ => return unknown().fail(),
    };
    if ty == "array" {
        let (item, _) = describe(s.get("items").context(unknown())?, defs, depth + 1)?;
        return Ok((format!("[{item}]"), None));
    }
    Ok((ty, None))
}

/// The reference as `Name{prop, prop?: a|b}`, comma-separated: `render`'s component list.
#[must_use]
pub fn render_components(reference: &[ComponentReference]) -> String {
    reference
        .iter()
        .map(|c| {
            let props: Vec<String> = c
                .props
                .iter()
                .map(|p| {
                    let mark = if p.required { "" } else { "?" };
                    match &p.values {
                        Some(v) => format!("{}{mark}: {}", p.name, v.join("|")),
                        None => format!("{}{mark}", p.name),
                    }
                })
                .collect();
            format!("{}{{{}}}", c.component, props.join(", "))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
#[path = "reference_tests.rs"]
mod tests;
```

Notes for the implementer:
- `reference_from`, `clean_doc` and `Inner` stay private: the `tests` child module reaches
  them, and a `pub(super)` function returning the private `Inner` would trip
  `private_interfaces`. `agent_reference` converts `Inner` with `?` (snafu generates
  `From<Inner> for PlaybookError`).
- The cache holds the `Result`: a fault is computed once and every caller sees the same
  `&'static PlaybookError` (a `&E` is itself an `Error`, so it can be a snafu `source`).
- `if let … && let …` (let chains) needs edition 2024: the workspace is on 2024.
- `with_transform` requires `Clone` on the transform, and `Debug` keeps it printable:
  `CleanDocs` derives both. The transform reaches `$defs` (schemars 1.2 applies it to every
  subschema of the root), so no extra pass is needed.
- `#[path = "reference_tests.rs"]` inside `playbook/reference.rs` resolves relative to
  `src/playbook/`.
- `rest[..start]` slicing is on ASCII markers (`[`, `` ` ``): byte offsets from `find` are
  char boundaries.

In `playbook.rs`: `pub(crate) mod reference;` (crate-visible so the test helper reaches
`branch_name` by path; a `pub(crate) use` re-export would be unused outside tests and warn)
and `pub use reference::{ComponentReference, ExampleReference, PlaybookError, PropReference, agent_reference, render_components};`. In `lib.rs`: extend the `pub use playbook::{…}`
line with the six public names.

In `catalog_schema_tests.rs`, delete the local `branch_name` and call the production one, so
the crate reads a branch name in one place:

```rust
pub(crate) fn catalog() -> Vec<String> {
    let schema = serde_json::to_value(schemars::schema_for!(ComponentKind)).unwrap();
    schema["oneOf"].as_array().unwrap().iter().map(|b| crate::playbook::reference::branch_name(b).unwrap().to_owned()).collect()
}
```

- [ ] **Step 4: Run, review the JSON snapshot, accept it**

```bash
INSTA_UPDATE=new cargo nextest run -p oku-slides-a2ui reference
cat crates/oku-slides-a2ui/src/playbook/snapshots/*agent_reference_matches_its_snapshot*.snap.new
```

Every test but the snapshot must already PASS; `render_components_lists_exactly…` is an
exact assertion (`tone` shows only `accent`: the agent examples never write `neutral`, they
leave the tone out; spec decision 10). Read the pending JSON snapshot: every `description`
is one clean English sentence, every example `surface` is the tree of `entries.rs`. Accept
it by renaming the `.snap.new` file to `.snap` (or `cargo insta accept` if installed), then
run `cargo nextest run -p oku-slides-a2ui`: all PASS.

- [ ] **Step 5: Docs**

`crates/oku-slides-a2ui/ARCHITECTURE.md` is at 46 lines; these edits bring it to 49 (check
with `wc -l`, the cap is 50):
- "Prop types" bullet: replace "(the only type with a JSON schema, via `schemars`: its docs
  stay free of rustdoc links)" with "(its docs stay free of rustdoc links: MCP input schemas
  embed it outside the playbook's cleaning)".
- "Helpers on `ComponentKind`" bullet: after `` `actions` (actions it carries), `` insert
  `` `is_field` (a data-bound input, refused by `render`), ``.
- Add after the `SeriesSwatch` bullet, three lines:
  ``- `playbook` (`playbook.rs`, `playbook/`): `examples()`, every component in its variants
  (`agent` = `render` accepts it); `agent_reference()` and `render_components()`, the
  `JsonSchema` of `ComponentKind` cut to what the agent examples use, cached.``
- "Extension point", first sentence becomes: "A new component is added here (a
  `ComponentKind` variant, with its helpers, its `kind_info` arm and its playbook examples)
  **and** in `ui/src/a2ui/catalog.tsx`."
- "Dependencies": `` `serde`, `serde_json`, `schemars`, `snafu`; no internal dependency. ``

`crates/oku-slides-a2ui/AGENTS.md`, under "## Invariants":

```markdown
- Give every ComponentKind variant playbook examples, with agent true exactly when render accepts the example. (spec:2026-10-30-playbook-a2ui-design)
```

Run: `just agents-lint` — Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/oku-slides-a2ui Cargo.lock
git commit -F <scratchpad>/msg-3.txt
# feat(a2ui): the agent reference of the playbook, from the catalog schema (t-1312)
```

---

### Task 4: `a2ui_examples` tool and generated `render` description

**Files:**
- Create: `crates/oku-slides-mcp/src/a2ui_tools.rs`, `crates/oku-slides-mcp/src/a2ui_tools_tests.rs`
- Modify: `crates/oku-slides-mcp/src/lib.rs` (`mod a2ui_tools;`)
- Modify: `crates/oku-slides-mcp/src/tools.rs` (`list_all`: new entry, `render_description()`)
- Modify: `crates/oku-slides-mcp/src/handler.rs` (dispatch arm)
- Modify: `crates/oku-slides-mcp/src/tool_error.rs` (+ `tool_error_tests.rs` `KEYS`)
- Modify: `crates/oku-slides-mcp/src/tools_tests.rs` (tool lists, description test)
- Modify: `crates/oku-slides-i18n/src/locales/*/agent.ftl` (20 files)
- Modify: `crates/oku-slides-local-agent/src/families.rs` (+ `families_tests.rs`, `compact_tests.rs`)
- Modify: `crates/oku-slides-mcp/ARCHITECTURE.md`

The tool gets its own module, `a2ui_tools`, rather than a place in `read_tools` (the spec's
first draft): it reads the playbook, not the deck, and owns its refusal logic.

**Interfaces:**
- Consumes: `oku_slides_a2ui::{agent_reference, render_components, examples, ComponentKind, ComponentReference, PlaybookError}`
  (`agent_reference() -> Result<&'static [ComponentReference], &'static PlaybookError>`);
  `crate::tools::kinds_of`, `Kinds::has`, `Tools.kind: DocumentKind`.
- Produces: MCP tool `a2ui_examples`, input `{ components?: [String] }` (unknown keys refused,
  at most 25 names), output JSON `[ComponentReference]`;
  `ToolError::{A2uiUnknown, A2uiElsewhere, A2uiHostOnly, Playbook}` (three Fluent keys; a
  playbook fault reuses `agent-tool-failed`).

- [ ] **Step 1: Write the failing tests** (`a2ui_tools_tests.rs`)

```rust
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_session::DocumentKind;
use rstest::rstest;
use speculoos::prelude::*;

use super::{MAX_NAMES, examples_for};
use crate::tool_error::ToolError;

/// The component names of a reference.
fn names(r: &[oku_slides_a2ui::ComponentReference]) -> Vec<String> {
    r.iter().map(|c| c.component.clone()).collect()
}

/// `examples_for` in a deck tab with these names.
fn deck(wanted: &[&str]) -> Result<Vec<oku_slides_a2ui::ComponentReference>, ToolError> {
    let wanted: Vec<String> = wanted.iter().map(|w| (*w).to_owned()).collect();
    examples_for(Some(&wanted), DocumentKind::Deck)
}

#[test]
fn examples_for_no_list_returns_every_render_component() {
    let all = examples_for(None, DocumentKind::Deck).unwrap();
    assert_that!(names(&all)).is_equal_to(
        ["Column", "Card", "Text", "Row", "Button", "DiffText", "Disclosure", "Badge", "Markdown"]
            .map(str::to_owned)
            .to_vec(),
    );
}

#[test]
fn examples_for_a_list_keeps_only_those_components_in_catalog_order() {
    assert_that!(names(&deck(&["Badge", "Card"]).unwrap())).is_equal_to(vec!["Card".to_owned(), "Badge".to_owned()]);
}

#[test]
fn examples_for_an_empty_list_returns_nothing() {
    assert_that!(deck(&[]).unwrap()).is_empty();
}

#[rstest]
#[case::alone(&["Carousel"])]
#[case::mixed(&["Card", "Carousel"])]
fn examples_for_an_unknown_name_is_a2ui_unknown(#[case] wanted: &[&str]) {
    let e = deck(wanted).unwrap_err();
    assert_that!(matches!(e, ToolError::A2uiUnknown { ref name, .. } if name == "Carousel")).is_true();
}

#[test]
fn examples_for_an_unknown_name_echoes_it_sanitized_and_bounded() {
    let long = format!("<b>{}</b>", "x".repeat(200));
    let e = deck(&[&long]).unwrap_err();
    assert_that!(matches!(e, ToolError::A2uiUnknown { ref name, .. } if name.len() <= 32 && name.chars().all(|c| c.is_ascii_alphanumeric()))).is_true();
}

#[test]
fn examples_for_more_names_than_the_catalog_is_an_argument_error() {
    let many: Vec<String> = (0..=MAX_NAMES).map(|i| format!("C{i}")).collect();
    let e = examples_for(Some(&many), DocumentKind::Deck).unwrap_err();
    assert_that!(matches!(e, ToolError::Arguments { .. })).is_true();
}

#[rstest]
#[case::text_field("TextField", "show_params")]
#[case::list_table("ListTable", "show_params")]
#[case::slider("Slider", "show_params")]
#[case::choice_chips("ChoiceChips", "show_params")]
#[case::receipt("Receipt", "report")]
#[case::thumb("SlideThumb", "report")]
#[case::theme("ThemeChoice", "show_themes")]
fn examples_for_a_component_render_refuses_names_its_tool(#[case] name: &str, #[case] tool: &str) {
    let e = deck(&[name]).unwrap_err();
    assert_that!(matches!(e, ToolError::A2uiElsewhere { tool: t, .. } if t == tool)).is_true();
}

#[test]
fn examples_for_theme_choice_outside_a_deck_tab_is_host_only() {
    let wanted = vec!["ThemeChoice".to_owned()];
    let e = examples_for(Some(&wanted), DocumentKind::Pack).unwrap_err();
    assert_that!(matches!(e, ToolError::A2uiHostOnly { .. })).is_true();
}

#[rstest]
#[case::prompt("Prompt")]
#[case::timeline("Timeline")]
#[case::notice("Notice")]
#[case::chart_data("ChartData")]
#[case::table_data("TableData")]
fn examples_for_a_host_only_component_is_a2ui_host_only(#[case] name: &str) {
    let e = deck(&[name]).unwrap_err();
    assert_that!(matches!(e, ToolError::A2uiHostOnly { .. })).is_true();
}

#[test]
fn a2ui_refusal_is_translated_in_french_and_names_the_tool() {
    let e = deck(&["TextField"]).unwrap_err();
    let fr = e.localize(&Localizer::new(Some("fr")));
    let en = e.localize(&Localizer::new(Some("en")));
    assert_that!(fr.as_str()).is_not_equal_to(en.as_str());
    assert_that!(fr).contains("show_params");
}
```

Append to `tools_tests.rs`:

```rust
#[test]
fn tools_render_description_lists_the_playbook_components() {
    let tool = list().into_iter().find(|t| t.name == "render").unwrap();
    let components = oku_slides_a2ui::render_components(oku_slides_a2ui::agent_reference().unwrap());
    assert_that!(tool.description.unwrap_or_default())
        .contains(format!("Show a card composed of these A2UI components: {components}.").as_str());
}

#[test]
fn tools_render_description_points_to_a2ui_examples() {
    let tool = list().into_iter().find(|t| t.name == "render").unwrap();
    assert_that!(tool.description.unwrap_or_default()).contains("Call a2ui_examples");
}
```

Add `"a2ui_examples",` as the first entry of `DECK_TOOLS`, `PACK_TOOLS` and `THEME_TOOLS`
(sorted lists: `"a2ui…"` sorts before `"apply"`). In `tool_error_tests.rs` `KEYS`, add
`"agent-error-a2ui-unknown"`, `"agent-error-a2ui-elsewhere"`, `"agent-error-a2ui-host-only"`.
In `families_tests.rs`:

```rust
#[test]
fn families_of_a2ui_examples_is_ask_report() {
    assert_that!(families_of("a2ui_examples")).is_equal_to(&[Family::AskReport][..]);
}
```

In `compact_tests.rs` (the local agent keeps only a description's first sentence, so the
component list must be in it):

```rust
#[rstest::rstest]
#[case::render("render", "Markdown{text}")]
#[case::examples("a2ui_examples", "Props and full examples")]
fn compact_of_an_a2ui_tool_keeps_what_the_agent_needs(#[case] name: &str, #[case] kept: &str) {
    let (name, description) = oku_slides_mcp::tool_descriptions_for(oku_slides_session::DocumentKind::Deck)
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap();
    let spec = ToolSpec { name, description, schema: json!({ "type": "object" }) };
    assert_that!(compact(&spec).description).contains(kept);
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo nextest run -p oku-slides-mcp -p oku-slides-local-agent a2ui`
Expected: compile error, `examples_for` not found.

- [ ] **Step 3: Errors and Fluent keys**

In `tool_error.rs`, add to `ToolError`:

```rust
    #[snafu(display("unknown catalog component {name}"))]
    A2uiUnknown { name: String, known: String, backtrace: Backtrace },
    #[snafu(display("{name} is shown by {tool}, not render"))]
    A2uiElsewhere { name: String, tool: &'static str, backtrace: Backtrace },
    #[snafu(display("{name} is reserved to the host"))]
    A2uiHostOnly { name: String, backtrace: Backtrace },
    #[snafu(display("the playbook reference could not be built"))]
    Playbook { source: &'static oku_slides_a2ui::PlaybookError, backtrace: Backtrace },
```

and to `localize` (add the four names to its `use ToolError::{…}` list):

```rust
            A2uiUnknown { name, known, .. } => l.tr_with(
                "agent-error-a2ui-unknown",
                &[("name", name.as_str().into()), ("known", known.as_str().into())],
            ),
            A2uiElsewhere { name, tool, .. } => {
                l.tr_with("agent-error-a2ui-elsewhere", &[("name", name.as_str().into()), ("tool", (*tool).into())])
            }
            A2uiHostOnly { name, .. } => l.tr_with("agent-error-a2ui-host-only", &[("name", name.as_str().into())]),
            Playbook { .. } => l.tr("agent-tool-failed"),
```

Fluent keys, after `agent-error-not-shown` in every `agent.ftl` (same position in all 20).
`render`, `{ $name }`, `{ $known }`, `{ $tool }` stay as they are. Before pasting, compare
each locale's words for "component" and "app" with its other `agent-*` messages and use
theirs if they differ (`crates/oku-slides-i18n/AGENTS.md`); pt is Brazilian.
`every_error_key_is_translated_in_every_shipped_language` refuses an English copy.

| Locale | `agent-error-a2ui-unknown` | `agent-error-a2ui-elsewhere` | `agent-error-a2ui-host-only` |
|---|---|---|---|
| en | { $name } is not a component render accepts. Components: { $known }. | render does not show { $name }: use { $tool }. | { $name } is reserved to the app: render cannot show it. |
| fr | { $name } n'est pas un composant que render accepte. Composants : { $known }. | render ne montre pas { $name } : utilisez { $tool }. | { $name } est réservé à l'application : render ne peut pas l'afficher. |
| ar | { $name } ليس مكوّنًا يقبله render. المكوّنات: { $known }. | لا يعرض render ‏{ $name }: استخدم { $tool }. | { $name } مخصّص للتطبيق: لا يستطيع render عرضه. |
| bn | { $name } এমন কোনো কম্পোনেন্ট নয় যা render গ্রহণ করে। কম্পোনেন্ট: { $known }। | render { $name } দেখায় না: { $tool } ব্যবহার করুন। | { $name } অ্যাপের জন্য সংরক্ষিত: render এটি দেখাতে পারে না। |
| de | { $name } ist keine Komponente, die render akzeptiert. Komponenten: { $known }. | render zeigt { $name } nicht an: Verwende { $tool }. | { $name } ist der App vorbehalten: render kann es nicht anzeigen. |
| es | { $name } no es un componente que render acepte. Componentes: { $known }. | render no muestra { $name }: usa { $tool }. | { $name } está reservado a la aplicación: render no puede mostrarlo. |
| hi | { $name } ऐसा कंपोनेंट नहीं है जिसे render स्वीकार करता है। कंपोनेंट: { $known }। | render { $name } नहीं दिखाता: { $tool } का उपयोग करें। | { $name } ऐप के लिए आरक्षित है: render इसे नहीं दिखा सकता। |
| it | { $name } non è un componente accettato da render. Componenti: { $known }. | render non mostra { $name }: usa { $tool }. | { $name } è riservato all'app: render non può mostrarlo. |
| ja | { $name } は render が受け付けるコンポーネントではありません。コンポーネント: { $known }。 | render は { $name } を表示しません: { $tool } を使ってください。 | { $name } はアプリ専用です: render では表示できません。 |
| ko | { $name }은(는) render가 받는 컴포넌트가 아닙니다. 컴포넌트: { $known }. | render는 { $name }을(를) 표시하지 않습니다: { $tool }을(를) 사용하세요. | { $name }은(는) 앱 전용입니다: render로 표시할 수 없습니다. |
| nl | { $name } is geen component die render accepteert. Componenten: { $known }. | render toont { $name } niet: gebruik { $tool }. | { $name } is voorbehouden aan de app: render kan het niet tonen. |
| pl | { $name } nie jest komponentem akceptowanym przez render. Komponenty: { $known }. | render nie wyświetla { $name }: użyj { $tool }. | { $name } jest zarezerwowany dla aplikacji: render nie może go wyświetlić. |
| pt | { $name } não é um componente aceito por render. Componentes: { $known }. | render não mostra { $name }: use { $tool }. | { $name } é reservado ao app: render não pode exibi-lo. |
| ru | { $name } — не компонент, который принимает render. Компоненты: { $known }. | render не показывает { $name }: используйте { $tool }. | { $name } зарезервирован для приложения: render не может его показать. |
| th | { $name } ไม่ใช่คอมโพเนนต์ที่ render รับ คอมโพเนนต์: { $known } | render ไม่แสดง { $name }: ใช้ { $tool } | { $name } สงวนไว้สำหรับแอป: render แสดงไม่ได้ |
| tr | { $name }, render'ın kabul ettiği bir bileşen değil. Bileşenler: { $known }. | render { $name } öğesini göstermez: { $tool } kullanın. | { $name } uygulamaya ayrılmıştır: render onu gösteremez. |
| uk | { $name } — не компонент, який приймає render. Компоненти: { $known }. | render не показує { $name }: використайте { $tool }. | { $name } зарезервовано для застосунку: render не може його показати. |
| vi | { $name } không phải là thành phần mà render chấp nhận. Thành phần: { $known }. | render không hiển thị { $name }: hãy dùng { $tool }. | { $name } dành riêng cho ứng dụng: render không thể hiển thị nó. |
| zh-Hans | { $name } 不是 render 接受的组件。组件：{ $known }。 | render 不显示 { $name }：请使用 { $tool }。 | { $name } 为应用保留：render 无法显示它。 |
| zh-Hant | { $name } 不是 render 接受的元件。元件：{ $known }。 | render 不顯示 { $name }：請使用 { $tool }。 | { $name } 保留給應用程式：render 無法顯示它。 |

In the `.ftl` files each cell is one line, `agent-error-a2ui-unknown = <cell>`. The `‏` in the
ar row is a right-to-left mark before the placeable; keep it.

- [ ] **Step 4: The tool** (`a2ui_tools.rs`)

```rust
//! `a2ui_examples`: the props and examples of the components `render` accepts.

use oku_slides_a2ui::{ComponentKind, ComponentReference, agent_reference, examples};
use oku_slides_session::DocumentKind;
use rust_mcp_sdk::schema::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::de::Error as _;
use serde_json::Value;
use snafu::{IntoError, ResultExt};

use crate::tool_error::{
    A2uiElsewhereSnafu, A2uiHostOnlySnafu, A2uiUnknownSnafu, ArgumentsSnafu, PlaybookSnafu, ToolError,
};
use crate::tools::{Tools, kinds_of};

/// At most this many names per call: the catalog's size.
pub(crate) const MAX_NAMES: usize = 25;

/// The longest name echoed back in a refusal; catalog names are shorter.
const MAX_ECHO: usize = 32;

/// The components `a2ui_examples` reads.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct A2uiExamplesArgs {
    /// Component names, such as `Card`; absent: every component render accepts.
    #[serde(default)]
    pub components: Option<Vec<String>>,
}

impl Tools {
    /// Answers `a2ui_examples` for this tab's kind.
    pub(crate) fn a2ui_examples(&self, args: Value) -> CallToolResult {
        let args: A2uiExamplesArgs = match self.parse(args) {
            Ok(a) => a,
            Err(reply) => return reply,
        };
        self.reply(examples_for(args.components.as_deref(), self.kind), |r| self.json_result(&r))
    }
}

/// The reference of `wanted` (absent: all), in catalog order, or why a name is refused.
pub(crate) fn examples_for(wanted: Option<&[String]>, kind: DocumentKind) -> Result<Vec<ComponentReference>, ToolError> {
    let reference = agent_reference().context(PlaybookSnafu)?;
    let Some(wanted) = wanted else { return Ok(reference.to_vec()) };
    if wanted.len() > MAX_NAMES {
        let detail = serde_json::Error::custom(format!("components: at most {MAX_NAMES} names"));
        return Err(ArgumentsSnafu.into_error(detail));
    }
    if let Some(missing) = wanted.iter().find(|w| !reference.iter().any(|c| &c.component == *w)) {
        return Err(refusal(missing, reference, kind));
    }
    Ok(reference.iter().filter(|c| wanted.contains(&c.component)).cloned().collect())
}

/// Why `render` does not take `wanted`: unknown, shown by another tool, or host only.
fn refusal(wanted: &str, reference: &[ComponentReference], kind: DocumentKind) -> ToolError {
    let name: String = wanted.chars().filter(char::is_ascii_alphanumeric).take(MAX_ECHO).collect();
    let root = examples().into_iter().find(|e| e.component == wanted).map(|e| e.components[0].kind.clone());
    let themes_here = kinds_of("show_themes").is_some_and(|k| k.has(kind));
    match root {
        None => {
            let known = reference.iter().map(|c| c.component.as_str()).collect::<Vec<_>>().join(", ");
            A2uiUnknownSnafu { name, known }.build()
        }
        Some(k) if k.is_field() => A2uiElsewhereSnafu { name, tool: "show_params" }.build(),
        Some(ComponentKind::Receipt { .. } | ComponentKind::SlideThumb { .. }) => {
            A2uiElsewhereSnafu { name, tool: "report" }.build()
        }
        Some(ComponentKind::ThemeChoice { .. }) if themes_here => A2uiElsewhereSnafu { name, tool: "show_themes" }.build(),
        Some(_) => A2uiHostOnlySnafu { name }.build(),
    }
}

#[cfg(test)]
#[path = "a2ui_tools_tests.rs"]
mod tests;
```

Check while writing: `ArgumentsSnafu` is `pub(crate)` (the enum has
`#[snafu(visibility(pub(crate)))]`); `kinds_of` is `pub(crate)` in `tools.rs` (it is);
`Tools.kind` is the tab's `DocumentKind` (it is, `pub(crate) kind`). A `show_params` refusal
needs no tab check: `show_params` is listed in every tab kind (verify with
`kinds_of("show_params")`; if it is not, apply the same `has(kind)` test as `show_themes`
and add a test case for it).

`lib.rs`: `mod a2ui_tools;` (alphabetical, before `mod app_tools;`). `handler.rs`, next to
`"focus" => self.focus(args),`: `"a2ui_examples" => self.a2ui_examples(args),`.
`families.rs`: extend the `AskReport` arm:
`"ask_choice" | "ask_confirm" | "ask_rewrite" | "report" | "render" | "a2ui_examples" => &[AskReport],`.

- [ ] **Step 5: Listing and the generated `render` description** (`tools.rs`)

Add after `import_diff_description`:

```rust
/// The description of `render`, its component list generated from the playbook.
///
/// The list is in the first sentence: the local agent keeps only that one. A playbook fault
/// (a programming error, caught by the a2ui tests) is logged, and the description then
/// leaves the list to `a2ui_examples`, which reports the fault to the agent.
fn render_description() -> String {
    let components = match oku_slides_a2ui::agent_reference() {
        Ok(r) => format!("Show a card composed of these A2UI components: {}.", oku_slides_a2ui::render_components(r)),
        Err(e) => {
            tracing::error!(error = %e, "playbook reference unavailable for the render description");
            "Show a card composed of A2UI components (call a2ui_examples for the list).".to_owned()
        }
    };
    format!(
        "{components} Call a2ui_examples for a component's props and full examples. Shape: v0.9 [{{id, component, …}}], one id \"root\". All values literal (no {{path}}). Only action, on a Button whose label is the prompt: {{event: {{name: \"suggest\", context: {{prompt: <the label>}}}}}}. No primary/danger/icon styles. For a turn's conclusion use report (Receipt and slide thumbnails are reserved to it). At most {RENDER_MAX} components forming a tree (each id used once), {RENDER_DEPTH} levels, 64 KiB. Use it only when report, ask_* and show_* do not fit. Errors say what to fix."
    )
}
```

with `use oku_slides_session::agent::render::{RENDER_DEPTH, RENDER_MAX};` at its top (the
existing `tools_render_description_states_the_real_limits` test reads these constants from
that path: copy its `use`). Replace the `render` entry of `list_all` with
`(tool::<oku_slides_session::agent::Render>("render", &render_description()), Kinds::ALL),`
and add right after it:

```rust
            (tool::<crate::a2ui_tools::A2uiExamplesArgs>("a2ui_examples", "Props and full examples of the components render accepts. Output: [{component, props [{name, required, type, values?, description}], examples [{variant, note, surface}]}]; surface is a valid render input. components: the names to read (absent: all, at most 25). A catalog component render refuses names the tool that shows it."), Kinds::ALL),
```

- [ ] **Step 6: Run the tests**

```bash
cargo nextest run -p oku-slides-mcp -p oku-slides-local-agent -p oku-slides-session -p oku-slides-i18n
```
Expected: all PASS, including `tools_render_description_states_the_real_limits`,
`families_every_mcp_tool_has_a_family` and the 20-locale key test.

- [ ] **Step 7: Docs** — `crates/oku-slides-mcp/ARCHITECTURE.md` is at 50 lines: on the
"Tool families" bullet, after `` `surface_tools` (…), `` insert
`` `a2ui_tools` (`a2ui_examples`: the playbook's agent reference, cached), ``. Rewrap only
that bullet; if it then grows by a line, join the `tool_error` and `error` bullets into one
bullet so the file stays at 50 (`wc -l`). Run `just agents-lint`.

- [ ] **Step 8: Commit**

```bash
git add crates/oku-slides-mcp crates/oku-slides-i18n/src/locales crates/oku-slides-local-agent/src
git commit -F <scratchpad>/msg-4.txt
# feat(mcp,local-agent,i18n): a2ui_examples and a render description generated from the playbook (t-1312)
```

---

### Task 5: Dev gallery page

**Files:**
- Create: `ui/playbook.html`
- Create: `ui/src/playbook/main.tsx`, `ui/src/playbook/Playbook.tsx`, `ui/src/playbook/playbook.css`
- Create: `ui/src/playbook/playbookEntries.ts`, `ui/src/playbook/playbookEntries.test.ts`
- Create: `ui/src/playbook/playbook.schemas.render.test.tsx`, `ui/src/playbook/Playbook.render.test.tsx`
- Modify: `ui/AGENTS.md` (the English-literals exception), root `AGENTS.md` (Commands)

**Interfaces:**
- Consumes: `ui/src/a2ui/fixtures/playbook.json` (Task 2): entries
  `{component, variant, note, group, agent, surface, messages}`; `SurfaceView`
  (`ui/src/a2ui/SurfaceView.tsx`, props `{messages, surfaceId, onAction, onError}`);
  `Button`, `Field` (`ui/src/primitives.tsx`); `applyDefaultScheme` (`ui/src/appearance.ts`),
  `currentScheme`, `type Scheme` (`ui/src/scheme.ts`).
- Produces: `/playbook.html` in `just web`.

The page uses the app's own scheme switch, `applyDefaultScheme` (the spec's first draft
named it `applyScheme`, which does not exist).

- [ ] **Step 1: Write the failing pure tests** (`playbookEntries.test.ts`, first Bun pass, no DOM)

```ts
import { describe, expect, test } from "bun:test";
import { GROUPS, byGroup, componentsOf, visible, type PlaybookEntry } from "./playbookEntries";

const entry = (component: string, group: PlaybookEntry["group"]): PlaybookEntry => ({
  component, variant: "default", note: "", group, agent: false, surface: `playbook-${component}-default`,
  messages: [{ version: "v0.9", updateComponents: { surfaceId: "s", components: [{ id: "root", component }] } }] as never,
});

describe("playbook entries", () => {
  test("visible with a filter keeps matching components, case-insensitive", () => {
    const all = [entry("Button", "atom"), entry("Badge", "atom"), entry("Row", "layout")];
    expect(visible(all, "bu").map((e) => e.component)).toEqual(["Button"]);
  });

  test("visible with a blank filter keeps everything", () => {
    expect(visible([entry("Row", "layout")], "  ")).toHaveLength(1);
  });

  test("byGroup returns the four groups in gallery order", () => {
    const groups = byGroup([entry("Row", "layout"), entry("Receipt", "host_only")]);
    expect(groups.map((g) => [g.title, g.entries.length])).toEqual([
      ["Layout", 1], ["Atoms", 0], ["Fields", 0], ["Host only", 1],
    ]);
  });

  test("componentsOf reads the updateComponents message", () => {
    expect(componentsOf(entry("Row", "layout"))).toEqual([{ id: "root", component: "Row" }]);
  });

  test("GROUPS names every group once", () => {
    expect(GROUPS.map((g) => g.group)).toEqual(["layout", "atom", "field", "host_only"]);
  });
});
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd ui && bun test src/playbook/playbookEntries.test.ts`
Expected: FAIL, cannot find module `./playbookEntries`.

- [ ] **Step 3: Write `playbookEntries.ts`**

```ts
// The playbook's entries (fixture written by oku-slides-a2ui) and how the gallery groups them.

import type { A2uiMessage } from "@a2ui/web_core/v0_9";

/** A gallery group, as the Rust `Group` serializes it. */
export type PlaybookGroup = "layout" | "atom" | "field" | "host_only";

/** One example: a component in one variant, as v0.9 messages. */
export interface PlaybookEntry {
  component: string;
  variant: string;
  note: string;
  group: PlaybookGroup;
  agent: boolean;
  surface: string;
  messages: A2uiMessage[];
}

/** One gallery section: a group, its title and its entries. */
export interface PlaybookSection {
  group: PlaybookGroup;
  title: string;
  entries: PlaybookEntry[];
}

/** The groups in gallery order, with their titles. */
export const GROUPS: readonly { group: PlaybookGroup; title: string }[] = [
  { group: "layout", title: "Layout" },
  { group: "atom", title: "Atoms" },
  { group: "field", title: "Fields" },
  { group: "host_only", title: "Host only" },
];

/** The entries whose component name contains `filter`, ignoring case. */
export function visible(entries: PlaybookEntry[], filter: string): PlaybookEntry[] {
  const f = filter.trim().toLowerCase();
  return f ? entries.filter((e) => e.component.toLowerCase().includes(f)) : entries;
}

/** The entries of each group, in gallery order, empty groups included. */
export function byGroup(entries: PlaybookEntry[]): PlaybookSection[] {
  return GROUPS.map(({ group, title }) => ({ group, title, entries: entries.filter((e) => e.group === group) }));
}

/** The components of an entry, as its updateComponents message carries them. */
export function componentsOf(entry: PlaybookEntry): unknown[] {
  return entry.messages.flatMap((m) => ("updateComponents" in m ? m.updateComponents.components : []));
}
```

If `"updateComponents" in m` does not narrow `A2uiMessage` under tsc, read it as
`(m as { updateComponents?: { components: unknown[] } }).updateComponents?.components ?? []`.

- [ ] **Step 4: Run it** — `cd ui && bun test src/playbook/playbookEntries.test.ts` — Expected: PASS.

- [ ] **Step 5: Write the failing render tests**

`playbook.schemas.render.test.tsx`:

```tsx
// Every playbook example passes the catalog: the processor and each component's zod schema.

import { describe, expect, test } from "bun:test";
import "../test/dom";
import { MessageProcessor } from "@a2ui/web_core/v0_9";
import fixture from "../a2ui/fixtures/playbook.json";

// Bun does not evaluate static imports in written order: the catalog loads after `../test/dom`.
const { CATALOG_ID, catalog } = await import("../a2ui/catalog");
const { componentsOf } = await import("./playbookEntries");
type Entry = import("./playbookEntries").PlaybookEntry;
const entries = fixture as unknown as Entry[];

describe("playbook fixture", () => {
  test("covers exactly the catalog's components", () => {
    const shown = [...new Set(entries.map((e) => e.component))].sort();
    expect(shown).toEqual([...catalog.components.keys()].sort());
  });

  test("every example builds a surface with a root", () => {
    const missing = entries.filter((e) => {
      const processor = new MessageProcessor([catalog]);
      processor.processMessages(e.messages as Parameters<typeof processor.processMessages>[0]);
      const model = processor.model.surfacesMap.get(e.surface);
      const ok = model?.catalog.id === CATALOG_ID && model.componentsModel.get("root") !== undefined;
      model?.dispose();
      return !ok;
    });
    expect(missing.map((e) => `${e.component}/${e.variant}`)).toEqual([]);
  });

  test("every component's props pass its zod schema", () => {
    const failures = entries.flatMap((e) =>
      (componentsOf(e) as { id: string; component: string }[])
        .filter(({ id: _id, component, ...props }) => !catalog.components.get(component)?.schema.safeParse(props).success)
        .map((c) => `${e.component}/${e.variant} ${c.id}`),
    );
    expect(failures).toEqual([]);
  });
});
```

Check against `catalog.schemas.render.test.tsx` that `catalog.components` is a `Map` keyed
by component name and that a surface model has `dispose()` (`SurfaceView.tsx` calls it); if
the processor itself has a `dispose`, call that instead.

`Playbook.render.test.tsx`:

```tsx
// The gallery mounts every example without a surface error and logs the actions sent.

import { afterEach, describe, expect, test } from "bun:test";
import "../test/dom";
import fixture from "../a2ui/fixtures/playbook.json";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { Playbook } = await import("./Playbook");
type Entry = import("./playbookEntries").PlaybookEntry;

let cleanup: (() => Promise<void>) | null = null;
afterEach(async () => {
  await cleanup?.();
  cleanup = null;
});

async function mountGallery(): Promise<HTMLElement> {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(createElement(Playbook, { entries: fixture as unknown as Entry[] })));
  cleanup = async () => {
    await act(async () => root.unmount());
    host.remove();
  };
  return host;
}

describe("playbook gallery", () => {
  test("shows the four groups in order", async () => {
    const host = await mountGallery();
    const titles = [...host.querySelectorAll(".playbook-group > h2")].map((h) => h.textContent);
    expect(titles).toEqual(["Layout", "Atoms", "Fields", "Host only"]);
  });

  test("renders one card per example", async () => {
    const host = await mountGallery();
    expect(host.querySelectorAll(".playbook-example")).toHaveLength(fixture.length);
  });

  test("reports no surface error", async () => {
    const host = await mountGallery();
    expect([...host.querySelectorAll(".playbook-error")].map((e) => e.textContent)).toEqual([]);
  });

  test("draws something in every surface", async () => {
    const host = await mountGallery();
    const empty = [...host.querySelectorAll<HTMLElement>(".playbook-example")]
      .filter((card) => !card.querySelector(".playbook-surface *"))
      .map((card) => card.dataset.example);
    expect(empty).toEqual([]);
  });

  test("a click on the suggest button logs its event and prompt", async () => {
    const host = await mountGallery();
    const button = host.querySelector<HTMLButtonElement>('[data-example="Button/suggest"] .playbook-surface button')!;
    await act(async () => button.click());
    const logged = host.querySelector(".playbook-log")?.textContent ?? "";
    expect(logged).toContain('"name":"suggest"');
    expect(logged).toContain('"prompt":"Make it shorter"');
  });
});
```

(The last test checks one behavior, the logged action, through its two fields.)

Run: `cd ui && bun test .render.test. --test-name-pattern playbook`
Expected: FAIL, cannot find module `./Playbook`.

- [ ] **Step 6: Write `Playbook.tsx`**

```tsx
// The dev gallery: every playbook example rendered live, by group, with an action log.

import { Component, useRef, useState, type ErrorInfo, type ReactNode } from "react";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import { SurfaceView } from "../a2ui/SurfaceView";
import { applyDefaultScheme } from "../appearance";
import { Button, Field } from "../primitives";
import { currentScheme, type Scheme } from "../scheme";
import { byGroup, componentsOf, visible, type PlaybookEntry } from "./playbookEntries";

/** The most recent actions kept in the log. */
const LOG_SIZE = 50;

/** One logged action, with a stable key. */
interface LogLine {
  id: number;
  text: string;
}

/** Keeps one example's crash inside its card instead of unmounting the gallery. */
class ExampleBoundary extends Component<{ children: ReactNode }, { error: string | null }> {
  state = { error: null as string | null };

  static getDerivedStateFromError(e: unknown): { error: string } {
    return { error: e instanceof Error ? e.message : String(e) };
  }

  componentDidCatch(e: unknown, info: ErrorInfo): void {
    console.error("playbook example", e, info.componentStack);
  }

  render(): ReactNode {
    return this.state.error ? (
      <p role="alert" className="playbook-error">
        {this.state.error}
      </p>
    ) : (
      this.props.children
    );
  }
}

/** One example: its name, note, live surface, error and JSON. */
function Example({ entry, onAction }: { entry: PlaybookEntry; onAction: (a: A2uiClientAction) => void }): ReactNode {
  const [error, setError] = useState<string | null>(null);
  return (
    <article className="playbook-example" data-example={`${entry.component}/${entry.variant}`}>
      <header>
        <h3>{entry.component}</h3>
        <span className="playbook-variant">{entry.variant}</span>
        {entry.agent && <span className="playbook-agent">agent</span>}
      </header>
      <p className="playbook-note">{entry.note}</p>
      <div className="playbook-surface">
        <ExampleBoundary>
          <SurfaceView messages={entry.messages} surfaceId={entry.surface} onAction={onAction} onError={setError} />
        </ExampleBoundary>
      </div>
      {error && (
        <p role="alert" className="playbook-error">
          {error}
        </p>
      )}
      <details>
        <summary>JSON</summary>
        <pre>{JSON.stringify(componentsOf(entry), null, 2)}</pre>
      </details>
    </article>
  );
}

/** The gallery of `entries`, filtered by component name. */
export function Playbook({ entries }: { entries: PlaybookEntry[] }): ReactNode {
  const [filter, setFilter] = useState("");
  const [scheme, setScheme] = useState<Scheme>(currentScheme);
  const [log, setLog] = useState<LogLine[]>([]);
  const nextId = useRef(0);
  const toggleScheme = (): void => {
    const next: Scheme = scheme === "dark" ? "light" : "dark";
    applyDefaultScheme(next);
    setScheme(next);
  };
  const logFrom = (entry: PlaybookEntry) => (a: A2uiClientAction): void => {
    const line = { id: nextId.current++, text: `${entry.component}/${entry.variant} ${JSON.stringify(a)}` };
    setLog((l) => [line, ...l].slice(0, LOG_SIZE));
  };
  return (
    <main className="playbook">
      <header className="playbook-bar">
        <h1>A2UI playbook</h1>
        <Field variant="menu" label="Filter">
          <input aria-label="Filter components" value={filter} onChange={(e) => setFilter(e.target.value)} />
        </Field>
        <Button onClick={toggleScheme}>{scheme === "dark" ? "Light" : "Dark"}</Button>
      </header>
      <ol className="playbook-log" aria-label="Actions">
        {log.map((line) => (
          <li key={line.id}>{line.text}</li>
        ))}
      </ol>
      {byGroup(visible(entries, filter)).map(({ group, title, entries: shown }) => (
        <section key={group} className="playbook-group">
          <h2>{title}</h2>
          {shown.map((e) => (
            <Example key={e.surface} entry={e} onAction={logFrom(e)} />
          ))}
        </section>
      ))}
    </main>
  );
}
```

`playbook.css` (tokens from `ui/src/styles.css`: `--paper-000` the desk, `--ink`, `--sans`,
`--danger`; `body` there has `overflow: hidden` and `user-select: none`, so the page is its
own scroll container):

```css
/* The dev gallery's layout; components keep the app's own styles. */
.playbook { height: 100vh; overflow-y: auto; user-select: text; box-sizing: border-box; padding: 0 max(16px, calc((100% - 1100px) / 2)) 32px; font-family: var(--sans); color: var(--ink); background: var(--paper-000); }
.playbook-bar { position: sticky; top: 0; z-index: 1; display: flex; gap: 12px; align-items: center; padding: 8px 0; background: var(--paper-000); }
.playbook-bar h1 { flex: 1; font-size: 18px; margin: 0; }
.playbook-log { max-height: 120px; overflow: auto; font: 12px ui-monospace, monospace; }
.playbook-group { display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 16px; margin-bottom: 24px; }
.playbook-group > h2 { grid-column: 1 / -1; margin: 8px 0 0; }
.playbook-example { border: 1px solid color-mix(in oklab, var(--ink) 15%, transparent); border-radius: 8px; padding: 12px; }
.playbook-example header { display: flex; gap: 8px; align-items: baseline; }
.playbook-example h3 { margin: 0; }
.playbook-agent { font-size: 11px; padding: 0 6px; border-radius: 4px; background: color-mix(in oklab, var(--ink) 12%, transparent); }
.playbook-error { color: var(--danger); }
.playbook-example pre { max-height: 240px; overflow: auto; font-size: 11px; }
```

- [ ] **Step 7: Entry point and HTML**

`ui/src/playbook/main.tsx`:

```tsx
// Dev-only entry of the A2UI playbook, never part of the app build.
// Opened with `just web`, then /playbook.html.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import fixture from "../a2ui/fixtures/playbook.json";
import { applyDefaultScheme } from "../appearance";
import { loadMessages } from "../i18n";
import { currentScheme, loadScheme } from "../scheme";
import { Playbook } from "./Playbook";
import type { PlaybookEntry } from "./playbookEntries";
import "../styles.css";
import "../a2ui/a2ui.css";
import "./playbook.css";

Promise.all([
  loadMessages().catch((e) => console.error("interface messages", e)),
  loadScheme().catch((e) => console.error("interface scheme", e)),
]).finally(() => {
  applyDefaultScheme(currentScheme());
  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <Playbook entries={fixture as unknown as PlaybookEntry[]} />
    </StrictMode>,
  );
});
```

Check the import paths and the startup calls against `ui/src/main.tsx` (the spec says the
page loads i18n and the scheme as it does) and copy its exact names if they differ.

`ui/playbook.html`:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>A2UI playbook</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/playbook/main.tsx"></script>
  </body>
</html>
```

Do not touch `vite.config.ts`: without `build.rollupOptions.input`, `vite build` bundles only
`index.html`.

- [ ] **Step 8: The English-literals exception** — `ui/AGENTS.md`, under "## Invariants",
right after the `t()` rule:

```markdown
- Keep the dev-only playbook page (ui/playbook.html, ui/src/playbook) in English literals without t(); it never ships. (spec:2026-10-30-playbook-a2ui-design)
```

Run `just agents-lint`: Expected PASS.

- [ ] **Step 9: Run the UI checks**

```bash
cd ui && bun test src/playbook && bun test .render.test. && bunx tsc -b && bun run build && test -f dist/index.html && test ! -e dist/playbook.html && echo "playbook not shipped"
```
Expected: tests PASS, tsc clean, the last line prints `playbook not shipped`.

- [ ] **Step 10: See it** — start `just web` with fresh data dirs
(`OKU_SLIDES_PATHS__DATA=$(mktemp -d) OKU_SLIDES_PATHS__DECKS=$(mktemp -d) just web`, in the
background), open `http://localhost:5173/playbook.html` in the browser pane, screenshot the
four groups in light and dark, scroll to the bottom (the page must scroll), click the
`Button/suggest` button and check the log, then stop the server.

- [ ] **Step 11: Commands line** — in the root `AGENTS.md` code block under "## Commands",
append to the existing `just web` line's comment, so it reads:

```bash
just web           # UI with HMR on :5173 + Rust API; /playbook.html shows every A2UI component
```

then `just agents-lint`.

- [ ] **Step 12: Full gate**

Run: `just check` then `just test` (background; long) — Expected: both PASS.

- [ ] **Step 13: Commit**

```bash
git add ui/playbook.html ui/src/playbook ui/AGENTS.md AGENTS.md
git commit -F <scratchpad>/msg-5.txt
# feat(ui): the A2UI playbook dev gallery (t-1312)
```

---

## Oku sub-tasks

One sub-task of t-1312 per task above, titled exactly as the task headings, created before
Task 1 starts:

```bash
for title in "Catalog JSON schema" "Playbook examples, UI fixture, render acceptance" \
  "Agent reference from the schema" "\`a2ui_examples\` tool and generated \`render\` description" \
  "Dev gallery page"; do
  oku cli task create --project slider --parent t-1312 --name "$title"
done
```

Mark each sub-task done (`oku cli task done --project slider --id <id>`) as its task's
commit lands, not in a batch.
