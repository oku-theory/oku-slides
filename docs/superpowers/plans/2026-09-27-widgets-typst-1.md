# Widgets Typst, plan 1 (additif) : système de widgets et registre

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Déclarer tous les composants et toutes les slides comme widgets Typst avec un schéma de paramètres, lire ce schéma côté Rust, et faire rendre le deck v1 actuel par ces widgets, sans changer un pixel.

**Architecture:** Typst gagne `p` (descripteurs), `widget()`, `field()` et un registre de base (`src/widgets.typ`). L'ancien dispatcher de `data.typ` devient un adaptateur v1 → arguments de widget. Le cœur reçoit `WidgetRegistry` (données pures, sans Typst). Le rendu compile une sonde pour lire le registre, avec le `widgets.typ` optionnel d'un pack, et le met en cache. Rien ne change pour le modèle, les ops, Loro, la session ni l'UI : c'est le plan 2.

**Tech Stack:** Rust (snafu, serde, speculoos, rstest, insta), Typst 0.15 embarqué, Fluent (20 langues).

**Spec:** `docs/superpowers/specs/2026-09-27-widgets-typst-design.md` (t-270)

## Global Constraints

- Branche et worktree : `270-feature/widgets-typst`. `main` reçoit du travail en parallèle : ne jamais travailler dedans. Dans le worktree : `git config oku.review deferred`.
- `git add` dans une commande à part, puis `git commit` : un hook bloqué annule toute la commande.
- Chaque commit cite `(t-270)`. Pas de ligne `Co-Authored-By`.
- Le rendu du deck de référence reste **identique au pixel près** à chaque tâche (test doré de la tâche 2).
- Pas de `.unwrap()` ni de `.expect()` hors tests. Erreurs Snafu opaques, avec backtrace, messages en minuscules.
- Tout texte destiné à l'utilisateur passe par Fluent, dans les 20 langues (`crates/oku-slides-i18n/src/locales/*`). `every_locale_has_exactly_the_reference_keys` doit rester vert.
- Tests nommés `<unité>_<scénario>_<résultat>`, fichiers `*_tests.rs` à côté du code, speculoos.
- Vocabulaire : un **champ** (`field`) est une région de texte éditable liée à un paramètre. La **zone** reste la zone de corps des éléments libres (`<oku-slides-zone>`). Un **slot** est un paramètre qui contient des éléments.
- Commandes : `cargo nextest run -p <crate>`, `cargo clippy -p <crate> --all-targets -- -D warnings`, `cargo fmt --all`.

## Décisions de ce plan (précisions de la spec)

- **Descripteurs** : `src/params.typ` est importé comme module (`#import "params.typ" as p`). `p.plain(..)` est un appel de fonction de module, là où un dictionnaire de fonctions ne s'appellerait pas avec `p.plain(..)`.
- **Champs** : dans ce plan, seuls les widgets d'élément émettent des `field()`. Les slides gardent leurs `tracked("@title")` actuels : l'app s'en sert encore. Le plan 2 les remplace.
- **`ctx`** passé à `render(args, ctx)` : `(id, tokens, vars, style, framed, load)`. `style` est le style de l'élément (attribut commun, pas un paramètre). `framed` dit si l'élément a un cadre. `load` vaut `none` ou une fonction sans argument qui appelle le chargeur d'image ou de graphique de l'hôte.
- **Entrées de slot** : slot `flow`, `(kind, id, body)` ; slot `free`, `(kind, id, frame, cell, show)`, où `show(frame)` rend l'enfant. `free-area()` (`src/zone.typ`) place les enfants d'un slot `free` de slide ; `canvas` les place sur leur `frame`.
- **Libellés** : les widgets de base ne déclarent pas de `label`. Le plan 2 les localisera par clé Fluent. `label` sert aux packs.
- **Cache** : `Registries` range les registres en mémoire par empreinte sha-256 de `id` et de la source du pack. La bibliothèque est compilée dans le binaire, donc constante pour un processus.

## Structure des fichiers

| Fichier | Rôle |
|---|---|
| `crates/oku-slides-core/src/widget.rs` (créé) | `WidgetRegistry`, `WidgetSchema`, `ParamSchema`, `ParamType`, contrôles et surcharge |
| `crates/oku-slides-core/src/widget_tests.rs` (créé) | tests du registre |
| `crates/oku-slides-core/src/error.rs` | `WidgetError`, `WidgetErrorKind`, `SchemaFault` |
| `crates/oku-slides-core/src/lib.rs` | exports |
| `typst/oku-slides/src/content.typ` (créé) | `_text-args`, `_text-over`, `_runs`, `_paragraphs`, déplacés de `data.typ` |
| `typst/oku-slides/src/zone.typ` (créé) | `grid-frame`, `default-grid`, `zone-marker`, `free-area` |
| `typst/oku-slides/src/params.typ` (créé) | descripteurs `p.*` |
| `typst/oku-slides/src/widget.typ` (créé) | `widget()`, `field()`, `by-name()` |
| `typst/oku-slides/src/widgets.typ` (créé) | widgets de base et `registry` |
| `typst/oku-slides/src/core.typ` | `_measured()`, partagé par `tracked()` et `field()` |
| `typst/oku-slides/src/slides.typ` | `card(wrap:)` |
| `typst/oku-slides/src/data.typ` | adaptateur v1 → widgets |
| `typst/oku-slides/lib.typ` | exports pour les packs |
| `crates/oku-slides-render/src/world.rs` | fichiers embarqués, texte principal libre, source de pack |
| `crates/oku-slides-render/src/registry.rs` (créé) | `PackWidgets`, `load_registry`, `Registries` |
| `crates/oku-slides-render/src/error.rs` | `RegistryError`, `RegistryErrorKind` |
| `crates/oku-slides-render/src/lib.rs` | `FieldBox`, `Layout::fields`, exports |
| `crates/oku-slides-render/tests/golden.rs` (créé) | rendu figé |
| `crates/oku-slides-render/tests/golden/widgets.json` (créé) | deck de référence |
| `crates/oku-slides-render/tests/widgets.rs` (créé) | registre, champs, packs |

---

### Task 1: Registre des widgets dans le cœur

**Files:**
- Create: `crates/oku-slides-core/src/widget.rs`
- Create: `crates/oku-slides-core/src/widget_tests.rs`
- Modify: `crates/oku-slides-core/src/error.rs` (après `ReplicaError`)
- Modify: `crates/oku-slides-core/src/lib.rs`
- Modify: `crates/oku-slides-i18n/src/locales/*/core.ftl` (20 langues)

**Interfaces:**
- Consumes: rien.
- Produces:
  - `oku_slides_core::WidgetRegistry` : `from_probe(serde_json::Value) -> Result<WidgetRegistry, WidgetError>`, `get(&str) -> Option<&WidgetSchema>`, `iter() -> impl Iterator<Item = &WidgetSchema>`, `len() -> usize`, `Serialize`.
  - `WidgetSchema { pub name: String, pub label: Option<String>, pub category: Category, pub params: Vec<ParamSchema> }`, avec `param(&str) -> Option<&ParamSchema>`.
  - `ParamSchema { pub name: String, pub label: Option<String>, pub ty: ParamType, pub default: Option<Value>, pub bind: Option<String> }`.
  - `ParamType::{Plain, Rich, Flag, Number { min, max }, Choice { values }, Asset, Slot { mode, accepts }, Grid, Host { host }}`, `Category::{Slide, Element}`, `SlotMode::{Flow, Free}`, `HostType::Chart`.
  - `WidgetError` (opaque), `WidgetErrorKind::{Unreadable, BadSchema(SchemaFault), IncompatibleOverride, IncompatibleCategory}`, `SchemaFault::{EmptyName, DuplicateWidget, DuplicateParam, EmptyChoice, DefaultNotInChoice, BindNotPlain, UnknownSlotKind}`, `Localize`.
  - Format de la sonde : `{"base": [WidgetSchema…], "pack": [WidgetSchema…] | null}`. Un paramètre est un objet plat `{"name", "type", "label", "default", "bind", …champs du type}`.

- [ ] **Step 1: Write the failing tests**

`crates/oku-slides-core/src/widget_tests.rs` :

```rust
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::{Category, ParamType, SchemaFault, SlotMode, WidgetErrorKind, WidgetRegistry};

fn widget(name: &str, category: &str, params: Value) -> Value {
    json!({ "name": name, "label": null, "category": category, "params": params })
}

fn card() -> Value {
    widget("card", "element", json!([
        { "name": "kicker", "type": "plain", "label": null, "default": null, "bind": null },
        { "name": "title", "type": "plain", "label": null, "default": null, "bind": null },
        { "name": "body", "type": "rich", "label": null, "default": null },
        { "name": "highlight", "type": "flag", "label": null, "default": false },
    ]))
}

fn probe(base: Vec<Value>, pack: Option<Vec<Value>>) -> Value {
    json!({ "base": base, "pack": pack })
}

#[test]
fn from_probe_base_keeps_params_in_declaration_order() {
    let r = WidgetRegistry::from_probe(probe(vec![card()], None)).unwrap();
    let names: Vec<&str> = r.get("card").unwrap().params.iter().map(|p| p.name.as_str()).collect();
    assert_that!(names).is_equal_to(vec!["kicker", "title", "body", "highlight"]);
}

#[test]
fn from_probe_reads_every_param_type() {
    let all = widget("all", "slide", json!([
        { "name": "a", "type": "plain", "default": "x", "bind": "title" },
        { "name": "b", "type": "rich" },
        { "name": "c", "type": "flag", "default": true },
        { "name": "d", "type": "number", "min": 1, "max": null, "default": null },
        { "name": "e", "type": "choice", "values": ["cover", "contain"], "default": "cover" },
        { "name": "f", "type": "asset" },
        { "name": "g", "type": "slot", "mode": "free", "accepts": ["card"] },
        { "name": "h", "type": "grid", "default": { "columns": 12, "rows": 6 } },
        { "name": "i", "type": "host", "host": "chart" },
    ]));
    let r = WidgetRegistry::from_probe(probe(vec![card(), all], None)).unwrap();
    let w = r.get("all").unwrap();
    assert_that!(w.category).is_equal_to(Category::Slide);
    let ty = |n: &str| w.param(n).unwrap().ty.clone();
    assert_that!(ty("a")).is_equal_to(ParamType::Plain);
    assert_that!(w.param("a").unwrap().bind.as_deref()).is_equal_to(Some("title"));
    assert_that!(ty("d")).is_equal_to(ParamType::Number { min: Some(1.0), max: None });
    assert_that!(ty("e")).is_equal_to(ParamType::Choice { values: vec!["cover".into(), "contain".into()] });
    assert_that!(ty("g")).is_equal_to(ParamType::Slot { mode: SlotMode::Free, accepts: Some(vec!["card".into()]) });
    assert_that!(ty("i")).is_equal_to(ParamType::Host { host: crate::HostType::Chart });
}

#[test]
fn from_probe_unreadable_json_returns_unreadable() {
    let err = WidgetRegistry::from_probe(json!({ "base": 3 })).unwrap_err();
    assert_that!(err.kind()).is_equal_to(WidgetErrorKind::Unreadable);
}

#[rstest]
#[case::empty_name(widget("", "element", json!([])), SchemaFault::EmptyName)]
#[case::duplicate_param(
    widget("w", "element", json!([{ "name": "a", "type": "rich" }, { "name": "a", "type": "rich" }])),
    SchemaFault::DuplicateParam
)]
#[case::empty_choice(
    widget("w", "element", json!([{ "name": "a", "type": "choice", "values": [] }])),
    SchemaFault::EmptyChoice
)]
#[case::default_not_in_choice(
    widget("w", "element", json!([{ "name": "a", "type": "choice", "values": ["x"], "default": "y" }])),
    SchemaFault::DefaultNotInChoice
)]
#[case::bind_not_plain(
    widget("w", "element", json!([{ "name": "a", "type": "rich", "bind": "title" }])),
    SchemaFault::BindNotPlain
)]
#[case::unknown_slot_kind(
    widget("w", "element", json!([{ "name": "a", "type": "slot", "mode": "flow", "accepts": ["nope"] }])),
    SchemaFault::UnknownSlotKind
)]
fn from_probe_bad_widget_returns_its_fault(#[case] bad: Value, #[case] fault: SchemaFault) {
    let err = WidgetRegistry::from_probe(probe(vec![card(), bad], None)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(WidgetErrorKind::BadSchema(fault));
}

#[test]
fn from_probe_duplicate_widget_returns_duplicate_widget() {
    let err = WidgetRegistry::from_probe(probe(vec![card(), card()], None)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(WidgetErrorKind::BadSchema(SchemaFault::DuplicateWidget));
}

#[test]
fn from_probe_slot_accepting_a_slide_widget_returns_unknown_slot_kind() {
    let slide = widget("content", "slide", json!([{ "name": "b", "type": "slot", "mode": "flow", "accepts": ["content"] }]));
    let err = WidgetRegistry::from_probe(probe(vec![slide], None)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(WidgetErrorKind::BadSchema(SchemaFault::UnknownSlotKind));
}

#[test]
fn from_probe_pack_adds_a_new_widget() {
    let quote = widget("quote", "element", json!([{ "name": "body", "type": "rich" }]));
    let r = WidgetRegistry::from_probe(probe(vec![card()], Some(vec![quote]))).unwrap();
    assert_that!(r.len()).is_equal_to(2);
}

#[test]
fn from_probe_pack_override_adding_a_param_replaces_the_base_widget() {
    let mut params = card()["params"].as_array().unwrap().clone();
    params.push(json!({ "name": "icon", "type": "asset" }));
    let r = WidgetRegistry::from_probe(probe(vec![card()], Some(vec![widget("card", "element", Value::Array(params))])))
        .unwrap();
    assert_that!(r.get("card").unwrap().param("icon")).is_some();
}

#[rstest]
#[case::dropped(json!([{ "name": "title", "type": "plain" }, { "name": "body", "type": "rich" }, { "name": "highlight", "type": "flag" }]), "kicker")]
#[case::retyped(json!([{ "name": "kicker", "type": "rich" }, { "name": "title", "type": "plain" }, { "name": "body", "type": "rich" }, { "name": "highlight", "type": "flag" }]), "kicker")]
fn from_probe_pack_override_breaking_a_param_returns_incompatible_override(#[case] params: Value, #[case] param: &str) {
    let err = WidgetRegistry::from_probe(probe(vec![card()], Some(vec![widget("card", "element", params)]))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(WidgetErrorKind::IncompatibleOverride);
    assert_that!(err.to_string()).contains(param);
}

#[test]
fn from_probe_pack_override_narrowing_a_choice_returns_incompatible_override() {
    let base = widget("image", "element", json!([{ "name": "fit", "type": "choice", "values": ["cover", "contain"] }]));
    let pack = widget("image", "element", json!([{ "name": "fit", "type": "choice", "values": ["cover"] }]));
    let err = WidgetRegistry::from_probe(probe(vec![base], Some(vec![pack]))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(WidgetErrorKind::IncompatibleOverride);
}

#[test]
fn from_probe_pack_override_changing_category_returns_incompatible_category() {
    let err = WidgetRegistry::from_probe(probe(vec![card()], Some(vec![widget("card", "slide", card()["params"].clone())])))
        .unwrap_err();
    assert_that!(err.kind()).is_equal_to(WidgetErrorKind::IncompatibleCategory);
}

#[test]
fn localize_incompatible_override_names_widget_and_param() {
    use oku_slides_i18n::Localize;
    let pack = widget("card", "element", json!([{ "name": "title", "type": "plain" }]));
    let err = WidgetRegistry::from_probe(probe(vec![card()], Some(vec![pack]))).unwrap_err();
    let text = err.localize(oku_slides_i18n::english());
    assert_that!(text).contains("card");
    assert_that!(text).contains("kicker");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo nextest run -p oku-slides-core widget`
Expected: compilation échoue, `widget_tests` introuvable ou `WidgetRegistry` non défini.

- [ ] **Step 3: Add the errors**

Dans `crates/oku-slides-core/src/error.rs`, à la fin du fichier :

```rust
/// Le registre des widgets est mal déclaré.
#[derive(Debug, Snafu)]
pub struct WidgetError(WidgetInner);

/// Défaut d'une déclaration de widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchemaFault {
    EmptyName,
    DuplicateWidget,
    DuplicateParam,
    EmptyChoice,
    DefaultNotInChoice,
    /// Seul un paramètre `plain` se lie à `deck.params`.
    BindNotPlain,
    /// Un slot accepte un widget absent, ou qui n'est pas un élément.
    UnknownSlotKind,
}

impl SchemaFault {
    fn message_id(self) -> &'static str {
        match self {
            SchemaFault::EmptyName => "widget-fault-empty-name",
            SchemaFault::DuplicateWidget => "widget-fault-duplicate-widget",
            SchemaFault::DuplicateParam => "widget-fault-duplicate-param",
            SchemaFault::EmptyChoice => "widget-fault-empty-choice",
            SchemaFault::DefaultNotInChoice => "widget-fault-default-not-in-choice",
            SchemaFault::BindNotPlain => "widget-fault-bind-not-plain",
            SchemaFault::UnknownSlotKind => "widget-fault-unknown-slot-kind",
        }
    }
}

/// Nature d'une [`WidgetError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum WidgetErrorKind {
    Unreadable,
    BadSchema(SchemaFault),
    /// Un pack remplace un widget de base en retirant ou retypant un paramètre.
    IncompatibleOverride,
    /// Un pack remplace un widget de base par un widget d'une autre catégorie.
    IncompatibleCategory,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum WidgetInner {
    #[snafu(display("unreadable widget registry"))]
    Unreadable { source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("widget {widget}: {fault:?}"))]
    BadSchema { widget: String, fault: SchemaFault, backtrace: Backtrace },
    #[snafu(display("widget {widget} overrides parameter {param} incompatibly"))]
    IncompatibleOverride { widget: String, param: String, backtrace: Backtrace },
    #[snafu(display("widget {widget} overrides the base widget with another category"))]
    IncompatibleCategory { widget: String, backtrace: Backtrace },
}

impl WidgetError {
    pub fn kind(&self) -> WidgetErrorKind {
        match &self.0 {
            WidgetInner::Unreadable { .. } => WidgetErrorKind::Unreadable,
            WidgetInner::BadSchema { fault, .. } => WidgetErrorKind::BadSchema(*fault),
            WidgetInner::IncompatibleOverride { .. } => WidgetErrorKind::IncompatibleOverride,
            WidgetInner::IncompatibleCategory { .. } => WidgetErrorKind::IncompatibleCategory,
        }
    }
}

impl Localize for WidgetError {
    fn localize(&self, l: &Localizer) -> String {
        match &self.0 {
            WidgetInner::Unreadable { .. } => l.tr("widget-unreadable"),
            WidgetInner::BadSchema { widget, fault, .. } => {
                l.tr_with(fault.message_id(), &[("widget", widget.as_str().into())])
            }
            WidgetInner::IncompatibleOverride { widget, param, .. } => l.tr_with(
                "widget-incompatible-override",
                &[("widget", widget.as_str().into()), ("param", param.as_str().into())],
            ),
            WidgetInner::IncompatibleCategory { widget, .. } => {
                l.tr_with("widget-incompatible-category", &[("widget", widget.as_str().into())])
            }
        }
    }
}
```

- [ ] **Step 4: Write the registry**

`crates/oku-slides-core/src/widget.rs` :

```rust
//! Registre des widgets : le schéma des paramètres de chaque composant.
//!
//! Les widgets sont déclarés en Typst (`typst/oku-slides/src/widgets.typ`,
//! et le `widgets.typ` d'un pack). Le rendu compile une sonde qui émet leurs
//! schémas en JSON ; [`WidgetRegistry::from_probe`] les lit, les contrôle et
//! applique les surcharges du pack. Le cœur ne voit que ce JSON.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use snafu::ResultExt;

use crate::error::{
    BadSchemaSnafu, IncompatibleCategorySnafu, IncompatibleOverrideSnafu, SchemaFault, UnreadableSnafu, WidgetError,
};

/// Ce qu'un widget rend : une slide entière, ou un élément de slide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Slide,
    Element,
}

/// Placement des éléments d'un slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SlotMode {
    /// Agencés par le widget (pile, colonnes, grille).
    Flow,
    /// Placés sur leur cadre ou leur cellule de grille.
    Free,
}

/// Types de paramètres fournis et validés par l'hôte Rust.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HostType {
    Chart,
}

/// Type d'un paramètre de widget.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ParamType {
    /// Texte sur une ligne, sans mise en forme.
    Plain,
    /// Paragraphes à runs.
    Rich,
    Flag,
    Number {
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    Choice {
        values: Vec<String>,
    },
    /// Identifiant d'asset `sha256:…`.
    Asset,
    Slot {
        mode: SlotMode,
        /// Widgets d'élément admis ; `None` : tous.
        #[serde(default)]
        accepts: Option<Vec<String>>,
    },
    Grid,
    Host {
        host: HostType,
    },
}

/// Un paramètre déclaré par un widget.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamSchema {
    pub name: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(flatten)]
    pub ty: ParamType,
    #[serde(default)]
    pub default: Option<Value>,
    /// Clé de `deck.params` où la valeur est lue et écrite.
    #[serde(default)]
    pub bind: Option<String>,
}

/// Un widget : son nom, sa catégorie et ses paramètres dans l'ordre déclaré.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WidgetSchema {
    pub name: String,
    #[serde(default)]
    pub label: Option<String>,
    pub category: Category,
    pub params: Vec<ParamSchema>,
}

impl WidgetSchema {
    /// Le paramètre `name`, s'il est déclaré.
    pub fn param(&self, name: &str) -> Option<&ParamSchema> {
        self.params.iter().find(|p| p.name == name)
    }
}

/// Les widgets disponibles : ceux de la base, surchargés par ceux du pack.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WidgetRegistry {
    widgets: BTreeMap<String, WidgetSchema>,
}

#[derive(Deserialize)]
struct Probe {
    base: Vec<WidgetSchema>,
    #[serde(default)]
    pack: Option<Vec<WidgetSchema>>,
}

impl WidgetRegistry {
    /// Lit la sonde `{"base": [..], "pack": [..] | null}` émise par Typst.
    pub fn from_probe(probe: Value) -> Result<Self, WidgetError> {
        let probe: Probe = serde_json::from_value(probe).context(UnreadableSnafu)?;
        let mut widgets = collect(probe.base)?;
        if let Some(pack) = probe.pack {
            for (name, w) in collect(pack)? {
                if let Some(base) = widgets.get(&name) {
                    check_override(base, &w)?;
                }
                widgets.insert(name, w);
            }
        }
        let registry = Self { widgets };
        registry.check_slots()?;
        Ok(registry)
    }

    pub fn get(&self, name: &str) -> Option<&WidgetSchema> {
        self.widgets.get(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = &WidgetSchema> {
        self.widgets.values()
    }

    pub fn len(&self) -> usize {
        self.widgets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.widgets.is_empty()
    }

    /// Chaque slot n'accepte que des widgets d'élément déclarés.
    fn check_slots(&self) -> Result<(), WidgetError> {
        for w in self.widgets.values() {
            for p in &w.params {
                let ParamType::Slot { accepts: Some(kinds), .. } = &p.ty else { continue };
                let known = |k: &String| self.get(k).is_some_and(|x| x.category == Category::Element);
                if !kinds.iter().all(known) {
                    return Err(fault(w, SchemaFault::UnknownSlotKind));
                }
            }
        }
        Ok(())
    }
}

fn fault(w: &WidgetSchema, fault: SchemaFault) -> WidgetError {
    BadSchemaSnafu { widget: w.name.clone(), fault }.build().into()
}

/// Contrôle chaque widget d'une liste et les range par nom.
fn collect(list: Vec<WidgetSchema>) -> Result<BTreeMap<String, WidgetSchema>, WidgetError> {
    let mut out = BTreeMap::new();
    for w in list {
        check_widget(&w)?;
        if out.contains_key(&w.name) {
            return Err(fault(&w, SchemaFault::DuplicateWidget));
        }
        out.insert(w.name.clone(), w);
    }
    Ok(out)
}

fn check_widget(w: &WidgetSchema) -> Result<(), WidgetError> {
    if w.name.is_empty() || w.params.iter().any(|p| p.name.is_empty()) {
        return Err(fault(w, SchemaFault::EmptyName));
    }
    let mut seen = BTreeSet::new();
    for p in &w.params {
        if !seen.insert(p.name.as_str()) {
            return Err(fault(w, SchemaFault::DuplicateParam));
        }
        if p.bind.is_some() && p.ty != ParamType::Plain {
            return Err(fault(w, SchemaFault::BindNotPlain));
        }
        if let ParamType::Choice { values } = &p.ty {
            if values.is_empty() {
                return Err(fault(w, SchemaFault::EmptyChoice));
            }
            let in_values = |d: &Value| d.as_str().is_some_and(|d| values.iter().any(|v| v == d));
            if p.default.as_ref().is_some_and(|d| !in_values(d)) {
                return Err(fault(w, SchemaFault::DefaultNotInChoice));
            }
        }
    }
    Ok(())
}

/// Un widget de pack peut remplacer un widget de base s'il garde sa
/// catégorie et chacun de ses paramètres, avec un type compatible.
fn check_override(base: &WidgetSchema, pack: &WidgetSchema) -> Result<(), WidgetError> {
    if base.category != pack.category {
        return Err(IncompatibleCategorySnafu { widget: base.name.clone() }.build().into());
    }
    for bp in &base.params {
        if !pack.param(&bp.name).is_some_and(|pp| compatible(&bp.ty, &pp.ty)) {
            return Err(IncompatibleOverrideSnafu { widget: base.name.clone(), param: bp.name.clone() }.build().into());
        }
    }
    Ok(())
}

/// Une valeur valide pour `base` l'est-elle pour `pack` ?
fn compatible(base: &ParamType, pack: &ParamType) -> bool {
    match (base, pack) {
        (ParamType::Choice { values: b }, ParamType::Choice { values: p }) => b.iter().all(|v| p.contains(v)),
        (ParamType::Number { .. }, ParamType::Number { .. }) => true,
        (ParamType::Slot { mode: b, .. }, ParamType::Slot { mode: p, .. }) => b == p,
        (b, p) => b == p,
    }
}

#[cfg(test)]
#[path = "widget_tests.rs"]
mod widget_tests;
```

Dans `crates/oku-slides-core/src/lib.rs` : ajouter `pub mod widget;`, compléter le doc du module par une ligne
`//! - [`WidgetRegistry`] : schéma des widgets déclarés en Typst, lu depuis la sonde du rendu.`, et exporter :

```rust
pub use error::{SchemaFault, WidgetError, WidgetErrorKind};
pub use widget::{Category, HostType, ParamSchema, ParamType, SlotMode, WidgetRegistry, WidgetSchema};
```

(ajouter `SchemaFault, WidgetError, WidgetErrorKind` à la liste `pub use error::{…}` existante).

- [ ] **Step 5: Add the messages (20 languages)**

À la fin de `crates/oku-slides-i18n/src/locales/en/core.ftl` :

```ftl

widget-unreadable = The widget declarations could not be read
widget-fault-empty-name = Widget { $widget }: a widget or parameter has no name
widget-fault-duplicate-widget = Widget { $widget } is declared twice
widget-fault-duplicate-param = Widget { $widget } declares the same parameter twice
widget-fault-empty-choice = Widget { $widget }: a choice has no values
widget-fault-default-not-in-choice = Widget { $widget }: a choice default is not one of its values
widget-fault-bind-not-plain = Widget { $widget }: only plain text parameters can be bound to the deck
widget-fault-unknown-slot-kind = Widget { $widget }: a slot accepts an unknown element widget
widget-incompatible-override = Widget { $widget } replaces the built-in one but changes or drops its parameter { $param }
widget-incompatible-category = Widget { $widget } replaces the built-in one with a different category
```

À la fin de `fr/core.ftl` :

```ftl

widget-unreadable = Les déclarations de widgets sont illisibles
widget-fault-empty-name = Widget { $widget } : un widget ou un paramètre n'a pas de nom
widget-fault-duplicate-widget = Le widget { $widget } est déclaré deux fois
widget-fault-duplicate-param = Le widget { $widget } déclare deux fois le même paramètre
widget-fault-empty-choice = Widget { $widget } : un choix n'a aucune valeur
widget-fault-default-not-in-choice = Widget { $widget } : la valeur par défaut d'un choix n'en fait pas partie
widget-fault-bind-not-plain = Widget { $widget } : seul un texte simple peut être lié au deck
widget-fault-unknown-slot-kind = Widget { $widget } : un slot accepte un widget d'élément inconnu
widget-incompatible-override = Le widget { $widget } remplace celui de base mais modifie ou retire son paramètre { $param }
widget-incompatible-category = Le widget { $widget } remplace celui de base avec une autre catégorie
```

Même bloc, traduit, dans `core.ftl` des 18 autres langues : `ar bn de es hi it ja ko nl pl pt ru th tr uk vi zh-Hans zh-Hant`. Les variables `{ $widget }` et `{ $param }` restent telles quelles. Suivre le ton des messages voisins du même fichier.

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo nextest run -p oku-slides-core widget && cargo nextest run -p oku-slides-i18n`
Expected: PASS. `every_locale_has_exactly_the_reference_keys` est vert.

- [ ] **Step 7: Lint and commit**

```bash
cargo fmt --all && cargo clippy -p oku-slides-core --all-targets -- -D warnings
```
```bash
git add crates/oku-slides-core/src/widget.rs crates/oku-slides-core/src/widget_tests.rs crates/oku-slides-core/src/error.rs crates/oku-slides-core/src/lib.rs crates/oku-slides-i18n/src/locales
```
```bash
git commit -m "feat(core) : registre des widgets, schéma des paramètres et surcharge par pack (t-270)"
```

---

### Task 2: Rendu figé du deck de référence

Avant de toucher au Typst, on fige le rendu : chaque page du deck de référence est rendue en PNG et son empreinte sha-256 est gardée dans un instantané `insta`. Toute tâche suivante doit garder cet instantané inchangé.

**Files:**
- Create: `crates/oku-slides-render/tests/golden/widgets.json`
- Create: `crates/oku-slides-render/tests/golden.rs`
- Modify: `crates/oku-slides-render/Cargo.toml` (`[dev-dependencies]` : `insta.workspace = true`, `sha2.workspace = true`, `hex.workspace = true`)

**Interfaces:**
- Consumes: `Renderer::render`, `Compiled::png` (existants).
- Produces: l'instantané `crates/oku-slides-render/tests/snapshots/golden__reference_pages.snap`.

- [ ] **Step 1: Write the reference deck**

`crates/oku-slides-render/tests/golden/widgets.json` couvre chaque type, layout et placement du format v1 :

```json
{
  "params": { "title": "Référence", "subtitle": "Tous les widgets", "author": "Oku", "date": "2026-09-27", "organization": "Oku Theory" },
  "vars": { "client": "Acme" },
  "slides": [
    { "id": "s-titre", "kind": "title" },
    { "id": "s-sommaire", "kind": "toc" },
    { "id": "s-chap", "kind": "chapter", "title": "Chapitre {{client}}", "subtitle": "Sous-titre" },
    { "id": "s-flux", "kind": "content", "kicker": "Surtitre", "title": "Flux",
      "blocks": [
        { "id": "t1", "kind": "text", "style": { "size": "subtitle", "color": "accent", "align": "center", "leading": "tight" },
          "text": [ { "runs": [ { "t": "Texte " }, { "t": "gras", "bold": true }, { "t": " et " }, { "t": "pastille", "highlight": 2 } ] } ] },
        { "id": "c1", "kind": "card", "kicker": "Blague", "title": "Toto et la géographie",
          "text": [ { "runs": [ { "t": "La maîtresse : « Où se trouve la Seine ? »" } ] }, { "list": "bullet", "runs": [ { "t": "Dans son lit !" } ] } ] },
        { "id": "g1", "kind": "chart", "option": {
          "dataset": { "source": [["Mois", "Ventes"], ["Jan", 3], ["Fév", 5]] },
          "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } }
      ] },
    { "id": "s-colonnes", "kind": "content", "title": "Colonnes", "layout": "columns",
      "blocks": [
        { "id": "c2", "kind": "card", "highlight": true, "title": "Mise en avant", "text": [ { "list": "number", "runs": [ { "t": "Un" } ] }, { "list": "number", "runs": [ { "t": "Deux" } ] } ] },
        { "id": "c3", "kind": "card", "kicker": "Sans corps", "title": "Carte vide" }
      ] },
    { "id": "s-split", "kind": "content", "title": "Split", "layout": "split",
      "blocks": [ { "id": "t2", "kind": "text", "text": [ { "runs": [ { "t": "Gauche" } ] } ] },
                  { "id": "i1", "kind": "image", "asset": "sha256:00", "fit": "contain" } ] },
    { "id": "s-centre", "kind": "content", "title": "Centre", "layout": "center",
      "blocks": [ { "id": "t3", "kind": "text", "text": [ { "runs": [ { "t": "Au centre" } ] } ] } ] },
    { "id": "s-citation", "kind": "content", "layout": "quote",
      "blocks": [ { "id": "t4", "kind": "text", "text": [ { "runs": [ { "t": "Une citation", "italic": true } ] } ] } ] },
    { "id": "s-groupes", "kind": "content", "title": "Groupes",
      "blocks": [
        { "id": "grp-col", "kind": "group", "layout": { "type": "columns", "count": 3, "gap": "md" }, "children": [
          { "id": "gc1", "kind": "card", "title": "A" }, { "id": "gc2", "kind": "card", "title": "B" }, { "id": "gc3", "kind": "card", "title": "C" } ] },
        { "id": "grp-rows", "kind": "group", "layout": { "type": "rows", "gap": "sm" }, "children": [
          { "id": "gr1", "kind": "text", "text": [ { "runs": [ { "t": "Ligne 1" } ] } ] },
          { "id": "gr2", "kind": "text", "text": [ { "runs": [ { "t": "Ligne 2" } ] } ] } ] },
        { "id": "grp-grid", "kind": "group", "layout": { "type": "grid", "count": 2 }, "children": [
          { "id": "gg1", "kind": "shape", "shape": "rect", "style": { "fill": "accent", "radius": "sm" } },
          { "id": "gg2", "kind": "shape", "shape": "ellipse", "style": { "fill": "surface", "stroke": "rule" } } ] }
      ] },
    { "id": "s-grille", "kind": "content", "title": "Grille", "grid": { "columns": 12, "rows": 6, "gap": "md" },
      "elements": [
        { "id": "e-cell", "kind": "card", "title": "Case", "cell": { "col": 0, "row": 0, "cols": 6, "rows": 3 } },
        { "id": "e-frame", "kind": "shape", "shape": "line", "frame": { "x": 900, "y": 500, "w": 600, "h": 10 } },
        { "id": "e-libre", "kind": "group", "layout": { "type": "free" }, "frame": { "x": 800, "y": 0, "w": 800, "h": 400 }, "children": [
          { "id": "el1", "kind": "text", "frame": { "x": 20, "y": 20, "w": 300, "h": 100 }, "text": [ { "runs": [ { "t": "Libre" } ] } ] },
          { "id": "el2", "kind": "shape", "frame": { "x": 400, "y": 50, "w": 200, "h": 200 }, "style": { "fill": "accent" } } ] }
      ] },
    { "id": "s-nue", "kind": "blank",
      "elements": [ { "id": "b-texte", "kind": "text", "frame": { "x": 100, "y": 100, "w": 800, "h": 200 }, "style": { "size": "title" }, "text": [ { "runs": [ { "t": "Slide nue" } ] } ] } ] },
    { "id": "s-fin", "kind": "end", "text": "Merci {{client}}" }
  ]
}
```

- [ ] **Step 2: Write the golden test**

`crates/oku-slides-render/tests/golden.rs` :

```rust
//! Rendu figé : les pages des decks de référence restent identiques au
//! pixel près pendant le passage aux widgets Typst (t-270).

use std::collections::BTreeMap;

use oku_slides_core::Deck;
use oku_slides_render::{Fonts, Renderer};
use sha2::{Digest, Sha256};

fn deck(path: &str) -> Deck {
    let path = format!("{}/{path}", env!("CARGO_MANIFEST_DIR"));
    let json = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path} illisible : {e}"));
    Deck::from_json(&json).unwrap_or_else(|e| panic!("{path} invalide : {e}"))
}

#[test]
fn render_reference_decks_keeps_every_page_pixel_identical() {
    let mut r = Renderer::new(Fonts::default()).unwrap();
    let mut lines = Vec::new();
    for (name, path) in [("roadmap", "../../fixtures/roadmap.json"), ("widgets", "tests/golden/widgets.json")] {
        let compiled = r.render(&deck(path), &BTreeMap::new()).unwrap();
        for page in 0..compiled.page_count() {
            let png = compiled.png(page, 0.5).unwrap();
            lines.push(format!("{name} p{} {}", page + 1, hex::encode(Sha256::digest(&png))));
        }
    }
    insta::assert_snapshot!("reference_pages", lines.join("\n"));
}
```

- [ ] **Step 3: Record the snapshot on the current code**

Run: `INSTA_UPDATE=always cargo nextest run -p oku-slides-render --test golden`
Expected: PASS, `tests/snapshots/golden__reference_pages.snap` créé, avec une ligne par page (8 pour roadmap, 12 pour widgets).

- [ ] **Step 4: Check the snapshot is stable**

Run: `cargo nextest run -p oku-slides-render --test golden` deux fois.
Expected: PASS les deux fois, aucun fichier `.snap.new`.

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-render/Cargo.toml crates/oku-slides-render/tests/golden.rs crates/oku-slides-render/tests/golden crates/oku-slides-render/tests/snapshots
```
```bash
git commit -m "test(render) : rendu figé des decks de référence avant les widgets (t-270)"
```

---

### Task 3: Extraire `content.typ` et `zone.typ`

Pur déplacement, pour que les widgets importent ces briques sans cycle (`data.typ` importera `widgets.typ`).

**Files:**
- Create: `typst/oku-slides/src/content.typ`
- Create: `typst/oku-slides/src/zone.typ`
- Modify: `typst/oku-slides/src/data.typ`
- Modify: `crates/oku-slides-render/src/world.rs` (`LIBRARY`)

**Interfaces:**
- Produces (Typst) :
  - `content.typ` : `_text-args(t, style)`, `_text-over(t, style)`, `_runs(t, runs, vars)`, `_paragraphs(t, paras, vars)`.
  - `zone.typ` : `default-grid`, `grid-frame(zone-w, zone-h, g, gap, cell)`, `zone-marker(t, grid)`.

- [ ] **Step 1: Create `content.typ`**

`typst/oku-slides/src/content.typ` reçoit, tels quels, `_text-args`, `_runs` et `_paragraphs` de `data.typ`, avec en tête :

```typst
// Texte du deck JSON en contenu Typst : style d'un texte, runs, paragraphes.
// Aucun texte n'est interprété comme du code Typst.

#import "tokens.typ": *
#import "core.typ": interp
```

Et une nouvelle fonction, extraite du bloc `over` de `render-element` :

```typst
// Surcharges typographiques d'un élément texte, pour la sonde de style de
// sa boîte (voir `text-probe`).
#let _text-over(t, style) = {
  let o = _text-args(t, style)
  if "leading" in style { o.insert("leading", leading-of(t, style.leading)) }
  o.insert("align", (left: left, center: center, right: right, start: start, end: end)
    .at(style.at("align", default: "start"), default: start))
  o
}
```

- [ ] **Step 2: Create `zone.typ`**

`typst/oku-slides/src/zone.typ` reçoit, tels quels, `grid-frame` (avec son commentaire « Même calcul que ui/src/grid.ts »), `default-grid` et `zone-marker` de `data.typ`, avec en tête :

```typst
// Zone de corps d'une slide : là où se placent les éléments libres, sur
// leur cadre ou sur une cellule de la grille.

#import "tokens.typ": *
```

- [ ] **Step 3: Rewire `data.typ`**

Dans `data.typ` : supprimer les définitions déplacées, ajouter
`#import "content.typ": _text-args, _text-over, _runs, _paragraphs` et `#import "zone.typ": default-grid, grid-frame, zone-marker`. Remplacer le bloc `let over = if kind == "text" { … } else { (:) }` de `render-element` par `let over = if kind == "text" { _text-over(t, style) } else { (:) }`.

- [ ] **Step 4: Embed the new files**

Dans `crates/oku-slides-render/src/world.rs`, ajouter `"src/content.typ"` et `"src/zone.typ"` à `lib_files!` (avant `"src/data.typ"`).

- [ ] **Step 5: Run the render tests**

Run: `cargo nextest run -p oku-slides-render`
Expected: PASS, instantané doré inchangé.

- [ ] **Step 6: Commit**

```bash
git add typst/oku-slides/src crates/oku-slides-render/src/world.rs
```
```bash
git commit -m "refactor(render) : content.typ et zone.typ extraits de data.typ (t-270)"
```

---

### Task 4: `widget`, `field`, `p` et lecture du registre de base

**Files:**
- Create: `typst/oku-slides/src/params.typ`
- Create: `typst/oku-slides/src/widget.typ`
- Create: `typst/oku-slides/src/widgets.typ`
- Modify: `typst/oku-slides/src/core.typ` (`_measured`, `tracked`)
- Modify: `crates/oku-slides-render/src/world.rs`
- Create: `crates/oku-slides-render/src/registry.rs`
- Modify: `crates/oku-slides-render/src/error.rs`
- Modify: `crates/oku-slides-render/src/lib.rs`
- Modify: `crates/oku-slides-i18n/src/locales/*/document.ftl` (20 langues)
- Create: `crates/oku-slides-render/tests/widgets.rs`

**Interfaces:**
- Consumes: `WidgetRegistry::from_probe`, `WidgetError` (tâche 1).
- Produces:
  - Typst : module `params.typ` (`plain`, `rich`, `flag`, `number`, `choice`, `asset`, `slot`, `grid`, `host`). `widget(name, label:, category:, params:, render) -> (name, schema, render)`. `field(ctx, name, body, style:)` émet `<oku-slides-field>`. `by-name(registry) -> dictionary`. `_measured(meta, label-name, body, style)` dans `core.typ`. `widgets.typ` exporte `registry` (tableau).
  - Rust : `oku_slides_render::load_registry(fonts: &Fonts, pack: Option<&PackWidgets>) -> Result<WidgetRegistry, RegistryError>`. `PackWidgets::new(id: impl Into<String>, source: impl Into<String>) -> Result<PackWidgets, RegistryError>` (le pack sert à la tâche 8). `RegistryError`, `RegistryErrorKind::{BadPackId, Compile, Missing, Schema, Setup}`. `RegistryError::diagnostics() -> &[Diagnostic]`, `RegistryError::widget_error() -> Option<&WidgetError>`.
  - `OkuSlidesWorld::set_main_text(&mut self, text: &str)`, `OkuSlidesWorld::add_source(&mut self, path: &str, text: String) -> Result<(), RenderError>`, `OkuSlidesWorld::diagnostic(&self, &SourceDiagnostic) -> Diagnostic` (déplacé de `Renderer`).

- [ ] **Step 1: Write the failing test**

`crates/oku-slides-render/tests/widgets.rs` :

```rust
use oku_slides_core::{Category, ParamType};
use oku_slides_render::{Fonts, load_registry};
use speculoos::prelude::*;

#[test]
fn load_registry_base_declares_the_text_widget_with_a_rich_body() {
    let r = load_registry(&Fonts::default(), None).unwrap();
    let text = r.get("text").unwrap();
    assert_that!(text.category).is_equal_to(Category::Element);
    assert_that!(text.param("body").map(|p| p.ty.clone())).is_equal_to(Some(ParamType::Rich));
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo nextest run -p oku-slides-render --test widgets`
Expected: FAIL à la compilation, `load_registry` introuvable.

- [ ] **Step 3: Write `params.typ`**

```typst
// Descripteurs des paramètres d'un widget, à importer comme module :
// `#import "params.typ" as p`, puis `p.plain(label: "Titre")`.
// Chaque descripteur est un dictionnaire sérialisable, lu par l'hôte ; il
// ne porte que les clés de son type.

#let _base(type, label, default) = (type: type, label: label, default: default)

/// Texte sur une ligne. bind : clé de `deck.params` où lire et écrire la valeur.
#let plain(label: none, default: none, bind: none) = _base("plain", label, default) + (bind: bind)
/// Paragraphes à runs.
#let rich(label: none) = _base("rich", label, none)
#let flag(label: none, default: false) = _base("flag", label, default)
#let number(label: none, default: none, min: none, max: none) = _base("number", label, default) + (min: min, max: max)
/// Une valeur parmi `values` ; par défaut la première.
#let choice(..values, label: none, default: auto) = {
  let vs = values.pos()
  _base("choice", label, if default == auto { vs.at(0, default: none) } else { default }) + (values: vs)
}
/// Identifiant d'asset `sha256:…`.
#let asset(label: none) = _base("asset", label, none)
/// Éléments enfants. mode : "flow" (agencés par le widget) ou "free" (sur
/// leur cadre ou leur cellule). accepts : noms de widgets admis, ou none.
#let slot(label: none, mode: "flow", accepts: none) = _base("slot", label, none) + (mode: mode, accepts: accepts)
/// Grille { columns, rows, gap }.
#let grid(label: none, default: none) = _base("grid", label, default)
/// Type fourni et validé par l'hôte (`"chart"`).
#let host(name, label: none) = _base("host", label, none) + (host: name)
```

- [ ] **Step 4: Share the box emitter in `core.typ`**

Remplacer `tracked` par :

```typst
// Émet la boîte réelle d'un contenu (page, position, taille) et sa
// typographie sous l'étiquette `label-name`, avec les clés de `meta`.
// La boîte suit le texte, alignée dans la zone disponible.
#let _measured(meta, label-name, body, style) = layout(sz => {
  let m = measure(body, width: sz.width)
  context {
    let p = here().position()
    let st = text-probe(style)
    let free = sz.width - m.width
    let dx = if st.align == "center" { free / 2 } else if st.align == "right" { free } else { 0pt }
    [#metadata(meta + (page: p.page, x: (p.x + dx) / 1pt, y: p.y / 1pt, w: m.width / 1pt, h: m.height / 1pt,
      ax: p.x / 1pt, aw: sz.width / 1pt, style: st))#label(label-name)]
  }
  body
})

#let tracked(id, body, style: (:)) = _measured((id: id), "oku-slides-box", body, style)
```

- [ ] **Step 5: Write `widget.typ`**

```typst
// Widgets : composants déclarés avec le schéma de leurs paramètres.
//
//   #import "params.typ" as p
//   #let card = widget("card", params: (title: p.plain(), body: p.rich()),
//     (args, ctx) => field(ctx, "title", args.title) + args.body)
//
// args : valeurs des paramètres, prêtes à rendre. Slot flow : entrées
// (kind, id, body) ; slot free : entrées (kind, id, frame, cell, show),
// `show(frame)` rendant l'enfant.
// ctx : id (élément ou slide), tokens, vars, style (style de l'élément),
// framed (l'élément a un cadre), load (chargeur d'image ou de graphique de
// l'hôte, fonction sans argument, ou none).

#import "core.typ": _measured

#let _types = ("plain", "rich", "flag", "number", "choice", "asset", "slot", "grid", "host")

/// Déclare un widget. category : "element" ou "slide".
#let widget(name, label: none, category: "element", params: (:), render) = {
  assert(type(name) == str and name != "", message: "oku-slides : un widget a besoin d'un nom")
  assert(category in ("slide", "element"),
    message: "oku-slides : widget « " + name + " » : catégorie inconnue « " + repr(category) + " »")
  let list = ()
  for (key, d) in params {
    assert(type(d) == dictionary and d.at("type", default: none) in _types,
      message: "oku-slides : widget « " + name + " » : le paramètre « " + key + " » n'est pas un descripteur p.*")
    list.push((name: key) + d)
  }
  (name: name, schema: (name: name, label: label, category: category, params: list), render: render)
}

/// Widgets d'un registre, par nom ; un nom répété remplace le précédent.
#let by-name(registry) = {
  let d = (:)
  for w in registry { d.insert(w.name, w) }
  d
}

/// Champ éditable : émet la boîte de `body` pour le paramètre `name` de
/// l'objet `ctx.id` (<oku-slides-field>), avec sa typographie.
#let field(ctx, name, body, style: (:)) = if ctx.id == none { body } else {
  _measured((owner: str(ctx.id), field: name), "oku-slides-field", body, style)
}
```

- [ ] **Step 6: Start `widgets.typ` with `text`**

Le widget n'est pas encore branché au rendu (tâche 5) : seul son schéma compte ici.

```typst
// Widgets de base d'Oku Slides : éléments et slides. `registry` est lu par
// l'hôte (sonde <oku-widgets>) et par le rendu du deck.

#import "tokens.typ": *
#import "params.typ" as p
#import "widget.typ": widget, field
#import "content.typ": _text-args, _text-over

#let _text = widget("text", params: (body: p.rich()), (args, ctx) => {
  let t = ctx.tokens
  let style = ctx.style
  set text(.._text-args(t, style))
  if "leading" in style { set par(leading: leading-of(t, style.leading)) }
  // Sans alignement choisi, le texte part du début de la ligne : à gauche,
  // ou à droite dans une langue écrite de droite à gauche.
  let al = style.at("align", default: "start")
  field(ctx, "body", style: _text-over(t, style),
    align((left: left, center: center, right: right, start: start, end: end).at(al, default: start), args.body))
})

#let registry = (_text,)
```

- [ ] **Step 7: Let the world take any main text and extra sources**

Dans `world.rs` : ajouter `"src/params.typ"`, `"src/widget.typ"` et `"src/widgets.typ"` à `lib_files!` (avant `"src/data.typ"`), puis dans `impl OkuSlidesWorld` :

```rust
    /// Remplace la source principale (sonde du registre).
    pub(crate) fn set_main_text(&mut self, text: &str) {
        if self.main.text() != text {
            self.main.replace(text);
        }
    }

    /// Ajoute une source Typst au système de fichiers virtuel.
    pub(crate) fn add_source(&mut self, path: &str, text: String) -> Result<(), RenderError> {
        let id = file_id(path)?;
        self.sources.insert(id, Source::new(id, text));
        Ok(())
    }

    /// Un message de Typst, avec sa place (`fichier:ligne`).
    pub(crate) fn diagnostic(&self, d: &SourceDiagnostic) -> Diagnostic {
        // corps actuel de `Renderer::diagnostic`, avec `self.range(..)` et `self.source_of(..)`
    }
```

Déplacer le corps de `Renderer::diagnostic` (lib.rs) dans `OkuSlidesWorld::diagnostic`. `Renderer::compile` appelle alors `self.world.diagnostic(d)`. Importer `typst::diag::SourceDiagnostic`, `typst::WorldExt` et `crate::diagnostic::{Diagnostic, Severity}` dans `world.rs`.

- [ ] **Step 8: Add `RegistryError`**

À la fin de `crates/oku-slides-render/src/error.rs` :

```rust
/// Le registre des widgets n'a pas pu être lu.
#[derive(Debug, Snafu)]
pub struct RegistryError(pub(crate) RegistryInner);

/// Nature d'une [`RegistryError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RegistryErrorKind {
    /// Identifiant de pack refusé (il deviendrait un chemin).
    BadPackId,
    /// Les widgets ne compilent pas (voir [`RegistryError::diagnostics`]).
    Compile,
    /// La sonde n'a pas émis de registre.
    Missing,
    /// Un widget est mal déclaré ou remplace mal un widget de base.
    Schema,
    Setup,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum RegistryInner {
    #[snafu(display("invalid template pack identifier: {id}"))]
    BadPackId { id: String, backtrace: Backtrace },
    #[snafu(display("widget compilation failed:\n{}", lines(diagnostics)))]
    RegistryCompile { diagnostics: Vec<Diagnostic>, backtrace: Backtrace },
    #[snafu(display("the widget probe emitted no registry"))]
    Missing { backtrace: Backtrace },
    #[snafu(display("invalid widget registry"))]
    Schema { source: oku_slides_core::WidgetError, backtrace: Backtrace },
    #[snafu(display("the widget probe could not start"))]
    Setup { source: RenderError, backtrace: Backtrace },
}

impl RegistryError {
    pub fn kind(&self) -> RegistryErrorKind {
        match self.0 {
            RegistryInner::BadPackId { .. } => RegistryErrorKind::BadPackId,
            RegistryInner::RegistryCompile { .. } => RegistryErrorKind::Compile,
            RegistryInner::Missing { .. } => RegistryErrorKind::Missing,
            RegistryInner::Schema { .. } => RegistryErrorKind::Schema,
            RegistryInner::Setup { .. } => RegistryErrorKind::Setup,
        }
    }

    /// Messages de Typst, un par problème, si les widgets ne compilent pas.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        match &self.0 {
            RegistryInner::RegistryCompile { diagnostics, .. } => diagnostics,
            _ => &[],
        }
    }

    /// Le défaut de déclaration, si un widget est mal déclaré.
    pub fn widget_error(&self) -> Option<&oku_slides_core::WidgetError> {
        match &self.0 {
            RegistryInner::Schema { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl Localize for RegistryError {
    fn localize(&self, l: &Localizer) -> String {
        match &self.0 {
            RegistryInner::BadPackId { id, .. } => l.tr_with("registry-bad-pack-id", &[("id", id.as_str().into())]),
            RegistryInner::RegistryCompile { diagnostics, .. } => {
                let mut out = l.tr("registry-compile");
                for d in diagnostics {
                    out.push('\n');
                    out.push_str(&d.localize(l));
                }
                out
            }
            RegistryInner::Missing { .. } => l.tr("registry-missing"),
            RegistryInner::Schema { source, .. } => source.localize(l),
            RegistryInner::Setup { source, .. } => source.localize(l),
        }
    }
}
```

(La variante s'appelle `RegistryCompile` pour que son sélecteur Snafu ne heurte pas `CompileSnafu` de `RenderInner`.)

Messages, à la fin de `en/document.ftl` :

```ftl

registry-bad-pack-id = Invalid template pack identifier: { $id }
registry-compile = The widgets of the template pack could not be compiled
registry-missing = The template pack does not declare its widgets
```

`fr/document.ftl` :

```ftl

registry-bad-pack-id = Identifiant de pack de modèles invalide : { $id }
registry-compile = Les widgets du pack de modèles ne compilent pas
registry-missing = Le pack de modèles ne déclare pas ses widgets
```

Même bloc, traduit, dans `document.ftl` des 18 autres langues.

- [ ] **Step 9: Write `registry.rs`**

```rust
//! Registre des widgets : compile une sonde Typst qui émet le schéma des
//! widgets de base et, s'il y en a, de ceux d'un pack.

use serde_json::Value as Json;
use snafu::{OptionExt, ResultExt};
use typst::foundations::{Label, Selector};
use typst::utils::PicoStr;
use typst_layout::PagedDocument;

use oku_slides_core::WidgetRegistry;

use crate::error::{BadPackIdSnafu, MissingSnafu, RegistryCompileSnafu, RegistryError, SchemaSnafu, SetupSnafu};
use crate::fonts::Fonts;
use crate::world::OkuSlidesWorld;

/// Les widgets d'un pack de modèles : son identifiant et son `widgets.typ`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackWidgets {
    id: String,
    source: String,
}

impl PackWidgets {
    /// `id` : lettres minuscules ASCII, chiffres, `-` et `_` ; il devient un chemin.
    pub fn new(id: impl Into<String>, source: impl Into<String>) -> Result<Self, RegistryError> {
        let id = id.into();
        let ok = !id.is_empty() && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_');
        if !ok {
            return Err(BadPackIdSnafu { id }.build().into());
        }
        Ok(Self { id, source: source.into() })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn source(&self) -> &str {
        &self.source
    }

    fn path(&self) -> String {
        format!("/packs/{}/widgets.typ", self.id)
    }
}

/// Source de la sonde : importe les registres, émet leurs schémas.
fn probe_source(pack: Option<&PackWidgets>) -> String {
    let mut s = String::from("#import \"/oku-slides/src/widgets.typ\": registry as base\n");
    let pack_expr = match pack {
        Some(p) => {
            s.push_str(&format!("#import \"{}\": registry as pack\n", p.path()));
            "pack.map(w => w.schema)"
        }
        None => "none",
    };
    s.push_str(&format!("#metadata((base: base.map(w => w.schema), pack: {pack_expr}))<oku-widgets>\n"));
    s
}

/// Compile la sonde et lit le registre (base, surchargée par le pack).
pub fn load_registry(fonts: &Fonts, pack: Option<&PackWidgets>) -> Result<WidgetRegistry, RegistryError> {
    let mut world = OkuSlidesWorld::new(fonts.clone()).context(SetupSnafu)?;
    if let Some(p) = pack {
        world.add_source(&p.path(), p.source().to_owned()).context(SetupSnafu)?;
    }
    world.set_main_text(&probe_source(pack));
    let result = typst::compile::<PagedDocument>(&world);
    let doc = result.output.map_err(|errors| {
        RegistryCompileSnafu { diagnostics: errors.iter().map(|d| world.diagnostic(d)).collect::<Vec<_>>() }.build()
    })?;
    let label = Label::new(PicoStr::intern("oku-widgets")).context(MissingSnafu)?;
    let found = doc.introspector().query(&Selector::Label(label));
    let value: Json = found
        .first()
        .and_then(|c| c.get_by_name("value").ok())
        .and_then(|v| serde_json::to_value(&v).ok())
        .context(MissingSnafu)?;
    WidgetRegistry::from_probe(value).context(SchemaSnafu)
}
```

Dans `lib.rs` : `mod registry;`, `pub use registry::{PackWidgets, load_registry};`, `pub use error::{RegistryError, RegistryErrorKind};`. Ajouter au doc du module : `//! Le registre des widgets ([`load_registry`]) se lit par une sonde Typst.`

- [ ] **Step 10: Run the tests**

Run: `cargo nextest run -p oku-slides-render && cargo nextest run -p oku-slides-i18n`
Expected: PASS, instantané doré inchangé (`tracked` a le même rendu).

- [ ] **Step 11: Lint and commit**

```bash
cargo fmt --all && cargo clippy -p oku-slides-render --all-targets -- -D warnings
```
```bash
git add typst/oku-slides/src crates/oku-slides-render crates/oku-slides-i18n/src/locales
```
```bash
git commit -m "feat(render) : widget, field et descripteurs p.* en Typst, registre lu par une sonde (t-270)"
```

---

### Task 5: Widgets `text` et `card` branchés, champs émis

**Files:**
- Modify: `typst/oku-slides/src/slides.typ` (`card`)
- Modify: `typst/oku-slides/src/widgets.typ`
- Modify: `typst/oku-slides/src/data.typ`
- Modify: `crates/oku-slides-render/src/lib.rs` (`FieldBox`, `Layout::fields`, `fields_on`)
- Modify: `crates/oku-slides-render/tests/widgets.rs`

**Interfaces:**
- Consumes: `widget`, `field`, `by-name`, `registry` (tâche 4).
- Produces:
  - `oku_slides_render::FieldBox { pub owner: String, pub field: String, pub page: usize, pub x: f64, pub y: f64, pub w: f64, pub h: f64, pub ax: Option<f64>, pub aw: Option<f64>, pub style: Option<TextStyle> }`.
  - `Layout::fields: Vec<FieldBox>` (rangés par page), `Layout::fields_on(page) -> &[FieldBox]`.
  - Typst : `card(.., wrap: (name, body, style) => content, body)`. `render-element` passe par le registre.

- [ ] **Step 1: Write the failing tests**

Ajouter à `tests/widgets.rs` :

```rust
use std::collections::BTreeMap;

use oku_slides_core::Deck;
use oku_slides_render::Renderer;

fn roadmap() -> Deck {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");
    Deck::from_json(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn compile_card_emits_one_field_per_filled_text_param() {
    let mut r = Renderer::new(Fonts::default()).unwrap();
    let compiled = r.render(&roadmap(), &BTreeMap::new()).unwrap();
    let layout = compiled.layout();
    let fields: Vec<&str> =
        layout.fields.iter().filter(|f| f.owner == "c1").map(|f| f.field.as_str()).collect();
    assert_that!(fields).is_equal_to(vec!["kicker", "title", "body"]);
    let card = layout.boxes.iter().find(|b| b.id == "c1").unwrap();
    for f in layout.fields.iter().filter(|f| f.owner == "c1") {
        asserting(&format!("{f:?} dans la carte"))
            .that(&(f.x >= card.x && f.y >= card.y && f.x + f.w <= card.x + card.w + 0.01))
            .is_true();
    }
}

#[test]
fn compile_card_title_field_carries_the_card_title_style() {
    let mut r = Renderer::new(Fonts::default()).unwrap();
    let compiled = r.render(&roadmap(), &BTreeMap::new()).unwrap();
    let layout = compiled.layout();
    let title = layout.fields.iter().find(|f| f.owner == "c1" && f.field == "title").unwrap();
    let body = layout.fields.iter().find(|f| f.owner == "c1" && f.field == "body").unwrap();
    let size = |f: &oku_slides_render::FieldBox| f.style.as_ref().unwrap().size;
    assert_that!(size(title)).is_greater_than(size(body));
}

#[test]
fn compile_text_element_emits_its_body_field() {
    let mut r = Renderer::new(Fonts::default()).unwrap();
    let compiled = r.render(&roadmap(), &BTreeMap::new()).unwrap();
    let layout = compiled.layout();
    assert_that!(layout.fields.iter().any(|f| f.owner == "b-points" && f.field == "body")).is_true();
}

#[test]
fn compile_fields_on_page_keep_only_that_page() {
    let mut r = Renderer::new(Fonts::default()).unwrap();
    let compiled = r.render(&roadmap(), &BTreeMap::new()).unwrap();
    let layout = compiled.layout();
    let page = layout.fields.iter().find(|f| f.owner == "c1").unwrap().page;
    assert_that!(layout.fields_on(page).iter().all(|f| f.page == page)).is_true();
    assert_that!(layout.fields_on(page).iter().any(|f| f.owner == "c1")).is_true();
}
```

Vérifier d'abord dans `fixtures/roadmap.json` que `c1` a un `kicker`, un `title` et un `text` non vide, et que `b-points` est un `text`. Sinon, prendre des ids qui remplissent ces conditions.

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo nextest run -p oku-slides-render --test widgets`
Expected: FAIL, `fields` n'existe pas sur `Layout`.

- [ ] **Step 3: Collect field boxes in `lib.rs`**

```rust
/// Boîte d'un champ éditable : un paramètre texte d'un élément ou d'une slide.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FieldBox {
    /// Id de l'élément ou de la slide.
    pub owner: String,
    /// Nom du paramètre.
    pub field: String,
    pub page: usize,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    #[serde(default)]
    pub ax: Option<f64>,
    #[serde(default)]
    pub aw: Option<f64>,
    #[serde(default)]
    pub style: Option<TextStyle>,
}
```

Dans `Layout`, ajouter `pub fields: Vec<FieldBox>` (doc : « Champs éditables, pour l'édition en place ») et :

```rust
    /// Champs éditables d'une page.
    pub fn fields_on(&self, page: usize) -> &[FieldBox] {
        let start = self.fields.partition_point(|f| f.page < page);
        let end = self.fields.partition_point(|f| f.page <= page);
        &self.fields[start..end]
    }
```

Dans `Compiled::layout`, après les boîtes :

```rust
            let mut fields: Vec<FieldBox> = parsed(values("oku-slides-field"));
            fields.sort_by_key(|f| f.page);
```

puis `Layout { slides, boxes, fields, zones, tokens }`.

- [ ] **Step 4: Let `card` wrap its parts**

Dans `slides.typ`, `card` prend un paramètre `wrap`. Chaque partie passe par lui, avec le style qu'elle a réellement :

```typst
/// wrap : (nom, contenu, style) => contenu ; le widget y pose ses champs.
#let card(title: none, kicker: none, highlight: false, fill: auto, wrap: (name, body, style) => body, body) = context {
  let t = config().tokens
  let k = t.component.card
  // … bg, ink, border, block(…) inchangés jusqu'au stack …
        let kicker-c = t.component.kicker + if highlight { (color: k.highlight-kicker) } else { (:) }
        let title-style = (size: size-of(t, k.title-size), weight: weight-of(t, k.title-weight),
          fill: color-of(t, if highlight { k.highlight-title } else { k.title-color }))
        stack(spacing: space-of(t, k.gap),
          ..if kicker != none {
            (wrap("kicker", styled(t, kicker-c, kicker), text-style(t, kicker-c) + (case: kicker-c.at("case", default: none))),)
          },
          ..if title != none { (wrap("title", text(..title-style, title), title-style),) },
          wrap("body", if ink != none { text(fill: ink, body) } else { body }, if ink != none { (fill: ink) } else { (:) }),
        )
```

Le reste de `card` ne change pas.

- [ ] **Step 5: Add the `card` widget**

Dans `widgets.typ`, importer `#import "slides.typ": card` et ajouter :

```typst
#let _card = widget("card",
  params: (kicker: p.plain(), title: p.plain(), body: p.rich(), highlight: p.flag()),
  (args, ctx) => card(
    title: args.title, kicker: args.kicker, highlight: args.highlight,
    // Dans un cadre (placement libre ou case de grille), la carte remplit sa place.
    fill: if ctx.framed { true } else { auto },
    wrap: (name, body, style) => field(ctx, name, body, style: style),
    args.body,
  ))
```

Puis `#let registry = (_text, _card)`.

- [ ] **Step 6: Route `text` and `card` through the registry in `data.typ`**

Importer `#import "widgets.typ": registry` et `#import "widget.typ": by-name`, puis ajouter avant `render-element` :

```typst
#let _widgets = by-name(registry)

// Arguments d'un élément v1 pour son widget. MVP: t-270, plan 2 — remplacé
// par le rendu générique du modèle v2.
#let _element-args(t, e, vars, image-loader, render) = {
  let kind = e.at("kind", default: "text")
  let body = if "text" in e { _paragraphs(t, e.text, vars) } else { [] }
  if kind == "text" { (body: body) }
  else if kind == "card" { (
    kicker: interp(e.at("kicker", default: none), vars),
    title: interp(e.at("title", default: none), vars),
    highlight: e.at("highlight", default: false),
    body: body,
  ) }
  else { (:) }
}
```

Dans `render-element`, remplacer les branches `kind == "text"` et `kind == "card"` de `let el = …` par une branche commune :

```typst
  let ctx = (id: e.at("id", default: none), tokens: t, vars: vars, style: style, framed: "frame" in e,
    load: if image-loader != none { () => image-loader(e) } else { none })
  let el = if kind in ("text", "card") {
    (_widgets.at(kind).render)(_element-args(t, e, vars, image-loader, render-element), ctx)
  } else if kind == "image" {
    // … branches image, chart, shape, group, inconnu inchangées …
```

- [ ] **Step 7: Run all render tests**

Run: `cargo nextest run -p oku-slides-render`
Expected: PASS, dont le test doré, instantané inchangé. S'il change : comparer les PNG avant et après (`Compiled::png`) pour trouver l'écart. Le plus probable est un `wrap` qui change l'espacement du `stack` de la carte. Corriger `card`, jamais l'instantané.

- [ ] **Step 8: Lint and commit**

```bash
cargo fmt --all && cargo clippy -p oku-slides-render --all-targets -- -D warnings
```
```bash
git add typst/oku-slides/src crates/oku-slides-render
```
```bash
git commit -m "feat(render) : widgets text et card, un champ éditable par paramètre texte (t-270)"
```

---

### Task 6: Widgets `image`, `shape`, `chart`, `stack` et `canvas`

**Files:**
- Modify: `typst/oku-slides/src/widgets.typ`
- Modify: `typst/oku-slides/src/data.typ`
- Modify: `crates/oku-slides-render/tests/widgets.rs`

**Interfaces:**
- Consumes: `_element-args`, `_widgets` (tâche 5).
- Produces: widgets `image` (`asset: asset`, `fit: choice("cover", "contain", "stretch")`), `shape` (`shape: choice("rect", "ellipse", "line")`), `chart` (`option: host("chart")`), `stack` (`layout: choice("rows", "columns", "grid")`, `count: number(min: 1)`, `gap: choice("xs", "sm", "md", "lg", "xl", "xxl", default: "lg")`, `children: slot(mode: "flow")`), `canvas` (`children: slot(mode: "free")`). Le `group` v1 est rendu par `canvas` (layout `free`) ou `stack` (les autres).

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn load_registry_base_declares_every_element_widget() {
    let r = load_registry(&Fonts::default(), None).unwrap();
    for name in ["text", "card", "image", "shape", "chart", "stack", "canvas"] {
        asserting(name).that(&r.get(name).map(|w| w.category)).is_equal_to(Some(Category::Element));
    }
    let stack = r.get("stack").unwrap();
    assert_that!(stack.param("children").map(|p| p.ty.clone()))
        .is_equal_to(Some(ParamType::Slot { mode: oku_slides_core::SlotMode::Flow, accepts: None }));
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo nextest run -p oku-slides-render --test widgets load_registry_base_declares_every_element_widget`
Expected: FAIL, `image` absent.

- [ ] **Step 3: Move the placeholders and add the widgets**

Déplacer `_image-placeholder` et `_chart-placeholder` de `data.typ` dans `widgets.typ`. `_image-placeholder` prend alors l'identifiant d'asset :

```typst
#let _image-placeholder(t, asset) = rect(
  width: 100%, height: 100%, fill: color-of(t, "surface"), radius: radius-of(t, "sm"),
  align(center + horizon, text(size: size-of(t, "caption"), fill: color-of(t, "ink-muted"),
    "image : " + if asset == none { "?" } else { asset })),
)

#let _image = widget("image",
  params: (asset: p.asset(), fit: p.choice("cover", "contain", "stretch")),
  (args, ctx) => if ctx.load != none { (ctx.load)() } else { _image-placeholder(ctx.tokens, args.asset) })

// Rendu en SVG par l'hôte, chargé comme une image.
#let _chart = widget("chart", params: (option: p.host("chart")),
  (args, ctx) => if ctx.load != none { (ctx.load)() } else { _chart-placeholder(ctx.tokens) })

#let _shape = widget("shape", params: (shape: p.choice("rect", "ellipse", "line")), (args, ctx) => {
  let t = ctx.tokens
  let style = ctx.style
  let fill = color-of(t, style.at("fill", default: none))
  let stroke = if "stroke" in style { stroke-of(t, style.at("stroke-width", default: "rule")) + color-of(t, style.stroke) }
  if args.shape == "ellipse" { ellipse(width: 100%, height: 100%, fill: fill, stroke: stroke) }
  else if args.shape == "line" { line(length: 100%, stroke: if stroke != none { stroke } else { stroke-of(t, "rule") + color-of(t, "rule") }) }
  else { rect(width: 100%, height: 100%, fill: fill, stroke: stroke, radius: radius-of(t, style.at("radius", default: "flat"))) }
})

#let _stack = widget("stack",
  params: (
    layout: p.choice("rows", "columns", "grid"),
    count: p.number(min: 1),
    gap: p.choice("xs", "sm", "md", "lg", "xl", "xxl", default: "lg"),
    children: p.slot(mode: "flow"),
  ),
  (args, ctx) => {
    let t = ctx.tokens
    let gap = space-of(t, args.gap)
    let bodies = args.children.map(c => c.body)
    if args.layout == "columns" or args.layout == "grid" {
      let n = if args.count == none { bodies.len() } else { args.count }
      grid(columns: (1fr,) * n, column-gutter: gap, row-gutter: gap, ..bodies)
    } else {
      stack(spacing: gap, ..bodies)
    }
  })

// Enfants placés sur leur cadre, relatif au canevas.
#let _canvas = widget("canvas", params: (children: p.slot(mode: "free")), (args, ctx) => {
  for c in args.children {
    let f = c.frame
    place(top + left, dx: f.x * 1pt, dy: f.y * 1pt, block(width: f.w * 1pt, height: f.h * 1pt, (c.show)(f)))
  }
})
```

Le code de `_shape` reprend celui de la branche `shape` actuelle de `data.typ`, et celui de `_stack` la branche `group`. Mettre à jour `#let registry = (_text, _card, _image, _shape, _chart, _stack, _canvas)`.

- [ ] **Step 4: Adapt v1 elements to these widgets**

Dans `data.typ`, compléter `_element-args` :

```typst
  else if kind == "image" { (asset: e.at("asset", default: none), fit: e.at("fit", default: "cover")) }
  else if kind == "chart" { (option: e.at("option", default: none)) }
  else if kind == "shape" { (shape: e.at("shape", default: "rect")) }
  else if kind == "group" {
    let children = e.at("children", default: ())
    let lay = e.at("layout", default: (type: "free"))
    if lay.type == "free" {
      (children: children.map(c => (kind: c.at("kind", default: "text"), id: c.at("id", default: none),
        frame: c.frame, cell: none, show: f => render(t, c, vars, image-loader: image-loader))))
    } else {
      (layout: lay.type, count: lay.at("count", default: none), gap: lay.at("gap", default: "lg"),
        children: children.map(c => (kind: c.at("kind", default: "text"), id: c.at("id", default: none),
          body: render(t, c, vars, image-loader: image-loader))))
    }
  }
```

Le nom du widget d'un élément v1 :

```typst
// Widget qui rend un élément v1 : le groupe se scinde en canvas et stack.
#let _widget-of(e) = {
  let kind = e.at("kind", default: "text")
  if kind != "group" { kind }
  else if e.at("layout", default: (type: "free")).type == "free" { "canvas" } else { "stack" }
}
```

`render-element` devient :

```typst
  let w = _widgets.at(_widget-of(e), default: none)
  let el = if w == none { text(fill: red)[oku-slides : élément inconnu « #kind »] }
    else { (w.render)(_element-args(t, e, vars, image-loader, render-element), ctx) }
```

Supprimer les anciennes branches `image`, `chart`, `shape` et `group`. `show: f => render(t, c, …)` rend `c` tel quel, avec son propre `frame`, comme l'ancienne branche `group`.

- [ ] **Step 5: Run all render tests**

Run: `cargo nextest run -p oku-slides-render`
Expected: PASS, instantané doré inchangé.

- [ ] **Step 6: Lint and commit**

```bash
cargo fmt --all && cargo clippy -p oku-slides-render --all-targets -- -D warnings
```
```bash
git add typst/oku-slides/src crates/oku-slides-render/tests/widgets.rs
```
```bash
git commit -m "feat(render) : widgets image, shape, chart, stack et canvas ; le groupe v1 se scinde (t-270)"
```

---

### Task 7: Widgets de slide

**Files:**
- Modify: `typst/oku-slides/src/zone.typ` (`free-area`)
- Modify: `typst/oku-slides/src/widgets.typ`
- Modify: `typst/oku-slides/src/data.typ` (`render-slide`, `_free` supprimé)
- Modify: `crates/oku-slides-render/tests/widgets.rs`
- Create: `crates/oku-slides-render/tests/snapshots/widgets__base_registry.snap` (par insta)

**Interfaces:**
- Consumes: widgets d'élément (tâches 5-6), `default-grid`, `grid-frame`, `zone-marker`.
- Produces:
  - `free-area(t, children, grid: none)` : place des entrées de slot `free`.
  - Widgets de catégorie `slide` :
    - `title` : `title`, `subtitle`, `author`, `date` en `plain`, liés à la clé homonyme de `deck.params` ; `kicker` en `plain`, lié à `organization`.
    - `toc` : `title: plain`.
    - `chapter` : `title: plain`, `subtitle: plain`.
    - `content` : `kicker: plain`, `title: plain`, `layout: choice("default", "columns", "split", "center", "quote")`, `grid: grid(default: (columns: 12, rows: 6))`, `blocks: slot(mode: "flow")`, `elements: slot(mode: "free")`.
    - `end` : `text: plain`.
    - `blank` : `grid: grid(default: (columns: 12, rows: 6))`, `elements: slot(mode: "free")`.
  - Les slides émettent encore `tracked("@…")`, pas de `field()` (plan 2).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn load_registry_base_declares_every_slide_widget() {
    let r = load_registry(&Fonts::default(), None).unwrap();
    for name in ["title", "toc", "chapter", "content", "end", "blank"] {
        asserting(name).that(&r.get(name).map(|w| w.category)).is_equal_to(Some(Category::Slide));
    }
    let title = r.get("title").unwrap();
    assert_that!(title.param("title").and_then(|p| p.bind.as_deref())).is_equal_to(Some("title"));
    assert_that!(title.param("kicker").and_then(|p| p.bind.as_deref())).is_equal_to(Some("organization"));
}

#[test]
fn load_registry_base_matches_its_snapshot() {
    let r = load_registry(&Fonts::default(), None).unwrap();
    insta::assert_json_snapshot!("base_registry", r);
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo nextest run -p oku-slides-render --test widgets`
Expected: FAIL, `title` absent.

- [ ] **Step 3: Add `free-area` to `zone.typ`**

```typst
// Zone de corps libre : place chaque entrée de slot free sur sa cellule de
// grille (résolue dans la taille réelle de la zone) ou sur son cadre, et
// signale la zone à l'app.
#let free-area(t, children, grid: none) = block(width: 100%, height: 100%, layout(sz => {
  let zw = sz.width / 1pt
  let zh = sz.height / 1pt
  let gap = if grid != none { space-of(t, grid.at("gap", default: "lg")) / 1pt } else { 0 }
  if grid != none { place(top + left, zone-marker(t, grid)) }
  for c in children {
    let f = if grid != none and c.cell != none { grid-frame(zw, zh, grid, gap, c.cell) } else { c.frame }
    place(top + left, dx: f.x * 1pt, dy: f.y * 1pt, (c.show)(f))
  }
}))
```

- [ ] **Step 4: Add the slide widgets**

Dans `widgets.typ`, importer `#import "slides.typ": card, slide, title-slide, chapter, toc, end-slide, blank-slide` et `#import "zone.typ": free-area, zone-marker`, puis :

```typst
#let _grid-default = (columns: 12, rows: 6)

#let _title = widget("title", category: "slide",
  params: (
    title: p.plain(bind: "title"),
    subtitle: p.plain(bind: "subtitle"),
    author: p.plain(bind: "author"),
    date: p.plain(bind: "date"),
    kicker: p.plain(bind: "organization"),
  ),
  (args, ctx) => title-slide(title: args.title, subtitle: args.subtitle, author: args.author,
    date: args.date, kicker: args.kicker, id: ctx.id))

#let _toc = widget("toc", category: "slide", params: (title: p.plain()),
  (args, ctx) => toc(title: args.title, id: ctx.id))

#let _chapter = widget("chapter", category: "slide", params: (title: p.plain(), subtitle: p.plain()),
  (args, ctx) => chapter(args.title, subtitle: args.subtitle, id: ctx.id))

#let _content = widget("content", category: "slide",
  params: (
    kicker: p.plain(),
    title: p.plain(),
    layout: p.choice("default", "columns", "split", "center", "quote"),
    grid: p.grid(default: _grid-default),
    blocks: p.slot(mode: "flow"),
    elements: p.slot(mode: "free"),
  ),
  (args, ctx) => {
    let t = ctx.tokens
    // Sans blocs mis en page, la zone de corps est libre, sur sa grille.
    let free = args.blocks.len() == 0
    slide(
      // Slide à blocs : la grille existe aussi (l'app y accroche un bloc
      // dès qu'on le déplace, et la slide passe alors sur la grille).
      zone: if free { none } else { zone-marker(t, args.grid) },
      title: args.title, kicker: args.kicker, layout: args.layout, id: ctx.id,
      // Un graphique dans les blocs occupe la place qui reste sous les autres.
      grow: if free { () } else { args.blocks.map(b => b.kind == "chart") },
      ..if free { (free-area(t, args.elements, grid: args.grid),) } else { args.blocks.map(b => b.body) },
    )
  })

#let _end = widget("end", category: "slide", params: (text: p.plain()),
  (args, ctx) => end-slide(args.text, id: ctx.id))

#let _blank = widget("blank", category: "slide",
  params: (grid: p.grid(default: _grid-default), elements: p.slot(mode: "free")),
  (args, ctx) => blank-slide(free-area(ctx.tokens, args.elements, grid: args.grid), id: ctx.id))

#let registry = (_text, _card, _image, _shape, _chart, _stack, _canvas,
  _title, _toc, _chapter, _content, _end, _blank)
```

- [ ] **Step 5: Turn `render-slide` into an adapter**

Dans `data.typ`, supprimer `_free` et remplacer `render-slide` par :

```typst
// Entrées de slot d'une liste d'éléments v1.
#let _flow(t, list, vars, image-loader) = list.map(b => (kind: b.at("kind", default: "text"),
  id: b.at("id", default: none), body: render-element(t, b, vars, image-loader: image-loader)))
#let _placed(t, list, vars, image-loader) = list.map(e => (kind: e.at("kind", default: "text"),
  id: e.at("id", default: none), frame: e.at("frame", default: none), cell: e.at("cell", default: none),
  show: f => render-element(t, e + (frame: f), vars, image-loader: image-loader)))

// Arguments d'une slide v1 pour son widget. MVP: t-270, plan 2 — remplacé
// par le rendu générique du modèle v2.
#let _slide-args(t, s, vars, image-loader) = {
  let kind = s.at("kind", default: "content")
  let opt(key) = s.at(key, default: auto)
  if kind == "title" { (title: opt("title"), subtitle: opt("subtitle"), author: opt("author"), date: opt("date"), kicker: auto) }
  else if kind == "chapter" { (title: s.title, subtitle: s.at("subtitle", default: none)) }
  else if kind == "toc" { (title: opt("title")) }
  else if kind == "end" { (text: interp(s.at("text", default: ""), vars)) }
  else if kind == "blank" {
    (grid: s.at("grid", default: default-grid), elements: _placed(t, s.at("elements", default: ()), vars, image-loader))
  } else {
    let free = "elements" in s or "grid" in s or s.at("blocks", default: ()).len() == 0
    (
      kicker: s.at("kicker", default: none),
      title: s.at("title", default: none),
      layout: s.at("layout", default: "default"),
      grid: s.at("grid", default: default-grid),
      blocks: if free { () } else { _flow(t, s.at("blocks", default: ()), vars, image-loader) },
      elements: if free { _placed(t, s.at("elements", default: ()), vars, image-loader) } else { () },
    )
  }
}

#let render-slide(s, image-loader: none) = context {
  let c = config()
  let t = c.tokens
  let kind = s.at("kind", default: "content")
  let name = if kind in ("title", "chapter", "toc", "end", "blank") { kind } else { "content" }
  let ctx = (id: s.at("id", default: none), tokens: t, vars: c.vars, style: (:), framed: false, load: none)
  (_widgets.at(name).render)(_slide-args(t, s, c.vars, image-loader), ctx)
}
```

Retirer de l'import de `slides.typ` dans `data.typ` les fonctions devenues inutiles (`slide`, `title-slide`, `chapter`, `toc`, `end-slide`, `blank-slide`, `card`). Garder `deck`.

- [ ] **Step 6: Run all render tests, record the registry snapshot**

Run: `INSTA_UPDATE=always cargo nextest run -p oku-slides-render --test widgets` puis `cargo nextest run -p oku-slides-render`
Expected: PASS, instantané doré inchangé. Relire `widgets__base_registry.snap` : 13 widgets, paramètres dans l'ordre déclaré, aucun `label`.

- [ ] **Step 7: Lint and commit**

```bash
cargo fmt --all && cargo clippy -p oku-slides-render --all-targets -- -D warnings
```
```bash
git add typst/oku-slides/src crates/oku-slides-render/tests
```
```bash
git commit -m "feat(render) : slides déclarées en widgets, render-slide devient un adaptateur v1 (t-270)"
```

---

### Task 8: Widgets d'un pack et cache des registres

**Files:**
- Modify: `crates/oku-slides-render/src/registry.rs` (`Registries`)
- Modify: `crates/oku-slides-render/src/lib.rs` (export)
- Modify: `crates/oku-slides-render/Cargo.toml` (`sha2.workspace = true` en dépendance)
- Modify: `typst/oku-slides/lib.typ` (exports pour les packs)
- Modify: `typst/oku-slides/README.md` (section « Widgets »)
- Modify: `crates/oku-slides-render/tests/widgets.rs`

**Interfaces:**
- Consumes: `load_registry`, `PackWidgets` (tâche 4).
- Produces:
  - `oku_slides_render::Registries::new(fonts: Fonts) -> Registries`.
  - `Registries::get(&mut self, pack: Option<&PackWidgets>) -> Result<Arc<WidgetRegistry>, RegistryError>`.
  - `lib.typ` exporte `widget`, `field`, `by-name`, `params` (module, à importer `as p`), `free-area`, `zone-marker`, `grid-frame`, `default-grid`, `_paragraphs`, `_text-args`, `_text-over`.

- [ ] **Step 1: Write the failing tests**

```rust
use std::sync::Arc;

use oku_slides_core::WidgetErrorKind;
use oku_slides_render::{PackWidgets, Registries, RegistryErrorKind};

const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let quote = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
#let registry = (quote,)
"#;

#[test]
fn load_registry_with_pack_adds_its_widgets_to_the_base() {
    let pack = PackWidgets::new("citations", QUOTE).unwrap();
    let r = load_registry(&Fonts::default(), Some(&pack)).unwrap();
    assert_that!(r.get("quote").map(|w| w.params.len())).is_equal_to(Some(2));
    assert_that!(r.get("card")).is_some();
}

#[test]
fn load_registry_pack_breaking_card_returns_schema_error() {
    let src = r#"#import "/oku-slides/lib.typ": widget, params as p
#let registry = (widget("card", params: (title: p.plain()), (args, ctx) => []),)
"#;
    let pack = PackWidgets::new("casse", src).unwrap();
    let err = load_registry(&Fonts::default(), Some(&pack)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RegistryErrorKind::Schema);
    assert_that!(err.widget_error().map(|e| e.kind())).is_equal_to(Some(WidgetErrorKind::IncompatibleOverride));
}

#[test]
fn load_registry_pack_that_does_not_compile_returns_its_diagnostics() {
    let pack = PackWidgets::new("casse", "#let registry = (").unwrap();
    let err = load_registry(&Fonts::default(), Some(&pack)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RegistryErrorKind::Compile);
    assert_that!(err.diagnostics().iter().any(|d| d.place.as_deref().is_some_and(|p| p.contains("packs/casse")))).is_true();
}

#[test]
fn pack_widgets_new_with_a_path_like_id_returns_bad_pack_id() {
    for id in ["", "../x", "A", "a/b", "é"] {
        asserting(id).that(&PackWidgets::new(id, "").map(|_| ()).map_err(|e| e.kind())).is_equal_to(Err(RegistryErrorKind::BadPackId));
    }
}

#[test]
fn registries_get_same_pack_twice_reuses_the_registry() {
    let mut regs = Registries::new(Fonts::default());
    let pack = PackWidgets::new("citations", QUOTE).unwrap();
    let a = regs.get(Some(&pack)).unwrap();
    let b = regs.get(Some(&pack)).unwrap();
    assert_that!(Arc::ptr_eq(&a, &b)).is_true();
    let base = regs.get(None).unwrap();
    assert_that!(Arc::ptr_eq(&a, &base)).is_false();
}

#[test]
fn registries_get_changed_pack_source_reloads() {
    let mut regs = Registries::new(Fonts::default());
    let a = regs.get(Some(&PackWidgets::new("citations", QUOTE).unwrap())).unwrap();
    let changed = QUOTE.replace("source: p.plain()", "source: p.plain(), note: p.plain()");
    let b = regs.get(Some(&PackWidgets::new("citations", changed).unwrap())).unwrap();
    assert_that!(b.get("quote").map(|w| w.params.len())).is_equal_to(Some(3));
    assert_that!(Arc::ptr_eq(&a, &b)).is_false();
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo nextest run -p oku-slides-render --test widgets`
Expected: FAIL à la compilation, `Registries` introuvable, puis `params` non exporté par `lib.typ`.

- [ ] **Step 3: Export the widget system from `lib.typ`**

Ajouter à `typst/oku-slides/lib.typ` :

```typst
// Widgets : pour déclarer ceux d'un pack (`#import "/oku-slides/lib.typ": widget, field, params as p`).
#import "src/params.typ" as params
#import "src/widget.typ": widget, field, by-name
#import "src/zone.typ": free-area, zone-marker, grid-frame, default-grid
#import "src/content.typ": _paragraphs, _text-args, _text-over
```

- [ ] **Step 4: Write `Registries`**

Dans `registry.rs` :

```rust
use std::collections::HashMap;
use std::sync::Arc;

use sha2::{Digest, Sha256};

/// Registres déjà lus, par pack : une sonde ne compile qu'une fois par
/// source de pack. La bibliothèque est dans le binaire : constante.
pub struct Registries {
    fonts: Fonts,
    cache: HashMap<Option<[u8; 32]>, Arc<WidgetRegistry>>,
}

impl std::fmt::Debug for Registries {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Registries").field("cached", &self.cache.len()).finish_non_exhaustive()
    }
}

impl Registries {
    pub fn new(fonts: Fonts) -> Self {
        Self { fonts, cache: HashMap::new() }
    }

    /// Le registre de la base, surchargé par `pack`.
    pub fn get(&mut self, pack: Option<&PackWidgets>) -> Result<Arc<WidgetRegistry>, RegistryError> {
        let key = pack.map(|p| {
            let mut h = Sha256::new();
            h.update(p.id().as_bytes());
            h.update([0]);
            h.update(p.source().as_bytes());
            <[u8; 32]>::from(h.finalize())
        });
        if let Some(r) = self.cache.get(&key) {
            return Ok(r.clone());
        }
        let r = Arc::new(load_registry(&self.fonts, pack)?);
        self.cache.insert(key, r.clone());
        Ok(r)
    }
}
```

Exporter `Registries` dans `lib.rs` (`pub use registry::{PackWidgets, Registries, load_registry};`). Ajouter `sha2.workspace = true` aux `[dependencies]` de `oku-slides-render`.

- [ ] **Step 5: Document widgets for pack authors**

Ajouter à `typst/oku-slides/README.md` une section « Widgets » :
- le rôle de `widgets.typ` dans un pack (tableau `registry`) ;
- l'import `#import "/oku-slides/lib.typ": widget, field, params as p` ;
- la table des descripteurs `p.*` de la spec ;
- la forme de `args` et de `ctx` (en-tête de `widget.typ`) ;
- les entrées de slot `flow` et `free` ;
- `field()` et l'étiquette `<oku-slides-field>` ;
- la règle de surcharge d'un widget de base.

Reprendre l'exemple `quote` du test.

- [ ] **Step 6: Run the whole workspace**

Run: `cargo nextest run --workspace --exclude oku-slides-app && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS, instantané doré inchangé. Rien hors de `core`, `render` et `i18n` n'a changé.

- [ ] **Step 7: Commit**

```bash
git add crates/oku-slides-render typst/oku-slides/lib.typ typst/oku-slides/README.md
```
```bash
git commit -m "feat(render) : widgets d'un pack, registres en cache par empreinte (t-270)"
```

---

## Après le plan 1

- Le plan 2 (bascule v2) s'écrit sur cette base : modèle `{kind, params}`, replica Loro guidé par le schéma, ops (`SetParam`, `SetKind`, `SetDeckParam`, `AddElement` par slot), validation, rendu générique (l'adaptateur v1 de `data.typ` disparaît, les slides passent à `field()`), session, MCP, agent, UI, packs au format dossier, fixtures, i18n des libellés de widgets.
- Le test doré sera alors retiré : le format du deck change, et les tests de rendu du plan 2 prennent le relais.
