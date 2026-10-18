# Typed packs, lot 1 — several widget packs per deck — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A deck embeds several namespaced widget packs (`deck.widgets`), pack widgets are named `namespace/name`, and each pack renders, probes and falls back on its own.

**Architecture:** `oku-slides-core` gains `Namespace`, `PackPin` and the `qualify` / `split_kind` pair (the one place a qualified kind is built); `Deck.templates` becomes `Deck.widgets: BTreeMap<Namespace, PackPin>`, edited by `Op::AddPack` / `Op::RemovePack`. The registry keys pack widgets by qualified name and drops base overrides. `oku-slides-render` assembles a `PackSet` (one assembly module per pack, prefixed from the manifest); the worker protocol carries the set. The session resolves, embeds and probes each pinned pack, disables a broken one alone (and what requires it), and routes widget writes to an explicit namespace. The core widgets keep their bare names in this lot (they leave in lots 2 and 3).

**Tech Stack:** Rust (Snafu, serde, schemars, Loro 1.16.2, Typst, speculoos, rstest, insta), Fluent (20 locales), React/TS (Bun), Playwright e2e.

**Spec:** `docs/superpowers/specs/2026-10-18-packs-layouts-widgets-themes-design.md`

**Task:** parent t-1008; sub-tasks t-1010 … t-1019 (Task 1 … Task 10).

## Global Constraints

- Namespace syntax: `[a-z][a-z0-9-]*(\.[a-z][a-z0-9-]*)?`, at most 64 bytes. The namespace `deck` is reserved for the deck's own pack (scope `deck`).
- Qualified kind: `<namespace>/<name>`, built only by `oku_slides_core::qualify`, split only by `oku_slides_core::split_kind`. A bare kind is a core widget.
- Built-in packs: Essentiel `namespace: "essentiel"`, Grilles `namespace: "grilles"` (its widget becomes `grilles/citation`). Both keep their templates until lot 3.
- Formats: `DECK_VERSION` 5 → 6, `REPLICA_FORMAT` 4 → 5, `SOURCES_VERSION` 10 → 11, worker `PROTOCOL_VERSION` 3 → 4. Older decks and sources are refused, never migrated.
- Pack manifest file: `widgets.json` (was `pack.json`), in the library, beside a deck, and in the sources zip under `widgets/<hex64>/widgets.json` + `widgets/<hex64>/widgets/<name>.typ` (was `packs/`).
- No `unwrap`/`expect` outside tests; no silent fallback; Snafu errors with a backtrace per variant, opaque public types, Fluent messages.
- Every new Fluent key in all 20 locales (`crates/oku-slides-i18n/src/locales/*/`), same position, translated; every removed key removed from all 20.
- Rust unit tests in sibling `x_tests.rs`, speculoos, names `unit_scenario_result`, rstest for tables; regressions in `tests/regressions/issue_NNNN_topic.rs`.
- Every pub item documented, first sentence under 15 words; code comments in French; docs in English.
- Source files under 500 lines when possible, never above 1000.
- Commits: `type(scope) : description (t-N)`, message in a file passed with `git commit -F`; **no `Co-Authored-By` line** (user instruction).
- Runs outside `just e2e` set `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS` to a fresh temp dir.

## Deviations from the spec (approved with this plan; spec §9 updated in the same commit)

1. **Lot 1 covers widget packs only.** `deck.layouts` and layout packs move to lot 3 (with the layouts), theme `extends` to lot 4. No empty field is introduced ahead of its content.
2. **`deck.widgets` is a map** `BTreeMap<Namespace, PackPin>` instead of `Vec<PackRef>`: "one version per namespace" is enforced by the type, and two peers adding different packs both keep theirs on merge (one LWW atom per namespace).
3. **A pin carries `requires`.** `PackPin { id, version, requires }` lets core refuse a `RemovePack` and check closures without reading pack files; sources check pin ↔ manifest consistency.
4. **The session materializes the closure.** `Op::AddPack` only sets one pin and refuses a missing requirement; `Session::apply` prepends `AddPack` for every missing required namespace (dependencies first), from the library.
5. **Deck-mode widget writes** go to an explicit namespace; without one, to the deck's own pack `deck` (scope `deck`), created on first write. `AgentScope::Widget { pack }` and the atelier keep targeting the pack they edit.
6. **Per-pack fallback renders the existing "unknown element" box** (`data.typ:151`) for the widgets of a disabled pack, plus one notice naming every disabled pack. No new fallback frame.
7. **Registries memoize the assembled registry per `PackSet` fingerprint**; per-file probes stay memoized by content (`FileProbeCache`), so changing one pack never re-probes another pack's files.
8. **No `CrossPackImport` error.** `probe_file` compiles each widget file alone with its pack's `_commun` only, so an `#import` of another pack's path fails there and the widget is dropped like any broken file. Task 4 proves it with a test.
9. **No `SourcesVersionTooOld` error.** An older sources version already raises `SourcesErrorKind::NotSources` (`sources-not-sources`); Task 3 adds version 10 to that test table.
10. **Intermediate commits.** Tasks 2–7 change one crate layer at a time, bottom-up; each task's gate is the crates it touches (`cargo nextest run -p …`). The whole workspace is green again from Task 8; `just ci` and `just e2e` run in Task 10. The branch is squash-merged.

---

### Task 1: `Namespace`, `qualify`, and namespaced pack manifests

Additive: the workspace stays green.

**Files:**
- Create: `crates/oku-slides-core/src/namespace.rs`, `crates/oku-slides-core/src/namespace_tests.rs`
- Modify: `crates/oku-slides-core/src/lib.rs` (module + re-exports), `crates/oku-slides-core/src/pack.rs:332-356` (`PackManifest`), `crates/oku-slides-core/src/pack_tests.rs`, `crates/oku-slides-core/src/error/mod.rs` (`NamespaceError`)
- Modify (struct literals gain `namespace`, `requires`): `crates/oku-slides-import/src/hifi/pack.rs` (namespace `deck`), `crates/oku-slides-session/src/{pack.rs,templates_mode.rs,test_support.rs}`, `crates/oku-slides-session/tests/regressions/issue_0327*.rs`, `issue_0328*.rs`, `issue_0329*.rs`, `crates/oku-slides-cli/tests/{cli.rs,pack_broken.rs}`, `crates/oku-slides-cli/tests/regressions/issue_0355*.rs`
- Modify: `templates/essentiel/pack.json` (`"namespace": "essentiel", "requires": []`), `templates/grilles/pack.json` (`"namespace": "grilles", "requires": []`)

**Interfaces:**
- Produces: `pub struct Namespace(String)` (`parse`, `as_str`, `deck()`, `is_deck()`, `Display`, serde as string, `JsonSchema` as string, `Ord`); `pub fn qualify(ns: &Namespace, name: &str) -> String`; `pub fn split_kind(kind: &str) -> Option<(&str, &str)>`; `PackManifest.namespace: Namespace`, `PackManifest.requires: Vec<Namespace>`.

- [ ] **Step 1: Write the failing tests** (`namespace_tests.rs`)

```rust
use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
#[case("base")]
#[case("essentiel")]
#[case("grilles-2")]
#[case("acme.charts")]
fn namespace_parse_valid_is_some(#[case] s: &str) {
    assert_that!(Namespace::parse(s).map(|n| n.as_str().to_owned())).is_equal_to(Some(s.to_owned()));
}

#[rstest]
#[case("")]
#[case("Base")]
#[case("2d")]
#[case("a/b")]
#[case("a.b.c")]
#[case("a.")]
#[case(".a")]
#[case("a_b")]
fn namespace_parse_invalid_is_none(#[case] s: &str) {
    assert_that!(Namespace::parse(s)).is_none();
}

#[test]
fn namespace_parse_too_long_is_none() {
    assert_that!(Namespace::parse(&"a".repeat(65))).is_none();
}

#[test]
fn qualify_joins_with_a_slash() {
    let ns = Namespace::parse("base").unwrap();
    assert_that!(qualify(&ns, "card")).is_equal_to("base/card".to_owned());
}

#[rstest]
#[case("base/card", Some(("base", "card")))]
#[case("acme.charts/bar", Some(("acme.charts", "bar")))]
#[case("text", None)]
#[case("/card", None)]
#[case("base/", None)]
#[case("a/b/c", None)]
fn split_kind_reads_namespace_and_name(#[case] kind: &str, #[case] expected: Option<(&str, &str)>) {
    assert_that!(split_kind(kind)).is_equal_to(expected);
}

#[test]
fn namespace_deck_is_reserved_name() {
    assert_that!(Namespace::deck().is_deck()).is_true();
    assert_that!(Namespace::parse("deck").unwrap().is_deck()).is_true();
}

#[test]
fn namespace_serde_round_trips_as_string() {
    let ns: Namespace = serde_json::from_str("\"essentiel\"").unwrap();
    assert_that!(serde_json::to_string(&ns).unwrap()).is_equal_to("\"essentiel\"".to_owned());
    assert_that!(serde_json::from_str::<Namespace>("\"Bad\"")).is_err();
}
```

In `pack_tests.rs`, add:

```rust
#[test]
fn pack_manifest_without_namespace_is_refused() {
    let json = json!({"id": Uuid::nil(), "name": "P", "created": "2026-10-18T00:00:00Z", "templates": []});
    assert_that!(serde_json::from_value::<PackManifest>(json)).is_err();
}

#[test]
fn pack_manifest_reads_namespace_and_requires() {
    let json = json!({"id": Uuid::nil(), "name": "P", "namespace": "acme", "requires": ["base"],
        "created": "2026-10-18T00:00:00Z", "templates": []});
    let m: PackManifest = serde_json::from_value(json).unwrap();
    assert_that!((m.namespace.as_str().to_owned(), m.requires.len())).is_equal_to(("acme".to_owned(), 1));
}
```

- [ ] **Step 2: Run, expect FAIL** — `cargo nextest run -p oku-slides-core namespace pack_manifest` → does not compile (`Namespace` unknown).

- [ ] **Step 3: Implement `namespace.rs`**

```rust
//! Espaces de noms des packs et noms qualifiés des widgets de pack.
//!
//! Un widget de pack s'appelle `<namespace>/<nom>` dans un deck ; seul
//! [`qualify`] forme ce nom et seul [`split_kind`] le lit. Un nom sans `/`
//! est un widget du noyau.

use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{NamespaceError, NamespaceSnafu};

/// Longueur maximale d'un espace de noms, en octets.
const MAX_NAMESPACE: usize = 64;

/// L'espace de noms réservé au pack propre d'un deck.
const DECK: &str = "deck";

/// Espace de noms d'un pack : `[a-z][a-z0-9-]*`, éventuellement `éditeur.nom`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
#[schemars(with = "String")]
pub struct Namespace(String);

/// Un segment : `[a-z][a-z0-9-]*`.
fn segment(s: &str) -> bool {
    let mut bytes = s.bytes();
    bytes.next().is_some_and(|b| b.is_ascii_lowercase())
        && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

impl Namespace {
    /// Lit un espace de noms ; `None` hors de la syntaxe permise.
    pub fn parse(s: &str) -> Option<Self> {
        let ok = s.len() <= MAX_NAMESPACE && {
            let mut parts = s.split('.');
            let first = parts.next().is_some_and(segment);
            let second = parts.next().is_none_or(segment);
            first && second && parts.next().is_none()
        };
        ok.then(|| Self(s.to_owned()))
    }

    /// L'espace de noms `deck`, celui du pack propre d'un deck.
    pub fn deck() -> Self {
        Self(DECK.to_owned())
    }

    /// Vrai pour l'espace de noms réservé `deck`.
    pub fn is_deck(&self) -> bool {
        self.0 == DECK
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Namespace {
    type Error = NamespaceError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(&s).ok_or_else(|| NamespaceSnafu { value: s }.build())
    }
}

impl From<Namespace> for String {
    fn from(n: Namespace) -> Self {
        n.0
    }
}

impl fmt::Display for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Le nom qualifié d'un widget de pack : `<namespace>/<nom>`.
pub fn qualify(ns: &Namespace, name: &str) -> String {
    format!("{ns}/{name}")
}

/// Espace de noms et nom d'un `kind` qualifié ; `None` pour un widget du noyau.
pub fn split_kind(kind: &str) -> Option<(&str, &str)> {
    let (ns, name) = kind.split_once('/')?;
    (Namespace::parse(ns).is_some() && !name.is_empty() && !name.contains('/')).then_some((ns, name))
}

#[cfg(test)]
#[path = "namespace_tests.rs"]
mod namespace_tests;
```

In `error/mod.rs`, next to `WidgetFileError` (same pattern, Fluent key not needed: it surfaces only through serde):

```rust
/// Un espace de noms de pack hors de la syntaxe permise.
#[derive(Debug, Snafu)]
#[snafu(display("invalid pack namespace: {value}"))]
pub struct NamespaceError {
    value: String,
    backtrace: Backtrace,
}
```

with `#[snafu(context(suffix(Snafu)), visibility(pub(crate)))]` exactly like `WidgetFileError`. In `lib.rs`: `mod namespace;` and `pub use namespace::{Namespace, qualify, split_kind};`, `pub use error::NamespaceError;`.

In `PackManifest` (after `name`):

```rust
    /// Espace de noms des widgets du pack : leur `kind` est `<namespace>/<nom>`.
    pub namespace: Namespace,
    /// Espaces de noms des packs de widgets dont ce pack a besoin.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<Namespace>,
```

- [ ] **Step 4: Update every `PackManifest { … }` literal** listed under Files (`rg -n 'PackManifest \{' crates`): test packs get `namespace: Namespace::parse("<lowercase name of the test pack>")` with an explicit value per pack, `requires: vec![]`; `oku-slides-import/src/hifi/pack.rs` uses `Namespace::deck()`. Add `"namespace"` and `"requires": []` to both built-in `pack.json` files.

- [ ] **Step 5: Run, expect PASS** — `cargo nextest run --workspace --exclude oku-slides-app` and `just check`.

- [ ] **Step 6: Commit** — `feat(core) : espaces de noms des packs et noms qualifiés des widgets (t-1010)`.

---

### Task 2: `deck.widgets`, `PackPin`, `AddPack` / `RemovePack`

Core only. Downstream crates stop compiling until Task 6–9 (deviation 8).

**Files:**
- Modify: `crates/oku-slides-core/src/pack.rs` (`PackPin`, remove `PackRef`), `src/model.rs:19-43,455-465` (`DECK_VERSION = 6`, field), `src/ops/mod.rs:77-253,493,672-679` (ops), `src/error/mod.rs` (`OpInner` / `OpErrorKind` / `Localize`), `src/replica/schema.rs:15-30,105-120` (`Ctx::PackPins`), `src/replica/mod.rs:68` (`REPLICA_FORMAT = 5`), `src/lib.rs`
- Create: `src/ops/packs.rs`, `src/ops/packs_tests.rs`
- Modify tests: `src/model_tests.rs:19-28`, `src/canonical_tests.rs:27,423`, `src/replica/mod_tests.rs:727-750`, `src/test_registry.rs`, `tests/ops.rs:100-105`, `src/ops/schema_tests.rs`
- Modify i18n: `crates/oku-slides-i18n/src/locales/*/core.ftl` (4 new `op-*` keys)

**Interfaces:**
- Consumes: `Namespace`, `split_kind` (Task 1).
- Produces:
  - `pub struct PackPin { pub id: Uuid, pub version: PackVersion, #[serde(default, skip_serializing_if = "Vec::is_empty")] pub requires: Vec<Namespace> }` (derives like `PackRef` today, `JsonSchema`).
  - `Deck.widgets: BTreeMap<Namespace, PackPin>` (`#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]`), replacing `templates`.
  - `Op::AddPack { namespace: Namespace, pin: PackPin }`, `Op::RemovePack { namespace: Namespace }` (serde `add_pack`, `remove_pack`), replacing `Op::SetTemplates`.
  - `OpErrorKind::{UnknownNamespace, PackRequired, PackInUse, PackRequirementMissing}`; Fluent keys `op-unknown-namespace`, `op-pack-required`, `op-pack-in-use`, `op-pack-requirement-missing`.
  - `pub fn pack_users(deck: &Deck, ns: &str) -> Vec<String>` (ids of slides and elements whose kind is in `ns`), in `ops/packs.rs`, re-exported.

- [ ] **Step 1: Write the failing tests** (`ops/packs_tests.rs`)

```rust
use speculoos::prelude::*;
use uuid::Uuid;

use super::*;
use crate::test_registry::registry;
use crate::{Deck, Element, Namespace, OpErrorKind, PackPin, PackVersion, Slide, apply_all};

fn ns(s: &str) -> Namespace {
    Namespace::parse(s).unwrap()
}

fn pin(requires: &[&str]) -> PackPin {
    PackPin {
        id: Uuid::new_v4(),
        version: PackVersion::parse(&format!("sha256:{}", "a".repeat(64))).unwrap(),
        requires: requires.iter().map(|r| ns(r)).collect(),
    }
}

fn add(n: &str, p: PackPin) -> Op {
    Op::AddPack { namespace: ns(n), pin: p }
}

#[test]
fn add_pack_new_namespace_pins_it() {
    let mut deck = Deck::empty();
    apply_all(&mut deck, &registry(), vec![add("acme", pin(&[]))]).unwrap();
    assert_that!(deck.widgets.contains_key(&ns("acme"))).is_true();
}

#[test]
fn add_pack_same_namespace_replaces_the_pin() {
    let mut deck = Deck::empty();
    let (a, b) = (pin(&[]), pin(&[]));
    apply_all(&mut deck, &registry(), vec![add("acme", a), add("acme", b.clone())]).unwrap();
    assert_that!(deck.widgets.get(&ns("acme"))).is_equal_to(Some(&b));
}

#[test]
fn add_pack_missing_requirement_is_refused() {
    let mut deck = Deck::empty();
    let err = apply_all(&mut deck, &registry(), vec![add("acme", pin(&["base"]))]).unwrap_err();
    assert_that!(err.op_kind()).is_equal_to(Some(OpErrorKind::PackRequirementMissing));
}

#[test]
fn add_pack_requirement_added_first_in_same_transaction_is_accepted() {
    let mut deck = Deck::empty();
    let ops = vec![add("base", pin(&[])), add("acme", pin(&["base"]))];
    assert_that!(apply_all(&mut deck, &registry(), ops)).is_ok();
}

#[test]
fn remove_pack_unknown_namespace_is_refused() {
    let mut deck = Deck::empty();
    let err = apply_all(&mut deck, &registry(), vec![Op::RemovePack { namespace: ns("acme") }]).unwrap_err();
    assert_that!(err.op_kind()).is_equal_to(Some(OpErrorKind::UnknownNamespace));
}

#[test]
fn remove_pack_required_by_another_is_refused() {
    let mut deck = Deck::empty();
    apply_all(&mut deck, &registry(), vec![add("base", pin(&[])), add("acme", pin(&["base"]))]).unwrap();
    let err = apply_all(&mut deck, &registry(), vec![Op::RemovePack { namespace: ns("base") }]).unwrap_err();
    assert_that!(err.op_kind()).is_equal_to(Some(OpErrorKind::PackRequired));
}

#[test]
fn remove_pack_used_by_an_element_is_refused() {
    let mut deck = Deck::empty();
    deck.widgets.insert(ns("acme"), pin(&[]));
    let mut slide = Slide::new("s1", "blank");
    slide.params.insert_elements("elements", vec![Element::new("e1", "acme/card")]);
    deck.slides.push(slide);
    let err = apply_all(&mut deck, &registry(), vec![Op::RemovePack { namespace: ns("acme") }]).unwrap_err();
    assert_that!(err.op_kind()).is_equal_to(Some(OpErrorKind::PackInUse));
}

#[test]
fn remove_pack_unused_unpins_it() {
    let mut deck = Deck::empty();
    deck.widgets.insert(ns("acme"), pin(&[]));
    apply_all(&mut deck, &registry(), vec![Op::RemovePack { namespace: ns("acme") }]).unwrap();
    assert_that!(deck.widgets.is_empty()).is_true();
}

#[test]
fn pack_users_lists_slides_and_nested_elements() {
    let mut deck = Deck::empty();
    let mut slide = Slide::new("s1", "acme/cover");
    slide.params.insert_elements("elements", vec![Element::new("e1", "acme/card"), Element::new("e2", "text")]);
    deck.slides.push(slide);
    assert_that!(pack_users(&deck, "acme")).is_equal_to(vec!["s1".to_owned(), "e1".to_owned()]);
}
```

Before writing these tests, read `model.rs` / `params.rs` for the real helper that puts elements into a slot param; if `insert_elements` does not exist, build the `elements` slot with the same JSON construction `tests/ops.rs` uses, and use the existing accessor for an `OpError`'s kind in place of `op_kind()` (read `apply_all`'s error type: a `TransactionError` wrapping an `OpError`). Keep one behavior per test.

In `model_tests.rs` replace the two `templates` tests by:

```rust
#[test]
fn deck_from_json_reads_widget_pins_by_namespace() {
    let json = json!({"version": 6, "widgets": {"essentiel": {"id": Uuid::nil(), "version": format!("sha256:{}", "b".repeat(64))}}, "slides": []});
    let deck = Deck::from_json(&json.to_string()).unwrap();
    assert_that!(deck.widgets.keys().map(Namespace::as_str).collect::<Vec<_>>()).is_equal_to(vec!["essentiel"]);
}

#[test]
fn deck_from_json_version_5_is_refused() {
    assert_that!(Deck::from_json(r#"{"version": 5, "slides": []}"#)).is_err();
}
```

In `replica/mod_tests.rs` replace `merge_of_concurrent_templates_keeps_one_whole_pack_ref` by two tests:
`merge_of_concurrent_adds_of_two_namespaces_keeps_both` (peer A pins `acme`, peer B pins `base` → merged deck has both) and `merge_of_concurrent_pins_of_one_namespace_keeps_one_whole_pin` (both pin `acme` with different versions and requires → merged pin equals one of the two, never a mix). Follow the existing test's peer setup verbatim.

- [ ] **Step 2: Run, expect FAIL** — `cargo nextest run -p oku-slides-core` → does not compile.

- [ ] **Step 3: Implement**

`pack.rs`: replace `PackRef` by

```rust
/// Un pack de widgets épinglé par un deck : id, version embarquée, besoins.
///
/// `requires` recopie celui du manifeste de la version : le cœur refuse ainsi
/// un retrait sans lire les fichiers ; les sources vérifient la copie.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct PackPin {
    #[schemars(with = "String")]
    pub id: Uuid,
    pub version: PackVersion,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<Namespace>,
}
```

`model.rs`: `pub const DECK_VERSION: u32 = 6;` and the field

```rust
    /// Packs de widgets du deck, par espace de noms ; fermeture de leurs `requires` comprise.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub widgets: BTreeMap<Namespace, PackPin>,
```

`Deck::empty()` sets `widgets: BTreeMap::new()`.

`ops/mod.rs`: replace `SetTemplates` by

```rust
    /// Épingle la version `pin` du pack de widgets `namespace` (ajout ou remplacement).
    ///
    /// Chaque espace de noms de `pin.requires` doit déjà être épinglé : la
    /// session ajoute les besoins d'abord, dans la même transaction.
    AddPack { namespace: Namespace, pin: PackPin },
    /// Retire le pack `namespace`, s'il n'est ni requis ni utilisé.
    RemovePack { namespace: Namespace },
```

and in `Op::apply`: `Op::AddPack { namespace, pin } => packs::add(deck, namespace, pin)`, `Op::RemovePack { namespace } => packs::remove(deck, &namespace)`.

`ops/packs.rs`:

```rust
//! Épinglage des packs de widgets d'un deck : ajout, retrait, utilisateurs.

use snafu::ensure;

use crate::error::{OpError, PackInUseSnafu, PackRequiredSnafu, PackRequirementMissingSnafu, UnknownNamespaceSnafu};
use crate::{Deck, Namespace, PackPin, split_kind};

/// Épingle `pin` sous `ns` ; refuse un besoin non épinglé.
pub(super) fn add(deck: &mut Deck, ns: Namespace, pin: PackPin) -> Result<(), OpError> {
    if let Some(missing) = pin.requires.iter().find(|r| **r != ns && !deck.widgets.contains_key(*r)) {
        return Err(PackRequirementMissingSnafu { namespace: ns.to_string(), required: missing.to_string() }.build().into());
    }
    deck.widgets.insert(ns, pin);
    Ok(())
}

/// Retire le pack `ns` ; refuse s'il est requis par un autre ou utilisé.
pub(super) fn remove(deck: &mut Deck, ns: &Namespace) -> Result<(), OpError> {
    ensure!(deck.widgets.contains_key(ns), UnknownNamespaceSnafu { namespace: ns.to_string() });
    if let Some((by, _)) = deck.widgets.iter().find(|(other, p)| *other != ns && p.requires.contains(ns)) {
        return Err(PackRequiredSnafu { namespace: ns.to_string(), by: by.to_string() }.build().into());
    }
    let users = pack_users(deck, ns.as_str());
    ensure!(users.is_empty(), PackInUseSnafu { namespace: ns.to_string(), users });
    deck.widgets.remove(ns);
    Ok(())
}

/// Ids des slides et éléments (imbriqués compris) dont le widget est du pack `ns`.
pub fn pack_users(deck: &Deck, ns: &str) -> Vec<String> {
    let of_ns = |kind: &str| split_kind(kind).is_some_and(|(n, _)| n == ns);
    let mut out = Vec::new();
    for slide in &deck.slides {
        if of_ns(&slide.kind) {
            out.push(slide.id.clone());
        }
        slide.walk_elements(&mut |e| {
            if of_ns(&e.kind) {
                out.push(e.id.clone());
            }
        });
    }
    out
}

#[cfg(test)]
#[path = "packs_tests.rs"]
mod packs_tests;
```

Use the slide's existing recursive element traversal (`locate.rs` / `fields.rs` walk slots up to `MAX_SLOT_DEPTH`); if no visitor exists, add `Slide::walk_elements(&self, f: &mut impl FnMut(&Element))` in `model.rs` built on the same slot iteration `locate.rs` uses, with its own test in `model_tests.rs`.

`error/mod.rs` (`OpInner`, same style as `UnknownWidget`):

```rust
    #[snafu(display("no widget pack {namespace} in this deck"))]
    UnknownNamespace { namespace: String, backtrace: Backtrace },
    #[snafu(display("widget pack {namespace} is required by {by}"))]
    PackRequired { namespace: String, by: String, backtrace: Backtrace },
    #[snafu(display("widget pack {namespace} is used by {}", users.join(", ")))]
    PackInUse { namespace: String, users: Vec<String>, backtrace: Backtrace },
    #[snafu(display("widget pack {namespace} needs {required} first"))]
    PackRequirementMissing { namespace: String, required: String, backtrace: Backtrace },
```

Map them to the four new `OpErrorKind` variants, and in `Localize for OpError` handle them before the generic `id` match, like `ListBound`:
`op-unknown-namespace { $pack }`, `op-pack-required { $pack } { $by }`, `op-pack-in-use { $pack } { $count }` (count of users), `op-pack-requirement-missing { $pack } { $required }`.
English (`en/core.ftl`, after `op-unknown-widget`):

```ftl
op-unknown-namespace = This deck has no widget pack { $pack }.
op-pack-required = The widget pack { $pack } is needed by { $by }; remove that one first.
op-pack-in-use = The widget pack { $pack } is still used by { $count } { $count -> [one] item *[other] items }.
op-pack-requirement-missing = The widget pack { $pack } needs { $required }, which this deck does not have.
```

Add the four keys at the same position in the 19 other locales, translated.

`replica/schema.rs`: add `Ctx::PackPins` (doc: "chaque entrée de `widgets` est une feuille LWW : un épinglage entier, jamais champ à champ"), map `(Deck, "widgets") => Kind::Nested(PackPins)`, `(PackPins, _) => Kind::Atom`, remove `"templates"` from the `(Deck, "templates" | "theme")` arm, and add `Ctx::PackPins` to the `&[]` arm of `texts()`/`trees()`. `replica/mod.rs:68`: `REPLICA_FORMAT: i64 = 5`.

`ops/schema_tests.rs` and `ui/src/api.ts` stay in sync later (Task 9): for now the schema test lists `add_pack` and `remove_pack` — mark nothing; it will fail until Task 9 updates `api.ts`. Note this in the task report.

`test_registry.rs` / `tests/common/mod.rs` / `tests/ops.rs`: replace `Op::SetTemplates` and `templates: None` literals with `Op::AddPack` / `widgets: BTreeMap::new()`.

- [ ] **Step 4: Run, expect PASS** — `cargo nextest run -p oku-slides-core` (except `ops::schema_tests`, which waits for Task 9) and `cargo nextest run -p oku-slides-i18n`.

- [ ] **Step 5: Commit** — `feat(core)! : packs de widgets épinglés par espace de noms, ops add_pack et remove_pack (t-1011)`.

---

### Task 3: Sources format 11 — `widgets/` in the zip, pin checks

**Files:**
- Modify: `crates/oku-slides-core/src/sources.rs:1-16,45-46,136-165` (`SOURCES_VERSION = 11`, doc, `embedded_issues`), `src/sources_packs.rs` (dir `widgets/`, manifest `widgets.json`), `src/validate.rs:36-63` (new issues), tests `src/sources_tests.rs:27,72-117,519-552`, `src/sources_packs_tests.rs`, `tests/zip_limits.rs:56`
- Modify i18n: `*/core.ftl` (2 new `issue-*` keys)

**Interfaces:**
- Consumes: `PackPin`, `Deck.widgets` (Task 2).
- Produces: `Issue::pack_pin_mismatch(ns: &Namespace)` (`issue-pack-pin-mismatch`: the embedded manifest's id, namespace or requires differ from the pin), `Issue::pack_requirement_missing(ns, required)` (`issue-pack-requirement-missing`), `Issue::pack_version_missing` unchanged; `PACKS_DIR = "widgets/"`, `PACK_JSON = "widgets.json"`.

- [ ] **Step 1: Write the failing tests** (`sources_packs_tests.rs`, `sources_tests.rs`)

```rust
#[test]
fn sources_pack_written_under_widgets_dir() {
    let (bundle, files) = bundle_with_pack("acme");
    let names = zip_names(&bundle.to_zip().unwrap());
    let hex = files.version().hex().to_owned();
    assert_that!(names.contains(&format!("widgets/{hex}/widgets.json"))).is_true();
}

#[test]
fn sources_pin_with_other_requires_than_manifest_reports_mismatch() {
    let (mut bundle, files) = bundle_with_pack("acme");
    let mut deck = bundle.deck().clone();
    deck.widgets.get_mut(&ns("acme")).unwrap().requires = vec![ns("base")];
    deck.widgets.insert(ns("base"), pin_of(&files));
    bundle.apply_silent(deck).unwrap();
    let (_, issues) = SourceBundle::from_zip(&bundle.to_zip().unwrap(), &limits()).unwrap();
    assert_that!(keys(&issues)).contains("issue-pack-pin-mismatch".to_owned());
}

#[test]
fn sources_pin_requiring_an_unpinned_namespace_reports_it() {
    // deck JSON written by hand with acme requiring base, base absent
    let issues = issues_of_deck_json(json!({"version": 6, "widgets": {"acme": pin_json("acme", &["base"])}, "slides": []}));
    assert_that!(keys(&issues)).contains("issue-pack-requirement-missing".to_owned());
}

#[rstest]
#[case(10)]
#[case(9)]
fn sources_older_version_is_not_sources(#[case] version: u32) {
    assert_that!(read_manifest_version(version).map_err(|e| e.kind())).is_equal_to(Err(SourcesErrorKind::NotSources));
}
```

`bundle_with_pack`, `pin_of`, `pin_json`, `zip_names`, `keys`, `limits`, `issues_of_deck_json`, `read_manifest_version` are local helpers: reuse the existing ones in these test files (the current `set_templates` helper becomes `pin_pack(bundle, ns, files)`); write the missing ones at the top of the file, each a few lines over `SourceBundle` public API. Update the `MANIFEST` constant to version 11 and add 10 to the existing `NotSources` versions table.

- [ ] **Step 2: Run, expect FAIL** — `cargo nextest run -p oku-slides-core sources`.

- [ ] **Step 3: Implement**
  - `sources.rs`: `SOURCES_VERSION: u32 = 11`; module doc lists version 11 and adds 10 to the versions no longer read.
  - `sources_packs.rs`: `PACKS_DIR = "widgets/"`, the manifest entry name `widgets.json` (a constant `PACK_JSON` used by both `pack_entry` and `write_packs`); everything else unchanged.
  - `embedded_issues()`:

```rust
    /// Chaque épinglage du deck : version embarquée, manifeste conforme, besoins épinglés.
    fn embedded_issues(&self) -> Vec<Issue> {
        let mut issues = Vec::new();
        for (ns, pin) in &self.deck.widgets {
            match self.packs.get(&pin.version).map(PackManifest::from_files) {
                None => issues.push(Issue::pack_version_missing(&pin.version)),
                Some(Ok(m)) if m.id == pin.id && m.namespace == *ns && m.requires == pin.requires => {}
                Some(_) => issues.push(Issue::pack_pin_mismatch(ns)),
            }
            for r in pin.requires.iter().filter(|r| !self.deck.widgets.contains_key(*r)) {
                issues.push(Issue::pack_requirement_missing(ns, r));
            }
        }
        issues.extend(theme_issues(&self.deck, &self.themes));
        issues
    }
```

  - `validate.rs`: the two constructors, beside `pack_version_missing`, with keys and args `pack` / `required`.
  - Fluent (`en/core.ftl`, after `issue-pack-version-missing`):

```ftl
issue-pack-pin-mismatch = The embedded widget pack { $pack } does not match what the deck expects.
issue-pack-requirement-missing = The widget pack { $pack } needs { $required }, which this deck does not have.
```

  and the 19 other locales.

- [ ] **Step 4: Run, expect PASS** — `cargo nextest run -p oku-slides-core -p oku-slides-i18n`.

- [ ] **Step 5: Commit** — `feat(core)! : sources 11, packs de widgets sous widgets/ et épinglages vérifiés (t-1012)`.

---

### Task 4: Registry of qualified pack widgets, `PackSet` assembly in render

**Files:**
- Modify: `crates/oku-slides-core/src/widget.rs:163-222,330-400` (probe shape, delete `check_override` / `compatible`), `src/widget_tests.rs`, `src/error/mod.rs` (delete `IncompatibleCategory`, `IncompatibleOverride` and their Fluent keys in 20 locales)
- Modify: `crates/oku-slides-render/src/registry.rs` (`PackWidgets.namespace`, `PackSet`, assembly, probe source, `Registries`), `src/world.rs:66-130,265` (`main_source`, `set_pack` → `set_packs`), `src/lib.rs:208` (`Renderer::set_widgets(&PackSet, …)`), `src/widget_probe.rs:174` (`assemble`), `src/registry_tests.rs`, `src/lib.rs` re-exports, `tests/common/mod.rs`, render tests that build a `PackWidgets`

**Interfaces:**
- Consumes: `Namespace`, `qualify` (Task 1).
- Produces:
  - core: probe JSON `{"base": [schema], "packs": [{"namespace": "acme", "widgets": [schema]}]}`; `WidgetRegistry::from_probe` keys pack widgets by `qualify(namespace, schema.name)` and stores that qualified name in `WidgetSchema.name`.
  - render: `PackWidgets::new(id: impl Into<String>, namespace: Namespace, files) -> Result<Self, RegistryError>`, `PackWidgets::namespace(&self) -> &Namespace`; `pub struct PackSet(Vec<PackWidgets>)` with `PackSet::new(Vec<PackWidgets>) -> Self` (sorted by namespace), `packs()`, `is_empty()`, `fingerprint() -> [u8; 32]`; `Registries::get(&mut self, set: &PackSet)`; `load_registry(fonts, &PackSet)`; `Renderer::set_widgets(&mut self, set: &PackSet, registry: Arc<WidgetRegistry>)`.

- [ ] **Step 1: Write the failing tests**

`widget_tests.rs`:

```rust
#[test]
fn registry_from_probe_qualifies_pack_widgets() {
    let probe = json!({"base": [schema("text")], "packs": [{"namespace": "acme", "widgets": [schema("card")]}]});
    let r = WidgetRegistry::from_probe(probe).unwrap();
    assert_that!(r.get("acme/card").map(|w| w.name.clone())).is_equal_to(Some("acme/card".to_owned()));
}

#[test]
fn registry_from_probe_pack_widget_named_like_base_keeps_both() {
    let probe = json!({"base": [schema("card")], "packs": [{"namespace": "acme", "widgets": [schema("card")]}]});
    let r = WidgetRegistry::from_probe(probe).unwrap();
    assert_that!((r.get("card").is_some(), r.get("acme/card").is_some())).is_equal_to((true, true));
}

#[test]
fn registry_from_probe_slot_accepting_a_qualified_sibling_is_valid() {
    let holder = slot_schema("holder", &["acme/card"]);
    let probe = json!({"base": [], "packs": [{"namespace": "acme", "widgets": [schema("card"), holder]}]});
    assert_that!(WidgetRegistry::from_probe(probe)).is_ok();
}
```

(`schema`, `slot_schema`: JSON builders in the test file, mirroring the schemas already used there.) Delete the override tests (`rg -n 'override' crates/oku-slides-core/src/widget_tests.rs`).

`registry_tests.rs`:

```rust
#[test]
fn pack_set_assembly_prefixes_names_with_the_namespace() {
    let pack = PackWidgets::new("abc", ns("acme"), files(&[("card", CARD_SRC)])).unwrap();
    assert_that!(pack.assembly()).contains("\"acme/\" + w.name");
}

#[test]
fn load_registry_two_packs_registers_both_namespaces() {
    let set = PackSet::new(vec![pack("a1", "acme", "card"), pack("b1", "beta", "card")]);
    let r = load_registry(&fonts(), &set).unwrap();
    assert_that!((r.get("acme/card").is_some(), r.get("beta/card").is_some())).is_equal_to((true, true));
}

#[test]
fn render_deck_with_two_packs_renders_each_qualified_widget() {
    // deck with elements acme/card and beta/card; both boxes present in layout()
}

#[test]
fn pack_set_fingerprint_is_order_independent() {
    let a = PackSet::new(vec![pack("a1", "acme", "card"), pack("b1", "beta", "card")]);
    let b = PackSet::new(vec![pack("b1", "beta", "card"), pack("a1", "acme", "card")]);
    assert_that!(a.fingerprint()).is_equal_to(b.fingerprint());
}
```

Write the third test fully with the render test helpers in `tests/common/mod.rs` (deck JSON with one free slide holding two elements, `Renderer::set_widgets`, `render`, then assert `layout()` has an `ElementBox` per element id).

Also in `registry_tests.rs` (or the `widget_probe` tests):

```rust
#[test]
fn probe_file_importing_another_pack_is_broken() {
    let src = "#import \"/packs/0123abcd/widgets/card.typ\": widget as other\n#let widget = other\n";
    let probe = registries().probe_file(&WidgetFile::parse("thief").unwrap(), src, None);
    assert_that!(probe.is_broken()).is_true();
}
```

Use the `FileProbe` accessor that tells a failed probe (read `widget_probe.rs`); the test documents spec §2's cross-pack import ban.

- [ ] **Step 2: Run, expect FAIL** — `cargo nextest run -p oku-slides-core -p oku-slides-render registry`. (The cross-pack test may already pass: it pins existing behavior.)

- [ ] **Step 3: Implement**

core `widget.rs`:

```rust
#[derive(Deserialize)]
struct ProbedPack {
    namespace: Namespace,
    widgets: Vec<WidgetSchema>,
}

#[derive(Deserialize)]
struct Probe {
    base: Vec<WidgetSchema>,
    #[serde(default)]
    packs: Vec<ProbedPack>,
}

impl WidgetRegistry {
    /// Lit la sonde `{"base": [..], "packs": [{namespace, widgets}]}` émise par Typst.
    ///
    /// Un widget de pack est rangé sous son nom qualifié (`acme/card`), qui
    /// devient aussi son `name` : il ne remplace jamais un widget du noyau.
    pub fn from_probe(probe: Value) -> Result<Self, WidgetError> {
        let probe: Probe = serde_json::from_value(probe).context(UnreadableSnafu)?;
        let mut widgets = collect(probe.base)?;
        for pack in probe.packs {
            for (name, mut w) in collect(pack.widgets)? {
                let qualified = qualify(&pack.namespace, &name);
                w.name = qualified.clone();
                widgets.insert(qualified, w);
            }
        }
        let registry = Self { widgets };
        registry.check_slots()?;
        Ok(registry)
    }
```

Delete `check_override`, `compatible` and their error variants, kinds, Fluent keys (20 locales) and tests. Update the struct doc ("ceux du noyau, et ceux de chaque pack sous leur nom qualifié").

render `registry.rs`:
- `PackWidgets { id, namespace: Namespace, files }`; `fingerprint` hashes the namespace after the id.
- `assembly()`:

```rust
    /// Module d'assemblage : importe chaque widget (pas `_commun`) et forme `registry`,
    /// chaque nom préfixé par l'espace de noms du pack (voir `oku_slides_core::qualify`).
    pub(crate) fn assembly(&self) -> String {
        let mut out = String::new();
        let mut names = String::new();
        for (i, name) in self.files.keys().filter(|n| !n.is_common()).enumerate() {
            out.push_str(&format!("#import \"widgets/{}\": widget as w{i}\n", name.file_name()));
            names.push_str(&format!("w{i},"));
        }
        // Le préfixe suit `qualify` : `<namespace>/<nom>` ; l'espace de noms est contrôlé.
        out.push_str(&format!(
            "#let registry = ({names}).map(w => w + (name: \"{ns}/\" + w.name))\n",
            ns = self.namespace
        ));
        out
    }
```

  Keep `schema.name` unprefixed in Typst: Rust qualifies it in `from_probe`, so the prefix rule lives in `qualify` for the registry and in this one generated line for rendering. Add a unit test asserting that the generated prefix equals `qualify(ns, "x").trim_end_matches('x')`, so the two stay equal.
- `PackSet`:

```rust
/// Les packs de widgets d'un deck, triés par espace de noms.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackSet(Vec<PackWidgets>);

impl PackSet {
    /// L'ensemble de `packs`, trié par espace de noms.
    pub fn new(mut packs: Vec<PackWidgets>) -> Self {
        packs.sort_by(|a, b| a.namespace.cmp(&b.namespace));
        Self(packs)
    }

    pub fn packs(&self) -> &[PackWidgets] {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Empreinte de l'ensemble : clé du cache des registres assemblés.
    pub fn fingerprint(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        for p in &self.0 {
            h.update(p.fingerprint());
        }
        h.finalize().into()
    }
}
```

- `probe_source(packs: &PackSet)`: one `#import "<path>": registry as pack{i}` per pack, then
  `#metadata((base: base.map(w => w.schema), packs: ((namespace: "<ns>", widgets: pack0.map(w => w.schema)), …)))<oku-widgets>`.
- `probe_registry`, `load_registry`, `Registries::get` take `&PackSet`; an empty set returns `base_registry`. The cache key becomes `PackSet::fingerprint()`.
- `world.rs` `main_source(has_theme, packs: &PackSet, …)`: one import per pack (`registry as pack-registry-{i}`) and `widgets: by-name(base-registry + pack-registry-0 + …)`; `set_pack` → `set_packs(&PackSet, &registry)` adding every pack's sources.
- `widget_probe::assemble(base, namespace, files)` returns the registry of base + that one pack (used by the worker per pack, Task 5).
- Fix every render test that builds `PackWidgets::new(id, files)` (add a namespace) and calls `set_widgets(Some(&p), …)` (pass `&PackSet::new(vec![p])`, or `&PackSet::default()` for `None`).

- [ ] **Step 4: Run, expect PASS** — `cargo nextest run -p oku-slides-core -p oku-slides-render`, including the golden test (`rg -l golden crates/oku-slides-render/tests`), which must stay pixel-identical.

- [ ] **Step 5: Commit** — `feat(render)! : registre des widgets qualifiés et assemblage de plusieurs packs (t-1013)`.

---

### Task 5: Render worker protocol 4 — a pack set per slot, per-pack probes

**Files:**
- Modify: `crates/oku-slides-render-worker/src/protocol.rs:10,17,62` (`PROTOCOL_VERSION = 4`, `WirePack.namespace`, `SetWidgets { slot, packs: Vec<WirePack> }`), `src/client.rs:112,243` (`wire`, `is_faulty`), `src/link.rs:48`, `src/serve.rs:132,167`, `src/slot.rs:50`, `src/pack_probe.rs:54-68`, `src/test_packs.rs`, their tests

**Interfaces:**
- Consumes: `PackWidgets`, `PackSet`, `assemble(base, namespace, files)` (Task 4).
- Produces: `RenderWorker::probe_pack(&self, pack: &PackWidgets) -> Result<PackProbe, RegistryError>` (unchanged shape: `PackProbe { files, pack: Option<PackWidgets>, registry, unprobed }`, the registry being base + that pack); `RenderWorker::registry_of(&self, set: &PackSet) -> Result<Arc<WidgetRegistry>, RegistryError>` (assembled registry of a set of already probed packs, memoized by set fingerprint); `RenderWorker::is_faulty(&PackWidgets)` unchanged; slot API `set_widgets(&PackSet, Arc<WidgetRegistry>)`.

- [ ] **Step 1: Write the failing tests** (in the crate's existing test files for `pack_probe` and `serve`)

```rust
#[test]
fn set_widgets_two_packs_renders_both_namespaces() {
    let worker = worker();
    let set = PackSet::new(vec![test_pack("acme"), test_pack("beta")]);
    let registry = worker.registry_of(&set).unwrap();
    let slot = worker.open_slot(&fonts()).unwrap();
    slot.set_widgets(&set, registry).unwrap();
    let layout = slot.render(&deck_with(&["acme/note", "beta/note"])).unwrap().layout();
    assert_that!(layout.element_ids()).is_equal_to(vec!["e-acme".to_owned(), "e-beta".to_owned()]);
}

#[test]
fn probe_pack_reports_namespace_qualified_registry() {
    let probe = worker().probe_pack(&test_pack("acme")).unwrap();
    assert_that!(probe.registry.get("acme/note").is_some()).is_true();
}

#[test]
fn wire_pack_round_trip_keeps_namespace_and_fingerprint() {
    let p = test_pack("acme");
    let back = pack_of(wire(&p)).unwrap();
    assert_that!((back.namespace().clone(), back.fingerprint())).is_equal_to((p.namespace().clone(), p.fingerprint()));
}

#[test]
fn hello_with_protocol_3_is_refused() { /* same as the existing protocol-mismatch test, with 3 */ }
```

`test_pack(ns)` in `test_packs.rs` builds a one-widget pack `note` under `ns`; `deck_with` builds a free slide with one element per kind (ids `e-<ns>`). Use the existing worker test harness for `worker()`, `open_slot`, `render`; adapt names to the real ones in the crate.

- [ ] **Step 2: Run, expect FAIL** — `cargo nextest run -p oku-slides-render-worker`.

- [ ] **Step 3: Implement** — bump `PROTOCOL_VERSION`; `WirePack { id, namespace: String, files }`; `pack_of` parses the namespace (`Namespace::parse`, else `WireError` like a bad file name); `link::fingerprint_hex` hashes exactly as `PackWidgets::fingerprint`; `serve.rs` handles `SetWidgets { slot, packs }` by building a `PackSet` and calling `Registries::get(&set)`; `slot.rs` stores the set; `pack_probe.rs` keeps the per-pack probe and adds `registry_of` with the `kept_registry` LRU keyed by `PackSet::fingerprint()` (`MAX_ASSEMBLED` unchanged, `Arc::ptr_eq` stability kept). Update `ARCHITECTURE.md` of the crate in Task 10.

- [ ] **Step 4: Run, expect PASS** — `cargo nextest run -p oku-slides-render -p oku-slides-render-worker`.

- [ ] **Step 5: Commit** — `feat(render-worker)! : protocole 4, un ensemble de packs par emplacement (t-1014)`.

---

### Task 6: Session — library namespaces, default packs, closure, per-pack widgets and fallback

**Files:**
- Modify: `crates/oku-slides-session/src/library_kind.rs:60-78` (`DIR = "widgets"`, `MANIFEST_FILE = "widgets.json"`, builtins), `src/library.rs:361-440` (namespace uniqueness), `src/pack.rs` (`PackInfo.namespace`, `requires`; `DEFAULT_PACK` doc), `src/deck_pack.rs` (whole file), `src/deck_widgets.rs` (whole file), `src/lib.rs:221-226,261-263` (fields), `src/error.rs` (new variants), `src/probed_widgets.rs:95-172`, their tests (`deck_pack_tests.rs`, `deck_widgets_tests.rs`, `deck_widgets_fallback_tests.rs`, `library_tests.rs`, `library_cache_tests.rs`, `library_lookup_tests.rs`), `crates/oku-slides-platform/src/config.default.toml` + `config.rs` if `paths.templates` names the library dir (rename to `paths.widgets`)
- Create: `src/deck_closure.rs`, `src/deck_closure_tests.rs`, `tests/regressions/issue_NNNN_pack_fault_isolated.rs`
- Modify i18n: `*/session.ftl` (`session-namespace-taken`, `session-packs-disabled`; remove `session-pack-missing` if replaced)

**Interfaces:**
- Consumes: `PackPin`, `Op::AddPack`, `Deck.widgets` (Task 2); `PackSet`, `PackWidgets::new(id, ns, files)` (Task 4); `probe_pack`, `registry_of`, `is_faulty` (Task 5).
- Produces:
  - `pub fn adopt_default_packs(library: &PackLibrary, sources: &mut SourceBundle) -> Result<()>` (replaces `adopt_default_pack`; a deck with no pin gets `essentiel` and its closure).
  - `pub(crate) fn closure_ops(library: &PackLibrary, deck: &Deck, wanted: &[Op]) -> Result<Vec<Op>>` in `deck_closure.rs`: for each `AddPack` in `wanted`, prepend `AddPack` of each missing required namespace (current library version, recursively, dependencies first, each namespace once); unknown namespace in the library → `UnknownNamespace` session error.
  - `pub(crate) struct DeckWidgets { pins: BTreeMap<Namespace, PackPin>, set: PackSet, registry: Arc<WidgetRegistry>, unprobed: BTreeMap<Namespace, BTreeSet<WidgetFile>>, disabled: BTreeMap<Namespace, String> }`.
  - `Session::resolve_widgets(&self, pins: &BTreeMap<Namespace, PackPin>, packs: &BTreeMap<PackVersion, PackFiles>) -> Result<(DeckWidgets, Option<String>)>`.
  - Session fields `widgets_pins: BTreeMap<Namespace, PackPin>` (was `widgets_templates`), `packs: PackSet` (was `pack`), `pack_notices_closed: HashSet<PackPin>`.
  - `PackLibrary::add_version` refuses a manifest whose namespace belongs to another pack id (`NamespaceTaken { namespace, owner }`), except deck-scoped packs, which never enter the library.

- [ ] **Step 1: Write the failing tests**

`deck_closure_tests.rs`:

```rust
#[test]
fn closure_ops_prepends_missing_requirement() {
    let lib = library_with(&[("base", &[]), ("acme", &["base"])]);
    let ops = closure_ops(&lib, &Deck::empty(), &[add_current(&lib, "acme")]).unwrap();
    assert_that!(namespaces_added(&ops)).is_equal_to(vec!["base".to_owned(), "acme".to_owned()]);
}

#[test]
fn closure_ops_skips_requirement_already_pinned() {
    let lib = library_with(&[("base", &[]), ("acme", &["base"])]);
    let deck = deck_pinning(&lib, &["base"]);
    let ops = closure_ops(&lib, &deck, &[add_current(&lib, "acme")]).unwrap();
    assert_that!(namespaces_added(&ops)).is_equal_to(vec!["acme".to_owned()]);
}

#[test]
fn closure_ops_diamond_adds_shared_requirement_once() {
    let lib = library_with(&[("base", &[]), ("left", &["base"]), ("right", &["base"]), ("top", &["left", "right"])]);
    let ops = closure_ops(&lib, &Deck::empty(), &[add_current(&lib, "top")]).unwrap();
    assert_that!(namespaces_added(&ops)).is_equal_to(vec!["base".into(), "left".into(), "right".into(), "top".into()]);
}

#[test]
fn closure_ops_unknown_requirement_is_refused() {
    let lib = library_with(&[("acme", &["ghost"])]);
    let err = closure_ops(&lib, &Deck::empty(), &[add_current(&lib, "acme")]).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ErrorKind::UnknownNamespace);
}
```

(`library_with`, `add_current`, `deck_pinning`, `namespaces_added`: helpers in `test_support.rs`, built on the existing `add_version` / `manifest` helpers, which gain `namespace` and `requires` arguments.)

`library_tests.rs`:

```rust
#[test]
fn library_add_version_namespace_of_another_pack_is_refused() {
    let lib = lib();
    add_version(&lib, &manifest_ns(Uuid::new_v4(), "acme", &[]), &[]);
    let err = lib.add_version(Uuid::new_v4(), &files_of(&manifest_ns(Uuid::new_v4(), "acme", &[]))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ErrorKind::NamespaceTaken);
}

#[test]
fn library_builtins_have_their_namespaces() {
    let names: Vec<_> = builtin_versions().iter().map(|b| b.manifest.namespace.to_string()).collect();
    assert_that!(names).is_equal_to(vec!["essentiel".to_owned(), "grilles".to_owned()]);
}
```

`deck_pack_tests.rs`:

```rust
#[test]
fn adopt_default_packs_empty_deck_pins_essentiel() {
    let (lib, mut sources) = (library(), blank_sources());
    adopt_default_packs(&lib, &mut sources).unwrap();
    assert_that!(sources.deck().widgets.keys().map(ToString::to_string).collect::<Vec<_>>()).is_equal_to(vec!["essentiel".to_owned()]);
}

#[test]
fn adopt_default_packs_deck_with_a_pin_is_unchanged() { /* pin acme, adopt, widgets == {acme} */ }
```

`deck_widgets_fallback_tests.rs` — rewrite the base-fallback tests as per-pack tests:

```rust
#[test]
fn faulty_pack_is_disabled_alone() {
    let mut s = session_with_packs(&[("good", GOOD_NOTE), ("bad", LOOPING_NOTE)]);
    s.render_deck().unwrap();
    assert_that!((s.registry().get("good/note").is_some(), s.registry().get("bad/note").is_some())).is_equal_to((true, false));
}

#[test]
fn pack_requiring_a_faulty_pack_is_disabled_too() { /* "top" requires "bad": both disabled, "good" kept */ }

#[test]
fn disabled_packs_notice_names_each_pack() { /* notice text (key session-packs-disabled) lists bad and top */ }
```

Use the existing faulty-pack harness of `deck_widgets_fallback_tests.rs` (the looping widget that trips the watchdog) for `LOOPING_NOTE`. Add the regression `tests/regressions/issue_NNNN_pack_fault_isolated.rs` (number = this sub-task's id) reproducing "a faulty pack hides every other pack's widgets", declared in `tests/regressions/main.rs`.

- [ ] **Step 2: Run, expect FAIL** — `cargo nextest run -p oku-slides-session deck_closure library deck_pack deck_widgets`.

- [ ] **Step 3: Implement**
  - `library_kind.rs`: `DIR = "widgets"`, `MANIFEST_FILE = "widgets.json"`; builtins keep `include_str!` of `templates/essentiel/pack.json` and `templates/grilles/pack.json` (the repo files keep their path until lot 3 deletes templates; only the library and the zip use `widgets.json`). Old user libraries (`templates/` dir) are not read: no migration.
  - `library.rs` `add`: before writing, if `scope.is_library()` and another id's current manifest has the same namespace → `NamespaceTaken`. Add `PackLibrary::by_namespace(&self, ns: &Namespace) -> Option<StoredVersion<PackManifest>>` (current version of the pack owning `ns`), used by `closure_ops`.
  - `deck_closure.rs`: depth-first post-order over `requires` from the library (`by_namespace`), a visited set, and a recursion guard: a cycle in `requires` → `RequiresCycle { chain }` session error (Fluent key `session-requires-cycle`), test included (`closure_ops_cycle_is_refused`).
  - `deck_pack.rs`: `adopt_default_packs` embeds the current `essentiel` version and its closure (`closure_ops` + `sources.add_pack` for each, then `apply_silent` of the deck with the pins); `check_pack_id(ns, pin, files)` checks id, namespace and requires against the manifest; `embed_from_library(&mut self, pin)`; `deck_pins(&self) -> &BTreeMap<Namespace, PackPin>` replaces `deck_pack()`; `library_pack_name` unchanged; `pack_name()` becomes `pack_names()` (names of the installed set).
  - `deck_widgets.rs` `apply`:
    1. `let ops = closure_ops(&self.library, self.user_deck(), &ops)?.into_iter().chain(ops).collect()` only when `ops` holds an `AddPack`;
    2. every `AddPack` pin is embedded (`embed_from_library`);
    3. if the resulting pins differ from `widgets_pins`, `resolve_widgets(&new_pins, &doc.sources.packs)` before applying, as today for `SetTemplates`; restore on commit failure (unchanged logic).
  - `resolve_widgets`: for each pin, missing version → disabled (`session-pack-missing` reason); otherwise `PackWidgets::new(pin.version.hex(), ns.clone(), files.widgets().clone())`, `probe_pack`, `is_faulty` → disabled; then disable transitively every pin whose `requires` hits a disabled namespace; build `PackSet` of the usable (reduced) packs and `registry_of(&set)`. Notice: one `session-packs-disabled` message listing the disabled pack names, plus the existing broken-widgets notice per pack.
  - `fall_back_to_base` → `disable_pack(&mut self, ns, reason)`: re-resolve without that pack (and dependents), install, keep `deck.widgets` unchanged. `fall_back_or_log`, `with_fallback`, `probe_remaining`, `usable_pack`, `sync_widgets` follow (compare `deck().widgets` with `widgets_pins`).
  - `probed_widgets.rs`: `ProbedWidgets::probe` builds the set from `deck.widgets`; `sources_with_pack` → `sources_with_packs` calling `adopt_default_packs`.
  - Fluent (`en/session.ftl`):

```ftl
session-namespace-taken = Another pack already uses the name { $namespace } ({ $owner }).
session-unknown-namespace = No pack named { $namespace } in the library.
session-requires-cycle = These packs require each other in a loop: { $chain }.
session-packs-disabled = Some widget packs are turned off because they failed: { $packs }.
```

  and the 19 other locales; remove `render-pack-disabled` usages replaced by `session-packs-disabled` (keep the key only if still used elsewhere).

- [ ] **Step 4: Run, expect PASS** — `cargo nextest run -p oku-slides-session deck_closure library deck_pack deck_widgets` and the new regression. Other session modules are fixed in Task 7.

- [ ] **Step 5: Commit** — `feat(session)! : packs de widgets épinglés, fermeture des besoins et repli par pack (t-1015)`.

---

### Task 7: Session — atelier, deck-mode writes, notices, history, import, agent, snapshot

**Files:**
- Modify: `src/widgets_edit.rs:150-440`, `src/widgets_deck.rs:45-170`, `src/templates_mode.rs` (all `deck.templates` sites: :75,:112,:284,:295,:332,:354,:392), `src/templates_library.rs:28-76`, `src/pack_notice.rs` (whole file), `src/history.rs:99,136-150`, `src/merge.rs:93`, `src/undo.rs:34`, `src/resume.rs:662`, `src/help.rs:66-72`, `src/deck_file.rs:64-112`, `src/views.rs:51-101,271`, `src/lib.rs:322,359,406,440-460,641`, `src/request.rs` (`SetWidget` gains `pack: Option<Namespace>`), `src/import/{child_pack.rs,mod.rs,retrofit.rs,hifi.rs,queue.rs}`, `src/agent/{context.rs,conversation.rs}`, `src/widget_preview.rs:134`, `src/deck_theme_edit.rs:71`, all session tests listed in the map (`rg -n 'templates|SetTemplates|PackRef' crates/oku-slides-session`)
- Create: `tests/regressions/issue_NNNN_deck_widget_goes_to_deck_pack.rs`

**Interfaces:**
- Consumes: everything from Task 6.
- Produces:
  - `WidgetTarget { namespace: Namespace, id: Uuid, version: PackVersion, files: PackFiles }`; `Session::widget_target(&self, pack: Option<&Namespace>) -> Result<WidgetTarget>`: atelier (templates mode) → the edited set; `Some(ns)` → that pin; `None` → the `deck` pin, or a new deck-scoped pack `{ namespace: deck, scope: Deck, id: Uuid::new_v4(), name: "<deck name>" }` created on first write.
  - `Session::set_widget(&mut self, pack: Option<&Namespace>, name: &str, source: String) -> Result<WidgetChange>` (and `set_widget_by_agent`, `remove_widget`, `rename_widget` take the same `pack` argument).
  - `ensure_unused(ns, name)` matches `qualify(ns, name)`; `add_widgets` and `pack_widget_names` return qualified kinds.
  - `PackNotice { namespace: Namespace, pin: PackPin, name, relation }`; one notice at a time, for the first namespace (sorted) with a newer library version; `pack_use_mine` applies `Op::AddPack`.
  - `Snapshot.widgets: &BTreeMap<Namespace, PackPin>` (was `templates`), `Snapshot.pack_widgets: Vec<String>` qualified.
  - Retrofit child pack inherits the target's namespace and replaces its pin; hifi pins its generated pack under `deck`.
  - Agent context line `widget_packs: <ns>=<id>, …` (was `template_set`); `widgets_pack()` → the pack id of `AgentScope`'s target or the `deck` pin.

- [ ] **Step 1: Write the failing tests** (in the existing sibling test files)

```rust
#[test]
fn set_widget_without_pack_creates_the_deck_pack() {
    let mut s = session();
    s.set_widget(None, "note", NOTE_SRC.into()).unwrap();
    let pin = s.deck().widgets.get(&Namespace::deck()).cloned();
    assert_that!(pin.is_some() && s.registry().get("deck/note").is_some()).is_true();
}

#[test]
fn set_widget_into_named_pack_makes_a_new_version_of_it() {
    let mut s = session_with_packs(&[("acme", NOTE_SRC)]);
    let before = s.deck().widgets[&ns("acme")].version.clone();
    s.set_widget(Some(&ns("acme")), "other", NOTE_SRC.into()).unwrap();
    assert_that!(s.deck().widgets[&ns("acme")].version).is_not_equal_to(before);
}

#[test]
fn remove_widget_in_use_names_the_qualified_kind() { /* element acme/note → WidgetInUse */ }

#[test]
fn pack_notice_for_second_namespace_is_shown_when_first_is_current() { /* essentiel current, acme newer → notice for acme */ }

#[test]
fn pack_use_mine_pins_the_library_version() { /* AddPack applied, pin version == library current */ }

#[test]
fn undo_of_add_pack_restores_previous_widgets() { /* add acme, undo → registry has no acme/* */ }

#[test]
fn retrofit_child_pack_keeps_target_namespace() { /* import retrofit on essentiel, write widget → pin essentiel is deck-scoped child */ }

#[test]
fn hifi_import_pins_generated_pack_under_deck() { /* deck().widgets has key "deck" */ }

#[test]
fn snapshot_lists_widget_pins_by_namespace() { /* snapshot JSON has "widgets": {"essentiel": …} */ }
```

Write each body with the existing helpers of the file it lands in (`session()`, `session_with_packs`, the import fixtures `fixtures/import/*.pptx` used by `import/mod_tests.rs`). Regression: "a widget written in deck mode overwrote the deck's built-in pack" → `issue_NNNN_deck_widget_goes_to_deck_pack.rs`.

- [ ] **Step 2: Run, expect FAIL** — `cargo nextest run -p oku-slides-session`.

- [ ] **Step 3: Implement** — apply the interfaces above at every site of the map:
  - replace each `deck.templates = Some(r)` in `templates_mode.rs` by inserting the edited pin (`deck.widgets.insert(ns, pin)`), and each read by the edited namespace's pin;
  - `widgets_deck.rs::adopt_deck_widgets` applies `Op::AddPack { namespace: t.namespace, pin }` with the same merge key; `write_deck_widgets` only for library scope (deck-scoped `deck` packs never enter the library — existing rule);
  - `help.rs` pins the built-in `essentiel` (`PackPin` from `builtin_versions()`);
  - `import/queue.rs` fingerprint hashes every pin's templates, examples and widgets (sorted by namespace);
  - `request.rs`: `SetWidget { pack: Option<Namespace>, name, source }` and the same `pack` field on `RemoveWidget`, `RenameWidget`; `#[serde(default)]` on `pack`;
  - `views.rs`/`lib.rs` snapshot: `widgets` instead of `templates`.

- [ ] **Step 4: Run, expect PASS** — `cargo nextest run -p oku-slides-session` (whole crate).

- [ ] **Step 5: Commit** — `feat(session)! : écriture de widgets par espace de noms, pack du deck, avis par pack (t-1016)`.

---

### Task 8: Import crate, CLI and MCP — workspace green

**Files:**
- Modify: `crates/oku-slides-import/src/hifi/mod.rs:11,156`, `src/plan.rs:83`, `tests/hifi.rs`, snapshots (`cargo insta review`: only `set_templates` → `add_pack` lines may change)
- Modify: `crates/oku-slides-cli/src/main.rs:85,147,189,220-227,318-369,418-420`, `src/import.rs:78-128`, `src/library.rs:17-39` (`deck_pack_version` → `deck_pack_versions` over pins), `src/library_tests.rs`, `tests/{cli.rs,import.rs,pack_broken.rs}`, `tests/regressions/issue_0355*.rs`, `crates/oku-slides-i18n/src/locales/*/cli.ftl` (`cli-arg-pack` text: "id, name or namespace")
- Modify: `crates/oku-slides-mcp/src/{read_tools.rs:41-60,tools.rs:231-276,widget_tools.rs:60-118,handler.rs}`, `tests/import.rs:309-335`, `tests/surfaces.rs`

**Interfaces:**
- Consumes: `Op::AddPack`, `PackPin`, `Namespace` (Tasks 1–2); session APIs (Tasks 6–7).
- Produces: CLI `open --out` writes `widgets/<hex>/widgets.json` beside the deck; `--pack` resolves an id, a name or a namespace; MCP state outputs `"widget_packs": {ns: {id, version}}`; MCP `set_widget`, `remove_widget`, `widget_sources` take an optional `pack` (namespace) argument; tool descriptions name `add_pack` / `remove_pack` ops and qualified kinds.

- [ ] **Step 1: Write the failing tests**

```rust
// crates/oku-slides-cli/tests/cli.rs
#[test]
fn open_out_writes_each_pinned_pack_under_widgets_dir() {
    let out = run_open_out(&deck_with_pins(&["essentiel", "grilles"]));
    assert_that!(dirs_under(&out, "widgets")).has_length(2);
}

#[test]
fn import_pack_by_namespace_is_accepted() {
    let deck = run_import_retrofit(&["--pack", "grilles"]);
    assert_that!(deck.widgets.contains_key(&ns("grilles"))).is_true();
}

// crates/oku-slides-mcp/tests/surfaces.rs (or read tools test file)
#[test]
fn state_lists_widget_packs_by_namespace() {
    let state = call_tool(&hub(), "state", json!({}));
    assert_that!(state["widget_packs"]["essentiel"]["id"].is_string()).is_true();
}

// crates/oku-slides-import/tests/hifi.rs
#[test]
fn hifi_plan_pins_its_pack_under_deck_namespace() {
    let ops = hifi_ops(MINIMAL);
    assert_that!(ops.iter().any(|o| matches!(o, Op::AddPack { namespace, .. } if namespace.is_deck()))).is_true();
}
```

(Use each file's existing harness functions; rename where needed.)

- [ ] **Step 2: Run, expect FAIL** — `cargo nextest run -p oku-slides-import -p oku-slides-cli -p oku-slides-mcp`.

- [ ] **Step 3: Implement** the items under Files/Produces. Review import snapshot diffs with `cargo insta review`: accept only `set_templates` → `add_pack` changes; anything else is a bug.

- [ ] **Step 4: Run, expect PASS** — `just test` (Rust part green across the workspace; UI may still fail on `api.ts`, fixed next).

- [ ] **Step 5: Commit** — `feat(cli)! : import, CLI et MCP sur les packs de widgets épinglés (t-1017)`.

---

### Task 9: UI — snapshot pins, `add_pack` / `remove_pack`, qualified kinds

**Files:**
- Modify: `ui/src/api.ts:85-95,175,231,544,572-583,785-791` (`Snapshot.widgets: Record<string, PackPin>`, `PackPin { id, version, requires? }`, `PackInfo.namespace`, `requires`, ops `add_pack` / `remove_pack`, widget calls with optional `pack`), `ui/src/packOps.ts` (+ test), `ui/src/useTemplatesMode.ts:53-85`, `ui/src/App.tsx:492`, `ui/src/useTemplateWidgets.ts:52`, `ui/src/useHostSettings.ts:53`, `ui/src/useTargetSettings.ts:44`, `ui/src/agent/scope.ts:6-22`, their tests

**Interfaces:**
- Consumes: snapshot and request shapes from Tasks 7–8.
- Produces: `addPackOp(pack: PackInfo): Op` (`{ op: "add_pack", namespace: pack.namespace, pin: { id, version, requires } }`), `pinsKey(pins: Record<string, PackPin>): string` (stable cache key: sorted `ns@version` joined by `,`) used by the three hooks instead of `snap.templates?.version`.

- [ ] **Step 1: Write the failing tests**

```ts
// ui/src/packOps.test.ts
test("addPackOp pins the pack under its namespace", () => {
  const op = addPackOp({ ...packInfo, namespace: "acme", requires: ["base"] });
  expect(op).toEqual({ op: "add_pack", namespace: "acme", pin: { id: packInfo.id, version: packInfo.version, requires: ["base"] } });
});

test("pinsKey is independent of key order", () => {
  expect(pinsKey({ b: pin("2"), a: pin("1") })).toBe(pinsKey({ a: pin("1"), b: pin("2") }));
});
```

- [ ] **Step 2: Run, expect FAIL** — `cd ui && bun test packOps`.

- [ ] **Step 3: Implement** — update the types and call sites listed; `useTemplatesMode.adoptSet` sends `addPackOp`; `initialSet` = `snap.editing?.set ?? Object.values(snap.widgets)[0]?.id`; `ScopeView` reads the pack id from the edited set or the `deck` pin. `ADD_KINDS` / `newElement.ts` keep the bare core kinds (lot 2 removes them).

- [ ] **Step 4: Run, expect PASS** — `just test` (Rust + UI, including `ops::schema_tests`, now in sync with `api.ts`) and `cd ui && bunx tsc --noEmit`.

- [ ] **Step 5: Commit** — `feat(ui)! : instantané des packs épinglés et op add_pack (t-1018)`.

---

### Task 10: Docs, rules, full gate

**Files:**
- Modify: `crates/oku-slides-core/ARCHITECTURE.md` (model, `namespace.rs`, `ops/packs.rs`, zip `widgets/`), `crates/oku-slides-render/ARCHITECTURE.md` (`PackSet`, assembly prefix, no base override), `crates/oku-slides-render-worker/ARCHITECTURE.md` (protocol 4), `crates/oku-slides-session/ARCHITECTURE.md` (pins, closure, per-pack fallback, `deck` pack), each ≤ 50 lines
- Modify rules (with sources `(plan:2026-10-18-packs-lot1-widget-packs)`):
  - `crates/oku-slides-core/AGENTS.md`: replace "Treat deck templates as one atomic optional PackRef; never patch it field by field. (t-347)" by "Keep deck.widgets a map of whole PackPins by namespace; never patch a pin field by field. (plan:2026-10-18-packs-lot1-widget-packs)"; delete "Allow a pack to override a base widget only with compatible param types, binds and category."; add "Build a qualified kind only with qualify and read it only with split_kind; a bare kind is a core widget. (plan:2026-10-18-packs-lot1-widget-packs)".
  - `crates/oku-slides-session/AGENTS.md`: add "Add a pack's missing requirements through closure_ops in the same transaction, dependencies first. (plan:2026-10-18-packs-lot1-widget-packs)" and "Disable a failing widget pack alone with its dependents; never fall back to core widgets for the whole deck. (plan:2026-10-18-packs-lot1-widget-packs)".
- Modify: `docs/aide/{en,fr}.json` where they mention `set_templates`, the deck's pack, or `packs/` in the sources zip (keep the s-format-loro pitfall strings intact).

- [ ] **Step 1:** Write the docs and rules above.
- [ ] **Step 2:** `just ci` → green (fmt, clippy, tsc, fonts-lfs, agents-lint, tests, audit).
- [ ] **Step 3:** `just e2e 4999` (pick a free port) → green; fix any spec reading the old snapshot shape (`templates`) with the smallest change, under `ui/e2e/`.
- [ ] **Step 4:** Consolidation pass (G10): `rg -n 'templates:? *(Option<PackRef>|None)|SetTemplates|set_templates|PackRef\b|base_registry\(|check_override' crates ui/src src-tauri` → only lot-3 template code (`PackManifest.templates`, templates mode) may remain; list leftovers in the report.
- [ ] **Step 5: Commit** — `docs(packs) : architecture et règles des packs de widgets épinglés (t-1019)`.
