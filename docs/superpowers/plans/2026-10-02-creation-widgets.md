# Création de widgets par l'agent et éditeur Typst — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** l'utilisateur fait créer ou modifier des widgets par l'agent (« fais-moi un widget KPI »), ou les écrit lui-même dans un éditeur Typst ; un widget est un fichier `widgets/<nom>.typ` d'une version de pack (plus un `_commun` facultatif), il voyage avec le deck (t-328), se compile seul dans le processus de rendu borné (t-327) et, cassé, est écarté avec ses diagnostics sans emporter les autres.

**Architecture:** le cœur remplace `widgets.typ` par `PackFiles.widgets: BTreeMap<WidgetFile, String>` (contenu canonique multi-fichiers, zip des sources v5 `packs/<hex>/widgets/<nom>.typ`). Le rendu sert chaque fichier sous `/packs/<id>/widgets/<nom>.typ` et un module d'assemblage généré `/packs/<id>/widgets.typ` ; il sonde **un fichier à la fois** (`probe_file` → `FileProbe` : schéma ou diagnostics à ligne et colonne, cache `(nom, hash du fichier, hash de _commun importé)`) et assemble le registre des seuls valides (`assemble`). Le processus de rendu expose `RenderWorker::probe_pack` (sonde par fichier, jamais d'accusation ; un délai devient un diagnostic) et `trial_slot` (emplacement d'aperçu qui n'écarte aucun pack). La session migre bibliothèque, reprise et rendu, puis gagne `widget_status`, `set_widget`, `remove_widget`, `rename_widget`, `widget_source` (mode modèles : version de séance de t-328 ; mode deck : version du pack du deck, `SetTemplates`, une étape d'annulation par séance) et `preview_widget`. L'agent reçoit cinq outils MCP et une section « Créer un widget » ; l'UI un panneau Widgets (liste, CodeMirror Typst minimal, diagnostics, aperçu à 400 ms).

**Tech Stack:** Rust 2024, serde/serde_json (`preserve_order`), sha2/hex, Typst 0.15.1 (`typst-syntax::Lines::byte_to_line_column`), OpenDAL (`Storage`), snafu, speculoos, rstest, Fluent ; React 19 + TypeScript, CodeMirror 6 (`@codemirror/view`, `state`, `commands` déjà présents, aucune dépendance nouvelle), bun test.

**Spec:** `docs/superpowers/specs/2026-10-02-creation-widgets-design.md` (t-329)

## Global Constraints

- `SOURCES_VERSION = 5` ; `manifest.json` : `{ format: "oku-slides-sources", version: 5 }`. Aucune rétrocompatibilité : zips v4 refusés (`NotSources`), tests et fixtures de t-328 qui utilisent `widgets.typ` réécrits en `widgets/<nom>.typ`. Les packs intégrés n'ont pas de widgets.
- Zip des sources :
  ```
  manifest.json                 { format: "oku-slides-sources", version: 5 }
  deck.loro
  assets/<hex>
  fonts/<nom>
  packs/<hex>/pack.json
  packs/<hex>/widgets/<nom>.typ  (un par fichier de widget, `_commun.typ` compris)
  ```
  Un nom invalide sous `packs/<hex>/widgets/` (ou un sous-dossier) refuse le zip : `SourcesErrorKind::BadName`, comme un chemin `packs/` invalide.
- `WidgetFile` : `[a-z][a-z0-9-]*` (64 caractères au plus) ou exactement `_commun`. Accepte `kpi`, `_commun` ; refuse `Kpi`, `../x`, `_autre`. Le nom du fichier est le nom du widget ; `_commun` n'est pas un widget.
- **Contenu canonique** : `pack.json` canonique (t-328 : JSON compact à clés triées), octet `0`, puis pour chaque fichier dans l'ordre des noms (`_commun` avant `a…z`) : nom, `0`, source, `0`. Un fichier vide vaut absent. `PackVersion` = `sha256:<hex>` de ce contenu. Vecteurs figés (calculés par `printf … | shasum -a 256`) :
  - `{"a":[{"c":"x","d":2}],"b":1}\0` (sans widget) → `sha256:99b62953dd3d649f39732438515923d2db4f402b3e1ee7a15ab5b343a4974d3a` (inchangé depuis t-328) ;
  - `{"a":[{"c":"x","d":2}],"b":1}\0kpi\0w\0` → `sha256:e74910e70629b0f16334d49b9a4ce504ea98c4d68e87510241480412ac18e73d` ;
  - `{"a":[{"c":"x","d":2}],"b":1}\0_commun\0C\0kpi\0K\0` → `sha256:b6aaacfa8364494dd04f71654aa607b38c26ef327b5a8a36e06fc100c644608d` ;
  - `{"name":"Café \"Ünï\"\t\\ 漢","ratio":2.5,"z":[0.1,-3]}\0kpi\0w\0` → `sha256:e855c082bca6d0f2ef7a4e107750238ea482de41a53a8e92cc57f2787287e928`.
- Bibliothèque : `<data>/templates/<uuid>/versions/<hex>/widgets/<nom>.typ` (widgets écrits avant `pack.json`). Reprise : `packs/<hex>.json` et `packs/<hex>.<nom>.typ`.
- Limites (`[render]`) : `max_widget_kib = 256` par fichier, `max_widgets = 64` fichiers par version (`_commun` compris) ; les fichiers comptent dans `max_sources_mib` (lecture du zip). La session refuse au-delà (`WidgetTooLarge`, `TooManyWidgets`) ; la sonde d'un pack reçu n'exécute pas un fichier hors limites (diagnostic).
- Intégrité : uniquement `PackFiles::matches` / `PackVersion::verifies` (un seul contrôle). Pack par défaut : uniquement `adopt_default_pack`.
- Module d'assemblage `/packs/<id>/widgets.typ` (généré, `_commun` absent) :
  ```typ
  #import "widgets/kpi.typ": widget as w0
  #import "widgets/quote.typ": widget as w1
  #let registry = (w0,w1,)
  ```
  Chaque `widgets/<nom>.typ` exporte la liaison `widget`, résultat de `widget("<nom>", …)` ; il peut importer `"_commun.typ"` (relatif) et `/oku-slides/lib.typ`.
- Sonde par fichier : pack `sonde` (`PROBE_PACK`), indépendante de la version ; un fichier dont le nom déclaré diffère du nom de fichier est refusé avec un diagnostic ; cache indexé par `probe_key(nom, source, _commun si le fichier contient "_commun.typ")`, échecs bornés compris. Pendant l'édition, aucun pack n'est accusé ; l'écartement de t-327 reste celui du rendu du deck.
- Séance (t-328) : en mode modèles, première modification → nouvelle version (`parent` = courante au début de la séance), rendue courante ; suivantes → `replace_version`. En mode deck : version du pack du deck (`parent` = version embarquée), ajoutée à la bibliothèque, courante, embarquée, `SetTemplates` sous la clé `templates-adopt:<id>:<parent>` (une étape d'annulation) ; la séance finit à la première opération qui n'est pas une modification de widget.
- Erreurs session (Snafu, `backtrace` dans chaque variante, messages en minuscules) : `InvalidWidgetName { name }`, `WidgetTooLarge { name, max }`, `TooManyWidgets { max }`, `WidgetInUse { name, templates }`, `UnknownWidget { name }`, plus `WidgetExists { name }` (renommage). Cœur : `WidgetFileError` (Snafu, backtrace).
- Fluent, **20 langues** (`crates/oku-slides-i18n/src/locales/<l>/*.ftl`) : fr et en donnés verbatim ci-dessous, **traduire les 18 autres** (ar, bn, de, es, hi, it, ja, ko, nl, pl, pt, ru, th, tr, uk, vi, zh-Hans, zh-Hant) dans le registre des clés voisines. Clés : session.ftl `session-invalid-widget-name`, `session-widget-too-large`, `session-too-many-widgets`, `session-widget-in-use`, `session-unknown-widget`, `session-widget-exists`, `session-widgets-broken`, `widgets-edit-label`, `widget-preview-plain`, `widget-preview-rich` ; agent.ftl `agent-instructions-widgets` ; ui.ftl `ui-widgets-*` (Task 11).
- Pas de `.unwrap()`/`.expect()` hors `#[test]`/`#[cfg(test)]` (aides de test hors `#[test]` : `#[cfg(test)]` ou `unwrap_or_else(|e| panic!("{e}"))`) ; pas de `std::fs` en production (`Storage`) ; commentaires de doc en français, première phrase de moins de 15 mots ; `ARCHITECTURE.md` de chaque crate sous 50 lignes ; fichiers visés à 500 lignes, plafond 1000 (`crates/oku-slides-session/src/lib.rs` fait 971 lignes : la Task 5 en sort `Request`).
- Tests côte à côte `foo_tests.rs` (`#[cfg(test)] #[path = "foo_tests.rs"] mod foo_tests;`), speculoos, nommage `<unité>_<scénario>_<résultat>`, TDD (test en échec d'abord).
- Commits : `<type>(<scope>) : <description> (t-329)`, rupture → `feat(<scope>)! : …`, **sans** ligne `Co-Authored-By`. `git add` dans une commande séparée du `git commit` (le hook de revue bloque sinon toute la commande). Dans ce worktree SDD, une fois : `git config --worktree oku.review deferred`.
- Vérification d'un crate : `cargo nextest run -p <crate>` ; UI : `cd ui && bun test && bun run typecheck` ; en fin de plan : `just check`.

### Règle de verdeur (choisie pour ce plan)

`PackFiles` (Task 1) et `PackWidgets` (Task 3) changent de forme : leurs utilisateurs en aval ne compilent plus jusqu'à leur migration. Règle : **à chaque tâche, le crate en cours et ceux en amont sont verts** :

| Après la tâche | Verts |
|---|---|
| 1, 2 | core, chart, i18n, platform, pdf, render, render-worker |
| 3 | core, chart, i18n, platform, pdf, render |
| 4 | + render-worker |
| 5 | + session, mcp, agent, src-tauri |
| 6 | tout le workspace (`cargo build --workspace`) |
| 7 → 10 | tout le workspace |
| 11, 12 | tout, UI comprise ; `just check` à la Task 12 |

---

## Carte des fichiers

| Fichier | Rôle |
|---|---|
| `crates/oku-slides-core/src/pack.rs` | `WidgetFile`, `PackFiles.widgets: BTreeMap<WidgetFile, String>`, contenu canonique multi-fichiers |
| `crates/oku-slides-core/src/error.rs` | `WidgetFileError`, `WidgetError::widget()` |
| `crates/oku-slides-core/src/sources.rs` | zip v5, `pack_entry` → `PackPart`, `packs/<hex>/widgets/<nom>.typ` |
| `crates/oku-slides-pdf/src/lib.rs`, `ARCHITECTURE.md` | description du zip v5 |
| `crates/oku-slides-platform/src/config.rs`, `config.default.toml` | `max_widget_kib`, `max_widgets` |
| `crates/oku-slides-render/src/diagnostic.rs` | `SourceAt`, `Diagnostic.at`, `widget_file()`, `widget_error()` |
| `crates/oku-slides-render/src/registry.rs` | `PackWidgets` multi-fichiers, assemblage, `base_probe`, `Registries::probe_file` |
| `crates/oku-slides-render/src/widget_probe.rs` (nouveau) | `FileProbe`, `probe_file`, `probe_key`, `assemble`, `PROBE_PACK` |
| `crates/oku-slides-render/src/world.rs` | sert les fichiers du pack et l'assemblage ; diagnostics à ligne et colonne |
| `crates/oku-slides-render-worker/src/{protocol,serve,client,link}.rs` | `WirePack.files`, `ProbeFile`, `trial_slot` |
| `crates/oku-slides-render-worker/src/pack_probe.rs` (nouveau) | `PackProbe`, `RenderWorker::probe_pack` |
| `crates/oku-slides-session/src/request.rs` (nouveau) | `Request` sorti de `lib.rs` (+ requêtes des widgets) |
| `crates/oku-slides-session/src/{library,resume,deck_widgets,templates_mode,pack}.rs` | packs à plusieurs fichiers |
| `crates/oku-slides-session/src/widgets_edit.rs` (nouveau) | `WidgetStatus`, `WidgetDiagnostic`, `WidgetChange`, opérations |
| `crates/oku-slides-session/src/widgets_deck.rs` (nouveau) | séance du mode deck |
| `crates/oku-slides-session/src/widget_preview.rs` (nouveau) | `preview_widget`, `WidgetPreview`, `PreviewFormat` |
| `crates/oku-slides-session/tests/regressions/issue_0329_widget_casse_garde_les_autres.rs` | régression |
| `crates/oku-slides-cli/src/{pack_render,main}.rs` | sonde par fichier, `open --out` → `widgets/<nom>.typ` |
| `crates/oku-slides-mcp/src/widget_tools.rs` (nouveau), `tools.rs`, `handler.rs` | cinq outils |
| `crates/oku-slides-agent/src/turn.rs` | section « Créer un widget » |
| `fixtures/widgets/kpi/{kpi.typ,_commun.typ}` (nouveaux), `typst/oku-slides/README.md` | widget exemple |
| `ui/src/{api.ts,App.tsx,styles.css}` | requêtes, menu, panneau |
| `ui/src/{typstHighlight,widgetDiagnostics,debounce,widgetList}.ts`, `TypstEditor.tsx`, `WidgetWorkbench.tsx` (nouveaux) | panneau Widgets |

---

### Task 1 : fichiers de widgets dans le cœur, contenu canonique et zip v5

**Files:**
- Modify: `crates/oku-slides-core/src/pack.rs` (doc `:1-7`, `PackFiles` `:93-150`)
- Modify: `crates/oku-slides-core/src/pack_tests.rs` (réécrit)
- Modify: `crates/oku-slides-core/src/error.rs` (après `PackVersionError` `:442-448` ; `impl WidgetError` `:404-417`)
- Modify: `crates/oku-slides-core/src/lib.rs` (`:18` doc, `:39-42` erreurs, `:47` `pub use pack::…`)
- Modify: `crates/oku-slides-core/src/widget_tests.rs` (un test)
- Modify: `crates/oku-slides-core/src/sources.rs` (doc `:1-15`, `SOURCES_VERSION` `:40`, `:45-46`, `RawPack` `:122-127`, `pack_entry` `:129-136`, `write_zip` `:398-406`, `read_zip` `:455-468` et `:500-515`)
- Modify: `crates/oku-slides-core/src/sources_tests.rs` (`:27`, `:80-81`, `:422-558`), `crates/oku-slides-core/tests/zip_limits.rs:56`
- Modify: `crates/oku-slides-pdf/src/lib.rs:18-27`, `crates/oku-slides-pdf/ARCHITECTURE.md:22-30`

**Interfaces:**
- Produces :
  - `pub const COMMON_WIDGET_FILE: &str = "_commun";`
  - `pub struct WidgetFile(String)` — `Clone + Eq + Ord + Hash + Serialize + Deserialize + JsonSchema + Display` ; `WidgetFile::parse(&str) -> Option<WidgetFile>`, `WidgetFile::common() -> WidgetFile`, `is_common(&self) -> bool`, `as_str(&self) -> &str`, `file_name(&self) -> String` (`<nom>.typ`), `WidgetFile::from_file_name(&str) -> Option<WidgetFile>` ; sérialisé en chaîne, refusé sinon (`WidgetFileError`).
  - `PackFiles::new(Map<String, Value>, BTreeMap<WidgetFile, String>) -> PackFiles` (fichiers vides retirés), `PackFiles::from_bytes<'a>(&[u8], impl IntoIterator<Item = (WidgetFile, &'a [u8])>) -> Option<PackFiles>`, `widgets(&self) -> &BTreeMap<WidgetFile, String>`, `widget(&self, &WidgetFile) -> Option<&str>` ; `manifest`, `version`, `matches`, `pack_json`, `canonical` gardent leur signature.
  - `WidgetError::widget(&self) -> Option<&str>` : le widget refusé (`None` pour `Unreadable`).
  - `SOURCES_VERSION = 5` ; `from_zip` lit `packs/<hex>/widgets/<nom>.typ`.

- [ ] **Step 1 : tests en échec** — `crates/oku-slides-core/src/pack_tests.rs` réécrit :

```rust
use std::collections::BTreeMap;

use rstest::rstest;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

use super::*;

fn object(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(m) => m,
        other => panic!("objet attendu : {other}"),
    }
}

fn wf(name: &str) -> WidgetFile {
    WidgetFile::parse(name).unwrap_or_else(|| panic!("nom invalide : {name}"))
}

/// Des fichiers de widgets, par nom.
fn files(entries: &[(&str, &str)]) -> BTreeMap<WidgetFile, String> {
    entries.iter().map(|(n, s)| (wf(n), (*s).to_owned())).collect()
}

#[test]
fn pack_version_of_known_content_is_stable() {
    // Canonique : {"a":[{"c":"x","d":2}],"b":1} 0 kpi 0 w 0
    let pack = PackFiles::new(object(json!({ "b": 1, "a": [{ "d": 2, "c": "x" }] })), files(&[("kpi", "w")]));
    assert_that!(pack.version().as_str())
        .is_equal_to("sha256:e74910e70629b0f16334d49b9a4ce504ea98c4d68e87510241480412ac18e73d");
}

/// t-329 : vecteur figé, calculé hors de Rust (`printf … | shasum -a 256`).
#[test]
fn pack_version_of_several_files_follows_name_order_and_matches_a_frozen_vector() {
    let pack =
        PackFiles::new(object(json!({ "b": 1, "a": [{ "d": 2, "c": "x" }] })), files(&[("kpi", "K"), ("_commun", "C")]));
    let canonical = "{\"a\":[{\"c\":\"x\",\"d\":2}],\"b\":1}\0_commun\0C\0kpi\0K\0";
    assert_that!(pack.canonical()).is_equal_to(canonical.as_bytes().to_vec());
    assert_that!(pack.version().as_str())
        .is_equal_to("sha256:b6aaacfa8364494dd04f71654aa607b38c26ef327b5a8a36e06fc100c644608d");
}

#[test]
fn pack_version_without_widgets_hashes_the_manifest_and_one_zero() {
    let pack = PackFiles::new(object(json!({ "b": 1, "a": [{ "d": 2, "c": "x" }] })), BTreeMap::new());
    assert_that!(pack.version().as_str())
        .is_equal_to("sha256:99b62953dd3d649f39732438515923d2db4f402b3e1ee7a15ab5b343a4974d3a");
    let empty = PackFiles::new(pack.manifest().clone(), files(&[("kpi", "")]));
    assert_that!(empty.version()).is_equal_to(pack.version());
    assert_that!(empty.widgets().is_empty()).is_true();
}

#[test]
fn pack_version_ignores_key_order_at_every_depth() {
    let a = PackFiles::new(object(json!({ "name": "P", "templates": [{ "id": "t", "name": "T" }] })), BTreeMap::new());
    let b = PackFiles::new(object(json!({ "templates": [{ "name": "T", "id": "t" }], "name": "P" })), BTreeMap::new());
    assert_that!(a.version()).is_equal_to(b.version());
}

#[test]
fn pack_version_changes_with_name_source_or_file_name() {
    let base = PackFiles::new(object(json!({ "name": "P" })), files(&[("kpi", "#let widget = none")]));
    let renamed = PackFiles::new(object(json!({ "name": "Q" })), files(&[("kpi", "#let widget = none")]));
    let other = PackFiles::new(object(json!({ "name": "P" })), files(&[("kpi", "#let widget = none ")]));
    let moved = PackFiles::new(object(json!({ "name": "P" })), files(&[("kpj", "#let widget = none")]));
    for changed in [renamed, other, moved] {
        assert_that!(changed.version()).is_not_equal_to(base.version());
    }
}

#[rstest]
#[case("kpi")]
#[case("_commun")]
#[case("kpi-2")]
#[case("a")]
fn widget_file_parse_accepts_lowercase_names_and_common(#[case] name: &str) {
    assert_that!(WidgetFile::parse(name).map(|w| w.as_str().to_owned())).is_equal_to(Some(name.to_owned()));
}

#[rstest]
#[case("Kpi")]
#[case("../x")]
#[case("_autre")]
#[case("")]
#[case("2kpi")]
#[case("kpi.typ")]
#[case("k_pi")]
#[case("é")]
#[case(&"a".repeat(65))]
fn widget_file_parse_refuses_anything_else(#[case] name: &str) {
    assert_that!(WidgetFile::parse(name)).is_none();
}

#[test]
fn widget_file_round_trips_through_json_and_refuses_a_bad_name() {
    let json = serde_json::to_value(wf("kpi")).unwrap();
    assert_that!(json.clone()).is_equal_to(json!("kpi"));
    assert_that!(serde_json::from_value::<WidgetFile>(json).unwrap()).is_equal_to(wf("kpi"));
    assert_that!(serde_json::from_value::<WidgetFile>(json!("Kpi")).is_err()).is_true();
}

#[test]
fn widget_file_file_name_adds_and_strips_typ() {
    assert_that!(wf("kpi").file_name()).is_equal_to("kpi.typ".to_owned());
    assert_that!(WidgetFile::from_file_name("_commun.typ")).is_equal_to(Some(WidgetFile::common()));
    assert_that!(WidgetFile::from_file_name("kpi")).is_none();
    assert_that!(WidgetFile::from_file_name("../x.typ")).is_none();
    assert_that!(WidgetFile::common().is_common()).is_true();
    assert_that!(wf("kpi").is_common()).is_false();
}

#[rstest]
#[case("3356f340f7d5efcbd0e55f5f06a60729765c18942d666015444191d545839756")]
#[case("sha256:3356F340F7D5EFCBD0E55F5F06A60729765C18942D666015444191D545839756")]
#[case("sha256:3356f340")]
#[case("sha256:gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg")]
#[case("sha256:../../../../../../../../../../../../../../../../../../../../../../")]
fn pack_version_parse_refuses_anything_but_lowercase_sha256(#[case] text: &str) {
    assert_that!(PackVersion::parse(text)).is_none();
}

#[test]
fn pack_version_hex_is_the_folder_name() {
    let v = PackVersion::from_hex(&"ab".repeat(32)).unwrap();
    assert_that!(v.hex()).is_equal_to(&*"ab".repeat(32));
    assert_that!(v.as_str()).starts_with("sha256:");
    assert_that!(PackVersion::parse(v.as_str())).is_equal_to(Some(v));
}

#[test]
fn pack_ref_round_trips_through_json() {
    let r = PackRef { id: Uuid::new_v4(), version: PackVersion::from_hex(&"0f".repeat(32)).unwrap() };
    let json = serde_json::to_value(&r).unwrap();
    assert_that!(json["version"].as_str()).is_equal_to(Some(r.version.as_str()));
    assert_that!(serde_json::from_value::<PackRef>(json).unwrap()).is_equal_to(r);
}

#[test]
fn pack_ref_with_a_bad_version_is_refused() {
    let json = json!({ "id": Uuid::new_v4(), "version": "sha256:court" });
    assert_that!(serde_json::from_value::<PackRef>(json).is_err()).is_true();
}

#[test]
fn pack_files_from_bytes_refuses_non_object_and_non_utf8() {
    assert_that!(PackFiles::from_bytes(b"[1]", [])).is_none();
    assert_that!(PackFiles::from_bytes(b"pas du json", [])).is_none();
    assert_that!(PackFiles::from_bytes(br#"{"name":"P"}"#, [(wf("kpi"), &[0xff_u8, 0xfe][..])])).is_none();
    let pack = PackFiles::from_bytes(br#"{"name":"P"}"#, [(wf("kpi"), &b"#let widget = none"[..])]).unwrap();
    assert_that!(pack.widget(&wf("kpi"))).is_equal_to(Some("#let widget = none"));
    assert_that!(pack.widget(&wf("autre"))).is_none();
}

#[test]
fn pack_files_pack_json_reads_back_to_the_same_version() {
    let pack = PackFiles::new(object(json!({ "name": "P", "id": "x" })), files(&[("kpi", "w"), ("_commun", "c")]));
    let widgets = pack.widgets().iter().map(|(n, s)| (n.clone(), s.as_bytes()));
    let back = PackFiles::from_bytes(&pack.pack_json().unwrap(), widgets).unwrap();
    assert_that!(back.version()).is_equal_to(pack.version());
}

#[test]
fn pack_files_matches_only_their_own_version() {
    let pack = PackFiles::new(object(json!({ "name": "P" })), files(&[("kpi", "w")]));
    let tampered = PackFiles::new(object(json!({ "name": "P" })), files(&[("kpi", "x")]));
    assert_that!(pack.matches(&pack.version())).is_true();
    assert_that!(tampered.matches(&pack.version())).is_false();
    assert_that!(pack.version().verifies(&pack)).is_true();
}

/// Un `pack.json` brut, espacé, avec un flottant, du non-ASCII et des échappements.
const SPACED: &[u8] = br#"{ "z" : [ 0.1 , -3 ],
  "ratio": 2.50,
  "name" : "Café \"Ünï\"\t\\ 漢" }"#;

/// t-329 : vecteur figé, calculé hors de Rust (`printf … | shasum -a 256`)
/// sur les octets canoniques ci-dessous.
#[test]
fn pack_version_of_floats_non_ascii_and_escapes_matches_a_frozen_vector() {
    let pack = PackFiles::from_bytes(SPACED, [(wf("kpi"), &b"w"[..])]).unwrap();
    let canonical = "{\"name\":\"Café \\\"Ünï\\\"\\t\\\\ 漢\",\"ratio\":2.5,\"z\":[0.1,-3]}\0kpi\0w\0";
    assert_that!(pack.canonical()).is_equal_to(canonical.as_bytes().to_vec());
    assert_that!(pack.version().as_str())
        .is_equal_to("sha256:e855c082bca6d0f2ef7a4e107750238ea482de41a53a8e92cc57f2787287e928");
}

#[test]
fn pack_files_from_bytes_spaced_differently_give_the_same_version() {
    let compact = br#"{"name":"Café \"Ünï\"\t\\ 漢","ratio":2.5,"z":[0.1,-3]}"#;
    let pretty = "{\n    \"ratio\": 2.5e0,\n    \"name\": \"Café \\\"Ünï\\\"\\t\\\\ 漢\",\n    \"z\": [\n        1e-1,\n        -3\n    ]\n}\n";
    let a = PackFiles::from_bytes(SPACED, [(wf("kpi"), &b"w"[..])]).unwrap();
    let b = PackFiles::from_bytes(compact, [(wf("kpi"), &b"w"[..])]).unwrap();
    let c = PackFiles::from_bytes(pretty.as_bytes(), [(wf("kpi"), &b"w"[..])]).unwrap();
    assert_that!(b.version()).is_equal_to(a.version());
    assert_that!(c.version()).is_equal_to(a.version());
}
```

`crates/oku-slides-core/src/widget_tests.rs`, à la fin :

```rust
#[test]
fn widget_error_names_the_refused_widget() {
    let pack = vec![widget("card", "element", json!([]))];
    let err = WidgetRegistry::from_probe(probe(vec![card()], Some(pack))).unwrap_err();
    assert_that!(err.widget()).is_equal_to(Some("card"));
    let unreadable = WidgetRegistry::from_probe(json!({ "pack": 1 })).unwrap_err();
    assert_that!(unreadable.widget()).is_none();
}
```

`crates/oku-slides-core/src/sources_tests.rs` :
- `:27` : `const MANIFEST: &[u8] = br#"{"format":"oku-slides-sources","version":5}"#;`
- `:80-81` : `from_zip_with_sources_version_3_returns_not_sources` devient `from_zip_with_sources_version_4_returns_not_sources`, manifeste `"version":4`.
- `:422-425`, l'aide `pack` :

```rust
/// Un pack de test : `pack.json` minimal et ses fichiers de widgets.
fn pack(name: &str, widgets: &[(&str, &str)]) -> PackFiles {
    let manifest = serde_json::json!({ "id": "6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10", "name": name });
    let widgets = widgets
        .iter()
        .map(|(n, s)| (crate::WidgetFile::parse(n).unwrap(), (*s).to_owned()))
        .collect();
    PackFiles::new(manifest.as_object().cloned().unwrap(), widgets)
}
```

  Chaque appel `pack("P", None)` devient `pack("P", &[])`, `pack("P", Some(x))` devient `pack("P", &[("kpi", x)])`. Les deux premiers tests de packs deviennent :

```rust
#[test]
fn to_zip_then_from_zip_keeps_packs_and_each_widget_file() {
    let files = pack("P", &[("kpi", "#let widget = none\n"), ("_commun", "#let x = 1\n")]);
    let b = bundle_with(&files);
    let zip = b.to_zip().unwrap();
    let za = zip::ZipArchive::new(Cursor::new(zip.clone())).unwrap();
    let hex = files.version().hex().to_owned();
    for name in ["pack.json", "widgets/kpi.typ", "widgets/_commun.typ"] {
        assert_that!(za.file_names().any(|n| n == format!("packs/{hex}/{name}"))).named(name).is_true();
    }
    let (back, issues) = SourceBundle::from_zip(&zip, MAX, crate::test_registry::shared()).unwrap();
    assert_that!(issues).is_empty();
    assert_that!(back.packs.get(&files.version())).is_equal_to(Some(&files));
}

#[test]
fn to_zip_of_a_pack_without_widgets_writes_no_widgets_folder() {
    let files = pack("P", &[]);
    let zip = bundle_with(&files).to_zip().unwrap();
    let za = zip::ZipArchive::new(Cursor::new(zip)).unwrap();
    assert_that!(za.file_names().any(|n| n.contains("/widgets"))).is_false();
}
```

  À ajouter :

```rust
/// t-329 : un nom invalide sous `packs/<hex>/widgets/` refuse le zip.
#[rstest::rstest]
#[case("widgets/Kpi.typ")]
#[case("widgets/_autre.typ")]
#[case("widgets/kpi")]
#[case("widgets/a/b.typ")]
#[case("autre/x.typ")]
fn from_zip_pack_with_an_invalid_widget_path_returns_bad_name(#[case] file: &str) {
    let loro = deck_loro();
    let name = format!("packs/{}/{file}", "a".repeat(64));
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.loro", &loro), (name.as_str(), b"x")];
    let err = SourceBundle::from_zip(&zip(entries), MAX, crate::test_registry::shared()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::BadName);
}

#[test]
fn from_zip_pack_whose_widget_file_was_altered_is_discarded_and_reported() {
    let files = pack("P", &[("kpi", "#let widget = none\n")]);
    let zip = bundle_with(&files).to_zip().unwrap();
    // Réécrit le zip avec un `kpi.typ` modifié : l'empreinte ne correspond plus.
    let mut za = zip::ZipArchive::new(Cursor::new(zip)).unwrap();
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..za.len() {
        let mut f = za.by_index(i).unwrap();
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut f, &mut data).unwrap();
        if f.name().ends_with("widgets/kpi.typ") {
            data.extend_from_slice(b"// altéré\n");
        }
        entries.push((f.name().to_owned(), data));
    }
    let refs: Vec<(&str, &[u8])> = entries.iter().map(|(n, d)| (n.as_str(), d.as_slice())).collect();
    let (b, issues) = SourceBundle::from_zip(&zip(&refs), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(b.packs.is_empty()).is_true();
    let ids: Vec<&str> = issues.iter().map(|i| i.message.id()).collect();
    assert_that!(ids).contains("issue-pack-version-mismatch");
}
```

  `from_zip_pack_with_widgets_but_no_pack_json_is_discarded_and_reported` (`:536`) : le nom devient `format!("packs/{}/widgets/kpi.typ", "a".repeat(64))`. `from_zip_bare_packs_directory_entries_are_ignored` gagne `#[case(&format!("packs/{}/widgets/", "a".repeat(64)))]` et `#[case(&format!("packs/{}/widgets.typ", "a".repeat(64)))]` (ancien nom : ignoré). `crates/oku-slides-core/tests/zip_limits.rs:56` : `"version":5`.

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-core` → FAIL à la compilation (`WidgetFile`, `PackFiles::new` à deux arguments de map, `widget()` absents).

- [ ] **Step 3 : implémentation.**

`crates/oku-slides-core/src/error.rs`, après `PackVersionError` (`:448`) :

```rust
/// Une chaîne qui n'est pas un [`WidgetFile`](crate::WidgetFile).
#[derive(Debug, Snafu)]
#[snafu(display("invalid widget file name: {value}"), visibility(pub(crate)))]
pub struct WidgetFileError {
    value: String,
    backtrace: Backtrace,
}
```

  Dans `impl WidgetError` (`:404-417`), après `kind` :

```rust
    /// Le widget refusé ; `None` si le registre est illisible.
    pub fn widget(&self) -> Option<&str> {
        match &self.0 {
            WidgetInner::Unreadable { .. } => None,
            WidgetInner::BadSchema { widget, .. }
            | WidgetInner::IncompatibleOverride { widget, .. }
            | WidgetInner::IncompatibleCategory { widget, .. } => Some(widget),
        }
    }
```

`crates/oku-slides-core/src/pack.rs` : la doc `:1-7` devient

```rust
//! Packs de modèles portables : identité, version et fichiers.
//!
//! Un pack a un id ([`PackRef::id`], UUID v4) et des versions adressées par
//! contenu ([`PackVersion`]). Une version : un `pack.json` et des fichiers de
//! widgets `widgets/<nom>.typ` ([`WidgetFile`], `_commun` pour le code
//! partagé). Contenu canonique : `pack.json` en JSON compact à clés triées,
//! un octet `0`, puis pour chaque fichier dans l'ordre des noms : nom, `0`,
//! source, `0`. La version en est le sha-256, écrit `sha256:<hex>`.
```

  imports : `use std::collections::BTreeMap;` et `use crate::error::{PackVersionError, PackVersionSnafu, WidgetFileError, WidgetFileSnafu};`. Avant `PackRef` (`:85`), ajouter :

```rust
/// Le fichier de code partagé d'un pack : `widgets/_commun.typ`.
pub const COMMON_WIDGET_FILE: &str = "_commun";

/// Longueur maximale d'un nom de fichier de widget.
const MAX_WIDGET_NAME: usize = 64;

/// Nom d'un fichier de widget : `[a-z][a-z0-9-]*`, ou `_commun`.
///
/// C'est le nom du widget qu'il déclare ; `_commun` n'est pas un widget.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
#[schemars(with = "String")]
pub struct WidgetFile(String);

impl WidgetFile {
    /// Lit un nom ; `None` hors de `[a-z][a-z0-9-]*` et de `_commun`.
    pub fn parse(s: &str) -> Option<Self> {
        let mut bytes = s.bytes();
        let plain = s.len() <= MAX_WIDGET_NAME
            && bytes.next().is_some_and(|b| b.is_ascii_lowercase())
            && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        (plain || s == COMMON_WIDGET_FILE).then(|| Self(s.to_owned()))
    }

    /// Le fichier partagé `_commun`.
    pub fn common() -> Self {
        Self(COMMON_WIDGET_FILE.to_owned())
    }

    /// Vrai pour `_commun`, qui n'est pas un widget.
    pub fn is_common(&self) -> bool {
        self.0 == COMMON_WIDGET_FILE
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Nom sur disque et dans le zip : `<nom>.typ`.
    pub fn file_name(&self) -> String {
        format!("{}.typ", self.0)
    }

    /// Depuis `<nom>.typ` ; `None` pour tout autre nom.
    pub fn from_file_name(name: &str) -> Option<Self> {
        name.strip_suffix(".typ").and_then(Self::parse)
    }
}

impl TryFrom<String> for WidgetFile {
    type Error = WidgetFileError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(&s).ok_or_else(|| WidgetFileSnafu { value: s }.build())
    }
}

impl From<WidgetFile> for String {
    fn from(w: WidgetFile) -> Self {
        w.0
    }
}

impl fmt::Display for WidgetFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
```

  `PackFiles` (`:93-150`) devient :

```rust
/// Les fichiers d'une version : `pack.json` (un objet) et ses widgets.
#[derive(Debug, Clone, PartialEq)]
pub struct PackFiles {
    manifest: Map<String, Value>,
    widgets: BTreeMap<WidgetFile, String>,
}

impl PackFiles {
    /// Un fichier vide vaut absent : même contenu canonique.
    pub fn new(manifest: Map<String, Value>, mut widgets: BTreeMap<WidgetFile, String>) -> Self {
        widgets.retain(|_, source| !source.is_empty());
        Self { manifest, widgets }
    }

    /// Depuis des octets lus (zip, disque) ; `None` si `pack.json` n'est pas
    /// un objet JSON ou si un fichier de widget n'est pas de l'UTF-8.
    pub fn from_bytes<'a>(
        pack_json: &[u8],
        widgets: impl IntoIterator<Item = (WidgetFile, &'a [u8])>,
    ) -> Option<Self> {
        let Ok(Value::Object(manifest)) = serde_json::from_slice(pack_json) else { return None };
        let widgets = widgets
            .into_iter()
            .map(|(name, bytes)| Some((name, String::from_utf8(bytes.to_vec()).ok()?)))
            .collect::<Option<BTreeMap<_, _>>>()?;
        Some(Self::new(manifest, widgets))
    }

    /// Le `pack.json`, tel que reçu.
    pub fn manifest(&self) -> &Map<String, Value> {
        &self.manifest
    }

    /// Les fichiers de widgets, par nom (`_commun` compris).
    pub fn widgets(&self) -> &BTreeMap<WidgetFile, String> {
        &self.widgets
    }

    /// La source du fichier `name` ; `None` s'il est absent.
    pub fn widget(&self, name: &WidgetFile) -> Option<&str> {
        self.widgets.get(name).map(String::as_str)
    }

    /// La version de ces fichiers.
    pub fn version(&self) -> PackVersion {
        PackVersion::of(self)
    }

    /// Vrai si ces fichiers ont la version `claimed`.
    pub fn matches(&self, claimed: &PackVersion) -> bool {
        claimed.verifies(self)
    }

    /// `pack.json` à écrire : indenté, clés dans l'ordre reçu.
    pub fn pack_json(&self) -> serde_json::Result<Vec<u8>> {
        serde_json::to_vec_pretty(&self.manifest)
    }

    /// Contenu canonique : `pack.json` canonique, `0`, puis nom, `0`, source, `0` par fichier.
    pub fn canonical(&self) -> Vec<u8> {
        // Un `Value` s'écrit toujours : ses clés sont des chaînes.
        let mut out = serde_json::to_vec(&sorted(&Value::Object(self.manifest.clone()))).unwrap_or_default();
        out.push(0);
        for (name, source) in &self.widgets {
            out.extend_from_slice(name.as_str().as_bytes());
            out.push(0);
            out.extend_from_slice(source.as_bytes());
            out.push(0);
        }
        out
    }
}
```

`crates/oku-slides-core/src/lib.rs` : `:18` « [`PackRef`], [`PackVersion`], [`PackFiles`], [`WidgetFile`] : pack de modèles d'un deck, version adressée par contenu, fichiers de widgets. » ; `:39-42` ajouter `WidgetFileError` aux réexports d'`error` ; `:47` : `pub use pack::{COMMON_WIDGET_FILE, PackFiles, PackRef, PackVersion, WidgetFile};`.

`crates/oku-slides-core/src/sources.rs` :
- doc `:1-4` : « Le zip des sources (format `oku-slides-sources`, version 5) : … et les versions de packs de modèles (`packs/<hex>/pack.json`, `packs/<hex>/widgets/<nom>.typ`). » ; `:14-15` : « Un zip d'une version antérieure (1 : `deck.json` ; 2 à 4) n'est plus lu. »
- `:40` : `pub const SOURCES_VERSION: u32 = 5;`
- après `:46` :

```rust
/// Le manifeste d'une version de pack, dans son dossier.
const PACK_JSON: &str = "pack.json";
/// Les fichiers de widgets d'une version de pack, dans son dossier.
const WIDGETS_DIR: &str = "widgets";
```

- import : `use crate::pack::{PackFiles, PackVersion, WidgetFile};`
- `RawPack` et `pack_entry` (`:122-136`) deviennent :

```rust
/// Fichiers d'une version de pack lus du zip, avant vérification.
#[derive(Default)]
struct RawPack {
    pack_json: Option<Vec<u8>>,
    widgets: BTreeMap<WidgetFile, Vec<u8>>,
}

/// Ce qu'est une entrée `packs/<hex>/…`.
enum PackPart {
    Manifest,
    Widget(WidgetFile),
    /// Dossier nu, ou fichier inconnu : ni lu, ni compté.
    Ignored,
}

/// `packs/<hex>/<fichier>` : l'empreinte (64 hex minuscules, sinon refusée)
/// et la partie. Sous `widgets/`, un nom hors de [`WidgetFile`] est refusé,
/// comme tout sous-dossier autre que `widgets/`.
fn pack_entry(name: &str) -> Result<(&str, PackPart), SourcesInner> {
    let rest = name.strip_prefix(PACKS_DIR).unwrap_or(name);
    let (hex, file) = rest.split_once('/').unwrap_or((rest, ""));
    ensure!(PackVersion::from_hex(hex).is_some(), BadNameSnafu { name });
    let part = match file.split_once('/') {
        None if file == PACK_JSON => PackPart::Manifest,
        None => PackPart::Ignored,
        Some((WIDGETS_DIR, "")) => PackPart::Ignored,
        Some((WIDGETS_DIR, widget)) => {
            PackPart::Widget(WidgetFile::from_file_name(widget).context(BadNameSnafu { name })?)
        }
        Some(_) => return BadNameSnafu { name }.fail(),
    };
    Ok((hex, part))
}
```

- `write_zip` `:398-406` :

```rust
            for (version, files) in &self.packs {
                let dir = format!("{PACKS_DIR}{}", version.hex());
                zw.start_file(format!("{dir}/{PACK_JSON}"), deflate).context(ZipSnafu)?;
                zw.write_all(&files.pack_json().context(JsonSnafu { file: PACK_JSON })?).context(IoSnafu)?;
                for (name, source) in files.widgets() {
                    zw.start_file(format!("{dir}/{WIDGETS_DIR}/{}", name.file_name()), deflate).context(ZipSnafu)?;
                    zw.write_all(source.as_bytes()).context(IoSnafu)?;
                }
            }
```

- `read_zip` `:455-468` :

```rust
            if name.starts_with(PACKS_DIR) {
                let (hex, part) = pack_entry(&name)?;
                if matches!(part, PackPart::Ignored) {
                    continue; // Dossier ou fichier inconnu : ni lu, ni compté.
                }
                let data = read_entry(f, size, usize::MAX, &mut budget, limits.total_bytes)?;
                let slot = raw_packs.entry(hex.to_owned()).or_default();
                match part {
                    PackPart::Manifest => slot.pack_json = Some(data),
                    PackPart::Widget(widget) => {
                        slot.widgets.insert(widget, data);
                    }
                    PackPart::Ignored => {}
                }
                continue;
            }
```

- `:502-505` : `for (hex, RawPack { pack_json, widgets }) in raw_packs {` … `let files = pack_json.and_then(|p| PackFiles::from_bytes(&p, widgets.iter().map(|(n, b)| (n.clone(), b.as_slice()))));`

`crates/oku-slides-pdf/src/lib.rs:18-27` et `ARCHITECTURE.md:22-30` : « (version 5) », `{ format: "oku-slides-sources", version: 5 }`, la ligne `widgets.typ` remplacée par `packs/<hex>/widgets/<nom>.typ  un par fichier de widget (_commun compris)`.

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-core -p oku-slides-pdf` → PASS. `cargo build -p oku-slides-render -p oku-slides-render-worker` → OK (ils n'utilisent pas `PackFiles`).

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-core crates/oku-slides-pdf
```
```bash
git commit -m "feat(core)! : fichiers de widgets d'un pack, contenu canonique multi-fichiers et zip des sources v5 (t-329)"
```

---

### Task 2 : bornes des fichiers de widgets (`[render]`)

**Files:**
- Modify: `crates/oku-slides-platform/src/config.rs` (`RenderSettings` `:85-94`, `impl RenderSettings` `:108-123`)
- Modify: `crates/oku-slides-platform/src/config.default.toml` (`[render]` `:34-41`)
- Modify: `crates/oku-slides-platform/src/config_tests.rs` (`:150-162` + un test)
- Modify (littéraux `RenderSettings { … }`) : `crates/oku-slides-render-worker/src/link_tests.rs:13,131,144`, `client_tests.rs:14`, `tests/process.rs:23`

**Interfaces:**
- Produces : `RenderSettings.max_widget_kib: usize`, `RenderSettings.max_widgets: usize`, `RenderSettings::max_widget_bytes(&self) -> usize`.

- [ ] **Step 1 : test en échec** — `config_tests.rs`, à la fin :

```rust
#[test]
fn defaults_bound_widget_files() {
    let s = Settings::defaults().unwrap();
    assert_that!(s.render.max_widget_kib).is_equal_to(256);
    assert_that!(s.render.max_widgets).is_equal_to(64);
    assert_that!(s.render.max_widget_bytes()).is_equal_to(256 * 1024);
}
```

  Les littéraux `RenderSettings { timeout_ms: …, memory_mib: …, max_frame_mib: … }` (`config_tests.rs:150,156,162` et les fichiers listés) gagnent `max_widget_kib: 256, max_widgets: 64`.

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-platform` → FAIL (champs absents).

- [ ] **Step 3 : implémentation.** `RenderSettings`, après `max_frame_mib` :

```rust
    /// Taille maximale d'un fichier de widget d'un pack (Kio).
    pub max_widget_kib: usize,
    /// Nombre maximal de fichiers de widgets d'une version de pack, `_commun` compris.
    pub max_widgets: usize,
```

  `impl RenderSettings` :

```rust
    /// [`Self::max_widget_kib`] en octets.
    pub fn max_widget_bytes(&self) -> usize {
        self.max_widget_kib.saturating_mul(1024)
    }
```

  `config.default.toml`, après `max_frame_mib = 512` :

```toml
# Taille maximale d'un fichier de widget d'un pack (Kio).
max_widget_kib = 256
# Nombre maximal de fichiers de widgets d'une version de pack, `_commun` compris.
max_widgets = 64
```

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-platform -p oku-slides-render-worker` → PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-platform crates/oku-slides-render-worker
```
```bash
git commit -m "feat(platform) : bornes des fichiers de widgets dans [render] (t-329)"
```

---

### Task 3 : pack en fichiers, sonde par fichier et registre assemblé (render)

**Files:**
- Modify: `crates/oku-slides-render/src/diagnostic.rs` (`Diagnostic` `:13-22`, nouveau `SourceAt`)
- Modify: `crates/oku-slides-render/src/registry.rs` (`PackWidgets` `:27-65`, `probe_source`/`probe_registry` `:67-96`, base `:108-121`, `Registries` `:131-172`)
- Create: `crates/oku-slides-render/src/widget_probe.rs`
- Modify: `crates/oku-slides-render/src/world.rs` (champ `pack` `:168-169`, `set_pack` `:243-256`, `diagnostic` `:312-329`)
- Modify: `crates/oku-slides-render/src/lib.rs` (`mod` `:20-27`, réexports `:42-49`, littéral `Diagnostic` `:352-357`)
- Modify: `crates/oku-slides-render/src/registry_tests.rs`, `world_tests.rs:21`, `wire_tests.rs:8`, `tests/render.rs:275`, `tests/widgets.rs`, `tests/fields.rs:118`

**Interfaces:**
- Consumes : `WidgetFile`, `WidgetError::widget` (Task 1).
- Produces :
  - `pub struct SourceAt { pub path: String, pub line: usize, pub column: usize }` (à partir de 1) ; `Diagnostic.at: Option<SourceAt>` (`#[serde(default)]`) ; `Diagnostic::widget_file(&self) -> Option<WidgetFile>` ; `Diagnostic::widget_error(&WidgetFile, impl Into<String>) -> Diagnostic`.
  - `PackWidgets::new(impl Into<String>, BTreeMap<WidgetFile, String>) -> Result<PackWidgets, RegistryError>` ; `id(&self) -> &str`, `files(&self) -> &BTreeMap<WidgetFile, String>`, `common(&self) -> Option<&str>`, `only(&self, &BTreeSet<WidgetFile>) -> PackWidgets`, `fingerprint(&self) -> [u8; 32]` (id, `0`, puis nom, `0`, source, `0` par fichier).
  - `pub fn base_probe(&Fonts) -> Result<Arc<Json>, RegistryError>` (une fois par processus).
  - `pub const PROBE_PACK: &str = "sonde";` `pub struct FileProbe { pub schema: Option<Json>, pub diagnostics: Vec<Diagnostic> }` (`Serialize + Deserialize + Clone + PartialEq`), `FileProbe::broken(&WidgetFile, impl Into<String>) -> FileProbe`, `FileProbe::is_valid(&self) -> bool`.
  - `pub fn probe_key(&WidgetFile, &str, Option<&str>) -> [u8; 32]`, `pub fn probe_file(&Fonts, &WidgetFile, &str, Option<&str>) -> FileProbe`, `pub fn assemble(&Json, &mut BTreeMap<WidgetFile, FileProbe>) -> Result<WidgetRegistry, RegistryError>`.
  - `Registries::probe_file(&mut self, &WidgetFile, &str, Option<&str>) -> FileProbe` (mémoïsé par `probe_key`).
  - Inchangés : `load_registry`, `probe_registry`, `Registries::{get, probe}` (pack assemblé).

- [ ] **Step 1 : tests en échec** — `crates/oku-slides-render/src/registry_tests.rs`, à la suite des tests existants (les deux tests de `PackWidgets` `:39-53` réécrits avec `files(…)`) :

```rust
use std::collections::BTreeMap;

use oku_slides_core::{WidgetFile, WidgetRegistry};

use super::{compile_probe, probe_source};
use crate::base_registry;
use crate::widget_probe::{FileProbe, assemble, probe_file};
use crate::world::OkuSlidesWorld;

fn wf(name: &str) -> WidgetFile {
    WidgetFile::parse(name).unwrap()
}

fn files(entries: &[(&str, &str)]) -> BTreeMap<WidgetFile, String> {
    entries.iter().map(|(n, s)| (wf(n), (*s).to_owned())).collect()
}

const QUOTE_FILE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
"#;

const KPI_FILE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#import "_commun.typ": cadre
#let widget = widget("kpi", params: (value: p.plain()), (args, ctx) => cadre(field(ctx, "value", args.value)))
"#;

const COMMON: &str = "#let cadre(body) = block(inset: 8pt, body)\n";

/// Le même pack, en un seul `widgets.typ` écrit à la main.
const HAND_WRITTEN: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let cadre(body) = block(inset: 8pt, body)
#let quote = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
#let kpi = widget("kpi", params: (value: p.plain()), (args, ctx) => cadre(field(ctx, "value", args.value)))
#let registry = (kpi, quote)
"#;

/// Le registre d'un `widgets.typ` écrit à la main, servi sous `/packs/main/`.
fn hand_written_registry(source: &str) -> WidgetRegistry {
    let mut world = OkuSlidesWorld::new(Fonts::default()).unwrap();
    world.add_source("/packs/main/widgets.typ", source.to_owned()).unwrap();
    world.set_main_text(&probe_source(Some("/packs/main/widgets.typ")));
    registry_from_probe(compile_probe(&world).unwrap()).unwrap()
}

#[test]
fn load_registry_of_files_equals_the_hand_written_widgets_typ() {
    let pack = PackWidgets::new("p", files(&[("quote", QUOTE_FILE), ("kpi", KPI_FILE), ("_commun", COMMON)])).unwrap();
    let assembled = load_registry(&Fonts::default(), Some(&pack)).unwrap();
    assert_that!(assembled).is_equal_to(hand_written_registry(HAND_WRITTEN));
    assert_that!(assembled.get("_commun")).is_none();
}

#[test]
fn pack_assembly_imports_each_widget_but_not_common() {
    let pack = PackWidgets::new("p", files(&[("quote", QUOTE_FILE), ("kpi", KPI_FILE), ("_commun", COMMON)])).unwrap();
    assert_that!(pack.assembly()).is_equal_to(
        "#import \"widgets/kpi.typ\": widget as w0\n#import \"widgets/quote.typ\": widget as w1\n#let registry = (w0,w1,)\n"
            .to_owned(),
    );
    let empty = PackWidgets::new("p", files(&[("_commun", COMMON)])).unwrap();
    assert_that!(empty.assembly()).is_equal_to("#let registry = ()\n".to_owned());
}

#[test]
fn probe_file_of_a_broken_widget_returns_diagnostics_at_line_and_column() {
    let broken = "// Cassé\n#let widget = (\n";
    let probe = probe_file(&Fonts::default(), &wf("casse"), broken, None);
    assert_that!(probe.is_valid()).is_false();
    let d = probe.diagnostics.iter().find(|d| d.widget_file() == Some(wf("casse"))).unwrap();
    assert_that!(d.at.as_ref().map(|a| a.line)).is_equal_to(Some(2));
    assert_that!(d.at.as_ref().is_some_and(|a| a.column >= 1)).is_true();
}

#[test]
fn probe_file_of_common_is_valid_without_schema() {
    let probe = probe_file(&Fonts::default(), &WidgetFile::common(), COMMON, None);
    assert_that!(probe.is_valid()).is_true();
    assert_that!(probe.schema).is_none();
}

#[test]
fn probe_file_importing_common_reads_it() {
    let probe = probe_file(&Fonts::default(), &wf("kpi"), KPI_FILE, Some(COMMON));
    assert_that!(probe.is_valid()).is_true();
    assert_that!(probe.schema.as_ref().and_then(|s| s["name"].as_str())).is_equal_to(Some("kpi"));
    let without = probe_file(&Fonts::default(), &wf("kpi"), KPI_FILE, None);
    assert_that!(without.is_valid()).is_false();
}

#[test]
fn probe_file_whose_declared_name_differs_from_its_file_is_refused() {
    let src = "#import \"/oku-slides/lib.typ\": widget\n#let widget = widget(\"autre\", (args, ctx) => [])\n";
    let probe = probe_file(&Fonts::default(), &wf("kpi"), src, None);
    assert_that!(probe.is_valid()).is_false();
    assert_that!(probe.diagnostics[0].message.as_str()).contains("autre");
    assert_that!(probe.diagnostics[0].at.as_ref().map(|a| a.line)).is_equal_to(Some(2));
}

#[test]
fn assemble_keeps_the_valid_widgets_and_sets_aside_the_broken_one() {
    let fonts = Fonts::default();
    let base = crate::registry::base_probe(&fonts).unwrap();
    let mut probes: BTreeMap<WidgetFile, FileProbe> = BTreeMap::from([
        (wf("quote"), probe_file(&fonts, &wf("quote"), QUOTE_FILE, None)),
        (wf("casse"), probe_file(&fonts, &wf("casse"), "#let widget = (", None)),
    ]);
    let registry = assemble(&base, &mut probes).unwrap();
    assert_that!(registry.get("quote")).is_some();
    assert_that!(registry.get("casse")).is_none();
    assert_that!(registry.get("card")).is_some();
}

#[test]
fn assemble_sets_aside_an_incompatible_override_with_a_diagnostic() {
    let fonts = Fonts::default();
    let base = crate::registry::base_probe(&fonts).unwrap();
    let card = "#import \"/oku-slides/lib.typ\": widget, params as p\n#let widget = widget(\"card\", params: (title: p.plain()), (args, ctx) => [])\n";
    let mut probes = BTreeMap::from([(wf("card"), probe_file(&fonts, &wf("card"), card, None))]);
    assert_that!(probes[&wf("card")].is_valid()).is_true();
    let registry = assemble(&base, &mut probes).unwrap();
    assert_that!(probes[&wf("card")].is_valid()).is_false();
    assert_that!(registry.get("card")).is_equal_to(base_registry(&fonts).unwrap().get("card"));
}

#[test]
fn registries_probe_file_recompiles_only_what_changed() {
    let mut r = Registries::new(Fonts::default());
    r.probe_file(&wf("quote"), QUOTE_FILE, Some(COMMON));
    r.probe_file(&wf("kpi"), KPI_FILE, Some(COMMON));
    assert_that!(r.file_compiles).is_equal_to(2);
    // Un widget modifié : lui seul est recompilé.
    let changed = QUOTE_FILE.replace("source: p.plain()", "source: p.plain(), note: p.plain()");
    r.probe_file(&wf("quote"), &changed, Some(COMMON));
    r.probe_file(&wf("kpi"), KPI_FILE, Some(COMMON));
    assert_that!(r.file_compiles).is_equal_to(3);
    // `_commun` modifié : seuls ceux qui l'importent sont recompilés.
    let common = "#let cadre(body) = block(inset: 4pt, body)\n";
    r.probe_file(&wf("quote"), &changed, Some(common));
    assert_that!(r.file_compiles).is_equal_to(3);
    r.probe_file(&wf("kpi"), KPI_FILE, Some(common));
    assert_that!(r.file_compiles).is_equal_to(4);
}

#[test]
fn pack_fingerprint_changes_with_any_file() {
    let a = PackWidgets::new("p", files(&[("kpi", KPI_FILE), ("_commun", COMMON)])).unwrap();
    let b = PackWidgets::new("p", files(&[("kpi", KPI_FILE), ("_commun", "#let cadre(body) = body\n")])).unwrap();
    assert_that!(a.fingerprint()).is_not_equal_to(b.fingerprint());
}
```

  Réécriture des tests existants (même règle partout) : un pack `#let x = widget("n", …)` + `#let registry = (x,)` devient le fichier `n` contenant `#let widget = widget("n", …)` (ligne `registry` retirée), passé par `PackWidgets::new(id, files(&[("n", SOURCE)]))` ; un pack vide (`""`, `#let registry = ()`) devient `BTreeMap::new()`.
  - `registry_tests.rs:39-53` : `PackWidgets::new("p", files(&[("quote", QUOTE_FILE)]))` vs `files(&[("quote", &format!("{QUOTE_FILE} "))])` ; `registries_probe_is_memoized_by_fingerprint` avec `files(&[("quote", QUOTE_FILE)])`.
  - `world_tests.rs:21` : `crate::PackWidgets::new("citations", BTreeMap::new())` (l'import attendu reste `/packs/citations/widgets.typ`).
  - `wire_tests.rs:8` et `tests/render.rs:275` : le littéral `Diagnostic { … }` gagne `at: None`.
  - `tests/widgets.rs` : `QUOTE` et `RED_CARD` en fichiers `quote` et `card` ; `load_registry_pack_breaking_card_returns_schema_error` : fichier `card` ; `load_registry_pack_that_does_not_compile_returns_its_diagnostics` : `files(&[("casse", "#let widget = (")])`, la place attendue contient `packs/casse/widgets/casse.typ` ; `pack_widgets_new_with_a_path_like_id_returns_bad_pack_id` : `PackWidgets::new(id, BTreeMap::new())`. `tests/fields.rs:118` : même règle.

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-render` → FAIL à la compilation (`widget_probe`, `PackWidgets::new` à map, `SourceAt` absents).

- [ ] **Step 3 : implémentation.**

`crates/oku-slides-render/src/diagnostic.rs`, avant `Diagnostic` :

```rust
use oku_slides_core::WidgetFile;

use crate::widget_probe::PROBE_PACK;

/// Position d'un message dans un fichier Typst, à partir de 1.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SourceAt {
    /// Chemin dans le monde Typst, sans `/` initial (`packs/<id>/widgets/kpi.typ`).
    pub path: String,
    pub line: usize,
    /// Colonne en caractères.
    pub column: usize,
}
```

  `Diagnostic` gagne, après `hints` :

```rust
    /// Fichier, ligne et colonne, s'ils sont connus.
    #[serde(default)]
    pub at: Option<SourceAt>,
```

  et, après `impl Localize` :

```rust
impl Diagnostic {
    /// Le fichier de widget d'un pack où se trouve le message, s'il y en a un.
    pub fn widget_file(&self) -> Option<WidgetFile> {
        let path = match &self.at {
            Some(at) => at.path.as_str(),
            None => {
                let place = self.place.as_deref()?;
                place.split_once(':').map_or(place, |(path, _)| path)
            }
        };
        let rest = path.trim_start_matches('/').strip_prefix("packs/")?;
        let (_, file) = rest.split_once("/widgets/")?;
        WidgetFile::from_file_name(file)
    }

    /// Une erreur sur le fichier de widget `name`, sans position.
    pub fn widget_error(name: &WidgetFile, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            place: Some(format!("packs/{PROBE_PACK}/widgets/{}", name.file_name())),
            message: message.into(),
            hints: Vec::new(),
            at: None,
        }
    }
}
```

`world.rs` : import `use crate::diagnostic::{Diagnostic, Severity, SourceAt};` ; `diagnostic` (`:312-329`) :

```rust
    /// Un message de Typst, avec sa place (`fichier:ligne`) et sa colonne.
    pub(crate) fn diagnostic(&self, d: &SourceDiagnostic) -> Diagnostic {
        let at = d.span.id().and_then(|id| {
            let range = self.range(d.span)?;
            let source = self.source_of(id)?;
            let (line, column) = source.lines().byte_to_line_column(range.start)?;
            Some(SourceAt { path: id.vpath().get_without_slash().to_string(), line: line + 1, column: column + 1 })
        });
        Diagnostic {
            severity: match d.severity {
                typst::diag::Severity::Error => Severity::Error,
                typst::diag::Severity::Warning => Severity::Warning,
            },
            place: at.as_ref().map(|a| format!("{}:{}", a.path, a.line)),
            message: d.message.to_string(),
            hints: d.hints.iter().map(|h| h.v.to_string()).collect(),
            at,
        }
    }
```

  `:168` : `/// Widgets du pack : ses fichiers sous /packs/<id>/widgets/, l'assemblage sous /packs/<id>/widgets.typ.` ; `set_pack` (`:243-256`) :

```rust
    pub(crate) fn set_pack(
        &mut self,
        pack: Option<&PackWidgets>,
        registry: &WidgetRegistry,
    ) -> Result<(), RenderError> {
        if let Some(old) = self.pack.take() {
            for (path, _) in old.sources() {
                self.sources.remove(&file_id(&path)?);
            }
        }
        if let Some(p) = pack {
            for (path, text) in p.sources() {
                self.add_source(&path, text)?;
            }
        }
        self.pack = pack.cloned();
        self.redraw_charts(registry)
    }
```

  `lib.rs:352-357` : le littéral gagne `at: None`.

`crates/oku-slides-render/src/registry.rs` : doc `:1-2` « Registre des widgets : une sonde Typst par pack (base, puis le module d'assemblage du pack) et une sonde par fichier de widget (`widget_probe`). » ; imports `use std::collections::{BTreeMap, BTreeSet, HashMap};`, `use oku_slides_core::{WidgetFile, WidgetRegistry};`, `use crate::widget_probe::{FileProbe, probe_key};`. `PackWidgets` (`:27-65`) :

```rust
/// Les widgets d'un pack de modèles : son identifiant et ses fichiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackWidgets {
    id: String,
    files: BTreeMap<WidgetFile, String>,
}

impl PackWidgets {
    /// `id` : voir [`is_pack_id`] ; il devient un chemin.
    pub fn new(id: impl Into<String>, files: BTreeMap<WidgetFile, String>) -> Result<Self, RegistryError> {
        let id = id.into();
        if !is_pack_id(&id) {
            return Err(BadPackIdSnafu { id }.build().into());
        }
        Ok(Self { id, files })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    /// Les fichiers du pack, `_commun` compris.
    pub fn files(&self) -> &BTreeMap<WidgetFile, String> {
        &self.files
    }

    /// La source de `_commun`, s'il y en a un.
    pub fn common(&self) -> Option<&str> {
        self.files.get(&WidgetFile::common()).map(String::as_str)
    }

    /// Le même pack, réduit à `_commun` et aux widgets de `keep`.
    pub fn only(&self, keep: &BTreeSet<WidgetFile>) -> Self {
        let files = self
            .files
            .iter()
            .filter(|(n, _)| n.is_common() || keep.contains(*n))
            .map(|(n, s)| (n.clone(), s.clone()))
            .collect();
        Self { id: self.id.clone(), files }
    }

    /// Empreinte (identifiant et fichiers) : clé des caches de registre.
    pub fn fingerprint(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(self.id.as_bytes());
        h.update([0]);
        for (name, source) in &self.files {
            h.update(name.as_str().as_bytes());
            h.update([0]);
            h.update(source.as_bytes());
            h.update([0]);
        }
        h.finalize().into()
    }

    /// Module d'assemblage : importe chaque widget (pas `_commun`) et forme `registry`.
    pub(crate) fn assembly(&self) -> String {
        let mut out = String::new();
        let mut names = String::new();
        for (i, name) in self.files.keys().filter(|n| !n.is_common()).enumerate() {
            out.push_str(&format!("#import \"widgets/{}\": widget as w{i}\n", name.file_name()));
            names.push_str(&format!("w{i},"));
        }
        out.push_str(&format!("#let registry = ({names})\n"));
        out
    }

    /// Chemin du module d'assemblage.
    pub(crate) fn path(&self) -> String {
        format!("/packs/{}/widgets.typ", self.id)
    }

    /// Chaque source du pack et son chemin, assemblage compris.
    pub(crate) fn sources(&self) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = self
            .files
            .iter()
            .map(|(n, s)| (format!("/packs/{}/widgets/{}", self.id, n.file_name()), s.clone()))
            .collect();
        out.push((self.path(), self.assembly()));
        out
    }
}
```

  `probe_source`/`probe_registry` (`:67-96`) :

```rust
/// Source de la sonde : importe la base et le module `pack_path`, émet leurs schémas.
pub(crate) fn probe_source(pack_path: Option<&str>) -> String {
    let mut s = String::from("#import \"/oku-slides/src/widgets.typ\": registry as base\n");
    let pack_expr = match pack_path {
        Some(path) => {
            s.push_str(&format!("#import \"{path}\": registry as pack\n"));
            "pack.map(w => w.schema)"
        }
        None => "none",
    };
    s.push_str(&format!("#metadata((base: base.map(w => w.schema), pack: {pack_expr}))<oku-widgets>\n"));
    s
}

/// Compile la sonde posée dans `world` et rend son JSON brut.
pub(crate) fn compile_probe(world: &OkuSlidesWorld) -> Result<Json, RegistryError> {
    let result = typst::compile::<PagedDocument>(world);
    let doc = result.output.map_err(|errors| {
        RegistryCompileSnafu { diagnostics: errors.iter().map(|d| world.diagnostic(d)).collect::<Vec<_>>() }.build()
    })?;
    let label = Label::new(PicoStr::intern("oku-widgets")).context(MissingSnafu)?;
    let found = doc.introspector().query(&Selector::Label(label));
    let metadata = found.first().and_then(|c| c.get_by_name("value").ok());
    probe_json(metadata.map(|v| serde_json::to_value(&v)))
}

/// Compile la sonde et rend son JSON brut (schémas de la base et du pack).
pub fn probe_registry(fonts: &Fonts, pack: Option<&PackWidgets>) -> Result<Json, RegistryError> {
    let mut world = OkuSlidesWorld::new(fonts.clone()).map_err(Box::new).context(SetupSnafu)?;
    if let Some(p) = pack {
        for (path, text) in p.sources() {
            world.add_source(&path, text).map_err(Box::new).context(SetupSnafu)?;
        }
    }
    world.set_main_text(&probe_source(pack.map(PackWidgets::path).as_deref()));
    compile_probe(&world)
}
```

  Base (`:108-121`) : ajouter, avant `BASE` :

```rust
/// Sonde brute de la base, lue une fois par processus.
static BASE_PROBE: Mutex<Option<Arc<Json>>> = Mutex::new(None);

/// La sonde brute (JSON) de la base, compilée au premier appel.
pub fn base_probe(fonts: &Fonts) -> Result<Arc<Json>, RegistryError> {
    let mut probe = BASE_PROBE.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(p) = probe.as_ref() {
        return Ok(p.clone());
    }
    let p = Arc::new(probe_registry(fonts, None)?);
    *probe = Some(p.clone());
    Ok(p)
}
```

  et `base_registry` lit `registry_from_probe((*base_probe(fonts)?).clone())` au lieu de `load_registry(fonts, None)`.

  `Registries` (`:131-172`) : champs `files: HashMap<[u8; 32], FileProbe>` et `#[cfg(test)] pub(crate) file_compiles: usize` (initialisés dans `new`), et :

```rust
    /// La sonde du fichier `name` (avec `_commun`), mémoïsée par [`probe_key`].
    pub fn probe_file(&mut self, name: &WidgetFile, source: &str, common: Option<&str>) -> FileProbe {
        let key = probe_key(name, source, common);
        if let Some(probe) = self.files.get(&key) {
            return probe.clone();
        }
        #[cfg(test)]
        {
            self.file_compiles += 1;
        }
        let probe = crate::widget_probe::probe_file(&self.fonts, name, source, common);
        self.files.insert(key, probe.clone());
        probe
    }
```

`crates/oku-slides-render/src/widget_probe.rs` (nouveau) :

```rust
//! Sonde d'un fichier de widget, compilé seul : son schéma ou ses
//! diagnostics. Un widget cassé est écarté sans emporter les autres.
//!
//! Règles :
//! - Le fichier est servi sous `/packs/sonde/widgets/<nom>.typ`, avec
//!   `_commun.typ` : la sonde ne dépend pas de la version du pack.
//! - Le nom déclaré doit être celui du fichier, sinon le fichier est refusé.
//! - [`assemble`] forme le registre des valides ; un widget que le registre
//!   refuse (surcharge incompatible, schéma invalide) devient écarté.

use std::collections::BTreeMap;

use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use snafu::IntoError;
use typst::foundations::{Label, Selector};
use typst::introspection::Introspector;
use typst::utils::PicoStr;
use typst_layout::PagedDocument;

use oku_slides_core::{WidgetFile, WidgetRegistry};

use crate::diagnostic::{Diagnostic, Severity, SourceAt};
use crate::error::{RegistryError, SchemaSnafu};
use crate::fonts::Fonts;
use crate::world::OkuSlidesWorld;

/// Identifiant de pack des sondes de fichiers.
pub const PROBE_PACK: &str = "sonde";

/// Ce que la sonde dit d'un fichier : son schéma, ou pourquoi il est écarté.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FileProbe {
    /// Schéma brut du widget ; `None` pour `_commun` ou un fichier écarté.
    pub schema: Option<Json>,
    /// Erreurs et avertissements de Typst, ou du contrôle du schéma.
    pub diagnostics: Vec<Diagnostic>,
}

impl FileProbe {
    /// Un fichier écarté pour `message`, sans position.
    pub fn broken(name: &WidgetFile, message: impl Into<String>) -> Self {
        Self { schema: None, diagnostics: vec![Diagnostic::widget_error(name, message)] }
    }

    /// Vrai si aucune erreur ne l'écarte.
    pub fn is_valid(&self) -> bool {
        !self.diagnostics.iter().any(|d| d.severity == Severity::Error)
    }
}

/// Le fichier importe-t-il `_commun` ? Lecture textuelle, assez pour le cache.
fn imports_common(source: &str) -> bool {
    source.contains("_commun.typ")
}

/// Clé de cache : nom, source, et `_commun` si le fichier l'importe.
pub fn probe_key(name: &WidgetFile, source: &str, common: Option<&str>) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(name.as_str().as_bytes());
    h.update([0]);
    h.update(source.as_bytes());
    h.update([0]);
    match common.filter(|_| imports_common(source)) {
        Some(c) => {
            h.update([1]);
            h.update(c.as_bytes());
        }
        None => h.update([0]),
    }
    h.finalize().into()
}

/// Chemin d'un fichier dans le monde de la sonde.
fn path_of(name: &WidgetFile) -> String {
    format!("/packs/{PROBE_PACK}/widgets/{}", name.file_name())
}

/// Source de la sonde : importe le fichier et émet son schéma (`_commun` : rien).
fn probe_source(name: &WidgetFile) -> String {
    if name.is_common() {
        format!("#import \"{}\": *\n", path_of(name))
    } else {
        format!("#import \"{}\": widget as candidate\n#metadata(candidate.schema)<oku-widget-file>\n", path_of(name))
    }
}

/// Compile `name` seul (avec `_commun`) ; tout échec devient un diagnostic.
pub fn probe_file(fonts: &Fonts, name: &WidgetFile, source: &str, common: Option<&str>) -> FileProbe {
    let mut world = match OkuSlidesWorld::new(fonts.clone()) {
        Ok(world) => world,
        Err(e) => return FileProbe::broken(name, e.to_string()),
    };
    let mut files = vec![(name.clone(), source)];
    if let Some(c) = common.filter(|_| !name.is_common()) {
        files.push((WidgetFile::common(), c));
    }
    for (file, text) in files {
        if let Err(e) = world.add_source(&path_of(&file), text.to_owned()) {
            return FileProbe::broken(name, e.to_string());
        }
    }
    world.set_main_text(&probe_source(name));
    let result = typst::compile::<PagedDocument>(&world);
    let mut diagnostics: Vec<Diagnostic> = result.warnings.iter().map(|d| world.diagnostic(d)).collect();
    let doc = match result.output {
        Ok(doc) => doc,
        Err(errors) => {
            diagnostics.splice(0..0, errors.iter().map(|d| world.diagnostic(d)));
            return FileProbe { schema: None, diagnostics };
        }
    };
    if name.is_common() {
        return FileProbe { schema: None, diagnostics };
    }
    let schema = Label::new(PicoStr::intern("oku-widget-file"))
        .and_then(|label| {
            doc.introspector().query(&Selector::Label(label)).first().and_then(|c| c.get_by_name("value").ok())
        })
        .and_then(|v| serde_json::to_value(&v).ok());
    let Some(schema) = schema else { return FileProbe::broken(name, "the widget file emits no schema") };
    let declared = schema.get("name").and_then(Json::as_str).unwrap_or_default().to_owned();
    if declared != name.as_str() {
        diagnostics.insert(0, misnamed(name, source, &declared));
        return FileProbe { schema: None, diagnostics };
    }
    FileProbe { schema: Some(schema), diagnostics }
}

/// Le nom déclaré diffère du nom de fichier : diagnostic à la déclaration.
fn misnamed(name: &WidgetFile, source: &str, declared: &str) -> Diagnostic {
    let message = format!("the widget declares the name \"{declared}\" but its file is {}", name.file_name());
    let mut d = Diagnostic::widget_error(name, message);
    let found = (!declared.is_empty()).then(|| source.find(&format!("\"{declared}\""))).flatten();
    if let Some(byte) = found {
        let before = &source[..byte];
        let line = before.matches('\n').count() + 1;
        let column = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
        d.at = Some(SourceAt { path: path_of(name).trim_start_matches('/').to_owned(), line, column });
    }
    d
}

/// Le registre de la base (`base` : sonde brute) surchargée par les widgets
/// valides de `files`. Un widget que le registre refuse y devient écarté,
/// avec un diagnostic, et l'assemblage reprend sans lui.
pub fn assemble(base: &Json, files: &mut BTreeMap<WidgetFile, FileProbe>) -> Result<WidgetRegistry, RegistryError> {
    let base = base.get("base").cloned().unwrap_or_else(|| Json::Array(Vec::new()));
    loop {
        let pack: Vec<Json> = files
            .iter()
            .filter(|(n, p)| !n.is_common() && p.is_valid())
            .filter_map(|(_, p)| p.schema.clone())
            .collect();
        let err = match WidgetRegistry::from_probe(json!({ "base": base, "pack": pack })) {
            Ok(registry) => return Ok(registry),
            Err(e) => e,
        };
        let culprit = err.widget().and_then(WidgetFile::parse).filter(|n| files.get(n).is_some_and(FileProbe::is_valid));
        let Some(name) = culprit else { return Err(SchemaSnafu.into_error(Box::new(err)).into()) };
        if let Some(probe) = files.get_mut(&name) {
            probe.schema = None;
            probe.diagnostics.insert(0, Diagnostic::widget_error(&name, err.to_string()));
        }
    }
}
```

  (Si `SchemaSnafu` n'est pas une variante de `RegistryInner` construite ainsi, reprendre la forme de `registry_from_probe` : `Err(Box::new(err)).context(SchemaSnafu).map_err(Into::into)`.)

`lib.rs` : `mod widget_probe;` ; réexports `pub use diagnostic::{Diagnostic, Severity, SourceAt};`, `pub use registry::{PackWidgets, Registries, base_probe, base_registry, is_pack_id, load_registry, probe_registry, registry_from_probe};`, `pub use widget_probe::{FileProbe, PROBE_PACK, assemble, probe_file, probe_key};` ; doc `:17-18` : « … lu par une sonde Typst (pack assemblé) ou fichier par fichier ([`probe_file`], [`assemble`]). »

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-render` → PASS (snapshots inchangés : `cargo insta test -p oku-slides-render` ne doit rien proposer).

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-render
```
```bash
git commit -m "feat(render)! : pack de widgets en fichiers, sonde par fichier et registre assemblé (t-329)"
```

---

### Task 4 : sonde d'un pack fichier par fichier et emplacement d'essai (render-worker)

**Files:**
- Modify: `crates/oku-slides-render-worker/src/protocol.rs` (`WirePack` `:8-13`, `Request` `:30-32`, `Response` `:86-97`)
- Modify: `crates/oku-slides-render-worker/src/serve.rs` (`:113`, `:154-158`, `:178-185`)
- Modify: `crates/oku-slides-render-worker/src/link.rs` (`fingerprint_hex` `:41-48`, rejeu `:267`)
- Modify: `crates/oku-slides-render-worker/src/client.rs` (`Shared` `:53-68`, `RenderWorker` `:72`, `wire` `:81-83`, `blame`/`faulty_pack` `:98-111`, `new` `:122-124`, `link`/`prepare` `:127,135`, `probe`/`is_faulty` `:172,199`, `slot` `:202-224`, `RenderSlot` `:247-285`, `render` `:300-323`, `RemoteCompiled` `:384-476`)
- Create: `crates/oku-slides-render-worker/src/pack_probe.rs`
- Modify: `crates/oku-slides-render-worker/src/lib.rs` (`mod pack_probe;`, `pub use pack_probe::PackProbe;`, doc `:7-13`)
- Modify: `crates/oku-slides-render-worker/src/test_packs.rs`, `client_tests.rs`, `link_tests.rs:17,57-58,87-88,105`, `serve_tests.rs:90`, `tests/process.rs:51,58,77`

**Interfaces:**
- Consumes : `PackWidgets`, `FileProbe`, `probe_key`, `assemble`, `base_probe` (Task 3) ; `RenderSettings::{max_widgets, max_widget_bytes}` (Task 2).
- Produces :
  - `WirePack { id: String, files: BTreeMap<String, String> }` ; `Request::ProbeFile { name: String, source: String, common: Option<String> }` → `Response::FileProbe { probe: FileProbe }`.
  - `pub struct PackProbe { pub files: BTreeMap<WidgetFile, FileProbe>, pub pack: Option<PackWidgets>, pub registry: Arc<WidgetRegistry> }`.
  - `RenderWorker::probe_pack(&self, &PackWidgets) -> Result<PackProbe, RegistryError>` : jamais d'accusation ; échec borné → diagnostic du fichier ; cache par `probe_key`, échecs bornés compris ; au-delà de `max_widgets` fichiers ou de `max_widget_kib`, diagnostic sans exécution.
  - `RenderWorker::trial_slot(&self, &Fonts, &Localizer, Arc<WidgetRegistry>) -> Result<RenderSlot, RenderError>` : emplacement dont les échecs bornés n'accusent aucun pack (ni `set_widgets`, ni `render`, ni export) et qui ignore la liste des packs fautifs.
  - Inchangés : `probe` (pack assemblé, accuse), `is_faulty`, `slot`.

- [ ] **Step 1 : tests en échec** — `test_packs.rs` (constantes en fichiers, plus l'aide) :

```rust
/// Boucle au chargement du fichier `boucle` (sonde et `SetWidgets`), hors des gardes de Typst.
///
/// Deux `for` imbriqués sur des tableaux courts : Typst borne les `while`
/// (boucle jugée infinie), pas les `for`, et `range` y reste petit.
pub(crate) const LOOP_ON_IMPORT: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let _spin = { let n = 0; for _ in range(100000) { for _ in range(100000) { n += 1 } }; n }
#let widget = widget("boucle", params: (body: p.plain()), (args, ctx) => [])
"#;

/// Le fichier `boucle` : boucle au rendu du widget seulement.
pub(crate) const LOOP_ON_RENDER: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("boucle", params: (body: p.plain()),
  (args, ctx) => { let n = 0; for _ in range(100000) { for _ in range(100000) { n += 1 } }; [#n] })
"#;

/// Le fichier `gros` : alloue sans fin au chargement.
pub(crate) const ALLOCATE: &str = r#"#import "/oku-slides/lib.typ": widget
#let _big = range(400000000).map(i => str(i))
#let widget = widget("gros", (args, ctx) => [])
"#;

/// Le fichier `quote`, valide.
pub(crate) const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich()), (args, ctx) => field(ctx, "body", args.body))
"#;

/// Des fichiers de widgets, par nom.
pub(crate) fn files(entries: &[(&str, &str)]) -> std::collections::BTreeMap<oku_slides_core::WidgetFile, String> {
    entries
        .iter()
        .map(|(n, s)| {
            let name = oku_slides_core::WidgetFile::parse(n).unwrap_or_else(|| panic!("nom invalide : {n}"));
            (name, (*s).to_owned())
        })
        .collect()
}
```

  `tiny_deck` reste ; `client_tests.rs` importe `use crate::test_packs::{LOOP_ON_IMPORT, LOOP_ON_RENDER, QUOTE, files, tiny_deck};`. Réécriture : `PackWidgets::new("boucle", LOOP_ON_IMPORT)` → `PackWidgets::new("boucle", files(&[("boucle", LOOP_ON_IMPORT)]))` (de même `rendu`/`LOOP_ON_RENDER`, `gros`/`ALLOCATE`, et `"#let registry = ()"` → `BTreeMap::new()`) ; `WirePack { id, source }` → `WirePack { id, files: BTreeMap::from([("boucle".into(), LOOP_ON_IMPORT.into())]) }` ; `link_tests.rs:87-88` : `fingerprint_hex(&wire)` égal à `hex::encode(pack.fingerprint())`. `client_tests.rs`, nouveaux tests :

```rust
fn worker_with(timeout_ms: u64, max_widgets: usize) -> RenderWorker {
    let settings = RenderSettings { timeout_ms, memory_mib: 4096, max_frame_mib: 64, max_widget_kib: 256, max_widgets };
    RenderWorker::new(RenderLaunch::InProcess, settings)
}

fn wf(name: &str) -> oku_slides_core::WidgetFile {
    oku_slides_core::WidgetFile::parse(name).unwrap()
}

#[test]
fn probe_pack_keeps_the_valid_widgets_and_reports_the_broken_one() {
    let w = worker(10_000);
    let pack = PackWidgets::new("mixte", files(&[("quote", QUOTE), ("casse", "#let widget = (")])).unwrap();
    let probe = w.probe_pack(&pack).unwrap();
    assert_that!(probe.files[&wf("quote")].is_valid()).is_true();
    assert_that!(probe.files[&wf("casse")].is_valid()).is_false();
    assert_that!(probe.registry.get("quote")).is_some();
    let kept = probe.pack.unwrap();
    assert_that!(kept.files().keys().cloned().collect::<Vec<_>>()).is_equal_to(vec![wf("quote")]);
}

#[test]
fn probe_pack_with_a_file_that_loops_gives_a_diagnostic_and_accuses_nothing() {
    let w = worker(300);
    let pack = PackWidgets::new("boucle", files(&[("boucle", LOOP_ON_IMPORT), ("quote", QUOTE)])).unwrap();
    let probe = w.probe_pack(&pack).unwrap();
    assert_that!(probe.files[&wf("boucle")].is_valid()).is_false();
    assert_that!(probe.files[&wf("quote")].is_valid()).is_true();
    assert_that!(w.is_faulty(&pack)).is_false();
    assert_that!(w.is_faulty(probe.pack.as_ref().unwrap())).is_false();
    // Gardé : la même sonde ne coûte plus le délai.
    let started = std::time::Instant::now();
    w.probe_pack(&pack).unwrap();
    assert_that!(started.elapsed()).is_less_than(std::time::Duration::from_millis(300));
}

#[test]
fn probe_pack_beyond_max_widgets_does_not_load_the_extra_files() {
    let w = worker_with(10_000, 1);
    let pack = PackWidgets::new("trop", files(&[("a", QUOTE), ("b", QUOTE)])).unwrap();
    let probe = w.probe_pack(&pack).unwrap();
    assert_that!(probe.files[&wf("b")].diagnostics[0].message.as_str()).contains("more than 1");
}

#[test]
fn trial_slot_render_that_loops_times_out_and_accuses_nothing() {
    let w = worker(300);
    let pack = PackWidgets::new("rendu", files(&[("boucle", LOOP_ON_RENDER)])).unwrap();
    let probe = w.probe_pack(&pack).unwrap();
    let kept = probe.pack.clone().unwrap();
    let base = base_registry(&Fonts::default()).unwrap();
    let mut s = w.trial_slot(&Fonts::default(), &Localizer::new(Some("fr")), base).unwrap();
    s.set_widgets(Some(&kept), probe.registry.clone()).unwrap();
    let mut deck = tiny_deck();
    deck.slides[0] = serde_json::from_value(serde_json::json!({
        "id": "s", "kind": "content",
        "params": { "blocks": [{ "id": "b", "kind": "boucle", "params": { "body": "x" } }] }
    }))
    .unwrap();
    let err = s.render(&deck, &BTreeMap::new()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Timeout);
    assert_that!(w.is_faulty(&kept)).is_false();
    // Pas d'écartement : l'essai suivant retente (délai, pas `PackDisabled`).
    s.set_widgets(Some(&kept), probe.registry).unwrap();
    assert_that!(s.render(&deck, &BTreeMap::new()).unwrap_err().kind()).is_equal_to(RenderErrorKind::Timeout);
}
```

  `serve_tests.rs`, nouveau :

```rust
#[test]
fn probe_file_request_returns_the_file_probe() {
    let mut worker = Worker::default();
    let req = Request::ProbeFile { name: "casse".into(), source: "#let widget = (".into(), common: None };
    let (reply, _) = worker.handle(req, Vec::new());
    let Response::FileProbe { probe } = reply else { panic!("réponse inattendue : {reply:?}") };
    assert_that!(probe.is_valid()).is_false();
}
```

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-render-worker` → FAIL à la compilation.

- [ ] **Step 3 : implémentation.**

`protocol.rs` :

```rust
/// Un pack de widgets : son identifiant et ses fichiers (nom → source).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct WirePack {
    pub(crate) id: String,
    pub(crate) files: BTreeMap<String, String>,
}
```

  `Request`, après `Probe { … }` :

```rust
    /// Sonde un fichier de widget seul ; `common` : la source de `_commun`.
    ProbeFile {
        name: String,
        source: String,
        #[serde(default)]
        common: Option<String>,
    },
```

  `Response`, après `Probe { … }` : `FileProbe { probe: oku_slides_render::FileProbe },`.

`link.rs:41-48` :

```rust
/// Empreinte hexadécimale d'un pack, égale à celle de `PackWidgets::fingerprint`.
pub(crate) fn fingerprint_hex(pack: &WirePack) -> String {
    let mut h = Sha256::new();
    h.update(pack.id.as_bytes());
    h.update([0]);
    for (name, source) in &pack.files {
        h.update(name.as_bytes());
        h.update([0]);
        h.update(source.as_bytes());
        h.update([0]);
    }
    hex::encode(h.finalize())
}
```

  `:267` : `fingerprint_hex(p)`.

`serve.rs` : `:113` ajoute `Request::ProbeFile { name, source, common } => self.probe_file(&name, &source, common.as_deref()),` ; `probe` (`:154-158`) et `set_widgets` (`:178-185`) construisent le pack par `pack_of(pack)?` :

```rust
/// Le pack d'une requête ; un nom de fichier invalide répond `Error Setup`.
fn pack_of(p: WirePack) -> Result<PackWidgets, WireError> {
    let files = p
        .files
        .into_iter()
        .map(|(n, s)| WidgetFile::parse(&n).map(|n| (n, s)).ok_or_else(|| setup(format!("bad widget file {n}"))))
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    PackWidgets::new(p.id, files).map_err(|e| WireError::from_registry(&e))
}
```

```rust
    fn probe_file(&mut self, name: &str, source: &str, common: Option<&str>) -> Result<Reply, WireError> {
        let name = WidgetFile::parse(name).ok_or_else(|| setup(format!("bad widget file {name}")))?;
        let probe = self.registries().probe_file(&name, source, common);
        Ok((Response::FileProbe { probe }, Vec::new()))
    }
```

  et la règle de la doc (`:9`) : « `ProbeFile` sonde un fichier seul (`Registries::probe_file`, mémoïsé). »

`client.rs` :
- `Shared` devient `pub(crate)` avec, en plus : `pub(crate) file_probes: Mutex<HashMap<[u8; 32], FileProbe>>`, `pub(crate) max_widgets: usize`, `pub(crate) max_widget_bytes: usize` ; `pub struct RenderWorker(pub(crate) Arc<Mutex<Link>>, pub(crate) Arc<Shared>);` ; `fn link` et `fn prepare` deviennent `pub(crate)`, `fn unexpected` aussi.
- `new` :

```rust
    pub fn new(launch: RenderLaunch, settings: RenderSettings) -> Self {
        let shared =
            Shared { max_widgets: settings.max_widgets, max_widget_bytes: settings.max_widget_bytes(), ..Shared::default() };
        Self(Arc::new(Mutex::new(Link::new(launch, settings))), Arc::new(shared))
    }
```

- `wire` : `WirePack { id: pack.id().to_owned(), files: pack.files().iter().map(|(n, s)| (n.to_string(), s.clone())).collect() }` ; `fingerprint_hex(&p.id, &p.source)` → `fingerprint_hex(p)` ; `fingerprint_hex(pack.id(), pack.source())` → `hex::encode(pack.fingerprint())`.
- `RenderSlot` et `RemoteCompiled` gagnent `accuse: bool` (« Un échec borné y accuse le pack ; `false` pour un emplacement d'essai. ») ; `slot` le pose à `true`, `clone_handle_for_test` le recopie ; puis :

```rust
    /// Ouvre un emplacement d'essai : aucun échec n'y accuse un pack.
    ///
    /// Pour l'aperçu d'un widget en cours d'écriture : le pack n'est pas
    /// écarté, et un pack déjà fautif y est tout de même posé.
    pub fn trial_slot(&self, fonts: &Fonts, l: &Localizer, base: Arc<WidgetRegistry>) -> Result<RenderSlot, RenderError> {
        let mut slot = self.slot(fonts, l, base)?;
        slot.accuse = false;
        Ok(slot)
    }
```

- `RenderSlot::set_widgets` : le contrôle des fautifs devient `if self.accuse && let Some(p) = &wire && link.faulty_packs.contains(&fingerprint_hex(p))` ; l'accusé est `let blamed = wire.as_ref().filter(|_| self.accuse);`, passé à `prepare_for` et `blame`. `render` : `if self.accuse && let Some(pack) = faulty_pack(&link, self.id)` ; `let pack = link.state.slots.get(&self.id).and_then(|s| s.pack.clone()).filter(|_| self.accuse);` ; le `RemoteCompiled` rendu reçoit `accuse: self.accuse`. `RemoteCompiled::bytes` : `let pack = … .filter(|_| self.accuse);`.

`pack_probe.rs` (nouveau) :

```rust
//! Sonde d'un pack fichier par fichier, côté hôte : chaque fichier de
//! widget est compilé seul dans le travailleur, puis le registre des seuls
//! valides est assemblé ici.
//!
//! Règles :
//! - Un échec borné (délai, mémoire, arrêt) pendant la sonde d'un fichier
//!   donne un diagnostic pour ce fichier ; aucun pack n'est accusé.
//! - Les sondes sont gardées par `probe_key` (nom, source, `_commun`
//!   importé), échecs bornés compris : un fichier qui boucle ne se recompile
//!   qu'une fois modifié. Un autre échec du lien n'est pas gardé.
//! - Au-delà de `max_widgets` fichiers (dans l'ordre des noms), ou pour un
//!   fichier de plus de `max_widget_kib`, rien n'est exécuté : un diagnostic.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, PoisonError};

use oku_slides_core::{WidgetFile, WidgetRegistry};
use oku_slides_render::{FileProbe, Fonts, PackWidgets, RegistryError, assemble, base_probe, probe_key};

use crate::client::{RenderWorker, unexpected};
use crate::protocol::{Request, Response};

/// La sonde d'un pack, fichier par fichier.
#[derive(Debug, Clone)]
pub struct PackProbe {
    /// Par fichier (`_commun` compris) : schéma ou diagnostics.
    pub files: BTreeMap<WidgetFile, FileProbe>,
    /// `_commun` et les widgets valides ; `None` sans widget valide.
    pub pack: Option<PackWidgets>,
    /// La base surchargée par les widgets valides.
    pub registry: Arc<WidgetRegistry>,
}

impl RenderWorker {
    /// Sonde chaque fichier de `pack`, puis assemble le registre des valides.
    ///
    /// Seule une base illisible, ou un registre refusé sans widget à
    /// écarter, est une erreur.
    pub fn probe_pack(&self, pack: &PackWidgets) -> Result<PackProbe, RegistryError> {
        let (max_widgets, max_bytes) = (self.1.max_widgets, self.1.max_widget_bytes);
        let common = pack.common();
        let mut files = BTreeMap::new();
        for (i, (name, source)) in pack.files().iter().enumerate() {
            let probe = if i >= max_widgets {
                FileProbe::broken(name, format!("more than {max_widgets} widget files in the pack: not loaded"))
            } else if source.len() > max_bytes {
                FileProbe::broken(name, format!("larger than {} KiB: not loaded", max_bytes / 1024))
            } else {
                self.probe_file(name, source, common.filter(|_| !name.is_common()))
            };
            files.insert(name.clone(), probe);
        }
        // Les schémas ne dépendent pas des polices : celles de Typst suffisent.
        let base = base_probe(&Fonts::default())?;
        let registry = Arc::new(assemble(&base, &mut files)?);
        let valid: BTreeSet<WidgetFile> =
            files.iter().filter(|(n, p)| !n.is_common() && p.is_valid()).map(|(n, _)| n.clone()).collect();
        let pack = (!valid.is_empty()).then(|| pack.only(&valid));
        Ok(PackProbe { files, pack, registry })
    }

    /// La sonde d'un fichier dans le travailleur, gardée par `probe_key`.
    fn probe_file(&self, name: &WidgetFile, source: &str, common: Option<&str>) -> FileProbe {
        let key = probe_key(name, source, common);
        if let Some(p) = self.1.file_probes.lock().unwrap_or_else(PoisonError::into_inner).get(&key) {
            return p.clone();
        }
        let req =
            Request::ProbeFile { name: name.to_string(), source: source.to_owned(), common: common.map(str::to_owned) };
        let mut link = self.link();
        let reply = self.prepare(&mut link).and_then(|()| link.call(&req, &[]));
        drop(link);
        let (probe, keep) = match reply {
            Ok((Response::FileProbe { probe }, _)) => (probe, true),
            Ok((Response::Error { error }, _)) => (FileProbe::broken(name, error.into_render().to_string()), false),
            Ok((other, _)) => (FileProbe::broken(name, unexpected(&other).to_string()), false),
            // Délai, mémoire, arrêt : ce fichier est en cause, lui seul.
            Err(e) if e.kind().is_bounded() => (FileProbe::broken(name, e.to_string()), true),
            Err(e) => (FileProbe::broken(name, e.to_string()), false),
        };
        if keep {
            self.1.file_probes.lock().unwrap_or_else(PoisonError::into_inner).insert(key, probe.clone());
        }
        probe
    }
}
```

`lib.rs` : `mod pack_probe;`, `pub use pack_probe::PackProbe;`, et la doc : « [`RenderWorker`] : … `probe_pack` (sonde par fichier, sans accusation), `trial_slot` (emplacement d'aperçu). »

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-render-worker` → PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-render-worker
```
```bash
git commit -m "feat(render-worker) : sonde d'un pack fichier par fichier et emplacement d'essai (t-329)"
```

---

### Task 5 : la session lit, garde et rend les packs à plusieurs fichiers (migration)

**Files:**
- Create: `crates/oku-slides-session/src/request.rs` (`Request` et `impl Request` sortis de `lib.rs:142-288`, `Session::edits` `lib.rs:663-682` devenu `Request::edits`)
- Modify: `crates/oku-slides-session/src/lib.rs` (`mod request; pub use request::Request;`, `handle` : `Self::edits(&req)` → `req.edits()`, imports devenus inutiles retirés)
- Modify: `crates/oku-slides-session/src/library.rs` (doc `:6-10`, `WIDGETS_FILE` `:39`, `BUILTIN` `:62`, `read_version` `:160-175`, `add_version` `:246-251`)
- Modify: `crates/oku-slides-session/src/resume.rs` (`:280-295` écriture, `:452-466` lecture, doc `:7-8`)
- Modify: `crates/oku-slides-session/src/deck_widgets.rs` (doc `:1-3`, `resolve_widgets` `:51-82`)
- Modify: `crates/oku-slides-session/src/templates_mode.rs:175-178`
- Modify: `crates/oku-slides-session/src/pack.rs:50-56` (`to_files`)
- Modify: `crates/oku-slides-session/src/test_support.rs:61-68` (+ `widget_files`)
- Modify (tests) : `deck_widgets_tests.rs`, `library_tests.rs`, `resume_tests.rs:306-352`, `pack_tests.rs`, `pack_notice_tests.rs:13`, `templates_mode_tests.rs`, `tests/session.rs:361`, `tests/regressions/issue_0327_pack_qui_boucle.rs`, `tests/regressions/issue_0328_pack_recu_garde_la_version_courante.rs:30`
- Modify: `crates/oku-slides-i18n/src/locales/*/session.ftl` (`session-widgets-broken`, 20 langues)

**Interfaces:**
- Consumes : `PackFiles` multi-fichiers, `WidgetFile` (Task 1) ; `PackWidgets::new(_, BTreeMap)`, `FileProbe` (Task 3) ; `RenderWorker::probe_pack`, `PackProbe` (Task 4).
- Produces :
  - `PackManifest::to_files(&self, widgets: BTreeMap<WidgetFile, String>) -> serde_json::Result<PackFiles>`.
  - `Request` dans `request.rs` (réexporté tel quel), `Request::edits(&self) -> bool` (`pub(crate)`).
  - `resolve_widgets` : chaque fichier sondé à part ; widgets cassés écartés, avis `session-widgets-broken` (`{ $names }`) ; pack sans widget valide → base ; pack assemblé fautif → base + avis de `RenderError::pack_disabled`.
  - `Session::broken_widgets_notice(&self, &BTreeMap<WidgetFile, FileProbe>) -> Option<String>` (`pub(crate)`).
  - `test_support::widget_files(&[(&str, &str)]) -> BTreeMap<WidgetFile, String>` ; `test_support::add_version(library, manifest, widgets: &[(&str, &str)]) -> PackRef`.

- [ ] **Step 1 : découpe de `lib.rs`, sans changement de comportement.** Créer `request.rs` :

```rust
//! Les requêtes de l'interface (même forme pour Tauri et HTTP).

use std::path::PathBuf;

use serde::Deserialize;
use uuid::Uuid;

use oku_slides_core::Op;

use crate::views::UiState;
```

  puis y déplacer **tels quels** l'énum `Request` et `impl Request { pub fn changes_deck … }` (`lib.rs:144-288`) et, dans le même `impl`, `Session::edits` (`lib.rs:663-682`) devenu :

```rust
    /// Requêtes refusées pendant l'historique : elles modifieraient le deck,
    /// l'enregistreraient, ou passeraient aux modèles.
    pub(crate) fn edits(&self) -> bool {
        matches!(
            self,
            Self::Apply { .. }
                | Self::Undo
                | Self::Redo
                | Self::Save { .. }
                | Self::Merge { .. }
                | Self::Move { .. }
                | Self::SaveCopy { .. }
                | Self::ChartKind { .. }
                | Self::EditTemplates { .. }
                | Self::SaveTemplates { .. }
                | Self::CloseTemplates
                | Self::DeleteTemplateSet { .. }
                | Self::PackUseMine
        )
    }
```

  `lib.rs` : `mod request;` (ordre alphabétique des `mod`), `pub use request::Request;`, `if matches!(self.mode, Mode::History { .. }) && req.edits()` ; retirer les `use` que `cargo clippy -p oku-slides-session` signale inutilisés. `cargo nextest run -p oku-slides-session` ne compile pas encore (Task 1) : vérifier seulement `wc -l crates/oku-slides-session/src/lib.rs` (≈ 810 lignes).

- [ ] **Step 2 : tests en échec.** `test_support.rs` :

```rust
/// Des fichiers de widgets, par nom.
pub(crate) fn widget_files(entries: &[(&str, &str)]) -> std::collections::BTreeMap<oku_slides_core::WidgetFile, String> {
    entries
        .iter()
        .map(|(n, s)| {
            let name = oku_slides_core::WidgetFile::parse(n).unwrap_or_else(|| panic!("nom invalide : {n}"));
            (name, (*s).to_owned())
        })
        .collect()
}

/// Ajoute `manifest` (et ses fichiers de widgets) à `library` ; sa référence.
pub(crate) fn add_version(
    library: &crate::library::Library,
    manifest: &crate::pack::PackManifest,
    widgets: &[(&str, &str)],
) -> oku_slides_core::PackRef {
    let files = manifest.to_files(widget_files(widgets)).unwrap_or_else(|e| panic!("pack.json : {e}"));
    library.add_version(manifest.id, &files).unwrap_or_else(|e| panic!("version : {}", e.one_line()));
    oku_slides_core::PackRef { id: manifest.id, version: files.version() }
}
```

  `deck_widgets_tests.rs` : constantes en fichiers (règle de la Task 3) —

```rust
/// Le fichier `quote` : un widget d'élément, avec un corps riche.
const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
"#;

/// Un fichier `casse` qui ne compile pas.
const BROKEN: &str = "#let widget = (";

/// Le fichier `boucle` : boucle au rendu, hors des gardes de Typst.
const LOOP_ON_RENDER: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("boucle", params: (body: p.plain()),
  (args, ctx) => { let n = 0; for _ in range(100000) { for _ in range(100000) { n += 1 } }; [#n] })
"#;

/// Le fichier `boucle` : boucle dès son chargement (sonde).
const LOOP_ON_IMPORT: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let _spin = { let n = 0; for _ in range(100000) { for _ in range(100000) { n += 1 } }; n }
#let widget = widget("boucle", params: (body: p.plain()), (args, ctx) => [])
"#;
```

  `user_pack(s, name, widgets: &[(&str, &str)])` et `bundle_with_pack(s, name, widgets: &[(&str, &str)])` (corps : `m.to_files(crate::test_support::widget_files(widgets))`) ; les appels passent `&[("quote", QUOTE)]`, `&[("casse", BROKEN)]`, `&[("boucle", LOOP_ON_RENDER)]`, `&[("boucle", LOOP_ON_IMPORT)]`. `load_sources_with_a_broken_pack_keeps_the_base_registry_and_sets_a_notice` : l'avis attendu contient `"casse"` (avis `session-widgets-broken`) au lieu de `registry-compile`. `save_templates_as_a_copy_takes_the_pack_widgets_along` : `stored.files.widget(&WidgetFile::parse("quote").unwrap())` vaut `Some(QUOTE)`. `restart_worker_outside_the_deck` : `PackWidgets::new("autre", crate::test_support::widget_files(&[("boucle", LOOP_ON_IMPORT)]))` (`worker.probe` assemble et accuse toujours). Ajouter :

```rust
/// t-329 : un widget cassé est écarté, les autres du même pack restent.
#[test]
fn apply_set_templates_to_a_pack_with_one_broken_widget_keeps_the_others() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    let mixed = user_pack(&s, "Mixte", &[("quote", QUOTE), ("casse", BROKEN)]);
    s.apply("Pack", vec![set_templates(Some(&mixed)), add_quote()], None).unwrap();
    assert_that!(widgets_have_quote(&mut s)).is_true();
    assert_that!(has_quote(&s)).is_true();
    assert_that!(s.doc.notice.clone().unwrap_or_default().as_str()).contains("casse");
    assert_registry_agrees(&s);
}
```

  `library_tests.rs` : `to_files(None)` → `to_files(BTreeMap::new())` ; `:45` : `on_disk.join("widgets").join("kpi.typ")` (version écrite avec `&[("kpi", "#let widget = none")]`) ; ajouter :

```rust
#[test]
fn add_version_then_versions_reads_every_widget_file_and_ignores_strangers() {
    let dir = crate::test_support::fresh_dir("oku-slides-library-widgets-");
    let lib = library(dir.path());
    let id = uuid::Uuid::new_v4();
    let m = manifest(id, "A", None, "2026-10-02T10:00:00Z");
    let r = crate::test_support::add_version(&lib, &m, &[("kpi", "K"), ("_commun", "C")]);
    let at = dir.path().join(id.to_string()).join("versions").join(r.version.hex()).join("widgets");
    std::fs::write(at.join(".DS_Store"), b"x").unwrap();
    let files = lib.version(id, &r.version).unwrap();
    assert_that!(files.widgets().len()).is_equal_to(2);
    assert_that!(files.matches(&r.version)).is_true();
}
```

  `resume_tests.rs:306-352` : `pack_files(name, widgets: &[(&str, &str)])` ; `write_snapshot_then_read_doc_keeps_the_packs_and_their_widgets` vérifie `packs.join(format!("{}.kpi.typ", with.hex()))` et `packs.join(format!("{}._commun.typ", with.hex()))` (pack `&[("kpi", "K"), ("_commun", "C")]`) et l'absence de tout `{without}.*.typ` ; le test d'altération réécrit `{v}.kpi.typ` ; `write_snapshot_removes_orphan_packs` vérifie l'absence de `{v}.json` et `{v}.kpi.typ`. `pack_tests.rs:10-12` : `m.to_files(widget_files(&[("kpi", "#let widget = none")]))`, `files.widget(&WidgetFile::parse("kpi").unwrap())`. `pack_notice_tests.rs:13`, `templates_mode_tests.rs`, `tests/session.rs:361` (`from_bytes(…, [])`), `issue_0328…:30` : `to_files(BTreeMap::new())`. `issue_0327_pack_qui_boucle.rs` : constante en fichier `boucle` (`#let widget = widget("boucle", …)`, ligne `registry` retirée), `manifest.to_files(BTreeMap::from([(WidgetFile::parse("boucle").unwrap_or_else(|| panic!("nom")), LOOP_ON_RENDER.into())]))`.

- [ ] **Step 3 :** `cargo nextest run -p oku-slides-session` → FAIL à la compilation (`to_files` à map, `probe_pack` non utilisé, etc.).

- [ ] **Step 4 : implémentation.**

`pack.rs:50-56` :

```rust
    /// Les fichiers de cette version, avec ses fichiers de widgets.
    pub fn to_files(&self, widgets: BTreeMap<WidgetFile, String>) -> serde_json::Result<PackFiles> {
        match serde_json::to_value(self)? {
            Value::Object(manifest) => Ok(PackFiles::new(manifest, widgets)),
            _ => Err(serde::ser::Error::custom("pack manifest is not an object")),
        }
    }
```

`library.rs` : doc `:9` → `<data>/templates/<uuid>/versions/<hex>/widgets/<nom>.typ  (un par fichier de widget)` ; `:39` → `const WIDGETS_DIR: &str = "widgets";` ; `:62` → `PackFiles::from_bytes(json.as_bytes(), [])` ; `read_version` `:163-170` :

```rust
        let pack = self.storage.read(&folder.join(PACK_FILE)).map_err(|e| one_line(&e))?;
        let mut widgets = BTreeMap::new();
        for path in self.storage.list_files(&folder.join(WIDGETS_DIR)).map_err(|e| one_line(&e))? {
            let file = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            let Some(name) = WidgetFile::from_file_name(file) else {
                tracing::debug!(path = %path.display(), "fichier étranger aux widgets ignoré");
                continue;
            };
            widgets.insert(name, self.storage.read(&path).map_err(|e| one_line(&e))?);
        }
        let files = PackFiles::from_bytes(&pack, widgets.iter().map(|(n, b)| (n.clone(), b.as_slice())))
            .ok_or_else(|| "unreadable pack files".to_owned())?;
```

  `add_version` `:247-251` :

```rust
        // Les widgets d'abord : un `pack.json` présent dit la version complète.
        for (name, source) in files.widgets() {
            let path = at.join(WIDGETS_DIR).join(name.file_name());
            self.storage.write_atomic(&path, source.as_bytes()).context(WriteSnafu { path: &path })?;
        }
```

`resume.rs` : doc `:7-8` « … plus ses assets, polices et packs (`<hex>.json`, `<hex>.<nom>.typ`), écrits une fois par nom. » ; `:280-295` :

```rust
        // Les packs sont adressés par contenu : `<hex>.json` et un `<hex>.<nom>.typ` par fichier de widget.
        let packs_dir = dir.join("packs");
        let mut keep_packs = BTreeSet::new();
        for (version, files) in &doc.blobs.packs {
            let hex = version.hex();
            let json = packs_dir.join(format!("{hex}.json"));
            if !self.storage.exists(&json).context(AccessSnafu { path: &json })? {
                // Les widgets d'abord : un `<hex>.json` présent dit le pack complet.
                for (name, source) in files.widgets() {
                    self.put(&packs_dir.join(format!("{hex}.{}", name.file_name())), source.as_bytes().to_vec())?;
                }
                self.put(&json, files.pack_json().context(JsonSnafu { path: &json })?)?;
            }
            keep_packs.insert(format!("{hex}.json"));
            keep_packs.extend(files.widgets().keys().map(|n| format!("{hex}.{}", n.file_name())));
        }
```

  `:452-466` :

```rust
        let raw_packs = self.read_files(&dir.join("packs"))?;
        let mut packs = BTreeMap::new();
        for (name, bytes) in &raw_packs {
            let Some(hex) = name.strip_suffix(".json") else { continue };
            let prefix = format!("{hex}.");
            let widgets = raw_packs
                .iter()
                .filter_map(|(n, b)| WidgetFile::from_file_name(n.strip_prefix(&prefix)?).map(|w| (w, b.as_slice())));
            // Vérifiée par son empreinte : un pack altéré est ignoré.
            let checked = PackVersion::from_hex(hex)
                .and_then(|v| PackFiles::from_bytes(bytes, widgets).filter(|f| f.matches(&v)).map(|f| (v, f)));
            match checked {
                Some((version, files)) => {
                    packs.insert(version, files);
                }
                None => tracing::warn!(version = hex, "pack de reprise altéré ignoré"),
            }
        }
```

`deck_widgets.rs` : doc `:1-3` « Widgets du deck : la base, surchargée par les widgets valides de la version de pack qu'il embarque (`deck.templates`, cherchée dans `SourceBundle.packs`, jamais dans la bibliothèque) ; chaque fichier est sondé à part. » ; `resolve_widgets` (`:51-82`) :

```rust
    /// Les widgets de la version de pack `templates`, lue dans `packs` (les
    /// sources du document) : la base, surchargée par ses widgets valides.
    ///
    /// Version absente des sources : la base, et l'avis `session-pack-missing`.
    /// Pas de widget : la base, sans avis. Chaque fichier est sondé à part :
    /// un widget cassé est écarté (avis `session-widgets-broken`), les autres
    /// restent. Pack écarté pour la session ou registre illisible : la base
    /// et l'avis de son erreur. Seule la base qui ne se lit pas est une erreur.
    pub(crate) fn resolve_widgets(
        &self,
        templates: Option<&PackRef>,
        packs: &BTreeMap<PackVersion, PackFiles>,
    ) -> Result<(DeckWidgets, Option<String>)> {
        let base = |notice| -> Result<(DeckWidgets, Option<String>)> {
            let registry = oku_slides_render::base_registry(&self.app_fonts).context(RegistrySnafu)?;
            Ok((DeckWidgets { templates: templates.cloned(), pack: None, registry }, notice))
        };
        let Some(r) = templates else { return base(None) };
        let Some(files) = packs.get(&r.version) else {
            tracing::warn!(pack = %r.id, version = %r.version, "version du pack absente des sources : widgets de base");
            return base(Some(self.localizer.tr("session-pack-missing")));
        };
        if files.widgets().is_empty() {
            return base(None);
        }
        let probed = PackWidgets::new(r.version.hex(), files.widgets().clone())
            .and_then(|pack| self.worker.probe_pack(&pack));
        let probe = match probed {
            Ok(probe) => probe,
            Err(e) => {
                tracing::warn!(pack = %r.id, error = %oku_slides_platform::errors::one_line(&e), "pack de widgets illisible : widgets de base");
                return base(Some(e.localize(&self.localizer)));
            }
        };
        let notice = self.broken_widgets_notice(&probe.files);
        match probe.pack {
            Some(pack) if self.worker.is_faulty(&pack) => {
                base(Some(RenderError::pack_disabled(pack.id()).localize(&self.localizer)))
            }
            Some(pack) => {
                Ok((DeckWidgets { templates: Some(r.clone()), pack: Some(pack), registry: probe.registry }, notice))
            }
            None => base(notice),
        }
    }

    /// L'avis des widgets écartés de `files`, s'il y en a.
    pub(crate) fn broken_widgets_notice(&self, files: &BTreeMap<WidgetFile, FileProbe>) -> Option<String> {
        let names: Vec<&str> = files.iter().filter(|(_, p)| !p.is_valid()).map(|(n, _)| n.as_str()).collect();
        (!names.is_empty())
            .then(|| self.localizer.tr_with("session-widgets-broken", &[("names", names.join(", ").into())]))
    }
```

  (imports : `oku_slides_core::WidgetFile`, `oku_slides_render::FileProbe`.)

`templates_mode.rs:175-178` :

```rust
        let widgets = base.map(|f| f.widgets().clone()).unwrap_or_default();
        let (id, parent) = if as_new { (Uuid::new_v4(), None) } else { (edit.set, edit.parent.clone()) };
        let manifest = self.edited_manifest(&deck, id, parent.clone(), description);
        let files = manifest.to_files(widgets).context(PackJsonSnafu { id })?;
```

  Fluent, `session.ftl` après `templates-adopt-label` (`:36`) :
  - fr : `session-widgets-broken = Widgets écartés, ils ne compilent pas : { $names }. Les autres widgets du pack restent.`
  - en : `session-widgets-broken = Widgets left out because they do not compile: { $names }. The pack's other widgets remain.`
  - traduire les 18 autres.

- [ ] **Step 5 :** `cargo nextest run -p oku-slides-session -p oku-slides-i18n` → PASS ; `cargo build -p oku-slides-mcp -p oku-slides-agent -p oku-slides-tauri` → OK (`cargo build --workspace` échoue encore sur `oku-slides-cli`).

- [ ] **Step 6 : commit**

```bash
git add crates/oku-slides-session crates/oku-slides-i18n
```
```bash
git commit -m "feat(session)! : bibliothèque, reprise et rendu des packs à plusieurs fichiers de widgets (t-329)"
```

---

### Task 6 : CLI — rendu par fichier et `open --out` en `widgets/<nom>.typ`

**Files:**
- Modify: `crates/oku-slides-cli/src/pack_render.rs` (doc `:1-3`, `render` `:61-93`)
- Modify: `crates/oku-slides-cli/src/main.rs` (`:22` imports, `beside_deck` `:193-213`, extraction `:348-354`)
- Modify: `crates/oku-slides-cli/tests/cli.rs` (`:372-405` constantes et aides, `:550`, `:569`)

**Interfaces:**
- Consumes : `RenderWorker::probe_pack`, `PackProbe` (Task 4) ; `PackFiles::from_bytes` à fichiers (Task 1).
- Produces : `open --out <dir>` écrit `packs/<hex>/pack.json` et `packs/<hex>/widgets/<nom>.typ` ; `save` relit ce dossier (`beside_deck`) ; `render`/`save` rendent les widgets valides d'un pack partiellement cassé (les diagnostics des autres au journal).

- [ ] **Step 1 : tests en échec** — `tests/cli.rs` : `QUOTE` (`:372`) et `LOOP_ON_RENDER` en fichiers (`quote`, `boucle`, règle de la Task 3) ; `pack_files(id, widgets: &[(&str, &str)])` construit `BTreeMap<WidgetFile, String>` ; `user_pack(dir, widgets: &[(&str, &str)])` ; appels `&[("quote", QUOTE)]`, `&[("boucle", LOOP_ON_RENDER)]`. `:550` : `assert_that!(pack_dir.join("widgets").join("quote.typ").exists()).is_true();` ; `:569` : `let widgets = extract.join("packs").join(r.version.hex()).join("widgets").join("quote.typ");`. Ajouter :

```rust
/// t-329 : un widget cassé n'empêche pas `save` ni `render` de rendre les autres.
#[test]
fn save_of_a_deck_whose_pack_has_a_broken_widget_renders_the_others() {
    let dir = workdir("widget-casse");
    let r = user_pack(&dir, &[("quote", QUOTE), ("casse", "#let widget = (")]);
    let deck = dir.join("deck.json");
    let json = serde_json::json!({ "version": 3, "templates": r,
        "slides": [{ "id": "s", "kind": "content", "params": { "title": "T", "blocks": [
            { "id": "q", "kind": "quote", "params": { "body": [{ "runs": [{ "t": "Tout coule." }] }], "source": "H" } }
        ] } }] });
    std::fs::write(&deck, serde_json::to_vec(&json).unwrap()).unwrap();
    let pdf = dir.join("deck.oku-slides.pdf");
    let saved = run(oku_slides(&dir, "en").arg("save").arg(&deck).arg(&pdf));
    assert_that!(saved.status.success()).is_true();
    assert_that!(stderr(&saved)).contains("casse");
    let out = dir.join("out");
    assert_that!(run(oku_slides(&dir, "en").arg("render").arg(&pdf).arg("--out").arg(&out)).status.success()).is_true();
}
```

  (Même forme que `save_embeds_the_referenced_pack_and_renders_its_widgets`, `:434` ; `stderr` contient le diagnostic journalisé du widget `casse`.)

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-cli` → FAIL à la compilation (`files.widgets()` n'est plus une `Option<&str>`).

- [ ] **Step 3 : implémentation.** `pack_render.rs` doc : « … quand le pack qu'il embarque a des widgets : chaque fichier y est sondé à part, les widgets cassés écartés (diagnostics au journal). » ; `render` `:79-90` :

```rust
    if files.widgets().is_empty() {
        return local(l, fonts, bundle, deck);
    }
    let worker = RenderWorker::new(platform.render_launch.clone(), platform.settings.render.clone());
    let probed = PackWidgets::new(r.version.hex(), files.widgets().clone()).and_then(|pack| worker.probe_pack(&pack));
    let probe = match probed {
        Ok(probe) => probe,
        Err(e) => {
            tracing::warn!(pack = %r.id, error = %one_line(&e), "{}", e.localize(l));
            return local(l, fonts, bundle, deck);
        }
    };
    for (name, file) in probe.files.iter().filter(|(_, p)| !p.is_valid()) {
        for d in &file.diagnostics {
            tracing::warn!(pack = %r.id, widget = %name, "{}", d.localize(l));
        }
    }
    let Some(pack) = probe.pack else { return local(l, fonts, bundle, deck) };
    let mut slot = worker.slot(&fonts, l, probe.registry.clone())?;
    slot.set_widgets(Some(&pack), probe.registry)?;
```

  `main.rs` : import `WidgetFile` (`:22`) ; `beside_deck` :

```rust
/// La version `v` extraite à côté du deck, si elle s'y trouve intacte.
fn beside_deck(storage: &Storage, deck_dir: &Path, v: &PackVersion) -> Option<PackFiles> {
    let dir = deck_dir.join("packs").join(v.hex());
    let read = |path: &Path| -> Option<Vec<u8>> {
        match storage.len(path) {
            Ok(Some(_)) => storage
                .read(path)
                .inspect_err(|e| tracing::warn!(path = %path.display(), error = %e, "fichier du pack illisible"))
                .ok(),
            _ => None,
        }
    };
    let mut widgets = BTreeMap::new();
    let listed = storage.list_files(&dir.join("widgets")).unwrap_or_default();
    for path in listed {
        let Some(name) = path.file_name().and_then(|n| n.to_str()).and_then(WidgetFile::from_file_name) else {
            continue;
        };
        widgets.insert(name, read(&path)?);
    }
    let files =
        PackFiles::from_bytes(&read(&dir.join("pack.json"))?, widgets.iter().map(|(n, b)| (n.clone(), b.as_slice())))?;
    if files.matches(v) {
        Some(files)
    } else {
        tracing::warn!(dir = %dir.display(), "pack altéré à côté du deck : ignoré");
        None
    }
}
```

  extraction `:350-353` :

```rust
                    for (name, source) in files.widgets() {
                        storage.write(&at.join("widgets").join(name.file_name()), source.as_bytes())?;
                    }
```

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-cli` → PASS ; `cargo build --workspace` → OK.

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-cli
```
```bash
git commit -m "feat(cli) : rendu des widgets valides d'un pack et extraction en widgets/<nom>.typ (t-329)"
```

---

### Task 7 : statut et modification des widgets du pack en cours (session, mode modèles)

**Files:**
- Create: `crates/oku-slides-session/src/widgets_edit.rs`, `crates/oku-slides-session/src/widgets_edit_tests.rs`
- Modify: `crates/oku-slides-session/src/templates_mode.rs` (`adopt_merge_key` `:34`, `styled_like` `:57` → `pub(crate)` ; `save_templates` `:159-217` : fin sortie dans `adopt_session_version`)
- Modify: `crates/oku-slides-session/src/request.rs` (variantes, `changes_deck`, `edits`, `is_widget_edit`)
- Modify: `crates/oku-slides-session/src/lib.rs` (`mod widgets_edit;`, réexports, champs `widget_limits` et `widget_edit` de `Session` + init dans `Session::new`, bras de `handle`)
- Modify: `crates/oku-slides-session/src/deck_widgets.rs` (`broken_widgets_notice` se tait pendant `widget_edit`)
- Modify: `crates/oku-slides-session/src/error.rs` (variantes, kinds, `localize`)
- Modify: `crates/oku-slides-session/src/lib_tests.rs:136-180`
- Modify: `crates/oku-slides-i18n/src/locales/*/session.ftl` (20 langues)

**Interfaces:**
- Consumes : `RenderWorker::probe_pack` (Task 4), `Diagnostic::{widget_file, at}` (Task 3), `RenderSettings::{max_widget_kib, max_widgets}` (Task 2).
- Produces :
  - `pub struct WidgetDiagnostic { pub file: Option<WidgetFile>, pub line: Option<usize>, pub column: Option<usize>, pub error: bool, pub message: String }` (`Serialize`).
  - `pub struct WidgetStatus { pub name: WidgetFile, pub schema: Option<WidgetSchema>, pub diagnostics: Vec<WidgetDiagnostic> }`, `WidgetStatus::is_valid(&self) -> bool`.
  - `pub struct WidgetChange { pub status: WidgetStatus, pub broken: Vec<WidgetStatus> }`.
  - `Session::widget_status(&self) -> Result<Vec<WidgetStatus>>`, `widget_source(&self, &str) -> Result<String>`, `set_widget(&mut self, &str, String) -> Result<WidgetChange>`, `remove_widget(&mut self, &str) -> Result<Vec<WidgetStatus>>`, `rename_widget(&mut self, &str, &str) -> Result<Vec<WidgetStatus>>`.
  - `pub(crate) struct WidgetTarget { id: Uuid, version: PackVersion, files: PackFiles }`, `Session::widget_target(&self) -> Result<WidgetTarget>`, `Session::statuses(&self, &PackVersion, &PackFiles) -> Result<Vec<WidgetStatus>>`, `widgets_edit::parse_name(&str) -> Result<WidgetFile>`, `WidgetDiagnostic::of(&Diagnostic) -> WidgetDiagnostic`.
  - `Session::adopt_session_version(&mut self, Deck, PackFiles, &TemplateMode, bool) -> Result<PackVersion>` (`pub(crate)`), partagé par `save_templates` et les widgets.
  - `Request::{WidgetStatus, WidgetSource { name }, SetWidget { name, source }, RemoveWidget { name }, RenameWidget { from, to }}` (`cmd` : `widget_status`, `widget_source`, `set_widget`, `remove_widget`, `rename_widget`) ; `Request::is_widget_edit(&self) -> bool`.
  - `SessionErrorKind::{InvalidWidget, WidgetInUse, UnknownWidget}`.
  - Mode deck : livré à la Task 8 (`commit_deck_widgets`). D'ici là, une écriture en mode deck rend `NotEditingTemplates` (bras provisoire de `commit_files`, remplacé à la Task 8) ; les lectures (`widget_status`, `widget_source`) marchent déjà dans les deux modes.

- [ ] **Step 1 : tests en échec** — `widgets_edit_tests.rs` :

```rust
use speculoos::prelude::*;

use crate::pack::DEFAULT_PACK;
use crate::{Request, Session, SessionErrorKind, Settings};

const KPI: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("kpi", params: (value: p.plain()), (args, ctx) => field(ctx, "value", args.value))
"#;

const KPI_V2: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => field(ctx, "value", args.value))
"#;

const COMMON: &str = "#let cadre(body) = block(inset: 8pt, body)\n";

const FRAMED: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#import "_commun.typ": cadre
#let widget = widget("cadre", params: (value: p.plain()), (args, ctx) => cadre(field(ctx, "value", args.value)))
"#;

/// Une session sur un dossier neuf, en édition du pack par défaut.
fn editing(data: &std::path::Path) -> Session {
    let mut s = crate::test_support::session_on(data);
    s.edit_templates(DEFAULT_PACK).unwrap();
    s
}

/// Une session dont les fichiers de widgets sont bornés.
fn bounded(data: &std::path::Path, max_widget_kib: usize, max_widgets: usize) -> Session {
    let mut settings = Settings::defaults().unwrap();
    settings.paths.data = Some(data.to_owned());
    settings.paths.decks = Some(data.join("decks"));
    settings.i18n.locale = Some("fr".into());
    settings.render.max_widget_kib = max_widget_kib;
    settings.render.max_widgets = max_widgets;
    let platform = crate::Platform::with_settings(settings).unwrap();
    let mut s = Session::new(crate::Fonts::default(), &platform).unwrap();
    s.edit_templates(DEFAULT_PACK).unwrap();
    s
}

#[test]
fn set_widget_twice_then_save_makes_one_version_holding_the_file() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget("kpi", KPI.into()).unwrap();
    s.set_widget("kpi", KPI_V2.into()).unwrap();
    s.save_templates(false).unwrap();
    let lib = crate::test_support::library_of(&s);
    // L'intégrée, puis la seule version de la séance.
    assert_that!(lib.versions(DEFAULT_PACK).len()).is_equal_to(2);
    let current = lib.current_version(DEFAULT_PACK).unwrap();
    let kpi = oku_slides_core::WidgetFile::parse("kpi").unwrap();
    assert_that!(current.files.widget(&kpi)).is_equal_to(Some(KPI_V2));
    assert_that!(current.manifest.parent.is_some()).is_true();
}

#[test]
fn set_widget_returns_the_status_with_its_schema() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    let change = s.set_widget("kpi", KPI.into()).unwrap();
    assert_that!(change.status.is_valid()).is_true();
    assert_that!(change.status.schema.as_ref().map(|w| w.params.len())).is_equal_to(Some(1));
    assert_that!(change.broken).is_empty();
    let reply: serde_json::Value = serde_json::from_str(&s.handle(Request::Widgets).unwrap()).unwrap();
    assert_that!(reply["widgets"].get("kpi").is_some()).is_true();
}

#[test]
fn set_widget_on_common_reports_the_widgets_it_broke() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget("_commun", COMMON.into()).unwrap();
    assert_that!(s.set_widget("cadre", FRAMED.into()).unwrap().status.is_valid()).is_true();
    let change = s.set_widget("_commun", "#let cadre = (".into()).unwrap();
    let broken: Vec<&str> = change.broken.iter().map(|b| b.name.as_str()).collect();
    assert_that!(broken).is_equal_to(vec!["cadre"]);
    assert_that!(change.status.diagnostics.iter().any(|d| d.error && d.line == Some(1))).is_true();
}

#[test]
fn widget_status_lists_common_first() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget("kpi", KPI.into()).unwrap();
    s.set_widget("_commun", COMMON.into()).unwrap();
    let names: Vec<String> = s.widget_status().unwrap().into_iter().map(|w| w.name.to_string()).collect();
    assert_that!(names).is_equal_to(vec!["_commun".to_owned(), "kpi".to_owned()]);
    assert_that!(s.widget_source("kpi").unwrap()).is_equal_to(KPI.to_owned());
}

#[test]
fn remove_widget_used_by_a_template_is_refused_with_the_template_names() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget("kpi", KPI.into()).unwrap();
    let slide = serde_json::json!({
        "op": "add_slide", "index": 0,
        "slide": { "id": "t-kpi", "kind": "blank", "name": "Indicateur", "params": { "elements": [
            { "id": "k", "kind": "kpi", "cell": { "col": 0, "row": 0, "cols": 4, "rows": 2 }, "params": { "value": "1" } }
        ] } }
    });
    s.apply("Modèle", vec![serde_json::from_value(slide).unwrap()], None).unwrap();
    let err = s.remove_widget("kpi").unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::WidgetInUse);
    assert_that!(err.one_line()).contains("Indicateur");
}

#[test]
fn set_widget_with_an_empty_source_removes_the_file() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget("kpi", KPI.into()).unwrap();
    s.set_widget("kpi", String::new()).unwrap();
    assert_that!(s.widget_status().unwrap()).is_empty();
}

#[test]
fn rename_widget_renames_the_file_and_its_declared_name() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = editing(data.path());
    s.set_widget("kpi", KPI.into()).unwrap();
    let statuses = s.rename_widget("kpi", "indicateur").unwrap();
    assert_that!(statuses.iter().map(|w| w.name.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["indicateur"]);
    assert_that!(statuses[0].is_valid()).is_true();
    assert_that!(s.widget_source("indicateur").unwrap().as_str()).contains("widget(\"indicateur\"");
}

#[test]
fn widget_operations_refuse_bad_names_unknown_files_and_limits() {
    let data = crate::test_support::fresh_dir("oku-slides-widgets-");
    let mut s = bounded(data.path(), 1, 1);
    assert_that!(s.set_widget("Kpi", KPI.into()).unwrap_err().kind()).is_equal_to(SessionErrorKind::InvalidWidget);
    assert_that!(s.widget_source("absent").unwrap_err().kind()).is_equal_to(SessionErrorKind::UnknownWidget);
    assert_that!(s.set_widget("kpi", "x".repeat(2048)).unwrap_err().kind()).is_equal_to(SessionErrorKind::InvalidWidget);
    s.set_widget("kpi", KPI.into()).unwrap();
    assert_that!(s.set_widget("autre", KPI.into()).unwrap_err().kind()).is_equal_to(SessionErrorKind::InvalidWidget);
    assert_that!(s.rename_widget("kpi", "_commun").unwrap_err().kind()).is_equal_to(SessionErrorKind::InvalidWidget);
}
```

  `lib_tests.rs:136-180` : `reads` gagne `json!({ "cmd": "widget_status" })`, `json!({ "cmd": "widget_source", "name": "kpi" })` ; `edits` gagne `json!({ "cmd": "set_widget", "name": "kpi", "source": "x" })`, `json!({ "cmd": "remove_widget", "name": "kpi" })`, `json!({ "cmd": "rename_widget", "from": "kpi", "to": "k" })`.

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-session widgets_edit lib_tests` → FAIL à la compilation.

- [ ] **Step 3 : implémentation.**

`error.rs` — `SessionErrorKind`, après `InHistory` :

```rust
    /// Nom, taille ou nombre de fichiers de widgets refusé.
    InvalidWidget,
    /// Widget utilisé par un modèle du pack.
    WidgetInUse,
    /// Fichier de widget introuvable.
    UnknownWidget,
```

  `SessionInner`, après `Version` :

```rust
    #[snafu(display("invalid widget name: {name}"))]
    InvalidWidgetName { name: String, backtrace: Backtrace },
    #[snafu(display("widget {name} is larger than {max} KiB"))]
    WidgetTooLarge { name: String, max: usize, backtrace: Backtrace },
    #[snafu(display("a pack holds at most {max} widget files"))]
    TooManyWidgets { max: usize, backtrace: Backtrace },
    #[snafu(display("widget {name} is used by templates: {}", templates.join(", ")))]
    WidgetInUse { name: String, templates: Vec<String>, backtrace: Backtrace },
    #[snafu(display("widget not found: {name}"))]
    UnknownWidget { name: String, backtrace: Backtrace },
    #[snafu(display("widget already exists: {name}"))]
    WidgetExists { name: String, backtrace: Backtrace },
```

  `kind` : `InvalidWidgetName { .. } | WidgetTooLarge { .. } | TooManyWidgets { .. } | WidgetExists { .. } => SessionErrorKind::InvalidWidget`, `WidgetInUse { .. } => SessionErrorKind::WidgetInUse`, `UnknownWidget { .. } => SessionErrorKind::UnknownWidget`. `localize` :

```rust
            InvalidWidgetName { name, .. } => l.tr_with("session-invalid-widget-name", &[("name", name.as_str().into())]),
            WidgetTooLarge { name, max, .. } => {
                l.tr_with("session-widget-too-large", &[("name", name.as_str().into()), ("max", (*max).into())])
            }
            TooManyWidgets { max, .. } => l.tr_with("session-too-many-widgets", &[("max", (*max).into())]),
            WidgetInUse { name, templates, .. } => l.tr_with(
                "session-widget-in-use",
                &[("name", name.as_str().into()), ("templates", templates.join(", ").into())],
            ),
            UnknownWidget { name, .. } => l.tr_with("session-unknown-widget", &[("name", name.as_str().into())]),
            WidgetExists { name, .. } => l.tr_with("session-widget-exists", &[("name", name.as_str().into())]),
```

  (Si `usize` n'a pas de `Into<FluentValue>`, passer `(*max as u64).into()` comme les autres compteurs du fichier.)

`templates_mode.rs` : `fn adopt_merge_key` et `fn styled_like` deviennent `pub(crate)` ; `save_templates` (`:159-217`) :

```rust
    pub fn save_templates(&mut self, as_new: bool) -> Result<()> {
        let Mode::Templates { edit, .. } = &self.mode else {
            return NotEditingTemplatesSnafu.fail()?;
        };
        let edit = edit.clone();
        // Le deck modifié (titre de la copie, version) remplace la vue d'un
        // coup, sans étape d'annulation : ce n'est pas une édition.
        let mut deck = self.doc.sources.deck().clone();
        if as_new && deck.params.title.as_deref() == Some(edit.name.as_str()) {
            let name = self.localizer.tr_with("templates-copy-name", &[("name", edit.name.as_str().into())]);
            deck.params.title = Some(name);
        }
        // Description et widgets : ceux de la dernière version enregistrée.
        let base = self.doc.sources.packs.get(&edit.base);
        let description =
            base.and_then(|f| PackManifest::from_files(f).ok()).map(|m| m.description).unwrap_or_default();
        let widgets = base.map(|f| f.widgets().clone()).unwrap_or_default();
        let (id, parent) = if as_new { (Uuid::new_v4(), None) } else { (edit.set, edit.parent.clone()) };
        let manifest = self.edited_manifest(&deck, id, parent, description);
        let files = manifest.to_files(widgets).context(PackJsonSnafu { id })?;
        self.adopt_session_version(deck, files, &edit, as_new)?;
        self.doc.dirty = false;
        Ok(())
    }

    /// `files` devient la version de la séance d'édition `edit`.
    ///
    /// Créée la première fois (ou toujours, pour une copie `as_new`),
    /// remplacée ensuite, rendue courante ; `deck` (le document des modèles)
    /// et le deck mis de côté y passent, celui-ci en une étape d'annulation.
    pub(crate) fn adopt_session_version(
        &mut self,
        mut deck: Deck,
        files: PackFiles,
        edit: &TemplateMode,
        as_new: bool,
    ) -> Result<PackVersion> {
        let manifest = PackManifest::from_files(&files).context(PackJsonSnafu { id: edit.set })?;
        let (id, parent) = (manifest.id, manifest.parent.clone());
        // Une version par séance : la première est créée, les suivantes la remplacent.
        let version = match edit.session_version.as_ref().filter(|_| !as_new) {
            Some(old) => self.library.replace_version(id, old, &files)?,
            None => {
                let version = files.version();
                self.library.add_version(id, &files)?;
                self.library.set_current(id, &version)?;
                version
            }
        };
        let r = PackRef { id, version: version.clone() };
        // Le document des modèles passe à la nouvelle version, sans étape d'annulation.
        self.doc.sources.add_pack(files.clone());
        deck.templates = Some(r.clone());
        self.doc.sources.apply_silent(deck).context(SourcesSnafu)?;
        let label = self.localizer.tr_with("templates-adopt-label", &[("name", manifest.name.as_str().into())]);
        let merge = adopt_merge_key(id, parent.as_ref());
        if let Mode::Templates { stash, edit } = &mut self.mode {
            // Le deck mis de côté embarque la version et y passe, en une étape d'annulation.
            stash.sources.add_pack(files);
            let mut user = stash.sources.deck().clone();
            user.templates = Some(r);
            stash.sources.apply(user, &label, Some(&merge)).context(SourcesSnafu)?;
            stash.dirty = true;
            *edit = TemplateMode {
                set: id,
                name: manifest.name,
                builtin: Library::is_builtin(id),
                base: version.clone(),
                parent,
                session_version: Some(version.clone()),
            };
        }
        self.sync_widgets()?;
        self.template_cache = None;
        self.rerender()?;
        Ok(version)
    }
```

  (Le seul changement de comportement : `doc.dirty = false` après `rerender`, qui ne le lit pas.)

`widgets_edit.rs` (nouveau) :

```rust
//! Widgets du pack en cours : statut, source et modifications.
//!
//! Le pack en cours : en mode modèles, la dernière version enregistrée du
//! pack édité (`TemplateMode::base`) ; sinon celle qu'embarque le deck.
//! Chaque modification produit une version selon la règle de t-328 (une
//! version par séance) : `adopt_session_version` en mode modèles,
//! `commit_deck_widgets` (`widgets_deck.rs`) en mode deck.
//!
//! Limites (`[render]`) : `max_widget_kib` par fichier, `max_widgets`
//! fichiers par version, `_commun` compris. Un fichier vide vaut absent :
//! l'écrire le supprime.

use std::collections::BTreeMap;

use serde::Serialize;
use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::{PackFiles, PackRef, PackVersion, Slide, WidgetFile, WidgetRegistry, WidgetSchema};
use oku_slides_platform::config::RenderSettings;
use oku_slides_render::{Diagnostic, FileProbe, PackWidgets, Severity};

use crate::error::{
    InHistorySnafu, InvalidWidgetNameSnafu, NotEditingTemplatesSnafu, PackJsonSnafu, PackVersionMissingSnafu,
    RegistrySnafu, TooManyWidgetsSnafu, UnknownPackSnafu, UnknownWidgetSnafu, WidgetExistsSnafu, WidgetInUseSnafu,
    WidgetTooLargeSnafu,
};
use crate::pack::{DEFAULT_PACK, PackManifest, now_created};
use crate::{Mode, Result, Session};

/// Bornes des fichiers de widgets (`[render]`).
#[derive(Debug, Clone, Copy)]
pub(crate) struct WidgetLimits {
    max_bytes: usize,
    max_kib: usize,
    max_files: usize,
}

impl WidgetLimits {
    pub(crate) fn of(r: &RenderSettings) -> Self {
        Self { max_bytes: r.max_widget_bytes(), max_kib: r.max_widget_kib, max_files: r.max_widgets }
    }
}

/// Un message de Typst, ou du contrôle, sur un fichier de widget.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WidgetDiagnostic {
    /// Fichier du pack où il se trouve ; `None` hors du pack (bibliothèque).
    pub file: Option<WidgetFile>,
    /// Ligne et colonne, à partir de 1, si elles sont connues.
    pub line: Option<usize>,
    pub column: Option<usize>,
    /// Une erreur (sinon un avertissement).
    pub error: bool,
    /// Le message (en anglais, comme ceux de Typst) et ses indices.
    pub message: String,
}

impl WidgetDiagnostic {
    /// Depuis un diagnostic du rendu.
    pub(crate) fn of(d: &Diagnostic) -> Self {
        let mut message = d.message.clone();
        for hint in &d.hints {
            message.push('\n');
            message.push_str(hint);
        }
        Self {
            file: d.widget_file(),
            line: d.at.as_ref().map(|a| a.line),
            column: d.at.as_ref().map(|a| a.column),
            error: d.severity == Severity::Error,
            message,
        }
    }
}

/// Un fichier du pack et son statut.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WidgetStatus {
    pub name: WidgetFile,
    /// Le schéma du widget, s'il est valide ; `None` pour `_commun`.
    pub schema: Option<WidgetSchema>,
    pub diagnostics: Vec<WidgetDiagnostic>,
}

impl WidgetStatus {
    /// Valide : aucune erreur.
    pub fn is_valid(&self) -> bool {
        !self.diagnostics.iter().any(|d| d.error)
    }
}

/// Ce que rend une écriture : le fichier, et les widgets qu'elle a cassés.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WidgetChange {
    pub status: WidgetStatus,
    /// Widgets valides avant, cassés après (un `_commun` modifié).
    pub broken: Vec<WidgetStatus>,
}

/// Le pack dont on édite les widgets : id, version et fichiers.
pub(crate) struct WidgetTarget {
    pub(crate) id: Uuid,
    pub(crate) version: PackVersion,
    pub(crate) files: PackFiles,
}

/// Lit un nom de fichier de widget (`InvalidWidgetName` sinon).
pub(crate) fn parse_name(name: &str) -> Result<WidgetFile> {
    Ok(WidgetFile::parse(name).context(InvalidWidgetNameSnafu { name })?)
}

/// Le statut d'un fichier sondé, schéma lu dans le registre assemblé.
fn status_of(name: &WidgetFile, probe: &FileProbe, registry: &WidgetRegistry) -> WidgetStatus {
    let schema = (probe.is_valid() && !name.is_common()).then(|| registry.get(name.as_str()).cloned()).flatten();
    WidgetStatus { name: name.clone(), schema, diagnostics: probe.diagnostics.iter().map(WidgetDiagnostic::of).collect() }
}

impl Session {
    /// Le pack dont on édite les widgets.
    pub(crate) fn widget_target(&self) -> Result<WidgetTarget> {
        let (r, sources) = match &self.mode {
            Mode::Templates { edit, .. } => {
                (PackRef { id: edit.set, version: edit.base.clone() }, &self.doc.sources)
            }
            Mode::Deck | Mode::History { .. } => (
                self.deck_pack().cloned().context(UnknownPackSnafu { id: DEFAULT_PACK })?,
                self.user_sources(),
            ),
        };
        let files = sources
            .packs
            .get(&r.version)
            .cloned()
            .context(PackVersionMissingSnafu { id: r.id, version: r.version.clone() })?;
        Ok(WidgetTarget { id: r.id, version: r.version, files })
    }

    /// Statut de chaque fichier de `files` (version `version`), dans l'ordre des noms.
    pub(crate) fn statuses(&self, version: &PackVersion, files: &PackFiles) -> Result<Vec<WidgetStatus>> {
        if files.widgets().is_empty() {
            return Ok(Vec::new());
        }
        let pack = PackWidgets::new(version.hex(), files.widgets().clone()).context(RegistrySnafu)?;
        let probe = self.worker.probe_pack(&pack).context(RegistrySnafu)?;
        Ok(probe.files.iter().map(|(name, p)| status_of(name, p, &probe.registry)).collect())
    }

    /// Statut des fichiers du pack en cours, `_commun` en tête.
    pub fn widget_status(&self) -> Result<Vec<WidgetStatus>> {
        let t = self.widget_target()?;
        self.statuses(&t.version, &t.files)
    }

    /// Source du fichier `name` du pack en cours.
    pub fn widget_source(&self, name: &str) -> Result<String> {
        let name = parse_name(name)?;
        let t = self.widget_target()?;
        Ok(t.files.widget(&name).context(UnknownWidgetSnafu { name: name.as_str() })?.to_owned())
    }

    /// Crée ou remplace le fichier `name` ; une source vide le supprime.
    ///
    /// Rend son statut et celui des widgets que l'écriture a cassés.
    pub fn set_widget(&mut self, name: &str, source: String) -> Result<WidgetChange> {
        let name = parse_name(name)?;
        let t = self.widget_target()?;
        let mut widgets = t.files.widgets().clone();
        if source.is_empty() {
            ensure!(widgets.contains_key(&name), UnknownWidgetSnafu { name: name.as_str() });
            self.ensure_unused(&t, &name)?;
            widgets.remove(&name);
        } else {
            let limits = self.widget_limits;
            ensure!(source.len() <= limits.max_bytes, WidgetTooLargeSnafu { name: name.as_str(), max: limits.max_kib });
            ensure!(
                widgets.contains_key(&name) || widgets.len() < limits.max_files,
                TooManyWidgetsSnafu { max: limits.max_files }
            );
            widgets.insert(name.clone(), source);
        }
        let before = self.statuses(&t.version, &t.files)?;
        let after = self.commit_widgets(&t, widgets)?;
        let status = after
            .iter()
            .find(|s| s.name == name)
            .cloned()
            .unwrap_or_else(|| WidgetStatus { name: name.clone(), schema: None, diagnostics: Vec::new() });
        let was_valid = |n: &WidgetFile| before.iter().any(|b| b.name == *n && b.is_valid());
        let broken = after.into_iter().filter(|s| s.name != name && !s.is_valid() && was_valid(&s.name)).collect();
        Ok(WidgetChange { status, broken })
    }

    /// Supprime le fichier `name` ; refusé si un modèle du pack l'utilise.
    pub fn remove_widget(&mut self, name: &str) -> Result<Vec<WidgetStatus>> {
        let name = parse_name(name)?;
        let t = self.widget_target()?;
        ensure!(t.files.widget(&name).is_some(), UnknownWidgetSnafu { name: name.as_str() });
        self.ensure_unused(&t, &name)?;
        let mut widgets = t.files.widgets().clone();
        widgets.remove(&name);
        self.commit_widgets(&t, widgets)
    }

    /// Renomme `from` en `to` dans la version éditée ; le nom déclaré suit.
    ///
    /// Refusé pour `_commun`, vers un nom pris, ou si un modèle utilise `from`.
    pub fn rename_widget(&mut self, from: &str, to: &str) -> Result<Vec<WidgetStatus>> {
        let (from, to) = (parse_name(from)?, parse_name(to)?);
        ensure!(!from.is_common(), InvalidWidgetNameSnafu { name: from.as_str() });
        ensure!(!to.is_common(), InvalidWidgetNameSnafu { name: to.as_str() });
        let t = self.widget_target()?;
        let source = t.files.widget(&from).context(UnknownWidgetSnafu { name: from.as_str() })?;
        ensure!(t.files.widget(&to).is_none(), WidgetExistsSnafu { name: to.as_str() });
        self.ensure_unused(&t, &from)?;
        let renamed = source.replacen(&format!("widget(\"{from}\""), &format!("widget(\"{to}\""), 1);
        let mut widgets = t.files.widgets().clone();
        widgets.remove(&from);
        widgets.insert(to, renamed);
        self.commit_widgets(&t, widgets)
    }

    /// Refuse si un modèle du pack (enregistré, ou en cours d'édition) utilise `name`.
    fn ensure_unused(&self, t: &WidgetTarget, name: &WidgetFile) -> Result<()> {
        let uses = |s: &Slide| s.kind == name.as_str() || s.walk().any(|l| l.element.kind == name.as_str());
        let manifest = PackManifest::from_files(&t.files).context(PackJsonSnafu { id: t.id })?;
        let mut templates: Vec<String> =
            manifest.templates.iter().filter(|tpl| uses(&tpl.slide)).map(|tpl| tpl.name.clone()).collect();
        if matches!(self.mode, Mode::Templates { .. }) {
            for s in self.doc.sources.deck().slides.iter().filter(|s| uses(s)) {
                let shown = s.name.clone().unwrap_or_else(|| s.id.clone());
                if !templates.contains(&shown) {
                    templates.push(shown);
                }
            }
        }
        ensure!(templates.is_empty(), WidgetInUseSnafu { name: name.as_str(), templates });
        Ok(())
    }

    /// Enregistre `widgets` dans la version de la séance ; les statuts d'après.
    ///
    /// L'avis des widgets écartés se tait pendant l'écriture : le panneau et
    /// l'agent ont les statuts.
    fn commit_widgets(&mut self, t: &WidgetTarget, widgets: BTreeMap<WidgetFile, String>) -> Result<Vec<WidgetStatus>> {
        ensure!(!matches!(self.mode, Mode::History { .. }), InHistorySnafu);
        let mut manifest = PackManifest::from_files(&t.files).context(PackJsonSnafu { id: t.id })?;
        manifest.created = now_created();
        self.widget_edit = true;
        let committed = self.commit_files(t, manifest, widgets);
        self.widget_edit = false;
        committed?;
        self.widget_status()
    }

    /// La version produite, selon le mode.
    fn commit_files(
        &mut self,
        t: &WidgetTarget,
        mut manifest: PackManifest,
        widgets: BTreeMap<WidgetFile, String>,
    ) -> Result<()> {
        match &self.mode {
            Mode::Templates { edit, .. } => {
                let edit = edit.clone();
                manifest.parent = edit.parent.clone();
                let files = manifest.to_files(widgets).context(PackJsonSnafu { id: t.id })?;
                let deck = self.doc.sources.deck().clone();
                self.adopt_session_version(deck, files, &edit, false)?;
                Ok(())
            }
            // Livré par la Task 8 (`widgets_deck.rs`).
            Mode::Deck | Mode::History { .. } => NotEditingTemplatesSnafu.fail()?,
        }
    }
}

#[cfg(test)]
#[path = "widgets_edit_tests.rs"]
mod widgets_edit_tests;
```

`deck_widgets.rs` — `broken_widgets_notice` commence par `if self.widget_edit { return None; }`.

`request.rs` — variantes, après `PackNoticeClose` :

```rust
    /// Statut des fichiers de widgets du pack en cours (tableau de `WidgetStatus`).
    WidgetStatus,
    /// Source d'un fichier de widget du pack en cours.
    WidgetSource {
        name: String,
    },
    /// Crée ou remplace un fichier de widget ; une source vide le supprime (`WidgetChange`).
    SetWidget {
        name: String,
        source: String,
    },
    /// Supprime un fichier de widget ; refusé s'il est utilisé par un modèle.
    RemoveWidget {
        name: String,
    },
    /// Renomme un fichier de widget dans la version éditée.
    RenameWidget {
        from: String,
        to: String,
    },
```

  `changes_deck` : `SetWidget | RemoveWidget | RenameWidget` → `true` ; `WidgetStatus | WidgetSource` → `false`. `edits` : ajoute `Self::SetWidget { .. } | Self::RemoveWidget { .. } | Self::RenameWidget { .. }`. Et :

```rust
    /// Vrai pour une modification de widget : elle ne termine pas la séance des widgets.
    pub(crate) fn is_widget_edit(&self) -> bool {
        matches!(self, Self::SetWidget { .. } | Self::RemoveWidget { .. } | Self::RenameWidget { .. })
    }
```

`lib.rs` : `mod widgets_edit;` ; `pub use widgets_edit::{WidgetChange, WidgetDiagnostic, WidgetStatus};` ; champs de `Session` après `pack_notices_closed` :

```rust
    /// Bornes des fichiers de widgets (`[render]`).
    widget_limits: widgets_edit::WidgetLimits,
    /// Une écriture de widgets est en cours : l'avis des widgets écartés se tait.
    widget_edit: bool,
```

  init dans `Session::new` : `widget_limits: widgets_edit::WidgetLimits::of(&settings.render), widget_edit: false,` ; bras de `handle`, après `Request::TemplateSets` :

```rust
            Request::WidgetStatus => return json(&self.widget_status()?),
            Request::WidgetSource { name } => return json(&self.widget_source(&name)?),
            Request::SetWidget { name, source } => {
                let change = self.set_widget(&name, source)?;
                self.persist(true);
                return json(&change);
            }
            Request::RemoveWidget { name } => {
                let statuses = self.remove_widget(&name)?;
                self.persist(true);
                return json(&statuses);
            }
            Request::RenameWidget { from, to } => {
                let statuses = self.rename_widget(&from, &to)?;
                self.persist(true);
                return json(&statuses);
            }
```

  Fluent, `session.ftl` après `session-widgets-broken` :
  - fr :
    ```
    session-invalid-widget-name = Nom de widget invalide : « { $name } ». Lettres minuscules, chiffres et tirets, ou _commun.
    session-widget-too-large = Le widget « { $name } » dépasse { $max } Kio.
    session-too-many-widgets = Un pack ne peut pas avoir plus de { $max } fichiers de widgets.
    session-widget-in-use = Le widget « { $name } » est utilisé par ces modèles : { $templates }.
    session-unknown-widget = Widget introuvable : « { $name } ».
    session-widget-exists = Un widget « { $name } » existe déjà.
    ```
  - en :
    ```
    session-invalid-widget-name = Invalid widget name: “{ $name }”. Use lowercase letters, digits and hyphens, or _commun.
    session-widget-too-large = Widget “{ $name }” is larger than { $max } KiB.
    session-too-many-widgets = A pack cannot have more than { $max } widget files.
    session-widget-in-use = Widget “{ $name }” is used by these templates: { $templates }.
    session-unknown-widget = Widget not found: “{ $name }”.
    session-widget-exists = A widget “{ $name }” already exists.
    ```
  - traduire les 18 autres.

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-session -p oku-slides-i18n` → PASS ; `wc -l crates/oku-slides-session/src/lib.rs` < 900.

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-session crates/oku-slides-i18n
```
```bash
git commit -m "feat(session) : statut et modification des widgets du pack en mode modèles (t-329)"
```

---

### Task 8 : widgets en mode deck, une version par séance

**Files:**
- Create: `crates/oku-slides-session/src/widgets_deck.rs`, `crates/oku-slides-session/src/widgets_deck_tests.rs`
- Modify: `crates/oku-slides-session/src/widgets_edit.rs` (bras `Mode::Deck` de `commit_files`)
- Modify: `crates/oku-slides-session/src/lib.rs` (`mod widgets_deck;`, champ `widget_session`, fin de séance dans `handle`, `undo`, `redo`, `load_document`)
- Modify: `crates/oku-slides-session/src/deck_widgets.rs` (`apply` `:123` : fin de séance)
- Modify: `crates/oku-slides-i18n/src/locales/*/session.ftl` (`widgets-edit-label`)

**Interfaces:**
- Consumes : `WidgetTarget`, `commit_files` (Task 7), `adopt_merge_key` (`pub(crate)`, Task 7), `Library::{add_version, set_current, replace_version}`, `Session::apply`, `pack_notice_for_deck`.
- Produces :
  - `pub(crate) struct WidgetSession { id: Uuid, parent: PackVersion, version: PackVersion }` ; `Session.widget_session: Option<WidgetSession>`.
  - `Session::end_widget_session(&mut self)` (`pub(crate)`) : appelée au début de `apply`, `undo`, `redo`, `load_document`, et par `handle` pour toute requête `changes_deck()` qui n'est pas `is_widget_edit()`.
  - `Session::commit_deck_widgets(&mut self, &WidgetTarget, PackManifest, BTreeMap<WidgetFile, String>) -> Result<()>` (`pub(crate)`).

- [ ] **Step 1 : tests en échec** — `widgets_deck_tests.rs` :

```rust
use oku_slides_core::{Op, WidgetFile};
use serde_json::json;
use speculoos::prelude::*;

use crate::pack::PackManifest;

const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
"#;

const QUOTE_V2: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", strong(args.body)))
"#;

fn quote() -> WidgetFile {
    WidgetFile::parse("quote").unwrap()
}

#[test]
fn set_widget_twice_in_deck_mode_makes_one_current_version_and_one_undo_step() {
    let mut s = crate::test_support::session();
    let before = s.deck().templates.clone().unwrap();
    s.set_widget("quote", QUOTE.into()).unwrap();
    s.set_widget("quote", QUOTE_V2.into()).unwrap();
    let after = s.deck().templates.clone().unwrap();
    assert_that!(after.id).is_equal_to(before.id);
    let lib = crate::test_support::library_of(&s);
    assert_that!(lib.current(after.id)).is_equal_to(Some(after.version.clone()));
    let files = lib.version(after.id, &after.version).unwrap();
    assert_that!(files.widget(&quote())).is_equal_to(Some(QUOTE_V2));
    let manifest = PackManifest::from_files(&files).unwrap();
    assert_that!(manifest.parent).is_equal_to(Some(before.version.clone()));
    // Une seule étape : annuler rend la version d'avant, la bibliothèque garde la nouvelle.
    assert_that!(s.snapshot().undo_label.unwrap_or_default().as_str()).starts_with("Widgets");
    s.undo().unwrap();
    assert_that!(s.deck().templates.clone()).is_equal_to(Some(before));
    assert_that!(crate::test_support::library_of(&s).version(after.id, &after.version).is_some()).is_true();
}

#[test]
fn a_non_widget_operation_ends_the_widget_session() {
    let mut s = crate::test_support::session();
    s.set_widget("quote", QUOTE.into()).unwrap();
    let first = s.deck().templates.clone().unwrap();
    s.apply("Thème", vec![Op::SetTheme { theme: Some("pro".into()) }], None).unwrap();
    s.set_widget("quote", QUOTE_V2.into()).unwrap();
    let second = s.deck().templates.clone().unwrap();
    assert_that!(second.version).is_not_equal_to(first.version.clone());
    let lib = crate::test_support::library_of(&s);
    let manifest = PackManifest::from_files(&lib.version(second.id, &second.version).unwrap()).unwrap();
    assert_that!(manifest.parent).is_equal_to(Some(first.version.clone()));
    assert_that!(lib.version(first.id, &first.version).is_some()).is_true();
}

#[test]
fn a_widget_set_in_deck_mode_can_be_added_to_the_deck() {
    let mut s = crate::test_support::session();
    s.set_widget("quote", QUOTE.into()).unwrap();
    let add: Op = serde_json::from_value(json!({
        "op": "add_element",
        "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "element": { "id": "q", "kind": "quote", "params": { "body": [{ "runs": [{ "t": "Tout coule." }] }] } }
    }))
    .unwrap();
    s.apply("Citation", vec![add], None).unwrap();
    assert_that!(s.deck().slide("s-constat").is_some_and(|sl| sl.locate("q").is_some())).is_true();
}
```

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-session widgets_deck` → FAIL (`NotEditingTemplates` en mode deck).

- [ ] **Step 3 : implémentation.** `widgets_deck.rs` :

```rust
//! Widgets modifiés en mode deck : le pack du deck gagne une version.
//!
//! Séance : la première modification crée une version du pack du deck
//! (`parent` = la version embarquée), l'ajoute à la bibliothèque, la rend
//! courante et y fait passer le deck (`SetTemplates`, qui l'embarque) ; les
//! suivantes la remplacent (`Library::replace_version`, nouveau
//! `SetTemplates`), en une seule étape d'annulation (clé
//! `templates-adopt:<id>:<parent>`). Annuler rend la version d'avant ; la
//! bibliothèque garde la version créée. La séance finit à la première
//! opération qui n'est pas une modification de widget.

use std::collections::BTreeMap;

use snafu::ResultExt;
use uuid::Uuid;

use oku_slides_core::{Op, PackRef, PackVersion, WidgetFile};

use crate::error::PackJsonSnafu;
use crate::pack::PackManifest;
use crate::templates_mode::adopt_merge_key;
use crate::widgets_edit::WidgetTarget;
use crate::{Result, Session};

/// La séance de modification des widgets en mode deck.
#[derive(Debug, Clone)]
pub(crate) struct WidgetSession {
    id: Uuid,
    /// La version embarquée au début de la séance.
    parent: PackVersion,
    /// La version de la séance, celle du deck.
    version: PackVersion,
}

impl Session {
    /// Termine la séance de widgets du mode deck, s'il y en a une.
    pub(crate) fn end_widget_session(&mut self) {
        self.widget_session = None;
    }

    /// `manifest` et `widgets` deviennent la version de la séance ; le deck y passe.
    pub(crate) fn commit_deck_widgets(
        &mut self,
        t: &WidgetTarget,
        mut manifest: PackManifest,
        widgets: BTreeMap<WidgetFile, String>,
    ) -> Result<()> {
        // La séance continue tant que le deck est sur sa version.
        let session = self.widget_session.take().filter(|s| s.id == t.id && s.version == t.version);
        let parent = session.as_ref().map_or_else(|| t.version.clone(), |s| s.parent.clone());
        manifest.parent = Some(parent.clone());
        let files = manifest.to_files(widgets).context(PackJsonSnafu { id: t.id })?;
        let version = match &session {
            Some(s) => self.library.replace_version(t.id, &s.version, &files)?,
            None => {
                let version = files.version();
                self.library.add_version(t.id, &files)?;
                self.library.set_current(t.id, &version)?;
                version
            }
        };
        let label = self.localizer.tr_with("widgets-edit-label", &[("name", manifest.name.as_str().into())]);
        let merge = adopt_merge_key(t.id, Some(&parent));
        let templates = Some(PackRef { id: t.id, version: version.clone() });
        self.apply(&label, vec![Op::SetTemplates { templates }], Some(&merge))?;
        self.pack_notice = self.pack_notice_for_deck();
        self.widget_session = Some(WidgetSession { id: t.id, parent, version });
        Ok(())
    }
}

#[cfg(test)]
#[path = "widgets_deck_tests.rs"]
mod widgets_deck_tests;
```

  `widgets_edit.rs`, `commit_files` : le bras devient `Mode::Deck => self.commit_deck_widgets(t, manifest, widgets), Mode::History { .. } => InHistorySnafu.fail()?,` (et l'import `NotEditingTemplatesSnafu` disparaît).

  `lib.rs` : `mod widgets_deck;` ; champ `/// Séance de modification des widgets en mode deck. widget_session: Option<widgets_deck::WidgetSession>,` (init `None`) ; dans `handle`, avant le `match req` : `if req.changes_deck() && !req.is_widget_edit() { self.end_widget_session(); }` ; première ligne de `undo`, `redo` et `load_document` : `self.end_widget_session();`. `deck_widgets.rs`, `apply` (`:123`), avant `self.ensure_not_in_history()?;` : `self.end_widget_session();` (la séance est reprise par `commit_deck_widgets`, qui l'a prise avant d'appeler `apply`).

  Fluent, `session.ftl` après `session-widget-exists` :
  - fr : `widgets-edit-label = Widgets : { $name }`
  - en : `widgets-edit-label = Widgets: { $name }`
  - traduire les 18 autres.

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-session` → PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-session crates/oku-slides-i18n
```
```bash
git commit -m "feat(session) : modification des widgets en mode deck, une version par séance (t-329)"
```

---

### Task 9 : aperçu d'un widget seul (`preview_widget`)

**Files:**
- Create: `crates/oku-slides-session/src/widget_preview.rs`, `crates/oku-slides-session/src/widget_preview_tests.rs`
- Modify: `crates/oku-slides-session/src/request.rs` (`PreviewWidget`)
- Modify: `crates/oku-slides-session/src/lib.rs` (`mod widget_preview;`, réexports, champ `widget_preview`, bras de `handle`)
- Modify: `crates/oku-slides-i18n/src/locales/*/session.ftl` (`widget-preview-plain`, `widget-preview-rich`)

**Interfaces:**
- Consumes : `RenderWorker::{probe_pack, trial_slot}` (Task 4) ; `widget_target`, `parse_name`, `WidgetDiagnostic::of` (Task 7) ; `styled_like` (`pub(crate)`, Task 7).
- Produces :
  - `pub enum PreviewFormat { Svg, Png }`.
  - `pub struct WidgetPreview { pub svg: Option<String>, #[serde(skip)] pub png: Option<Vec<u8>>, pub diagnostics: Vec<WidgetDiagnostic> }` (`Serialize + Default`).
  - `Session::preview_widget(&mut self, &str, &Map<String, Value>, PreviewFormat) -> Result<WidgetPreview>` : fichier cassé → ses diagnostics sans image ; rendu raté (Typst, délai, mémoire) → diagnostics, pack non écarté.
  - `pub(crate) fn preview_params(&WidgetSchema, &Map<String, Value>, &Localizer) -> Params`.
  - `Request::PreviewWidget { name: String, params: Map<String, Value> }` (`cmd` : `preview_widget`, `params` par défaut vide), lecture (`changes_deck` faux).

- [ ] **Step 1 : tests en échec** — `widget_preview_tests.rs` :

```rust
use serde_json::{Map, json};
use speculoos::prelude::*;

use oku_slides_core::{ParamValue, WidgetSchema};

use super::{PreviewFormat, preview_params};

const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
"#;

const LOOP_ON_RENDER: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("boucle", params: (body: p.plain()),
  (args, ctx) => { let n = 0; for _ in range(100000) { for _ in range(100000) { n += 1 } }; [#n] })
"#;

fn schema() -> WidgetSchema {
    serde_json::from_value(json!({
        "name": "kpi", "label": null, "category": "element",
        "params": [
            { "name": "value", "type": "plain", "label": null, "default": "42 %", "bind": null },
            { "name": "label", "type": "plain", "label": null, "default": null, "bind": null },
            { "name": "body", "type": "rich", "label": null, "default": null },
            { "name": "trend", "type": "choice", "values": ["up", "down"], "label": null, "default": null },
            { "name": "count", "type": "number", "min": 3.0, "max": null, "label": null, "default": null },
            { "name": "on", "type": "flag", "label": null, "default": null },
            { "name": "logo", "type": "asset", "label": null, "default": null }
        ]
    }))
    .unwrap()
}

#[test]
fn preview_params_fill_given_then_defaults_then_examples_and_neutral_values() {
    let l = oku_slides_i18n::Localizer::new(Some("fr"));
    let given: Map<String, serde_json::Value> = serde_json::from_value(json!({ "label": "Donné" })).unwrap();
    let p = preview_params(&schema(), &given, &l);
    assert_that!(p.get("value")).is_equal_to(Some(&ParamValue::Text("42 %".into())));
    assert_that!(p.get("label")).is_equal_to(Some(&ParamValue::Text("Donné".into())));
    assert_that!(p.get("body").is_some_and(|v| matches!(v, ParamValue::Rich(_)))).is_true();
    assert_that!(p.get("trend")).is_equal_to(Some(&ParamValue::Text("up".into())));
    assert_that!(p.get("count")).is_equal_to(Some(&ParamValue::Value(json!(3.0))));
    assert_that!(p.get("on")).is_equal_to(Some(&ParamValue::Value(json!(false))));
    assert_that!(p.get("logo")).is_none();
    let example = l.tr("widget-preview-plain");
    let without = preview_params(&schema(), &Map::new(), &l);
    assert_that!(without.get("label")).is_equal_to(Some(&ParamValue::Text(example)));
}

#[test]
fn preview_widget_renders_an_svg_in_deck_mode() {
    let mut s = crate::test_support::session();
    s.set_widget("quote", QUOTE.into()).unwrap();
    let preview = s.preview_widget("quote", &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview.svg.unwrap_or_default().as_str()).starts_with("<svg");
    assert_that!(preview.diagnostics.iter().any(|d| d.error)).is_false();
}

#[test]
fn preview_widget_png_is_an_image_for_the_agent() {
    let mut s = crate::test_support::session();
    s.set_widget("quote", QUOTE.into()).unwrap();
    let preview = s.preview_widget("quote", &Map::new(), PreviewFormat::Png).unwrap();
    assert_that!(preview.png.unwrap_or_default().starts_with(b"\x89PNG")).is_true();
}

#[test]
fn preview_widget_of_a_broken_file_returns_its_diagnostics_without_image() {
    let mut s = crate::test_support::session();
    s.set_widget("casse", "#let widget = (".into()).unwrap();
    let preview = s.preview_widget("casse", &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview.svg).is_none();
    assert_that!(preview.diagnostics.iter().any(|d| d.error)).is_true();
}

#[test]
fn preview_widget_that_loops_returns_a_diagnostic_and_keeps_the_pack() {
    let data = crate::test_support::fresh_dir("oku-slides-preview-");
    let mut settings = crate::Settings::defaults().unwrap();
    settings.paths.data = Some(data.path().to_owned());
    settings.paths.decks = Some(data.path().join("decks"));
    settings.i18n.locale = Some("fr".into());
    settings.render.timeout_ms = 2000;
    let mut s = crate::Session::new(crate::Fonts::default(), &crate::Platform::with_settings(settings).unwrap()).unwrap();
    s.set_widget("boucle", LOOP_ON_RENDER.into()).unwrap();
    let preview = s.preview_widget("boucle", &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview.svg).is_none();
    assert_that!(preview.diagnostics.iter().any(|d| d.error)).is_true();
    // Le pack n'est pas écarté : le deck garde ses widgets.
    let pack = s.pack.clone().unwrap();
    assert_that!(s.worker.is_faulty(&pack)).is_false();
    assert_that!(s.registry.get("boucle")).is_some();
}
```

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-session widget_preview` → FAIL à la compilation.

- [ ] **Step 3 : implémentation.** `widget_preview.rs` :

```rust
//! Aperçu d'un widget seul : une slide temporaire au thème et aux jetons du
//! deck, rendue dans un emplacement d'essai du processus de rendu (un échec
//! borné n'y écarte pas le pack ; rien n'est ajouté au pack).
//!
//! Paramètres manquants : le `default` du schéma, sinon un exemple localisé
//! (`plain`, `rich`) ou la valeur neutre du type ; sans valeur neutre
//! (asset, slot, grille, hôte), le paramètre reste absent. Un widget
//! d'élément occupe toute la grille d'une slide `blank`.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::Serialize;
use serde_json::{Map, Value, json};
use snafu::{OptionExt, ResultExt, ensure};

use oku_slides_core::{
    Category, Cell, Deck, Element, ParamType, ParamValue, Params, Slide, WidgetFile, WidgetRegistry, WidgetSchema,
};
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_render::{PackWidgets, RenderError};
use oku_slides_render_worker::RenderSlot;

use crate::error::{NotRenderedSnafu, RegistrySnafu, RenderSnafu, UnknownWidgetSnafu};
use crate::templates_mode::styled_like;
use crate::widgets_edit::{WidgetDiagnostic, parse_name};
use crate::{Result, Session};

/// Identifiant de la slide d'aperçu.
const PREVIEW_SLIDE: &str = "apercu";
/// Identifiant de l'élément d'aperçu.
const PREVIEW_ELEMENT: &str = "apercu-element";
/// Pixels par point de l'aperçu PNG : 960 × 540, comme `render_slide`.
const PREVIEW_PIXEL_PER_PT: f32 = 0.5;

/// Le format de l'image d'aperçu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewFormat {
    /// Pour l'interface.
    Svg,
    /// Pour l'agent.
    Png,
}

/// L'aperçu d'un widget : son image, ou les diagnostics qui l'en empêchent.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct WidgetPreview {
    pub svg: Option<String>,
    #[serde(skip)]
    pub png: Option<Vec<u8>>,
    pub diagnostics: Vec<WidgetDiagnostic>,
}

/// L'emplacement d'essai, les widgets qui y sont posés et ses polices.
pub(crate) struct PreviewSlot {
    slot: RenderSlot,
    pack: Option<PackWidgets>,
    /// Nombre de polices du deck à sa création.
    fonts: usize,
}

/// Les paramètres de l'aperçu : ceux donnés, puis les défauts.
pub(crate) fn preview_params(schema: &WidgetSchema, given: &Map<String, Value>, l: &Localizer) -> Params {
    let mut params = Params::default();
    for p in &schema.params {
        let value = given
            .get(&p.name)
            .cloned()
            .or_else(|| p.default.clone().filter(|v| !v.is_null()))
            .or_else(|| neutral(&p.ty, l));
        if let Some(v) = value.and_then(|v| serde_json::from_value::<ParamValue>(v).ok()) {
            params.set(p.name.clone(), v);
        }
    }
    params
}

/// La valeur d'exemple, ou neutre, d'un type de paramètre.
fn neutral(ty: &ParamType, l: &Localizer) -> Option<Value> {
    match ty {
        ParamType::Plain => Some(Value::String(l.tr("widget-preview-plain"))),
        ParamType::Rich => Some(json!([{ "runs": [{ "t": l.tr("widget-preview-rich") }] }])),
        ParamType::Flag => Some(Value::Bool(false)),
        ParamType::Number { min, .. } => Some(json!(min.unwrap_or(0.0))),
        ParamType::Choice { values } => values.first().map(|v| Value::String(v.clone())),
        ParamType::Asset | ParamType::Slot { .. } | ParamType::Grid | ParamType::Host { .. } => None,
    }
}

/// Un rendu d'aperçu raté : les diagnostics de Typst, ou un seul (délai, mémoire…).
fn failure(name: &WidgetFile, e: &RenderError, l: &Localizer) -> Vec<WidgetDiagnostic> {
    if e.diagnostics().is_empty() {
        vec![WidgetDiagnostic { file: Some(name.clone()), line: None, column: None, error: true, message: e.localize(l) }]
    } else {
        e.diagnostics().iter().map(WidgetDiagnostic::of).collect()
    }
}

impl Session {
    /// Aperçu du widget `name` seul, sur une slide temporaire au style du deck.
    pub fn preview_widget(
        &mut self,
        name: &str,
        params: &Map<String, Value>,
        format: PreviewFormat,
    ) -> Result<WidgetPreview> {
        let name = parse_name(name)?;
        let t = self.widget_target()?;
        // `_commun` n'est pas un widget : rien à montrer.
        ensure!(t.files.widget(&name).is_some() && !name.is_common(), UnknownWidgetSnafu { name: name.as_str() });
        let pack = PackWidgets::new(t.version.hex(), t.files.widgets().clone()).context(RegistrySnafu)?;
        let probe = self.worker.probe_pack(&pack).context(RegistrySnafu)?;
        let broken = |file: Option<&oku_slides_render::FileProbe>| WidgetPreview {
            diagnostics: file.map(|f| f.diagnostics.iter().map(WidgetDiagnostic::of).collect()).unwrap_or_default(),
            ..WidgetPreview::default()
        };
        let file = probe.files.get(&name);
        if !file.is_some_and(oku_slides_render::FileProbe::is_valid) {
            return Ok(broken(file));
        }
        let Some(schema) = probe.registry.get(name.as_str()).cloned() else { return Ok(broken(file)) };
        let deck = self.preview_deck(&schema, params);
        self.render_preview(&name, probe.pack.as_ref(), probe.registry.clone(), &deck, format)
    }

    /// Le deck d'aperçu : la slide du widget, ou une `blank` qui porte l'élément.
    fn preview_deck(&self, schema: &WidgetSchema, given: &Map<String, Value>) -> Deck {
        let params = preview_params(schema, given, &self.localizer);
        let slide = match schema.category {
            Category::Slide => {
                let mut slide = Slide::new(PREVIEW_SLIDE, &schema.name);
                slide.params = params;
                slide
            }
            Category::Element => {
                let mut slide = Slide::new(PREVIEW_SLIDE, "blank");
                let grid = slide.grid();
                let mut element = Element::new(PREVIEW_ELEMENT, &schema.name);
                element.params = params;
                element.cell = Some(Cell { col: 0, row: 0, cols: grid.columns, rows: grid.rows });
                slide.params.set("elements", ParamValue::Slot(vec![element]));
                slide
            }
        };
        styled_like(self.user_deck(), vec![slide])
    }

    /// Rend `deck` dans l'emplacement d'essai, avec les widgets valides `pack`.
    fn render_preview(
        &mut self,
        name: &WidgetFile,
        pack: Option<&PackWidgets>,
        registry: Arc<WidgetRegistry>,
        deck: &Deck,
        format: PreviewFormat,
    ) -> Result<WidgetPreview> {
        let fonts = self.doc.sources.fonts.len();
        if self.widget_preview.as_ref().is_none_or(|p| p.fonts != fonts) {
            let base = oku_slides_render::base_registry(&self.app_fonts).context(RegistrySnafu)?;
            let slot = self.worker.trial_slot(&self.render_fonts, &self.localizer, base).context(RenderSnafu)?;
            self.widget_preview = Some(PreviewSlot { slot, pack: None, fonts });
        }
        let preview = self.widget_preview.as_mut().context(NotRenderedSnafu)?;
        let attempt = (|| -> std::result::Result<WidgetPreview, RenderError> {
            if preview.pack.as_ref() != pack {
                preview.slot.set_widgets(pack, registry)?;
                preview.pack = pack.cloned();
            }
            let compiled = preview.slot.render(deck, &BTreeMap::new())?;
            let diagnostics = compiled.warnings.iter().map(WidgetDiagnostic::of).collect();
            Ok(match format {
                PreviewFormat::Svg => WidgetPreview { svg: Some(compiled.svg(0)?), png: None, diagnostics },
                PreviewFormat::Png => {
                    WidgetPreview { svg: None, png: Some(compiled.png(0, PREVIEW_PIXEL_PER_PT)?), diagnostics }
                }
            })
        })();
        Ok(attempt.unwrap_or_else(|e| WidgetPreview { diagnostics: failure(name, &e, &self.localizer), ..Default::default() }))
    }
}

#[cfg(test)]
#[path = "widget_preview_tests.rs"]
mod widget_preview_tests;
```

  `request.rs` (après `RenameWidget`) :

```rust
    /// Aperçu SVG d'un widget seul (`WidgetPreview`) ; `params` complète ses défauts.
    PreviewWidget {
        name: String,
        #[serde(default)]
        params: serde_json::Map<String, serde_json::Value>,
    },
```

  (`changes_deck` : `false` ; absent de `edits`.) `lib.rs` : `mod widget_preview;`, `pub use widget_preview::{PreviewFormat, WidgetPreview};`, champ `/// Emplacement d'essai de l'aperçu des widgets. widget_preview: Option<widget_preview::PreviewSlot>,` (init `None`), bras `Request::PreviewWidget { name, params } => return json(&self.preview_widget(&name, &params, PreviewFormat::Svg)?),`. `lib_tests.rs` : `reads` gagne `json!({ "cmd": "preview_widget", "name": "kpi" })`.

  Fluent, `session.ftl` après `widgets-edit-label` :
  - fr : `widget-preview-plain = Texte d'exemple` ; `widget-preview-rich = Un paragraphe d'exemple, pour voir le widget en situation.`
  - en : `widget-preview-plain = Sample text` ; `widget-preview-rich = A sample paragraph, to see the widget in context.`
  - traduire les 18 autres.

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-session -p oku-slides-i18n` → PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-session crates/oku-slides-i18n
```
```bash
git commit -m "feat(session) : aperçu d'un widget seul dans un emplacement d'essai (t-329)"
```

---

### Task 10 : outils MCP, section « Créer un widget » et widget exemple

**Files:**
- Create: `crates/oku-slides-mcp/src/widget_tools.rs`, `crates/oku-slides-mcp/tests/widgets.rs`
- Modify: `crates/oku-slides-mcp/src/lib.rs` (`mod widget_tools;`), `tools.rs` (`list` `:179-221`), `handler.rs` (`call` `:44-69`), `ARCHITECTURE.md` (module `widget_tools`)
- Modify: `crates/oku-slides-agent/src/turn.rs:61-64`
- Modify: `crates/oku-slides-i18n/src/locales/*/agent.ftl` (`agent-instructions-widgets`, après `:3`)
- Create: `fixtures/widgets/kpi/kpi.typ`, `fixtures/widgets/kpi/_commun.typ`, `crates/oku-slides-render/tests/widget_files.rs`
- Modify: `typst/oku-slides/README.md` (section « Widgets », `:168-182`)

**Interfaces:**
- Consumes : `Session::{widget_status, widget_source, preview_widget}`, `Request::{WidgetStatus, SetWidget, RemoveWidget}`, `PreviewFormat::Png` (Tasks 7-9).
- Produces : outils `widget_sources`, `widget_source(name)`, `set_widget(name, source)`, `remove_widget(name)`, `preview_widget(name, params?)`, disponibles dans les deux modes ; `set_widget` et `remove_widget` émettent `McpEvent::Changed`.

- [ ] **Step 1 : tests en échec** — `crates/oku-slides-mcp/tests/widgets.rs` :

```rust
mod common;

use common::*;
use rust_mcp_sdk::McpClient;
use rust_mcp_sdk::schema::ContentBlock;
use serde_json::{Value, json};
use speculoos::prelude::*;

const KPI: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#import "_commun.typ": cadre
#let widget = widget("kpi", params: (value: p.plain()), (args, ctx) => cadre(field(ctx, "value", args.value)))
"#;

const COMMON: &str = "#let cadre(body) = block(inset: 8pt, body)\n";

/// Le JSON d'une réponse texte.
fn json_of(r: &rust_mcp_sdk::schema::CallToolResult) -> Value {
    serde_json::from_str(&text_of(r)).unwrap_or_else(|e| panic!("réponse non JSON : {e}"))
}

#[tokio::test(flavor = "multi_thread")]
async fn the_five_widget_tools_are_listed() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let names: Vec<String> = c.request_tool_list(None).await.unwrap().tools.into_iter().map(|t| t.name).collect();
    for tool in ["widget_sources", "widget_source", "set_widget", "remove_widget", "preview_widget"] {
        assert_that!(names.contains(&tool.to_owned())).named(tool).is_true();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_in_deck_mode_returns_its_status_then_previews_it() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    let r = call(&c, "set_widget", json!({ "name": "kpi", "source": KPI })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    let change = json_of(&r);
    assert_that!(change["status"]["name"].as_str()).is_equal_to(Some("kpi"));
    assert_that!(change["status"]["diagnostics"].as_array().map(Vec::len)).is_equal_to(Some(0));
    let sources = json_of(&call(&c, "widget_sources", json!({})).await);
    assert_that!(sources.as_array().map(Vec::len)).is_equal_to(Some(2));
    assert_that!(text_of(&call(&c, "widget_source", json!({ "name": "kpi" })).await)).is_equal_to(KPI.to_owned());
    let preview = call(&c, "preview_widget", json!({ "name": "kpi", "params": { "value": "12" } })).await;
    assert_that!(matches!(preview.content.first(), Some(ContentBlock::ImageContent(_)))).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_on_common_reports_diagnostics_and_broken_widgets() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    call(&c, "set_widget", json!({ "name": "kpi", "source": KPI })).await;
    let change = json_of(&call(&c, "set_widget", json!({ "name": "_commun", "source": "#let cadre = (" })).await);
    assert_that!(change["broken"][0]["name"].as_str()).is_equal_to(Some("kpi"));
    assert_that!(change["status"]["diagnostics"][0]["line"].as_u64()).is_equal_to(Some(1));
}

#[tokio::test(flavor = "multi_thread")]
async fn widget_tools_work_in_templates_mode_and_remove_refuses_a_used_widget() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "edit_templates", json!({ "set": "6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10" })).await;
    call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    assert_that!(call(&c, "set_widget", json!({ "name": "kpi", "source": KPI })).await.is_error).is_not_equal_to(Some(true));
    let slide = json!({ "op": "add_slide", "index": 0, "slide": {
        "id": "t-kpi", "kind": "blank", "name": "Indicateur", "params": { "elements": [
            { "id": "k", "kind": "kpi", "cell": { "col": 0, "row": 0, "cols": 4, "rows": 2 }, "params": { "value": "1" } }
        ] } } });
    call(&c, "apply", json!({ "ops": [slide] })).await;
    let r = call(&c, "remove_widget", json!({ "name": "kpi" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).contains("Indicateur");
}
```

  `crates/oku-slides-render/tests/widget_files.rs` :

```rust
//! Le widget exemple du README de `typst/oku-slides` : il est la fixture, et il se rend.

use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_core::{Deck, WidgetFile};
use oku_slides_render::{Fonts, PackWidgets, Renderer, load_registry, probe_file};
use speculoos::prelude::*;

const KPI: &str = include_str!("../../../fixtures/widgets/kpi/kpi.typ");
const COMMON: &str = include_str!("../../../fixtures/widgets/kpi/_commun.typ");
const README: &str = include_str!("../../../typst/oku-slides/README.md");

fn wf(name: &str) -> WidgetFile {
    WidgetFile::parse(name).unwrap()
}

#[test]
fn readme_example_widget_is_the_fixture() {
    assert_that!(README.contains(KPI)).is_true();
    assert_that!(README.contains(COMMON)).is_true();
}

#[test]
fn readme_example_widget_probes_valid_and_renders_its_fields() {
    let fonts = Fonts::default();
    assert_that!(probe_file(&fonts, &wf("kpi"), KPI, Some(COMMON)).is_valid()).is_true();
    let files = BTreeMap::from([(wf("kpi"), KPI.to_owned()), (wf("_commun"), COMMON.to_owned())]);
    let pack = PackWidgets::new("exemple", files).unwrap();
    let registry = Arc::new(load_registry(&fonts, Some(&pack)).unwrap());
    let mut r = Renderer::new(fonts).unwrap();
    r.set_widgets(Some(&pack), registry).unwrap();
    let deck = Deck::from_json(
        r#"{"version":3,"slides":[{"id":"s","kind":"content","params":{"blocks":[
            {"id":"k","kind":"kpi","params":{"value":"42 %","label":"Croissance"}}]}}]}"#,
    )
    .unwrap();
    let compiled = r.render(&deck, &BTreeMap::new()).unwrap();
    let fields: Vec<&str> = compiled.layout().fields.iter().map(|f| f.field.as_str()).collect();
    assert_that!(fields).contains("value");
    assert_that!(fields).contains("label");
}
```

- [ ] **Step 2 :** `cargo nextest run -p oku-slides-mcp -p oku-slides-render` → FAIL (outils inconnus, fixtures absentes).

- [ ] **Step 3 : implémentation.**

`fixtures/widgets/kpi/_commun.typ` :

```typ
// Code partagé des widgets du pack : `#import "_commun.typ": carte`.
#import "/oku-slides/lib.typ": color-of

/// Un cadre aux couleurs du thème.
#let carte(t, body) = block(width: 100%, inset: 24pt, radius: 8pt, fill: color-of(t, "surface"), body)
```

`fixtures/widgets/kpi/kpi.typ` :

```typ
// Un indicateur : une valeur en grand, sa légende dessous.
#import "/oku-slides/lib.typ": widget, field, fields-stack, params as p, color-of, size-of
#import "_commun.typ": carte

#let widget = widget("kpi", label: "KPI",
  params: (value: p.plain(default: "42 %"), label: p.plain(), trend: p.choice("up", "down", "flat")),
  (args, ctx) => carte(ctx.tokens, fields-stack(spacing: 8pt,
    field(ctx, "value", { set text(size: size-of(ctx.tokens, "display"), fill: color-of(ctx.tokens, "accent")); args.value }),
    field(ctx, "label", { set text(size: size-of(ctx.tokens, "body"), fill: color-of(ctx.tokens, "ink-muted")); args.label }),
  )))
```

  (Si Typst refuse un détail — `size-of` qui rendrait un nombre, `fields-stack` d'un contenu stylé —, corriger la fixture **et** le README ensemble : le test les compare.)

`typst/oku-slides/README.md`, section « Widgets » (`:168-182`) : le paragraphe sur `widgets.typ` devient :

> Un **pack de modèles** peut ajouter ses propres widgets, ou surcharger ceux de la base : **un fichier par widget**, `widgets/<nom>.typ`, dont la liaison `widget` est le résultat de `widget("<nom>", …)` (le nom déclaré est celui du fichier, `[a-z][a-z0-9-]*`). Le fichier facultatif `widgets/_commun.typ` porte le code partagé : un widget l'importe par `#import "_commun.typ": …` ; ce n'est pas un widget. L'app génère le `registry` du pack (un module d'assemblage qui importe chaque widget) ; un widget qui ne compile pas en est écarté avec ses diagnostics, les autres restent.
>
> Exemple : un indicateur, `widgets/kpi.typ` et `widgets/_commun.typ`.

  suivi de deux blocs ```` ```typ ```` contenant **exactement** le texte des deux fixtures (`_commun.typ` puis `kpi.typ`), puis : « Pour ses couleurs et ses tailles, un widget lit les jetons du thème (`color-of(ctx.tokens, "accent")`, `size-of(ctx.tokens, "body")`) plutôt que des valeurs en dur. »

`crates/oku-slides-mcp/src/widget_tools.rs` :

```rust
//! Outils des widgets du pack : statut, source, écriture, suppression et
//! aperçu. Disponibles en mode deck comme en mode modèles.

use base64::Engine as _;
use oku_slides_session::{PreviewFormat, Request};
use rust_mcp_sdk::schema::{CallToolResult, ContentBlock};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::McpEvent;
use crate::tools::{Tools, fail, json_result, parse, text};

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct WidgetName {
    /// Nom du fichier : `[a-z][a-z0-9-]*` (le nom du widget), ou `_commun` pour le code partagé.
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SetWidgetArgs {
    /// Nom du fichier : `[a-z][a-z0-9-]*` (le nom du widget), ou `_commun`.
    pub name: String,
    /// Source Typst complète du fichier ; vide : le fichier est supprimé.
    pub source: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct PreviewWidgetArgs {
    /// Nom du widget à montrer.
    pub name: String,
    /// Valeurs de paramètres ; les autres prennent leur défaut ou un exemple.
    #[serde(default)]
    pub params: Map<String, Value>,
}

impl Tools {
    /// Une requête de lecture des widgets, rendue telle quelle (JSON).
    async fn widget_read(&self, req: Request) -> CallToolResult {
        match self.with_session(move |s| s.handle(req).map_err(|e| e.one_line())).await {
            Ok(json) => text(json),
            Err(e) => fail(e),
        }
    }

    /// Une modification de widget : sa réponse JSON, et `Changed` si elle aboutit.
    async fn widget_change(&self, req: Request) -> CallToolResult {
        match self.with_session(move |s| s.handle(req).map_err(|e| e.one_line())).await {
            Ok(json) => {
                self.hub.emit(McpEvent::Changed);
                text(json)
            }
            Err(e) => fail(e),
        }
    }

    pub(crate) async fn widget_sources(&self) -> CallToolResult {
        self.widget_read(Request::WidgetStatus).await
    }

    pub(crate) async fn widget_source(&self, args: Value) -> CallToolResult {
        match parse::<WidgetName>(args) {
            Ok(WidgetName { name }) => {
                match self.with_session(move |s| s.widget_source(&name).map_err(|e| e.one_line())).await {
                    Ok(source) => text(source),
                    Err(e) => fail(e),
                }
            }
            Err(r) => r,
        }
    }

    pub(crate) async fn set_widget(&self, args: Value) -> CallToolResult {
        match parse::<SetWidgetArgs>(args) {
            Ok(SetWidgetArgs { name, source }) => self.widget_change(Request::SetWidget { name, source }).await,
            Err(r) => r,
        }
    }

    pub(crate) async fn remove_widget(&self, args: Value) -> CallToolResult {
        match parse::<WidgetName>(args) {
            Ok(WidgetName { name }) => self.widget_change(Request::RemoveWidget { name }).await,
            Err(r) => r,
        }
    }

    /// L'image PNG du widget, suivie de ses diagnostics s'il en a.
    pub(crate) async fn preview_widget(&self, args: Value) -> CallToolResult {
        let PreviewWidgetArgs { name, params } = match parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let preview = self
            .with_session(move |s| s.preview_widget(&name, &params, PreviewFormat::Png).map_err(|e| e.one_line()))
            .await;
        let preview = match preview {
            Ok(p) => p,
            Err(e) => return fail(e),
        };
        let Some(png) = &preview.png else { return json_result(&preview.diagnostics) };
        let mut content =
            vec![ContentBlock::image_content(base64::engine::general_purpose::STANDARD.encode(png), "image/png".into())];
        if !preview.diagnostics.is_empty() {
            match serde_json::to_string(&preview.diagnostics) {
                Ok(json) => content.push(ContentBlock::text_content(json)),
                Err(e) => return fail(format!("response not serializable: {e}")),
            }
        }
        CallToolResult { content, is_error: None, meta: None, structured_content: None }
    }
}
```

`tools.rs`, `list`, après `close_templates` :

```rust
            tool::<NoArgs>("widget_sources", "Widget files of the pack in use (the edited template pack, or the deck's pack): name, schema when valid, diagnostics (file, line, column, message). `_commun` holds shared code. Works in deck and template modes."),
            tool::<crate::widget_tools::WidgetName>("widget_source", "Typst source of one widget file."),
            tool::<crate::widget_tools::SetWidgetArgs>("set_widget", "Create or replace a widget file (Typst). Returns its status and the widgets this change broke. In deck mode the deck moves to a new version of its pack; successive widget edits make one undo step."),
            tool::<crate::widget_tools::WidgetName>("remove_widget", "Delete a widget file; refused while a template of the pack uses it."),
            tool::<crate::widget_tools::PreviewWidgetArgs>("preview_widget", "PNG of one widget alone on a temporary slide (deck theme and tokens), followed by its diagnostics if any. Missing params take their default or a sample value. Look at it after each set_widget."),
```

`handler.rs`, `call`, après `"close_templates"` :

```rust
            "widget_sources" => self.widget_sources().await,
            "widget_source" => self.widget_source(args).await,
            "set_widget" => self.set_widget(args).await,
            "remove_widget" => self.remove_widget(args).await,
            "preview_widget" => self.preview_widget(args).await,
```

  `lib.rs` : `mod widget_tools;`. `ARCHITECTURE.md`, après `read_tools` : « - `widget_tools` : les outils des widgets du pack (`widget_sources`, `widget_source`, `set_widget`, `remove_widget`, `preview_widget` en PNG), dans les deux modes. »

`crates/oku-slides-agent/src/turn.rs:61-64` :

```rust
    let mut blocks = Vec::with_capacity(4);
    if first {
        blocks.push(ContentBlock::Text(TextContent::new(ctx.localizer.tr("agent-instructions"))));
        blocks.push(ContentBlock::Text(TextContent::new(ctx.localizer.tr("agent-instructions-widgets"))));
    }
```

  Fluent, `agent.ftl` après `agent-instructions` (`:3`), sur une ligne (aucune accolade : Fluent les lirait) :
  - fr : `agent-instructions-widgets = Créer un widget : un widget est un fichier Typst du pack de modèles ; utilise widget_sources, widget_source, set_widget, remove_widget et preview_widget, en mode deck comme en mode modèles. Forme d'un fichier kpi : #import "/oku-slides/lib.typ": widget, field, fields-stack, params as p, color-of, size-of, puis #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …) ; le nom déclaré est celui du fichier. Paramètres : descripteurs p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid. Textes éditables : field(ctx, "value", args.value), empilés par fields-stack. Couleurs et tailles : jetons du thème (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")), jamais de valeurs en dur. Code partagé : le fichier _commun, importé par #import "_commun.typ": …. Boucle : écrire avec set_widget, lire le statut et ses diagnostics (ligne, colonne), regarder preview_widget, corriger. Pour l'insérer dans le deck : add_element avec kind = nom du widget.`
  - en : `agent-instructions-widgets = Creating a widget: a widget is a Typst file of the template pack; use widget_sources, widget_source, set_widget, remove_widget and preview_widget, in deck and template modes alike. Shape of a file kpi: #import "/oku-slides/lib.typ": widget, field, fields-stack, params as p, color-of, size-of, then #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …); the declared name is the file name. Parameters: descriptors p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid. Editable texts: field(ctx, "value", args.value), stacked with fields-stack. Colors and sizes: theme tokens (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")), never hard-coded values. Shared code: the _commun file, imported with #import "_commun.typ": …. Loop: write with set_widget, read the status and its diagnostics (line, column), look at preview_widget, fix. To put it in the deck: add_element with kind = the widget name.`
  - traduire les 18 autres (code Typst et noms d'outils inchangés).

- [ ] **Step 4 :** `cargo nextest run -p oku-slides-mcp -p oku-slides-render -p oku-slides-agent -p oku-slides-i18n` → PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-mcp crates/oku-slides-agent crates/oku-slides-i18n crates/oku-slides-render/tests/widget_files.rs fixtures/widgets/kpi typst/oku-slides/README.md
```
```bash
git commit -m "feat(mcp) : outils des widgets, section « Créer un widget » du prompt et widget exemple (t-329)"
```

---

### Task 11 : UI — panneau Widgets, éditeur Typst et aperçu

**Files:**
- Modify: `ui/src/api.ts` (types après `TemplatesView` `:143-147`, `Request` `:333-361`, `tabApi` `:495-537`)
- Create: `ui/src/typstHighlight.ts`, `ui/src/typstHighlight.test.ts`
- Create: `ui/src/widgetDiagnostics.ts`, `ui/src/widgetDiagnostics.test.ts`
- Create: `ui/src/debounce.ts`, `ui/src/debounce.test.ts`
- Create: `ui/src/widgetList.ts`, `ui/src/widgetList.test.ts`
- Create: `ui/src/TypstEditor.tsx`, `ui/src/WidgetWorkbench.tsx`, `ui/src/WidgetWorkbench.render.test.tsx`
- Modify: `ui/src/App.tsx` (`lazy` `:46-50`, état `:388`, gardes clavier `:254` et `:536`, menus `:715-739`, rendu après `TemplatePicker` `:911-921`)
- Modify: `ui/src/styles.css` (fin du fichier)
- Modify: `crates/oku-slides-i18n/src/locales/*/ui.ftl` (fin, 20 langues)

**Interfaces:**
- Consumes : requêtes `widget_status`, `widget_source`, `set_widget`, `remove_widget`, `rename_widget`, `preview_widget` (Tasks 7-9) ; `Snapshot.templates`, `Snapshot.editing.base`.
- Produces :
  - `api.ts` : `WidgetDiagnostic { file: string | null; line: number | null; column: number | null; error: boolean; message: string }`, `WidgetStatus { name: string; schema: unknown | null; diagnostics: WidgetDiagnostic[] }`, `WidgetChange { status: WidgetStatus; broken: WidgetStatus[] }`, `WidgetPreview { svg: string | null; diagnostics: WidgetDiagnostic[] }` ; `tabApi` : `widgetStatus()`, `widgetSource(name)`, `setWidget(name, source)`, `removeWidget(name)`, `renameWidget(from, to)`, `previewWidget(name, params?)`.
  - `typstTokens(line: string): TypstToken[]` (`cm-typ-comment|string|keyword|hash|param`).
  - `diagnosticRange(doc: string, d: WidgetDiagnostic): { from: number; to: number; line: number } | null`.
  - `debounced(fn, ms, timers?)` (+ `.cancel()`), `PREVIEW_DELAY_MS = 400`.
  - `COMMON`, `isWidgetName`, `widgetLabel`, `stateOf`, `ordered`, `fileDiagnostics`, `skeleton` (`widgetList.ts`).
  - `TypstEditor` (props `text`, `diagnostics`, `label`, `onChange`), `WidgetWorkbench` (props `api`, `revision`, `onChanged`, `onClose`, `onError`).

- [ ] **Step 1 : tests en échec.**

`ui/src/typstHighlight.test.ts` :

```ts
import { describe, expect, test } from "bun:test";
import { typstTokens } from "./typstHighlight";

describe("typstTokens", () => {
  test("dièse, mot-clé, chaîne et descripteur p.*", () => {
    expect(typstTokens('#let w = widget("kpi", params: (v: p.plain()))')).toEqual([
      { from: 0, to: 1, className: "cm-typ-hash" },
      { from: 1, to: 4, className: "cm-typ-keyword" },
      { from: 16, to: 21, className: "cm-typ-string" },
      { from: 35, to: 42, className: "cm-typ-param" },
    ]);
  });

  test("commentaire de ligne jusqu'au bout, chaîne échappée", () => {
    expect(typstTokens('"a\\"b" // fin')).toEqual([
      { from: 0, to: 6, className: "cm-typ-string" },
      { from: 7, to: 13, className: "cm-typ-comment" },
    ]);
  });

  test("p.* collé à un identifiant n'est pas un descripteur", () => {
    expect(typstTokens("top.plain")).toEqual([]);
  });
});
```

`ui/src/widgetDiagnostics.test.ts` :

```ts
import { describe, expect, test } from "bun:test";
import type { WidgetDiagnostic } from "./api";
import { diagnosticRange } from "./widgetDiagnostics";

const at = (line: number | null, column: number | null): WidgetDiagnostic => ({
  file: "kpi",
  line,
  column,
  error: true,
  message: "m",
});

describe("diagnosticRange", () => {
  test("ligne 2, colonne 6 : le mot souligné à la bonne place", () => {
    expect(diagnosticRange("#let a = 1\n#let bad = (", at(2, 6))).toEqual({ from: 16, to: 19, line: 2 });
  });

  test("sans colonne : le premier caractère de la ligne", () => {
    expect(diagnosticRange("x\ny = (", at(2, null))).toEqual({ from: 2, to: 3, line: 2 });
  });

  test("hors du texte ou sans ligne : rien", () => {
    expect(diagnosticRange("x", at(5, 1))).toBeNull();
    expect(diagnosticRange("x", at(null, null))).toBeNull();
  });
});
```

`ui/src/debounce.test.ts` :

```ts
import { describe, expect, test } from "bun:test";
import { PREVIEW_DELAY_MS, type Timers, debounced } from "./debounce";

/** Minuteries à la main : on déclenche ce qui attend. */
function manual() {
  let pending: { fn: () => void; ms: number } | null = null;
  const timers: Timers = {
    set: (fn, ms) => {
      pending = { fn, ms };
      return pending;
    },
    clear: (h) => {
      if (h === pending) pending = null;
    },
  };
  return { timers, fire: () => pending?.fn(), delay: () => pending?.ms ?? null };
}

describe("debounced", () => {
  test("seule la dernière frappe part, après le délai de l'aperçu", () => {
    const m = manual();
    const seen: string[] = [];
    const push = debounced((s: string) => seen.push(s), PREVIEW_DELAY_MS, m.timers);
    push("a");
    push("ab");
    push("abc");
    expect(seen).toEqual([]);
    expect(m.delay()).toBe(400);
    m.fire();
    expect(seen).toEqual(["abc"]);
  });

  test("cancel oublie ce qui attend", () => {
    const m = manual();
    const seen: string[] = [];
    const push = debounced((s: string) => seen.push(s), 400, m.timers);
    push("a");
    push.cancel();
    m.fire();
    expect(seen).toEqual([]);
  });
});
```

`ui/src/widgetList.test.ts` :

```ts
import { describe, expect, test } from "bun:test";
import type { WidgetStatus } from "./api";
import { COMMON, fileDiagnostics, isWidgetName, ordered, skeleton, stateOf } from "./widgetList";

const status = (name: string, error = false): WidgetStatus => ({
  name,
  schema: null,
  diagnostics: error ? [{ file: name, line: 1, column: 1, error: true, message: "x" }] : [],
});

describe("widgetList", () => {
  test("_commun en tête, puis par nom", () => {
    expect(ordered([status("kpi"), status(COMMON), status("a")]).map((s) => s.name)).toEqual([COMMON, "a", "kpi"]);
  });

  test("état : valide, ou erreurs", () => {
    expect(stateOf(status("kpi"))).toBe("valid");
    expect(stateOf(status("kpi", true))).toBe("errors");
  });

  test("noms admis : ceux de la session", () => {
    expect(isWidgetName("kpi-2")).toBe(true);
    expect(isWidgetName(COMMON)).toBe(true);
    expect(isWidgetName("Kpi")).toBe(false);
    expect(isWidgetName("_autre")).toBe(false);
  });

  test("diagnostics du seul fichier montré", () => {
    const s: WidgetStatus = {
      name: "kpi",
      schema: null,
      diagnostics: [
        { file: "kpi", line: 1, column: 1, error: true, message: "ici" },
        { file: COMMON, line: 2, column: 1, error: true, message: "là" },
      ],
    };
    expect(fileDiagnostics(s, "kpi").map((d) => d.message)).toEqual(["ici"]);
  });

  test("squelette : déclare le nom de son fichier", () => {
    expect(skeleton("kpi")).toContain('widget("kpi"');
    expect(skeleton(COMMON)).not.toContain("#let widget");
  });
});
```

`ui/src/WidgetWorkbench.render.test.tsx` :

```tsx
// Rendu du panneau Widgets dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { TabApi, WidgetStatus } from "./api";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const WidgetWorkbench = (await import("./WidgetWorkbench")).default;

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const statuses: WidgetStatus[] = [
  { name: "kpi", schema: {}, diagnostics: [] },
  { name: "_commun", schema: null, diagnostics: [{ file: "_commun", line: 1, column: 1, error: true, message: "x" }] },
];

const api = {
  widgetStatus: async () => statuses,
  widgetSource: async () => "",
  previewWidget: async () => ({ svg: null, diagnostics: [] }),
} as unknown as TabApi;

async function mount() {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => {
    root.render(
      createElement(WidgetWorkbench, { api, revision: "r", onChanged: () => {}, onClose: () => {}, onError: () => {} }),
    );
  });
  return host;
}

describe("WidgetWorkbench", () => {
  test("liste : _commun en tête, chacun avec sa pastille d'état", async () => {
    const host = await mount();
    const items = [...host.querySelectorAll(".workbench-list li")];
    expect(items.map((li) => li.querySelector(".pill")?.className)).toEqual(["pill errors", "pill valid"]);
    expect(items[1]?.textContent).toContain("kpi");
  });
});
```

- [ ] **Step 2 :** `cd ui && bun test` → FAIL (modules absents).

- [ ] **Step 3 : implémentation.**

`api.ts`, après `TemplatesView` :

```ts
/** Un message de Typst (ou du contrôle) sur un fichier de widget ; ligne et colonne à partir de 1. */
export interface WidgetDiagnostic {
  file: string | null;
  line: number | null;
  column: number | null;
  error: boolean;
  message: string;
}

/** Un fichier de widget du pack en cours et son statut. */
export interface WidgetStatus {
  name: string;
  /** Schéma du widget s'il est valide ; `null` pour `_commun`. */
  schema: unknown | null;
  diagnostics: WidgetDiagnostic[];
}

/** Réponse d'une écriture : le fichier, et les widgets qu'elle a cassés. */
export interface WidgetChange {
  status: WidgetStatus;
  broken: WidgetStatus[];
}

/** Aperçu d'un widget seul : SVG, ou les diagnostics qui l'en empêchent. */
export interface WidgetPreview {
  svg: string | null;
  diagnostics: WidgetDiagnostic[];
}
```

  `Request`, avant `| { cmd: "history_versions" }` :

```ts
  | { cmd: "widget_status" }
  | { cmd: "widget_source"; name: string }
  | { cmd: "set_widget"; name: string; source: string }
  | { cmd: "remove_widget"; name: string }
  | { cmd: "rename_widget"; from: string; to: string }
  | { cmd: "preview_widget"; name: string; params: Record<string, unknown> }
```

  `tabApi`, après `deleteTemplateSet` :

```ts
    /** Fichiers de widgets du pack en cours, `_commun` en tête. */
    widgetStatus: () => call<WidgetStatus[]>({ cmd: "widget_status" }),
    widgetSource: (name: string) => call<string>({ cmd: "widget_source", name }),
    /** Crée ou remplace un fichier ; en mode deck, le deck passe à la nouvelle version de son pack. */
    setWidget: (name: string, source: string) => call<WidgetChange>({ cmd: "set_widget", name, source }),
    removeWidget: (name: string) => call<WidgetStatus[]>({ cmd: "remove_widget", name }),
    renameWidget: (from: string, to: string) => call<WidgetStatus[]>({ cmd: "rename_widget", from, to }),
    previewWidget: (name: string, params: Record<string, unknown> = {}) =>
      call<WidgetPreview>({ cmd: "preview_widget", name, params }),
```

`ui/src/typstHighlight.ts` :

```ts
/**
 * Coloration Typst minimale, ligne par ligne : commentaires, chaînes,
 * mots-clés, `#` et descripteurs `p.*`. Pas d'analyse : un commentaire
 * `/* … *\/` sur plusieurs lignes n'est vu que sur sa première.
 */

/** Une plage colorée d'une ligne. */
export interface TypstToken {
  from: number;
  to: number;
  className: string;
}

const KEYWORDS = new Set([
  "let", "set", "show", "import", "include", "if", "else", "for", "in", "while", "return",
  "context", "none", "auto", "true", "false", "and", "or", "not", "as", "break", "continue",
]);

/** Les plages colorées de `line`, dans l'ordre. */
export function typstTokens(line: string): TypstToken[] {
  const out: TypstToken[] = [];
  const push = (from: number, length: number, kind: string) => out.push({ from, to: from + length, className: `cm-typ-${kind}` });
  let i = 0;
  while (i < line.length) {
    const rest = line.slice(i);
    if (rest.startsWith("//")) {
      push(i, line.length - i, "comment");
      break;
    }
    const block = rest.match(/^\/\*.*?(\*\/|$)/);
    const string = rest.match(/^"(?:[^"\\]|\\.)*"?/);
    const param = rest.match(/^p\.[a-z]+/);
    const word = rest.match(/^[A-Za-z_][\w-]*/);
    const glued = i > 0 && /[\w.-]/.test(line[i - 1] ?? "");
    if (block) {
      push(i, block[0].length, "comment");
      i += block[0].length;
    } else if (string) {
      push(i, string[0].length, "string");
      i += string[0].length;
    } else if (rest[0] === "#") {
      push(i, 1, "hash");
      i += 1;
    } else if (param && !glued) {
      push(i, param[0].length, "param");
      i += param[0].length;
    } else if (word) {
      if (KEYWORDS.has(word[0]) && !glued) push(i, word[0].length, "keyword");
      i += word[0].length;
    } else {
      i += 1;
    }
  }
  return out;
}
```

`ui/src/widgetDiagnostics.ts` :

```ts
/**
 * Place d'un diagnostic dans le texte d'un fichier : ligne et colonne de
 * Typst (à partir de 1, colonnes en caractères) vers des positions
 * CodeMirror. Le mot sous la colonne est souligné, sinon un caractère.
 */
import type { WidgetDiagnostic } from "./api";

/** Plage soulignée et ligne d'un diagnostic ; `null` s'il n'a pas de place dans `doc`. */
export function diagnosticRange(doc: string, d: WidgetDiagnostic): { from: number; to: number; line: number } | null {
  if (d.line == null) return null;
  const lines = doc.split("\n");
  if (d.line < 1 || d.line > lines.length) return null;
  let start = 0;
  for (let i = 0; i < d.line - 1; i++) start += (lines[i] ?? "").length + 1;
  const text = lines[d.line - 1] ?? "";
  const col = Math.min(Math.max((d.column ?? 1) - 1, 0), text.length);
  const word = text.slice(col).match(/^[\w-]+/)?.[0].length ?? 1;
  const end = Math.min(col + Math.max(word, 1), text.length);
  return { from: start + col, to: start + Math.max(end, col), line: d.line };
}
```

`ui/src/debounce.ts` :

```ts
/** Appel retardé : seul le dernier d'une rafale part, `ms` après lui. */

/** Minuteries injectables (tests). */
export interface Timers {
  set(fn: () => void, ms: number): unknown;
  clear(handle: unknown): void;
}

const REAL: Timers = {
  set: (fn, ms) => setTimeout(fn, ms),
  clear: (h) => clearTimeout(h as ReturnType<typeof setTimeout>),
};

/** Délai de l'aperçu d'un widget après la dernière frappe. */
export const PREVIEW_DELAY_MS = 400;

/** `fn` retardé de `ms` ; `.cancel()` oublie l'appel en attente. */
export function debounced<A extends unknown[]>(fn: (...args: A) => void, ms: number, timers: Timers = REAL) {
  let handle: unknown = null;
  const call = (...args: A) => {
    if (handle !== null) timers.clear(handle);
    handle = timers.set(() => {
      handle = null;
      fn(...args);
    }, ms);
  };
  call.cancel = () => {
    if (handle !== null) timers.clear(handle);
    handle = null;
  };
  return call;
}
```

`ui/src/widgetList.ts` :

```ts
/** Liste des fichiers de widgets : ordre, libellés, états, squelettes. */
import type { WidgetDiagnostic, WidgetStatus } from "./api";
import { t } from "./i18n";

/** Le fichier de code partagé du pack. */
export const COMMON = "_commun";

const NAME = /^[a-z][a-z0-9-]{0,63}$/;

/** Un nom de fichier admis par la session. */
export function isWidgetName(name: string): boolean {
  return name === COMMON || NAME.test(name);
}

/** Libellé d'un fichier : « Commun » pour `_commun`. */
export function widgetLabel(name: string): string {
  return name === COMMON ? t("ui-widgets-common") : name;
}

export type WidgetState = "valid" | "errors";

export function stateOf(s: WidgetStatus): WidgetState {
  return s.diagnostics.some((d) => d.error) ? "errors" : "valid";
}

/** `_commun` en tête, puis par nom. */
export function ordered(list: WidgetStatus[]): WidgetStatus[] {
  return [...list].sort((a, b) => (a.name === COMMON ? -1 : b.name === COMMON ? 1 : a.name.localeCompare(b.name)));
}

/** Les diagnostics de `s` placés dans le fichier `file`. */
export function fileDiagnostics(s: WidgetStatus | undefined, file: string): WidgetDiagnostic[] {
  return (s?.diagnostics ?? []).filter((d) => d.file === file);
}

/** Squelette d'un nouveau fichier : imports, déclaration, un champ éditable. */
export function skeleton(name: string): string {
  if (name === COMMON) return '// Code partagé : #import "_commun.typ": …\n#import "/oku-slides/lib.typ": color-of, size-of\n';
  return [
    '#import "/oku-slides/lib.typ": widget, field, fields-stack, params as p, color-of, size-of',
    "",
    `#let widget = widget("${name}", params: (title: p.plain()),`,
    '  (args, ctx) => field(ctx, "title", { set text(fill: color-of(ctx.tokens, "accent")); args.title }))',
    "",
  ].join("\n");
}
```

`ui/src/TypstEditor.tsx` :

```tsx
/**
 * Éditeur CodeMirror d'un fichier de widget : coloration Typst minimale,
 * diagnostics soulignés et marqués dans la marge, à leur ligne.
 *
 * Une vue par fichier : le parent la recrée par `key`. Une frappe efface
 * les diagnostics affichés ; les suivants arrivent après l'écriture.
 */
import { useEffect, useEffectEvent, useLayoutEffect, useRef } from "react";
import { history, historyKeymap, indentWithTab, standardKeymap } from "@codemirror/commands";
import type { Range } from "@codemirror/state";
import { EditorState, RangeSetBuilder, StateEffect, StateField } from "@codemirror/state";
import type { DecorationSet, ViewUpdate } from "@codemirror/view";
import { Decoration, EditorView, GutterMarker, ViewPlugin, gutter, keymap, lineNumbers } from "@codemirror/view";
import type { WidgetDiagnostic } from "./api";
import { typstTokens } from "./typstHighlight";
import { diagnosticRange } from "./widgetDiagnostics";

const MARKS = new Map(
  ["comment", "string", "keyword", "hash", "param"].map((k) => [`cm-typ-${k}`, Decoration.mark({ class: `cm-typ-${k}` })] as const),
);

/** Décorations de coloration des lignes visibles. */
function highlight(view: EditorView): DecorationSet {
  const ranges: Range<Decoration>[] = [];
  for (const { from, to } of view.visibleRanges) {
    for (let pos = from; pos <= to; ) {
      const line = view.state.doc.lineAt(pos);
      for (const tk of typstTokens(line.text)) {
        const mark = MARKS.get(tk.className);
        if (mark && tk.to > tk.from) ranges.push(mark.range(line.from + tk.from, line.from + tk.to));
      }
      pos = line.to + 1;
    }
  }
  return Decoration.set(ranges, true);
}

const typstHighlight = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;
    constructor(view: EditorView) {
      this.decorations = highlight(view);
    }
    update(u: ViewUpdate) {
      if (u.docChanged || u.viewportChanged) this.decorations = highlight(u.view);
    }
  },
  { decorations: (v) => v.decorations },
);

const setDiagnostics = StateEffect.define<WidgetDiagnostic[]>();

/** Les diagnostics affichés ; une frappe les efface. */
const diagnosticsField = StateField.define<WidgetDiagnostic[]>({
  create: () => [],
  update(value, tr) {
    for (const e of tr.effects) if (e.is(setDiagnostics)) return e.value;
    return tr.docChanged ? [] : value;
  },
});

const underline = Decoration.mark({ class: "cm-diag-underline" });

const diagnosticMarks = EditorView.decorations.compute([diagnosticsField], (state) => {
  const doc = state.doc.toString();
  const ranges = state
    .field(diagnosticsField)
    .map((d) => diagnosticRange(doc, d))
    .filter((r): r is NonNullable<typeof r> => r !== null && r.to > r.from)
    .map((r) => underline.range(r.from, r.to));
  return Decoration.set(ranges, true);
});

class DiagnosticMarker extends GutterMarker {
  constructor(readonly message: string) {
    super();
  }
  override toDOM() {
    const el = document.createElement("span");
    el.className = "cm-diag-marker";
    el.textContent = "●";
    el.title = this.message;
    return el;
  }
}

/** La marge : un repère par ligne qui porte un diagnostic, ses messages en titre. */
const diagnosticGutter = gutter({
  class: "cm-diag-gutter",
  markers: (view) => {
    const doc = view.state.doc;
    const byLine = new Map<number, string[]>();
    for (const d of view.state.field(diagnosticsField)) {
      if (d.line == null || d.line < 1 || d.line > doc.lines) continue;
      byLine.set(d.line, [...(byLine.get(d.line) ?? []), d.message]);
    }
    const builder = new RangeSetBuilder<GutterMarker>();
    for (const line of [...byLine.keys()].sort((a, b) => a - b)) {
      const at = doc.line(line).from;
      builder.add(at, at, new DiagnosticMarker((byLine.get(line) ?? []).join("\n")));
    }
    return builder.finish();
  },
});

interface Props {
  /** Texte à l'ouverture (lu une fois). */
  text: string;
  /** Diagnostics de ce fichier. */
  diagnostics: WidgetDiagnostic[];
  /** Nom accessible de la zone éditable. */
  label: string;
  /** Texte après chaque modification. */
  onChange: (text: string) => void;
}

export default function TypstEditor({ text, diagnostics, label, onChange }: Props) {
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);
  const changed = useEffectEvent((t: string) => onChange(t));
  useLayoutEffect(() => {
    const parent = host.current;
    if (!parent) return;
    const v = new EditorView({
      parent,
      state: EditorState.create({
        doc: text,
        extensions: [
          lineNumbers(),
          diagnosticGutter,
          history(),
          keymap.of([indentWithTab, ...standardKeymap, ...historyKeymap]),
          typstHighlight,
          diagnosticsField,
          diagnosticMarks,
          EditorView.contentAttributes.of({ "aria-label": label }),
          EditorView.updateListener.of((u) => {
            if (u.docChanged) changed(u.state.doc.toString());
          }),
        ],
      }),
    });
    view.current = v;
    return () => {
      v.destroy();
      view.current = null;
    };
    // Le texte n'est lu qu'à l'ouverture : le parent recrée la vue par `key`.
  }, []);
  useEffect(() => {
    view.current?.dispatch({ effects: setDiagnostics.of(diagnostics) });
  }, [diagnostics]);
  return <div className="typst-editor" ref={host} />;
}
```

`ui/src/WidgetWorkbench.tsx` :

```tsx
/**
 * Panneau Widgets : les fichiers du pack en cours (mode modèles ou deck),
 * l'éditeur Typst du fichier choisi et l'aperçu du widget seul.
 *
 * La frappe passe par `set_widget`, 400 ms après la dernière touche, puis
 * l'aperçu est redemandé. L'agent passe par les mêmes opérations : quand
 * `revision` (version du pack) change, la liste est relue, et la source
 * aussi si ce n'est pas notre propre écriture.
 */
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { errorText, type TabApi, type WidgetPreview, type WidgetStatus } from "./api";
import { PREVIEW_DELAY_MS, debounced } from "./debounce";
import { t } from "./i18n";
import { Icon } from "./icons";
import TypstEditor from "./TypstEditor";
import { COMMON, fileDiagnostics, isWidgetName, ordered, skeleton, stateOf, widgetLabel } from "./widgetList";

interface Props {
  api: TabApi;
  /** Change avec la version du pack en cours (le deck ou les modèles). */
  revision: string;
  /** Une écriture a abouti : le snapshot est à relire. */
  onChanged: () => void;
  onClose: () => void;
  onError: (message: string) => void;
}

export default function WidgetWorkbench({ api, revision, onChanged, onClose, onError }: Props) {
  const [statuses, setStatuses] = useState<WidgetStatus[] | null>(null);
  const [current, setCurrent] = useState<string | null>(null);
  const [source, setSource] = useState<string | null>(null);
  const [editorKey, setEditorKey] = useState(0);
  const [preview, setPreview] = useState<WidgetPreview | null>(null);
  const [naming, setNaming] = useState<{ from: string | null; value: string } | null>(null);
  /** Dernier texte envoyé, et une frappe en attente : la relecture ne l'écrase pas. */
  const sent = useRef<string | null>(null);
  const pending = useRef(false);

  const fail = useCallback((e: unknown) => onError(errorText(e)), [onError]);
  const refresh = useCallback(() => api.widgetStatus().then((s) => setStatuses(ordered(s))), [api]);
  const showPreview = useCallback(
    (name: string) => {
      if (name === COMMON) return setPreview(null);
      api.previewWidget(name).then(setPreview).catch(fail);
    },
    [api, fail],
  );

  useEffect(() => {
    refresh().catch(fail);
  }, [refresh, revision, fail]);

  // Fichier choisi, ou pack changé ailleurs : sa source, si ce n'est pas notre écriture.
  useEffect(() => {
    if (!current) return setSource(null);
    let alive = true;
    api
      .widgetSource(current)
      .then((s) => {
        if (!alive || pending.current || s === sent.current) return;
        sent.current = s;
        setSource(s);
        setEditorKey((k) => k + 1);
        showPreview(current);
      })
      .catch(fail);
    return () => {
      alive = false;
    };
  }, [api, current, revision, fail, showPreview]);

  const push = useMemo(
    () =>
      debounced((name: string, text: string) => {
        pending.current = false;
        // Un fichier vide vaudrait suppression : on attend du texte.
        if (text === "") return;
        sent.current = text;
        api
          .setWidget(name, text)
          .then(() => {
            onChanged();
            return refresh();
          })
          .then(() => showPreview(name))
          .catch(fail);
      }, PREVIEW_DELAY_MS),
    [api, onChanged, refresh, showPreview, fail],
  );
  useEffect(() => () => push.cancel(), [push]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || naming) return;
      e.preventDefault();
      e.stopPropagation();
      onClose();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [onClose, naming]);

  const commitName = () => {
    if (!naming || !isWidgetName(naming.value)) return;
    const { from, value } = naming;
    setNaming(null);
    const done = from ? api.renameWidget(from, value) : api.setWidget(value, skeleton(value));
    Promise.resolve(done)
      .then(() => {
        onChanged();
        setCurrent(value);
        return refresh();
      })
      .catch(fail);
  };

  const remove = () => {
    if (!current) return;
    api
      .removeWidget(current)
      .then(() => {
        onChanged();
        setCurrent(null);
        setPreview(null);
        return refresh();
      })
      .catch(fail);
  };

  const currentStatus = statuses?.find((s) => s.name === current);
  const previewUrl = useMemo(
    () => (preview?.svg ? URL.createObjectURL(new Blob([preview.svg], { type: "image/svg+xml" })) : null),
    [preview],
  );
  useEffect(() => () => {
    if (previewUrl) URL.revokeObjectURL(previewUrl);
  }, [previewUrl]);

  return (
    <div className="picker-scrim" onPointerDown={onClose}>
      <div className="picker workbench" role="dialog" aria-modal="true" aria-labelledby="workbench-title" onPointerDown={(e) => e.stopPropagation()}>
        <div className="picker-head">
          <h2 id="workbench-title">{t("ui-widgets-title")}</h2>
          <span className="caption" />
          <button className="object token" aria-label={t("ui-widgets-close")} title={t("ui-widgets-close")} onClick={onClose}>{Icon.close}</button>
        </div>
        <div className="workbench-body">
          <nav className="workbench-list" aria-label={t("ui-widgets-files")}>
            <ul>
              {(statuses ?? []).map((s) => {
                const state = stateOf(s);
                return (
                  <li key={s.name}>
                    <button className="workbench-file" aria-current={s.name === current} onClick={() => setCurrent(s.name)}>
                      <span className={`pill ${state}`} role="img" aria-label={t(state === "valid" ? "ui-widgets-valid" : "ui-widgets-errors")} />
                      {widgetLabel(s.name)}
                    </button>
                  </li>
                );
              })}
            </ul>
            {statuses?.length === 0 && <p className="caption">{t("ui-widgets-empty")}</p>}
            {naming ? (
              <form className="workbench-name" onSubmit={(e) => { e.preventDefault(); commitName(); }}>
                <input
                  className="select"
                  autoFocus
                  aria-label={t("ui-widgets-name")}
                  aria-invalid={naming.value !== "" && !isWidgetName(naming.value)}
                  value={naming.value}
                  onChange={(e) => setNaming({ ...naming, value: e.target.value })}
                  onKeyDown={(e) => { if (e.key === "Escape") setNaming(null); }}
                />
              </form>
            ) : (
              <div className="workbench-actions">
                <button className="btn" onClick={() => setNaming({ from: null, value: "" })}>{t("ui-widgets-new")}</button>
                <button className="btn" disabled={!current || current === COMMON} onClick={() => current && setNaming({ from: current, value: current })}>{t("ui-widgets-rename")}</button>
                <button className="btn danger" disabled={!current} onClick={remove}>{t("ui-widgets-delete")}</button>
              </div>
            )}
          </nav>
          <section className="workbench-editor">
            {current && source !== null ? (
              <TypstEditor
                key={`${current}:${editorKey}`}
                text={source}
                diagnostics={fileDiagnostics(currentStatus, current)}
                label={widgetLabel(current)}
                onChange={(text) => {
                  pending.current = true;
                  push(current, text);
                }}
              />
            ) : (
              <p className="caption">{t("ui-widgets-pick")}</p>
            )}
          </section>
          <aside className="workbench-preview" aria-label={t("ui-widgets-preview")}>
            {previewUrl ? <img src={previewUrl} alt={t("ui-widgets-preview")} /> : <p className="caption">{t("ui-widgets-no-preview")}</p>}
            <ul className="workbench-diagnostics">
              {(preview?.diagnostics ?? []).map((d, i) => (
                <li key={i} className={d.error ? "error" : undefined}>
                  {d.line != null ? `${d.file ?? ""}:${d.line}:${d.column ?? 1} ` : ""}{d.message}
                </li>
              ))}
            </ul>
          </aside>
        </div>
      </div>
    </div>
  );
}
```

`App.tsx` :
- après `:50` : `const WidgetWorkbench = lazy(() => import("./WidgetWorkbench"));`
- après `:388` : `const [widgetsOpen, setWidgetsOpen] = useState(false);` et `const widgetsEntry: MenuEntry = { kind: "item", label: t("ui-widgets-menu"), onSelect: () => setWidgetsOpen(true) };` (placé avec `gridEntry`, `:703-714`).
- gardes clavier `:254` (copier-coller) et `:536` (raccourcis) : ajouter `widgetsOpen` à côté de `picking`.
- menus : mode modèles, après l'entrée « Enregistrer une copie » ; mode deck, après `ui-toolbar-templates` (`:733`) : `widgetsEntry,`.
- après le bloc `TemplatePicker` (`:921`) :

```tsx
      {widgetsOpen && (
        <Suspense fallback={null}>
          <WidgetWorkbench
            api={api}
            revision={`${snap.templates?.version ?? ""}|${snap.editing?.base ?? ""}`}
            onChanged={() => run(api.snapshot())}
            onClose={() => setWidgetsOpen(false)}
            onError={(m) => say(m, true)}
          />
        </Suspense>
      )}
```

`styles.css`, à la fin :

```css
/* Panneau Widgets : fichiers, éditeur Typst, aperçu. */
.workbench { width: min(1280px, 94vw); height: min(780px, 90vh); }
.workbench-body { flex: 1; min-height: 0; display: grid; grid-template-columns: 200px 1fr 360px; gap: var(--space-4); }
.workbench-list ul { list-style: none; margin: 0; padding: 0; }
.workbench-file { display: flex; align-items: center; gap: var(--space-2); width: 100%; padding: 6px 8px; border: 0; border-radius: var(--radius-sm); background: transparent; color: inherit; text-align: start; }
.workbench-file[aria-current="true"] { background: var(--accent-soft); }
.workbench-actions { display: flex; flex-wrap: wrap; gap: var(--space-2); margin-top: var(--space-3); }
.pill { flex: none; width: 8px; height: 8px; border-radius: 50%; }
.pill.valid { background: color-mix(in oklab, #2f8a4f 80%, var(--ink)); }
.pill.errors { background: var(--danger); }
.workbench-editor { min-width: 0; overflow: auto; border-radius: var(--radius-sm); background: var(--paper-200); }
.typst-editor .cm-editor { height: 100%; font: 13px/20px var(--mono, ui-monospace, monospace); }
.workbench-preview { min-width: 0; overflow: auto; }
.workbench-preview img { width: 100%; border-radius: var(--radius-sm); box-shadow: var(--shadow-sheet-lifted); }
.workbench-diagnostics { margin: var(--space-2) 0 0; padding: 0; list-style: none; font-size: 12px; }
.workbench-diagnostics .error { color: var(--danger); }
.cm-typ-comment { color: color-mix(in oklab, var(--ink) 50%, transparent); font-style: italic; }
.cm-typ-string { color: color-mix(in oklab, #2f8a4f 85%, var(--ink)); }
.cm-typ-keyword, .cm-typ-hash { color: var(--accent); font-weight: 600; }
.cm-typ-param { color: color-mix(in oklab, #8a4f2f 85%, var(--ink)); }
.cm-diag-underline { text-decoration: underline wavy var(--danger); text-underline-offset: 3px; }
.cm-diag-marker { color: var(--danger); cursor: help; }
```

  Fluent, `ui.ftl`, à la fin (après `ui-pack-notice-close`) :
  - fr :
    ```
    ui-widgets-menu = Widgets
    ui-widgets-title = Widgets du pack
    ui-widgets-files = Fichiers de widgets
    ui-widgets-common = Commun
    ui-widgets-new = Nouveau
    ui-widgets-rename = Renommer
    ui-widgets-delete = Supprimer
    ui-widgets-name = Nom du widget (minuscules, chiffres et tirets)
    ui-widgets-valid = Valide
    ui-widgets-errors = Erreurs
    ui-widgets-empty = Ce pack n'a pas encore de widget.
    ui-widgets-pick = Choisissez un fichier, ou créez un widget.
    ui-widgets-preview = Aperçu du widget
    ui-widgets-no-preview = Pas d'aperçu
    ui-widgets-close = Fermer
    ```
  - en :
    ```
    ui-widgets-menu = Widgets
    ui-widgets-title = Pack widgets
    ui-widgets-files = Widget files
    ui-widgets-common = Shared
    ui-widgets-new = New
    ui-widgets-rename = Rename
    ui-widgets-delete = Delete
    ui-widgets-name = Widget name (lowercase letters, digits and hyphens)
    ui-widgets-valid = Valid
    ui-widgets-errors = Errors
    ui-widgets-empty = This pack has no widget yet.
    ui-widgets-pick = Pick a file, or create a widget.
    ui-widgets-preview = Widget preview
    ui-widgets-no-preview = No preview
    ui-widgets-close = Close
    ```
  - traduire les 18 autres.

- [ ] **Step 4 :** `cd ui && bun test && bun run typecheck` → PASS ; `cargo nextest run -p oku-slides-i18n` → PASS.

- [ ] **Step 5 : commit**

```bash
git add ui/src crates/oku-slides-i18n
```
```bash
git commit -m "feat(ui) : panneau Widgets, éditeur Typst et aperçu en direct (t-329)"
```

---

### Task 12 : régression, documentation, consolidation et fermeture

**Files:**
- Create: `crates/oku-slides-session/tests/regressions/issue_0329_widget_casse_garde_les_autres.rs`
- Modify: `crates/oku-slides-session/tests/regressions/main.rs` (`mod`)
- Modify: `crates/oku-slides-core/ARCHITECTURE.md:25-30`, `crates/oku-slides-render/ARCHITECTURE.md:21-25`, `crates/oku-slides-render-worker/ARCHITECTURE.md:6-9`, `crates/oku-slides-session/ARCHITECTURE.md:12-18,41`
- Vérifier : `crates/oku-slides-pdf/ARCHITECTURE.md` (Task 1), `crates/oku-slides-mcp/ARCHITECTURE.md` (Task 10)

**Interfaces:**
- Consumes : tout ce qui précède.
- Produces : la régression t-329 ; documentation à jour.

- [ ] **Step 1 : régression** — `issue_0329_widget_casse_garde_les_autres.rs` :

```rust
//! t-329 : un widget qui ne compile pas n'empêche pas les autres widgets du
//! même pack de se rendre dans le deck.
//!
//! Avant t-329, un pack était un seul `widgets.typ` : une erreur dans un
//! widget faisait tomber tout le pack, et le deck revenait aux widgets de base.

use std::collections::BTreeMap;

use oku_slides_core::{Op, WidgetFile};
use oku_slides_session::{Library, PackManifest, Request, Storage};
use serde_json::json;
use speculoos::prelude::*;
use uuid::Uuid;

use crate::common::{data_dir, session_in};

const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
"#;

const BROKEN: &str = "#import \"/oku-slides/lib.typ\": widget\n#let widget = widget(\"casse\", (args, ctx) => [\n";

fn wf(name: &str) -> WidgetFile {
    WidgetFile::parse(name).unwrap_or_else(|| panic!("nom invalide : {name}"))
}

#[test]
fn a_broken_widget_does_not_keep_the_other_widgets_of_its_pack_from_rendering() {
    let data = data_dir();
    let id = Uuid::new_v4();
    let manifest = PackManifest {
        id,
        name: "Mixte".into(),
        description: String::new(),
        parent: None,
        created: "2026-10-02T00:00:00Z".into(),
        templates: Vec::new(),
    };
    let widgets = BTreeMap::from([(wf("quote"), QUOTE.to_owned()), (wf("casse"), BROKEN.to_owned())]);
    let files = manifest.to_files(widgets).unwrap_or_else(|e| panic!("{e}"));
    let storage = Storage::local().unwrap_or_else(|e| panic!("{e}"));
    Library::new(Some(data.join("templates")), storage).add_version(id, &files).unwrap_or_else(|e| panic!("{e}"));

    let mut s = session_in(&data);
    s.open(&crate::common::fixture_path()).unwrap_or_else(|e| panic!("{e}"));
    let ops: Vec<Op> = serde_json::from_value(json!([
        { "op": "set_templates", "templates": { "id": id, "version": files.version() } },
        { "op": "add_element",
          "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
          "element": { "id": "q", "kind": "quote", "params": { "body": [{ "runs": [{ "t": "Tout coule." }] }] } } }
    ]))
    .unwrap_or_else(|e| panic!("{e}"));
    s.apply("Pack", ops, None).unwrap_or_else(|e| panic!("{e}"));

    let widgets: serde_json::Value =
        serde_json::from_str(&s.handle(Request::Widgets).unwrap_or_else(|e| panic!("{e}"))).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(widgets["widgets"].get("quote").is_some()).is_true();
    assert_that!(widgets["widgets"].get("casse").is_none()).is_true();
    // Le widget valide est rendu dans le deck : sa boîte existe.
    let slide = s.slide("s-constat").unwrap_or_else(|e| panic!("{e}"));
    assert_that!(slide.targets.iter().any(|t| t.id == "q" && t.kind == "quote")).is_true();
    let status = s.widget_status().unwrap_or_else(|e| panic!("{e}"));
    let casse = status.iter().find(|w| w.name == wf("casse")).unwrap_or_else(|| panic!("casse absent"));
    assert_that!(casse.diagnostics.iter().any(|d| d.error && d.line.is_some())).is_true();
}
```

  `main.rs` : `mod issue_0329_widget_casse_garde_les_autres;` (après `issue_0328_…`). `cargo nextest run -p oku-slides-session --test regressions` → PASS (le comportement est livré : ce test garde la règle).

- [ ] **Step 2 : documents** (chacun **sous 50 lignes** : core, render et session en comptent 49, condenser une puce en même temps) :
  - core : puce `PackVersion/…` → « `PackVersion`/`PackRef`/`PackFiles`/`WidgetFile` (`pack.rs`) : version adressée par contenu (sha-256 de `pack.json` à clés triées, `0`, puis nom, `0`, source, `0` par fichier de widget) » ; « Zip v5 : …, `packs/<hex>/{pack.json, widgets/<nom>.typ}` en ajout seul, vérifiés à la lecture (`issue-pack-version-mismatch`), nom invalide → `BadName`, unis à la fusion. »
  - render : la puce `probe_registry …` devient « `PackWidgets` : fichiers `widgets/<nom>.typ` (`_commun` partagé) servis sous `/packs/<id>/`, module d'assemblage généré `/packs/<id>/widgets.typ`. `probe_file` (`widget_probe.rs`) : un fichier seul → `FileProbe` (schéma ou diagnostics à ligne et colonne, `SourceAt`), nom déclaré = nom de fichier ; `assemble` : registre des seuls valides. `Registries` mémoïse par pack et par `probe_key`. »
  - render-worker : « `RenderWorker::probe_pack` (`pack_probe.rs`) : sonde par fichier, jamais d'accusation (un délai = un diagnostic), gardée par `probe_key` ; `trial_slot` : emplacement d'aperçu qui n'accuse aucun pack. »
  - session `:12-18` : « `deck_widgets.rs` : la base surchargée par les widgets **valides** de la version embarquée (sonde par fichier, avis `session-widgets-broken`) … » ; nouvelle puce « `widgets_edit.rs` (statut, `set/remove/rename_widget`, mode modèles par `adopt_session_version`), `widgets_deck.rs` (mode deck : version du pack du deck, `SetTemplates`, une étape d'annulation par séance), `widget_preview.rs` (`preview_widget`, emplacement d'essai). `request.rs` : `Request`. » ; disposition de la reprise `:41` : `packs/<hex>.json|<hex>.<nom>.typ`.
- [ ] **Step 3 : consolidation** —
  - `rg -n 'widgets\.typ' crates typst ui/src fixtures --glob '!*.snap'` → seulement le module d'assemblage (`registry.rs`, `world.rs`, leurs tests), la base `typst/oku-slides/src/widgets.typ` et la doc qui les nomme.
  - `rg -n '"version":4|version: 4' crates/oku-slides-core crates/oku-slides-pdf` → seulement le test qui refuse la v4.
  - `rg -n "to_files\(None|to_files\(Some|from_bytes\([^)]*None" crates` → rien.
  - `rg -n "std::fs::" crates/oku-slides-session/src crates/oku-slides-core/src crates/oku-slides-cli/src crates/oku-slides-render/src -g '!*_tests.rs' -g '!test_support.rs'` → rien de nouveau.
  - `wc -l crates/oku-slides-session/src/lib.rs crates/oku-slides-render-worker/src/client.rs ui/src/App.tsx` → sous 1000 (lib.rs sous 900).
  - `rg -n "Co-Authored-By" $(git log --format=%H main..HEAD | head -1)` inutile : `git log main..HEAD --format=%B | rg -c Co-Authored-By` → 0.
- [ ] **Step 4** : `just check` → PASS.
- [ ] **Step 5 : commits**

```bash
git add crates/oku-slides-session/tests/regressions
```
```bash
git commit -m "test(session) : régression t-329, un widget cassé n'empêche pas les autres de se rendre (t-329)"
```

```bash
git add crates/*/ARCHITECTURE.md
```
```bash
git commit -m "docs(core) : packs à fichiers de widgets, sonde par fichier et édition des widgets (t-329)"
```
