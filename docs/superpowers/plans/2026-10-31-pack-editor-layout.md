# Pack Editor Layout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reorganise a pack tab's left part (slim item list, preview and form side by side, one-line header), add a preview slot picker, and make the parameter form readable (localized labels, help, a switch for flags).

**Architecture:** The preview slot is a `PreviewSpan` the UI keeps like `code` and sends to `preview_widget`, which narrows the preview element's grid cell. Parameter labels become `LocalizedText` with an optional `help`, resolved in the session (schema text, then Fluent keys for built-in packs, then the name). A new A2UI `Switch` replaces the Yes/No select for flags. The layout itself is CSS grid areas in `pack.css` plus a `rows` variant of `SlidePile`.

**Tech Stack:** Rust (session, core, a2ui, i18n crates; snafu, serde, rstest, speculoos), Typst (`typst/oku-slides/src`), React/TS (Bun tests, happy-dom render tests), Playwright e2e (`ui/e2e/*.cjs`).

**Spec:** `docs/superpowers/specs/2026-10-31-pack-editor-layout-design.md` (amended while planning, see its "Amendments" section).

## Global Constraints

- Every commit: `type(scope): description (t-1392)`, English, Conventional Commits; no `Co-Authored-By` line.
- No `unwrap`/`expect` outside tests; no `#[allow]`, use `#[expect(..., reason = "...")]`.
- Every new `pub` item and module documented, first sentence under 15 words.
- Rust unit tests in a sibling `x_tests.rs` included by `#[cfg(test)] #[path = "x_tests.rs"] mod x_tests;`, asserting with speculoos, named `unit_scenario_result`, rstest for tables.
- Every new Fluent key goes into all 20 locales (`crates/oku-slides-i18n/src/locales/*/`), translated (never the English text copied), same key order everywhere; English first.
- Change `ComponentKind` (Rust) and `ui/src/a2ui/catalog.tsx` in the same commit.
- Source files stay under 1000 lines (aim 500).
- Keep each `ARCHITECTURE.md` at most 50 lines and current, committed with the code it describes.
- Every run outside `just e2e` sets `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS` to a fresh temp dir.
- In this worktree the first nextest build is cold (over 10 min): run it in the background.

## File Map

| File | Change |
|---|---|
| `crates/oku-slides-session/src/preview_span.rs` (+ `_tests.rs`) | new: `PreviewSpan` and its column count |
| `crates/oku-slides-session/src/widget_preview.rs` | `preview_widget(.., span)`, `preview_deck` cell |
| `crates/oku-slides-session/src/request.rs` | `PreviewWidget { span }` |
| `crates/oku-slides-session/src/views.rs` | `UiState.preview_span` |
| `crates/oku-slides-mcp/src/widget_tools.rs` | passes `PreviewSpan::Full` |
| `crates/oku-slides-core/src/widget.rs` | `ParamSchema.label: Option<LocalizedText>`, `help` |
| `typst/oku-slides/src/params.typ`, `widget.typ` | `help:` on every helper; checked like a widget label |
| `crates/oku-slides-session/src/param_fields.rs` | `param_label`, `param_help`; flag → `Switch` |
| `crates/oku-slides-session/src/agent/forms.rs` | `ParamField.help`, help caption, `FieldKind::Switch` |
| `crates/oku-slides-session/src/fields.rs` | `field_views(.., locale)` |
| `crates/oku-slides-session/src/widget_params.rs` | kind passed to the field, title from `kind_name` |
| `crates/oku-slides-i18n/src/locales/*/session.ftl` | built-in parameter labels and help |
| `crates/oku-slides-i18n/src/locales/*/ui.ftl` | preview label and slot picker |
| `crates/oku-slides-a2ui/src/component.rs`, `playbook/entries.rs`, `playbook.rs`, `catalog_schema_tests.rs` | `Switch` |
| `ui/src/a2ui/catalog.tsx`, `a2ui.css`, `fixtures/playbook.json` | `Switch` |
| `ui/src/api.ts`, `useKeptUi.ts` | `PreviewSpan`, `preview_span`, `previewWidget(.., span)` |
| `ui/src/PreviewSpanMenu.tsx` (+ render test) | new: the slot select |
| `ui/src/PackPanels.tsx`, `WidgetPane.tsx`, `WidgetTab.tsx` | span plumbing, header row |
| `ui/src/pack.css`, `styles.css` | grid areas, header row, rows pile |
| `ui/src/SlidePile.tsx`, `PackPile.tsx` | `variant: "rows"` for widget packs |
| `ui/e2e/170-code-layout.cjs`, `180-widget-values.cjs` | follow the new layout and labels |
| `ui/e2e/310-pack-editor-layout.cjs` | new e2e spec |

---

### Task 1: Preview slot in the session (`PreviewSpan`)

**Files:**
- Create: `crates/oku-slides-session/src/preview_span.rs`, `crates/oku-slides-session/src/preview_span_tests.rs`
- Modify: `crates/oku-slides-session/src/lib.rs` (mod + `pub use`), `widget_preview.rs:299-374`, `request.rs:339-349` and `:521-522`, `views.rs:160-185`, `crates/oku-slides-mcp/src/widget_tools.rs:249`
- Modify (callers, mechanical): every `.preview_widget(` call in `crates/oku-slides-session/src/*_tests.rs` and `crates/oku-slides-session/tests/regressions/issue_0524_*.rs`
- Test: `preview_span_tests.rs`, `widget_preview_tests.rs`, `views_tests.rs`

**Interfaces:**
- Produces: `pub enum PreviewSpan { Full, Half, Third }` (serde `snake_case`, `Default` = `Full`), `impl PreviewSpan { pub fn columns(self, grid: u32) -> u32 }`; `Session::preview_widget(&mut self, pack: Option<&Namespace>, name: &str, params: &Map<String, Value>, format: PreviewFormat, span: PreviewSpan) -> Result<WidgetPreview>`; JSON request `{"cmd":"preview_widget","name":..,"span":"half"}`; `UiState.preview_span: PreviewSpan` (JSON `preview_span`).

- [ ] **Step 1: Write the failing tests**

`preview_span_tests.rs`:

```rust
use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
#[case::full_12(PreviewSpan::Full, 12, 12)]
#[case::half_12(PreviewSpan::Half, 12, 6)]
#[case::third_12(PreviewSpan::Third, 12, 4)]
#[case::half_odd(PreviewSpan::Half, 5, 3)]
#[case::third_odd(PreviewSpan::Third, 5, 2)]
#[case::third_of_one(PreviewSpan::Third, 1, 1)]
fn preview_span_columns_rounds_up_and_keeps_one(#[case] span: PreviewSpan, #[case] grid: u32, #[case] cols: u32) {
    assert_that!(span.columns(grid)).is_equal_to(cols);
}

#[test]
fn preview_span_default_is_full() {
    assert_that!(PreviewSpan::default()).is_equal_to(PreviewSpan::Full);
}

#[test]
fn preview_span_serializes_snake_case() {
    assert_that!(serde_json::to_value(PreviewSpan::Third).ok()).is_equal_to(Some(serde_json::json!("third")));
}
```

In `widget_preview_tests.rs`, next to `preview_of_an_element_widget_gives_the_box_of_what_it_draws` (same fixture: `fresh_dir`, `opened_on`, `set_widget`), add a widget that fills its cell's width:

```rust
const BAR: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("bar", params: (:), (args, ctx) => block(width: 100%, height: 20pt, fill: black))
"#;

#[rstest]
#[case::half(PreviewSpan::Half, 0.5)]
#[case::third(PreviewSpan::Third, 1.0 / 3.0)]
fn preview_widget_with_a_span_narrows_the_element_box(#[case] span: PreviewSpan, #[case] share: f64) {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    s.set_widget(None, "bar", BAR.into()).unwrap();
    let full = s.preview_widget(None, "bar", &Map::new(), PreviewFormat::Svg, PreviewSpan::Full).unwrap();
    let part = s.preview_widget(None, "bar", &Map::new(), PreviewFormat::Svg, span).unwrap();
    let (f, p) = (full.bbox.unwrap().w, part.bbox.unwrap().w);
    // The column gaps make the share a little more than its fraction of the full width.
    assert_that!(p).is_greater_than(f * share * 0.9);
    assert_that!(p).is_less_than(f * share + 60.0);
}
```

(Add `use rstest::rstest;` and `PreviewSpan` to the file's imports; check `bbox.w`'s type and cast `share` to it.)

In `views_tests.rs`:

```rust
#[test]
fn ui_state_without_preview_span_reads_full() {
    let ui: UiState = serde_json::from_str(r#"{"slide":null,"code":false}"#).unwrap();
    assert_that!(ui.preview_span).is_equal_to(PreviewSpan::Full);
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo nextest run -p oku-slides-session -E 'test(preview_span) | test(ui_state_without_preview_span)'`
Expected: compile error, `PreviewSpan` not found.

- [ ] **Step 3: Implement `preview_span.rs`**

```rust
//! The slot a widget preview fills: the whole grid, half or a third of its columns.

use serde::{Deserialize, Serialize};

/// The share of the grid's columns an element widget's preview spans.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewSpan {
    /// Every column of the grid.
    #[default]
    Full,
    /// Half the columns, rounded up.
    Half,
    /// A third of the columns, rounded up.
    Third,
}

impl PreviewSpan {
    /// The columns spanned on a grid of `grid` columns; at least one.
    #[must_use]
    pub fn columns(self, grid: u32) -> u32 {
        let part = match self {
            Self::Full => 1,
            Self::Half => 2,
            Self::Third => 3,
        };
        grid.div_ceil(part).max(1)
    }
}

#[cfg(test)]
#[path = "preview_span_tests.rs"]
mod preview_span_tests;
```

In `lib.rs`: `mod preview_span;` (alphabetical, near `mod preview_crop;`) and `pub use preview_span::PreviewSpan;` next to the `widget_preview` re-export.

- [ ] **Step 4: Thread `span` through the preview**

`widget_preview.rs`: add `span: PreviewSpan` as the last parameter of `preview_widget` (doc line: "`span`: the element's share of the grid; ignored for a slide widget."), pass it to `preview_deck(&schema, &inputs, span)`, and in `preview_deck`'s `Category::Element` arm:

```rust
element.cell = Some(Cell { col: 0, row: 0, cols: span.columns(grid.columns), rows: grid.rows });
```

`request.rs`, `PreviewWidget`:

```rust
        /// The element's share of the grid; full by default.
        #[serde(default)]
        span: PreviewSpan,
```

and the arm: `Request::PreviewWidget { pack, name, params, span } => return json(&self.preview_widget(pack.as_ref(), &name, &params, PreviewFormat::Svg, span)?),`.

`views.rs`, `UiState` after `code_width`:

```rust
    /// The preview slot of a pack tab's element widgets; full by default.
    #[serde(default)]
    pub preview_span: PreviewSpan,
```

`widget_tools.rs:249`: `s.preview_widget(pack.as_ref(), &name, &params, PreviewFormat::Png, PreviewSpan::Full)` (the agent's PNG stays full width).

Callers in tests: `PreviewFormat::Svg)` → `PreviewFormat::Svg, PreviewSpan::Full)` and `PreviewFormat::Png)` → `PreviewFormat::Png, PreviewSpan::Full)`; import `PreviewSpan` where needed. Check with `grep -rn "\.preview_widget(" crates` that all 38 calls now pass five arguments.

- [ ] **Step 5: Run the tests**

Run: `cargo nextest run -p oku-slides-session -p oku-slides-mcp` (background on a cold build)
Expected: PASS. Then `cargo clippy -p oku-slides-session -p oku-slides-mcp --all-targets -- -D warnings`: clean.

- [ ] **Step 6: Update docs and commit**

Add one line to `crates/oku-slides-session/ARCHITECTURE.md` where `widget_preview` is described: "`preview_span.rs`: the preview element's share of the grid (`PreviewSpan`)." Keep it at most 50 lines.

```bash
git add crates/oku-slides-session crates/oku-slides-mcp/src/widget_tools.rs
git commit -m "feat(session,mcp): preview an element widget in a half or third slot (t-1392)"
```

---

### Task 2: Localized parameter labels and help (schema, Typst, form)

**Files:**
- Modify: `crates/oku-slides-core/src/widget.rs:207-222` (+ `widget_tests.rs`), `typst/oku-slides/src/params.typ`, `typst/oku-slides/src/widget.typ:56-62`
- Modify: `crates/oku-slides-session/src/param_fields.rs`, `widget_params.rs`, `agent/forms.rs` (`ParamField`, `params_with`), `fields.rs:121-150`, `lib.rs:880`, `widget_preview.rs:253`
- Modify (literals, mechanical): every `ParamSchema {` literal (`crates/oku-slides-core/src/ops/list_tests.rs`, `list_path_tests.rs`, `crates/oku-slides-import/src/hifi/widget.rs:198`, `crates/oku-slides-session/src/widget_params_tests.rs`, `param_fields_tests.rs`) gets `help: None`; every `ParamField {` literal without `..` (30 across `crates/oku-slides-session`) gets `help: None`
- Test: `crates/oku-slides-render/tests/widgets.rs`, `crates/oku-slides-session/src/param_fields_tests.rs`, `crates/oku-slides-session/src/agent/forms_tests.rs`

**Interfaces:**
- Consumes: `LocalizedText::pick(&self, locale: &str) -> Option<&str>` (core).
- Produces: `ParamSchema { label: Option<LocalizedText>, help: Option<LocalizedText>, .. }`; `param_fields::param_label(kind: &str, p: &ParamSchema, l: &Localizer) -> String`; `param_fields::param_help(kind: &str, p: &ParamSchema, l: &Localizer) -> Option<String>`; `param_fields::field(kind: &str, p: &ParamSchema, shown: Option<&ParamValue>, l: &Localizer) -> Option<ParamField>`; `ParamField.help: Option<String>` (`#[serde(default)] #[schemars(skip)]`); `fields::field_views(deck, slide, registry, fields, locale: &str)`. Task 3 adds the Fluent fallback inside `param_label`/`param_help`.

- [ ] **Step 1: Write the failing tests**

`crates/oku-slides-render/tests/widgets.rs`, after `probe_file_text_label_or_description_is_kept_or_blank_becomes_none` (reuse its pack-building helper; read it first):

```rust
/// A parameter's label and help: a text or a dictionary by language; blank becomes none.
#[rstest]
#[case::label_text("label: \"Titre\"", "label", json!("Titre"))]
#[case::help_by_locale("help: (en: \"Shown above\", fr: \"Au-dessus\")", "help", json!({"en": "Shown above", "fr": "Au-dessus"}))]
#[case::help_blank("help: \"  \"", "help", Value::Null)]
fn probe_file_param_label_or_help_is_kept_or_blank_becomes_none(#[case] arg: &str, #[case] key: &str, #[case] read: Value) {
    let src = format!(
        "#import \"/oku-slides/lib.typ\": widget, params as p\n#let widget = widget(\"kpi\", params: (t: p.plain({arg})), (args, ctx) => [])\n"
    );
    let probe = probe_file(&Fonts::default(), PackKind::Widgets, &WidgetFile::parse("kpi").unwrap(), &src, None);
    assert_that!(probe.is_valid()).is_true();
    let param = probe.schema.as_ref().and_then(|s| s.get("params")).and_then(|ps| ps.get(0)).cloned();
    assert_that!(param.and_then(|p| p.get(key).cloned()).unwrap_or(Value::Null)).is_equal_to(read);
}

#[test]
fn probe_file_with_a_non_text_param_help_is_set_aside() {
    let src = "#import \"/oku-slides/lib.typ\": widget, params as p\n#let widget = widget(\"kpi\", params: (t: p.plain(help: 42)), (args, ctx) => [])\n";
    let probe = probe_file(&Fonts::default(), PackKind::Widgets, &WidgetFile::parse("kpi").unwrap(), src, None);
    assert_that!(probe.is_valid()).is_false();
    assert_that!(probe.diagnostics.iter().any(|d| d.message.contains("l'aide du paramètre « t »"))).is_true();
}
```

`crates/oku-slides-session/src/param_fields_tests.rs`:

```rust
fn plain_with(label: Option<LocalizedText>, help: Option<LocalizedText>) -> ParamSchema {
    ParamSchema { name: "title".into(), label, help, ty: ParamType::Plain, default: None, bind: None }
}

#[test]
fn param_label_by_locale_picks_the_ui_language() {
    let p = plain_with(Some(serde_json::from_value(json!({"en": "Title", "fr": "Titre"})).unwrap()), None);
    assert_that!(param_label("acme/card", &p, &Localizer::new(Some("fr")))).is_equal_to("Titre".to_owned());
}

#[test]
fn param_label_without_label_is_the_name() {
    assert_that!(param_label("acme/card", &plain_with(None, None), &Localizer::new(Some("fr")))).is_equal_to("title".to_owned());
}

#[test]
fn field_with_help_carries_it_in_the_ui_language() {
    let help = serde_json::from_value(json!({"en": "Above the title", "fr": "Au-dessus du titre"})).unwrap();
    let f = field("acme/card", &plain_with(None, Some(help)), None, &Localizer::new(Some("fr"))).unwrap();
    assert_that!(f.help).is_equal_to(Some("Au-dessus du titre".to_owned()));
}
```

`crates/oku-slides-session/src/agent/forms_tests.rs`, next to `params_with_notes_shows_them`:

```rust
#[test]
fn params_with_field_help_adds_a_caption_after_the_field() {
    let spec: ShowParams = serde_json::from_value(json!({ "title": "T", "slide": "", "fields": [
        { "label": "Surtitre", "kind": "text", "value": "x", "op_template": null, "help": "Au-dessus du titre" }
    ] }))
    .unwrap();
    let extras = FormExtras { keys: &[], notes: &[], closable: false };
    let json = serde_json::to_string(&params_with("host-widget", &spec, &extras, &[], &fr()).messages).unwrap();
    assert_that!(json.as_str()).contains("\"children\":[\"title\",\"f0\",\"f0-help\"]");
    assert_that!(json.as_str()).contains("Au-dessus du titre");
}
```

(If the serialized column spells its children differently, read one `params_with` message with `dbg!` first and match its exact form.)

- [ ] **Step 2: Run them to see them fail**

Run: `cargo nextest run -p oku-slides-render -p oku-slides-session -E 'test(param_label) | test(param_help) | test(field_with_help) | test(params_with_field_help) | test(probe_file_param) | test(probe_file_with_a_non_text_param_help)'`
Expected: compile errors (`help` field missing, `param_label` not found).

- [ ] **Step 3: Implement the schema and Typst side**

`widget.rs`, `ParamSchema`:

```rust
    /// Label shown in the editor, plain or by language; absent: the name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<LocalizedText>,
    /// One line of help under the field, plain or by language.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<LocalizedText>,
```

`params.typ`: `_base(type, label, help, default)` returns `(type: type, label: label, help: help, default: default)`, and every helper takes `help: none` after `label: none` and passes it on (`plain`, `rich`, `flag`, `number`, `choice`, `asset`, `slot`, `list`, `grid`, `host`). Update the file's header comment example to `p.plain(label: "Titre", help: "Au-dessus du corps")`.

`widget.typ`, in the `for (key, d) in params` loop, before `list.push`:

```typst
    let d = d + (
      label: _shown-text(name, "le label du paramètre « " + key + " »", d.at("label", default: none)),
      help: _shown-text(name, "l'aide du paramètre « " + key + " »", d.at("help", default: none)),
    )
```

Also document `help` in the `widget` doc comment: "A parameter's `label` and `help` (one line under its field) take the same forms."

- [ ] **Step 4: Implement the session side**

`param_fields.rs`: replace `label(p)` with the full resolver (Task 3 only adds the Fluent keys it reads, so a built-in widget's parameter still shows its name until then):

```rust
/// The shown label of `p`, a parameter of widget `kind`: its schema label in the UI language, else the
/// translated label of a built-in pack's parameter (`session-widget-param-label-<key>-<param>`), else its name.
pub(crate) fn param_label(kind: &str, p: &ParamSchema, l: &Localizer) -> String {
    match p.label.as_ref().and_then(|t| t.pick(l.locale())) {
        Some(label) => label.to_owned(),
        None => builtin_param(kind, p, "label", l).unwrap_or_else(|| p.name.clone()),
    }
}

/// The help under `p`'s field: the schema's in the UI language, else a built-in pack's translated one.
pub(crate) fn param_help(kind: &str, p: &ParamSchema, l: &Localizer) -> Option<String> {
    match p.help.as_ref().and_then(|t| t.pick(l.locale())) {
        Some(help) => Some(help.to_owned()),
        None => builtin_param(kind, p, "help", l),
    }
}

/// A built-in pack parameter's translated `what` (`label` or `help`); `None` outside built-in packs.
fn builtin_param(kind: &str, p: &ParamSchema, what: &str, l: &Localizer) -> Option<String> {
    kind_key(kind).and_then(|k| l.try_tr(&format!("session-widget-param-{what}-{k}-{}", p.name)))
}
```

`field(kind, p, shown, l)`: every `label(p)` becomes `param_label(kind, p, l)`; both `ParamField { .. }` constructions get `help: param_help(kind, p, l)`; `column()` (list columns) uses `param_label(kind, c, l)` too (thread `kind` and `l` into it).

`agent/forms.rs`: add to `ParamField`:

```rust
    /// One line of help under the field (host settings).
    #[serde(default)]
    #[schemars(skip)]
    pub help: Option<String>,
```

In `params_with`, right after `body.push(key);`:

```rust
        if let Some(help) = &f.help {
            let hid = format!("{key}-help");
            components.push(Component::text(&hid, help.clone(), Some(TextVariant::Caption)));
            body.push(hid);
        }
```

`widget_params.rs`: `field(&schema.name, p, ..)`, the fixed-param note uses `param_label(&schema.name, p, l)`, and the title becomes `l.tr_with("session-widget-params", &[("name", kind_name(&schema.name, schema.label.as_ref(), l).into())])` (move the title after `valid_schema`; an invalid file keeps the file name).

`fields.rs`: `field_views` takes `locale: &str` last; `label: tf.param.label.as_ref().and_then(|t| t.pick(locale)),`. Callers: `lib.rs:880` and `widget_preview.rs:253` pass `self.localizer.locale()`.

Literals: add `help: None` to every `ParamSchema {` and `ParamField {` literal listed in **Files**; a `ParamSchema` literal's `label: Some("x".into())` becomes `label: Some(LocalizedText::Plain("x".into()))`.

- [ ] **Step 5: Run the tests**

Run: `cargo nextest run -p oku-slides-core -p oku-slides-render -p oku-slides-session -p oku-slides-import -p oku-slides-mcp -p oku-slides-local-agent` (background)
Expected: PASS. Snapshot tests of the tools' JSON schema must not change (`help` is `schemars(skip)`); if one does, read the diff before accepting it with `cargo insta review`.
Run: `cargo clippy --workspace --all-targets -- -D warnings`: clean.

- [ ] **Step 6: Update `typst/oku-slides/AGENTS.md` if it lists the params helpers' arguments, then commit**

```bash
git add crates typst
git commit -m "feat(core,session,import): localized label and help for widget parameters (t-1392)"
```

---

### Task 3: Built-in parameter labels and help in Fluent

**Files:**
- Modify: `crates/oku-slides-i18n/src/locales/*/session.ftl` (20 files), after `session-widget-description-*`
- Modify: `ui/e2e/180-widget-values.cjs:27` (`name: 'title'` → `name: 'Titre'`)
- Test: `crates/oku-slides-session/src/param_fields_tests.rs`, `crates/oku-slides-i18n/src/lib_tests.rs`

**Interfaces:**
- Consumes: `param_label`/`param_help` from Task 2, which already look these keys up.
- Produces: Fluent keys `session-widget-param-label-<pack>-<widget>-<param>` and `session-widget-param-help-<pack>-<widget>-<param>`.

Keys (English reference; `fr` given, the 18 other locales translated by you):

| Key suffix | en label | fr label | en help | fr help |
|---|---|---|---|---|
| `base-card-kicker` | Kicker | Surtitre | Small line above the title | Petite ligne au-dessus du titre |
| `base-card-title` | Title | Titre | | |
| `base-card-body` | Body | Corps | | |
| `base-card-highlight` | Highlight | Mise en avant | Puts the card on the theme's dark background | Passe la carte sur le fond sombre du thème |
| `base-image-fit` | Fit | Ajustement | How the image fills its box | Comment l'image remplit son cadre |
| `base-shape-shape` | Shape | Forme | | |
| `base-stack-layout` | Layout | Disposition | Rows, columns or a grid | Lignes, colonnes ou grille |
| `base-stack-count` | Columns | Colonnes | Columns of a column or grid layout; empty: one per child | Colonnes d'une disposition en colonnes ou en grille ; vide : une par enfant |
| `base-stack-gap` | Gap | Espacement | | |
| `base-text-body` | Text | Texte | | |

Only rows with a help get a `session-widget-param-help-*` key. Check the settable parameters with `grep -n "params: (" typst/oku-slides/packs/base/widgets/*.typ` before writing; `asset`, `slot` and `host` parameters are not settable and get no key.

- [ ] **Step 1: Write the failing tests**

`param_fields_tests.rs`:

```rust
#[rstest]
#[case::label("fr", "Mise en avant")]
#[case::label_en("en", "Highlight")]
fn param_label_of_a_builtin_widget_reads_its_fluent_key(#[case] locale: &str, #[case] shown: &str) {
    let p = ParamSchema { name: "highlight".into(), label: None, help: None, ty: ParamType::Flag, default: None, bind: None };
    assert_that!(param_label("base/card", &p, &Localizer::new(Some(locale)))).is_equal_to(shown.to_owned());
}

#[test]
fn param_help_of_a_builtin_widget_reads_its_fluent_key() {
    let p = ParamSchema { name: "highlight".into(), label: None, help: None, ty: ParamType::Flag, default: None, bind: None };
    assert_that!(param_help("base/card", &p, &Localizer::new(Some("fr"))))
        .is_equal_to(Some("Passe la carte sur le fond sombre du thème".to_owned()));
}

#[test]
fn param_label_of_a_pack_widget_without_label_is_the_name() {
    let p = ParamSchema { name: "highlight".into(), label: None, help: None, ty: ParamType::Flag, default: None, bind: None };
    assert_that!(param_label("acme/card", &p, &Localizer::new(Some("fr")))).is_equal_to("highlight".to_owned());
}
```

`crates/oku-slides-i18n/src/lib_tests.rs`, next to the `session-widget-param-yes` test:

```rust
#[test]
fn builtin_param_label_is_translated_in_japanese() {
    let ja = Localizer::new(Some("ja"));
    assert_that!(ja.try_tr("session-widget-param-label-base-card-title")).is_some().is_not_equal_to("Title".to_owned());
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo nextest run -p oku-slides-session -p oku-slides-i18n -E 'test(builtin) | test(param_label) | test(param_help)'`
Expected: FAIL (`highlight` returned instead of `Mise en avant`; key missing).

- [ ] **Step 3: Add the Fluent keys to all 20 locales**

In each `crates/oku-slides-i18n/src/locales/<locale>/session.ftl`, right after the last `session-widget-description-*` line, the labels then the helps, in the table's order, e.g. for `en`:

```ftl
session-widget-param-label-base-card-kicker = Kicker
session-widget-param-label-base-card-title = Title
session-widget-param-label-base-card-body = Body
session-widget-param-label-base-card-highlight = Highlight
session-widget-param-label-base-image-fit = Fit
session-widget-param-label-base-shape-shape = Shape
session-widget-param-label-base-stack-layout = Layout
session-widget-param-label-base-stack-count = Columns
session-widget-param-label-base-stack-gap = Gap
session-widget-param-label-base-text-body = Text
session-widget-param-help-base-card-kicker = Small line above the title
session-widget-param-help-base-card-highlight = Puts the card on the theme's dark background
session-widget-param-help-base-image-fit = How the image fills its box
session-widget-param-help-base-stack-layout = Rows, columns or a grid
session-widget-param-help-base-stack-count = Columns of a column or grid layout; empty: one per child
```

Follow `crates/oku-slides-i18n/AGENTS.md` (pt is Brazilian, no copied English, same order everywhere). Run the crate's key-parity test to confirm.

- [ ] **Step 4: Run the tests**

Run: `cargo nextest run -p oku-slides-session -p oku-slides-i18n` (background)
Expected: PASS. `cd ui && bun test src/i18n-keys.test.ts`: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-session/src/param_fields_tests.rs crates/oku-slides-i18n ui/e2e/180-widget-values.cjs
git commit -m "feat(session,i18n): readable labels and help for the base pack's parameters (t-1392)"
```

---

### Task 4: A2UI `Switch` for flags

**Files:**
- Modify: `crates/oku-slides-a2ui/src/component.rs` (variant after `Select`, `is_field`, `children`), `catalog_schema_tests.rs` (`CATALOG`: 26 names, `"Switch"` after `"Select"`), `component_tests.rs`, `playbook.rs:74` (name and group), `playbook/entries.rs` (`switch()` after `select()`)
- Modify: `ui/src/a2ui/catalog.tsx` (`SwitchApi`, `Switch`, catalog list at line ~606), `ui/src/a2ui/a2ui.css`, `ui/src/a2ui/fixtures/playbook.json` (regenerated)
- Modify: `crates/oku-slides-session/src/agent/forms.rs` (`FieldKind::Switch`, its component), `param_fields.rs` (`Flag` → `Switch`, bool value, `coerce`), `param_fields_tests.rs:15-16`
- Modify: `crates/oku-slides-i18n/src/locales/*/session.ftl` (remove `session-widget-param-yes`/`-no`), `crates/oku-slides-i18n/src/lib_tests.rs:82-83`
- Test: `component_tests.rs`, `ui/src/a2ui/catalog.render.test.tsx`, `param_fields_tests.rs`

**Interfaces:**
- Produces: `ComponentKind::Switch { label: Dynamic<String>, value: Dynamic<bool>, action: Option<Action> }` (JSON `"component": "Switch"`); `FieldKind::Switch` (value: JSON bool).

- [ ] **Step 1: Write the failing tests**

`component_tests.rs`, next to the `Select` test:

```rust
#[test]
fn switch_serializes_its_name_and_bound_value() {
    let switch = component_json(ComponentKind::Switch { label: Dynamic::Literal("On".into()), value: Dynamic::path("/on"), action: None });
    assert_that!(switch.as_ref().and_then(|v| v.get("component").cloned())).is_some().is_equal_to(json!("Switch"));
    assert_that!(switch.and_then(|v| v.get("value").cloned())).is_some().is_equal_to(json!({"path": "/on"}));
}
```

`catalog.render.test.tsx`:

```tsx
describe("Switch", () => {
  const sw = { component: "Switch", label: "Highlight", value: { path: "/on" }, action: { event: { name: "param", context: { value: { path: "/on" } } } } };

  test("Switch shows a checkbox switch named by its label, checked from the model", async () => {
    const { host } = await mount(sw, { on: true });
    const input = host.querySelector<HTMLInputElement>('input[role="switch"]');
    expect(input?.checked).toBe(true);
    expect(input?.closest("label")?.textContent).toContain("Highlight");
  });

  test("Switch click writes false and sends its action", async () => {
    const { host, actions } = await mount(sw, { on: true });
    await click(host.querySelector('input[role="switch"]') as HTMLInputElement);
    expect(actions.at(-1)?.event?.context?.value).toBe(false);
  });
});
```

`param_fields_tests.rs`: replace the two `ParamType::Flag` string cases of the `coerce` table with `#[case(ParamType::Flag, json!(true), Some(json!(true)))]` and `#[case(ParamType::Flag, json!(false), Some(json!(false)))]`, and add:

```rust
#[test]
fn field_of_a_flag_is_a_switch_with_a_bool_value() {
    let p = ParamSchema { name: "on".into(), label: None, help: None, ty: ParamType::Flag, default: None, bind: None };
    let f = field("acme/w", &p, Some(&ParamValue::Value(json!(true))), &Localizer::new(None)).unwrap();
    assert_that!((f.kind, f.value)).is_equal_to((FieldKind::Switch, json!(true)));
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo nextest run -p oku-slides-a2ui -p oku-slides-session -E 'test(switch) | test(coerce)'` and `cd ui && bun test .render.test. -t Switch`
Expected: FAIL (variant and component missing).

- [ ] **Step 3: Implement the Rust component**

`component.rs`, after `Select`:

```rust
    /// An on/off switch bound to a boolean.
    Switch {
        /// Switch label.
        label: Dynamic<String>,
        /// Whether it is on.
        value: Dynamic<bool>,
        /// Sent after the new state is written into `value`.
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<Action>,
    },
```

Add `| Self::Switch { .. }` to `is_field` and to the childless arm of `children`. `playbook.rs`: `K::Switch { .. } => ("Switch", Field),`. `entries.rs`:

```rust
fn switch() -> Vec<Example> {
    vec![example(
        "bound",
        "An on/off setting; it writes a boolean.",
        false,
        K::Switch { label: lit("Highlight"), value: Dynamic::path("/on"), action: Some(Action::named("toggle")) },
        vec![],
        json!({ "on": true }),
    )]
}
```

and list `switch(),` after `select(),` in `all()`. `catalog_schema_tests.rs`: `CATALOG: [&str; 26]`, `"Switch"` after `"Select"`; update the doc line to "The 26 catalog components".

- [ ] **Step 4: Implement the UI component (same commit)**

`catalog.tsx`, after `SelectApi`:

```tsx
const SwitchApi = {
  name: "Switch",
  schema: z
    .object({
      label: CommonSchemas.DynamicString,
      value: CommonSchemas.DynamicBoolean,
      action: CommonSchemas.Action.optional(),
    })
    .strict(),
};
```

after `Select`:

```tsx
/** Un interrupteur : la case écrit le booléen, puis envoie l'action. */
const Switch = createComponentImplementation(SwitchApi, ({ props }) => (
  <label className="a2ui-switch">
    <input
      type="checkbox"
      role="switch"
      checked={props.value === true}
      onChange={(e) => {
        props.setValue(e.currentTarget.checked);
        props.action?.();
      }}
    />
    <span>{props.label}</span>
  </label>
));
```

Add `Switch` after `Select` in the catalog list (line ~606). `a2ui.css`: a switch track and thumb on the checkbox (`appearance: none`, 32×18 px track, `var(--accent)` when checked, focus ring like other fields, reduced motion respected). Regenerate the fixture: `OKU_UPDATE_FIXTURES=1 cargo nextest run -p oku-slides-a2ui -E 'test(playbook_ui_fixture_is_current)'`, then rerun without the variable. Accept the agent reference snapshot only after reading its diff (`cargo insta review`).

- [ ] **Step 5: Map flags to the switch**

`forms.rs`, `FieldKind`: add `/// On or off: \`value\` is a boolean.` `Switch,` after `Select`, and in `field_components`:

```rust
        FieldKind::Switch => ComponentKind::Switch { label, value: Dynamic::path(&path), action },
```

`param_fields.rs`: `ParamType::Flag => (FieldKind::Switch, None, None, None, Vec::new()),`; in the value match, before the generic bool arm: `(ParamType::Flag, Some(ParamValue::Value(Value::Bool(b)))) => json!(b),`; in `coerce`, delete the `(ParamType::Flag, Value::String(s))` arm and its doc mention. Remove `session-widget-param-yes` and `session-widget-param-no` from the 20 `session.ftl` files and their test in `lib_tests.rs:82-83` (check with `grep -rn "session-widget-param-yes" crates ui/src` that nothing else reads them; `theme_controls.rs` uses `theme-option-*`, unaffected).

- [ ] **Step 6: Run the tests**

Run: `cargo nextest run -p oku-slides-a2ui -p oku-slides-session -p oku-slides-i18n -p oku-slides-mcp` (background) and `cd ui && bun run test && bun run typecheck`
Expected: PASS.

- [ ] **Step 7: Update docs and commit**

Add `Switch` wherever `crates/oku-slides-a2ui/ARCHITECTURE.md` and `ui/src/a2ui` docs list the catalog's fields.

```bash
git add crates/oku-slides-a2ui crates/oku-slides-session crates/oku-slides-i18n ui/src/a2ui
git commit -m "feat(a2ui,session,i18n,ui): a Switch for flag parameters (t-1392)"
```

---

### Task 5: Slot picker and one-line header (UI)

**Files:**
- Create: `ui/src/PreviewSpanMenu.tsx`, `ui/src/PreviewSpanMenu.render.test.tsx`
- Modify: `ui/src/api.ts` (`PreviewSpan`, `UiState.preview_span`, `previewWidget`), `ui/src/useKeptUi.ts`, `ui/src/useKeptUi.render.test.tsx`, `ui/src/WidgetPane.tsx`, `ui/src/WidgetTab.tsx:131-151`, `ui/src/PackPanels.tsx:79-95`, `ui/src/styles.css` (`.label.editing-pack`)
- Modify: `crates/oku-slides-i18n/src/locales/*/ui.ftl` (20 files)
- Test: `PreviewSpanMenu.render.test.tsx`, `useKeptUi.render.test.tsx`, `PackPanels.render.test.tsx`

**Interfaces:**
- Consumes: JSON `span` on `preview_widget`, `preview_span` on the UI state (Task 1).
- Produces: `export type PreviewSpan = "full" | "half" | "third";` (api.ts); `previewWidget(name: string, pack?: string, span: PreviewSpan = "full")`; `KeptUi.previewSpan: PreviewSpan`, `KeptUi.setPreviewSpan(span: PreviewSpan): void`; `PreviewSpanMenu({ span, onPick })`; `WidgetPane` and `WidgetTab` prop `span: PreviewSpan`.

Fluent keys (en; fr; the 18 others translated), in `ui.ftl` right after `ui-preview-theme`:

```ftl
ui-preview-label = Preview:
ui-preview-span = Preview slot
ui-preview-span-full = Full width
ui-preview-span-half = Half
ui-preview-span-third = Third
```

fr: `Aperçu :` (narrow no-break space before the colon, as other fr keys do), `Case de l'aperçu`, `Pleine largeur`, `Moitié`, `Tiers`.

- [ ] **Step 1: Write the failing tests**

`PreviewSpanMenu.render.test.tsx` (copy the mount/act setup from `PreviewThemeMenu`'s or `PackBrand.render.test.tsx`):

```tsx
test("PreviewSpanMenu offers full, half and third, current selected", async () => {
  const { host } = await mount(createElement(PreviewSpanMenu, { span: "half", onPick: () => {} }));
  const select = host.querySelector("select") as HTMLSelectElement;
  expect([...select.options].map((o) => o.value)).toEqual(["full", "half", "third"]);
  expect(select.value).toBe("half");
});

test("PreviewSpanMenu change picks the chosen span", async () => {
  const picked: string[] = [];
  const { host } = await mount(createElement(PreviewSpanMenu, { span: "full", onPick: (s: string) => picked.push(s) }));
  await change(host.querySelector("select") as HTMLSelectElement, "third");
  expect(picked).toEqual(["third"]);
});
```

`useKeptUi.render.test.tsx`: on the model of the existing `code` test, `previewSpan` is restored from `kept.preview_span` and a `setPreviewSpan("half")` sends `preview_span: "half"` in the next save.

`PackPanels.render.test.tsx`: in a widget pack tab with an item selected, the header holds `.preview-span-menu`; in a layout pack tab it does not; with Commun selected it does not.

- [ ] **Step 2: Run them to see them fail**

Run: `cd ui && bun test .render.test. -t "PreviewSpan|previewSpan|preview-span"`
Expected: FAIL (module missing).

- [ ] **Step 3: Implement**

`api.ts`: the type, `UiState.preview_span: PreviewSpan` (doc: "A pack tab's preview slot for element widgets; full by default."), and

```ts
    previewWidget: (name: string, pack?: string, span: PreviewSpan = "full") =>
      call<WidgetPreview>({ cmd: "preview_widget", name, pack, span }),
```

`PreviewSpanMenu.tsx`:

```tsx
// The preview-slot select of a pack tab's header.

import type { PreviewSpan } from "./api";
import { t } from "./i18n";
import { Select } from "./primitives";

const SPANS: readonly PreviewSpan[] = ["full", "half", "third"];

/** Picks the share of the grid an element widget's preview spans. */
export default function PreviewSpanMenu({ span, onPick }: { span: PreviewSpan; onPick(span: PreviewSpan): void }) {
  // `Select` takes no className: the wrapper carries the hook for styles and tests.
  return (
    <span className="preview-span-menu">
      <Select
        aria-label={t("ui-preview-span")}
        title={t("ui-preview-span")}
        value={span}
        options={SPANS.map((s) => ({ value: s, label: t(`ui-preview-span-${s}`) }))}
        onChange={(v) => onPick(v as PreviewSpan)}
      />
    </span>
  );
}
```

`useKeptUi.ts`: `const [previewSpan, setPreviewSpan] = useState<PreviewSpan>("full");`, restored with `setPreviewSpan(kept.preview_span ?? "full")`, saved as `preview_span: previewSpan`, added to the effect deps, the `KeptUi` interface (documented) and the returned object; update the file header comment.

`WidgetPane.tsx` and `WidgetTab.tsx`: a `span: PreviewSpan` prop passed down; `WidgetTab`'s preview effect calls `api.previewWidget(name, undefined, span)` and lists `span` in its deps.

`PackPanels.tsx`: pass `span={keptUi.previewSpan}` to `WidgetPane`; in the header, after the caption:

```tsx
        <span className="preview-label">{t("ui-preview-label")}</span>
        <PreviewThemeMenu themes={snap.themes} theme={snap.theme} onPick={(ns) => void run(api.setPreviewTheme(ns))} />
        {kind === "widgets" && item.selected !== null && item.selected !== COMMON && (
          <PreviewSpanMenu span={keptUi.previewSpan} onPick={keptUi.setPreviewSpan} />
        )}
```

`styles.css`, after `.label.editing-pack .brand`:

```css
/* A pack tab's header: one row, from the sheet's left edge, the finish tray at its end. */
.label.editing-pack {
  justify-self: stretch; flex-direction: row; align-items: center; gap: var(--space-2) var(--space-4);
  max-width: none; padding-inline: var(--space-6) var(--tray-reserve); text-align: start;
}
.label.editing-pack .doc-name-button, .label.editing-pack .doc-name-input { text-align: start; }
.label.editing-pack .preview-label { margin-inline-start: auto; font-size: 12px; color: var(--ink-muted); }
```

- [ ] **Step 4: Run the tests**

Run: `cd ui && bun run test && bun run typecheck`, `cargo nextest run -p oku-slides-i18n`
Expected: PASS.

- [ ] **Step 5: Check it in the app**

Use the `run-oku-slides` skill (fresh `OKU_SLIDES_PATHS__DATA`/`__DECKS` temp dir): open the Base pack tab, select `card`, switch the slot to Half: the card renders at half width and larger in the preview; the header is one row. Take a screenshot.

- [ ] **Step 6: Commit**

```bash
git add ui/src crates/oku-slides-i18n
git commit -m "feat(ui,i18n): preview slot picker and a one-line pack header (t-1392)"
```

---

### Task 6: Preview and form side by side (UI)

**Files:**
- Modify: `ui/src/pack.css:15-60` (`.widget-tab` grid), `ui/src/WidgetTab.tsx` (header comment only)
- Modify: `ui/e2e/170-code-layout.cjs`
- Test: `ui/src/WidgetTab.render.test.tsx`, `ui/e2e/170-code-layout.cjs`

**Interfaces:**
- Consumes: nothing new; `--code-w` is still set by `WidgetTab` from `codeTrack` (`codeSplit.ts`).

This applies to every pack tab (widgets and layouts): `WidgetTab` is shared.

- [ ] **Step 1: Write the failing tests**

`WidgetTab.render.test.tsx`: happy-dom does not lay out, so assert the grid declarations through the stylesheet is not possible there; instead keep the render test to structure (the preview, `.widget-tab-side` and `.widget-tab-code` stay siblings in the same order) and move the geometry check to e2e. Update `170-code-layout.cjs`:

```js
  // Code hidden: the preview on the left over the full height, the form on its right.
  let r = await boxes(p);
  assert.ok(r.side.left >= r.preview.right, `réglages pas à droite de l'aperçu : ${JSON.stringify(r)}`);
  assert.ok(Math.abs(r.side.width - 300) <= 2, `réglages larges de ${r.side.width}, 300 attendus`);

  // Code shown: the code on the right; the form under the preview.
  await p.getByRole('button', { name: 'Code source' }).click();
  await p.waitForSelector(`${TAB}[data-code="shown"] .code-divider[aria-orientation="vertical"]`);
  r = await boxes(p);
  assert.ok(r.code.left >= r.preview.right && r.code.left >= r.side.right, `code pas à droite : ${JSON.stringify(r)}`);
  assert.ok(r.side.top >= r.preview.bottom, `réglages pas sous l'aperçu : ${JSON.stringify(r)}`);
```

Keep the rest of the spec (divider drag, reload, narrow window), adjusting its comments: the divider now sizes the code against the preview column, whose minimum stays 240 px (`SIDE_MIN` in `codeSplit.ts`).

- [ ] **Step 2: Run it to see it fail**

Run: `just e2e 4999` (or only up to spec 170 if `run.cjs` takes a filter; read `ui/e2e/lib/run.cjs`)
Expected: FAIL on « réglages pas à droite de l'aperçu ».

- [ ] **Step 3: Implement the grid**

Replace the `.widget-tab` rules in `pack.css` (and the French comments above them, in English):

```css
/* Named areas. Code hidden: the preview on the left, full height; the form on its right at 300px.
   Code shown: the preview over the form on the left, the code on the right at the divider's width
   (--code-w, set by WidgetTab); the left column keeps SIDE_MIN (240px) and the divider SPLIT (12px),
   as codeSplit.ts. The code track at 0px hidden assumes .widget-tab-code has no padding or border. */
.widget-tab {
  display: grid; grid-template-areas: "preview split side";
  grid-template-rows: minmax(0, 1fr);
  grid-template-columns: minmax(0, 1fr) 12px 300px; height: 100%; min-height: 0;
}
.widget-tab-code { grid-area: code; }
.widget-tab:not([data-code="shown"]) > .widget-tab-code { position: absolute; width: 0; height: 0; overflow: hidden; }
.widget-tab[data-code="shown"] {
  grid-template-areas: "preview split code" "side split code";
  grid-template-rows: minmax(0, 1fr) fit-content(40%);
  grid-template-columns: minmax(240px, 1fr) 12px var(--code-w, minmax(0, 1fr));
}
```

Keep `.widget-tab-preview { grid-area: preview; }`, `.widget-tab-side { grid-area: side; }` and the divider rules. Check that the hidden editor stays mounted and inert (`[inert] { visibility: hidden; }` already). Update `WidgetTab.tsx`'s header comment ("its preview on the left; the settings on its right, or under it when the code is shown").

- [ ] **Step 4: Run the tests**

Run: `cd ui && bun run test`, then `just e2e 4999`
Expected: PASS, including 160 and 180.

- [ ] **Step 5: Commit**

```bash
git add ui/src/pack.css ui/src/WidgetTab.tsx ui/e2e/170-code-layout.cjs
git commit -m "feat(ui): a pack item's form beside its preview, the code in its place (t-1392)"
```

---

### Task 7: Slim item list for widget packs (UI)

**Files:**
- Modify: `ui/src/SlidePile.tsx` (prop `variant`), `ui/src/PackPile.tsx` (passes it), `ui/src/styles.css` (`.pile.rows`, widget pack pile width)
- Test: `ui/src/SlidePile.render.test.tsx`, `ui/src/PackPile.render.test.tsx`

**Interfaces:**
- Produces: `SlidePile` prop `variant?: "thumbs" | "rows"` (default `"thumbs"`); rows keep the `.thumb`, `.thumb-name`, `.thumb-remove` classes so e2e helpers (`pileThumb`, `300-pile-ordre.cjs`) still find them.

- [ ] **Step 1: Write the failing tests**

`SlidePile.render.test.tsx`:

```tsx
test("SlidePile rows variant shows names without images and asks no SVG", async () => {
  const asked: string[] = [];
  const { host } = await mount({ variant: "rows", names: { a: "card" }, svg: (id: string) => (asked.push(id), "blob:x") });
  expect(host.querySelector(".pile")?.classList.contains("rows")).toBe(true);
  expect(host.querySelector(".thumb-img")).toBeNull();
  expect(host.querySelector(".thumb-name")?.textContent).toBe("card");
  expect(asked).toEqual([]);
});
```

(Use the file's existing `mount` helper and slide fixtures; add the `variant` prop to its options.)

`PackPile.render.test.tsx`:

```tsx
test("PackPile of a widget pack lists rows; of a layout pack, thumbnails", async () => {
  const widgets = await mount(stub(memory({ card: "" })));
  expect(widgets.host.querySelector(".pile.rows")).not.toBeNull();
});
```

and the same mount with `{ kind: "layouts" }` expecting `.pile.rows` to be null (follow the file's existing two-mount pattern; `afterEach` unmounts).

- [ ] **Step 2: Run them to see them fail**

Run: `cd ui && bun test .render.test. -t "rows"`
Expected: FAIL.

- [ ] **Step 3: Implement**

`SlidePile.tsx`: add the documented prop (`/** Thumbnails (default), or rows with the name only (a widget pack's pile): no image is asked. */`), the class `${variant === "rows" ? " rows" : ""}` on the `nav`, and in each item:

```tsx
              {variant !== "rows" && <span className="thumb-n">{i + 1}</span>}
              <span className="thumb-body">
                {variant !== "rows" && (url ? <img … /> : <span className="thumb-img" />)}
```

computing `const url = variant === "rows" ? null : svg(s.id, s.hash);` so rows never ask an SVG; the drag ghost shows the name in rows (`{variant === "rows" ? <span className="thumb-name">{names?.[dragged.id]}</span> : ghostUrl ? … }`), with `ghostUrl` likewise not asked in rows.

`PackPile.tsx`: `variant={kind === "widgets" ? "rows" : "thumbs"}`; update its header comment ("one row per item in a widget pack, one thumbnail per layout").

`styles.css`:

```css
/* A widget pack's pile: one row per item, its name; the trash shows on hover or focus. */
.table:has(> .pile-column .pile.rows) { --pile-w: 180px; }
.pile.rows { gap: 2px; padding-inline: var(--space-4); }
.pile.rows .thumb { display: flex; align-items: center; width: 100%; padding: 6px 8px; border-radius: var(--radius-sm); background: transparent; box-shadow: none; }
.pile.rows .thumb[aria-current="true"] { background: var(--accent-soft); }
.pile.rows .thumb-body { display: flex; align-items: center; gap: var(--space-2); min-width: 0; }
.pile.rows .thumb-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.pile.rows .thumb-slot { position: relative; display: flex; align-items: center; }
.pile.rows .thumb-remove { opacity: 0; width: 28px; height: 28px; }
.pile.rows .thumb-slot:hover .thumb-remove, .pile.rows .thumb-slot:focus-within .thumb-remove { opacity: 1; }
.pile.rows .drop-line { height: 2px; }
```

(180 px is `PILE_MIN` in `columns.ts`; a width chosen at the divider, set inline on `.table`, still wins.) Read the existing `.thumb`, `.thumb-slot`, `.thumb-remove` and `.drop-line` rules first and override only what differs.

- [ ] **Step 4: Run the tests**

Run: `cd ui && bun run test && bun run typecheck`, then `just e2e 4999`
Expected: PASS (300-pile-ordre uses a layout pack and keeps thumbnails; 160 and 180 select widgets by name).

- [ ] **Step 5: Check it in the app and commit**

With `run-oku-slides`, open the Base pack: the left column shows a slim list; drag a row to reorder; hover shows the trash. Screenshot.

```bash
git add ui/src/SlidePile.tsx ui/src/PackPile.tsx ui/src/styles.css ui/src/SlidePile.render.test.tsx ui/src/PackPile.render.test.tsx
git commit -m "feat(ui): a slim item list in a widget pack's pile (t-1392)"
```

---

### Task 8: End-to-end check of the new editor

**Files:**
- Create: `ui/e2e/310-pack-editor-layout.cjs`
- Modify: `ui/ARCHITECTURE.md` (pack tab layout, slot picker), `ui/AGENTS.md` only if a rule changes

- [ ] **Step 1: Write the spec**

```js
// The pack editor's layout (t-1392): in the Base widget pack, `card` selected, the preview slot set to
// « Moitié » narrows the preview image and is kept after a reload; the form sits on the preview's right,
// with « Mise en avant » as a switch; the pile lists rows, without thumbnails.
// Writes: the UI state only. Leaves: no pack tab.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { closePackTabs, openPackTab } = require('./lib/atelier.cjs');
const { untilValue } = require('./lib/until.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const TAB = '.widget-tab';

const width = (p, sel) => p.locator(sel).first().evaluate((el) => el.getBoundingClientRect().width);

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1600, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await closePackTabs(p);
  await p.waitForSelector('.stage img');
  await openPackTab(p, 't-1', 'base', 'card');
  await p.waitForSelector(`${TAB} .widget-tab-preview .stage, ${TAB} .widget-tab-preview img`);

  assert.equal(await p.locator('.pack-pile .pile.rows').count(), 1, 'la pile n\'est pas en lignes');
  assert.equal(await p.locator('.pack-pile .thumb-img').count(), 0, 'la pile montre des miniatures');
  assert.equal(await p.locator(`${TAB} .host-settings`).getByRole('switch', { name: 'Mise en avant' }).count(), 1);

  const full = await width(p, `${TAB} .widget-tab-preview .stage, ${TAB} .widget-tab-preview img`);
  await p.getByRole('combobox', { name: "Case de l'aperçu" }).selectOption('half');
  await untilValue(p, () => width(p, `${TAB} .widget-tab-preview .stage, ${TAB} .widget-tab-preview img`),
    (w) => w < full * 0.8, 'l\'aperçu ne rétrécit pas en case ½');

  await p.reload();
  await p.waitForSelector(`${TAB} .widget-tab-preview`);
  assert.equal(await p.getByRole('combobox', { name: "Case de l'aperçu" }).inputValue(), 'half', 'case non gardée');

  await closePackTabs(p);
  assert.deepEqual(errors, []);
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
```

Before running, read `openPackTab`'s signature in `ui/e2e/lib/atelier.cjs` (`openPackTab(p, from, pack, widget)`) and how other specs end (some reset the UI state); match them. If the size stays at `half` for later specs, reset it to `full` before closing.

- [ ] **Step 2: Run it**

Run: `just e2e 4999`
Expected: every spec PASS.

- [ ] **Step 3: Docs and commit**

Update `ui/ARCHITECTURE.md`'s pack tab line (slim list, side-by-side form, slot picker), at most 50 lines.

```bash
git add ui/e2e/310-pack-editor-layout.cjs ui/ARCHITECTURE.md
git commit -m "test(ui): e2e of the pack editor layout and preview slot (t-1392)"
```

- [ ] **Step 4: Full gate**

Run: `just ci` (background; nohup with a log, see the long-runs memory)
Expected: green. Then mark the sub-tasks and t-1392 done with `oku cli task done --project slider --id t-N --result "…"`.

---

## Out of scope (follow-ups)

- Theme tab boolean tokens still use a Yes/No `Select` (`theme_controls.rs` `Control::Bool`); they could use `Switch` too.
- Choice options (`cover`, `contain`, `rows`…) are still shown by id.
