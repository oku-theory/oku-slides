# Paramètres à la sélection — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** le widget ciblé (élément sélectionné, sinon la slide) montre ses paramètres dans un panneau à droite, une liste en `ListTable` ; les « + » d'ajout d'item disparaissent de la scène et de l'aperçu.

**Architecture:** la session construit une surface A2UI `host-target` à partir du schéma du widget et des valeurs **du deck** (pas de valeurs d'exemple ni neutres), avec la même correspondance type → champ que l'atelier (code extrait de `widget_params.rs`). L'interface la lit dans un hook partagé avec les réglages de l'hôte et route ses actions vers deux requêtes typées, `target_param` (valeur → `set_param`) et `target_list` (`ListOp` → `edit_list`), appliquées par `apply` en un pas d'annulation. Le retrait des « + » supprime le code d'insertion visuelle et ce qui n'existait que pour lui.

**Tech Stack:** Rust (oku-slides-session, oku-slides-core, oku-slides-i18n ; Snafu, speculoos, rstest), React 19 + TypeScript (Bun test, happy-dom), Playwright e2e.

**Spec:** `docs/superpowers/specs/2026-10-05-parametres-a-la-selection-design.md`

## Global Constraints

- Tâche parente t-548 ; sous-tâches Task 1→t-549, 2→t-550, 3→t-551, 4→t-552, 5→t-553 ; chaque commit référence sa sous-tâche `(t-N)`.
- TDD : test rouge d'abord, vu échouer, puis le code. Noms de tests `<unité>_<scénario>_<résultat>` (Rust) / `<unité> <scénario> <résultat>` (TS).
- Aucune chaîne visible en dur : Fluent, **20 langues** (`crates/oku-slides-i18n/src/locales/*/`), anglais et français exacts, les 18 autres traduites. `ui/src/i18n-keys.test.ts` exige que chaque clé `en` soit utilisée et que `fr` ait exactement les clés `en` ; `crates/oku-slides-i18n/src/lib_tests.rs` a des listes de clés fixes.
- Pas d'`unwrap`/`expect`/indexation `[i]` hors tests ; `.get(i).context(HostKeySnafu { key })`.
- Erreurs réelles de la session : `NoSlide` (via `self.slide(id)?`), `HostKey`, `InHistory`, `ReadOnly`, `Op` (refus du cœur, déjà localisés). **Pas de nouvelle variante d'erreur.** Il n'existe ni `read_only()` ni `in_history()` : utiliser `self.ensure_editable()` (`history.rs:125`).
- Fichiers ≤ 500 lignes visés (1000 max). `lib.rs` (≈ 910) et `App.tsx` (≈ 940) ne reçoivent que du câblage ; la logique va dans les nouveaux modules.
- `crates/oku-slides-session/ARCHITECTURE.md` : une ligne par nouveau module (`param_fields.rs`, `target_params.rs`), sans l'allonger davantage.
- Tests de session : `cargo test -p oku-slides-session` ; UI : `cd ui && bun run typecheck && bun run test`.
- Toute exécution de l'app ou de l'e2e : `OKU_SLIDES_PATHS__DATA` et `OKU_SLIDES_PATHS__DECKS` vers un répertoire temporaire.
- Travail dans le worktree du plan (`git config --worktree oku.review deferred`) ; les commits de tâche ne passent pas par la revue du hook, la revue SDD par tâche la remplace.

---

### Task 1: Champs de paramètre partagés (refactor)

**Files:**
- Create: `crates/oku-slides-session/src/param_fields.rs`, `crates/oku-slides-session/src/param_fields_tests.rs`
- Modify: `crates/oku-slides-session/src/widget_params.rs` (`//!` mis à jour ; garde `widget_settings`, `checked_value`, `same_as`), `crates/oku-slides-session/src/widget_preview.rs` (`preview_params` utilise `schema_default`), `crates/oku-slides-session/src/widget_examples.rs:27` (imports), `crates/oku-slides-session/src/lib.rs` (`mod param_fields;`), `crates/oku-slides-session/ARCHITECTURE.md`

**Interfaces:**
- Produces (module `crate::param_fields`, doc `//!` : « Champs de réglage d'un paramètre de widget, partagés par l'atelier (valeurs d'exemple) et le panneau de la cible (valeurs du deck). ») :
  - `pub(crate) fn field(p: &ParamSchema, shown: Option<&ParamValue>, l: &Localizer) -> Option<ParamField>` — déplacée telle quelle (une liste `shown == None` montre `default_items` : c'est le comportement de l'atelier ; la cible passera une liste vide explicite).
  - `pub(crate) fn label(p: &ParamSchema) -> String`
  - `pub(crate) fn settable(ty: &ParamType) -> bool`
  - `pub(crate) fn coerce(p: &ParamSchema, value: Value) -> Option<Value>` — la conversion pure, extraite de `checked_value` : `null` ou texte blanc → `None` ; `flag` `"true"`/`"false"` → booléen ; `number` en texte → nombre (sinon le texte, refusé plus loin) ; le reste inchangé.
  - `pub(crate) fn schema_default(p: &ParamSchema) -> Option<ParamValue>` — extraite de la closure `default` de `preview_params` (`widget_preview.rs:168-173`) : `None` pour une liste, sinon le `default` non nul lu en `ParamValue`.
  - `column`, `slider_step` privés au module.
- `widget_params::checked_value` reste (contrôles de l'atelier, erreurs `Trial*`) et commence par `let Some(value) = coerce(p, value) else { return Ok(None) };` après son `ensure!(settable…)`.

- [ ] **Step 1: tests rouges** (`param_fields_tests.rs`, branché par `#[cfg(test)] #[path = "param_fields_tests.rs"] mod param_fields_tests;`) :

```rust
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use oku_slides_core::{ParamSchema, ParamType, ParamValue};

use super::{coerce, schema_default};

fn schema(ty: ParamType, default: Option<Value>) -> ParamSchema {
    ParamSchema { name: "p".into(), label: None, ty, default, bind: None }
}

#[rstest]
#[case(ParamType::Flag, json!("true"), Some(json!(true)))]
#[case(ParamType::Flag, json!("false"), Some(json!(false)))]
#[case(ParamType::Number { min: None, max: None }, json!(" 4.5 "), Some(json!(4.5)))]
#[case(ParamType::Number { min: None, max: None }, json!("abc"), Some(json!("abc")))]
#[case(ParamType::Plain, json!("   "), None)]
#[case(ParamType::Plain, Value::Null, None)]
#[case(ParamType::Plain, json!("Lyon"), Some(json!("Lyon")))]
fn coerce_converts_select_and_text_input(#[case] ty: ParamType, #[case] input: Value, #[case] expected: Option<Value>) {
    assert_that!(coerce(&schema(ty, None), input)).is_equal_to(expected);
}

#[test]
fn schema_default_plain_returns_the_default_text() {
    assert_that!(schema_default(&schema(ParamType::Plain, Some(json!("Titre")))))
        .is_equal_to(Some(ParamValue::Text("Titre".into())));
}

#[test]
fn schema_default_null_or_absent_returns_none() {
    assert_that!(schema_default(&schema(ParamType::Plain, Some(Value::Null)))).is_none();
    assert_that!(schema_default(&schema(ParamType::Plain, None))).is_none();
}
```

(Adapter `ParamType::Number { .. }` et le littéral `ParamSchema` aux champs réels de `crates/oku-slides-core/src/widget.rs:47,110` ; `ParamValue::Text` selon `params.rs:26`.)
- [ ] **Step 2:** `cargo test -p oku-slides-session param_fields` → échec de compilation (module absent).
- [ ] **Step 3:** Déplacer `field`, `column`, `label`, `slider_step`, `settable` dans `param_fields.rs` ; écrire `coerce` et `schema_default` en extrayant le code existant (pas de réécriture) ; `checked_value` et `preview_params` les appellent. `widget_examples.rs` importe `checked_value, same_as` de `widget_params` comme avant ; tout autre import de `settable` passe à `param_fields`.
- [ ] **Step 4:** `cargo test -p oku-slides-session` → tout vert (les tests de `widget_params_tests.rs` et `widget_preview*` gardent le comportement) ; `cargo clippy -p oku-slides-session --all-targets -- -D warnings`.
- [ ] **Step 5:** `ARCHITECTURE.md` : une ligne `param_fields.rs`. Commit `refactor(session) : champs de paramètre partagés entre l'atelier et la cible (t-549)`.

---

### Task 2: Surface et réglages de la cible (session)

**Files:**
- Create: `crates/oku-slides-session/src/target_params.rs`, `crates/oku-slides-session/src/target_params_tests.rs`
- Modify: `crates/oku-slides-session/src/panel.rs` (`HOST_TARGET`, `HostParams::extras` en `pub(crate)`, helper `slide_palette`), `request.rs` (3 variantes), `lib.rs` (dispatch, `mod target_params;`, exclusion `changes`), `crates/oku-slides-session/tests/session.rs` (aller-retour JSON), `crates/oku-slides-session/src/lib_tests.rs` (classification des requêtes), `ARCHITECTURE.md`
- Modify: `crates/oku-slides-i18n/src/locales/*/session.ftl` (20 langues : `session-undo-param`, `session-target-bound`)

**Interfaces:**
- Consumes (Task 1): `param_fields::{field, label, settable, coerce, schema_default}`.
- Produces:
  - `pub(crate) const HOST_TARGET: &str = "host-target";` (`panel.rs`)
  - `Session::slide_palette(&self) -> Vec<String>` (`panel.rs`), utilisé par `host_settings`, `agent/variants.rs:108` et la cible (remplace les trois copies de `self.appearance.slide.values().cloned().collect()`).
  - `Request::TargetSettings { slide: String, #[serde(default)] element: Option<String> }` → JSON `Surface | null` (`cmd: "target_settings"`).
  - `Request::TargetParam { slide: String, #[serde(default)] element: Option<String>, key: String, value: serde_json::Value }` → snapshot (`cmd: "target_param"`). Pour une liste, `value` est le tableau d'items entier (collage, remplacement).
  - `Request::TargetList { slide: String, #[serde(default)] element: Option<String>, key: String, change: oku_slides_core::ListOp }` → snapshot (`cmd: "target_list"`).
  - `Session::target_settings(&self, slide: &str, element: Option<&str>) -> Result<Option<Surface>>`
  - `Session::target_param(&mut self, slide: &str, element: Option<&str>, key: &str, value: Value) -> Result<()>`
  - `Session::target_list(&mut self, slide: &str, element: Option<&str>, key: &str, change: ListOp) -> Result<()>`

**Règles (spec) :**
- Cible : `self.slide(slide)?.slide` (vue courante, deck ou modèles), puis `slide.locate(id)` pour un élément. Widget `kind` du sélectionné ou de la slide ; schéma `self.registry.get(kind)`.
- Un graphique : `crate::charts::chart_param(&self.registry, kind).is_some()` → pas de surface.
- Pour chaque paramètre du schéma, dans l'ordre :
  - `bind: Some(var)` → note `session-target-bound` (`name` = `label(p)`, `var`), pas de champ, pas de clé.
  - `!settable` → omis.
  - liste → `field(p, Some(params.get(name).unwrap_or(&ParamValue::List(vec![]))), l)` : **une liste absente du deck est vide** (rendu : `typst/oku-slides/src/data.typ:96`), jamais les items par défaut.
  - autre → `field(p, params.get(name).cloned().or_else(|| schema_default(p)).as_ref(), l)`. Jamais de valeur neutre (`preview_params` n'est pas utilisé).
- Aucun **champ** → pas de surface (des notes seules n'en font pas une).
- `target_settings` rend `Ok(None)` si `self.ensure_editable().is_err()` (historique, aide), si la slide ou l'élément n'existent plus (sélection périmée : pas d'erreur), sans schéma, pour un graphique, sans champ. Titre : `session-widget-params` avec `name` = `schema.label` sinon `kind`.
- `target_param` / `target_list` : la clé doit être une clé de champ (un `bind`, un type non réglable, une clé inconnue → `HostKey`) ; slide inconnue → `NoSlide` ; élément inconnu → `HostKey`. 
  - `target_param` : `Op::SetParam { target, param: key, value: coerce(p, value).unwrap_or(Value::Null) }` ; pour une liste, `value` doit être un tableau, sinon `HostKey` (un `null` ou un objet n'efface jamais la liste). Fusion d'annulation : `merge = Some(format!("target:{slide}/{}/{key}", element.unwrap_or("")))` — un curseur qu'on glisse fait un seul pas.
  - `target_list` : `Op::EditList { target, param: key, change }`, sans fusion.
  - Libellé : `session-undo-param` avec `label` = libellé du champ. Validation par `apply` (refus `Op` du cœur, déjà localisé).
- Requêtes : `TargetParam` et `TargetList` → `changes_deck()` vrai, `edits()` vrai (refus `InHistory` / `ReadOnly` par `handle`), ajoutées à l'exclusion `!matches!(req, Snapshot | Apply | HostParam | …)` de `lib.rs:518` (`apply` persiste déjà). `TargetSettings` → `changes_deck()` faux.

- [ ] **Step 1: tests rouges** (`target_params_tests.rs`, branché par `#[path]`). Fixtures : `crate::lib_tests_support::{session_with_fixture, session_with_frise}` (élément `fr`, widget `frise`, sur `s-constat`), slides `s-ch1` (`chapter` : `title`, `subtitle` plain, valeurs dans le deck), `s-title` (`title` : tous ses paramètres `bind`), élément `e-image` (`asset`), `c3` (carte avec `flag`/`choice`). Un nombre : ajouter par `apply` un élément d'un widget de test à paramètre `number` (`set_widget` d'un pack comme `test_support::set_frise_widget`, ou un `stack` si son schéma a un `number` — vérifier `fixtures/widgets/base-registry.json`). Aide : `s.open_help("en", None)` (`help_tests.rs:21`) ; historique : `s.history_open()`.

```rust
use serde_json::{Value, json};
use speculoos::prelude::*;

use oku_slides_core::ListOp;

use crate::SessionErrorKind;
use crate::lib_tests_support::{session_with_fixture, session_with_frise};

/// Le JSON de la surface (messages A2UI), pour y chercher composants et valeurs.
fn surface_json(s: &crate::Session, slide: &str, element: Option<&str>) -> Option<Value> {
    s.target_settings(slide, element).unwrap().map(|x| serde_json::to_value(x).unwrap())
}
fn has(v: &Value, needle: &str) -> bool { v.to_string().contains(needle) }
fn kind(e: crate::Error) -> SessionErrorKind { e.kind() }

#[test]
fn target_settings_selected_frise_shows_its_list_table_with_deck_rows() {
    let s = session_with_frise();
    let v = surface_json(&s, "s-constat", Some("fr")).unwrap();
    assert_that!(has(&v, "\"host-target\"")).is_true();
    assert_that!(has(&v, "ListTable")).is_true();
    // les ids des items du deck (lire ceux posés par add_frise), pas d0…
}

#[test]
fn target_settings_absent_list_shows_an_empty_table() { /* retirer le param de la frise par apply set_param null, puis rows == [] */ }

#[test]
fn target_settings_nothing_selected_uses_the_slide_widget_fields() {
    let s = session_with_fixture();
    let v = surface_json(&s, "s-ch1", None).unwrap();
    // les textes title/subtitle du deck pour s-ch1 apparaissent comme valeurs
}

#[test]
fn target_settings_value_is_the_deck_value_else_the_schema_default_never_a_placeholder() {
    // un plain absent sans défaut : valeur nulle ; jamais le texte de « widget-preview-plain »
}

#[test]
fn target_settings_only_bound_params_returns_none() {
    let s = session_with_fixture();
    assert_that!(s.target_settings("s-title", None).unwrap()).is_none();
}

#[test]
fn target_settings_bound_param_next_to_fields_becomes_a_note() { /* widget de test : un plain lié + un plain libre → une note, un champ */ }

#[test]
fn target_settings_asset_param_is_omitted() { /* e-image : pas de surface, ou pas de champ asset */ }

#[test]
fn target_settings_removed_element_returns_none() {
    let s = session_with_fixture();
    assert_that!(s.target_settings("s-constat", Some("absent")).unwrap()).is_none();
}

#[test]
fn target_settings_in_history_returns_none() {}
#[test]
fn target_settings_in_help_returns_none() {}
#[test]
fn target_settings_template_slide_in_templates_mode_shows_fields() {}

#[test]
fn target_param_plain_sets_param_in_one_named_undo_step() {
    let mut s = session_with_fixture();
    s.target_param("s-ch1", None, "title", json!("Nouveau")).unwrap();
    assert_that!(s.snapshot().undo_label.as_deref()).is_equal_to(Some("Réglage : …"));  // libellé exact du champ
    s.undo().unwrap();
    // title revenu à sa valeur
}

#[test]
fn target_param_two_edits_of_the_same_field_merge_into_one_undo_step() {}
#[test]
fn target_param_flag_text_becomes_bool() {}
#[test]
fn target_param_number_text_becomes_number() {}
#[test]
fn target_param_blank_text_removes_the_param() {}
#[test]
fn target_param_invalid_value_is_refused_by_apply_and_deck_unchanged() { /* kind() == SessionErrorKind::Op */ }
#[test]
fn target_param_list_items_replace_the_list() {}
#[test]
fn target_param_list_with_non_array_is_host_key_and_list_kept() {
    // json!(null) et json!({"op":"replace"}) → HostKey ; les items restent
}
#[test]
fn target_param_bound_key_is_host_key() {}
#[test]
fn target_param_unknown_key_is_host_key() {}
#[test]
fn target_param_unknown_slide_is_no_slide() {}
#[test]
fn target_list_insert_appends_an_item() {}
#[test]
fn target_list_move_remove_set_field_apply() {}
#[test]
fn target_list_on_an_absent_list_inserts_into_an_empty_list() {}
#[test]
fn target_list_beyond_max_is_refused() {}
```

Chaque `{}` ci-dessus est écrit en entier par l'implémenteur selon son commentaire, avec `assert_that!` et l'erreur attendue par `SessionErrorKind` (lire `error.rs` pour la forme exacte de `kind()`). Le libellé `undo_label` attendu se lit du snapshot (`lib.rs`, champ `undo_label`).

Dans `crates/oku-slides-session/tests/session.rs`, à côté des tests `host_settings` / `host_param` (≈ l. 857-950), par `handle` et JSON :
  - `{"cmd":"target_settings","slide":"s-ch1"}` (sans `element`) rend une surface ;
  - `{"cmd":"target_param","slide":"s-ch1","key":"title","value":"X"}` change le titre ;
  - `{"cmd":"target_list","slide":"s-constat","element":"fr","key":"etapes","change":{"op":"insert","id":"n1","values":{"date":"2030"}}}` ajoute l'item ;
  - en historique : `target_param` et `target_list` → `InHistory` ; en aide → `ReadOnly`.

Dans `lib_tests.rs` (≈ 136-186), ajouter les trois commandes aux tests de classification `changes_deck` / `edits`.
- [ ] **Step 2:** `cargo test -p oku-slides-session target` → échec de compilation.
- [ ] **Step 3: implémentation** (`target_params.rs`, `//!` en tête) :

```rust
//! Paramètres de la cible : l'élément sélectionné, sinon le widget de la slide.
//! Lus dans le deck (jamais de valeur d'exemple ni neutre) et réglés par
//! `set_param` / `edit_list`, un pas d'annulation par réglage.

use oku_slides_core::{ListOp, Op, ParamSchema, ParamType, ParamValue, Params, Target, WidgetSchema};
use serde_json::Value;
use snafu::{OptionExt, ResultExt};

use crate::agent::forms::params_with;
use crate::agent::{ShowParams, Surface};
use crate::error::HostKeySnafu;
use crate::panel::{HOST_TARGET, HostParams};
use crate::param_fields::{coerce, field, label, schema_default, settable};
use crate::{Result, Session};

/// La cible résolue : son schéma, ses paramètres et les réglages qu'on en montre.
struct Resolved<'a> {
    schema: &'a WidgetSchema,
    params: HostParams,
}

impl Session {
    /// `None` : pas de cible montrable (disparue, graphique, sans champ).
    fn resolve_target(&self, slide: &str, element: Option<&str>) -> Result<Option<Resolved<'_>>> {
        let view = self.slide(slide)?;
        let (kind, params): (&str, &Params) = match element {
            Some(id) => match view.slide.locate(id) {
                Some(l) => (l.element.kind.as_str(), &l.element.params),
                None => return Ok(None),
            },
            None => (view.slide.kind.as_str(), &view.slide.params),
        };
        if crate::charts::chart_param(&self.registry, kind).is_some() {
            return Ok(None);
        }
        let Some(schema) = self.registry.get(kind) else { return Ok(None) };
        let l = &self.localizer;
        let (mut fields, mut keys, mut notes) = (Vec::new(), Vec::new(), Vec::new());
        for p in &schema.params {
            if let Some(var) = &p.bind {
                notes.push(l.tr_with("session-target-bound", &[("name", label(p).into()), ("var", var.as_str().into())]));
                continue;
            }
            if !settable(&p.ty) {
                continue;
            }
            let empty = ParamValue::List(Vec::new());
            let shown = match &p.ty {
                ParamType::List { .. } => Some(params.get(&p.name).cloned().unwrap_or(empty)),
                _ => params.get(&p.name).cloned().or_else(|| schema_default(p)),
            };
            if let Some(f) = field(p, shown.as_ref(), l) {
                keys.push(p.name.clone());
                fields.push(f);
            }
        }
        if fields.is_empty() {
            return Ok(None);
        }
        let name = schema.label.clone().unwrap_or_else(|| kind.to_owned());
        let title = l.tr_with("session-widget-params", &[("name", name.into())]);
        let spec = ShowParams { title, slide: slide.to_owned(), fields };
        Ok(Some(Resolved { schema, params: HostParams { spec, keys, notes } }))
    }
    // target_settings, target_param, target_list, et un helper privé
    // `param_of(&self, slide, element, key) -> Result<(ParamSchema, String /* libellé */)>`
    // qui fait resolve_target(…)?.context(HostKeySnafu { key })?, trouve la clé dans `keys`
    // (sinon HostKey), et rend le `ParamSchema` cloné et le libellé du champ (`.get(i)`).
}
```

(Ajuster les chemins d'import aux modules réels ; `params.get` selon l'API de `Params`, `params.rs:95` ; `schema.label` selon `WidgetSchema`. Si l'emprunt de `self.registry` par `Resolved` gêne `apply(&mut self)`, cloner le `ParamSchema` et le libellé avant d'appeler `apply`.)
- [ ] **Step 4:** i18n, 20 fichiers `session.ftl` :
  - en : `session-undo-param = Setting: { $label }` ; `session-target-bound = { $name } follows the deck variable “{ $var }”`
  - fr : `session-undo-param = Réglage : { $label }` ; `session-target-bound = { $name } suit la variable du deck « { $var } »`
  - les 18 autres traduites (même ponctuation que leurs voisines).
- [ ] **Step 5:** `cargo test -p oku-slides-session -p oku-slides-i18n` → vert ; `cargo clippy -p oku-slides-session --all-targets -- -D warnings` ; `ARCHITECTURE.md` : une ligne `target_params.rs` (surface `host-target`).
- [ ] **Step 6:** Commit `feat(session) : réglages de la cible — surface host-target, target_param et target_list annulables (t-550)`.

---

### Task 3: Panneau de la cible dans l'interface

**Files:**
- Create: `ui/src/useTargetSettings.ts`, `ui/src/useTargetSettings.render.test.tsx`, `ui/src/TargetPanel.tsx`, `ui/src/TargetPanel.render.test.tsx`
- Modify: `ui/src/useHostSettings.ts` (extraire le cycle partagé `useSessionSurface`), `ui/src/useHostSettings.render.test.tsx` (inchangé, doit rester vert), `ui/src/api.ts` (union des requêtes ≈ l. 487 et méthodes ≈ l. 641), `ui/src/App.tsx` (câblage seulement), `ui/src/HostSettings.tsx` (doc), `ui/src/styles.css` (`.notes > .host-settings`)

**Interfaces:**
- Consumes (Task 2): `cmd: "target_settings" { slide, element? }` → `Surface | null` ; `cmd: "target_param" { slide, element?, key, value }` → `Snapshot` ; `cmd: "target_list" { slide, element?, key, change }` → `Snapshot`.
- Produces:
  - `api.targetSettings(slide: string, element: string | null): Promise<Surface | null>`
  - `api.targetParam(slide: string, element: string | null, key: string, value: unknown): Promise<Snapshot>`
  - `api.targetList(slide: string, element: string | null, key: string, change: ListOp): Promise<Snapshot>`
  - `useSessionSurface(opts)` dans `useHostSettings.ts` : le cycle « lire / garder si même contenu / relire après refus / refuser une surface remplacée » aujourd'hui inline dans `useHostSettings` (l. 45-104). Signature :

```ts
export function useSessionSurface<S>(opts: {
  /** Ids des surfaces réclamées par ce hook. */
  ids: readonly string[];
  active: boolean;
  /** La portée lue : la surface reçue est rattachée à elle (envoi des actions). */
  scope: S;
  /** Tout ce qui peut changer la surface, en une chaîne. */
  key: string;
  fetch: (scope: S) => Promise<Surface | null>;
  send: (scope: S, field: string, context: Record<string, unknown>) => Promise<Snapshot>;
  run: (p: Promise<Snapshot>) => Promise<Snapshot | null>;
  say: (m: string, error?: boolean) => void;
}): HostSettings;
```

    `useHostSettings` devient un appel de `useSessionSurface` (scope `{slide, widget}`, comportement identique : ses tests restent verts sans modification). `onAction` réclame **toute** surface dont l'id est dans `ids`, même inactive ou remplacée (rend `true` sans rien envoyer), comme aujourd'hui.
  - `export const HOST_TARGET = "host-target";`
  - `useTargetSettings(api, snap, current, element, widgetActive, run, say): HostSettings` — `element` est l'id de l'élément **résolu dans la vue** (`selectedElement?.id ?? null`, App.tsx:571), jamais l'état brut `selected`. Actif si `current && snap && !snap.read_only && !snap.history && !widgetActive`. Clé : `[current, element, slide?.hash, snap?.editing?.base, snap?.templates?.version, snap?.undo_label, snap?.redo_label].join("|")`. Commentaire dans le hook : deux réglages de suite du même champ gardent le même libellé ; si le rendu (`hash`) ne change pas, la surface n'est pas relue, ce qui est voulu — le champ garde sa valeur locale, et un refus force la relecture (`sync`). `send` : `context.change` (ListChange) → `op === "replace"` ? `api.targetParam(scope.slide, scope.element, field, change.items)` : `api.targetList(…, change)` ; sinon `api.targetParam(…, context.value)`. Les actions partent toujours vers la portée **de la surface affichée** (`scope` gardée avec elle), jamais vers la sélection courante.
  - `TargetPanel` (`TargetPanel.tsx`) : la carte en tête de la colonne des notes.

```tsx
type Props = {
  view: SlideView | null;
  readOnly: boolean;
  /** L'élément sélectionné résolu dans la vue. */
  selected: Element | undefined;
  target: HostSettings;
  palette: string[];
  onApply: (label: string, ops: unknown[]) => void;
  onKind: (kind: string) => void;
  onSurfaceAction: (s: Surface, a: A2uiClientAction) => void;
  onError: (m: string) => void;
};
// graphique sélectionné → <ChartPanel …/> ; sinon target.surface → <HostSettings surface={target.surface} …/> ; sinon rien ;
// view absent ou readOnly → rien.
```

- [ ] **Step 1: tests rouges.**
  - `useTargetSettings.render.test.tsx` (sur le modèle de `useHostSettings.render.test.tsx`) :
    - `useTargetSettings element selected reads its surface` ; relue quand `element` change ;
    - `useTargetSettings nothing selected reads the slide widget` (`targetSettings(slide, null)`) ;
    - `useTargetSettings read only or history sends no request` ; `useTargetSettings widget sub-tab active sends no request` ;
    - `useTargetSettings value action sends targetParam to the shown target` ; `useTargetSettings list change sends targetList` ; `useTargetSettings replace sends targetParam with the items` ;
    - `useTargetSettings selection changed before the action keeps the shown target` (la surface de `fr` affichée, `element` passe à `c1`, l'action part vers `fr` ou rien, jamais `c1`) ;
    - `useTargetSettings refusal rereads an identical surface` ;
    - `useTargetSettings claims a stale host-target action without sending` ; `useTargetSettings ignores host-template and host-widget` ; et dans `useHostSettings.render.test.tsx` : `useHostSettings ignores host-target`.
  - `TargetPanel.render.test.tsx` : `TargetPanel chart selected shows ChartPanel only` ; `TargetPanel widget selected shows its settings` ; `TargetPanel read only shows nothing` ; `TargetPanel no surface shows nothing`.
- [ ] **Step 2:** `cd ui && bun test src/useTargetSettings.render.test.tsx src/TargetPanel.render.test.tsx` → échec (modules absents).
- [ ] **Step 3:** Extraire `useSessionSurface` (refactor ; `bun test src/useHostSettings.render.test.tsx` vert), puis écrire `useTargetSettings`, `api.*`, `TargetPanel`.
- [ ] **Step 4: App.tsx** (câblage seulement) : `const target = useTargetSettings(api, snap, current, selectedElement?.id ?? null, widgetActive, run, say);` (déplacer le calcul de `selectedElement` avant si besoin) ; `onSurfaceAction` : `if (host.onAction(s, a) || target.onAction(s, a)) return;` ; dans `<aside className="notes">`, remplacer le bloc `ChartPanel` (≈ l. 867-877) par `<TargetPanel …/>`.
- [ ] **Step 5: CSS** (`styles.css`, près de `.notes`) : `.notes > .host-settings { flex: none; width: auto; max-height: 60%; overflow: auto; }` (comme `.chart-panel`, `chart/ChartDataGrid.css:20`) ; le popover du tableau (`.lt-popover`) reste dans la carte. Doc de `HostSettings.tsx` : surface de l'hôte en mode modèles **ou** paramètres de la cible.
- [ ] **Step 6:** `bun run typecheck && bun run test` → vert.
- [ ] **Step 7: vérification manuelle** (serveur isolé) : frise sélectionnée → « Paramètres de frise », tableau des étapes ; ajouter une étape → la slide la montre ; ⌘Z l'annule ; rien de sélectionné sur `s-ch1` → titre et sous-titre ; graphique → `ChartPanel` seul ; atelier de widget ouvert → pas de panneau à droite ; thème sombre ; colonne étroite : l'agent reste visible.
- [ ] **Step 8:** Commit `feat(ui) : panneau des paramètres du widget ciblé, à droite (t-551)`.

---

### Task 4: Retrait des « + » d'ajout d'item (après Task 3)

**Files:**
- Modify (UI) : `ui/src/ItemLayer.tsx` (retirer `INSERT_SIZE`, `insert`, `plus`, `empty`, les « + », imports devenus inutiles `newId`, `fieldKey`, `FieldKey`, `insertPoints`, `within` ; en-tête l. 1-5), `ui/src/stageItems.ts` (retirer `within`, `ItemHandles.onInserted`, `HandlesSource.onEditWhenShown` ; garder `insertPoints`, `EDGE`, `dropTarget` et mettre leurs docs à jour : ils servent au repère du glisser ; `OnListOp` : si plus aucun appelant ne lit son `Promise<boolean>`, le rendre `void` et retirer `.then((done) => …)` de `WidgetTab.tsx`), `ui/src/Stage.tsx` (props `onEditWhenShown`), `ui/src/App.tsx` (`editKeyWhenShown`), `ui/src/useFieldEditing.ts` (retirer `editKeyWhenShown`, l'état `pendingKey` et son effet, doc l. 13-14 ; garder `editWhenShown`), `ui/src/WidgetTab.tsx`, `ui/src/widgetPreview.ts` (doc de `croppedView` ; garder `targets`, lu par `stageItems.ts:150`), `ui/src/styles.css` (`.item-insert` et ses règles l. 282-283 ; garder `.item-remove, .item-move`), `ui/src/api.ts:354` (`first_text`)
- Modify (Rust) : `crates/oku-slides-session/src/items.rs` (retirer `first_text` et l'import de `is_text` ; **ne pas** toucher `oku_slides_core::is_text`, lu par `core/src/fields.rs`), `crates/oku-slides-session/src/items_tests.rs` (l. 13, 37)
- Modify (i18n) : `crates/oku-slides-i18n/src/locales/*/ui.ftl` (20 langues : retirer `ui-undo-list-add` ; `ui-list-add` reste, lu par `ListTable`), `crates/oku-slides-i18n/src/lib_tests.rs:259-277` (`LIST_KEYS` : 17 entrées sans `ui-undo-list-add`)
- Modify (aide) : `docs/aide/en.json` et `docs/aide/fr.json` (entrée ≈ l. 1938 « + entre les items » → l'ajout se fait dans le tableau du panneau des paramètres, à droite)
- Modify (e2e) : `ui/e2e/lists.cjs` (l. 71-155), `ui/e2e/lists-dock.cjs` (en-tête l. 3 ; supprimer le bloc l. 85-91 qui clique le « + » de l'aperçu : l'ajout par `[data-list-add]` est déjà testé l. 65-67)
- Test : `ui/src/ItemLayer.render.test.tsx`, `ui/src/Stage.render.test.tsx`, `ui/src/WidgetTab.render.test.tsx`, `ui/src/stageItems.test.ts`, `ui/src/useFieldEditing.render.test.tsx`, `ui/src/widgetPreview.test.ts`

**Interfaces:**
- Consumes (Task 3) : le panneau `host-target` et son `[data-list-add]` (e2e `lists.cjs`).
- Produces : plus aucun `[data-item-insert]` dans le DOM ; `ItemView`/`ListView` sans `first_text` (`refactor(ui)!`).

- [ ] **Step 1: tests rouges** : `ItemLayer a list shows remove and move handles but no plus` (une liste de 3 items et une liste vide : `[data-item-insert]` absent, `.item-remove` et `.item-move` présents) ; même assertion dans `Stage.render.test.tsx` (liste de slide) et `WidgetTab.render.test.tsx` (aperçu). Les voir échouer.
- [ ] **Step 2:** Retirer le code (fichiers ci-dessus).
- [ ] **Step 3: tests existants, par nom** (`rg -n "data-item-insert|insert|plus|onInserted|onEditWhenShown|editKeyWhenShown|first_text|within" ui/src/*.test.ts*`) :
  - supprimer ceux qui testent l'ajout lui-même (clic sur un « + », index d'insertion, ouverture du champ de l'item inséré, `within`, `editKeyWhenShown`) ;
  - **réécrire sur `[data-item-remove]`** (position, cible de l'op de retrait) ceux qui testaient, à travers les « + », quel propriétaire montre ses poignées (`Stage.render.test.tsx` ≈ 433-522, `WidgetTab.render.test.tsx` ≈ 551-635, `stageItems.test.ts` `itemHandles` ≈ 72-88) ;
  - garder intacts les tests de « × », ⋮⋮, glisser et repère ;
  - retirer `first_text` des fixtures (`Stage`, `stageItems`, `ItemLayer`, `WidgetTab`, `widgetPreview` tests).
- [ ] **Step 4:** Rust : `first_text` ; `cargo test -p oku-slides-session`. i18n : `ui-undo-list-add` ×20 et `LIST_KEYS` ; `cargo test -p oku-slides-i18n` ; `bun test src/i18n-keys.test.ts`. Aide en/fr.
- [ ] **Step 5: e2e `lists.cjs`** : l'ajout passe par le panneau — sélectionner l'élément frise de `s-constat`, cliquer `[data-surface-id="host-target"] [data-list-add]`, attendre 5 items (`waitLen`), remplir la date du nouvel item dans le tableau (`2027`), cliquer la scène hors du tableau ; la suite (déplacement au clavier « depuis un champ édité », ×, glisser) ouvre le champ du nouvel item par double-clic sur la scène au lieu de compter sur l'ouverture automatique. `lists-dock.cjs` : en-tête et bloc supprimé.
- [ ] **Step 6:** `rg -n "item-insert|data-item-insert|onInserted|onEditWhenShown|editKeyWhenShown|ui-undo-list-add|first_text" ui/src ui/e2e crates docs/aide` → vide ; `bun run typecheck && bun run test` ; e2e `lists.cjs` et `lists-dock.cjs` contre un serveur isolé → verts.
- [ ] **Step 7:** Commit `refactor(ui)! : plus de « + » d'ajout d'item sur la slide ni l'aperçu — l'ajout passe par le tableau (t-552)` avec le pied `BREAKING CHANGE: ItemView et ListView n'ont plus first_text.`

---

### Task 5: Bout en bout du panneau

**Files:**
- Create: `ui/e2e/target-panel.cjs` (sur le modèle de `lists.cjs`, mode deck, `fixtures/roadmap.json`)
- Modify: `ui/package.json` (script `e2e` : `bun e2e/target-panel.cjs` juste après `bun e2e/lists.cjs`, avant `widgets.cjs`)

- [ ] **Step 1:** Écrire `target-panel.cjs` :
  - sélectionner l'élément frise de `s-constat` sur la scène → `[data-surface-id="host-target"]` visible, titre « Paramètres de … », autant de lignes que d'items (lus par l'API de vue, comme `lists.cjs`) ;
  - modifier la date de la première ligne → l'item du deck porte la nouvelle date (API) ;
  - ajouter une ligne → ids + 1 ; cliquer la scène (le focus quitte le tableau), ⌘Z → exactement les ids d'origine ;
  - cliquer le fond de la slide `s-ch1` → la carte montre « Paramètres de chapter » (libellé réel du widget) et le champ titre avec le titre du deck ;
  - sélectionner un graphique (s'il y en a un dans `roadmap.json`, sinon sauter ce point) → pas de carte `host-target`.
- [ ] **Step 2:** Lancer contre un serveur isolé (`OKU_SLIDES_PATHS__DATA/DECKS` temporaires) → vert ; puis `just e2e` complet, isolé → vert.
- [ ] **Step 3:** Commit `test(e2e) : panneau des paramètres de la cible (t-553)`.

---

## Revue finale

`common:review` sur `main..HEAD` du worktree, corrections, puis squash sur `main` (voir la mémoire « keep-commit-review-hook » : attendre un `main` sans index ni fichiers suivis modifiés sur les mêmes fichiers ; `ui.ftl` et `App.tsx` sont aussi modifiés par d'autres sessions). Tâches t-549…t-553 puis t-548 closes.

## Écarts assumés par rapport à la première version du plan

- `target_settings` rend une `Surface` (la spec disait `HostParams`) ; `HostParams` reste interne.
- Deux requêtes d'écriture typées (`target_param`, `target_list`) au lieu d'une valeur « reniflée ».
- Une liste absente du deck s'affiche vide ; les valeurs absentes prennent le `default` du schéma, jamais une valeur neutre.
- Des notes seules (paramètres tous liés) ne font pas de panneau.
- Pas de panneau quand l'atelier de widget est ouvert.
- L'insertion à une position précise disparaît avec les « + » : on ajoute à la fin du tableau puis on glisse la ligne.
