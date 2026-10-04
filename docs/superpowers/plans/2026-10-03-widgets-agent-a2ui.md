# Édition de widgets avec l'agent dans le dock A2UI — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Les réglages du modèle et les paramètres d'essai d'un widget vivent dans le dock A2UI de l'agent ; l'agent voit et règle ces essais, et l'éditeur de widgets suit ses écritures.

**Architecture:** Les essais sont un état de session (`Mode::Templates`), lus par l'aperçu, le contexte de l'agent et les réglages de l'hôte. La grille passe par une op du deck `set_grid`, qui recadre les éléments en Rust à partir de la grille résolue par le registre. En phase B, les réglages de l'hôte deviennent des `ShowParams` rendus par le constructeur de `show_params`, au dock de t-416, avec un nouveau composant A2UI de texte riche.

**Tech Stack:** Rust (oku-slides-core, -session, -mcp, -a2ui, -i18n ; snafu, speculoos, rstest, nextest), TypeScript/React (Bun test, happy-dom, CodeMirror), Playwright (e2e).

**Spec:** `docs/superpowers/specs/2026-10-03-widgets-agent-a2ui-design.md` (t-423)

## Global Constraints

- Phase A (tâches 1 à 5) ne dépend de rien. Phase B (tâches 6 à 9) exige la tâche 11 de t-416 (`docs/superpowers/plans/2026-10-03-agent-a2ui.md`, dock et catalogue v3) fusionnée.
- Chaque tâche se termine sur un dépôt qui compile, passe ses tests et `clippy -D warnings` : une fonction arrive dans la tâche qui l'utilise la première (pas de code mort entre deux tâches, pas d'`#[allow]`).
- Valeurs d'essai : par widget, en mémoire, dans `Mode::Templates`, jamais écrites dans le pack ; remises à zéro à l'entrée en mode modèles, au changement de pack et à la reprise d'un onglet. Un essai n'est pas un changement du deck et n'écrit pas les documents de reprise.
- Champs de l'hôte adressés par clé (`name`, `description`, `columns`, `rows`, ou le nom du paramètre), jamais par position.
- Un seul constructeur A2UI de réglages : `forms::params` (via `forms::params_with`).
- Les frappes non envoyées, en cours d'envoi ou non enregistrées gagnent sur une écriture de l'agent ; une suppression par l'agent gagne sur les frappes en attente du fichier supprimé, écritures déjà en file comprises.
- Grille : 1 à 24 pistes (`GRID_TRACKS` du cœur, une seule définition).
- Limites d'un essai : `plain` ≤ 4096 octets ; `rich` ≤ 64 paragraphes et ≤ 16384 octets de JSON.
- i18n : toute clé ajoutée ou retirée l'est dans les 20 langues (`ar bn de en es fr hi it ja ko nl pl pt ru th tr uk vi zh-Hans zh-Hant`). Le plan donne fr et en ; l'implémenteur traduit les 18 autres (même ton, mêmes variables). Contrôles : `cargo nextest run -p oku-slides-i18n` (session.ftl, agent.ftl) et `cd ui && bun test src/i18n-keys.test.ts` (ui.ftl).
- Fichiers : 500 lignes visées, 1000 au plus (CI). `ui/src/App.tsx` et `crates/oku-slides-session/src/lib.rs` ne reçoivent que du câblage. Le plan cite des symboles ; un numéro de ligne n'est qu'un repère.
- Lints : clippy pedantic n'est pas activé ; `collapsible_if` l'est (édition 2024 : écrire `if a && let Some(x) = …`). `cargo clippy --workspace --exclude oku-slides-app --all-targets -- -D warnings` à chaque tâche Rust.
- Pas de `.unwrap()`/`.expect()` ni d'indexation hors tests ; erreurs Snafu avec backtrace, localisées (Fluent), source gardée.
- Tests : `<unit>_<scenario>_<expected>`, un comportement par test, rstest pour les cas.
- Visibilité minimale : `pub(crate)` sauf ce qu'un autre crate appelle.
- `ARCHITECTURE.md` (≤ 50 lignes) mis à jour dans la tâche qui change le paquet, en réécrivant une ligne existante.
- Commits : conventionnels, en français, `(t-423)` ; jamais de ligne `Co-Authored-By`. `git add` dans un appel à part, puis `git diff --cached --stat` juste avant `git commit` (d'autres sessions travaillent sur `main`). Après chaque tâche, `oku cli task done --project slider --id <sous-tâche>`.
- Avant de lancer l'app ou l'e2e : `OKU_SLIDES_PATHS__DATA` et `OKU_SLIDES_PATHS__DECKS` dans un dossier temporaire.

## Sous-tâches Oku (t-423)

| Tâche | Sous-tâche |
|---|---|
| 1 | t-447 |
| 2 | t-438 |
| 3 | t-439 |
| 4 | t-440 |
| 5 | t-441 |
| 6 | t-442 |
| 7 | t-443 |
| 8 | t-444 |
| 9 | t-445 |

## Carte des fichiers

| Fichier | Rôle | Tâche |
|---|---|---|
| `crates/oku-slides-core/src/model.rs` | `Slide::resolved_grid`, défaut déclaré | 1 |
| `crates/oku-slides-core/src/ops/grid.rs` (+ `grid_tests.rs`), `ops/{mod,value}.rs`, `validate.rs` | `rescale_cell`, `set_grid`, `set_cell` et validation sur la grille résolue | 1 |
| `crates/oku-slides-core/tests/grid.rs` | tests d'intégration de `set_grid` | 1 |
| `ui/src/a2ui/panelActions.ts`, `ui/src/grid.ts`, `ui/src/App.tsx`, `crates/oku-slides-session/src/panel.rs` | `set_grid` envoyé tel quel ; `rescaleCell` et `GRID_MIN/MAX` supprimés | 1 |
| `crates/oku-slides-session/src/widget_trials.rs` (+ tests), `widget_params.rs` (+ tests) | brouillon (essais), `set_widget_trials` | 2 |
| `crates/oku-slides-session/src/{widgets_edit,widget_preview}.rs` | `valid_schema`, élagage à l'écriture, aperçu des essais | 2 |
| `crates/oku-slides-session/src/agent/context.rs`, `widgets_edit.rs`, `widget_trials.rs` | `AgentUi.widget`, contexte, `set_widget_by_agent`, marques | 3 |
| `crates/oku-slides-mcp/src/{widget_tools,hub,tools,handler}.rs` | `set_widget_trial`, `WidgetTouched` | 4 |
| `ui/src/{useWidgetFiles,useTemplateWidgets,widgetTabs}.ts`, `WidgetTab.tsx`, `WidgetPane.tsx`, `App.tsx` | l'éditeur suit l'agent ; l'aperçu suit les essais | 5 |
| `crates/oku-slides-a2ui/src/message.rs`, `ui/src/a2ui/{catalog.tsx,RichTextField.tsx}`, `ui/src/markdownEditing.ts`, `agent/forms.rs` | composant `RichTextField`, champ `rich` | 6 |
| `crates/oku-slides-session/src/{panel.rs,widget_params.rs,request.rs,lib.rs}` | réglages de l'hôte (session, en plus de `panel`) | 7 |
| `ui/src/{useHostSettings.ts,App.tsx,api.ts}` ; suppression de `panel` | réglages de l'hôte au dock | 8 |
| `ui/e2e/{panel,widgets}.cjs` | bout en bout | 9 |

---

## Phase A — indépendante du dock

### Task 1: Op `set_grid` en Rust, sur la grille résolue par le registre

**Files:**
- Create: `crates/oku-slides-core/src/ops/grid.rs`, `crates/oku-slides-core/src/ops/grid_tests.rs`, `crates/oku-slides-core/tests/grid.rs`
- Modify: `crates/oku-slides-core/src/model.rs` (`impl Slide`, à côté de `fn grid`), `crates/oku-slides-core/src/ops/mod.rs` (enum `Op`, `Op::apply` et son bras `SetCell`, `mod`, ré-exports), `crates/oku-slides-core/src/ops/value.rs` (`GRID_TRACKS` en `pub`), `crates/oku-slides-core/src/validate.rs` (`let grid = s.grid();` dans `validate`), `crates/oku-slides-core/src/lib.rs`, `crates/oku-slides-core/tests/ops.rs` (`ops_schema_generates_schema_naming_ops`), `crates/oku-slides-core/ARCHITECTURE.md`
- Modify: `crates/oku-slides-session/src/widget_preview.rs` (`preview_deck`), `crates/oku-slides-session/src/panel.rs` (doc d'en-tête, `GRID_MIN`/`GRID_MAX`), `crates/oku-slides-mcp/src/tools.rs` (description de `apply`), `crates/oku-slides-i18n/src/locales/*/agent.ftl` (`agent-instructions`)
- Modify: `ui/src/a2ui/panelActions.ts`, `ui/src/a2ui/panelActions.test.ts`, `ui/src/grid.ts` (`rescaleCell`), `ui/src/App.tsx` (`slideGrid`, appel de `panelActionOps`)

**Interfaces:**
- Produces:
  - `Op::SetGrid { slide: String, columns: Option<u32>, rows: Option<u32> }` (JSON `{"op":"set_grid","slide":"…","columns":6}`).
  - `impl Slide { pub fn resolved_grid(&self, registry: &WidgetRegistry) -> Grid; pub fn declared_grid(&self, registry: &WidgetRegistry) -> Option<Value> }` : paramètre `grid`, sinon défaut du paramètre `grid` (type `grid`) du widget de la slide, sinon 12 × 6 ; `declared_grid` donne ce défaut tel qu'écrit dans le schéma.
  - `pub const GRID_TRACKS: RangeInclusive<u32> = 1..=24` (ré-exporté à la racine).
  - `pub(crate) fn rescale_cell(cell: Cell, from: &Grid, to: &Grid) -> Cell`.
  - `panelActionOps(action, slide)` (3ᵉ argument retiré).

- [ ] **Step 1: Tests unitaires (échouent)** — `crates/oku-slides-core/src/ops/grid_tests.rs` :

```rust
use rstest::rstest;
use speculoos::prelude::*;

use super::rescale_cell;
use crate::{Cell, Grid};

fn grid(columns: u32, rows: u32) -> Grid {
    Grid { columns, rows, gap: None }
}

fn cell(col: u32, row: u32, cols: u32, rows: u32) -> Cell {
    Cell { col, row, cols, rows }
}

// Valeurs de l'ancien `rescaleCell` (ui/src/grid.ts, Math.round).
#[rstest]
#[case::halving((12, 6), (6, 6), cell(6, 3, 6, 3), cell(3, 3, 3, 3))]
#[case::single_cell((12, 6), (1, 1), cell(6, 3, 6, 3), cell(0, 0, 1, 1))]
#[case::doubling((4, 2), (8, 4), cell(1, 0, 2, 1), cell(2, 0, 4, 2))]
#[case::fraction((12, 6), (5, 6), cell(5, 0, 2, 1), cell(2, 0, 1, 1))]
#[case::tie_rounds_up((4, 6), (6, 6), cell(1, 0, 1, 1), cell(2, 0, 1, 1))]
#[case::span_never_collapses((12, 6), (3, 6), cell(0, 0, 1, 1), cell(0, 0, 1, 1))]
#[case::last_track((12, 6), (6, 6), cell(11, 0, 1, 1), cell(5, 0, 1, 1))]
fn rescale_cell_valid_cell_keeps_proportions(
    #[case] from: (u32, u32),
    #[case] to: (u32, u32),
    #[case] before: Cell,
    #[case] after: Cell,
) {
    assert_that!(rescale_cell(before, &grid(from.0, from.1), &grid(to.0, to.1))).is_equal_to(after);
}

#[test]
fn rescale_cell_overflowing_cell_lands_on_the_last_track() {
    let c = rescale_cell(cell(u32::MAX, 0, 2, 1), &grid(12, 6), &grid(6, 6));
    assert_that!(c).is_equal_to(cell(5, 0, 1, 1));
}

#[test]
fn rescale_cell_from_an_empty_grid_stays_inside() {
    let c = rescale_cell(cell(0, 0, 1, 1), &grid(0, 0), &grid(6, 6));
    assert_that!(c).is_equal_to(cell(0, 0, 6, 6));
}

#[test]
fn rescale_cell_to_a_huge_grid_does_not_overflow() {
    let c = rescale_cell(cell(1, 0, 1, 1), &grid(2, 1), &grid(u32::MAX, 1));
    assert_that!(c.col + c.cols <= u32::MAX).is_true();
}
```

- [ ] **Step 2: Tests d'intégration (échouent)** — `crates/oku-slides-core/tests/grid.rs` (nouveau binaire de test : `tests/ops.rs` approche déjà 800 lignes) :

```rust
//! L'op `set_grid` et la grille résolue par le registre.

mod common;

use common::registry;
use oku_slides_core::*;
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

fn fixture() -> Deck {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");
    Deck::from_json(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// Le registre de base, plus un widget de slide `quatre` dont la grille par défaut est 4 × 3.
fn with_quatre() -> WidgetRegistry {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/base-registry.json");
    let mut probe: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    probe["pack"] = json!([{
        "name": "quatre", "label": null, "category": "slide", "params": [
            { "name": "grid", "label": null, "type": "grid", "default": { "columns": 4, "rows": 3, "gap": "sm" }, "bind": null },
            { "name": "elements", "label": null, "type": "slot", "mode": "free", "accepts": null, "default": null, "bind": null }
        ]
    }]);
    WidgetRegistry::from_probe(probe).unwrap()
}

/// Un deck d'une slide `quatre` dont l'élément `e` occupe `cell`.
fn quatre_deck(cell: Cell) -> Deck {
    let mut deck = fixture();
    let mut slide = Slide::new("s-4", "quatre");
    let mut e = Element::new("e", "text");
    e.cell = Some(cell);
    slide.params.set("elements", ParamValue::Slot(vec![e]));
    deck.slides.push(slide);
    deck
}

fn set_grid(slide: &str, columns: Option<u32>, rows: Option<u32>) -> Op {
    Op::SetGrid { slide: slide.into(), columns, rows }
}

fn cell_of(deck: &Deck, slide: &str) -> Option<Cell> {
    deck.slide(slide).unwrap().locate("e").unwrap().element.cell
}

#[test]
fn resolved_grid_without_param_reads_the_widget_default() {
    let deck = quatre_deck(Cell { col: 0, row: 0, cols: 1, rows: 1 });
    let g = deck.slide("s-4").unwrap().resolved_grid(&with_quatre());
    assert_that!((g.columns, g.rows)).is_equal_to((4, 3));
}

#[test]
fn set_grid_from_the_widget_default_rescales_cells() {
    let mut deck = quatre_deck(Cell { col: 2, row: 1, cols: 2, rows: 1 });
    apply_all(&mut deck, vec![set_grid("s-4", Some(8), Some(6))], &with_quatre()).unwrap();
    assert_that!(cell_of(&deck, "s-4")).is_equal_to(Some(Cell { col: 4, row: 2, cols: 4, rows: 2 }));
}

#[test]
fn set_grid_from_the_widget_default_keeps_its_gap() {
    let mut deck = quatre_deck(Cell { col: 0, row: 0, cols: 1, rows: 1 });
    apply_all(&mut deck, vec![set_grid("s-4", Some(8), None)], &with_quatre()).unwrap();
    let grid = deck.slide("s-4").unwrap().params.value("grid").cloned();
    assert_that!(grid).is_equal_to(Some(json!({ "columns": 8, "rows": 3, "gap": "sm" })));
}

#[test]
fn set_cell_beyond_the_widget_default_is_refused() {
    let mut deck = quatre_deck(Cell { col: 0, row: 0, cols: 1, rows: 1 });
    let op = Op::SetCell { slide: "s-4".into(), element: "e".into(), cell: Some(Cell { col: 4, row: 0, cols: 1, rows: 1 }) };
    assert_that!(apply_all(&mut deck, vec![op], &with_quatre())).is_err();
}

#[test]
fn validate_cell_inside_the_widget_default_reports_nothing() {
    let deck = quatre_deck(Cell { col: 3, row: 2, cols: 1, rows: 1 });
    let issues = validate(&deck, &with_quatre(), None);
    assert_that!(issues.iter().any(|i| format!("{i:?}").contains("cell"))).is_false();
}

#[test]
fn set_grid_keeps_an_explicit_gap() {
    let mut deck = quatre_deck(Cell { col: 0, row: 0, cols: 1, rows: 1 });
    let reg = with_quatre();
    let grid = json!({ "columns": 4, "rows": 3, "gap": "lg" });
    let set = Op::SetParam { target: Target { slide: "s-4".into(), element: None }, param: "grid".into(), value: grid };
    apply_all(&mut deck, vec![set, set_grid("s-4", Some(6), None)], &reg).unwrap();
    let grid = deck.slide("s-4").unwrap().params.value("grid").cloned();
    assert_that!(grid).is_equal_to(Some(json!({ "columns": 6, "rows": 3, "gap": "lg" })));
}

#[rstest]
#[case::zero(Some(0))]
#[case::too_many(Some(25))]
fn set_grid_out_of_range_is_refused(#[case] columns: Option<u32>) {
    let mut deck = quatre_deck(Cell { col: 0, row: 0, cols: 1, rows: 1 });
    let before = deck.clone();
    let err = apply_all(&mut deck, vec![set_grid("s-4", columns, None)], &with_quatre()).unwrap_err();
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::InvalidValue);
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn set_grid_on_an_unknown_slide_is_refused() {
    let mut deck = fixture();
    let err = apply_all(&mut deck, vec![set_grid("absente", Some(6), None)], &registry()).unwrap_err();
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::SlideNotFound);
}

#[test]
fn set_grid_to_the_same_grid_changes_nothing() {
    let mut deck = quatre_deck(Cell { col: 1, row: 1, cols: 1, rows: 1 });
    let before = deck.clone();
    apply_all(&mut deck, vec![set_grid("s-4", Some(4), Some(3))], &with_quatre()).unwrap();
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn set_grid_reads_from_json() {
    let op: Op = serde_json::from_value(json!({ "op": "set_grid", "slide": "s-4", "rows": 4 })).unwrap();
    assert_that!(op).is_equal_to(set_grid("s-4", None, Some(4)));
}
```

(`apply_all` renvoie `TransactionError`, qui a `op_error()` ; `validate` et `Issue` sont ré-exportés à la racine — sinon importer `oku_slides_core::validate::validate`.) Dans `tests/ops.rs`, `ops_schema_generates_schema_naming_ops` : remplacer `assert_that!(s.as_str()).does_not_contain("\"set_grid\"");` par

```rust
    // t-423 : `set_grid` redevient une op (grille et recadrage des éléments).
    assert_that!(s.as_str()).contains("\"set_grid\"");
```

Run: `cargo nextest run -p oku-slides-core` — Expected: échec de compilation.

- [ ] **Step 3: La grille résolue** — `model.rs`, dans `impl Slide`, après `fn grid` :

```rust
    /// Grille d'une slide : paramètre, défaut déclaré par son widget, sinon 12 × 6.
    pub fn resolved_grid(&self, registry: &WidgetRegistry) -> Grid {
        if self.params.value("grid").is_some() {
            return self.grid();
        }
        self.declared_grid(registry).and_then(|v| serde_json::from_value(v).ok()).unwrap_or_else(|| self.grid())
    }

    /// Défaut du paramètre `grid` déclaré par le widget de la slide, tel qu'écrit.
    pub fn declared_grid(&self, registry: &WidgetRegistry) -> Option<Value> {
        registry
            .get(&self.kind)
            .and_then(|w| w.param("grid"))
            .filter(|p| matches!(p.ty, ParamType::Grid))
            .and_then(|p| p.default.clone())
            .filter(|v| v.is_object())
    }
```

(importer `WidgetRegistry`, `ParamType` dans `model.rs` s'ils n'y sont pas.)

- [ ] **Step 4: L'op** — `ops/value.rs` : `/// Pistes d'une grille : 1 à 24 colonnes ou lignes. pub const GRID_TRACKS: …`. `crates/oku-slides-core/src/ops/grid.rs` :

```rust
//! Changement de grille d'une slide (`set_grid`).
//!
//! Les cellules des éléments des slots libres gardent leurs proportions ;
//! les cadres ne bougent pas.

use serde_json::{Map, Value};

use super::{GRID_TRACKS, Target, set_param, slide_mut, slide_ref};
use crate::error::{InvalidValueSnafu, OpError};
use crate::{Cell, Deck, Grid, ParamType, SlotMode, WidgetRegistry};

/// Nouvelle place de `cell` quand la grille passe de `from` à `to`.
///
/// Arrondi au plus proche (½ vers le haut) ; toujours dans la grille.
pub(crate) fn rescale_cell(cell: Cell, from: &Grid, to: &Grid) -> Cell {
    let (col, cols) = fit(cell.col, cell.cols, from.columns, to.columns);
    let (row, rows) = fit(cell.row, cell.rows, from.rows, to.rows);
    Cell { col, row, cols, rows }
}

/// Début et longueur d'une plage de `n` pistes ramenée à `m` pistes.
fn fit(start: u32, len: u32, n: u32, m: u32) -> (u32, u32) {
    let (n, m) = (u128::from(n.max(1)), u128::from(m.max(1)));
    // round(x · m / n) en entiers : ⌊(2xm + n) / 2n⌋ ; u128 : aucun dépassement.
    let scale = |x: u128| (2 * x * m + n) / (2 * n);
    let s = scale(u128::from(start)).min(m - 1);
    let e = scale(u128::from(start) + u128::from(len)).clamp(s + 1, m);
    // s < e ≤ m ≤ u32::MAX.
    (u32::try_from(s).unwrap_or(0), u32::try_from(e - s).unwrap_or(1))
}

/// Applique `set_grid` : la grille de `slide`, puis les cellules de ses slots libres.
pub(super) fn set_grid(
    deck: &mut Deck,
    registry: &WidgetRegistry,
    slide: &str,
    columns: Option<u32>,
    rows: Option<u32>,
) -> Result<(), OpError> {
    let s = slide_ref(deck, slide)?;
    let from = s.resolved_grid(registry);
    let to = Grid { columns: columns.unwrap_or(from.columns), rows: rows.unwrap_or(from.rows), gap: None };
    if !GRID_TRACKS.contains(&to.columns) || !GRID_TRACKS.contains(&to.rows) {
        return Err(InvalidValueSnafu { field: "grid", reason: "1 to 24 columns and rows" }.build().into());
    }
    if (to.columns, to.rows) == (from.columns, from.rows) {
        return Ok(());
    }
    let free: Vec<String> = registry
        .get(&s.kind)
        .map(|w| {
            w.params
                .iter()
                .filter(|p| matches!(p.ty, ParamType::Slot { mode: SlotMode::Free, .. }))
                .map(|p| p.name.clone())
                .collect()
        })
        .unwrap_or_default();
    // Gouttière gardée : celle du paramètre, sinon celle du défaut déclaré.
    let mut value = match s.params.value("grid").cloned().or_else(|| s.declared_grid(registry)) {
        Some(Value::Object(m)) => m,
        _ => Map::new(),
    };
    value.insert("columns".into(), to.columns.into());
    value.insert("rows".into(), to.rows.into());
    set_param(deck, registry, &Target { slide: slide.to_owned(), element: None }, "grid", Value::Object(value))?;
    // `slots_mut` ne crée aucun slot : seuls ceux qui existent sont parcourus.
    for (name, elements) in slide_mut(deck, slide)?.params.slots_mut() {
        if !free.iter().any(|f| f == name) {
            continue;
        }
        for e in elements.iter_mut() {
            if let Some(c) = e.cell {
                e.cell = Some(rescale_cell(c, &from, &to));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "grid_tests.rs"]
mod grid_tests;
```

(Le message `"1 to 24 columns and rows"` est celui du contrôle existant de `ops/value.rs` : une seule formulation.) `ops/mod.rs` : `mod grid;` ; `pub use value::{GRID_TRACKS, param_value};` ; après la variante `SetCell` :

```rust
    /// Change la grille d'une slide et recadre les éléments de ses slots libres.
    SetGrid {
        slide: String,
        /// Colonnes, de 1 à 24 ; absent : inchangé.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        columns: Option<u32>,
        /// Lignes, de 1 à 24 ; absent : inchangé.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rows: Option<u32>,
    },
```

dans `Op::apply`, après le bras `SetCell` : `Op::SetGrid { slide, columns, rows } => grid::set_grid(deck, registry, &slide, columns, rows),` ; dans le bras `SetCell`, `c.fits(&s.grid())` devient `c.fits(&s.resolved_grid(registry))`. `validate.rs`, fonction `validate` : `let grid = s.grid();` devient `let grid = s.resolved_grid(registry);`. `lib.rs` du cœur : `GRID_TRACKS` dans les ré-exports de `ops`. `ARCHITECTURE.md` du cœur : réécrire la ligne « `Op` + `apply_all` (`ops/mod.rs`) » en « `Op` + `apply_all` (`ops/mod.rs`) ; `ops/grid.rs` : `set_grid`, sur la grille résolue (`Slide::resolved_grid`). ».

- [ ] **Step 5: Vérifier** — Run: `cargo nextest run -p oku-slides-core && cargo clippy -p oku-slides-core --all-targets -- -D warnings` — Expected: PASS.

- [ ] **Step 6: Consommateurs Rust.**
  - `widget_preview.rs`, `preview_deck`, bras `Category::Element` : `let grid = slide.grid();` devient `let grid = slide.resolved_grid(&self.registry);`.
  - `panel.rs` : supprimer `GRID_MIN`/`GRID_MAX` et utiliser `Some(i64::from(*GRID_TRACKS.start()))` / `Some(i64::from(*GRID_TRACKS.end()))` dans `grid_card` ; la doc d'en-tête dit désormais « `set_grid` (contexte `slide`, `columns`, `rows`) ; l'interface en fait l'op `set_grid` ».
  - `crates/oku-slides-mcp/src/tools.rs`, description de `apply` : ajouter « set_grid changes a slide's grid (columns, rows; placed elements follow); » après la phrase sur `set_param`.
  - `agent-instructions` (20 langues), en fin de valeur — fr : « Pour changer la grille d'une slide, utilise l'op set_grid : les éléments placés suivent. » ; en : “To change a slide's grid, use the set_grid op: placed elements follow.”
  Run: `cargo nextest run -p oku-slides-session -p oku-slides-mcp -p oku-slides-i18n` — Expected: PASS.

- [ ] **Step 7: UI — test d'abord.** `ui/src/a2ui/panelActions.test.ts` : supprimer la constante `grid`, retirer le 3ᵉ argument de tous les appels, et remplacer les deux tests `set_grid` par :

```ts
  test("set_grid envoie l'op set_grid : la session recadre les éléments", () => {
    expect(panelActionOps(action("set_grid", { slide: "t-1", columns: 6, rows: 6 }), slide)).toEqual({
      label: t("ui-undo-grid", { columns: 6, rows: 6 }),
      ops: [{ op: "set_grid", slide: "t-1", columns: 6, rows: 6 }],
    });
  });

  test("set_grid sans nombres valides ne fait rien", () => {
    expect(panelActionOps(action("set_grid", { slide: "t-1", columns: null, rows: 6 }), slide)).toBeNull();
  });
```

Run: `cd ui && bun test src/a2ui/panelActions.test.ts` — Expected: FAIL.

- [ ] **Step 8: UI — implémenter.** `panelActions.ts` : signature `panelActionOps(action: PanelAction, slide: SlideView["slide"])`, imports `slot` et `rescaleCell` retirés, dernière phrase de l'en-tête remplacée par « La session recadre les éléments accrochés (`set_grid`). », et :

```ts
    case "set_grid": {
      const { columns, rows } = ctx;
      if (typeof columns !== "number" || typeof rows !== "number") return null;
      return { label: t("ui-undo-grid", { columns, rows }), ops: [{ op: "set_grid", slide: slide.id, columns, rows }] };
    }
```

`App.tsx` : supprimer `const slideGrid = …` (et son commentaire ; seul usage : l'appel ci-dessous) et écrire `panelActionOps(action, view.slide)`. `grid.ts` : supprimer `rescaleCell` et son commentaire ; `clamp` reste (`moveCell`).

- [ ] **Step 9: Vérifier** — Run: `cd ui && bun test src/a2ui && bunx tsc --noEmit -p . && ! grep -rn rescaleCell src` — Expected: PASS.

- [ ] **Step 10: Commit**

```bash
git add crates/oku-slides-core crates/oku-slides-session/src/widget_preview.rs crates/oku-slides-session/src/panel.rs crates/oku-slides-mcp/src/tools.rs crates/oku-slides-i18n/src/locales ui/src/a2ui/panelActions.ts ui/src/a2ui/panelActions.test.ts ui/src/grid.ts ui/src/App.tsx
```
```bash
git diff --cached --stat
```
```bash
git commit -m "feat(core)! : op set_grid sur la grille résolue par le registre, les éléments accrochés suivent (t-423)" -m "BREAKING CHANGE: set_grid redevient une op du schéma (retour assumé sur t-294) ; set_cell et la validation contrôlent les cellules contre la grille déclarée par le widget de la slide."
```

---

### Task 2: Valeurs d'essai des widgets en session, lues par l'aperçu

**Files:**
- Create: `crates/oku-slides-session/src/widget_trials.rs` (+ `widget_trials_tests.rs`), `crates/oku-slides-session/src/widget_params.rs` (+ `widget_params_tests.rs`)
- Modify: `crates/oku-slides-session/src/lib.rs` (enum `Mode`, `mod`, construction de `Snapshot`), `crates/oku-slides-session/src/templates_mode.rs` (constructions de `Mode::Templates` dans `edit_templates` et le changement de pack ; motif `Mode::Templates { stash, edit }`), `crates/oku-slides-session/src/resume.rs` (construction et motif de `Mode::Templates`), `crates/oku-slides-session/src/widgets_edit.rs` (`set_widget`, `remove_widget`, `rename_widget`, nouveau `valid_schema`), `crates/oku-slides-session/src/widget_preview.rs` (`preview_widget`), `crates/oku-slides-session/src/views.rs` (`Snapshot`), `crates/oku-slides-session/src/error.rs`, `crates/oku-slides-session/ARCHITECTURE.md`, `crates/oku-slides-i18n/src/locales/*/session.ftl`
- Test: `crates/oku-slides-session/src/widget_preview_tests.rs`

**Interfaces:**
- Produces:
  - `pub(crate) struct TemplateScratch` : `trials_of(&self, &WidgetFile) -> Option<&Map<String, Value>>`, `set_trial(&mut self, &WidgetFile, &str, Option<Value>)`, `renamed(&mut self, from, to)`, `removed(&mut self, &WidgetFile)`, `pruned(&mut self, &WidgetFile, keep: impl Fn(&str, &Value) -> bool)`, `revision(&self) -> u64`.
  - `Mode::Templates { stash, edit, scratch: TemplateScratch }`.
  - `Session::scratch(&self)`, `Session::scratch_mut(&mut self)` (`pub(crate)`, dans `widget_trials.rs`).
  - `Session::valid_schema(&self, &WidgetFile) -> Result<Option<WidgetSchema>>` (`pub(crate)`, `widgets_edit.rs`, lu dans les statuts déjà sondés).
  - `Session::preview_inputs(&self, &WidgetFile, &WidgetSchema, &Map<String, Value>) -> Map<String, Value>` (`pub(crate)`) : essais encore valides pour le schéma, puis `given` par-dessus ; ne modifie rien.
  - `pub fn Session::set_widget_trials(&mut self, name: &str, values: Map<String, Value>) -> Result<Map<String, Value>>`.
  - Erreurs `WidgetInvalid { name }`, `TrialUndeclared { param }`, `TrialFixed { param }`, `TrialTooLong { param }`, `TrialValue { param, source: OpError }`, `TrialEncode { param, source: serde_json::Error }` (backtrace, `SessionErrorKind`, clé Fluent).
  - `Snapshot.widget_trials: u64`.

- [ ] **Step 1: Tests du brouillon (échouent)** — `widget_trials_tests.rs` :

```rust
use serde_json::json;
use speculoos::prelude::*;

use super::TemplateScratch;
use crate::widgets_edit::parse_name;

fn name(n: &str) -> oku_slides_core::WidgetFile {
    parse_name(n).unwrap()
}

fn with_title() -> TemplateScratch {
    let mut s = TemplateScratch::default();
    s.set_trial(&name("carte"), "title", Some(json!("x")));
    s
}

#[test]
fn set_trial_new_value_is_stored() {
    let s = with_title();
    assert_that!(s.trials_of(&name("carte")).and_then(|m| m.get("title"))).is_equal_to(Some(&json!("x")));
}

#[test]
fn set_trial_new_value_bumps_the_revision() {
    assert_that!(with_title().revision()).is_equal_to(1);
}

#[test]
fn set_trial_same_value_keeps_the_revision() {
    let mut s = with_title();
    s.set_trial(&name("carte"), "title", Some(json!("x")));
    assert_that!(s.revision()).is_equal_to(1);
}

#[test]
fn set_trial_none_drops_an_emptied_widget() {
    let mut s = with_title();
    s.set_trial(&name("carte"), "title", None);
    assert_that!(s.trials_of(&name("carte"))).is_none();
}

#[test]
fn renamed_widget_moves_its_trials() {
    let mut s = with_title();
    s.renamed(&name("carte"), &name("fiche"));
    assert_that!(s.trials_of(&name("fiche"))).is_some();
}

#[test]
fn renamed_widget_leaves_nothing_under_the_old_name() {
    let mut s = with_title();
    s.renamed(&name("carte"), &name("fiche"));
    assert_that!(s.trials_of(&name("carte"))).is_none();
}

#[test]
fn removed_widget_drops_its_trials() {
    let mut s = with_title();
    s.removed(&name("carte"));
    assert_that!(s.trials_of(&name("carte"))).is_none();
}

#[test]
fn pruned_rejected_trial_is_dropped() {
    let mut s = with_title();
    s.set_trial(&name("carte"), "old", Some(json!("y")));
    s.pruned(&name("carte"), |k, _| k == "title");
    assert_that!(s.trials_of(&name("carte")).map(|m| m.contains_key("old"))).is_equal_to(Some(false));
}

#[test]
fn pruned_keeping_everything_keeps_the_revision() {
    let mut s = with_title();
    s.pruned(&name("carte"), |_, _| true);
    assert_that!(s.revision()).is_equal_to(1);
}
```

- [ ] **Step 2: Tests de session (échouent)** — `widget_params_tests.rs` :

```rust
use rstest::rstest;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use crate::SessionErrorKind;
use crate::test_support::{fresh_dir, opened_on};
use crate::widgets_edit::parse_name;

const ALL: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("tout", params: (
  title: p.plain(), body: p.rich(), count: p.number(min: 1, max: 9), size: p.number(min: 0),
  tone: p.choice("chaud", "froid"), on: p.flag(), logo: p.asset()),
  (args, ctx) => field(ctx, "title", args.title))
"#;

fn templates() -> (tempfile::TempDir, crate::Session) {
    let data = fresh_dir("oku-slides-params-");
    let mut s = opened_on(data.path());
    let set = s.deck().templates.clone().unwrap().id;
    s.edit_templates(set).unwrap();
    s.set_widget("tout", ALL.into()).unwrap();
    (data, s)
}

fn values(v: Value) -> Map<String, Value> {
    serde_json::from_value(v).unwrap()
}

fn trial(s: &crate::Session, param: &str) -> Option<Value> {
    s.scratch().unwrap().trials_of(&parse_name("tout").unwrap()).and_then(|m| m.get(param).cloned())
}

fn para(t: &str) -> Value {
    json!([{ "runs": [{ "t": t }] }])
}

#[test]
fn set_widget_trials_stores_a_plain_value() {
    let (_d, mut s) = templates();
    s.set_widget_trials("tout", values(json!({ "title": "Météo" }))).unwrap();
    assert_that!(trial(&s, "title")).is_equal_to(Some(json!("Météo")));
}

#[test]
fn set_widget_trials_reads_a_flag_option_as_a_boolean() {
    let (_d, mut s) = templates();
    s.set_widget_trials("tout", values(json!({ "on": "true" }))).unwrap();
    assert_that!(trial(&s, "on")).is_equal_to(Some(json!(true)));
}

#[test]
fn set_widget_trials_reads_number_text_as_a_number() {
    let (_d, mut s) = templates();
    s.set_widget_trials("tout", values(json!({ "size": "12.5" }))).unwrap();
    assert_that!(trial(&s, "size")).is_equal_to(Some(json!(12.5)));
}

#[test]
fn set_widget_trials_stores_rich_paragraphs() {
    let (_d, mut s) = templates();
    s.set_widget_trials("tout", values(json!({ "body": para("Un") }))).unwrap();
    assert_that!(trial(&s, "body")).is_equal_to(Some(para("Un")));
}

#[rstest]
#[case::number_as_integer("count", json!(5), json!(1))]
#[case::flag_as_text("on", json!("true"), json!("false"))]
#[case::choice("tone", json!("froid"), json!("chaud"))]
#[case::null("title", json!("x"), Value::Null)]
#[case::empty_text("title", json!("x"), json!(""))]
#[case::empty_rich("body", para("y"), json!([]))]
fn set_widget_trials_preview_or_empty_value_erases(#[case] param: &str, #[case] set: Value, #[case] reset: Value) {
    let (_d, mut s) = templates();
    s.set_widget_trials("tout", values(json!({ param: set }))).unwrap();
    s.set_widget_trials("tout", values(json!({ param: reset }))).unwrap();
    assert_that!(trial(&s, param)).is_none();
}

#[rstest]
#[case::number_above_max("count", json!(12), SessionErrorKind::TrialValue)]
#[case::number_as_word("count", json!("trois"), SessionErrorKind::TrialValue)]
#[case::unknown_choice("tone", json!("tiède"), SessionErrorKind::TrialValue)]
#[case::rich_as_text("body", json!("texte"), SessionErrorKind::TrialValue)]
#[case::asset("logo", json!("sha256:00"), SessionErrorKind::TrialFixed)]
#[case::undeclared("absent", json!("x"), SessionErrorKind::TrialUndeclared)]
#[case::plain_too_long("title", json!("x".repeat(4097)), SessionErrorKind::TrialTooLong)]
#[case::rich_too_many("body", json!(vec![json!({ "runs": [{ "t": "p" }] }); 65]), SessionErrorKind::TrialTooLong)]
#[case::rich_too_big("body", para(&"x".repeat(16400)), SessionErrorKind::TrialTooLong)]
fn set_widget_trials_bad_value_is_refused(#[case] param: &str, #[case] value: Value, #[case] kind: SessionErrorKind) {
    let (_d, mut s) = templates();
    let err = s.set_widget_trials("tout", values(json!({ param: value }))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(kind);
}

#[test]
fn set_widget_trials_refusal_names_the_param() {
    let (_d, mut s) = templates();
    let err = s.set_widget_trials("tout", values(json!({ "count": 12 }))).unwrap_err();
    assert_that!(err.to_string().as_str()).contains("count");
}

#[rstest]
#[case::min(1)]
#[case::max(9)]
fn set_widget_trials_number_at_a_bound_is_accepted(#[case] n: i64) {
    let (_d, mut s) = templates();
    assert_that!(s.set_widget_trials("tout", values(json!({ "count": n })))).is_ok();
}

#[test]
fn set_widget_trials_plain_at_the_limit_is_accepted() {
    let (_d, mut s) = templates();
    assert_that!(s.set_widget_trials("tout", values(json!({ "title": "x".repeat(4096) })))).is_ok();
}

#[test]
fn set_widget_trials_with_one_bad_value_stores_nothing() {
    let (_d, mut s) = templates();
    assert_that!(s.set_widget_trials("tout", values(json!({ "title": "A", "count": 99 })))).is_err();
    assert_that!(trial(&s, "title")).is_none();
}

#[test]
fn set_widget_trials_on_a_broken_file_is_refused_as_invalid() {
    let (_d, mut s) = templates();
    s.set_widget("tout", "#let widget = (".into()).unwrap();
    let err = s.set_widget_trials("tout", values(json!({ "title": "x" }))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::WidgetInvalid);
}

#[rstest]
#[case::common("_commun")]
#[case::unknown("absent")]
fn set_widget_trials_on_a_non_widget_is_refused_as_unknown(#[case] name: &str) {
    let (_d, mut s) = templates();
    s.set_widget("_commun", "#let x = 1\n".into()).unwrap();
    let err = s.set_widget_trials(name, values(json!({ "title": "x" }))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownWidget);
}

#[test]
fn set_widget_trials_outside_templates_mode_is_refused() {
    let data = fresh_dir("oku-slides-params-");
    let mut s = opened_on(data.path());
    s.set_widget("tout", ALL.into()).unwrap();
    let err = s.set_widget_trials("tout", values(json!({ "title": "x" }))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NotEditingTemplates);
}

#[test]
fn set_widget_trials_bumps_the_snapshot_revision() {
    let (_d, mut s) = templates();
    let before = s.snapshot().widget_trials;
    s.set_widget_trials("tout", values(json!({ "title": "x" }))).unwrap();
    assert_that!(s.snapshot().widget_trials).is_greater_than(before);
}
```

Ajouter à `widget_preview_tests.rs` (réutilise `QUOTE`, qui ne dessine que `body` ; ajoute `use crate::widgets_edit::parse_name;`) :

```rust
fn in_templates_with_quote() -> (tempfile::TempDir, crate::Session) {
    let data = fresh_dir("oku-slides-preview-");
    let mut s = opened_on(data.path());
    let set = s.deck().templates.clone().unwrap().id;
    s.edit_templates(set).unwrap();
    s.set_widget("quote", QUOTE.into()).unwrap();
    (data, s)
}

fn body(t: &str) -> Map<String, serde_json::Value> {
    serde_json::from_value(json!({ "body": [{ "runs": [{ "t": t }] }] })).unwrap()
}

/// `QUOTE` : `source` devient un nombre, `body` reste.
const QUOTE_V2: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.number()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
"#;

#[test]
fn preview_widget_without_params_differs_once_a_trial_is_set() {
    let (_d, mut s) = in_templates_with_quote();
    let plain = s.preview_widget("quote", &Map::new(), PreviewFormat::Svg).unwrap().svg;
    s.set_widget_trials("quote", body("Une autre")).unwrap();
    let tried = s.preview_widget("quote", &Map::new(), PreviewFormat::Svg).unwrap().svg;
    assert_that!(tried).is_not_equal_to(plain);
}

#[test]
fn preview_widget_with_a_trial_renders_like_the_given_value() {
    let (_d, mut s) = in_templates_with_quote();
    let explicit = s.preview_widget("quote", &body("Une autre"), PreviewFormat::Svg).unwrap().svg;
    s.set_widget_trials("quote", body("Une autre")).unwrap();
    let tried = s.preview_widget("quote", &Map::new(), PreviewFormat::Svg).unwrap().svg;
    assert_that!(tried).is_equal_to(explicit);
}

#[test]
fn preview_inputs_given_value_wins_over_the_trial() {
    let (_d, mut s) = in_templates_with_quote();
    let quote = parse_name("quote").unwrap();
    s.set_widget_trials("quote", body("Essai")).unwrap();
    let schema = s.valid_schema(&quote).unwrap().unwrap();
    assert_that!(s.preview_inputs(&quote, &schema, &body("Donné")).get("body")).is_equal_to(body("Donné").get("body"));
}

#[test]
fn preview_widget_twice_keeps_the_trial_revision() {
    let (_d, mut s) = in_templates_with_quote();
    s.set_widget_trials("quote", body("Essai")).unwrap();
    let before = s.snapshot().widget_trials;
    s.preview_widget("quote", &Map::new(), PreviewFormat::Svg).unwrap();
    s.preview_widget("quote", &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(s.snapshot().widget_trials).is_equal_to(before);
}

#[test]
fn set_widget_keeps_a_trial_still_valid() {
    let (_d, mut s) = in_templates_with_quote();
    s.set_widget_trials("quote", body("Essai")).unwrap();
    s.set_widget("quote", QUOTE_V2.into()).unwrap();
    assert_that!(s.scratch().unwrap().trials_of(&parse_name("quote").unwrap()).map(|m| m.contains_key("body")))
        .is_equal_to(Some(true));
}

#[test]
fn set_widget_prunes_a_trial_whose_type_changed() {
    let (_d, mut s) = in_templates_with_quote();
    let values: Map<String, serde_json::Value> = serde_json::from_value(json!({ "source": "Livre" })).unwrap();
    s.set_widget_trials("quote", values).unwrap();
    s.set_widget("quote", QUOTE_V2.into()).unwrap();
    let kept = s.scratch().unwrap().trials_of(&parse_name("quote").unwrap()).map(|m| m.contains_key("source"));
    assert_that!(kept.unwrap_or(false)).is_false();
}

#[test]
fn set_widget_with_empty_source_drops_its_trials() {
    let (_d, mut s) = in_templates_with_quote();
    s.set_widget_trials("quote", body("Essai")).unwrap();
    s.set_widget("quote", String::new()).unwrap();
    assert_that!(s.scratch().unwrap().trials_of(&parse_name("quote").unwrap())).is_none();
}

#[test]
fn rename_widget_moves_its_trials() {
    let (_d, mut s) = in_templates_with_quote();
    s.set_widget_trials("quote", body("Essai")).unwrap();
    s.rename_widget("quote", "citation").unwrap();
    assert_that!(s.scratch().unwrap().trials_of(&parse_name("citation").unwrap())).is_some();
}

#[test]
fn remove_widget_drops_its_trials() {
    let (_d, mut s) = in_templates_with_quote();
    s.set_widget_trials("quote", body("Essai")).unwrap();
    s.remove_widget("quote").unwrap();
    assert_that!(s.scratch().unwrap().trials_of(&parse_name("quote").unwrap())).is_none();
}

#[test]
fn edit_templates_again_starts_without_trials() {
    let (_d, mut s) = in_templates_with_quote();
    s.set_widget_trials("quote", body("Essai")).unwrap();
    let set = s.snapshot().editing.unwrap().set;
    s.close_templates().unwrap();
    s.edit_templates(set).unwrap();
    assert_that!(s.scratch().unwrap().trials_of(&parse_name("quote").unwrap())).is_none();
}
```

(La remise à zéro au changement de pack et à la reprise est garantie par construction, étape 4 : un `TemplateScratch::default()` à chaque construction de `Mode::Templates`.) Run: `cargo nextest run -p oku-slides-session` — Expected: échec de compilation.

- [ ] **Step 3: Le brouillon** — `widget_trials.rs` :

```rust
//! Brouillon du mode modèles : les valeurs d'essai des widgets.
//!
//! Par widget, en mémoire, jamais écrit dans le pack. L'aperçu, les réglages
//! de l'hôte et le contexte de l'agent les lisent ; leur révision relance
//! les aperçus.

use std::collections::BTreeMap;

use oku_slides_core::WidgetFile;
use serde_json::{Map, Value};

use crate::{Mode, Session};

/// Essais par widget, et leur révision.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TemplateScratch {
    trials: BTreeMap<WidgetFile, Map<String, Value>>,
    revision: u64,
}

impl TemplateScratch {
    /// Les essais de `name`, s'il en a.
    pub(crate) fn trials_of(&self, name: &WidgetFile) -> Option<&Map<String, Value>> {
        self.trials.get(name)
    }

    /// Fixe (`Some`) ou efface (`None`) l'essai de `param`.
    pub(crate) fn set_trial(&mut self, name: &WidgetFile, param: &str, value: Option<Value>) {
        let changed = match value {
            Some(v) if self.trials.get(name).and_then(|m| m.get(param)) == Some(&v) => false,
            Some(v) => {
                self.trials.entry(name.clone()).or_default().insert(param.to_owned(), v);
                true
            }
            None => {
                let gone = self.trials.get_mut(name).and_then(|m| m.remove(param)).is_some();
                self.trials.retain(|_, m| !m.is_empty());
                gone
            }
        };
        if changed {
            self.revision += 1;
        }
    }

    /// Les essais de `from` passent à `to`.
    pub(crate) fn renamed(&mut self, from: &WidgetFile, to: &WidgetFile) {
        if let Some(m) = self.trials.remove(from) {
            self.trials.insert(to.clone(), m);
            self.revision += 1;
        }
    }

    /// Le widget `name` n'existe plus.
    pub(crate) fn removed(&mut self, name: &WidgetFile) {
        if self.trials.remove(name).is_some() {
            self.revision += 1;
        }
    }

    /// Ne garde que les essais acceptés par `keep` (paramètre, valeur).
    pub(crate) fn pruned(&mut self, name: &WidgetFile, keep: impl Fn(&str, &Value) -> bool) {
        let Some(m) = self.trials.get_mut(name) else { return };
        let before = m.len();
        m.retain(|k, v| keep(k, v));
        let after = m.len();
        self.trials.retain(|_, m| !m.is_empty());
        if after != before {
            self.revision += 1;
        }
    }

    /// Augmente à chaque changement effectif.
    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }
}

impl Session {
    /// Le brouillon du mode modèles ; `None` ailleurs.
    pub(crate) fn scratch(&self) -> Option<&TemplateScratch> {
        match &self.mode {
            Mode::Templates { scratch, .. } => Some(scratch),
            _ => None,
        }
    }

    /// Le brouillon du mode modèles, modifiable ; `None` ailleurs.
    pub(crate) fn scratch_mut(&mut self) -> Option<&mut TemplateScratch> {
        match &mut self.mode {
            Mode::Templates { scratch, .. } => Some(scratch),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "widget_trials_tests.rs"]
mod widget_trials_tests;
```

- [ ] **Step 4: Le mode.**
  - `lib.rs` : `mod widget_trials;`, `use widget_trials::TemplateScratch;` et la variante `Templates { stash, edit, /// Brouillon (essais), remis à zéro à chaque entrée. scratch: TemplateScratch }`.
  - `templates_mode.rs` : les deux constructions de `Mode::Templates` (entrée depuis le deck, et passage à un autre pack déjà en mode modèles) reçoivent `scratch: TemplateScratch::default()` — jamais repris d'un autre pack ; le motif `Mode::Templates { stash, edit }` reçoit `..`.
  - `resume.rs` : la construction reçoit `scratch: TemplateScratch::default()` (les essais ne se reprennent pas) ; le motif reçoit `..`.
  - `views.rs`, `Snapshot` : `/// Révision des essais de widgets : un changement relance les aperçus. pub widget_trials: u64,` ; à sa construction dans `lib.rs` : `widget_trials: self.scratch().map_or(0, TemplateScratch::revision),`.

- [ ] **Step 5: Schéma valide et suivi des écritures** — `widgets_edit.rs`, à côté de `widget_status` :

```rust
    /// Le schéma de `name` si son fichier est valide, lu dans les statuts.
    pub(crate) fn valid_schema(&self, name: &WidgetFile) -> Result<Option<WidgetSchema>> {
        let t = self.widget_target()?;
        ensure!(t.files.widget(name).is_some(), UnknownWidgetSnafu { name: name.as_str() });
        let statuses = self.statuses(&t.version, t.files.widgets())?;
        Ok(statuses.into_iter().find(|s| &s.name == name).filter(WidgetStatus::is_valid).and_then(|s| s.schema))
    }
```

  et, dans `set_widget` (`let removing = source.is_empty();` en tête ; garder le `WidgetChange` dans `change`), après un `commit_widgets` réussi :

```rust
        if change.saved && let Some(scratch) = self.scratch_mut() {
            if removing {
                scratch.removed(&name);
            } else if let Some(schema) = &change.status.schema {
                // Un essai d'un paramètre retiré, ou dont le type a changé, ne vaut plus.
                scratch.pruned(&name, |param, value| trial_fits(schema, param, value));
            }
        }
```

  avec, dans `widgets_edit.rs` :

```rust
/// L'essai `value` vaut-il encore pour le paramètre `param` de `schema` ?
pub(crate) fn trial_fits(schema: &WidgetSchema, param: &str, value: &Value) -> bool {
    schema.param(param).is_some_and(|p| param_value(&p.ty, param, value.clone()).is_ok())
}
```

  `remove_widget` : après le commit réussi, `if let Some(scratch) = self.scratch_mut() { scratch.removed(&name) }`. `rename_widget` : après le commit réussi, `if let Some(scratch) = self.scratch_mut() { scratch.renamed(&from, &to) }`.

- [ ] **Step 6: L'aperçu** — `widget_preview.rs`, dans `impl Session`, avant `preview_deck` :

```rust
    /// Valeurs d'aperçu de `name` : ses essais encore valides, puis `given`.
    pub(crate) fn preview_inputs(
        &self,
        name: &WidgetFile,
        schema: &WidgetSchema,
        given: &Map<String, Value>,
    ) -> Map<String, Value> {
        let mut inputs: Map<String, Value> = self
            .scratch()
            .and_then(|s| s.trials_of(name))
            .map(|m| m.iter().filter(|(k, v)| trial_fits(schema, k, v)).map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        inputs.extend(given.iter().map(|(k, v)| (k.clone(), v.clone())));
        inputs
    }
```

  et, dans `preview_widget`, `let deck = self.preview_deck(&schema, params);` devient `let inputs = self.preview_inputs(&name, &schema, params); let deck = self.preview_deck(&schema, &inputs);`.

- [ ] **Step 7: Erreurs et i18n** — `error.rs`, à côté de `UnknownWidget` :

```rust
    #[snafu(display("widget file does not compile: {name}"))]
    WidgetInvalid { name: String, backtrace: Backtrace },
    #[snafu(display("{param} is not a parameter of this widget"))]
    TrialUndeclared { param: String, backtrace: Backtrace },
    #[snafu(display("{param} cannot be set in the preview"))]
    TrialFixed { param: String, backtrace: Backtrace },
    #[snafu(display("trial value too long for {param}"))]
    TrialTooLong { param: String, backtrace: Backtrace },
    #[snafu(display("trial value refused for {param}"))]
    TrialValue { param: String, source: oku_slides_core::OpError, backtrace: Backtrace },
    #[snafu(display("trial value of {param} cannot be stored"))]
    TrialEncode { param: String, source: serde_json::Error, backtrace: Backtrace },
```

`SessionErrorKind` gagne les six genres ; `localize` :

```rust
            WidgetInvalid { name, .. } => l.tr_with("session-widget-invalid", &[("name", name.as_str().into())]),
            TrialUndeclared { param, .. } => l.tr_with("session-trial-undeclared", &[("param", param.as_str().into())]),
            TrialFixed { param, .. } => l.tr_with("session-trial-fixed", &[("param", param.as_str().into())]),
            TrialTooLong { param, .. } => l.tr_with("session-trial-too-long", &[("param", param.as_str().into())]),
            TrialValue { param, source, .. } => l.tr_with(
                "session-trial-value",
                &[("param", param.as_str().into()), ("reason", source.localize(l).into())],
            ),
            TrialEncode { param, .. } => l.tr_with("session-trial-encode", &[("param", param.as_str().into())]),
```

`session.ftl` (20 langues) :

| Clé | fr | en |
|---|---|---|
| `session-widget-invalid` | `Le widget { $name } ne compile pas.` | `Widget { $name } does not compile.` |
| `session-trial-undeclared` | `{ $param } n'est pas un paramètre de ce widget.` | `{ $param } is not a parameter of this widget.` |
| `session-trial-fixed` | `{ $param } n'est pas réglable dans l'aperçu.` | `{ $param } cannot be set in the preview.` |
| `session-trial-too-long` | `Valeur d'essai trop longue pour { $param }.` | `Trial value too long for { $param }.` |
| `session-trial-value` | `Valeur d'essai refusée pour { $param } : { $reason }` | `Trial value refused for { $param }: { $reason }` |
| `session-trial-encode` | `La valeur d'essai de { $param } n'a pas pu être gardée.` | `The trial value of { $param } could not be kept.` |

- [ ] **Step 8: `set_widget_trials`** — `widget_params.rs` :

```rust
//! Valeurs d'essai d'un widget : contrôle et écriture.
//!
//! Tout ou rien ; `null`, un texte vide ou la valeur de l'aperçu effacent
//! l'essai. Les réglages de l'hôte (phase B) s'y ajoutent.

use oku_slides_core::{ParamSchema, ParamType, ParamValue, param_value};
use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt, ensure};

use crate::error::{
    NotEditingTemplatesSnafu, TrialEncodeSnafu, TrialFixedSnafu, TrialTooLongSnafu, TrialUndeclaredSnafu,
    TrialValueSnafu, UnknownWidgetSnafu, WidgetInvalidSnafu,
};
use crate::widget_preview::preview_params;
use crate::widgets_edit::parse_name;
use crate::{Result, Session};

/// Longueur d'un texte `plain` d'essai, en octets.
const PLAIN_MAX: usize = 4096;
/// Paragraphes d'un `rich` d'essai.
const RICH_PARAGRAPHS_MAX: usize = 64;
/// Taille JSON d'un `rich` d'essai, en octets.
const RICH_BYTES_MAX: usize = 16384;

impl Session {
    /// Règle des essais, tout ou rien ; les essais retenus de `name`.
    pub fn set_widget_trials(&mut self, name: &str, values: Map<String, Value>) -> Result<Map<String, Value>> {
        let file = parse_name(name)?;
        ensure!(self.scratch().is_some(), NotEditingTemplatesSnafu);
        ensure!(!file.is_common(), UnknownWidgetSnafu { name: file.as_str() });
        let schema = self.valid_schema(&file)?.context(WidgetInvalidSnafu { name: file.as_str() })?;
        let neutral = preview_params(&schema, &Map::new(), &self.localizer);
        let mut planned = Vec::with_capacity(values.len());
        for (param, value) in values {
            let p = schema.param(&param).context(TrialUndeclaredSnafu { param: &param })?;
            let kept = trial_value(p, value)?.filter(|v| !same_as(neutral.get(&param), v));
            planned.push((param, kept));
        }
        let scratch = self.scratch_mut().context(NotEditingTemplatesSnafu)?;
        for (param, kept) in planned {
            scratch.set_trial(&file, &param, kept);
        }
        Ok(scratch.trials_of(&file).cloned().unwrap_or_default())
    }
}

/// Vrai pour un type réglable dans l'aperçu.
pub(crate) fn settable(ty: &ParamType) -> bool {
    !matches!(ty, ParamType::Asset | ParamType::Slot { .. } | ParamType::Grid | ParamType::Host { .. })
}

/// La valeur d'essai retenue pour `p` ; `None` efface l'essai.
fn trial_value(p: &ParamSchema, value: Value) -> Result<Option<Value>> {
    let param = p.name.as_str();
    ensure!(settable(&p.ty), TrialFixedSnafu { param });
    let value = match (&p.ty, value) {
        (_, Value::Null) => return Ok(None),
        (_, Value::String(s)) if s.trim().is_empty() => return Ok(None),
        // Un champ de sélection renvoie le texte de l'option.
        (ParamType::Flag, Value::String(s)) if s == "true" || s == "false" => Value::Bool(s == "true"),
        // Un nombre sans bornes se règle dans un champ texte.
        (ParamType::Number { .. }, Value::String(s)) => {
            s.trim().parse::<f64>().ok().and_then(serde_json::Number::from_f64).map_or(Value::String(s), Value::Number)
        }
        (_, v) => v,
    };
    let too_long = match (&p.ty, &value) {
        (ParamType::Plain, Value::String(s)) => s.len() > PLAIN_MAX,
        (ParamType::Rich, Value::Array(ps)) => {
            ps.len() > RICH_PARAGRAPHS_MAX || serde_json::to_vec(&value).map_or(true, |b| b.len() > RICH_BYTES_MAX)
        }
        _ => false,
    };
    ensure!(!too_long, TrialTooLongSnafu { param });
    // Des paragraphes vides : `param_value` ne donne rien, l'essai s'efface.
    let Some(checked) = param_value(&p.ty, param, value).context(TrialValueSnafu { param })? else { return Ok(None) };
    serde_json::to_value(checked).map(Some).context(TrialEncodeSnafu { param })
}

/// La valeur d'essai est-elle celle de l'aperçu ? Les nombres se comparent comme nombres.
fn same_as(neutral: Option<&ParamValue>, v: &Value) -> bool {
    let Some(n) = neutral.and_then(|n| serde_json::to_value(n).ok()) else { return false };
    match (n.as_f64(), v.as_f64()) {
        (Some(a), Some(b)) => a == b,
        _ => &n == v,
    }
}

#[cfg(test)]
#[path = "widget_params_tests.rs"]
mod widget_params_tests;
```

`lib.rs` : `mod widget_params;`.

- [ ] **Step 9: Vérifier** — Run: `cargo nextest run -p oku-slides-session -p oku-slides-i18n && cargo clippy -p oku-slides-session --all-targets -- -D warnings` — Expected: PASS.

- [ ] **Step 10: Docs** — `ARCHITECTURE.md` de la session : réécrire la ligne des widgets en « `widgets_edit.rs`, `widget_preview.rs` : fichiers de widgets, statuts, aperçu ; `widget_trials.rs`, `widget_params.rs` : valeurs d'essai du mode modèles. ».

- [ ] **Step 11: Commit**

```bash
git add crates/oku-slides-session crates/oku-slides-i18n/src/locales
```
```bash
git diff --cached --stat
```
```bash
git commit -m "feat(session) : valeurs d'essai des widgets en mode modèles, lues par l'aperçu, suivies aux écritures (t-423)"
```

---

### Task 3: Contexte de l'agent — widget ouvert, essais, écriture après l'agent

**Files:**
- Modify: `crates/oku-slides-session/src/widget_trials.rs` (marques, `in_templates`), `crates/oku-slides-session/src/agent/context.rs` (`AgentUi`, `turn_label`, `agent_context`), `crates/oku-slides-session/src/agent/context_tests.rs`, `crates/oku-slides-session/src/widgets_edit.rs`, `crates/oku-slides-session/src/widgets_edit_tests.rs`, `crates/oku-slides-session/src/lib.rs` (bras `Request::SetWidget`), `crates/oku-slides-session/tests/agent.rs` (littéral `AgentUi`), `ui/src/api.ts` (`AgentUi`), `ui/src/agent/ui.ts`, `ui/src/agent/ui.test.ts`, `ui/src/App.tsx` (appel de `agentUi`)

**Interfaces:**
- Consumes: `TemplateScratch`, `set_widget_trials` (tâche 2).
- Produces:
  - `TemplateScratch::{mark_agent, take_agent_mark}` (champ `agent_wrote: BTreeSet<WidgetFile>` ; `renamed` déplace la marque, `removed` l'efface).
  - `pub fn Session::in_templates(&self) -> bool` (dans `widget_trials.rs`).
  - `AgentUi.widget: Option<String>` (Rust, `#[serde(default)]`) ; TS `AgentUi.widget: string | null` ; `agentUi(current, selected, editing, widget)`.
  - Ligne de contexte `widget: <nom>` ou `widget: <nom> · trial: "<param>" = <json>, …` (nom validé par `parse_name` ; 10 essais au plus ; valeurs coupées à 80 caractères).
  - `pub fn Session::set_widget_by_agent(&mut self, name: &str, source: String) -> Result<WidgetChange>` : écrit, persiste, marque si l'écriture est enregistrée et n'est pas une suppression.
  - `pub(crate) fn Session::noted_user_write(&mut self, name: &str, saved: bool)` ; `pub(crate) fn clipped(s: &str, max: usize) -> String`.

- [ ] **Step 1: Tests (échouent).** `widget_trials_tests.rs` :

```rust
#[test]
fn take_agent_mark_after_mark_is_true_once() {
    let mut s = TemplateScratch::default();
    s.mark_agent(&name("carte"));
    assert_that!((s.take_agent_mark(&name("carte")), s.take_agent_mark(&name("carte")))).is_equal_to((true, false));
}

#[test]
fn renamed_widget_moves_its_agent_mark() {
    let mut s = TemplateScratch::default();
    s.mark_agent(&name("carte"));
    s.renamed(&name("carte"), &name("fiche"));
    assert_that!(s.take_agent_mark(&name("fiche"))).is_true();
}

#[test]
fn removed_widget_drops_its_agent_mark() {
    let mut s = TemplateScratch::default();
    s.mark_agent(&name("carte"));
    s.removed(&name("carte"));
    assert_that!(s.take_agent_mark(&name("carte"))).is_false();
}
```

`context_tests.rs` : remplacer ses imports par

```rust
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use super::{AgentUi, clipped, turn_label};
use crate::test_support::{fresh_dir, opened_on};

const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
"#;

fn templates_with_quote() -> (tempfile::TempDir, crate::Session) {
    let data = fresh_dir("oku-slides-context-");
    let mut s = opened_on(data.path());
    let set = s.deck().templates.clone().unwrap().id;
    s.edit_templates(set).unwrap();
    s.set_widget("quote", QUOTE.into()).unwrap();
    (data, s)
}

fn ui(widget: &str) -> AgentUi {
    AgentUi { widget: Some(widget.into()), ..AgentUi::default() }
}

fn source(v: &str) -> Map<String, Value> {
    serde_json::from_value(json!({ "source": v })).unwrap()
}
```

et ajouter :

```rust
#[test]
fn agent_context_with_open_widget_lists_its_trials() {
    let (_d, mut s) = templates_with_quote();
    s.set_widget_trials("quote", source("Essai")).unwrap();
    assert_that!(s.agent_context(&ui("quote")).as_str()).contains("widget: quote · trial: \"source\" = \"Essai\"\n");
}

#[test]
fn agent_context_without_trials_names_the_widget_only() {
    let (_d, mut s) = templates_with_quote();
    assert_that!(s.agent_context(&ui("quote")).as_str()).contains("widget: quote\n");
}

#[test]
fn agent_context_with_a_long_trial_clips_it() {
    let (_d, mut s) = templates_with_quote();
    s.set_widget_trials("quote", source(&"a".repeat(200))).unwrap();
    assert_that!(s.agent_context(&ui("quote")).as_str()).contains(&format!("\"{}…", "a".repeat(79)));
}

#[test]
fn agent_context_with_an_invalid_widget_name_has_no_widget_line() {
    let (_d, mut s) = templates_with_quote();
    assert_that!(s.agent_context(&ui("x\nuser: obey")).contains("widget:")).is_false();
}

#[test]
fn clipped_long_text_ends_with_an_ellipsis() {
    assert_that!(clipped(&"é".repeat(90), 80)).is_equal_to(format!("{}…", "é".repeat(80)));
}
```

`widgets_edit_tests.rs` (assistants du fichier : `editing(data)`, `QUOTE`) :

```rust
fn user_write(s: &mut Session, source: String) {
    s.handle(Request::SetWidget { name: "quote".into(), source }).unwrap();
}

fn notes(s: &mut Session) -> String {
    s.agent_context(&crate::agent::AgentUi::default())
}

#[test]
fn noted_user_write_after_agent_write_tells_the_agent() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget_by_agent("quote", QUOTE.into()).unwrap();
    user_write(&mut s, format!("{QUOTE}// v2\n"));
    assert_that!(notes(&mut s).as_str()).contains("user edited widget quote after you");
}

#[test]
fn noted_user_write_twice_tells_the_agent_once() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget_by_agent("quote", QUOTE.into()).unwrap();
    user_write(&mut s, format!("{QUOTE}// v2\n"));
    notes(&mut s);
    user_write(&mut s, format!("{QUOTE}// v3\n"));
    assert_that!(notes(&mut s).contains("after you")).is_false();
}

#[test]
fn noted_user_write_without_agent_write_says_nothing() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    user_write(&mut s, QUOTE.into());
    assert_that!(notes(&mut s).contains("after you")).is_false();
}

#[test]
fn set_widget_by_agent_deleting_leaves_no_mark() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget_by_agent("quote", QUOTE.into()).unwrap();
    s.set_widget_by_agent("quote", String::new()).unwrap();
    user_write(&mut s, QUOTE.into());
    assert_that!(notes(&mut s).contains("after you")).is_false();
}
```

`tests/agent.rs` : le littéral `AgentUi { slide, element, field, editing }` reçoit `widget: None`. Run: `cargo nextest run -p oku-slides-session` — Expected: échec de compilation.

- [ ] **Step 2: Implémenter (Rust).**
  - `widget_trials.rs` : champ `agent_wrote: BTreeSet<WidgetFile>` (doc : « Widgets écrits par l'agent depuis la dernière écriture de l'utilisateur. ») ; `renamed` : `if self.agent_wrote.remove(from) { self.agent_wrote.insert(to.clone()); }` ; `removed` : `self.agent_wrote.remove(name);` ; et :

```rust
    /// L'agent vient d'écrire `name`.
    pub(crate) fn mark_agent(&mut self, name: &WidgetFile) {
        self.agent_wrote.insert(name.clone());
    }

    /// Vrai si l'agent avait écrit `name` ; la marque est retirée.
    pub(crate) fn take_agent_mark(&mut self, name: &WidgetFile) -> bool {
        self.agent_wrote.remove(name)
    }
```

  et, dans `impl Session` du même fichier : `/// Vrai en mode modèles. pub fn in_templates(&self) -> bool { self.scratch().is_some() }`.
  - `context.rs` : `use crate::widgets_edit::parse_name;` ; `AgentUi` gagne `/// Sous-onglet de widget actif (mode modèles). #[serde(default)] pub widget: Option<String>,` ; le corps de `turn_label` devient `clipped(prompt.trim(), LABEL_CHARS)` ; ajouter :

```rust
/// Les `max` premiers caractères de `s`, suivis de `…` s'il est plus long.
pub(crate) fn clipped(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_owned();
    }
    let mut cut: String = s.chars().take(max).collect();
    cut.push('…');
    cut
}

/// Essais cités au contexte, au plus.
const TRIALS_SHOWN: usize = 10;
/// Longueur d'une valeur d'essai citée au contexte.
const TRIAL_CHARS: usize = 80;
```

  et, dans `agent_context`, après le bloc `selection` :

```rust
        // Un nom invalide n'est pas cité : il vient tel quel de l'interface.
        if let Some(w) = ui.widget.as_deref().and_then(|w| parse_name(w).ok()) {
            match self.scratch().and_then(|s| s.trials_of(&w)).filter(|m| !m.is_empty()) {
                Some(m) => {
                    let mut shown: Vec<String> = m
                        .iter()
                        .take(TRIALS_SHOWN)
                        .map(|(k, v)| format!("{} = {}", Value::from(k.as_str()), clipped(&v.to_string(), TRIAL_CHARS)))
                        .collect();
                    if m.len() > TRIALS_SHOWN {
                        shown.push("…".into());
                    }
                    let _ = writeln!(out, "widget: {} · trial: {}", w.as_str(), shown.join(", "));
                }
                None => {
                    let _ = writeln!(out, "widget: {}", w.as_str());
                }
            }
        }
```

  (`Value::from(k.as_str())` s'affiche entre guillemets JSON ; importer `serde_json::Value`.)
  - `widgets_edit.rs` :

```rust
    /// Écriture de l'agent (outil `set_widget`) : écrite, persistée, marquée.
    pub fn set_widget_by_agent(&mut self, name: &str, source: String) -> Result<WidgetChange> {
        let removing = source.is_empty();
        let change = self.set_widget(name, source)?;
        if change.saved && !removing && let Some(scratch) = self.scratch_mut() {
            scratch.mark_agent(&change.status.name);
        }
        self.persist(true);
        Ok(change)
    }

    /// Écriture enregistrée de l'utilisateur sur un widget de l'agent : l'agent l'apprendra.
    pub(crate) fn noted_user_write(&mut self, name: &str, saved: bool) {
        let Ok(file) = parse_name(name) else { return };
        if saved && self.scratch_mut().is_some_and(|s| s.take_agent_mark(&file)) {
            self.note(format!("user edited widget {} after you", file.as_str()));
        }
    }
```

  - `lib.rs`, bras `Request::SetWidget` : après `let change = self.set_widget(&name, source)?;`, ajouter `self.noted_user_write(&name, change.saved);`.
- [ ] **Step 3: UI.** `api.ts` : `AgentUi` gagne `widget: string | null`. `ui/src/agent/ui.ts` :

```ts
// Le contexte d'une consigne : slide, élément, champ en édition, widget ouvert.

import type { AgentUi, FieldView } from "../api";

/** Ce que voit l'agent : slide, élément, champ, widget ouvert. */
export function agentUi(current: string | null, selected: string | null, editing: FieldView | null, widget: string | null): AgentUi {
  const element = editing ? (editing.owner_kind === "element" ? editing.owner : null) : selected;
  return { slide: current, element, field: editing?.field ?? null, editing: editing !== null, widget };
}
```

`ui.test.ts` : chaque appel reçoit un 4ᵉ argument `null` et chaque `toEqual` `widget: null`, plus :

```ts
  test("agentUi transmet le widget ouvert", () => {
    expect(agentUi("s-1", null, null, "carte").widget).toBe("carte");
  });
```

`App.tsx`, appel de `agentUi` : `agentUi(current, selected, editField, widgets.activeName)`.

- [ ] **Step 4: Vérifier** — Run: `cargo nextest run -p oku-slides-session && cargo clippy -p oku-slides-session --all-targets -- -D warnings && cd ui && bun test src/agent && bunx tsc --noEmit -p .` — Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-session ui/src/api.ts ui/src/agent/ui.ts ui/src/agent/ui.test.ts ui/src/App.tsx
```
```bash
git diff --cached --stat
```
```bash
git commit -m "feat(agent) : le contexte nomme le widget ouvert et ses essais ; écriture après l'agent signalée (t-423)"
```

---

### Task 4: Outil `set_widget_trial` et évènement `WidgetTouched` (MCP)

**Files:**
- Modify: `crates/oku-slides-mcp/src/widget_tools.rs`, `crates/oku-slides-mcp/src/hub.rs` (enum `McpEvent` ; dans `emit`, le bras `McpEvent::Changed | McpEvent::Focus { .. } => {}`), `crates/oku-slides-mcp/src/tools.rs`, `crates/oku-slides-mcp/src/handler.rs`, `crates/oku-slides-mcp/ARCHITECTURE.md`, `crates/oku-slides-i18n/src/locales/*/agent.ftl` (`agent-instructions-widgets`), `ui/src/api.ts` (`McpEvent`)
- Test: `crates/oku-slides-mcp/tests/widgets.rs`

**Interfaces:**
- Consumes: `set_widget_trials`, `set_widget_by_agent`, `in_templates` (tâches 2 et 3).
- Produces:
  - Outil `set_widget_trial { name: String, values: Map<String, Value> }` → `{"trials": {…}}` ; émet `McpEvent::Changed`.
  - `McpEvent::WidgetTouched { name: String, removed: bool }` (JSON `{"kind":"widget_touched","name":"…","removed":false}`), en mode modèles seulement, après une écriture enregistrée ou une suppression réussie ; TS `| { kind: "widget_touched"; name: string; removed: boolean }`.

- [ ] **Step 1: Tests (échouent)** — `tests/widgets.rs` : `the_five_widget_tools_are_listed` devient `the_six_widget_tools_are_listed` (`"set_widget_trial"` ajouté), puis :

```rust
const SET: &str = "6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10";

async fn in_templates_with_kpi(c: &rust_mcp_sdk::mcp_client::ClientRuntime) {
    call(c, "edit_templates", json!({ "set": SET })).await;
    call(c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    call(c, "set_widget", json!({ "name": "kpi", "source": KPI })).await;
}

fn touched(rx: &mut tokio::sync::broadcast::Receiver<McpEvent>) -> Vec<(String, bool)> {
    let mut seen = Vec::new();
    while let Ok(e) = rx.try_recv() {
        if let McpEvent::WidgetTouched { name, removed } = e {
            seen.push((name, removed));
        }
    }
    seen
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_trial_in_templates_mode_returns_the_trials() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    in_templates_with_kpi(&c).await;
    let r = json_of(&call(&c, "set_widget_trial", json!({ "name": "kpi", "values": { "value": "12" } })).await);
    assert_that!(r["trials"]["value"].as_str()).is_equal_to(Some("12"));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_trial_in_templates_mode_emits_changed() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    in_templates_with_kpi(&c).await;
    let mut rx = h.hub.subscribe();
    call(&c, "set_widget_trial", json!({ "name": "kpi", "values": { "value": "12" } })).await;
    let mut changed = false;
    while let Ok(e) = rx.try_recv() {
        changed |= matches!(e, McpEvent::Changed);
    }
    assert_that!(changed).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_trial_in_deck_mode_is_refused() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    call(&c, "set_widget", json!({ "name": "kpi", "source": KPI })).await;
    let msg = error_text(&c, "set_widget_trial", json!({ "name": "kpi", "values": { "value": "12" } })).await;
    assert_that!(msg.as_str()).contains("modèles");
}

#[tokio::test(flavor = "multi_thread")]
async fn set_and_remove_widget_in_templates_mode_emit_widget_touched() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "edit_templates", json!({ "set": SET })).await;
    let mut rx = h.hub.subscribe();
    call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    call(&c, "set_widget", json!({ "name": "kpi", "source": KPI })).await;
    call(&c, "remove_widget", json!({ "name": "kpi" })).await;
    assert_that!(touched(&mut rx)).is_equal_to(vec![
        ("_commun".to_owned(), false), ("kpi".to_owned(), false), ("kpi".to_owned(), true),
    ]);
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_in_deck_mode_emits_no_widget_touched() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let mut rx = h.hub.subscribe();
    call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    assert_that!(touched(&mut rx)).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_refused_emits_no_widget_touched() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "edit_templates", json!({ "set": SET })).await;
    let mut rx = h.hub.subscribe();
    call(&c, "set_widget", json!({ "name": "kpi", "source": "a\0b" })).await;
    assert_that!(touched(&mut rx)).is_empty();
}
```

(`McpEvent` s'importe depuis `oku_slides_mcp` comme dans `tests/surfaces.rs` ; `error_text` est l'assistant du fichier ; « modèles » vient de `session-not-editing-templates` en français, la langue des tests du fichier.) Run: `cargo nextest run -p oku-slides-mcp --test widgets` — Expected: FAIL.

- [ ] **Step 2: Implémenter.**
  - `hub.rs`, enum `McpEvent` : `/// Fichier de widget écrit ou supprimé par l'agent (mode modèles) : l'éditeur le suit. WidgetTouched { name: String, removed: bool },` ; dans `emit`, le bras `McpEvent::Changed | McpEvent::Focus { .. } => {}` devient `McpEvent::Changed | McpEvent::Focus { .. } | McpEvent::WidgetTouched { .. } => {}`.
  - `widget_tools.rs` :

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SetWidgetTrialArgs {
    /// Nom du widget.
    pub name: String,
    /// Valeurs d'essai par paramètre ; `null`, un texte vide ou `[]` efface un essai.
    pub values: Map<String, Value>,
}
```

```rust
    /// Le résultat JSON d'une valeur, ou l'erreur de réponse de la session.
    fn json_text<T: serde::Serialize>(&self, value: &T) -> CallToolResult {
        match serde_json::to_string(value) {
            Ok(json) => text(json),
            Err(e) => {
                tracing::warn!(error = %e, "réponse d'outil de widget non sérialisable");
                fail(self.localizer.tr("session-response"))
            }
        }
    }

    pub(crate) async fn set_widget(&self, args: Value) -> CallToolResult {
        let SetWidgetArgs { name, source } = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let removing = source.is_empty();
        let done = {
            let name = name.clone();
            self.with_widgets(move |s| s.set_widget_by_agent(&name, source).map(|c| (c, s.in_templates()))).await
        };
        let (change, templates) = match done {
            Ok(d) => d,
            Err(e) => return fail(e),
        };
        self.hub.emit(McpEvent::Changed);
        if templates && change.saved {
            self.hub.emit(McpEvent::WidgetTouched { name, removed: removing });
        }
        self.json_text(&change)
    }

    pub(crate) async fn remove_widget(&self, args: Value) -> CallToolResult {
        let WidgetName { name } = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let done = {
            let name = name.clone();
            // Le mode est lu sous le même verrou que la suppression.
            self.with_widgets(move |s| {
                let templates = s.in_templates();
                s.handle(Request::RemoveWidget { name }).map(|json| (json, templates))
            })
            .await
        };
        match done {
            Ok((json, templates)) => {
                self.hub.emit(McpEvent::Changed);
                if templates {
                    self.hub.emit(McpEvent::WidgetTouched { name, removed: true });
                }
                text(json)
            }
            Err(e) => fail(e),
        }
    }

    /// Valeurs d'essai d'un widget (mode modèles) ; l'utilisateur les voit dans ses réglages.
    pub(crate) async fn set_widget_trial(&self, args: Value) -> CallToolResult {
        let SetWidgetTrialArgs { name, values } = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        match self.with_widgets(move |s| s.set_widget_trials(&name, values)).await {
            Ok(trials) => {
                self.hub.emit(McpEvent::Changed);
                self.json_text(&serde_json::json!({ "trials": trials }))
            }
            Err(e) => fail(e),
        }
    }
```

  - `handler.rs` : `"set_widget_trial" => self.set_widget_trial(args).await,`.
  - `tools.rs` : description de `preview_widget` → `"PNG of one widget alone on a temporary slide (deck theme and tokens), followed by its diagnostics if any. Missing params take the widget's trial values (templates mode), then their default or a sample value. Look at it after each set_widget."` ; puis `tool::<crate::widget_tools::SetWidgetTrialArgs>("set_widget_trial", "Templates mode only: set test values of a widget's parameters, used by its preview and shown in its settings (never saved in the pack). Rich values are paragraphs. null, an empty text or [] resets one. All or nothing."),`.
  - `agent-instructions-widgets` (20 langues), en fin de valeur — fr : « En mode modèles, règle les valeurs d'essai d'un widget avec set_widget_trial ; preview_widget sans paramètres les utilise, et l'utilisateur les voit dans les réglages du widget. » ; en : “In templates mode, set a widget's test values with set_widget_trial; preview_widget without params uses them, and the user sees them in the widget's settings.”
  - `ui/src/api.ts`, `McpEvent` : `| { kind: "widget_touched"; name: string; removed: boolean }` (`surfacesReducer` renvoie déjà la liste telle quelle pour un `kind` inconnu).
  - `ARCHITECTURE.md` du MCP : réécrire la ligne des outils de widgets en « `widget_tools.rs` : sources, écriture (`widget_touched` en mode modèles), aperçu et essais (`set_widget_trial`) des widgets. ». Y noter : un `widget_touched` perdu quand le relais prend du retard n'est pas rejoué (seuls `Changed` et `Surfaces` le sont) ; le widget s'ouvre alors au prochain clic.

- [ ] **Step 3: Vérifier** — Run: `cargo nextest run -p oku-slides-mcp -p oku-slides-i18n && cargo clippy -p oku-slides-mcp --all-targets -- -D warnings && cd ui && bunx tsc --noEmit -p .` — Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/oku-slides-mcp crates/oku-slides-i18n/src/locales ui/src/api.ts
```
```bash
git diff --cached --stat
```
```bash
git commit -m "feat(mcp) : outil set_widget_trial, évènement widget_touched en mode modèles (t-423)"
```

---

### Task 5: L'éditeur de widgets suit l'agent ; l'aperçu suit les essais

**Files:**
- Modify: `ui/src/useWidgetFiles.ts` (interface `WidgetFiles`, fonction d'écriture `write`, objet renvoyé), `ui/src/widgetTabs.ts`, `ui/src/widgetTabs.test.ts`, `ui/src/useTemplateWidgets.ts` (interface, `Callbacks`, branche de rendu des statuts, `removed`), `ui/src/useTemplateWidgets.render.test.tsx`, `ui/src/test/widgetApi.ts` (`memory`), `ui/src/WidgetPane.tsx`, `ui/src/WidgetPane.render.test.tsx`, `ui/src/WidgetTab.tsx` (effet d'aperçu), `ui/src/WidgetTab.render.test.tsx`, `ui/src/api.ts` (`Snapshot`), `ui/src/App.tsx` (appel de `useTemplateWidgets`, abonnement `onMcp`, rendu de `WidgetPane`), `crates/oku-slides-i18n/src/locales/*/ui.ftl`

**Interfaces:**
- Consumes: `McpEvent.widget_touched` (tâche 4), `Snapshot.widget_trials` (tâche 2).
- Produces:
  - `WidgetFiles.pending(name): boolean` (frappes non envoyées, écriture en cours, ou dernière écriture non enregistrée) ; `keep(name): void` (frappes envoyées tout de suite, ou dernier texte tenté renvoyé) ; `forget(name): void` (frappes en attente et écritures déjà en file abandonnées).
  - `openedBehind(s, name)` (`widgetTabs.ts`) ; `placeTouched(s, name, pending)` (pure, `widgetTabs.ts`).
  - `TemplateWidgets.touched(name: string, removed: boolean): void` ; `Callbacks.notice(message: string): void`.
  - `Snapshot.widget_trials: number` (TS) ; `WidgetPane`/`WidgetTab` prop `trials: number`.
  - Clé `ui-widgets-removed-by-agent` (fr `L'agent a supprimé le widget { $name }.`, en `The agent deleted widget { $name }.`).

- [ ] **Step 1: Tests purs (échouent)** — `widgetTabs.test.ts` (importer `openedBehind`, `placeTouched`) :

```ts
  test("openedBehind ajoute en fin sans changer l'actif", () => {
    const s = openedBehind(opened(NO_TABS, "a"), "b");
    expect(names(s)).toEqual(["a", "b"]);
    expect(activeName(s)).toBe("a");
  });

  test("openedBehind d'un nom déjà ouvert ne change rien", () => {
    const s = opened(opened(NO_TABS, "a"), "b");
    expect(openedBehind(s, "a")).toBe(s);
  });

  test("placeTouched active le widget si l'actif n'attend rien", () => {
    expect(activeName(placeTouched(opened(NO_TABS, "a"), "b", () => false))).toBe("b");
  });

  test("placeTouched ouvre derrière si l'actif a des frappes en attente", () => {
    expect(activeName(placeTouched(opened(NO_TABS, "a"), "b", (n) => n === "a"))).toBe("a");
  });
```

- [ ] **Step 2: Implémenter** — `widgetTabs.ts`, après `opened` :

```ts
/** Ouvre `name` en fin de barre sans l'activer (déjà ouvert : inchangé). */
export function openedBehind(s: WidgetTabs, name: string): WidgetTabs {
  if (s.tabs.some((t) => t.name === name)) return s;
  return { ...s, tabs: [...s.tabs, { id: s.next, name }], next: s.next + 1 };
}

/** Ouvre `name` écrit par l'agent : devant, sauf si l'actif a des frappes en attente. */
export function placeTouched(s: WidgetTabs, name: string, pending: (name: string) => boolean): WidgetTabs {
  const current = activeName(s);
  return current !== null && current !== name && pending(current) ? openedBehind(s, name) : opened(s, name);
}
```

Run: `cd ui && bun test src/widgetTabs.test.ts` — Expected: PASS.

- [ ] **Step 3: `memory` expose ses fichiers** — `ui/src/test/widgetApi.ts`, fin de `memory` : `return { fake, log, calls, files };` (le dictionnaire passé, modifié par les faux `setWidget`/`removeWidget`).

- [ ] **Step 4: Tests du hook (échouent)** — `useTemplateWidgets.render.test.tsx` (assistants du fichier : `mount(fake, snapOf(…))` → `{ out, render, names, text, typeIn, … }`, `memory`, `wait`, `later`). `Out` gagne `notices: string[]`, et les callbacks de `mount` `notice: (m: string) => out.notices.push(m)` ; importer `const { t } = await import("./i18n");`.

```ts
describe("useTemplateWidgets, l'agent écrit", () => {
  test("touched ouvre et active le widget écrit", async () => {
    const { fake } = memory({ kpi: "k", quote: "q" });
    const m = await mount(fake, snapOf({}));
    await act(async () => m.out.w!.touched("quote", false));
    expect(m.out.w!.activeName).toBe("quote");
  });

  test("touched ouvre derrière si l'actif a des frappes en attente", async () => {
    const { fake } = memory({ kpi: "k", quote: "q" });
    const m = await mount(fake, snapOf({}));
    await act(async () => m.out.w!.open("kpi"));
    await wait(10);
    await m.typeIn("kpi", "k2"); // < 400 ms : la frappe attend encore
    await act(async () => m.out.w!.touched("quote", false));
    expect(m.out.w!.activeName).toBe("kpi");
  });

  test("après Changed puis touched, les frappes en attente l'emportent sur l'agent", async () => {
    const mem = memory({ kpi: "k" });
    const m = await mount(mem.fake, snapOf({}));
    await act(async () => m.out.w!.open("kpi"));
    await wait(10);
    await m.typeIn("kpi", "tapé");
    mem.files.kpi = "agent";
    await m.render(snapOf({ version: "v2" }));
    await act(async () => m.out.w!.touched("kpi", false));
    await later();
    expect(mem.files.kpi).toBe("tapé");
  });

  test("après touched, la relecture ne remet pas le texte de l'agent", async () => {
    const mem = memory({ kpi: "k" });
    const m = await mount(mem.fake, snapOf({}));
    await act(async () => m.out.w!.open("kpi"));
    await wait(10);
    await m.typeIn("kpi", "tapé");
    mem.files.kpi = "agent";
    await act(async () => m.out.w!.touched("kpi", false));
    await later();
    await m.render(snapOf({ version: "v3" }));
    expect(m.text("kpi")).toBe("tapé");
  });

  test("touched d'un widget nouveau l'ouvre une fois la liste relue", async () => {
    const mem = memory({ kpi: "k" });
    const m = await mount(mem.fake, snapOf({}));
    mem.files.carte = "c";
    await act(async () => m.out.w!.touched("carte", false));
    await m.render(snapOf({ version: "v2" }));
    expect(m.names()).toEqual(["carte"]);
  });

  test("touched removed ferme le sous-onglet", async () => {
    const mem = memory({ kpi: "k" });
    const m = await mount(mem.fake, snapOf({}));
    await act(async () => m.out.w!.open("kpi"));
    await wait(10);
    delete mem.files.kpi;
    await act(async () => m.out.w!.touched("kpi", true));
    expect(m.names()).toEqual([]);
  });

  test("touched removed abandonne les frappes en attente", async () => {
    const mem = memory({ kpi: "k" });
    const m = await mount(mem.fake, snapOf({}));
    await act(async () => m.out.w!.open("kpi"));
    await wait(10);
    await m.typeIn("kpi", "tapé");
    delete mem.files.kpi;
    await act(async () => m.out.w!.touched("kpi", true));
    await later();
    expect(mem.log.some((l) => l.startsWith("set kpi:"))).toBe(false);
  });

  test("touched removed prévient l'utilisateur", async () => {
    const mem = memory({ kpi: "k" });
    const m = await mount(mem.fake, snapOf({}));
    await act(async () => m.out.w!.touched("kpi", true));
    expect(m.out.notices).toEqual([t("ui-widgets-removed-by-agent", { name: "kpi" })]);
  });

  test("hors mode modèles, touched ne laisse rien à ouvrir plus tard", async () => {
    const mem = memory({ kpi: "k" });
    const m = await mount(mem.fake, snapOf({ set: null }));
    await act(async () => m.out.w!.touched("kpi", false));
    await m.render(snapOf({}));
    expect(m.names()).toEqual([]);
  });

  test("l'aperçu actif est redemandé quand la révision des essais change", async () => {
    const { fake, calls } = memory({ kpi: "k" });
    const m = await mount(fake, snapOf({}));
    await act(async () => m.out.w!.open("kpi"));
    await wait(10);
    const before = calls.filter((c) => c === "preview").length;
    await m.render({ ...snapOf({}), widget_trials: 1 } as Snapshot);
    expect(calls.filter((c) => c === "preview").length).toBe(before + 1);
  });
});
```

(`later()` = `wait(PREVIEW_DELAY_MS + 50)` ; l'ajouter en tête s'il n'y est pas.) Le `Probe` de `mount` passe `trials: snap.widget_trials ?? 0` à `WidgetPane`. `WidgetPane.render.test.tsx` : ses callbacks reçoivent `notice: () => {}`, et `createElement(WidgetPane, { api, widgets, trials: 0 })`. `WidgetTab.render.test.tsx` : `interface Shown` gagne `trials?: number`, `Probe({ …, trials = 0 })` le passe à `WidgetTab`. Run: `cd ui && bun test src/useTemplateWidgets.render.test.tsx` — Expected: FAIL.

- [ ] **Step 5: `useWidgetFiles`.** Interface `WidgetFiles` :

```ts
  /** Frappes non envoyées, écriture en cours, ou dernière écriture non enregistrée. */
  pending(name: string): boolean;
  /** Les frappes de `name` gagnent : envoyées tout de suite, ou le dernier texte tenté renvoyé. */
  keep(name: string): void;
  /** Abandonne les frappes de `name` et ses écritures en file (fichier supprimé ailleurs). */
  forget(name: string): void;
```

Dans le hook, à côté des autres références :

```ts
  /** Dernier texte tenté par fichier (enregistré ou non). */
  const attempted = useRef(new Map<string, string>());
  /** Génération d'abandon par fichier : une écriture en file d'une génération passée ne part pas. */
  const forgotten = useRef(new Map<string, number>());
```

Dans la fonction d'écriture `write(name, text)` (celle qui appelle `apiRef.current.setWidget`), en tête : `attempted.current.set(name, text); const born = forgotten.current.get(name) ?? 0;` ; et, dans la tâche qu'elle met en file (`enqueue((live) => …)`), avant l'appel à `setWidget` : `if ((forgotten.current.get(name) ?? 0) !== born) return Promise.resolve(false);`. Puis, après `flush` :

```ts
  const pending = useCallback(
    (name: string) => unsent.current.has(name) || unsavedNow.current.has(name) || (writing.current.get(name) ?? 0) > 0,
    [],
  );
  const keep = useCallback(
    (name: string) => {
      if (unsent.current.has(name)) {
        pushes.current.get(name)?.flush();
        return;
      }
      const last = attempted.current.get(name);
      if (unsavedNow.current.has(name) && last !== undefined) {
        edit(name, last);
        pushes.current.get(name)?.flush();
      }
    },
    [edit],
  );
  const forget = useCallback(
    (name: string) => {
      forgotten.current.set(name, (forgotten.current.get(name) ?? 0) + 1);
      pushes.current.get(name)?.cancel();
      pushes.current.delete(name);
      for (const r of [unsent, typing, sent, attempted, moves, trials, reread]) r.current.delete(name);
      markUnsaved(name, false);
    },
    [markUnsaved],
  );
```

et `pending, keep, forget` dans l'objet renvoyé et sa liste de dépendances. (`moves`, `trials`, `reread` sont les références que la remise à zéro du hook vide déjà ; garder celles qui sont des `Map`/`Set` indexées par nom.)

- [ ] **Step 6: `useTemplateWidgets`.** `Callbacks` : `/** Information (pas une erreur) pour l'utilisateur. */ notice(message: string): void;`. Interface : `/** L'agent a écrit (ou supprimé) `name` : l'ouvrir, sans écraser les frappes. */ touched(name: string, removed: boolean): void;`. Avant la branche de rendu des statuts :

```ts
  /** Écrits par l'agent, pas encore dans la liste des fichiers : ouverts quand elle les contient. */
  const [awaiting, setAwaiting] = useState<ReadonlySet<string>>(new Set());
```

Dans la branche qui détecte un changement de pack (celle qui compare `pack` au pack vu), ajouter `setAwaiting(new Set());`. Dans la branche `else if (pack !== null && files.statuses && files.statusesFor === revision)`, juste après `const names = …` et avant la restauration :

```ts
    const arrived = [...awaiting].filter((n) => names.has(n));
    if (arrived.length > 0 && restoredFor === pack) {
      setTabs((s) => arrived.reduce((acc, n) => placeTouched(acc, n, files.pending), s));
      setAwaiting((a) => new Set([...a].filter((n) => !names.has(n))));
    }
```

(la restauration passe d'abord, au rendu suivant ces noms s'ouvrent). Après `removed` :

```ts
  const touched = useCallback(
    (name: string, gone: boolean) => {
      if (packRef.current === null) return;
      if (gone) {
        files.forget(name);
        removed(name);
        setAwaiting((a) => (a.has(name) ? new Set([...a].filter((n) => n !== name)) : a));
        onRef.current.notice(t("ui-widgets-removed-by-agent", { name }));
        return;
      }
      if (files.pending(name)) files.keep(name);
      if (files.statuses?.some((s) => s.name === name)) setTabs((s) => placeTouched(s, name, files.pending));
      else setAwaiting((a) => (a.has(name) ? a : new Set(a).add(name)));
    },
    [files.forget, files.keep, files.pending, files.statuses, removed],
  );
```

(`files.forget`, `keep`, `pending` sont stables ; `onRef` est la référence des callbacks du hook.) `touched` dans l'objet renvoyé. Importer `placeTouched` de `./widgetTabs` et `t` de `./i18n`.

- [ ] **Step 7: Aperçu relancé par les essais.** `api.ts`, `Snapshot` : `widget_trials: number`. `WidgetTab.tsx` : prop `/** Révision des essais (commune au pack) : l'aperçu se relance quand elle change. */ trials: number`, ajoutée aux dépendances de l'effet d'aperçu. `WidgetPane.tsx` : prop `trials: number` (même doc), transmise à chaque `WidgetTab`.

- [ ] **Step 8: `App.tsx`.** Appel `useTemplateWidgets(api, snap, { … })` : ajouter `notice: (m) => say(m)`. Rendu : `<WidgetPane api={api} widgets={widgets} trials={snap.widget_trials} />`. Avant l'abonnement : `const touchedRef = useRef(widgets.touched); touchedRef.current = widgets.touched;` (même idiome que `snapRef`) ; dans `onMcp`, après le cas `focus` : `if (e.kind === "widget_touched") touchedRef.current(e.name, e.removed);`.

- [ ] **Step 9: i18n** — `ui.ftl` (20 langues) : `ui-widgets-removed-by-agent`.

- [ ] **Step 10: Vérifier** — Run: `cd ui && bun test && bunx tsc --noEmit -p . && wc -l src/App.tsx` — Expected: PASS (comparer un éventuel échec de la suite complète à `main` : `git stash && bun test 2>&1 | grep -c "(fail)" && git stash pop`), App < 1000 lignes. `cargo nextest run -p oku-slides-i18n` — Expected: PASS.

- [ ] **Step 11: Commit**

```bash
git add ui/src crates/oku-slides-i18n/src/locales
```
```bash
git diff --cached --stat
```
```bash
git commit -m "feat(ui) : un widget écrit par l'agent s'ouvre sans écraser les frappes ; l'aperçu suit les essais (t-423)"
```

---

## Phase B — après la fusion de t-416 (tâche 11, dock et catalogue v3)

**Avant de commencer — à vérifier dans le code fusionné de t-416** ; si une forme diffère, mettre ce plan à jour d'abord (G9), puis exécuter :
- `Surface` : champs `turn`, `live`, `place` (`Place::Thread`) ; `forms::params(id, spec, palette, l)`, `live: true`, le bouton « × » (`dismiss`), et l'usage de `spec.slide` (ancrage) : une surface de l'hôte a `slide` vide pour un widget — vérifier que `routeSurfaces`/`AgentDock` ne l'écartent pas.
- Le catalogue v3 (`ui/src/a2ui/catalog.tsx`) : forme d'un composant, `CommonSchemas.DynamicValue`, le schéma `Hex`, la classe de légende des cartes (`a2ui-hint` ou `postit-hint`), le fichier CSS des cartes.
- `routeSurfaces` → `Routed.dock` ; `AgentThread` et sa prop `dock` ; l'attribut `data-surface-id` posé sur chaque surface ; si le dock reconstruit une surface relue au lieu de mettre à jour son modèle de données (un brouillon en cours serait perdu).
- `ui/e2e/panel.cjs` tel que t-416 l'a laissé (`.notes .a2ui-card`).

Vérifié le 2026-10-04 sur `main` (50d19c79, t-416 = 39d34c50) : `Surface { id, place, anchor, turn, live, messages }`, `forms::params` → `Place::Thread`, `live: true`, ancre `None`, « × » `dismiss` ; `AgentThread` reçoit `dock` (`Pick<Routed, "turns" | "loose" | "dock">`) et rend `AgentDock`, qui pose `data-surface-id` ; un `ask-*` en attente replie les autres surfaces du dock ; catalogue v3 (`CATALOG_ID`), légende `a2ui-hint`, cartes dans `ui/src/a2ui/a2ui.css`, `Hex` dans `ui/src/a2ui/schemas.ts`, `CommonSchemas.DynamicValue` disponible ; `SurfaceView` reconstruit une surface dont la liste de messages ne prolonge pas la précédente ; `PanelSurface`, `Request::Panel`, `panel.rs` (messages bruts, pas de `Surface`) et `panelActions` sont toujours là ; `fromA2ui` vient de `./agent/quick` ; `run(p: Promise<Snapshot | null>)`, `say(text, error = false)`.
- `fromA2ui` (`ui/src/agent/quick.ts`) et la forme d'une `A2uiClientAction`.
- `CATALOG_ID` : un composant ajouté au catalogue v3 en demande-t-il une nouvelle version ? Décider ici et le noter dans la tâche 6.

### Task 6: Composant A2UI `RichTextField` et champ `rich` de `show_params`

**Files:**
- Create: `ui/src/markdownEditing.ts`, `ui/src/a2ui/RichTextField.tsx`, `ui/src/a2ui/RichTextField.render.test.tsx`, `ui/src/test/surface.ts`
- Modify: `ui/src/TextEditor.tsx` (`applyEdit` et les touches de mise en forme déplacés), `ui/src/a2ui/catalog.tsx`, `ui/src/a2ui/catalog.test.tsx`, `ui/src/a2ui/catalog.render.test.tsx` (assistants déplacés vers `test/surface.ts`), `ui/src/styles.css`, `ui/src/useTemplateWidgets.ts` (garde de champ des raccourcis), `crates/oku-slides-a2ui/src/message.rs`, `crates/oku-slides-a2ui/src/message_tests.rs`, `crates/oku-slides-a2ui/src/lib.rs` (doc), `crates/oku-slides-session/src/agent/forms.rs`, `crates/oku-slides-session/src/agent/forms_tests.rs`, `crates/oku-slides-session/src/agent/variants_tests.rs` (ou le fichier de tests de `params_surface`), `ui/src/a2ui/fixtures/agent-surfaces.json` (régénéré)

**Interfaces:**
- Produces:
  - `ComponentKind::RichTextField { label: Dynamic<String>, value: Dynamic<String>, source: Dynamic<Value>, palette: Vec<String>, action: Option<Action> }` : `value` lit le markdown d'édition, `source` les paragraphes d'origine ; à la validation, l'interface écrit les deux, et l'action part.
  - `FieldKind::Rich` (`"rich"`) dans `show_params` : sa `value` est une liste de paragraphes ; `forms::params` met `paragraphs_to_text(value)` en `/{clé}` et la liste en `/{clé}-source`, et l'action envoie `/{clé}-source`.
  - `ui/src/markdownEditing.ts` : `applyEdit(view, edit, userEvent)` et `markdownFormatKeys(): KeyBinding[]` (⌘B, ⌘I, ⌘K), utilisés par `TextEditor` et `RichTextField`.
  - Composant `export function RichTextField(props)` ; `ui/src/test/surface.ts` : `mountSurface(messages)` et `messagesOf(components, data)` partagés par les tests du catalogue.

- [ ] **Step 1: Tests Rust (échouent).** `message_tests.rs`, sur le modèle de `text_field_binds_value_and_sends_action_with_path_context` :

```rust
#[test]
fn rich_text_field_binds_markdown_and_source() {
    let c = Component::new("b", ComponentKind::RichTextField {
        label: Dynamic::Literal("Corps".into()),
        value: Dynamic::path("/v"),
        source: Dynamic::path("/v-source"),
        palette: vec!["#112233".into()],
        action: None,
    });
    assert_that!(serde_json::to_value(&c).unwrap()).is_equal_to(json!({
        "id": "b", "component": "RichTextField", "label": "Corps",
        "value": { "path": "/v" }, "source": { "path": "/v-source" }, "palette": ["#112233"]
    }));
}
```

`forms_tests.rs` :

```rust
fn rich_spec() -> ShowParams {
    serde_json::from_value(json!({ "title": "T", "slide": "s-1", "fields": [{
        "label": "Corps", "kind": "rich", "value": [{ "runs": [{ "t": "Un", "bold": true }] }],
        "op_template": { "op": "set_param", "target": { "slide": "s-1" }, "param": "body", "value": "$value" }
    }] })).unwrap()
}

#[test]
fn params_with_a_rich_field_shows_its_markdown() {
    let s = params("show-1", &rich_spec(), &[], &fr());
    assert_that!(serde_json::to_string(&s.messages).unwrap().as_str()).contains("**Un**");
}

#[test]
fn params_with_a_rich_field_sends_its_source_paragraphs() {
    let s = params("show-1", &rich_spec(), &[], &fr());
    assert_that!(serde_json::to_string(&s.messages).unwrap().as_str()).contains("\"path\":\"/f0-source\"");
}
```

(La clé de champ est celle de `param_key(0)` ; si elle n'est pas `f0`, ajuster la chaîne.) Et, dans les tests de `params_surface` (à côté des tests existants de `show_params`), un test qui construit une surface `show_params` avec ce champ `rich` sur une slide du fixture ayant un paramètre `rich` et attend `is_ok()`. Ajouter un champ `rich` à `every_form()` (même forme). Run: `cargo nextest run -p oku-slides-a2ui -p oku-slides-session rich` — Expected: FAIL.

- [ ] **Step 2: Implémenter (Rust).** `message.rs`, après `TextField` :

```rust
    /// Texte riche multiligne : `value` lit le markdown montré, `source` la
    /// valeur structurée d'origine ; à la validation, l'interface écrit les
    /// deux, puis `action` part.
    RichTextField {
        label: Dynamic<String>,
        value: Dynamic<String>,
        source: Dynamic<Value>,
        palette: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<Action>,
    },
```

`crates/oku-slides-a2ui/src/lib.rs` : `RichTextField` dans la liste des composants de la doc. Dans `message.rs`, les trois `match` exhaustifs sur `ComponentKind` (enfants, actions, gel `action: None`) reçoivent le bras `RichTextField` (pas d'enfant ; son action comme celle de `TextField`). `forms.rs` : `FieldKind` gagne `/// Texte riche : paragraphes, montrés en markdown d'édition. Rich,` ; dans `params`, le bras :

```rust
            FieldKind::Rich => {
                let source = format!("{path}-source");
                let paragraphs: Vec<Paragraph> = serde_json::from_value(f.value.clone()).unwrap_or_default();
                data.insert(format!("{key}-source"), f.value.clone());
                markdown = Some(paragraphs_to_text(&paragraphs));
                let action = Some(Action::event("param", [("field", lit(json!(i))), ("value", Dynamic::path(&source))]));
                ComponentKind::RichTextField {
                    label,
                    value: Dynamic::path(&path),
                    source: Dynamic::path(&source),
                    palette: palette.to_vec(),
                    action,
                }
            }
```

et, après le `match`, `data.insert(key.clone(), markdown.take().map_or_else(|| f.value.clone(), Value::from));` à la place de `data.insert(key.clone(), f.value.clone());` (`let mut markdown: Option<String> = None;` en tête de la boucle ; importer `Paragraph` et `crate::text::paragraphs_to_text`).

- [ ] **Step 3: Régénérer le contrat** — Run: `OKU_UPDATE_FIXTURES=1 cargo nextest run -p oku-slides-session every_form && cargo nextest run -p oku-slides-a2ui -p oku-slides-session` — Expected: PASS, `ui/src/a2ui/fixtures/agent-surfaces.json` mis à jour.

- [ ] **Step 4: Édition markdown partagée** — `ui/src/markdownEditing.ts` :

```ts
// Édition du markdown d'édition partagée par l'éditeur de la scène et le
// champ riche des cartes : une édition pure appliquée en un pas d'annulation,
// et les touches de mise en forme (⌘B, ⌘I, ⌘K).

import { isolateHistory } from "@codemirror/commands";
import type { KeyBinding } from "@codemirror/view";
import type { EditorView } from "@codemirror/view";
import type { Edit } from "./textEdits";
import { insertLink, minimalChange, toggleEmphasis } from "./textEdits";

/** Applique une édition pure : seul le texte changé, en un pas d'annulation à part. */
export function applyEdit(view: EditorView, edit: (value: string, start: number, end: number) => Edit, userEvent: string): boolean {
  const { from, to } = view.state.selection.main;
  const before = view.state.doc.toString();
  const e = edit(before, from, to);
  view.dispatch({
    changes: minimalChange(before, e.value),
    selection: { anchor: e.start, head: e.end },
    userEvent,
    annotations: isolateHistory.of("full"),
  });
  return true;
}

/** ⌘B, ⌘I, ⌘K sur le markdown d'édition. */
export function markdownFormatKeys(): KeyBinding[] {
  const format = (edit: (value: string, start: number, end: number) => Edit) => (view: EditorView) =>
    applyEdit(view, edit, "input.format");
  return [
    { key: "Mod-b", run: format((s, a, b) => toggleEmphasis(s, a, b, "bold")) },
    { key: "Mod-i", run: format((s, a, b) => toggleEmphasis(s, a, b, "italic")) },
    { key: "Mod-k", run: format(insertLink) },
  ];
}
```

`TextEditor.tsx` : supprimer sa fonction `applyEdit` (l'importer de `./markdownEditing`), et, dans la branche riche des touches, remplacer les trois liaisons ⌘B/⌘I/⌘K par `...markdownFormatKeys()`. Run: `cd ui && bun test src/TextEditor.test.ts src/Stage.render.test.tsx` — Expected: PASS (comportement inchangé).

- [ ] **Step 5: Assistants de surface partagés** — déplacer de `catalog.render.test.tsx` vers `ui/src/test/surface.ts` le montage d'une `SurfaceView` et la construction des messages (`createSurface`, `updateComponents`, `updateDataModel`), exportés en `mountSurface(messages) → { host, actions, errors }` et `messagesOf(components, data)` ; `catalog.render.test.tsx` les importe.

- [ ] **Step 6: Tests TS (échouent)** — `ui/src/a2ui/RichTextField.render.test.tsx` :

```ts
// Champ riche d'une carte : markdown mis en forme, validé en paragraphes.

import { describe, expect, test } from "bun:test";
import "../test/dom";

const { act } = await import("react");
const { EditorView } = await import("@codemirror/view");
const { mountSurface, messagesOf } = await import("../test/surface");

const PARAS = [{ runs: [{ t: "Un", color: "#ff0000" }] }, { runs: [{ t: "Deux" }] }];
const rich = () =>
  messagesOf(
    [{ id: "f", component: "RichTextField", label: "Corps", value: { path: "/v" }, source: { path: "/s" }, palette: [],
       action: { event: { name: "param", context: { value: { path: "/s" } } } } }],
    { v: "Un\nDeux", s: PARAS },
  );

async function editor() {
  const m = await mountSurface(rich());
  const content = m.host.querySelector(".cm-content") as HTMLElement;
  const view = EditorView.findFromDOM(content)!;
  const type = (text: string) => act(async () => view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text } }));
  const blur = () => act(async () => content.dispatchEvent(new FocusEvent("blur")));
  const key = (k: string) => act(async () => content.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true })));
  return { ...m, view, type, blur, key };
}

describe("RichTextField", () => {
  test("montre une ligne par paragraphe", async () => {
    const m = await editor();
    expect(m.host.querySelectorAll(".cm-line").length).toBe(2);
  });

  test("met le gras en forme", async () => {
    const m = await editor();
    await m.type("**Un**");
    expect(m.host.querySelector(".cm-line .cm-md-bold")).not.toBeNull();
  });

  test("à la perte du focus, envoie des paragraphes", async () => {
    const m = await editor();
    await m.type("Un\nTrois");
    await m.blur();
    expect(m.actions.at(-1)?.context?.value).toEqual([PARAS[0], { runs: [{ t: "Trois" }] }]);
  });

  test("une ligne inchangée garde sa couleur", async () => {
    const m = await editor();
    await m.type("Un\nTrois");
    await m.blur();
    expect((m.actions.at(-1)?.context?.value as typeof PARAS)[0]).toEqual(PARAS[0]);
  });

  test("Échap rétablit le texte", async () => {
    const m = await editor();
    await m.type("Autre");
    await m.key("Escape");
    expect(m.view.state.doc.toString()).toBe("Un\nDeux");
  });

  test("Échap n'envoie rien", async () => {
    const m = await editor();
    await m.type("Autre");
    await m.key("Escape");
    expect(m.actions).toHaveLength(0);
  });
});
```

(La forme de `m.actions[i]` suit `mountSurface` ; adapter l'accès à `context.value` à celle d'une `A2uiClientAction`.) `catalog.test.tsx` : `"RichTextField"` dans la liste des composants du catalogue. Run: `cd ui && bun test src/a2ui` — Expected: FAIL.

- [ ] **Step 7: Implémenter (TS)** — `ui/src/a2ui/RichTextField.tsx` :

```tsx
// Texte riche d'une carte de réglages : CodeMirror multiligne, markdown
// d'édition mis en forme à la frappe (comme l'éditeur de la scène). Validé à
// la perte du focus ou ⌘↩ : des paragraphes, construits à partir de ceux
// d'origine (couleur et taille d'une ligne inchangée gardées) ; Échap rétablit.

import { useEffect, useLayoutEffect, useRef } from "react";
import { history, historyKeymap, standardKeymap } from "@codemirror/commands";
import { Compartment, EditorState, Transaction } from "@codemirror/state";
import { EditorView, keymap } from "@codemirror/view";
import type { Paragraph } from "../api";
import { textToParagraphs } from "../deck";
import { markdownDecorations } from "../markdownDecorations";
import { markdownFormatKeys } from "../markdownEditing";

export interface RichTextFieldProps {
  label: string;
  /** Markdown d'édition montré. */
  value: string;
  /** Paragraphes d'origine, alignés ligne à ligne sur `value`. */
  source: Paragraph[];
  palette: string[];
  /** Validation : le markdown et ses paragraphes. */
  onCommit: (markdown: string, paragraphs: Paragraph[]) => void;
}

/** Champ riche multiligne d'une carte. */
export function RichTextField({ label, value, source, palette, onCommit }: RichTextFieldProps) {
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);
  const looks = useRef(new Compartment());
  const shown = useRef({ text: value, paragraphs: source });
  const commitRef = useRef(onCommit);
  useLayoutEffect(() => {
    commitRef.current = onCommit;
  });

  useEffect(() => {
    const parent = host.current;
    if (!parent) return;
    const commit = (v: EditorView) => {
      const text = v.state.doc.toString();
      if (text === shown.current.text) return true;
      const paragraphs = textToParagraphs(text, shown.current);
      shown.current = { text, paragraphs };
      commitRef.current(text, paragraphs);
      return true;
    };
    const revert = (v: EditorView) => {
      v.dispatch({
        changes: { from: 0, to: v.state.doc.length, insert: shown.current.text },
        annotations: Transaction.addToHistory.of(false),
      });
      v.contentDOM.blur();
      return true;
    };
    const v = new EditorView({
      parent,
      state: EditorState.create({
        doc: shown.current.text,
        extensions: [
          keymap.of([
            { key: "Mod-Enter", run: (target) => (target.contentDOM.blur(), true) },
            { key: "Escape", run: revert },
            ...markdownFormatKeys(),
          ]),
          history(),
          keymap.of([...historyKeymap, ...standardKeymap]),
          EditorView.lineWrapping,
          looks.current.of([]),
          EditorView.domEventHandlers({ blur: (_e, target) => void commit(target) }),
        ],
      }),
    });
    view.current = v;
    return () => {
      // Démonté pendant une saisie : le texte tapé part quand même.
      if (v.hasFocus) commit(v);
      view.current = null;
      v.destroy();
    };
  }, []);

  // Libellé et palette suivent la langue et le thème.
  useEffect(() => {
    view.current?.dispatch({
      effects: looks.current.reconfigure([
        markdownDecorations(palette),
        EditorView.contentAttributes.of({ "aria-label": label, "aria-multiline": "true" }),
      ]),
    });
  }, [label, palette]);

  // Une nouvelle valeur de la session, hors saisie.
  useEffect(() => {
    const v = view.current;
    if (!v || v.hasFocus || value === shown.current.text) return;
    shown.current = { text: value, paragraphs: source };
    v.dispatch({ changes: { from: 0, to: v.state.doc.length, insert: value }, annotations: Transaction.addToHistory.of(false) });
  }, [value, source]);

  return (
    <div className="note-field rich-field" onKeyDown={(e) => e.stopPropagation()}>
      <span className="a2ui-hint">{label}</span>
      <div ref={host} className="rich-field-editor" />
    </div>
  );
}
```

(Vérifié avant la phase B : la légende des champs du catalogue v3 est `a2ui-hint`, les styles des cartes vivent dans `ui/src/a2ui/a2ui.css`, `Hex` s'importe de `./schemas`.) Dans `catalog.tsx`, à côté de `TextFieldApi` :

```tsx
const RichTextFieldApi = {
  name: "RichTextField",
  schema: z
    .object({
      label: CommonSchemas.DynamicString,
      value: CommonSchemas.DynamicString,
      source: CommonSchemas.DynamicValue,
      palette: z.array(Hex).readonly(),
      action: CommonSchemas.Action.optional(),
    })
    .strict(),
};

/** Texte riche multiligne : markdown et paragraphes écrits, puis l'action. */
const RichText = createComponentImplementation(RichTextFieldApi, ({ props }) => (
  <RichTextField
    label={props.label}
    value={props.value ?? ""}
    source={Array.isArray(props.source) ? (props.source as Paragraph[]) : []}
    palette={[...props.palette]}
    onCommit={(markdown, paragraphs) => {
      props.setValue(markdown);
      props.setSource(paragraphs);
      props.action?.();
    }}
  />
));
```

et `RichText` dans la liste des composants. `ui/src/a2ui/a2ui.css` (près de `.note-field`) : `.rich-field-editor { min-height: 4.5em; max-height: 12em; overflow: auto; }`. `useTemplateWidgets.ts`, garde des raccourcis ⌘W/⌘Z : `const inField = !inEditor && !!target?.closest?.('input, textarea, select, [contenteditable="true"]');` (comme `App.tsx`).

- [ ] **Step 8: Vérifier** — Run: `cargo nextest run -p oku-slides-a2ui -p oku-slides-session && cd ui && bun test && bunx tsc --noEmit -p .` — Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add crates/oku-slides-a2ui crates/oku-slides-session/src/agent ui/src
```
```bash
git diff --cached --stat
```
```bash
git commit -m "feat(a2ui) : composant RichTextField (markdown mis en forme, multiligne) et champ rich de show_params (t-423)"
```

---

### Task 7: Réglages de l'hôte (session), en plus de `panel`

**Files:**
- Modify: `crates/oku-slides-session/src/widget_params.rs` (`widget_settings`), `crates/oku-slides-session/src/widget_params_tests.rs`, `crates/oku-slides-session/src/panel.rs` (+ `panel_tests.rs`), `crates/oku-slides-session/src/agent/forms.rs` (`FormExtras`, `params_with`), `crates/oku-slides-session/src/agent/forms_tests.rs`, `crates/oku-slides-session/src/request.rs`, `crates/oku-slides-session/src/lib.rs` (bras de `handle` ; drapeaux `changes`/`documents`), `crates/oku-slides-session/src/lib_tests.rs`, `crates/oku-slides-session/src/error.rs`, `crates/oku-slides-session/tests/session.rs`, `crates/oku-slides-session/ARCHITECTURE.md`, `crates/oku-slides-i18n/src/locales/*/session.ftl`, `ui/src/a2ui/fixtures/agent-surfaces.json` (régénéré)

**Interfaces:**
- Consumes: `set_widget_trials`, `settable` (tâche 2) ; `FieldKind::Rich` (tâche 6) ; `Slide::resolved_grid`, `GRID_TRACKS` (tâche 1) ; `Session::param_op`.
- Produces:
  - `pub(crate) struct FormExtras<'a> { pub keys: &'a [String], pub notes: &'a [String], pub closable: bool }` ; `forms::params_with(id, spec, extras, palette, l)` (l'action d'un champ porte `key` quand `keys[i]` existe ; notes en légendes ; « × » si `closable`) ; `forms::params` délègue. `params_with` ne lit ni `spec.slide` ni `op_template`.
  - `widget_params.rs` : `pub(crate) fn Session::widget_settings(&self, name: &str) -> Result<Option<HostParams>>` (`None` pour Commun ; carte « fichier invalide » si le fichier ne compile pas).
  - `panel.rs` : `pub(crate) const HOST_TEMPLATE`, `HOST_WIDGET` ; `pub(crate) struct HostParams { spec, keys, notes }` ; `pub(crate) fn template_params(slide, grid: Option<Grid>, l) -> HostParams` ; `impl Session { pub(crate) fn host_settings(&self, slide, widget) -> Result<Option<Surface>>; pub(crate) fn host_param(&mut self, slide, key, value) -> Result<()> }`.
  - Requêtes ajoutées (`Panel` reste jusqu'à la tâche 8) : `HostSettings { slide, widget }` (lecture), `HostParam { slide, key, value }` (change le deck ; refusée hors mode modèles et en historique), `SetWidgetTrial { name, param, value }` (ne change ni le deck ni les documents ; refusée en historique) ; réponse des deux dernières : l'instantané.
  - Erreur `HostKey { key }` ; clés `session-widget-params`, `session-widget-no-params`, `session-widget-param-fixed`, `session-widget-param-yes`, `session-widget-param-no`, `session-widget-invalid-file`, `session-host-key`, `session-undo-rename-template`, `session-undo-describe-template`, `session-undo-grid`.

- [ ] **Step 1: Tests (échouent).** `widget_params_tests.rs` (fichier `ALL` de la tâche 2 ; `HostParams` depuis `crate::panel`) :

```rust
fn settings(s: &crate::Session, name: &str) -> crate::panel::HostParams {
    s.widget_settings(name).unwrap().unwrap()
}

#[test]
fn widget_settings_keys_follow_the_schema_order() {
    let (_d, s) = templates();
    assert_that!(settings(&s, "tout").keys).is_equal_to(
        ["title", "body", "count", "size", "tone", "on"].map(String::from).to_vec(),
    );
}

#[rstest]
#[case::plain(0, crate::agent::FieldKind::Text)]
#[case::rich(1, crate::agent::FieldKind::Rich)]
#[case::bounded_number(2, crate::agent::FieldKind::Slider)]
#[case::open_number(3, crate::agent::FieldKind::Text)]
#[case::choice(4, crate::agent::FieldKind::Select)]
#[case::flag(5, crate::agent::FieldKind::Select)]
fn widget_settings_field_kind_matches_the_param_type(#[case] i: usize, #[case] kind: crate::agent::FieldKind) {
    let (_d, s) = templates();
    assert_that!(settings(&s, "tout").spec.fields.get(i).map(|f| f.kind)).is_equal_to(Some(kind));
}

#[test]
fn widget_settings_lists_an_unsettable_param_as_a_note() {
    let (_d, s) = templates();
    assert_that!(settings(&s, "tout").notes.join(" ").as_str()).contains("logo");
}

#[test]
fn widget_settings_value_is_the_preview_value() {
    let (_d, s) = templates();
    let l = oku_slides_i18n::Localizer::new(Some("fr"));
    assert_that!(settings(&s, "tout").spec.fields[0].value.clone()).is_equal_to(json!(l.tr("widget-preview-plain")));
}

#[test]
fn widget_settings_value_shows_the_trial() {
    let (_d, mut s) = templates();
    s.set_widget_trials("tout", values(json!({ "on": "true" }))).unwrap();
    assert_that!(settings(&s, "tout").spec.fields[5].value.clone()).is_equal_to(json!("true"));
}

#[test]
fn widget_settings_slider_step_fits_a_small_range() {
    let (_d, s) = templates();
    assert_that!(settings(&s, "tout").spec.fields[2].step).is_equal_to(Some(0.08));
}

#[test]
fn widget_settings_of_common_is_none() {
    let (_d, mut s) = templates();
    s.set_widget("_commun", "#let x = 1\n".into()).unwrap();
    assert_that!(s.widget_settings("_commun").unwrap()).is_none();
}

#[test]
fn widget_settings_of_a_broken_file_keeps_one_note() {
    let (_d, mut s) = templates();
    s.set_widget("tout", "#let widget = (".into()).unwrap();
    let p = settings(&s, "tout");
    assert_that!((p.spec.fields.len(), p.notes.len())).is_equal_to((0, 1));
}
```

`panel_tests.rs` (et `#[cfg(test)] #[path = "panel_tests.rs"] mod panel_tests;` dans `panel.rs`) :

```rust
use oku_slides_core::{Grid, Slide};
use serde_json::json;
use speculoos::prelude::*;

use super::template_params;

fn l() -> oku_slides_i18n::Localizer {
    oku_slides_i18n::Localizer::new(Some("fr"))
}

fn grid() -> Option<Grid> {
    Some(Grid { columns: 12, rows: 6, gap: None })
}

#[test]
fn template_params_with_grid_has_four_keys() {
    let p = template_params(&Slide::new("t-1", "blank"), grid(), &l());
    assert_that!(p.keys).is_equal_to(["name", "description", "columns", "rows"].map(String::from).to_vec());
}

#[test]
fn template_params_name_field_shows_the_name() {
    let mut slide = Slide::new("t-1", "blank");
    slide.name = Some("Libre".into());
    assert_that!(template_params(&slide, grid(), &l()).spec.fields[0].value.clone()).is_equal_to(json!("Libre"));
}

#[test]
fn template_params_columns_field_applies_set_grid() {
    let p = template_params(&Slide::new("t-1", "blank"), grid(), &l());
    assert_that!(p.spec.fields[2].op_template.clone()).is_equal_to(json!({ "op": "set_grid", "slide": "t-1", "columns": "$value" }));
}

#[test]
fn template_params_grid_fields_are_bounded_by_grid_tracks() {
    let p = template_params(&Slide::new("t-1", "blank"), grid(), &l());
    assert_that!((p.spec.fields[2].min, p.spec.fields[2].max)).is_equal_to((Some(1.0), Some(24.0)));
}

#[test]
fn template_params_without_grid_has_two_keys() {
    assert_that!(template_params(&Slide::new("t-1", "blank"), None, &l()).keys.len()).is_equal_to(2);
}
```

`forms_tests.rs` :

```rust
fn host_spec() -> ShowParams {
    serde_json::from_value(json!({ "title": "T", "slide": "", "fields": [
        { "label": "Titre", "kind": "text", "value": "x", "op_template": null }
    ] })).unwrap()
}

#[test]
fn params_with_keys_puts_the_key_in_the_action() {
    let extras = FormExtras { keys: &["title".into()], notes: &[], closable: false };
    let json = serde_json::to_string(&params_with("host-widget", &host_spec(), &extras, &[], &fr()).messages).unwrap();
    assert_that!(json.as_str()).contains("\"key\"");
}

#[test]
fn params_with_notes_shows_them() {
    let extras = FormExtras { keys: &[], notes: &["n1".into()], closable: false };
    let json = serde_json::to_string(&params_with("host-widget", &host_spec(), &extras, &[], &fr()).messages).unwrap();
    assert_that!(json.as_str()).contains("n1");
}

#[test]
fn params_with_not_closable_has_no_close_button() {
    let extras = FormExtras { keys: &[], notes: &[], closable: false };
    let json = serde_json::to_string(&params_with("host-widget", &host_spec(), &extras, &[], &fr()).messages).unwrap();
    assert_that!(json.contains("dismiss")).is_false();
}
```

et un cas « hôte » dans `every_form()` (`params_with` avec clés, une note, sans « × »). `tests/session.rs` : ajouter `Isolated` à `use common::{…}` et :

```rust
const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
"#;

fn editing() -> (Isolated, String) {
    let mut s = session();
    s.edit_templates(oku_slides_session::GRID_PACK).unwrap();
    let slide = s.snapshot().slides[0].id.to_owned();
    (s, slide)
}

/// La surface de `host_settings`, en JSON.
fn host(s: &mut Session, slide: &str, widget: Option<&str>) -> serde_json::Value {
    let reply = s.handle(Request::HostSettings { slide: slide.into(), widget: widget.map(Into::into) }).unwrap();
    serde_json::from_str(&reply).unwrap()
}

fn host_param(s: &mut Session, slide: &str, key: &str, value: serde_json::Value) -> oku_slides_session::Result<String> {
    s.handle(Request::HostParam { slide: slide.into(), key: key.into(), value })
}

#[test]
fn host_settings_outside_templates_mode_is_null() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    assert_that!(host(&mut s, "s-libre", None)).is_equal_to(json!(null));
}

#[test]
fn host_settings_shows_the_template_name() {
    let (mut s, slide) = editing();
    let name = s.deck().slide(&slide).unwrap().name.clone().unwrap_or_default();
    assert_that!(host(&mut s, &slide, None).to_string().as_str()).contains(&name);
}

#[test]
fn host_settings_of_a_template_with_blocks_has_no_grid_fields() {
    let dir = data_dir();
    let id = uuid::Uuid::new_v4();
    let pack = json!({ "id": id, "name": "À blocs", "created": "2026-10-01T00:00:00Z", "templates": [{
        "id": "cartes", "name": "Cartes", "slide": { "id": "cartes", "kind": "content", "params": {
            "title": "Titre", "blocks": [{ "id": "b", "kind": "text", "params": { "body": [{ "runs": [{ "t": "x" }] }] } }]
        } }
    }] });
    let files = oku_slides_core::PackFiles::from_bytes(pack.to_string().as_bytes(), []).unwrap();
    let library =
        oku_slides_session::Library::new(Some(dir.to_path_buf()), oku_slides_session::Storage::local().unwrap());
    library.add_version(id, &files).unwrap();
    let mut s = session();
    s.set_templates_dir(&*dir);
    s.edit_templates(id).unwrap();
    let cards = s
        .deck()
        .slides
        .iter()
        .find(|sl| sl.params.slot("blocks").is_some_and(|b| !b.is_empty()))
        .map(|sl| sl.id.clone())
        .unwrap();
    assert_that!(host(&mut s, &cards, None).to_string().contains("\"columns\"")).is_false();
}

#[test]
fn host_settings_of_a_widget_shows_its_trial() {
    let (mut s, slide) = editing();
    s.set_widget("quote", QUOTE.into()).unwrap();
    s.handle(Request::SetWidgetTrial { name: "quote".into(), param: "source".into(), value: json!("Essai") }).unwrap();
    assert_that!(host(&mut s, &slide, Some("quote")).to_string().as_str()).contains("Essai");
}

#[test]
fn host_settings_of_a_broken_widget_keeps_the_widget_card() {
    let (mut s, slide) = editing();
    s.set_widget("quote", "#let widget = (".into()).unwrap();
    assert_that!(host(&mut s, &slide, Some("quote"))["id"].as_str()).is_equal_to(Some("host-widget"));
}

#[test]
fn host_settings_of_a_stale_widget_shows_the_template() {
    let (mut s, slide) = editing();
    assert_that!(host(&mut s, &slide, Some("absent"))["id"].as_str()).is_equal_to(Some("host-template"));
}

#[test]
fn host_param_name_renames_the_template() {
    let (mut s, slide) = editing();
    host_param(&mut s, &slide, "name", json!("Renommé")).unwrap();
    assert_that!(s.deck().slide(&slide).unwrap().name.as_deref()).is_equal_to(Some("Renommé"));
}

#[test]
fn host_param_columns_undoes_one_change_at_a_time() {
    let (mut s, slide) = editing();
    host_param(&mut s, &slide, "columns", json!(6)).unwrap();
    let after_first = s.deck().slide(&slide).unwrap().clone();
    host_param(&mut s, &slide, "columns", json!(4)).unwrap();
    s.handle(Request::Undo).unwrap();
    assert_that!(s.deck().slide(&slide).unwrap().clone()).is_equal_to(after_first);
}

#[test]
fn host_param_with_an_unknown_key_is_refused() {
    let (mut s, slide) = editing();
    let err = host_param(&mut s, &slide, "couleur", json!(1)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(oku_slides_session::SessionErrorKind::HostKey);
}

#[test]
fn host_param_outside_templates_mode_is_refused() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let err = host_param(&mut s, "s-libre", "name", json!("x")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(oku_slides_session::SessionErrorKind::NotEditingTemplates);
}

#[test]
fn set_widget_trial_request_leaves_the_deck_unchanged() {
    let (mut s, _) = editing();
    s.set_widget("quote", QUOTE.into()).unwrap();
    let before = s.deck().clone();
    s.handle(Request::SetWidgetTrial { name: "quote".into(), param: "source".into(), value: json!("Essai") }).unwrap();
    assert_that!(s.deck().clone()).is_equal_to(before);
}
```

`lib_tests.rs` : `{"cmd":"set_widget_trial","name":"q","param":"p","value":1}` et `{"cmd":"host_settings","slide":"s-1","widget":null}` dans la liste des requêtes en lecture de `changes_deck_read_requests_return_false` ; `{"cmd":"host_param","slide":"s-1","key":"name","value":"x"}` dans `changes_deck_edit_requests_return_true`. Run: `cargo nextest run -p oku-slides-session` — Expected: échec de compilation.

- [ ] **Step 2: `params_with`** — `forms.rs` :

```rust
/// Extras des réglages de l'hôte : clés des champs, lignes en lecture seule, « × ».
pub(crate) struct FormExtras<'a> {
    /// Clé de chaque champ, même ordre que les champs ; vide : pas de clé.
    pub keys: &'a [String],
    /// Lignes en légende, après les champs.
    pub notes: &'a [String],
    /// Bouton « × » (`dismiss`).
    pub closable: bool,
}
```

Renommer le corps de `params` en `params_with(id, spec, extras, palette, l)` ; l'action d'un champ reçoit `("key", lit(json!(k)))` quand `extras.keys.get(i)` est `Some(k)` (les deux actions : celle des champs et celle de `Rich`) ; après les champs, pour chaque note `i` : `components.push(Component::text(&format!("note-{i}"), note.clone(), Some(TextVariant::Caption))); body.push(format!("note-{i}"));` ; le bouton `close` n'est ajouté que si `extras.closable`. `params(id, spec, palette, l)` appelle `params_with(id, spec, &FormExtras { keys: &[], notes: &[], closable: true }, palette, l)`. Doc de `params_with` : « Ne lit ni `spec.slide` ni les `op_template` : les réglages de l'hôte en ont des vides. ».

- [ ] **Step 3: `panel.rs`** — garder l'ancienne fonction `panel` (supprimée en tâche 8) et ajouter :

```rust
/// Surface des réglages du modèle affiché.
pub(crate) const HOST_TEMPLATE: &str = "host-template";
/// Surface des réglages du widget du sous-onglet actif.
pub(crate) const HOST_WIDGET: &str = "host-widget";

/// Réglages de l'hôte : champs, leurs clés et des lignes en lecture seule.
pub(crate) struct HostParams {
    /// Les champs, rendus par `forms::params_with`.
    pub spec: ShowParams,
    /// Clé de chaque champ, même ordre.
    pub keys: Vec<String>,
    /// Lignes en lecture seule.
    pub notes: Vec<String>,
}

impl HostParams {
    fn extras(&self) -> FormExtras<'_> {
        FormExtras { keys: &self.keys, notes: &self.notes, closable: false }
    }
}

fn text_field(label: String, value: Value, op_template: Value) -> ParamField {
    ParamField { label, kind: FieldKind::Text, value, op_template, min: None, max: None, step: None, options: Vec::new() }
}

/// Réglages d'un modèle : nom, description, puis grille si `grid`.
pub(crate) fn template_params(slide: &Slide, grid: Option<Grid>, l: &Localizer) -> HostParams {
    let id = slide.id.as_str();
    let field = |key: &str| json!({ "op": "set_slide_field", "slide": id, "field": key, "value": "$value" });
    let mut fields = vec![
        text_field(l.tr("session-panel-name"), json!(slide.name.as_deref().unwrap_or_default()), field("name")),
        text_field(l.tr("session-panel-description"), json!(slide.description.as_deref().unwrap_or_default()), field("description")),
    ];
    let mut keys = vec!["name".to_owned(), "description".to_owned()];
    if let Some(g) = grid {
        for (key, label, value) in [("columns", "session-panel-columns", g.columns), ("rows", "session-panel-rows", g.rows)] {
            fields.push(ParamField {
                label: l.tr(label),
                kind: FieldKind::Stepper,
                value: json!(value),
                op_template: json!({ "op": "set_grid", "slide": id, key: "$value" }),
                min: Some(f64::from(*GRID_TRACKS.start())),
                max: Some(f64::from(*GRID_TRACKS.end())),
                step: None,
                options: Vec::new(),
            });
            keys.push(key.to_owned());
        }
    }
    let spec = ShowParams { title: l.tr("session-panel-template"), slide: slide.id.clone(), fields };
    HostParams { spec, keys, notes: Vec::new() }
}

impl Session {
    /// Réglages de l'hôte : le widget `widget` s'il en a, sinon le modèle `slide`.
    pub(crate) fn host_settings(&self, slide: &str, widget: Option<&str>) -> Result<Option<Surface>> {
        if !self.in_templates() {
            return Ok(None);
        }
        let palette: Vec<String> = self.appearance.slide.values().cloned().collect();
        // Un sous-onglet périmé (widget supprimé, renommé) donne la carte du modèle.
        let widget = widget.and_then(|w| self.widget_settings(w).ok().flatten());
        if let Some(p) = widget {
            return Ok(Some(params_with(HOST_WIDGET, &p.spec, &p.extras(), &palette, &self.localizer)));
        }
        let Ok(p) = self.template_host_params(slide) else { return Ok(None) };
        Ok(Some(params_with(HOST_TEMPLATE, &p.spec, &p.extras(), &palette, &self.localizer)))
    }

    /// Applique le réglage `key` du modèle `slide` : un pas d'annulation.
    pub(crate) fn host_param(&mut self, slide: &str, key: &str, value: Value) -> Result<()> {
        ensure!(self.in_templates(), NotEditingTemplatesSnafu);
        let p = self.template_host_params(slide)?;
        let field = p.keys.iter().position(|k| k == key).and_then(|i| p.spec.fields.get(i)).context(HostKeySnafu { key })?;
        let op = self.param_op(field, &value)?;
        let label = self.host_label(slide, &op);
        self.apply(&label, vec![op], None)
    }

    fn template_host_params(&self, slide: &str) -> Result<HostParams> {
        let view = self.slide(slide)?;
        let s = view.slide;
        // Une grille cacherait des blocs ; un widget sans paramètre `grid` n'en a pas de réglable.
        let has_blocks = s.params.slot("blocks").is_some_and(|b| !b.is_empty());
        let declares_grid = self.registry.get(&s.kind).and_then(|w| w.param("grid")).is_some();
        let grid = (view.zone.is_some() && declares_grid && !has_blocks).then(|| s.resolved_grid(&self.registry));
        Ok(template_params(s, grid, &self.localizer))
    }

    /// Libellé d'annulation d'un réglage du modèle.
    fn host_label(&self, slide: &str, op: &Op) -> String {
        let l = &self.localizer;
        match op {
            Op::SetGrid { columns, rows, .. } => {
                let g = self.deck().slide(slide).map(|s| s.resolved_grid(&self.registry));
                let c = columns.or(g.as_ref().map(|g| g.columns)).unwrap_or_default();
                let r = rows.or(g.as_ref().map(|g| g.rows)).unwrap_or_default();
                l.tr_with("session-undo-grid", &[("columns", c.into()), ("rows", r.into())])
            }
            Op::SetSlideField { field: SlideField::Description, .. } => l.tr("session-undo-describe-template"),
            Op::SetSlideField { field: SlideField::Name, .. } => l.tr("session-undo-rename-template"),
            _ => l.tr("session-undo-host-setting"),
        }
    }
}
```

Imports de `panel.rs` : `oku_slides_core::{GRID_TRACKS, Grid, Op, Slide, SlideField}`, `serde_json::{Value, json}`, `snafu::{OptionExt, ensure}`, `crate::agent::forms::{FormExtras, params_with}`, `crate::agent::{FieldKind, ParamField, ShowParams, Surface}`, `crate::error::{HostKeySnafu, NotEditingTemplatesSnafu}`, `crate::{Result, Session}`. Doc d'en-tête : « Réglages de l'hôte en mode modèles : le modèle affiché ou le widget du sous-onglet actif, adressés par clé (`host_settings`, `host_param`). L'ancien panneau `panel` vit jusqu'au passage au dock. ».

- [ ] **Step 4: `widget_settings`** — `widget_params.rs` :

```rust
impl Session {
    /// Réglages d'essai du widget `name` ; `None` pour Commun.
    pub(crate) fn widget_settings(&self, name: &str) -> Result<Option<HostParams>> {
        let file = parse_name(name)?;
        ensure!(self.in_templates(), NotEditingTemplatesSnafu);
        if file.is_common() {
            return Ok(None);
        }
        let l = &self.localizer;
        let title = l.tr_with("session-widget-params", &[("name", file.as_str().into())]);
        let empty = |title, notes| HostParams { spec: ShowParams { title, slide: String::new(), fields: Vec::new() }, keys: Vec::new(), notes };
        let Some(schema) = self.valid_schema(&file)? else {
            return Ok(Some(empty(title, vec![l.tr("session-widget-invalid-file")])));
        };
        let shown = preview_params(&schema, &self.preview_inputs(&file, &schema, &Map::new()), l);
        let (mut fields, mut keys, mut notes) = (Vec::new(), Vec::new(), Vec::new());
        for p in &schema.params {
            match field(p, shown.get(&p.name), l) {
                Some(f) => {
                    keys.push(p.name.clone());
                    fields.push(f);
                }
                None => notes.push(l.tr_with("session-widget-param-fixed", &[("name", label(p).into())])),
            }
        }
        if schema.params.is_empty() {
            notes.push(l.tr("session-widget-no-params"));
        }
        Ok(Some(HostParams { spec: ShowParams { title, slide: String::new(), fields }, keys, notes }))
    }
}

fn label(p: &ParamSchema) -> String {
    p.label.clone().unwrap_or_else(|| p.name.clone())
}

/// Le champ d'un paramètre réglable ; sa valeur est celle de l'aperçu.
fn field(p: &ParamSchema, shown: Option<&ParamValue>, l: &Localizer) -> Option<ParamField> {
    if !settable(&p.ty) {
        return None;
    }
    let option = |id: &str, label: &str| ChoiceOption { id: id.to_owned(), label: label.to_owned(), hint: None };
    let (kind, min, max, step, options) = match &p.ty {
        ParamType::Rich => (FieldKind::Rich, None, None, None, Vec::new()),
        ParamType::Number { min: Some(a), max: Some(b) } => {
            // Petite étendue (opacité…) : cent pas ; sinon des unités.
            let step = if b - a <= 10.0 { (b - a) / 100.0 } else { 1.0 };
            (FieldKind::Slider, Some(*a), Some(*b), Some(step), Vec::new())
        }
        ParamType::Choice { values } => (FieldKind::Select, None, None, None, values.iter().map(|v| option(v, v)).collect()),
        ParamType::Flag => (
            FieldKind::Select,
            None,
            None,
            None,
            vec![option("true", &l.tr("session-widget-param-yes")), option("false", &l.tr("session-widget-param-no"))],
        ),
        // `plain`, et un nombre sans bornes (lu comme un nombre, `trial_value`).
        _ => (FieldKind::Text, None, None, None, Vec::new()),
    };
    let value = match (&p.ty, shown) {
        (ParamType::Number { min: Some(_), max: Some(_) }, Some(ParamValue::Value(v))) => v.clone(),
        (ParamType::Number { .. }, Some(ParamValue::Value(v))) => json!(v.to_string()),
        (_, Some(ParamValue::Value(Value::Bool(b)))) => json!(b.to_string()),
        (_, Some(v)) => serde_json::to_value(v).unwrap_or(Value::Null),
        (_, None) => Value::Null,
    };
    Some(ParamField { label: label(p), kind, value, op_template: Value::Null, min, max, step, options })
}
```

(un `rich` montré est une liste de paragraphes : `params_with` en fait le markdown ; imports à ajouter : `ChoiceOption`, `Localizer`, `ParamSchema`, `json`, `crate::agent::{FieldKind, ParamField, ShowParams}`, `crate::panel::HostParams`. Corriger la valeur de pas attendue par `widget_settings_slider_step_fits_a_small_range` : `count` va de 1 à 9, soit 0,08.)

- [ ] **Step 5: Requêtes et erreurs.**
  - `error.rs` : `#[snafu(display("unknown host setting: {key}"))] HostKey { key: String, backtrace: Backtrace }`, `SessionErrorKind::HostKey`, `HostKey { key, .. } => l.tr_with("session-host-key", &[("key", key.as_str().into())])`.
  - `request.rs` : ajouter `HostSettings { slide: String, #[serde(default)] widget: Option<String> }`, `HostParam { slide: String, key: String, value: serde_json::Value }`, `SetWidgetTrial { name: String, param: String, value: serde_json::Value }` (une ligne de doc chacune) ; `changes_deck` : `HostParam` vrai, `HostSettings` et `SetWidgetTrial` faux ; `edits()` : `HostParam`, `SetWidgetTrial`.
  - `lib.rs` : dans `let (changes, documents) = …`, `Request::HostParam { .. }` rejoint `Request::Apply { .. }` (le premier `matches!` : `apply` écrit déjà son état) et `Request::SetWidgetTrial { .. }` rejoint la liste du second (`documents` faux) ; bras :

```rust
            Request::HostSettings { slide, widget } => return json(&self.host_settings(&slide, widget.as_deref())?),
```
```rust
            Request::HostParam { slide, key, value } => self.host_param(&slide, &key, value)?,
            Request::SetWidgetTrial { name, param, value } => {
                self.set_widget_trials(&name, serde_json::Map::from_iter([(param, value)]))?;
            }
```

  (les deux derniers tombent dans l'instantané final, comme `Apply`).
- [ ] **Step 6: i18n** — `session.ftl` (20 langues) :

| Clé | fr | en |
|---|---|---|
| `session-widget-params` | `Paramètres de { $name }` | `Parameters of { $name }` |
| `session-widget-no-params` | `Ce widget n'a pas de paramètre.` | `This widget has no parameters.` |
| `session-widget-param-fixed` | `{ $name } : non réglable dans l'aperçu` | `{ $name }: not adjustable in the preview` |
| `session-widget-param-yes` | `Oui` | `Yes` |
| `session-widget-param-no` | `Non` | `No` |
| `session-widget-invalid-file` | `Fichier invalide : réglages indisponibles` | `Invalid file: settings unavailable` |
| `session-host-key` | `Réglage inconnu : { $key }` | `Unknown setting: { $key }` |
| `session-undo-host-setting` | `Réglage du modèle` | `Template setting` |
| `session-undo-rename-template` | (valeur de `ui-undo-rename-template`) | idem |
| `session-undo-describe-template` | (valeur de `ui-undo-describe-template`) | idem |
| `session-undo-grid` | (valeur de `ui-undo-grid`) | idem |

  (les `ui-undo-*` restent jusqu'à la tâche 8.)
- [ ] **Step 7: Contrat** — Run: `OKU_UPDATE_FIXTURES=1 cargo nextest run -p oku-slides-session every_form` puis `cd ui && bun test src/a2ui/catalog.test.tsx` — Expected: PASS (le catalogue accepte la surface de l'hôte).
- [ ] **Step 8: Vérifier** — Run: `cargo nextest run --workspace --exclude oku-slides-app && cargo clippy --workspace --exclude oku-slides-app --all-targets -- -D warnings` — Expected: PASS. `ARCHITECTURE.md` de la session : réécrire la ligne de `panel.rs` (ou celle du panneau A2UI) en « `panel.rs` : réglages de l'hôte au dock (modèle, widget), adressés par clé. ».
- [ ] **Step 9: Commit**

```bash
git add crates/oku-slides-session crates/oku-slides-i18n/src/locales ui/src/a2ui/fixtures
```
```bash
git diff --cached --stat
```
```bash
git commit -m "feat(session) : réglages de l'hôte (modèle, widget) en surfaces adressées par clé ; requêtes host_settings, host_param, set_widget_trial (t-423)"
```

---

### Task 8: Réglages de l'hôte au dock ; fin du panneau `panel`

**Files:**
- Create: `ui/src/useHostSettings.ts`, `ui/src/useHostSettings.render.test.tsx`
- Modify: `ui/src/api.ts`, `ui/src/App.tsx` (état et effet du panneau, `onSurfaceAction`, rendu de `PanelSurface`, prop `dock` de `AgentThread`), `ui/src/a2ui/catalog.test.tsx` (fixture `panelMessages`), `crates/oku-slides-session/src/{panel.rs,request.rs,lib.rs,lib_tests.rs}` (`Panel`, `fn panel`, `PANEL_SURFACE` retirés), `crates/oku-slides-session/tests/session.rs` (`panel()` et les tests `panel_*` retirés), `crates/oku-slides-i18n/src/locales/*/{ui,session}.ftl`
- Delete: `ui/src/a2ui/PanelSurface.tsx`, `ui/src/a2ui/panelActions.ts`, `ui/src/a2ui/panelActions.test.ts`

**Interfaces:**
- Consumes: requêtes de la tâche 7 ; `AgentThread` prop `dock`, `fromA2ui` (t-416).
- Produces:
  - `api.hostSettings(slide, widget): Promise<Surface | null>`, `api.hostParam(slide, key, value): Promise<Snapshot>`, `api.setWidgetTrial(name, param, value): Promise<Snapshot>`.
  - `useHostSettings(api, snap, current, widget, run, say): HostSettings` ; `export const HOST_TEMPLATE = "host-template"`, `HOST_WIDGET = "host-widget"`.

- [ ] **Step 1: Tests (échouent)** — `ui/src/useHostSettings.render.test.tsx` :

```ts
// Réglages de l'hôte : relus sur leurs clés, actions adressées par clé.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { Snapshot, Surface, TabApi } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { HOST_TEMPLATE, HOST_WIDGET, useHostSettings } = await import("./useHostSettings");
const { deferred, memory, stub, wait } = await import("./test/widgetApi");

const surface = (id: string): Surface => ({ id, place: "thread", live: true, turn: null, messages: [] }) as unknown as Surface;
interface Snap { trials?: number; history?: boolean; hash?: string; undo?: string | null }
const snapshot = (o: Snap = {}): Snapshot =>
  ({
    editing: { set: "p", base: "b" }, templates: { version: "v" }, history: o.history ? {} : null,
    widget_trials: o.trials ?? 0, undo_label: o.undo ?? null, redo_label: null,
    slides: [{ id: "t-1", hash: o.hash ?? "h", name: "Libre" }],
  }) as unknown as Snapshot;
const action = (context: Record<string, unknown>): A2uiClientAction =>
  ({ name: "param", surfaceId: HOST_WIDGET, sourceComponentId: "f0", timestamp: new Date(0).toISOString(), context }) as unknown as A2uiClientAction;

interface Props { snap: Snapshot; current: string; widget: string | null }
let unmount: (() => Promise<void>) | null = null;
afterEach(async () => {
  await unmount?.();
  unmount = null;
});

async function mountHost(api: TabApi, first: Props) {
  const out: { h: ReturnType<typeof useHostSettings> | null; toasts: string[]; runs: number } = { h: null, toasts: [], runs: 0 };
  // Comme `run` de l'App : attend, et dit l'erreur en toast.
  const run = async (p: Promise<Snapshot>) => {
    out.runs += 1;
    try {
      return await p;
    } catch (e) {
      out.toasts.push(String(e));
      return null;
    }
  };
  function Probe(p: Props) {
    out.h = useHostSettings(api, p.snap, p.current, p.widget, run, (m) => out.toasts.push(m));
    return null;
  }
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  unmount = async () => {
    await act(async () => root.unmount());
    host.remove();
  };
  const render = async (p: Props) => {
    await act(async () => root.render(createElement(Probe, p)));
    await wait(10);
  };
  await render(first);
  return { out, render };
}

function counted(id: string) {
  const { fake } = memory({});
  const reads = { n: 0 };
  stub(fake, "hostSettings", async () => (reads.n++, surface(id)));
  return { fake, reads };
}

describe("useHostSettings", () => {
  test("montre le modèle sans widget", async () => {
    const { fake } = memory({});
    stub(fake, "hostSettings", async (_s: string, w: string | null) => surface(w ? HOST_WIDGET : HOST_TEMPLATE));
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: null });
    expect(m.out.h!.surface?.id).toBe(HOST_TEMPLATE);
  });

  test("montre le widget du sous-onglet actif", async () => {
    const { fake } = memory({});
    stub(fake, "hostSettings", async (_s: string, w: string | null) => surface(w ? HOST_WIDGET : HOST_TEMPLATE));
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: "carte" });
    expect(m.out.h!.surface?.id).toBe(HOST_WIDGET);
  });

  test("un instantané identique ne relit pas la surface", async () => {
    const { fake, reads } = counted(HOST_TEMPLATE);
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: null });
    await m.render({ snap: snapshot(), current: "t-1", widget: null });
    expect(reads.n).toBe(1);
  });

  test("la révision des essais relit la surface", async () => {
    const { fake, reads } = counted(HOST_WIDGET);
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: "carte" });
    await m.render({ snap: snapshot({ trials: 1 }), current: "t-1", widget: "carte" });
    expect(reads.n).toBe(2);
  });

  test("une annulation relit la surface du modèle", async () => {
    const { fake, reads } = counted(HOST_TEMPLATE);
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: null });
    await m.render({ snap: snapshot({ undo: "Grille 6 × 6" }), current: "t-1", widget: null });
    expect(reads.n).toBe(2);
  });

  test("pendant l'historique, aucune surface", async () => {
    const { fake } = counted(HOST_TEMPLATE);
    const m = await mountHost(fake, { snap: snapshot({ history: true }), current: "t-1", widget: null });
    expect(m.out.h!.surface).toBeNull();
  });

  test("une réponse en retard ne remplace pas la surface plus récente", async () => {
    const { fake } = memory({});
    const slow = deferred<Surface>();
    stub(fake, "hostSettings", (_s: string, w: string | null) => (w === "lent" ? slow.promise : Promise.resolve(surface(HOST_TEMPLATE))));
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: "lent" });
    await m.render({ snap: snapshot(), current: "t-1", widget: null });
    slow.resolve(surface(HOST_WIDGET));
    await wait(10);
    expect(m.out.h!.surface?.id).toBe(HOST_TEMPLATE);
  });

  test("un réglage du widget part en essai, avec le widget de la surface montrée", async () => {
    const { fake } = memory({});
    const sent: unknown[] = [];
    const later = deferred<Surface>();
    stub(fake, "hostSettings", (_s: string, w: string | null) => (w === "carte" ? Promise.resolve(surface(HOST_WIDGET)) : later.promise));
    stub(fake, "setWidgetTrial", async (...a: unknown[]) => (sent.push(a), snapshot()));
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: "carte" });
    const shown = m.out.h!.surface!;
    await m.render({ snap: snapshot(), current: "t-1", widget: "autre" });
    m.out.h!.onAction(shown, action({ key: "title", value: "Météo" }));
    await wait(10);
    expect(sent).toEqual([["carte", "title", "Météo"]]);
  });

  test("un réglage du modèle part en host_param avec sa clé", async () => {
    const { fake } = memory({});
    const sent: unknown[] = [];
    stub(fake, "hostSettings", async () => surface(HOST_TEMPLATE));
    stub(fake, "hostParam", async (...a: unknown[]) => (sent.push(a), snapshot()));
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: null });
    m.out.h!.onAction(m.out.h!.surface!, action({ key: "columns", value: 6 }));
    await wait(10);
    expect(sent).toEqual([["t-1", "columns", 6]]);
  });

  test("une surface d'une autre origine n'est pas traitée", async () => {
    const { fake } = counted(HOST_TEMPLATE);
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: null });
    expect(m.out.h!.onAction(surface("show-1"), action({ key: "name", value: "x" }))).toBe(false);
  });

  test("une action sans clé n'envoie rien", async () => {
    const { fake } = counted(HOST_TEMPLATE);
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: null });
    m.out.h!.onAction(m.out.h!.surface!, action({ value: "x" }));
    expect(m.out.runs).toBe(0);
  });

  test("un refus de la session est dit en toast", async () => {
    const { fake } = memory({});
    stub(fake, "hostSettings", async () => surface(HOST_WIDGET));
    stub(fake, "setWidgetTrial", async () => {
      throw new Error("refusé");
    });
    const m = await mountHost(fake, { snap: snapshot(), current: "t-1", widget: "carte" });
    m.out.h!.onAction(m.out.h!.surface!, action({ key: "count", value: 99 }));
    await wait(10);
    expect(m.out.toasts).toEqual(["Error: refusé"]);
  });
});
```

(`deferred` est l'assistant de `test/widgetApi.ts` ; s'il n'y est pas exporté sous ce nom, prendre celui de `WidgetTab.render.test.tsx`.) Run: `cd ui && bun test src/useHostSettings.render.test.tsx` — Expected: FAIL.

- [ ] **Step 2: Implémenter.**
  - `api.ts` : requêtes `{ cmd: "host_settings"; slide: string; widget: string | null }`, `{ cmd: "host_param"; slide: string; key: string; value: unknown }`, `{ cmd: "set_widget_trial"; name: string; param: string; value: unknown }` à la place de `{ cmd: "panel"; slide: string }` ; méthodes `hostSettings`, `hostParam`, `setWidgetTrial` à la place de `panel`.
  - `ui/src/useHostSettings.ts` :

```ts
// Réglages de l'hôte en mode modèles (modèle affiché, ou widget du sous-onglet
// actif) : une surface de réglages construite par la session, rendue au dock.
// Chaque champ porte sa clé ; une action part avec la slide et le widget de la
// surface affichée.

import { useEffect, useState } from "react";
import type { A2uiClientAction } from "@a2ui/web_core/v0_9";
import type { Snapshot, Surface, TabApi } from "./api";
import { fromA2ui } from "./agent/quick";

/** Surface des réglages du modèle affiché. */
export const HOST_TEMPLATE = "host-template";
/** Surface des réglages du widget du sous-onglet actif. */
export const HOST_WIDGET = "host-widget";

/** La surface de réglages de l'hôte, et le routage de ses actions. */
export interface HostSettings {
  surface: Surface | null;
  /** Vrai si `s` est la surface de l'hôte affichée (action traitée, ou ignorée sans clé). */
  onAction(s: Surface, a: A2uiClientAction): boolean;
}

/** Réglages de l'hôte au dock : relus quand la slide, le widget ou le deck changent. */
export function useHostSettings(
  api: TabApi,
  snap: Snapshot | null,
  current: string | null,
  widget: string | null,
  run: (p: Promise<Snapshot>) => unknown,
  say: (m: string, error?: boolean) => void,
): HostSettings {
  const [shown, setShown] = useState<{ surface: Surface; slide: string; widget: string | null } | null>(null);
  const active = !!current && !!snap?.editing && !snap.history;
  const slide = snap?.slides.find((s) => s.id === current);
  // Tout ce qui peut changer la carte : pack, essais, slide (rendu, nom), annulation.
  const key = [snap?.editing?.base, snap?.templates?.version, snap?.widget_trials, slide?.hash, slide?.name, snap?.undo_label, snap?.redo_label].join("|");
  useEffect(() => {
    if (!active || !current) return setShown(null);
    let alive = true;
    api
      .hostSettings(current, widget)
      // Même contenu : la surface est gardée (SurfaceView reconstruit une surface reçue à nouveau).
      .then((surface) =>
        alive &&
        setShown((prev) =>
          surface && prev && prev.slide === current && prev.widget === widget && JSON.stringify(prev.surface) === JSON.stringify(surface)
            ? prev
            : surface
              ? { surface, slide: current, widget }
              : null,
        ),
      )
      .catch((e) => alive && say(String(e), true));
    return () => {
      alive = false;
    };
    // `say` ne fait que poser un toast : son identité ne compte pas.
  }, [api, active, current, widget, key]);
  const onAction = (s: Surface, a: A2uiClientAction): boolean => {
    if (!shown || s !== shown.surface) return false;
    const { name, context } = fromA2ui(a);
    const field = context.key;
    if (name !== "param" || typeof field !== "string") return true;
    void run(
      s.id === HOST_WIDGET && shown.widget
        ? api.setWidgetTrial(shown.widget, field, context.value)
        : api.hostParam(shown.slide, field, context.value),
    );
    return true;
  };
  return { surface: shown?.surface ?? null, onAction };
}
```

  - `App.tsx` : supprimer `NO_PANEL`, l'état `panel` et son effet, `onPanelAction`, l'import et le rendu de `PanelSurface`, l'import de `panelActionOps` ; ajouter `const host = useHostSettings(api, snap, current, widgets.activeName, run, say);` ; en première ligne de `onSurfaceAction` : `if (host.onAction(s, a)) return;` ; `const dockWithHost = useMemo(() => (host.surface ? [host.surface, ...dock] : dock), [host.surface, dock]);` passé à `AgentThread` (`dock` : la valeur que t-416 lui passe).
  - Session : retirer `Request::Panel` (et ses bras dans `changes_deck`, `handle`), `fn panel` de `lib.rs`, `pub use panel::PANEL_SURFACE;`, l'ancienne fonction `panel` de `panel.rs` et ses cartes ; `lib_tests.rs` : retirer `{"cmd":"panel",…}` ; `tests/session.rs` : retirer `panel()`, `component()` et les quatre tests `panel_*` (le cas « blocs » vit dans `host_settings_of_a_template_with_blocks_has_no_grid_fields`).
  - i18n : retirer `ui-undo-rename-template`, `ui-undo-describe-template`, `ui-undo-grid` des 20 `ui.ftl` ; retirer `session-panel-grid` des 20 `session.ftl`.
  - `catalog.test.tsx` : retirer la fixture `panelMessages` et ses tests (la surface de l'hôte est couverte par `agent-surfaces.json`).
  - Supprimer `PanelSurface.tsx`, `panelActions.ts`, `panelActions.test.ts`.

- [ ] **Step 3: Vérifier** — Run: `cargo nextest run --workspace --exclude oku-slides-app && cargo clippy --workspace --exclude oku-slides-app --all-targets -- -D warnings && cd ui && bun test && bunx tsc --noEmit -p . && wc -l src/App.tsx && ! grep -rn "PanelSurface\|PanelAction\|panelAction\|PANEL_SURFACE\|cmd: \"panel\"\|ui-undo-grid\|ui-undo-rename-template\|ui-undo-describe-template" src ../crates` — Expected: PASS, App < 1000 lignes, grep vide.

- [ ] **Step 4: Commit**

```bash
git add crates/oku-slides-session crates/oku-slides-i18n/src/locales ui/src
```
```bash
git diff --cached --stat
```
```bash
git commit -m "feat(ui)! : réglages du modèle et du widget au dock de l'agent ; panneau à part retiré (t-423)"
```

---

### Task 9: Bout en bout, vérification visuelle, consolidation

**Files:**
- Modify: `ui/e2e/panel.cjs`, `ui/package.json` (script `e2e`)
- Create: `ui/e2e/widgets.cjs`

- [ ] **Step 1: `panel.cjs`** — à partir de l'état laissé par t-416 :
  - `state(p)` lit la carte `[data-surface-id="host-template"]` : `cards` = nombre de `[data-surface-id="host-template"]` (attendu 1), `name` = son premier `input`, `grid` = ses `.stepper-value` ;
  - attentes : `await p.waitForSelector('[data-surface-id="host-template"] .stepper-value')` à la place de l'attente de la 2ᵉ carte ; `assert.equal(before.cards, 1)` ; « hors des modèles, aucune carte » : `cards` vaut 0 ;
  - le reste (renommer, une colonne de moins, annuler) ne change pas.

- [ ] **Step 2: `widgets.cjs`**

```js
// Réglages d'essai d'un widget au dock : changer un paramètre change le contenu de l'aperçu.
// Contre `oku-slides serve fixtures/roadmap.json`. Lancer : just e2e.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const IMG = '.widget-tab:not([hidden]) .widget-tab-side img';
const svgOf = (p) => p.evaluate(async (sel) => {
  const img = document.querySelector(sel);
  return img ? (await fetch(img.src)).text() : null;
}, IMG);
(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await p.waitForSelector('.stage img');
  await p.getByRole('button', { name: 'Menu' }).click();
  await p.getByRole('menuitem', { name: 'Éditer les widgets' }).click();
  await p.getByRole('button', { name: 'Nouveau widget' }).click();
  const name = p.getByRole('textbox', { name: /Nom du widget/ });
  await name.fill('carte');
  await name.press('Enter');
  await p.waitForSelector(IMG);
  const before = await svgOf(p);
  const field = p.locator('[data-surface-id="host-widget"] input').first();
  await field.fill('Météo du jour');
  await field.press('Enter');
  await p.waitForFunction(async ([sel, was]) => {
    const img = document.querySelector(sel);
    return img && (await (await fetch(img.src)).text()) !== was;
  }, [IMG, before]);
  assert.notEqual(await svgOf(p), before);
  assert.equal(await p.locator('[data-surface-id="host-widget"] input').first().inputValue(), 'Météo du jour');
  assert.deepEqual(errors, []);
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
```

et `&& bun e2e/widgets.cjs` en fin du script `e2e` de `ui/package.json`.

- [ ] **Step 3: Vérifier** — Run: `just e2e` (données isolées) — Expected: tous les scénarios passent.

- [ ] **Step 4: Vérification visuelle** — lancer l'app (skill `run`, données isolées), mode modèles :
  - le dock montre les réglages du modèle, et ⌘Z y remet la valeur annulée ;
  - ouvrir le sous-onglet `carte` : réglages du widget à la place ;
  - un `rich` se met en forme, ⌘Z dans ce champ n'agit pas sur l'éditeur Typst ;
  - un fichier cassé garde la carte du widget avec sa ligne « fichier invalide » ;
  - une question de l'agent en attente replie les réglages ;
  - captures en clair et en sombre, thèmes Papier et Verre.

- [ ] **Step 5: Consolidation (G10)**
  - `! grep -rn "PANEL_SURFACE\|PanelSurface\|rescaleCell\|panelAction\|PanelAction\|GRID_MIN\|GRID_MAX" crates ui/src` (la classe `panel-card`, si t-416 l'a gardée pour toutes les cartes, n'est pas du code mort).
  - Une seule définition des pistes de grille (`GRID_TRACKS`) ; le défaut `min = 1, max = 24` du `Stepper` du catalogue n'est qu'un repli (la session envoie toujours ses bornes).
  - Le sondage des schémas passe par les statuts (`valid_schema`) ; aucune nouvelle copie de `PackWidgets::new` + `probe_pack`.
  - Une seule édition markdown (`markdownEditing.ts`) pour `TextEditor` et `RichTextField`.
  - Les `ARCHITECTURE.md` du cœur, de la session et du MCP sont à jour et ≤ 50 lignes.
  - Run: `just check`.

- [ ] **Step 6: Commit**

```bash
git add ui/e2e ui/package.json
```
```bash
git diff --cached --stat
```
```bash
git commit -m "test(e2e) : réglages du modèle et d'essai du widget au dock (t-423)"
```

## Ordre et dépendances

1 → 2 → 3 → 4 → 5 (phase A). Après la fusion de la tâche 11 de t-416 : 6 → 7 → 8 → 9. Chaque tâche laisse le dépôt vert ; la tâche 7 ajoute les requêtes de l'hôte sans retirer `panel`, que la tâche 8 retire des deux côtés.

## Décisions de revue (2026-10-03)

Prises avec l'utilisateur après la première revue (15 relecteurs), reportées dans la spec :
1. `set_grid` est une op du deck, visible de l'agent (retour assumé sur t-294).
2. Grille de départ résolue par le registre (`Slide::resolved_grid`), aussi pour `set_cell`, la validation et l'aperçu.
3. Réglages adressés par clé ; deux requêtes : `host_param` (deck) et `set_widget_trial` (pas un changement du deck).
4. Provenance : `set_widget_by_agent` côté session, aucun drapeau dans la requête ; marques dans `Mode::Templates`.
5. Suppression par l'agent : elle gagne sur les frappes en attente et les écritures en file (`files.forget`), avec un message.
6. Fichier invalide : la carte du widget reste, avec une ligne « fichier invalide ».
7. `widget_touched` en mode modèles seulement.
8. Champ texte vidé : l'essai s'efface.
9. Texte riche : nouveau composant A2UI `RichTextField` (multiligne, mis en forme), qui garde les paragraphes d'origine.

La seconde revue (14 relecteurs) n'a demandé aucune nouvelle décision : ses corrections sont intégrées ci-dessus (tâches réordonnées sans code mort, tests vérifiés contre les assistants réels, `keep`/`forget`, relecture de la carte du modèle, nombres, contrat Rust ↔ TS).
