# Paramètre liste `p.list` — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** un type de paramètre `list` (items à id stable, colonnes typées), saisi en tableau dans le dock de l'éditeur de widget et éditable sur la slide (champs, « + », « × », ⋮⋮).

**Architecture:** le cœur ajoute `ParamType::List`, `ParamValue::List(Vec<ListItem>)`, l'op `edit_list` (`ListOp`) et un conteneur Loro `{ list: LoroMovableList }` ; Typst passe les items au widget et `list-item` émet une boîte `<oku-slides-item>` ; la session expose les champs d'item (`liste/id/colonne`) et les items ; l'interface ajoute `ListTable` (A2UI) et `ItemLayer` (scène).

**Tech Stack:** Rust (snafu, serde, schemars, Loro), Typst, TypeScript/React (TanStack Table v9, bun test + happy-dom), Playwright (e2e), Fluent (20 langues).

**Spec:** `docs/superpowers/specs/2026-10-04-parametre-liste-design.md` — lire d'abord la section « Amendements après lecture du code » (A1–A11), qui prime.

## Global Constraints

- Tâche parente : t-494 ; chaque commit se termine par `(t-494)`. Une sous-tâche par Task (titre identique), `task done` à la fin de chaque Task : `oku cli task done --project slider --id <t-N>`.
- Commits : `git add <fichiers>` séparé, puis `git diff --cached --name-only` juste avant `git commit` (d'autres sessions indexent sur `main`) ; **pas de ligne `Co-Authored-By`** ; format `type(scope) : description (t-494)`.
- TDD : test rouge, puis code minimal, puis vert. Tests côte à côte (`foo_tests.rs`, `foo.test.ts`), speculoos, rstest ; noms `<unité>_<cas>_<résultat>`.
- Pas de `.unwrap()`/`.expect()` hors tests ; erreurs Snafu ; tout texte visible passe par Fluent, **dans les 20 langues** (`crates/oku-slides-i18n/src/locales/<lang>/`) : `ar bn de en es fr hi it ja ko nl pl pt ru th tr uk vi zh-Hans zh-Hant`. Le test `every_locale_has_exactly_the_reference_keys` l'impose.
- Fichiers ≤ 500 lignes visés, 1000 max.
- Types de colonne admis : `plain`, `rich`, `number`, `flag`, `choice`, `asset`. Noms de colonne réservés : `id`, `kind` ; `/` interdit dans tout nom de paramètre ou de colonne et dans un id d'item.
- Adresse d'un champ d'item : `<liste>/<id>/<colonne>`.
- Liste absente = liste vide ; le `default` du schéma n'amorce qu'un objet créé, ids `d0`, `d1`… (A1 bis).
- Avant de lancer l'app ou l'e2e : `export OKU_SLIDES_PATHS__DATA=$(mktemp -d) OKU_SLIDES_PATHS__DECKS=$(mktemp -d)`.
- Vérifications : `cargo nextest run -p <crate>` ; `cd ui && bun run test && bun run typecheck` ; `just check` avant le dernier commit de chaque Task.

## Structure des fichiers

| Fichier | Rôle |
|---|---|
| `crates/oku-slides-core/src/widget.rs` | `ParamType::List`, `is_column_type`, `check_param` (refacto de `check_widget`) |
| `crates/oku-slides-core/src/error.rs` | `SchemaFault::{EmptyList, ColumnType, ReservedName, ListBounds}`, `OpInner::ItemNotFound` |
| `crates/oku-slides-core/src/params.rs` | `ListItem`, `ParamValue::List`, `is_items`, `list()/list_mut()/lists()` |
| `crates/oku-slides-core/src/ops/list.rs` (nouveau) | `ListOp`, `ListSpec`, `default_items`, `list_value`, `edit_list`, `seed_lists` |
| `crates/oku-slides-core/src/ops/mod.rs` | `Op::EditList`, dispatch, appels à `seed_lists` |
| `crates/oku-slides-core/src/replica/{schema,write,read}.rs` | `Kind::List`, conteneur `{list}` |
| `crates/oku-slides-core/src/{validate,fields,canonical}.rs` | validation, champs d'item, liste vide retirée |
| `typst/oku-slides/src/{params,widget,data,widgets}.typ`, `lib.typ` | `p.list`, `list-item`, préfixe de champ, `_args`, widget `frise` |
| `crates/oku-slides-render/src/lib.rs` | `ItemBox`, `Layout.items`, `items_on` |
| `crates/oku-slides-session/src/{fields,items,views,widget_params,widget_preview,request,lib}.rs` | `FieldView` d'item, `ItemView`, dock et essais |
| `crates/oku-slides-session/src/agent/forms.rs`, `crates/oku-slides-a2ui/src/message.rs` | `FieldKind::List`, `ComponentKind::ListTable` |
| `ui/src/table/cells.ts` (nouveau) | `parseTsv`, `parseCell`, `parseCheck` partagés |
| `ui/src/a2ui/ListTable.tsx` (+ `.css`) | tableau du dock |
| `ui/src/stageItems.ts`, `ui/src/ItemLayer.tsx` (nouveaux) | géométrie et poignées des items |
| `ui/src/{api,deck,Stage,useFieldEditing,useHostSettings}.ts(x)` | types, `fieldOps`, câblage |
| `ui/e2e/lists.cjs` (nouveau) | recette e2e |

---

### Task 1: Schéma `ParamType::List` et ses contrôles

**Files:**
- Modify: `crates/oku-slides-core/src/widget.rs:43-88` (enum), `:186-228` (`check_widget`)
- Modify: `crates/oku-slides-core/src/error.rs:336-380` (`SchemaFault`, `Display`, `message_id`)
- Modify: `crates/oku-slides-i18n/src/locales/*/core.ftl` (clés `widget-fault-*`)
- Test: `crates/oku-slides-core/src/widget_tests.rs` (créer s'il n'existe pas, `#[cfg(test)] #[path = "widget_tests.rs"] mod widget_tests;`)

**Interfaces:**
- Produces: `ParamType::List { item: Vec<ParamSchema>, item_label: Option<String>, min: usize, max: Option<usize> }` (JSON : `{"type":"list","item":[…],"item-label":…,"min":0,"max":null}`) ; `pub fn is_column_type(ty: &ParamType) -> bool` ; `pub const RESERVED_COLUMNS: [&str; 2]`.

- [ ] **Step 1: Tests RED**

```rust
use serde_json::json;
use speculoos::prelude::*;
use crate::{ParamType, WidgetRegistry};

fn probe(params: serde_json::Value) -> Result<WidgetRegistry, crate::WidgetError> {
    WidgetRegistry::from_probe(json!({ "base": [{ "name": "w", "category": "element", "params": params }] }))
}
fn col(name: &str, ty: &str) -> serde_json::Value { json!({ "name": name, "type": ty }) }
fn list(item: serde_json::Value) -> serde_json::Value { json!([{ "name": "etapes", "type": "list", "item": item, "min": 0, "max": null }]) }

#[test]
fn from_probe_list_reads_columns_and_bounds() {
    let r = probe(json!([{ "name": "etapes", "type": "list", "item-label": "étape",
        "item": [col("date", "plain"), col("texte", "rich")], "min": 1, "max": 5 }])).unwrap();
    let p = r.get("w").unwrap().param("etapes").unwrap();
    let ParamType::List { item, item_label, min, max } = &p.ty else { panic!("pas une liste") };
    assert_that!(item.iter().map(|c| c.name.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["date", "texte"]);
    assert_that!(item_label.as_deref()).is_equal_to(Some("étape"));
    assert_that!((*min, *max)).is_equal_to((1, Some(5)));
}

#[rstest::rstest]
#[case(list(json!([col("x", "slot")])))]
#[case(list(json!([col("x", "list")])))]
#[case(list(json!([col("x", "grid")])))]
fn from_probe_list_with_excluded_column_type_fails(#[case] params: serde_json::Value) {
    assert_that!(probe(params)).is_err();
}

#[rstest::rstest]
#[case(list(json!([col("id", "plain")])))]
#[case(list(json!([col("kind", "plain")])))]
#[case(list(json!([col("a/b", "plain")])))]
#[case(json!([col("a/b", "plain")]))]
fn from_probe_reserved_name_fails(#[case] params: serde_json::Value) {
    assert_that!(probe(params)).is_err();
}

#[test]
fn from_probe_list_without_columns_fails() {
    assert_that!(probe(list(json!([])))).is_err();
}

#[test]
fn from_probe_list_with_max_below_min_fails() {
    let p = json!([{ "name": "l", "type": "list", "item": [col("a", "plain")], "min": 3, "max": 2 }]);
    assert_that!(probe(p)).is_err();
}

#[test]
fn from_probe_list_with_wrong_column_default_fails() {
    let p = list(json!([{ "name": "n", "type": "number", "default": "x" }]));
    assert_that!(probe(p)).is_err();
}

#[test]
fn from_probe_list_default_with_unknown_column_fails() {
    let p = json!([{ "name": "l", "type": "list", "item": [col("a", "plain")], "min": 0, "max": null,
        "default": [{ "b": "x" }] }]);
    assert_that!(probe(p)).is_err();
}
```

- [ ] **Step 2: Vérifier l'échec** — Run: `cargo nextest run -p oku-slides-core widget_tests` · Expected: échec de compilation (`ParamType::List` absent).

- [ ] **Step 3: Implémentation**

Dans `ParamType`, après `Host` :

```rust
    /// Liste d'items aux colonnes typées (`item`), saisie en tableau ;
    /// items adressés par leur `id`.
    List {
        item: Vec<ParamSchema>,
        #[serde(default, rename = "item-label")]
        item_label: Option<String>,
        #[serde(default)]
        min: usize,
        #[serde(default)]
        max: Option<usize>,
    },
```

Après l'enum :

```rust
/// Noms interdits à une colonne de `list` : clés de l'item et d'un élément.
pub const RESERVED_COLUMNS: [&str; 2] = ["id", "kind"];

/// Types admis pour une colonne de `list` ; liste imbriquée : t-495.
pub fn is_column_type(ty: &ParamType) -> bool {
    matches!(
        ty,
        ParamType::Plain | ParamType::Rich | ParamType::Number { .. } | ParamType::Flag
            | ParamType::Choice { .. } | ParamType::Asset
    )
}
```

Refactoriser `check_widget` : le corps du `for p in &w.params` (sauf le contrôle de doublon) devient `fn check_param(w: &WidgetSchema, p: &ParamSchema) -> Result<(), WidgetError>`, appelé pour chaque paramètre et, récursivement, pour chaque colonne. Ajouter en tête de `check_param` :

```rust
    if p.name.contains('/') {
        return Err(fault(w, SchemaFault::ReservedName));
    }
```

et le bras de `match &p.ty` :

```rust
            ParamType::List { item, min, max, .. } => {
                if item.is_empty() {
                    return Err(fault(w, SchemaFault::EmptyList));
                }
                let mut names = BTreeSet::new();
                for c in item {
                    if !is_column_type(&c.ty) {
                        return Err(fault(w, SchemaFault::ColumnType));
                    }
                    if c.name.is_empty() || RESERVED_COLUMNS.contains(&c.name.as_str()) {
                        return Err(fault(w, SchemaFault::ReservedName));
                    }
                    if !names.insert(c.name.as_str()) {
                        return Err(fault(w, SchemaFault::DuplicateParam));
                    }
                    check_param(w, c)?;
                }
                if max.is_some_and(|m| m < *min) {
                    return Err(fault(w, SchemaFault::ListBounds));
                }
                let known = |o: &serde_json::Map<String, Value>| o.keys().all(|k| names.contains(k.as_str()));
                let ok = match &p.default {
                    None => true,
                    Some(Value::Array(a)) => {
                        !max.is_some_and(|m| a.len() > m) && a.iter().all(|x| x.as_object().is_some_and(known))
                    }
                    Some(_) => false,
                };
                if !ok {
                    return Err(fault(w, SchemaFault::DefaultWrongType));
                }
            }
```

`compatible()` (surcharge d'un widget de base) : rien à faire, le bras `(b, p) => b == p` exige une liste identique.

Dans `error.rs`, ajouter à `SchemaFault` `EmptyList, ColumnType, ReservedName, ListBounds`, leurs bras `Display` (« list without columns », « column type not allowed in a list », « reserved parameter name », « list max below min ») et `message_id` (`widget-fault-empty-list`, `widget-fault-column-type`, `widget-fault-reserved-name`, `widget-fault-list-bounds`). Clés dans les 20 `core.ftl`, par exemple :

```ftl
# en
widget-fault-empty-list = a list needs at least one column
widget-fault-column-type = a list column can be plain, rich, number, flag, choice or asset; for free content per item, use p.slot with a child widget
widget-fault-reserved-name = reserved name: id, kind, or a name containing /
widget-fault-list-bounds = a list max is below its min
# fr
widget-fault-empty-list = une liste a besoin d'au moins une colonne
widget-fault-column-type = une colonne de liste est plain, rich, number, flag, choice ou asset ; pour un contenu libre par item, utilisez p.slot avec un widget enfant
widget-fault-reserved-name = nom réservé : id, kind, ou un nom contenant /
widget-fault-list-bounds = le max d'une liste est sous son min
```

Les 18 autres langues : traduction fidèle (mêmes arguments, noms `p.slot`, `id`, `kind` inchangés).

Compléter les `match` exhaustifs que le compilateur signale : `ops/check.rs` `expected` → `ParamType::List { .. } => "a list of items"` ; `ops/value.rs` → provisoirement `ParamType::List { .. } => Err(wrong("a list of items"))` (remplacé en Task 3).

- [ ] **Step 4: Vérifier** — Run: `cargo nextest run -p oku-slides-core && cargo nextest run -p oku-slides-i18n` · Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-core/src/widget.rs crates/oku-slides-core/src/widget_tests.rs crates/oku-slides-core/src/error.rs crates/oku-slides-core/src/ops crates/oku-slides-i18n/src/locales
git diff --cached --name-only
git commit -m "feat(core) : schéma du paramètre list, colonnes typées et noms réservés (t-494)"
```

---

### Task 2: Valeur `ParamValue::List`

**Files:**
- Modify: `crates/oku-slides-core/src/params.rs` (enum l.17, `Deserialize` l.30-44, accesseurs)
- Modify: `crates/oku-slides-core/src/canonical.rs` (liste vide retirée)
- Modify: `crates/oku-slides-core/src/lib.rs` (réexport `ListItem`)
- Test: `crates/oku-slides-core/src/params_tests.rs`, `crates/oku-slides-core/src/canonical_tests.rs` (ou le fichier de tests existant de `canonical.rs`)

**Interfaces:**
- Produces: `pub struct ListItem { pub id: String, pub values: Params }` (serde : `id` + `flatten`) ; `ParamValue::List(Vec<ListItem>)` ; `Params::list(&self, &str) -> Option<&[ListItem]>`, `Params::list_mut(&mut self, &str) -> Option<&mut Vec<ListItem>>`, `Params::lists(&self) -> impl Iterator<Item = (&str, &[ListItem])>` ; `pub(crate) fn is_items(&[Value]) -> bool`.

- [ ] **Step 1: Tests RED** (fin de `params_tests.rs`)

```rust
#[test]
fn deserialize_items_without_kind_gives_list() {
    let p = params(json!({ "etapes": [{ "id": "a", "date": "2019" }, { "id": "b", "date": "2021" }] }));
    let l = p.list("etapes").unwrap();
    assert_that!(l.iter().map(|i| i.id.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["a", "b"]);
    assert_that!(l[0].values.text("date")).is_equal_to(Some("2019"));
}

#[test]
fn deserialize_item_with_rich_column_reads_paragraphs() {
    let p = params(json!({ "l": [{ "id": "a", "texte": [{ "runs": [{ "t": "Hé" }] }] }] }));
    assert_that!(p.list("l").unwrap()[0].values.rich("texte").map(<[_]>::len)).is_equal_to(Some(1));
}

#[test]
fn deserialize_objects_with_id_and_kind_stay_a_slot() {
    let p = params(json!({ "c": [{ "id": "e", "kind": "text" }] }));
    assert_that!(p.list("c")).is_none();
    assert_that!(p.slot("c").map(<[_]>::len)).is_equal_to(Some(1));
}

#[test]
fn list_reads_an_empty_array_as_an_empty_list() {
    let p = params(json!({ "l": [] }));
    assert_that!(p.list("l").map(<[_]>::len)).is_equal_to(Some(0));
}

#[test]
fn serialize_list_round_trips() {
    let v = json!({ "l": [{ "id": "a", "date": "2019", "on": true }] });
    assert_that!(serde_json::to_value(params(v.clone())).unwrap()).is_equal_to(v);
}

#[test]
fn list_mut_turns_an_absent_param_into_an_empty_list() {
    let mut p = Params::default();
    p.list_mut("l").unwrap().push(crate::ListItem { id: "a".into(), values: Params::default() });
    assert_that!(p.list("l").map(<[_]>::len)).is_equal_to(Some(1));
}
```

Test de `canonicalize` : un élément dont `params` vaut `{ "l": ParamValue::List(vec![]) }` n'a plus `l` après `canonicalize`.

- [ ] **Step 2: Vérifier l'échec** — Run: `cargo nextest run -p oku-slides-core params_tests` · Expected: échec de compilation.

- [ ] **Step 3: Implémentation**

```rust
/// Un item de liste (`list`) : son id, unique dans la liste, et ses
/// valeurs par colonne.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListItem {
    pub id: String,
    #[serde(flatten)]
    pub values: Params,
}
```

Variante `ParamValue::List(Vec<ListItem>)` (doc : « Items d'une liste (`list`) : tableau non vide d'objets à `id` chaîne, sans `kind` »), placée **après** `Slot`. Dans `Deserialize`, entre les bras `is_elements` et `is_paragraphs` :

```rust
            Value::Array(a) if is_items(&a) => {
                ParamValue::List(serde_json::from_value(Value::Array(a)).map_err(serde::de::Error::custom)?)
            }
```

```rust
/// Items de liste : non vide, chacun avec `id` en chaîne et sans `kind`
/// (un élément en a un).
pub(crate) fn is_items(a: &[Value]) -> bool {
    let is_item = |x: &Value| x.get("id").is_some_and(Value::is_string) && x.get("kind").is_none();
    !a.is_empty() && a.iter().all(is_item)
}
```

Accesseurs, sur le modèle de `slot`/`slot_mut`/`slots` :

```rust
    /// Items d'une liste ; un texte riche ou un slot vide se lit comme une liste vide.
    pub fn list(&self, name: &str) -> Option<&[ListItem]> {
        match self.get(name)? {
            ParamValue::List(l) => Some(l),
            ParamValue::Rich(p) if p.is_empty() => Some(&[]),
            ParamValue::Slot(s) if s.is_empty() => Some(&[]),
            _ => None,
        }
    }

    /// La liste `name`, créée vide si elle est absente ou porte une autre valeur.
    pub fn list_mut(&mut self, name: &str) -> Option<&mut Vec<ListItem>> {
        let value = self.0.entry(name.to_owned()).or_insert_with(|| ParamValue::List(Vec::new()));
        if !matches!(value, ParamValue::List(_)) {
            *value = ParamValue::List(Vec::new());
        }
        match value {
            ParamValue::List(list) => Some(list),
            _ => None,
        }
    }

    /// Toutes les listes, avec leur nom.
    pub fn lists(&self) -> impl Iterator<Item = (&str, &[ListItem])> {
        self.0.iter().filter_map(|(k, v)| match v {
            ParamValue::List(l) => Some((k.as_str(), l.as_slice())),
            _ => None,
        })
    }
```

`canonical.rs` : là où « texte riche et slot vides retirés », retirer aussi `ParamValue::List(l) if l.is_empty()`, et canoniser les valeurs de chaque item comme des paramètres (même fonction appliquée à `item.values`). Mettre à jour la doc de `canonicalize` (« …, slot et liste vides retirés ; valeurs des items canonisées comme des paramètres »). Réexporter `ListItem` depuis `lib.rs` (liste explicite). `cargo build --workspace` signale les `match` exhaustifs sur `ParamValue` à compléter.

- [ ] **Step 4: Vérifier** — Run: `cargo nextest run -p oku-slides-core` · Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-core/src/params.rs crates/oku-slides-core/src/params_tests.rs crates/oku-slides-core/src/canonical.rs crates/oku-slides-core/src/lib.rs
git diff --cached --name-only
git commit -m "feat(core) : valeur ParamValue::List, items à id sans kind (t-494)"
```

---

### Task 3: Op `edit_list`, `set_param` d'une liste, amorçage des défauts

**Files:**
- Create: `crates/oku-slides-core/src/ops/list.rs`, `crates/oku-slides-core/src/ops/list_tests.rs`
- Modify: `crates/oku-slides-core/src/ops/mod.rs` (variante, dispatch, `AddElement`/`AddSlide`/`SetKind`/`ReplaceElement`), `ops/value.rs` (bras `List`), `ops/check.rs`
- Modify: `crates/oku-slides-core/src/error.rs` (`ItemNotFound`), `crates/oku-slides-i18n/src/locales/*/core.ftl` (`op-item-not-found`)
- Modify: `crates/oku-slides-core/src/lib.rs` (réexports `ListOp`, `ListSpec`, `default_items`)
- Test: `crates/oku-slides-core/src/ops/list_tests.rs`, `crates/oku-slides-core/tests/lists.rs`

**Interfaces:**
- Consumes: `ParamType::List` (Task 1), `ListItem`, `Params::list` (Task 2), `param_value` (`ops/value.rs`).
- Produces:
  - `Op::EditList { target: Target, param: String, change: ListOp }` (JSON `{"op":"edit_list",…}`)
  - `pub enum ListOp { Insert { index: Option<usize>, id: String, values: Map<String, Value> }, Remove { item: String }, Move { item: String, to: usize }, SetField { item: String, field: String, value: Value } }` (JSON `{"op":"insert",…}`)
  - `pub struct ListSpec<'a> { pub name: &'a str, pub columns: &'a [ParamSchema], pub min: usize, pub max: Option<usize> }` + `ListSpec::of(name: &'a str, ty: &'a ParamType) -> Option<Self>`
  - `impl ListOp { pub fn apply(self, items: &mut Vec<ListItem>, spec: &ListSpec<'_>) -> Result<(), OpError> }`
  - `pub fn default_items(p: &ParamSchema) -> Result<Vec<ListItem>, OpError>` (ids `d0`, `d1`…)
  - `pub(crate) fn list_value(spec: &ListSpec<'_>, value: Value) -> Result<Option<ParamValue>, OpError>`
  - `pub(crate) fn seed_lists(registry: &WidgetRegistry, kind: &str, params: &mut Params)`

- [ ] **Step 1: Tests RED — unitaires** (`ops/list_tests.rs`)

```rust
use serde_json::{Map, json};
use speculoos::prelude::*;
use super::*;
use crate::{ListItem, ParamSchema, ParamType, Params};

fn column(name: &str, ty: ParamType, default: Option<serde_json::Value>) -> ParamSchema {
    ParamSchema { name: name.into(), label: None, ty, default, bind: None }
}
fn schema(min: usize, max: Option<usize>) -> ParamType {
    ParamType::List {
        item: vec![
            column("date", ParamType::Plain, None),
            column("texte", ParamType::Plain, Some(json!("Nouvelle étape"))),
            column("on", ParamType::Flag, None),
        ],
        item_label: None, min, max,
    }
}
fn items(ids: &[&str]) -> Vec<ListItem> {
    ids.iter().map(|id| ListItem { id: (*id).into(), values: Params::default() }).collect()
}
fn ids(l: &[ListItem]) -> Vec<&str> { l.iter().map(|i| i.id.as_str()).collect() }
fn insert(index: Option<usize>, id: &str) -> ListOp { ListOp::Insert { index, id: id.into(), values: Map::new() } }

#[test]
fn apply_insert_at_index_fills_column_defaults() {
    let ty = schema(0, None);
    let spec = ListSpec::of("etapes", &ty).unwrap();
    let mut l = items(&["a", "b"]);
    insert(Some(1), "n").apply(&mut l, &spec).unwrap();
    assert_that!(ids(&l)).is_equal_to(vec!["a", "n", "b"]);
    assert_that!(l[1].values.text("texte")).is_equal_to(Some("Nouvelle étape"));
}

#[test]
fn apply_insert_without_index_appends() {
    let ty = schema(0, None);
    let mut l = items(&["a"]);
    insert(None, "n").apply(&mut l, &ListSpec::of("l", &ty).unwrap()).unwrap();
    assert_that!(ids(&l)).is_equal_to(vec!["a", "n"]);
}

#[rstest::rstest]
#[case(insert(None, "a"))]
#[case(insert(None, ""))]
#[case(insert(None, "x/y"))]
#[case(insert(Some(9), "n"))]
fn apply_insert_invalid_fails_and_keeps_items(#[case] op: ListOp) {
    let ty = schema(0, None);
    let mut l = items(&["a"]);
    assert_that!(op.apply(&mut l, &ListSpec::of("l", &ty).unwrap())).is_err();
    assert_that!(ids(&l)).is_equal_to(vec!["a"]);
}

#[test]
fn apply_insert_beyond_max_fails() {
    let ty = schema(0, Some(1));
    let mut l = items(&["a"]);
    assert_that!(insert(None, "n").apply(&mut l, &ListSpec::of("l", &ty).unwrap())).is_err();
}

#[test]
fn apply_remove_below_min_fails() {
    let ty = schema(1, None);
    let mut l = items(&["a"]);
    let r = ListOp::Remove { item: "a".into() }.apply(&mut l, &ListSpec::of("l", &ty).unwrap());
    assert_that!(r).is_err();
}

#[test]
fn apply_remove_unknown_item_is_item_not_found() {
    let ty = schema(0, None);
    let mut l = items(&["a"]);
    let e = ListOp::Remove { item: "z".into() }.apply(&mut l, &ListSpec::of("l", &ty).unwrap()).unwrap_err();
    assert_that!(e.kind()).is_equal_to(crate::OpErrorKind::ItemNotFound);
}

#[test]
fn apply_move_puts_the_item_at_its_new_index() {
    let ty = schema(0, None);
    let mut l = items(&["a", "b", "c"]);
    ListOp::Move { item: "a".into(), to: 2 }.apply(&mut l, &ListSpec::of("l", &ty).unwrap()).unwrap();
    assert_that!(ids(&l)).is_equal_to(vec!["b", "c", "a"]);
}

#[test]
fn apply_set_field_converts_by_column_type() {
    let ty = schema(0, None);
    let spec = ListSpec::of("l", &ty).unwrap();
    let mut l = items(&["a"]);
    ListOp::SetField { item: "a".into(), field: "date".into(), value: json!(" 2019 ") }.apply(&mut l, &spec).unwrap();
    assert_that!(l[0].values.text("date")).is_equal_to(Some("2019"));
    let bad = ListOp::SetField { item: "a".into(), field: "on".into(), value: json!("oui") };
    assert_that!(bad.apply(&mut l, &spec)).is_err();
    let unknown = ListOp::SetField { item: "a".into(), field: "zz".into(), value: json!("x") };
    assert_that!(unknown.apply(&mut l, &spec)).is_err();
}

#[test]
fn default_items_gives_deterministic_ids() {
    let p = ParamSchema { name: "l".into(), label: None, ty: schema(0, None),
        default: Some(json!([{ "date": "2019" }, { "date": "2021" }])), bind: None };
    let l = default_items(&p).unwrap();
    assert_that!(ids(&l)).is_equal_to(vec!["d0", "d1"]);
    assert_that!(l[1].values.text("date")).is_equal_to(Some("2021"));
}

#[test]
fn list_value_null_clears_and_duplicates_fail() {
    let ty = schema(0, None);
    let spec = ListSpec::of("l", &ty).unwrap();
    assert_that!(list_value(&spec, json!(null)).unwrap()).is_none();
    assert_that!(list_value(&spec, json!([{ "id": "a" }, { "id": "a" }]))).is_err();
    assert_that!(list_value(&spec, json!([{ "date": "x" }]))).is_err();
}
```

- [ ] **Step 2: Tests RED — intégration** (`tests/lists.rs`, helpers de `tests/ops.rs` copiés en tête : `registry()` de `tests/common`, `apply`, `apply_err`). Le registre de base n'a pas encore de liste : construire un registre de test avec `WidgetRegistry::from_probe` qui ajoute à la sonde de `fixtures/widgets/base-registry.json` un widget d'élément `liste` à `etapes: list(date: plain, texte: plain(default: "Nouvelle étape"))`, `default: [{date: "2019"}]`.

```rust
#[test]
fn add_element_seeds_the_list_default_with_d_ids() {
    let mut deck = fixture();
    apply(&mut deck, add(json!({ "id": "f", "kind": "liste" }))).unwrap();
    let f = deck.slide("s-constat").unwrap().locate("f").unwrap().element;
    assert_that!(f.params.list("etapes").map(|l| l[0].id.clone())).is_equal_to(Some("d0".to_owned()));
}

#[test]
fn edit_list_insert_then_set_field_edits_the_deck() {
    let mut deck = fixture();
    apply(&mut deck, add(json!({ "id": "f", "kind": "liste" }))).unwrap();
    apply(&mut deck, op(json!({ "op": "edit_list", "target": { "slide": "s-constat", "element": "f" },
        "param": "etapes", "change": { "op": "insert", "id": "i-1" } }))).unwrap();
    apply(&mut deck, op(json!({ "op": "edit_list", "target": { "slide": "s-constat", "element": "f" },
        "param": "etapes", "change": { "op": "set_field", "item": "i-1", "field": "date", "value": "2027" } }))).unwrap();
    let f = deck.slide("s-constat").unwrap().locate("f").unwrap().element;
    let l = f.params.list("etapes").unwrap();
    assert_that!(l.iter().map(|i| i.id.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["d0", "i-1"]);
    assert_that!(l[1].values.text("date")).is_equal_to(Some("2027"));
}

#[test]
fn edit_list_on_a_non_list_param_is_param_type() {
    let mut deck = fixture();
    let e = apply_err(&mut deck, op(json!({ "op": "edit_list", "target": { "slide": "s-priorites", "element": "c1" },
        "param": "title", "change": { "op": "remove", "item": "x" } })));
    assert_that!(e.kind()).is_equal_to(oku_slides_core::OpErrorKind::ParamType);
}

#[test]
fn set_param_null_empties_the_list() {
    let mut deck = fixture();
    apply(&mut deck, add(json!({ "id": "f", "kind": "liste" }))).unwrap();
    apply(&mut deck, set_param(at("s-constat", "f"), "etapes", json!(null))).unwrap();
    let f = deck.slide("s-constat").unwrap().locate("f").unwrap().element;
    assert_that!(f.params.list("etapes").map(<[_]>::len).unwrap_or(0)).is_equal_to(0);
}
```

(`add(element)` = `add_element` dans le slot `blocks` de `s-constat` ; `op(v)` = `serde_json::from_value::<Op>(v).unwrap()` ; `apply` passe par `apply_all` pour que `canonicalize` s'applique.)

- [ ] **Step 3: Vérifier l'échec** — Run: `cargo nextest run -p oku-slides-core list` · Expected: échec de compilation.

- [ ] **Step 4: Implémentation** — `ops/list.rs` :

```rust
//! `edit_list` : insérer, retirer, déplacer un item d'une liste (`list`), ou
//! modifier un de ses champs. Les items sont adressés par leur id. Aussi :
//! `set_param` d'une liste entière, et amorçage des listes d'un objet créé.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::value::param_value;
use super::{Target, declared, target_mut, target_ref, widget};
use crate::error::{DuplicateIdSnafu, IndexOutOfRangeSnafu, InvalidValueSnafu, ItemNotFoundSnafu,
    OpError, ParamTypeSnafu, UnknownParamSnafu};
use crate::{Deck, ListItem, ParamSchema, ParamType, ParamValue, Params, WidgetRegistry};

/// Un changement d'une liste ; la liste entière se remplace par `set_param`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum ListOp {
    /// Insère l'item `id` (unique dans la liste, sans `/`) ; `index` absent :
    /// à la fin ; colonnes absentes de `values` : leur défaut.
    Insert {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<usize>,
        id: String,
        #[serde(default)]
        values: Map<String, Value>,
    },
    Remove { item: String },
    /// Déplace l'item à l'index `to` de la liste après déplacement.
    Move { item: String, to: usize },
    /// Valeur d'une colonne, convertie selon son type ; `null` la retire.
    SetField { item: String, field: String, value: Value },
}

/// Colonnes et bornes d'un paramètre `list`.
#[derive(Debug, Clone, Copy)]
pub struct ListSpec<'a> {
    pub name: &'a str,
    pub columns: &'a [ParamSchema],
    pub min: usize,
    pub max: Option<usize>,
}

impl<'a> ListSpec<'a> {
    /// Le paramètre `name` de type `ty`, s'il est une liste.
    pub fn of(name: &'a str, ty: &'a ParamType) -> Option<Self> {
        match ty {
            ParamType::List { item, min, max, .. } => Some(Self { name, columns: item, min: *min, max: *max }),
            _ => None,
        }
    }

    fn column(&self, field: &str) -> Result<&'a ParamSchema, OpError> {
        self.columns.iter().find(|c| c.name == field)
            .ok_or_else(|| UnknownParamSnafu { widget: self.name, param: field }.build().into())
    }

    fn full(&self, len: usize) -> bool {
        self.max.is_some_and(|m| len >= m)
    }
}

/// Valeurs d'un item : colonnes déclarées, converties ; absentes : défaut.
fn item_values(spec: &ListSpec<'_>, values: &Map<String, Value>) -> Result<Params, OpError> {
    if let Some(k) = values.keys().find(|k| !spec.columns.iter().any(|c| &c.name == *k)) {
        return Err(UnknownParamSnafu { widget: spec.name, param: k.as_str() }.build().into());
    }
    let mut out = Params::default();
    for c in spec.columns {
        let raw = values.get(&c.name).cloned().or_else(|| c.default.clone()).unwrap_or(Value::Null);
        if let Some(v) = param_value(&c.ty, &c.name, raw)? {
            out.set(c.name.clone(), v);
        }
    }
    Ok(out)
}

fn check_id(spec: &ListSpec<'_>, items: &[ListItem], id: &str) -> Result<(), OpError> {
    if id.is_empty() || id.contains('/') {
        return Err(InvalidValueSnafu { field: spec.name, reason: "an item id is a non-empty string without /" }
            .build().into());
    }
    if items.iter().any(|i| i.id == id) {
        return Err(DuplicateIdSnafu { id }.build().into());
    }
    Ok(())
}

fn position(spec: &ListSpec<'_>, items: &[ListItem], id: &str) -> Result<usize, OpError> {
    items.iter().position(|i| i.id == id)
        .ok_or_else(|| ItemNotFoundSnafu { list: spec.name, item: id }.build().into())
}

impl ListOp {
    /// Applique le changement à `items` ; en cas d'erreur, `items` est intact.
    pub fn apply(self, items: &mut Vec<ListItem>, spec: &ListSpec<'_>) -> Result<(), OpError> {
        match self {
            ListOp::Insert { index, id, values } => {
                check_id(spec, items, &id)?;
                if spec.full(items.len()) {
                    return Err(InvalidValueSnafu { field: spec.name, reason: "the list is full" }.build().into());
                }
                let at = index.unwrap_or(items.len());
                if at > items.len() {
                    return Err(IndexOutOfRangeSnafu { index: at, len: items.len() }.build().into());
                }
                let values = item_values(spec, &values)?;
                items.insert(at, ListItem { id, values });
            }
            ListOp::Remove { item } => {
                let i = position(spec, items, &item)?;
                if items.len() <= spec.min {
                    return Err(InvalidValueSnafu { field: spec.name, reason: "the list has its minimum of items" }
                        .build().into());
                }
                items.remove(i);
            }
            ListOp::Move { item, to } => {
                let i = position(spec, items, &item)?;
                if to >= items.len() {
                    return Err(IndexOutOfRangeSnafu { index: to, len: items.len() }.build().into());
                }
                let it = items.remove(i);
                items.insert(to, it);
            }
            ListOp::SetField { item, field, value } => {
                let i = position(spec, items, &item)?;
                let c = spec.column(&field)?;
                match param_value(&c.ty, &field, value)? {
                    Some(v) => drop(items[i].values.set(field, v)),
                    None => drop(items[i].values.remove(&field)),
                }
            }
        }
        Ok(())
    }
}

/// Items par défaut du schéma, aux ids `d0`, `d1`… (les mêmes que Typst).
pub fn default_items(p: &ParamSchema) -> Result<Vec<ListItem>, OpError> {
    let Some(spec) = ListSpec::of(&p.name, &p.ty) else { return Ok(Vec::new()) };
    let Some(Value::Array(rows)) = &p.default else { return Ok(Vec::new()) };
    rows.iter().enumerate().map(|(i, row)| {
        let empty = Map::new();
        let values = item_values(&spec, row.as_object().unwrap_or(&empty))?;
        Ok(ListItem { id: format!("d{i}"), values })
    }).collect()
}

/// `set_param` d'une liste entière : ids uniques, colonnes converties,
/// bornes respectées ; `null` vide la liste.
pub(crate) fn list_value(spec: &ListSpec<'_>, value: Value) -> Result<Option<ParamValue>, OpError> {
    let rows = match value {
        Value::Null => return Ok(None),
        Value::Array(rows) => rows,
        _ => return Err(ParamTypeSnafu { param: spec.name, expected: "a list of items or null" }.build().into()),
    };
    let mut items: Vec<ListItem> = Vec::with_capacity(rows.len());
    for row in rows {
        let Value::Object(mut obj) = row else {
            return Err(ParamTypeSnafu { param: spec.name, expected: "a list of items or null" }.build().into());
        };
        let id = match obj.remove("id") {
            Some(Value::String(s)) => s,
            _ => return Err(InvalidValueSnafu { field: spec.name, reason: "every item needs an id" }.build().into()),
        };
        check_id(spec, &items, &id)?;
        let values = item_values(spec, &obj)?;
        items.push(ListItem { id, values });
    }
    if items.len() < spec.min || spec.max.is_some_and(|m| items.len() > m) {
        return Err(InvalidValueSnafu { field: spec.name, reason: "the list is outside its bounds" }.build().into());
    }
    Ok(if items.is_empty() { None } else { Some(ParamValue::List(items)) })
}

/// `edit_list` : la liste du paramètre `param` de la cible, changée par `change`.
pub(super) fn edit_list(deck: &mut Deck, registry: &WidgetRegistry, target: &Target, param: &str,
    change: ListOp) -> Result<(), OpError> {
    let (kind, _, _) = target_ref(deck, target)?;
    let p = declared(widget(registry, kind)?, param)?.clone();
    let spec = ListSpec::of(&p.name, &p.ty)
        .ok_or_else(|| OpError::from(ParamTypeSnafu { param, expected: "a list" }.build()))?;
    let (_, params) = target_mut(deck, target)?;
    let mut items = params.list(param).map(<[_]>::to_vec).unwrap_or_default();
    change.apply(&mut items, &spec)?;
    if items.is_empty() {
        params.remove(param);
    } else {
        params.set(param, ParamValue::List(items));
    }
    Ok(())
}

/// Amorce les listes absentes d'un objet créé avec le défaut du schéma de `kind`.
pub(crate) fn seed_lists(registry: &WidgetRegistry, kind: &str, params: &mut Params) {
    let Some(schema) = registry.get(kind) else { return };
    for p in &schema.params {
        if params.get(&p.name).is_none()
            && let Ok(items) = default_items(p)
            && !items.is_empty()
        {
            params.set(p.name.clone(), ParamValue::List(items));
        }
    }
}
```

Les noms des sélecteurs `DuplicateIdSnafu`, `IndexOutOfRangeSnafu` et les types de leurs champs sont ceux de `error.rs:47-49` (`id: String` ; `index`, `len`) : ajuster les conversions (`.to_owned()`, `as`) si le compilateur l'exige.

`error.rs` : variante `#[snafu(display("{list} has no item {item}"))] ItemNotFound { list: String, item: String, backtrace: Backtrace }` dans `OpInner`, `OpErrorKind::ItemNotFound`, bras `kind()`, `subject()` (→ `item`) et `Localize` (→ `op-item-not-found`). Dans les 20 `core.ftl` : `op-item-not-found = no item { $subject }` (en), `op-item-not-found = aucun item { $subject }` (fr), traductions pour les autres.

`ops/mod.rs` : `mod list; pub use list::{ListOp, ListSpec, default_items};` ; variante

```rust
    /// Modifie une liste (`list`) : items adressés par leur id. Remplacer
    /// toute la liste : `set_param`.
    EditList {
        target: Target,
        param: String,
        change: ListOp,
    },
```

placée après `SetChartData`, et dans `apply` : `Op::EditList { target, param, change } => list::edit_list(deck, registry, &target, &param, change),`. Dans les gestionnaires `AddElement`, `AddSlide`, `SetKind` et `ReplaceElement`, après le contrôle des paramètres et avant l'insertion, appeler `list::seed_lists(registry, &e.kind, &mut e.params)` pour l'objet **et chaque élément de ses slots** (parcours récursif des `params.slots_mut()`).

`ops/value.rs` : `ParamType::List { .. } => list_value(&ListSpec::of(param, ty).ok_or_else(|| wrong("a list of items"))?, value),` (import `super::list::{ListSpec, list_value}`). Pour une liste, `param` est le nom du paramètre.

Réexporter `ListOp`, `ListSpec`, `default_items` dans `lib.rs`. `ops_schema` inclut `edit_list` sans autre changement.

- [ ] **Step 5: Vérifier** — Run: `cargo nextest run -p oku-slides-core && cargo nextest run -p oku-slides-i18n` · Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/oku-slides-core/src/ops crates/oku-slides-core/src/error.rs crates/oku-slides-core/src/lib.rs crates/oku-slides-core/tests/lists.rs crates/oku-slides-i18n/src/locales
git diff --cached --name-only
git commit -m "feat(core) : op edit_list, set_param d'une liste et amorçage des défauts à la création (t-494)"
```

---

### Task 4: Replica — conteneur `{ list }`

**Files:**
- Modify: `crates/oku-slides-core/src/replica/schema.rs` (`Kind::List`, `declared_kind`, `shape_kind`, `fits`, `declared`, const `LIST`)
- Modify: `crates/oku-slides-core/src/replica/write.rs` (`sync_params` prend des colonnes, `sync_list` générique, `list_at_map`)
- Modify: `crates/oku-slides-core/src/replica/read.rs` (`read_params` : `Map{list}`)
- Test: `crates/oku-slides-core/src/replica/mod_tests.rs`, `crates/oku-slides-core/tests/convergence.rs`

**Interfaces:**
- Consumes: `ParamType::List`, `is_items` (Task 2), registre de test de Task 3.
- Produces: `Kind::List` ; `pub(super) const LIST: &str = "list";` ; `Writer::sync_params(&self, map, obj, params: Option<&[ParamSchema]>)` ; `Writer::sync_list(&self, list, items, each: impl Fn(&LoroMap, &Map<String, Value>) -> Res)`.

- [ ] **Step 1: Tests RED** (`mod_tests.rs`, sur le modèle des tests de slot existants : replica ouvert sur un deck, `apply` d'ops, relecture)

```rust
#[test]
fn reconcile_list_reads_back_items_and_text_columns() {
    // deck avec l'élément `f` (kind `liste`, items d0 et i-1), réconcilié puis relu
    let read = round_trip(&deck_with_list(&["d0", "i-1"]));
    let f = read.slide("s-constat").unwrap().locate("f").unwrap().element;
    assert_that!(f.params.list("etapes").map(|l| l.len())).is_equal_to(Some(2));
    assert_that!(f.params.list("etapes").unwrap()[0].values.text("date")).is_equal_to(Some("2019"));
}

#[test]
fn reconcile_list_plain_column_is_a_loro_text() {
    // le champ `date` de l'item d0 est un Container::Text dans la carte de l'item
}

#[test]
fn reconcile_emptied_list_reads_back_absent() {}

#[test]
fn undo_list_insert_move_remove_each_restores_the_previous_list() {}
```

`tests/convergence.rs` :

```rust
#[test]
fn concurrent_list_inserts_are_both_kept() {}      // deux pairs insèrent i-a et i-b : les deux restent, ordre identique sur les deux pairs
#[test]
fn concurrent_move_and_set_field_merge() {}        // pair A déplace d0, pair B modifie d0.date : les deux effets restent
#[test]
fn concurrent_typing_in_one_plain_cell_merges() {} // A tape au début, B à la fin de d0.date : le texte fusionné contient les deux
```

Remplir chaque corps selon les helpers des tests existants du fichier (pairs, `import`, `apply`) ; chaque test fait au moins une assertion sur les ids et les valeurs relus.

- [ ] **Step 2: Vérifier l'échec** — Run: `cargo nextest run -p oku-slides-core replica convergence` · Expected: FAIL (items relus perdus : `read_list` lit les items comme des éléments sans `kind`).

- [ ] **Step 3: Implémentation**

`schema.rs` :

```rust
/// Clé de la carte d'un paramètre `list`, qui porte sa `LoroMovableList`.
pub(super) const LIST: &str = "list";
```

`Kind` gagne `/// LoroMap { list: LoroMovableList de LoroMap à id }` → `List`. `declared_kind` : `ParamType::List { .. } => Kind::List`. `shape_kind` : `Value::Array(a) if is_items(a) => Kind::List` (après `is_elements`). `fits` : `(Kind::List, Value::Array(a)) => a.is_empty() || is_items(a)`. `declared()` garde `Kind::List` (non `Atom` : créé d'avance). Ajouter :

```rust
/// Catégorie de la colonne `name` d'un item : type déclaré, sinon forme de
/// la valeur ; une colonne n'est jamais une collection.
pub(super) fn column_kind(columns: Option<&[ParamSchema]>, name: &str, value: &Value) -> Kind {
    let kind = columns
        .and_then(|cs| cs.iter().find(|c| c.name == name))
        .map(|c| declared_kind(&c.ty))
        .filter(|k| fits(*k, value))
        .unwrap_or_else(|| shape_kind(value));
    if matches!(kind, Kind::Collection(_) | Kind::List) { Kind::Atom } else { kind }
}
```

et faire de `param_kind(schema, name, value)` un appel à une forme commune `kind_in(params: Option<&[ParamSchema]>, …)` (une seule mise en œuvre pour les paramètres et les colonnes, la différence étant le rabattement des collections).

`write.rs` :

- `sync_list` reçoit la réconciliation par item en fermeture : `fn sync_list(&self, list: &LoroMovableList, items: &[Value], each: &dyn Fn(&LoroMap, &Map<String, Value>) -> Res) -> Res` ; les appels existants passent `&|m, o| self.sync_map(m, o, child)`.
- `fn list_map_at(map: &LoroMap, key: &str) -> Result<LoroMap, ReplicaInner>` (comme `rich_at`) puis `list_at(&m, LIST)`.
- Dans `sync_params` (et dans `declared` pour la création d'avance), bras `Kind::List` :

```rust
                Kind::List => {
                    let columns = schema.and_then(|w| w.param(key)).and_then(|p| match &p.ty {
                        ParamType::List { item, .. } => Some(item.as_slice()),
                        _ => None,
                    });
                    let items = value.as_array().map(Vec::as_slice).unwrap_or_default();
                    self.sync_list(&list_at(&list_map_at(map, key)?, LIST)?, items, &|m, o| self.sync_item(m, o, columns))?
                }
```

```rust
    /// Un item : `id` en valeur (clé de réconciliation), colonnes comme des paramètres.
    fn sync_item(&self, map: &LoroMap, obj: &Map<String, Value>, columns: Option<&[ParamSchema]>) -> Res {
        let id = obj.get("id").cloned().unwrap_or(Value::Null);
        set_value(map, "id", json_to_loro(&id))?;
        let mut rest = obj.clone();
        rest.remove("id");
        self.sync_columns(map, &rest, columns)
    }
```

`sync_columns` reprend la boucle de `sync_params` (suppression des clés périmées par `empty_or_delete`, colonnes `plain`/`rich` créées d'avance, puis `column_kind`) en ignorant la clé `id` dans les clés périmées. Factoriser la boucle commune entre `sync_params` et `sync_columns` (une fonction prenant le calcul de `Kind` en paramètre).
- `empty_or_delete` d'une carte `{list}` : vider sa liste (comme une `MovableList`).

`read.rs`, dans `read_params`, avant le bras `Map{rich}` :

```rust
                ValueOrContainer::Container(Container::Map(m)) if m.get(LIST).is_some() => {
                    let items = self.read_items(&m, depth.deeper());
                    if items.is_empty() { continue; }
                    Value::Array(items)
                }
```

```rust
    /// Items d'une liste : chaque carte relue comme des paramètres (son `id` en valeur).
    fn read_items(&mut self, m: &LoroMap, depth: Depth) -> Vec<Value> {
        let Some(ValueOrContainer::Container(Container::MovableList(l))) = m.get(LIST) else { return Vec::new() };
        (0..l.len()).filter_map(|i| match l.get(i) {
            Some(ValueOrContainer::Container(Container::Map(item))) => Some(self.read_params_value(&item, depth)),
            _ => None,
        }).collect()
    }
```

`read_params_value` est la partie de `read_params` qui produit l'objet JSON d'une carte de paramètres (l'extraire si besoin). Un item sans `id` chaîne est écarté avec l'issue `issue-unreadable-item` (ajouter la clé dans les 20 `core.ftl` : en « unreadable list item », fr « item de liste illisible »).

- [ ] **Step 4: Vérifier** — Run: `cargo nextest run -p oku-slides-core` · Expected: PASS (dont les tests de convergence et d'annulation existants).

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-core/src/replica crates/oku-slides-core/tests/convergence.rs crates/oku-slides-i18n/src/locales
git diff --cached --name-only
git commit -m "feat(core) : liste dans le replica, carte {list} lisible sans registre (t-494)"
```

---

### Task 5: Validation et champs d'item

**Files:**
- Modify: `crates/oku-slides-core/src/validate.rs` (`fits` l.227-257, `Validator::params` l.168-181, `assets` l.184-193)
- Modify: `crates/oku-slides-core/src/fields.rs`
- Modify: `crates/oku-slides-i18n/src/locales/*/core.ftl` (`issue-empty-item-id`, `issue-duplicate-item-id`, `issue-list-bounds`)
- Test: `crates/oku-slides-core/src/validate_tests.rs`, `crates/oku-slides-core/src/fields_tests.rs`

**Interfaces:**
- Consumes: Tasks 1–3.
- Produces: `TextField { owner, owner_id, param: &ParamSchema /* colonne pour un item */, values: &'a Params, list: Option<(&'a str, &'a ListItem)> }` et `TextField::name(&self) -> Cow<'a, str>` (`"etapes/d0/date"` pour un item, sinon le nom du paramètre).

- [ ] **Step 1: Tests RED**

`validate_tests.rs` (registre de test à widget `liste`, comme Task 3) :

```rust
#[test]
fn validate_list_with_duplicate_item_ids_reports_it() {
    let issues = validate_element(json!({ "id": "f", "kind": "liste",
        "params": { "etapes": [{ "id": "a" }, { "id": "a" }] } }));
    assert_that!(ids(&issues)).contains("issue-duplicate-item-id".to_owned());
}
#[test]
fn validate_list_above_max_reports_bounds() {}         // max 2, trois items → issue-list-bounds
#[test]
fn validate_list_column_of_wrong_type_reports_param_type() {}  // date: 3 → issue-param-type
#[test]
fn validate_list_asset_column_missing_from_sources_reports_it() {} // colonne asset → issue-missing-asset
```

`fields_tests.rs` :

```rust
#[test]
fn text_fields_expand_list_items_in_item_order() {
    // élément f de kind liste, items d0 et i-1, colonnes date (plain) et texte (plain), on (flag)
    let names: Vec<String> = slide.text_fields(&registry).iter().map(|f| format!("{}.{}", f.owner_id, f.name())).collect();
    assert_that!(names).is_equal_to(vec!["f.etapes/d0/date".to_owned(), "f.etapes/d0/texte".into(),
        "f.etapes/i-1/date".into(), "f.etapes/i-1/texte".into()]);
}
```

- [ ] **Step 2: Vérifier l'échec** — Run: `cargo nextest run -p oku-slides-core validate fields` · Expected: FAIL.

- [ ] **Step 3: Implémentation**

`validate.rs`, `fits` :

```rust
        (ParamType::List { item, .. }, ParamValue::List(items)) => items.iter().all(|it| {
            it.values.iter().all(|(k, v)| {
                item.iter().find(|c| c.name == k).is_some_and(|c| matches!(fits(&c.ty, v), Fit::Ok))
            })
        }),
        (ParamType::List { .. }, ParamValue::Rich(p)) => p.is_empty(),
        (ParamType::List { .. }, ParamValue::Slot(s)) => s.is_empty(),
```

`Validator::params`, après le contrôle de type, pour un paramètre `List { min, max, .. }` lu par `params.list(name)` :

```rust
            if let (ParamType::List { min, max, .. }, Some(items)) = (&p.ty, params.list(name)) {
                let mut seen = HashSet::new();
                for it in items {
                    if it.id.is_empty() {
                        self.push(at, Message::new("issue-empty-item-id").arg("param", name));
                    } else if !seen.insert(it.id.as_str()) {
                        self.push(at, Message::new("issue-duplicate-item-id").arg("param", name));
                    }
                }
                if !items.is_empty() && (items.len() < *min || max.is_some_and(|m| items.len() > m)) {
                    self.push(at, Message::new("issue-list-bounds").arg("param", name));
                }
            }
```

`assets` : parcourir aussi, pour chaque paramètre `List`, les colonnes `asset` de chaque item (`it.values.text(&c.name)`).

Clés (20 langues) : en `issue-empty-item-id = list { $param } has an item without identifier`, `issue-duplicate-item-id = list { $param } has a duplicate item identifier`, `issue-list-bounds = list { $param } has too few or too many items` ; fr `issue-empty-item-id = la liste { $param } a un item sans identifiant`, `issue-duplicate-item-id = la liste { $param } a un identifiant d'item en double`, `issue-list-bounds = la liste { $param } a trop peu ou trop d'items`.

`fields.rs` : `TextField` gagne `values` et `list` (doc : « Paramètres où lire `param` : ceux du propriétaire, ou ceux de l'item ») ; `text_params` est remplacé par une fonction qui, dans l'ordre du schéma, rend un champ par paramètre `plain`/`rich` et, pour un paramètre `List`, un champ par colonne `plain`/`rich` de chaque item de `params.list(name)` :

```rust
impl<'a> TextField<'a> {
    /// Nom du champ : `liste/id/colonne` pour un item, sinon le paramètre.
    pub fn name(&self) -> Cow<'a, str> {
        match self.list {
            Some((list, item)) => Cow::Owned(format!("{list}/{}/{}", item.id, self.param.name)),
            None => Cow::Borrowed(self.param.name.as_str()),
        }
    }
}
```

Mettre à jour la doc du module (« … et les colonnes texte des items de chaque liste, dans l'ordre des items »).

- [ ] **Step 4: Vérifier** — Run: `cargo nextest run -p oku-slides-core && cargo build --workspace` · Expected: PASS ; la session compile encore (elle lit `tf.param.name` : remplacé en Task 7, garder la compilation verte en utilisant `tf.name()` dès maintenant dans `crates/oku-slides-session/src/fields.rs:77-127` si le compilateur l'exige).

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-core/src/validate.rs crates/oku-slides-core/src/validate_tests.rs crates/oku-slides-core/src/fields.rs crates/oku-slides-core/src/fields_tests.rs crates/oku-slides-i18n/src/locales crates/oku-slides-session/src/fields.rs
git diff --cached --name-only
git commit -m "feat(core) : validation des listes et champs texte des items (t-494)"
```

---

### Task 6: Typst — `p.list`, `list-item`, widget `frise` ; boîtes d'item au rendu

**Files:**
- Modify: `typst/oku-slides/src/params.typ`, `src/widget.typ` (`_types`, `field`, `list-item`), `src/data.typ` (`_fits`, `_args`), `src/widgets.typ` (widget `frise`), `lib.typ` (export `list-item`)
- Modify: `crates/oku-slides-render/src/lib.rs` (`ItemBox`, `Layout.items`, `items_on`)
- Modify: `fixtures/widgets/base-registry.json` (régénérer : la sonde de base gagne `frise`)
- Test: `crates/oku-slides-render/tests/lists.rs` (nouveau)

**Interfaces:**
- Consumes: schéma JSON de Task 1.
- Produces: `p.list(label:, item-label:, item:, min:, max:, default:)` ; `list-item(ctx, name, it, body)` où `body: (c) => content` ; métadonnée `<oku-slides-item>` `{owner, list, item, page, x, y, w, h}` ; `pub struct ItemBox { pub owner: String, pub list: String, pub item: String, pub page: usize, pub x: f64, pub y: f64, pub w: f64, pub h: f64 }` ; `Layout::items_on(&self, page: usize) -> &[ItemBox]` ; widget de base `frise` (élément, `etapes: list(date: plain, texte: plain(default: "Nouvelle étape"))`, `min: 0, max: 8`, `item-label: "étape"`, défaut quatre étapes 2019 → 2025).

- [ ] **Step 1: Tests RED** (`tests/lists.rs`, sur le modèle de `tests/fields.rs:107-140`)

```rust
const FRISE: &str = r#"#import "/oku-slides/lib.typ": widget, field, list-item, params as p
#let widget = widget("mini", params: (etapes: p.list(item: (date: p.plain(), texte: p.plain()),
    default: ((date: "2019", texte: "A"),))),
  (args, ctx) => stack(dir: ltr, spacing: 10pt, ..args.etapes.map(e => list-item(ctx, "etapes", e,
    c => stack(field(c, "date", e.date), field(c, "texte", e.texte))))))
"#;

#[test]
fn list_item_emits_item_boxes_and_prefixed_fields() {
    let layout = render_pack_deck(FRISE, json!({ "id": "m", "kind": "mini", "params": { "etapes": [
        { "id": "a", "date": "2019", "texte": "Un" }, { "id": "b", "date": "2021", "texte": "Deux" } ] } }));
    let items: Vec<&str> = layout.items.iter().filter(|i| i.owner == "m").map(|i| i.item.as_str()).collect();
    assert_that!(items).is_equal_to(vec!["a", "b"]);
    assert_that!(layout.fields.iter().any(|f| f.owner == "m" && f.field == "etapes/b/texte")).is_true();
    let a = layout.items.iter().find(|i| i.item == "a").unwrap();
    let b = layout.items.iter().find(|i| i.item == "b").unwrap();
    assert_that!(b.x).is_greater_than(a.x);
}

#[test]
fn list_absent_renders_no_item() {
    let layout = render_pack_deck(FRISE, json!({ "id": "m", "kind": "mini" }));
    assert_that!(layout.items.iter().filter(|i| i.owner == "m").count()).is_equal_to(0);
}

#[test]
fn list_item_without_id_gets_d_index() {
    let layout = render_pack_deck(FRISE, json!({ "id": "m", "kind": "mini", "params": { "etapes": [{ "date": "x" }] } }));
    assert_that!(layout.items.iter().any(|i| i.item == "d0")).is_true();
}

#[test]
fn base_registry_has_frise_with_a_list_param() {
    let r = load_registry(&Fonts::default(), None).unwrap();
    assert_that!(matches!(r.get("frise").unwrap().param("etapes").unwrap().ty, ParamType::List { .. })).is_true();
}
```

`render_pack_deck(src, element)` : helper local qui fait ce que fait le test de `fields.rs:107-140` (pack d'un widget `mini`, deck d'une slide `content` dont `blocks` contient `element`) et rend `compiled.layout().clone()`.

- [ ] **Step 2: Vérifier l'échec** — Run: `cargo nextest run -p oku-slides-render lists` · Expected: FAIL (`list-item` inconnu).

- [ ] **Step 3: Implémentation**

`params.typ` :

```typst
/// Liste d'items aux colonnes typées (`item` : dictionnaire de descripteurs),
/// saisie en tableau dans le dock. item-label : nom d'un item (« étape »).
#let list(label: none, item-label: none, item: (:), min: 0, max: none, default: none) = (
  _base("list", label, default)
    + (item-label: item-label, item: item.pairs().map(((k, d)) => (name: k) + d), min: min, max: max)
)
```

`widget.typ` : `_types` gagne `"list"` ; dans `field`, `let meta = (owner: str(ctx.id), field: ctx.at("field-prefix", default: "") + name)` ; ajouter :

```typst
/// Item `it` de la liste `name` : boîte <oku-slides-item> (poignées de la
/// slide) ; `body(c)` reçoit un ctx dont les champs s'adressent à l'item.
#let list-item(ctx, name, it, body) = if ctx.id == none { body(ctx) } else {
  let c = ctx + (field-prefix: ctx.at("field-prefix", default: "") + name + "/" + it.id + "/")
  _measured((owner: str(ctx.id), list: name, item: it.id), "oku-slides-item", body(c), (:))
}
```

`data.typ` : `_fits` gagne `else if ty == "list" { type(v) == array and v.all(x => type(x) == dictionary) }`. Extraire de `_args` la conversion d'une valeur scalaire :

```typst
// Valeur prête à rendre d'un paramètre ou d'une colonne `d` (hors slot et liste).
#let _ready(t, d, v, vars) = if d.type == "plain" { interp(v, vars) }
  else if d.type == "rich" { if v == none or v == () { [] } else { _paragraphs(t, v, vars) } }
  else { v }

// Items d'une liste : id (`d<i>` s'il manque), colonnes au défaut, valeurs prêtes.
#let _items(t, d, v, vars) = (if v == none { () } else { v }).enumerate().map(((i, it)) => {
  let out = (id: it.at("id", default: "d" + str(i)))
  for c in d.item {
    let cv = it.at(c.name, default: c.default)
    let cv = if cv == none or _fits(c, cv) { cv } else { c.default }
    out.insert(c.name, _ready(t, c, cv, vars))
  }
  out
})
```

et dans `_args` : la valeur d'une liste se lit `params.at(d.name, default: none)` (liste absente = vide, A1 bis) ; `out.insert(d.name, if d.type == "slot" {…} else if d.type == "list" { _items(t, d, v, vars) } else { _ready(t, d, v, vars) })`.

`lib.typ` : `#import "src/widget.typ": widget, field, fields-stack, list-item, by-name` ; mettre à jour le commentaire d'import des widgets de pack.

`widgets.typ`, widget de base (reprend la frise de la capture, couleurs et tailles du thème) :

```typst
#let _frise = widget("frise", label: "Frise chronologique",
  params: (etapes: p.list(label: "Étapes", item-label: "étape", min: 0, max: 8,
    item: (date: p.plain(label: "Date"), texte: p.plain(label: "Étape", default: "Nouvelle étape")),
    default: ((date: "2019", texte: "Création"), (date: "2021", texte: "Premier client"),
      (date: "2023", texte: "International"), (date: "2025", texte: "Série B")))),
  (args, ctx) => {
    let t = ctx.tokens
    let accent = color-of(t, "accent")
    set text(size: size-of(t, "body"))
    let n = calc.max(args.etapes.len(), 1)
    block(width: 100%, {
      place(top + left, dy: 2.4em + 0.45em, line(length: 100%, stroke: 1.5pt + accent))
      grid(columns: (1fr,) * n, column-gutter: 0.5em,
        ..args.etapes.map(e => list-item(ctx, "etapes", e, c => align(center, stack(spacing: 0.5em,
          box(height: 2.4em, align(bottom, text(weight: "bold", field(c, "date", e.date)))),
          circle(radius: 0.45em, fill: accent),
          field(c, "texte", e.texte))))))
    })
  })
```

Ajouter `_frise` à `registry`. Régénérer `fixtures/widgets/base-registry.json` : chercher la commande ou le test qui l'écrit (`rg "base-registry" crates justfile`), sinon écrire la sonde de base sérialisée avec un test `#[ignore]` existant ou la mettre à jour à la main à partir de `load_registry(&Fonts::default(), None)` sérialisé.

`render/src/lib.rs` :

```rust
/// Boîte d'un item de liste (`list-item`), pour les poignées de la slide.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemBox {
    /// Id de l'élément ou de la slide.
    pub owner: String,
    /// Nom du paramètre `list`.
    pub list: String,
    /// Id de l'item.
    pub item: String,
    pub page: usize,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}
```

`Layout` gagne `pub items: Vec<ItemBox>` ; dans `layout()`, `let mut items: Vec<ItemBox> = parsed(values("oku-slides-item")); items.sort_by_key(|i| i.page);` ; `items_on(page)` sur le modèle de `fields_on` (`partition_point`). Réexporter `ItemBox`.

- [ ] **Step 4: Vérifier** — Run: `cargo nextest run -p oku-slides-render && cargo nextest run --workspace --exclude oku-slides-app` · Expected: PASS (les tests qui comptent les widgets de base sont mis à jour pour `frise`).

- [ ] **Step 5: Commit**

```bash
git add typst/oku-slides crates/oku-slides-render fixtures/widgets/base-registry.json
git diff --cached --name-only
git commit -m "feat(typst) : p.list, list-item et widget de base frise ; boîtes d'item au rendu (t-494)"
```

---

### Task 7: Session — champs d'item, `ItemView`, valeurs d'essai d'une liste

**Files:**
- Modify: `crates/oku-slides-session/src/fields.rs` (`FieldView`, `field_views`)
- Create: `crates/oku-slides-session/src/items.rs`, `items_tests.rs`
- Modify: `crates/oku-slides-session/src/views.rs:309` (`SlideView.items`), `lib.rs:862-890` (construction), `lib.rs:572-576` (dispatch)
- Modify: `crates/oku-slides-session/src/request.rs` (`EditWidgetTrialList`, `changes_deck`, `edits`)
- Modify: `crates/oku-slides-session/src/widget_params.rs` (`settable`, `field`, `trial_value`, `edit_widget_trial_list`), `widget_preview.rs:170-175`
- Test: `crates/oku-slides-session/src/fields_tests.rs`, `items_tests.rs`, `widget_params_tests.rs`

**Interfaces:**
- Consumes: `TextField::name`, `values`, `list` (Task 5) ; `ItemBox`, `items_on` (Task 6) ; `ListOp`, `ListSpec`, `default_items` (Task 3).
- Produces:
  - `FieldView` : `field: Cow<'a, str>`, `list: Option<&'a str>`, `item: Option<&'a str>`, `column: &'a str`
  - `pub struct ItemView<'a> { pub owner: &'a str, pub owner_kind: OwnerKind, pub list: &'a str, pub item: &'a str, pub index: usize, pub len: usize, pub min: usize, pub max: Option<usize>, pub item_label: Option<&'a str>, pub first_text: Option<&'a str>, pub x: f64, pub y: f64, pub w: f64, pub h: f64 }`
  - `SlideView.items: Vec<ItemView<'a>>`
  - `Request::EditWidgetTrialList { name: String, param: String, change: ListOp }` (JSON `{"cmd":"edit_widget_trial_list",…}`)
  - `Session::edit_widget_trial_list(&mut self, name: &str, param: &str, change: ListOp) -> Result<Map<String, Value>>`

- [ ] **Step 1: Tests RED**

`fields_tests.rs` (deck de la fixture + élément `frise` ajouté par `add_element`) :

```rust
#[test]
fn field_views_of_a_frise_carry_list_item_and_column() {
    let mut s = session_with_fixture();
    s.apply("frise", vec![add_frise()], None).unwrap();
    let view = s.slide("s-constat").unwrap();
    let f = view.fields.iter().find(|f| f.field == "etapes/d1/texte").unwrap();
    assert_that!((f.list, f.item, f.column)).is_equal_to((Some("etapes"), Some("d1"), "texte"));
    assert_that!(f.text.as_ref()).is_equal_to("Premier client");
}
```

`items_tests.rs` :

```rust
#[test]
fn item_views_follow_list_order_with_bounds() {
    let mut s = session_with_fixture();
    s.apply("frise", vec![add_frise()], None).unwrap();
    let view = s.slide("s-constat").unwrap();
    let items: Vec<(&str, usize)> = view.items.iter().map(|i| (i.item, i.index)).collect();
    assert_that!(items).is_equal_to(vec![("d0", 0), ("d1", 1), ("d2", 2), ("d3", 3)]);
    let i = &view.items[0];
    assert_that!((i.len, i.min, i.max, i.item_label, i.first_text)).is_equal_to((4, 0, Some(8), Some("étape"), Some("date")));
}
```

`widget_params_tests.rs` (widget de pack à liste dans `templates()`) :

```rust
#[test]
fn widget_settings_lists_a_list_param_as_a_list_field() {}     // FieldKind::List, colonnes date/texte, valeur = items d0…
#[test]
fn edit_widget_trial_list_inserts_into_the_default_items() {
    let (_d, mut s) = templates_with_list();
    s.edit_widget_trial_list("frise2", "etapes", ListOp::Insert { index: None, id: "i-1".into(), values: Default::default() }).unwrap();
    let ids: Vec<String> = trial(&s, "etapes").unwrap().as_array().unwrap().iter()
        .map(|i| i["id"].as_str().unwrap().to_owned()).collect();
    assert_that!(ids.last().map(String::as_str)).is_equal_to(Some("i-1"));
}
#[test]
fn set_widget_trials_accepts_a_whole_list() {}                  // collage : liste entière vérifiée par list_value
```

- [ ] **Step 2: Vérifier l'échec** — Run: `cargo nextest run -p oku-slides-session fields items widget_params` · Expected: FAIL.

- [ ] **Step 3: Implémentation**

`fields.rs` : `FieldView` gagne `list`, `item`, `column`, et `field` devient `Cow<'a, str>`. Dans `field_views` : `let name = tf.name();`, boîte trouvée par `b.field == name`, texte lu dans `tf.values` (`bind` seulement hors item), `column: &tf.param.name`, `list: tf.list.map(|(l, _)| l)`, `item: tf.list.map(|(_, i)| i.id.as_str())`, `label: tf.param.label.as_deref()`.

`items.rs` :

```rust
//! Items de liste d'une page : boîte, rang et bornes de leur liste, pour les
//! poignées de la scène (ajouter, retirer, déplacer).

use oku_slides_core::{ListSpec, ParamType, Slide, WidgetRegistry};
use oku_slides_render::ItemBox;
use serde::Serialize;

use crate::fields::OwnerKind;

/// Un item sur la page et sa liste.
#[derive(Debug, Serialize)]
pub struct ItemView<'a> { /* champs de « Produces » */ }

/// Items des boîtes `boxes` qui appartiennent à `slide`, dans l'ordre des boîtes.
pub(crate) fn item_views<'a>(slide: &'a Slide, registry: &'a WidgetRegistry, boxes: &'a [ItemBox]) -> Vec<ItemView<'a>> {
    boxes.iter().filter_map(|b| {
        let (owner_kind, kind, params) = if b.owner == slide.id {
            (OwnerKind::Slide, slide.kind.as_str(), &slide.params)
        } else {
            let e = slide.locate(&b.owner)?.element;
            (OwnerKind::Element, e.kind.as_str(), &e.params)
        };
        let p = registry.get(kind)?.param(&b.list)?;
        let spec = ListSpec::of(&p.name, &p.ty)?;
        let items = params.list(&b.list)?;
        let index = items.iter().position(|i| i.id == b.item)?;
        let ParamType::List { item_label, .. } = &p.ty else { return None };
        let first_text = spec.columns.iter()
            .find(|c| matches!(c.ty, ParamType::Plain | ParamType::Rich)).map(|c| c.name.as_str());
        Some(ItemView { owner: &b.owner, owner_kind, list: &p.name, item: &items[index].id, index,
            len: items.len(), min: spec.min, max: spec.max, item_label: item_label.as_deref(), first_text,
            x: b.x, y: b.y, w: b.w, h: b.h })
    }).collect()
}
```

`views.rs` : `SlideView` gagne `/// Items de liste de la page (poignées). pub items: Vec<ItemView<'a>>`. `lib.rs:878` : `items: items::item_views(slide, &self.registry, layout.items_on(page)),` ; `mod items;` et réexport `ItemView`.

`request.rs` : variante `EditWidgetTrialList { name: String, param: String, change: oku_slides_core::ListOp }`, classée comme `SetWidgetTrial` dans `changes_deck()` et `edits()`. `lib.rs` : `Request::EditWidgetTrialList { name, param, change } => { self.edit_widget_trial_list(&name, &param, change)?; }`.

`widget_params.rs` :

- `settable` : `List` est réglable (retirer de l'exclusion s'il y figure par le `_`).
- `field` : bras `ParamType::List { item, item_label, min, max }` → `ParamField { kind: FieldKind::List, columns: item.iter().map(column).collect(), item_label: item_label.clone(), min: Some(*min as f64), max: max.map(|m| m as f64), value: shown items (essai, sinon `default_items(p)`) en JSON, op_template: Value::Null, .. }`, avec `fn column(c: &ParamSchema) -> ListColumn` : `plain` → `"text"`, `rich` → `"rich"`, `number` → `"number"`, `flag` → `"flag"`, `choice` → `"select"` (+ options), `asset` → `"asset"`.
- `trial_value` : pour `List`, `list_value` via `param_value` (liste entière, collage) puis sérialisation.
- `edit_widget_trial_list` :

```rust
    /// Change la liste d'essai `param` du widget `name` (défaut du schéma si
    /// aucune) ; renvoie les essais du widget.
    pub fn edit_widget_trial_list(&mut self, name: &str, param: &str, change: ListOp) -> Result<Map<String, Value>> {
        let file = parse_name(name)?;
        ensure!(self.scratch().is_some(), NotEditingTemplatesSnafu);
        let schema = self.valid_schema(&file)?.context(WidgetInvalidSnafu { name: file.as_str() })?;
        let p = schema.param(param).context(TrialUndeclaredSnafu { param: clipped(param, 80) })?;
        let spec = ListSpec::of(&p.name, &p.ty).context(TrialUndeclaredSnafu { param: clipped(param, 80) })?;
        let scratch = self.scratch_mut().context(NotEditingTemplatesSnafu)?;
        let mut items: Vec<ListItem> = match scratch.trials_of(&file).and_then(|t| t.get(param)) {
            Some(v) => serde_json::from_value(v.clone()).unwrap_or_default(),
            None => default_items(p).context(OpSnafu)?,
        };
        change.apply(&mut items, &spec).context(OpSnafu)?;
        scratch.set_trial(&file, param, Some(serde_json::to_value(&items).unwrap_or(Value::Null)));
        Ok(scratch.trials_of(&file).cloned().unwrap_or_default())
    }
```

(`OpSnafu` : le sélecteur d'erreur de session qui enveloppe un `OpError` — utiliser celui qu'emploie déjà `Session::apply`, ou en ajouter un dans `error.rs` de la session si aucun n'existe.)

`widget_preview.rs:170-175` : `ParamType::List { .. }` → items par défaut (`default_items(p)` sérialisés) comme valeur neutre d'aperçu.

- [ ] **Step 4: Vérifier** — Run: `cargo nextest run -p oku-slides-session && cargo nextest run -p oku-slides-mcp` · Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-session
git diff --cached --name-only
git commit -m "feat(session) : champs et items de liste dans la vue, liste d'essai du widget (t-494)"
```

---

### Task 8: Dock — `FieldKind::List` et composant `ListTable`

**Files:**
- Modify: `crates/oku-slides-session/src/agent/forms.rs` (`FieldKind::List`, `ParamField.columns`/`item_label`, `ListColumn`, bras de `params_with`)
- Modify: `crates/oku-slides-a2ui/src/message.rs:84+` (`ComponentKind::ListTable`, et la liste de l'arm l.346)
- Create: `ui/src/table/cells.ts`, `ui/src/table/cells.test.ts`
- Modify: `ui/src/chart/ChartDataGrid.tsx` (importe `parseCell`, `parseCheck`, `parseTsv` de `../table/cells`), `ui/src/chart/ChartDataGrid.test.ts`
- Create: `ui/src/a2ui/ListTable.tsx`, `ui/src/a2ui/ListTable.css`, `ui/src/a2ui/ListTable.render.test.tsx`
- Modify: `ui/src/a2ui/catalog.tsx` (Api + implémentation + liste), `ui/src/a2ui/catalog.test.tsx` (`V3`), `ui/src/useHostSettings.ts:77-93`, `ui/src/api.ts` (`editWidgetTrialList`)
- Modify: `crates/oku-slides-i18n/src/locales/*/ui.ftl` (`ui-list-*`)
- Test: `crates/oku-slides-session/src/agent/forms_tests.rs` (ou fichier de tests de `forms.rs` existant)

**Interfaces:**
- Consumes: `ParamField { kind: List, columns, item_label, min, max, value }` (Task 7).
- Produces:
  - Rust : `pub struct ListColumn { pub key: String, pub label: String, pub kind: ColumnKind, #[serde(default)] pub options: Vec<ChoiceOption> }`, `pub enum ColumnKind { Text, Rich, Number, Flag, Select, Asset }` (snake_case) ; `ComponentKind::ListTable { label: Dynamic<String>, rows: Dynamic<Value>, change: Dynamic<Value>, columns: Vec<ListColumn>, min: u32, max: Option<u32>, item_label: Option<String>, palette: Vec<String>, action: Option<Action> }`.
  - Action du dock : `event("param", [("field", i), ("change", path), ("key", k)])`.
  - TS : `api.editWidgetTrialList(name: string, param: string, change: ListOp): Promise<Snapshot>` ; `type ListOp = { op: "insert"; index?: number; id: string; values?: Record<string, unknown> } | { op: "remove"; item: string } | { op: "move"; item: string; to: number } | { op: "set_field"; item: string; field: string; value: unknown }` (dans `api.ts`).

- [ ] **Step 1: Tests RED**

`ui/src/table/cells.test.ts` : reprendre les cas existants de `parseCheck` et `parseTsv` de `ChartDataGrid.test.ts`, plus :

```ts
import { expect, test } from "bun:test";
import { coerce } from "./cells";

test("coerce : texte rogné, nombre à virgule, case, choix hors liste refusé", () => {
  expect(coerce("text", " a ")).toEqual({ ok: true, value: "a" });
  expect(coerce("number", "3,5")).toEqual({ ok: true, value: 3.5 });
  expect(coerce("flag", "oui")).toEqual({ ok: true, value: true });
  expect(coerce("select", "z", ["a", "b"])).toEqual({ ok: false });
  expect(coerce("number", "abc")).toEqual({ ok: false });
});
```

`ListTable.render.test.tsx` (sur le modèle de `catalog.render.test.tsx`, `mount` de `ListTable` avec `rows` dans les données et l'action `param`) :

```ts
test("ListTable : « Ajouter » envoie un insert à la fin avec un id neuf", async () => {
  const { host, actions, data } = await mount(table({ rows: [{ id: "d0", date: "2019" }], max: 3 }));
  await click(host.querySelector("[data-list-add]") as Element);
  const change = data("/f0-change") as { op: string; id: string; index?: number };
  expect(change.op).toBe("insert");
  expect(change.id.startsWith("i-")).toBe(true);
  expect(actions.map((a) => a.name)).toEqual(["param"]);
});
test("ListTable : « Ajouter » désactivé à max, « × » désactivé à min", async () => {});
test("ListTable : frappe dans une cellule texte → set_field au blur", async () => {});
test("ListTable : Alt+↓ sur une ligne → move à l'index suivant", async () => {});
test("ListTable : collage TSV de deux lignes → un seul changement de liste entière", async () => {});
test("ListTable : cellule rich → popover avec RichTextField", async () => {});
test("ListTable : colonne asset en lecture seule", async () => {});
```

(Si le collage doit produire une liste entière, le composant émet `{ op: "replace", items }` côté TS, que `useHostSettings` envoie par `setWidgetTrial(name, param, items)` : préciser dans le test que l'action reçoit `change.op === "replace"`.)

`forms_tests.rs` : une `ShowParams` à un champ `FieldKind::List` produit un composant `ListTable` dont `rows` pointe sur `/f0` et l'action porte `change` → `/f0-change`.

- [ ] **Step 2: Vérifier l'échec** — Run: `cargo nextest run -p oku-slides-session forms && cd ui && bun test src/table src/a2ui` · Expected: FAIL.

- [ ] **Step 3: Implémentation**

Rust, `forms.rs` : `FieldKind::List` (doc : « Liste d'items, saisie en tableau : `value` = items ») ; `ParamField` gagne `#[serde(default)] pub columns: Vec<ListColumn>` et `#[serde(default)] pub item_label: Option<String>` (mettre `columns: Vec::new(), item_label: None` aux constructions existantes, `panel.rs` compris). Dans `params_with` :

```rust
            FieldKind::List => {
                let change = format!("{path}-change");
                data.insert(format!("{key}-change"), Value::Null);
                let mut context = vec![("field", lit(json!(i))), ("change", Dynamic::path(&change))];
                if let Some(k) = extras.keys.get(i) { context.push(("key", lit(json!(k)))); }
                ComponentKind::ListTable {
                    label, rows: Dynamic::path(&path), change: Dynamic::path(&change), columns: f.columns.clone(),
                    min: f.min.map_or(0, |m| m as u32), max: f.max.map(|m| m as u32),
                    item_label: f.item_label.clone(), palette: palette.to_vec(),
                    action: Some(Action::event("param", context)),
                }
            }
```

`message.rs` : variante `ListTable` (champs de « Produces », `#[serde(skip_serializing_if = "Option::is_none")]` sur les `Option`), ajoutée au même arm que `Select` l.346 ; `ListColumn`/`ColumnKind` vivent dans `oku-slides-a2ui` (réexportés et utilisés par `forms.rs`).

TS, `ui/src/table/cells.ts` (module doc : « Cellules d'une grille : collage TSV et conversion d'une saisie, partagés par les tableaux du dock et des graphiques ») : déplacer `parseCell`, `parseCheck`, `parseTsv` depuis `ChartDataGrid.tsx` (qui les importe) et ajouter :

```ts
export type ColumnKind = "text" | "rich" | "number" | "flag" | "select" | "asset";
/** Saisie `raw` convertie au type de colonne ; `ok: false` si elle n'y tient pas. */
export function coerce(kind: ColumnKind, raw: string, options: readonly string[] = []):
  { ok: true; value: unknown } | { ok: false } {
  const t = raw.trim();
  if (kind === "number") { const c = parseCell(t); return typeof c === "number" ? { ok: true, value: c } : { ok: false }; }
  if (kind === "flag") return { ok: true, value: parseCheck(t) };
  if (kind === "select") return options.includes(t) ? { ok: true, value: t } : { ok: false };
  if (kind === "asset") return { ok: false };
  return { ok: true, value: t };
}
```

`ui/src/a2ui/ListTable.tsx` — composant contrôlé (aucune donnée gardée localement hors brouillon de cellule) :

```tsx
// Tableau d'une liste au dock : une ligne par item, colonnes du schéma.
// Contrôlé : il émet un ListOp (ou un remplacement au collage) ; le cœur
// applique, et les lignes reviennent par `rows`.
export type ListTableProps = {
  label: string; rows: ListRow[]; columns: ListColumn[]; min: number; max: number | null;
  itemLabel: string | null; palette: string[]; onChange: (change: ListChange) => void;
};
export type ListRow = { id: string } & Record<string, unknown>;
export type ListChange = ListOp | { op: "replace"; items: ListRow[] };
```

Rendu : `<table className="lt">` avec TanStack Table (`useTable`, `tableFeatures({})`, comme `ChartDataGrid`) ; colonne de poignée (`⋮⋮`, `aria-label={t("ui-list-move", { item })}`, Alt+↑/↓ → `move`, glisser au pointeur entre lignes → `move`) ; une colonne par `columns` (`text` : `<input>` qui commit au blur/Entrée, Échap annule ; `number` : `inputMode="decimal"` + `coerce` ; `flag` : case ; `select` : `<select>` ; `rich` : aperçu une ligne (`paragraphsToText` existant) + bouton qui ouvre un popover avec `RichTextField` (`onCommit(markdown, paragraphs)` → `set_field` avec `paragraphs`) ; `asset` : texte court en lecture seule) ; colonne « × » (`aria-label={t("ui-list-remove", { item })}`, `disabled={rows.length <= min}`) ; pied `<button data-list-add disabled={max !== null && rows.length >= max} title={…ui-list-full…}>{t("ui-list-add", { item })}</button>` qui émet `{ op: "insert", id: newId("i") }`, et Entrée dans la dernière ligne aussi. Collage : `onPaste` d'une cellule, si le texte contient tabulation ou saut de ligne → `parseTsv`, conversion par `coerce` à partir de la cellule courante, lignes manquantes créées avec `newId("i")` (jusqu'à `max`), cellules invalides laissées et signalées par `data-invalid` pendant 2 s → `{ op: "replace", items }`. `item` = `itemLabel ?? t("ui-list-item")`. Hauteur bornée par `.lt-scroll { max-height: 18rem; overflow: auto }`.

`catalog.tsx` : `ListTableApi = { name: "ListTable", schema: z.object({ label: CommonSchemas.DynamicString, rows: <même type que `source` de RichText>, change: <idem>, columns: z.array(ListColumnSchema).readonly(), min: z.number(), max: z.number().nullable().optional(), item_label: z.string().nullable().optional(), palette: z.array(z.string()).readonly(), action: CommonSchemas.Action.optional() }).strict() }` ; implémentation : `onChange={(c) => { props.setChange(c); props.action?.(); }}` ; ajouter `ListTable` à `new Catalog(...)` et à `V3` dans `catalog.test.tsx`.

`useHostSettings.ts` : avant le routage actuel,

```ts
if (name === "param" && typeof field === "string" && context.change && s.id === HOST_WIDGET && now.widget) {
  const c = context.change as ListChange;
  void run(c.op === "replace" ? api.setWidgetTrial(now.widget, field, c.items) : api.editWidgetTrialList(now.widget, field, c));
  return true;
}
```

`api.ts` : `editWidgetTrialList: (name, param, change) => call<Snapshot>({ cmd: "edit_widget_trial_list", name, param, change })`, type de requête et `ListOp`.

`ui.ftl` (20 langues) — en / fr :

```ftl
ui-list-item = item
ui-list-add = Add { $item }
ui-list-remove = Remove { $item }
ui-list-move = Move { $item }
ui-list-full = The list is full ({ $max })
ui-undo-list-add = Add { $item }
ui-undo-list-remove = Remove { $item }
ui-undo-list-move = Move { $item }
```

```ftl
ui-list-item = élément
ui-list-add = Ajouter { $item }
ui-list-remove = Supprimer { $item }
ui-list-move = Déplacer { $item }
ui-list-full = La liste est pleine ({ $max })
ui-undo-list-add = Ajouter { $item }
ui-undo-list-remove = Supprimer { $item }
ui-undo-list-move = Déplacer { $item }
```

(`$item` reçoit `item-label` ou `ui-list-item` ; « Ajouter une étape » vient d'un `item-label` « une étape » — la doc du descripteur le dit.)

- [ ] **Step 4: Vérifier** — Run: `cargo nextest run -p oku-slides-session -p oku-slides-a2ui -p oku-slides-i18n && cd ui && bun run test && bun run typecheck` · Expected: PASS (dont `catalog.test.tsx`, `i18n-keys.test.ts`).

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-session/src/agent crates/oku-slides-session/src/panel.rs crates/oku-slides-a2ui crates/oku-slides-i18n/src/locales ui/src/table ui/src/chart ui/src/a2ui ui/src/useHostSettings.ts ui/src/api.ts
git diff --cached --name-only
git commit -m "feat(ui) : tableau d'une liste au dock de l'éditeur de widget (t-494)"
```

---

### Task 9: Scène — champs d'item et poignées `ItemLayer`

**Files:**
- Modify: `ui/src/api.ts:293-331` (`FieldView`, `ItemView`, `SlideView.items`)
- Modify: `ui/src/deck.ts:106` (`fieldOps`)
- Create: `ui/src/stageItems.ts`, `ui/src/stageItems.test.ts`, `ui/src/ItemLayer.tsx`, `ui/src/ItemLayer.render.test.tsx`
- Modify: `ui/src/Stage.tsx` (couche, Alt+flèches), `ui/src/useFieldEditing.ts` (`editKeyWhenShown`), `ui/src/App.tsx:879-888` (câblage), `ui/src/index.css` ou la feuille de la scène (styles)
- Test: `ui/src/deck.test.ts` (existant ou créé)

**Interfaces:**
- Consumes: `FieldView.list/item/column`, `SlideView.items` (Task 7), `ListOp` TS (Task 8).
- Produces:
  - `export interface ItemView { owner: string; owner_kind: "slide" | "element"; list: string; item: string; index: number; len: number; min: number; max: number | null; item_label: string | null; first_text: string | null; x: number; y: number; w: number; h: number }`
  - `stageItems.ts` : `groups(items: ItemView[]): ItemView[][]` (par `owner`+`list`, ordre `index`) ; `axisOf(group: ItemView[], frame?: {w: number; h: number}): "x" | "y"` ; `insertPoints(group, axis): { index: number; x: number; y: number }[]` (avant le premier, entre deux, après le dernier) ; `dropIndex(group, axis, at: number): number`.
  - `useFieldEditing` : `editKeyWhenShown(key: FieldKey)`.

- [ ] **Step 1: Tests RED**

`stageItems.test.ts` :

```ts
import { expect, test } from "bun:test";
import { axisOf, dropIndex, groups, insertPoints } from "./stageItems";
const item = (item: string, index: number, x: number, y = 0) =>
  ({ owner: "f", owner_kind: "element", list: "etapes", item, index, len: 3, min: 0, max: null,
     item_label: null, first_text: "date", x, y, w: 100, h: 50 }) as const;

test("axisOf : items côte à côte → x, empilés → y", () => {
  expect(axisOf([item("a", 0, 0), item("b", 1, 120)])).toBe("x");
  expect(axisOf([item("a", 0, 0, 0), item("b", 1, 0, 80)])).toBe("y");
});
test("insertPoints : avant, entre, après", () => {
  const pts = insertPoints([item("a", 0, 0), item("b", 1, 120)], "x");
  expect(pts.map((p) => p.index)).toEqual([0, 1, 2]);
  expect(pts[1].x).toBe(110);
});
test("dropIndex : position le long de l'axe → index cible", () => {
  const g = [item("a", 0, 0), item("b", 1, 120), item("c", 2, 240)];
  expect(dropIndex(g, "x", 10)).toBe(0);
  expect(dropIndex(g, "x", 300)).toBe(2);
});
test("groups : regroupe par propriétaire et liste, trié par index", () => {
  expect(groups([item("b", 1, 120), item("a", 0, 0)])[0].map((i) => i.item)).toEqual(["a", "b"]);
});
```

`deck.test.ts` :

```ts
test("fieldOps : un champ d'item plain → edit_list set_field", () => {
  const ops = fieldOps(view, { ...field, field: "etapes/d0/date", list: "etapes", item: "d0", column: "date", kind: "plain" }, "2020");
  expect(ops).toEqual([{ op: "edit_list", target: { slide: "s", element: "f" }, param: "etapes",
    change: { op: "set_field", item: "d0", field: "date", value: "2020" } }]);
});
```

`ItemLayer.render.test.tsx` : avec un groupe de deux items et l'élément sélectionné, trois boutons « + » (`[data-item-insert]`), un clic sur le deuxième appelle `onApply` avec `{ op: "edit_list", …, change: { op: "insert", index: 1, id } }` puis `onInserted` avec la clé `f\u001fetapes/<id>/date` ; « + » absent à `max` ; « × » absent à `min` ; aucun bouton si l'élément n'est ni sélectionné ni en édition.

- [ ] **Step 2: Vérifier l'échec** — Run: `cd ui && bun test src/stageItems src/deck && bun test .render.test. --test-name-pattern ItemLayer` · Expected: FAIL.

- [ ] **Step 3: Implémentation**

`api.ts` : `FieldView` gagne `list: string | null; item: string | null; column: string;` ; `ItemView` ; `SlideView.items: ItemView[]`.

`deck.ts`, en tête de `fieldOps` :

```ts
  if (f.list && f.item) {
    const value = f.kind === "plain" ? text : textToParagraphs(text, { paragraphs: itemRich(owner, f) ?? [], text: f.text });
    return [{ op: "edit_list", target, param: f.list, change: { op: "set_field", item: f.item, field: f.column, value } }];
  }
```

avec `itemRich(owner, f)` qui lit `owner.params[f.list]` → item `f.item` → `[f.column]` en paragraphes (réordonner le calcul de `owner` avant ce bloc).

`stageItems.ts` : fonctions pures de « Produces » ; `axisOf` compare l'écart des centres en x et en y du premier et du dernier item ; un seul item : `"x"` si `frame.w - item.w >= frame.h - item.h`, sinon `"y"` ; `insertPoints` place les points au milieu des intervalles (avant le premier : bord de début moins 10 pt ; après le dernier : bord de fin plus 10 pt) ; `dropIndex` rend le nombre de centres d'items situés avant `at`, borné à `len - 1`.

`ItemLayer.tsx` :

```tsx
// Poignées des items de liste sur la scène : « + » entre les items, « × »
// et ⋮⋮ sur l'item survolé. Montrées pour l'élément sélectionné ou en édition.
export function ItemLayer({ view, scale, active, onApply, onInserted }: {
  view: SlideView; scale: number; active: string | null;
  onApply: (label: string, ops: Op[]) => void; onInserted: (key: FieldKey) => void;
}) { /* … */ }
```

- `active` = id de l'élément sélectionné, ou propriétaire du champ en édition ; seuls les groupes de ce propriétaire sont rendus.
- `target` = `{ slide }` ou `{ slide, element: owner }` selon `owner_kind`.
- « + » (`data-item-insert`, `aria-label={t("ui-list-add", { item })}`) : `id = newId("i")`, `onApply(t("ui-undo-list-add", { item }), [{ op: "edit_list", target, param: list, change: { op: "insert", index, id } }])`, puis `onInserted(fieldKey({ owner, field: `${list}/${id}/${first_text}` }))` si `first_text`. Caché si `max !== null && len >= max`.
- « × » (`data-item-remove`) au coin de l'item survolé ou focalisé : `remove` ; caché si `len <= min`.
- ⋮⋮ (`data-item-move`) : glisser au pointeur ; un repère est dessiné à `insertPoints` le plus proche ; au relâcher, `to = dropIndex(group, axis, position)` ; si `to !== index`, `move`.
- Positions en pt × `scale`, comme `GridLayer`.

`Stage.tsx` : rendre `<ItemLayer view={view} scale={scale} active={editField?.owner ?? selected} onApply={onApply} onInserted={onEditWhenShown} />` après la couche `fields`, si `!readOnly`. Alt+flèche le long de l'axe pendant l'édition d'un champ d'item : enregistrer le texte en cours (`commitEditing`), puis `move` à `index ± 1` (borné), avec le libellé `ui-undo-list-move`. Nouvelle prop `onEditWhenShown: (key: FieldKey) => void`.

`useFieldEditing.ts` : `editKeyWhenShown(key)` garde une clé en attente ; quand `view.fields` contient un champ de cette clé, `setEditing(key)` et la clé est oubliée. Le champ s'ouvre texte sélectionné (option déjà offerte par l'éditeur, sinon sélectionner tout à l'ouverture pour ce cas). `App.tsx` passe `onEditWhenShown={editKeyWhenShown}`.

Styles : `.item-insert`, `.item-remove`, `.item-move` (pastilles 20 px, couleur d'accent de l'interface, visibles au survol de la scène pour l'élément actif ; cible tactile ≥ 24 px).

- [ ] **Step 4: Vérifier** — Run: `cd ui && bun run test && bun run typecheck` · Expected: PASS.

- [ ] **Step 5: Vérifier dans l'app** — `export OKU_SLIDES_PATHS__DATA=$(mktemp -d) OKU_SLIDES_PATHS__DECKS=$(mktemp -d)`, lancer la préversion (`preview_start`), ajouter une frise, vérifier « + », « × », ⋮⋮, focus après insertion, Alt+flèches ; capture d'écran.

- [ ] **Step 6: Commit**

```bash
git add ui/src/api.ts ui/src/deck.ts ui/src/deck.test.ts ui/src/stageItems.ts ui/src/stageItems.test.ts ui/src/ItemLayer.tsx ui/src/ItemLayer.render.test.tsx ui/src/Stage.tsx ui/src/useFieldEditing.ts ui/src/App.tsx ui/src/index.css
git diff --cached --name-only
git commit -m "feat(ui) : poignées des items de liste sur la slide, ajout, retrait et déplacement (t-494)"
```

---

### Task 10: Agent et documentation

**Files:**
- Modify: `crates/oku-slides-i18n/src/locales/*/agent.ftl` (`agent-instructions-widgets`)
- Modify: `crates/oku-slides-i18n/src/lib_tests.rs:162` (test des consignes)
- Modify: `crates/oku-slides-core/src/ops/mod.rs` (doc de `EditList`, déjà écrite en Task 3 : relire)
- Modify: `typst/oku-slides/README.md` (`### Descripteurs de paramètres`, nouvelle section `### list-item(ctx, name, it, body)`, `### Surcharger un widget de base`, paragraphe des exports, exemple frise)
- Modify: `crates/oku-slides-core/ARCHITECTURE.md`, `crates/oku-slides-render/ARCHITECTURE.md`, `crates/oku-slides-session/ARCHITECTURE.md`
- Modify: deck d'aide en lecture seule (section bibliothèque Typst, t-469 : `rg -l "fields-stack" fixtures ui/src crates/*/assets` pour le trouver), FR et EN
- Modify: `crates/oku-slides-mcp/src/tools.rs` (description de l'outil `apply`, si elle énumère les ops)

**Interfaces:**
- Consumes: tout ce qui précède.

- [ ] **Step 1: Test RED** — dans `lib_tests.rs`, ajouter au test des consignes de widgets (ou un test voisin) :

```rust
#[test]
fn widgets_prompt_mentions_p_list_and_list_item_in_every_locale() {
    for loc in LOCALES {
        let text = localizer(loc).tr("agent-instructions-widgets");
        assert_that!(text.contains("p.list")).is_true();
        assert_that!(text.contains("list-item")).is_true();
    }
}
```

- [ ] **Step 2: Vérifier l'échec** — Run: `cargo nextest run -p oku-slides-i18n widgets_prompt` · Expected: FAIL.

- [ ] **Step 3: Implémentation** — ajouter aux 20 `agent-instructions-widgets` (en, puis traduit) : « Lists: p.list(item: (date: p.plain(), texte: p.plain()), item-label: "a step", max: 8) for repeated items (timeline, KPIs, team); render each with list-item(ctx, "etapes", e, c => field(c, "date", e.date)). Items keep stable ids; edit them with edit_list (insert/remove/move/set_field by id) or replace the whole list with set_param. Never simulate a list with numbered params (date1, date2…). » README : ligne de tableau `| \`p.list(label:, item-label:, item:, min:, max:, default:)\` | Liste d'items aux colonnes typées (plain, rich, number, flag, choice, asset) ; saisie en tableau ; liste absente = vide, \`default\` amorce un élément créé |`, section `list-item` avec l'exemple de Task 6, règle de surcharge (« une liste doit rester identique »). ARCHITECTURE.md : une ligne chacun (cœur : `ListOp`, conteneur `{list}` ; rendu : `ItemBox` ; session : `ItemView`, `edit_widget_trial_list`). Deck d'aide : une diapositive « Listes » (FR/EN).

- [ ] **Step 4: Vérifier** — Run: `cargo nextest run -p oku-slides-i18n && just check` · Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-i18n typst/oku-slides/README.md crates/*/ARCHITECTURE.md crates/oku-slides-mcp/src/tools.rs <fichiers du deck d'aide>
git diff --cached --name-only
git commit -m "docs : listes p.list pour l'agent, la bibliothèque Typst et l'aide (t-494)"
```

---

### Task 11: Recette e2e de la frise

**Files:**
- Create: `ui/e2e/lists.cjs`
- Modify: `ui/package.json` (script `e2e` : ajouter `node e2e/lists.cjs` ou la forme des autres scripts, en fin de chaîne)

**Interfaces:**
- Consumes: widget de base `frise` (Task 6), `ItemLayer` (Task 9), `ListTable` (Task 8).

- [ ] **Step 1: Écrire le test** (squelette de `ui/e2e/fields.cjs`)

```js
// Frise : ajouter une étape sur la slide, la saisir, la déplacer, la supprimer puis annuler.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const post = (p, endpoint, req) => p.evaluate(([e, r]) => fetch(e, { method: 'POST',
  headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(r) }).then((x) => x.json()), [endpoint, req]);
const ids = async (p) => {
  const v = await post(p, '/api', { tab: 't-1', req: { cmd: 'slide', id: 's-constat' } });
  return v.items.filter((i) => i.owner === 'f').map((i) => i.item);
};
(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 2600, height: 1500 } });
  const errors = []; p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  await post(p, '/api', { tab: 't-1', req: { cmd: 'apply', label: 'frise', merge: null, ops: [{ op: 'add_element',
    slot: { owner: { slide: 's-constat' }, param: 'blocks' }, element: { id: 'f', kind: 'frise' } }] } });
  await p.reload(); await p.waitForSelector('.stage img');
  // aller à s-constat, sélectionner la frise
  // « + » après la dernière étape → 5 items, champ date du nouvel item en édition
  // taper « 2027 », Ctrl/Cmd+Entrée → set_field relu dans la vue (text === "2027")
  // glisser ⋮⋮ du nouvel item vers la première place → ids()[0] est le nouvel id
  // « × » sur le nouvel item → 4 items ; Cmd/Ctrl+Z → 5 items
  assert.deepEqual(errors, []);
  await b.close(); console.log('lists : ok');
})().catch((e) => { console.error(e); process.exit(1); });
```

Écrire chaque étape commentée avec les sélecteurs `[data-item-insert]`, `[data-item-move]`, `[data-item-remove]`, `.editor .cm-editor.cm-focused`, et une assertion `assert.equal((await ids(p)).length, N)` après chacune.

- [ ] **Step 2: Lancer** — `export OKU_SLIDES_PATHS__DATA=$(mktemp -d) OKU_SLIDES_PATHS__DECKS=$(mktemp -d) && just e2e` · Expected: `lists : ok` et les autres scripts verts.

- [ ] **Step 3: Recette manuelle de la spec** — dans l'éditeur de widget, ouvrir `frise` : le dock montre le tableau (ajout, retrait, Alt+↓, collage de deux lignes TSV, popover d'une colonne `rich` sur un widget de test qui en a une). Capture d'écran.

- [ ] **Step 4: Commit**

```bash
git add ui/e2e/lists.cjs ui/package.json
git diff --cached --name-only
git commit -m "test(e2e) : recette de la frise, items ajoutés, déplacés et supprimés sur la slide (t-494)"
```

- [ ] **Step 5: Clôture** — `oku cli task done --project slider --id t-494` après la dernière sous-tâche ; passe de consolidation (G10) : `rg "fn .*list" crates/oku-slides-core/src` pour repérer tout doublon de conversion de colonne.
