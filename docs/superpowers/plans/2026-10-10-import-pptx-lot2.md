# Import PowerPoint — lot 2 : projecteur retrofit et CLI — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `oku-slides import deck.pptx --mode retrofit --pack <id|chemin|nom> [--theme <id|nom>] -o out.oku-slides.pdf [--report r.json]` produit un deck Oku complet et rendable sur un pack et un thème existants, avec un rapport des pertes, des replis, des demandes de widget et de la couverture du contenu.

**Architecture:** La crate pure `oku-slides-import` gagne un projecteur `retrofit::project(&SourceDeck, &RetrofitTarget) -> ImportPlan` : décor répété omis, titres déduits des slides sans placeholder, formes et textes regroupés en cartes, choix déterministe d'un modèle par slide, remplissage des champs, placement en blocs `flow` ou en éléments libres (cadres, ou cellules de la grille du modèle), couleurs projetées sur les jetons du thème cible (ΔE OKLab, accents déduits, contraste WCAG), replis pour les motifs sans widget avec `WidgetRequest`. Le plan est un `Vec<Op>` appliqué en une transaction par `oku_slides_core::apply_all`. La CLI résout pack et thème, construit le `SourceBundle`, applique, rend, enregistre et écrit le rapport. `Template`, `PackManifest` et le calcul des jetons effectifs d'un thème descendent dans `oku-slides-core` pour que la crate d'import reste sans session.

**Tech Stack:** Rust 2024 (rust 1.98 local), `oku-slides-core` (ops, modèle, `WidgetRegistry`, `PackFiles`), `oku-slides-chart` (validation tableaux/graphiques par les ops), `oku-slides-i18n` (Fluent, 20 langues), `clap` + `color-eyre` côté CLI ; tests `speculoos`, `rstest`, `insta` (json), nextest.

**Spec:** `docs/superpowers/specs/2026-10-09-import-pptx-design.md` (« Projecteur retrofit », « `ImportPlan` », « Rapport d'import », « Points d'entrée », « Tests », lot 2). Lot 1 : `docs/superpowers/plans/2026-10-09-import-pptx-lot1.md` (la `SourceDeck` et ses garanties).

## Décisions validées le 2026-10-10 (elles priment sur la spec)

1. **Modèles dans le cœur.** `Template` et `PackManifest` (avec `from_files`, `to_files`, `now_created`) passent de `oku-slides-session/src/pack.rs` à `oku-slides-core/src/pack.rs` ; la session les réexporte (aucun appelant ne change d'import). `PackInfo`, `Relation`, `DEFAULT_PACK`, `GRID_PACK` restent dans la session. La fusion profonde des jetons (`merge`) et `effective_tokens(defaults, theme)` vivent dans `oku-slides-core/src/tokens.rs` ; `theme_schema::merge` les réexporte, la CLI et les aides de test de l'import s'en servent (G11 : un seul calcul).
2. **Pas de pack enfant au lot 2.** Le retrofit émet les `WidgetRequest` (au plan et au rapport) ; la création du pack enfant passe au lot 5, avec le premier widget créé par l'agent. `ImportPlan` n'a pas de champ `pack` au lot 2.
3. **Polices : rien au lot 2.** Le style vient du thème cible ; `fonts.rs` (recherche système, substitution) passe au lot 3.
4. **Couverture du contenu au lot 2** (`coverage.rs`) : rappel des runs de texte, présence des images, égalité des données de tableaux et de graphiques, mesurés sur le `Deck` obtenu. Le lot 4 garde la référence LibreOffice et l'écart visuel.
5. **Choix du modèle** : profil déduit du schéma du widget de la slide d'exemple (`kind`, champs `plain`/`rich` présents, slot `flow` rempli ou non, `grid`, `layout`) et score contre la slide source (`Layout.kind`, rôles et sortes d'items). Égalité → ordre du pack.
6. **Placement** (révisée le 2026-10-10) : avant le choix, les items se regroupent en unités (une forme remplie qui porte des textes devient une carte ; des textes voisins dont les cellules se chevauchent fusionnent). Une slide « titre + un corps » (au plus une unité, ou une rangée d'unités côte à côte de même hauteur, seule ou entre des unités pleine largeur) va en blocs `flow` avec l'agencement du modèle (`default`, `columns`, `split`) ; une rangée entourée d'autres unités devient une pile `stack` en colonnes dans l'agencement `default` ; une forme sans texte n'entre jamais dans un flux. Sinon les unités vont dans `elements`, leur boîte englobante ajustée dans la zone de corps (échelle uniforme, centrée) : sur un modèle libre sans `grid` (Essentiel `libre`), en `frame` (`to_zone`) ; sur un modèle à grille, en `cell` (portage Rust de `snapRect` de `ui/src/grid.ts`) si aucun contenu n'y recouvre un autre, sinon en `frame`. La zone de corps est approchée par les jetons `page` du thème cible.
7. **Slides de titre** : seule la première slide source de layout `title` (ou dont les placeholders sont `ctrTitle`/`subTitle`, ou dont le titre déduit fait au moins 40 pt sans autre texte de titre) remplit les paramètres du deck (`title`, `subtitle`) ; les suivantes deviennent des `chapter`. Une slide sans placeholder de titre a ses titres déduits de la taille et de la position des textes (titre, surtitre, sous-titre).
8. **Pertes signalées** (jamais d'échec) : niveaux de liste aplatis, fusions de cellules aplaties (texte dans la cellule d'ancrage, `null` dans les couvertes), `PercentStacked` → `stack`, soulignement/barré/indices/petites capitales perdus (les capitales `Caps::All` sont écrites en majuscules), rotation/retournement/rognage perdus, couleurs sans jeton proche abandonnées, texte tronqué aux bornes du cœur (`RICH_PARAGRAPHS_MAX`, `RICH_BYTES_MAX`, `PLAIN_MAX`), tableaux au-delà de `MAX_TABLE_ROWS`/`MAX_TABLE_COLUMNS`, `Opaque` sans repli image, en-tête ou pied répété omis (`RepeatedDecor`, une fois par motif), contenu sans slot (`NoSlot`).
9. **Aucune surcharge de jetons** (décision 4 de la spec) : seules des clés existantes du `color` fusionné du thème cible sont émises ; jamais de nom inconnu (le rendu Typst panique sur une couleur inconnue). Les couleurs saturées du deck sans jeton proche prennent, par usage décroissant, `accent` puis les autres clés `accent-*` du thème ; une couleur de texte n'est émise que lisible sur son fond (contraste WCAG ≥ 3:1).
10. **`--mode hifi`** est reconnu par la CLI et refusé avec un message localisé « disponible au lot 3 ».

## Global Constraints

- Workspace : Rust 2024 ; `cargo clippy --workspace --all-targets -- -D warnings` ; `unwrap_used`/`expect_used` refusés hors tests ; `#[expect(..., reason = "...")]` plutôt que `#[allow]` ; rustfmt largeur 120 ; seuil `too-many-arguments` 6 ; `missing_debug_implementations` en warn.
- `oku-slides-import` ne dépend ni de `oku-slides-session`, ni de `oku-slides-render`, ni de la CLI. Dépendances autorisées : `oku-slides-core`, `oku-slides-chart` (si besoin des constantes de bornes), `oku-slides-i18n`, `serde`, `serde_json`, `snafu`, et celles du lot 1.
- Pas de ré-export à plat de `retrofit` : `lib.rs` exporte `pub mod retrofit`, `pub mod plan`, `pub mod report`, et ré-exporte seulement `ImportPlan`, `ImportReport`, `RetrofitTarget`.
- Les ids de slides et d'éléments sont choisis par le projecteur, déterministes : slide `s<n>` (numéro source), éléments `<slide>-<k>` (rang dans la slide). Aucun aléa, aucune horloge dans le projecteur.
- Unités : la `SourceDeck` est en pt de page source ; le deck Oku en pt sur la page 1920 × 1080, cadres et cellules relatifs à la zone de corps. Échelle `1920 / source.size.w`.
- Couleurs émises : nom de clé du `color` du thème cible ou rien ; `#rrggbb` jamais émis au lot 2 (décision 4 de la spec : style du thème cible), sauf dans les données de graphique où aucune couleur n'est émise.
- Tout texte venant du fichier passe déjà par `clip` (lot 1) ; les bornes du cœur (`PLAIN_MAX`, `RICH_PARAGRAPHS_MAX`, `RICH_BYTES_MAX`, `oku_slides_chart` `MAX_TABLE_*`) sont respectées à l'émission, avec une perte au rapport en cas de coupe.
- Le plan est valide par construction : chaque test de bout en bout applique `plan.ops` avec `apply_all(&mut Deck::empty() + modèle, ops, &registry)` puis `validate(&deck, &registry, Some(&assets))` sans `Issue`.
- Décor (G11) : la projection et la couverture omettent les mêmes items par le même prédicat (`retrofit::decor::Decor::skips`).
- Messages : toute chaîne vue par l'utilisateur passe par Fluent ; nouvelles clés dans les 20 locales (`ar bn de en es fr hi it ja ko nl pl pt ru th tr uk vi zh-Hans zh-Hant`), mêmes variables partout (`every_locale_has_exactly_the_reference_keys`). Clés du rapport dans `document.ftl` (préfixe `import-`), clés de la CLI dans `cli.ftl` (préfixe `cli-`).
- Fichiers ≤ 500 lignes ; tests à côté (`foo_tests.rs`, `#[cfg(test)] #[path = "foo_tests.rs"] mod foo_tests;`) ; noms `<unité>_<scénario>_<résultat>` ; régressions dans `tests/regressions/issue_692_*.rs`.
- Snapshots : `INSTA_UPDATE=new cargo nextest run …`, lecture du `.snap.new`, puis `cargo insta accept --manifest-path <crate>/Cargo.toml` ; jamais `INSTA_UPDATE=always`.
- Commits : conventionnels en français, `(t-692)` en fin de sujet ; message dans un fichier, `git commit -F <fichier>` ; `git add` séparé ; aucune ligne d'attribution.
- Tests de la CLI : `OKU_SLIDES_PATHS__DATA` (et `__DECKS`) vers un dossier temporaire (le helper `oku_slides()` des tests le fait).

## Structure

```
crates/oku-slides-core/src/pack.rs          + Template, PackManifest, now_created (déplacés)
crates/oku-slides-core/src/tokens.rs        merge, effective_tokens (jetons effectifs d'un thème)
crates/oku-slides-session/src/pack.rs       − ces types, ré-exportés depuis le cœur
crates/oku-slides-import/src/
  lib.rs                 pub mod retrofit, plan, report ; ré-exports ImportPlan, ImportReport, RetrofitTarget
  plan.rs                ImportPlan { ops, assets, widget_requests, report }, WidgetRequest
  report.rs              ImportReport, SlideReport, Loss, LossKind (Localize), Coverage
  pattern.rs             signature de motif (multiset des sortes + disposition normalisée)
  coverage.rs            couverture du contenu : SourceDeck × Deck
  retrofit/
    mod.rs               project(&SourceDeck, &RetrofitTarget) -> ImportPlan ; RetrofitTarget, PageTokens, ColorTokens
    color.rs             sRGB → OKLab, ΔE, table du schéma → jetons, accents déduits, contraste WCAG
    zone.rs              zone de corps, géométrie de grille, ajustement (Fit), snap d'un cadre en cellule
    text.rs              TextBox → Vec<Paragraph> / String plain, bornes, couleurs lisibles
    table.rs             Table source → option de tableau (JSON) + pertes
    chart.rs             Chart source → option de graphique canonique (JSON) + pertes
    cluster.rs           unités de contenu : cartes (forme + textes), textes fusionnés
    content.rs           Unit → Element (text, card, image, shape, filet, table, chart)
    group.rs             repli stack des groupes et SmartArt, dit s'il perd quelque chose
    heading.rs           titres des placeholders, sinon déduits (titre, surtitre, sous-titre, couverture)
    template.rs          profils des modèles, forme d'une slide (unités, rangée), score
    decor.rs             décor omis par règle (date, pied, numéro ; en-tête et pied répétés)
    slide.rs             une slide source → ops (AddSlide, SetParam, SetNotes…)
  test_support.rs        registre de base, modèles Essentiel/Grilles, jetons Papier/Pro (cfg(test))
crates/oku-slides-import/tests/retrofit.rs  bout en bout sur minimal, kitchen, alto (apply_all + validate + snapshots)
crates/oku-slides-cli/src/import.rs         commande import : résolution pack/thème, bundle, application, rendu, rapport
crates/oku-slides-cli/tests/import.rs       bout en bout : pptx → .oku-slides.pdf rouvrable
crates/oku-slides-i18n/src/locales/*/{document,cli}.ftl   nouvelles clés
```

---

### Task 1: `Template`, `PackManifest` et les jetons effectifs descendent dans le cœur

**Écarts d'interface :** `PackManifest::info(&self, version, builtin) -> PackInfo` devient `PackInfo::of(manifest: &PackManifest, version: PackVersion, builtin: bool) -> PackInfo` (session) : une méthode inhérente ne peut pas être définie hors de la crate du type, et `PackInfo` (avec `DEFAULT_PACK`) reste dans la session. Deux appelants changent (`library.rs`, `templates_mode.rs`) ; aucun hors de la session.

La crate d'import doit lire les modèles d'un pack (`Template`, la slide d'exemple) sans dépendre de la session. Le contenu de `pack.json` — `Template`, `PackManifest` (avec la lecture tolérante de `examples` et son journal de diagnostics) et `now_created` — descend tel quel dans `oku-slides-core/src/pack.rs`, à côté de `PackFiles` qu'il lit et écrit. Le cœur gagne la dépendance `time` (déjà dans l'espace de travail, fonctions `formatting` et `macros`). La session les ré-exporte depuis `pack.rs` : `oku_slides_session::{PackManifest, Template}`, `crate::pack::{PackManifest, Template, now_created}` et `oku_slides_session::pack::now_created` gardent leurs chemins ; l'impl `VersionManifest for PackManifest` reste dans la session (trait local, type étranger : permis). Les tests du manifeste suivent le code dans le cœur ; la session ne garde que ceux de `PackInfo`.

Les jetons effectifs d'un thème (les défauts du rendu, puis ceux du thème, fusionnés objet par objet) se calculent aujourd'hui dans la session (`theme_schema::merge`) ; la CLI et l'import en ont besoin aussi. La fusion `merge` et `effective_tokens(defaults, theme)` vivent désormais dans un module `oku_slides_core::tokens` ; `theme_schema::merge` devient un ré-export (ses appelants ne changent pas), et la CLI (tâche 11) comme les aides de test de l'import (tâches 3 et 9) appellent `effective_tokens` : un seul calcul (G11).

**Files:**
- Modify: `crates/oku-slides-core/Cargo.toml` (`time.workspace = true`), `crates/oku-slides-core/src/pack.rs` (doc du module, imports, `Template`, `PackManifest`, `lenient_examples`, `diagnose`, `now_created`), `crates/oku-slides-core/src/pack_tests.rs` (tests du manifeste), `crates/oku-slides-core/src/lib.rs` (ré-exports)
- Create: `crates/oku-slides-core/src/tokens.rs`, `crates/oku-slides-core/src/tokens_tests.rs`
- Modify: `crates/oku-slides-session/src/pack.rs` (réécrit : ré-exports, `PackInfo::of`), `crates/oku-slides-session/src/pack_tests.rs` (réécrit), `crates/oku-slides-session/src/library.rs` (`packs`), `crates/oku-slides-session/src/templates_mode.rs` (`templates_view`), `crates/oku-slides-session/src/theme_schema.rs` (`merge` ré-exporté du cœur)

**Interfaces:**
- Consumes : `oku_slides_core::{PackFiles, PackVersion, WidgetFile, Slide}` (inchangés).
- Produces :

```rust
// oku_slides_core (racine) et oku_slides_core::pack
pub struct Template { pub id: String, pub name: String, pub description: String, pub slide: Slide }
pub struct PackManifest {
    pub id: Uuid, pub name: String, pub description: String, pub parent: Option<PackVersion>,
    pub created: String, pub templates: Vec<Template>, pub examples: BTreeMap<WidgetFile, Map<String, Value>>,
}
impl PackManifest {
    pub fn from_files(files: &PackFiles) -> serde_json::Result<Self>;
    pub fn to_files(&self, widgets: BTreeMap<WidgetFile, String>) -> serde_json::Result<PackFiles>;
}
pub fn now_created() -> String; // UTC à la seconde, `AAAA-MM-JJTHH:MM:SSZ`

// oku_slides_core::tokens
pub fn merge(base: &mut Map<String, Value>, over: &Map<String, Value>);
pub fn effective_tokens(defaults: &Map<String, Value>, theme: &Map<String, Value>) -> Map<String, Value>;

// oku_slides_session::pack
pub use oku_slides_core::{PackManifest, Template, now_created};
impl PackInfo { pub fn of(manifest: &PackManifest, version: PackVersion, builtin: bool) -> Self; }
// oku_slides_session::theme_schema
pub use oku_slides_core::tokens::merge;
```

- Règles : déplacement sans changement de comportement (mêmes dérivations serde, même lecture tolérante d'`examples`, mêmes diagnostics) ; `DEFAULT_PACK`, `GRID_PACK`, `PackInfo`, `Relation` restent dans la session. `merge` : deux objets se fusionnent clé par clé, toute autre valeur remplace (comme `merge` côté Typst) ; `effective_tokens` rend une copie des défauts avec le thème fusionné par-dessus, sans toucher aux défauts.

- [ ] **Step 1 : tests qui échouent**

Ajouter à la fin de `crates/oku-slides-core/src/pack_tests.rs` (les aides `object`, `wf`, `files` et les imports `BTreeMap`, `json`, `Map`, `Value`, `Uuid` y sont déjà) :

```rust
fn manifest(name: &str) -> PackManifest {
    PackManifest {
        id: Uuid::new_v4(),
        name: name.into(),
        description: String::new(),
        parent: None,
        created: "2026-10-01T10:00:00Z".into(),
        templates: Vec::new(),
        examples: BTreeMap::new(),
    }
}

#[test]
fn pack_manifest_round_trips_through_pack_files() {
    let m = manifest("Mien");
    let files = m.to_files(files(&[("kpi", "#let widget = none")])).unwrap();
    assert_that!(PackManifest::from_files(&files).unwrap()).is_equal_to(&m);
    assert_that!(files.widget(&wf("kpi"))).is_equal_to(Some("#let widget = none"));
}

#[test]
fn pack_manifest_with_a_template_round_trips_through_pack_files() {
    let mut m = manifest("Mien");
    m.templates.push(Template {
        id: "texte".into(),
        name: "Texte".into(),
        description: String::new(),
        slide: Slide::new("s-texte", "content"),
    });
    let files = m.to_files(BTreeMap::new()).unwrap();
    assert_that!(PackManifest::from_files(&files).unwrap().templates).is_equal_to(&m.templates);
}

#[test]
fn pack_manifest_without_uuid_id_is_refused() {
    let json = json!({ "id": "essentiel", "name": "E", "parent": null, "created": "x", "templates": [] });
    let files = PackFiles::new(object(json), BTreeMap::new());
    assert_that!(PackManifest::from_files(&files).is_err()).is_true();
}

#[test]
fn now_created_is_utc_to_the_second() {
    let s = now_created();
    assert_that!(s.len()).is_equal_to(20);
    assert_that!(s.ends_with('Z') && s.as_bytes()[10] == b'T').is_true();
}

fn with_values(m: &mut PackManifest, widget: &str, values: Value) {
    m.examples.insert(wf(widget), object(values));
}

#[test]
fn pack_manifest_examples_round_trip_through_pack_files() {
    let mut m = manifest("Mien");
    with_values(&mut m, "kpi", json!({ "value": "12" }));
    let files = m.to_files(files(&[("kpi", "#let widget = none")])).unwrap();
    assert_that!(PackManifest::from_files(&files).unwrap()).is_equal_to(&m);
    assert_that!(files.manifest()["examples"]["kpi"]["value"].as_str()).is_equal_to(Some("12"));
}

#[test]
fn pack_manifest_examples_change_the_version() {
    let mut m = manifest("Mien");
    let before = m.to_files(BTreeMap::new()).unwrap().version();
    with_values(&mut m, "kpi", json!({ "value": "12" }));
    let after = m.to_files(BTreeMap::new()).unwrap().version();
    assert_that!(after).is_not_equal_to(before);
}

fn read_examples(examples: Value) -> PackManifest {
    let mut json = serde_json::to_value(manifest("Mien")).unwrap();
    json["examples"] = examples;
    PackManifest::from_files(&PackFiles::new(object(json), BTreeMap::new())).unwrap()
}

/// Un `pack.json` d'avant t-541 (une liste de cas nommés par widget) se lit : le
/// premier cas donne les valeurs du widget, les autres sont oubliés.
#[test]
fn pack_manifest_with_the_old_example_cases_keeps_the_first_ones_values() {
    let m = read_examples(json!({
        "kpi": [{ "name": "Court", "values": { "value": "12" } }, { "name": "Long", "values": { "value": "1 234" } }]
    }));
    assert_that!(m.examples.get(&wf("kpi")).cloned()).is_equal_to(Some(object(json!({ "value": "12" }))));
}

/// Un widget aux valeurs vides (ancien sans cas, premier cas sans valeurs, objet vide) n'a pas d'entrée.
#[test]
fn pack_manifest_with_empty_values_keeps_no_entry() {
    let m = read_examples(json!({ "kpi": [], "quote": [{ "name": "Exemple", "values": {} }], "carte": {} }));
    assert_that!(m.examples.is_empty()).is_true();
}

/// Une entrée mal formée est écartée ; les autres restent.
#[test]
fn pack_manifest_with_a_malformed_example_entry_keeps_the_others() {
    let m = read_examples(json!({ "Mauvais Nom": { "value": "1" }, "kpi": 3, "carte": { "title": "T" } }));
    assert_that!(m.examples.keys().cloned().collect::<Vec<_>>()).is_equal_to(vec![wf("carte")]);
}

/// Sans valeurs, `pack.json` n'a pas de clé `examples` : les versions existantes gardent leur empreinte.
#[test]
fn pack_manifest_without_examples_writes_no_examples_key() {
    let files = manifest("Mien").to_files(BTreeMap::new()).unwrap();
    assert_that!(files.manifest().contains_key("examples")).is_false();
}
```

`crates/oku-slides-core/src/tokens_tests.rs` :

```rust
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use super::{effective_tokens, merge};

fn map(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(m) => m,
        _ => panic!("objet attendu"),
    }
}

#[test]
fn merge_of_nested_objects_keeps_the_keys_it_does_not_override() {
    let mut base = map(json!({"color": {"ink": "#000000", "bg": "#ffffff"}, "page": {"width": 1920}}));
    merge(&mut base, &map(json!({"color": {"ink": "#222222"}})));
    assert_that!(Value::Object(base))
        .is_equal_to(json!({"color": {"ink": "#222222", "bg": "#ffffff"}, "page": {"width": 1920}}));
}

#[test]
fn merge_of_a_non_object_replaces_the_value() {
    let mut base = map(json!({"component": {"title": {"rule": {"height": 5}}}, "size": {"body": 32}}));
    merge(&mut base, &map(json!({"component": {"title": {"rule": null}}, "size": 40})));
    assert_that!(Value::Object(base)).is_equal_to(json!({"component": {"title": {"rule": null}}, "size": 40}));
}

#[test]
fn effective_tokens_layer_the_theme_over_the_defaults_without_touching_them() {
    let defaults = map(json!({"color": {"ink": "#000000", "accent": "#2f5fb3"}}));
    let tokens = effective_tokens(&defaults, &map(json!({"color": {"accent": "#00bef4", "white": "#ffffff"}})));
    assert_that!(Value::Object(tokens))
        .is_equal_to(json!({"color": {"ink": "#000000", "accent": "#00bef4", "white": "#ffffff"}}));
    assert_that!(defaults.get("color").and_then(|c| c.get("accent"))).is_equal_to(Some(&json!("#2f5fb3")));
}
```

Remplacer `crates/oku-slides-session/src/pack_tests.rs` par (les tests du manifeste sont partis dans le cœur ; `manifest` vient de `crate::test_support`, inchangé) :

```rust
use speculoos::prelude::*;

use super::*;

use crate::test_support::manifest;

#[test]
fn pack_info_of_marks_the_default_pack() {
    let m = manifest(DEFAULT_PACK, "Essentiel", None, "2026-10-01T00:00:00Z");
    let v = m.to_files(std::collections::BTreeMap::new()).unwrap().version();
    let info = PackInfo::of(&m, v.clone(), true);
    assert_that!((info.id, info.version, info.default, info.builtin)).is_equal_to((DEFAULT_PACK, v, true, true));
}

#[test]
fn pack_info_of_counts_the_templates_of_a_user_pack() {
    let mut m = manifest(uuid::Uuid::new_v4(), "Mien", None, "2026-10-01T00:00:00Z");
    m.templates.push(Template {
        id: "texte".into(),
        name: "Texte".into(),
        description: String::new(),
        slide: oku_slides_core::Slide::new("s-texte", "content"),
    });
    let v = m.to_files(std::collections::BTreeMap::new()).unwrap().version();
    let info = PackInfo::of(&m, v, false);
    assert_that!((info.default, info.builtin, info.count)).is_equal_to((false, false, 1));
}
```

- [ ] **Step 2 : lancer, constater l'échec**

```bash
export CARGO_TARGET_DIR=$PWD/target
cargo nextest run -p oku-slides-core pack_tests
```

Attendu : échec de compilation, `cannot find type `PackManifest` in this scope`, `cannot find struct, variant or union type `Template``, `cannot find function `now_created``, `failed to resolve: use of undeclared type `Slide``. (Côté session, `PackInfo::of` n'existe pas encore : `no function or associated item named `of``. Les tests de `tokens` entrent avec leur module, au Step 3 : `cargo nextest run -p oku-slides-core tokens` les voit alors passer.)

- [ ] **Step 3 : implémentation**

`crates/oku-slides-core/Cargo.toml`, dans `[dependencies]`, après `tracing.workspace = true` :

```toml
time.workspace = true
```

`crates/oku-slides-core/src/pack.rs` : la doc du module et les imports deviennent (début du fichier, jusqu'à `PREFIX` exclu) :

```rust
//! Packs de modèles portables : identité, version et fichiers.
//!
//! Un pack a un id ([`PackRef::id`], UUID v4) et des versions adressées par
//! contenu ([`PackVersion`]). Une version : un `pack.json` et des fichiers de
//! widgets `widgets/<nom>.typ` ([`WidgetFile`], `_commun` pour le code
//! partagé). Contenu canonique : `pack.json` en JSON compact à clés triées,
//! un octet `0`, puis pour chaque fichier dans l'ordre des noms : nom, `0`,
//! source, `0`. La version en est le sha-256, écrit `sha256:<hex>`. Une
//! source ne contient pas d'octet `0` (refusée à la lecture et à l'écriture),
//! et le JSON compact l'échappe : le contenu canonique est sans ambiguïté.
//!
//! Un thème ([`ThemeRef`]) suit le même modèle : une version de thème est un
//! [`PackFiles`] dont le manifeste est `theme.json`, sans fichier de widget.
//!
//! Le contenu d'une version de pack, `pack.json`, se lit en [`PackManifest`] :
//! des modèles ([`Template`], des slides complètes, nommées), indépendants du
//! thème (ils ne parlent qu'en jetons).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::{Mutex, PoisonError};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use time::macros::format_description;
use uuid::Uuid;

use crate::error::{PackVersionError, PackVersionSnafu, WidgetFileError, WidgetFileSnafu};
use crate::model::Slide;
```

Puis, entre la fonction `sorted` et `#[cfg(test)] #[path = "pack_tests.rs"] mod pack_tests;` (fin du fichier), le code déplacé de la session — `Template`, `PackManifest`, `StoredValues`, `StoredCase`, `lenient_examples`, `DIAGNOSED_MAX`, `diagnose`, `impl PackManifest` sans `info`, `now_created` :

```rust
/// Un modèle : une slide complète, nommée.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub slide: Slide,
}

/// `pack.json` : un pack dans une version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackManifest {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Version dont celle-ci est issue ; `None` pour une première version.
    #[serde(default)]
    pub parent: Option<PackVersion>,
    /// Création, UTC à la seconde : `AAAA-MM-JJTHH:MM:SSZ`.
    pub created: String,
    pub templates: Vec<Template>,
    /// Valeurs d'exemple de chaque widget : celles de son aperçu et de ses réglages.
    ///
    /// Les valeurs ont la forme de celles de l'aperçu (texte riche : tableau de
    /// paragraphes) ; une valeur égale à la valeur neutre n'est pas gardée. Un
    /// widget sans valeurs n'a pas d'entrée ; vide, la clé n'est pas écrite (les
    /// versions sans valeurs gardent leur empreinte). Lu aussi dans sa forme
    /// d'avant t-541 (voir [`lenient_examples`]).
    #[serde(default, deserialize_with = "lenient_examples", skip_serializing_if = "BTreeMap::is_empty")]
    pub examples: BTreeMap<WidgetFile, Map<String, Value>>,
}

/// Les valeurs d'exemple d'un widget dans `pack.json`, telles qu'écrites.
#[derive(Deserialize)]
#[serde(untagged)]
enum StoredValues {
    /// Forme actuelle : les valeurs.
    Values(Map<String, Value>),
    /// Forme d'avant t-541 : une liste de cas nommés (`{"name", "values"}`).
    Cases(Vec<StoredCase>),
}

/// Un cas nommé d'avant t-541 : seules ses valeurs comptent.
#[derive(Deserialize)]
struct StoredCase {
    #[serde(default)]
    values: Map<String, Value>,
}

/// Lit `examples` sans rendre illisible le pack qui le porte (t-524) : d'un
/// pack d'avant t-541 (une liste de cas), le premier cas donne les valeurs du
/// widget (celles que montraient l'aperçu et les réglages par défaut), les
/// autres sont oubliés. Une entrée mal formée (nom qui n'est pas celui d'un
/// fichier de widget, valeurs qui ne sont pas un objet) est écartée avec un
/// diagnostic au journal ; les autres restent. Les valeurs vides sont omises.
///
/// Les bornes des valeurs (`PLAIN_MAX`…) dépendent du schéma du widget : elles
/// s'appliquent à leur usage (`trial_fits`), pas ici.
fn lenient_examples<'de, D>(de: D) -> std::result::Result<BTreeMap<WidgetFile, Map<String, Value>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let Value::Object(stored) = Value::deserialize(de)? else {
        diagnose("", "`examples` n'est pas un objet, ignoré");
        return Ok(BTreeMap::new());
    };
    Ok(stored
        .into_iter()
        .filter_map(|(name, v)| {
            let Some(widget) = WidgetFile::parse(&name) else {
                diagnose(&name, "valeurs d'exemple d'un nom de widget invalide, ignorées");
                return None;
            };
            let values = match serde_json::from_value(v) {
                Ok(StoredValues::Values(m)) => m,
                Ok(StoredValues::Cases(cases)) => cases.into_iter().next().map(|c| c.values).unwrap_or_default(),
                Err(e) => {
                    diagnose(&name, &format!("valeurs d'exemple mal formées, ignorées : {e}"));
                    return None;
                }
            };
            (!values.is_empty()).then_some((widget, values))
        })
        .collect())
}

/// Problèmes distincts déjà signalés : au-delà, plus rien n'est retenu.
const DIAGNOSED_MAX: usize = 256;

/// Signale un problème de `examples` : au journal (`warn`) la première fois
/// dans le processus, puis en `debug` (le manifeste est relu à chaque accès
/// à la bibliothèque).
fn diagnose(widget: &str, problem: &str) {
    static SEEN: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());
    let key = format!("{widget}\0{problem}");
    let first = {
        let mut seen = SEEN.lock().unwrap_or_else(PoisonError::into_inner);
        seen.len() < DIAGNOSED_MAX && seen.insert(key)
    };
    if first {
        tracing::warn!(widget, "pack.json : {problem}");
    } else {
        tracing::debug!(widget, "pack.json : {problem}");
    }
}

impl PackManifest {
    /// Lit le `pack.json` de `files`.
    pub fn from_files(files: &PackFiles) -> serde_json::Result<Self> {
        serde_json::from_value(Value::Object(files.manifest().clone()))
    }

    /// Les fichiers de cette version, avec ses fichiers de widgets.
    pub fn to_files(&self, widgets: BTreeMap<WidgetFile, String>) -> serde_json::Result<PackFiles> {
        match serde_json::to_value(self)? {
            Value::Object(manifest) => Ok(PackFiles::new(manifest, widgets)),
            _ => Err(serde::ser::Error::custom("pack manifest is not an object")),
        }
    }
}

/// Maintenant, au format de `created`.
pub fn now_created() -> String {
    OffsetDateTime::now_utc()
        .format(format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]Z"))
        .unwrap_or_default()
}
```

`crates/oku-slides-core/src/lib.rs` : la ligne `pub use pack::{COMMON_WIDGET_FILE, PackFiles, PackRef, PackVersion, ThemeRef, WidgetFile};` devient :

```rust
pub use pack::{
    COMMON_WIDGET_FILE, PackFiles, PackManifest, PackRef, PackVersion, Template, ThemeRef, WidgetFile, now_created,
};
```

et, dans la même `lib.rs`, le module `tokens` se déclare après `test_registry`, avec une ligne de doc à la fin de la liste des modules du `//!` :

```rust
//! - [`tokens`] : jetons effectifs d'un thème ([`tokens::effective_tokens`], fusion [`tokens::merge`]).
```

```rust
#[cfg(test)]
pub(crate) mod test_registry;
pub mod tokens;
pub mod validate;
```

`crates/oku-slides-core/src/tokens.rs` :

```rust
//! Jetons de design d'un thème : fusion des couches.
//!
//! Les jetons effectifs d'un deck sont les défauts du rendu
//! (`default-tokens.json`), puis ceux de son thème, fusionnés objet par objet
//! comme `merge` côté Typst. La session (éditeur de thème), la CLI et l'import
//! partagent ce calcul.

use serde_json::{Map, Value};

/// Fusion profonde de `over` dans `base` : deux objets se fusionnent clé par
/// clé, toute autre valeur de `over` remplace celle de `base`.
pub fn merge(base: &mut Map<String, Value>, over: &Map<String, Value>) {
    for (k, v) in over {
        match (base.get_mut(k), v) {
            (Some(Value::Object(b)), Value::Object(o)) => merge(b, o),
            _ => {
                base.insert(k.clone(), v.clone());
            }
        }
    }
}

/// Les jetons effectifs : `defaults`, puis `theme` fusionné par-dessus.
pub fn effective_tokens(defaults: &Map<String, Value>, theme: &Map<String, Value>) -> Map<String, Value> {
    let mut tokens = defaults.clone();
    merge(&mut tokens, theme);
    tokens
}

#[cfg(test)]
#[path = "tokens_tests.rs"]
mod tokens_tests;
```

`crates/oku-slides-session/src/theme_schema.rs` : la fonction `merge` (et sa doc) est remplacée par le ré-export, à la même place :

```rust
/// Fusion profonde de `over` dans `base`, comme `merge` côté Typst (celle du cœur).
pub use oku_slides_core::tokens::merge;
```

Remplacer `crates/oku-slides-session/src/pack.rs` par :

```rust
//! Le résumé d'un pack de modèles et les ids des packs intégrés.
//!
//! Le contenu d'un pack (`pack.json` : [`PackManifest`], ses [`Template`])
//! est défini dans le cœur, qui le partage avec l'import ; il est ré-exporté ici.

use serde::Serialize;
use uuid::Uuid;

use oku_slides_core::{PackFiles, PackVersion};
pub use oku_slides_core::{PackManifest, Template, now_created};

use crate::library_kind::VersionManifest;

/// Pack par défaut, `templates/essentiel` : id constant.
pub const DEFAULT_PACK: Uuid = Uuid::from_u128(0x6f3c1e2a_8b4d_4c1e_9a57_2d0e5b7c9f10);
/// Pack intégré `templates/grilles` : id constant.
pub const GRID_PACK: Uuid = Uuid::from_u128(0xb8e2d4f6_1a3c_4e5b_8d7f_9c0a2e4b6d81);

impl VersionManifest for PackManifest {
    fn from_files(files: &PackFiles) -> serde_json::Result<Self> {
        Self::from_files(files)
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn parent(&self) -> Option<&PackVersion> {
        self.parent.as_ref()
    }

    fn created(&self) -> &str {
        &self.created
    }
}

/// Résumé d'un pack dans une version, pour les listes.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PackInfo {
    pub id: Uuid,
    pub version: PackVersion,
    pub name: String,
    pub description: String,
    pub builtin: bool,
    /// Le pack d'un deck neuf ([`DEFAULT_PACK`]).
    pub default: bool,
    pub count: usize,
}

impl PackInfo {
    /// Résumé de la version `version` de `manifest` pour les listes.
    pub fn of(manifest: &PackManifest, version: PackVersion, builtin: bool) -> Self {
        Self {
            id: manifest.id,
            version,
            name: manifest.name.clone(),
            description: manifest.description.clone(),
            builtin,
            default: manifest.id == DEFAULT_PACK,
            count: manifest.templates.len(),
        }
    }
}

/// Une version par rapport à une autre, par la chaîne des `parent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    Same,
    /// Plus ancienne : l'autre en descend.
    Ancestor,
    /// Plus récente : elle descend de l'autre.
    Descendant,
    /// Parallèle : aucune ne descend de l'autre.
    Sibling,
}

#[cfg(test)]
#[path = "pack_tests.rs"]
mod pack_tests;
```

`crates/oku-slides-session/src/library.rs`, `PackLibrary::packs` :

```rust
    pub fn packs(&self) -> Vec<PackInfo> {
        self.entries()
            .into_iter()
            .map(|s| PackInfo::of(&s.manifest, s.version, Self::is_builtin(s.manifest.id)))
            .collect()
    }
```

`crates/oku-slides-session/src/templates_mode.rs` (vue des modèles) : la ligne `let info: PackInfo = shown.manifest.info(shown.version, PackLibrary::is_builtin(shown.manifest.id));` devient :

```rust
        let info = PackInfo::of(&shown.manifest, shown.version, PackLibrary::is_builtin(shown.manifest.id));
```

> Aucun autre appelant ne change : `oku-slides-cli` (tests `cli.rs`, `pack_broken.rs`, régression 0355), `oku-slides-app` et les autres modules de la session passent par `oku_slides_session::{PackManifest, Template}` ou `crate::pack::…`, que les ré-exports conservent. `cargo check --workspace --all-targets` le vérifie.

- [ ] **Step 4 : lancer, constater le succès**

```bash
cargo check --workspace --all-targets
OKU_SLIDES_PATHS__DATA=$(mktemp -d) OKU_SLIDES_PATHS__DECKS=$(mktemp -d) cargo nextest run -p oku-slides-core -p oku-slides-session
```

Attendu : la vérification passe sans avertissement ; tous les tests passent (2250 au brouillon, dont les 10 tests du manifeste et les 3 de `tokens` dans le cœur, et les 2 de `PackInfo::of` dans la session).

- [ ] **Step 5 : vérifier, commit, sous-tâche**

```bash
cargo fmt --all && cargo clippy -p oku-slides-core -p oku-slides-session --all-targets -- -D warnings && cargo nextest run -p oku-slides-core -p oku-slides-session
git add Cargo.lock crates/oku-slides-core/Cargo.toml crates/oku-slides-core/src/lib.rs crates/oku-slides-core/src/pack.rs crates/oku-slides-core/src/pack_tests.rs crates/oku-slides-core/src/tokens.rs crates/oku-slides-core/src/tokens_tests.rs crates/oku-slides-session/src/library.rs crates/oku-slides-session/src/pack.rs crates/oku-slides-session/src/pack_tests.rs crates/oku-slides-session/src/templates_mode.rs crates/oku-slides-session/src/theme_schema.rs && git diff --cached --name-only
git commit -F <scratchpad>/msg.txt   # refactor(core) : Template, PackManifest et jetons effectifs dans le cœur (t-692)
oku cli task done --project slider --id t-797
```

---

### Task 2: Plan, rapport et pertes (types, messages en 20 langues)

**Écarts d'interface :**
- `LossKind::RunStyle(String)` devient `LossKind::RunStyle(RunEffect)` avec `pub enum RunEffect { Underline, Strike, Baseline, Caps, Highlight }` : un nom d'effet typé se localise (variante Fluent), une chaîne libre non. `Highlight` couvre le surlignage d'un run, que le texte riche du cœur n'exprime pas (sa `highlight` est un indice de palette).
- `LossKind::Opaque(String)` devient `LossKind::Opaque(OpaqueReason)` (le type du lot 1, `source::OpaqueReason`) : même raison typée que la `SourceDeck`, localisée par variante.
- Ajout de `LossKind::UnsupportedImage(String /* type MIME */)` : la tâche 7 abandonne les images `emf`/`wmf`/`tiff`/`wdp` (`WarningKind::UnsupportedImage` du lecteur) et doit pouvoir le dire.
- Ajout de `LossKind::RepeatedDecor { text: String, slides: u32 }` (révision 1 : en-tête ou pied répété omis, dit une fois, tâche 9) et de `LossKind::NoSlot` (contenu d'une slide dont le modèle n'a aucun slot : jamais omis en silence, tâche 9). Elles sont définies ici avec toutes les pertes, pour que leurs messages arrivent avec les autres.
- Ajouts : `Loss::on(item: &str, kind) -> Loss`, `Loss::slide(kind) -> Loss`, `ImportReport::new(mode) -> ImportReport`, `ImportReport::loss_count(&self) -> usize` (compte de la ligne `cli-imported`, tâche 11), `Ratio::is_complete(&self) -> bool`, `LossKind::message_id(&self) -> &'static str` (`pub(crate)`).

**Files:**
- Create: `crates/oku-slides-import/src/report.rs`, `crates/oku-slides-import/src/report_tests.rs`, `crates/oku-slides-import/src/plan.rs`, `crates/oku-slides-import/src/plan_tests.rs`
- Modify: `crates/oku-slides-import/src/lib.rs` (`pub mod plan; pub mod report;`, ré-exports, doc), `crates/oku-slides-import/Cargo.toml` (`serde_json` passe des dev-dependencies aux dependencies), `crates/oku-slides-i18n/src/locales/*/document.ftl` (14 clés `import-loss-*` × 20 langues)

**Interfaces:**
- Consumes : `oku_slides_core::Op` ; `crate::source::{Asset, Item, OpaqueReason, Warning}` (lot 1) ; `oku_slides_i18n::{Localize, Localizer}`.
- Produces :

```rust
// crate::plan
pub struct WidgetRequest { pub signature: String, pub slides: Vec<String>, pub sample: Vec<Item>, pub fallback: String }
pub struct ImportPlan {
    pub ops: Vec<Op>, pub assets: BTreeMap<String, Asset>, pub widget_requests: Vec<WidgetRequest>, pub report: ImportReport,
}
// crate::report
pub enum Mode { Retrofit }                                   // serde "retrofit"
pub enum Placement { Flow, Grid }                            // serde "flow" | "grid"
pub enum RunEffect { Underline, Strike, Baseline, Caps, Highlight }
pub enum LossKind { ListLevels, MergedCells, PercentStacked, RunStyle(RunEffect), Transform, ColorDropped(String),
    TextTruncated, TableTruncated, Opaque(OpaqueReason), UnsupportedImage(String), HiddenSlide, Effects,
    RepeatedDecor { text: String, slides: u32 }, NoSlot }
pub struct Loss { pub item: Option<String>, pub kind: LossKind }
impl Loss { pub fn on(item: &str, kind: LossKind) -> Self; pub fn slide(kind: LossKind) -> Self; }
impl Localize for Loss;
impl LossKind { pub(crate) fn message_id(&self) -> &'static str; }
pub struct SlideReport { pub source: u32, pub slide: String, pub template: String, pub placement: Placement,
    pub losses: Vec<Loss>, pub fallbacks: Vec<String> }
pub struct Ratio { pub found: u32, pub total: u32 }           // Default ; is_complete()
pub enum Missing { Run { slide: u32, text: String }, Image { slide: u32, item: String },
    Table { slide: u32, item: String }, Chart { slide: u32, item: String } }
pub struct Coverage { pub runs: Ratio, pub images: Ratio, pub tables: Ratio, pub charts: Ratio, pub missing: Vec<Missing> }
pub struct ImportReport { pub mode: Mode, pub slides: Vec<SlideReport>, pub warnings: Vec<Warning>, pub coverage: Option<Coverage> }
impl ImportReport {
    pub fn new(mode: Mode) -> Self;
    pub fn loss_count(&self) -> usize;
    pub fn to_json(&self, l: &Localizer) -> serde_json::Value;
}
// crate (racine)
pub use plan::ImportPlan;
pub use report::ImportReport;
```

- Règles :
  - Tout se sérialise (`Serialize`) ; `Asset` s'écrit `{"mime", "len"}` (lot 1), jamais ses octets.
  - `LossKind` s'écrit `{"type": "<snake_case>", "detail": …}` comme `WarningKind` ; sans détail, `{"type"}` seul.
  - `to_json` = la sérialisation, plus `"message"` (localisé) dans chaque objet de `slides[i].losses[j]`. `warnings` restent tels quels.
  - Messages : une clé par nature ; `import-loss-run-style` choisit sur `$effect` (`underline`, `strike`, `baseline`, `caps`, `*highlight`), `import-loss-opaque` sur `$reason` (`media`, `ole`, `math`, `smartart`, `*unknown`) ; `import-loss-color-dropped` cite `$color`, `import-loss-unsupported-image` cite `$format` ; `import-loss-repeated-decor` cite `$text` et accorde le nombre `$slides` par les catégories plurielles de la langue (`one`/`other` ; `one`/`few`/`many`/`other` en polonais, russe, ukrainien ; `zero`/`one`/`two`/`few`/`many`/`other` en arabe ; une seule forme en japonais, coréen, chinois, thaï, vietnamien et turc, le nombre collé au classificateur en japonais et en chinois).

- [ ] **Step 1 : tests qui échouent**

`crates/oku-slides-import/src/report_tests.rs` :

```rust
use oku_slides_i18n::{Localize, Localizer, english};
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::*;

fn slide(losses: Vec<Loss>) -> SlideReport {
    SlideReport {
        source: 2,
        slide: "s2".into(),
        template: "texte".into(),
        placement: Placement::Flow,
        losses,
        fallbacks: vec!["stack".into()],
    }
}

#[rstest]
#[case(LossKind::ListLevels)]
#[case(LossKind::MergedCells)]
#[case(LossKind::PercentStacked)]
#[case(LossKind::RunStyle(RunEffect::Underline))]
#[case(LossKind::RunStyle(RunEffect::Strike))]
#[case(LossKind::RunStyle(RunEffect::Baseline))]
#[case(LossKind::RunStyle(RunEffect::Caps))]
#[case(LossKind::RunStyle(RunEffect::Highlight))]
#[case(LossKind::Transform)]
#[case(LossKind::ColorDropped("#ff0000".into()))]
#[case(LossKind::TextTruncated)]
#[case(LossKind::TableTruncated)]
#[case(LossKind::Opaque(OpaqueReason::Media))]
#[case(LossKind::Opaque(OpaqueReason::Ole))]
#[case(LossKind::Opaque(OpaqueReason::Math))]
#[case(LossKind::Opaque(OpaqueReason::SmartArt))]
#[case(LossKind::Opaque(OpaqueReason::Unknown("x".into())))]
#[case(LossKind::UnsupportedImage("image/x-emf".into()))]
#[case(LossKind::HiddenSlide)]
#[case(LossKind::Effects)]
#[case(LossKind::RepeatedDecor { text: "CONFIDENTIAL".into(), slides: 7 })]
#[case(LossKind::RepeatedDecor { text: "CONFIDENTIAL".into(), slides: 1 })]
#[case(LossKind::NoSlot)]
fn loss_localize_in_every_locale_formats_its_own_message(#[case] kind: LossKind) {
    let id = kind.message_id();
    let loss = Loss::slide(kind);
    for tag in oku_slides_i18n::available() {
        let l = Localizer::new(Some(tag));
        let msg = loss.localize(&l);
        assert_that!(l.has(id)).named(tag).is_true();
        assert_that!(msg.is_empty() || msg.contains('{')).named(tag).is_false();
    }
}

#[test]
fn loss_localize_color_dropped_names_the_color() {
    let msg = Loss::on("s1/4", LossKind::ColorDropped("#ff0000".into())).localize(english());
    assert_that!(msg).contains("#ff0000");
}

#[test]
fn loss_localize_run_style_in_french_names_the_effect() {
    let msg = Loss::slide(LossKind::RunStyle(RunEffect::Underline)).localize(&Localizer::new(Some("fr")));
    assert_that!(msg).is_equal_to("Soulignement perdu".to_owned());
}

#[test]
fn loss_localize_opaque_in_english_names_the_reason() {
    let msg = Loss::slide(LossKind::Opaque(OpaqueReason::Math)).localize(english());
    assert_that!(msg).is_equal_to("Equation not imported (no fallback image)".to_owned());
}

#[test]
fn loss_localize_repeated_decor_names_the_text_and_counts_the_slides() {
    let msg = Loss::on("s2/18", LossKind::RepeatedDecor { text: "CONFIDENTIAL".into(), slides: 7 }).localize(english());
    assert_that!(msg).is_equal_to(
        "Header or footer “CONFIDENTIAL” repeated on 7 slides left out: the theme draws its own".to_owned(),
    );
}

#[test]
fn loss_localize_repeated_decor_in_french_agrees_with_one_slide() {
    let loss = Loss::on("s2/18", LossKind::RepeatedDecor { text: "Logo".into(), slides: 1 });
    let msg = loss.localize(&Localizer::new(Some("fr")));
    assert_that!(msg)
        .is_equal_to("En-tête ou pied « Logo » répété sur 1 slide omis : le thème dessine les siens".to_owned());
}

#[test]
fn loss_serializes_kind_as_type_and_detail() {
    let v = serde_json::to_value(Loss::on("s1/4", LossKind::ColorDropped("#ff0000".into()))).unwrap();
    assert_that!(v).is_equal_to(json!({ "item": "s1/4", "kind": { "type": "color_dropped", "detail": "#ff0000" } }));
}

#[test]
fn loss_without_detail_serializes_type_only() {
    let v = serde_json::to_value(Loss::slide(LossKind::HiddenSlide)).unwrap();
    assert_that!(v).is_equal_to(json!({ "item": null, "kind": { "type": "hidden_slide" } }));
}

#[test]
fn import_report_to_json_adds_a_message_beside_each_loss() {
    let mut report = ImportReport::new(Mode::Retrofit);
    report.slides.push(slide(vec![Loss::slide(LossKind::HiddenSlide), Loss::on("s2/3", LossKind::MergedCells)]));
    let v = report.to_json(english());
    assert_that!(v["mode"]).is_equal_to(json!("retrofit"));
    assert_that!(v["slides"][0]["placement"]).is_equal_to(json!("flow"));
    assert_that!(v["slides"][0]["losses"][0]["message"]).is_equal_to(json!("Hidden slide imported as visible"));
    assert_that!(v["slides"][0]["losses"][1]["message"]).is_equal_to(json!("Merged cells flattened"));
    assert_that!(v["slides"][0]["losses"][1]["item"]).is_equal_to(json!("s2/3"));
}

#[test]
fn import_report_to_json_writes_coverage_when_measured() {
    let mut report = ImportReport::new(Mode::Retrofit);
    report.coverage = Some(Coverage {
        runs: Ratio { found: 3, total: 4 },
        missing: vec![Missing::Run { slide: 1, text: "Perdu".into() }],
        ..Coverage::default()
    });
    let v = report.to_json(english());
    assert_that!(v["coverage"]["runs"]).is_equal_to(json!({ "found": 3, "total": 4 }));
    assert_that!(v["coverage"]["missing"][0]).is_equal_to(json!({ "type": "run", "slide": 1, "text": "Perdu" }));
}

#[test]
fn import_report_loss_count_sums_every_slide() {
    let mut report = ImportReport::new(Mode::Retrofit);
    report.slides.push(slide(vec![Loss::slide(LossKind::HiddenSlide)]));
    report.slides.push(slide(vec![Loss::slide(LossKind::Effects), Loss::slide(LossKind::Transform)]));
    assert_that!(report.loss_count()).is_equal_to(3);
}

#[rstest]
#[case(Ratio { found: 0, total: 0 }, true)]
#[case(Ratio { found: 4, total: 4 }, true)]
#[case(Ratio { found: 3, total: 4 }, false)]
fn ratio_is_complete_when_everything_is_found(#[case] r: Ratio, #[case] complete: bool) {
    assert_that!(r.is_complete()).is_equal_to(complete);
}
```

> `loss_localize_in_every_locale_formats_its_own_message` vérifie que chaque langue a la clé (`Localizer::has` ignore le repli anglais) et que le message est formaté (un argument manquant laisserait `{$x}`). Les attendus anglais et français sont ceux des tableaux de l'étape 3.

`crates/oku-slides-import/src/plan_tests.rs` :

```rust
use std::collections::BTreeMap;

use oku_slides_core::{Op, Slide};
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::report::{ImportReport, Mode};
use crate::source::{Frame, Item, ItemKind, TextBox};

fn item() -> Item {
    Item {
        id: "s3/7".into(),
        name: "Cercle".into(),
        alt: None,
        frame: Frame::new(10.0, 20.0, 30.0, 40.0),
        z: 0,
        role: None,
        placeholder: None,
        kind: ItemKind::Text(TextBox::default()),
    }
}

#[test]
fn import_plan_serializes_ops_assets_by_length_and_requests() {
    let plan = ImportPlan {
        ops: vec![Op::AddSlide { index: None, defaults: false, slide: Slide::new("s1", "content") }],
        assets: BTreeMap::from([("sha256:ab".to_owned(), Asset { mime: "image/png".into(), bytes: vec![1, 2, 3] })]),
        widget_requests: vec![WidgetRequest {
            signature: "text×1|0000".into(),
            slides: vec!["s3".into(), "s5".into()],
            sample: vec![item()],
            fallback: "stack".into(),
        }],
        report: ImportReport::new(Mode::Retrofit),
    };
    let v = serde_json::to_value(&plan).unwrap();
    assert_that!(v["ops"][0]["op"]).is_equal_to(json!("add_slide"));
    assert_that!(v["assets"]["sha256:ab"]).is_equal_to(json!({ "mime": "image/png", "len": 3 }));
    assert_that!(v["widget_requests"][0]["slides"]).is_equal_to(json!(["s3", "s5"]));
    assert_that!(v["widget_requests"][0]["sample"][0]["id"]).is_equal_to(json!("s3/7"));
    assert_that!(v["report"]["mode"]).is_equal_to(json!("retrofit"));
}
```

Dans `crates/oku-slides-import/src/lib.rs`, ajouter `pub mod plan;` et `pub mod report;` avant `pub mod source;` (pour que les tests compilent, les fichiers `plan.rs` et `report.rs` doivent exister : créer chacun avec sa seule ligne `#[cfg(test)] #[path = "…_tests.rs"] mod …_tests;`).

- [ ] **Step 2 : lancer, constater l'échec**

```bash
export CARGO_TARGET_DIR=$PWD/target
cargo nextest run -p oku-slides-import report_tests plan_tests
```

Attendu : échec de compilation (`ImportReport`, `Loss`, `LossKind`, `ImportPlan`, `WidgetRequest`… introuvables).

- [ ] **Step 3 : implémentation**

`crates/oku-slides-import/Cargo.toml` : ajouter `serde_json.workspace = true` à `[dependencies]` (après `serde.workspace = true`) et le retirer de `[dev-dependencies]`.

`crates/oku-slides-import/src/report.rs` :

```rust
//! Le rapport d'import : par slide, le modèle choisi, le placement, les pertes
//! et les replis ; les avertissements du lecteur ; la couverture du contenu.
//!
//! Une perte n'est jamais une erreur : l'import continue, le rapport la dit.
//! [`ImportReport::to_json`] donne la forme écrite par `--report`, avec le
//! message localisé de chaque perte.

use oku_slides_i18n::{Localize, Localizer};
use serde::Serialize;
use serde_json::Value;

use crate::source::{OpaqueReason, Warning};

/// Mode d'import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Contenu projeté sur un pack et un thème existants.
    Retrofit,
}

/// Où vont les contenus d'une slide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Placement {
    /// Blocs du slot `flow`, dans l'agencement du modèle.
    Flow,
    /// Éléments du slot libre, placés en cellules de la grille du modèle.
    Grid,
}

/// Effet de texte que le texte riche Oku n'exprime pas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunEffect {
    /// Souligné.
    Underline,
    /// Barré.
    Strike,
    /// Exposant ou indice.
    Baseline,
    /// Capitales ou petites capitales.
    Caps,
    /// Surlignage.
    Highlight,
}

/// Nature d'une perte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "detail")]
pub enum LossKind {
    /// Niveaux de liste aplatis.
    ListLevels,
    /// Fusions de cellules aplaties.
    MergedCells,
    /// Empilement à 100 % rendu en empilement simple.
    PercentStacked,
    /// Effet de texte perdu.
    RunStyle(RunEffect),
    /// Rotation, miroir ou rognage perdus.
    Transform,
    /// Couleur sans jeton proche dans le thème cible (`#rrggbb`).
    ColorDropped(String),
    /// Texte coupé aux bornes du cœur.
    TextTruncated,
    /// Tableau coupé aux bornes du cœur.
    TableTruncated,
    /// Item intraduisible sans image de repli.
    Opaque(OpaqueReason),
    /// Image dans un format que le rendu ne dessine pas (type MIME).
    UnsupportedImage(String),
    /// Slide masquée en diaporama : importée visible.
    HiddenSlide,
    /// Effets (ombres, reflets, 3D) perdus.
    Effects,
    /// En-tête ou pied répété sur les slides (texte, chiffres ôtés, ou nom de
    /// l'image), omis : le thème dessine les siens. Dit une fois, à sa première slide.
    RepeatedDecor {
        /// Texte du motif, ou nom de l'image.
        text: String,
        /// Nombre de slides où il apparaît.
        slides: u32,
    },
    /// Contenu sans place : le modèle choisi n'a aucun slot.
    NoSlot,
}

/// Une perte, sur un item source ou sur la slide entière.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Loss {
    /// Id de l'item source ; `None` pour la slide.
    pub item: Option<String>,
    /// Nature.
    pub kind: LossKind,
}

impl Loss {
    /// Perte sur l'item `item`.
    pub fn on(item: &str, kind: LossKind) -> Self {
        Self { item: Some(item.to_owned()), kind }
    }

    /// Perte sur la slide entière.
    pub fn slide(kind: LossKind) -> Self {
        Self { item: None, kind }
    }
}

/// Argument `effect` du message `import-loss-run-style`.
fn effect_name(e: RunEffect) -> &'static str {
    match e {
        RunEffect::Underline => "underline",
        RunEffect::Strike => "strike",
        RunEffect::Baseline => "baseline",
        RunEffect::Caps => "caps",
        RunEffect::Highlight => "highlight",
    }
}

/// Argument `reason` du message `import-loss-opaque`.
fn reason_name(r: &OpaqueReason) -> &'static str {
    match r {
        OpaqueReason::Media => "media",
        OpaqueReason::Ole => "ole",
        OpaqueReason::Math => "math",
        OpaqueReason::SmartArt => "smartart",
        OpaqueReason::Unknown(_) => "unknown",
    }
}

impl LossKind {
    /// Clé Fluent du message.
    pub(crate) fn message_id(&self) -> &'static str {
        match self {
            LossKind::ListLevels => "import-loss-list-levels",
            LossKind::MergedCells => "import-loss-merged-cells",
            LossKind::PercentStacked => "import-loss-percent-stacked",
            LossKind::RunStyle(_) => "import-loss-run-style",
            LossKind::Transform => "import-loss-transform",
            LossKind::ColorDropped(_) => "import-loss-color-dropped",
            LossKind::TextTruncated => "import-loss-text-truncated",
            LossKind::TableTruncated => "import-loss-table-truncated",
            LossKind::Opaque(_) => "import-loss-opaque",
            LossKind::UnsupportedImage(_) => "import-loss-unsupported-image",
            LossKind::HiddenSlide => "import-loss-hidden-slide",
            LossKind::Effects => "import-loss-effects",
            LossKind::RepeatedDecor { .. } => "import-loss-repeated-decor",
            LossKind::NoSlot => "import-loss-no-slot",
        }
    }
}

impl Localize for Loss {
    fn localize(&self, l: &Localizer) -> String {
        let id = self.kind.message_id();
        match &self.kind {
            LossKind::RunStyle(e) => l.tr_with(id, &[("effect", effect_name(*e).into())]),
            LossKind::ColorDropped(hex) => l.tr_with(id, &[("color", hex.as_str().into())]),
            LossKind::Opaque(r) => l.tr_with(id, &[("reason", reason_name(r).into())]),
            LossKind::UnsupportedImage(mime) => l.tr_with(id, &[("format", mime.as_str().into())]),
            LossKind::RepeatedDecor { text, slides } => {
                l.tr_with(id, &[("text", text.as_str().into()), ("slides", (*slides).into())])
            }
            LossKind::ListLevels
            | LossKind::MergedCells
            | LossKind::PercentStacked
            | LossKind::Transform
            | LossKind::TextTruncated
            | LossKind::TableTruncated
            | LossKind::HiddenSlide
            | LossKind::Effects
            | LossKind::NoSlot => l.tr(id),
        }
    }
}

/// Ce que devient une slide source.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SlideReport {
    /// Numéro de la slide source (1..).
    pub source: u32,
    /// Id de la slide Oku produite.
    pub slide: String,
    /// Id du modèle choisi.
    pub template: String,
    /// Placement des contenus.
    pub placement: Placement,
    /// Pertes.
    pub losses: Vec<Loss>,
    /// Sortes de widget de repli employées (`stack`…).
    pub fallbacks: Vec<String>,
}

/// Une proportion : `found` sur `total`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct Ratio {
    /// Retrouvés.
    pub found: u32,
    /// Attendus.
    pub total: u32,
}

impl Ratio {
    /// Vrai si tout est retrouvé (vrai aussi quand rien n'est attendu).
    pub fn is_complete(&self) -> bool {
        self.found >= self.total
    }
}

/// Un contenu source absent du deck obtenu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Missing {
    /// Texte d'un run.
    Run {
        /// Numéro de slide source.
        slide: u32,
        /// Texte du run, espaces normalisés.
        text: String,
    },
    /// Image.
    Image {
        /// Numéro de slide source.
        slide: u32,
        /// Id de l'item source.
        item: String,
    },
    /// Tableau.
    Table {
        /// Numéro de slide source.
        slide: u32,
        /// Id de l'item source.
        item: String,
    },
    /// Graphique.
    Chart {
        /// Numéro de slide source.
        slide: u32,
        /// Id de l'item source.
        item: String,
    },
}

/// Couverture du contenu, mesurée sur le deck obtenu.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Coverage {
    /// Runs de texte retrouvés.
    pub runs: Ratio,
    /// Images retrouvées.
    pub images: Ratio,
    /// Tableaux aux données égales.
    pub tables: Ratio,
    /// Graphiques aux données égales.
    pub charts: Ratio,
    /// Ce qui manque.
    pub missing: Vec<Missing>,
}

/// Le rapport d'un import.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImportReport {
    /// Mode.
    pub mode: Mode,
    /// Une entrée par slide source, dans l'ordre.
    pub slides: Vec<SlideReport>,
    /// Avertissements du lecteur.
    pub warnings: Vec<Warning>,
    /// Couverture du contenu ; `None` tant qu'elle n'est pas mesurée.
    pub coverage: Option<Coverage>,
}

impl ImportReport {
    /// Rapport vide du mode `mode`.
    pub fn new(mode: Mode) -> Self {
        Self { mode, slides: Vec::new(), warnings: Vec::new(), coverage: None }
    }

    /// Nombre total de pertes.
    pub fn loss_count(&self) -> usize {
        self.slides.iter().map(|s| s.losses.len()).sum()
    }

    /// La forme écrite : la sérialisation, plus un `message` localisé à côté de chaque perte.
    pub fn to_json(&self, l: &Localizer) -> Value {
        // Ces types se sérialisent toujours : clés textuelles, nombres finis.
        let mut v = serde_json::to_value(self).unwrap_or_default();
        let Some(slides) = v.get_mut("slides").and_then(Value::as_array_mut) else { return v };
        for (slide, report) in slides.iter_mut().zip(&self.slides) {
            let Some(losses) = slide.get_mut("losses").and_then(Value::as_array_mut) else { continue };
            for (written, loss) in losses.iter_mut().zip(&report.losses) {
                if let Some(o) = written.as_object_mut() {
                    o.insert("message".to_owned(), Value::String(loss.localize(l)));
                }
            }
        }
        v
    }
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod report_tests;
```

`crates/oku-slides-import/src/plan.rs` :

```rust
//! Le plan d'import : les opérations à appliquer en une transaction, les
//! assets qu'elles référencent, les demandes de widget et le rapport.
//!
//! Le plan est valide par construction : `oku_slides_core::apply_all` sur un
//! deck vide portant le pack et le thème cibles l'accepte, et le deck obtenu
//! ne lève aucune `Issue` une fois les assets ajoutés au bundle.

use std::collections::BTreeMap;

use oku_slides_core::Op;
use serde::Serialize;

use crate::report::ImportReport;
use crate::source::{Asset, Item};

/// Un motif récurrent sans widget dédié, rendu par un repli.
///
/// Le lot 5 en fera un widget de pack enfant ; au lot 2, la demande est
/// seulement émise (plan et rapport).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WidgetRequest {
    /// Signature du motif (`crate::pattern::signature`).
    pub signature: String,
    /// Ids des slides Oku où il apparaît, dans l'ordre.
    pub slides: Vec<String>,
    /// Les items source de sa première occurrence.
    pub sample: Vec<Item>,
    /// Sorte du widget de repli employé (`stack`…).
    pub fallback: String,
}

/// Le résultat d'un projecteur.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImportPlan {
    /// Opérations, à appliquer dans l'ordre en une transaction.
    pub ops: Vec<Op>,
    /// Les assets référencés par `ops`, par identifiant (`asset_id` de leurs octets).
    pub assets: BTreeMap<String, Asset>,
    /// Demandes de widget, une par signature.
    pub widget_requests: Vec<WidgetRequest>,
    /// Rapport.
    pub report: ImportReport,
}

#[cfg(test)]
#[path = "plan_tests.rs"]
mod plan_tests;
```

`crates/oku-slides-import/src/lib.rs` complet :

```rust
//! Import d'une présentation PowerPoint en représentation intermédiaire.
//!
//! [`read_pptx`] lit les octets d'un `.pptx` et rend une [`source::SourceDeck`] :
//! géométrie en pt, styles texte résolus, couleurs avec leur provenance,
//! tableaux, graphiques, SmartArt, replis, polices. Bornes : [`ReadLimits`] ;
//! erreurs : [`ImportError`].
//!
//! Un projecteur rend un [`ImportPlan`] : les opérations qui construisent le
//! deck Oku, les assets qu'elles référencent et un [`ImportReport`] (pertes,
//! replis, couverture du contenu).

pub mod plan;
pub mod report;
pub mod source;

mod collect;
mod error;
mod limits;
mod pptx;

pub use error::{ImportError, ImportErrorKind};
pub use limits::ReadLimits;
pub use plan::ImportPlan;
pub use pptx::read_pptx;
pub use report::ImportReport;
```

Messages : ajouter à la fin de chaque `crates/oku-slides-i18n/src/locales/<langue>/document.ftl`, après `import-too-large`, le bloc de sa langue (mêmes clés, mêmes variables `$effect`, `$color`, `$reason`, `$format`, `$text`, `$slides`, mêmes clés de variantes partout pour `$effect` et `$reason` ; variante par défaut `*[highlight]`, `*[unknown]` et `*[other]`).

`ar/document.ftl` :

```ftl
import-loss-list-levels = تم دمج مستويات القائمة في مستوى واحد
import-loss-merged-cells = تم فصل الخلايا المدمجة
import-loss-percent-stacked = عُرض المخطط المكدس بنسبة 100% كمخطط مكدس
import-loss-run-style = { $effect ->
    [underline] فُقد التسطير
    [strike] فُقد الشطب
    [baseline] فُقد النص المرتفع أو المنخفض
    [caps] فُقدت الأحرف الكبيرة
   *[highlight] فُقد التمييز
}
import-loss-transform = فُقد التدوير أو القلب أو الاقتصاص
import-loss-color-dropped = لا يوجد للون { $color } لون قريب في السمة: استُخدم لون السمة
import-loss-text-truncated = اختُصر النص ليبقى ضمن الحدود
import-loss-table-truncated = اختُصر الجدول ليبقى ضمن الحدود
import-loss-opaque = { $reason ->
    [media] لم يُستورد الفيديو أو الصوت (لا توجد صورة بديلة)
    [ole] لم يُستورد الكائن المضمّن (لا توجد صورة بديلة)
    [math] لم تُستورد المعادلة (لا توجد صورة بديلة)
    [smartart] لم يُستورد SmartArt (لا توجد صورة بديلة)
   *[unknown] لم يُستورد الكائن غير المدعوم (لا توجد صورة بديلة)
}
import-loss-unsupported-image = لم تُستورد الصورة بتنسيق { $format }
import-loss-hidden-slide = استُوردت الشريحة المخفية كشريحة ظاهرة
import-loss-effects = فُقدت الظلال والانعكاسات والتأثيرات ثلاثية الأبعاد
import-loss-repeated-decor = { $slides ->
   [zero] حُذف الرأس أو التذييل «{ $text }» المكرر في { $slides } شريحة: يرسم النسق رأسه وتذييله
   [one] حُذف الرأس أو التذييل «{ $text }» المكرر في شريحة واحدة: يرسم النسق رأسه وتذييله
   [two] حُذف الرأس أو التذييل «{ $text }» المكرر في شريحتين: يرسم النسق رأسه وتذييله
   [few] حُذف الرأس أو التذييل «{ $text }» المكرر في { $slides } شرائح: يرسم النسق رأسه وتذييله
   [many] حُذف الرأس أو التذييل «{ $text }» المكرر في { $slides } شريحة: يرسم النسق رأسه وتذييله
  *[other] حُذف الرأس أو التذييل «{ $text }» المكرر في { $slides } شريحة: يرسم النسق رأسه وتذييله
}
import-loss-no-slot = حُذف المحتوى: لا مكان له في القالب المختار
```

`bn/document.ftl` :

```ftl
import-loss-list-levels = তালিকার স্তরগুলো একটি স্তরে নামানো হয়েছে
import-loss-merged-cells = একত্রিত ঘরগুলো আলাদা করা হয়েছে
import-loss-percent-stacked = ১০০% স্তূপীকৃত চার্ট স্তূপীকৃত হিসেবে দেখানো হয়েছে
import-loss-run-style = { $effect ->
    [underline] আন্ডারলাইন হারিয়েছে
    [strike] কাটা দাগ হারিয়েছে
    [baseline] সুপারস্ক্রিপ্ট বা সাবস্ক্রিপ্ট হারিয়েছে
    [caps] বড় হাতের অক্ষর হারিয়েছে
   *[highlight] হাইলাইট হারিয়েছে
}
import-loss-transform = ঘোরানো, উল্টানো বা ক্রপ হারিয়েছে
import-loss-color-dropped = { $color } রঙের কাছাকাছি কোনো থিম রঙ নেই: থিমের রঙ ব্যবহার করা হয়েছে
import-loss-text-truncated = সীমার মধ্যে রাখতে লেখা ছোট করা হয়েছে
import-loss-table-truncated = সীমার মধ্যে রাখতে টেবিল ছোট করা হয়েছে
import-loss-opaque = { $reason ->
    [media] ভিডিও বা শব্দ ইমপোর্ট করা হয়নি (বিকল্প ছবি নেই)
    [ole] এমবেড করা অবজেক্ট ইমপোর্ট করা হয়নি (বিকল্প ছবি নেই)
    [math] সমীকরণ ইমপোর্ট করা হয়নি (বিকল্প ছবি নেই)
    [smartart] SmartArt ইমপোর্ট করা হয়নি (বিকল্প ছবি নেই)
   *[unknown] অসমর্থিত অবজেক্ট ইমপোর্ট করা হয়নি (বিকল্প ছবি নেই)
}
import-loss-unsupported-image = { $format } ফরম্যাটের ছবি ইমপোর্ট করা হয়নি
import-loss-hidden-slide = লুকানো স্লাইড দৃশ্যমান হিসেবে ইমপোর্ট করা হয়েছে
import-loss-effects = ছায়া, প্রতিফলন ও 3D প্রভাব হারিয়েছে
import-loss-repeated-decor = { $slides ->
   [one] { $slides }টি স্লাইডে পুনরাবৃত্ত হেডার বা ফুটার “{ $text }” বাদ দেওয়া হয়েছে: থিম নিজেরটি আঁকে
  *[other] { $slides }টি স্লাইডে পুনরাবৃত্ত হেডার বা ফুটার “{ $text }” বাদ দেওয়া হয়েছে: থিম নিজেরটি আঁকে
}
import-loss-no-slot = বিষয়বস্তু বাদ দেওয়া হয়েছে: বেছে নেওয়া টেমপ্লেটে এর জায়গা নেই
```

`de/document.ftl` :

```ftl
import-loss-list-levels = Listenebenen auf eine Ebene reduziert
import-loss-merged-cells = Verbundene Zellen aufgelöst
import-loss-percent-stacked = 100-%-gestapeltes Diagramm als gestapelt dargestellt
import-loss-run-style = { $effect ->
    [underline] Unterstreichung verloren
    [strike] Durchstreichung verloren
    [baseline] Hoch- oder Tiefstellung verloren
    [caps] Großbuchstaben verloren
   *[highlight] Hervorhebung verloren
}
import-loss-transform = Drehung, Spiegelung oder Zuschnitt verloren
import-loss-color-dropped = Die Farbe { $color } hat keine ähnliche Designfarbe: Designfarbe verwendet
import-loss-text-truncated = Text gekürzt, um die Grenzen einzuhalten
import-loss-table-truncated = Tabelle gekürzt, um die Grenzen einzuhalten
import-loss-opaque = { $reason ->
    [media] Video oder Ton nicht importiert (kein Ersatzbild)
    [ole] Eingebettetes Objekt nicht importiert (kein Ersatzbild)
    [math] Formel nicht importiert (kein Ersatzbild)
    [smartart] SmartArt nicht importiert (kein Ersatzbild)
   *[unknown] Nicht unterstütztes Objekt nicht importiert (kein Ersatzbild)
}
import-loss-unsupported-image = Bild im Format { $format } nicht importiert
import-loss-hidden-slide = Ausgeblendete Folie als sichtbar importiert
import-loss-effects = Schatten, Spiegelungen und 3D-Effekte verloren
import-loss-repeated-decor = { $slides ->
   [one] Kopf- oder Fußzeile „{ $text }“ auf { $slides } Folie wiederholt, weggelassen: das Design zeichnet eigene
  *[other] Kopf- oder Fußzeile „{ $text }“ auf { $slides } Folien wiederholt, weggelassen: das Design zeichnet eigene
}
import-loss-no-slot = Inhalt weggelassen: die gewählte Vorlage hat keinen Platz dafür
```

`en/document.ftl` :

```ftl
import-loss-list-levels = List levels flattened to one level
import-loss-merged-cells = Merged cells flattened
import-loss-percent-stacked = 100% stacked chart shown as stacked
import-loss-run-style = { $effect ->
    [underline] Underline lost
    [strike] Strikethrough lost
    [baseline] Superscript or subscript lost
    [caps] Capitals lost
   *[highlight] Highlight lost
}
import-loss-transform = Rotation, flip or crop lost
import-loss-color-dropped = Color { $color } has no close theme color: theme color used
import-loss-text-truncated = Text shortened to fit the limits
import-loss-table-truncated = Table shortened to fit the limits
import-loss-opaque = { $reason ->
    [media] Video or sound not imported (no fallback image)
    [ole] Embedded object not imported (no fallback image)
    [math] Equation not imported (no fallback image)
    [smartart] SmartArt not imported (no fallback image)
   *[unknown] Unsupported object not imported (no fallback image)
}
import-loss-unsupported-image = Image in { $format } format not imported
import-loss-hidden-slide = Hidden slide imported as visible
import-loss-effects = Shadows, reflections and 3D effects lost
import-loss-repeated-decor = { $slides ->
   [one] Header or footer “{ $text }” repeated on { $slides } slide left out: the theme draws its own
  *[other] Header or footer “{ $text }” repeated on { $slides } slides left out: the theme draws its own
}
import-loss-no-slot = Content left out: the chosen template has no place for it
```

`es/document.ftl` :

```ftl
import-loss-list-levels = Niveles de lista reducidos a uno
import-loss-merged-cells = Celdas combinadas separadas
import-loss-percent-stacked = Gráfico apilado al 100 % mostrado como apilado
import-loss-run-style = { $effect ->
    [underline] Subrayado perdido
    [strike] Tachado perdido
    [baseline] Superíndice o subíndice perdido
    [caps] Mayúsculas perdidas
   *[highlight] Resaltado perdido
}
import-loss-transform = Rotación, volteo o recorte perdidos
import-loss-color-dropped = El color { $color } no tiene un color cercano en el tema: se usa un color del tema
import-loss-text-truncated = Texto acortado para respetar los límites
import-loss-table-truncated = Tabla acortada para respetar los límites
import-loss-opaque = { $reason ->
    [media] Vídeo o sonido no importado (sin imagen de respaldo)
    [ole] Objeto incrustado no importado (sin imagen de respaldo)
    [math] Ecuación no importada (sin imagen de respaldo)
    [smartart] SmartArt no importado (sin imagen de respaldo)
   *[unknown] Objeto no compatible no importado (sin imagen de respaldo)
}
import-loss-unsupported-image = Imagen en formato { $format } no importada
import-loss-hidden-slide = Diapositiva oculta importada como visible
import-loss-effects = Sombras, reflejos y efectos 3D perdidos
import-loss-repeated-decor = { $slides ->
   [one] Encabezado o pie «{ $text }» repetido en { $slides } diapositiva omitido: el tema dibuja los suyos
  *[other] Encabezado o pie «{ $text }» repetido en { $slides } diapositivas omitido: el tema dibuja los suyos
}
import-loss-no-slot = Contenido omitido: la plantilla elegida no tiene sitio para él
```

`fr/document.ftl` :

```ftl
import-loss-list-levels = Niveaux de liste ramenés à un seul
import-loss-merged-cells = Cellules fusionnées séparées
import-loss-percent-stacked = Graphique empilé à 100 % rendu en empilé simple
import-loss-run-style = { $effect ->
    [underline] Soulignement perdu
    [strike] Barré perdu
    [baseline] Exposant ou indice perdu
    [caps] Capitales perdues
   *[highlight] Surlignage perdu
}
import-loss-transform = Rotation, miroir ou rognage perdus
import-loss-color-dropped = La couleur { $color } n'a pas de couleur proche dans le thème : couleur du thème utilisée
import-loss-text-truncated = Texte raccourci pour tenir dans les limites
import-loss-table-truncated = Tableau raccourci pour tenir dans les limites
import-loss-opaque = { $reason ->
    [media] Vidéo ou son non importé (pas d'image de repli)
    [ole] Objet incorporé non importé (pas d'image de repli)
    [math] Équation non importée (pas d'image de repli)
    [smartart] SmartArt non importé (pas d'image de repli)
   *[unknown] Objet non pris en charge non importé (pas d'image de repli)
}
import-loss-unsupported-image = Image au format { $format } non importée
import-loss-hidden-slide = Slide masquée importée comme visible
import-loss-effects = Ombres, reflets et effets 3D perdus
import-loss-repeated-decor = { $slides ->
   [one] En-tête ou pied « { $text } » répété sur { $slides } slide omis : le thème dessine les siens
  *[other] En-tête ou pied « { $text } » répété sur { $slides } slides omis : le thème dessine les siens
}
import-loss-no-slot = Contenu omis : le modèle choisi n'a pas de place pour lui
```

`hi/document.ftl` :

```ftl
import-loss-list-levels = सूची के स्तर एक स्तर में बदल दिए गए
import-loss-merged-cells = मर्ज किए गए सेल अलग कर दिए गए
import-loss-percent-stacked = 100% स्टैक्ड चार्ट स्टैक्ड के रूप में दिखाया गया
import-loss-run-style = { $effect ->
    [underline] रेखांकन खो गया
    [strike] काट-रेखा खो गई
    [baseline] सुपरस्क्रिप्ट या सबस्क्रिप्ट खो गया
    [caps] बड़े अक्षर खो गए
   *[highlight] हाइलाइट खो गया
}
import-loss-transform = घुमाव, पलटाव या क्रॉप खो गया
import-loss-color-dropped = रंग { $color } के लिए थीम में कोई निकट रंग नहीं है: थीम का रंग इस्तेमाल हुआ
import-loss-text-truncated = सीमाओं में रखने के लिए टेक्स्ट छोटा किया गया
import-loss-table-truncated = सीमाओं में रखने के लिए तालिका छोटी की गई
import-loss-opaque = { $reason ->
    [media] वीडियो या ध्वनि आयात नहीं हुई (कोई वैकल्पिक छवि नहीं)
    [ole] एम्बेड किया गया ऑब्जेक्ट आयात नहीं हुआ (कोई वैकल्पिक छवि नहीं)
    [math] समीकरण आयात नहीं हुआ (कोई वैकल्पिक छवि नहीं)
    [smartart] SmartArt आयात नहीं हुआ (कोई वैकल्पिक छवि नहीं)
   *[unknown] असमर्थित ऑब्जेक्ट आयात नहीं हुआ (कोई वैकल्पिक छवि नहीं)
}
import-loss-unsupported-image = { $format } फ़ॉर्मैट की छवि आयात नहीं हुई
import-loss-hidden-slide = छिपी स्लाइड दिखाई देने वाली के रूप में आयात हुई
import-loss-effects = छाया, प्रतिबिंब और 3D प्रभाव खो गए
import-loss-repeated-decor = { $slides ->
   [one] { $slides } स्लाइड पर दोहराया गया हेडर या फ़ुटर “{ $text }” छोड़ दिया गया: थीम अपना बनाती है
  *[other] { $slides } स्लाइडों पर दोहराया गया हेडर या फ़ुटर “{ $text }” छोड़ दिया गया: थीम अपना बनाती है
}
import-loss-no-slot = सामग्री छोड़ दी गई: चुने गए टेम्पलेट में इसके लिए जगह नहीं है
```

`it/document.ftl` :

```ftl
import-loss-list-levels = Livelli dell'elenco ridotti a uno
import-loss-merged-cells = Celle unite separate
import-loss-percent-stacked = Grafico in pila al 100% mostrato in pila
import-loss-run-style = { $effect ->
    [underline] Sottolineatura persa
    [strike] Barrato perso
    [baseline] Apice o pedice perso
    [caps] Maiuscole perse
   *[highlight] Evidenziazione persa
}
import-loss-transform = Rotazione, capovolgimento o ritaglio persi
import-loss-color-dropped = Il colore { $color } non ha un colore vicino nel tema: usato un colore del tema
import-loss-text-truncated = Testo accorciato per rispettare i limiti
import-loss-table-truncated = Tabella accorciata per rispettare i limiti
import-loss-opaque = { $reason ->
    [media] Video o audio non importato (nessuna immagine sostitutiva)
    [ole] Oggetto incorporato non importato (nessuna immagine sostitutiva)
    [math] Equazione non importata (nessuna immagine sostitutiva)
    [smartart] SmartArt non importato (nessuna immagine sostitutiva)
   *[unknown] Oggetto non supportato non importato (nessuna immagine sostitutiva)
}
import-loss-unsupported-image = Immagine in formato { $format } non importata
import-loss-hidden-slide = Diapositiva nascosta importata come visibile
import-loss-effects = Ombre, riflessi ed effetti 3D persi
import-loss-repeated-decor = { $slides ->
   [one] Intestazione o piè di pagina «{ $text }» ripetuto su { $slides } diapositiva omesso: il tema disegna i propri
  *[other] Intestazione o piè di pagina «{ $text }» ripetuto su { $slides } diapositive omesso: il tema disegna i propri
}
import-loss-no-slot = Contenuto omesso: il modello scelto non ha posto per esso
```

`ja/document.ftl` :

```ftl
import-loss-list-levels = リストの階層を 1 段にまとめました
import-loss-merged-cells = 結合セルを解除しました
import-loss-percent-stacked = 100% 積み上げグラフを積み上げグラフとして表示しました
import-loss-run-style = { $effect ->
    [underline] 下線が失われました
    [strike] 取り消し線が失われました
    [baseline] 上付きまたは下付きが失われました
    [caps] 大文字表示が失われました
   *[highlight] 蛍光ペンが失われました
}
import-loss-transform = 回転、反転、トリミングが失われました
import-loss-color-dropped = 色 { $color } に近いテーマの色がありません。テーマの色を使用しました
import-loss-text-truncated = 上限に収まるようテキストを短くしました
import-loss-table-truncated = 上限に収まるよう表を短くしました
import-loss-opaque = { $reason ->
    [media] 動画または音声はインポートされませんでした (代替画像なし)
    [ole] 埋め込みオブジェクトはインポートされませんでした (代替画像なし)
    [math] 数式はインポートされませんでした (代替画像なし)
    [smartart] SmartArt はインポートされませんでした (代替画像なし)
   *[unknown] 未対応のオブジェクトはインポートされませんでした (代替画像なし)
}
import-loss-unsupported-image = { $format } 形式の画像はインポートされませんでした
import-loss-hidden-slide = 非表示のスライドを表示状態でインポートしました
import-loss-effects = 影、反射、3D 効果が失われました
import-loss-repeated-decor = { $slides }枚のスライドで繰り返されるヘッダーまたはフッター「{ $text }」を省きました。テーマが独自に描画します
import-loss-no-slot = コンテンツを省きました。選んだテンプレートに配置できる場所がありません
```

`ko/document.ftl` :

```ftl
import-loss-list-levels = 목록 수준을 한 단계로 합쳤습니다
import-loss-merged-cells = 병합된 셀을 분리했습니다
import-loss-percent-stacked = 100% 누적 차트를 누적 차트로 표시했습니다
import-loss-run-style = { $effect ->
    [underline] 밑줄이 사라졌습니다
    [strike] 취소선이 사라졌습니다
    [baseline] 위 첨자 또는 아래 첨자가 사라졌습니다
    [caps] 대문자 표시가 사라졌습니다
   *[highlight] 강조 표시가 사라졌습니다
}
import-loss-transform = 회전, 대칭 또는 자르기가 사라졌습니다
import-loss-color-dropped = 색 { $color }에 가까운 테마 색이 없습니다. 테마 색을 사용했습니다
import-loss-text-truncated = 한도에 맞게 텍스트를 줄였습니다
import-loss-table-truncated = 한도에 맞게 표를 줄였습니다
import-loss-opaque = { $reason ->
    [media] 동영상 또는 소리를 가져오지 않았습니다 (대체 이미지 없음)
    [ole] 포함된 개체를 가져오지 않았습니다 (대체 이미지 없음)
    [math] 수식을 가져오지 않았습니다 (대체 이미지 없음)
    [smartart] SmartArt를 가져오지 않았습니다 (대체 이미지 없음)
   *[unknown] 지원되지 않는 개체를 가져오지 않았습니다 (대체 이미지 없음)
}
import-loss-unsupported-image = { $format } 형식의 이미지를 가져오지 않았습니다
import-loss-hidden-slide = 숨긴 슬라이드를 표시 상태로 가져왔습니다
import-loss-effects = 그림자, 반사, 3D 효과가 사라졌습니다
import-loss-repeated-decor = 슬라이드 { $slides }장에서 반복되는 머리글 또는 바닥글 “{ $text }”을(를) 생략했습니다. 테마가 자체적으로 그립니다
import-loss-no-slot = 콘텐츠를 생략했습니다. 선택한 템플릿에 놓을 자리가 없습니다
```

`nl/document.ftl` :

```ftl
import-loss-list-levels = Lijstniveaus teruggebracht tot één
import-loss-merged-cells = Samengevoegde cellen gesplitst
import-loss-percent-stacked = 100%-gestapelde grafiek als gestapeld weergegeven
import-loss-run-style = { $effect ->
    [underline] Onderstreping verloren
    [strike] Doorhaling verloren
    [baseline] Superscript of subscript verloren
    [caps] Hoofdletters verloren
   *[highlight] Markering verloren
}
import-loss-transform = Draaiing, spiegeling of bijsnijding verloren
import-loss-color-dropped = De kleur { $color } heeft geen nabije themakleur: themakleur gebruikt
import-loss-text-truncated = Tekst ingekort om binnen de grenzen te blijven
import-loss-table-truncated = Tabel ingekort om binnen de grenzen te blijven
import-loss-opaque = { $reason ->
    [media] Video of geluid niet geïmporteerd (geen vervangende afbeelding)
    [ole] Ingesloten object niet geïmporteerd (geen vervangende afbeelding)
    [math] Vergelijking niet geïmporteerd (geen vervangende afbeelding)
    [smartart] SmartArt niet geïmporteerd (geen vervangende afbeelding)
   *[unknown] Niet-ondersteund object niet geïmporteerd (geen vervangende afbeelding)
}
import-loss-unsupported-image = Afbeelding in { $format }-indeling niet geïmporteerd
import-loss-hidden-slide = Verborgen dia als zichtbaar geïmporteerd
import-loss-effects = Schaduwen, weerspiegelingen en 3D-effecten verloren
import-loss-repeated-decor = { $slides ->
   [one] Kop- of voettekst “{ $text }” herhaald op { $slides } dia weggelaten: het thema tekent er zelf een
  *[other] Kop- of voettekst “{ $text }” herhaald op { $slides } dia’s weggelaten: het thema tekent er zelf een
}
import-loss-no-slot = Inhoud weggelaten: de gekozen sjabloon heeft er geen plaats voor
```

`pl/document.ftl` :

```ftl
import-loss-list-levels = Poziomy listy spłaszczone do jednego
import-loss-merged-cells = Scalone komórki rozdzielone
import-loss-percent-stacked = Wykres skumulowany 100% pokazany jako skumulowany
import-loss-run-style = { $effect ->
    [underline] Utracono podkreślenie
    [strike] Utracono przekreślenie
    [baseline] Utracono indeks górny lub dolny
    [caps] Utracono wielkie litery
   *[highlight] Utracono wyróżnienie
}
import-loss-transform = Utracono obrót, odbicie lub przycięcie
import-loss-color-dropped = Kolor { $color } nie ma bliskiego koloru w motywie: użyto koloru motywu
import-loss-text-truncated = Tekst skrócony, aby zmieścić się w limitach
import-loss-table-truncated = Tabela skrócona, aby zmieścić się w limitach
import-loss-opaque = { $reason ->
    [media] Nie zaimportowano wideo ani dźwięku (brak obrazu zastępczego)
    [ole] Nie zaimportowano osadzonego obiektu (brak obrazu zastępczego)
    [math] Nie zaimportowano równania (brak obrazu zastępczego)
    [smartart] Nie zaimportowano grafiki SmartArt (brak obrazu zastępczego)
   *[unknown] Nie zaimportowano nieobsługiwanego obiektu (brak obrazu zastępczego)
}
import-loss-unsupported-image = Nie zaimportowano obrazu w formacie { $format }
import-loss-hidden-slide = Ukryty slajd zaimportowany jako widoczny
import-loss-effects = Utracono cienie, odbicia i efekty 3D
import-loss-repeated-decor = { $slides ->
   [one] Nagłówek lub stopka „{ $text }” powtórzone na { $slides } slajdzie pominięte: motyw rysuje własne
   [few] Nagłówek lub stopka „{ $text }” powtórzone na { $slides } slajdach pominięte: motyw rysuje własne
   [many] Nagłówek lub stopka „{ $text }” powtórzone na { $slides } slajdach pominięte: motyw rysuje własne
  *[other] Nagłówek lub stopka „{ $text }” powtórzone na { $slides } slajdu pominięte: motyw rysuje własne
}
import-loss-no-slot = Treść pominięta: wybrany szablon nie ma na nią miejsca
```

`pt/document.ftl` :

```ftl
import-loss-list-levels = Níveis de lista reduzidos a um
import-loss-merged-cells = Células mescladas separadas
import-loss-percent-stacked = Gráfico empilhado 100% exibido como empilhado
import-loss-run-style = { $effect ->
    [underline] Sublinhado perdido
    [strike] Tachado perdido
    [baseline] Sobrescrito ou subscrito perdido
    [caps] Maiúsculas perdidas
   *[highlight] Realce perdido
}
import-loss-transform = Rotação, espelhamento ou corte perdidos
import-loss-color-dropped = A cor { $color } não tem cor próxima no tema: cor do tema usada
import-loss-text-truncated = Texto encurtado para caber nos limites
import-loss-table-truncated = Tabela encurtada para caber nos limites
import-loss-opaque = { $reason ->
    [media] Vídeo ou som não importado (sem imagem substituta)
    [ole] Objeto incorporado não importado (sem imagem substituta)
    [math] Equação não importada (sem imagem substituta)
    [smartart] SmartArt não importado (sem imagem substituta)
   *[unknown] Objeto não suportado não importado (sem imagem substituta)
}
import-loss-unsupported-image = Imagem no formato { $format } não importada
import-loss-hidden-slide = Slide oculto importado como visível
import-loss-effects = Sombras, reflexos e efeitos 3D perdidos
import-loss-repeated-decor = { $slides ->
   [one] Cabeçalho ou rodapé “{ $text }” repetido em { $slides } slide omitido: o tema desenha os seus
  *[other] Cabeçalho ou rodapé “{ $text }” repetido em { $slides } slides omitido: o tema desenha os seus
}
import-loss-no-slot = Conteúdo omitido: o modelo escolhido não tem lugar para ele
```

`ru/document.ftl` :

```ftl
import-loss-list-levels = Уровни списка сведены к одному
import-loss-merged-cells = Объединённые ячейки разделены
import-loss-percent-stacked = Нормированная диаграмма с накоплением показана как диаграмма с накоплением
import-loss-run-style = { $effect ->
    [underline] Подчёркивание потеряно
    [strike] Зачёркивание потеряно
    [baseline] Надстрочный или подстрочный индекс потерян
    [caps] Прописные буквы потеряны
   *[highlight] Выделение цветом потеряно
}
import-loss-transform = Поворот, отражение или обрезка потеряны
import-loss-color-dropped = Для цвета { $color } нет близкого цвета темы: использован цвет темы
import-loss-text-truncated = Текст сокращён, чтобы уложиться в ограничения
import-loss-table-truncated = Таблица сокращена, чтобы уложиться в ограничения
import-loss-opaque = { $reason ->
    [media] Видео или звук не импортированы (нет запасного изображения)
    [ole] Внедрённый объект не импортирован (нет запасного изображения)
    [math] Формула не импортирована (нет запасного изображения)
    [smartart] SmartArt не импортирован (нет запасного изображения)
   *[unknown] Неподдерживаемый объект не импортирован (нет запасного изображения)
}
import-loss-unsupported-image = Изображение в формате { $format } не импортировано
import-loss-hidden-slide = Скрытый слайд импортирован как видимый
import-loss-effects = Тени, отражения и 3D-эффекты потеряны
import-loss-repeated-decor = { $slides ->
   [one] Колонтитул «{ $text }», повторённый на { $slides } слайде, опущен: тема рисует свой
   [few] Колонтитул «{ $text }», повторённый на { $slides } слайдах, опущен: тема рисует свой
   [many] Колонтитул «{ $text }», повторённый на { $slides } слайдах, опущен: тема рисует свой
  *[other] Колонтитул «{ $text }», повторённый на { $slides } слайда, опущен: тема рисует свой
}
import-loss-no-slot = Содержимое опущено: в выбранном шаблоне для него нет места
```

`th/document.ftl` :

```ftl
import-loss-list-levels = ลดระดับรายการเหลือระดับเดียว
import-loss-merged-cells = แยกเซลล์ที่ผสานไว้
import-loss-percent-stacked = แสดงแผนภูมิแบบเรียงซ้อน 100% เป็นแบบเรียงซ้อน
import-loss-run-style = { $effect ->
    [underline] ขีดเส้นใต้หายไป
    [strike] ขีดทับหายไป
    [baseline] ตัวยกหรือตัวห้อยหายไป
    [caps] ตัวพิมพ์ใหญ่หายไป
   *[highlight] การไฮไลต์หายไป
}
import-loss-transform = การหมุน การพลิก หรือการครอบตัดหายไป
import-loss-color-dropped = สี { $color } ไม่มีสีใกล้เคียงในธีม: ใช้สีของธีมแทน
import-loss-text-truncated = ย่อข้อความให้อยู่ในขีดจำกัด
import-loss-table-truncated = ย่อตารางให้อยู่ในขีดจำกัด
import-loss-opaque = { $reason ->
    [media] ไม่ได้นำเข้าวิดีโอหรือเสียง (ไม่มีภาพสำรอง)
    [ole] ไม่ได้นำเข้าวัตถุที่ฝังไว้ (ไม่มีภาพสำรอง)
    [math] ไม่ได้นำเข้าสมการ (ไม่มีภาพสำรอง)
    [smartart] ไม่ได้นำเข้า SmartArt (ไม่มีภาพสำรอง)
   *[unknown] ไม่ได้นำเข้าวัตถุที่ไม่รองรับ (ไม่มีภาพสำรอง)
}
import-loss-unsupported-image = ไม่ได้นำเข้าภาพรูปแบบ { $format }
import-loss-hidden-slide = นำเข้าสไลด์ที่ซ่อนไว้เป็นสไลด์ที่มองเห็น
import-loss-effects = เงา การสะท้อน และเอฟเฟกต์ 3 มิติหายไป
import-loss-repeated-decor = ละส่วนหัวหรือท้ายกระดาษ “{ $text }” ที่ซ้ำใน { $slides } สไลด์: ธีมวาดของตัวเอง
import-loss-no-slot = ละเนื้อหา: เทมเพลตที่เลือกไม่มีที่สำหรับเนื้อหานี้
```

`tr/document.ftl` :

```ftl
import-loss-list-levels = Liste düzeyleri tek düzeye indirildi
import-loss-merged-cells = Birleştirilmiş hücreler ayrıldı
import-loss-percent-stacked = %100 yığılmış grafik yığılmış olarak gösterildi
import-loss-run-style = { $effect ->
    [underline] Altı çizili biçim kayboldu
    [strike] Üstü çizili biçim kayboldu
    [baseline] Üst simge veya alt simge kayboldu
    [caps] Büyük harfler kayboldu
   *[highlight] Vurgulama kayboldu
}
import-loss-transform = Döndürme, çevirme veya kırpma kayboldu
import-loss-color-dropped = { $color } renginin temada yakın bir rengi yok: tema rengi kullanıldı
import-loss-text-truncated = Metin sınırlara sığması için kısaltıldı
import-loss-table-truncated = Tablo sınırlara sığması için kısaltıldı
import-loss-opaque = { $reason ->
    [media] Video veya ses içe aktarılmadı (yedek görüntü yok)
    [ole] Gömülü nesne içe aktarılmadı (yedek görüntü yok)
    [math] Denklem içe aktarılmadı (yedek görüntü yok)
    [smartart] SmartArt içe aktarılmadı (yedek görüntü yok)
   *[unknown] Desteklenmeyen nesne içe aktarılmadı (yedek görüntü yok)
}
import-loss-unsupported-image = { $format } biçimindeki görüntü içe aktarılmadı
import-loss-hidden-slide = Gizli slayt görünür olarak içe aktarıldı
import-loss-effects = Gölgeler, yansımalar ve 3B efektler kayboldu
import-loss-repeated-decor = { $slides } slaytta tekrarlanan “{ $text }” üst veya alt bilgisi atlandı: tema kendisininkini çizer
import-loss-no-slot = İçerik atlandı: seçilen şablonda ona yer yok
```

`uk/document.ftl` :

```ftl
import-loss-list-levels = Рівні списку зведено до одного
import-loss-merged-cells = Об'єднані клітинки розділено
import-loss-percent-stacked = Нормовану діаграму з накопиченням показано як діаграму з накопиченням
import-loss-run-style = { $effect ->
    [underline] Підкреслення втрачено
    [strike] Закреслення втрачено
    [baseline] Надрядковий або підрядковий індекс втрачено
    [caps] Великі літери втрачено
   *[highlight] Виділення кольором втрачено
}
import-loss-transform = Поворот, віддзеркалення або обрізання втрачено
import-loss-color-dropped = Для кольору { $color } немає близького кольору теми: використано колір теми
import-loss-text-truncated = Текст скорочено, щоб вкластися в обмеження
import-loss-table-truncated = Таблицю скорочено, щоб вкластися в обмеження
import-loss-opaque = { $reason ->
    [media] Відео або звук не імпортовано (немає запасного зображення)
    [ole] Вбудований об'єкт не імпортовано (немає запасного зображення)
    [math] Формулу не імпортовано (немає запасного зображення)
    [smartart] SmartArt не імпортовано (немає запасного зображення)
   *[unknown] Непідтримуваний об'єкт не імпортовано (немає запасного зображення)
}
import-loss-unsupported-image = Зображення у форматі { $format } не імпортовано
import-loss-hidden-slide = Прихований слайд імпортовано як видимий
import-loss-effects = Тіні, відображення та 3D-ефекти втрачено
import-loss-repeated-decor = { $slides ->
   [one] Колонтитул «{ $text }», повторений на { $slides } слайді, пропущено: тема малює власний
   [few] Колонтитул «{ $text }», повторений на { $slides } слайдах, пропущено: тема малює власний
   [many] Колонтитул «{ $text }», повторений на { $slides } слайдах, пропущено: тема малює власний
  *[other] Колонтитул «{ $text }», повторений на { $slides } слайда, пропущено: тема малює власний
}
import-loss-no-slot = Вміст пропущено: у вибраному шаблоні для нього немає місця
```

`vi/document.ftl` :

```ftl
import-loss-list-levels = Các cấp danh sách được gộp thành một cấp
import-loss-merged-cells = Các ô đã hợp nhất được tách ra
import-loss-percent-stacked = Biểu đồ xếp chồng 100% được hiển thị dạng xếp chồng
import-loss-run-style = { $effect ->
    [underline] Mất gạch chân
    [strike] Mất gạch ngang
    [baseline] Mất chỉ số trên hoặc chỉ số dưới
    [caps] Mất chữ in hoa
   *[highlight] Mất tô sáng
}
import-loss-transform = Mất xoay, lật hoặc cắt xén
import-loss-color-dropped = Màu { $color } không có màu gần giống trong chủ đề: đã dùng màu của chủ đề
import-loss-text-truncated = Văn bản đã được rút gọn để nằm trong giới hạn
import-loss-table-truncated = Bảng đã được rút gọn để nằm trong giới hạn
import-loss-opaque = { $reason ->
    [media] Không nhập video hoặc âm thanh (không có ảnh thay thế)
    [ole] Không nhập đối tượng nhúng (không có ảnh thay thế)
    [math] Không nhập phương trình (không có ảnh thay thế)
    [smartart] Không nhập SmartArt (không có ảnh thay thế)
   *[unknown] Không nhập đối tượng không được hỗ trợ (không có ảnh thay thế)
}
import-loss-unsupported-image = Không nhập ảnh định dạng { $format }
import-loss-hidden-slide = Trang chiếu ẩn được nhập dưới dạng hiển thị
import-loss-effects = Mất bóng đổ, phản chiếu và hiệu ứng 3D
import-loss-repeated-decor = Đã bỏ đầu trang hoặc chân trang “{ $text }” lặp lại trên { $slides } trang chiếu: chủ đề tự vẽ của nó
import-loss-no-slot = Đã bỏ nội dung: mẫu đã chọn không có chỗ cho nó
```

`zh-Hans/document.ftl` :

```ftl
import-loss-list-levels = 列表级别已合并为一级
import-loss-merged-cells = 已拆分合并的单元格
import-loss-percent-stacked = 百分比堆积图已显示为堆积图
import-loss-run-style = { $effect ->
    [underline] 下划线已丢失
    [strike] 删除线已丢失
    [baseline] 上标或下标已丢失
    [caps] 大写已丢失
   *[highlight] 突出显示已丢失
}
import-loss-transform = 旋转、翻转或裁剪已丢失
import-loss-color-dropped = 颜色 { $color } 在主题中没有相近的颜色：已使用主题颜色
import-loss-text-truncated = 文本已缩短以符合限制
import-loss-table-truncated = 表格已缩短以符合限制
import-loss-opaque = { $reason ->
    [media] 视频或声音未导入（无替代图像）
    [ole] 嵌入对象未导入（无替代图像）
    [math] 公式未导入（无替代图像）
    [smartart] SmartArt 未导入（无替代图像）
   *[unknown] 不支持的对象未导入（无替代图像）
}
import-loss-unsupported-image = { $format } 格式的图像未导入
import-loss-hidden-slide = 隐藏的幻灯片已作为可见幻灯片导入
import-loss-effects = 阴影、映像和三维效果已丢失
import-loss-repeated-decor = 已省略在 { $slides }张幻灯片上重复的页眉或页脚“{ $text }”：主题会绘制自己的页眉页脚
import-loss-no-slot = 已省略内容：所选模板中没有可放置它的位置
```

`zh-Hant/document.ftl` :

```ftl
import-loss-list-levels = 清單層級已合併為一層
import-loss-merged-cells = 已分割合併的儲存格
import-loss-percent-stacked = 百分比堆疊圖已顯示為堆疊圖
import-loss-run-style = { $effect ->
    [underline] 底線已遺失
    [strike] 刪除線已遺失
    [baseline] 上標或下標已遺失
    [caps] 大寫已遺失
   *[highlight] 醒目提示已遺失
}
import-loss-transform = 旋轉、翻轉或裁剪已遺失
import-loss-color-dropped = 色彩 { $color } 在佈景主題中沒有相近的色彩：已使用佈景主題色彩
import-loss-text-truncated = 文字已縮短以符合限制
import-loss-table-truncated = 表格已縮短以符合限制
import-loss-opaque = { $reason ->
    [media] 影片或聲音未匯入（無替代影像）
    [ole] 內嵌物件未匯入（無替代影像）
    [math] 方程式未匯入（無替代影像）
    [smartart] SmartArt 未匯入（無替代影像）
   *[unknown] 不支援的物件未匯入（無替代影像）
}
import-loss-unsupported-image = { $format } 格式的影像未匯入
import-loss-hidden-slide = 隱藏的投影片已以可見方式匯入
import-loss-effects = 陰影、反射與立體效果已遺失
import-loss-repeated-decor = 已省略在 { $slides }張投影片上重複的頁首或頁尾「{ $text }」：佈景主題會繪製自己的頁首頁尾
import-loss-no-slot = 已省略內容：所選範本中沒有可放置它的位置
```

- [ ] **Step 4 : lancer, constater le succès**

```bash
cargo nextest run -p oku-slides-import -p oku-slides-i18n
```

Attendu : tout passe (347 tests au brouillon) ; côté i18n, `every_locale_has_exactly_the_reference_keys`, `every_locale_parses_without_error` et `every_locale_formats_every_message_without_errors` couvrent les nouvelles clés.

- [ ] **Step 5 : vérifier, commit, sous-tâche**

```bash
cargo fmt --all && cargo clippy -p oku-slides-import -p oku-slides-i18n --all-targets -- -D warnings && cargo nextest run -p oku-slides-import -p oku-slides-i18n
git add Cargo.lock crates/oku-slides-import/Cargo.toml crates/oku-slides-import/src/lib.rs crates/oku-slides-import/src/plan.rs crates/oku-slides-import/src/plan_tests.rs crates/oku-slides-import/src/report.rs crates/oku-slides-import/src/report_tests.rs crates/oku-slides-i18n/src/locales && git diff --cached --name-only
git commit -F <scratchpad>/msg.txt   # feat(import) : plan, rapport et pertes de l'import, messages en 20 langues (t-692)
oku cli task done --project slider --id t-798
```

---

### Task 3: Couleurs — OKLab, ΔE, table du schéma, accents et contraste

**Écarts d'interface :**
- `project_color` rend `Result<Option<String>, LossKind>` : `Ok(None)` = valeur par défaut du lecteur, `Ok(Some(jeton))`, `Err(LossKind::ColorDropped(hex))`.
- `ColorTokens::from_tokens` prend les **jetons effectifs entiers** du thème (et non le seul groupe `color`) : il y lit aussi les fonds d'une carte, `component.card.bg` et `component.card.highlight-bg` (noms de couleur), dont la règle de contraste a besoin. `ColorTokens` devient une structure à champs privés (couleurs, fonds de carte, accents déduits).
- `ColorTokens` gagne `get`, `contains`, `with_accents(self, deck: &SourceDeck) -> Self` (publics), `nearest`, `card(highlight)`, `page()`, `on_dark(bg)` (`pub(crate)`) ; `Oklab` gagne `chroma()`. `slot_token`, `contrast`, `is_dark`, `text_color` et `MIN_CONTRAST` sont `pub(crate)`.
- Règle ajoutée entre la table du schéma et le plus proche : un noir neutre devient `ink`, un blanc neutre `bg` (voir Règles) ; sans elle, tout run noir explicite d'un PowerPoint serait une perte (`#000000` est à 29 de l'encre de Papier).
- `Item::text(&self) -> Option<&TextBox>` et `Item::has_text(&self) -> bool` arrivent ici dans `source/mod.rs` (lot 1) : le poids d'usage des accents lit le texte de chaque item, et toutes les tâches suivantes s'en servent (une seule définition, révision 1, item 12).
- Crée `src/test_support.rs` (`cfg(test)`) avec les jetons effectifs des thèmes intégrés (par `oku_slides_core::tokens::effective_tokens`, tâche 1) et les constructeurs de texte, d'items, de slides et de decks source ; les tâches suivantes y ajoutent leurs aides (registre, modèles).

Les couleurs émises par le retrofit sont toujours des noms de jetons du thème cible (décision 9) : le rendu Typst résout un nom par `color-of` et panique sur un nom inconnu. `ColorTokens` est construit par l'appelant depuis les jetons effectifs du thème (défauts de `default-tokens.json` fusionnés avec ceux du thème) ; une couleur y est un `#hex` ou le nom d'un autre jeton (alias, résolu comme `color-of`).

Deux règles de la révision 1 vivent ici. **Accents déduits** : un deck qui n'emploie pas le schéma de couleurs (alto écrit toutes ses couleurs en dur) perdrait ses couleurs de marque dans un thème lointain ; ses couleurs saturées sans jeton proche, rangées par usage, prennent `accent` puis les autres clés `accent-*` du thème cible. **Contraste** : un jeton de texte n'est émis que lisible sur son fond (WCAG ≥ 3:1), jamais en dessous ; sur un fond sombre, un texte clair prend un jeton de texte clair du thème.

**Files:**
- Create: `crates/oku-slides-import/src/retrofit/mod.rs`, `crates/oku-slides-import/src/retrofit/color.rs`, `crates/oku-slides-import/src/retrofit/color_tests.rs`, `crates/oku-slides-import/src/test_support.rs`
- Modify: `crates/oku-slides-import/src/lib.rs` (`pub mod retrofit;`, `#[cfg(test)] mod test_support;`), `crates/oku-slides-import/src/source/mod.rs` (`impl Item`), `crates/oku-slides-import/src/source/mod_tests.rs` (un test)

**Interfaces:**
- Consumes : `crate::source::{Color, Origin, Rgba, ColorMod, Styled, Fill, Item, ItemKind, SourceDeck, TextBox}` (lot 1) ; `crate::report::LossKind` (tâche 2) ; `oku_slides_core::tokens::effective_tokens` (tâche 1, aides de test).
- Produces :

```rust
// crate::retrofit (ré-export de retrofit::color)
pub struct ColorTokens { /* couleurs, fonds de carte, accents déduits */ }   // Debug, Clone, PartialEq, Default
impl ColorTokens {
    pub fn from_tokens(tokens: &serde_json::Map<String, Value>) -> Self;   // jetons effectifs entiers
    pub fn get(&self, name: &str) -> Option<Rgba>;
    pub fn contains(&self, name: &str) -> bool;
    pub fn with_accents(self, deck: &SourceDeck) -> Self;
    pub(crate) fn card(&self, highlight: bool) -> Rgba;
    pub(crate) fn page(&self) -> Rgba;
    pub(crate) fn on_dark(&self, bg: Rgba) -> Option<&'static str>;
    pub(crate) fn nearest(&self, c: Rgba) -> Option<(&str, f64)>;
}
// crate::retrofit::color
pub(crate) const MAX_DELTA_E: f64 = 12.0;
pub(crate) const MIN_CONTRAST: f64 = 3.0;
pub(crate) struct Oklab { pub l: f64, pub a: f64, pub b: f64 }   // chroma()
pub(crate) fn oklab(c: Rgba) -> Oklab;
pub(crate) fn delta_e(x: Oklab, y: Oklab) -> f64;
pub(crate) fn contrast(a: Rgba, b: Rgba) -> f64;
pub(crate) fn is_dark(c: Rgba) -> bool;
pub(crate) fn slot_token(slot: &str) -> Option<&'static str>;
pub(crate) fn project_color(c: &Color, tokens: &ColorTokens) -> Result<Option<String>, LossKind>;
pub(crate) fn text_color(c: &Color, bg: Rgba, tokens: &ColorTokens) -> Result<Option<String>, LossKind>;
// crate::source (lot 1)
impl Item { pub fn text(&self) -> Option<&TextBox>; pub fn has_text(&self) -> bool; }
// crate::test_support (cfg(test))
pub(crate) const DEFAULT_TOKENS: &str; pub(crate) const PAPIER: &str; pub(crate) const PRO: &str;
pub(crate) fn tokens(theme: &str) -> serde_json::Map<String, Value>;  // effective_tokens(défauts, jetons du thème)
pub(crate) fn colors(theme: &str) -> ColorTokens;
pub(crate) fn tx1() -> Color; pub(crate) fn run_style() -> RunStyle; pub(crate) fn run(text: &str) -> Run;
pub(crate) fn run_with(text: &str, f: impl FnOnce(&mut RunStyle)) -> Run;
pub(crate) fn para(runs: Vec<Run>) -> Paragraph; pub(crate) fn text_box(paragraphs: Vec<Paragraph>) -> TextBox;
pub(crate) fn item(id: &str, frame: (f64, f64, f64, f64), kind: ItemKind) -> Item;
pub(crate) fn source_slide(number: u32, layout: Option<&str>, items: Vec<Item>) -> SourceSlide;
pub(crate) fn source_deck(slides: Vec<SourceSlide>) -> SourceDeck;   // 960 × 540, sans thème ni asset
```

- Règles (`project_color`, dans l'ordre) :

| Couleur source | Résultat |
|---|---|
| `Origin::Default` | `Ok(None)` : le thème cible décide, sans perte |
| `Origin::Scheme` sans modificateur, emplacement `dk1`/`lt1`/`dk2`/`lt2`/`accent1`/`hlink`, jeton cible présent | `ink` / `bg` / `ink-muted` / `surface` / `accent` / `accent` |
| accent déduit du deck (`with_accents`) pour cette valeur (opacité ignorée) | son jeton |
| neutre (chroma OKLab < 0,02) de clarté < 0,25, `ink` présent | `ink` |
| neutre de clarté > 0,95, `bg` présent | `bg` |
| jeton le plus proche à `ΔE ≤ 12` (à égalité : le premier par nom) | ce jeton |
| sinon | `Err(ColorDropped("#rrggbb"))` (8 chiffres si la couleur n'est pas opaque) |

  `accent2..6`, `folHlink`, un emplacement modifié (`lumMod`…) ou un jeton cible absent passent au plus proche. L'opacité est ignorée. `ΔE` = distance euclidienne OKLab × 100 ; seuil 12 : environ six différences juste perceptibles, assez pour garder une nuance de la même famille, pas assez pour changer de teinte (valeurs dans la doc de `MAX_DELTA_E` et la note des tests).
- Règles (`with_accents`) : les couleurs `Origin::Explicit` du deck (runs, fonds unis, traits, groupes compris) pèsent leur usage en pt² (caractères × taille² pour un run, surface pour un fond, longueur × épaisseur pour un trait) ; celles de chroma OKLab ≥ 0,08 dont le jeton le plus proche est au-delà de `MAX_DELTA_E` sont rangées par usage décroissant (à égalité, par valeur hexadécimale) et prennent dans l'ordre `accent` (s'il existe) puis les autres clés `accent-*` du thème, par nom. Les suivantes restent sans accent (plus proche ou perte). Seules des clés existantes (décision 9).
- Règles (contraste) : `contrast` = rapport WCAG des luminances relatives, `(L1 + 0,05) / (L2 + 0,05)`. `text_color(c, bg)` garde le jeton de `project_color` s'il fait au moins 3:1 sur `bg` ; sinon, un texte plus clair qu'un fond sombre (`is_dark` : le blanc y contraste plus que le noir) prend le premier de `on-dark-body`, `white`, `on-accent` présent et lisible sur `bg` (`on_dark`) ; sinon rien (la couleur du thème). Jamais un jeton sous 3:1. Fonds : `card(false)` = `component.card.bg`, `card(true)` = `component.card.highlight-bg` (replis `bg` et `ink`, ceux du rendu), `page()` = `bg`.

- [ ] **Step 1 : tests qui échouent**

`crates/oku-slides-import/src/test_support.rs` :

```rust
//! Aides de test du projecteur : jetons effectifs des thèmes intégrés, texte, items,
//! slides et decks source.

use std::collections::BTreeMap;

use oku_slides_core::tokens::effective_tokens;
use serde_json::{Map, Value};

use crate::retrofit::ColorTokens;
use crate::source::{
    Align, Bullet, Caps, Color, Fill, Frame, Item, ItemKind, ParaStyle, Paragraph, Rgba, Run, RunStyle, Size,
    SourceDeck, SourceSlide, Spacing, Styled, TextBox,
};

/// Jetons par défaut du rendu.
pub(crate) const DEFAULT_TOKENS: &str = include_str!("../../../typst/oku-slides/src/default-tokens.json");
/// Thème intégré Papier (`theme.json`).
pub(crate) const PAPIER: &str = include_str!("../../../typst/oku-slides/themes/papier.json");
/// Thème intégré Pro (`theme.json`).
pub(crate) const PRO: &str = include_str!("../../../typst/oku-slides/themes/pro.json");

/// Les jetons effectifs du thème `theme` (un `theme.json`) : les défauts, puis les siens.
pub(crate) fn tokens(theme: &str) -> Map<String, Value> {
    let Value::Object(defaults) = serde_json::from_str(DEFAULT_TOKENS).unwrap() else { panic!("jetons par défaut") };
    let theme: Value = serde_json::from_str(theme).unwrap();
    effective_tokens(&defaults, theme["tokens"].as_object().unwrap())
}

/// Les couleurs effectives du thème `theme`.
pub(crate) fn colors(theme: &str) -> ColorTokens {
    ColorTokens::from_tokens(&tokens(theme))
}

/// Couleur du texte par défaut d'un thème Office : `tx1`, emplacement `dk1`, noir.
pub(crate) fn tx1() -> Color {
    Styled::mapped(Rgba::new(0, 0, 0), "tx1", "dk1", Vec::new())
}

/// Style de run neutre : Calibri 18 pt, `tx1`, sans effet.
pub(crate) fn run_style() -> RunStyle {
    RunStyle {
        font: Styled::mapped("Calibri".to_owned(), "+mn-lt", "minor", Vec::new()),
        size: 18.0,
        bold: false,
        italic: false,
        underline: false,
        strike: false,
        color: tx1(),
        highlight: None,
        baseline: 0,
        caps: Caps::None,
        char_spacing: 0.0,
        link: None,
    }
}

/// Run `text` au style neutre.
pub(crate) fn run(text: &str) -> Run {
    Run { text: text.to_owned(), field: None, style: run_style() }
}

/// Run `text` au style `f(neutre)`.
pub(crate) fn run_with(text: &str, f: impl FnOnce(&mut RunStyle)) -> Run {
    let mut r = run(text);
    f(&mut r.style);
    r
}

/// Paragraphe de niveau 0 sans puce.
pub(crate) fn para(runs: Vec<Run>) -> Paragraph {
    Paragraph {
        level: 0,
        style: ParaStyle {
            align: Align::Left,
            indent: 0.0,
            margin_left: 0.0,
            line_spacing: Spacing::Percent(1.0),
            space_before: Spacing::Points(0.0),
            space_after: Spacing::Points(0.0),
            bullet: Bullet::None,
        },
        runs,
    }
}

/// Boîte de texte aux paragraphes `paragraphs`.
pub(crate) fn text_box(paragraphs: Vec<Paragraph>) -> TextBox {
    TextBox { paragraphs, ..TextBox::default() }
}

/// Un item `kind` d'id `id` dans la boîte `(x, y, w, h)` (pt source), sans rôle.
pub(crate) fn item(id: &str, (x, y, w, h): (f64, f64, f64, f64), kind: ItemKind) -> Item {
    Item {
        id: id.to_owned(),
        name: id.to_owned(),
        alt: None,
        frame: Frame::new(x, y, w, h),
        z: 0,
        role: None,
        placeholder: None,
        kind,
    }
}

/// La slide source `number`, de layout `layout`, aux items `items`.
pub(crate) fn source_slide(number: u32, layout: Option<&str>, items: Vec<Item>) -> SourceSlide {
    SourceSlide {
        id: format!("s{number}"),
        number,
        layout: layout.map(str::to_owned),
        background: Fill::None,
        show_layout_decor: true,
        hidden: false,
        items,
        notes: String::new(),
    }
}

/// Un deck source 960 × 540 pt aux slides `slides`, sans thème, layout ni asset.
pub(crate) fn source_deck(slides: Vec<SourceSlide>) -> SourceDeck {
    SourceDeck {
        size: Size { w: 960.0, h: 540.0 },
        themes: Vec::new(),
        masters: Vec::new(),
        layouts: Vec::new(),
        slides,
        assets: BTreeMap::new(),
        fonts: Vec::new(),
        warnings: Vec::new(),
    }
}
```

`crates/oku-slides-import/src/retrofit/color_tests.rs` :

```rust
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::source::{ColorMod, Fill, Geometry, ItemKind, Shape as SourceShape, SourceDeck, Styled};
use crate::test_support::{PAPIER, PRO, colors, item, para, run_with, source_deck, source_slide, text_box};

fn hex(s: &str) -> Rgba {
    parse_hex(s).unwrap()
}

fn close(x: Oklab, l: f64, a: f64, b: f64) -> bool {
    (x.l - l).abs() < 1e-4 && (x.a - a).abs() < 1e-4 && (x.b - b).abs() < 1e-4
}

#[test]
fn oklab_of_white_black_and_red_matches_the_reference_values() {
    assert_that!(close(oklab(hex("#ffffff")), 1.0, 0.0, 0.0)).is_true();
    assert_that!(close(oklab(hex("#000000")), 0.0, 0.0, 0.0)).is_true();
    assert_that!(close(oklab(hex("#ff0000")), 0.627_955, 0.224_863, 0.125_846)).is_true();
}

#[test]
fn delta_e_between_black_and_white_is_one_hundred() {
    assert_that!(delta_e(oklab(hex("#000000")), oklab(hex("#ffffff")))).is_close_to(100.0, 1e-3);
}

#[test]
fn oklab_chroma_of_a_grey_is_zero() {
    assert_that!(oklab(hex("#808080")).chroma()).is_less_than(1e-6);
}

#[rstest]
#[case("#abc", Some(Rgba::new(0xaa, 0xbb, 0xcc)))]
#[case("#2e2b26", Some(Rgba::new(0x2e, 0x2b, 0x26)))]
#[case("#2e2b2680", Some(Rgba { r: 0x2e, g: 0x2b, b: 0x26, a: 0x80 }))]
#[case("#2e2b2", None)]
#[case("#ggg", None)]
#[case("2e2b26", None)]
#[case("#+1+", None)]
fn parse_hex_reads_three_six_and_eight_digits_only(#[case] s: &str, #[case] expected: Option<Rgba>) {
    assert_that!(parse_hex(s)).is_equal_to(expected);
}

#[test]
fn color_tokens_from_tokens_resolves_aliases_and_skips_the_unresolvable() {
    let theme = json!({ "color": {
        "ink": "#000", "text": "ink", "body": "text",
        "loop-a": "loop-b", "loop-b": "loop-a",
        "lost": "nowhere", "number": 3, "bad": "#12"
    }});
    let tokens = ColorTokens::from_tokens(theme.as_object().unwrap());
    assert_that!(tokens.get("body")).is_equal_to(Some(Rgba::new(0, 0, 0)));
    assert_that!(tokens.get("text")).is_equal_to(Some(Rgba::new(0, 0, 0)));
    for skipped in ["loop-a", "loop-b", "lost", "number", "bad"] {
        assert_that!(tokens.contains(skipped)).named(skipped).is_false();
    }
}

#[test]
fn color_tokens_of_papier_carry_the_theme_over_the_defaults() {
    let papier = colors(PAPIER);
    assert_that!(papier.get("ink")).is_equal_to(Some(Rgba::new(0x2e, 0x2b, 0x26)));
    assert_that!(papier.contains("accent-soft")).is_true();
}

/// Pro : `accent` et `accent-on-dark` valent tous deux `#00bef4` ; `rule` et
/// `surface` tous deux `#e1e5ea`. À égalité, le premier par nom l'emporte.
#[rstest]
#[case("#00bef4", "accent")]
#[case("#e7e6e6", "rule")]
fn color_tokens_nearest_on_a_tie_takes_the_first_name(#[case] c: &str, #[case] expected: &str) {
    let pro = colors(PRO);
    assert_that!(pro.nearest(hex(c)).map(|(n, _)| n)).is_equal_to(Some(expected));
}

#[test]
fn color_tokens_nearest_without_tokens_is_none() {
    assert_that!(ColorTokens::default().nearest(Rgba::new(1, 2, 3))).is_none();
}

fn scheme(slot: &str, value: &str) -> Color {
    Styled::mapped(hex(value), slot, slot, Vec::new())
}

/// Écarts (×100) au jeton le plus proche, calculés en OKLab :
/// Papier — `#40695f`→`accent` 0,7 ; `#333333`→`ink` 3,2 ; `#ff0000`→`ink-muted` 27,9 ;
/// `#ed7d31`→`rule` 23,1 ; `#2f5597`→`accent` 13,1.
/// Pro — `#4472c4`→`accent-text` 5,1 ; `#00b0f0`→`accent` 3,9 ; `#2f5597`→`accent-text` 8,4.
#[rstest]
#[case::default_value(PAPIER, Styled::default_value(hex("#123456")), Ok(None))]
#[case::dk1(PAPIER, scheme("dk1", "#000000"), Ok(Some("ink")))]
#[case::lt1(PAPIER, scheme("lt1", "#ffffff"), Ok(Some("bg")))]
#[case::dk2(PAPIER, scheme("dk2", "#44546a"), Ok(Some("ink-muted")))]
#[case::lt2(PAPIER, scheme("lt2", "#e7e6e6"), Ok(Some("surface")))]
#[case::accent1(PAPIER, scheme("accent1", "#4472c4"), Ok(Some("accent")))]
#[case::hlink(PAPIER, scheme("hlink", "#0563c1"), Ok(Some("accent")))]
#[case::accent2_far(PAPIER, scheme("accent2", "#ed7d31"), Err("#ed7d31"))]
#[case::modified_accent1_far(
    PAPIER,
    Styled::mapped(hex("#2f5597"), "accent1", "accent1", vec![ColorMod::LumMod(0.75)]),
    Err("#2f5597")
)]
#[case::modified_accent1_near(
    PRO,
    Styled::mapped(hex("#2f5597"), "accent1", "accent1", vec![ColorMod::LumMod(0.75)]),
    Ok(Some("accent-text"))
)]
#[case::pure_black(PAPIER, Styled::explicit(hex("#000000")), Ok(Some("ink")))]
#[case::pure_white(PRO, Styled::explicit(hex("#ffffff")), Ok(Some("bg")))]
#[case::dark_grey(PAPIER, Styled::explicit(hex("#333333")), Ok(Some("ink")))]
#[case::near_accent(PAPIER, Styled::explicit(hex("#40695f")), Ok(Some("accent")))]
#[case::red(PAPIER, Styled::explicit(hex("#ff0000")), Err("#ff0000"))]
#[case::office_blue(PRO, Styled::explicit(hex("#4472c4")), Ok(Some("accent-text")))]
#[case::cyan(PRO, Styled::explicit(hex("#00b0f0")), Ok(Some("accent")))]
fn project_color_maps_to_a_theme_token_or_drops(
    #[case] theme: &str,
    #[case] c: Color,
    #[case] expected: Result<Option<&str>, &str>,
) {
    let expected = expected.map(|o| o.map(str::to_owned)).map_err(|h| LossKind::ColorDropped(h.to_owned()));
    assert_that!(project_color(&c, &colors(theme))).is_equal_to(expected);
}

#[test]
fn project_color_of_a_slot_whose_token_is_missing_takes_the_nearest() {
    let theme = json!({ "color": { "bg": "#ffffff", "paper": "#e8e8e8" } });
    let tokens = ColorTokens::from_tokens(theme.as_object().unwrap());
    assert_that!(project_color(&scheme("lt2", "#e7e6e6"), &tokens)).is_equal_to(Ok(Some("paper".to_owned())));
}

#[test]
fn project_color_ignores_opacity() {
    let c = Styled::explicit(Rgba { r: 0x40, g: 0x69, b: 0x5f, a: 0x40 });
    assert_that!(project_color(&c, &colors(PAPIER))).is_equal_to(Ok(Some("accent".to_owned())));
}

#[test]
fn project_color_dropped_with_opacity_names_the_eight_digit_hex() {
    let c = Styled::explicit(Rgba { r: 0xff, g: 0, b: 0, a: 0x80 });
    assert_that!(project_color(&c, &colors(PAPIER))).is_equal_to(Err(LossKind::ColorDropped("#ff000080".to_owned())));
}

#[rstest]
#[case("#000000", "#ffffff", 21.0)]
#[case("#ffffff", "#ffffff", 1.0)]
#[case("#00bef4", "#ffffff", 2.17)]
#[case("#ffffff", "#00154e", 17.25)]
#[case("#b9c4d6", "#fffdf9", 1.73)]
fn contrast_follows_the_wcag_formula_in_both_orders(#[case] a: &str, #[case] b: &str, #[case] expected: f64) {
    assert_that!(contrast(hex(a), hex(b))).is_close_to(expected, 0.01);
    assert_that!(contrast(hex(b), hex(a))).is_close_to(expected, 0.01);
}

#[test]
fn color_tokens_card_backgrounds_follow_the_card_component() {
    let pro = colors(PRO);
    assert_that!((pro.card(false), pro.card(true))).is_equal_to((hex("#ffffff"), hex("#00154e")));
    // Papier garde les défauts du rendu : `bg`, et `ink` pour le relief.
    let papier = colors(PAPIER);
    assert_that!((papier.card(false), papier.card(true))).is_equal_to((hex("#fffdf9"), hex("#2e2b26")));
}

#[test]
fn color_tokens_on_dark_prefers_on_dark_body_then_white_then_on_accent() {
    assert_that!(colors(PRO).on_dark(hex("#00154e"))).is_equal_to(Some("on-dark-body"));
    assert_that!(colors(PAPIER).on_dark(hex("#2e2b26"))).is_equal_to(Some("on-accent"));
    assert_that!(colors(PAPIER).on_dark(hex("#fffdf9"))).is_none();
}

/// Un run de 10 caractères en 10 pt pèse 1 000 pt² ; un fond de 100 × 50, 5 000 pt².
fn deck_with(colors: &[(&str, f64)]) -> SourceDeck {
    let items = colors
        .iter()
        .enumerate()
        .map(|(k, (c, area))| {
            let shape = SourceShape {
                geom: Geometry::Preset { name: "rect".into(), adj: Default::default() },
                fill: Fill::Solid { color: Styled::explicit(hex(c)) },
                stroke: None,
                text: None,
            };
            item(&format!("s1/{k}"), (0.0, 0.0, *area, 1.0), ItemKind::Shape(Box::new(shape)))
        })
        .chain([item(
            "s1/t",
            (0.0, 0.0, 10.0, 10.0),
            ItemKind::Text(text_box(vec![para(vec![run_with("0123456789", |s| {
                s.size = 10.0;
                s.color = Styled::explicit(hex("#ff0000"));
            })])])),
        )])
        .collect();
    source_deck(vec![source_slide(1, None, items)])
}

/// Papier : `#00154e` (bleu nuit, ΔE 13,4 de `ink`) et `#00bef4` (cyan, 20,7 de
/// `rule`) n'ont pas de jeton proche ; le rouge (1 000 pt²) non plus.
/// `#3f6b5e` est l'accent de Papier : il ne compte pas. `#808080` est neutre.
#[test]
fn color_tokens_with_accents_ranks_saturated_colors_without_a_close_token_by_usage() {
    let deck = deck_with(&[("#00bef4", 300.0), ("#00154e", 5000.0), ("#3f6b5e", 9000.0), ("#808080", 9000.0)]);
    let tokens = colors(PAPIER).with_accents(&deck);
    let projected: Vec<_> =
        ["#00154e", "#ff0000", "#00bef4"].iter().map(|c| project_color(&Styled::explicit(hex(c)), &tokens)).collect();
    assert_that!(projected).is_equal_to(vec![
        Ok(Some("accent".to_owned())),
        Ok(Some("accent-soft".to_owned())),
        Err(LossKind::ColorDropped("#00bef4".to_owned())),
    ]);
}

#[test]
fn color_tokens_with_accents_leaves_a_theme_with_close_tokens_alone() {
    let deck = deck_with(&[("#00bef4", 5000.0), ("#00154e", 300.0)]);
    let tokens = colors(PRO).with_accents(&deck);
    assert_that!(project_color(&Styled::explicit(hex("#00154e")), &tokens))
        .is_equal_to(Ok(Some("brand-ink".to_owned())));
}

/// Pro : un titre blanc sur le bandeau `#00154e` reste lisible (`bg`, 17,3:1) ;
/// `accent` (`#00bef4`) sur blanc ne fait que 2,2:1 : rien. Papier : le gris
/// bleuté `#b9c4d6` se projette sur `rule` (`#ddd7cc`), 1,3:1 sur `bg` : rien.
#[rstest]
#[case::white_on_navy(PRO, "#ffffff", "#00154e", Some("bg"))]
#[case::cyan_on_white(PRO, "#00bef4", "#ffffff", None)]
#[case::cyan_on_navy(PRO, "#00bef4", "#00154e", Some("accent"))]
#[case::pale_on_papier(PAPIER, "#b9c4d6", "#fffdf9", None)]
#[case::pale_on_navy(PRO, "#8fa3c2", "#00154e", Some("on-dark-muted"))]
#[case::pale_on_dark_papier(PAPIER, "#b9c4d6", "#2e2b26", Some("rule"))]
fn text_color_keeps_only_readable_tokens(
    #[case] theme: &str,
    #[case] text: &str,
    #[case] bg: &str,
    #[case] expected: Option<&str>,
) {
    let got = text_color(&Styled::explicit(hex(text)), hex(bg), &colors(theme));
    assert_that!(got).is_equal_to(Ok(expected.map(str::to_owned)));
}

/// Papier n'a ni `on-dark-body` ni `white` : `on-accent` (`#fffdf9`). Le vert
/// `#3f6b5e` (`accent`) sur `ink` (`#2e2b26`) ne fait que 2,3:1 ; plus clair
/// que ce fond sombre, il prend `on-accent` (15,0:1).
#[test]
fn text_color_of_a_light_text_on_a_dark_ground_takes_an_on_dark_token() {
    let got = text_color(&Styled::explicit(hex("#3f6b5e")), hex("#2e2b26"), &colors(PAPIER));
    assert_that!(got).is_equal_to(Ok(Some("on-accent".to_owned())));
}
```

Dans `crates/oku-slides-import/src/source/mod_tests.rs`, avant `item_text_serializes_with_kind_tag_and_role` :

```rust
#[test]
fn item_text_is_the_box_of_a_text_or_a_shape_only() {
    let shape = |text| Shape { geom: Geometry::Path { paths: Vec::new() }, fill: Fill::None, stroke: None, text };
    let mut item = text_item();
    assert_that!(item.text()).is_equal_to(Some(&TextBox::default()));
    item.kind = ItemKind::Shape(Box::new(shape(Some(TextBox::default()))));
    assert_that!(item.text()).is_equal_to(Some(&TextBox::default()));
    item.kind = ItemKind::Shape(Box::new(shape(None)));
    assert_that!(item.text()).is_none();
    item.kind = ItemKind::Group { items: Vec::new() };
    assert_that!(item.text()).is_none();
}
```

> Contrastes (WCAG) : `#00bef4` sur blanc 2,17 ; blanc sur `#00154e` 17,25 ; `#b9c4d6` sur le `bg` de Papier (`#fffdf9`) 1,73. `color_tokens_with_accents_ranks_…` : `#00154e` (5 000 pt²) devance le rouge (10 caractères × 10² = 1 000 pt²) et `#00bef4` (300 pt²) ; Papier n'a que deux clés d'accent (`accent`, `accent-soft`), le troisième reste sans jeton (ΔE 20,7 de `rule`) : perte. `#3f6b5e` (l'accent de Papier, ΔE 0) et `#808080` (neutre) ne comptent pas. Dans Pro, `#00154e` est `brand-ink` (ΔE 0) : pas d'accent déduit. `text_color` : `#8fa3c2` → `on-dark-muted` de Pro (ΔE 5,3), 5,9:1 sur `#00154e` ; `#b9c4d6` → `rule` de Papier (ΔE 7,7), 1,3:1 sur `bg` mais 10,1:1 sur `ink`.
>
> Les écarts de la note de `project_color_maps_to_a_theme_token_or_drops` sont calculés en OKLab (formules d'Ottosson) entre la couleur et chaque jeton effectif : par exemple `#2f5597` (`accent1` modifié par `lumMod 0.75`) est à 13,1 de `accent` (`#3f6b5e`) dans Papier, au-delà de 12 : abandonnée ; dans Pro elle est à 8,4 de `accent-text` (`#0072b0`). `#e7e6e6` dans Pro est à 1,02 de `rule` et de `surface` (tous deux `#e1e5ea`) : `rule` l'emporte par l'ordre des noms.

`crates/oku-slides-import/src/retrofit/mod.rs` (l'`allow` tombe à la tâche 9, quand `project` utilise tout) :

```rust
//! Projecteur retrofit : une [`SourceDeck`](crate::source::SourceDeck) projetée sur
//! un pack et un thème existants.
//!
//! Le style vient du thème cible : couleurs ramenées à ses jetons ([`ColorTokens`]),
//! texte riche réduit à ce que le cœur exprime.

#[allow(dead_code, reason = "branché par le projecteur, tâche 9")]
mod color;

pub use color::ColorTokens;
```

Dans `crates/oku-slides-import/src/lib.rs`, ajouter `pub mod retrofit;` après `pub mod report;`, et après `mod pptx;` :

```rust
#[cfg(test)]
mod test_support;
```

- [ ] **Step 2 : lancer, constater l'échec**

```bash
export CARGO_TARGET_DIR=$PWD/target
cargo nextest run -p oku-slides-import retrofit::color source::
```

Attendu : échec de compilation (`retrofit/color.rs` absent, puis `ColorTokens`, `oklab`, `project_color`… introuvables).

- [ ] **Step 3 : implémentation**

`crates/oku-slides-import/src/source/mod.rs`, après `pub struct Item { … }` :

```rust
impl Item {
    /// Le texte de l'item : celui d'une boîte de texte ou d'une forme.
    pub fn text(&self) -> Option<&TextBox> {
        match &self.kind {
            ItemKind::Text(tb) => Some(tb),
            ItemKind::Shape(s) => s.text.as_ref(),
            _ => None,
        }
    }

    /// L'item porte-t-il un texte non vide (autre que des espaces) ?
    pub fn has_text(&self) -> bool {
        self.text().is_some_and(|tb| tb.paragraphs.iter().flat_map(|p| &p.runs).any(|r| !r.text.trim().is_empty()))
    }
}
```

`crates/oku-slides-import/src/retrofit/color.rs` :

```rust
//! Couleurs source projetées sur les jetons `color` du thème cible.
//!
//! Le retrofit garde le style du thème cible : il n'émet jamais de `#rrggbb`,
//! seulement le nom d'une couleur existante du thème (le rendu Typst panique
//! sur un nom inconnu). Une couleur source devient, dans l'ordre :
//!
//! 1. rien, si elle est la valeur par défaut du lecteur (`Origin::Default`) :
//!    le thème cible décide ;
//! 2. le jeton de son emplacement de thème ([`slot_token`]), s'il existe et
//!    qu'aucun modificateur ne l'altère ;
//! 3. l'accent déduit du deck ([`ColorTokens::with_accents`]) ;
//! 4. `ink` ou `bg` pour un noir ou un blanc neutres ([`neutral_token`]) ;
//! 5. le jeton le plus proche en OKLab si `ΔE ≤ MAX_DELTA_E` ;
//! 6. sinon elle est abandonnée : perte [`LossKind::ColorDropped`].
//!
//! L'opacité est ignorée : les jetons sont opaques. La lisibilité d'un texte
//! se mesure par le contraste WCAG ([`contrast`]) : un jeton de texte sous
//! [`MIN_CONTRAST`] contre son fond n'est jamais émis ([`text_color`]).

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::report::LossKind;
use crate::source::{Color, Fill, Item, ItemKind, Origin, Rgba, SourceDeck};

/// Écart OKLab (×100) au-delà duquel une couleur n'a pas de jeton proche.
///
/// Une différence juste perceptible vaut environ 2 ; à 12, une couleur garde
/// un jeton de même famille (un bleu foncé `#1f4e79` → `ink` de Pro, 6,3)
/// mais n'en prend pas un d'une autre teinte (un rouge pur → `ink-muted` de
/// Papier, 27,9 ; un orange `#ed7d31` → `rule`, 23,1).
pub(crate) const MAX_DELTA_E: f64 = 12.0;

/// Chroma OKLab sous laquelle une couleur est neutre (un gris).
const NEUTRAL_CHROMA: f64 = 0.02;

/// Clarté OKLab sous laquelle un neutre est un noir d'encre.
const BLACK_LIGHTNESS: f64 = 0.25;

/// Clarté OKLab au-dessus de laquelle un neutre est un blanc de fond.
const WHITE_LIGHTNESS: f64 = 0.95;

/// Profondeur maximale d'une chaîne d'alias entre jetons de couleur.
const MAX_ALIAS_DEPTH: usize = 16;

/// Contraste WCAG minimal d'un texte émis contre son fond (grand texte, AA).
pub(crate) const MIN_CONTRAST: f64 = 3.0;

/// Chroma OKLab à partir de laquelle une couleur du deck compte comme accent.
const ACCENT_CHROMA: f64 = 0.08;

/// Jetons d'un texte clair sur fond sombre, dans l'ordre de préférence.
const ON_DARK: [&str; 3] = ["on-dark-body", "white", "on-accent"];

/// Une couleur dans l'espace OKLab (Björn Ottosson, 2020).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Oklab {
    /// Clarté, 0 (noir) à 1 (blanc).
    pub l: f64,
    /// Axe vert (négatif) – rouge (positif).
    pub a: f64,
    /// Axe bleu (négatif) – jaune (positif).
    pub b: f64,
}

impl Oklab {
    /// Chroma : distance à l'axe des gris.
    pub(crate) fn chroma(&self) -> f64 {
        self.a.hypot(self.b)
    }
}

/// Composante sRGB 8 bits → linéaire.
fn linear(c: u8) -> f64 {
    let c = f64::from(c) / 255.0;
    if c <= 0.040_45 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

/// `c` en OKLab (opacité ignorée).
pub(crate) fn oklab(c: Rgba) -> Oklab {
    let (r, g, b) = (linear(c.r), linear(c.g), linear(c.b));
    let l = (0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b).cbrt();
    let m = (0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b).cbrt();
    let s = (0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b).cbrt();
    Oklab {
        l: 0.210_454_255_3 * l + 0.793_617_785_0 * m - 0.004_072_046_8 * s,
        a: 1.977_998_495_1 * l - 2.428_592_205_0 * m + 0.450_593_709_9 * s,
        b: 0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766_0 * s,
    }
}

/// Écart entre deux couleurs : distance euclidienne OKLab, ×100.
pub(crate) fn delta_e(x: Oklab, y: Oklab) -> f64 {
    100.0 * ((x.l - y.l).powi(2) + (x.a - y.a).powi(2) + (x.b - y.b).powi(2)).sqrt()
}

/// Luminance relative WCAG de `c` (opacité ignorée).
fn luminance(c: Rgba) -> f64 {
    0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b)
}

/// Contraste WCAG entre deux couleurs : 1 (identiques) à 21 (noir sur blanc).
pub(crate) fn contrast(a: Rgba, b: Rgba) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// Un fond sombre : un texte blanc y contraste plus qu'un texte noir.
pub(crate) fn is_dark(c: Rgba) -> bool {
    contrast(c, Rgba::new(255, 255, 255)) > contrast(c, Rgba::new(0, 0, 0))
}

/// La couleur `name` du groupe `color`, alias résolus ; `None` pour un nom
/// inconnu, une boucle d'alias ou une autre valeur.
fn resolve(color: &Map<String, Value>, name: &str) -> Option<Rgba> {
    let mut v = color.get(name)?;
    for _ in 0..MAX_ALIAS_DEPTH {
        let s = v.as_str()?;
        if s.starts_with('#') {
            return parse_hex(s);
        }
        v = color.get(s)?;
    }
    None
}

/// Les couleurs du thème cible, par nom de jeton, alias résolus ; les fonds
/// d'une carte ; les accents déduits du deck source.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ColorTokens {
    colors: BTreeMap<String, Rgba>,
    /// Fond d'une carte (`component.card.bg`), puis d'une carte en relief (`highlight-bg`).
    cards: [Option<Rgba>; 2],
    /// Couleurs du deck ramenées à un accent, par ordre d'usage.
    accents: Vec<(Rgba, String)>,
}

impl ColorTokens {
    /// Depuis les jetons effectifs du thème (défauts fusionnés) : le groupe
    /// `color`, dont une valeur est `#rgb`, `#rrggbb`, `#rrggbbaa` ou le nom
    /// d'un autre jeton de couleur (un nom inconnu, une boucle d'alias ou une
    /// autre valeur : jeton écarté) ; les fonds `component.card.bg` et
    /// `component.card.highlight-bg`, des noms de couleur.
    pub fn from_tokens(tokens: &Map<String, Value>) -> Self {
        let empty = Map::new();
        let color = tokens.get("color").and_then(Value::as_object).unwrap_or(&empty);
        let card = tokens.get("component").and_then(|c| c.get("card"));
        let card_color =
            |key: &str| card.and_then(|c| c.get(key)).and_then(Value::as_str).and_then(|n| resolve(color, n));
        Self {
            colors: color.keys().filter_map(|k| resolve(color, k).map(|c| (k.clone(), c))).collect(),
            cards: [card_color("bg"), card_color("highlight-bg")],
            accents: Vec::new(),
        }
    }

    /// La couleur du jeton `name`.
    pub fn get(&self, name: &str) -> Option<Rgba> {
        self.colors.get(name).copied()
    }

    /// Vrai si le thème a un jeton `name`.
    pub fn contains(&self, name: &str) -> bool {
        self.colors.contains_key(name)
    }

    /// Le fond d'une carte, en relief (`highlight`) ou non ; repli : `ink`
    /// pour le relief, `bg` sinon (les défauts du rendu), puis blanc.
    pub(crate) fn card(&self, highlight: bool) -> Rgba {
        let (own, fallback) = if highlight { (self.cards[1], "ink") } else { (self.cards[0], "bg") };
        own.or_else(|| self.get(fallback)).unwrap_or(Rgba::new(255, 255, 255))
    }

    /// Le fond de page (`bg`) ; repli : blanc.
    pub(crate) fn page(&self) -> Rgba {
        self.get("bg").unwrap_or(Rgba::new(255, 255, 255))
    }

    /// Le premier jeton de texte clair ([`ON_DARK`]) lisible sur `bg`.
    pub(crate) fn on_dark(&self, bg: Rgba) -> Option<&'static str> {
        ON_DARK.into_iter().find(|t| self.get(t).is_some_and(|c| contrast(c, bg) >= MIN_CONTRAST))
    }

    /// Le jeton le plus proche de `c` et son écart ; à égalité, le premier par nom.
    pub(crate) fn nearest(&self, c: Rgba) -> Option<(&str, f64)> {
        let target = oklab(c);
        let mut best: Option<(&str, f64)> = None;
        for (name, token) in &self.colors {
            let d = delta_e(target, oklab(*token));
            if best.is_none_or(|(_, b)| d < b) {
                best = Some((name, d));
            }
        }
        best
    }

    /// Les accents déduits de `deck` : ses couleurs explicites saturées
    /// (chroma OKLab ≥ 0,08) sans jeton proche (`ΔE > MAX_DELTA_E`), rangées
    /// par usage décroissant ([`usage`] ; à égalité, par valeur hexadécimale),
    /// prennent dans l'ordre `accent` puis les autres clés `accent-*` du thème
    /// (par nom). Déterministe ; seules des clés existantes sont prises.
    pub fn with_accents(mut self, deck: &SourceDeck) -> Self {
        let mut used: BTreeMap<String, (Rgba, f64)> = BTreeMap::new();
        for slide in &deck.slides {
            for item in &slide.items {
                usage(item, &mut |c, weight| {
                    let c = Rgba { a: 255, ..c };
                    let e = used.entry(c.to_hex()).or_insert((c, 0.0));
                    e.1 += weight;
                });
            }
        }
        let mut ranked: Vec<(String, Rgba, f64)> = used
            .into_iter()
            .map(|(hex, (c, w))| (hex, c, w))
            .filter(|(_, c, _)| {
                oklab(*c).chroma() >= ACCENT_CHROMA && self.nearest(*c).is_none_or(|(_, d)| d > MAX_DELTA_E)
            })
            .collect();
        ranked.sort_by(|a, b| b.2.total_cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
        let keys = ["accent"]
            .into_iter()
            .filter(|k| self.contains(k))
            .map(str::to_owned)
            .chain(self.colors.keys().filter(|k| k.starts_with("accent-")).cloned());
        self.accents = ranked.into_iter().zip(keys).map(|((_, c, _), key)| (c, key)).collect();
        self
    }

    /// L'accent déduit pour la couleur `c`.
    fn accent(&self, c: Rgba) -> Option<&str> {
        let c = Rgba { a: 255, ..c };
        self.accents.iter().find(|(a, _)| Rgba { a: 255, ..*a } == c).map(|(_, k)| k.as_str())
    }
}

/// Appelle `f` pour chaque couleur explicite de `item` (et de ses enfants),
/// avec son poids d'usage en pt² : caractères × taille² pour un run de texte,
/// surface pour un fond uni, longueur × épaisseur pour un trait.
fn usage(item: &Item, f: &mut impl FnMut(Rgba, f64)) {
    let explicit = |c: &Color| matches!(c.origin, Origin::Explicit).then_some(c.value);
    if let Some(tb) = item.text() {
        for r in tb.paragraphs.iter().flat_map(|p| &p.runs) {
            if let Some(c) = explicit(&r.style.color) {
                // Le nombre de caractères d'un run est borné par `clip` (lot 1) : la conversion est exacte.
                f(c, r.text.chars().count() as f64 * r.style.size * r.style.size);
            }
        }
    }
    let frame = &item.frame;
    match &item.kind {
        ItemKind::Shape(s) => {
            if let Fill::Solid { color } = &s.fill
                && let Some(c) = explicit(color)
            {
                f(c, frame.w * frame.h);
            }
            if let Some(st) = &s.stroke
                && let Some(c) = explicit(&st.color)
            {
                f(c, 2.0 * (frame.w + frame.h) * st.width);
            }
        }
        ItemKind::Group { items, .. } => {
            for child in items {
                usage(child, f);
            }
        }
        _ => {}
    }
}

/// `#rgb`, `#rrggbb` ou `#rrggbbaa` ; `None` pour toute autre forme.
fn parse_hex(s: &str) -> Option<Rgba> {
    let h = s.strip_prefix('#')?;
    if !h.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
    let nibble = |i: usize| u8::from_str_radix(h.get(i..=i)?, 16).ok().map(|n| n * 17);
    match h.len() {
        3 => Some(Rgba::new(nibble(0)?, nibble(1)?, nibble(2)?)),
        6 => Some(Rgba::new(byte(0)?, byte(2)?, byte(4)?)),
        8 => Some(Rgba { r: byte(0)?, g: byte(2)?, b: byte(4)?, a: byte(6)? }),
        _ => None,
    }
}

/// Le jeton fixe d'un emplacement du thème source ; `None` pour les autres
/// (`accent2..6`, `folHlink` : couleur la plus proche).
pub(crate) fn slot_token(slot: &str) -> Option<&'static str> {
    match slot {
        "dk1" => Some("ink"),
        "lt1" => Some("bg"),
        "dk2" => Some("ink-muted"),
        "lt2" => Some("surface"),
        "accent1" | "hlink" => Some("accent"),
        _ => None,
    }
}

/// `ink` pour un noir neutre, `bg` pour un blanc neutre : un thème clair ne les
/// reprend jamais à l'identique (l'encre de Papier, `#2e2b26`, est à 29 d'un noir pur).
fn neutral_token(c: Oklab) -> Option<&'static str> {
    if c.chroma() >= NEUTRAL_CHROMA {
        return None;
    }
    if c.l < BLACK_LIGHTNESS {
        Some("ink")
    } else if c.l > WHITE_LIGHTNESS {
        Some("bg")
    } else {
        None
    }
}

/// La couleur source `c` en nom de jeton du thème cible.
///
/// `Ok(None)` : valeur par défaut du lecteur, le thème cible décide ;
/// `Ok(Some(nom))` : un jeton existant ; `Err(ColorDropped(hex))` : aucun jeton proche.
pub(crate) fn project_color(c: &Color, tokens: &ColorTokens) -> Result<Option<String>, LossKind> {
    if let Origin::Default = c.origin {
        return Ok(None);
    }
    if let Origin::Scheme { slot, mods, .. } = &c.origin
        && mods.is_empty()
        && let Some(token) = slot_token(slot).filter(|t| tokens.contains(t))
    {
        return Ok(Some(token.to_owned()));
    }
    if let Some(token) = tokens.accent(c.value) {
        return Ok(Some(token.to_owned()));
    }
    let lab = oklab(c.value);
    if let Some(token) = neutral_token(lab).filter(|t| tokens.contains(t)) {
        return Ok(Some(token.to_owned()));
    }
    match tokens.nearest(c.value) {
        Some((name, d)) if d <= MAX_DELTA_E => Ok(Some(name.to_owned())),
        _ => Err(LossKind::ColorDropped(c.value.to_hex())),
    }
}

/// Le jeton d'un texte de couleur `c` posé sur le fond `bg`, s'il y est lisible.
///
/// Le jeton projeté ([`project_color`]) est gardé s'il contraste d'au moins
/// [`MIN_CONTRAST`] avec `bg`. Sinon, un texte plus clair que son fond sombre
/// prend le premier jeton de texte clair lisible ([`ON_DARK`]) ; à défaut, rien
/// (la couleur du thème). Jamais un jeton sous [`MIN_CONTRAST`].
pub(crate) fn text_color(c: &Color, bg: Rgba, tokens: &ColorTokens) -> Result<Option<String>, LossKind> {
    let Some(token) = project_color(c, tokens)? else { return Ok(None) };
    if tokens.get(&token).is_some_and(|t| contrast(t, bg) >= MIN_CONTRAST) {
        return Ok(Some(token));
    }
    let lighter = luminance(c.value) > luminance(bg);
    Ok((lighter && is_dark(bg)).then(|| tokens.on_dark(bg)).flatten().map(str::to_owned))
}

#[cfg(test)]
#[path = "color_tests.rs"]
mod color_tests;
```

- [ ] **Step 4 : lancer, constater le succès**

```bash
cargo nextest run -p oku-slides-import retrofit::color source::
```

Attendu : 63 tests passent (360 pour la crate entière au brouillon).

- [ ] **Step 5 : vérifier, commit, sous-tâche**

```bash
cargo fmt --all && cargo clippy -p oku-slides-import --all-targets -- -D warnings && cargo nextest run -p oku-slides-import
git add crates/oku-slides-import/src/lib.rs crates/oku-slides-import/src/test_support.rs crates/oku-slides-import/src/retrofit crates/oku-slides-import/src/source/mod.rs crates/oku-slides-import/src/source/mod_tests.rs && git diff --cached --name-only
git commit -F <scratchpad>/msg.txt   # feat(import) : couleurs projetées sur les jetons du thème cible, OKLab, ΔE, accents et contraste (t-692)
oku cli task done --project slider --id t-799
```

---

### Task 4: Texte riche et plain

**Écarts d'interface :**
- `rich` prend le fond du texte : `rich(boxes: impl IntoIterator<Item = &TextBox>, tokens, bg: Rgba)`. Le fond (page, carte, carte en relief) décide quelles couleurs sont lisibles (révision 1, item 6) ; plusieurs boîtes mises bout à bout (les textes d'une carte, des textes fusionnés, tâche 7) passent dans les mêmes bornes.
- Couleur d'un run : émise, qu'elle soit la couleur dominante de la boîte ou non, quand son jeton est lisible sur `bg` (`text_color`, tâche 3 : contraste ≥ 3:1, ou jeton de texte clair sur fond sombre) et n'est pas `ink`, l'encre par défaut. Un texte blanc d'une couverture sombre (`lt1` → `bg`) n'est donc pas émis sur la page claire du thème cible (1:1) ; un gris pâle illisible non plus ; une couleur sans jeton proche est une perte.
- Capitales : `Caps::All` met le texte en majuscules (riche et plain), sans perte ; seules les petites capitales (`Caps::Small`) lèvent `RunStyle(Caps)`.
- `plain` rend aussi les pertes d'effets (`RunStyle`), comme `rich` ; gras, italique et couleur d'un plain viennent du thème (aucune perte).
- Ajout : `pub(crate) fn note<T: PartialEq>(losses: &mut Vec<T>, loss: T)` (ajout sans doublon, d'une perte ou d'une perte sur un item), la seule de la crate : les tâches 6 à 9 la réutilisent.
- `test_support.rs` gagne `bullet(level, runs)` (les autres constructeurs de texte viennent de la tâche 3).

**Files:**
- Create: `crates/oku-slides-import/src/retrofit/text.rs`, `crates/oku-slides-import/src/retrofit/text_tests.rs`
- Modify: `crates/oku-slides-import/src/retrofit/mod.rs` (`mod text;`), `crates/oku-slides-import/src/test_support.rs` (constructeurs de texte)

**Interfaces:**
- Consumes : `ColorTokens`, `text_color` (tâche 3) ; `LossKind`, `RunEffect` (tâche 2) ; `crate::source::{TextBox, Paragraph, Run, RunStyle, Bullet, Caps, Rgba}` ; `oku_slides_core::{Paragraph, Run, ListKind, PLAIN_MAX, RICH_PARAGRAPHS_MAX, RICH_BYTES_MAX}`.
- Produces :

```rust
// crate::retrofit::text
pub(crate) struct TextOut { pub paragraphs: Vec<oku_slides_core::Paragraph>, pub losses: Vec<LossKind> }
pub(crate) fn note<T: PartialEq>(losses: &mut Vec<T>, loss: T);
pub(crate) fn rich<'t>(boxes: impl IntoIterator<Item = &'t TextBox>, tokens: &ColorTokens, bg: Rgba) -> TextOut;
pub(crate) fn plain(tb: &source::TextBox) -> (String, Vec<LossKind>);
```

- Règles (`rich`) :
  - runs vides ignorés ; paragraphe sans texte omis ; runs adjacents de même forme (gras, italique, couleur, lien) fusionnés ; un run `"\n"` (`a:br`) reste un saut de ligne dans le paragraphe ;
  - gras, italique gardés ; lien gardé s'il commence par `http://`, `https://` ou `mailto:` (casse ignorée) ; taille, police, espacement : thème cible, sans perte ;
  - puce `Char` → `ListKind::Bullet`, `AutoNum` → `ListKind::Number` ; un paragraphe de niveau > 0 garde sa liste et lève `ListLevels` (une fois) ;
  - souligné, barré, exposant/indice (`baseline ≠ 0`), petites capitales, surlignage → `RunStyle(effet)`, une fois par effet ; capitales → texte en majuscules ;
  - couleur : voir Écarts ; une couleur sans jeton proche → `ColorDropped(hex)`, une fois par couleur ;
  - les paragraphes des boîtes se suivent dans l'ordre donné ;
  - bornes : au plus `RICH_PARAGRAPHS_MAX` (64) paragraphes et `RICH_BYTES_MAX` (16 384) octets de JSON du tableau (mesure de `text_fits`) ; le paragraphe qui déborde est coupé au plus long début qui tient (dichotomie sur les frontières de caractère du run qui déborde), les suivants abandonnés, perte `TextTruncated`.
- Règles (`plain`) : texte des runs (capitales appliquées), paragraphes et blancs (sauts de ligne compris) ramenés à une espace, bords coupés ; au plus `PLAIN_MAX` (4 096) octets coupés à une frontière de caractère → `TextTruncated` ; effets perdus → `RunStyle`.

- [ ] **Step 1 : tests qui échouent**

Dans `crates/oku-slides-import/src/test_support.rs`, après `para` :

```rust
/// Paragraphe à puce `•` de niveau `level`.
pub(crate) fn bullet(level: u8, runs: Vec<Run>) -> Paragraph {
    let mut p = para(runs);
    p.level = level;
    p.style.bullet = Bullet::Char { char: "•".into(), color: None, size_pct: 1.0, font: None };
    p
}

```

`crates/oku-slides-import/src/retrofit/text_tests.rs` :

```rust
use oku_slides_core::{ParamType, ParamValue, text_fits};
use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::source::{Origin, Styled};
use crate::test_support::{PAPIER, bullet, colors, para, run, run_with, text_box};

fn papier() -> ColorTokens {
    colors(PAPIER)
}

/// Le fond de page de Papier, `#fffdf9`.
fn page() -> Rgba {
    papier().page()
}

fn texts(out: &TextOut) -> Vec<Vec<&str>> {
    out.paragraphs.iter().map(|p| p.runs.iter().map(|r| r.t.as_str()).collect()).collect()
}

#[test]
fn rich_keeps_bold_and_italic_runs() {
    let tb =
        text_box(vec![para(vec![run_with("Bonjour ", |s| s.bold = true), run_with("monde", |s| s.italic = true)])]);
    let out = rich([&tb], &papier(), page());
    let runs = &out.paragraphs[0].runs;
    assert_that!((runs[0].bold, runs[0].italic, runs[1].bold, runs[1].italic)).is_equal_to((true, false, false, true));
    assert_that!(out.losses).is_empty();
}

#[test]
fn rich_merges_adjacent_runs_of_the_same_format() {
    let tb = text_box(vec![para(vec![run("Un "), run("deux"), run_with(" trois", |s| s.size = 40.0)])]);
    assert_that!(texts(&rich([&tb], &papier(), page()))).is_equal_to(vec![vec!["Un deux trois"]]);
}

#[test]
fn rich_omits_paragraphs_without_text() {
    let tb = text_box(vec![para(vec![run("A")]), para(vec![]), para(vec![run("")]), para(vec![run("B")])]);
    assert_that!(texts(&rich([&tb], &papier(), page()))).is_equal_to(vec![vec!["A"], vec!["B"]]);
}

#[test]
fn rich_keeps_line_breaks_inside_a_paragraph() {
    let tb = text_box(vec![para(vec![run("Ligne 1"), run("\n"), run("Ligne 2")])]);
    assert_that!(texts(&rich([&tb], &papier(), page()))).is_equal_to(vec![vec!["Ligne 1\nLigne 2"]]);
}

#[rstest]
#[case(Bullet::None, None)]
#[case(Bullet::Char { char: "–".into(), color: None, size_pct: 1.0, font: None }, Some(ListKind::Bullet))]
#[case(Bullet::AutoNum { scheme: "arabicPeriod".into(), start: 1 }, Some(ListKind::Number))]
fn rich_maps_bullets_to_lists(#[case] b: Bullet, #[case] expected: Option<ListKind>) {
    let mut p = para(vec![run("Point")]);
    p.style.bullet = b;
    assert_that!(rich([&text_box(vec![p])], &papier(), page()).paragraphs[0].list).is_equal_to(expected);
}

#[test]
fn rich_flattens_list_levels_and_reports_it_once() {
    let tb = text_box(vec![bullet(0, vec![run("Un")]), bullet(1, vec![run("Un.a")]), bullet(2, vec![run("Un.a.i")])]);
    let out = rich([&tb], &papier(), page());
    assert_that!(out.paragraphs.iter().all(|p| p.list == Some(ListKind::Bullet))).is_true();
    assert_that!(out.losses).is_equal_to(vec![LossKind::ListLevels]);
}

#[rstest]
#[case::underline(run_with("x", |s| s.underline = true), RunEffect::Underline)]
#[case::strike(run_with("x", |s| s.strike = true), RunEffect::Strike)]
#[case::superscript(run_with("x", |s| s.baseline = 30), RunEffect::Baseline)]
#[case::subscript(run_with("x", |s| s.baseline = -25), RunEffect::Baseline)]
#[case::small_caps(run_with("x", |s| s.caps = Caps::Small), RunEffect::Caps)]
#[case::highlight(run_with("x", |s| s.highlight = Some(Styled::explicit(Rgba::new(255, 255, 0)))), RunEffect::Highlight)]
fn rich_reports_each_lost_effect_and_keeps_the_text(#[case] r: source::Run, #[case] effect: RunEffect) {
    let tb = text_box(vec![para(vec![r.clone(), r])]);
    let out = rich([&tb], &papier(), page());
    assert_that!(texts(&out)).is_equal_to(vec![vec!["xx"]]);
    assert_that!(out.losses).is_equal_to(vec![LossKind::RunStyle(effect)]);
}

#[rstest]
#[case("https://oku.test/a", Some("https://oku.test/a"))]
#[case("MAILTO:a@b.c", Some("MAILTO:a@b.c"))]
#[case("javascript:alert(1)", None)]
#[case("file:///etc/passwd", None)]
fn rich_keeps_safe_links_only(#[case] link: &str, #[case] expected: Option<&str>) {
    let tb = text_box(vec![para(vec![run_with("lien", |s| s.link = Some(link.to_owned()))])]);
    assert_that!(rich([&tb], &papier(), page()).paragraphs[0].runs[0].link.as_deref()).is_equal_to(expected);
}

fn red() -> source::Color {
    Styled::explicit(Rgba::new(255, 0, 0))
}

fn accent1() -> source::Color {
    Styled::mapped(Rgba::new(0x44, 0x72, 0xc4), "accent1", "accent1", Vec::new())
}

/// Toute la boîte en `accent1` : `accent` (`#3f6b5e`) est lisible sur `bg` (5,9:1) et émis.
#[test]
fn rich_of_a_box_in_one_color_emits_its_readable_token() {
    let tb = text_box(vec![para(vec![run_with("Tout en accent", |s| s.color = accent1())])]);
    let out = rich([&tb], &papier(), page());
    assert_that!(out.paragraphs[0].runs[0].color.as_deref()).is_equal_to(Some("accent"));
    assert_that!(out.losses).is_empty();
}

/// Le gris bleuté `#b9c4d6` d'alto se projette sur `rule` (`#ddd7cc`) : 1,3:1 sur
/// `bg`, illisible, rien n'est émis ; sur le fond sombre `ink` d'une carte en
/// relief (`#2e2b26`), 10,1:1 : `rule` est émis.
#[test]
fn rich_emits_a_pale_color_only_on_a_dark_ground() {
    let tb = text_box(vec![para(vec![run_with("Pâle", |s| s.color = Styled::explicit(Rgba::new(0xb9, 0xc4, 0xd6)))])]);
    let dark = papier().card(true);
    let colors: Vec<Option<String>> =
        [page(), dark].iter().map(|bg| rich([&tb], &papier(), *bg).paragraphs[0].runs[0].color.clone()).collect();
    assert_that!(colors).is_equal_to(vec![None, Some("rule".to_owned())]);
}

#[test]
fn rich_upper_cases_all_caps_runs_without_a_loss() {
    let tb = text_box(vec![para(vec![run_with("Notre réponse", |s| s.caps = Caps::All)])]);
    let out = rich([&tb], &papier(), page());
    assert_that!(texts(&out)).is_equal_to(vec![vec!["NOTRE RÉPONSE"]]);
    assert_that!(out.losses).is_empty();
}

#[test]
fn rich_emits_the_token_of_a_run_that_stands_out() {
    let tb = text_box(vec![para(vec![run("Un mot "), run_with("accentué", |s| s.color = accent1()), run(" ici")])]);
    let runs = &rich([&tb], &papier(), page()).paragraphs[0].runs;
    assert_that!(runs.iter().map(|r| r.color.as_deref()).collect::<Vec<_>>()).is_equal_to(vec![
        None,
        Some("accent"),
        None,
    ]);
}

#[test]
fn rich_drops_a_standing_out_color_without_close_token_and_reports_it() {
    let tb = text_box(vec![para(vec![run("Un mot "), run_with("rouge", |s| s.color = red()), run(" ici")])]);
    let out = rich([&tb], &papier(), page());
    assert_that!(texts(&out)).is_equal_to(vec![vec!["Un mot rouge ici"]]);
    assert_that!(out.losses).is_equal_to(vec![LossKind::ColorDropped("#ff0000".into())]);
}

/// Le corps est en `accent1` ; le mot en `dk1` se projette sur `ink`, la couleur par défaut : rien n'est émis.
#[test]
fn rich_never_emits_the_ink_token() {
    let tb = text_box(vec![para(vec![run_with("Corps accentué ", |s| s.color = accent1()), run("noir")])]);
    let runs = &rich([&tb], &papier(), page()).paragraphs[0].runs;
    assert_that!(runs.iter().map(|r| r.color.as_deref()).collect::<Vec<_>>()).is_equal_to(vec![Some("accent"), None]);
}

#[test]
fn rich_of_a_default_color_emits_nothing() {
    let default = Styled { value: Rgba::new(1, 2, 3), origin: Origin::Default };
    let tb = text_box(vec![para(vec![run("Un "), run_with("défaut", |s| s.color = default)])]);
    let out = rich([&tb], &papier(), page());
    assert_that!(out.paragraphs[0].runs.iter().all(|r| r.color.is_none())).is_true();
    assert_that!(out.losses).is_empty();
}

#[test]
fn rich_keeps_at_most_the_core_paragraph_count() {
    let tb = text_box((0..70).map(|i| para(vec![run(&format!("P{i}"))])).collect());
    let out = rich([&tb], &papier(), page());
    assert_that!(out.paragraphs.len()).is_equal_to(RICH_PARAGRAPHS_MAX);
    assert_that!(out.losses).is_equal_to(vec![LossKind::TextTruncated]);
}

/// Un paragraphe d'un seul run s'écrit `{"runs":[{"t":"…"}]}` (19 octets plus le texte),
/// dans `[…]` (2 octets) : il reste `16384 − 21 = 16363` octets de texte.
#[test]
fn rich_cuts_an_oversized_paragraph_to_the_byte_bound() {
    let tb = text_box(vec![para(vec![run(&"a".repeat(20_000))]), para(vec![run("suite")])]);
    let out = rich([&tb], &papier(), page());
    assert_that!(texts(&out)).has_length(1);
    assert_that!(out.paragraphs[0].runs[0].t.len()).is_equal_to(16_363);
    assert_that!(text_fits(&ParamType::Rich, &ParamValue::Rich(out.paragraphs.clone()))).is_true();
    assert_that!(out.losses).is_equal_to(vec![LossKind::TextTruncated]);
}

/// `é` vaut 2 octets, non échappés en JSON : `⌊(16384 − 21) / 2⌋ = 8181` caractères.
#[test]
fn rich_cuts_multibyte_text_on_a_character_boundary() {
    let tb = text_box(vec![para(vec![run(&"é".repeat(10_000))])]);
    let out = rich([&tb], &papier(), page());
    assert_that!(out.paragraphs[0].runs[0].t.chars().count()).is_equal_to(8181);
    assert_that!(out.paragraphs[0].runs[0].t.chars().all(|c| c == 'é')).is_true();
}

/// Chaque paragraphe pèse `19 + 6000 = 6019` octets : après deux (`2 + 6019 + 1 + 6019 = 12041`),
/// virgule comprise il reste `16384 − 12042 = 4342` octets, soit `4342 − 19 = 4323` octets de texte.
#[test]
fn rich_stops_at_the_paragraph_that_overflows_the_byte_bound() {
    let big = || para(vec![run(&"b".repeat(6_000))]);
    let out = rich([&text_box(vec![big(), big(), big()])], &papier(), page());
    assert_that!(out.paragraphs).has_length(3);
    assert_that!(out.paragraphs[2].runs[0].t.len()).is_equal_to(4_323);
    assert_that!(text_fits(&ParamType::Rich, &ParamValue::Rich(out.paragraphs.clone()))).is_true();
    assert_that!(out.losses).is_equal_to(vec![LossKind::TextTruncated]);
}

#[test]
fn rich_within_bounds_reports_no_truncation() {
    let tb = text_box((0..RICH_PARAGRAPHS_MAX).map(|_| para(vec![run("court")])).collect());
    assert_that!(rich([&tb], &papier(), page()).losses).is_empty();
}

#[test]
fn plain_joins_paragraphs_and_line_breaks_with_single_spaces() {
    let tb = text_box(vec![para(vec![run("  Rapport"), run("\n"), run("annuel ")]), para(vec![run("2026")])]);
    assert_that!(plain(&tb)).is_equal_to(("Rapport annuel 2026".to_owned(), Vec::new()));
}

#[test]
fn plain_of_an_empty_box_is_empty() {
    assert_that!(plain(&text_box(vec![para(vec![])]))).is_equal_to((String::new(), Vec::new()));
}

#[test]
fn plain_upper_cases_all_caps_runs() {
    let tb = text_box(vec![para(vec![run_with("the shift", |s| s.caps = Caps::All)])]);
    assert_that!(plain(&tb)).is_equal_to(("THE SHIFT".to_owned(), Vec::new()));
}

#[test]
fn plain_reports_lost_effects_once() {
    let tb = text_box(vec![para(vec![run_with("A", |s| s.underline = true), run_with("B", |s| s.underline = true)])]);
    assert_that!(plain(&tb).1).is_equal_to(vec![LossKind::RunStyle(RunEffect::Underline)]);
}

/// 3 000 `é` font 6 000 octets ; `PLAIN_MAX` = 4 096 octets = 2 048 `é`.
#[test]
fn plain_cuts_to_the_byte_bound_on_a_character_boundary() {
    let (s, losses) = plain(&text_box(vec![para(vec![run(&"é".repeat(3_000))])]));
    assert_that!(s.len()).is_equal_to(PLAIN_MAX);
    assert_that!(s.chars().count()).is_equal_to(2048);
    assert_that!(losses).is_equal_to(vec![LossKind::TextTruncated]);
}
```

> `#b9c4d6` (gris bleuté d'alto) se projette sur `rule` de Papier (`#ddd7cc`, ΔE 7,7) : 1,3:1 sur `bg` (`#fffdf9`), illisible ; 10,1:1 sur `ink` (`#2e2b26`), le fond d'une carte en relief de Papier. `accent` (`#3f6b5e`) fait 5,9:1 sur `bg`.

`crates/oku-slides-import/src/retrofit/mod.rs` complet :

```rust
//! Projecteur retrofit : une [`SourceDeck`](crate::source::SourceDeck) projetée sur
//! un pack et un thème existants.
//!
//! Le style vient du thème cible : couleurs ramenées à ses jetons ([`ColorTokens`]),
//! texte riche réduit à ce que le cœur exprime.

#[allow(dead_code, reason = "branché par le projecteur, tâche 9")]
mod color;
#[allow(dead_code, reason = "branché par le projecteur, tâche 7")]
mod text;

pub use color::ColorTokens;
```

- [ ] **Step 2 : lancer, constater l'échec**

```bash
export CARGO_TARGET_DIR=$PWD/target
cargo nextest run -p oku-slides-import retrofit::text
```

Attendu : échec de compilation (`retrofit/text.rs` absent, puis `rich`, `plain`, `TextOut` introuvables).

- [ ] **Step 3 : implémentation**

`crates/oku-slides-import/src/retrofit/text.rs` :

```rust
//! Texte source → texte riche ou plain du cœur, dans ses bornes.
//!
//! Le style vient du thème cible : seuls passent le gras, l'italique, les
//! liens sûrs, les listes (un niveau), les capitales (le texte est mis en
//! majuscules) et la couleur des runs, quand son jeton est lisible sur le fond
//! du texte ([`text_color`]). Le reste est perdu et dit au rapport.

use oku_slides_core::{ListKind, PLAIN_MAX, Paragraph, RICH_BYTES_MAX, RICH_PARAGRAPHS_MAX, Run};

use super::color::{ColorTokens, text_color};
use crate::report::{LossKind, RunEffect};
use crate::source::{self, Bullet, Caps, Rgba, TextBox};

/// Jeton de l'encre du texte : la couleur par défaut, jamais émise.
const INK: &str = "ink";

/// Texte riche projeté et ses pertes (chacune une fois).
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct TextOut {
    /// Paragraphes, dans les bornes `RICH_PARAGRAPHS_MAX` et `RICH_BYTES_MAX`.
    pub paragraphs: Vec<Paragraph>,
    /// Pertes.
    pub losses: Vec<LossKind>,
}

/// Ajoute `loss` à `losses` s'il n'y est pas (une perte, ou une perte sur un item).
pub(crate) fn note<T: PartialEq>(losses: &mut Vec<T>, loss: T) {
    if !losses.contains(&loss) {
        losses.push(loss);
    }
}

/// Les effets d'un run que le texte du cœur n'exprime pas.
fn effects(style: &source::RunStyle) -> impl Iterator<Item = RunEffect> {
    [
        (style.underline, RunEffect::Underline),
        (style.strike, RunEffect::Strike),
        (style.baseline != 0, RunEffect::Baseline),
        (style.caps == Caps::Small, RunEffect::Caps),
        (style.highlight.is_some(), RunEffect::Highlight),
    ]
    .into_iter()
    .filter_map(|(lost, e)| lost.then_some(e))
}

/// Le texte d'un run tel qu'il s'affiche : en majuscules pour `Caps::All`.
fn shown(r: &source::Run) -> String {
    match r.style.caps {
        Caps::All => r.text.to_uppercase(),
        Caps::None | Caps::Small => r.text.clone(),
    }
}

/// Même forme : les runs se fusionnent.
fn same_format(a: &Run, b: &Run) -> bool {
    a.bold == b.bold && a.italic == b.italic && a.color == b.color && a.link == b.link
}

/// Lien gardé : `http`, `https` ou `mailto`, quelle que soit la casse.
fn safe_link(link: Option<&str>) -> Option<String> {
    let link = link?;
    let lower = link.to_ascii_lowercase();
    ["http://", "https://", "mailto:"].iter().any(|s| lower.starts_with(s)).then(|| link.to_owned())
}

/// Liste du cœur pour une puce source.
fn list_kind(bullet: &Bullet) -> Option<ListKind> {
    match bullet {
        Bullet::None => None,
        Bullet::Char { .. } => Some(ListKind::Bullet),
        Bullet::AutoNum { .. } => Some(ListKind::Number),
    }
}

/// Taille de `p` en JSON, comme la mesure `text_fits`.
fn json_len(p: &Paragraph) -> usize {
    serde_json::to_vec(p).map_or(usize::MAX, |b| b.len())
}

/// Le plus long début de `p` dont la forme JSON tient en `budget` octets ;
/// `None` si même un caractère ne tient pas.
fn cut(p: &Paragraph, budget: usize) -> Option<Paragraph> {
    let mut out = Paragraph { runs: Vec::new(), list: p.list };
    for run in &p.runs {
        out.runs.push(run.clone());
        if json_len(&out) <= budget {
            continue;
        }
        // Coupe du dernier run à une frontière de caractère, par dichotomie.
        let bounds: Vec<usize> = run.t.char_indices().map(|(i, _)| i).skip(1).chain([run.t.len()]).collect();
        let (mut lo, mut hi) = (0usize, bounds.len());
        while lo < hi {
            let mid = (lo + hi).div_ceil(2);
            if let Some(last) = out.runs.last_mut() {
                last.t = run.t[..bounds[mid - 1]].to_owned();
            }
            if json_len(&out) <= budget { lo = mid } else { hi = mid - 1 }
        }
        if lo == 0 {
            out.runs.pop();
        } else if let Some(last) = out.runs.last_mut() {
            last.t = run.t[..bounds[lo - 1]].to_owned();
        }
        break;
    }
    (!out.runs.is_empty() && json_len(&out) <= budget).then_some(out)
}

/// `paragraphs` bornés à `RICH_PARAGRAPHS_MAX` paragraphes et `RICH_BYTES_MAX`
/// octets de JSON (crochets et virgules compris) ; vrai si quelque chose est coupé.
fn bound(paragraphs: Vec<Paragraph>) -> (Vec<Paragraph>, bool) {
    let mut out = Vec::new();
    let mut used = 2; // `[` et `]`
    let total = paragraphs.len();
    for p in paragraphs {
        if out.len() == RICH_PARAGRAPHS_MAX {
            return (out, true);
        }
        let comma = usize::from(!out.is_empty());
        let budget = RICH_BYTES_MAX.saturating_sub(used + comma);
        let len = json_len(&p);
        if len <= budget {
            used += comma + len;
            out.push(p);
            continue;
        }
        if let Some(p) = cut(&p, budget) {
            out.push(p);
        }
        return (out, true);
    }
    let cut_short = out.len() < total;
    (out, cut_short)
}

/// Le texte riche des boîtes `boxes` mises bout à bout (paragraphes dans
/// l'ordre), projeté sur les couleurs `tokens`, posé sur le fond `bg`.
///
/// Un paragraphe sans texte est omis ; les runs adjacents de même forme sont
/// fusionnés ; un niveau de liste > 0 est aplati (perte `ListLevels`). La
/// couleur d'un run est émise si son jeton est lisible sur `bg`
/// ([`text_color`]) et n'est pas `ink`, l'encre par défaut.
pub(crate) fn rich<'t>(boxes: impl IntoIterator<Item = &'t TextBox>, tokens: &ColorTokens, bg: Rgba) -> TextOut {
    let mut losses = Vec::new();
    let mut paragraphs = Vec::new();
    for sp in boxes.into_iter().flat_map(|tb| &tb.paragraphs) {
        let mut out: Vec<Run> = Vec::new();
        for r in sp.runs.iter().filter(|r| !r.text.is_empty()) {
            for e in effects(&r.style) {
                note(&mut losses, LossKind::RunStyle(e));
            }
            let color = match text_color(&r.style.color, bg, tokens) {
                Ok(token) => token.filter(|t| t != INK),
                Err(loss) => {
                    note(&mut losses, loss);
                    None
                }
            };
            let run = Run {
                t: shown(r),
                bold: r.style.bold,
                italic: r.style.italic,
                color,
                link: safe_link(r.style.link.as_deref()),
                ..Run::default()
            };
            match out.last_mut() {
                Some(last) if same_format(last, &run) => last.t.push_str(&run.t),
                _ => out.push(run),
            }
        }
        if out.is_empty() {
            continue;
        }
        if sp.level > 0 {
            note(&mut losses, LossKind::ListLevels);
        }
        paragraphs.push(Paragraph { runs: out, list: list_kind(&sp.style.bullet) });
    }
    let (paragraphs, truncated) = bound(paragraphs);
    if truncated {
        note(&mut losses, LossKind::TextTruncated);
    }
    TextOut { paragraphs, losses }
}

/// Le texte plain de `tb` (titre, sous-titre) : paragraphes et sauts de ligne
/// ramenés à une espace, espaces fusionnées, au plus `PLAIN_MAX` octets coupés
/// à une frontière de caractère, capitales appliquées ; pertes : effets de texte, coupe.
pub(crate) fn plain(tb: &TextBox) -> (String, Vec<LossKind>) {
    let mut losses = Vec::new();
    let mut words = Vec::new();
    for sp in &tb.paragraphs {
        let mut text = String::new();
        for r in sp.runs.iter().filter(|r| !r.text.is_empty()) {
            for e in effects(&r.style) {
                note(&mut losses, LossKind::RunStyle(e));
            }
            text.push_str(&shown(r));
        }
        words.extend(text.split_whitespace().map(str::to_owned));
    }
    let mut s = words.join(" ");
    if s.len() > PLAIN_MAX {
        let end = (0..=PLAIN_MAX).rev().find(|i| s.is_char_boundary(*i)).unwrap_or(0);
        s.truncate(end);
        s.truncate(s.trim_end().len());
        note(&mut losses, LossKind::TextTruncated);
    }
    (s, losses)
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod text_tests;
```

- [ ] **Step 4 : lancer, constater le succès**

```bash
cargo nextest run -p oku-slides-import retrofit::text
```

Attendu : 35 tests passent (395 pour la crate entière au brouillon).

- [ ] **Step 5 : vérifier, commit, sous-tâche**

```bash
cargo fmt --all && cargo clippy -p oku-slides-import --all-targets -- -D warnings && cargo nextest run -p oku-slides-import
git add crates/oku-slides-import/src/retrofit/mod.rs crates/oku-slides-import/src/retrofit/text.rs crates/oku-slides-import/src/retrofit/text_tests.rs crates/oku-slides-import/src/test_support.rs && git diff --cached --name-only
git commit -F <scratchpad>/msg.txt   # feat(import) : texte source en texte riche et plain du cœur, dans ses bornes (t-692)
oku cli task done --project slider --id t-800
```

---

### Task 5: Zone de corps et grille

**Écarts d'interface :**
- Ajout : `pub(crate) fn grid_gap(grid: &Grid, page: &PageTokens) -> f64` : la gouttière d'une grille en pt (un nombre tel quel, sinon `page.gap`). `cell_rect` et `snap` prennent la gouttière résolue, comme prévu ; la tâche 9 l'obtient par `grid_gap`.
- `PageTokens` dérive `Debug, Clone, Copy, PartialEq` ; `Zone` dérive `Debug, Clone, Copy, PartialEq`.
- `to_zone` prend un ajustement : `to_zone(item: &source::Frame, fit: &Fit, zone: &Zone) -> Frame` (révision 1, item 8 : ajuster, pas tronquer). `Fit { scale, dx, dy }` passe des pt source aux pt relatifs à la zone ; `Fit::new(frames, scale, zone)` ajuste la boîte englobante des items placés d'une slide dans la zone (échelle uniforme, centrée). Le ramené dans la zone reste en filet de sûreté : un cadre d'au moins 1 pt de côté (le cœur refuse `w` ou `h` nuls). Les cadres sont arrondis au centième de pt.
- Ajout : `pub(crate) fn track_size(zone, grid, gap) -> (f64, f64)` (largeur d'une colonne, hauteur d'une ligne) : un filet est une forme plus fine qu'un quart de piste (tâche 7).
- `cell_rect` n'est compilé que pour les tests (`#[cfg(test)]`) : le projecteur n'émet que des cellules accrochées par `snap` ; ce calcul en est l'oracle.

Le cœur place un élément libre relativement à la **zone de corps**, que seul le rendu connaît (marqueur `<oku-slides-zone>`). Le projecteur est pur : il l'approche par les jetons du thème cible. Pour une slide `content`, la grille Typst est `rows: (auto, 1fr)` avec la gouttière `component.title.gap` sous la ligne de titre ; la zone vaut donc la page moins ses marges, moins la ligne de titre et sa gouttière si la slide a un titre. La hauteur de la ligne de titre est approchée par une hauteur d'em de `component.title.size` (un peu plus que la ligne réelle, pour ne pas déborder), plus le filet `component.title.rule` (sa hauteur et l'espace `sm`) s'il existe. Papier : `60 + 40 = 100` ; Pro : `56 + (5 + 16) + 44 = 121`. La géométrie de grille est celle de `grid-frame` (`typst/oku-slides/src/zone.typ`) et de `ui/src/grid.ts` ; `snap` porte `snapRect` (même règle : départ de piste le plus proche, fin de piste la plus proche, une case au moins, égalité vers la première piste). `ui/src/grid.ts` n'a pas de tests de `snapRect` : ceux d'ici en tiennent lieu.

**Files:**
- Create: `crates/oku-slides-import/src/retrofit/zone.rs`, `crates/oku-slides-import/src/retrofit/zone_tests.rs`
- Modify: `crates/oku-slides-import/src/retrofit/mod.rs` (`mod zone;`, ré-export `PageTokens`)

**Interfaces:**
- Consumes : `oku_slides_core::{Cell, Frame, Grid, TokenValue}` ; `crate::source::Frame` (lot 1) ; `crate::test_support::{tokens, PAPIER, PRO}` (tâche 3).
- Produces :

```rust
// crate::retrofit (ré-export de retrofit::zone)
pub struct PageTokens {
    pub width: f64, pub height: f64, pub margin_x: f64, pub margin_top: f64, pub margin_bottom: f64,
    pub title_band: f64, pub gap: f64,
}                                                    // Debug, Clone, Copy, PartialEq ; Default = default-tokens.json
impl PageTokens { pub fn from_tokens(tokens: &serde_json::Map<String, Value>) -> Self; }
// crate::retrofit::zone
pub(crate) struct Zone { pub x: f64, pub y: f64, pub w: f64, pub h: f64 }   // pt de page
pub(crate) fn body_zone(page: &PageTokens, titled: bool) -> Zone;
pub(crate) fn grid_gap(grid: &Grid, page: &PageTokens) -> f64;
pub(crate) fn track_size(zone: &Zone, grid: &Grid, gap: f64) -> (f64, f64);
#[cfg(test)] pub(crate) fn cell_rect(zone: &Zone, grid: &Grid, gap: f64, cell: &Cell) -> Frame;   // relatif à la zone
pub(crate) fn snap(zone: &Zone, grid: &Grid, gap: f64, r: Frame) -> Cell;               // r relatif à la zone
pub(crate) struct Fit { pub scale: f64, pub dx: f64, pub dy: f64 }                     // Debug, Clone, Copy, PartialEq
impl Fit { pub(crate) fn new<'f>(frames: impl IntoIterator<Item = &'f source::Frame>, scale: f64, zone: &Zone) -> Self; }
pub(crate) fn to_zone(item: &source::Frame, fit: &Fit, zone: &Zone) -> Frame;           // source → relatif à la zone
```

- Règles :
  - `PageTokens::from_tokens` lit `page.width`, `page.height`, `page.margin-x`, `page.margin-top`, `page.margin-bottom`, `space.lg` (gouttière), et `component.title.{size, gap, rule}` (une valeur nommée se résout dans `size` ou `space`) ; un jeton absent garde la valeur par défaut. `title_band = taille du titre + (hauteur du filet + space.sm si filet) + gouttière du titre`.
  - `body_zone(page, titled)` = `(margin_x, margin_top + bande, width − 2·margin_x, height − margin_top − margin_bottom − bande)`, bande = `title_band` si `titled`, sinon 0. Papier sans titre : `(120, 150, 1680, 790)`.
  - `cell_rect` : `cw = (w − (colonnes − 1)·gap) / colonnes`, `rh = (h − (lignes − 1)·gap) / lignes`, `x = col·(cw + gap)`, `y = row·(rh + gap)`, `w = cols·cw + (cols − 1)·gap`, `h = rows·rh + (rows − 1)·gap`.
  - `snap` : colonne et ligne = départ de piste le plus proche de `r.x`, `r.y` ; dernière colonne et ligne = fin de piste la plus proche de `r.x + r.w`, `r.y + r.h`, jamais avant la première ; à égalité, la première piste. Le résultat tient toujours dans la grille (`Cell::fits`).
  - `Fit::new` : boîte englobante `(x0, y0)–(x1, y1)` des cadres ; échelle `s = scale` (= `1920 / largeur source`, calculée par l'appelant) si la boîte y tient, sinon réduite à `zone.w / (x1 − x0)` puis `zone.h / (y1 − y0)` ; décalage centré : `dx = (zone.w − (x1 − x0)·s) / 2 − x0·s`, de même `dy`. Sans cadre : `s = scale`, `dx = −zone.x`, `dy = −zone.y` (la page source posée sur la page Oku).
  - `to_zone` : `x·s + dx`, `y·s + dy`, puis ramené dans `[0, w] × [0, h]`, côté ≥ 1 pt, au centième de pt.

- [ ] **Step 1 : tests qui échouent**

`crates/oku-slides-import/src/retrofit/zone_tests.rs` :

```rust
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::test_support::{PAPIER, PRO, tokens};

fn grid(columns: u32, rows: u32) -> Grid {
    Grid { columns, rows, gap: None }
}

fn cell(col: u32, row: u32, cols: u32, rows: u32) -> Cell {
    Cell { col, row, cols, rows }
}

fn frame(x: f64, y: f64, w: f64, h: f64) -> Frame {
    Frame { x, y, w, h }
}

/// Zone sans titre de Papier : 1920 − 2 × 120 = 1680 de large, 1080 − 150 − 140 = 790 de haut.
fn papier_zone() -> Zone {
    body_zone(&PageTokens::from_tokens(&tokens(PAPIER)), false)
}

#[test]
fn page_tokens_of_papier_are_the_default_tokens() {
    assert_that!(PageTokens::from_tokens(&tokens(PAPIER))).is_equal_to(PageTokens::default());
}

/// Pro : titre `size.title` = 56, filet de 5 pt plus l'espace `sm` (16), gouttière
/// `component.title.gap` = 44 : bande de `56 + 21 + 44 = 121` ; `space.lg` = 32.
#[test]
fn page_tokens_of_pro_read_its_margins_title_rule_and_spacing() {
    let page = PageTokens::from_tokens(&tokens(PRO));
    assert_that!(page).is_equal_to(PageTokens {
        width: 1920.0,
        height: 1080.0,
        margin_x: 128.0,
        margin_top: 112.0,
        margin_bottom: 160.0,
        title_band: 121.0,
        gap: 32.0,
    });
}

#[test]
fn page_tokens_of_empty_tokens_fall_back_to_the_defaults() {
    assert_that!(PageTokens::from_tokens(&Map::new())).is_equal_to(PageTokens::default());
}

#[test]
fn page_tokens_read_a_numeric_title_size() {
    let t = json!({ "component": { "title": { "size": 80, "gap": 10 } } });
    assert_that!(PageTokens::from_tokens(t.as_object().unwrap()).title_band).is_equal_to(90.0);
}

#[rstest]
#[case(false, Zone { x: 120.0, y: 150.0, w: 1680.0, h: 790.0 })]
#[case(true, Zone { x: 120.0, y: 250.0, w: 1680.0, h: 690.0 })]
fn body_zone_takes_the_margins_and_the_title_band(#[case] titled: bool, #[case] expected: Zone) {
    assert_that!(body_zone(&PageTokens::default(), titled)).is_equal_to(expected);
}

#[rstest]
#[case(None, 40.0)]
#[case(Some(TokenValue::Number(12.0)), 12.0)]
#[case(Some(TokenValue::Name("xl".into())), 40.0)]
fn grid_gap_reads_a_number_else_the_page_gap(#[case] gap: Option<TokenValue>, #[case] expected: f64) {
    let g = Grid { columns: 12, rows: 6, gap };
    assert_that!(grid_gap(&g, &PageTokens::default())).is_equal_to(expected);
}

/// 12 colonnes dans 1680 avec 11 gouttières de 40 : `(1680 − 440) / 12 = 103.33…` ;
/// 6 lignes dans 790 avec 5 gouttières : `(790 − 200) / 6 = 98.33…`.
#[test]
fn cell_rect_follows_the_grid_frame_formula() {
    let r = cell_rect(&papier_zone(), &grid(12, 6), 40.0, &cell(8, 0, 4, 6));
    assert_that!(r.x).is_close_to(8.0 * (1240.0 / 12.0 + 40.0), 1e-9);
    assert_that!(r.y).is_equal_to(0.0);
    assert_that!(r.w).is_close_to(4.0 * 1240.0 / 12.0 + 3.0 * 40.0, 1e-9);
    assert_that!(r.h).is_close_to(790.0, 1e-9);
}

#[test]
fn cell_rect_of_the_whole_grid_is_the_zone() {
    let r = cell_rect(&papier_zone(), &grid(3, 2), 40.0, &cell(0, 0, 3, 2));
    assert_that!(r).is_equal_to(frame(0.0, 0.0, 1680.0, 790.0));
}

#[rstest]
#[case::whole_zone(frame(0.0, 0.0, 1680.0, 790.0), cell(0, 0, 12, 6))]
#[case::left_two_thirds(frame(0.0, 0.0, 1100.0, 790.0), cell(0, 0, 8, 6))]
#[case::right_third(frame(1150.0, 0.0, 530.0, 790.0), cell(8, 0, 4, 6))]
#[case::top_left_cell(frame(5.0, 3.0, 90.0, 90.0), cell(0, 0, 1, 1))]
#[case::tiny_box_keeps_one_cell(frame(700.0, 400.0, 2.0, 2.0), cell(5, 3, 1, 1))]
#[case::outside_is_clamped_to_the_edges(frame(-500.0, -500.0, 5000.0, 5000.0), cell(0, 0, 12, 6))]
fn snap_takes_the_nearest_track_edges(#[case] r: Frame, #[case] expected: Cell) {
    assert_that!(snap(&papier_zone(), &grid(12, 6), 40.0, r)).is_equal_to(expected);
}

#[test]
fn snap_of_a_cell_rect_gives_back_the_cell() {
    let (zone, g) = (papier_zone(), grid(12, 6));
    for c in [cell(0, 0, 1, 1), cell(3, 2, 4, 3), cell(11, 5, 1, 1), cell(8, 0, 4, 6)] {
        assert_that!(snap(&zone, &g, 40.0, cell_rect(&zone, &g, 40.0, &c))).is_equal_to(c);
    }
}

/// Bords de colonne d'une grille 2 × 1 dans 1680 : départs 0 et 860, fins 820 et 1680.
/// Un départ à 430 est à égale distance de 0 et 860 : le premier gagne.
#[test]
fn snap_breaks_ties_towards_the_first_track() {
    let c = snap(&papier_zone(), &grid(2, 1), 40.0, frame(430.0, 0.0, 400.0, 790.0));
    assert_that!(c).is_equal_to(cell(0, 0, 1, 1));
}

#[test]
fn snap_always_fits_the_grid() {
    let (zone, g) = (papier_zone(), grid(3, 3));
    for r in [frame(1600.0, 700.0, 10.0, 10.0), frame(0.0, 0.0, 1.0, 1.0), frame(900.0, 100.0, 9000.0, 1.0)] {
        assert_that!(snap(&zone, &g, 40.0, r).fits(&g)).is_true();
    }
}

/// Échelle de page 2, sans ajustement : la page source (0, 0) tombe sur la page
/// Oku (0, 0), soit (−120, −150) relativement à la zone (120, 150).
fn page_fit() -> Fit {
    Fit { scale: 2.0, dx: -120.0, dy: -150.0 }
}

/// La boîte (100, 100, 200, 100) devient (200, 200, 400, 200) sur la page, soit
/// (80, 50, 400, 200) dans la zone.
#[test]
fn to_zone_scales_then_makes_the_frame_zone_relative() {
    let f = to_zone(&source::Frame::new(100.0, 100.0, 200.0, 100.0), &page_fit(), &papier_zone());
    assert_that!(f).is_equal_to(frame(80.0, 50.0, 400.0, 200.0));
}

#[test]
fn to_zone_clamps_a_frame_overflowing_the_zone() {
    let f = to_zone(&source::Frame::new(0.0, 0.0, 960.0, 540.0), &page_fit(), &papier_zone());
    assert_that!(f).is_equal_to(frame(0.0, 0.0, 1680.0, 790.0));
}

#[test]
fn to_zone_keeps_a_minimal_frame_for_a_box_outside_the_zone() {
    let f = to_zone(&source::Frame::new(0.0, 0.0, 10.0, 10.0), &page_fit(), &papier_zone());
    assert_that!(f).is_equal_to(frame(0.0, 0.0, 1.0, 1.0));
    let far = to_zone(&source::Frame::new(2000.0, 2000.0, 10.0, 10.0), &page_fit(), &papier_zone());
    assert_that!(far).is_equal_to(frame(1679.0, 789.0, 1.0, 1.0));
}

#[test]
fn fit_of_no_frame_lays_the_source_page_on_the_page() {
    assert_that!(Fit::new([], 2.0, &papier_zone())).is_equal_to(page_fit());
}

/// Boîte englobante (64, 176)–(896, 460), 832 × 284 pt source ; à l'échelle 2,
/// 1 664 × 568 tient dans 1 680 × 790 : échelle 2, centrée, soit
/// `dx = (1680 − 1664) / 2 − 128 = −120` et `dy = (790 − 568) / 2 − 352 = −241`.
#[test]
fn fit_of_frames_that_fit_keeps_the_page_scale_and_centres_them() {
    let frames = [source::Frame::new(64.0, 176.0, 277.0, 284.0), source::Frame::new(619.0, 176.0, 277.0, 100.0)];
    assert_that!(Fit::new(&frames, 2.0, &papier_zone())).is_equal_to(Fit { scale: 2.0, dx: -120.0, dy: -241.0 });
}

/// Boîte (0, 0)–(960, 540) : 1 920 × 1 080 à l'échelle 2 ne tient pas ; la
/// largeur demande 1680 / 960 = 1,75, la hauteur 790 / 540 ≈ 1,463 : la plus
/// petite, centrée en largeur (`(1680 − 1404,4) / 2`).
#[test]
fn fit_of_frames_too_large_shrinks_them_uniformly_into_the_zone() {
    let zone = papier_zone();
    let fit = Fit::new(&[source::Frame::new(0.0, 0.0, 960.0, 540.0)], 2.0, &zone);
    assert_that!(fit.scale).is_close_to(790.0 / 540.0, 1e-9);
    assert_that!(fit.dx).is_close_to((1680.0 - 960.0 * 790.0 / 540.0) / 2.0, 1e-9);
    assert_that!(fit.dy).is_close_to(0.0, 1e-9);
    let whole = to_zone(&source::Frame::new(0.0, 0.0, 960.0, 540.0), &fit, &zone);
    assert_that!(whole.h).is_close_to(790.0, 1e-9);
}

/// Grille 12 × 6 de Papier sans titre : colonne `(1680 − 11 × 40) / 12 ≈ 103,3`,
/// ligne `(790 − 5 × 40) / 6 = 98,3`.
#[test]
fn track_size_divides_the_zone_between_the_gutters() {
    let (w, h) = track_size(&papier_zone(), &grid(12, 6), 40.0);
    assert_that!(w).is_close_to(1240.0 / 12.0, 1e-9);
    assert_that!(h).is_close_to(590.0 / 6.0, 1e-9);
}
```

> `fit_of_frames_that_fit_…` : boîte (64, 176)–(896, 460) = 832 × 284 pt source, 1 664 × 568 à l'échelle 2 dans 1 680 × 790 : elle tient ; `dx = (1680 − 1664) / 2 − 64 × 2 = −120`, `dy = (790 − 568) / 2 − 176 × 2 = −241`. `track_size` : `(1680 − 11 × 40) / 12` et `(790 − 5 × 40) / 6`.

`crates/oku-slides-import/src/retrofit/mod.rs` complet :

```rust
//! Projecteur retrofit : une [`SourceDeck`](crate::source::SourceDeck) projetée sur
//! un pack et un thème existants.
//!
//! Le style vient du thème cible : couleurs ramenées à ses jetons ([`ColorTokens`]),
//! texte riche réduit à ce que le cœur exprime, éléments libres placés sur la
//! grille d'une zone de corps approchée par ses jetons de page ([`PageTokens`]).

#[allow(dead_code, reason = "branché par le projecteur, tâche 9")]
mod color;
#[allow(dead_code, reason = "branché par le projecteur, tâche 7")]
mod text;
#[allow(dead_code, reason = "branché par le projecteur, tâche 9")]
mod zone;

pub use color::ColorTokens;
pub use zone::PageTokens;
```

- [ ] **Step 2 : lancer, constater l'échec**

```bash
export CARGO_TARGET_DIR=$PWD/target
cargo nextest run -p oku-slides-import retrofit::zone
```

Attendu : échec de compilation (`retrofit/zone.rs` absent).

- [ ] **Step 3 : implémentation**

`crates/oku-slides-import/src/retrofit/zone.rs` :

```rust
//! Zone de corps et grille : où vont les éléments libres d'une slide.
//!
//! Le cœur place un élément libre relativement à la zone de corps, connue
//! seulement au rendu (marqueur `<oku-slides-zone>`). Le projecteur l'approche
//! par les jetons `page`, `space` et `component.title` du thème cible
//! ([`PageTokens`]). La géométrie de la grille est celle de `grid-frame`
//! (`typst/oku-slides/src/zone.typ`) et de `ui/src/grid.ts` ; [`snap`] porte
//! `snapRect`. Les items d'une slide entrent dans la zone ensemble ([`Fit`]) :
//! leur boîte englobante y est ajustée à échelle uniforme et centrée.

use oku_slides_core::{Cell, Frame, Grid, TokenValue};
use serde_json::{Map, Value};

use crate::source;

/// Plus petit côté d'un cadre projeté, en pt : le cœur refuse un cadre vide.
const MIN_SIDE: f64 = 1.0;

/// Les jetons de page du thème cible, en pt sur la page 1920 × 1080.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageTokens {
    /// Largeur de la page (`page.width`).
    pub width: f64,
    /// Hauteur de la page (`page.height`).
    pub height: f64,
    /// Marge gauche et droite (`page.margin-x`).
    pub margin_x: f64,
    /// Marge haute (`page.margin-top`).
    pub margin_top: f64,
    /// Marge basse (`page.margin-bottom`).
    pub margin_bottom: f64,
    /// Hauteur approchée de la ligne de titre et de sa gouttière.
    pub title_band: f64,
    /// Gouttière de grille par défaut (`space.lg`).
    pub gap: f64,
}

impl Default for PageTokens {
    /// Les valeurs de `default-tokens.json` : titre 60 pt, gouttière `lg` = 40 pt.
    fn default() -> Self {
        Self {
            width: 1920.0,
            height: 1080.0,
            margin_x: 120.0,
            margin_top: 150.0,
            margin_bottom: 140.0,
            title_band: 100.0,
            gap: 40.0,
        }
    }
}

/// La valeur `path` de `tokens` (`["page", "width"]`), si c'est un nombre.
fn number(tokens: &Map<String, Value>, path: &[&str]) -> Option<f64> {
    let (last, groups) = path.split_last()?;
    let mut map = tokens;
    for g in groups {
        map = map.get(*g)?.as_object()?;
    }
    map.get(*last)?.as_f64()
}

/// Une valeur de jeton : un nombre, ou le nom d'un jeton de la famille `family`.
fn token(tokens: &Map<String, Value>, family: &str, v: Option<&Value>) -> Option<f64> {
    match v? {
        Value::Number(n) => n.as_f64(),
        Value::String(name) => number(tokens, &[family, name]),
        _ => None,
    }
}

impl PageTokens {
    /// Les jetons de page des jetons effectifs `tokens` d'un thème (défauts
    /// fusionnés avec ceux du thème) ; un jeton absent ou illisible garde la
    /// valeur de [`PageTokens::default`].
    ///
    /// `title_band` approche la ligne de titre d'une slide de contenu : une
    /// ligne de `component.title.size` (une hauteur d'em, un peu plus que la
    /// ligne réelle de Typst, pour ne pas déborder), le filet
    /// `component.title.rule` s'il existe (sa hauteur et l'espace `sm` qui le
    /// sépare du titre), puis la gouttière `component.title.gap`. Le surtitre
    /// n'est pas compté : le retrofit n'en émet pas.
    pub fn from_tokens(tokens: &Map<String, Value>) -> Self {
        let d = Self::default();
        let get = |path: &[&str], default: f64| number(tokens, path).unwrap_or(default);
        let title = tokens.get("component").and_then(|c| c.get("title")).and_then(Value::as_object);
        let field = |name: &str| title.and_then(|t| t.get(name));
        let size = token(tokens, "size", field("size")).or_else(|| number(tokens, &["size", "title"])).unwrap_or(60.0);
        let gap = get(&["space", "lg"], d.gap);
        let title_gap = token(tokens, "space", field("gap")).unwrap_or(gap);
        let rule = match field("rule") {
            Some(Value::Object(r)) => {
                r.get("height").and_then(Value::as_f64).unwrap_or(0.0) + get(&["space", "sm"], 16.0)
            }
            _ => 0.0,
        };
        Self {
            width: get(&["page", "width"], d.width),
            height: get(&["page", "height"], d.height),
            margin_x: get(&["page", "margin-x"], d.margin_x),
            margin_top: get(&["page", "margin-top"], d.margin_top),
            margin_bottom: get(&["page", "margin-bottom"], d.margin_bottom),
            title_band: size + rule + title_gap,
            gap,
        }
    }
}

/// Un rectangle en pt sur la page 1920 × 1080.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Zone {
    /// Bord gauche.
    pub x: f64,
    /// Bord haut.
    pub y: f64,
    /// Largeur.
    pub w: f64,
    /// Hauteur.
    pub h: f64,
}

/// La zone de corps d'une slide de contenu, en coordonnées de page : la page
/// moins ses marges, moins la ligne de titre si la slide en a une.
pub(crate) fn body_zone(page: &PageTokens, titled: bool) -> Zone {
    let band = if titled { page.title_band } else { 0.0 };
    Zone {
        x: page.margin_x,
        y: page.margin_top + band,
        w: (page.width - 2.0 * page.margin_x).max(MIN_SIDE),
        h: (page.height - page.margin_top - page.margin_bottom - band).max(MIN_SIDE),
    }
}

/// La gouttière de `grid` en pt : un nombre tel quel, sinon `page.gap`.
///
/// Un nom de jeton autre que `lg` est approché par `page.gap` : aucun modèle
/// intégré n'en déclare, et le projecteur n'émet jamais de gouttière.
pub(crate) fn grid_gap(grid: &Grid, page: &PageTokens) -> f64 {
    match &grid.gap {
        Some(TokenValue::Number(n)) => *n,
        Some(TokenValue::Name(_)) | None => page.gap,
    }
}

/// Largeur d'une colonne et hauteur d'une ligne de `grid` dans `zone`.
pub(crate) fn track_size(zone: &Zone, grid: &Grid, gap: f64) -> (f64, f64) {
    let (columns, rows) = (f64::from(grid.columns.max(1)), f64::from(grid.rows.max(1)));
    ((zone.w - (columns - 1.0) * gap) / columns, (zone.h - (rows - 1.0) * gap) / rows)
}

/// Le cadre de `cell`, relatif à `zone` (même calcul que `grid-frame`).
///
/// Le projecteur n'émet que des cellules ; ce calcul est l'oracle des tests
/// (une cellule accrochée retombe sur les pistes de la grille rendue).
#[cfg(test)]
pub(crate) fn cell_rect(zone: &Zone, grid: &Grid, gap: f64, cell: &Cell) -> Frame {
    let (cw, rh) = track_size(zone, grid, gap);
    let (cols, rows) = (f64::from(cell.cols), f64::from(cell.rows));
    Frame {
        x: f64::from(cell.col) * (cw + gap),
        y: f64::from(cell.row) * (rh + gap),
        w: cols * cw + (cols - 1.0) * gap,
        h: rows * rh + (rows - 1.0) * gap,
    }
}

/// L'indice du bord de `edges` le plus proche de `v` ; à égalité, le premier.
fn nearest(edges: impl Iterator<Item = f64>, v: f64) -> u32 {
    let mut best = (0u32, f64::INFINITY);
    for (i, e) in (0u32..).zip(edges) {
        if (e - v).abs() < best.1 {
            best = (i, (e - v).abs());
        }
    }
    best.0
}

/// La cellule de `grid` la plus proche du cadre `r` (relatif à `zone`) :
/// colonne et ligne au début de piste le plus proche, étendue à la fin de
/// piste la plus proche, une case au moins. Portage de `snapRect`.
pub(crate) fn snap(zone: &Zone, grid: &Grid, gap: f64, r: Frame) -> Cell {
    let (cw, rh) = track_size(zone, grid, gap);
    let starts = |n: u32, size: f64| (0..n.max(1)).map(move |i| f64::from(i) * (size + gap));
    let col = nearest(starts(grid.columns, cw), r.x);
    let row = nearest(starts(grid.rows, rh), r.y);
    let end_col = nearest(starts(grid.columns, cw).map(|s| s + cw), r.x + r.w).max(col);
    let end_row = nearest(starts(grid.rows, rh).map(|s| s + rh), r.y + r.h).max(row);
    Cell { col, row, cols: end_col - col + 1, rows: end_row - row + 1 }
}

/// Passage des pt source aux pt relatifs à la zone : `x × scale + dx`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Fit {
    /// Échelle uniforme.
    pub scale: f64,
    /// Décalage horizontal.
    pub dx: f64,
    /// Décalage vertical.
    pub dy: f64,
}

impl Fit {
    /// La boîte englobante de `frames` (pt source) ajustée dans `zone` : à
    /// l'échelle de page `scale` si elle y tient, sinon réduite pour y tenir
    /// (échelle uniforme), et centrée. Sans cadre : l'échelle de page, la page
    /// source posée sur la page Oku.
    pub(crate) fn new<'f>(frames: impl IntoIterator<Item = &'f source::Frame>, scale: f64, zone: &Zone) -> Self {
        let bounds = frames.into_iter().fold(None, |acc: Option<(f64, f64, f64, f64)>, f| {
            let (x0, y0, x1, y1) = acc.unwrap_or((f.x, f.y, f.x + f.w, f.y + f.h));
            Some((x0.min(f.x), y0.min(f.y), x1.max(f.x + f.w), y1.max(f.y + f.h)))
        });
        let Some((x0, y0, x1, y1)) = bounds else { return Self { scale, dx: -zone.x, dy: -zone.y } };
        let (w, h) = (x1 - x0, y1 - y0);
        let mut s = scale;
        if w * s > zone.w && w > 0.0 {
            s = zone.w / w;
        }
        if h * s > zone.h && h > 0.0 {
            s = zone.h / h;
        }
        Self { scale: s, dx: (zone.w - w * s) / 2.0 - x0 * s, dy: (zone.h - h * s) / 2.0 - y0 * s }
    }
}

/// `[a, b]` ramené dans `[0, len]`, d'au moins [`MIN_SIDE`] de long.
fn clamp_span(a: f64, b: f64, len: f64) -> (f64, f64) {
    let lo = a.clamp(0.0, len);
    let hi = b.clamp(0.0, len);
    if hi - lo >= MIN_SIDE {
        return (lo, hi);
    }
    let hi = (lo + MIN_SIDE).min(len.max(MIN_SIDE));
    (hi - MIN_SIDE, hi)
}

/// `v` arrondi au centième de pt : des cadres lisibles, sans écart visible.
fn hundredth(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// Le cadre source `item` (pt de page source) relatif à `zone` par `fit`, et
/// ramené dedans (filet de sûreté : un cadre ajusté y est déjà), au centième de pt.
pub(crate) fn to_zone(item: &source::Frame, fit: &Fit, zone: &Zone) -> Frame {
    let (x0, x1) = clamp_span(item.x * fit.scale + fit.dx, (item.x + item.w) * fit.scale + fit.dx, zone.w);
    let (y0, y1) = clamp_span(item.y * fit.scale + fit.dy, (item.y + item.h) * fit.scale + fit.dy, zone.h);
    let (x0, x1, y0, y1) = (hundredth(x0), hundredth(x1), hundredth(y0), hundredth(y1));
    Frame { x: x0, y: y0, w: hundredth(x1 - x0), h: hundredth(y1 - y0) }
}

#[cfg(test)]
#[path = "zone_tests.rs"]
mod zone_tests;
```

- [ ] **Step 4 : lancer, constater le succès**

```bash
cargo nextest run -p oku-slides-import retrofit::zone
```

Attendu : 27 tests passent (422 pour la crate entière au brouillon).

- [ ] **Step 5 : vérifier, commit, sous-tâche**

```bash
cargo fmt --all && cargo clippy -p oku-slides-import --all-targets -- -D warnings && cargo nextest run -p oku-slides-import
git add crates/oku-slides-import/src/retrofit/mod.rs crates/oku-slides-import/src/retrofit/zone.rs crates/oku-slides-import/src/retrofit/zone_tests.rs && git diff --cached --name-only
git commit -F <scratchpad>/msg.txt   # feat(import) : zone de corps approchée par les jetons de page, ajustement et accroche à la grille (t-692)
oku cli task done --project slider --id t-801
```

---

### Task 6: Tableaux et graphiques

**Écarts d'interface :**
- `table` et `chart` rendent `Option<(serde_json::Value, Vec<LossKind>)>` : `None` pour un tableau sans cellule ou un graphique sans série, que le cœur refuserait (`chart-table-empty`, `chart-no-series`) ; la tâche 7 n'émet alors aucun élément.
- La crate d'import dépend de `oku-slides-chart` (permis par les Global Constraints) : bornes `MAX_TABLE_ROWS`, `MAX_TABLE_COLUMNS`, `MAX_TABLE_CELL_CHARS` ; les tests valident par `parse_table`, `parse_option`, `normalize`, `canonicalize`.
- `test_support.rs` gagne `registry()` (registre de base figé `fixtures/widgets/base-registry.json`), `element_with(id, kind, param, value)` et `apply_blocks(blocks) -> Result<Deck, TransactionError>` (une slide `content` aux blocs donnés, par `Op::AddSlide` sur un deck vide) : la preuve que le cœur accepte ce qui est émis, réutilisée aux tâches 7 et 9.

Le cœur contrôle les paramètres d'hôte à l'`AddSlide` (`ops::host::check_host_params` → `parse_table` / `parse_option`, qui normalise, puis `ensure_ids`) ; il ne canonicalise pas : le projecteur émet directement la forme canonique (données dans `dataset`, séries nues). Un tableau du cœur n'a ni fusion, ni style de cellule, ni texte riche (`TableOption` : `dataset`, `header-row`, `header-column`, `striped`, `align`). Une cellule `null` est permise (`Cell::Null`). Aucune couleur n'est émise dans un graphique : `option.color` et `itemStyle.color` n'acceptent que de l'hexadécimal, et le style vient du thème cible.

**Files:**
- Create: `crates/oku-slides-import/src/retrofit/table.rs`, `crates/oku-slides-import/src/retrofit/table_tests.rs`, `crates/oku-slides-import/src/retrofit/chart.rs`, `crates/oku-slides-import/src/retrofit/chart_tests.rs`
- Modify: `crates/oku-slides-import/Cargo.toml` (`oku-slides-chart`), `Cargo.lock`, `crates/oku-slides-import/src/retrofit/mod.rs` (`mod table; mod chart;`), `crates/oku-slides-import/src/test_support.rs` (registre, application d'une slide)

**Interfaces:**
- Consumes : `text::{plain, note}` (tâche 4) ; `LossKind` (tâche 2) ; `crate::source::{Table, Cell, TableFlags, Chart, ChartKind, Grouping, Series}` (lot 1) ; `oku_slides_chart::{MAX_TABLE_ROWS, MAX_TABLE_COLUMNS, MAX_TABLE_CELL_CHARS}`.
- Produces :

```rust
// crate::retrofit::table
pub(crate) fn table(t: &source::Table) -> Option<(serde_json::Value, Vec<LossKind>)>;
// crate::retrofit::chart
pub(crate) fn chart(c: &source::Chart) -> Option<(serde_json::Value, Vec<LossKind>)>;
// crate::test_support (cfg(test))
pub(crate) fn registry() -> WidgetRegistry;
pub(crate) fn element_with(id: &str, kind: &str, param: &str, value: Value) -> Element;
pub(crate) fn apply_blocks(blocks: Vec<Element>) -> Result<Deck, oku_slides_core::TransactionError>;
```

- Règles (`table`) :
  - `{"dataset": {"source": [...]}, "header-row": first_row, "header-column": first_col, "striped": band_row}` ; la ligne 0 de la source est la ligne d'en-tête du `dataset`.
  - Cellule = son texte plain (`text::plain` : paragraphes joints d'une espace), toujours du texte (un texte qui ressemble à un nombre reste du texte) ; effets perdus des cellules → `RunStyle`, une fois chacun.
  - Fusion : la cellule d'ancrage garde son texte, une cellule couverte vaut `null` ; une fusion présente → `MergedCells` (une fois).
  - Bornes : au plus 100 lignes et 20 colonnes, au plus 1 000 caractères par cellule (coupe à une frontière de caractère, fin d'espaces retirée) → `TableTruncated` (une fois). Une ligne courte est complétée de `null`.
- Règles (`chart`) :

| `ChartKind` | options canoniques |
|---|---|
| `Column` | `xAxis: category`, `yAxis: value`, une série `bar` par série source |
| `Bar` | `xAxis: value`, `yAxis: category` (barres horizontales), séries `bar` |
| `Line` | comme `Column`, séries `line` |
| `Area` | séries `line` avec `"areaStyle": {}` |
| `Pie` | `[{"type": "pie"}]`, première série seule |
| `Doughnut` | `[{"type": "pie", "radius": ["40%", "70%"]}]`, première série seule |
| `Scatter` | une paire de colonnes `X` / nom par série, séries `scatter` avec `encode {x: 2i, y: 2i + 1}` |
| `Radar` | `[{"type": "radar"}]`, axes = catégories (le cœur déduit les maximums) |

  - `dataset.source` : en-tête `["", noms des séries…]` (catégorie sans nom : la source n'en a pas), puis une ligne par catégorie ; les catégories manquantes sont numérotées `1`, `2`… jusqu'à la plus longue série ; une valeur absente ou non finie vaut `null`.
  - Nuage : les catégories lues comme nombres servent d'abscisses si toutes se lisent, sinon `1`, `2`… ; un point sans valeur est `[null, null]`.
  - `Stacked` → `"stack": "total"` sur chaque série cartésienne ; `PercentStacked` aussi, plus la perte `PercentStacked` (secteurs, nuage, radar : empilement ignoré, sans perte).
  - Titre non vide → `"title": {"text": …}`.

- [ ] **Step 1 : tests qui échouent**

Dans `crates/oku-slides-import/Cargo.toml`, la section `[dependencies]` devient :

```toml
[dependencies]
oku-slides-chart.workspace = true
oku-slides-core.workspace = true
oku-slides-i18n.workspace = true
roxmltree.workspace = true
serde.workspace = true
serde_json.workspace = true
snafu.workspace = true
zip.workspace = true
```

Dans `crates/oku-slides-import/src/test_support.rs`, la doc et les imports deviennent :

```rust
//! Aides de test du projecteur : jetons effectifs des thèmes intégrés, texte, items,
//! slides et decks source, registre de base, application d'une slide par les ops du cœur.

use std::collections::BTreeMap;

use oku_slides_core::tokens::effective_tokens;
use oku_slides_core::{Deck, Element, Op, ParamValue, Slide, WidgetRegistry, apply_all};
use serde_json::{Map, Value};

use crate::retrofit::ColorTokens;
use crate::source::{
    Align, Bullet, Caps, Color, Fill, Frame, Item, ItemKind, ParaStyle, Paragraph, Rgba, Run, RunStyle, Size,
    SourceDeck, SourceSlide, Spacing, Styled, TextBox,
};
```

puis, après `text_box`, ajouter :

```rust
/// Le registre des widgets de base (`fixtures/widgets/base-registry.json`).
pub(crate) fn registry() -> WidgetRegistry {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/base-registry.json");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} : {e}"));
    WidgetRegistry::from_probe(serde_json::from_str(&text).unwrap()).unwrap()
}

/// Un élément `kind` d'id `id` dont le paramètre `param` vaut `value`.
pub(crate) fn element_with(id: &str, kind: &str, param: &str, value: Value) -> Element {
    let mut e = Element::new(id, kind);
    e.params.set(param, ParamValue::Value(value));
    e
}

/// Le deck obtenu en ajoutant une slide `content` dont les blocs sont `blocks`,
/// par `Op::AddSlide` sur le registre de base.
pub(crate) fn apply_blocks(blocks: Vec<Element>) -> Result<Deck, oku_slides_core::TransactionError> {
    let mut slide = Slide::new("s1", "content");
    slide.params.set("blocks", ParamValue::Slot(blocks));
    let mut deck = Deck::empty();
    apply_all(&mut deck, vec![Op::AddSlide { index: None, defaults: false, slide }], &registry())?;
    Ok(deck)
}
```

`crates/oku-slides-import/src/retrofit/table_tests.rs` :

```rust
use oku_slides_chart::parse_table;
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::report::RunEffect;
use crate::source::{Borders, Fill, TableFlags};
use crate::test_support::{apply_blocks, element_with, para, run, run_with, text_box};

fn cell(text: &str) -> source::Cell {
    source::Cell {
        text: text_box(vec![para(vec![run(text)])]),
        fill: Fill::None,
        borders: Borders::default(),
        row_span: 1,
        col_span: 1,
        covered: false,
    }
}

fn covered() -> source::Cell {
    source::Cell { text: text_box(Vec::new()), covered: true, ..cell("") }
}

fn source_table(rows: Vec<Vec<source::Cell>>) -> source::Table {
    let flags = TableFlags { first_row: true, band_row: true, ..TableFlags::default() };
    source::Table { col_widths: Vec::new(), row_heights: Vec::new(), flags, rows }
}

fn grid(rows: usize, cols: usize) -> source::Table {
    source_table((0..rows).map(|r| (0..cols).map(|c| cell(&format!("{r}.{c}"))).collect()).collect())
}

fn data(t: &source::Table) -> (Value, Vec<LossKind>) {
    table(t).unwrap()
}

#[test]
fn table_writes_row_zero_as_the_header_and_cells_as_text() {
    let t = source_table(vec![vec![cell("Pays"), cell("Ventes")], vec![cell("FR"), cell("10")]]);
    let (option, losses) = data(&t);
    assert_that!(option).is_equal_to(json!({
        "dataset": { "source": [["Pays", "Ventes"], ["FR", "10"]] },
        "header-row": true, "header-column": false, "striped": true,
    }));
    assert_that!(losses).is_empty();
}

#[test]
fn table_option_is_accepted_by_the_table_host() {
    let (option, _) = data(&grid(3, 4));
    assert_that!(parse_table(&option)).is_ok();
}

#[test]
fn table_flattens_merged_cells_into_their_anchor() {
    let mut anchor = cell("Fusion");
    anchor.col_span = 2;
    let t = source_table(vec![vec![anchor, covered()], vec![cell("a"), cell("b")]]);
    let (option, losses) = data(&t);
    assert_that!(option["dataset"]["source"]).is_equal_to(json!([["Fusion", null], ["a", "b"]]));
    assert_that!(losses).is_equal_to(vec![LossKind::MergedCells]);
    assert_that!(parse_table(&option)).is_ok();
}

#[test]
fn table_pads_a_short_row_with_empty_cells() {
    let t = source_table(vec![vec![cell("a"), cell("b")], vec![cell("c")]]);
    assert_that!(data(&t).0["dataset"]["source"]).is_equal_to(json!([["a", "b"], ["c", null]]));
}

#[test]
fn table_joins_the_paragraphs_of_a_cell() {
    let mut c = cell("");
    c.text = text_box(vec![para(vec![run("Ligne 1")]), para(vec![run("Ligne 2")])]);
    assert_that!(data(&source_table(vec![vec![c]])).0["dataset"]["source"]).is_equal_to(json!([["Ligne 1 Ligne 2"]]));
}

#[test]
fn table_reports_the_effects_lost_in_cells_once() {
    let mut c = cell("");
    c.text = text_box(vec![para(vec![run_with("souligné", |s| s.underline = true)])]);
    let t = source_table(vec![vec![c.clone(), c]]);
    assert_that!(data(&t).1).is_equal_to(vec![LossKind::RunStyle(RunEffect::Underline)]);
}

/// 120 lignes × 25 colonnes : il en reste `MAX_TABLE_ROWS` = 100 et `MAX_TABLE_COLUMNS` = 20.
#[test]
fn table_cuts_to_the_core_row_and_column_bounds() {
    let (option, losses) = data(&grid(120, 25));
    let source = option["dataset"]["source"].as_array().unwrap();
    assert_that!(source.len()).is_equal_to(MAX_TABLE_ROWS);
    assert_that!(source.iter().all(|r| r.as_array().unwrap().len() == MAX_TABLE_COLUMNS)).is_true();
    assert_that!(losses).is_equal_to(vec![LossKind::TableTruncated]);
    assert_that!(parse_table(&option)).is_ok();
}

#[test]
fn table_cuts_a_long_cell_to_the_core_character_bound() {
    let t = source_table(vec![vec![cell(&"é".repeat(1500))]]);
    let (option, losses) = data(&t);
    assert_that!(option["dataset"]["source"][0][0].as_str().unwrap().chars().count()).is_equal_to(MAX_TABLE_CELL_CHARS);
    assert_that!(losses).is_equal_to(vec![LossKind::TableTruncated]);
    assert_that!(parse_table(&option)).is_ok();
}

#[test]
fn table_without_cells_is_none() {
    assert_that!(table(&source_table(Vec::new()))).is_none();
    assert_that!(table(&source_table(vec![Vec::new()]))).is_none();
}

#[test]
fn table_element_is_accepted_by_add_slide() {
    let (option, _) = data(&grid(4, 3));
    let deck = apply_blocks(vec![element_with("t", "table", "data", option)]);
    assert_that!(deck).is_ok();
}
```

`crates/oku-slides-import/src/retrofit/chart_tests.rs` :

```rust
use oku_slides_chart::{ChartLabels, ChartModel, Length, canonicalize, normalize, parse_option};
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::test_support::{apply_blocks, element_with};

fn series(name: &str, values: &[f64]) -> source::Series {
    source::Series { name: name.to_owned(), values: values.iter().copied().map(Some).collect(), color: None }
}

fn source_chart(kind: ChartKind, grouping: Grouping) -> source::Chart {
    source::Chart {
        kind,
        grouping,
        title: Some("Ventes".to_owned()),
        categories: vec!["T1".into(), "T2".into(), "T3".into()],
        series: vec![series("2025", &[1.0, 2.0, 3.0]), series("2026", &[4.0, 5.0, 6.0])],
    }
}

fn option_of(c: &source::Chart) -> Value {
    chart(c).unwrap().0
}

/// Le modèle que le cœur tire des options : elles passent `parse_option`.
fn model(c: &source::Chart) -> ChartModel {
    let option = parse_option(&option_of(c)).unwrap();
    normalize(&option, ChartLabels::english()).unwrap()
}

/// Ce que le cœur lit d'un graphique cartésien.
#[derive(Debug, PartialEq)]
struct Read {
    horizontal: bool,
    area: bool,
    categories: Vec<String>,
    names: Vec<String>,
    stacks: Vec<Option<String>>,
}

fn cartesian(c: &source::Chart) -> Read {
    match model(c).kind {
        oku_slides_chart::ChartKind::Cartesian(x) => Read {
            horizontal: x.horizontal,
            area: x.area,
            categories: x.categories,
            names: x.series.iter().map(|s| s.name.clone()).collect(),
            stacks: x.series.iter().map(|s| s.stack.clone()).collect(),
        },
        other => panic!("cartésien attendu : {other:?}"),
    }
}

#[test]
fn chart_of_columns_is_the_canonical_bar_option() {
    let (option, losses) = chart(&source_chart(ChartKind::Column, Grouping::Clustered)).unwrap();
    assert_that!(option).is_equal_to(json!({
        "dataset": { "source": [["", "2025", "2026"], ["T1", 1.0, 4.0], ["T2", 2.0, 5.0], ["T3", 3.0, 6.0]] },
        "xAxis": { "type": "category" },
        "yAxis": { "type": "value" },
        "series": [{ "type": "bar" }, { "type": "bar" }],
        "title": { "text": "Ventes" },
    }));
    assert_that!(losses).is_empty();
}

/// Déjà canonique : `canonicalize` rend les options telles quelles.
#[test]
fn chart_option_is_already_canonical() {
    let option = parse_option(&option_of(&source_chart(ChartKind::Line, Grouping::Standard))).unwrap();
    assert_that!(canonicalize(&option, ChartLabels::english()).unwrap()).is_equal_to(&option);
}

#[rstest]
#[case::column(ChartKind::Column, "bar", false, false)]
#[case::bar(ChartKind::Bar, "bar", true, false)]
#[case::line(ChartKind::Line, "line", false, false)]
#[case::area(ChartKind::Area, "line", false, true)]
fn chart_maps_cartesian_kinds(
    #[case] kind: ChartKind,
    #[case] series_type: &str,
    #[case] horizontal: bool,
    #[case] area: bool,
) {
    let c = source_chart(kind, Grouping::Clustered);
    assert_that!(option_of(&c)["series"][0]["type"].as_str()).is_equal_to(Some(series_type));
    let read = cartesian(&c);
    assert_that!((read.horizontal, read.area)).is_equal_to((horizontal, area));
    assert_that!(read.categories).is_equal_to(vec!["T1".to_owned(), "T2".to_owned(), "T3".to_owned()]);
    assert_that!(read.names).is_equal_to(vec!["2025".to_owned(), "2026".to_owned()]);
}

#[rstest]
#[case(Grouping::Stacked, vec![])]
#[case(Grouping::PercentStacked, vec![LossKind::PercentStacked])]
fn chart_stacks_series_of_a_stacked_chart(#[case] grouping: Grouping, #[case] losses: Vec<LossKind>) {
    let c = source_chart(ChartKind::Column, grouping);
    assert_that!(chart(&c).unwrap().1).is_equal_to(losses);
    assert_that!(cartesian(&c).stacks).is_equal_to(vec![Some(STACK.to_owned()); 2]);
}

#[test]
fn chart_of_clustered_columns_does_not_stack() {
    assert_that!(cartesian(&source_chart(ChartKind::Column, Grouping::Clustered)).stacks).is_equal_to(vec![None, None]);
}

#[test]
fn chart_of_a_pie_keeps_the_first_series() {
    match model(&source_chart(ChartKind::Pie, Grouping::Clustered)).kind {
        oku_slides_chart::ChartKind::Pie(p) => {
            assert_that!(p.slices).is_equal_to(vec![
                ("T1".to_owned(), Some(1.0)),
                ("T2".to_owned(), Some(2.0)),
                ("T3".to_owned(), Some(3.0)),
            ]);
            assert_that!(p.inner).is_none();
        }
        other => panic!("secteurs attendus : {other:?}"),
    }
}

#[test]
fn chart_of_a_doughnut_is_a_pie_with_two_radii() {
    match model(&source_chart(ChartKind::Doughnut, Grouping::Clustered)).kind {
        oku_slides_chart::ChartKind::Pie(p) => {
            assert_that!(p.inner).is_equal_to(Some(Length::Percent("40%".into())));
            assert_that!(p.outer).is_equal_to(Some(Length::Percent("70%".into())));
        }
        other => panic!("secteurs attendus : {other:?}"),
    }
}

#[test]
fn chart_of_a_scatter_reads_numeric_categories_as_x() {
    let mut c = source_chart(ChartKind::Scatter, Grouping::Clustered);
    c.categories = vec!["0.5".into(), "1".into(), "2".into()];
    c.series[1].values[1] = None;
    match model(&c).kind {
        oku_slides_chart::ChartKind::Scatter(s) => {
            assert_that!(s.series[0].points).is_equal_to(vec![(0.5, 1.0), (1.0, 2.0), (2.0, 3.0)]);
            assert_that!(s.series[1].points).is_equal_to(vec![(0.5, 4.0), (2.0, 6.0)]);
        }
        other => panic!("nuage attendu : {other:?}"),
    }
}

#[test]
fn chart_of_a_scatter_numbers_text_categories() {
    match model(&source_chart(ChartKind::Scatter, Grouping::Clustered)).kind {
        oku_slides_chart::ChartKind::Scatter(s) => {
            assert_that!(s.series[0].points).is_equal_to(vec![(1.0, 1.0), (2.0, 2.0), (3.0, 3.0)]);
        }
        other => panic!("nuage attendu : {other:?}"),
    }
}

#[test]
fn chart_of_a_radar_takes_categories_as_axes() {
    match model(&source_chart(ChartKind::Radar, Grouping::Standard)).kind {
        oku_slides_chart::ChartKind::Radar(r) => {
            assert_that!(r.indicators.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>())
                .is_equal_to(vec!["T1", "T2", "T3"]);
            assert_that!(r.series).has_length(2);
        }
        other => panic!("radar attendu : {other:?}"),
    }
}

#[test]
fn chart_numbers_missing_categories_and_empties_missing_values() {
    let mut c = source_chart(ChartKind::Column, Grouping::Clustered);
    c.categories = vec!["A".into()];
    c.series[0].values = vec![Some(1.0), None, Some(f64::NAN)];
    let source = &option_of(&c)["dataset"]["source"];
    assert_that!(source).is_equal_to(&json!([
        ["", "2025", "2026"],
        ["A", 1.0, 4.0],
        ["2", null, 5.0],
        ["3", null, 6.0]
    ]));
}

#[test]
fn chart_without_title_has_no_title() {
    let mut c = source_chart(ChartKind::Column, Grouping::Clustered);
    c.title = Some("  ".into());
    assert_that!(option_of(&c).get("title")).is_none();
}

#[test]
fn chart_without_series_is_none() {
    let mut c = source_chart(ChartKind::Column, Grouping::Clustered);
    c.series.clear();
    assert_that!(chart(&c)).is_none();
}

#[rstest]
fn chart_element_of_every_kind_is_accepted_by_add_slide(
    #[values(
        ChartKind::Column,
        ChartKind::Bar,
        ChartKind::Line,
        ChartKind::Area,
        ChartKind::Pie,
        ChartKind::Doughnut,
        ChartKind::Scatter,
        ChartKind::Radar
    )]
    kind: ChartKind,
) {
    let option = option_of(&source_chart(kind, Grouping::Stacked));
    assert_that!(apply_blocks(vec![element_with("c", "chart", "option", option)])).is_ok();
}
```

`crates/oku-slides-import/src/retrofit/mod.rs` complet :

```rust
//! Projecteur retrofit : une [`SourceDeck`](crate::source::SourceDeck) projetée sur
//! un pack et un thème existants.
//!
//! Le style vient du thème cible : couleurs ramenées à ses jetons ([`ColorTokens`]),
//! texte riche réduit à ce que le cœur exprime, éléments libres placés sur la
//! grille d'une zone de corps approchée par ses jetons de page ([`PageTokens`]).

#[allow(dead_code, reason = "branché par les contenus, tâche 7")]
mod chart;
#[allow(dead_code, reason = "branché par le projecteur, tâche 9")]
mod color;
#[allow(dead_code, reason = "branché par les contenus, tâche 7")]
mod table;
#[allow(dead_code, reason = "branché par le projecteur, tâche 7")]
mod text;
#[allow(dead_code, reason = "branché par le projecteur, tâche 9")]
mod zone;

pub use color::ColorTokens;
pub use zone::PageTokens;
```

> `chart_option_is_already_canonical` : `canonicalize` rend ses options inchangées quand elles ont un `dataset` et aucune donnée en ligne (`series[].data`) ; la forme exacte (séries nues) est fixée par `chart_of_columns_is_the_canonical_bar_option`. `chart_numbers_missing_categories_and_empties_missing_values` : une catégorie (`A`) pour trois valeurs, d'où les lignes `2` et `3` ; `NAN` devient `null`.

- [ ] **Step 2 : lancer, constater l'échec**

```bash
export CARGO_TARGET_DIR=$PWD/target
cargo nextest run -p oku-slides-import retrofit::table retrofit::chart
```

Attendu : échec de compilation (`retrofit/table.rs` et `retrofit/chart.rs` absents).

- [ ] **Step 3 : implémentation**

`crates/oku-slides-import/src/retrofit/table.rs` :

```rust
//! Tableau source → options d'un widget `table` (`host(table)`).
//!
//! Le tableau du cœur n'a ni fusion, ni style de cellule, ni texte riche :
//! chaque cellule devient son texte plain ; une fusion est aplatie (le texte
//! reste dans la cellule d'ancrage, les cellules couvertes sont vides).

use oku_slides_chart::{MAX_TABLE_CELL_CHARS, MAX_TABLE_COLUMNS, MAX_TABLE_ROWS};
use serde_json::{Value, json};

use super::text::{note, plain};
use crate::report::LossKind;
use crate::source;

/// Le texte de `cell`, au plus `MAX_TABLE_CELL_CHARS` caractères ; vrai s'il est coupé.
fn cell_text(cell: &source::Cell, losses: &mut Vec<LossKind>) -> String {
    let (text, lost) = plain(&cell.text);
    for l in lost {
        note(losses, l);
    }
    match text.char_indices().nth(MAX_TABLE_CELL_CHARS) {
        Some((end, _)) => {
            note(losses, LossKind::TableTruncated);
            text[..end].trim_end().to_owned()
        }
        None => text,
    }
}

/// Les options du tableau `t` et ses pertes ; `None` pour un tableau sans cellule.
///
/// La ligne 0 de la source est la ligne d'en-tête du `dataset` ;
/// `header-row`, `header-column` et `striped` viennent des options PowerPoint
/// (`firstRow`, `firstCol`, `bandRow`). Un texte qui ressemble à un nombre
/// reste du texte. Au plus `MAX_TABLE_ROWS` lignes et `MAX_TABLE_COLUMNS`
/// colonnes : le reste est coupé (`TableTruncated`). Une ligne plus courte que
/// les autres est complétée de cellules vides.
pub(crate) fn table(t: &source::Table) -> Option<(Value, Vec<LossKind>)> {
    let width = t.rows.iter().map(Vec::len).max().unwrap_or(0);
    if width == 0 {
        return None;
    }
    let mut losses = Vec::new();
    if t.rows.len() > MAX_TABLE_ROWS || width > MAX_TABLE_COLUMNS {
        note(&mut losses, LossKind::TableTruncated);
    }
    let width = width.min(MAX_TABLE_COLUMNS);
    let mut merged = false;
    let source: Vec<Vec<Value>> = t
        .rows
        .iter()
        .take(MAX_TABLE_ROWS)
        .map(|row| {
            let mut cells: Vec<Value> = row
                .iter()
                .take(width)
                .map(|c| {
                    merged |= c.covered || c.row_span > 1 || c.col_span > 1;
                    if c.covered { Value::Null } else { Value::String(cell_text(c, &mut losses)) }
                })
                .collect();
            cells.resize(width, Value::Null);
            cells
        })
        .collect();
    if merged {
        note(&mut losses, LossKind::MergedCells);
    }
    let option = json!({
        "dataset": { "source": source },
        "header-row": t.flags.first_row,
        "header-column": t.flags.first_col,
        "striped": t.flags.band_row,
    });
    Some((option, losses))
}

#[cfg(test)]
#[path = "table_tests.rs"]
mod table_tests;
```

`crates/oku-slides-import/src/retrofit/chart.rs` :

```rust
//! Graphique source → options canoniques d'un widget `chart` (`host(chart)`).
//!
//! Forme canonique (`oku_slides_chart::canonicalize`) : toutes les données
//! dans `dataset`, des séries nues qui lisent chacune leur colonne. Aucune
//! couleur n'est émise : la palette du thème cible décide.

use serde_json::{Value, json};

use crate::report::LossKind;
use crate::source::{self, ChartKind, Grouping};

/// Groupe d'empilement des séries empilées.
const STACK: &str = "total";
/// Rayons d'un anneau.
const DOUGHNUT: [&str; 2] = ["40%", "70%"];

/// Une valeur : un nombre fini, sinon une cellule vide.
fn value(v: Option<f64>) -> Value {
    v.filter(|v| v.is_finite()).map_or(Value::Null, Value::from)
}

/// Les libellés des catégories, complétés `1`, `2`… jusqu'à la plus longue série.
fn categories(c: &source::Chart) -> Vec<String> {
    let rows = c.series.iter().map(|s| s.values.len()).max().unwrap_or(0).max(c.categories.len());
    (0..rows).map(|i| c.categories.get(i).cloned().unwrap_or_else(|| (i + 1).to_string())).collect()
}

/// `dataset.source` : en-tête (catégorie vide puis noms des séries), une ligne par catégorie.
fn grid(c: &source::Chart, series: &[source::Series]) -> Vec<Vec<Value>> {
    let mut header = vec![Value::from("")];
    header.extend(series.iter().map(|s| Value::from(s.name.as_str())));
    let rows = categories(c).into_iter().enumerate().map(|(i, cat)| {
        let mut row = vec![Value::from(cat)];
        row.extend(series.iter().map(|s| value(s.values.get(i).copied().flatten())));
        row
    });
    std::iter::once(header).chain(rows).collect()
}

/// Une paire de colonnes (x, y) par série ; les abscisses sont les catégories
/// lues comme nombres, sinon `1`, `2`… ; une valeur absente n'est pas un point.
fn scatter_grid(c: &source::Chart) -> Vec<Vec<Value>> {
    let cats = categories(c);
    let parsed: Option<Vec<f64>> =
        cats.iter().map(|s| s.trim().parse::<f64>().ok().filter(|x| x.is_finite())).collect();
    let xs = parsed.unwrap_or_else(|| (1..=cats.len()).map(|i| i as f64).collect());
    let header = c.series.iter().flat_map(|s| [Value::from("X"), Value::from(s.name.as_str())]).collect();
    let rows = xs.iter().enumerate().map(|(i, x)| {
        c.series
            .iter()
            .flat_map(|s| match value(s.values.get(i).copied().flatten()) {
                Value::Null => [Value::Null, Value::Null],
                y => [Value::from(*x), y],
            })
            .collect()
    });
    std::iter::once(header).chain(rows).collect()
}

/// Les options canoniques du graphique `c` et ses pertes ; `None` sans série.
///
/// | source | options |
/// |---|---|
/// | `Column` | barres, catégories en X |
/// | `Bar` | barres, catégories en Y |
/// | `Line` | lignes |
/// | `Area` | lignes avec `areaStyle` |
/// | `Pie` | secteurs, première série seule |
/// | `Doughnut` | secteurs, rayons `40%` et `70%` |
/// | `Scatter` | une paire de colonnes (x, y) par série |
/// | `Radar` | radar, axes = catégories |
///
/// `Stacked` pose `stack` sur les séries cartésiennes ; `PercentStacked` aussi,
/// avec la perte `PercentStacked`. Le titre devient `title.text`.
pub(crate) fn chart(c: &source::Chart) -> Option<(Value, Vec<LossKind>)> {
    if c.series.is_empty() {
        return None;
    }
    let mut losses = Vec::new();
    let cartesian = |kind: &str, area: bool| {
        let mut s = json!({ "type": kind });
        if matches!(c.grouping, Grouping::Stacked | Grouping::PercentStacked) {
            s["stack"] = json!(STACK);
        }
        if area {
            s["areaStyle"] = json!({});
        }
        vec![s; c.series.len()]
    };
    let category = json!({ "type": "category" });
    let value_axis = json!({ "type": "value" });
    let mut option = match c.kind {
        ChartKind::Column | ChartKind::Line | ChartKind::Area => {
            let kind = if c.kind == ChartKind::Column { "bar" } else { "line" };
            json!({
                "dataset": { "source": grid(c, &c.series) },
                "xAxis": category, "yAxis": value_axis,
                "series": cartesian(kind, c.kind == ChartKind::Area),
            })
        }
        ChartKind::Bar => json!({
            "dataset": { "source": grid(c, &c.series) },
            "xAxis": value_axis, "yAxis": category,
            "series": cartesian("bar", false),
        }),
        ChartKind::Pie | ChartKind::Doughnut => {
            let mut s = json!({ "type": "pie" });
            if c.kind == ChartKind::Doughnut {
                s["radius"] = json!(DOUGHNUT);
            }
            json!({ "dataset": { "source": grid(c, &c.series[..1]) }, "series": [s] })
        }
        ChartKind::Scatter => {
            let series: Vec<Value> = (0..c.series.len())
                .map(|i| json!({ "type": "scatter", "encode": { "x": 2 * i, "y": 2 * i + 1 } }))
                .collect();
            json!({ "dataset": { "source": scatter_grid(c) }, "series": series })
        }
        ChartKind::Radar => json!({ "dataset": { "source": grid(c, &c.series) }, "series": [{ "type": "radar" }] }),
    };
    let stacked = matches!(c.kind, ChartKind::Column | ChartKind::Bar | ChartKind::Line | ChartKind::Area);
    if stacked && c.grouping == Grouping::PercentStacked {
        losses.push(LossKind::PercentStacked);
    }
    if let Some(title) = c.title.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        option["title"] = json!({ "text": title });
    }
    Some((option, losses))
}

#[cfg(test)]
#[path = "chart_tests.rs"]
mod chart_tests;
```

- [ ] **Step 4 : lancer, constater le succès**

```bash
cargo nextest run -p oku-slides-import retrofit::table retrofit::chart
```

Attendu : 35 tests passent (457 pour la crate entière au brouillon).

- [ ] **Step 5 : vérifier, commit, sous-tâche**

```bash
cargo fmt --all && cargo clippy -p oku-slides-import --all-targets -- -D warnings && cargo nextest run -p oku-slides-import
git add Cargo.lock crates/oku-slides-import/Cargo.toml crates/oku-slides-import/src/retrofit/mod.rs crates/oku-slides-import/src/retrofit/table.rs crates/oku-slides-import/src/retrofit/table_tests.rs crates/oku-slides-import/src/retrofit/chart.rs crates/oku-slides-import/src/retrofit/chart_tests.rs crates/oku-slides-import/src/test_support.rs && git diff --cached --name-only
git commit -F <scratchpad>/msg.txt   # feat(import) : tableaux et graphiques source en options des widgets table et chart (t-692)
oku cli task done --project slider --id t-802
```

---

### Task 7: Unités de contenu, cartes et replis ; signature de motif

**Écarts d'interface :**
- La conversion part d'une **unité de contenu** (`cluster::Unit`), plus d'un item (révision 1, item 2) : `element(unit: &Unit<'_>, id: String, cx: &ContentCx<'_>) -> Content`. `Content` n'a plus de durée de vie ni de champ `item` (l'appelant connaît son unité, `Unit::lead`) ; son champ `element` est une `Option<Element>` : une unité qui ne donne rien (intraduisible sans repli, image que le rendu ne dessine pas) rend quand même ses pertes.
- Nouveau module `retrofit/cluster.rs` : `Unit` (`Item`, `Card { shape, texts, inner }`, `Text(Vec<&Item>)`) et `units(items, cell) -> Vec<Unit>`, qui regroupe les cartes puis fusionne les textes voisins dont les cellules se chevauchent (sur la grille de référence que fournit l'appelant, tâche 9). Le choix du modèle (tâche 8) compte les unités, pas les items.
- Nouveau module `retrofit/group.rs` (le repli des groupes sort de `content.rs`, qui dépasserait 500 lignes) : `group(items, smartart, id, cx, losses) -> (Option<Element>, Option<String>)`. Le repli n'est dit (`Content.fallback`, d'où la `WidgetRequest` de la tâche 9) que s'il perd quelque chose (révision 1, item 9, décision 5 de la spec) : une forme omise (flèche, connecteur), un motif régulier d'au moins trois feuilles, ou un SmartArt.
- Lot 1 : `ItemKind::Group` gagne `smartart: bool` (vrai pour la version dessinée d'un SmartArt, posé par `pptx/smartart.rs` ; sérialisé seulement s'il est vrai). Les correspondances `Group { items }` du lecteur deviennent `Group { items, .. }` ; le snapshot `kitchen__kitchen` gagne `"smartart": true` sur le SmartArt de la slide 3.
- `ContentCx` gagne `track: (f64, f64)` (une colonne et une ligne de la grille de la slide en pt source) : une forme plus fine qu'un quart de piste est un **filet** (révision 1, item 3), `shape: line` tracé du jeton de son trait, sinon de son fond.
- Forme remplie avec texte, et carte d'unité → `card` ; une carte est en relief (`highlight: true`) quand son fond se projette sur un jeton qui contraste (≥ 3:1) avec le fond d'une carte du thème et que le thème a un jeton de texte clair pour le relief ; sinon le fond est abandonné (révision 1, item 6). Ses textes sont colorés contre le fond de la carte rendue (`Ground::Card`), ceux d'un `text` contre la page (`Ground::Page`).
- Une forme dont ni le fond ni le trait n'ont de jeton ne donne rien.
- Images : `fit: "contain"` (une case de grille change le rapport largeur/hauteur ; l'image reste entière). Types dessinés : `image/png`, `image/jpeg`, `image/gif`, `image/webp`, `image/svg+xml` (formats de Typst 0.15) ; tout autre type (`emf`, `wmf`, `tiff`, `wdp`, `bmp`) → rien et `UnsupportedImage(mime)`. Un asset absent → rien, sans perte (le lecteur a déjà averti `MissingPart`).
- `pattern` reste un module privé (`mod pattern;` dans `lib.rs`, sans ré-export, conformément aux Global Constraints) avec `#[allow(dead_code)]` jusqu'à la tâche 9 ; `signature` reste `pub` dans ce module. Sa doc dit la limite de l'arrondi au quart (un bord près d'un huitième bascule : une demande en trop, jamais un mauvais rendu).
- Les `#[allow(dead_code)]` de `retrofit::color`, `retrofit::text`, `retrofit::table` et `retrofit::chart` tombent : `content` les emploie. `content`, `cluster` et `group` portent le leur jusqu'à la tâche 9.
- `test_support.rs` gagne `text_item(id, frame, text) -> Item` (`item` vient de la tâche 3).

**Files:**
- Create: `crates/oku-slides-import/src/retrofit/cluster.rs`, `crates/oku-slides-import/src/retrofit/cluster_tests.rs`, `crates/oku-slides-import/src/retrofit/content.rs`, `crates/oku-slides-import/src/retrofit/content_tests.rs`, `crates/oku-slides-import/src/retrofit/group.rs`, `crates/oku-slides-import/src/retrofit/group_tests.rs`, `crates/oku-slides-import/src/pattern.rs`, `crates/oku-slides-import/src/pattern_tests.rs`
- Modify: `crates/oku-slides-import/src/retrofit/mod.rs` (modules, retrait des `allow`), `crates/oku-slides-import/src/lib.rs` (`mod pattern;`), `crates/oku-slides-import/src/test_support.rs` (`text_item`), lot 1 : `crates/oku-slides-import/src/source/mod.rs`, `crates/oku-slides-import/src/source/mod_tests.rs`, `crates/oku-slides-import/src/pptx/smartart.rs`, `crates/oku-slides-import/src/pptx/smartart_tests.rs`, `crates/oku-slides-import/src/pptx/shapes.rs`, `crates/oku-slides-import/src/pptx/shapes_tests.rs`, `crates/oku-slides-import/src/pptx/fonts.rs`, `crates/oku-slides-import/tests/kitchen.rs`, `crates/oku-slides-import/tests/snapshots/kitchen__kitchen.snap`

**Interfaces:**
- Consumes : `ColorTokens`, `project_color`, `contrast`, `MIN_CONTRAST` (tâche 3) ; `text::{rich, plain, note}` (tâche 4) ; `table::table`, `chart::chart` (tâche 6) ; `Loss`, `LossKind` (tâche 2) ; `crate::source::{Item, ItemKind, Shape, Geometry, Fill, Picture, Crop, Opaque, Asset, TextBox, Frame}`, `Item::text`, `Item::has_text` (lot 1, tâche 3) ; `oku_slides_core::{Cell, Element, ParamValue, Paragraph}`.
- Produces :

```rust
// crate::retrofit::cluster
pub(crate) enum Unit<'a> {
    Item(&'a Item),
    Card { shape: &'a Item, texts: Vec<&'a Item>, inner: Vec<&'a Item> },
    Text(Vec<&'a Item>),
}
impl<'a> Unit<'a> {
    pub(crate) fn frame(&self) -> Frame;          // forme d'une carte, englobante des textes fusionnés
    pub(crate) fn items(&self) -> Vec<&'a Item>;  // tous ses items source
    pub(crate) fn lead(&self) -> &'a Item;        // forme d'une carte, premier texte
}
pub(crate) fn bounds<'f>(frames: impl Iterator<Item = &'f Frame>) -> Frame;
pub(crate) fn reading(a: &Frame, b: &Frame) -> std::cmp::Ordering;   // haut en bas, puis gauche à droite
pub(crate) fn units<'a>(items: &[&'a Item], cell: impl Fn(&Frame) -> Cell) -> Vec<Unit<'a>>;
// crate::retrofit::content
pub(crate) struct ContentCx<'a> { pub tokens: &'a ColorTokens, pub assets: &'a BTreeMap<String, Asset>, pub track: (f64, f64) }   // Copy
pub(crate) struct Content { pub element: Option<Element>, pub losses: Vec<Loss>, pub fallback: Option<String> }
pub(crate) enum Ground { Page, Card(bool) }
pub(crate) fn element(unit: &Unit<'_>, id: String, cx: &ContentCx<'_>) -> Content;
pub(crate) fn is_thin(f: &source::Frame, track: (f64, f64)) -> bool;
// (et, pour group.rs : lose, body, plain_of, card, image, leaf, transform)
// crate::retrofit::group
pub(crate) const STACK: &str = "stack";
pub(crate) fn regular(frames: &[Frame]) -> bool;
pub(crate) fn group(items: &[Item], smartart: bool, id: &str, cx: &ContentCx<'_>, losses: &mut Vec<Loss>)
    -> (Option<Element>, Option<String>);
// crate::pattern
pub fn signature(items: &[Item]) -> String;
// crate::source (lot 1)
ItemKind::Group { items: Vec<Item>, smartart: bool }
// crate::test_support (cfg(test))
pub(crate) fn text_item(id: &str, frame: (f64, f64, f64, f64), text: &str) -> Item;
```

- Règles (`units`, doc de `cluster.rs`) :
  - **cartes** : une forme remplie (uni ou dégradé) sans texte qui contient au moins 90 % de la surface d'une ou plusieurs boîtes de texte devient une carte ; une boîte va sur la plus petite forme qui la contient ; une forme sans texte posée sur une carte (au moins 90 % de sa surface dedans, ou un filet sans surface dont les deux bouts y sont) y est absorbée ; les textes d'une carte sont rangés en ordre de lecture ;
  - **textes fusionnés** : les boîtes de texte restantes, en ordre de lecture, dont les cellules accrochées (`cell`) se chevauchent et qui se suivent, deviennent un seul `Unit::Text` ;
  - le reste est un `Unit::Item` ; les unités suivent l'ordre de dessin de leur premier item.
- Règles (`element`) : la table en tête de `content.rs` (Step 3). En plus :
  - carte : le texte le plus grand (plus grande taille de caractère) va en `title` s'il y en a plusieurs et qu'il dépasse strictement les autres ; le texte juste au-dessus du titre, d'un paragraphe d'au plus 80 caractères et plus petit, en `kicker` ; le reste, en ordre de lecture, en `body` (riche, dans les bornes, pertes sur la forme). Un seul texte : tout en `body` ;
  - fond et trait d'une `shape` projetés par `project_color` (fond uni, ou premier arrêt d'un dégradé) ; une couleur sans jeton → `ColorDropped` sur l'item ; forme dont ni fond ni trait n'ont de jeton → rien ; forme remplie d'une image → `image` ; filet → `line` ;
  - presets `ellipse`, `flowChartConnector`, `donut` → `ellipse` ; `line` et tout preset contenant `Connector` → `line` ; le reste (et un tracé libre) → `rect` ;
  - rotation, miroir (de l'item, du groupe, de la forme ou des textes d'une carte) et rognage d'image → `Transform`, une fois par item ;
  - toute perte est portée par l'id de l'item source qui la cause (un enfant de groupe porte les siennes), sans doublon.
- Règles (`group`, doc de `group.rs`) : groupes imbriqués aplatis ; une feuille avec texte → carte (premier paragraphe en `title` s'il y en a plusieurs, le reste en `body`) ; image, tableau, graphique gardent leur élément ; forme sans texte omise. Pile `columns` si les centres des enfants s'étalent plus en largeur qu'en hauteur, sinon `rows` ; ids `<id>-1`, `<id>-2`… Repli dit si une forme est omise, si c'est un SmartArt, ou si les feuilles gardées forment un motif régulier (`regular` : au moins trois boîtes de tailles à 25 % près, centrées sur une rangée ou une colonne à un quart de leur côté près, pas des centres à 25 % près).
- Règles (`signature`) : `<sorte>×<nombre>` triés par sorte et joints par `,`, puis `|`, puis chaque boîte ramenée à la boîte englobante de `items` et arrondie au quart (`x0 y0 x1 y1`, chiffres 0 à 4), triées et jointes par `,`. Indépendante de la position, de l'échelle et de l'ordre de dessin ; sensible aux sortes et à la disposition.

- [ ] **Step 1 : tests qui échouent**

Dans `crates/oku-slides-import/src/test_support.rs`, la doc et les imports deviennent :

```rust
//! Aides de test du projecteur : jetons effectifs des thèmes intégrés, texte, items,
//! slides et decks source, registre de base, application d'une slide par les ops du cœur.

use std::collections::BTreeMap;

use oku_slides_core::tokens::effective_tokens;
use oku_slides_core::{Deck, Element, Op, ParamValue, Slide, WidgetRegistry, apply_all};
use serde_json::{Map, Value};

use crate::retrofit::ColorTokens;
use crate::source::{
    Align, Bullet, Caps, Color, Fill, Frame, Item, ItemKind, ParaStyle, Paragraph, Rgba, Run, RunStyle, Size,
    SourceDeck, SourceSlide, Spacing, Styled, TextBox,
};
```

puis, après `item`, ajouter :

```rust
/// Une boîte de texte d'un paragraphe `text`.
pub(crate) fn text_item(id: &str, frame: (f64, f64, f64, f64), text: &str) -> Item {
    item(id, frame, ItemKind::Text(text_box(vec![para(vec![run(text)])])))
}
```

`crates/oku-slides-import/src/retrofit/cluster_tests.rs` :

```rust
use std::collections::BTreeMap;

use speculoos::prelude::*;

use super::*;
use crate::source::{self, Geometry, Rgba, Stroke, Styled};
use crate::test_support::{item, text_item};

fn shape(id: &str, frame: (f64, f64, f64, f64), fill: Fill, stroke: Option<Stroke>) -> Item {
    let s = source::Shape {
        geom: Geometry::Preset { name: "rect".into(), adj: BTreeMap::new() },
        fill,
        stroke,
        text: None,
    };
    item(id, frame, ItemKind::Shape(Box::new(s)))
}

fn filled(id: &str, frame: (f64, f64, f64, f64)) -> Item {
    shape(id, frame, Fill::Solid { color: Styled::explicit(Rgba::new(255, 255, 255)) }, None)
}

/// Une cellule par tranche de 100 pt en largeur et en hauteur.
fn cell(f: &Frame) -> Cell {
    let at = |v: f64| (v / 100.0).floor().max(0.0) as u32;
    let (col, row) = (at(f.x), at(f.y));
    Cell { col, row, cols: at(f.x + f.w - 0.01) - col + 1, rows: at(f.y + f.h - 0.01) - row + 1 }
}

fn ids(units: &[Unit<'_>]) -> Vec<Vec<String>> {
    units.iter().map(|u| u.items().iter().map(|i| i.id.clone()).collect()).collect()
}

/// Slide 2 d'alto : une carte = un fond blanc, un filet d'accent en haut, un titre, un corps.
#[test]
fn units_make_a_card_of_a_filled_shape_and_the_texts_on_it() {
    let rule = Stroke { width: 2.0, color: Styled::explicit(Rgba::new(0, 0xbe, 0xf4)), dash: source::Dash::Solid };
    let items = [
        filled("ground", (64.0, 176.0, 277.0, 284.0)),
        shape("rule", (64.0, 177.0, 277.0, 0.0), Fill::None, Some(rule)),
        text_item("title", (80.0, 196.0, 253.0, 18.0), "The only window"),
        text_item("body", (80.0, 220.0, 253.0, 69.0), "The agent becomes…"),
        text_item("outside", (64.0, 497.0, 270.0, 12.0), "Pied"),
    ];
    let refs: Vec<&Item> = items.iter().collect();
    let units = units(&refs, cell);
    assert_that!(ids(&units)).is_equal_to(vec![
        vec!["ground".to_owned(), "title".to_owned(), "body".to_owned(), "rule".to_owned()],
        vec!["outside".to_owned()],
    ]);
    assert_that!(matches!(units[0], Unit::Card { .. })).is_true();
}

/// Un texte posé sur deux fonds emboîtés va sur le plus petit ; un texte qui ne
/// recouvre un fond qu'à moitié reste seul.
#[test]
fn units_put_a_text_on_the_smallest_ground_that_holds_it() {
    let items = [
        filled("panel", (0.0, 0.0, 900.0, 500.0)),
        filled("card", (100.0, 100.0, 200.0, 200.0)),
        text_item("in-card", (110.0, 110.0, 180.0, 40.0), "Carte"),
        text_item("in-panel", (500.0, 100.0, 200.0, 40.0), "Panneau"),
        text_item("half", (250.0, 350.0, 100.0, 300.0), "À cheval"),
    ];
    let refs: Vec<&Item> = items.iter().collect();
    let units = units(&refs, cell);
    assert_that!(ids(&units)).is_equal_to(vec![
        vec!["panel".to_owned(), "in-panel".to_owned()],
        vec!["card".to_owned(), "in-card".to_owned()],
        vec!["half".to_owned()],
    ]);
}

#[test]
fn units_merge_neighbouring_texts_whose_cells_overlap() {
    let items = [
        text_item("a", (0.0, 410.0, 300.0, 15.0), "Distribution"),
        text_item("far", (0.0, 100.0, 300.0, 15.0), "Loin"),
        text_item("b", (0.0, 450.0, 300.0, 15.0), "Matthieu"),
        text_item("c", (500.0, 450.0, 300.0, 15.0), "Ailleurs"),
    ];
    let refs: Vec<&Item> = items.iter().collect();
    let units = units(&refs, cell);
    assert_that!(ids(&units)).is_equal_to(vec![
        vec!["a".to_owned(), "b".to_owned()],
        vec!["far".to_owned()],
        vec!["c".to_owned()],
    ]);
    assert_that!(units[0].frame()).is_equal_to(Frame::new(0.0, 410.0, 300.0, 55.0));
}

#[test]
fn units_leave_a_bare_shape_without_text_alone() {
    let items = [filled("a", (0.0, 0.0, 100.0, 100.0)), filled("b", (10.0, 10.0, 20.0, 20.0))];
    let refs: Vec<&Item> = items.iter().collect();
    assert_that!(ids(&units(&refs, cell))).is_equal_to(vec![vec!["a".to_owned()], vec!["b".to_owned()]]);
}
```

> Les cellules du test sont des tranches de 100 pt : `a` (y 410–425) et `b` (y 450–465) tombent dans la ligne 4, colonnes 0 à 2, et se suivent en ordre de lecture : fusionnés ; `c` (x 500) est dans les colonnes 5 à 7 : seul. `half` n'a que 50 % de sa surface sur `panel` : seul.

`crates/oku-slides-import/src/retrofit/content_tests.rs` :

```rust
use oku_slides_core::asset_id;
use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::report::RunEffect;
use crate::source::{Crop, Dash, Opaque, OpaqueReason, Picture, Rgba, Stroke, Styled};
use crate::test_support::{PAPIER, apply_blocks, colors, item, para, run, run_with, text_box, text_item};

const BOX: (f64, f64, f64, f64) = (10.0, 10.0, 200.0, 100.0);

fn png() -> String {
    asset_id(b"png")
}

fn emf() -> String {
    asset_id(b"emf")
}

fn assets() -> BTreeMap<String, Asset> {
    BTreeMap::from([
        (png(), Asset { mime: "image/png".into(), bytes: b"png".to_vec() }),
        (emf(), Asset { mime: "image/emf".into(), bytes: b"emf".to_vec() }),
    ])
}

/// Pistes d'une grille 12 × 6 de Papier titrée, ramenées en pt source (échelle 2).
const TRACK: (f64, f64) = (51.7, 40.8);

/// `unit` converti sur Papier, en élément `e1`.
fn convert_unit(unit: &Unit<'_>) -> (Option<Element>, Vec<Loss>, Option<String>) {
    let (tokens, assets) = (colors(PAPIER), assets());
    let c = element(unit, "e1".into(), &ContentCx { tokens: &tokens, assets: &assets, track: TRACK });
    (c.element, c.losses, c.fallback)
}

/// `item` seul converti sur Papier, en élément `e1`.
fn convert(item: &Item) -> (Option<Element>, Vec<Loss>, Option<String>) {
    convert_unit(&Unit::Item(item))
}

fn accent1() -> source::Color {
    Styled::mapped(Rgba::new(0x44, 0x72, 0xc4), "accent1", "accent1", Vec::new())
}

fn solid(color: source::Color) -> Fill {
    Fill::Solid { color }
}

fn shape_item(id: &str, geom: &str, fill: Fill, stroke: Option<Stroke>, text: Option<&str>) -> Item {
    let s = source::Shape {
        geom: Geometry::Preset { name: geom.into(), adj: BTreeMap::new() },
        fill,
        stroke,
        text: text.map(|t| text_box(vec![para(vec![run(t)])])),
    };
    item(id, BOX, ItemKind::Shape(Box::new(s)))
}

fn picture(asset: &str) -> Item {
    item("p", BOX, ItemKind::Picture(Picture { asset: asset.into(), crop: Crop::default() }))
}

fn texts(e: &Element, param: &str) -> Vec<String> {
    e.params.rich(param).unwrap_or_default().iter().map(|p| p.runs.iter().map(|r| r.t.as_str()).collect()).collect()
}

#[test]
fn element_of_a_text_box_is_a_text_element() {
    let (e, losses, fallback) = convert(&text_item("t", BOX, "Bonjour"));
    let e = e.unwrap();
    assert_that!((e.id.as_str(), e.kind.as_str())).is_equal_to(("e1", "text"));
    assert_that!(texts(&e, "body")).is_equal_to(vec!["Bonjour".to_owned()]);
    assert_that!(losses).is_empty();
    assert_that!(fallback).is_none();
}

#[test]
fn element_of_an_empty_text_box_is_nothing() {
    let (e, losses, _) = convert(&item("t", BOX, ItemKind::Text(text_box(vec![para(vec![])]))));
    assert_that!(e).is_none();
    assert_that!(losses).is_empty();
}

#[test]
fn element_reports_text_losses_on_the_item() {
    let tb = text_box(vec![para(vec![run_with("souligné", |s| s.underline = true)])]);
    let (_, losses, _) = convert(&item("t", BOX, ItemKind::Text(tb)));
    assert_that!(losses).is_equal_to(vec![Loss::on("t", LossKind::RunStyle(RunEffect::Underline))]);
}

#[test]
fn element_of_a_filled_shape_with_text_is_a_card() {
    let (e, _, _) =
        convert(&shape_item("s", "rect", solid(Styled::explicit(Rgba::new(0xf7, 0xf5, 0xf0))), None, Some("Note")));
    let e = e.unwrap();
    assert_that!(e.kind.as_str()).is_equal_to("card");
    assert_that!(texts(&e, "body")).is_equal_to(vec!["Note".to_owned()]);
    assert_that!(e.params.value("highlight")).is_none();
}

#[test]
fn element_of_a_shape_filled_with_the_accent_is_a_highlighted_card() {
    let (e, _, _) = convert(&shape_item("s", "roundRect", solid(accent1()), None, Some("Clé")));
    assert_that!(e.unwrap().params.value("highlight")).is_equal_to(Some(&Value::Bool(true)));
}

#[test]
fn element_of_an_outlined_shape_with_text_is_a_text_element() {
    let stroke = Stroke { width: 1.0, color: accent1(), dash: Dash::Solid };
    let (e, _, _) = convert(&shape_item("s", "rect", Fill::None, Some(stroke), Some("Encadré")));
    assert_that!(e.unwrap().kind.as_str()).is_equal_to("text");
}

#[rstest]
#[case("rect", "rect")]
#[case("roundRect", "rect")]
#[case("ellipse", "ellipse")]
#[case("line", "line")]
#[case("straightConnector1", "line")]
#[case("bentConnector3", "line")]
fn element_of_a_shape_without_text_maps_its_preset(#[case] preset: &str, #[case] expected: &str) {
    let (e, _, _) = convert(&shape_item("s", preset, solid(accent1()), None, None));
    let e = e.unwrap();
    assert_that!(e.kind.as_str()).is_equal_to("shape");
    assert_that!(e.params.text("shape")).is_equal_to(Some(expected));
}

#[test]
fn element_of_a_shape_projects_its_fill_and_stroke_on_tokens() {
    let stroke = Stroke { width: 2.0, color: Styled::scheme(Rgba::new(0, 0, 0), "dk1"), dash: Dash::Solid };
    let (e, losses, _) = convert(&shape_item("s", "rect", solid(accent1()), Some(stroke), None));
    let e = e.unwrap();
    assert_that!((e.style.fill.as_deref(), e.style.stroke.as_deref())).is_equal_to((Some("accent"), Some("ink")));
    assert_that!(losses).is_empty();
}

#[test]
fn element_of_a_shape_drops_a_fill_without_close_token() {
    let stroke = Stroke { width: 2.0, color: Styled::scheme(Rgba::new(0, 0, 0), "dk1"), dash: Dash::Solid };
    let red = solid(Styled::explicit(Rgba::new(255, 0, 0)));
    let (e, losses, _) = convert(&shape_item("s", "rect", red, Some(stroke), None));
    let e = e.unwrap();
    assert_that!((e.style.fill.as_deref(), e.style.stroke.as_deref())).is_equal_to((None, Some("ink")));
    assert_that!(losses).is_equal_to(vec![Loss::on("s", LossKind::ColorDropped("#ff0000".into()))]);
}

#[test]
fn element_of_an_invisible_shape_is_nothing() {
    assert_that!(convert(&shape_item("s", "rect", Fill::None, None, None)).0).is_none();
}

#[test]
fn element_of_a_shape_filled_with_a_picture_is_an_image() {
    let e = convert(&shape_item("s", "rect", Fill::Picture { asset: png() }, None, None)).0.unwrap();
    assert_that!((e.kind.as_str(), e.params.text("asset"))).is_equal_to(("image", Some(png().as_str())));
}

#[test]
fn element_of_a_picture_is_an_image_with_the_same_asset() {
    let e = convert(&picture(&png())).0.unwrap();
    assert_that!(e.kind.as_str()).is_equal_to("image");
    assert_that!(e.params.text("asset")).is_equal_to(Some(png().as_str()));
    assert_that!(e.params.text("fit")).is_equal_to(Some("contain"));
}

#[test]
fn element_drops_an_image_the_renderer_cannot_draw() {
    let (e, losses, _) = convert(&picture(&emf()));
    assert_that!(e).is_none();
    assert_that!(losses).is_equal_to(vec![Loss::on("p", LossKind::UnsupportedImage("image/emf".into()))]);
}

#[test]
fn element_of_a_picture_without_asset_is_nothing() {
    assert_that!(convert(&picture("sha256:absent")).0).is_none();
}

#[test]
fn element_reports_crop_rotation_and_flip_as_one_transform_loss() {
    let mut p = picture(&png());
    p.frame.rot = 90.0;
    p.frame.flip_h = true;
    if let ItemKind::Picture(pic) = &mut p.kind {
        pic.crop = Crop { l: 0.1, ..Crop::default() };
    }
    let (e, losses, _) = convert(&p);
    assert_that!(e).is_some();
    assert_that!(losses).is_equal_to(vec![Loss::on("p", LossKind::Transform)]);
}

#[test]
fn element_of_a_table_is_a_table_element() {
    let cell = source::Cell {
        text: text_box(vec![para(vec![run("A")])]),
        fill: Fill::None,
        borders: source::Borders::default(),
        row_span: 1,
        col_span: 1,
        covered: false,
    };
    let t = source::Table {
        col_widths: vec![100.0],
        row_heights: vec![20.0],
        flags: source::TableFlags::default(),
        rows: vec![vec![cell]],
    };
    let e = convert(&item("t", BOX, ItemKind::Table(t))).0.unwrap();
    assert_that!(e.kind.as_str()).is_equal_to("table");
    assert_that!(e.params.value("data").map(|d| d["dataset"]["source"].clone()))
        .is_equal_to(Some(serde_json::json!([["A"]])));
}

#[test]
fn element_of_a_chart_is_a_chart_element_with_its_losses() {
    let c = source::Chart {
        kind: source::ChartKind::Column,
        grouping: source::Grouping::PercentStacked,
        title: None,
        categories: vec!["a".into()],
        series: vec![source::Series { name: "S".into(), values: vec![Some(1.0)], color: None }],
    };
    let (e, losses, _) = convert(&item("c", BOX, ItemKind::Chart(c)));
    assert_that!(e.unwrap().kind.as_str()).is_equal_to("chart");
    assert_that!(losses).is_equal_to(vec![Loss::on("c", LossKind::PercentStacked)]);
}

#[rstest]
#[case(Some(png()), true)]
#[case(None, false)]
fn element_of_an_opaque_item_uses_its_fallback_image_and_reports_it(
    #[case] fallback: Option<String>,
    #[case] image: bool,
) {
    let o = item("o", BOX, ItemKind::Opaque(Opaque { reason: OpaqueReason::Ole, fallback }));
    let (e, losses, _) = convert(&o);
    assert_that!(e.map(|e| e.kind)).is_equal_to(image.then(|| "image".to_owned()));
    assert_that!(losses).is_equal_to(vec![Loss::on("o", LossKind::Opaque(OpaqueReason::Ole))]);
}

#[test]
fn element_of_a_shape_whose_fill_and_stroke_are_dropped_is_nothing() {
    let red = || Styled::explicit(Rgba::new(255, 0, 0));
    let stroke = Stroke { width: 1.0, color: red(), dash: Dash::Solid };
    let (e, losses, _) = convert(&shape_item("s", "rect", solid(red()), Some(stroke), None));
    assert_that!(e).is_none();
    assert_that!(losses).is_equal_to(vec![Loss::on("s", LossKind::ColorDropped("#ff0000".into()))]);
}

/// Un filet de 200 × 2 pt : moins d'un quart d'une ligne de grille (40,8 pt) ;
/// son fond `accent1` devient le trait d'une ligne.
#[test]
fn element_of_a_thin_shape_is_a_line_stroked_with_its_fill() {
    let mut bar = shape_item("s", "rect", solid(accent1()), None, None);
    bar.frame.h = 2.0;
    let e = convert(&bar).0.unwrap();
    assert_that!((e.params.text("shape"), e.style.stroke.as_deref(), e.style.fill.as_deref())).is_equal_to((
        Some("line"),
        Some("accent"),
        None,
    ));
}

fn sized(id: &str, frame: (f64, f64, f64, f64), text: &str, size: f64) -> Item {
    item(id, frame, ItemKind::Text(text_box(vec![para(vec![run_with(text, |s| s.size = size)])])))
}

/// Une carte d'alto (slide 4) : fond blanc, surtitre 9,5 pt, titre 13 pt, sous-titre 9,5 pt, corps 10,5 pt.
fn pillar() -> (Item, Vec<Item>) {
    let ground = shape_item("c", "roundRect", solid(Styled::explicit(Rgba::new(255, 255, 255))), None, None);
    let texts = vec![
        sized("k", (16.0, 18.0, 180.0, 15.0), "PILLAR 1 — PRODUCT", 9.5),
        sized("t", (16.0, 39.0, 180.0, 18.0), "Amundi Funds", 13.0),
        sized("s", (16.0, 63.0, 180.0, 15.0), "Investment products", 9.5),
        sized("b", (16.0, 84.0, 180.0, 52.0), "A manufacturing capability", 10.5),
    ];
    (ground, texts)
}

#[test]
fn element_of_a_card_unit_takes_its_kicker_title_and_body() {
    let (ground, texts) = pillar();
    let unit = Unit::Card { shape: &ground, texts: texts.iter().collect(), inner: Vec::new() };
    let e = convert_unit(&unit).0.unwrap();
    assert_that!((e.kind.as_str(), e.params.text("kicker"), e.params.text("title"))).is_equal_to((
        "card",
        Some("PILLAR 1 — PRODUCT"),
        Some("Amundi Funds"),
    ));
    assert_that!(texts_of(&e))
        .is_equal_to(vec!["Investment products".to_owned(), "A manufacturing capability".to_owned()]);
    assert_that!(e.params.value("highlight")).is_none();
}

fn texts_of(e: &Element) -> Vec<String> {
    texts(e, "body")
}

#[test]
fn element_of_a_card_unit_with_a_single_text_keeps_it_as_body() {
    let (ground, texts) = pillar();
    let unit = Unit::Card { shape: &ground, texts: vec![&texts[1]], inner: Vec::new() };
    let e = convert_unit(&unit).0.unwrap();
    assert_that!((e.params.text("title"), texts_of(&e))).is_equal_to((None, vec!["Amundi Funds".to_owned()]));
}

/// Fond `accent1` → `accent` (`#3f6b5e`), 5,9:1 sur le fond d'une carte : relief.
/// Le gris pâle `#b9c4d6` (→ `rule`) se lit sur le relief `ink` (10,1:1) : émis.
#[test]
fn element_of_a_dark_card_unit_is_highlighted_and_colors_against_it() {
    let ground = shape_item("c", "roundRect", solid(accent1()), None, None);
    let pale = item(
        "b",
        (16.0, 40.0, 180.0, 40.0),
        ItemKind::Text(text_box(vec![para(vec![run_with("Pâle", |s| {
            s.color = Styled::explicit(Rgba::new(0xb9, 0xc4, 0xd6));
        })])])),
    );
    let e = convert_unit(&Unit::Card { shape: &ground, texts: vec![&pale], inner: Vec::new() }).0.unwrap();
    assert_that!(e.params.value("highlight")).is_equal_to(Some(&Value::Bool(true)));
    let color = e.params.rich("body").unwrap()[0].runs[0].color.clone();
    assert_that!(color.as_deref()).is_equal_to(Some("rule"));
}

#[test]
fn element_of_merged_texts_puts_their_paragraphs_end_to_end() {
    let (a, b) = (text_item("a", BOX, "Distribution"), text_item("b", BOX, "Matthieu"));
    let e = convert_unit(&Unit::Text(vec![&a, &b])).0.unwrap();
    assert_that!((e.kind.as_str(), texts_of(&e)))
        .is_equal_to(("text", vec!["Distribution".to_owned(), "Matthieu".to_owned()]));
}

#[test]
fn element_of_every_kind_is_accepted_by_add_slide() {
    let items = [
        text_item("t", BOX, "Texte"),
        shape_item("s1", "rect", solid(accent1()), None, Some("Carte")),
        shape_item("s2", "ellipse", solid(accent1()), None, None),
        picture(&png()),
    ];
    let (ground, texts) = pillar();
    let (tokens, assets) = (colors(PAPIER), assets());
    let cx = ContentCx { tokens: &tokens, assets: &assets, track: TRACK };
    let mut units: Vec<Unit<'_>> = items.iter().map(Unit::Item).collect();
    units.push(Unit::Card { shape: &ground, texts: texts.iter().collect(), inner: Vec::new() });
    let blocks: Vec<Element> =
        units.iter().enumerate().filter_map(|(i, u)| element(u, format!("e{i}"), &cx).element).collect();
    assert_that!(blocks).has_length(units.len());
    assert_that!(apply_blocks(blocks)).is_ok();
}
```

> `TRACK` : une colonne et une ligne de la grille 12 × 6 de Papier titrée (`(1680 − 11 × 40) / 12 = 103,3` et `(690 − 5 × 40) / 6 = 81,7` pt de page), ramenées en pt source par l'échelle 2. Un filet de 200 × 2 pt : moins d'un quart de 40,8. `element_of_a_shape_projects_its_fill_and_stroke_on_tokens` : `accent1` sans modificateur → `accent` par la table du schéma ; `dk1` → `ink`. `element_of_a_filled_shape_with_text_is_a_card` : `#f7f5f0` est le `surface` de Papier (contraste 1,1 avec `bg`) : pas de relief. `accent1` → `accent` (`#3f6b5e`), 5,9:1 sur `bg` : relief ; le gris pâle `#b9c4d6` → `rule`, 10,1:1 sur le relief `ink`.

`crates/oku-slides-import/src/retrofit/group_tests.rs` :

```rust
use std::collections::BTreeMap;

use oku_slides_core::{Element, Paragraph};
use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::retrofit::content::ContentCx;
use crate::source::{self, Fill, Geometry, Rgba, Styled};
use crate::test_support::{PAPIER, colors, item, para, run, text_box, text_item};

/// `items` repliés sur Papier, sous l'id `e1`.
fn fold(items: &[Item], smartart: bool) -> (Option<Element>, Vec<Loss>, Option<String>) {
    let (tokens, assets) = (colors(PAPIER), BTreeMap::new());
    let cx = ContentCx { tokens: &tokens, assets: &assets, track: (0.0, 0.0) };
    let mut losses = Vec::new();
    let (e, fallback) = group(items, smartart, "e1", &cx, &mut losses);
    (e, losses, fallback)
}

fn accent1() -> source::Color {
    Styled::mapped(Rgba::new(0x44, 0x72, 0xc4), "accent1", "accent1", Vec::new())
}

fn bare(id: &str, preset: &str, frame: (f64, f64, f64, f64)) -> Item {
    let s = source::Shape {
        geom: Geometry::Preset { name: preset.into(), adj: BTreeMap::new() },
        fill: Fill::Solid { color: accent1() },
        stroke: None,
        text: None,
    };
    item(id, frame, ItemKind::Shape(Box::new(s)))
}

fn body_texts(e: &Element) -> Vec<String> {
    let ps: &[Paragraph] = e.params.rich("body").unwrap_or_default();
    ps.iter().map(|p| p.runs.iter().map(|r| r.t.as_str()).collect()).collect()
}

/// Trois boîtes côte à côte (une étape, un titre et sa description chacune), une flèche,
/// la troisième étape dans un sous-groupe.
fn steps() -> Vec<Item> {
    let step = |i: usize| {
        let tb = text_box(vec![para(vec![run(&format!("Étape {i}"))]), para(vec![run(&format!("Détail {i}"))])]);
        let s = source::Shape {
            geom: Geometry::Preset { name: "roundRect".into(), adj: BTreeMap::new() },
            fill: Fill::Solid { color: accent1() },
            stroke: None,
            text: Some(tb),
        };
        item(&format!("g/{i}"), (i as f64 * 300.0, 0.0, 250.0, 100.0), ItemKind::Shape(Box::new(s)))
    };
    let nested = item("g/sub", (900.0, 0.0, 250.0, 100.0), ItemKind::Group { items: vec![step(3)], smartart: false });
    vec![step(1), bare("g/arrow", "rightArrow", (260.0, 40.0, 30.0, 20.0)), step(2), nested]
}

#[test]
fn group_falls_back_to_a_stack_of_cards() {
    let (e, losses, fallback) = fold(&steps(), false);
    let e = e.unwrap();
    assert_that!((e.kind.as_str(), e.params.text("layout"))).is_equal_to(("stack", Some("columns")));
    let children = e.params.slot("children").unwrap();
    assert_that!(children.iter().map(|c| (c.id.as_str(), c.kind.as_str())).collect::<Vec<_>>()).is_equal_to(vec![
        ("e1-1", "card"),
        ("e1-2", "card"),
        ("e1-3", "card"),
    ]);
    assert_that!(children[0].params.text("title")).is_equal_to(Some("Étape 1"));
    assert_that!(body_texts(&children[2])).is_equal_to(vec!["Détail 3".to_owned()]);
    assert_that!(fallback.as_deref()).is_equal_to(Some("stack"));
    assert_that!(losses).is_empty();
}

#[test]
fn group_of_two_stacked_texts_is_a_silent_row_stack() {
    let items = vec![text_item("a", (0.0, 0.0, 300.0, 50.0), "A"), text_item("b", (0.0, 200.0, 300.0, 50.0), "B")];
    let (e, _, fallback) = fold(&items, false);
    let e = e.unwrap();
    assert_that!(e.params.text("layout")).is_equal_to(Some("rows"));
    assert_that!(e.params.slot("children").unwrap()[0].params.text("title")).is_none();
    assert_that!(fallback).is_none();
}

#[test]
fn group_of_a_smartart_is_always_a_fallback() {
    let items = vec![text_item("a", (0.0, 0.0, 300.0, 50.0), "A"), text_item("b", (0.0, 200.0, 300.0, 50.0), "B")];
    assert_that!(fold(&items, true).2.as_deref()).is_equal_to(Some("stack"));
}

#[test]
fn group_of_bare_shapes_is_nothing() {
    assert_that!(fold(&[bare("a", "rect", (0.0, 0.0, 10.0, 10.0))], false)).is_equal_to((None, Vec::new(), None));
}

fn boxes(frames: &[(f64, f64, f64, f64)]) -> Vec<Frame> {
    frames.iter().map(|&(x, y, w, h)| Frame::new(x, y, w, h)).collect()
}

/// Pas des centres : 300 et 300 pour la rangée régulière ; 300 et 600 pour l'irrégulière (0,5 < 0,75).
#[rstest]
#[case::row(boxes(&[(0.0, 0.0, 250.0, 100.0), (300.0, 0.0, 250.0, 100.0), (600.0, 0.0, 250.0, 100.0)]), true)]
#[case::column(boxes(&[(0.0, 0.0, 250.0, 100.0), (0.0, 150.0, 250.0, 100.0), (0.0, 300.0, 250.0, 100.0)]), true)]
#[case::two(boxes(&[(0.0, 0.0, 250.0, 100.0), (300.0, 0.0, 250.0, 100.0)]), false)]
#[case::uneven_steps(boxes(&[(0.0, 0.0, 250.0, 100.0), (300.0, 0.0, 250.0, 100.0), (900.0, 0.0, 250.0, 100.0)]), false)]
#[case::uneven_sizes(boxes(&[(0.0, 0.0, 250.0, 100.0), (300.0, 0.0, 100.0, 100.0), (600.0, 0.0, 250.0, 100.0)]), false)]
#[case::scattered(boxes(&[(0.0, 0.0, 250.0, 100.0), (300.0, 200.0, 250.0, 100.0), (600.0, 0.0, 250.0, 100.0)]), false)]
fn regular_finds_rows_and_columns_of_similar_evenly_spaced_boxes(#[case] frames: Vec<Frame>, #[case] expected: bool) {
    assert_that!(regular(&frames)).is_equal_to(expected);
}
```

> Dans `steps()`, la flèche (`rightArrow` sans texte) est omise et l'étape 3, dans un sous-groupe, est aplatie : trois cartes `e1-1` à `e1-3`, centres à x = 425, 725 et 1 025, tous à y = 50 : colonnes ; la flèche omise fait dire le repli. Deux boîtes empilées, rien d'omis : repli muet.

`crates/oku-slides-import/src/pattern_tests.rs` :

```rust
use speculoos::prelude::*;

use super::*;
use crate::source::{Fill, Geometry, Shape};
use crate::test_support::{item, text_item};

fn rect(id: &str, frame: (f64, f64, f64, f64)) -> Item {
    let s = Shape {
        geom: Geometry::Preset { name: "rect".into(), adj: BTreeMap::new() },
        fill: Fill::None,
        stroke: None,
        text: None,
    };
    item(id, frame, ItemKind::Shape(Box::new(s)))
}

/// Trois formes de 200 × 100 à x = 0, 300, 600, décalées de `(dx, dy)` et agrandies de `k`.
fn row(dx: f64, dy: f64, k: f64) -> Vec<Item> {
    (0..3).map(|i| rect(&format!("r{i}"), (dx + k * 300.0 * f64::from(i), dy, k * 200.0, k * 100.0))).collect()
}

#[test]
fn signature_counts_kinds_and_normalizes_boxes() {
    assert_that!(signature(&row(0.0, 0.0, 1.0))).is_equal_to("shape×3|0014,2034,3044".to_owned());
}

#[test]
fn signature_ignores_translation_and_scale() {
    assert_that!(signature(&row(500.0, 80.0, 2.5))).is_equal_to(signature(&row(0.0, 0.0, 1.0)));
}

#[test]
fn signature_ignores_drawing_order() {
    let mut items = row(0.0, 0.0, 1.0);
    items.reverse();
    assert_that!(signature(&items)).is_equal_to(signature(&row(0.0, 0.0, 1.0)));
}

#[test]
fn signature_tells_a_row_from_a_column() {
    let column: Vec<Item> = (0..3).map(|i| rect(&format!("c{i}"), (0.0, 300.0 * f64::from(i), 200.0, 100.0))).collect();
    assert_that!(signature(&column)).is_not_equal_to(signature(&row(0.0, 0.0, 1.0)));
}

#[test]
fn signature_tells_kinds_apart() {
    let mut items = row(0.0, 0.0, 1.0);
    items[1] = text_item("t", (300.0, 0.0, 200.0, 100.0), "Texte");
    assert_that!(signature(&items)).is_equal_to("shape×2,text×1|0014,2034,3044".to_owned());
}

#[test]
fn signature_of_one_item_or_none_is_stable() {
    assert_that!(signature(&[rect("a", (5.0, 5.0, 10.0, 10.0))])).is_equal_to("shape×1|0044".to_owned());
    assert_that!(signature(&[rect("a", (5.0, 5.0, 0.0, 0.0))])).is_equal_to("shape×1|0000".to_owned());
    assert_that!(signature(&[])).is_equal_to("|".to_owned());
}
```

> `row(0, 0, 1)` : boîte englobante 800 × 100 ; la forme 2 va de x = 300 à 500 : `300 / 800 × 4 = 1,5` → 2 et `500 / 800 × 4 = 2,5` → 3 (arrondi au plus loin de zéro), d'où `2034`.

Lot 1, `crates/oku-slides-import/src/pptx/smartart_tests.rs` : `ItemKind::Group { items }` et `ItemKind::Group { items: shapes }` deviennent `ItemKind::Group { items, .. }` et `ItemKind::Group { items: shapes, .. }`, et avant `smartart_shapes_are_placed_relative_to_the_frame` :

```rust
#[test]
fn smartart_group_is_flagged_as_smartart() {
    let (items, _, _) = tree(FRAME);
    assert_that!(matches!(items[0].item.kind, ItemKind::Group { smartart: true, .. })).is_true();
}
```

Lot 1, `crates/oku-slides-import/src/pptx/shapes_tests.rs` : dans `kids`, `ItemKind::Group { items }` devient `ItemKind::Group { items, .. }` ; dans `nested_groups_compose_their_transforms`, la déstructuration devient :

```rust
    let ItemKind::Group { items: leaves, smartart } = &kids(&items)[0].kind else { panic!("groupe attendu") };
    assert_that!(*smartart).is_false();
```

Lot 1, `crates/oku-slides-import/src/source/mod_tests.rs` : `ItemKind::Group { items: Vec::new() }` devient `ItemKind::Group { items: Vec::new(), smartart: false }` ; `crates/oku-slides-import/tests/kitchen.rs` : `ItemKind::Group { items } =>` devient `ItemKind::Group { items, .. } =>`.

`crates/oku-slides-import/src/retrofit/mod.rs` complet :

```rust
//! Projecteur retrofit : une [`SourceDeck`](crate::source::SourceDeck) projetée sur
//! un pack et un thème existants.
//!
//! Le style vient du thème cible : couleurs ramenées à ses jetons ([`ColorTokens`]),
//! texte riche réduit à ce que le cœur exprime, éléments libres placés sur la
//! grille d'une zone de corps approchée par ses jetons de page ([`PageTokens`]).

mod chart;
#[allow(dead_code, reason = "branché par le projecteur, tâche 9")]
mod cluster;
mod color;
#[allow(dead_code, reason = "branché par le projecteur, tâche 9")]
mod content;
#[allow(dead_code, reason = "branché par le projecteur, tâche 9")]
mod group;
mod table;
mod text;
#[allow(dead_code, reason = "branché par le projecteur, tâche 9")]
mod zone;

pub use color::ColorTokens;
pub use zone::PageTokens;
```

`crates/oku-slides-import/src/lib.rs` complet :

```rust
//! Import d'une présentation PowerPoint en représentation intermédiaire.
//!
//! [`read_pptx`] lit les octets d'un `.pptx` et rend une [`source::SourceDeck`] :
//! géométrie en pt, styles texte résolus, couleurs avec leur provenance,
//! tableaux, graphiques, SmartArt, replis, polices. Bornes : [`ReadLimits`] ;
//! erreurs : [`ImportError`].
//!
//! Un projecteur rend un [`ImportPlan`] : les opérations qui construisent le
//! deck Oku, les assets qu'elles référencent et un [`ImportReport`] (pertes,
//! replis, couverture du contenu).

pub mod plan;
pub mod report;
pub mod retrofit;
pub mod source;

mod collect;
mod error;
mod limits;
#[allow(dead_code, reason = "branché par le projecteur, tâche 9")]
mod pattern;
mod pptx;
#[cfg(test)]
mod test_support;

pub use error::{ImportError, ImportErrorKind};
pub use limits::ReadLimits;
pub use plan::ImportPlan;
pub use pptx::read_pptx;
pub use report::ImportReport;
```

- [ ] **Step 2 : lancer, constater l'échec**

```bash
export CARGO_TARGET_DIR=$PWD/target
cargo nextest run -p oku-slides-import retrofit::content retrofit::cluster retrofit::group pattern:: pptx::
```

Attendu : échec de compilation (`retrofit/cluster.rs`, `content.rs`, `group.rs` et `pattern.rs` absents ; `ItemKind::Group` n'a pas de champ `smartart`).

- [ ] **Step 3 : implémentation**

Lot 1, `crates/oku-slides-import/src/source/mod.rs`, la variante `Group` de `ItemKind` :

```rust
    /// Groupe (ou SmartArt dessiné).
    Group {
        /// Enfants, dans l'ordre de dessin.
        items: Vec<Item>,
        /// Version dessinée d'un SmartArt (`dsp:drawing`), plutôt qu'un groupe de l'auteur.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        smartart: bool,
    },
```

Lot 1, `crates/oku-slides-import/src/pptx/smartart.rs` (fin de `smartart`) : `ItemKind::Group { items }` devient `ItemKind::Group { items, smartart: true }` ; `crates/oku-slides-import/src/pptx/shapes.rs` (`parse_group`) : `kind: ItemKind::Group { items }` devient `kind: ItemKind::Group { items, smartart: false }` (rustfmt range alors l'`Item` sur plusieurs lignes) ; `crates/oku-slides-import/src/pptx/fonts.rs` : `ItemKind::Group { items: kids }` devient `ItemKind::Group { items: kids, .. }`.

`crates/oku-slides-import/src/retrofit/cluster.rs` :

```rust
//! Regroupement des items d'une slide en unités de contenu, avant le choix
//! du modèle.
//!
//! Une diapositive dessinée à la main empile des formes et des boîtes de
//! texte : un fond arrondi, un filet d'accent, un titre et un paragraphe
//! posés dessus forment une carte. Deux règles, dans l'ordre :
//!
//! 1. **Cartes.** Une forme remplie sans texte qui contient au moins 90 % de
//!    la surface d'une ou plusieurs boîtes de texte devient une carte (la plus
//!    petite forme qui la contient, si plusieurs). Les formes sans texte
//!    posées dans la carte (filets, barres d'accent) y sont absorbées.
//! 2. **Textes fusionnés.** Les boîtes de texte restantes, prises en ordre de
//!    lecture, dont les cellules accrochées se chevauchent et qui se suivent,
//!    deviennent un seul texte (paragraphes mis bout à bout).
//!
//! Chaque autre item reste une unité à lui seul.

use oku_slides_core::Cell;

use crate::source::{Fill, Frame, Item, ItemKind};

/// Part minimale de la surface d'une boîte de texte couverte par une forme pour qu'elle y soit posée.
const INSIDE: f64 = 0.9;

/// Une unité de contenu : ce qui deviendra un élément.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Unit<'a> {
    /// Un item seul.
    Item(&'a Item),
    /// Une carte : sa forme, ses textes (ordre de lecture), ses formes absorbées.
    Card {
        /// La forme remplie.
        shape: &'a Item,
        /// Les boîtes de texte posées dessus, en ordre de lecture.
        texts: Vec<&'a Item>,
        /// Les formes sans texte posées dessus.
        inner: Vec<&'a Item>,
    },
    /// Des boîtes de texte fusionnées, en ordre de lecture (au moins deux).
    Text(Vec<&'a Item>),
}

impl<'a> Unit<'a> {
    /// La boîte de l'unité : celle de la forme d'une carte, l'englobante de textes fusionnés.
    pub(crate) fn frame(&self) -> Frame {
        match self {
            Unit::Item(i) | Unit::Card { shape: i, .. } => i.frame,
            Unit::Text(items) => bounds(items.iter().map(|i| &i.frame)),
        }
    }

    /// Les items source de l'unité.
    pub(crate) fn items(&self) -> Vec<&'a Item> {
        match self {
            Unit::Item(i) => vec![i],
            Unit::Card { shape, texts, inner } => {
                [*shape].into_iter().chain(texts.iter().copied()).chain(inner.iter().copied()).collect()
            }
            Unit::Text(items) => items.clone(),
        }
    }

    /// L'item qui nomme l'unité : la forme d'une carte, le premier texte fusionné.
    pub(crate) fn lead(&self) -> &'a Item {
        match self {
            Unit::Item(i) | Unit::Card { shape: i, .. } => i,
            Unit::Text(items) => items[0],
        }
    }
}

/// La boîte englobante de `frames` (vide : boîte nulle à l'origine).
pub(crate) fn bounds<'f>(frames: impl Iterator<Item = &'f Frame>) -> Frame {
    let mut acc: Option<(f64, f64, f64, f64)> = None;
    for f in frames {
        let (x0, y0, x1, y1) = acc.unwrap_or((f.x, f.y, f.x + f.w, f.y + f.h));
        acc = Some((x0.min(f.x), y0.min(f.y), x1.max(f.x + f.w), y1.max(f.y + f.h)));
    }
    let (x0, y0, x1, y1) = acc.unwrap_or_default();
    Frame::new(x0, y0, x1 - x0, y1 - y0)
}

/// Surface de l'intersection de deux boîtes.
fn overlap(a: &Frame, b: &Frame) -> f64 {
    let w = (a.x + a.w).min(b.x + b.w) - a.x.max(b.x);
    let h = (a.y + a.h).min(b.y + b.h) - a.y.max(b.y);
    w.max(0.0) * h.max(0.0)
}

/// `inner` est-elle posée sur `outer` ? Au moins 90 % de sa surface dedans ;
/// un filet sans surface, si ses deux extrémités y sont.
fn inside(inner: &Frame, outer: &Frame) -> bool {
    let area = inner.w * inner.h;
    if area > 0.0 {
        return overlap(inner, outer) >= INSIDE * area;
    }
    let within = |x: f64, y: f64| x >= outer.x && x <= outer.x + outer.w && y >= outer.y && y <= outer.y + outer.h;
    within(inner.x, inner.y) && within(inner.x + inner.w, inner.y + inner.h)
}

/// Une forme remplie sans texte : un fond possible de carte.
fn is_ground(item: &Item) -> bool {
    matches!(&item.kind, ItemKind::Shape(s) if !item.has_text() && matches!(s.fill, Fill::Solid { .. } | Fill::Gradient { .. }))
}

/// Une forme sans texte (fond, filet, décor).
fn is_bare_shape(item: &Item) -> bool {
    matches!(item.kind, ItemKind::Shape(_)) && !item.has_text()
}

/// Une boîte de texte (pas une forme) qui porte du texte.
fn is_text(item: &Item) -> bool {
    matches!(item.kind, ItemKind::Text(_)) && item.has_text()
}

/// Ordre de lecture : de haut en bas, puis de gauche à droite.
pub(crate) fn reading(a: &Frame, b: &Frame) -> std::cmp::Ordering {
    a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x))
}

/// Deux cellules se chevauchent-elles ?
fn cells_meet(a: &Cell, b: &Cell) -> bool {
    a.col < b.col + b.cols && b.col < a.col + a.cols && a.row < b.row + b.rows && b.row < a.row + a.rows
}

/// Les unités de `items` (ordre de dessin), cartes puis textes fusionnés ;
/// `cell` accroche une boîte source à la grille de référence de la slide.
///
/// Les unités suivent l'ordre de dessin de leur premier item.
pub(crate) fn units<'a>(items: &[&'a Item], cell: impl Fn(&Frame) -> Cell) -> Vec<Unit<'a>> {
    let area = |i: &Item| i.frame.w * i.frame.h;
    // Chaque texte va sur la plus petite forme qui le porte.
    let ground = |t: &Item| {
        items
            .iter()
            .enumerate()
            .filter(|(_, g)| is_ground(g) && inside(&t.frame, &g.frame))
            .min_by(|(_, a), (_, b)| area(a).total_cmp(&area(b)))
            .map(|(k, _)| k)
    };
    let mut owner: Vec<Option<usize>> = items.iter().map(|i| if is_text(i) { ground(i) } else { None }).collect();
    let cards: Vec<usize> = (0..items.len()).filter(|k| owner.contains(&Some(*k))).collect();
    for (k, i) in items.iter().enumerate() {
        if owner[k].is_none() && !cards.contains(&k) && is_bare_shape(i) {
            owner[k] = cards
                .iter()
                .filter(|c| inside(&i.frame, &items[**c].frame))
                .min_by(|a, b| area(items[**a]).total_cmp(&area(items[**b])))
                .copied();
        }
    }
    // Textes restants, en ordre de lecture : fusion des voisins dont les cellules se chevauchent.
    let mut loose: Vec<usize> = (0..items.len()).filter(|k| owner[*k].is_none() && is_text(items[*k])).collect();
    loose.sort_by(|a, b| reading(&items[*a].frame, &items[*b].frame));
    let mut runs: Vec<Vec<usize>> = Vec::new();
    for k in loose {
        match runs.last_mut() {
            Some(run) if run.last().is_some_and(|p| cells_meet(&cell(&items[*p].frame), &cell(&items[k].frame))) => {
                run.push(k);
            }
            _ => runs.push(vec![k]),
        }
    }
    let mut out = Vec::new();
    for (k, item) in items.iter().enumerate() {
        if cards.contains(&k) {
            let mut texts: Vec<&Item> =
                (0..items.len()).filter(|t| owner[*t] == Some(k) && is_text(items[*t])).map(|t| items[t]).collect();
            texts.sort_by(|a, b| reading(&a.frame, &b.frame));
            let inner =
                (0..items.len()).filter(|t| owner[*t] == Some(k) && !is_text(items[*t])).map(|t| items[t]).collect();
            out.push(Unit::Card { shape: item, texts, inner });
        } else if owner[k].is_some() {
            continue;
        } else if let Some(run) = runs.iter().find(|r| r.first() == Some(&k)) {
            out.push(match run.as_slice() {
                [single] => Unit::Item(items[*single]),
                _ => Unit::Text(run.iter().map(|t| items[*t]).collect()),
            });
        } else if !runs.iter().any(|r| r.contains(&k)) {
            out.push(Unit::Item(item));
        }
    }
    out
}

#[cfg(test)]
#[path = "cluster_tests.rs"]
mod cluster_tests;
```

`crates/oku-slides-import/src/retrofit/content.rs` :

```rust
//! Unité de contenu ([`Unit`]) → élément Oku, et replis des motifs sans widget.
//!
//! | unité | élément |
//! |---|---|
//! | carte (forme remplie portant des textes) | `card` : le plus grand texte en `title`, un court texte au-dessus en `kicker`, le reste en `body` |
//! | textes fusionnés | `text` (paragraphes mis bout à bout) |
//! | texte | `text` (corps riche) |
//! | forme remplie avec texte | `card` (corps riche) |
//! | forme non remplie avec texte | `text` |
//! | forme sans texte, remplie d'une image | `image` |
//! | filet (forme plus fine qu'un quart de piste) | `shape: line`, trait projeté (son trait, sinon son fond) |
//! | forme sans texte | `shape` (`rect`, `ellipse` ou `line` selon le preset), fond et trait projetés |
//! | image | `image` (`fit: contain`), si le rendu sait la dessiner |
//! | tableau | `table` |
//! | graphique | `chart` |
//! | groupe, SmartArt dessiné | `stack` de `card` (une par enfant avec texte) et d'éléments |
//! | intraduisible avec image de repli | `image`, perte `Opaque` |
//! | intraduisible sans repli | rien, perte `Opaque` |
//!
//! Les textes d'une carte ou des textes fusionnés sont mis bout à bout dans
//! les bornes du texte riche ; leurs pertes vont sur la forme de la carte ou
//! le premier texte. Une carte est en relief (`highlight`) quand son fond se projette sur un
//! jeton qui contraste (≥ 3:1) avec le fond d'une carte du thème et que le
//! thème a un jeton de texte clair pour le relief ; sinon son fond est
//! abandonné. Ses textes sont colorés contre le fond de la carte rendue.
//! Rotation, miroir et rognage sont perdus (`Transform`).

use std::collections::BTreeMap;

use oku_slides_core::{Element, Paragraph, ParamValue};
use serde_json::Value;

use super::chart::chart;
use super::cluster::Unit;
use super::color::{ColorTokens, MIN_CONTRAST, contrast, project_color};
use super::group::group;
use super::table::table;
use super::text::{note, plain, rich};
use crate::report::{Loss, LossKind};
use crate::source::{self, Asset, Fill, Geometry, Item, ItemKind, TextBox};

/// Types MIME que le rendu sait dessiner.
const DRAWABLE: [&str; 5] = ["image/png", "image/jpeg", "image/gif", "image/webp", "image/svg+xml"];
/// Presets dessinés en ellipse.
const ELLIPSES: [&str; 3] = ["ellipse", "flowChartConnector", "donut"];
/// Longueur maximale d'un surtitre de carte, en caractères.
const KICKER_MAX: usize = 80;
/// Part d'une piste de grille sous laquelle une forme est un filet.
const THIN_TRACK: f64 = 0.25;

/// Ce dont la conversion a besoin.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ContentCx<'a> {
    /// Couleurs du thème cible.
    pub tokens: &'a ColorTokens,
    /// Assets de la source, par id.
    pub assets: &'a BTreeMap<String, Asset>,
    /// Largeur d'une colonne et hauteur d'une ligne de la grille de la slide,
    /// en pt source : une forme plus fine qu'un quart de piste est un filet
    /// (`(0, 0)` : jamais).
    pub track: (f64, f64),
}

/// Une unité convertie.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Content {
    /// L'élément ; `None` si l'unité ne donne rien (vide, intraduisible).
    pub element: Option<Element>,
    /// Pertes, chacune une fois par item.
    pub losses: Vec<Loss>,
    /// Sorte du widget de repli employé (`stack`), pour un motif sans widget
    /// dont le repli perd quelque chose (voir [`group`]).
    pub fallback: Option<String>,
}

/// Ajoute la perte `kind` sur `item` si elle n'y est pas.
pub(crate) fn lose(losses: &mut Vec<Loss>, item: &str, kind: LossKind) {
    note(losses, Loss::on(item, kind));
}

/// Le jeton de `color`, ou rien (couleur par défaut, ou abandonnée avec une perte).
fn token(color: &source::Color, cx: &ContentCx<'_>, item: &str, losses: &mut Vec<Loss>) -> Option<String> {
    project_color(color, cx.tokens).unwrap_or_else(|loss| {
        lose(losses, item, loss);
        None
    })
}

/// La couleur unie d'un fond : la sienne, ou le premier arrêt d'un dégradé.
fn fill_color(fill: &Fill) -> Option<&source::Color> {
    match fill {
        Fill::Solid { color } => Some(color),
        Fill::Gradient { stops, .. } => stops.first().map(|s| &s.color),
        Fill::None | Fill::Picture { .. } => None,
    }
}

/// Où se pose un texte : sur la page, ou sur une carte (en relief ou non).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ground {
    /// Le fond de page `bg`.
    Page,
    /// Le fond d'une carte, en relief (`true`) ou non.
    Card(bool),
}

/// Le texte riche de `boxes` (pris dans l'ordre) posé sur `ground`, pertes notées sur `item`.
pub(crate) fn body(
    boxes: &[&TextBox],
    ground: Ground,
    cx: &ContentCx<'_>,
    item: &str,
    losses: &mut Vec<Loss>,
) -> Vec<Paragraph> {
    let bg = match ground {
        Ground::Page => cx.tokens.page(),
        Ground::Card(highlight) => cx.tokens.card(highlight),
    };
    let out = rich(boxes.iter().copied(), cx.tokens, bg);
    for l in out.losses {
        lose(losses, item, l);
    }
    out.paragraphs
}

/// Le texte plain de `tb`, pertes notées sur `item`.
pub(crate) fn plain_of(tb: &TextBox, item: &str, losses: &mut Vec<Loss>) -> String {
    let (text, lost) = plain(tb);
    for l in lost {
        lose(losses, item, l);
    }
    text
}

/// Un élément `text` au corps `paragraphs`, ou rien s'il est vide.
fn text(id: String, paragraphs: Vec<Paragraph>) -> Option<Element> {
    (!paragraphs.is_empty()).then(|| Element::text(id, paragraphs))
}

/// Une carte : surtitre et titre (plain) éventuels, corps riche.
pub(crate) fn card(
    id: String,
    kicker: Option<String>,
    title: Option<String>,
    paragraphs: Vec<Paragraph>,
    highlight: bool,
) -> Element {
    let mut e = Element::new(id, "card");
    for (name, value) in [("kicker", kicker), ("title", title)] {
        if let Some(t) = value.filter(|t| !t.is_empty()) {
            e.params.set(name, ParamValue::Text(t));
        }
    }
    if !paragraphs.is_empty() {
        e.params.set("body", ParamValue::Rich(paragraphs));
    }
    if highlight {
        e.params.set("highlight", ParamValue::Value(Value::Bool(true)));
    }
    e
}

/// Une carte au fond `fill` est-elle en relief ? Son jeton contraste avec le
/// fond d'une carte du thème, et le thème sait écrire clair sur le relief.
fn highlighted(fill: &Fill, cx: &ContentCx<'_>, item: &str, losses: &mut Vec<Loss>) -> bool {
    let Some(t) = fill_color(fill).and_then(|c| token(c, cx, item, losses)) else { return false };
    let tokens = cx.tokens;
    tokens.get(&t).is_some_and(|c| contrast(c, tokens.card(false)) >= MIN_CONTRAST)
        && tokens.on_dark(tokens.card(true)).is_some()
}

/// Un élément `image` de l'asset `asset`, ou rien s'il manque ou ne se dessine pas
/// (perte `UnsupportedImage`).
pub(crate) fn image(
    id: String,
    asset: &str,
    cx: &ContentCx<'_>,
    item: &str,
    losses: &mut Vec<Loss>,
) -> Option<Element> {
    let a = cx.assets.get(asset)?;
    if !DRAWABLE.contains(&a.mime.as_str()) {
        lose(losses, item, LossKind::UnsupportedImage(a.mime.clone()));
        return None;
    }
    let mut e = Element::new(id, "image");
    e.params.set("asset", ParamValue::Text(asset.to_owned()));
    e.params.set("fit", ParamValue::Text("contain".to_owned()));
    Some(e)
}

/// Un élément porteur d'une option d'hôte (`table.data`, `chart.option`).
fn host(
    id: String,
    kind: &str,
    param: &str,
    option: Option<(Value, Vec<LossKind>)>,
    item: &str,
    losses: &mut Vec<Loss>,
) -> Option<Element> {
    let (value, lost) = option?;
    for l in lost {
        lose(losses, item, l);
    }
    let mut e = Element::new(id, kind);
    e.params.set(param, ParamValue::Value(value));
    Some(e)
}

/// La forme du widget `shape` pour une géométrie source.
fn shape_kind(geom: &Geometry) -> &'static str {
    match geom {
        Geometry::Preset { name, .. } if ELLIPSES.contains(&name.as_str()) => "ellipse",
        Geometry::Preset { name, .. } if name == "line" || name.contains("Connector") => "line",
        Geometry::Preset { .. } | Geometry::Path { .. } => "rect",
    }
}

/// Une forme : carte ou texte si elle porte du texte, image si elle est
/// remplie d'une image, filet si elle est plus fine qu'un quart de piste,
/// sinon `shape` ; rien si ni son fond ni son trait n'ont de jeton.
fn shape(id: String, item: &Item, s: &source::Shape, cx: &ContentCx<'_>, losses: &mut Vec<Loss>) -> Option<Element> {
    if let Some(tb) = s.text.as_ref().filter(|_| item.has_text()) {
        if s.fill.is_none() {
            return text(id, body(&[tb], Ground::Page, cx, &item.id, losses));
        }
        let highlight = highlighted(&s.fill, cx, &item.id, losses);
        return Some(card(id, None, None, body(&[tb], Ground::Card(highlight), cx, &item.id, losses), highlight));
    }
    if let Fill::Picture { asset } = &s.fill {
        return image(id, asset, cx, &item.id, losses);
    }
    let thin = is_thin(&item.frame, cx.track);
    let fill = fill_color(&s.fill).and_then(|c| token(c, cx, &item.id, losses));
    let stroke = s.stroke.as_ref().and_then(|st| token(&st.color, cx, &item.id, losses));
    let mut e = Element::new(id, "shape");
    if thin {
        e.params.set("shape", ParamValue::Text("line".to_owned()));
        e.style.stroke = Some(stroke.or(fill)?);
        return Some(e);
    }
    if fill.is_none() && stroke.is_none() {
        return None;
    }
    e.params.set("shape", ParamValue::Text(shape_kind(&s.geom).to_owned()));
    e.style.fill = fill;
    e.style.stroke = stroke;
    Some(e)
}

/// Une boîte plus fine qu'un quart de piste (`track` : colonne et ligne en pt source) : un filet.
pub(crate) fn is_thin(f: &source::Frame, track: (f64, f64)) -> bool {
    f.w < THIN_TRACK * track.0 || f.h < THIN_TRACK * track.1
}

/// La perte `Transform` d'un item tourné ou retourné.
pub(crate) fn transform(item: &Item, losses: &mut Vec<Loss>) {
    let f = &item.frame;
    if f.rot != 0.0 || f.flip_h || f.flip_v {
        lose(losses, &item.id, LossKind::Transform);
    }
}

/// L'élément d'un item qui n'est pas un groupe.
pub(crate) fn leaf(item: &Item, id: String, cx: &ContentCx<'_>, losses: &mut Vec<Loss>) -> Option<Element> {
    transform(item, losses);
    match &item.kind {
        ItemKind::Text(tb) => text(id, body(&[tb], Ground::Page, cx, &item.id, losses)),
        ItemKind::Shape(s) => shape(id, item, s, cx, losses),
        ItemKind::Picture(p) => {
            if p.crop != source::Crop::default() {
                lose(losses, &item.id, LossKind::Transform);
            }
            image(id, &p.asset, cx, &item.id, losses)
        }
        ItemKind::Table(t) => host(id, "table", "data", table(t), &item.id, losses),
        ItemKind::Chart(c) => host(id, "chart", "option", chart(c), &item.id, losses),
        ItemKind::Opaque(o) => {
            lose(losses, &item.id, LossKind::Opaque(o.reason.clone()));
            o.fallback.as_deref().and_then(|a| image(id, a, cx, &item.id, losses))
        }
        ItemKind::Group { .. } => None,
    }
}

/// La plus grande taille de caractère d'un texte.
fn max_size(tb: &TextBox) -> f64 {
    tb.paragraphs.iter().flat_map(|p| &p.runs).map(|r| r.style.size).fold(0.0, f64::max)
}

/// Le titre d'une carte : le texte le plus grand, s'il y en a plusieurs et
/// qu'il dépasse strictement tous les autres.
fn card_title<'a>(texts: &[(&'a Item, &'a TextBox)]) -> Option<usize> {
    if texts.len() < 2 {
        return None;
    }
    let (best, size) = texts
        .iter()
        .enumerate()
        .map(|(k, (_, tb))| (k, max_size(tb)))
        .fold((0, 0.0), |acc, (k, s)| if s > acc.1 { (k, s) } else { acc });
    texts.iter().enumerate().all(|(k, (_, tb))| k == best || max_size(tb) < size).then_some(best)
}

/// Le surtitre d'une carte : le texte juste au-dessus du titre, d'un seul
/// paragraphe d'au plus 80 caractères, plus petit que lui.
fn card_kicker(texts: &[(&Item, &TextBox)], title: usize) -> Option<usize> {
    let (t_item, t_box) = texts[title];
    let above = |k: usize| {
        let (item, tb) = texts[k];
        item.frame.y + item.frame.h <= t_item.frame.y + 1.0
            && tb.paragraphs.iter().filter(|p| p.runs.iter().any(|r| !r.text.trim().is_empty())).count() == 1
            && plain(tb).0.chars().count() <= KICKER_MAX
            && max_size(tb) < max_size(t_box)
    };
    (0..title).rev().find(|k| above(*k))
}

/// La carte d'une forme remplie et des textes posés dessus.
fn cluster_card(id: String, ground: &Item, texts: &[&Item], cx: &ContentCx<'_>, losses: &mut Vec<Loss>) -> Element {
    transform(ground, losses);
    let fill = match &ground.kind {
        ItemKind::Shape(s) => &s.fill,
        _ => &Fill::None,
    };
    let highlight = highlighted(fill, cx, &ground.id, losses);
    let boxes: Vec<(&Item, &TextBox)> = texts.iter().filter_map(|i| i.text().map(|tb| (*i, tb))).collect();
    for (item, _) in &boxes {
        transform(item, losses);
    }
    let title = card_title(&boxes);
    let kicker = title.and_then(|t| card_kicker(&boxes, t));
    let head = |k: Option<usize>, losses: &mut Vec<Loss>| k.map(|k| plain_of(boxes[k].1, &boxes[k].0.id, losses));
    let (kicker_text, title_text) = (head(kicker, losses), head(title, losses));
    let rest: Vec<&TextBox> = boxes
        .iter()
        .enumerate()
        .filter(|(k, _)| Some(*k) != title && Some(*k) != kicker)
        .map(|(_, (_, tb))| *tb)
        .collect();
    let paragraphs = body(&rest, Ground::Card(highlight), cx, &ground.id, losses);
    card(id, kicker_text, title_text, paragraphs, highlight)
}

/// L'élément d'id `id` pour `unit`, ses pertes et son repli.
///
/// Les enfants d'un repli ont pour id `<id>-1`, `<id>-2`… Des textes fusionnés
/// gardent leurs pertes sur leurs propres items.
pub(crate) fn element(unit: &Unit<'_>, id: String, cx: &ContentCx<'_>) -> Content {
    let mut losses = Vec::new();
    let (element, fallback) = match unit {
        Unit::Card { shape, texts, .. } => (Some(cluster_card(id, shape, texts, cx, &mut losses)), None),
        Unit::Text(items) => {
            for item in items {
                transform(item, &mut losses);
            }
            let boxes: Vec<&TextBox> = items.iter().filter_map(|i| i.text()).collect();
            (text(id, body(&boxes, Ground::Page, cx, &unit.lead().id, &mut losses)), None)
        }
        Unit::Item(item) => match &item.kind {
            ItemKind::Group { items, smartart } => {
                transform(item, &mut losses);
                group(items, *smartart, &id, cx, &mut losses)
            }
            _ => (leaf(item, id, cx, &mut losses), None),
        },
    };
    Content { element, losses, fallback }
}

#[cfg(test)]
#[path = "content_tests.rs"]
mod content_tests;
```

`crates/oku-slides-import/src/retrofit/group.rs` :

```rust
//! Repli d'un groupe (ou d'un SmartArt dessiné) : une pile de cartes et
//! d'éléments.
//!
//! Les groupes imbriqués sont aplatis ; une feuille avec texte devient une
//! carte (premier paragraphe en `title` s'il y en a plusieurs, le reste en
//! `body`) ; une image, un tableau, un graphique gardent leur élément ; une
//! forme sans texte (flèche, connecteur, décor) est omise. La pile est en
//! colonnes si les centres des enfants s'étalent plus en largeur qu'en
//! hauteur, sinon en lignes.
//!
//! Le repli n'est dit (demande de widget, décision 5 de la spec) que s'il perd
//! quelque chose : une forme omise, un motif régulier d'au moins trois
//! feuilles (une rangée ou une colonne de boîtes semblables, également
//! espacées) dont la pile ne garde pas la géométrie, ou un SmartArt.

use oku_slides_core::{Element, ParamValue};

use super::content::{ContentCx, Ground, body, card, leaf, plain_of};
use crate::report::Loss;
use crate::source::{Frame, Item, ItemKind, TextBox};

/// Repli des groupes : une pile.
pub(crate) const STACK: &str = "stack";
/// Écart relatif toléré entre des tailles ou des pas « semblables ».
const SIMILAR: f64 = 0.75;

/// Les feuilles d'un groupe, groupes imbriqués aplatis, dans l'ordre de dessin.
fn leaves<'a>(items: &'a [Item], out: &mut Vec<&'a Item>) {
    for i in items {
        match &i.kind {
            ItemKind::Group { items, .. } => leaves(items, out),
            _ => out.push(i),
        }
    }
}

/// La carte d'une feuille de groupe avec texte : le premier paragraphe en
/// titre s'il y en a plusieurs, le reste en corps.
fn leaf_card(id: String, item: &Item, tb: &TextBox, cx: &ContentCx<'_>, losses: &mut Vec<Loss>) -> Element {
    let filled: Vec<&crate::source::Paragraph> =
        tb.paragraphs.iter().filter(|p| p.runs.iter().any(|r| !r.text.trim().is_empty())).collect();
    let (title, rest) = match filled.as_slice() {
        [first, rest @ ..] if !rest.is_empty() => {
            let head = TextBox { paragraphs: vec![(*first).clone()], ..tb.clone() };
            let rest = TextBox { paragraphs: rest.iter().map(|p| (*p).clone()).collect(), ..tb.clone() };
            (Some(plain_of(&head, &item.id, losses)), rest)
        }
        _ => (None, tb.clone()),
    };
    card(id, None, title, body(&[&rest], Ground::Card(false), cx, &item.id, losses), false)
}

/// Le plus petit sur le plus grand de `a` et `b` (1 si tous deux nuls).
fn ratio(a: f64, b: f64) -> f64 {
    if a.max(b) <= 0.0 { 1.0 } else { a.min(b) / a.max(b) }
}

/// Toutes les valeurs de `values` sont-elles semblables (à 25 % près) ?
fn similar(values: impl Iterator<Item = f64> + Clone) -> bool {
    let lo = values.clone().fold(f64::INFINITY, f64::min);
    let hi = values.fold(0.0, f64::max);
    ratio(lo, hi) >= SIMILAR
}

/// Un motif régulier : au moins trois boîtes de tailles semblables, centrées
/// sur une même rangée (ou colonne) à un quart de leur côté près, et également
/// espacées (pas semblables).
pub(crate) fn regular(frames: &[Frame]) -> bool {
    if frames.len() < 3 || !similar(frames.iter().map(|f| f.w)) || !similar(frames.iter().map(|f| f.h)) {
        return false;
    }
    let centers: Vec<(f64, f64)> = frames.iter().map(|f| (f.x + f.w / 2.0, f.y + f.h / 2.0)).collect();
    let spread = |f: fn(&(f64, f64)) -> f64| {
        let (lo, hi) =
            centers.iter().map(f).fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| (lo.min(v), hi.max(v)));
        hi - lo
    };
    let row = spread(|c| c.0) >= spread(|c| c.1);
    let side = frames.iter().map(|f| if row { f.h } else { f.w }).fold(f64::INFINITY, f64::min);
    if spread(if row { |c: &(f64, f64)| c.1 } else { |c: &(f64, f64)| c.0 }) > 0.25 * side {
        return false;
    }
    let mut along: Vec<f64> = centers.iter().map(|c| if row { c.0 } else { c.1 }).collect();
    along.sort_by(f64::total_cmp);
    similar(along.windows(2).map(|w| w[1] - w[0]))
}

/// Le repli d'un groupe aux enfants `items` (SmartArt dessiné si `smartart`)
/// et sa sorte de repli, si elle est à dire.
pub(crate) fn group(
    items: &[Item],
    smartart: bool,
    id: &str,
    cx: &ContentCx<'_>,
    losses: &mut Vec<Loss>,
) -> (Option<Element>, Option<String>) {
    let mut flat = Vec::new();
    leaves(items, &mut flat);
    let mut children = Vec::new();
    let mut frames = Vec::new();
    let mut omitted = false;
    for item in &flat {
        let child_id = format!("{id}-{}", children.len() + 1);
        let child = match item.text() {
            Some(tb) if item.has_text() => Some(leaf_card(child_id, item, tb, cx, losses)),
            _ => match &item.kind {
                ItemKind::Shape(_) => {
                    omitted = true;
                    None
                }
                _ => leaf(item, child_id, cx, losses),
            },
        };
        if let Some(c) = child {
            children.push(c);
            frames.push(item.frame);
        }
    }
    if children.is_empty() {
        return (None, None);
    }
    let spread = |f: fn(&Frame) -> f64| {
        let (lo, hi) =
            frames.iter().map(f).fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| (lo.min(v), hi.max(v)));
        hi - lo
    };
    let layout = if spread(|f| f.x + f.w / 2.0) > spread(|f| f.y + f.h / 2.0) { "columns" } else { "rows" };
    let mut e = Element::new(id, STACK);
    e.params.set("layout", ParamValue::Text(layout.to_owned()));
    e.params.set("children", ParamValue::Slot(children));
    let lossy = omitted || smartart || regular(&frames);
    (Some(e), lossy.then(|| STACK.to_owned()))
}

#[cfg(test)]
#[path = "group_tests.rs"]
mod group_tests;
```

`crates/oku-slides-import/src/pattern.rs` :

```rust
//! Signature d'un motif : ce qui fait que deux groupes d'items se ressemblent.
//!
//! Deux groupes ont la même signature s'ils ont les mêmes sortes d'items en
//! même nombre, disposés de la même façon une fois ramenés à leur boîte
//! englobante. Elle ne dépend ni de la position, ni de l'échelle, ni de
//! l'ordre de dessin : elle regroupe les occurrences d'un même motif (une
//! frise de trois étapes répétée sur plusieurs slides) en une seule demande
//! de widget.
//!
//! Limite assumée : chaque bord est arrondi au quart le plus proche ; un bord
//! posé près d'une frontière d'arrondi (un huitième : 1/8, 3/8…) bascule d'un
//! côté ou de l'autre pour un écart de quelques pt. Deux occurrences presque
//! identiques peuvent donc donner deux signatures, et deux demandes au lieu
//! d'une : une demande en trop, jamais un rendu faux.

use std::collections::BTreeMap;

use crate::source::{Frame, Item, ItemKind};

/// Pas de la grille de disposition : chaque bord est arrondi au quart.
const STEPS: f64 = 4.0;

/// Nom d'une sorte d'item.
fn kind(item: &Item) -> &'static str {
    match item.kind {
        ItemKind::Text(_) => "text",
        ItemKind::Shape(_) => "shape",
        ItemKind::Picture(_) => "picture",
        ItemKind::Table(_) => "table",
        ItemKind::Chart(_) => "chart",
        ItemKind::Group { .. } => "group",
        ItemKind::Opaque(_) => "opaque",
    }
}

/// `v` ramené à `[0, 1]` sur `[lo, lo + len]`, arrondi au pas `1 / STEPS`.
fn step(v: f64, lo: f64, len: f64) -> u8 {
    if len <= f64::EPSILON {
        return 0;
    }
    // Borné à [0, STEPS] : la conversion ne déborde pas.
    ((v - lo) / len * STEPS).round().clamp(0.0, STEPS) as u8
}

/// La signature de `items` : `<sorte>×<nombre>` triés par sorte, puis `|`,
/// puis la boîte de chaque item ramenée à la boîte englobante, arrondie au
/// quart (`x0 y0 x1 y1`, chiffres de 0 à 4), triée.
///
/// Exemple : trois formes de 200 × 100 à x = 0, 300 et 600 donnent
/// `shape×3|0014,2034,3044` (boîte englobante de 800 de large ; `300 / 800 × 4`
/// = 1,5 s'arrondit à 2).
pub fn signature(items: &[Item]) -> String {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for i in items {
        *counts.entry(kind(i)).or_default() += 1;
    }
    let kinds: Vec<String> = counts.iter().map(|(k, n)| format!("{k}×{n}")).collect();
    let frames: Vec<&Frame> = items.iter().map(|i| &i.frame).collect();
    let min = |f: fn(&Frame) -> f64| frames.iter().map(|x| f(x)).fold(f64::INFINITY, f64::min);
    let max = |f: fn(&Frame) -> f64| frames.iter().map(|x| f(x)).fold(f64::NEG_INFINITY, f64::max);
    let (x, y) = (min(|f| f.x), min(|f| f.y));
    let (w, h) = (max(|f| f.x + f.w) - x, max(|f| f.y + f.h) - y);
    let mut boxes: Vec<String> = frames
        .iter()
        .map(|f| format!("{}{}{}{}", step(f.x, x, w), step(f.y, y, h), step(f.x + f.w, x, w), step(f.y + f.h, y, h)))
        .collect();
    boxes.sort();
    format!("{}|{}", kinds.join(","), boxes.join(","))
}

#[cfg(test)]
#[path = "pattern_tests.rs"]
mod pattern_tests;
```

Le snapshot du lot 1 change : `INSTA_UPDATE=new cargo nextest run -p oku-slides-import --test kitchen`, lire `kitchen__kitchen.snap.new` (seul ajout : `"smartart": true` sur le groupe SmartArt de la slide 3), puis `cargo insta accept --manifest-path crates/oku-slides-import/Cargo.toml`.

- [ ] **Step 4 : lancer, constater le succès**

```bash
cargo nextest run -p oku-slides-import retrofit::content retrofit::cluster retrofit::group pattern::
```

Attendu : 51 tests passent (509 pour la crate entière au brouillon).

- [ ] **Step 5 : vérifier, commit, sous-tâche**

```bash
cargo fmt --all && cargo clippy -p oku-slides-import --all-targets -- -D warnings && cargo nextest run -p oku-slides-import
git add crates/oku-slides-import/src/lib.rs crates/oku-slides-import/src/pattern.rs crates/oku-slides-import/src/pattern_tests.rs crates/oku-slides-import/src/retrofit/mod.rs crates/oku-slides-import/src/retrofit/cluster.rs crates/oku-slides-import/src/retrofit/cluster_tests.rs crates/oku-slides-import/src/retrofit/content.rs crates/oku-slides-import/src/retrofit/content_tests.rs crates/oku-slides-import/src/retrofit/group.rs crates/oku-slides-import/src/retrofit/group_tests.rs crates/oku-slides-import/src/test_support.rs crates/oku-slides-import/src/source/mod.rs crates/oku-slides-import/src/source/mod_tests.rs crates/oku-slides-import/src/pptx/fonts.rs crates/oku-slides-import/src/pptx/shapes.rs crates/oku-slides-import/src/pptx/shapes_tests.rs crates/oku-slides-import/src/pptx/smartart.rs crates/oku-slides-import/src/pptx/smartart_tests.rs crates/oku-slides-import/tests/kitchen.rs crates/oku-slides-import/tests/snapshots/kitchen__kitchen.snap && git diff --cached --name-only
git commit -F <scratchpad>/msg.txt   # feat(import) : unités de contenu en éléments Oku, cartes, replis des groupes, signature de motif (t-692)
oku cli task done --project slider --id t-803
```

---

### Task 8: Titres déduits, profils des modèles et score

**Écarts d'interface :**
- `choose` rend `Option<&Profile>` avec deux durées de vie : `choose<'p, 't>(shape: &Shape, profiles: &'p [Profile<'t>]) -> Option<&'p Profile<'t>>`. `None` seulement si `profiles` est vide (un pack sans modèle) : la tâche 9 le traite (aucune slide n'est alors projetable sur ce pack).
- `SlideKind` gagne `Other` : un widget de slide d'un pack sans slot (ni `Title`, ni `Content`…) ; un widget de pack inconnu qui a un slot compte comme `Content`.
- Titres déduits (révision 1, item 1) : `infer_roles(items: &[&Item], page: Size) -> Vec<(String, Heading)>` rend des `Heading` (`Title`, `Kicker`, `Subtitle`), pas des `Role` : le lot 1 n'a pas de rôle « surtitre ». Il vit dans un module `retrofit/heading.rs` (`template.rs` dépasserait 500 lignes), avec `Headings` (titre, sous-titre, surtitre, couverture déduite) et `headings(items, page) -> Headings` : les placeholders s'il y a un titre, sinon les titres déduits. Il prend les items gardés de la slide (la tâche 9 en ôte d'abord le décor), pas la `SourceSlide`.
- La forme d'une slide compte des **unités** (tâche 7), plus des items : `shape(slide: &SourceSlide, layouts: &[Layout], heads: &Headings<'_>, units: &[Unit<'_>]) -> Shape` ; une carte compte une fois (`ContentKind::Card`). `content_items(items: &[&Item], heads: &Headings<'_>) -> Vec<&Item>` : le filtre des contenus (ni titres déduits, ni placeholders de titre, sous-titre, date, pied, numéro), réutilisé par la tâche 9 pour placer exactement ce qui a été compté.
- `Shape` gagne `row: Vec<usize>` (rangs des unités de la rangée de colonnes) : une rangée d'unités côte à côte entre des unités pleine largeur (introduction, bandeau) vaut des colonnes (`Want::Columns(n)`), pour que les slides de cartes d'alto prennent `cartes` / `colonnes` (révision 1, item 2) ; la tâche 9 la place en pile.
- Ajouts : `pub(crate) fn profile(template, registry) -> Profile`, `pub(crate) fn profiles(templates, registry) -> Vec<Profile>` (ordre du pack), `pub(crate) fn score(shape, profile) -> i32` (la table, testable seule).
- `Shape.subtitle` : un sous-titre n'est pas compté dans `content` (il remplit le `subtitle` d'une couverture ou d'un chapitre, tâche 9). Une forme sans texte compte comme contenu (`ContentKind::Shape`) : un décor posé sur la slide doit être placé, il pousse vers la grille.
- `test_support.rs` gagne `essentiel()`, `grilles()` (modèles des packs intégrés, `templates/*/pack.json`), `with_role(item, role)`, `layout(id, kind)`.

Une slide dessinée à la main (alto, généré par PptxGenJS) n'a aucun placeholder : son titre est une boîte de texte de 28 pt parmi d'autres. `infer_roles` le reconnaît à sa taille et à sa position ; le surtitre (« OUR CONVICTION ») et, sous un grand titre de couverture, le sous-titre suivent par proximité. Une couverture déduite (titre d'au moins 40 pt, aucun autre texte d'au moins 20 pt) remplit les paramètres du deck (décision 7, tâche 9).

Un modèle n'est qu'une slide d'exemple : son profil se lit dans cette slide et dans le schéma de son widget (les slots `flow` / `free` par leur `mode`, le paramètre de type `grid`, le choix `layout`), jamais dans les noms des modèles. La forme d'une slide source vient du type de son layout (`Layout.kind`) et de ses items de contenu. Le choix est un maximum de score, à égalité le premier dans l'ordre du pack ; il ne peut pas échouer tant que le pack a un modèle.

Table des scores de base (doc de `score`) :

| la slide source demande | modèle | score |
|---|---|---|
| couverture : layout `title`, ou sous-titre sans contenu | `Title` / `Chapter` / contenu | 100 / 50 / 10 |
| chapitre : layout `secHead` | `Chapter` / contenu | 100 / 10 |
| titre seul (aucun contenu) | `Grid` / `Default` / `Free` | 100 / 80 / 40 |
| un contenu | `Default` / `Grid` / `Free` / `Center` / `Quote` / `Split`, `Columns` | 100 / 60 / 50 / 40 / 30 / 20 |
| deux contenus en colonnes | `Split` / `Columns` / `Grid` / `Free` / `Default` | 100 / 90 / 60 / 50 / 40 |
| `n ≥ 3` contenus en colonnes | `Columns` / `Grid` / `Free` / `Default` / `Split` | 100 / 60 / 50 / 40 / 20 |
| plusieurs contenus disposés librement | `Grid` / `Free` / `Default` / `Columns`, `Split` | 100 / 80 / 30 / 20 |

Un modèle vierge (`blank`) compte comme `Grid` ; `Toc`, `End`, `Other` valent 0 ; une case absente vaut 0. Pour un modèle de contenu ou vierge : −15 si la présence d'un titre diffère entre la slide et l'exemple, puis moins l'écart (au plus 9) entre le nombre de blocs de l'exemple et le nombre de contenus de la slide. Si aucun score n'est positif : le premier modèle `Content`, sinon le premier modèle.

« En colonnes » : toutes les unités forment une rangée, ou une rangée d'au moins deux unités est entourée d'unités pleine largeur au-dessus ou au-dessous (`Want::Columns(n)`, `n` = la rangée).

Choix obtenus (tests) :

| slide source | Essentiel | Grilles |
|---|---|---|
| couverture | `couverture` | `grille-vide` |
| chapitre (`secHead`) | `chapitre` | — |
| titre seul | `texte` | `grille-vide` |
| titre + un corps | `texte` | `grille-vide` |
| deux colonnes | `colonnes` | `grille-deux-tiers` |
| trois colonnes | `cartes` | `grille-chiffres` |
| quatre contenus épars | `libre` | `grille-quatre` |
| trois cartes entre une introduction et un bandeau (alto, slide 4) | `cartes` | — |
| deux cartes entre une introduction et un bandeau (alto, slide 5) | `colonnes` | — |

Décision 7 (seule la première slide de titre remplit les paramètres du deck) reste à la tâche 9 : pour les slides de titre suivantes, elle passe une forme dont `layout_kind` vaut `secHead`, ce qui choisit le modèle `Chapter`.

**Files:**
- Create: `crates/oku-slides-import/src/retrofit/template.rs`, `crates/oku-slides-import/src/retrofit/template_tests.rs`, `crates/oku-slides-import/src/retrofit/heading.rs`, `crates/oku-slides-import/src/retrofit/heading_tests.rs`
- Modify: `crates/oku-slides-import/src/retrofit/mod.rs` (`mod heading; mod template;`), `crates/oku-slides-import/src/test_support.rs` (modèles des packs intégrés, layouts source)

**Interfaces:**
- Consumes : `oku_slides_core::{Template, WidgetRegistry, ParamType, SlotMode, Grid, Slide::resolved_grid}` (tâche 1 pour `Template`) ; `crate::source::{SourceSlide, Layout, Item, ItemKind, Role, Size, Frame}` (lot 1) ; `cluster::{Unit, bounds, units}` (tâche 7) ; `text::plain` (tâche 4).
- Produces :

```rust
// crate::retrofit::template
pub(crate) enum SlideKind { Title, Chapter, Toc, Content, End, Blank, Other }
pub(crate) enum ContentLayout { Default, Columns, Split, Center, Quote, Free, Grid }
pub(crate) struct Profile<'t> {
    pub template: &'t Template, pub kind: SlideKind, pub layout: ContentLayout, pub grid: Option<Grid>,
    pub blocks: usize, pub has_title: bool, pub has_kicker: bool,
}
pub(crate) enum ContentKind { Text, Picture, Table, Chart, Group, Shape, Card }
pub(crate) struct Shape {
    pub layout_kind: Option<String>, pub title: bool, pub subtitle: bool, pub content: Vec<ContentKind>,
    pub columns: usize, pub row: Vec<usize>,
}
pub(crate) fn profile<'t>(template: &'t Template, registry: &WidgetRegistry) -> Profile<'t>;
pub(crate) fn profiles<'t>(templates: &'t [Template], registry: &WidgetRegistry) -> Vec<Profile<'t>>;
pub(crate) fn content_items<'a>(items: &[&'a Item], heads: &Headings<'_>) -> Vec<&'a Item>;
pub(crate) fn shape(slide: &SourceSlide, layouts: &[Layout], heads: &Headings<'_>, units: &[Unit<'_>]) -> Shape;
pub(crate) fn score(shape: &Shape, profile: &Profile<'_>) -> i32;
pub(crate) fn choose<'p, 't>(shape: &Shape, profiles: &'p [Profile<'t>]) -> Option<&'p Profile<'t>>;
// crate::retrofit::heading
pub(crate) enum Heading { Title, Kicker, Subtitle }
pub(crate) struct Headings<'a> { pub title: Option<&'a Item>, pub subtitle: Option<&'a Item>, pub kicker: Option<&'a Item>, pub cover: bool }
impl Headings<'_> { pub(crate) fn holds(&self, item: &Item) -> bool; }
pub(crate) fn size_of(item: &Item) -> f64;      // plus grande taille de caractère
pub(crate) fn infer_roles(items: &[&Item], page: Size) -> Vec<(String, Heading)>;
pub(crate) fn headings<'a>(items: &[&'a Item], page: Size) -> Headings<'a>;
// crate::test_support (cfg(test))
pub(crate) fn essentiel() -> Vec<Template>;
pub(crate) fn grilles() -> Vec<Template>;
pub(crate) fn with_role(item: Item, role: Role) -> Item;
pub(crate) fn layout(id: &str, kind: &str) -> Layout;
```

- Règles (`infer_roles`, doc de la fonction) : la taille d'un item est sa plus grande taille de caractère ; la médiane, celle des items avec texte (basse pour un nombre pair).
  - titre : le plus grand (à égalité, le premier dessiné) qui commence dans les 40 % hauts de la page, d'au moins 20 pt et 1,4 × la médiane ;
  - surtitre : un texte d'un paragraphe d'au plus 80 caractères, plus petit que le titre, qui finit au plus 0,1 × la hauteur de page au-dessus de lui et le recouvre en largeur (le plus proche) ;
  - sous-titre, si le titre fait au moins 40 pt : le texte juste au-dessous (au plus 0,1 × la hauteur de page), qui le recouvre en largeur, d'au moins 1,15 × la médiane.
- Règles (`headings`) : un placeholder de titre avec du texte l'emporte (titre et sous-titre des placeholders, pas de surtitre, pas de couverture déduite) ; sinon les titres déduits, et `cover` quand le titre fait au moins 40 pt et qu'aucun autre texte (hors titres) n'atteint 20 pt.

- Règles (`profile`) :
  - `kind` : `title`, `chapter`, `toc`, `content`, `end`, `blank` ; un autre widget → `Content` s'il a un slot, sinon `Other` ;
  - `layout` : `Grid` pour `blank`, ou si l'exemple fixe un paramètre de type `grid` sans bloc `flow` ; sinon `Free` s'il a des éléments libres et aucun bloc ; sinon la valeur de `layout` (`columns`, `split`, `center`, `quote`, sinon `Default`) ;
  - `grid` : `Slide::resolved_grid` pour `Grid` ; `blocks` : nombre de blocs `flow`, sinon d'éléments libres ;
  - `has_title` : une couverture, ou un `title` non vide dans l'exemple ; `has_kicker` : un `kicker` non vide.
- Règles (`shape`) :
  - `layout_kind` : `kind` du layout de la slide ; `title` / `subtitle` : ceux de `heads` ;
  - contenus (`content_items`) : ni titres de `heads`, ni placeholders de titre, sous-titre, date, pied ou numéro ; texte non vide (boîte ou forme) → `Text`, forme sans texte → `Shape`, image → `Picture`, tableau, graphique, groupe ; intraduisible → `Picture` s'il a une image de repli, sinon ignoré ; `content` : une sorte par unité (`Card` pour une carte, `Text` pour des textes fusionnés) ;
  - rangée (`row`) : pour chaque unité, celles qui la recouvrent en hauteur d'au moins la moitié de la plus basse et dont la hauteur atteint 75 % de la plus haute d'entre elles ; la plus grande telle suite (au moins deux) dont les boîtes, triées par x, ne se chevauchent pas en largeur (à 5 % de la plus étroite près), ont des hauteurs à 25 % près et se recouvrent deux à deux en hauteur d'au moins la moitié de la plus basse. Elle compte si toutes les autres unités sont entièrement au-dessus ou au-dessous d'elle et au moins aussi larges que 60 % d'elle ; sinon pas de rangée ;
  - `columns` : 0 sans contenu ; la taille de la rangée s'il y en a une ; sinon 1.

- [ ] **Step 1 : tests qui échouent**

Dans `crates/oku-slides-import/src/test_support.rs`, la doc et les imports deviennent :

```rust
//! Aides de test du projecteur : jetons effectifs des thèmes intégrés, texte, items,
//! slides, layouts et decks source, registre de base, application d'une slide par les
//! ops du cœur, modèles des packs intégrés.

use std::collections::BTreeMap;

use oku_slides_core::tokens::effective_tokens;
use oku_slides_core::{Deck, Element, Op, ParamValue, Slide, Template, WidgetRegistry, apply_all};
use serde_json::{Map, Value};

use crate::retrofit::ColorTokens;
use crate::source::{
    Align, Bullet, Caps, Color, Fill, Frame, Item, ItemKind, Layout, ParaStyle, Paragraph, Rgba, Role, Run, RunStyle,
    Size, SourceDeck, SourceSlide, Spacing, Styled, TextBox,
};
```

puis, après `text_item`, ajouter :

```rust
/// Les modèles du pack intégré Essentiel, dans l'ordre du pack.
pub(crate) fn essentiel() -> Vec<Template> {
    templates(include_str!("../../../templates/essentiel/pack.json"))
}

/// Les modèles du pack intégré Grilles, dans l'ordre du pack.
pub(crate) fn grilles() -> Vec<Template> {
    templates(include_str!("../../../templates/grilles/pack.json"))
}

fn templates(pack_json: &str) -> Vec<Template> {
    let pack: Value = serde_json::from_str(pack_json).unwrap();
    serde_json::from_value(pack["templates"].clone()).unwrap()
}

/// `item` avec le rôle de placeholder `role`.
pub(crate) fn with_role(mut item: Item, role: Role) -> Item {
    item.role = Some(role);
    item
}

/// Un layout `id` de type `kind`, sans placeholder.
pub(crate) fn layout(id: &str, kind: &str) -> Layout {
    Layout {
        id: id.to_owned(),
        name: kind.to_owned(),
        kind: kind.to_owned(),
        master: "m1".to_owned(),
        background: None,
        show_master_decor: true,
        decor: Vec::new(),
        placeholders: Vec::new(),
    }
}
```

`crates/oku-slides-import/src/retrofit/heading_tests.rs` :

```rust
use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::source::ItemKind;
use crate::test_support::{item, para, run_with, text_box, text_item, with_role};

const PAGE: Size = Size { w: 960.0, h: 540.0 };

/// Une boîte de texte d'un paragraphe en taille `size`.
fn sized(id: &str, frame: (f64, f64, f64, f64), text: &str, size: f64) -> Item {
    item(id, frame, ItemKind::Text(text_box(vec![para(vec![run_with(text, |s| s.size = size)])])))
}

/// Slide 2 d'alto : surtitre 9,5 pt, titre 28 pt, cartes (titres 13 pt, corps 10,5 pt).
/// Tailles triées : 9,5 10,5 10,5 13 13 28 ; médiane basse (rang 2) : 10,5 ; 28 ≥ 1,4 × 10,5.
fn conviction() -> Vec<Item> {
    vec![
        sized("kicker", (64.0, 56.0, 857.0, 15.3), "OUR CONVICTION", 9.5),
        sized("title", (64.0, 89.8, 772.5, 66.4), "The agent becomes the only interface", 28.0),
        sized("c1", (80.0, 196.2, 252.7, 17.6), "The only window", 13.0),
        sized("b1", (80.0, 219.8, 252.7, 69.2), "The agent becomes the single window", 10.5),
        sized("c2", (357.3, 196.2, 252.7, 17.6), "A deeper confidant", 13.0),
        sized("b2", (357.3, 219.8, 252.7, 86.0), "People will confide", 10.5),
    ]
}

/// Slide 1 d'alto : bandeau (15 et 9,5 pt), surtitre 9,5, titre 52, sous-titre 12, corps 16, méta 9,5.
/// Tailles triées : 9,5 ×4, 12, 15, 16, 52 ; médiane basse (rang 3) : 9,5 ; 12 ≥ 1,15 × 9,5.
fn cover() -> Vec<Item> {
    vec![
        sized("brand", (64.0, 48.0, 117.8, 23.0), "ALTO WEALTH", 15.0),
        sized("doc", (637.5, 51.8, 165.9, 15.3), "D2C Strategy — Working Document", 9.5),
        sized("kicker", (64.0, 172.9, 857.0, 15.3), "FROM A SURVIVAL QUESTION", 9.5),
        sized("title", (64.0, 200.2, 857.0, 111.2), "The Agent Is the New Front Door", 52.0),
        sized("subtitle", (64.0, 323.4, 857.0, 18.8), "From Software Vendor to Wealth Infrastructure", 12.0),
        sized("body", (64.0, 354.2, 643.8, 69.2), "As the conversational agent becomes…", 16.0),
        sized("meta1", (64.0, 435.4, 857.0, 15.3), "Distribution to Customer", 9.5),
        sized("meta2", (64.0, 462.7, 857.0, 15.3), "Matthieu Dartiguenave", 9.5),
    ]
}

fn roles(items: &[Item]) -> Vec<(String, Heading)> {
    infer_roles(&items.iter().collect::<Vec<_>>(), PAGE)
}

#[test]
fn infer_roles_of_a_content_slide_finds_its_title_and_kicker() {
    assert_that!(roles(&conviction()))
        .is_equal_to(vec![("title".to_owned(), Heading::Title), ("kicker".to_owned(), Heading::Kicker)]);
}

#[test]
fn infer_roles_of_a_cover_adds_the_subtitle_below_a_large_title() {
    assert_that!(roles(&cover())).is_equal_to(vec![
        ("title".to_owned(), Heading::Title),
        ("kicker".to_owned(), Heading::Kicker),
        ("subtitle".to_owned(), Heading::Subtitle),
    ]);
}

/// Pas de sous-titre sous un titre de 28 pt, même juste au-dessous.
#[test]
fn infer_roles_under_a_small_title_finds_no_subtitle() {
    let mut items = conviction();
    items.push(sized("under", (64.0, 160.0, 700.0, 20.0), "Our client is disappearing", 14.0));
    assert_that!(roles(&items).iter().any(|(_, h)| *h == Heading::Subtitle)).is_false();
}

#[rstest]
#[case::too_small(vec![sized("t", (64.0, 90.0, 800.0, 30.0), "Petit", 18.0), sized("b", (64.0, 200.0, 800.0, 30.0), "Corps", 10.0)])]
#[case::too_low(vec![sized("t", (64.0, 300.0, 800.0, 30.0), "Bas", 28.0), sized("b", (64.0, 400.0, 800.0, 30.0), "Corps", 10.0)])]
#[case::not_above_the_rest(vec![sized("t", (64.0, 90.0, 800.0, 30.0), "Égal", 24.0), sized("b", (64.0, 200.0, 800.0, 30.0), "Corps", 20.0)])]
#[case::no_text(vec![])]
fn infer_roles_without_a_standing_out_title_finds_nothing(#[case] items: Vec<Item>) {
    assert_that!(roles(&items)).is_empty();
}

/// Un surtitre trop loin (plus de 0,1 × 540 = 54 pt au-dessus) ou de deux paragraphes n'en est pas un.
#[test]
fn infer_roles_rejects_a_far_or_long_kicker() {
    let mut items = conviction();
    items[0].frame.y = 10.0;
    assert_that!(roles(&items)).is_equal_to(vec![("title".to_owned(), Heading::Title)]);
    let mut items = conviction();
    items[0].kind = ItemKind::Text(text_box(vec![
        para(vec![run_with("UN", |s| s.size = 9.5)]),
        para(vec![run_with("DEUX", |s| s.size = 9.5)]),
    ]));
    assert_that!(roles(&items)).is_equal_to(vec![("title".to_owned(), Heading::Title)]);
}

#[test]
fn headings_of_a_cover_find_title_subtitle_kicker_and_the_cover() {
    let items = cover();
    let refs: Vec<&Item> = items.iter().collect();
    let h = headings(&refs, PAGE);
    let id = |i: Option<&Item>| i.map(|i| i.id.clone());
    assert_that!((id(h.title), id(h.subtitle), id(h.kicker), h.cover)).is_equal_to((
        Some("title".to_owned()),
        Some("subtitle".to_owned()),
        Some("kicker".to_owned()),
        true,
    ));
}

/// Une autre ligne de 28 pt sous le titre de 52 pt : un second titre, pas une couverture.
#[test]
fn headings_with_another_large_text_are_not_a_cover() {
    let mut items = cover();
    items.push(sized("second", (64.0, 440.0, 800.0, 40.0), "Encore un titre", 28.0));
    let refs: Vec<&Item> = items.iter().collect();
    assert_that!(headings(&refs, PAGE).cover).is_false();
}

#[test]
fn headings_prefer_the_placeholders() {
    let items = [
        with_role(text_item("ph", (40.0, 300.0, 880.0, 60.0), "Titre"), Role::Title),
        with_role(text_item("sub", (40.0, 380.0, 880.0, 30.0), "Sous-titre"), Role::Subtitle),
        sized("big", (40.0, 20.0, 880.0, 60.0), "Gros texte", 60.0),
    ];
    let refs: Vec<&Item> = items.iter().collect();
    let h = headings(&refs, PAGE);
    assert_that!((h.title.map(|i| i.id.as_str()), h.subtitle.map(|i| i.id.as_str()), h.kicker, h.cover)).is_equal_to((
        Some("ph"),
        Some("sub"),
        None,
        false,
    ));
}
```

> Slide 2 d'alto (`conviction`) : tailles triées 9,5 10,5 10,5 13 13 28, médiane basse (rang 2) 10,5 ; 28 ≥ 20 et ≥ 1,4 × 10,5 = 14,7 ; le surtitre finit à 71,3 pt, 18,5 pt au-dessus du titre (≤ 54). Slide 1 (`cover`) : tailles 9,5 ×4, 12, 15, 16, 52, médiane basse (rang 3) 9,5 ; le sous-titre de 12 pt ≥ 1,15 × 9,5 = 10,9 commence 12 pt sous le titre ; le corps de 16 pt est sous 20 pt : couverture.

`crates/oku-slides-import/src/retrofit/template_tests.rs` :

```rust
use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::retrofit::cluster::units;
use crate::retrofit::heading::headings;
use crate::source::{Fill, Geometry, Opaque, OpaqueReason, Picture, Rgba, Size, Styled};
use crate::test_support::{essentiel, grilles, item, layout, registry, source_slide, text_item, with_role};

/// La forme de `slide` comme la calcule la projection : titres, items de
/// contenu, unités (cellules d'un pt : pas de fusion de textes éloignés).
fn shape_of(slide: &SourceSlide, layouts: &[Layout]) -> Shape {
    let items: Vec<&Item> = slide.items.iter().collect();
    let heads = headings(&items, Size { w: 960.0, h: 540.0 });
    let content = content_items(&items, &heads);
    let cell = |f: &Frame| oku_slides_core::Cell { col: f.x as u32, row: f.y as u32, cols: 1, rows: 1 };
    shape(slide, layouts, &heads, &units(&content, cell))
}

fn summary<'a>(p: &Profile<'a>) -> (&'a str, SlideKind, ContentLayout, usize, bool, bool) {
    (p.template.id.as_str(), p.kind, p.layout, p.blocks, p.has_title, p.has_kicker)
}

#[test]
fn profiles_of_essentiel_follow_their_sample_slides() {
    use ContentLayout as L;
    use SlideKind as K;
    let (templates, registry) = (essentiel(), registry());
    let rows: Vec<_> = profiles(&templates, &registry).iter().map(summary).collect();
    assert_that!(rows).is_equal_to(vec![
        ("texte", K::Content, L::Default, 1, true, true),
        ("cartes", K::Content, L::Columns, 3, true, true),
        ("colonnes", K::Content, L::Split, 2, true, false),
        ("citation", K::Content, L::Quote, 1, true, false),
        ("message", K::Content, L::Center, 1, false, false),
        ("libre", K::Content, L::Free, 2, true, false),
        ("chapitre", K::Chapter, L::Default, 0, true, false),
        ("sommaire", K::Toc, L::Default, 0, false, false),
        ("couverture", K::Title, L::Default, 0, true, false),
        ("fin", K::End, L::Default, 0, false, false),
    ]);
}

#[test]
fn profiles_of_grilles_carry_their_grids() {
    let (templates, registry) = (grilles(), registry());
    let profiles = profiles(&templates, &registry);
    let grids: Vec<_> = profiles
        .iter()
        .map(|p| (p.template.id.as_str(), p.layout, p.grid.as_ref().map(|g| (g.columns, g.rows)), p.blocks))
        .collect();
    let grid = |id, g, n| (id, ContentLayout::Grid, Some(g), n);
    assert_that!(grids).is_equal_to(vec![
        grid("grille-vide", (12, 6), 0),
        grid("grille-deux-tiers", (12, 6), 2),
        grid("grille-quatre", (2, 2), 4),
        grid("grille-bandeau", (3, 3), 4),
        grid("grille-mosaique", (12, 6), 4),
        grid("grille-chiffres", (3, 2), 3),
    ]);
}

#[test]
fn profile_of_a_blank_slide_is_a_grid_without_title() {
    let mut t = essentiel().remove(0);
    t.slide = oku_slides_core::Slide::new("tpl-vierge", "blank");
    let p = profile(&t, &registry());
    assert_that!((p.kind, p.layout, p.has_title)).is_equal_to((SlideKind::Blank, ContentLayout::Grid, false));
    assert_that!(p.grid.map(|g| (g.columns, g.rows))).is_equal_to(Some((12, 6)));
}

fn title(text: &str) -> Item {
    with_role(text_item("t", (40.0, 20.0, 880.0, 60.0), text), Role::Title)
}

fn picture(id: &str, frame: (f64, f64, f64, f64)) -> Item {
    item(id, frame, ItemKind::Picture(Picture { asset: "sha256:a".into(), crop: Default::default() }))
}

#[test]
fn shape_reads_the_layout_kind_title_and_content() {
    let s = source_slide(
        1,
        Some("l2"),
        vec![
            title("Titre"),
            text_item("b", (40.0, 100.0, 880.0, 400.0), "Corps"),
            picture("p", (500.0, 100.0, 400.0, 300.0)),
        ],
    );
    let shape = shape_of(&s, &[layout("l1", "title"), layout("l2", "obj")]);
    assert_that!(shape).is_equal_to(Shape {
        layout_kind: Some("obj".into()),
        title: true,
        subtitle: false,
        content: vec![ContentKind::Text, ContentKind::Picture],
        columns: 1,
        row: Vec::new(),
    });
}

#[test]
fn shape_skips_empty_boxes_footers_and_untranslatable_items_without_fallback() {
    let opaque = |id: &str, fallback: Option<String>| {
        item(id, (0.0, 0.0, 10.0, 10.0), ItemKind::Opaque(Opaque { reason: OpaqueReason::Ole, fallback }))
    };
    let s = source_slide(
        3,
        None,
        vec![
            title(""),
            with_role(text_item("d", (0.0, 500.0, 100.0, 20.0), "10/10/2026"), Role::Date),
            with_role(text_item("f", (100.0, 500.0, 100.0, 20.0), "Pied"), Role::Footer),
            with_role(text_item("n", (900.0, 500.0, 40.0, 20.0), "3"), Role::SlideNum),
            text_item("vide", (0.0, 0.0, 10.0, 10.0), "  "),
            opaque("o1", None),
            opaque("o2", Some("sha256:b".into())),
        ],
    );
    let shape = shape_of(&s, &[]);
    assert_that!((shape.layout_kind, shape.title, shape.content)).is_equal_to((
        None,
        false,
        vec![ContentKind::Picture],
    ));
}

#[rstest]
#[case::side_by_side(vec![(40.0, 100.0, 400.0, 300.0), (480.0, 100.0, 400.0, 300.0)], 2)]
#[case::three_columns(vec![(40.0, 100.0, 260.0, 300.0), (340.0, 110.0, 260.0, 290.0), (640.0, 100.0, 260.0, 300.0)], 3)]
#[case::stacked(vec![(40.0, 100.0, 880.0, 150.0), (40.0, 280.0, 880.0, 150.0)], 1)]
#[case::uneven_heights(vec![(40.0, 100.0, 400.0, 300.0), (480.0, 100.0, 400.0, 100.0)], 1)]
#[case::overlapping(vec![(40.0, 100.0, 500.0, 300.0), (400.0, 100.0, 500.0, 300.0)], 1)]
#[case::single(vec![(40.0, 100.0, 400.0, 300.0)], 1)]
#[case::none(vec![], 0)]
fn shape_counts_columns_of_side_by_side_boxes(#[case] frames: Vec<(f64, f64, f64, f64)>, #[case] columns: usize) {
    let items = frames.into_iter().enumerate().map(|(i, f)| picture(&format!("p{i}"), f)).collect();
    assert_that!(shape_of(&source_slide(1, None, items), &[]).columns).is_equal_to(columns);
}

fn wants(layout_kind: Option<&str>, subtitle: bool, content: usize, columns: usize) -> Shape {
    Shape {
        layout_kind: layout_kind.map(str::to_owned),
        title: true,
        subtitle,
        content: vec![ContentKind::Text; content],
        columns,
        row: if columns >= 2 { (0..columns).collect() } else { Vec::new() },
    }
}

fn ground(id: &str, frame: (f64, f64, f64, f64)) -> Item {
    let s = crate::source::Shape {
        geom: Geometry::Preset { name: "roundRect".into(), adj: Default::default() },
        fill: Fill::Solid { color: Styled::explicit(Rgba::new(255, 255, 255)) },
        stroke: None,
        text: None,
    };
    item(id, frame, ItemKind::Shape(Box::new(s)))
}

/// Slide 4 d'alto : une introduction pleine largeur, trois cartes côte à côte
/// (fond et deux textes chacune), un bandeau pleine largeur : chaque carte compte
/// une fois, et la rangée de trois fait des colonnes.
#[test]
fn shape_counts_each_card_once_and_finds_a_row_between_wide_units() {
    let mut items = vec![text_item("intro", (64.0, 144.0, 772.0, 36.0), "Intro")];
    for (k, x) in [64.0, 346.0, 627.0].into_iter().enumerate() {
        items.push(ground(&format!("c{k}"), (x, 200.0, 269.0, 172.0)));
        items.push(text_item(&format!("t{k}"), (x + 16.0, 218.0, 243.0, 18.0), "Titre"));
        items.push(text_item(&format!("b{k}"), (x + 16.0, 284.0, 243.0, 52.0), "Corps"));
    }
    items.push(ground("banner", (64.0, 394.0, 832.0, 66.0)));
    items.push(text_item("result", (96.0, 418.0, 612.0, 19.0), "Résultat"));
    let shape = shape_of(&source_slide(4, None, items), &[]);
    assert_that!(shape.content).is_equal_to(vec![
        ContentKind::Text,
        ContentKind::Card,
        ContentKind::Card,
        ContentKind::Card,
        ContentKind::Card,
    ]);
    assert_that!((shape.columns, shape.row)).is_equal_to((3, vec![1, 2, 3]));
}

/// Une unité étroite au-dessous de la rangée : pas de colonnes.
#[test]
fn shape_without_wide_neighbours_has_no_row() {
    let items = vec![
        picture("a", (40.0, 100.0, 260.0, 200.0)),
        picture("b", (340.0, 100.0, 260.0, 200.0)),
        picture("c", (40.0, 350.0, 100.0, 50.0)),
    ];
    let shape = shape_of(&source_slide(1, None, items), &[]);
    assert_that!((shape.columns, shape.row.is_empty())).is_equal_to((1, true));
}

/// Les scores sont dans la doc de `score`. Par exemple, titre seul sur Essentiel :
/// `texte` = 80 (Default) − 1 (un bloc) = 79, `libre` = 40 − 2 = 38.
#[rstest]
#[case::cover(wants(Some("title"), true, 0, 0), "couverture")]
#[case::section(wants(Some("secHead"), false, 0, 0), "chapitre")]
#[case::subtitle_only(wants(None, true, 0, 0), "couverture")]
#[case::title_only(wants(Some("titleOnly"), false, 0, 0), "texte")]
#[case::one_body(wants(Some("obj"), false, 1, 1), "texte")]
#[case::two_columns(wants(Some("twoObj"), false, 2, 2), "colonnes")]
#[case::three_columns(wants(Some("obj"), false, 3, 3), "cartes")]
#[case::row_of_three_among_five(Shape { content: vec![ContentKind::Card; 5], ..wants(None, false, 3, 3) }, "cartes")]
#[case::row_of_two_among_four(Shape { content: vec![ContentKind::Card; 4], ..wants(None, false, 2, 2) }, "colonnes")]
#[case::scattered(wants(Some("blank"), false, 4, 1), "libre")]
fn choose_on_essentiel_picks_the_matching_template(#[case] shape: Shape, #[case] expected: &str) {
    let (templates, registry) = (essentiel(), registry());
    let profiles = profiles(&templates, &registry);
    assert_that!(choose(&shape, &profiles).map(|p| p.template.id.as_str())).is_equal_to(Some(expected));
}

/// Grilles n'a que des grilles : une couverture vaut 10 partout, `grille-vide` (0 bloc) l'emporte ;
/// trois colonnes : `60 − |blocs − 3|`, `grille-chiffres` (3 blocs) l'emporte.
#[rstest]
#[case::cover(wants(Some("title"), true, 0, 0), "grille-vide")]
#[case::title_only(wants(Some("titleOnly"), false, 0, 0), "grille-vide")]
#[case::one_body(wants(Some("obj"), false, 1, 1), "grille-vide")]
#[case::two_columns(wants(Some("twoObj"), false, 2, 2), "grille-deux-tiers")]
#[case::three_columns(wants(Some("obj"), false, 3, 3), "grille-chiffres")]
#[case::four_scattered(wants(Some("obj"), false, 4, 1), "grille-quatre")]
fn choose_on_grilles_picks_the_closest_grid(#[case] shape: Shape, #[case] expected: &str) {
    let (templates, registry) = (grilles(), registry());
    let profiles = profiles(&templates, &registry);
    assert_that!(choose(&shape, &profiles).map(|p| p.template.id.as_str())).is_equal_to(Some(expected));
}

#[test]
fn choose_breaks_ties_by_pack_order() {
    let (mut templates, registry) = (essentiel(), registry());
    let mut copy = templates[0].clone();
    copy.id = "texte-bis".into();
    templates.push(copy.clone());
    let one = wants(Some("obj"), false, 1, 1);
    let first = |templates: &[Template]| choose(&one, &profiles(templates, &registry)).map(|p| p.template.id.clone());
    assert_that!(first(&templates)).is_equal_to(Some("texte".to_owned()));
    templates.insert(0, copy);
    assert_that!(first(&templates)).is_equal_to(Some("texte-bis".to_owned()));
}

#[test]
fn choose_without_positive_score_takes_the_first_content_template_else_the_first() {
    let (templates, registry) = (essentiel(), registry());
    let all = profiles(&templates, &registry);
    let pick = |ids: &[&str]| -> Vec<Profile<'_>> {
        ids.iter().filter_map(|id| all.iter().find(|p| p.template.id == *id).cloned()).collect()
    };
    let one = wants(Some("obj"), false, 1, 1);
    let no_content = pick(&["sommaire", "fin"]);
    assert_that!(choose(&one, &no_content).map(|p| p.template.id.as_str())).is_equal_to(Some("sommaire"));
    // Chapitre demandé, `message` vaut 10 − 15 (titre) − 1 (bloc) = −6 : repli sur le premier contenu.
    let section = wants(Some("secHead"), false, 0, 0);
    let with_content = pick(&["fin", "message"]);
    assert_that!(choose(&section, &with_content).map(|p| p.template.id.as_str())).is_equal_to(Some("message"));
}

#[test]
fn choose_among_no_profile_is_none() {
    assert_that!(choose(&wants(None, false, 1, 1), &[])).is_none();
}

#[test]
fn score_penalizes_a_title_mismatch_and_a_block_gap() {
    let (templates, registry) = (essentiel(), registry());
    let all = profiles(&templates, &registry);
    let message = all.iter().find(|p| p.template.id == "message").unwrap();
    let cartes = all.iter().find(|p| p.template.id == "cartes").unwrap();
    // message : Center 40, sans titre (−15), un bloc (−0).
    assert_that!(score(&wants(Some("obj"), false, 1, 1), message)).is_equal_to(25);
    // cartes : Columns 20 pour un contenu, trois blocs (−2).
    assert_that!(score(&wants(Some("obj"), false, 1, 1), cartes)).is_equal_to(18);
}
```

> Essentiel, un contenu : `texte` = 100 (Default) − 0 = 100 ; `citation` = 30 ; `cartes` = 20 − 2 = 18 ; `message` = 40 − 15 (sans titre) = 25. Deux colonnes : `colonnes` = 100 (Split, deux blocs). Quatre contenus épars : `libre` = 80 (Free) − 2 = 78, `texte` = 30 − 3 = 27. Grilles, quatre contenus épars : `grille-quatre` et `grille-mosaique` valent 100 (quatre blocs chacune) : la première dans l'ordre du pack gagne. `choose_breaks_ties_by_pack_order` : une copie de `texte` en fin de pack perd, en tête elle gagne.

> `shape_counts_each_card_once_and_finds_a_row_between_wide_units` : l'introduction (772 pt) et le bandeau (832 pt) font au moins 60 % de la rangée (832 pt de large), entièrement au-dessus et au-dessous : rangée des unités 1 à 3. `row_of_three_among_five` : `Columns(3)` → `cartes` (Columns 100 − |3 − 5| = 98) devance `colonnes` (Split 20).

`crates/oku-slides-import/src/retrofit/mod.rs` complet :

```rust
//! Projecteur retrofit : une [`SourceDeck`](crate::source::SourceDeck) projetée sur
//! un pack et un thème existants.
//!
//! Le style vient du thème cible : couleurs ramenées à ses jetons ([`ColorTokens`]),
//! texte riche réduit à ce que le cœur exprime, éléments libres placés sur la
//! grille d'une zone de corps approchée par ses jetons de page ([`PageTokens`]).

mod chart;
#[allow(dead_code, reason = "branché par le projecteur, tâche 9")]
mod cluster;
mod color;
#[allow(dead_code, reason = "branché par le projecteur, tâche 9")]
mod content;
#[allow(dead_code, reason = "branché par le projecteur, tâche 9")]
mod group;
#[allow(dead_code, reason = "branché par le projecteur, tâche 9")]
mod heading;
mod table;
#[allow(dead_code, reason = "branché par le projecteur, tâche 9")]
mod template;
mod text;
#[allow(dead_code, reason = "branché par le projecteur, tâche 9")]
mod zone;

pub use color::ColorTokens;
pub use zone::PageTokens;
```

- [ ] **Step 2 : lancer, constater l'échec**

```bash
export CARGO_TARGET_DIR=$PWD/target
cargo nextest run -p oku-slides-import retrofit::template retrofit::heading
```

Attendu : échec de compilation (`retrofit/template.rs` et `retrofit/heading.rs` absents).

- [ ] **Step 3 : implémentation**

`crates/oku-slides-import/src/retrofit/heading.rs` :

```rust
//! Titres d'une slide : ceux des placeholders, sinon déduits de la taille et
//! de la position des textes (une slide dessinée à la main n'a pas de
//! placeholder ; [`infer_roles`]).

use super::text::plain;
use crate::source::{Frame, Item, Role, Size};

/// Taille minimale d'un titre déduit, en pt.
const TITLE_MIN: f64 = 20.0;
/// Rapport minimal entre un titre déduit et la taille médiane de la slide.
const TITLE_RATIO: f64 = 1.4;
/// Part haute de la page où commence un titre déduit.
const TITLE_ZONE: f64 = 0.4;
/// Taille à partir de laquelle un titre fait une couverture et admet un sous-titre.
const COVER_TITLE: f64 = 40.0;
/// Rapport minimal entre un sous-titre et la taille médiane.
const SUBTITLE_RATIO: f64 = 1.15;
/// Écart vertical maximal entre un titre et son surtitre ou son sous-titre, en part de la hauteur de page.
const NEAR: f64 = 0.1;
/// Longueur maximale d'un surtitre, en caractères.
const KICKER_MAX: usize = 80;

/// Un titre déduit d'une slide sans placeholder de titre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Heading {
    /// Titre.
    Title,
    /// Surtitre.
    Kicker,
    /// Sous-titre.
    Subtitle,
}

/// Les titres d'une slide.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Headings<'a> {
    /// Titre.
    pub title: Option<&'a Item>,
    /// Sous-titre.
    pub subtitle: Option<&'a Item>,
    /// Surtitre (déduit seulement).
    pub kicker: Option<&'a Item>,
    /// Couverture déduite : un titre d'au moins 40 pt, sans autre texte de titre.
    pub cover: bool,
}

impl Headings<'_> {
    /// L'item `item` est-il un des titres ?
    pub(crate) fn holds(&self, item: &Item) -> bool {
        [self.title, self.subtitle, self.kicker].into_iter().flatten().any(|h| h.id == item.id)
    }
}

/// La plus grande taille de caractère d'un item avec texte.
pub(crate) fn size_of(item: &Item) -> f64 {
    item.text().map_or(0.0, |tb| tb.paragraphs.iter().flat_map(|p| &p.runs).map(|r| r.style.size).fold(0.0, f64::max))
}

/// Les deux boîtes se recouvrent-elles en largeur ?
fn overlaps_x(a: &Frame, b: &Frame) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w
}

/// Les titres déduits de `items` (une slide sans placeholder de titre), sur une page `page`.
///
/// La taille d'un item est sa plus grande taille de caractère ; la médiane est
/// celle des items avec texte (médiane basse pour un nombre pair).
/// - titre : la plus grande taille (à égalité, le premier dessiné) d'un item
///   qui commence dans les 40 % hauts de la page, d'au moins 20 pt et 1,4 × la médiane ;
/// - surtitre : un texte d'un paragraphe d'au plus 80 caractères, plus petit
///   que le titre, qui finit au plus 0,1 × la hauteur de page au-dessus de lui
///   et le recouvre en largeur (le plus proche) ;
/// - sous-titre, si le titre fait au moins 40 pt : le texte juste au-dessous
///   (au plus 0,1 × la hauteur de page), qui le recouvre en largeur, d'au moins 1,15 × la médiane.
pub(crate) fn infer_roles(items: &[&Item], page: Size) -> Vec<(String, Heading)> {
    let texts: Vec<&Item> = items.iter().copied().filter(|i| i.has_text()).collect();
    let mut sizes: Vec<f64> = texts.iter().map(|i| size_of(i)).collect();
    sizes.sort_by(f64::total_cmp);
    let Some(median) = sizes.get(sizes.len().saturating_sub(1) / 2).copied() else { return Vec::new() };
    let title =
        texts.iter().copied().filter(|i| i.frame.y <= TITLE_ZONE * page.h).fold(None, |best: Option<&Item>, i| {
            if best.is_none_or(|b| size_of(i) > size_of(b)) { Some(i) } else { best }
        });
    let Some(title) = title.filter(|t| size_of(t) >= TITLE_MIN && size_of(t) >= TITLE_RATIO * median) else {
        return Vec::new();
    };
    let near = NEAR * page.h;
    let t = title.frame;
    let mut roles = vec![(title.id.clone(), Heading::Title)];
    let kicker = texts
        .iter()
        .filter(|i| i.id != title.id && overlaps_x(&i.frame, &t) && size_of(i) < size_of(title))
        .filter(|i| {
            let bottom = i.frame.y + i.frame.h;
            bottom <= t.y + 1.0 && t.y - bottom <= near
        })
        .filter(|i| {
            i.text().is_some_and(|tb| {
                tb.paragraphs.iter().filter(|p| p.runs.iter().any(|r| !r.text.trim().is_empty())).count() == 1
                    && plain(tb).0.chars().count() <= KICKER_MAX
            })
        })
        .max_by(|a, b| (a.frame.y + a.frame.h).total_cmp(&(b.frame.y + b.frame.h)));
    if let Some(k) = kicker {
        roles.push((k.id.clone(), Heading::Kicker));
    }
    if size_of(title) >= COVER_TITLE {
        let below = t.y + t.h;
        let subtitle = texts
            .iter()
            .filter(|i| i.id != title.id && overlaps_x(&i.frame, &t) && size_of(i) >= SUBTITLE_RATIO * median)
            .filter(|i| i.frame.y >= below - 1.0 && i.frame.y - below <= near)
            .min_by(|a, b| a.frame.y.total_cmp(&b.frame.y));
        if let Some(s) = subtitle {
            roles.push((s.id.clone(), Heading::Subtitle));
        }
    }
    roles
}

/// Les titres de `items` : ceux des placeholders s'il y a un titre, sinon
/// ceux que déduit [`infer_roles`] ; une couverture déduite est un titre
/// d'au moins 40 pt sans autre texte d'au moins 20 pt.
pub(crate) fn headings<'a>(items: &[&'a Item], page: Size) -> Headings<'a> {
    let role = |r: Role| items.iter().copied().find(|i| i.role == Some(r) && i.has_text());
    if let Some(title) = role(Role::Title) {
        return Headings { title: Some(title), subtitle: role(Role::Subtitle), kicker: None, cover: false };
    }
    let roles = infer_roles(items, page);
    let find = |h: Heading| {
        roles.iter().find(|(_, r)| *r == h).and_then(|(id, _)| items.iter().copied().find(|i| &i.id == id))
    };
    let mut heads = Headings {
        title: find(Heading::Title),
        subtitle: find(Heading::Subtitle),
        kicker: find(Heading::Kicker),
        cover: false,
    };
    heads.cover = heads.title.is_some_and(|t| size_of(t) >= COVER_TITLE)
        && !items.iter().any(|i| !heads.holds(i) && size_of(i) >= TITLE_MIN);
    heads
}

#[cfg(test)]
#[path = "heading_tests.rs"]
mod heading_tests;
```

`crates/oku-slides-import/src/retrofit/template.rs` :

```rust
//! Choix du modèle d'une slide : profil de chaque modèle du pack, forme de la
//! slide source, score.
//!
//! Un modèle ne déclare ni champ requis ni type de mise en page : son profil
//! se déduit de sa slide d'exemple et du schéma de son widget (sorte de
//! slide, slot `flow` rempli ou non, paramètre `grid`, choix `layout`). La
//! forme d'une slide source vient du type de son layout, de ses titres
//! (`heading`) et de ses unités de contenu (cartes regroupées, `cluster`). Le score (voir [`score`])
//! est déterministe ; à égalité, l'ordre du pack.

use oku_slides_core::{Grid, ParamType, SlotMode, Template, WidgetRegistry};

use super::cluster::{Unit, bounds};
use super::heading::Headings;
use crate::source::{Frame, Item, ItemKind, Layout, Role, SourceSlide};

/// Sorte d'une slide de modèle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlideKind {
    /// Couverture (`title`) : ses champs sont ceux du deck.
    Title,
    /// Chapitre.
    Chapter,
    /// Sommaire.
    Toc,
    /// Contenu.
    Content,
    /// Fin.
    End,
    /// Vierge (`blank`) : sans titre, éléments libres sur la grille.
    Blank,
    /// Widget de slide d'un pack, sans slot.
    Other,
}

/// Agencement du contenu d'un modèle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContentLayout {
    /// Blocs empilés.
    Default,
    /// Une colonne par bloc.
    Columns,
    /// Deux colonnes.
    Split,
    /// Blocs centrés.
    Center,
    /// Citation.
    Quote,
    /// Éléments libres sans grille déclarée.
    Free,
    /// Éléments libres sur une grille.
    Grid,
}

/// Ce qu'offre un modèle.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Profile<'t> {
    /// Le modèle.
    pub template: &'t Template,
    /// Sorte de slide.
    pub kind: SlideKind,
    /// Agencement.
    pub layout: ContentLayout,
    /// Grille, pour l'agencement `Grid`.
    pub grid: Option<Grid>,
    /// Nombre de blocs (ou d'éléments libres) de l'exemple.
    pub blocks: usize,
    /// L'exemple a un titre (une couverture en a toujours un).
    pub has_title: bool,
    /// L'exemple a un surtitre.
    pub has_kicker: bool,
}

/// Sorte d'un contenu de slide source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContentKind {
    /// Texte (boîte de texte ou forme avec texte).
    Text,
    /// Image (ou intraduisible avec image de repli).
    Picture,
    /// Tableau.
    Table,
    /// Graphique.
    Chart,
    /// Groupe ou SmartArt dessiné.
    Group,
    /// Forme sans texte.
    Shape,
    /// Carte : forme remplie et textes posés dessus.
    Card,
}

/// La forme d'une slide source.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Shape {
    /// Type du layout (`title`, `secHead`, `obj`, `twoObj`, `titleOnly`, `blank`…).
    pub layout_kind: Option<String>,
    /// Un titre non vide.
    pub title: bool,
    /// Un sous-titre non vide.
    pub subtitle: bool,
    /// Les unités de contenu (hors titres, date, pied, numéro), dans l'ordre de dessin.
    pub content: Vec<ContentKind>,
    /// Unités côte à côte : leur nombre si elles forment une rangée de colonnes
    /// (seules, ou entre des unités pleine largeur au-dessus et au-dessous), sinon 1 (0 sans contenu).
    pub columns: usize,
    /// Les rangs (dans `content`) des unités de la rangée de colonnes ; vide sans rangée.
    pub row: Vec<usize>,
}

/// La sorte d'une slide de widget `kind`.
fn slide_kind(kind: &str, has_slot: bool) -> SlideKind {
    match kind {
        "title" => SlideKind::Title,
        "chapter" => SlideKind::Chapter,
        "toc" => SlideKind::Toc,
        "content" => SlideKind::Content,
        "end" => SlideKind::End,
        "blank" => SlideKind::Blank,
        _ if has_slot => SlideKind::Content,
        _ => SlideKind::Other,
    }
}

/// Le profil de `template`, d'après sa slide d'exemple et le schéma de son widget.
pub(crate) fn profile<'t>(template: &'t Template, registry: &WidgetRegistry) -> Profile<'t> {
    let slide = &template.slide;
    let params = registry.get(&slide.kind).map(|w| w.params.as_slice()).unwrap_or_default();
    let filled = |mode: SlotMode| -> usize {
        params
            .iter()
            .filter(|p| matches!(&p.ty, ParamType::Slot { mode: m, .. } if *m == mode))
            .map(|p| slide.params.slot(&p.name).map_or(0, <[_]>::len))
            .sum()
    };
    let (flow, free) = (filled(SlotMode::Flow), filled(SlotMode::Free));
    let has_slot = params.iter().any(|p| matches!(p.ty, ParamType::Slot { .. }));
    let kind = slide_kind(&slide.kind, has_slot);
    let grid_set = params.iter().any(|p| matches!(p.ty, ParamType::Grid) && slide.params.value(&p.name).is_some());
    let layout = if kind == SlideKind::Blank || (grid_set && flow == 0) {
        ContentLayout::Grid
    } else if free > 0 && flow == 0 {
        ContentLayout::Free
    } else {
        match slide.params.text("layout") {
            Some("columns") => ContentLayout::Columns,
            Some("split") => ContentLayout::Split,
            Some("center") => ContentLayout::Center,
            Some("quote") => ContentLayout::Quote,
            _ => ContentLayout::Default,
        }
    };
    let non_empty = |name: &str| slide.params.text(name).is_some_and(|t| !t.trim().is_empty());
    Profile {
        template,
        kind,
        layout,
        grid: (layout == ContentLayout::Grid).then(|| slide.resolved_grid(registry)),
        blocks: if flow > 0 { flow } else { free },
        has_title: kind == SlideKind::Title || non_empty("title"),
        has_kicker: non_empty("kicker"),
    }
}

/// Les profils de `templates`, dans l'ordre du pack.
pub(crate) fn profiles<'t>(templates: &'t [Template], registry: &WidgetRegistry) -> Vec<Profile<'t>> {
    templates.iter().map(|t| profile(t, registry)).collect()
}

/// La sorte de contenu d'un item, `None` s'il ne donne rien.
fn content_kind(item: &Item) -> Option<ContentKind> {
    match &item.kind {
        ItemKind::Text(_) => item.has_text().then_some(ContentKind::Text),
        ItemKind::Shape(_) if item.has_text() => Some(ContentKind::Text),
        ItemKind::Shape(_) => Some(ContentKind::Shape),
        ItemKind::Picture(_) => Some(ContentKind::Picture),
        ItemKind::Table(_) => Some(ContentKind::Table),
        ItemKind::Chart(_) => Some(ContentKind::Chart),
        ItemKind::Group { .. } => Some(ContentKind::Group),
        ItemKind::Opaque(o) => o.fallback.as_ref().map(|_| ContentKind::Picture),
    }
}

/// La sorte de contenu d'une unité.
fn unit_kind(unit: &Unit<'_>) -> ContentKind {
    match unit {
        Unit::Card { .. } => ContentKind::Card,
        Unit::Text(_) => ContentKind::Text,
        Unit::Item(item) => content_kind(item).unwrap_or(ContentKind::Shape),
    }
}

/// Les items de contenu parmi `items` : ni titres (`heads`), ni placeholders de
/// titre, sous-titre, date, pied ou numéro de slide, et porteurs de quelque chose.
pub(crate) fn content_items<'a>(items: &[&'a Item], heads: &Headings<'_>) -> Vec<&'a Item> {
    items
        .iter()
        .copied()
        .filter(|i| !matches!(i.role, Some(Role::Title | Role::Subtitle | Role::Date | Role::Footer | Role::SlideNum)))
        .filter(|i| !heads.holds(i) && content_kind(i).is_some())
        .collect()
}

/// Les boîtes forment-elles des colonnes ? Triées par x : aucune ne chevauche
/// la suivante en largeur (à 5 % de la plus étroite près), hauteurs à 25 %
/// près, et chaque paire se recouvre en hauteur d'au moins la moitié de la plus basse.
fn side_by_side(frames: &[Frame]) -> bool {
    let mut frames = frames.to_vec();
    frames.sort_by(|a, b| a.x.total_cmp(&b.x));
    let narrowest = frames.iter().map(|f| f.w).fold(f64::INFINITY, f64::min);
    let (low, high) = frames.iter().fold((f64::INFINITY, 0.0_f64), |(lo, hi), f| (lo.min(f.h), hi.max(f.h)));
    let heights = high > 0.0 && low / high >= 0.75;
    heights
        && frames.windows(2).all(|w| {
            let overlap = (w[0].y + w[0].h).min(w[1].y + w[1].h) - w[0].y.max(w[1].y);
            w[1].x >= w[0].x + w[0].w - 0.05 * narrowest && overlap >= 0.5 * w[0].h.min(w[1].h)
        })
}

/// Part minimale de la largeur de la rangée qu'occupe une unité pleine largeur.
const WIDE: f64 = 0.6;

/// La rangée de colonnes de `frames` : la plus grande suite (au moins deux)
/// de boîtes côte à côte, prise parmi celles qui recouvrent en hauteur une
/// boîte donnée et dont la hauteur atteint 75 % de la plus haute d'entre
/// elles ; les autres boîtes doivent être entièrement au-dessus ou au-dessous
/// de la rangée et au moins aussi larges que 60 % d'elle. Vide sinon.
fn row(frames: &[Frame]) -> Vec<usize> {
    let mut best: Vec<usize> = Vec::new();
    for f in frames {
        let band: Vec<usize> = (0..frames.len())
            .filter(|k| {
                let g = &frames[*k];
                (f.y + f.h).min(g.y + g.h) - f.y.max(g.y) >= 0.5 * f.h.min(g.h)
            })
            .collect();
        let tallest = band.iter().map(|k| frames[*k].h).fold(0.0, f64::max);
        let tall: Vec<usize> = band.into_iter().filter(|k| frames[*k].h >= 0.75 * tallest).collect();
        let boxes: Vec<Frame> = tall.iter().map(|k| frames[*k]).collect();
        if tall.len() >= 2 && tall.len() > best.len() && side_by_side(&boxes) {
            best = tall;
        }
    }
    let span = bounds(best.iter().map(|k| &frames[*k]));
    let fits = (0..frames.len()).filter(|k| !best.contains(k)).all(|k| {
        let g = &frames[k];
        let apart = g.y + g.h <= span.y + 1.0 || g.y >= span.y + span.h - 1.0;
        apart && g.w >= WIDE * span.w
    });
    if fits { best } else { Vec::new() }
}

/// La forme de `slide` (layout cherché dans `layouts`), de titres `heads` et d'unités `units`.
pub(crate) fn shape(slide: &SourceSlide, layouts: &[Layout], heads: &Headings<'_>, units: &[Unit<'_>]) -> Shape {
    let layout_kind = slide.layout.as_ref().and_then(|id| layouts.iter().find(|l| &l.id == id)).map(|l| l.kind.clone());
    let frames: Vec<Frame> = units.iter().map(Unit::frame).collect();
    let row = row(&frames);
    let columns = match units.len() {
        0 => 0,
        _ if row.len() >= 2 => row.len(),
        _ => 1,
    };
    Shape {
        layout_kind,
        title: heads.title.is_some(),
        subtitle: heads.subtitle.is_some(),
        content: units.iter().map(unit_kind).collect(),
        columns,
        row,
    }
}

/// Ce que la slide source demande.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Want {
    /// Couverture.
    Title,
    /// Chapitre.
    Chapter,
    /// Titre seul.
    Empty,
    /// Un contenu.
    Single,
    /// `n` contenus en colonnes.
    Columns(usize),
    /// Plusieurs contenus disposés librement.
    Many,
}

fn want(shape: &Shape) -> Want {
    match shape.layout_kind.as_deref() {
        Some("title") => return Want::Title,
        Some("secHead") => return Want::Chapter,
        _ => {}
    }
    match shape.content.len() {
        0 if shape.subtitle => Want::Title,
        0 => Want::Empty,
        1 => Want::Single,
        _ if shape.columns >= 2 => Want::Columns(shape.columns),
        _ => Want::Many,
    }
}

/// Score de base d'un agencement de contenu pour `want` (table de [`score`]).
fn base(want: Want, layout: ContentLayout) -> i32 {
    use ContentLayout as L;
    match (want, layout) {
        (Want::Title | Want::Chapter, _) => 10,
        (Want::Empty, L::Grid) => 100,
        (Want::Empty, L::Default) => 80,
        (Want::Empty, L::Free) => 40,
        (Want::Single, L::Default) => 100,
        (Want::Single, L::Grid) => 60,
        (Want::Single, L::Free) => 50,
        (Want::Single, L::Center) => 40,
        (Want::Single, L::Quote) => 30,
        (Want::Single, L::Split | L::Columns) => 20,
        (Want::Columns(2), L::Split) => 100,
        (Want::Columns(2), L::Columns) => 90,
        (Want::Columns(_), L::Columns) => 100,
        (Want::Columns(_), L::Grid) => 60,
        (Want::Columns(_), L::Free) => 50,
        (Want::Columns(_), L::Default) => 40,
        (Want::Columns(_), L::Split) => 20,
        (Want::Many, L::Grid) => 100,
        (Want::Many, L::Free) => 80,
        (Want::Many, L::Default) => 30,
        (Want::Many, L::Columns | L::Split) => 20,
        _ => 0,
    }
}

/// Le score de `profile` pour `shape`.
///
/// | la slide source demande | modèle | score |
/// |---|---|---|
/// | couverture (layout `title`, ou sous-titre sans contenu) | `Title` / `Chapter` / contenu | 100 / 50 / 10 |
/// | chapitre (layout `secHead`) | `Chapter` / contenu | 100 / 10 |
/// | titre seul | `Grid` / `Default` / `Free` | 100 / 80 / 40 |
/// | un contenu | `Default` / `Grid` / `Free` / `Center` / `Quote` / `Split`, `Columns` | 100 / 60 / 50 / 40 / 30 / 20 |
/// | deux contenus en colonnes | `Split` / `Columns` / `Grid` / `Free` / `Default` | 100 / 90 / 60 / 50 / 40 |
/// | `n ≥ 3` contenus en colonnes | `Columns` / `Grid` / `Free` / `Default` / `Split` | 100 / 60 / 50 / 40 / 20 |
/// | plusieurs contenus disposés librement | `Grid` / `Free` / `Default` / `Columns`, `Split` | 100 / 80 / 30 / 20 |
///
/// Un modèle vierge compte comme `Grid` ; `Toc`, `End` et les autres sortes
/// valent 0. Pour un modèle de contenu (ou vierge) : −15 si la présence d'un
/// titre diffère, puis − l'écart (au plus 9) entre son nombre de blocs et le
/// nombre de contenus de la slide.
pub(crate) fn score(shape: &Shape, profile: &Profile<'_>) -> i32 {
    let want = want(shape);
    let layout = match profile.kind {
        SlideKind::Content => profile.layout,
        SlideKind::Blank => ContentLayout::Grid,
        SlideKind::Title => return if want == Want::Title { 100 } else { 0 },
        SlideKind::Chapter => {
            return match want {
                Want::Chapter => 100,
                Want::Title => 50,
                _ => 0,
            };
        }
        SlideKind::Toc | SlideKind::End | SlideKind::Other => return 0,
    };
    let titled = if profile.has_title == shape.title { 0 } else { -15 };
    let gap = profile.blocks.abs_diff(shape.content.len()).min(9);
    // Borné à 9 : la conversion ne déborde pas.
    base(want, layout) + titled - gap as i32
}

/// Le profil le mieux noté pour `shape` ; à égalité, le premier dans l'ordre du pack.
///
/// Si aucun n'a un score positif : le premier modèle de contenu, sinon le
/// premier modèle. `None` seulement si `profiles` est vide.
pub(crate) fn choose<'p, 't>(shape: &Shape, profiles: &'p [Profile<'t>]) -> Option<&'p Profile<'t>> {
    let mut best: Option<(&Profile<'t>, i32)> = None;
    for p in profiles {
        let s = score(shape, p);
        if best.is_none_or(|(_, b)| s > b) {
            best = Some((p, s));
        }
    }
    match best {
        Some((p, s)) if s > 0 => Some(p),
        _ => profiles.iter().find(|p| p.kind == SlideKind::Content).or_else(|| profiles.first()),
    }
}

#[cfg(test)]
#[path = "template_tests.rs"]
mod template_tests;
```

- [ ] **Step 4 : lancer, constater le succès**

```bash
cargo nextest run -p oku-slides-import retrofit::template retrofit::heading
```

Attendu : 45 tests passent (554 pour la crate entière au brouillon).

- [ ] **Step 5 : vérifier, commit, sous-tâche**

```bash
cargo fmt --all && cargo clippy -p oku-slides-import --all-targets -- -D warnings && cargo nextest run -p oku-slides-import
git add crates/oku-slides-import/src/retrofit/mod.rs crates/oku-slides-import/src/retrofit/template.rs crates/oku-slides-import/src/retrofit/template_tests.rs crates/oku-slides-import/src/retrofit/heading.rs crates/oku-slides-import/src/retrofit/heading_tests.rs crates/oku-slides-import/src/test_support.rs && git diff --cached --name-only
git commit -F <scratchpad>/msg.txt   # feat(import) : titres déduits, profils des modèles d'un pack et choix du modèle d'une slide source (t-692)
oku cli task done --project slider --id t-804
```

---

### Task 9: Une slide → des ops ; décor omis ; `retrofit::project`

**Écarts d'interface :**
- `RetrofitTarget` dérive `Debug, Clone` ; `project(deck: &SourceDeck, target: &RetrofitTarget<'_>) -> ImportPlan` comme prévu. `report.coverage` reste `None` (tâche 10).
- Nouveau module `retrofit/slide.rs` : `pub(crate) struct SlideCx<'a> { registry, page, content: ContentCx<'a>, layouts: &'a [Layout], size: Size, scale: f64, decor: &'a Decor }`, `pub(crate) struct Projected<'a> { ops: Vec<Op>, report: SlideReport, assets: BTreeSet<String>, fallbacks: Vec<(&'a Item, String)> }`, `pub(crate) fn project_slide<'a>(src: &'a SourceSlide, profiles: &[Profile<'_>], deck_titled: &mut bool, cx: &SlideCx<'_>) -> Option<Projected<'a>>` (`None` seulement si `profiles` est vide, ce que `project` empêche).
- Nouveau module `retrofit/decor.rs` (révision 1, item 5) : `Decor::of(deck) -> Decor`, `Decor::skips(&self, item) -> bool`, `Decor::losses(&self, number) -> &[Loss]` ; `in_band(frame, page) -> bool`, `is_backdrop(item, page) -> bool`. Un seul prédicat pour la projection et, à la tâche 10, pour la couverture (G11).
- `project` déduit les accents du deck (`ColorTokens::with_accents`, tâche 3) avant de projeter.
- Échelle : `min(page.width / size.w, page.height / size.h)` au lieu de `1920 / size.w` (identique pour une source 16:9 ; une source 4:3 n'est plus coupée en bas).
- Les instantanés : `plan.ops` complet pour `minimal` et `kitchen` ; pour `alto`, un résumé par slide (modèle, placement, surtitre, titre, et chaque élément : sorte, cellule ou cadre arrondi au pt, titre d'une carte, enfants d'une pile).
- `test_support.rs` gagne `applied(ops, assets) -> Deck` ; `tests/common/mod.rs` gagne `registry()`, `templates(name)`, `tokens(name)` (par `oku_slides_core::tokens::effective_tokens`, tâche 1 ; les tests d'intégration ne voient pas `test_support`).
- Plus aucun `#[allow(dead_code)]` dans `src/` : ceux de `cluster`, `content`, `group`, `heading`, `template`, `zone` (`retrofit/mod.rs`) et de `pattern` (`lib.rs`) sont retirés.

`project` parcourt les slides source dans l'ordre. Pour chacune : les items omis par règle (`Decor`) sont écartés ; les titres viennent des placeholders, sinon de la taille et de la position des textes (`heading::headings`, tâche 8) ; le reste, sans le décor propre à la slide, se regroupe en unités (`cluster::units`, tâche 7) ; la forme (`template::shape`) choisit un modèle ; la slide Oku `s<numéro>` prend la sorte du widget du modèle, et chaque unité devient un élément `s<numéro>-<rang>` (`content::element`). Aucun texte d'exemple du modèle n'est repris : seulement son agencement (`layout`, en placement `flow`) ou sa grille (`grid`, en placement libre).

Règles de `Decor::of` (doc de `decor.rs`) :

| item | règle |
|---|---|
| placeholder date, pied de page, numéro de slide | omis |
| texte ou image dont le centre est dans la bande haute ou basse (12 % de la hauteur de page) | motif : texte plain chiffres ôtés, espaces normalisées ; image : son asset |
| motif présent sur au moins la moitié des slides (et deux au moins) | tous ses items omis ; perte `RepeatedDecor { text, slides }` une fois, sur la première slide où il apparaît (texte de la première occurrence, nom ou texte alternatif pour une image) ; motifs dans l'ordre de leur clé |
| texte de la bande sans lettre ni chiffre une fois les chiffres ôtés (un numéro de page) | omis |

Règles de `project_slide` :

| étape | règle |
|---|---|
| titres | `headings(items gardés, taille de page)` ; couverture : layout `title`, sous-titre sans contenu, ou couverture déduite ; la première émet `SetDeckParam` `title` et `subtitle` (décision 7) ; les suivantes reçoivent `layout_kind = "secHead"` avant le choix (modèle `Chapter`) |
| décor de la slide | omis sans perte : une forme sans texte d'au moins la moitié de la page (un fond), une forme sans texte dans une bande d'en-tête ou de pied, un filet (plus fin qu'un quart de piste) posé au-dessus du bas du titre |
| unités | `units(contenus, cellule)` ; la cellule de référence : grille 12 × 6 sur la zone titrée, les contenus ajustés dedans (`Fit`) — la fusion des textes précède le choix du modèle |
| choix | si la slide a du contenu, parmi les modèles dont le widget a un slot (une couverture avec du contenu devient une slide de contenu, le titre du deck reste rempli) ; sinon parmi tous |
| titre, surtitre, sous-titre | texte plain de l'item (`text::plain`, pertes notées sur l'item) ; dans le paramètre du même nom si le widget le déclare en `plain` non lié ; sinon, sauf s'il est lié au deck sur la première couverture, l'item devient un contenu (les unités sont refaites avec lui) |
| placement | `flow` si le modèle a un slot `flow` et (n'a pas de slot libre, ou agence en blocs — ni `Free`, ni `Grid` — et la slide a au plus une unité ou une rangée de colonnes) ; sinon le slot libre ; sans slot, les items des unités sont dits perdus (`NoSlot`) |
| flux | unités en ordre de lecture (une rangée seule de gauche à droite) ; une rangée entourée d'autres unités devient une pile `stack` `columns` (id du bloc, enfants `<bloc>-1`…) à la place de sa première unité, et la slide prend `layout: default` ; une forme sans texte n'entre jamais dans un flux |
| éléments libres | ordre de dessin ; boîte englobante des unités ajustée dans `body_zone(page, titre présent)` (`Fit::new`) ; cadre `to_zone` sur un modèle libre sans `grid` ; sur un modèle à grille, `snap` sur la grille de la slide (`resolved_grid`, `grid` copié du modèle) si aucune cellule d'un contenu (forme exceptée) n'en recouvre une autre, sinon les cadres restent |
| filets | `ContentCx.track` = une piste de la grille de la slide ramenée en pt source par l'échelle de l'ajustement |
| notes | `SetNotes` après l'`AddSlide` si les notes ne sont pas vides |
| pertes | `RepeatedDecor` de la slide (`Decor::losses`), `HiddenSlide` sur la slide masquée, pertes des contenus (les enfants d'un repli sur leurs ids source) ; chaque perte une fois |
| assets | ceux des éléments `image` de la slide, descendants compris |

`project` : modèle `content` nu si le pack n'a aucun modèle ; `plan.assets` = assets source référencés ; `report.warnings` = avertissements du lecteur ; une `WidgetRequest` par signature (`pattern::signature` des enfants du groupe) pour chaque repli dit (`Content.fallback`), slides dans l'ordre, sans doublon consécutif, `sample` = enfants de la première occurrence.

**Files:**
- Create: `crates/oku-slides-import/src/retrofit/slide.rs`, `crates/oku-slides-import/src/retrofit/slide_tests.rs`, `crates/oku-slides-import/src/retrofit/decor.rs`, `crates/oku-slides-import/src/retrofit/decor_tests.rs`, `crates/oku-slides-import/src/retrofit/mod_tests.rs`, `crates/oku-slides-import/tests/retrofit.rs`, `crates/oku-slides-import/tests/snapshots/retrofit__minimal_ops.snap`, `crates/oku-slides-import/tests/snapshots/retrofit__kitchen_ops.snap`, `crates/oku-slides-import/tests/snapshots/retrofit__alto_retrofit_summary.snap`
- Modify: `crates/oku-slides-import/src/retrofit/mod.rs` (réécrit), `crates/oku-slides-import/src/lib.rs`, `crates/oku-slides-import/src/test_support.rs` (`applied`), `crates/oku-slides-import/tests/common/mod.rs` (registre, modèles, jetons)

**Interfaces:**
- Consumes : `template::{profiles, choose, shape, content_items, Profile, Shape, ContentLayout}`, `heading::{headings, Headings}` (8) ; `cluster::{units, Unit, reading}`, `content::{element, is_thin, plain_of, ContentCx}` (7) ; `zone::{PageTokens, Fit, Zone, body_zone, grid_gap, snap, to_zone, track_size}` (5) ; `text::{plain, note}` (4) ; `color::ColorTokens` (3) ; `pattern::signature` (7) ; `plan::{ImportPlan, WidgetRequest}`, `report::{ImportReport, Mode, Loss, LossKind, Placement, SlideReport}` (2) ; `oku_slides_core::{Cell, Element, Grid, Op, Slide, Template, WidgetRegistry, ParamType, SlotMode, WidgetSchema}`.
- Produces :

```rust
// crate::retrofit
pub struct RetrofitTarget<'a> {
    pub templates: &'a [Template], pub registry: &'a WidgetRegistry, pub colors: ColorTokens, pub page: PageTokens,
}
pub fn project(deck: &SourceDeck, target: &RetrofitTarget<'_>) -> ImportPlan;
// crate (lib.rs)
pub use retrofit::RetrofitTarget;
// crate::retrofit::decor
pub(crate) const BAND: f64 = 0.12;
pub(crate) fn in_band(frame: &Frame, page: Size) -> bool;
pub(crate) fn is_backdrop(item: &Item, page: Size) -> bool;
pub(crate) struct Decor { /* ids omis, pertes par slide */ }   // Debug, Clone, PartialEq, Default
impl Decor {
    pub(crate) fn of(deck: &SourceDeck) -> Self;
    pub(crate) fn skips(&self, item: &Item) -> bool;
    pub(crate) fn losses(&self, number: u32) -> &[Loss];
}
// crate::retrofit::slide
pub(crate) struct SlideCx<'a> {
    pub registry: &'a WidgetRegistry, pub page: &'a PageTokens, pub content: ContentCx<'a>, pub layouts: &'a [Layout],
    pub size: Size, pub scale: f64, pub decor: &'a Decor,
}
pub(crate) struct Projected<'a> {
    pub ops: Vec<Op>, pub report: SlideReport, pub assets: BTreeSet<String>, pub fallbacks: Vec<(&'a Item, String)>,
}
pub(crate) fn project_slide<'a>(src: &'a SourceSlide, profiles: &[Profile<'_>], deck_titled: &mut bool, cx: &SlideCx<'_>) -> Option<Projected<'a>>;
// crate::test_support (cfg(test))
pub(crate) fn applied(ops: Vec<Op>, assets: &[String]) -> Deck;
```

- [ ] **Step 1 : tests qui échouent**

Dans `crates/oku-slides-import/src/test_support.rs`, la doc et les imports deviennent :

```rust
//! Aides de test du projecteur : jetons effectifs des thèmes intégrés, texte, items,
//! slides, layouts et decks source, registre de base, application d'une slide ou d'un
//! plan par les ops du cœur, modèles des packs intégrés.

use std::collections::{BTreeMap, HashSet};

use oku_slides_core::tokens::effective_tokens;
use oku_slides_core::{Deck, Element, Op, ParamValue, Slide, Template, WidgetRegistry, apply_all, validate};
use serde_json::{Map, Value};

use crate::retrofit::ColorTokens;
use crate::source::{
    Align, Bullet, Caps, Color, Fill, Frame, Item, ItemKind, Layout, ParaStyle, Paragraph, Rgba, Role, Run, RunStyle,
    Size, SourceDeck, SourceSlide, Spacing, Styled, TextBox,
};
```

puis, après `source_deck`, ajouter :

```rust
/// Le deck obtenu par `ops` sur un deck vide et le registre de base ; panique
/// si une op est refusée ou si le deck lève une `Issue` (assets connus : `assets`).
pub(crate) fn applied(ops: Vec<Op>, assets: &[String]) -> Deck {
    let registry = registry();
    let mut deck = Deck::empty();
    apply_all(&mut deck, ops, &registry).unwrap_or_else(|e| panic!("ops refusées : {e:?}"));
    let known: HashSet<String> = assets.iter().cloned().collect();
    let issues = validate(&deck, &registry, Some(&known));
    assert!(issues.is_empty(), "issues : {issues:?}");
    deck
}
```

`crates/oku-slides-import/src/retrofit/decor_tests.rs` :

```rust
use speculoos::prelude::*;

use super::*;
use crate::source::Picture;
use crate::test_support::{item, source_deck, source_slide, text_item, with_role};

/// Page 960 × 540 : bande haute sous 64,8 pt, bande basse au-delà de 475,2 pt (centres).
fn footer(slide: u32) -> Item {
    text_item(&format!("s{slide}/f"), (64.0, 497.0, 270.0, 12.0), &format!("{slide} · Wealth Bank as an API"))
}

fn logo(slide: u32) -> Item {
    let mut i = item(
        &format!("s{slide}/logo"),
        (824.0, 484.0, 72.0, 32.0),
        ItemKind::Picture(Picture { asset: "sha256:logo".into(), crop: Default::default() }),
    );
    i.name = "Amundi".into();
    i
}

/// Quatre slides : pied numéroté et logo sur les slides 2 à 4 (3 ≥ la moitié de 4),
/// un en-tête propre à la slide 1, un numéro seul en slide 4, une date en slide 2.
fn deck() -> SourceDeck {
    let body = |s: u32| text_item(&format!("s{s}/body"), (64.0, 200.0, 800.0, 100.0), "Corps");
    source_deck(vec![
        source_slide(1, None, vec![text_item("s1/brand", (64.0, 40.0, 120.0, 20.0), "ALTO WEALTH"), body(1)]),
        source_slide(
            2,
            None,
            vec![
                body(2),
                footer(2),
                logo(2),
                with_role(text_item("s2/date", (0.0, 200.0, 100.0, 20.0), "10/10/2026"), Role::Date),
            ],
        ),
        source_slide(3, None, vec![body(3), footer(3), logo(3)]),
        source_slide(4, None, vec![body(4), footer(4), logo(4), text_item("s4/n", (64.0, 494.0, 10.0, 16.0), "7")]),
    ])
}

#[test]
fn decor_skips_repeated_header_and_footer_items_and_placeholders() {
    let d = deck();
    let decor = Decor::of(&d);
    let skipped: Vec<&str> =
        d.slides.iter().flat_map(|s| &s.items).filter(|i| decor.skips(i)).map(|i| i.id.as_str()).collect();
    assert_that!(skipped).is_equal_to(vec!["s2/f", "s2/logo", "s2/date", "s3/f", "s3/logo", "s4/f", "s4/logo", "s4/n"]);
}

#[test]
fn decor_reports_each_repeated_motif_once_on_its_first_slide() {
    let decor = Decor::of(&deck());
    // Motifs dans l'ordre de leur clé : `image:…` avant le texte.
    assert_that!(decor.losses(2).to_vec()).is_equal_to(vec![
        Loss::on("s2/logo", LossKind::RepeatedDecor { text: "Amundi".into(), slides: 3 }),
        Loss::on("s2/f", LossKind::RepeatedDecor { text: "2 · Wealth Bank as an API".into(), slides: 3 }),
    ]);
    assert_that!(decor.losses(3).is_empty() && decor.losses(1).is_empty()).is_true();
}

/// Sur deux slides seulement, un pied ne fait pas la moitié de cinq : il reste.
#[test]
fn decor_keeps_a_band_item_repeated_on_fewer_than_half_of_the_slides() {
    let mut d = deck();
    d.slides.push(source_slide(5, None, Vec::new()));
    d.slides.push(source_slide(6, None, Vec::new()));
    d.slides[3].items.retain(|i| i.id != "s4/f");
    let decor = Decor::of(&d);
    assert_that!(decor.skips(&footer(2))).is_false();
}

#[test]
fn decor_keeps_an_item_outside_the_bands() {
    let decor = Decor::of(&deck());
    let body = text_item("s2/body", (64.0, 200.0, 800.0, 100.0), "Corps");
    assert_that!(decor.skips(&body)).is_false();
}

#[test]
fn in_band_and_backdrop_follow_the_page() {
    let page = Size { w: 960.0, h: 540.0 };
    assert_that!(in_band(&Frame::new(0.0, 50.0, 10.0, 20.0), page)).is_true();
    assert_that!(in_band(&Frame::new(0.0, 60.0, 10.0, 20.0), page)).is_false();
    let panel = item(
        "p",
        (0.0, 108.0, 960.0, 432.0),
        ItemKind::Shape(Box::new(crate::source::Shape {
            geom: crate::source::Geometry::Preset { name: "rect".into(), adj: Default::default() },
            fill: crate::source::Fill::None,
            stroke: None,
            text: None,
        })),
    );
    assert_that!(is_backdrop(&panel, page)).is_true();
}
```

> Page 960 × 540 : bandes sous 64,8 pt et au-delà de 475,2 pt (centres). Le pied (centre 503) et le logo (centre 500) reviennent sur 3 slides de 4 (au moins `⌈4 / 2⌉ = 2`) ; « ALTO WEALTH » (centre 50) n'est que sur la slide 1 : gardé ; « 7 » n'a plus ni lettre ni chiffre une fois les chiffres ôtés : un numéro de page.

`crates/oku-slides-import/src/retrofit/slide_tests.rs` :

```rust
use std::collections::BTreeMap;

use oku_slides_core::{Cell, Deck, Template, asset_id};
use speculoos::prelude::*;

use super::*;
use crate::report::RunEffect;
use crate::retrofit::template::profiles;
use crate::source::{Asset, Fill, Geometry, ItemKind, Picture, Rgba, Role, Styled};
use crate::test_support::{
    PAPIER, applied, colors, essentiel, grilles, item, layout, para, registry, run_with, source_deck, source_slide,
    text_box, text_item, with_role,
};

/// Layouts des slides de test : couverture, contenu, chapitre.
fn layouts() -> Vec<Layout> {
    vec![layout("l1", "title"), layout("l2", "obj"), layout("l3", "secHead")]
}

fn png() -> String {
    asset_id(b"png")
}

/// Les slides `slides` projetées sur `templates` et Papier, dans l'ordre ; le
/// deck obtenu par `apply_all` (aucune `Issue`).
fn project(templates: &[Template], slides: &[SourceSlide]) -> (Vec<Op>, Vec<SlideReport>, Deck) {
    let (registry, tokens, layouts) = (registry(), colors(PAPIER), layouts());
    let assets = BTreeMap::from([(png(), Asset { mime: "image/png".into(), bytes: b"png".to_vec() })]);
    let page = PageTokens::default();
    let decor = Decor::of(&source_deck(slides.to_vec()));
    let cx = SlideCx {
        registry: &registry,
        page: &page,
        content: ContentCx { tokens: &tokens, assets: &assets, track: (0.0, 0.0) },
        layouts: &layouts,
        size: Size { w: 960.0, h: 540.0 },
        scale: 2.0,
        decor: &decor,
    };
    let profiles = profiles(templates, &registry);
    let mut titled = false;
    let (mut ops, mut reports) = (Vec::new(), Vec::new());
    for s in slides {
        let p = project_slide(s, &profiles, &mut titled, &cx).unwrap();
        ops.extend(p.ops);
        reports.push(p.report);
    }
    let deck = applied(ops.clone(), &[png()]);
    (ops, reports, deck)
}

fn title(text: &str) -> Item {
    with_role(text_item("t", (66.0, 28.0, 828.0, 104.0), text), Role::Title)
}

fn subtitle(text: &str) -> Item {
    with_role(text_item("st", (120.0, 283.0, 720.0, 130.0), text), Role::Subtitle)
}

fn cover(number: u32, t: &str, st: &str) -> SourceSlide {
    source_slide(number, Some("l1"), vec![title(t), subtitle(st)])
}

fn block_texts(slide: &Slide, slot: &str) -> Vec<String> {
    slide
        .params
        .slot(slot)
        .unwrap_or_default()
        .iter()
        .map(|e| {
            let body = e.params.rich("body").unwrap_or_default();
            body.iter().flat_map(|p| &p.runs).map(|r| r.t.as_str()).collect()
        })
        .collect()
}

#[test]
fn project_slide_first_cover_fills_the_deck_params() {
    let (ops, reports, deck) = project(&essentiel(), &[cover(1, "Rapport annuel", "Exercice 2026")]);
    assert_that!(ops[..2].to_vec()).is_equal_to(vec![
        Op::SetDeckParam { key: "title".into(), value: "Rapport annuel".into() },
        Op::SetDeckParam { key: "subtitle".into(), value: "Exercice 2026".into() },
    ]);
    assert_that!((deck.slides[0].kind.as_str(), deck.slides[0].params.is_empty())).is_equal_to(("title", true));
    assert_that!((deck.params.title.as_deref(), reports[0].template.as_str()))
        .is_equal_to((Some("Rapport annuel"), "couverture"));
}

#[test]
fn project_slide_later_cover_becomes_a_chapter() {
    let (_, reports, deck) = project(&essentiel(), &[cover(1, "Rapport", "2026"), cover(2, "Partie 2", "Les comptes")]);
    let s2 = &deck.slides[1];
    assert_that!((s2.kind.as_str(), s2.params.text("title"), s2.params.text("subtitle"))).is_equal_to((
        "chapter",
        Some("Partie 2"),
        Some("Les comptes"),
    ));
    assert_that!((deck.params.title.as_deref(), reports[1].template.as_str()))
        .is_equal_to((Some("Rapport"), "chapitre"));
}

#[test]
fn project_slide_title_and_body_go_to_flow_blocks() {
    let body = text_item("b", (66.0, 143.0, 828.0, 342.0), "Chiffre d'affaires en hausse");
    let (_, reports, deck) = project(&essentiel(), &[source_slide(2, Some("l2"), vec![title("Résultats"), body])]);
    let s = &deck.slides[0];
    assert_that!((s.id.as_str(), s.kind.as_str(), s.params.text("title"))).is_equal_to((
        "s2",
        "content",
        Some("Résultats"),
    ));
    assert_that!(block_texts(s, "blocks")).is_equal_to(vec!["Chiffre d'affaires en hausse".to_owned()]);
    assert_that!(s.params.slot("blocks").unwrap()[0].id.as_str()).is_equal_to("s2-1");
    assert_that!((reports[0].template.as_str(), reports[0].placement)).is_equal_to(("texte", Placement::Flow));
}

#[test]
fn project_slide_two_columns_keep_the_split_layout_in_reading_order() {
    let right = text_item("r", (500.0, 143.0, 394.0, 300.0), "Droite");
    let left = text_item("l", (66.0, 143.0, 394.0, 300.0), "Gauche");
    let (_, reports, deck) =
        project(&essentiel(), &[source_slide(1, Some("l2"), vec![title("Avant, après"), right, left])]);
    let s = &deck.slides[0];
    assert_that!(s.params.text("layout")).is_equal_to(Some("split"));
    assert_that!(block_texts(s, "blocks")).is_equal_to(vec!["Gauche".to_owned(), "Droite".to_owned()]);
    assert_that!(reports[0].template.as_str()).is_equal_to("colonnes");
}

#[test]
fn project_slide_scattered_items_snap_to_the_template_grid() {
    let boxes = [(60.0, 125.0), (500.0, 125.0), (60.0, 300.0), (500.0, 300.0)];
    let mut items = vec![title("Quatre idées")];
    items.extend(
        boxes.iter().enumerate().map(|(i, (x, y))| text_item(&format!("i{i}"), (*x, *y, 400.0, 150.0), "Idée")),
    );
    let (_, reports, deck) = project(&grilles(), &[source_slide(1, Some("l2"), items)]);
    let s = &deck.slides[0];
    let cells: Vec<Option<Cell>> = s.params.slot("elements").unwrap().iter().map(|e| e.cell).collect();
    let cell = |col, row| Some(Cell { col, row, cols: 1, rows: 1 });
    assert_that!(cells).is_equal_to(vec![cell(0, 0), cell(1, 0), cell(0, 1), cell(1, 1)]);
    assert_that!(s.params.value("grid")).is_equal_to(Some(&serde_json::json!({ "columns": 2, "rows": 2 })));
    assert_that!((reports[0].template.as_str(), reports[0].placement)).is_equal_to(("grille-quatre", Placement::Grid));
}

#[test]
fn project_slide_notes_become_a_set_notes_op() {
    let mut s = source_slide(3, Some("l2"), vec![title("Résultats")]);
    s.notes = "Dire bonjour.".into();
    let (ops, _, deck) = project(&essentiel(), &[s]);
    assert_that!(ops.last()).is_equal_to(Some(&Op::SetNotes { slide: "s3".into(), notes: "Dire bonjour.".into() }));
    assert_that!(deck.slides[0].notes.as_str()).is_equal_to("Dire bonjour.");
}

#[test]
fn project_slide_hidden_slide_is_imported_with_a_loss() {
    let mut s = source_slide(1, Some("l2"), vec![title("Annexe")]);
    s.hidden = true;
    let (_, reports, deck) = project(&essentiel(), &[s]);
    assert_that!(deck.slides.len()).is_equal_to(1);
    assert_that!(reports[0].losses.clone()).is_equal_to(vec![Loss::slide(LossKind::HiddenSlide)]);
}

#[test]
fn project_slide_date_footer_and_number_are_skipped() {
    let items = vec![
        title("Résultats"),
        text_item("b", (66.0, 143.0, 828.0, 342.0), "Corps"),
        with_role(text_item("d", (66.0, 500.0, 200.0, 20.0), "10/10/2026"), Role::Date),
        with_role(text_item("f", (300.0, 500.0, 300.0, 20.0), "Confidentiel"), Role::Footer),
        with_role(text_item("n", (800.0, 500.0, 60.0, 20.0), "3"), Role::SlideNum),
    ];
    let (_, _, deck) = project(&essentiel(), &[source_slide(1, Some("l2"), items)]);
    assert_that!(block_texts(&deck.slides[0], "blocks")).is_equal_to(vec!["Corps".to_owned()]);
}

#[test]
fn project_slide_group_falls_back_to_a_stack_with_child_losses_on_source_ids() {
    let underlined = run_with("Droite", |s| s.underline = true);
    let children = vec![
        text_item("g/1", (66.0, 432.0, 200.0, 40.0), "Gauche"),
        bare("g/arrow", "rightArrow", (270.0, 442.0, 26.0, 20.0)),
        item("g/2", (300.0, 432.0, 200.0, 40.0), ItemKind::Text(text_box(vec![para(vec![underlined])]))),
    ];
    let group = item("g", (66.0, 432.0, 434.0, 40.0), ItemKind::Group { items: children, smartart: false });
    let (_, reports, deck) = project(&essentiel(), &[source_slide(1, Some("l2"), vec![title("Processus"), group])]);
    let stack = &deck.slides[0].params.slot("blocks").unwrap()[0];
    let ids: Vec<&str> = stack.params.slot("children").unwrap().iter().map(|c| c.id.as_str()).collect();
    assert_that!((stack.kind.as_str(), ids)).is_equal_to(("stack", vec!["s1-1-1", "s1-1-2"]));
    assert_that!(reports[0].fallbacks.clone()).is_equal_to(vec!["stack".to_owned()]);
    assert_that!(reports[0].losses.clone())
        .is_equal_to(vec![Loss::on("g/2", LossKind::RunStyle(RunEffect::Underline))]);
}

#[test]
fn project_slide_subtitle_without_a_param_becomes_content() {
    let (_, reports, deck) = project(&grilles(), &[cover(1, "Rapport annuel", "Exercice 2026")]);
    let s = &deck.slides[0];
    assert_that!((s.kind.as_str(), s.params.text("title"))).is_equal_to(("content", Some("Rapport annuel")));
    assert_that!(block_texts(s, "elements")).is_equal_to(vec!["Exercice 2026".to_owned()]);
    assert_that!((deck.params.subtitle.as_deref(), reports[0].template.as_str()))
        .is_equal_to((Some("Exercice 2026"), "grille-vide"));
}

#[test]
fn project_slide_cover_with_content_avoids_templates_without_a_slot() {
    let logo =
        item("p", (400.0, 420.0, 160.0, 90.0), ItemKind::Picture(Picture { asset: png(), crop: Default::default() }));
    let (_, reports, deck) = project(&essentiel(), &[source_slide(1, Some("l1"), vec![title("Rapport"), logo])]);
    let s = &deck.slides[0];
    assert_that!((s.kind.as_str(), s.params.text("title"))).is_equal_to(("content", Some("Rapport")));
    assert_that!(s.params.slot("blocks").unwrap()[0].params.text("asset")).is_equal_to(Some(png().as_str()));
    assert_that!((deck.params.title.as_deref(), reports[0].template.as_str())).is_equal_to((Some("Rapport"), "texte"));
}

#[test]
fn project_slide_collects_the_assets_of_its_images() {
    let registry = registry();
    let tokens = colors(PAPIER);
    let assets = BTreeMap::from([(png(), Asset { mime: "image/png".into(), bytes: b"png".to_vec() })]);
    let (page, layouts, templates, decor) = (PageTokens::default(), layouts(), essentiel(), Decor::default());
    let cx = SlideCx {
        registry: &registry,
        page: &page,
        content: ContentCx { tokens: &tokens, assets: &assets, track: (0.0, 0.0) },
        layouts: &layouts,
        size: Size { w: 960.0, h: 540.0 },
        scale: 2.0,
        decor: &decor,
    };
    let pic =
        item("p", (66.0, 143.0, 300.0, 200.0), ItemKind::Picture(Picture { asset: png(), crop: Default::default() }));
    let src = source_slide(1, Some("l2"), vec![title("Image"), pic]);
    let p = project_slide(&src, &profiles(&templates, &registry), &mut false, &cx).unwrap();
    assert_that!(p.assets.into_iter().collect::<Vec<_>>()).is_equal_to(vec![png()]);
}

#[test]
fn project_slide_title_run_effects_are_lost_on_the_title_item() {
    let t = with_role(
        item(
            "t",
            (66.0, 28.0, 828.0, 104.0),
            ItemKind::Text(text_box(vec![para(vec![run_with("Barré", |s| s.strike = true)])])),
        ),
        Role::Title,
    );
    let (_, reports, _) = project(
        &essentiel(),
        &[source_slide(1, Some("l2"), vec![t, text_item("b", (66.0, 143.0, 828.0, 342.0), "Corps")])],
    );
    assert_that!(reports[0].losses.clone()).is_equal_to(vec![Loss::on("t", LossKind::RunStyle(RunEffect::Strike))]);
}

/// Une forme sans texte remplie de blanc.
fn bare(id: &str, preset: &str, frame: (f64, f64, f64, f64)) -> Item {
    let s = crate::source::Shape {
        geom: Geometry::Preset { name: preset.into(), adj: BTreeMap::new() },
        fill: Fill::Solid { color: Styled::explicit(Rgba::new(255, 255, 255)) },
        stroke: None,
        text: None,
    };
    item(id, frame, ItemKind::Shape(Box::new(s)))
}

/// Une boîte de texte d'un paragraphe en taille `size`.
fn sized(id: &str, frame: (f64, f64, f64, f64), text: &str, size: f64) -> Item {
    item(id, frame, ItemKind::Text(text_box(vec![para(vec![run_with(text, |s| s.size = size)])])))
}

/// Une slide d'alto sans placeholder : surtitre, titre 28 pt, filet d'accent,
/// trois cartes (fond, titre 13 pt, corps 10,5 pt).
fn conviction() -> SourceSlide {
    let mut items = vec![
        sized("k", (64.0, 56.0, 857.0, 15.0), "OUR CONVICTION", 9.5),
        bare("bar", "rect", (64.0, 78.0, 32.0, 2.5)),
        sized("t", (64.0, 90.0, 772.0, 66.0), "The agent becomes the only interface", 28.0),
    ];
    for (k, x) in [64.0, 341.0, 619.0].into_iter().enumerate() {
        items.push(bare(&format!("c{k}"), "rect", (x, 176.0, 277.0, 284.0)));
        items.push(sized(&format!("h{k}"), (x + 16.0, 196.0, 253.0, 18.0), &format!("Carte {k}"), 13.0));
        items.push(sized(&format!("b{k}"), (x + 16.0, 220.0, 253.0, 69.0), &format!("Corps {k}"), 10.5));
    }
    source_slide(2, None, items)
}

#[test]
fn project_slide_without_placeholders_infers_its_title_and_kicker_and_makes_cards() {
    let (_, reports, deck) = project(&essentiel(), &[conviction()]);
    let s = &deck.slides[0];
    assert_that!((s.params.text("kicker"), s.params.text("title")))
        .is_equal_to((Some("OUR CONVICTION"), Some("The agent becomes the only interface")));
    let blocks = s.params.slot("blocks").unwrap();
    let cards: Vec<(&str, Option<&str>)> = blocks.iter().map(|e| (e.kind.as_str(), e.params.text("title"))).collect();
    assert_that!(cards).is_equal_to(vec![
        ("card", Some("Carte 0")),
        ("card", Some("Carte 1")),
        ("card", Some("Carte 2")),
    ]);
    assert_that!(block_texts(s, "blocks")).is_equal_to(vec![
        "Corps 0".to_owned(),
        "Corps 1".to_owned(),
        "Corps 2".to_owned(),
    ]);
    assert_that!((reports[0].template.as_str(), s.params.text("layout"))).is_equal_to(("cartes", Some("columns")));
}

/// Une introduction et un bandeau pleine largeur autour de la rangée : agencement
/// `default`, la rangée en pile de colonnes à sa place de lecture.
#[test]
fn project_slide_row_between_wide_units_becomes_a_stack_of_columns() {
    let mut slide = conviction();
    slide.items.push(sized("intro", (64.0, 160.0, 772.0, 12.0), "Introduction", 10.5));
    for card in
        slide.items.iter_mut().filter(|i| i.id.starts_with('c') || i.id.starts_with('h') || i.id.starts_with('b'))
    {
        card.frame.y += 20.0;
        card.frame.h = card.frame.h.min(200.0);
    }
    slide.items.push(bare("banner", "rect", (64.0, 410.0, 832.0, 60.0)));
    slide.items.push(sized("result", (96.0, 420.0, 612.0, 20.0), "Résultat", 12.0));
    let (_, reports, deck) = project(&essentiel(), &[slide]);
    let s = &deck.slides[0];
    let blocks = s.params.slot("blocks").unwrap();
    let kinds: Vec<(&str, &str)> = blocks.iter().map(|e| (e.id.as_str(), e.kind.as_str())).collect();
    assert_that!(kinds).is_equal_to(vec![("s2-1", "text"), ("s2-2", "stack"), ("s2-3", "card")]);
    let row: Vec<&str> = blocks[1].params.slot("children").unwrap().iter().map(|c| c.id.as_str()).collect();
    assert_that!(row).is_equal_to(vec!["s2-2-1", "s2-2-2", "s2-2-3"]);
    assert_that!((reports[0].template.as_str(), s.params.text("layout"))).is_equal_to(("cartes", Some("default")));
}

/// `libre` n'a pas de grille : les unités sont posées en cadres, leur boîte
/// englobante ajustée dans la zone de corps.
#[test]
fn project_slide_on_a_free_template_places_frames_not_cells() {
    let items = vec![
        title("Libre"),
        text_item("a", (66.0, 150.0, 300.0, 60.0), "A"),
        text_item("b", (500.0, 260.0, 300.0, 60.0), "B"),
        text_item("c", (66.0, 380.0, 700.0, 60.0), "C"),
    ];
    let (_, reports, deck) = project(&essentiel(), &[source_slide(1, Some("l2"), items)]);
    let elements = deck.slides[0].params.slot("elements").unwrap();
    assert_that!(reports[0].template.as_str()).is_equal_to("libre");
    assert_that!(elements.iter().all(|e| e.frame.is_some() && e.cell.is_none())).is_true();
}

/// Un modèle sans aucun slot (une couverture seule dans le pack) : le contenu
/// est dit perdu, jamais omis en silence. Le titre, que le widget `title` lie au
/// deck sans être une couverture, devient un contenu lui aussi.
#[test]
fn project_slide_content_without_a_slot_is_reported() {
    let cover_only: Vec<Template> = essentiel().into_iter().filter(|t| t.id == "couverture").collect();
    let body = text_item("b", (66.0, 143.0, 828.0, 342.0), "Corps");
    let (_, reports, _) = project(&cover_only, &[source_slide(1, Some("l2"), vec![title("Seul"), body])]);
    assert_that!(reports[0].losses.clone())
        .is_equal_to(vec![Loss::on("t", LossKind::NoSlot), Loss::on("b", LossKind::NoSlot)]);
}

#[test]
fn project_slide_never_flows_a_bare_shape() {
    let items = vec![title("Formes"), bare("deco", "ellipse", (400.0, 200.0, 100.0, 100.0))];
    let (_, _, deck) = project(&essentiel(), &[source_slide(1, Some("l2"), items)]);
    assert_that!(deck.slides[0].params.slot("blocks")).is_none();
}
```

> Grille `grille-quatre` (2 × 2) sur Papier : source 960 × 540, échelle 2 ; zone titrée `x = 120, y = 150 + 100 = 250, w = 1680, h = 1080 − 150 − 140 − 100 = 690`. Les quatre boîtes `(60, 125)`… `(500, 300)` de 400 × 150 ont pour englobante 840 × 325 pt, 1 680 × 650 à l'échelle 2 : elle tient, centrée (20 pt au-dessus et au-dessous) ; pistes `(1680 − 40) / 2 = 820` de large, `(690 − 40) / 2 = 325` de haut : chaque boîte tombe dans sa case, sans recouvrement : cellules. Groupe : la flèche omise fait dire le repli ; les ids des enfants sont `s1-1-1`, `s1-1-2` (élément `s1-1`), la perte de soulignement reste sur l'item source `g/2`. `conviction()` (slide 2 d'alto sans placeholder) : titre 28 pt et surtitre déduits, trois cartes en rangée → `cartes`, layout `columns` ; avec une introduction et un bandeau pleine largeur, la rangée devient la pile `s2-2` et la slide `layout: default`. Couverture sur Grilles : `grille-vide` (aucune couverture dans ce pack) tient le titre, pas le sous-titre : il devient un élément libre, et le sous-titre du deck est rempli quand même.

`crates/oku-slides-import/src/retrofit/mod_tests.rs` :

```rust
use std::collections::BTreeMap;

use oku_slides_core::{Op, asset_id};
use speculoos::prelude::*;

use super::*;
use crate::source::{Asset, Picture, Size, SourceSlide, Warning, WarningKind};
use crate::test_support::{PAPIER, applied, colors, essentiel, item, layout, registry, source_slide, text_item};

fn png() -> String {
    asset_id(b"png")
}

fn jpeg() -> String {
    asset_id(b"jpeg")
}

fn deck(slides: Vec<SourceSlide>) -> SourceDeck {
    SourceDeck {
        size: Size { w: 960.0, h: 540.0 },
        themes: Vec::new(),
        masters: Vec::new(),
        layouts: vec![layout("l1", "title"), layout("l2", "obj")],
        slides,
        assets: BTreeMap::from([
            (png(), Asset { mime: "image/png".into(), bytes: b"png".to_vec() }),
            (jpeg(), Asset { mime: "image/jpeg".into(), bytes: b"jpeg".to_vec() }),
        ]),
        fonts: Vec::new(),
        warnings: Vec::new(),
    }
}

fn plan_of(source: &SourceDeck, templates: &[Template]) -> ImportPlan {
    let registry = registry();
    let target = RetrofitTarget { templates, registry: &registry, colors: colors(PAPIER), page: PageTokens::default() };
    project(source, &target)
}

/// Un groupe de trois boîtes de texte en frise, à `(x, y)`, de largeur `w` par boîte.
fn frieze(id: &str, (x, y): (f64, f64), w: f64) -> Item {
    let children =
        (0..3).map(|i| text_item(&format!("{id}/{i}"), (x + f64::from(i) * 1.5 * w, y, w, w / 2.0), "Étape")).collect();
    item(id, (x, y, 3.5 * w, w / 2.0), ItemKind::Group { items: children, smartart: false })
}

#[test]
fn project_names_slides_by_number_and_keeps_only_referenced_assets() {
    let pic =
        item("p", (66.0, 143.0, 300.0, 200.0), ItemKind::Picture(Picture { asset: png(), crop: Default::default() }));
    let source = deck(vec![source_slide(1, Some("l2"), vec![pic]), source_slide(4, Some("l2"), Vec::new())]);
    let plan = plan_of(&source, &essentiel());
    let ids: Vec<&str> = plan.report.slides.iter().map(|s| s.slide.as_str()).collect();
    assert_that!(ids).is_equal_to(vec!["s1", "s4"]);
    assert_that!(plan.assets.keys().cloned().collect::<Vec<_>>()).is_equal_to(vec![png()]);
    applied(plan.ops, &[png()]);
}

#[test]
fn project_groups_widget_requests_by_signature_across_slides() {
    let source = deck(vec![
        source_slide(1, Some("l2"), vec![frieze("a", (60.0, 100.0), 100.0)]),
        source_slide(2, Some("l2"), vec![text_item("x", (60.0, 100.0, 400.0, 100.0), "Seul")]),
        source_slide(3, Some("l2"), vec![frieze("b", (200.0, 300.0), 50.0)]),
    ]);
    let plan = plan_of(&source, &essentiel());
    let requests: Vec<(&str, Vec<String>, usize)> =
        plan.widget_requests.iter().map(|r| (r.fallback.as_str(), r.slides.clone(), r.sample.len())).collect();
    assert_that!(requests).is_equal_to(vec![("stack", vec!["s1".to_owned(), "s3".to_owned()], 3)]);
    assert_that!(plan.widget_requests[0].signature.as_str()).is_equal_to("text×3|0014,2034,3044");
    applied(plan.ops, &[]);
}

#[test]
fn project_copies_the_reader_warnings() {
    let mut source = deck(Vec::new());
    let warning = Warning { slide: Some(2), item: None, kind: WarningKind::UnsupportedChart("bubbleChart".into()) };
    source.warnings.push(warning.clone());
    assert_that!(plan_of(&source, &essentiel()).report.warnings).is_equal_to(vec![warning]);
}

#[test]
fn project_with_an_empty_pack_makes_bare_content_slides() {
    let source = deck(vec![source_slide(1, Some("l2"), vec![text_item("b", (66.0, 143.0, 828.0, 342.0), "Corps")])]);
    let plan = plan_of(&source, &[]);
    let Some(Op::AddSlide { slide, .. }) = plan.ops.first() else { panic!("AddSlide attendu") };
    assert_that!((slide.kind.as_str(), plan.report.slides[0].template.as_str())).is_equal_to(("content", "content"));
    applied(plan.ops, &[]);
}

#[test]
fn project_is_deterministic() {
    let source = deck(vec![source_slide(1, Some("l2"), vec![frieze("a", (60.0, 100.0), 100.0)])]);
    assert_that!(plan_of(&source, &essentiel())).is_equal_to(plan_of(&source, &essentiel()));
}
```

> Signature de la frise (boîtes de largeur `w` à `0`, `1,5 w`, `3 w`, englobante `4 w` × `w / 2`) : bords x ramenés au quart `0 → 0`, `1,5 / 4 × 4 = 1,5 → 2`, `3 → 3` ; bords droits `1`, `2,5 → 3`, `4` ; y toujours `0` et `4` : `text×3|0014,2034,3044`, la même à la slide 3 (translatée, deux fois plus petite). Trois boîtes semblables également espacées : un motif régulier, le repli est dit.

Ajouter à la fin de `crates/oku-slides-import/tests/common/mod.rs` :

```rust
/// Le registre des widgets de base (`fixtures/widgets/base-registry.json`).
pub fn registry() -> oku_slides_core::WidgetRegistry {
    let text = fs::read_to_string(root().join("../widgets/base-registry.json")).unwrap_or_else(|e| panic!("{e}"));
    oku_slides_core::WidgetRegistry::from_probe(serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}")))
        .unwrap_or_else(|e| panic!("{e}"))
}

/// Les modèles du pack intégré `name` (`templates/<name>/pack.json`), dans l'ordre du pack.
pub fn templates(name: &str) -> Vec<oku_slides_core::Template> {
    let path = root().join(format!("../../templates/{name}/pack.json"));
    let pack: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
            .unwrap_or_else(|e| panic!("{e}"));
    serde_json::from_value(pack["templates"].clone()).unwrap_or_else(|e| panic!("{e}"))
}

/// Les jetons effectifs du thème intégré `name` (`papier`, `pro`) : les défauts, puis les siens.
pub fn tokens(name: &str) -> serde_json::Map<String, serde_json::Value> {
    let typst = root().join("../../typst/oku-slides");
    let read = |p: PathBuf| -> serde_json::Value {
        serde_json::from_str(&fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display())))
            .unwrap_or_else(|e| panic!("{e}"))
    };
    let serde_json::Value::Object(defaults) = read(typst.join("src/default-tokens.json")) else { panic!("jetons") };
    let theme = read(typst.join(format!("themes/{name}.json")));
    oku_slides_core::tokens::effective_tokens(
        &defaults,
        theme["tokens"].as_object().unwrap_or_else(|| panic!("tokens de {name}")),
    )
}
```

`crates/oku-slides-import/tests/retrofit.rs` :

```rust
//! Le retrofit de bout en bout : les fixtures `minimal`, `kitchen` et le vrai
//! deck `alto-wealth.pptx`, projetés sur les packs intégrés (Essentiel,
//! Grilles) et les thèmes intégrés (Papier, Pro). Le plan s'applique en une
//! transaction et le deck obtenu ne lève aucune `Issue`.

mod common;

use std::collections::HashSet;

use oku_slides_core::{Deck, Element, Op, Template, WidgetRegistry, apply_all, validate};
use oku_slides_import::report::LossKind;
use oku_slides_import::retrofit::{ColorTokens, PageTokens, project};
use oku_slides_import::source::SourceDeck;
use oku_slides_import::{ImportPlan, RetrofitTarget, read_pptx};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

fn source(fixture: &str) -> SourceDeck {
    let bytes = match fixture {
        "alto" => common::file("alto-wealth.pptx"),
        name => common::pptx(name),
    };
    read_pptx(&bytes, &common::limits()).unwrap_or_else(|e| panic!("lecture de {fixture} : {e}"))
}

/// Le plan de `fixture` sur le pack `pack` et le thème `theme`, et le deck obtenu.
fn import(fixture: &str, pack: &str, theme: &str) -> (ImportPlan, Deck) {
    let (registry, templates, tokens): (WidgetRegistry, Vec<Template>, _) =
        (common::registry(), common::templates(pack), common::tokens(theme));
    let target = RetrofitTarget {
        templates: &templates,
        registry: &registry,
        colors: ColorTokens::from_tokens(&tokens),
        page: PageTokens::from_tokens(&tokens),
    };
    let plan = project(&source(fixture), &target);
    let mut deck = Deck::empty();
    apply_all(&mut deck, plan.ops.clone(), &registry).unwrap_or_else(|e| panic!("{fixture}/{pack}/{theme} : {e:?}"));
    let assets: HashSet<String> = plan.assets.keys().cloned().collect();
    let issues = validate(&deck, &registry, Some(&assets));
    assert!(issues.is_empty(), "{fixture}/{pack}/{theme} : {issues:?}");
    (plan, deck)
}

fn templates_chosen(plan: &ImportPlan) -> Vec<&str> {
    plan.report.slides.iter().map(|s| s.template.as_str()).collect()
}

/// Les sortes des éléments de `list` et de leurs descendants.
fn kinds(list: &[Element], out: &mut Vec<String>) {
    for e in list {
        out.push(e.kind.clone());
        for (_, children) in e.params.slots() {
            kinds(children, out);
        }
    }
}

fn deck_kinds(deck: &Deck) -> Vec<String> {
    let mut out = Vec::new();
    for s in &deck.slides {
        for (_, list) in s.params.slots() {
            kinds(list, &mut out);
        }
    }
    out
}

#[rstest]
fn retrofit_applies_without_issue(
    #[values("minimal", "kitchen", "alto")] fixture: &str,
    #[values("essentiel", "grilles")] pack: &str,
    #[values("papier", "pro")] theme: &str,
) {
    let (plan, deck) = import(fixture, pack, theme);
    assert_that!(deck.slides.len()).is_equal_to(source(fixture).slides.len());
    assert_that!(plan.report.slides.len()).is_equal_to(deck.slides.len());
}

#[test]
fn retrofit_minimal_fills_the_deck_title_and_picks_templates() {
    let (plan, deck) = import("minimal", "essentiel", "papier");
    assert_that!((deck.params.title.as_deref(), deck.params.subtitle.as_deref()))
        .is_equal_to((Some("Rapport annuel"), Some("Exercice 2026")));
    assert_that!(templates_chosen(&plan)).is_equal_to(vec!["couverture", "libre", "texte"]);
    assert_that!(deck.slides[1].notes.starts_with("Dire bonjour.")).is_true();
}

#[test]
fn retrofit_minimal_ops_match() {
    let (plan, _) = import("minimal", "essentiel", "papier");
    insta::assert_json_snapshot!("minimal_ops", plan.ops);
}

#[test]
fn retrofit_kitchen_keeps_table_chart_picture_and_fallbacks() {
    let (plan, deck) = import("kitchen", "essentiel", "papier");
    let kinds = deck_kinds(&deck);
    for k in ["table", "chart", "image", "stack", "card"] {
        assert_that!(kinds.iter().any(|x| x == k)).named(k).is_true();
    }
    // L'image de la slide 2 et le repli de l'équation de la slide 3 sont le même PNG : un seul asset.
    assert_that!(plan.assets.len()).is_equal_to(1);
    let requests: Vec<(Vec<String>, &str)> =
        plan.widget_requests.iter().map(|r| (r.slides.clone(), r.fallback.as_str())).collect();
    // Le groupe de la slide 2 (deux boîtes, rien d'omis) se replie sans demande ; le SmartArt de la slide 3 en fait une.
    assert_that!(requests).is_equal_to(vec![(vec!["s3".to_owned()], "stack")]);
}

#[test]
fn retrofit_kitchen_ops_match() {
    let (plan, _) = import("kitchen", "essentiel", "papier");
    insta::assert_json_snapshot!("kitchen_ops", plan.ops);
}

#[test]
fn retrofit_kitchen_on_grilles_places_on_cells_unless_they_collide() {
    let (plan, deck) = import("kitchen", "grilles", "pro");
    let placed: Vec<Vec<(bool, bool)>> = deck
        .slides
        .iter()
        .map(|s| {
            s.params
                .slot("elements")
                .unwrap_or_default()
                .iter()
                .map(|e| (e.cell.is_some(), e.frame.is_some()))
                .collect()
        })
        .collect();
    // Slide 2 : graphique, image et pile se recouvriraient sur la grille 3 × 2 de `grille-chiffres` : cadres.
    assert_that!(placed).is_equal_to(vec![vec![(true, false)], vec![(false, true); 3], vec![(true, false); 2]]);
    assert_that!(plan.report.slides.iter().all(|s| s.template.starts_with("grille-"))).is_true();
}

#[rstest]
fn retrofit_alto_infers_the_cover_and_the_titles(#[values("essentiel", "grilles")] pack: &str) {
    let (plan, deck) = import("alto", pack, "pro");
    assert_that!(deck.params.title.as_deref()).is_equal_to(Some("The Agent Is the New Front Door"));
    assert_that!(deck.params.subtitle.as_deref()).is_equal_to(Some("From Software Vendor to Wealth Infrastructure"));
    let titled: Vec<bool> = deck.slides[1..].iter().map(|s| s.params.text("title").is_some()).collect();
    assert_that!(titled).is_equal_to(vec![true; 7]);
    // Les flèches de la slide 3 restent des images ; le logo répété est omis.
    assert_that!(deck_kinds(&deck).iter().filter(|k| *k == "image").count()).is_equal_to(4);
    assert_that!(plan.assets.len()).is_equal_to(1);
}

/// Les éléments porteurs de texte (y compris leurs descendants) d'une liste libre.
fn texts_with_place(list: &[Element]) -> Vec<&Element> {
    list.iter().filter(|e| matches!(e.kind.as_str(), "text" | "card" | "stack" | "table" | "chart")).collect()
}

fn meet(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
}

/// Aucun texte n'en recouvre un autre : cellules (grilles) ou cadres (`libre`) disjoints.
#[rstest]
fn retrofit_alto_places_no_text_over_another(
    #[values("essentiel", "grilles")] pack: &str,
    #[values("papier", "pro")] theme: &str,
) {
    let (_, deck) = import("alto", pack, theme);
    for s in &deck.slides {
        let placed = texts_with_place(s.params.slot("elements").unwrap_or_default());
        let boxes: Vec<(f64, f64, f64, f64)> = placed
            .iter()
            .map(|e| match (e.cell, e.frame) {
                (Some(c), _) => (f64::from(c.col), f64::from(c.row), f64::from(c.cols), f64::from(c.rows)),
                (None, Some(f)) => (f.x, f.y, f.w, f.h),
                (None, None) => panic!("{} : élément libre sans place", e.id),
            })
            .collect();
        for (i, a) in boxes.iter().enumerate() {
            for (j, b) in boxes.iter().enumerate().skip(i + 1) {
                assert_that!(meet(*a, *b))
                    .named(&format!("{} : {} et {}", s.id, placed[i].id, placed[j].id))
                    .is_false();
            }
        }
    }
}

#[test]
fn retrofit_alto_makes_cards_and_picks_card_templates() {
    let (plan, deck) = import("alto", "essentiel", "papier");
    assert_that!(templates_chosen(&plan))
        .is_equal_to(vec!["libre", "cartes", "libre", "cartes", "colonnes", "cartes", "cartes", "cartes"]);
    let cards = deck_kinds(&deck).iter().filter(|k| *k == "card").count();
    // 3 + 6 + 4 + 3 + 4 + 4 + 4 cartes (slides 2 à 8).
    assert_that!(cards).is_equal_to(28);
}

#[test]
fn retrofit_alto_reports_its_repeated_footer_once() {
    let (plan, _) = import("alto", "essentiel", "pro");
    let repeated: Vec<(String, u32)> = plan
        .report
        .slides
        .iter()
        .flat_map(|s| &s.losses)
        .filter_map(|l| match &l.kind {
            LossKind::RepeatedDecor { text, slides } => Some((text.clone(), *slides)),
            _ => None,
        })
        .collect();
    assert_that!(repeated).is_equal_to(vec![
        ("CONFIDENTIAL".to_owned(), 8),
        ("Amundi".to_owned(), 7),
        ("1 · Wealth Bank as an API · September 20, 2026 · Matthieu Dartiguenave".to_owned(), 6),
    ]);
}

/// Résumé d'une slide : modèle, placement, titre, et chaque élément (sorte,
/// cellule ou cadre arrondi au pt, titre d'une carte, enfants d'une pile).
fn describe(e: &Element) -> String {
    let place = match (e.cell, e.frame) {
        (Some(c), _) => format!(" {}:{} {}×{}", c.col, c.row, c.cols, c.rows),
        (None, Some(f)) => format!(" @{:.0},{:.0} {:.0}×{:.0}", f.x, f.y, f.w, f.h),
        (None, None) => String::new(),
    };
    let title = e.params.text("title").map(|t| format!(" « {t} »")).unwrap_or_default();
    let children =
        e.params.slot("children").map(|c| format!(" [{}]", c.iter().map(describe).collect::<Vec<_>>().join(", ")));
    format!("{}{place}{title}{}", e.kind, children.unwrap_or_default())
}

fn summary(plan: &ImportPlan) -> Vec<Value> {
    plan.ops
        .iter()
        .filter_map(|op| match op {
            Op::AddSlide { slide, .. } => Some(slide),
            _ => None,
        })
        .zip(&plan.report.slides)
        .map(|(slide, report)| {
            let elements: Vec<String> = slide.params.slots().flat_map(|(_, list)| list).map(describe).collect();
            json!({
                "slide": slide.id,
                "template": report.template,
                "placement": report.placement,
                "kicker": slide.params.text("kicker"),
                "title": slide.params.text("title"),
                "elements": elements,
            })
        })
        .collect()
}

#[test]
fn retrofit_alto_summary_matches() {
    let (plan, _) = import("alto", "essentiel", "papier");
    insta::assert_json_snapshot!("alto_retrofit_summary", summary(&plan));
}
```

> `minimal` : slide 1 couverture (`couverture`) ; slide 2, titre plus trois contenus qui ne forment pas de colonnes (corps, source en bas à gauche, encart à droite) → `libre` ; le corps et la source se chevauchent en cellules (le placeholder de corps couvre toute la zone) : un seul texte ; slide 3, titre plus un texte → `texte`. `kitchen` : la slide 2 (graphique, image en rangée, groupe « Gauche / Droite » pleine largeur dessous) va en `flow` avec deux piles ; le groupe de deux boîtes ne perd rien : seul le SmartArt de la slide 3 fait une demande. Sur Grilles, les trois contenus de la slide 2 se recouvriraient sur la grille 3 × 2 de `grille-chiffres` : cadres. `alto` : titres déduits partout (« The Agent Is the New Front Door » remplit le deck), cartes 3 + 6 + 4 + 3 + 4 + 4 + 4 = 28, pied « n · Wealth Bank as an API · … » (6 slides), « CONFIDENTIAL » (8) et logo (7) omis et dits une fois ; aucun texte n'en recouvre un autre, sur les deux packs et les deux thèmes.

- [ ] **Step 2 : lancer, constater l'échec**

```bash
export CARGO_TARGET_DIR=$PWD/target
cargo nextest run -p oku-slides-import retrofit
```

Attendu : échec de compilation (`retrofit/slide.rs` et `retrofit/decor.rs` absents, `project`, `RetrofitTarget` et `test_support::applied` inconnus).

- [ ] **Step 3 : implémentation**

`crates/oku-slides-import/src/retrofit/decor.rs` :

```rust
//! Décor : ce que le retrofit omet d'une slide par règle, sans le compter
//! dans la couverture.
//!
//! - les placeholders date, pied de page et numéro de slide ;
//! - l'en-tête et le pied répétés : un item dans la bande haute ou basse de
//!   la page (son centre dans les 12 % hauts ou bas, hors du corps source)
//!   dont le texte (chiffres ôtés : un numéro de page varie) ou l'image revient
//!   sur au moins la moitié des slides (deux au moins). Le thème cible dessine
//!   ses propres en-tête et pied. Perte [`LossKind::RepeatedDecor`], une fois
//!   par motif, sur sa première slide ;
//! - un numéro seul dans la bande (texte fait de chiffres et de ponctuation).
//!
//! La projection ([`super::slide`]) et la couverture ([`crate::coverage`])
//! s'en remettent au même prédicat, [`Decor::skips`].

use std::collections::{BTreeMap, BTreeSet};

use crate::report::{Loss, LossKind};
use crate::source::{Frame, Item, ItemKind, Role, Size, SourceDeck};

use super::text::plain;

/// Part haute et basse de la page où vivent en-têtes et pieds.
pub(crate) const BAND: f64 = 0.12;

/// Part de la page au-delà de laquelle une forme sans texte est un fond.
const BACKDROP: f64 = 0.5;

/// Le centre de `frame` est-il dans la bande haute ou basse d'une page `page` ?
pub(crate) fn in_band(frame: &Frame, page: Size) -> bool {
    let center = frame.y + frame.h / 2.0;
    center < BAND * page.h || center > (1.0 - BAND) * page.h
}

/// Une forme sans texte qui couvre au moins la moitié de la page : un fond.
pub(crate) fn is_backdrop(item: &Item, page: Size) -> bool {
    matches!(item.kind, ItemKind::Shape(_))
        && !item.has_text()
        && item.frame.w * item.frame.h >= BACKDROP * page.w * page.h
}

/// Ce qui identifie un item répété : son texte sans chiffres, espaces
/// normalisées, ou son image ; `None` pour un autre item.
fn key(item: &Item) -> Option<String> {
    match &item.kind {
        ItemKind::Picture(p) => Some(format!("image:{}", p.asset)),
        _ => item.text().map(|tb| {
            let text = plain(tb).0;
            let kept: String = text.chars().filter(|c| !c.is_ascii_digit()).collect();
            kept.split_whitespace().collect::<Vec<_>>().join(" ")
        }),
    }
}

/// Un numéro : un texte non vide fait de chiffres, de ponctuation et d'espaces.
fn is_number(item: &Item, key: &str) -> bool {
    item.has_text() && !key.chars().any(char::is_alphanumeric)
}

/// Un motif de bande : les slides où il apparaît, ses items, sa première occurrence.
#[derive(Debug, Default)]
struct Motif<'a> {
    slides: BTreeSet<u32>,
    items: Vec<String>,
    first: Option<(u32, &'a Item)>,
}

/// Ce que la règle omet dans un deck source.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Decor {
    /// Ids des items omis.
    skipped: BTreeSet<String>,
    /// Pertes `RepeatedDecor`, par numéro de slide source.
    losses: BTreeMap<u32, Vec<Loss>>,
}

impl Decor {
    /// Le décor de `deck`.
    pub(crate) fn of(deck: &SourceDeck) -> Self {
        let mut decor = Self::default();
        let mut seen: BTreeMap<String, Motif<'_>> = BTreeMap::new();
        for slide in &deck.slides {
            for item in &slide.items {
                if matches!(item.role, Some(Role::Date | Role::Footer | Role::SlideNum)) {
                    decor.skipped.insert(item.id.clone());
                    continue;
                }
                if !in_band(&item.frame, deck.size) {
                    continue;
                }
                let Some(k) = key(item) else { continue };
                if is_number(item, &k) {
                    decor.skipped.insert(item.id.clone());
                    continue;
                }
                if k.is_empty() {
                    continue;
                }
                let e = seen.entry(k).or_default();
                e.slides.insert(slide.number);
                e.items.push(item.id.clone());
                e.first.get_or_insert((slide.number, item));
            }
        }
        let needed = deck.slides.len().div_ceil(2).max(2);
        for Motif { slides, items, first } in seen.into_values() {
            let Some((number, item)) = first.filter(|_| slides.len() >= needed) else { continue };
            decor.skipped.extend(items);
            let text = match &item.kind {
                ItemKind::Picture(_) => {
                    item.alt.clone().filter(|a| !a.trim().is_empty()).unwrap_or_else(|| item.name.clone())
                }
                _ => item.text().map(|tb| plain(tb).0).unwrap_or_default(),
            };
            // Au plus autant de slides que le deck en a (borné par le lecteur) : la conversion est exacte.
            let count = u32::try_from(slides.len()).unwrap_or(u32::MAX);
            decor
                .losses
                .entry(number)
                .or_default()
                .push(Loss::on(&item.id, LossKind::RepeatedDecor { text, slides: count }));
        }
        decor
    }

    /// L'item est-il omis par règle ?
    pub(crate) fn skips(&self, item: &Item) -> bool {
        self.skipped.contains(&item.id)
    }

    /// Les pertes de décor répété à dire sur la slide source `number`.
    pub(crate) fn losses(&self, number: u32) -> &[Loss] {
        self.losses.get(&number).map_or(&[], Vec::as_slice)
    }
}

#[cfg(test)]
#[path = "decor_tests.rs"]
mod decor_tests;
```

`crates/oku-slides-import/src/retrofit/slide.rs` :

```rust
//! Une slide source → les ops d'une slide Oku.
//!
//! Les items omis par règle ([`Decor`]) sont écartés ; les titres viennent
//! des placeholders, sinon de la taille et de la position des textes
//! ([`headings`]) ; le reste se regroupe en unités ([`units`] : cartes,
//! textes fusionnés), sans les fonds (une forme sans texte d'au moins la
//! moitié de la page), les formes sans texte des bandes d'en-tête et de pied,
//! ni les filets posés au-dessus du bas du titre. La forme de la slide choisit
//! un modèle ([`choose`]) ; la slide Oku prend la sorte du modèle, son
//! agencement (`layout`) ou sa grille (`grid`), jamais ses textes d'exemple.
//! Titre, surtitre et sous-titre vont dans les paramètres que le widget
//! déclare ; ceux qu'il ne déclare pas deviennent des contenus.
//!
//! Placement (décision 6) : une slide « titre + un corps » (au plus une
//! unité, ou une rangée de colonnes, seule ou entre des unités pleine
//! largeur) sur un modèle à blocs va dans le slot `flow`, en ordre de
//! lecture ; une rangée entourée d'autres unités devient une pile `stack` en
//! colonnes, et la slide garde l'agencement `default`. Une forme sans texte
//! n'entre jamais dans un flux. Sinon les unités vont dans le slot libre, dans
//! l'ordre de dessin, leur boîte englobante ajustée dans la zone de corps
//! ([`Fit`]) : posées en cadre (`frame`) sur un modèle libre, sans grille ;
//! sur un modèle à grille, accrochées à la cellule la plus proche ([`snap`])
//! si aucun contenu n'y recouvre un autre, sinon laissées en cadres (la
//! grille du modèle ne sait pas tenir l'arrangement).
//!
//! Slides de titre (décision 7) : la première couverture remplit les
//! paramètres du deck (`title`, `subtitle`) ; les suivantes sont choisies
//! comme des chapitres.

use std::collections::BTreeSet;

use oku_slides_core::{Cell, Element, Grid, Op, ParamType, ParamValue, Slide, SlotMode, WidgetRegistry, WidgetSchema};
use serde_json::Value;

use super::cluster::{Unit, reading, units};
use super::content::{ContentCx, element, is_thin, plain_of};
use super::decor::{Decor, in_band, is_backdrop};
use super::heading::{Headings, headings};
use super::template::{ContentLayout, Profile, Shape, choose, content_items, shape};
use super::text::note;
use super::zone::{Fit, PageTokens, Zone, body_zone, grid_gap, snap, to_zone, track_size};
use crate::report::{Loss, LossKind, Placement, SlideReport};
use crate::source::{Item, ItemKind, Layout, Size, SourceSlide};

/// Ce dont la projection d'une slide a besoin.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SlideCx<'a> {
    /// Registre du pack cible.
    pub registry: &'a WidgetRegistry,
    /// Jetons de page du thème cible.
    pub page: &'a PageTokens,
    /// Couleurs et assets, pour les contenus (`track` est fixé par slide).
    pub content: ContentCx<'a>,
    /// Layouts de la source.
    pub layouts: &'a [Layout],
    /// Taille de la page source, en pt.
    pub size: Size,
    /// Échelle des pt source vers la page Oku.
    pub scale: f64,
    /// Décor omis par règle.
    pub decor: &'a Decor,
}

/// Une slide projetée.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Projected<'a> {
    /// Ses ops, dans l'ordre.
    pub ops: Vec<Op>,
    /// Son entrée au rapport.
    pub report: SlideReport,
    /// Les assets que ses éléments référencent.
    pub assets: BTreeSet<String>,
    /// Les items source rendus par un repli, avec la sorte du repli.
    pub fallbacks: Vec<(&'a Item, String)>,
}

/// Une couverture : layout `title`, un sous-titre sans contenu, ou une couverture déduite.
fn is_cover(shape: &Shape, heads: &Headings<'_>) -> bool {
    shape.layout_kind.as_deref() == Some("title") || (shape.subtitle && shape.content.is_empty()) || heads.cover
}

/// Le paramètre `name` de `schema`, s'il est un texte plain non lié au deck.
fn holds(schema: Option<&WidgetSchema>, name: &str) -> bool {
    schema.and_then(|w| w.param(name)).is_some_and(|p| p.bind.is_none() && matches!(p.ty, ParamType::Plain))
}

/// Le paramètre `name` de `schema` est-il lié à la clé `name` du deck ?
fn binds(schema: Option<&WidgetSchema>, name: &str) -> bool {
    schema.and_then(|w| w.param(name)).is_some_and(|p| p.bind.as_deref() == Some(name))
}

/// Le premier slot de mode `mode` de `schema`.
fn slot(schema: Option<&WidgetSchema>, mode: SlotMode) -> Option<&str> {
    schema?.params.iter().find_map(|p| match &p.ty {
        ParamType::Slot { mode: m, .. } if *m == mode => Some(p.name.as_str()),
        _ => None,
    })
}

/// Le modèle de `shape` : parmi ceux qui ont un slot si la slide a du contenu.
fn pick<'t>(shape: &Shape, profiles: &[Profile<'t>], registry: &WidgetRegistry) -> Option<Profile<'t>> {
    if !shape.content.is_empty() {
        let roomy: Vec<Profile<'t>> = profiles
            .iter()
            .filter(|p| {
                registry
                    .get(&p.template.slide.kind)
                    .is_some_and(|w| w.params.iter().any(|q| matches!(q.ty, ParamType::Slot { .. })))
            })
            .cloned()
            .collect();
        if let Some(p) = choose(shape, &roomy) {
            return Some(p.clone());
        }
    }
    choose(shape, profiles).cloned()
}

/// Les assets des éléments `list` et de leurs descendants.
fn collect_assets(list: &[Element], out: &mut BTreeSet<String>) {
    for e in list {
        if e.kind == "image"
            && let Some(a) = e.params.text("asset")
        {
            out.insert(a.to_owned());
        }
        for (_, children) in e.params.slots() {
            collect_assets(children, out);
        }
    }
}

/// Un décor de la slide, omis sans perte : un fond, une forme sans texte
/// dans une bande d'en-tête ou de pied, ou un filet au-dessus du bas du titre.
fn is_slide_decor(item: &Item, size: Size, title_bottom: f64, track: (f64, f64)) -> bool {
    if !matches!(item.kind, ItemKind::Shape(_)) || item.has_text() {
        return false;
    }
    let f = &item.frame;
    is_backdrop(item, size) || in_band(f, size) || (is_thin(f, track) && f.y + f.h <= title_bottom)
}

/// La grille de référence d'une slide avant le choix du modèle : la grille
/// par défaut (12 × 6) sur la zone de corps titrée, les items ajustés dedans.
struct Reference {
    zone: Zone,
    grid: Grid,
    gap: f64,
    fit: Fit,
}

impl Reference {
    fn new(items: &[&Item], cx: &SlideCx<'_>) -> Self {
        let zone = body_zone(cx.page, true);
        let grid = Grid { columns: 12, rows: 6, gap: None };
        let gap = grid_gap(&grid, cx.page);
        let fit = Fit::new(items.iter().map(|i| &i.frame), cx.scale, &zone);
        Self { zone, grid, gap, fit }
    }

    fn cell(&self, f: &crate::source::Frame) -> Cell {
        snap(&self.zone, &self.grid, self.gap, to_zone(f, &self.fit, &self.zone))
    }

    /// Une piste de la grille en pt source.
    fn track(&self) -> (f64, f64) {
        track(&self.zone, &self.grid, self.gap, self.fit.scale)
    }
}

/// Largeur d'une colonne et hauteur d'une ligne de `grid` dans `zone`, ramenées en pt source par `scale`.
fn track(zone: &Zone, grid: &Grid, gap: f64, scale: f64) -> (f64, f64) {
    let (w, h) = track_size(zone, grid, gap);
    if scale > 0.0 { (w / scale, h / scale) } else { (0.0, 0.0) }
}

/// Les blocs d'un flux : les unités en ordre de lecture, la rangée de colonnes
/// (s'il faut une pile) à la place de sa première unité ; jamais une forme sans texte.
fn flow_order<'u, 'a>(units: &'u [Unit<'a>], shape: &Shape, stack: bool) -> Vec<Vec<&'u Unit<'a>>> {
    let mut order: Vec<usize> = (0..units.len()).collect();
    if shape.columns >= 2 {
        // Une rangée seule se lit de gauche à droite.
        order.sort_by(|a, b| {
            let (fa, fb) = (units[*a].frame(), units[*b].frame());
            if shape.row.contains(a) && shape.row.contains(b) { fa.x.total_cmp(&fb.x) } else { reading(&fa, &fb) }
        });
    } else {
        order.sort_by(|a, b| reading(&units[*a].frame(), &units[*b].frame()));
    }
    let mut blocks: Vec<Vec<&'u Unit<'a>>> = Vec::new();
    let mut row_at: Option<usize> = None;
    for k in order {
        let u = &units[k];
        if matches!(u, Unit::Item(i) if matches!(i.kind, ItemKind::Shape(_)) && !i.has_text()) {
            continue;
        }
        if stack && shape.row.contains(&k) {
            match row_at {
                Some(at) => blocks[at].push(u),
                None => {
                    row_at = Some(blocks.len());
                    blocks.push(vec![u]);
                }
            }
        } else {
            blocks.push(vec![u]);
        }
    }
    blocks
}

/// Les cellules se chevauchent-elles ?
fn cells_meet(a: &Cell, b: &Cell) -> bool {
    a.col < b.col + b.cols && b.col < a.col + a.cols && a.row < b.row + b.rows && b.row < a.row + a.rows
}

/// Accroche les éléments placés en cadres à la grille `grid` : chacun prend la
/// cellule la plus proche de son cadre ([`snap`]), si aucun contenu (forme
/// exceptée) n'y recouvre un autre ; sinon la grille ne sait pas tenir
/// l'arrangement et tous gardent leur cadre.
fn snap_all(elements: &mut [Element], zone: &Zone, grid: &Grid, gap: f64) {
    let cells: Vec<Option<Cell>> = elements.iter().map(|e| e.frame.map(|f| snap(zone, grid, gap, f))).collect();
    let solid: Vec<(usize, Cell)> = cells
        .iter()
        .enumerate()
        .filter(|(k, _)| elements[*k].kind != "shape")
        .filter_map(|(k, c)| c.map(|c| (k, c)))
        .collect();
    let disjoint = solid.iter().enumerate().all(|(i, (_, a))| solid[i + 1..].iter().all(|(_, b)| !cells_meet(a, b)));
    if disjoint {
        for (e, c) in elements.iter_mut().zip(cells) {
            if c.is_some() {
                e.frame = None;
                e.cell = c;
            }
        }
    }
}

/// Les titres posés en paramètres de la slide (ou du deck pour la première
/// couverture) ; ceux que le widget ne déclare pas restent des contenus.
struct Titles<'a> {
    ops: Vec<Op>,
    extra: Vec<&'a Item>,
}

fn place_titles<'a>(
    heads: &Headings<'a>,
    first_cover: bool,
    schema: Option<&WidgetSchema>,
    slide: &mut Slide,
    losses: &mut Vec<Loss>,
) -> Titles<'a> {
    let mut out = Titles { ops: Vec::new(), extra: Vec::new() };
    for (item, name) in [(heads.kicker, "kicker"), (heads.title, "title"), (heads.subtitle, "subtitle")] {
        let Some(item) = item else { continue };
        let Some(tb) = item.text() else { continue };
        let text = plain_of(tb, &item.id, losses);
        if text.is_empty() {
            continue;
        }
        if first_cover && name != "kicker" {
            out.ops.push(Op::SetDeckParam { key: name.to_owned(), value: Value::String(text.clone()) });
        }
        if holds(schema, name) {
            slide.params.set(name, ParamValue::Text(text));
        } else if !(first_cover && binds(schema, name)) {
            out.extra.push(item);
        }
    }
    out
}

/// La slide Oku de `src`, et ce qu'en dit le rapport.
///
/// `deck_titled` dit si une couverture a déjà rempli les paramètres du deck ;
/// la première couverture le passe à `true`. `None` seulement si `profiles`
/// est vide.
pub(crate) fn project_slide<'a>(
    src: &'a SourceSlide,
    profiles: &[Profile<'_>],
    deck_titled: &mut bool,
    cx: &SlideCx<'_>,
) -> Option<Projected<'a>> {
    let kept: Vec<&Item> = src.items.iter().filter(|i| !cx.decor.skips(i)).collect();
    let heads = headings(&kept, cx.size);
    let title_bottom = heads.title.map_or(0.0, |t| t.frame.y + t.frame.h);
    let reference = Reference::new(&content_items(&kept, &heads), cx);
    let content: Vec<&Item> = content_items(&kept, &heads)
        .into_iter()
        .filter(|i| !is_slide_decor(i, cx.size, title_bottom, reference.track()))
        .collect();
    let mut found = units(&content, |f| reference.cell(f));
    let mut shape = shape(src, cx.layouts, &heads, &found);
    let cover = is_cover(&shape, &heads);
    let first_cover = cover && !*deck_titled;
    if cover && !first_cover {
        shape.layout_kind = Some("secHead".to_owned());
    }
    let profile = pick(&shape, profiles, cx.registry)?;
    let template = &profile.template.slide;
    let schema = cx.registry.get(&template.kind);
    let id = format!("s{}", src.number);
    let mut losses: Vec<Loss> = cx.decor.losses(src.number).to_vec();
    if src.hidden {
        losses.push(Loss::slide(LossKind::HiddenSlide));
    }
    let mut slide = Slide::new(id.clone(), &template.kind);
    let titles = place_titles(&heads, first_cover, schema, &mut slide, &mut losses);
    let mut ops = titles.ops;
    if first_cover {
        *deck_titled = true;
    }
    if !titles.extra.is_empty() {
        // Un titre sans paramètre devient un contenu : les unités sont refaites avec lui.
        let mut all = content.clone();
        all.extend(titles.extra.iter().copied());
        all.sort_by_key(|i| src.items.iter().position(|j| j.id == i.id));
        found = units(&all, |f| reference.cell(f));
        shape = Shape { layout_kind: shape.layout_kind, ..super::template::shape(src, cx.layouts, &heads, &found) };
    }
    let (flow_slot, free_slot) = (slot(schema, SlotMode::Flow), slot(schema, SlotMode::Free));
    let flowable = !matches!(profile.layout, ContentLayout::Free | ContentLayout::Grid)
        && (found.len() <= 1 || shape.columns >= 2);
    let placement = match (flow_slot, free_slot) {
        (Some(_), None) => Placement::Flow,
        (Some(_), Some(_)) if flowable => Placement::Flow,
        (_, Some(_)) => Placement::Grid,
        (None, None) => Placement::Flow,
    };
    let stack = shape.columns >= 2 && shape.row.len() < found.len();
    if placement == Placement::Flow {
        if stack {
            slide.params.set("layout", ParamValue::Text("default".to_owned()));
        } else if let Some(layout) = template.params.get("layout") {
            slide.params.set("layout", layout.clone());
        }
    } else if let Some(grid) = template.params.get("grid") {
        slide.params.set("grid", grid.clone());
    }
    let free = template.params.get("grid").is_none() && profile.layout == ContentLayout::Free;
    let grid = slide.resolved_grid(cx.registry);
    let zone = body_zone(cx.page, slide.params.text("title").is_some());
    let gap = grid_gap(&grid, cx.page);
    let fit = Fit::new(found.iter().map(|u| u.frame()).collect::<Vec<_>>().iter(), cx.scale, &zone);
    let content_cx = ContentCx { track: track(&zone, &grid, gap, fit.scale), ..cx.content };
    let mut elements = Vec::new();
    let mut fallbacks = Vec::new();
    let mut kinds: Vec<String> = Vec::new();
    let blocks: Vec<Vec<&Unit<'a>>> = match placement {
        Placement::Flow => flow_order(&found, &shape, stack),
        Placement::Grid => found.iter().map(|u| vec![u]).collect(),
    };
    for (k, block) in (1..).zip(&blocks) {
        let block_id = format!("{id}-{k}");
        let mut made = Vec::new();
        for (j, unit) in (1..).zip(block) {
            let unit_id = if block.len() > 1 { format!("{block_id}-{j}") } else { block_id.clone() };
            let c = element(unit, unit_id, &content_cx);
            for l in c.losses {
                note(&mut losses, l);
            }
            if let Some(kind) = c.fallback {
                if !kinds.contains(&kind) {
                    kinds.push(kind.clone());
                }
                fallbacks.push((unit.lead(), kind));
            }
            let Some(mut e) = c.element else { continue };
            if placement == Placement::Grid {
                e.frame = Some(to_zone(&unit.frame(), &fit, &zone));
            }
            made.push(e);
        }
        if block.len() > 1 && !made.is_empty() {
            let mut row = Element::new(block_id, "stack");
            row.params.set("layout", ParamValue::Text("columns".to_owned()));
            row.params.set("children", ParamValue::Slot(made));
            elements.push(row);
        } else {
            elements.extend(made);
        }
    }
    if placement == Placement::Grid && !free {
        snap_all(&mut elements, &zone, &grid, gap);
    }
    let mut assets = BTreeSet::new();
    collect_assets(&elements, &mut assets);
    let target = match placement {
        Placement::Flow => flow_slot,
        Placement::Grid => free_slot,
    };
    match target {
        Some(name) if !elements.is_empty() => {
            slide.params.set(name, ParamValue::Slot(elements));
        }
        None if !elements.is_empty() => {
            for item in found.iter().flat_map(Unit::items) {
                note(&mut losses, Loss::on(&item.id, LossKind::NoSlot));
            }
        }
        _ => {}
    }
    ops.push(Op::AddSlide { index: None, defaults: false, slide });
    if !src.notes.trim().is_empty() {
        ops.push(Op::SetNotes { slide: id.clone(), notes: src.notes.clone() });
    }
    let report = SlideReport {
        source: src.number,
        slide: id,
        template: profile.template.id.clone(),
        placement,
        losses,
        fallbacks: kinds,
    };
    Some(Projected { ops, report, assets, fallbacks })
}

#[cfg(test)]
#[path = "slide_tests.rs"]
mod slide_tests;
```

`crates/oku-slides-import/src/retrofit/mod.rs` (réécrit ; les `#[allow(dead_code)]` disparaissent) :

```rust
//! Projecteur retrofit : une [`SourceDeck`] projetée sur un pack et un thème
//! existants.
//!
//! Le style vient du thème cible : couleurs ramenées à ses jetons ([`ColorTokens`]),
//! texte riche réduit à ce que le cœur exprime, éléments libres placés sur la
//! grille d'une zone de corps approchée par ses jetons de page ([`PageTokens`]).
//! [`project`] rend un [`ImportPlan`] : une `AddSlide` par slide source (ids
//! `s<numéro>`, éléments `s<numéro>-<rang>`), les paramètres du deck tirés de
//! la première couverture, les notes, les assets référencés, les demandes de
//! widget et le rapport. Aucun aléa, aucune horloge : même source, même plan.

mod chart;
mod cluster;
mod color;
mod content;
mod decor;
mod group;
mod heading;
mod slide;
mod table;
mod template;
mod text;
mod zone;

use std::collections::BTreeMap;

use oku_slides_core::{Slide, Template, WidgetRegistry};

pub use color::ColorTokens;
pub use zone::PageTokens;

use self::content::ContentCx;
use self::decor::Decor;
use self::slide::{SlideCx, project_slide};
use crate::pattern::signature;
use crate::plan::{ImportPlan, WidgetRequest};
use crate::report::{ImportReport, Mode};
use crate::source::{Item, ItemKind, SourceDeck};

/// La cible d'un retrofit : les modèles et le registre du pack, les jetons du thème.
#[derive(Debug, Clone)]
pub struct RetrofitTarget<'a> {
    /// Les modèles du pack, dans l'ordre du pack.
    pub templates: &'a [Template],
    /// Le registre des widgets du pack (widgets de base et du pack).
    pub registry: &'a WidgetRegistry,
    /// Les couleurs effectives du thème.
    pub colors: ColorTokens,
    /// Les jetons de page effectifs du thème.
    pub page: PageTokens,
}

/// Le modèle d'un pack sans modèle : une slide `content` nue.
fn bare_template() -> Template {
    Template {
        id: "content".to_owned(),
        name: "content".to_owned(),
        description: String::new(),
        slide: Slide::new("content", "content"),
    }
}

/// Ajoute l'occurrence `item` (rendue par le repli `fallback` sur la slide
/// `slide`) à la demande de même signature, ou en crée une.
fn request(requests: &mut Vec<WidgetRequest>, item: &Item, fallback: &str, slide: &str) {
    let children = match &item.kind {
        ItemKind::Group { items, .. } => items.as_slice(),
        _ => std::slice::from_ref(item),
    };
    let sig = signature(children);
    match requests.iter_mut().find(|r| r.signature == sig) {
        Some(r) => {
            if r.slides.last().map(String::as_str) != Some(slide) {
                r.slides.push(slide.to_owned());
            }
        }
        None => requests.push(WidgetRequest {
            signature: sig,
            slides: vec![slide.to_owned()],
            sample: children.to_vec(),
            fallback: fallback.to_owned(),
        }),
    }
}

/// Le plan qui construit, sur la cible `target`, le deck Oku de `deck`.
///
/// Échelle : la page source est ramenée dans la page du thème en gardant ses
/// proportions (`min(largeur / w, hauteur / h)`, soit `1920 / w` pour une
/// source 16:9). Un pack sans modèle reçoit des slides `content` nues.
pub fn project(deck: &SourceDeck, target: &RetrofitTarget<'_>) -> ImportPlan {
    let bare = [bare_template()];
    let templates = if target.templates.is_empty() { &bare[..] } else { target.templates };
    let profiles = template::profiles(templates, target.registry);
    let scale = if deck.size.w > 0.0 && deck.size.h > 0.0 {
        (target.page.width / deck.size.w).min(target.page.height / deck.size.h)
    } else {
        1.0
    };
    let colors = target.colors.clone().with_accents(deck);
    let decor = Decor::of(deck);
    let cx = SlideCx {
        registry: target.registry,
        page: &target.page,
        content: ContentCx { tokens: &colors, assets: &deck.assets, track: (0.0, 0.0) },
        layouts: &deck.layouts,
        size: deck.size,
        scale,
        decor: &decor,
    };
    let mut report = ImportReport::new(Mode::Retrofit);
    report.warnings = deck.warnings.clone();
    let mut plan = ImportPlan { ops: Vec::new(), assets: BTreeMap::new(), widget_requests: Vec::new(), report };
    let mut deck_titled = false;
    for src in &deck.slides {
        let Some(p) = project_slide(src, &profiles, &mut deck_titled, &cx) else { continue };
        plan.ops.extend(p.ops);
        for id in p.assets {
            if let Some(a) = deck.assets.get(&id) {
                plan.assets.insert(id, a.clone());
            }
        }
        for (item, fallback) in &p.fallbacks {
            request(&mut plan.widget_requests, item, fallback, &p.report.slide);
        }
        plan.report.slides.push(p.report);
    }
    plan
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
```

`crates/oku-slides-import/src/lib.rs` complet (l'`#[allow(dead_code)]` de `mod pattern;` disparaît) :

```rust
//! Import d'une présentation PowerPoint en représentation intermédiaire.
//!
//! [`read_pptx`] lit les octets d'un `.pptx` et rend une [`source::SourceDeck`] :
//! géométrie en pt, styles texte résolus, couleurs avec leur provenance,
//! tableaux, graphiques, SmartArt, replis, polices. Bornes : [`ReadLimits`] ;
//! erreurs : [`ImportError`].
//!
//! Un projecteur rend un [`ImportPlan`] : les opérations qui construisent le
//! deck Oku, les assets qu'elles référencent et un [`ImportReport`] (pertes,
//! replis, couverture du contenu). Le retrofit ([`retrofit::project`]) projette
//! la source sur un pack et un thème existants ([`RetrofitTarget`]).

pub mod plan;
pub mod report;
pub mod retrofit;
pub mod source;

mod collect;
mod error;
mod limits;
mod pattern;
mod pptx;
#[cfg(test)]
mod test_support;

pub use error::{ImportError, ImportErrorKind};
pub use limits::ReadLimits;
pub use plan::ImportPlan;
pub use pptx::read_pptx;
pub use report::ImportReport;
pub use retrofit::RetrofitTarget;
```

- [ ] **Step 4 : lancer, constater le succès ; instantanés**

```bash
cargo nextest run -p oku-slides-import retrofit
INSTA_UPDATE=new cargo nextest run -p oku-slides-import --test retrofit
```

Lire chaque `.snap.new` avant `cargo insta accept --manifest-path crates/oku-slides-import/Cargo.toml`. Attendu pour `retrofit__minimal_ops.snap` :

```json
---
source: crates/oku-slides-import/tests/retrofit.rs
expression: plan.ops
---
[
  {
    "op": "set_deck_param",
    "key": "title",
    "value": "Rapport annuel"
  },
  {
    "op": "set_deck_param",
    "key": "subtitle",
    "value": "Exercice 2026"
  },
  {
    "op": "add_slide",
    "slide": {
      "id": "s1",
      "kind": "title"
    }
  },
  {
    "op": "add_slide",
    "slide": {
      "id": "s2",
      "kind": "content",
      "params": {
        "elements": [
          {
            "id": "s2-1",
            "kind": "text",
            "frame": {
              "x": 58.08,
              "y": 0.0,
              "w": 1563.84,
              "h": 690.0
            },
            "params": {
              "body": [
                {
                  "runs": [
                    {
                      "t": "Chiffre d'affaires en hausse"
                    }
                  ],
                  "list": "bullet"
                },
                {
                  "runs": [
                    {
                      "t": "+12 % sur un an"
                    }
                  ],
                  "list": "bullet"
                },
                {
                  "runs": [
                    {
                      "t": "Source : comptes 2026",
                      "color": "accent"
                    }
                  ]
                }
              ]
            }
          },
          {
            "id": "s2-2",
            "kind": "card",
            "frame": {
              "x": 1179.97,
              "y": 0.0,
              "w": 441.95,
              "h": 203.98
            },
            "params": {
              "body": [
                {
                  "runs": [
                    {
                      "t": "+12 %",
                      "bold": true,
                      "color": "bg"
                    }
                  ]
                }
              ],
              "highlight": true
            }
          }
        ],
        "title": "Résultats"
      }
    }
  },
  {
    "op": "set_notes",
    "slide": "s2",
    "notes": "Dire bonjour.\nPuis\nenchaîner."
  },
  {
    "op": "add_slide",
    "slide": {
      "id": "s3",
      "kind": "content",
      "params": {
        "blocks": [
          {
            "id": "s3-1",
            "kind": "text",
            "params": {
              "body": [
                {
                  "runs": [
                    {
                      "t": "Texte sans couleur"
                    }
                  ]
                }
              ]
            }
          }
        ],
        "title": "Section sombre"
      }
    }
  }
]
```

> Slide 2 de `minimal` : unités = le texte fusionné (corps `(66, 143,8, 828, 342,6)` et source `(66, 480, 240, 29,1)`) et l'encart `(660, 143,8, 234, 108)` ; englobante 828 × 365,3 pt, trop haute pour la zone titrée (690) à l'échelle 2 : échelle `690 / 365,3 = 1,889`, centrée en largeur, `dx = (1680 − 828 × 1,889) / 2 − 66 × 1,889` ; d'où le texte en `x = 58,08`, `w = 1 563,84`, `h = 690`, et l'encart en `x = 1 179,97`. La source, rouge, est la seule couleur saturée du deck sans jeton proche : elle prend `accent` (accents déduits) ; l'encart `accent1` → `accent`, 5,9:1 sur la carte : relief, et son texte blanc → `bg`, lisible sur le relief `ink`.

À relire dans `retrofit__kitchen_ops.snap` : slide 1 en `flow` (`blocks`) avec le tableau `[["Titre", null], ["A", "1"], ["B", "2"]]`, `header-row` et `striped` ; slide 2 en `flow`, `layout: default` : pile `s2-1` (graphique, séries `bar` empilées `total`, catégories `T1 T2 T3`, titre « Ventes 2026 » ; image `fit: contain`), puis pile `s2-2` des deux cartes du groupe ; slide 3 en éléments libres sur `libre` : pile du SmartArt (`Étape 1`, `Étape 2`) et image de repli de l'équation, en cadres. Le résumé `retrofit__alto_retrofit_summary.snap` attendu :

```json
---
source: crates/oku-slides-import/tests/retrofit.rs
expression: summary(&plan)
---
[
  {
    "slide": "s1",
    "template": "libre",
    "placement": "grid",
    "kicker": "FROM A SURVIVAL QUESTION TO A GENERATIONAL OPPORTUNITY",
    "title": "The Agent Is the New Front Door",
    "elements": [
      "text @152,0 189×37",
      "text @1073,6 266×25",
      "text @152,442 1375×30",
      "text @152,491 1375×155",
      "text @152,665 1375×25"
    ]
  },
  {
    "slide": "s2",
    "template": "cartes",
    "placement": "flow",
    "kicker": "OUR CONVICTION",
    "title": "The agent becomes the only interface — and, soon, the more trusted confidant.",
    "elements": [
      "card « The only window »",
      "card « A deeper confidant »",
      "card « Attacked on two fronts »"
    ]
  },
  {
    "slide": "s3",
    "template": "libre",
    "placement": "grid",
    "kicker": "THE SHIFT",
    "title": "From selling software to a connected bank",
    "elements": [
      "text @0,44 1680×116",
      "card @0,173 481×132 « Alto Wealth »",
      "image @504,229 47×22",
      "card @575,173 481×132 « Private banker »",
      "image @1080,229 47×22",
      "card @1150,173 481×132 « End client »",
      "text @0,349 1680×34",
      "card @1,398 480×134 « End client »",
      "image @506,454 47×22",
      "card @577,398 480×134 « Claude · ChatGPT »",
      "image @1082,454 47×22",
      "card @1153,399 478×132 « Alto Wealth »",
      "text @0,576 1514×70"
    ]
  },
  {
    "slide": "s4",
    "template": "cartes",
    "placement": "flow",
    "kicker": "OUR MOAT",
    "title": "Our moat isn't the code. It's the ecosystem behind it.",
    "elements": [
      "text",
      "stack [card « Amundi Funds », card « Alto Wealth », card « Amundi Services »]",
      "card"
    ]
  },
  {
    "slide": "s5",
    "template": "colonnes",
    "placement": "flow",
    "kicker": "OUR RESPONSE",
    "title": "Expose the ecosystem, not just the software",
    "elements": [
      "text",
      "stack [card « MCP », card « AG-UI »]",
      "card"
    ]
  },
  {
    "slide": "s6",
    "template": "cartes",
    "placement": "flow",
    "kicker": "WHY US, NOT JUST ANY MCP SERVER",
    "title": "The financial coach for someone's entire life",
    "elements": [
      "text",
      "stack [card « A lifelong financial planner », card « Products no one else offers », card « A product for every life stage »]",
      "card"
    ]
  },
  {
    "slide": "s7",
    "template": "cartes",
    "placement": "flow",
    "kicker": "ROADMAP",
    "title": "First mover, standard setter",
    "elements": [
      "stack [card « Live in every agent marketplace », card « Transact from the marketplace », card « Become the default choice »]",
      "card"
    ]
  },
  {
    "slide": "s8",
    "template": "cartes",
    "placement": "flow",
    "kicker": "ORGANIZATIONAL ANATOMY",
    "title": "The wealth management value chain, front to back",
    "elements": [
      "card « Front & Distribution »",
      "card « Product & Investment »",
      "card « Operations & Custody »",
      "card « Governance & Control »"
    ]
  }
]
```

Attendu ensuite : `cargo nextest run -p oku-slides-import` vert (608 tests au brouillon).

- [ ] **Step 5 : vérifier, commit, sous-tâche**

```bash
cargo fmt --all && cargo clippy -p oku-slides-import --all-targets -- -D warnings && cargo nextest run -p oku-slides-import
git add crates/oku-slides-import/src/lib.rs crates/oku-slides-import/src/retrofit/mod.rs crates/oku-slides-import/src/retrofit/mod_tests.rs crates/oku-slides-import/src/retrofit/slide.rs crates/oku-slides-import/src/retrofit/slide_tests.rs crates/oku-slides-import/src/retrofit/decor.rs crates/oku-slides-import/src/retrofit/decor_tests.rs crates/oku-slides-import/src/test_support.rs crates/oku-slides-import/tests/common/mod.rs crates/oku-slides-import/tests/retrofit.rs crates/oku-slides-import/tests/snapshots/retrofit__minimal_ops.snap crates/oku-slides-import/tests/snapshots/retrofit__kitchen_ops.snap crates/oku-slides-import/tests/snapshots/retrofit__alto_retrofit_summary.snap && git diff --cached --name-only
git commit -F <scratchpad>/msg.txt   # feat(import) : projection retrofit d'une slide source en ops, décor omis, retrofit::project (t-692)
oku cli task done --project slider --id t-805
```

---

### Task 10: Couverture du contenu et rapport complet

**Écarts d'interface :**
- `pub mod coverage` (module public, comme `plan` et `report`) : `oku_slides_import::coverage::coverage(source: &SourceDeck, deck: &Deck, registry: &WidgetRegistry, origins: &BTreeMap<String, String>) -> Coverage`. `origins` (id d'item source → id de l'élément projeté depuis lui) est le quatrième argument (révision 1, item 10) : un run se cherche dans le texte de l'élément projeté depuis son item, sinon dans la slide. Le lot 3 (hifi) mesurera son propre deck avec.
- `Projected` (tâche 9) gagne `origins: Vec<(String, String)>` (chaque item d'une unité → l'id de son élément) ; `project` les réunit pour la couverture. `retrofit::decor` et `retrofit::text` deviennent `pub(crate)` : la couverture omet les mêmes items que la projection (`Decor::skips`, un seul prédicat, G11) et lit le texte d'une cellule comme le tableau (`text::plain`).
- `project` remplit `report.coverage` : ses ops appliquées (`apply_all`) à un `Deck::empty()` sans thème ni pack embarqué, avec le registre de la cible ; `None` seulement si le cœur refusait le plan (jamais sur les fixtures). La CLI n'a rien à recalculer.
- `LossKind::SeriesDropped { kept: u32, dropped: u32 }` (décision du contrôleur) : un graphique en secteurs ou en anneau garde sa première série ; `retrofit::chart` le signale. Sérialisé `{"type": "series_dropped", "detail": {"kept": 1, "dropped": 2}}`.
- `impl Localize for source::Warning` (dans `report.rs`) : une clé `import-warning-*` par variante de `WarningKind`, variable `$detail` (sauf `Truncated`) ; `ImportReport::to_json` ajoute un `message` à chaque avertissement, comme à chaque perte. Aide privée `warning_message(&WarningKind) -> (&'static str, Option<&str>)`.

La couverture se mesure sur le deck obtenu, slide source `n` contre slide Oku `s<n>` (ids du projecteur), plus le titre et le sous-titre du deck (remplis par la première couverture) :

| contenu source | compté | retrouvé si |
|---|---|---|
| run de texte | chaque run qui porte une lettre ou un chiffre (espaces normalisées) d'une boîte, d'une forme, d'un enfant de groupe, d'une cellule non couverte | son texte est dans un texte de l'élément projeté depuis son item (descendants compris ; sinon de la slide) — paramètre `plain`, paragraphe `rich` (runs concaténés), cellule textuelle d'un `dataset.source` — **à ses frontières de mots** |
| run de moins de 3 caractères | idem | son paragraphe source entier (sa cellule, pour un tableau) y est un texte, à l'identique |
| image | image, forme remplie d'une image, intraduisible avec repli | un élément `image` de la slide dont `asset` est le même |
| tableau | chaque tableau | un paramètre `host(table)` dont `dataset.source` égale la grille source : texte plain des cellules, `null` pour une cellule couverte, lignes complétées de `null` |
| graphique | chaque graphique | un paramètre `host(chart)` où chaque série source a une colonne de mêmes valeurs (une valeur absente ou non finie = `null`) |

Frontières de mots : un run qui commence (ou finit) par une lettre ou un chiffre séparé de son voisin dans la source (espace, ponctuation, bord du paragraphe) doit l'être aussi dans le texte trouvé ; un run collé à un voisin au milieu d'un mot (« Wealth » + « 's »), ou bordé de ponctuation (« : an AI's… » après « competence »), peut l'être dans le deck. Un run en capitales (`Caps::All`, mis en majuscules par la tâche 4) se compare sans la casse. Les items omis par règle (`Decor::skips` : date, pied, numéro, en-tête et pied répétés) ne sont pas comptés. Écarts justifiés (une perte les dit déjà) : texte coupé aux bornes (`TextTruncated`), tableau coupé (`TableTruncated`), image d'un format non dessiné (`UnsupportedImage`), secteurs à plusieurs séries (`SeriesDropped`), intraduisible sans repli (`Opaque`, ni run ni image compté), contenu sans slot (`NoSlot`). Sur les trois fixtures, avec les deux packs et les deux thèmes, rien ne manque.

**Files:**
- Create: `crates/oku-slides-import/src/coverage.rs`, `crates/oku-slides-import/src/coverage_tests.rs`
- Modify: `crates/oku-slides-import/src/retrofit/slide.rs` (`origins`), `crates/oku-slides-import/src/lib.rs` (`pub mod coverage;`), `crates/oku-slides-import/src/report.rs` (`SeriesDropped`, `Localize for Warning`, `to_json`), `crates/oku-slides-import/src/report_tests.rs`, `crates/oku-slides-import/src/retrofit/chart.rs`, `crates/oku-slides-import/src/retrofit/chart_tests.rs`, `crates/oku-slides-import/src/retrofit/mod.rs` (couverture), `crates/oku-slides-import/src/retrofit/mod_tests.rs`, `crates/oku-slides-import/tests/retrofit.rs`, `crates/oku-slides-i18n/src/locales/*/document.ftl` (×20)

**Interfaces:**
- Consumes : `report::{Coverage, Ratio, Missing}` (2) ; `retrofit::project`, `Decor` (9) ; `cluster::Unit::items` (7) ; `text::plain` (4) ; `oku_slides_core::{Deck, Element, Params, ParamValue, ParamType, HostType, WidgetRegistry, apply_all}` ; `source::{SourceDeck, Item, ItemKind, Table, Chart, Fill, Role, Warning, WarningKind}`.
- Produces :

```rust
// crate::coverage
pub fn coverage(source: &SourceDeck, deck: &Deck, registry: &WidgetRegistry, origins: &BTreeMap<String, String>) -> Coverage;
// crate::retrofit::slide
pub(crate) struct Projected<'a> { /* … */ pub origins: Vec<(String, String)> }
// crate::report
pub enum LossKind { /* … */ SeriesDropped { kept: u32, dropped: u32 } }
impl Localize for Warning { /* import-warning-* */ }
// ImportReport::to_json : « message » à côté de chaque perte et de chaque avertissement
```

- [ ] **Step 1 : tests qui échouent**

`crates/oku-slides-import/src/coverage_tests.rs` :

```rust
use std::collections::BTreeMap;

use oku_slides_core::{Deck, Element, Paragraph, Run as OkuRun};
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::source::{
    Caps, Cell, Chart, ChartKind, Grouping, Opaque, OpaqueReason, Picture, Role, Series, Size, SourceSlide, Table,
    TableFlags,
};
use crate::test_support::{
    element_with, item, para, registry, run, run_with, source_slide, text_box, text_item, with_role,
};

fn source(slides: Vec<SourceSlide>) -> SourceDeck {
    SourceDeck {
        size: Size { w: 960.0, h: 540.0 },
        themes: Vec::new(),
        masters: Vec::new(),
        layouts: Vec::new(),
        slides,
        assets: BTreeMap::new(),
        fonts: Vec::new(),
        warnings: Vec::new(),
    }
}

/// Un deck d'une slide `content` `s1`, de titre `title`, aux blocs `blocks`.
fn deck(title: Option<&str>, blocks: Vec<Element>) -> Deck {
    let mut s = Slide::new("s1", "content");
    if let Some(t) = title {
        s.params.set("title", ParamValue::Text(t.into()));
    }
    if !blocks.is_empty() {
        s.params.set("blocks", ParamValue::Slot(blocks));
    }
    let mut d = Deck::empty();
    d.slides.push(s);
    d
}

fn text(id: &str, t: &str) -> Element {
    Element::text(id, vec![Paragraph { runs: vec![OkuRun { t: t.into(), ..OkuRun::default() }], list: None }])
}

fn image(id: &str, asset: &str) -> Element {
    let mut e = Element::new(id, "image");
    e.params.set("asset", ParamValue::Text(asset.into()));
    e
}

fn cell(t: &str, covered: bool) -> Cell {
    Cell {
        text: text_box(vec![para(vec![run(t)])]),
        fill: Fill::None,
        borders: Default::default(),
        row_span: 1,
        col_span: if t == "Titre" { 2 } else { 1 },
        covered,
    }
}

fn table_item() -> Item {
    let t = Table {
        col_widths: vec![100.0, 100.0],
        row_heights: vec![20.0, 20.0],
        flags: TableFlags::default(),
        rows: vec![vec![cell("Titre", false), cell("", true)], vec![cell("A", false), cell("1", false)]],
    };
    item("tb", (0.0, 0.0, 200.0, 40.0), ItemKind::Table(t))
}

fn chart_item(kind: ChartKind) -> Item {
    let series = |name: &str, v: [f64; 2]| Series { name: name.into(), values: v.map(Some).to_vec(), color: None };
    let c = Chart {
        kind,
        grouping: Grouping::Clustered,
        title: None,
        categories: vec!["T1".into(), "T2".into()],
        series: vec![series("Nord", [1.0, 2.0]), series("Sud", [3.0, 4.0])],
    };
    item("ch", (0.0, 0.0, 200.0, 100.0), ItemKind::Chart(c))
}

#[test]
fn coverage_finds_runs_in_plain_rich_and_deck_params() {
    let title = with_role(text_item("t", (0.0, 0.0, 100.0, 20.0), "Résultats"), Role::Title);
    let body = item(
        "b",
        (0.0, 30.0, 100.0, 20.0),
        ItemKind::Text(text_box(vec![para(vec![run("Chiffre  d'affaires"), run(" en hausse")])])),
    );
    let sub = with_role(text_item("st", (0.0, 60.0, 100.0, 20.0), "Exercice 2026"), Role::Subtitle);
    let mut d = deck(Some("Résultats"), vec![text("s1-1", "Chiffre d'affaires en hausse")]);
    d.params.subtitle = Some("Exercice 2026".into());
    let c = coverage(&source(vec![source_slide(1, None, vec![title, body, sub])]), &d, &registry(), &BTreeMap::new());
    assert_that!((c.runs, c.missing.is_empty())).is_equal_to((Ratio { found: 4, total: 4 }, true));
}

#[test]
fn coverage_reports_a_missing_run_with_its_slide() {
    let src = source(vec![source_slide(1, None, vec![text_item("b", (0.0, 0.0, 10.0, 10.0), "Perdu")])]);
    let c = coverage(&src, &deck(Some("Autre"), Vec::new()), &registry(), &BTreeMap::new());
    assert_that!((c.runs, c.missing))
        .is_equal_to((Ratio { found: 0, total: 1 }, vec![Missing::Run { slide: 1, text: "Perdu".into() }]));
}

#[test]
fn coverage_compares_each_source_slide_with_its_own_slide() {
    let src = source(vec![
        source_slide(1, None, vec![text_item("a", (0.0, 200.0, 10.0, 10.0), "Ici")]),
        source_slide(2, None, vec![text_item("b", (0.0, 200.0, 10.0, 10.0), "Ici")]),
    ]);
    let c = coverage(&src, &deck(None, vec![text("s1-1", "Ici")]), &registry(), &BTreeMap::new());
    assert_that!(c.missing).is_equal_to(vec![Missing::Run { slide: 2, text: "Ici".into() }]);
}

#[test]
fn coverage_skips_date_footer_and_slide_number() {
    let items = [Role::Date, Role::Footer, Role::SlideNum]
        .into_iter()
        .map(|r| with_role(text_item("x", (0.0, 0.0, 10.0, 10.0), "10/10/2026"), r))
        .collect();
    let c =
        coverage(&source(vec![source_slide(1, None, items)]), &deck(None, Vec::new()), &registry(), &BTreeMap::new());
    assert_that!(c.runs).is_equal_to(Ratio::default());
}

#[test]
fn coverage_counts_runs_inside_groups() {
    let group = item(
        "g",
        (0.0, 0.0, 100.0, 20.0),
        ItemKind::Group {
            items: vec![
                text_item("g/1", (0.0, 0.0, 50.0, 20.0), "Étape 1"),
                text_item("g/2", (50.0, 0.0, 50.0, 20.0), "Étape 2"),
            ],
            smartart: false,
        },
    );
    let mut card = Element::new("s1-1-1", "card");
    card.params.set("title", ParamValue::Text("Étape 1".into()));
    let stack = {
        let mut e = Element::new("s1-1", "stack");
        e.params.set("children", ParamValue::Slot(vec![card, text("s1-1-2", "Étape 2")]));
        e
    };
    let c = coverage(
        &source(vec![source_slide(1, None, vec![group])]),
        &deck(None, vec![stack]),
        &registry(),
        &BTreeMap::new(),
    );
    assert_that!(c.runs).is_equal_to(Ratio { found: 2, total: 2 });
}

#[test]
fn coverage_counts_images_and_fallbacks_by_asset() {
    let pic = item(
        "p",
        (0.0, 0.0, 10.0, 10.0),
        ItemKind::Picture(Picture { asset: "sha256:a".into(), crop: Default::default() }),
    );
    let math = item(
        "m",
        (0.0, 0.0, 10.0, 10.0),
        ItemKind::Opaque(Opaque { reason: OpaqueReason::Math, fallback: Some("sha256:b".into()) }),
    );
    let c = coverage(
        &source(vec![source_slide(1, None, vec![pic, math])]),
        &deck(None, vec![image("s1-1", "sha256:a")]),
        &registry(),
        &BTreeMap::new(),
    );
    assert_that!((c.images, c.missing))
        .is_equal_to((Ratio { found: 1, total: 2 }, vec![Missing::Image { slide: 1, item: "m".into() }]));
}

#[test]
fn coverage_finds_a_table_by_its_grid_and_its_cell_runs() {
    let data = json!({ "dataset": { "source": [["Titre", null], ["A", "1"]] } });
    let d = deck(None, vec![element_with("s1-1", "table", "data", data)]);
    let c = coverage(&source(vec![source_slide(1, None, vec![table_item()])]), &d, &registry(), &BTreeMap::new());
    assert_that!((c.tables, c.runs)).is_equal_to((Ratio { found: 1, total: 1 }, Ratio { found: 3, total: 3 }));
}

#[test]
fn coverage_misses_a_table_whose_grid_differs() {
    let data = json!({ "dataset": { "source": [["Titre", null], ["A", "2"]] } });
    let d = deck(None, vec![element_with("s1-1", "table", "data", data)]);
    let c = coverage(&source(vec![source_slide(1, None, vec![table_item()])]), &d, &registry(), &BTreeMap::new());
    assert_that!(c.missing).contains(Missing::Table { slide: 1, item: "tb".into() });
}

#[test]
fn coverage_finds_a_chart_when_every_series_has_its_column() {
    let option = json!({
        "dataset": { "source": [["", "Nord", "Sud"], ["T1", 1.0, 3.0], ["T2", 2.0, 4.0]] },
        "series": [{ "type": "bar" }, { "type": "bar" }],
    });
    let d = deck(None, vec![element_with("s1-1", "chart", "option", option)]);
    let c = coverage(
        &source(vec![source_slide(1, None, vec![chart_item(ChartKind::Column)])]),
        &d,
        &registry(),
        &BTreeMap::new(),
    );
    assert_that!(c.charts).is_equal_to(Ratio { found: 1, total: 1 });
}

#[test]
fn coverage_misses_a_pie_that_kept_one_series_of_two() {
    let option =
        json!({ "dataset": { "source": [["", "Nord"], ["T1", 1.0], ["T2", 2.0]] }, "series": [{ "type": "pie" }] });
    let d = deck(None, vec![element_with("s1-1", "chart", "option", option)]);
    let c = coverage(
        &source(vec![source_slide(1, None, vec![chart_item(ChartKind::Pie)])]),
        &d,
        &registry(),
        &BTreeMap::new(),
    );
    assert_that!((c.charts, c.missing))
        .is_equal_to((Ratio { found: 0, total: 1 }, vec![Missing::Chart { slide: 1, item: "ch".into() }]));
}

fn origins(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs.iter().map(|(i, e)| ((*i).to_owned(), (*e).to_owned())).collect()
}

/// « art » est dans « smart » mais pas comme mot : le run manque ; le run
/// « Wealth » collé à « 's » au milieu d'un mot se trouve dans « Wealth's ».
#[test]
fn coverage_matches_runs_on_word_boundaries() {
    let glued = item(
        "g",
        (0.0, 30.0, 100.0, 20.0),
        ItemKind::Text(text_box(vec![para(vec![
            run("Alto Wealth"),
            run_with("'s", |s| s.bold = true),
            run(" layer"),
        ])])),
    );
    let src = source(vec![source_slide(1, None, vec![text_item("a", (0.0, 0.0, 10.0, 10.0), "art"), glued])]);
    let d = deck(None, vec![text("s1-1", "smart"), text("s1-2", "Alto Wealth's layer")]);
    let c = coverage(&src, &d, &registry(), &BTreeMap::new());
    assert_that!(c.missing).is_equal_to(vec![Missing::Run { slide: 1, text: "art".into() }]);
    assert_that!(c.runs).is_equal_to(Ratio { found: 3, total: 4 });
}

/// « AI » (2 caractères) est dans « MAIN » ; seul un paragraphe « AI » entier le retrouve.
#[test]
fn coverage_needs_the_whole_paragraph_for_a_short_run() {
    let src = source(vec![source_slide(1, None, vec![text_item("a", (0.0, 0.0, 10.0, 10.0), "AI")])]);
    let missed = coverage(&src, &deck(None, vec![text("s1-1", "MAIN AI ROAD")]), &registry(), &BTreeMap::new());
    assert_that!(missed.missing).is_equal_to(vec![Missing::Run { slide: 1, text: "AI".into() }]);
    let found = coverage(&src, &deck(None, vec![text("s1-1", "AI")]), &registry(), &BTreeMap::new());
    assert_that!(found.runs).is_equal_to(Ratio { found: 1, total: 1 });
}

/// Projeté dans `s1-1`, le texte « Corps » ne compte pas s'il n'est que dans `s1-2`.
#[test]
fn coverage_looks_for_a_run_in_the_element_projected_from_its_item() {
    let src = source(vec![source_slide(1, None, vec![text_item("b", (0.0, 0.0, 10.0, 10.0), "Corps")])]);
    let d = deck(None, vec![text("s1-1", "Autre chose"), text("s1-2", "Corps")]);
    let c = coverage(&src, &d, &registry(), &origins(&[("b", "s1-1")]));
    assert_that!(c.missing).is_equal_to(vec![Missing::Run { slide: 1, text: "Corps".into() }]);
    let c = coverage(&src, &d, &registry(), &origins(&[("b", "s1-2")]));
    assert_that!(c.missing).is_empty();
}

#[test]
fn coverage_compares_all_caps_runs_without_case_and_skips_punctuation() {
    let tb = text_box(vec![para(vec![run_with("the shift", |s| s.caps = Caps::All), run(" — ")])]);
    let src = source(vec![source_slide(1, None, vec![item("k", (0.0, 0.0, 10.0, 10.0), ItemKind::Text(tb))])]);
    let c = coverage(&src, &deck(Some("THE SHIFT"), Vec::new()), &registry(), &BTreeMap::new());
    assert_that!((c.runs, c.missing.is_empty())).is_equal_to((Ratio { found: 1, total: 1 }, true));
}

/// Le pied « n · Rapport » répété dans la bande basse de deux slides sur deux est omis par règle.
#[test]
fn coverage_skips_repeated_footers_like_the_projection() {
    let footer = |n: u32| text_item(&format!("s{n}/f"), (64.0, 497.0, 270.0, 12.0), &format!("{n} · Rapport"));
    let src = source(vec![source_slide(1, None, vec![footer(1)]), source_slide(2, None, vec![footer(2)])]);
    let c = coverage(&src, &deck(None, Vec::new()), &registry(), &BTreeMap::new());
    assert_that!(c.runs).is_equal_to(Ratio::default());
}
```

> `coverage_finds_runs_in_plain_rich_and_deck_params` : quatre runs (« Résultats », « Chiffre d'affaires » — deux espaces ramenées à une —, « en hausse », « Exercice 2026 ») ; les deux du corps sont dans le paragraphe riche « Chiffre d'affaires en hausse », le sous-titre dans les paramètres du deck. Le tableau : runs « Titre », « A », « 1 » (la cellule couverte n'en a pas ; « A » et « 1 », courts, se retrouvent par leur cellule entière) ; grille `[["Titre", null], ["A", "1"]]`. `coverage_matches_runs_on_word_boundaries` : « art » est dans « smart » mais n'y commence pas un mot : manquant ; « Wealth » est collé à « 's » dans la source, on le trouve dans « Wealth's » ; 3 runs sur 4. `coverage_needs_the_whole_paragraph_for_a_short_run` : « AI » est dans « MAIN AI ROAD » mais seul un paragraphe « AI » entier le retrouve. `coverage_compares_each_source_slide_with_its_own_slide` pose ses boîtes à y = 200 : en haut de page (y = 0), « Ici » sur les deux slides serait un en-tête répété, omis.

`crates/oku-slides-import/src/report_tests.rs` complet :

```rust
use oku_slides_i18n::{Localize, Localizer, english};
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::source::{Warning, WarningKind};

fn slide(losses: Vec<Loss>) -> SlideReport {
    SlideReport {
        source: 2,
        slide: "s2".into(),
        template: "texte".into(),
        placement: Placement::Flow,
        losses,
        fallbacks: vec!["stack".into()],
    }
}

#[rstest]
#[case(LossKind::ListLevels)]
#[case(LossKind::MergedCells)]
#[case(LossKind::PercentStacked)]
#[case(LossKind::RunStyle(RunEffect::Underline))]
#[case(LossKind::RunStyle(RunEffect::Strike))]
#[case(LossKind::RunStyle(RunEffect::Baseline))]
#[case(LossKind::RunStyle(RunEffect::Caps))]
#[case(LossKind::RunStyle(RunEffect::Highlight))]
#[case(LossKind::Transform)]
#[case(LossKind::ColorDropped("#ff0000".into()))]
#[case(LossKind::TextTruncated)]
#[case(LossKind::TableTruncated)]
#[case(LossKind::Opaque(OpaqueReason::Media))]
#[case(LossKind::Opaque(OpaqueReason::Ole))]
#[case(LossKind::Opaque(OpaqueReason::Math))]
#[case(LossKind::Opaque(OpaqueReason::SmartArt))]
#[case(LossKind::Opaque(OpaqueReason::Unknown("x".into())))]
#[case(LossKind::UnsupportedImage("image/x-emf".into()))]
#[case(LossKind::HiddenSlide)]
#[case(LossKind::Effects)]
#[case(LossKind::RepeatedDecor { text: "CONFIDENTIAL".into(), slides: 7 })]
#[case(LossKind::RepeatedDecor { text: "CONFIDENTIAL".into(), slides: 1 })]
#[case(LossKind::NoSlot)]
#[case(LossKind::SeriesDropped { kept: 1, dropped: 2 })]
fn loss_localize_in_every_locale_formats_its_own_message(#[case] kind: LossKind) {
    let id = kind.message_id();
    let loss = Loss::slide(kind);
    for tag in oku_slides_i18n::available() {
        let l = Localizer::new(Some(tag));
        let msg = loss.localize(&l);
        assert_that!(l.has(id)).named(tag).is_true();
        assert_that!(msg.is_empty() || msg.contains('{')).named(tag).is_false();
    }
}

#[test]
fn loss_localize_color_dropped_names_the_color() {
    let msg = Loss::on("s1/4", LossKind::ColorDropped("#ff0000".into())).localize(english());
    assert_that!(msg).contains("#ff0000");
}

#[test]
fn loss_localize_run_style_in_french_names_the_effect() {
    let msg = Loss::slide(LossKind::RunStyle(RunEffect::Underline)).localize(&Localizer::new(Some("fr")));
    assert_that!(msg).is_equal_to("Soulignement perdu".to_owned());
}

#[test]
fn loss_localize_opaque_in_english_names_the_reason() {
    let msg = Loss::slide(LossKind::Opaque(OpaqueReason::Math)).localize(english());
    assert_that!(msg).is_equal_to("Equation not imported (no fallback image)".to_owned());
}

#[test]
fn loss_localize_series_dropped_counts_the_series() {
    let msg = Loss::on("s2/4", LossKind::SeriesDropped { kept: 1, dropped: 2 }).localize(english());
    assert_that!(msg).is_equal_to("A pie chart shows a single series: 1 kept, 2 dropped".to_owned());
}

#[test]
fn loss_localize_repeated_decor_names_the_text_and_counts_the_slides() {
    let msg = Loss::on("s2/18", LossKind::RepeatedDecor { text: "CONFIDENTIAL".into(), slides: 7 }).localize(english());
    assert_that!(msg).is_equal_to(
        "Header or footer “CONFIDENTIAL” repeated on 7 slides left out: the theme draws its own".to_owned(),
    );
}

#[test]
fn loss_localize_repeated_decor_in_french_agrees_with_one_slide() {
    let loss = Loss::on("s2/18", LossKind::RepeatedDecor { text: "Logo".into(), slides: 1 });
    let msg = loss.localize(&Localizer::new(Some("fr")));
    assert_that!(msg)
        .is_equal_to("En-tête ou pied « Logo » répété sur 1 slide omis : le thème dessine les siens".to_owned());
}

#[rstest]
#[case(WarningKind::EffectIgnored("effectLst".into()))]
#[case(WarningKind::Approximated("pattFill".into()))]
#[case(WarningKind::UnreadableItem("s1/3".into()))]
#[case(WarningKind::UnreadablePart("ppt/slides/slide9.xml".into()))]
#[case(WarningKind::MissingPart("ppt/media/image1.png".into()))]
#[case(WarningKind::UnsupportedChart("combo: barChart".into()))]
#[case(WarningKind::UnsupportedImage("image/x-emf".into()))]
#[case(WarningKind::UnknownTableStyle("{5C22544A}".into()))]
#[case(WarningKind::UnsafeLink("javascript:".into()))]
#[case(WarningKind::NestingTooDeep("grpSp".into()))]
#[case(WarningKind::FontEmbeddedUnreadable("font1.fntdata".into()))]
#[case(WarningKind::Truncated)]
fn warning_localize_in_every_locale_formats_its_own_message(#[case] kind: WarningKind) {
    let (id, detail) = warning_message(&kind);
    let warning = Warning { slide: Some(1), item: None, kind: kind.clone() };
    for tag in oku_slides_i18n::available() {
        let l = Localizer::new(Some(tag));
        let msg = warning.localize(&l);
        assert_that!(l.has(id)).named(tag).is_true();
        assert_that!(detail.is_none_or(|d| msg.contains(d))).named(tag).is_true();
        // Aucune variable laissée telle quelle hors du détail (un style de tableau est un GUID entre accolades).
        let bare = detail.map_or(msg.clone(), |d| msg.replace(d, ""));
        assert_that!(bare.contains('{')).named(tag).is_false();
    }
}

#[test]
fn loss_serializes_kind_as_type_and_detail() {
    let v = serde_json::to_value(Loss::on("s1/4", LossKind::ColorDropped("#ff0000".into()))).unwrap();
    assert_that!(v).is_equal_to(json!({ "item": "s1/4", "kind": { "type": "color_dropped", "detail": "#ff0000" } }));
}

#[test]
fn loss_without_detail_serializes_type_only() {
    let v = serde_json::to_value(Loss::slide(LossKind::HiddenSlide)).unwrap();
    assert_that!(v).is_equal_to(json!({ "item": null, "kind": { "type": "hidden_slide" } }));
}

#[test]
fn import_report_to_json_adds_a_message_beside_each_loss() {
    let mut report = ImportReport::new(Mode::Retrofit);
    report.slides.push(slide(vec![Loss::slide(LossKind::HiddenSlide), Loss::on("s2/3", LossKind::MergedCells)]));
    let v = report.to_json(english());
    assert_that!(v["mode"]).is_equal_to(json!("retrofit"));
    assert_that!(v["slides"][0]["placement"]).is_equal_to(json!("flow"));
    assert_that!(v["slides"][0]["losses"][0]["message"]).is_equal_to(json!("Hidden slide imported as visible"));
    assert_that!(v["slides"][0]["losses"][1]["message"]).is_equal_to(json!("Merged cells flattened"));
    assert_that!(v["slides"][0]["losses"][1]["item"]).is_equal_to(json!("s2/3"));
}

#[test]
fn import_report_to_json_adds_a_message_beside_each_warning() {
    let mut report = ImportReport::new(Mode::Retrofit);
    report.warnings.push(Warning {
        slide: Some(2),
        item: Some("s2/4".into()),
        kind: WarningKind::UnsupportedChart("combo: barChart".into()),
    });
    let v = report.to_json(&Localizer::new(Some("fr")));
    assert_that!(v["warnings"][0]["message"]).is_equal_to(json!("Graphique non pris en charge : combo: barChart"));
    assert_that!(v["warnings"][0]["kind"])
        .is_equal_to(json!({ "type": "unsupported_chart", "detail": "combo: barChart" }));
}

#[test]
fn import_report_to_json_writes_coverage_when_measured() {
    let mut report = ImportReport::new(Mode::Retrofit);
    report.coverage = Some(Coverage {
        runs: Ratio { found: 3, total: 4 },
        missing: vec![Missing::Run { slide: 1, text: "Perdu".into() }],
        ..Coverage::default()
    });
    let v = report.to_json(english());
    assert_that!(v["coverage"]["runs"]).is_equal_to(json!({ "found": 3, "total": 4 }));
    assert_that!(v["coverage"]["missing"][0]).is_equal_to(json!({ "type": "run", "slide": 1, "text": "Perdu" }));
}

#[test]
fn import_report_loss_count_sums_every_slide() {
    let mut report = ImportReport::new(Mode::Retrofit);
    report.slides.push(slide(vec![Loss::slide(LossKind::HiddenSlide)]));
    report.slides.push(slide(vec![Loss::slide(LossKind::Effects), Loss::slide(LossKind::Transform)]));
    assert_that!(report.loss_count()).is_equal_to(3);
}

#[rstest]
#[case(Ratio { found: 0, total: 0 }, true)]
#[case(Ratio { found: 4, total: 4 }, true)]
#[case(Ratio { found: 3, total: 4 }, false)]
fn ratio_is_complete_when_everything_is_found(#[case] r: Ratio, #[case] complete: bool) {
    assert_that!(r.is_complete()).is_equal_to(complete);
}
```

> `warning_localize_in_every_locale_formats_its_own_message` : chaque message contient son détail, et aucune accolade ne reste hors du détail (un style de tableau est un GUID entre accolades, d'où le retrait du détail avant le contrôle).

Dans `crates/oku-slides-import/src/retrofit/chart_tests.rs`, avant `chart_of_a_doughnut_is_a_pie_with_two_radii` :

```rust
#[rstest]
#[case(ChartKind::Pie)]
#[case(ChartKind::Doughnut)]
fn chart_of_a_pie_reports_the_dropped_series(#[case] kind: ChartKind) {
    let mut c = source_chart(kind, Grouping::Clustered);
    c.series.push(series("2027", &[7.0, 8.0, 9.0]));
    assert_that!(chart(&c).unwrap().1).is_equal_to(vec![LossKind::SeriesDropped { kept: 1, dropped: 2 }]);
}

#[test]
fn chart_of_a_single_series_pie_loses_nothing() {
    let mut c = source_chart(ChartKind::Pie, Grouping::Clustered);
    c.series.truncate(1);
    assert_that!(chart(&c).unwrap().1).is_empty();
}

```

À la fin de `crates/oku-slides-import/src/retrofit/mod_tests.rs` :

```rust
#[test]
fn project_measures_the_coverage_of_its_own_deck() {
    let pic =
        item("p", (500.0, 143.0, 300.0, 200.0), ItemKind::Picture(Picture { asset: png(), crop: Default::default() }));
    let body = text_item("b", (66.0, 143.0, 300.0, 200.0), "Corps");
    let source = deck(vec![source_slide(1, Some("l2"), vec![body, pic])]);
    let c = plan_of(&source, &essentiel()).report.coverage.unwrap();
    assert_that!((c.runs.found, c.runs.total, c.images.found, c.images.total)).is_equal_to((1, 1, 1, 1));
}
```

Dans `crates/oku-slides-import/tests/retrofit.rs`, `retrofit_applies_without_issue` devient (et un test le suit) :

```rust
#[rstest]
fn retrofit_applies_without_issue_and_covers_everything(
    #[values("minimal", "kitchen", "alto")] fixture: &str,
    #[values("essentiel", "grilles")] pack: &str,
    #[values("papier", "pro")] theme: &str,
) {
    let (plan, deck) = import(fixture, pack, theme);
    assert_that!(deck.slides.len()).is_equal_to(source(fixture).slides.len());
    assert_that!(plan.report.slides.len()).is_equal_to(deck.slides.len());
    let c = plan.report.coverage.unwrap_or_else(|| panic!("couverture mesurée"));
    assert_that!(c.missing).is_empty();
    assert_that!([c.runs, c.images, c.tables, c.charts].iter().all(|r| r.is_complete())).is_true();
}

#[test]
fn retrofit_kitchen_coverage_counts_every_content() {
    let (plan, _) = import("kitchen", "essentiel", "papier");
    let c = plan.report.coverage.unwrap_or_else(|| panic!("couverture mesurée"));
    let totals = (c.runs.total, c.images.total, c.tables.total, c.charts.total);
    assert_that!(totals).is_equal_to((12, 2, 1, 1));
}

```

> `kitchen` : 12 runs — slide 1 « Tableau » et les cinq cellules non couvertes (« Titre », « A », « 1 », « B », « 2 ») ; slide 2 le titre, « Gauche », « Droite » ; slide 3 le titre, « Étape 1 », « Étape 2 » — ; 2 images (la photo de la slide 2, le repli de l'équation de la slide 3) ; 1 tableau ; 1 graphique.

- [ ] **Step 2 : lancer, constater l'échec**

```bash
cargo nextest run -p oku-slides-import
```

Attendu : échec de compilation (`coverage` absent, `LossKind::SeriesDropped`, `warning_message` inconnus).

- [ ] **Step 3 : implémentation**

`crates/oku-slides-import/src/coverage.rs` :

```rust
//! Couverture du contenu : ce que le deck Oku obtenu garde de la source.
//!
//! Mesurée sur le deck, pas sur le plan : un contenu compte s'il s'y
//! retrouve. La slide source `n` est comparée à la slide Oku `s<n>` (ids du
//! projecteur), plus les paramètres du deck (`title`, `subtitle`) que remplit
//! la couverture.
//!
//! | contenu source | retrouvé si la slide Oku contient |
//! |---|---|
//! | run de texte | son texte (espaces normalisées), à ses frontières de mots, dans un texte de l'élément projeté depuis son item (sinon de la slide) |
//! | run de moins de 3 caractères | son paragraphe (ou sa cellule) entier, à l'identique, dans l'élément (sinon la slide) |
//! | image (image, forme remplie d'une image, repli d'un intraduisible) | un élément `image` du même asset |
//! | tableau | un tableau dont `dataset.source` est la grille source (cellules couvertes `null`) |
//! | graphique | un graphique dont une colonne de `dataset.source` porte les valeurs de chaque série |
//!
//! Frontières de mots : un run qui commence (ou finit) par une lettre ou un
//! chiffre séparé de son voisin dans la source (espace, ponctuation, bord du
//! paragraphe) doit l'être aussi dans le texte trouvé ; un run collé à un
//! voisin au milieu d'un mot, ou bordé de ponctuation, peut l'être dans le deck. Un run
//! en capitales (`Caps::All`) se compare sans la casse ; un run sans lettre ni
//! chiffre ne compte pas. L'élément projeté depuis un item vient de
//! `origins` (id d'item → id d'élément, enfants compris) ; un titre posé en
//! paramètre n'y est pas et se cherche dans la slide.
//!
//! Les items omis par règle ([`Decor::skips`] : date, pied, numéro, en-tête et
//! pied répétés) sont hors du compte : la projection et la couverture partagent
//! ce prédicat.

use std::collections::BTreeMap;

use oku_slides_core::{Deck, Element, HostType, ParamType, ParamValue, Params, Slide, WidgetRegistry};
use serde_json::Value;

use crate::report::{Coverage, Missing, Ratio};
use crate::retrofit::decor::Decor;
use crate::retrofit::text::plain;
use crate::source::{self, Caps, Fill, Item, ItemKind, SourceDeck, TextBox};

/// Longueur (en caractères) sous laquelle un run se cherche par son paragraphe entier.
const SHORT_RUN: usize = 3;

/// `s` aux espaces normalisées.
fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Le texte de paragraphes riches, un paragraphe par ligne.
fn rich_text(paragraphs: &[oku_slides_core::Paragraph]) -> impl Iterator<Item = String> + '_ {
    paragraphs.iter().map(|p| p.runs.iter().map(|r| r.t.as_str()).collect())
}

/// Ce qu'une slide Oku montre : textes (de la slide, et de chaque élément avec
/// ses descendants), assets d'images, données de tableaux et de graphiques.
#[derive(Debug, Default)]
struct Seen<'d> {
    texts: Vec<String>,
    by_element: BTreeMap<&'d str, Vec<String>>,
    images: Vec<&'d str>,
    tables: Vec<&'d Value>,
    charts: Vec<&'d Value>,
}

impl<'d> Seen<'d> {
    /// Ajoute le texte `t` à la slide et aux éléments `owners`.
    fn text(&mut self, t: &str, owners: &[&'d str]) {
        let t = norm(t);
        for o in owners {
            self.by_element.entry(o).or_default().push(t.clone());
        }
        self.texts.push(t);
    }

    /// Les cellules textuelles d'un `dataset.source`.
    fn data_texts(&mut self, option: &'d Value, owners: &[&'d str]) {
        let rows = option.pointer("/dataset/source").and_then(Value::as_array).into_iter().flatten();
        for row in rows {
            for cell in row.as_array().into_iter().flatten() {
                if let Some(s) = cell.as_str() {
                    self.text(s, owners);
                }
            }
        }
    }

    /// Les paramètres `params` d'un widget `kind`, dans les éléments `owners`.
    fn params(&mut self, kind: &str, params: &'d Params, registry: &WidgetRegistry, owners: &[&'d str]) {
        let schema = registry.get(kind);
        for (name, value) in params.iter() {
            let ty = schema.and_then(|w| w.param(name)).map(|p| &p.ty);
            match (value, ty) {
                (ParamValue::Text(t), Some(ParamType::Plain) | None) => self.text(t, owners),
                (ParamValue::Text(a), Some(ParamType::Asset)) if kind == "image" => self.images.push(a),
                (ParamValue::Rich(ps), _) => {
                    for t in rich_text(ps) {
                        self.text(&t, owners);
                    }
                }
                (ParamValue::Slot(children), _) => self.elements(children, registry, owners),
                (ParamValue::Value(v), Some(t)) => {
                    match t.host() {
                        Some(HostType::Table) => self.tables.push(v),
                        Some(HostType::Chart) => self.charts.push(v),
                        None => continue,
                    }
                    self.data_texts(v, owners);
                }
                _ => {}
            }
        }
    }

    fn elements(&mut self, list: &'d [Element], registry: &WidgetRegistry, owners: &[&'d str]) {
        for e in list {
            let mut inner = owners.to_vec();
            inner.push(&e.id);
            self.params(&e.kind, &e.params, registry, &inner);
        }
    }

    /// Ce que montre `slide`, plus le titre et le sous-titre du deck.
    fn of(deck: &'d Deck, slide: Option<&'d Slide>, registry: &WidgetRegistry) -> Self {
        let mut seen = Seen::default();
        for t in [&deck.params.title, &deck.params.subtitle].into_iter().flatten() {
            seen.text(t, &[]);
        }
        if let Some(s) = slide {
            seen.params(&s.kind, &s.params, registry, &[]);
        }
        seen
    }

    /// Les textes où chercher le contenu de l'élément `element` (sinon de la slide).
    fn scope(&self, element: Option<&str>) -> &[String] {
        element.and_then(|e| self.by_element.get(e)).map_or(&self.texts, Vec::as_slice)
    }
}

/// Un run à chercher : son texte, s'il est collé à ses voisins, son paragraphe.
#[derive(Debug, Clone, PartialEq)]
struct Probe {
    /// Texte, espaces normalisées (en minuscules pour un run en capitales).
    text: String,
    /// Texte tel que relevé au rapport.
    shown: String,
    /// Le run commence un mot : le texte trouvé doit en commencer un aussi.
    word_start: bool,
    /// Le run finit un mot : le texte trouvé doit en finir un aussi.
    word_end: bool,
    /// Son paragraphe (ou sa cellule) entier, espaces normalisées.
    paragraph: String,
    /// Comparaison sans la casse.
    fold: bool,
}

/// Les runs à chercher de `tb` ; `whole` remplace le paragraphe (texte d'une cellule).
fn probes(tb: &TextBox, whole: Option<&str>) -> Vec<Probe> {
    let mut out = Vec::new();
    for p in &tb.paragraphs {
        let raw: Vec<&str> = p.runs.iter().map(|r| r.text.as_str()).collect();
        let paragraph = whole.map_or_else(|| norm(&raw.concat()), norm);
        for (k, r) in p.runs.iter().enumerate() {
            let text = norm(&r.text);
            if !text.chars().any(char::is_alphanumeric) {
                continue;
            }
            let before = raw[..k].concat().chars().last();
            let after = raw[k + 1..].concat().chars().next();
            let alnum = |c: Option<char>| c.is_some_and(char::is_alphanumeric);
            let fold = r.style.caps == Caps::All;
            let word_start = alnum(text.chars().next()) && !(alnum(before) && alnum(r.text.chars().next()));
            let word_end = alnum(text.chars().last()) && !(alnum(after) && alnum(r.text.chars().last()));
            out.push(Probe {
                text: if fold { text.to_lowercase() } else { text.clone() },
                shown: text,
                word_start,
                word_end,
                paragraph: paragraph.clone(),
                fold,
            });
        }
    }
    out
}

/// `probe` se trouve-t-il dans `hay`, à ses frontières de mots ?
fn within(hay: &str, probe: &Probe) -> bool {
    let hay = if probe.fold { hay.to_lowercase() } else { hay.to_owned() };
    hay.match_indices(probe.text.as_str()).any(|(at, m)| {
        let before = hay[..at].chars().last();
        let after = hay[at + m.len()..].chars().next();
        let open = |c: Option<char>| c.is_none_or(|c| !c.is_alphanumeric());
        (!probe.word_start || open(before)) && (!probe.word_end || open(after))
    })
}

/// `probe` est-il montré par l'un de `texts` ?
fn shows(texts: &[String], probe: &Probe) -> bool {
    if probe.text.chars().count() < SHORT_RUN {
        let same = |t: &String| {
            if probe.fold { t.to_lowercase() == probe.paragraph.to_lowercase() } else { *t == probe.paragraph }
        };
        return texts.iter().any(same);
    }
    texts.iter().any(|t| within(t, probe))
}
/// Le texte d'une cellule tel que le tableau du cœur le montre.
fn cell_text(c: &source::Cell) -> String {
    norm(&plain(&c.text).0)
}

/// La grille attendue d'un tableau source : texte plain des cellules, cellules couvertes `null`,
/// lignes complétées à la plus large.
fn table_grid(t: &source::Table) -> Value {
    let width = t.rows.iter().map(Vec::len).max().unwrap_or(0);
    let rows = t.rows.iter().map(|row| {
        let mut cells: Vec<Value> =
            row.iter().map(|c| if c.covered { Value::Null } else { Value::String(cell_text(c)) }).collect();
        cells.resize(width, Value::Null);
        Value::Array(cells)
    });
    Value::Array(rows.collect())
}

/// Les données de `option` portent-elles chaque série de `c` (une colonne de mêmes valeurs) ?
fn chart_matches(c: &source::Chart, option: &Value) -> bool {
    let Some(rows) = option.pointer("/dataset/source").and_then(Value::as_array) else { return false };
    let body = rows.get(1..).unwrap_or_default();
    let width = rows.first().and_then(Value::as_array).map_or(0, Vec::len);
    let column = |j: usize| -> Vec<Option<f64>> {
        body.iter().map(|r| r.as_array().and_then(|r| r.get(j)).and_then(Value::as_f64)).collect()
    };
    c.series.iter().all(|s| {
        let mut want: Vec<Option<f64>> = s.values.iter().map(|v| v.filter(|x| x.is_finite())).collect();
        want.resize(body.len(), None);
        want.len() == body.len() && (0..width).any(|j| column(j) == want)
    })
}

/// L'asset d'image d'un item : image, forme remplie d'une image, repli d'un intraduisible.
fn image_asset(item: &Item) -> Option<&str> {
    match &item.kind {
        ItemKind::Picture(p) => Some(&p.asset),
        ItemKind::Shape(s) => match &s.fill {
            Fill::Picture { asset } => Some(asset),
            _ => None,
        },
        ItemKind::Opaque(o) => o.fallback.as_deref(),
        _ => None,
    }
}

/// Les comptes en cours.
#[derive(Debug, Default)]
struct Tally {
    c: Coverage,
}

fn count(r: &mut Ratio, found: bool) {
    r.total += 1;
    r.found += u32::from(found);
}

impl Tally {
    /// Compte les runs `probes` de la slide `slide` dans `texts`.
    fn runs(&mut self, slide: u32, probes: Vec<Probe>, texts: &[String]) {
        for p in probes {
            let found = shows(texts, &p);
            count(&mut self.c.runs, found);
            if !found {
                self.c.missing.push(Missing::Run { slide, text: p.shown });
            }
        }
    }

    /// Compte `item` (et ses enfants) de la slide source `slide` contre `seen` ;
    /// `element` : l'élément projeté depuis un parent.
    fn item(&mut self, slide: u32, item: &Item, seen: &Seen<'_>, cx: &Cx<'_>, element: Option<&str>) {
        if cx.decor.skips(item) {
            return;
        }
        let element = cx.origins.get(&item.id).map(String::as_str).or(element);
        let texts = seen.scope(element);
        if let Some(tb) = item.text() {
            self.runs(slide, probes(tb, None), texts);
        }
        if let Some(asset) = image_asset(item) {
            let found = seen.images.contains(&asset);
            count(&mut self.c.images, found);
            if !found {
                self.c.missing.push(Missing::Image { slide, item: item.id.clone() });
            }
        }
        match &item.kind {
            ItemKind::Group { items, .. } => {
                for child in items {
                    self.item(slide, child, seen, cx, element);
                }
            }
            ItemKind::Table(t) => {
                for c in t.rows.iter().flatten().filter(|c| !c.covered) {
                    self.runs(slide, probes(&c.text, Some(&cell_text(c))), texts);
                }
                let grid = table_grid(t);
                let found = seen.tables.iter().any(|v| v.pointer("/dataset/source") == Some(&grid));
                count(&mut self.c.tables, found);
                if !found {
                    self.c.missing.push(Missing::Table { slide, item: item.id.clone() });
                }
            }
            ItemKind::Chart(c) => {
                let found = seen.charts.iter().any(|v| chart_matches(c, v));
                count(&mut self.c.charts, found);
                if !found {
                    self.c.missing.push(Missing::Chart { slide, item: item.id.clone() });
                }
            }
            _ => {}
        }
    }
}

/// Ce que le compte partage : le décor omis, l'origine des éléments.
struct Cx<'a> {
    decor: Decor,
    origins: &'a BTreeMap<String, String>,
}

/// La couverture du contenu de `source` dans `deck` (registre `registry` du
/// pack) ; `origins` : l'id de l'élément projeté depuis chaque item source.
pub fn coverage(
    source: &SourceDeck,
    deck: &Deck,
    registry: &WidgetRegistry,
    origins: &BTreeMap<String, String>,
) -> Coverage {
    let cx = Cx { decor: Decor::of(source), origins };
    let mut tally = Tally::default();
    for s in &source.slides {
        let seen = Seen::of(deck, deck.slide(&format!("s{}", s.number)), registry);
        for item in &s.items {
            tally.item(s.number, item, &seen, &cx, None);
        }
    }
    tally.c
}

#[cfg(test)]
#[path = "coverage_tests.rs"]
mod coverage_tests;
```

Dans `crates/oku-slides-import/src/retrofit/slide.rs`, `Projected` gagne son dernier champ :

```rust
    /// L'élément projeté depuis chaque item source (id d'item, id d'élément).
    pub origins: Vec<(String, String)>,
```

puis, dans `project_slide`, `let mut origins = Vec::new();` après `let mut fallbacks = Vec::new();`, et dans la boucle des unités, juste avant `let c = element(unit, unit_id, &content_cx);` :

```rust
            origins.extend(unit.items().into_iter().map(|i| (i.id.clone(), unit_id.clone())));
```

et la fin devient `Some(Projected { ops, report, assets, fallbacks, origins })`.

`crates/oku-slides-import/src/report.rs` complet :

```rust
//! Le rapport d'import : par slide, le modèle choisi, le placement, les pertes
//! et les replis ; les avertissements du lecteur ; la couverture du contenu.
//!
//! Une perte n'est jamais une erreur : l'import continue, le rapport la dit.
//! [`ImportReport::to_json`] donne la forme écrite par `--report`, avec le
//! message localisé de chaque perte et de chaque avertissement du lecteur.

use oku_slides_i18n::{Localize, Localizer};
use serde::Serialize;
use serde_json::Value;

use crate::source::{OpaqueReason, Warning, WarningKind};

/// Mode d'import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Contenu projeté sur un pack et un thème existants.
    Retrofit,
}

/// Où vont les contenus d'une slide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Placement {
    /// Blocs du slot `flow`, dans l'agencement du modèle.
    Flow,
    /// Éléments du slot libre, placés en cellules de la grille du modèle.
    Grid,
}

/// Effet de texte que le texte riche Oku n'exprime pas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunEffect {
    /// Souligné.
    Underline,
    /// Barré.
    Strike,
    /// Exposant ou indice.
    Baseline,
    /// Capitales ou petites capitales.
    Caps,
    /// Surlignage.
    Highlight,
}

/// Nature d'une perte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "detail")]
pub enum LossKind {
    /// Niveaux de liste aplatis.
    ListLevels,
    /// Fusions de cellules aplaties.
    MergedCells,
    /// Empilement à 100 % rendu en empilement simple.
    PercentStacked,
    /// Effet de texte perdu.
    RunStyle(RunEffect),
    /// Rotation, miroir ou rognage perdus.
    Transform,
    /// Couleur sans jeton proche dans le thème cible (`#rrggbb`).
    ColorDropped(String),
    /// Texte coupé aux bornes du cœur.
    TextTruncated,
    /// Tableau coupé aux bornes du cœur.
    TableTruncated,
    /// Item intraduisible sans image de repli.
    Opaque(OpaqueReason),
    /// Image dans un format que le rendu ne dessine pas (type MIME).
    UnsupportedImage(String),
    /// Slide masquée en diaporama : importée visible.
    HiddenSlide,
    /// Effets (ombres, reflets, 3D) perdus.
    Effects,
    /// Séries écartées : un graphique en secteurs n'en montre qu'une.
    SeriesDropped {
        /// Séries gardées.
        kept: u32,
        /// Séries écartées.
        dropped: u32,
    },
    /// En-tête ou pied répété sur les slides (texte, chiffres ôtés, ou nom de
    /// l'image), omis : le thème dessine les siens. Dit une fois, à sa première slide.
    RepeatedDecor {
        /// Texte du motif, ou nom de l'image.
        text: String,
        /// Nombre de slides où il apparaît.
        slides: u32,
    },
    /// Contenu sans place : le modèle choisi n'a aucun slot.
    NoSlot,
}

/// Une perte, sur un item source ou sur la slide entière.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Loss {
    /// Id de l'item source ; `None` pour la slide.
    pub item: Option<String>,
    /// Nature.
    pub kind: LossKind,
}

impl Loss {
    /// Perte sur l'item `item`.
    pub fn on(item: &str, kind: LossKind) -> Self {
        Self { item: Some(item.to_owned()), kind }
    }

    /// Perte sur la slide entière.
    pub fn slide(kind: LossKind) -> Self {
        Self { item: None, kind }
    }
}

/// Argument `effect` du message `import-loss-run-style`.
fn effect_name(e: RunEffect) -> &'static str {
    match e {
        RunEffect::Underline => "underline",
        RunEffect::Strike => "strike",
        RunEffect::Baseline => "baseline",
        RunEffect::Caps => "caps",
        RunEffect::Highlight => "highlight",
    }
}

/// Argument `reason` du message `import-loss-opaque`.
fn reason_name(r: &OpaqueReason) -> &'static str {
    match r {
        OpaqueReason::Media => "media",
        OpaqueReason::Ole => "ole",
        OpaqueReason::Math => "math",
        OpaqueReason::SmartArt => "smartart",
        OpaqueReason::Unknown(_) => "unknown",
    }
}

impl LossKind {
    /// Clé Fluent du message.
    pub(crate) fn message_id(&self) -> &'static str {
        match self {
            LossKind::ListLevels => "import-loss-list-levels",
            LossKind::MergedCells => "import-loss-merged-cells",
            LossKind::PercentStacked => "import-loss-percent-stacked",
            LossKind::RunStyle(_) => "import-loss-run-style",
            LossKind::Transform => "import-loss-transform",
            LossKind::ColorDropped(_) => "import-loss-color-dropped",
            LossKind::TextTruncated => "import-loss-text-truncated",
            LossKind::TableTruncated => "import-loss-table-truncated",
            LossKind::Opaque(_) => "import-loss-opaque",
            LossKind::UnsupportedImage(_) => "import-loss-unsupported-image",
            LossKind::HiddenSlide => "import-loss-hidden-slide",
            LossKind::Effects => "import-loss-effects",
            LossKind::SeriesDropped { .. } => "import-loss-series-dropped",
            LossKind::RepeatedDecor { .. } => "import-loss-repeated-decor",
            LossKind::NoSlot => "import-loss-no-slot",
        }
    }
}

impl Localize for Loss {
    fn localize(&self, l: &Localizer) -> String {
        let id = self.kind.message_id();
        match &self.kind {
            LossKind::RunStyle(e) => l.tr_with(id, &[("effect", effect_name(*e).into())]),
            LossKind::ColorDropped(hex) => l.tr_with(id, &[("color", hex.as_str().into())]),
            LossKind::Opaque(r) => l.tr_with(id, &[("reason", reason_name(r).into())]),
            LossKind::UnsupportedImage(mime) => l.tr_with(id, &[("format", mime.as_str().into())]),
            LossKind::SeriesDropped { kept, dropped } => {
                l.tr_with(id, &[("kept", (*kept).into()), ("dropped", (*dropped).into())])
            }
            LossKind::RepeatedDecor { text, slides } => {
                l.tr_with(id, &[("text", text.as_str().into()), ("slides", (*slides).into())])
            }
            LossKind::ListLevels
            | LossKind::MergedCells
            | LossKind::PercentStacked
            | LossKind::Transform
            | LossKind::TextTruncated
            | LossKind::TableTruncated
            | LossKind::HiddenSlide
            | LossKind::Effects
            | LossKind::NoSlot => l.tr(id),
        }
    }
}

/// Clé Fluent et détail du message d'un avertissement du lecteur.
fn warning_message(kind: &WarningKind) -> (&'static str, Option<&str>) {
    match kind {
        WarningKind::EffectIgnored(d) => ("import-warning-effect-ignored", Some(d)),
        WarningKind::Approximated(d) => ("import-warning-approximated", Some(d)),
        WarningKind::UnreadableItem(d) => ("import-warning-unreadable-item", Some(d)),
        WarningKind::UnreadablePart(d) => ("import-warning-unreadable-part", Some(d)),
        WarningKind::MissingPart(d) => ("import-warning-missing-part", Some(d)),
        WarningKind::UnsupportedChart(d) => ("import-warning-unsupported-chart", Some(d)),
        WarningKind::UnsupportedImage(d) => ("import-warning-unsupported-image", Some(d)),
        WarningKind::UnknownTableStyle(d) => ("import-warning-unknown-table-style", Some(d)),
        WarningKind::UnsafeLink(d) => ("import-warning-unsafe-link", Some(d)),
        WarningKind::NestingTooDeep(d) => ("import-warning-nesting-too-deep", Some(d)),
        WarningKind::FontEmbeddedUnreadable(d) => ("import-warning-font-embedded-unreadable", Some(d)),
        WarningKind::Truncated => ("import-warning-truncated", None),
    }
}

impl Localize for Warning {
    fn localize(&self, l: &Localizer) -> String {
        match warning_message(&self.kind) {
            (id, Some(detail)) => l.tr_with(id, &[("detail", detail.into())]),
            (id, None) => l.tr(id),
        }
    }
}

/// Ajoute à chaque objet de `written` (le tableau `key` de `v`) le message localisé de l'entrée de même rang.
fn add_messages<T: Localize>(v: &mut Value, key: &str, entries: &[T], l: &Localizer) {
    let Some(written) = v.get_mut(key).and_then(Value::as_array_mut) else { return };
    for (w, entry) in written.iter_mut().zip(entries) {
        if let Some(o) = w.as_object_mut() {
            o.insert("message".to_owned(), Value::String(entry.localize(l)));
        }
    }
}

/// Ce que devient une slide source.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SlideReport {
    /// Numéro de la slide source (1..).
    pub source: u32,
    /// Id de la slide Oku produite.
    pub slide: String,
    /// Id du modèle choisi.
    pub template: String,
    /// Placement des contenus.
    pub placement: Placement,
    /// Pertes.
    pub losses: Vec<Loss>,
    /// Sortes de widget de repli employées (`stack`…).
    pub fallbacks: Vec<String>,
}

/// Une proportion : `found` sur `total`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct Ratio {
    /// Retrouvés.
    pub found: u32,
    /// Attendus.
    pub total: u32,
}

impl Ratio {
    /// Vrai si tout est retrouvé (vrai aussi quand rien n'est attendu).
    pub fn is_complete(&self) -> bool {
        self.found >= self.total
    }
}

/// Un contenu source absent du deck obtenu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Missing {
    /// Texte d'un run.
    Run {
        /// Numéro de slide source.
        slide: u32,
        /// Texte du run, espaces normalisés.
        text: String,
    },
    /// Image.
    Image {
        /// Numéro de slide source.
        slide: u32,
        /// Id de l'item source.
        item: String,
    },
    /// Tableau.
    Table {
        /// Numéro de slide source.
        slide: u32,
        /// Id de l'item source.
        item: String,
    },
    /// Graphique.
    Chart {
        /// Numéro de slide source.
        slide: u32,
        /// Id de l'item source.
        item: String,
    },
}

/// Couverture du contenu, mesurée sur le deck obtenu.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Coverage {
    /// Runs de texte retrouvés.
    pub runs: Ratio,
    /// Images retrouvées.
    pub images: Ratio,
    /// Tableaux aux données égales.
    pub tables: Ratio,
    /// Graphiques aux données égales.
    pub charts: Ratio,
    /// Ce qui manque.
    pub missing: Vec<Missing>,
}

/// Le rapport d'un import.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImportReport {
    /// Mode.
    pub mode: Mode,
    /// Une entrée par slide source, dans l'ordre.
    pub slides: Vec<SlideReport>,
    /// Avertissements du lecteur.
    pub warnings: Vec<Warning>,
    /// Couverture du contenu ; `None` tant qu'elle n'est pas mesurée.
    pub coverage: Option<Coverage>,
}

impl ImportReport {
    /// Rapport vide du mode `mode`.
    pub fn new(mode: Mode) -> Self {
        Self { mode, slides: Vec::new(), warnings: Vec::new(), coverage: None }
    }

    /// Nombre total de pertes.
    pub fn loss_count(&self) -> usize {
        self.slides.iter().map(|s| s.losses.len()).sum()
    }

    /// La forme écrite : la sérialisation, plus un `message` localisé à côté
    /// de chaque perte et de chaque avertissement.
    pub fn to_json(&self, l: &Localizer) -> Value {
        // Ces types se sérialisent toujours : clés textuelles, nombres finis.
        let mut v = serde_json::to_value(self).unwrap_or_default();
        add_messages(&mut v, "warnings", &self.warnings, l);
        if let Some(slides) = v.get_mut("slides").and_then(Value::as_array_mut) {
            for (slide, report) in slides.iter_mut().zip(&self.slides) {
                add_messages(slide, "losses", &report.losses, l);
            }
        }
        v
    }
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod report_tests;
```

Dans `crates/oku-slides-import/src/retrofit/chart.rs`, la ligne `Pie` de la table de doc devient `/// | `Pie` | secteurs, première série seule (les autres : perte `SeriesDropped`) |` et le bras des secteurs :

```rust
        ChartKind::Pie | ChartKind::Doughnut => {
            let mut s = json!({ "type": "pie" });
            if c.kind == ChartKind::Doughnut {
                s["radius"] = json!(DOUGHNUT);
            }
            // `series` n'est pas vide (voir plus haut) ; au plus 256 séries (lot 1).
            let dropped = u32::try_from(c.series.len() - 1).unwrap_or(u32::MAX);
            if dropped > 0 {
                losses.push(LossKind::SeriesDropped { kept: 1, dropped });
            }
            json!({ "dataset": { "source": grid(c, &c.series[..1]) }, "series": [s] })
        }
```

`crates/oku-slides-import/src/retrofit/mod.rs` complet :

```rust
//! Projecteur retrofit : une [`SourceDeck`] projetée sur un pack et un thème
//! existants.
//!
//! Le style vient du thème cible : couleurs ramenées à ses jetons ([`ColorTokens`]),
//! texte riche réduit à ce que le cœur exprime, éléments libres placés sur la
//! grille d'une zone de corps approchée par ses jetons de page ([`PageTokens`]).
//! [`project`] rend un [`ImportPlan`] : une `AddSlide` par slide source (ids
//! `s<numéro>`, éléments `s<numéro>-<rang>`), les paramètres du deck tirés de
//! la première couverture, les notes, les assets référencés, les demandes de
//! widget et le rapport, couverture du contenu comprise. Aucun aléa, aucune
//! horloge : même source, même plan.

mod chart;
mod cluster;
mod color;
mod content;
pub(crate) mod decor;
mod group;
mod heading;
mod slide;
mod table;
mod template;
pub(crate) mod text;
mod zone;

use std::collections::BTreeMap;

use oku_slides_core::{Deck, Slide, Template, WidgetRegistry, apply_all};

pub use color::ColorTokens;
pub use zone::PageTokens;

use self::content::ContentCx;
use self::decor::Decor;
use self::slide::{SlideCx, project_slide};
use crate::coverage::coverage;
use crate::pattern::signature;
use crate::plan::{ImportPlan, WidgetRequest};
use crate::report::{ImportReport, Mode};
use crate::source::{Item, ItemKind, SourceDeck};

/// La cible d'un retrofit : les modèles et le registre du pack, les jetons du thème.
#[derive(Debug, Clone)]
pub struct RetrofitTarget<'a> {
    /// Les modèles du pack, dans l'ordre du pack.
    pub templates: &'a [Template],
    /// Le registre des widgets du pack (widgets de base et du pack).
    pub registry: &'a WidgetRegistry,
    /// Les couleurs effectives du thème.
    pub colors: ColorTokens,
    /// Les jetons de page effectifs du thème.
    pub page: PageTokens,
}

/// Le modèle d'un pack sans modèle : une slide `content` nue.
fn bare_template() -> Template {
    Template {
        id: "content".to_owned(),
        name: "content".to_owned(),
        description: String::new(),
        slide: Slide::new("content", "content"),
    }
}

/// Ajoute l'occurrence `item` (rendue par le repli `fallback` sur la slide
/// `slide`) à la demande de même signature, ou en crée une.
fn request(requests: &mut Vec<WidgetRequest>, item: &Item, fallback: &str, slide: &str) {
    let children = match &item.kind {
        ItemKind::Group { items, .. } => items.as_slice(),
        _ => std::slice::from_ref(item),
    };
    let sig = signature(children);
    match requests.iter_mut().find(|r| r.signature == sig) {
        Some(r) => {
            if r.slides.last().map(String::as_str) != Some(slide) {
                r.slides.push(slide.to_owned());
            }
        }
        None => requests.push(WidgetRequest {
            signature: sig,
            slides: vec![slide.to_owned()],
            sample: children.to_vec(),
            fallback: fallback.to_owned(),
        }),
    }
}

/// Le plan qui construit, sur la cible `target`, le deck Oku de `deck`.
///
/// Échelle : la page source est ramenée dans la page du thème en gardant ses
/// proportions (`min(largeur / w, hauteur / h)`, soit `1920 / w` pour une
/// source 16:9). Un pack sans modèle reçoit des slides `content` nues.
///
/// La couverture (`report.coverage`) est mesurée sur le deck que donnent les
/// ops appliquées à un deck vide sans thème ni pack embarqué : le thème ne
/// change pas le contenu, et le registre de la cible suffit à appliquer.
/// `None` seulement si le cœur refusait le plan.
pub fn project(deck: &SourceDeck, target: &RetrofitTarget<'_>) -> ImportPlan {
    let bare = [bare_template()];
    let templates = if target.templates.is_empty() { &bare[..] } else { target.templates };
    let profiles = template::profiles(templates, target.registry);
    let scale = if deck.size.w > 0.0 && deck.size.h > 0.0 {
        (target.page.width / deck.size.w).min(target.page.height / deck.size.h)
    } else {
        1.0
    };
    let colors = target.colors.clone().with_accents(deck);
    let decor = Decor::of(deck);
    let cx = SlideCx {
        registry: target.registry,
        page: &target.page,
        content: ContentCx { tokens: &colors, assets: &deck.assets, track: (0.0, 0.0) },
        layouts: &deck.layouts,
        size: deck.size,
        scale,
        decor: &decor,
    };
    let mut report = ImportReport::new(Mode::Retrofit);
    report.warnings = deck.warnings.clone();
    let mut plan = ImportPlan { ops: Vec::new(), assets: BTreeMap::new(), widget_requests: Vec::new(), report };
    let mut origins = BTreeMap::new();
    let mut deck_titled = false;
    for src in &deck.slides {
        let Some(p) = project_slide(src, &profiles, &mut deck_titled, &cx) else { continue };
        plan.ops.extend(p.ops);
        origins.extend(p.origins);
        for id in p.assets {
            if let Some(a) = deck.assets.get(&id) {
                plan.assets.insert(id, a.clone());
            }
        }
        for (item, fallback) in &p.fallbacks {
            request(&mut plan.widget_requests, item, fallback, &p.report.slide);
        }
        plan.report.slides.push(p.report);
    }
    let mut scratch = Deck::empty();
    plan.report.coverage = apply_all(&mut scratch, plan.ops.clone(), target.registry)
        .ok()
        .map(|()| coverage(deck, &scratch, target.registry, &origins));
    plan
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
```

`crates/oku-slides-import/src/lib.rs` complet :

```rust
//! Import d'une présentation PowerPoint en représentation intermédiaire.
//!
//! [`read_pptx`] lit les octets d'un `.pptx` et rend une [`source::SourceDeck`] :
//! géométrie en pt, styles texte résolus, couleurs avec leur provenance,
//! tableaux, graphiques, SmartArt, replis, polices. Bornes : [`ReadLimits`] ;
//! erreurs : [`ImportError`].
//!
//! Un projecteur rend un [`ImportPlan`] : les opérations qui construisent le
//! deck Oku, les assets qu'elles référencent et un [`ImportReport`] (pertes,
//! replis, couverture du contenu). Le retrofit ([`retrofit::project`]) projette
//! la source sur un pack et un thème existants ([`RetrofitTarget`]) ;
//! [`coverage::coverage`] mesure ce que le deck obtenu garde de la source.

pub mod coverage;
pub mod plan;
pub mod report;
pub mod retrofit;
pub mod source;

mod collect;
mod error;
mod limits;
mod pattern;
mod pptx;
#[cfg(test)]
mod test_support;

pub use error::{ImportError, ImportErrorKind};
pub use limits::ReadLimits;
pub use plan::ImportPlan;
pub use pptx::read_pptx;
pub use report::ImportReport;
pub use retrofit::RetrofitTarget;
```

Messages : dans chaque `crates/oku-slides-i18n/src/locales/<locale>/document.ftl`, ajouter `import-loss-series-dropped` juste après `import-loss-effects` (avant `import-loss-repeated-decor`) et les 12 clés `import-warning-*` à la fin, dans l'ordre ci-dessous (`import-loss-series-dropped` a les variables `$kept` et `$dropped` partout ; les `import-warning-*` ont `$detail`, sauf `import-warning-truncated`). En chinois, le nombre colle à son classificateur (`{ $kept }个`) : `numeric_arguments_stick_to_their_cjk_classifier` le vérifie.

`import-loss-series-dropped` :

| locale | message |
|---|---|
| ar | يعرض المخطط الدائري سلسلة واحدة: تم الاحتفاظ بـ { $kept } واستبعاد { $dropped } |
| bn | পাই চার্ট শুধু একটি সিরিজ দেখায়: { $kept }টি রাখা হয়েছে, { $dropped }টি বাদ দেওয়া হয়েছে |
| de | Ein Kreisdiagramm zeigt nur eine Datenreihe: { $kept } behalten, { $dropped } verworfen |
| en | A pie chart shows a single series: { $kept } kept, { $dropped } dropped |
| es | Un gráfico circular muestra una sola serie: { $kept } conservada, { $dropped } descartadas |
| fr | Un graphique en secteurs ne montre qu'une série : { $kept } gardée, { $dropped } écartées |
| hi | पाई चार्ट केवल एक शृंखला दिखाता है: { $kept } रखी गई, { $dropped } हटाई गईं |
| it | Un grafico a torta mostra una sola serie: { $kept } mantenuta, { $dropped } scartate |
| ja | 円グラフは 1 系列のみ表示します。{ $kept } 系列を保持、{ $dropped } 系列を除外しました |
| ko | 원형 차트는 계열 하나만 표시합니다. { $kept }개 유지, { $dropped }개 제외 |
| nl | Een cirkeldiagram toont één reeks: { $kept } behouden, { $dropped } weggelaten |
| pl | Wykres kołowy pokazuje jedną serię: zachowano { $kept }, pominięto { $dropped } |
| pt | Um gráfico de pizza mostra uma única série: { $kept } mantida, { $dropped } descartadas |
| ru | Круговая диаграмма показывает один ряд: сохранено { $kept }, отброшено { $dropped } |
| th | แผนภูมิวงกลมแสดงได้เพียงชุดข้อมูลเดียว: เก็บไว้ { $kept } ตัดออก { $dropped } |
| tr | Pasta grafik tek bir seri gösterir: { $kept } korundu, { $dropped } çıkarıldı |
| uk | Кругова діаграма показує один ряд: збережено { $kept }, відкинуто { $dropped } |
| vi | Biểu đồ tròn chỉ hiển thị một chuỗi: giữ { $kept }, bỏ { $dropped } |
| zh-Hans | 饼图只显示一个系列：保留 { $kept }个，舍弃 { $dropped }个 |
| zh-Hant | 圓餅圖只顯示一個數列：保留 { $kept }個，捨棄 { $dropped }個 |

`import-warning-effect-ignored` :

| locale | message |
|---|---|
| ar | لم يُستورد التأثير: { $detail } |
| bn | ইফেক্ট ইমপোর্ট করা হয়নি: { $detail } |
| de | Effekt nicht importiert: { $detail } |
| en | Effect not imported: { $detail } |
| es | Efecto no importado: { $detail } |
| fr | Effet non importé : { $detail } |
| hi | प्रभाव आयात नहीं हुआ: { $detail } |
| it | Effetto non importato: { $detail } |
| ja | 効果はインポートされませんでした: { $detail } |
| ko | 효과를 가져오지 못했습니다: { $detail } |
| nl | Effect niet geïmporteerd: { $detail } |
| pl | Nie zaimportowano efektu: { $detail } |
| pt | Efeito não importado: { $detail } |
| ru | Эффект не импортирован: { $detail } |
| th | ไม่ได้นำเข้าเอฟเฟกต์: { $detail } |
| tr | Efekt içe aktarılmadı: { $detail } |
| uk | Ефект не імпортовано: { $detail } |
| vi | Không nhập hiệu ứng: { $detail } |
| zh-Hans | 效果未导入：{ $detail } |
| zh-Hant | 效果未匯入：{ $detail } |

`import-warning-approximated` :

| locale | message |
|---|---|
| ar | عرض تقريبي: { $detail } |
| bn | আনুমানিকভাবে দেখানো হয়েছে: { $detail } |
| de | Näherungsweise dargestellt: { $detail } |
| en | Approximated: { $detail } |
| es | Representación aproximada: { $detail } |
| fr | Rendu approché : { $detail } |
| hi | अनुमानित रूप में दिखाया गया: { $detail } |
| it | Resa approssimata: { $detail } |
| ja | 近似して表示しました: { $detail } |
| ko | 근사치로 표시했습니다: { $detail } |
| nl | Benaderd weergegeven: { $detail } |
| pl | Odwzorowanie przybliżone: { $detail } |
| pt | Renderização aproximada: { $detail } |
| ru | Приблизительное отображение: { $detail } |
| th | แสดงผลแบบประมาณ: { $detail } |
| tr | Yaklaşık olarak gösterildi: { $detail } |
| uk | Наближене відображення: { $detail } |
| vi | Hiển thị gần đúng: { $detail } |
| zh-Hans | 近似呈现：{ $detail } |
| zh-Hant | 近似呈現：{ $detail } |

`import-warning-unreadable-item` :

| locale | message |
|---|---|
| ar | تم تخطي عنصر غير مقروء: { $detail } |
| bn | অপাঠযোগ্য আইটেম বাদ দেওয়া হয়েছে: { $detail } |
| de | Unlesbares Element übersprungen: { $detail } |
| en | Unreadable item skipped: { $detail } |
| es | Elemento ilegible omitido: { $detail } |
| fr | Élément illisible ignoré : { $detail } |
| hi | अपठनीय आइटम छोड़ा गया: { $detail } |
| it | Elemento illeggibile ignorato: { $detail } |
| ja | 読み取れない項目をスキップしました: { $detail } |
| ko | 읽을 수 없는 항목을 건너뛰었습니다: { $detail } |
| nl | Onleesbaar element overgeslagen: { $detail } |
| pl | Pominięto nieczytelny element: { $detail } |
| pt | Item ilegível ignorado: { $detail } |
| ru | Нечитаемый элемент пропущен: { $detail } |
| th | ข้ามรายการที่อ่านไม่ได้: { $detail } |
| tr | Okunamayan öğe atlandı: { $detail } |
| uk | Нечитабельний елемент пропущено: { $detail } |
| vi | Đã bỏ qua mục không đọc được: { $detail } |
| zh-Hans | 已跳过无法读取的项目：{ $detail } |
| zh-Hant | 已略過無法讀取的項目：{ $detail } |

`import-warning-unreadable-part` :

| locale | message |
|---|---|
| ar | جزء غير مقروء: { $detail } |
| bn | অপাঠযোগ্য অংশ: { $detail } |
| de | Unlesbarer Teil: { $detail } |
| en | Unreadable part: { $detail } |
| es | Parte ilegible: { $detail } |
| fr | Partie illisible : { $detail } |
| hi | अपठनीय भाग: { $detail } |
| it | Parte illeggibile: { $detail } |
| ja | 読み取れないパーツ: { $detail } |
| ko | 읽을 수 없는 부분: { $detail } |
| nl | Onleesbaar onderdeel: { $detail } |
| pl | Nieczytelna część: { $detail } |
| pt | Parte ilegível: { $detail } |
| ru | Нечитаемая часть: { $detail } |
| th | ส่วนที่อ่านไม่ได้: { $detail } |
| tr | Okunamayan bölüm: { $detail } |
| uk | Нечитабельна частина: { $detail } |
| vi | Phần không đọc được: { $detail } |
| zh-Hans | 无法读取的部件：{ $detail } |
| zh-Hant | 無法讀取的部分：{ $detail } |

`import-warning-missing-part` :

| locale | message |
|---|---|
| ar | جزء مفقود: { $detail } |
| bn | অনুপস্থিত অংশ: { $detail } |
| de | Fehlender Teil: { $detail } |
| en | Missing part: { $detail } |
| es | Falta una parte: { $detail } |
| fr | Partie manquante : { $detail } |
| hi | अनुपस्थित भाग: { $detail } |
| it | Parte mancante: { $detail } |
| ja | パーツがありません: { $detail } |
| ko | 누락된 부분: { $detail } |
| nl | Ontbrekend onderdeel: { $detail } |
| pl | Brakująca część: { $detail } |
| pt | Parte ausente: { $detail } |
| ru | Отсутствующая часть: { $detail } |
| th | ส่วนที่ขาดหายไป: { $detail } |
| tr | Eksik bölüm: { $detail } |
| uk | Відсутня частина: { $detail } |
| vi | Thiếu phần: { $detail } |
| zh-Hans | 缺少部件：{ $detail } |
| zh-Hant | 缺少部分：{ $detail } |

`import-warning-unsupported-chart` :

| locale | message |
|---|---|
| ar | مخطط غير مدعوم: { $detail } |
| bn | অসমর্থিত চার্ট: { $detail } |
| de | Nicht unterstütztes Diagramm: { $detail } |
| en | Unsupported chart: { $detail } |
| es | Gráfico no compatible: { $detail } |
| fr | Graphique non pris en charge : { $detail } |
| hi | असमर्थित चार्ट: { $detail } |
| it | Grafico non supportato: { $detail } |
| ja | 未対応のグラフ: { $detail } |
| ko | 지원되지 않는 차트: { $detail } |
| nl | Niet-ondersteunde grafiek: { $detail } |
| pl | Nieobsługiwany wykres: { $detail } |
| pt | Gráfico não suportado: { $detail } |
| ru | Неподдерживаемая диаграмма: { $detail } |
| th | แผนภูมิที่ไม่รองรับ: { $detail } |
| tr | Desteklenmeyen grafik: { $detail } |
| uk | Непідтримувана діаграма: { $detail } |
| vi | Biểu đồ không được hỗ trợ: { $detail } |
| zh-Hans | 不支持的图表：{ $detail } |
| zh-Hant | 不支援的圖表：{ $detail } |

`import-warning-unsupported-image` :

| locale | message |
|---|---|
| ar | تنسيق صورة غير مدعوم: { $detail } |
| bn | অসমর্থিত ছবির ফরম্যাট: { $detail } |
| de | Bildformat nicht unterstützt: { $detail } |
| en | Image format not supported: { $detail } |
| es | Formato de imagen no compatible: { $detail } |
| fr | Format d'image non pris en charge : { $detail } |
| hi | असमर्थित छवि फ़ॉर्मैट: { $detail } |
| it | Formato immagine non supportato: { $detail } |
| ja | 未対応の画像形式: { $detail } |
| ko | 지원되지 않는 이미지 형식: { $detail } |
| nl | Afbeeldingsindeling niet ondersteund: { $detail } |
| pl | Nieobsługiwany format obrazu: { $detail } |
| pt | Formato de imagem não suportado: { $detail } |
| ru | Неподдерживаемый формат изображения: { $detail } |
| th | รูปแบบรูปภาพที่ไม่รองรับ: { $detail } |
| tr | Desteklenmeyen görüntü biçimi: { $detail } |
| uk | Непідтримуваний формат зображення: { $detail } |
| vi | Định dạng ảnh không được hỗ trợ: { $detail } |
| zh-Hans | 不支持的图像格式：{ $detail } |
| zh-Hant | 不支援的影像格式：{ $detail } |

`import-warning-unknown-table-style` :

| locale | message |
|---|---|
| ar | نمط جدول غير معروف: { $detail } |
| bn | অজানা টেবিল স্টাইল: { $detail } |
| de | Unbekannte Tabellenformatvorlage: { $detail } |
| en | Unknown table style: { $detail } |
| es | Estilo de tabla desconocido: { $detail } |
| fr | Style de tableau inconnu : { $detail } |
| hi | अज्ञात तालिका शैली: { $detail } |
| it | Stile di tabella sconosciuto: { $detail } |
| ja | 不明な表のスタイル: { $detail } |
| ko | 알 수 없는 표 스타일: { $detail } |
| nl | Onbekende tabelstijl: { $detail } |
| pl | Nieznany styl tabeli: { $detail } |
| pt | Estilo de tabela desconhecido: { $detail } |
| ru | Неизвестный стиль таблицы: { $detail } |
| th | สไตล์ตารางที่ไม่รู้จัก: { $detail } |
| tr | Bilinmeyen tablo stili: { $detail } |
| uk | Невідомий стиль таблиці: { $detail } |
| vi | Kiểu bảng không xác định: { $detail } |
| zh-Hans | 未知的表格样式：{ $detail } |
| zh-Hant | 未知的表格樣式：{ $detail } |

`import-warning-unsafe-link` :

| locale | message |
|---|---|
| ar | تم رفض الرابط: { $detail } |
| bn | লিঙ্ক প্রত্যাখ্যান করা হয়েছে: { $detail } |
| de | Link abgelehnt: { $detail } |
| en | Link refused: { $detail } |
| es | Vínculo rechazado: { $detail } |
| fr | Lien refusé : { $detail } |
| hi | लिंक अस्वीकार किया गया: { $detail } |
| it | Collegamento rifiutato: { $detail } |
| ja | リンクを拒否しました: { $detail } |
| ko | 링크가 거부되었습니다: { $detail } |
| nl | Koppeling geweigerd: { $detail } |
| pl | Odrzucono łącze: { $detail } |
| pt | Link recusado: { $detail } |
| ru | Ссылка отклонена: { $detail } |
| th | ปฏิเสธลิงก์: { $detail } |
| tr | Bağlantı reddedildi: { $detail } |
| uk | Посилання відхилено: { $detail } |
| vi | Liên kết bị từ chối: { $detail } |
| zh-Hans | 链接被拒绝：{ $detail } |
| zh-Hant | 連結遭拒：{ $detail } |

`import-warning-nesting-too-deep` :

| locale | message |
|---|---|
| ar | تداخل عميق جدًا، تم قطعه: { $detail } |
| bn | নেস্টিং খুব গভীর, কেটে দেওয়া হয়েছে: { $detail } |
| de | Verschachtelung zu tief, abgeschnitten: { $detail } |
| en | Nesting too deep, cut off: { $detail } |
| es | Anidamiento demasiado profundo, cortado: { $detail } |
| fr | Imbrication trop profonde, coupée : { $detail } |
| hi | नेस्टिंग बहुत गहरी, काटी गई: { $detail } |
| it | Annidamento troppo profondo, troncato: { $detail } |
| ja | 入れ子が深すぎるため切り詰めました: { $detail } |
| ko | 중첩이 너무 깊어 잘렸습니다: { $detail } |
| nl | Nesting te diep, afgekapt: { $detail } |
| pl | Zbyt głębokie zagnieżdżenie, obcięto: { $detail } |
| pt | Aninhamento profundo demais, cortado: { $detail } |
| ru | Слишком глубокая вложенность, обрезано: { $detail } |
| th | การซ้อนลึกเกินไป ถูกตัด: { $detail } |
| tr | İç içe yerleştirme çok derin, kesildi: { $detail } |
| uk | Надто глибоке вкладення, обрізано: { $detail } |
| vi | Lồng nhau quá sâu, đã cắt: { $detail } |
| zh-Hans | 嵌套过深，已截断：{ $detail } |
| zh-Hant | 巢狀層級過深，已截斷：{ $detail } |

`import-warning-font-embedded-unreadable` :

| locale | message |
|---|---|
| ar | خط مضمَّن غير مقروء: { $detail } |
| bn | এমবেড করা ফন্ট পড়া যায়নি: { $detail } |
| de | Eingebettete Schriftart unlesbar: { $detail } |
| en | Embedded font unreadable: { $detail } |
| es | Fuente incrustada ilegible: { $detail } |
| fr | Police incorporée illisible : { $detail } |
| hi | एम्बेड किया गया फ़ॉन्ट अपठनीय: { $detail } |
| it | Carattere incorporato illeggibile: { $detail } |
| ja | 埋め込みフォントを読み取れません: { $detail } |
| ko | 포함된 글꼴을 읽을 수 없습니다: { $detail } |
| nl | Ingesloten lettertype onleesbaar: { $detail } |
| pl | Nieczytelna osadzona czcionka: { $detail } |
| pt | Fonte incorporada ilegível: { $detail } |
| ru | Нечитаемый внедрённый шрифт: { $detail } |
| th | อ่านฟอนต์ที่ฝังไว้ไม่ได้: { $detail } |
| tr | Gömülü yazı tipi okunamadı: { $detail } |
| uk | Нечитабельний вбудований шрифт: { $detail } |
| vi | Không đọc được phông chữ nhúng: { $detail } |
| zh-Hans | 无法读取嵌入的字体：{ $detail } |
| zh-Hant | 無法讀取內嵌字型：{ $detail } |

`import-warning-truncated` :

| locale | message |
|---|---|
| ar | تحذيرات كثيرة جدًا: تم حذف التحذيرات التالية |
| bn | অনেক বেশি সতর্কবার্তা: বাকিগুলো বাদ দেওয়া হয়েছে |
| de | Zu viele Warnungen: weitere werden ausgelassen |
| en | Too many warnings: the rest are omitted |
| es | Demasiadas advertencias: se omiten las siguientes |
| fr | Trop d'avertissements : les suivants sont omis |
| hi | बहुत अधिक चेतावनियाँ: बाकी छोड़ दी गईं |
| it | Troppi avvisi: i successivi sono omessi |
| ja | 警告が多すぎます。以降の警告は省略されます |
| ko | 경고가 너무 많습니다. 이후 경고는 생략됩니다 |
| nl | Te veel waarschuwingen: de rest wordt weggelaten |
| pl | Zbyt wiele ostrzeżeń: kolejne pominięto |
| pt | Avisos demais: os seguintes foram omitidos |
| ru | Слишком много предупреждений: остальные пропущены |
| th | คำเตือนมากเกินไป: ละคำเตือนที่เหลือ |
| tr | Çok fazla uyarı: sonrakiler atlandı |
| uk | Забагато попереджень: решту пропущено |
| vi | Quá nhiều cảnh báo: các cảnh báo tiếp theo bị bỏ qua |
| zh-Hans | 警告过多：其余警告已省略 |
| zh-Hant | 警告過多：其餘警告已省略 |


- [ ] **Step 4 : lancer, constater le succès**

```bash
cargo nextest run -p oku-slides-import -p oku-slides-i18n
```

Attendu : tout passe (682 tests au brouillon pour les deux crates) ; les instantanés de la tâche 9 ne changent pas (la couverture est au rapport, pas aux ops).

- [ ] **Step 5 : vérifier, commit, sous-tâche**

```bash
cargo fmt --all && cargo clippy -p oku-slides-import -p oku-slides-i18n --all-targets -- -D warnings && cargo nextest run -p oku-slides-import -p oku-slides-i18n
git add crates/oku-slides-import/src/retrofit/slide.rs crates/oku-slides-import/src/coverage.rs crates/oku-slides-import/src/coverage_tests.rs crates/oku-slides-import/src/lib.rs crates/oku-slides-import/src/report.rs crates/oku-slides-import/src/report_tests.rs crates/oku-slides-import/src/retrofit/chart.rs crates/oku-slides-import/src/retrofit/chart_tests.rs crates/oku-slides-import/src/retrofit/mod.rs crates/oku-slides-import/src/retrofit/mod_tests.rs crates/oku-slides-import/tests/retrofit.rs crates/oku-slides-i18n/src/locales/*/document.ftl && git diff --cached --name-only
git commit -F <scratchpad>/msg.txt   # feat(import) : couverture du contenu au rapport, avertissements localisés, séries écartées des secteurs (t-692)
oku cli task done --project slider --id t-806
```

---

### Task 11: Commande `oku-slides import`

**Écarts d'interface :**
- Aides partagées extraites de `main.rs` dans `src/library.rs` (au lieu de dupliquer) : `read_pack_dir(storage, dir) -> Option<PackFiles>` (la lecture `pack.json` + `widgets/*.typ` que faisait `beside_deck`), `beside_deck` (déplacé, il s'appuie dessus), `resolve_pack(storage, library, l, wanted) -> Result<PackFiles>` (id, dossier, nom), `embed_theme` (déplacé et généralisé : id **ou** nom), `theme_library`, `pack_library`. Dans `main.rs`, `write_document(platform, l, fonts, widgets, bundle, output) -> Result<Written>` (rendu, PDF, sources jointes, écriture atomique) sert `save` et `import`.
- `--mode` vaut `retrofit` par défaut (`#[arg(long, value_enum, default_value_t = ImportMode::Retrofit)]`) ; `hifi` est reconnu et refusé.
- `ImportPlan::report_json(&self, l: &Localizer) -> Value` (crate d'import) : `report.to_json(l)` plus `widget_requests` (décision 2 : les demandes vont au plan **et** au rapport). C'est ce qu'écrit `--report`.
- `uuid` passe des dépendances de dev aux dépendances de la CLI (lecture d'un id de pack ou de thème) ; `zip` entre en dépendance de dev (les fixtures `minimal` et `kitchen` sont des dossiers, zippés à la volée par le test).
- Avant toute lecture, la taille du fichier est comparée à `ReadLimits::default().max_input_bytes` (`Storage::read_limited`) : au-delà, le message localisé `import-too-large` de l'import, sans rien lire (révision 1, item 11).
- Erreurs localisées (révision 1, items 13 et mineurs) : un pack dont `PackManifest::from_files` échoue → `cli-import-bad-pack` (« pack illisible … ») ; un refus de `bundle.apply` → son message (`SourcesError: Localize`) ; un échec du rendu ou de l'écriture → contexte `cli-import-write-failed`.
- Jetons effectifs : `oku_slides_core::tokens::effective_tokens(default_tokens(), jetons du thème)` (tâche 1), puis `ColorTokens::from_tokens(&tokens)` (jetons entiers, tâche 3).
- `cli-imported` accorde le nombre de slides, de pages, de pertes et de demandes par les catégories plurielles de chaque langue (une seule forme en japonais, coréen, chinois, thaï, vietnamien ; en bengali, hindi et turc, le nom ne varie pas, sauf « हानि / हानियाँ »).
- Un fichier qui n'est pas un zip donne le message `import-unreadable` du lecteur (« This presentation could not be read ») ; `import-not-a-presentation` est réservé à un zip sans présentation (lot 1).
- Clés Fluent : `cli-imported`, `cli-import-label`, `cli-import-pack-required`, `cli-import-unknown-pack`, `cli-import-bad-pack`, `cli-import-write-failed`, `cli-import-hifi-later` (messages) ; `cli-cmd-import`, `cli-arg-pptx`, `cli-arg-mode`, `cli-arg-pack`, `cli-arg-import-theme`, `cli-arg-report` (aide ; `-o` reprend `cli-arg-output`).

Déroulé de `import` : refus de `hifi` ; `--pack` exigé ; pack résolu (id → version courante de la bibliothèque ; dossier contenant `pack.json` → lu tel quel ; sinon nom sans casse parmi `library.packs()`) ; `.pptx` lu s'il ne dépasse pas `max_input_bytes` (`read_limited`, sinon `import-too-large`), puis `read_pptx` (`ReadLimits::default()`, erreur → son message localisé) ; deck vide qui référence le pack, sources créées par `sources_with_pack` (pack embarqué, widgets sondés, sources recréées avec le registre sondé) ; thème `--theme` (id ou nom) embarqué, sinon `adopt_default_theme` ; jetons effectifs = `effective_tokens(default_tokens(), theme_tokens(…))` → `ColorTokens::from_tokens` et `PageTokens::from_tokens` ; `retrofit::project` ; octets des `plan.assets` ajoutés (`add_asset`, même id) ; `apply_all` sur la vue du bundle (refus → message localisé), puis `bundle.apply(deck, cli-import-label, None)` : un seul pas d'historique ; `report_issues` ; `write_document` (contexte `cli-import-write-failed`) ; `--report` → `serde_json::to_string_pretty(&plan.report_json(l))` ; ligne `cli-imported` (chemin, slides, pages, pertes, demandes de widget).

**Files:**
- Create: `crates/oku-slides-cli/src/import.rs`, `crates/oku-slides-cli/src/library.rs`, `crates/oku-slides-cli/src/library_tests.rs`, `crates/oku-slides-cli/tests/import.rs`
- Modify: `crates/oku-slides-cli/Cargo.toml`, `Cargo.lock`, `crates/oku-slides-cli/src/main.rs`, `crates/oku-slides-import/src/plan.rs`, `crates/oku-slides-import/src/plan_tests.rs`, `crates/oku-slides-i18n/src/locales/*/cli.ftl` (×20)

**Interfaces:**
- Consumes : `oku_slides_import::{read_pptx, ReadLimits, RetrofitTarget, ImportPlan}`, `oku_slides_import::retrofit::{project, ColorTokens, PageTokens}` (9, 10) ; `oku_slides_core::{PackManifest, PackRef, Deck, SourceBundle, apply_all}` (1) ; `oku_slides_core::tokens::effective_tokens` (1) ; `oku_slides_platform::storage::StorageErrorKind` ; `oku_slides_session::{PackLibrary, ThemeLibrary, adopt_default_theme, theme_tokens, check_pack_id}`, `oku_slides_session::theme_schema::default_tokens` ; dans `main.rs` : `registry`, `sources_with_pack`, `report_issues`, `pack_render::render`.
- Produces :

```rust
// oku_slides_import::plan
impl ImportPlan { pub fn report_json(&self, l: &Localizer) -> serde_json::Value; }
// oku-slides-cli : crate::import
pub(crate) enum ImportMode { Retrofit, Hifi } // clap::ValueEnum
pub(crate) struct ImportArgs { input: PathBuf, mode: ImportMode, pack: Option<String>, theme: Option<String>, output: PathBuf, report: Option<PathBuf> }
pub(crate) fn import(platform: &Platform, out: &mut Out<'_>, fonts: &Fonts, args: ImportArgs) -> Result<()>;
// crate::library
pub(crate) fn read_pack_dir(storage: &Storage, dir: &Path) -> Option<PackFiles>;
pub(crate) fn beside_deck(storage: &Storage, deck_dir: &Path, r: &PackRef) -> Option<PackFiles>;
pub(crate) fn resolve_pack(storage: &Storage, library: &PackLibrary, l: &Localizer, wanted: &str) -> Result<PackFiles>;
pub(crate) fn embed_theme(themes: &ThemeLibrary, l: &Localizer, wanted: &str, bundle: &mut SourceBundle) -> Result<ThemeRef>;
pub(crate) fn theme_library(platform: &Platform) -> ThemeLibrary;
pub(crate) fn pack_library(platform: &Platform) -> PackLibrary;
// main.rs
struct Written { pages: usize, pdf: usize, sources: usize }
fn write_document(platform: &Platform, l: &Localizer, fonts: Fonts, widgets: &DeckWidgets, bundle: &SourceBundle, output: &Path) -> Result<Written>;
```

- [ ] **Step 1 : tests qui échouent**

`crates/oku-slides-cli/src/library_tests.rs` :

```rust
use oku_slides_core::PackManifest;
use oku_slides_i18n::english;
use oku_slides_session::{DEFAULT_PACK, GRID_PACK};
use speculoos::prelude::*;

use super::*;

/// Une bibliothèque de packs vide dans un dossier temporaire (les intégrés seulement).
fn library(dir: &Path) -> (Storage, PackLibrary) {
    let storage = Storage::local().unwrap();
    let library = PackLibrary::new(Some(dir.join("templates")), storage.clone());
    (storage, library)
}

fn pack_id(files: &PackFiles) -> uuid::Uuid {
    PackManifest::from_files(files).unwrap().id
}

/// Écrit dans `dir` le `pack.json` du pack intégré Grilles et un widget `citation`.
fn write_pack_dir(dir: &Path) {
    let json = include_str!("../../../templates/grilles/pack.json");
    std::fs::create_dir_all(dir.join("widgets")).unwrap();
    std::fs::write(dir.join("pack.json"), json).unwrap();
    std::fs::write(dir.join("widgets").join("citation.typ"), "// widget").unwrap();
}

/// Des sources vides, sur le registre de base figé.
fn empty_bundle() -> SourceBundle {
    let probe = serde_json::from_str(include_str!("../../../fixtures/widgets/base-registry.json")).unwrap();
    let registry = std::sync::Arc::new(oku_slides_core::WidgetRegistry::from_probe(probe).unwrap());
    SourceBundle::new(oku_slides_core::Deck::empty(), registry).unwrap()
}

#[test]
fn read_pack_dir_reads_the_manifest_and_the_widget_files() {
    let tmp = tempfile::tempdir().unwrap();
    write_pack_dir(tmp.path());
    let (storage, _) = library(tmp.path());
    let files = read_pack_dir(&storage, tmp.path()).unwrap();
    let names: Vec<String> = files.widgets().keys().map(|w| w.file_name()).collect();
    assert_that!((pack_id(&files), names)).is_equal_to((GRID_PACK, vec!["citation.typ".to_owned()]));
}

#[test]
fn read_pack_dir_without_pack_json_is_none() {
    let tmp = tempfile::tempdir().unwrap();
    let (storage, _) = library(tmp.path());
    assert_that!(read_pack_dir(&storage, tmp.path())).is_none();
}

#[test]
fn resolve_pack_by_name_ignores_case() {
    let tmp = tempfile::tempdir().unwrap();
    let (storage, library) = library(tmp.path());
    let files = resolve_pack(&storage, &library, english(), "essentiel").unwrap();
    assert_that!(pack_id(&files)).is_equal_to(DEFAULT_PACK);
}

#[test]
fn resolve_pack_by_id_takes_the_current_version() {
    let tmp = tempfile::tempdir().unwrap();
    let (storage, library) = library(tmp.path());
    let files = resolve_pack(&storage, &library, english(), &GRID_PACK.to_string()).unwrap();
    assert_that!(files.version()).is_equal_to(library.current_version(GRID_PACK).unwrap().version);
}

#[test]
fn resolve_pack_by_folder_reads_it() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("mon-pack");
    write_pack_dir(&dir);
    let (storage, library) = library(tmp.path());
    let files = resolve_pack(&storage, &library, english(), dir.to_str().unwrap()).unwrap();
    assert_that!(files.widgets().len()).is_equal_to(1);
}

#[test]
fn resolve_pack_unknown_fails_with_a_localized_message() {
    let tmp = tempfile::tempdir().unwrap();
    let (storage, library) = library(tmp.path());
    let err = resolve_pack(&storage, &library, english(), "Nope").unwrap_err();
    assert_that!(err.to_string()).is_equal_to("unknown pack: Nope".to_owned());
}

#[test]
fn resolve_pack_unknown_id_fails_too() {
    let tmp = tempfile::tempdir().unwrap();
    let (storage, library) = library(tmp.path());
    let id = "00000000-0000-4000-8000-000000000000";
    assert_that!(resolve_pack(&storage, &library, english(), id)).is_err();
}

#[test]
fn embed_theme_by_name_or_id_embeds_the_same_version() {
    let tmp = tempfile::tempdir().unwrap();
    let themes = ThemeLibrary::new(Some(tmp.path().join("themes")), Storage::local().unwrap());
    let mut bundle = empty_bundle();
    let by_name = embed_theme(&themes, english(), "PRO", &mut bundle).unwrap();
    let by_id = embed_theme(&themes, english(), &oku_slides_session::PRO_THEME.to_string(), &mut bundle).unwrap();
    assert_that!(by_name).is_equal_to(by_id.clone());
    assert_that!(by_id.id).is_equal_to(oku_slides_session::PRO_THEME);
}

#[test]
fn embed_theme_unknown_fails_with_a_localized_message() {
    let tmp = tempfile::tempdir().unwrap();
    let themes = ThemeLibrary::new(Some(tmp.path().join("themes")), Storage::local().unwrap());
    let mut bundle = empty_bundle();
    let err = embed_theme(&themes, english(), "Nope", &mut bundle).unwrap_err();
    assert_that!(err.to_string()).is_equal_to("Unknown theme: Nope".to_owned());
}
```

`crates/oku-slides-cli/tests/import.rs` :

```rust
//! `oku-slides import`, lancée pour de vrai : une présentation PowerPoint en
//! `.oku-slides.pdf` rouvrable. Chaque test a son dossier de données
//! (OKU_SLIDES_PATHS__DATA, par `oku_slides`) ; aucun ne lance LibreOffice.

mod common;

use std::collections::HashSet;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;

use common::{assert_success, oku_slides, workdir};
use oku_slides_core::{SourceBundle, SourceLimits, WidgetRegistry, validate};
use oku_slides_session::{DEFAULT_PACK, GRID_PACK, PRO_THEME};
use speculoos::prelude::*;
use zip::write::SimpleFileOptions;

const LIMITS: SourceLimits = SourceLimits { replica_bytes: 64 << 20, total_bytes: 512 << 20 };

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/import")
}

fn run(cmd: &mut Command) -> Output {
    cmd.output().unwrap_or_else(|e| panic!("oku-slides ne se lance pas : {e}"))
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let p = e.unwrap_or_else(|e| panic!("{e}")).path();
        if p.file_name().is_some_and(|n| n == ".DS_Store") {
            continue;
        }
        if p.is_dir() { files(&p, out) } else { out.push(p) }
    }
}

/// Le `.pptx` de la fixture décompressée `name`, zippé dans `dir` ; son chemin.
fn pptx(dir: &Path, name: &str) -> PathBuf {
    let root = fixtures().join(name);
    let mut paths = Vec::new();
    files(&root, &mut paths);
    paths.sort_by_key(|p| (p.file_name() != Some("[Content_Types].xml".as_ref()), p.clone()));
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for p in paths {
        let rel = p.strip_prefix(&root).unwrap_or_else(|e| panic!("{e}")).to_string_lossy().replace('\\', "/");
        w.start_file(rel, SimpleFileOptions::default()).unwrap_or_else(|e| panic!("zip : {e}"));
        w.write_all(&std::fs::read(&p).unwrap_or_else(|e| panic!("{e}"))).unwrap_or_else(|e| panic!("zip : {e}"));
    }
    let bytes = w.finish().unwrap_or_else(|e| panic!("zip : {e}")).into_inner();
    let path = dir.join(format!("{name}.pptx"));
    std::fs::write(&path, bytes).unwrap_or_else(|e| panic!("{e}"));
    path
}

fn base_registry() -> Arc<WidgetRegistry> {
    let probe = serde_json::from_str(include_str!("../../../fixtures/widgets/base-registry.json"))
        .unwrap_or_else(|e| panic!("registre de base : {e}"));
    Arc::new(WidgetRegistry::from_probe(probe).unwrap_or_else(|e| panic!("registre de base : {e}")))
}

/// Les sources d'un `.oku-slides.pdf`, relues ; échoue si le deck lève une `Issue`.
fn reopened(pdf: &Path) -> SourceBundle {
    let bytes = std::fs::read(pdf).unwrap_or_else(|e| panic!("lecture du PDF : {e}"));
    let found = oku_slides_pdf::extract_sources(&bytes)
        .unwrap_or_else(|e| panic!("sources du PDF : {e}"))
        .unwrap_or_else(|| panic!("PDF sans sources"));
    let (bundle, issues) =
        SourceBundle::from_zip(&found.zip, LIMITS, base_registry()).unwrap_or_else(|e| panic!("sources : {e}"));
    assert_that!(issues).is_empty();
    let assets: HashSet<String> = bundle.assets.keys().cloned().collect();
    assert_that!(validate(bundle.deck(), &base_registry(), Some(&assets))).is_empty();
    bundle
}

#[test]
fn import_minimal_on_essentiel_and_pro_writes_a_reopenable_deck() {
    let dir = workdir("import-minimal");
    let (input, pdf) = (pptx(&dir, "minimal"), dir.join("minimal.oku-slides.pdf"));
    let out = run(oku_slides(&dir, "en")
        .arg("import")
        .arg(&input)
        .args(["--pack", "Essentiel", "--theme", "Pro", "-o"])
        .arg(&pdf));
    assert_success("import", &out, &[]);
    // Une perte : le second niveau de puce de la slide 2, aplati.
    assert_that!(stdout(&out))
        .is_equal_to(format!("{}: 3 slides, 3 pages, 1 loss, 0 widget requests\n", pdf.display()));
    let bundle = reopened(&pdf);
    let deck = bundle.deck();
    assert_that!((deck.slides.len(), deck.params.title.as_deref())).is_equal_to((3, Some("Rapport annuel")));
    assert_that!(deck.templates.as_ref().map(|r| r.id)).is_equal_to(Some(DEFAULT_PACK));
    assert_that!(deck.theme.as_ref().map(|r| r.id)).is_equal_to(Some(PRO_THEME));
    assert_that!(deck.slides[1].notes.starts_with("Dire bonjour.")).is_true();
}

#[test]
fn import_kitchen_on_grilles_writes_its_report_with_coverage_and_messages() {
    let dir = workdir("import-kitchen");
    let (input, pdf, report) = (pptx(&dir, "kitchen"), dir.join("k.oku-slides.pdf"), dir.join("k.json"));
    let out = run(oku_slides(&dir, "fr")
        .arg("import")
        .arg(&input)
        .args(["--mode", "retrofit", "--pack", "grilles", "-o"])
        .arg(&pdf)
        .arg("--report")
        .arg(&report));
    assert_success("import", &out, &[]);
    let bundle = reopened(&pdf);
    assert_that!(bundle.deck().templates.as_ref().map(|r| r.id)).is_equal_to(Some(GRID_PACK));
    assert_that!(bundle.assets.len()).is_equal_to(1);
    let r: serde_json::Value = serde_json::from_slice(&std::fs::read(&report).unwrap_or_else(|e| panic!("{e}")))
        .unwrap_or_else(|e| panic!("rapport : {e}"));
    assert_that!(r["mode"]).is_equal_to(serde_json::json!("retrofit"));
    assert_that!(r["coverage"]["runs"]["found"]).is_equal_to(r["coverage"]["runs"]["total"].clone());
    assert_that!(r["coverage"]["missing"]).is_equal_to(serde_json::json!([]));
    assert_that!(r["warnings"][0]["message"])
        .is_equal_to(serde_json::json!("Graphique non pris en charge : combo: barChart"));
    assert_that!(r["widget_requests"].as_array().map(Vec::len)).is_equal_to(Some(1));
}

#[test]
fn import_alto_with_pack_and_theme_ids_keeps_its_images() {
    let dir = workdir("import-alto");
    let pdf = dir.join("alto.oku-slides.pdf");
    let out = run(oku_slides(&dir, "en")
        .arg("import")
        .arg(fixtures().join("alto-wealth.pptx"))
        .args(["--pack", &DEFAULT_PACK.to_string(), "--theme", &PRO_THEME.to_string(), "-o"])
        .arg(&pdf));
    assert_success("import", &out, &[]);
    let bundle = reopened(&pdf);
    assert_that!(bundle.deck().slides.len()).is_equal_to(8);
    assert_that!(bundle.assets.is_empty()).is_false();
}

#[test]
fn import_with_a_pack_folder_uses_that_pack() {
    let dir = workdir("import-folder");
    let pack = dir.join("mon-pack");
    std::fs::create_dir_all(&pack).unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(pack.join("pack.json"), include_str!("../../../templates/grilles/pack.json"))
        .unwrap_or_else(|e| panic!("{e}"));
    let (input, pdf) = (pptx(&dir, "minimal"), dir.join("m.oku-slides.pdf"));
    let out = run(oku_slides(&dir, "en").arg("import").arg(&input).arg("--pack").arg(&pack).arg("-o").arg(&pdf));
    assert_success("import", &out, &[]);
    assert_that!(reopened(&pdf).deck().templates.as_ref().map(|r| r.id)).is_equal_to(Some(GRID_PACK));
}

#[test]
fn import_hifi_is_refused_with_a_localized_message() {
    let dir = workdir("import-hifi");
    let out = run(oku_slides(&dir, "en")
        .arg("import")
        .arg(fixtures().join("alto-wealth.pptx"))
        .args(["--mode", "hifi", "--pack", "Essentiel", "-o"])
        .arg(dir.join("x.oku-slides.pdf")));
    assert_that!(out.status.success()).is_false();
    assert_that!(stderr(&out)).contains("high-fidelity import (--mode hifi) is not available yet");
}

#[test]
fn import_without_pack_is_refused_in_retrofit_mode() {
    let dir = workdir("import-nopack");
    let out = run(oku_slides(&dir, "fr")
        .arg("import")
        .arg(fixtures().join("alto-wealth.pptx"))
        .arg("-o")
        .arg(dir.join("x.pdf")));
    assert_that!(out.status.success()).is_false();
    assert_that!(stderr(&out)).contains("--pack est obligatoire en mode retrofit");
}

#[test]
fn import_with_an_unknown_pack_fails_with_its_name() {
    let dir = workdir("import-unknown");
    let out = run(oku_slides(&dir, "en")
        .arg("import")
        .arg(fixtures().join("alto-wealth.pptx"))
        .args(["--pack", "Nope", "-o"])
        .arg(dir.join("x.pdf")));
    assert_that!(out.status.success()).is_false();
    assert_that!(stderr(&out)).contains("unknown pack: Nope");
}

#[test]
fn import_with_a_pack_folder_whose_manifest_is_not_a_pack_fails_with_its_name() {
    let dir = workdir("import-badpack");
    let pack = dir.join("faux-pack");
    std::fs::create_dir_all(&pack).unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(pack.join("pack.json"), r#"{"templates": "aucun"}"#).unwrap_or_else(|e| panic!("{e}"));
    let out = run(oku_slides(&dir, "fr")
        .arg("import")
        .arg(fixtures().join("alto-wealth.pptx"))
        .arg("--pack")
        .arg(&pack)
        .arg("-o")
        .arg(dir.join("x.pdf")));
    assert_that!(out.status.success()).is_false();
    assert_that!(stderr(&out)).contains("pack illisible");
}

/// Un fichier creux de 257 Mio (au-delà des 256 Mio de `ReadLimits::default()`) :
/// refusé sur sa taille, sans être lu.
#[test]
fn import_of_a_file_beyond_the_input_limit_is_refused_before_reading() {
    let dir = workdir("import-toolarge");
    let input = dir.join("huge.pptx");
    let file = std::fs::File::create(&input).unwrap_or_else(|e| panic!("{e}"));
    file.set_len(257 << 20).unwrap_or_else(|e| panic!("{e}"));
    let out = run(oku_slides(&dir, "en")
        .arg("import")
        .arg(&input)
        .args(["--pack", "Essentiel", "-o"])
        .arg(dir.join("x.pdf")));
    assert_that!(out.status.success()).is_false();
    assert_that!(stderr(&out)).contains("This presentation is too large to import");
}

#[test]
fn import_of_a_file_that_is_not_a_zip_fails_with_the_reader_message() {
    let dir = workdir("import-notpptx");
    let input = dir.join("deck.pptx");
    std::fs::write(&input, b"not a zip").unwrap_or_else(|e| panic!("{e}"));
    let out = run(oku_slides(&dir, "en")
        .arg("import")
        .arg(&input)
        .args(["--pack", "Essentiel", "-o"])
        .arg(dir.join("x.pdf")));
    assert_that!(out.status.success()).is_false();
    assert_that!(stderr(&out)).contains("This presentation could not be read");
}

#[test]
fn import_help_is_in_the_configured_language() {
    let dir = workdir("import-help");
    let out = run(oku_slides(&dir, "fr").args(["import", "--help"]));
    assert_that!(stdout(&out)).contains("Importe une présentation PowerPoint (.pptx)");
}
```

> `minimal` : une perte (`ListLevels` : la puce de niveau 1 de la slide 2), au singulier (« 1 loss ») ; aucune demande de widget (pas de groupe). `kitchen` sur Grilles : le PNG de la slide 2 et le repli de l'équation sont le même asset (1) ; l'avertissement du lecteur `UnsupportedChart("combo: barChart")` est le premier et seul, localisé en français ; seul le SmartArt fait une demande (le groupe de deux boîtes ne perd rien). Le fichier de 257 Mio est creux (`set_len`) : il ne coûte rien sur le disque, et la CLI le refuse sur sa seule taille. Les tests ne zippent qu'en mémoire et ne lancent ni LibreOffice ni l'app ; le rendu passe par Typst en mémoire (packs intégrés sans widgets).

À la fin de `crates/oku-slides-import/src/plan_tests.rs` :

```rust
#[test]
fn import_plan_report_json_adds_the_widget_requests_to_the_report() {
    let plan = ImportPlan {
        ops: Vec::new(),
        assets: BTreeMap::new(),
        widget_requests: vec![WidgetRequest {
            signature: "text×1|0044".into(),
            slides: vec!["s2".into()],
            sample: vec![item()],
            fallback: "stack".into(),
        }],
        report: ImportReport::new(Mode::Retrofit),
    };
    let v = plan.report_json(oku_slides_i18n::english());
    assert_that!(v["mode"]).is_equal_to(json!("retrofit"));
    assert_that!(v["widget_requests"][0]["slides"]).is_equal_to(json!(["s2"]));
    assert_that!(v["widget_requests"][0]["fallback"]).is_equal_to(json!("stack"));
}
```

- [ ] **Step 2 : lancer, constater l'échec**

```bash
cargo nextest run -p oku-slides-cli -p oku-slides-import plan
```

Attendu : échec de compilation (`library`, `import`, `report_json` absents ; `zip` inconnu des tests de la CLI).

- [ ] **Step 3 : implémentation**

Dépendances de `crates/oku-slides-cli/Cargo.toml` (sections `[dependencies]` et `[dev-dependencies]`) :

```toml
[dependencies]
oku-slides-core.workspace = true
oku-slides-render.workspace = true
oku-slides-pdf.workspace = true
oku-slides-session.workspace = true
oku-slides-render-worker.workspace = true
oku-slides-stt.workspace = true
oku-slides-agent.workspace = true
axum.workspace = true
http-body.workspace = true
ctrlc.workspace = true
color-eyre.workspace = true
tracing.workspace = true
oku-slides-platform = { workspace = true, features = ["listen"] }
oku-slides-i18n.workspace = true
oku-slides-import.workspace = true
clap.workspace = true
serde.workspace = true
serde_json.workspace = true
tokio = { workspace = true, features = ["rt-multi-thread", "net", "sync", "macros", "time"] }
uuid.workspace = true

[dev-dependencies]
speculoos.workspace = true
serde_json.workspace = true
tempfile.workspace = true
zip.workspace = true
```

Dans `crates/oku-slides-import/src/plan.rs`, importer `oku_slides_i18n::Localizer` et `serde_json::Value`, puis ajouter avant le module de tests :

```rust
impl ImportPlan {
    /// Le rapport écrit par `--report` : [`ImportReport::to_json`] (messages
    /// localisés), plus les demandes de widget.
    pub fn report_json(&self, l: &Localizer) -> Value {
        let mut v = self.report.to_json(l);
        if let Some(o) = v.as_object_mut() {
            // Une demande se sérialise toujours : clés textuelles, nombres finis.
            o.insert("widget_requests".to_owned(), serde_json::to_value(&self.widget_requests).unwrap_or_default());
        }
        v
    }
}
```

`crates/oku-slides-cli/src/library.rs` :

```rust
//! Packs et thèmes pour la CLI : un pack lu dans un dossier, un pack ou un
//! thème désigné par son id, son dossier ou son nom, embarqué dans des sources.

use std::collections::BTreeMap;
use std::path::Path;

use color_eyre::eyre::{Result, eyre};

use oku_slides_core::{PackFiles, PackRef, SourceBundle, ThemeRef, WidgetFile};
use oku_slides_i18n::Localizer;
use oku_slides_platform::Platform;
use oku_slides_platform::errors::one_line;
use oku_slides_session::{PackLibrary, Storage, ThemeLibrary, check_pack_id};

/// Le contenu d'un fichier existant, `None` s'il manque ou ne se lit pas (journalisé).
fn read_existing(storage: &Storage, path: &Path) -> Option<Vec<u8>> {
    match storage.len(path) {
        Ok(Some(_)) => storage
            .read(path)
            .inspect_err(|e| tracing::warn!(path = %path.display(), error = %e, "fichier du pack illisible"))
            .ok(),
        _ => None,
    }
}

/// Les fichiers du pack rangé dans `dir` (`pack.json`, `widgets/<nom>.typ`),
/// la disposition d'une version de pack ; `None` sans `pack.json` lisible.
pub(crate) fn read_pack_dir(storage: &Storage, dir: &Path) -> Option<PackFiles> {
    let mut widgets = BTreeMap::new();
    let listed = storage.list_files(&dir.join("widgets")).unwrap_or_default();
    for path in listed {
        let Some(name) = path.file_name().and_then(|n| n.to_str()).and_then(WidgetFile::from_file_name) else {
            continue;
        };
        widgets.insert(name, read_existing(storage, &path)?);
    }
    PackFiles::from_bytes(
        &read_existing(storage, &dir.join("pack.json"))?,
        widgets.iter().map(|(n, b)| (n.clone(), b.as_slice())),
    )
}

/// La version `r` extraite à côté du deck, si elle s'y trouve intacte et
/// que son `pack.json` est bien celui du pack `r.id`.
pub(crate) fn beside_deck(storage: &Storage, deck_dir: &Path, r: &PackRef) -> Option<PackFiles> {
    let dir = deck_dir.join("packs").join(r.version.hex());
    let files = read_pack_dir(storage, &dir)?;
    if !files.matches(&r.version) {
        tracing::warn!(dir = %dir.display(), "pack altéré à côté du deck : ignoré");
        return None;
    }
    match check_pack_id(r, &files) {
        Ok(()) => Some(files),
        Err(e) => {
            tracing::warn!(dir = %dir.display(), error = %one_line(&e), "pack à côté du deck : ignoré");
            None
        }
    }
}

/// Le pack désigné par `wanted` : un id (version courante de la bibliothèque),
/// un dossier de pack (`pack.json`, `widgets/`), sinon un nom de pack (intégré
/// ou de la bibliothèque, sans casse). Introuvable : erreur localisée.
pub(crate) fn resolve_pack(storage: &Storage, library: &PackLibrary, l: &Localizer, wanted: &str) -> Result<PackFiles> {
    let unknown = || eyre!(l.tr_with("cli-import-unknown-pack", &[("pack", wanted.into())]));
    if let Ok(id) = uuid::Uuid::parse_str(wanted) {
        return library.current_version(id).map(|s| s.files).ok_or_else(unknown);
    }
    let dir = Path::new(wanted);
    if matches!(storage.len(&dir.join("pack.json")), Ok(Some(_))) {
        return read_pack_dir(storage, dir).ok_or_else(unknown);
    }
    let info = library.packs().into_iter().find(|p| p.name.eq_ignore_ascii_case(wanted)).ok_or_else(unknown)?;
    library.current_version(info.id).map(|s| s.files).ok_or_else(unknown)
}

/// Embarque dans `bundle` la version courante du thème désigné par `wanted` :
/// son id, sinon son nom (intégré ou de la bibliothèque, sans casse) ; sa
/// référence. Inconnu : erreur localisée.
pub(crate) fn embed_theme(
    themes: &ThemeLibrary,
    l: &Localizer,
    wanted: &str,
    bundle: &mut SourceBundle,
) -> Result<ThemeRef> {
    let unknown = || eyre!(l.tr_with("session-unknown-theme", &[("theme", wanted.into())]));
    let id = match uuid::Uuid::parse_str(wanted) {
        Ok(id) => id,
        Err(_) => themes.themes().into_iter().find(|t| t.name.eq_ignore_ascii_case(wanted)).ok_or_else(unknown)?.id,
    };
    let current = themes.current_version(id).ok_or_else(unknown)?;
    Ok(ThemeRef { id, version: bundle.add_theme(current.files) })
}

/// La bibliothèque des thèmes de l'utilisateur (et les intégrés).
pub(crate) fn theme_library(platform: &Platform) -> ThemeLibrary {
    ThemeLibrary::new(Some(platform.settings.paths.themes()), platform.storage.clone())
}

/// La bibliothèque des packs de l'utilisateur (et les intégrés).
pub(crate) fn pack_library(platform: &Platform) -> PackLibrary {
    PackLibrary::new(Some(platform.settings.paths.templates()), platform.storage.clone())
}

#[cfg(test)]
#[path = "library_tests.rs"]
mod library_tests;
```

`crates/oku-slides-cli/src/import.rs` :

```rust
//! `oku-slides import` : une présentation PowerPoint en deck Oku.
//!
//! Mode `retrofit` (lot 2) : le contenu de la présentation est projeté sur
//! un pack et un thème existants (`oku_slides_import::retrofit`). Le pack est
//! obligatoire (id, dossier ou nom) ; le thème est facultatif (id ou nom ;
//! défaut : le thème par défaut). Les ops du plan s'appliquent en une
//! transaction, puis le deck est rendu et enregistré en `.oku-slides.pdf`,
//! comme `save`. Le mode `hifi` est reconnu et refusé : il arrive au lot 3.

use std::collections::BTreeMap;
use std::path::PathBuf;

use color_eyre::eyre::{Result, WrapErr, eyre};
use serde_json::{Map, Value};

use oku_slides_core::tokens::effective_tokens;
use oku_slides_core::{Deck, PackManifest, PackRef, SourceBundle, apply_all};
use oku_slides_i18n::Localize;
use oku_slides_import::retrofit::{self, ColorTokens, PageTokens};
use oku_slides_import::{ReadLimits, RetrofitTarget, read_pptx};
use oku_slides_platform::Platform;
use oku_slides_platform::storage::StorageErrorKind;
use oku_slides_render::Fonts;
use oku_slides_session::theme_schema::default_tokens;
use oku_slides_session::{ThemeLibrary, adopt_default_theme, theme_tokens};

use crate::library::{embed_theme, pack_library, resolve_pack, theme_library};
use crate::out::Out;

/// Mode d'import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum ImportMode {
    /// Contenu projeté sur un pack et un thème existants.
    Retrofit,
    /// Copie fidèle (lot 3) : refusée pour l'instant.
    Hifi,
}

/// Les arguments de `import`.
#[derive(Debug)]
pub(crate) struct ImportArgs {
    /// Le `.pptx` à importer.
    pub(crate) input: PathBuf,
    /// Le mode.
    pub(crate) mode: ImportMode,
    /// Le pack cible : id, dossier ou nom.
    pub(crate) pack: Option<String>,
    /// Le thème cible : id ou nom.
    pub(crate) theme: Option<String>,
    /// Le `.oku-slides.pdf` à écrire.
    pub(crate) output: PathBuf,
    /// Le rapport JSON à écrire.
    pub(crate) report: Option<PathBuf>,
}

/// Les jetons effectifs du thème de `bundle` : les défauts, puis ceux du thème.
fn deck_tokens(themes: &ThemeLibrary, bundle: &SourceBundle) -> Map<String, Value> {
    let own = theme_tokens(themes, &bundle.themes, bundle.deck()).unwrap_or_default();
    effective_tokens(default_tokens(), &own)
}

/// Importe `args.input` et écrit le deck obtenu dans `args.output`.
pub(crate) fn import(platform: &Platform, out: &mut Out<'_>, fonts: &Fonts, args: ImportArgs) -> Result<()> {
    let (storage, l) = (&platform.storage, out.localizer());
    if args.mode == ImportMode::Hifi {
        return Err(eyre!(l.tr("cli-import-hifi-later")));
    }
    let wanted = args.pack.as_deref().ok_or_else(|| eyre!(l.tr("cli-import-pack-required")))?;
    let library = pack_library(platform);
    let files = resolve_pack(storage, &library, l, wanted)?;
    let manifest = PackManifest::from_files(&files)
        .map_err(|_| eyre!(l.tr_with("cli-import-bad-pack", &[("pack", wanted.into())])))?;
    let shown = args.input.display().to_string();
    let limits = ReadLimits::default();
    // Taille vue avant toute lecture : un fichier trop gros n'entre jamais en mémoire.
    let bytes = storage.read_limited(&args.input, limits.max_input_bytes).map_err(|e| {
        if e.kind() == StorageErrorKind::TooLarge {
            eyre!(l.tr("import-too-large"))
        } else {
            eyre!(e).wrap_err(l.tr_with("cli-reading", &[("path", shown.as_str().into())]))
        }
    })?;
    let source = read_pptx(&bytes, &limits).map_err(|e| eyre!(e.localize(l)))?;

    let version = files.version();
    let mut deck = Deck::empty();
    deck.templates = Some(PackRef { id: manifest.id, version: version.clone() });
    let base = crate::registry(fonts)?;
    let packs = BTreeMap::from([(version, files)]);
    let (mut bundle, widgets) = crate::sources_with_pack(platform, l, &library, &base, deck, packs)?;
    let themes = theme_library(platform);
    if let Some(wanted) = &args.theme {
        let mut deck = bundle.deck().clone();
        deck.theme = Some(embed_theme(&themes, l, wanted, &mut bundle)?);
        bundle.apply_silent(deck)?;
    }
    adopt_default_theme(&themes, &mut bundle).map_err(|e| eyre!(e.localize(l)))?;

    let tokens = deck_tokens(&themes, &bundle);
    let target = RetrofitTarget {
        templates: &manifest.templates,
        registry: &widgets.registry,
        colors: ColorTokens::from_tokens(&tokens),
        page: PageTokens::from_tokens(&tokens),
    };
    let plan = retrofit::project(&source, &target);
    for asset in plan.assets.values() {
        bundle.add_asset(asset.bytes.clone());
    }
    let mut deck = bundle.deck().clone();
    apply_all(&mut deck, plan.ops.clone(), &widgets.registry).map_err(|e| eyre!(e.localize(l)))?;
    bundle.apply(deck, &l.tr("cli-import-label"), None).map_err(|e| eyre!(e.localize(l)))?;
    crate::report_issues(l, bundle.deck(), &widgets.registry, &bundle.assets);
    let target = args.output.display().to_string();
    let written = crate::write_document(platform, l, fonts.clone(), &widgets, &bundle, &args.output)
        .wrap_err_with(|| l.tr_with("cli-import-write-failed", &[("path", target.as_str().into())]))?;
    if let Some(path) = &args.report {
        storage.write(path, serde_json::to_string_pretty(&plan.report_json(l))?)?;
    }
    out.line(
        "cli-imported",
        &[
            ("path", args.output.display().to_string().into()),
            ("slides", bundle.deck().slides.len().into()),
            ("pages", written.pages.into()),
            ("losses", plan.report.loss_count().into()),
            ("requests", plan.widget_requests.len().into()),
        ],
    )?;
    Ok(())
}
```

`crates/oku-slides-cli/src/main.rs` complet (modules `import` et `library`, variante `Cmd::Import` et son aide, `beside_deck` / `embed_theme` / `theme_library` déplacés, `write_document` partagé par `save` et `import`) :

```rust
//! `oku-slides` : outil en ligne de commande pour rendre, enregistrer, rouvrir
//! et importer des decks Oku Slides sans l'app. Sert aussi aux tests et aux mesures.
//!
//! Les résultats vont sur stdout ([`out`]), les journaux sur stderr ; l'aide
//! et les messages sont dans la langue des réglages.

mod bench;
mod guard;
mod import;
mod library;
mod out;
mod pack_render;
mod serve;
mod sse;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use color_eyre::eyre::{Result, WrapErr};

use oku_slides_core::{Deck, PackFiles, PackVersion, SourceBundle, SourceLimits, WidgetRegistry, validate};
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_platform::Platform;
use oku_slides_render::Fonts;
use oku_slides_session::{PackLibrary, Storage, adopt_default_pack, adopt_default_theme, read_document, theme_tokens};

use crate::bench::ms;
use crate::import::{ImportArgs, ImportMode};
use crate::library::{beside_deck, embed_theme, pack_library, theme_library};
use crate::out::Out;
use crate::pack_render::DeckWidgets;

// L'aide de chaque commande et option vient des messages Fluent
// (`cli-cmd-…`, `cli-arg-…`) : voir `command`.
#[derive(Parser)]
#[command(name = "oku-slides", version)]
struct Cli {
    #[arg(long, global = true)]
    fonts: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Check {
        deck: PathBuf,
    },
    Render {
        input: PathBuf,
        #[arg(long, default_value = "out")]
        out: PathBuf,
        #[arg(long)]
        theme: Option<String>,
    },
    Save {
        deck: PathBuf,
        output: PathBuf,
        #[arg(long)]
        assets: Option<PathBuf>,
        #[arg(long)]
        embed_fonts: bool,
        #[arg(long)]
        theme: Option<String>,
    },
    Open {
        input: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Strip {
        input: PathBuf,
        output: PathBuf,
    },
    Import {
        input: PathBuf,
        #[arg(long, value_enum, default_value_t = ImportMode::Retrofit)]
        mode: ImportMode,
        #[arg(long)]
        pack: Option<String>,
        #[arg(long)]
        theme: Option<String>,
        #[arg(short = 'o', long)]
        output: PathBuf,
        #[arg(long)]
        report: Option<PathBuf>,
    },
    Bench {
        deck: PathBuf,
        #[arg(long, default_value_t = 5)]
        iterations: usize,
    },
    Schema,
    Serve {
        input: Option<PathBuf>,
        #[arg(long)]
        port: Option<u16>,
        #[arg(long, default_value = "ui/dist")]
        ui: PathBuf,
    },
}

/// La ligne de commande, avec l'aide dans la langue de l'utilisateur.
fn command(l: &Localizer) -> clap::Command {
    let help = |id: &str| l.tr(id);
    Cli::command()
        .about(help("cli-about"))
        .mut_arg("fonts", |a| a.help(help("cli-arg-fonts")))
        .mut_subcommand("check", |c| c.about(help("cli-cmd-check")).mut_arg("deck", |a| a.help(help("cli-arg-deck"))))
        .mut_subcommand("render", |c| {
            c.about(help("cli-cmd-render"))
                .mut_arg("input", |a| a.help(help("cli-arg-input")))
                .mut_arg("out", |a| a.help(help("cli-arg-out")))
                .mut_arg("theme", |a| a.help(help("cli-arg-theme")))
        })
        .mut_subcommand("save", |c| {
            c.about(help("cli-cmd-save"))
                .mut_arg("deck", |a| a.help(help("cli-arg-deck")))
                .mut_arg("output", |a| a.help(help("cli-arg-output")))
                .mut_arg("assets", |a| a.help(help("cli-arg-assets")))
                .mut_arg("embed_fonts", |a| a.help(help("cli-arg-embed-fonts")))
                .mut_arg("theme", |a| a.help(help("cli-arg-theme")))
        })
        .mut_subcommand("open", |c| {
            c.about(help("cli-cmd-open"))
                .mut_arg("input", |a| a.help(help("cli-arg-input")))
                .mut_arg("out", |a| a.help(help("cli-arg-extract")))
        })
        .mut_subcommand("strip", |c| {
            c.about(help("cli-cmd-strip"))
                .mut_arg("input", |a| a.help(help("cli-arg-input")))
                .mut_arg("output", |a| a.help(help("cli-arg-output")))
        })
        .mut_subcommand("import", |c| {
            c.about(help("cli-cmd-import"))
                .mut_arg("input", |a| a.help(help("cli-arg-pptx")))
                .mut_arg("mode", |a| a.help(help("cli-arg-mode")))
                .mut_arg("pack", |a| a.help(help("cli-arg-pack")))
                .mut_arg("theme", |a| a.help(help("cli-arg-import-theme")))
                .mut_arg("output", |a| a.help(help("cli-arg-output")))
                .mut_arg("report", |a| a.help(help("cli-arg-report")))
        })
        .mut_subcommand("bench", |c| {
            c.about(help("cli-cmd-bench"))
                .mut_arg("deck", |a| a.help(help("cli-arg-deck")))
                .mut_arg("iterations", |a| a.help(help("cli-arg-iterations")))
        })
        .mut_subcommand("schema", |c| c.about(help("cli-cmd-schema")))
        .mut_subcommand("serve", |c| {
            c.about(help("cli-cmd-serve"))
                .mut_arg("input", |a| a.help(help("cli-arg-serve-input")))
                .mut_arg("port", |a| a.help(help("cli-arg-port")))
                .mut_arg("ui", |a| a.help(help("cli-arg-ui")))
        })
}

fn read_deck(storage: &Storage, l: &Localizer, path: &Path) -> Result<Deck> {
    let shown = || path.display().to_string();
    let bytes = storage.read(path).wrap_err_with(|| l.tr_with("cli-reading", &[("path", shown().into())]))?;
    serde_json::from_slice(&bytes).wrap_err_with(|| l.tr_with("cli-invalid-deck", &[("path", shown().into())]))
}

/// Charge un deck depuis un JSON ou un .oku-slides.pdf ; `limits` borne les
/// sources lues dans le PDF. Rend aussi les widgets du pack qu'il embarque :
/// le deck est relu et validé avec eux, pas avec la seule base `base`.
fn load_bundle(
    platform: &Platform,
    l: &Localizer,
    path: &Path,
    limits: SourceLimits,
    base: &Arc<WidgetRegistry>,
) -> Result<(SourceBundle, DeckWidgets)> {
    let read = |registry| {
        read_document(&platform.storage, path, limits, registry)
            .wrap_err_with(|| l.tr_with("cli-reading", &[("path", path.display().to_string().into())]))
    };
    let mut opened = read(base)?;
    let widgets = DeckWidgets::probe(platform, l, base, opened.sources.deck(), &opened.sources.packs);
    if !Arc::ptr_eq(&widgets.registry, base) {
        // Relu avec les widgets du pack : sa validation ne les dit plus inconnus.
        opened = read(&widgets.registry)?;
    }
    if opened.modified_elsewhere {
        tracing::warn!(path = %path.display(), "{}", l.tr("cli-pdf-modified"));
    }
    for issue in &opened.issues {
        tracing::warn!("{}", l.tr_with("cli-deck-issue", &[("issue", issue.localize(l).into())]));
    }
    Ok((opened.sources, widgets))
}

/// Signale les problèmes du deck ; vrai s'il n'y en a pas.
fn report_issues(l: &Localizer, deck: &Deck, registry: &WidgetRegistry, assets: &BTreeMap<String, Vec<u8>>) -> bool {
    let ids = assets.keys().cloned().collect();
    let issues = validate(deck, registry, Some(&ids));
    for issue in &issues {
        tracing::warn!("{}", l.tr_with("cli-deck-issue", &[("issue", issue.localize(l).into())]));
    }
    issues.is_empty()
}

/// La version du pack que `deck` référence, par empreinte ; vide sans référence.
///
/// Elle vient d'abord de `<deck_dir>/packs/<hex>` (extrait par `open --out`,
/// vérifié), puis de la bibliothèque ; introuvable : erreur.
fn deck_pack(
    storage: &Storage,
    library: &PackLibrary,
    l: &Localizer,
    deck_dir: &Path,
    deck: &Deck,
) -> Result<BTreeMap<PackVersion, PackFiles>> {
    let Some(r) = &deck.templates else { return Ok(BTreeMap::new()) };
    let files = beside_deck(storage, deck_dir, r).or_else(|| library.version(r.id, &r.version));
    let Some(files) = files else {
        let args = [("id", r.id.to_string().into()), ("version", r.version.as_str().into())];
        return Err(color_eyre::eyre::eyre!(l.tr_with("session-pack-version-missing", &args)));
    };
    Ok(BTreeMap::from([(r.version.clone(), files)]))
}

/// Les sources de `deck` avec son pack (`packs`, trouvé par [`deck_pack`]),
/// et les widgets sondés de ce pack.
///
/// Sans pack référencé, le deck adopte la version courante du pack par défaut,
/// modifiable : elle peut avoir des widgets. Le pack est sondé avant toute
/// écriture ; les sources sont recréées avec le registre sondé, pour que le
/// replica et la validation aient ses widgets.
fn sources_with_pack(
    platform: &Platform,
    l: &Localizer,
    library: &PackLibrary,
    base: &Arc<WidgetRegistry>,
    deck: Deck,
    packs: BTreeMap<PackVersion, PackFiles>,
) -> Result<(SourceBundle, DeckWidgets)> {
    let mut bundle = SourceBundle::new(deck, base.clone())?;
    for files in packs.into_values() {
        bundle.add_pack(files);
    }
    adopt_default_pack(library, &mut bundle).map_err(|e| color_eyre::eyre::eyre!(e.localize(l)))?;
    let widgets = DeckWidgets::probe(platform, l, base, bundle.deck(), &bundle.packs);
    if !Arc::ptr_eq(&widgets.registry, base) {
        let packs = std::mem::take(&mut bundle.packs);
        bundle = SourceBundle::new(bundle.deck().clone(), widgets.registry.clone())?;
        bundle.packs = packs;
    }
    Ok((bundle, widgets))
}

/// Les tailles d'un document écrit : pages, PDF et sources en octets.
struct Written {
    pages: usize,
    pdf: usize,
    sources: usize,
}

/// Rend `bundle` avec `widgets`, joint ses sources au PDF et écrit le tout
/// dans `output` (écriture atomique).
fn write_document(
    platform: &Platform,
    l: &Localizer,
    fonts: Fonts,
    widgets: &DeckWidgets,
    bundle: &SourceBundle,
    output: &Path,
) -> Result<Written> {
    let doc = pack_render::render(platform, l, fonts, widgets, bundle, bundle.deck())?;
    let pdf = doc.pdf()?;
    let zip = bundle.to_zip()?;
    let written = oku_slides_pdf::embed_sources(&pdf, &zip)?;
    let pdf = written.len();
    platform.storage.write_atomic(output, written)?;
    Ok(Written { pages: doc.page_count(), pdf, sources: zip.len() })
}

/// Registre de base des widgets, compilé depuis Typst une fois par processus
/// et partagé avec le rendu en mémoire ; le pack d'un deck passe par
/// [`pack_render`].
fn registry(fonts: &Fonts) -> Result<Arc<WidgetRegistry>> {
    Ok(oku_slides_render::Registries::new(fonts.clone()).get(None)?)
}

fn main() -> Result<()> {
    // Relancé en processus de rendu (`--render-worker`) : sert le tube, puis termine.
    oku_slides_render_worker::exit_if_worker();
    color_eyre::install()?;
    oku_slides_platform::telemetry::init();
    let platform = Platform::load()?;
    let l: &Localizer = &platform.localizer;
    let cli = Cli::from_arg_matches(&command(l).get_matches())?;
    let mut out = Out::new(l);
    match run(cli, &platform, &mut out) {
        // Un tube fermé (`oku-slides schema | head`) n'est pas une erreur.
        Err(e) if e.downcast_ref::<std::io::Error>().is_some_and(|io| io.kind() == std::io::ErrorKind::BrokenPipe) => {
            Ok(())
        }
        result => result,
    }
}

fn run(cli: Cli, platform: &Platform, out: &mut Out<'_>) -> Result<()> {
    let (storage, l) = (&platform.storage, out.localizer());
    let fonts = match cli.fonts.as_ref().or(platform.settings.paths.fonts.as_ref()) {
        Some(dir) => storage
            .read_fonts(dir)
            .wrap_err_with(|| l.tr_with("cli-reading", &[("path", dir.display().to_string().into())]))?,
        None => BTreeMap::new(),
    };
    let app_fonts = Fonts::new(fonts.values().cloned());
    let limits = SourceLimits {
        replica_bytes: platform.settings.sources.max_replica_bytes(),
        total_bytes: platform.settings.sources.max_sources_bytes(),
    };

    match cli.cmd {
        Cmd::Check { deck } => {
            let base = registry(&app_fonts)?;
            let deck_dir = deck.parent().map(Path::to_path_buf).unwrap_or_default();
            let deck = read_deck(storage, l, &deck)?;
            let library = pack_library(platform);
            // Un pack introuvable n'arrête pas la vérification : signalé, ses widgets sont inconnus.
            let packs = deck_pack(storage, &library, l, &deck_dir, &deck).unwrap_or_else(|e| {
                tracing::warn!("{e}");
                BTreeMap::new()
            });
            let (bundle, widgets) = sources_with_pack(platform, l, &library, &base, deck, packs)?;
            let deck = bundle.deck();
            if report_issues(l, deck, &widgets.registry, &BTreeMap::new()) {
                out.line("cli-deck-valid", &[("slides", deck.slides.len().into())])?;
            }
        }
        Cmd::Render { input, out: dir, theme } => {
            let (mut bundle, widgets) = load_bundle(platform, l, &input, limits, &registry(&app_fonts)?)?;
            let themes = theme_library(platform);
            adopt_default_theme(&themes, &mut bundle).map_err(|e| color_eyre::eyre::eyre!(e.localize(l)))?;
            let mut deck = bundle.deck().clone();
            if let Some(name) = theme {
                deck.theme = Some(embed_theme(&themes, l, &name, &mut bundle)?);
            }
            let t = Instant::now();
            let fonts = app_fonts.with(bundle.fonts.values().cloned());
            let doc = pack_render::render(platform, l, fonts, &widgets, &bundle, &deck)?;
            let compile = t.elapsed();
            for w in doc.warnings() {
                tracing::warn!("{}", w.localize(l));
            }
            let t = Instant::now();
            for i in 0..doc.page_count() {
                storage.write(&dir.join(format!("page-{:02}.svg", i + 1)), doc.svg(i)?)?;
            }
            let svg = t.elapsed();
            storage.write(&dir.join("layout.json"), serde_json::to_string_pretty(doc.layout())?)?;
            out.line(
                "cli-rendered",
                &[
                    ("pages", doc.page_count().into()),
                    ("out", dir.display().to_string().into()),
                    ("compile", ms(compile).into()),
                    ("svg", ms(svg).into()),
                ],
            )?;
        }
        Cmd::Save { deck, output, assets, embed_fonts, theme } => {
            let base = registry(&app_fonts)?;
            let deck_dir = deck.parent().map(Path::to_path_buf).unwrap_or_default();
            let deck = read_deck(storage, l, &deck)?;
            let library = pack_library(platform);
            let packs = deck_pack(storage, &library, l, &deck_dir, &deck)?;
            let (mut bundle, widgets) = sources_with_pack(platform, l, &library, &base, deck, packs)?;
            let themes = theme_library(platform);
            if let Some(name) = theme {
                let mut deck = bundle.deck().clone();
                deck.theme = Some(embed_theme(&themes, l, &name, &mut bundle)?);
                bundle.apply_silent(deck)?;
            }
            adopt_default_theme(&themes, &mut bundle).map_err(|e| color_eyre::eyre::eyre!(e.localize(l)))?;
            if let Some(dir) = assets {
                for path in storage.list_files(&dir)? {
                    let id = bundle.add_asset(storage.read(&path)?);
                    out.line("cli-asset-added", &[("path", path.display().to_string().into()), ("id", id.into())])?;
                }
            }
            if embed_fonts {
                bundle.fonts = fonts;
            }
            report_issues(l, bundle.deck(), &widgets.registry, &bundle.assets);
            let written = write_document(platform, l, app_fonts.clone(), &widgets, &bundle, &output)?;
            out.line(
                "cli-saved",
                &[
                    ("path", output.display().to_string().into()),
                    ("pages", written.pages.into()),
                    ("pdf", (written.pdf / 1024).into()),
                    ("sources", (written.sources / 1024).into()),
                ],
            )?;
        }
        Cmd::Open { input, out: extract } => {
            let (bundle, _) = load_bundle(platform, l, &input, limits, &registry(&app_fonts)?)?;
            let deck = bundle.deck();
            out.line(
                "cli-opened",
                &[
                    ("path", input.display().to_string().into()),
                    ("slides", deck.slides.len().into()),
                    ("assets", bundle.assets.len().into()),
                    ("fonts", bundle.fonts.len().into()),
                ],
            )?;
            if let Some(dir) = extract {
                storage.write(&dir.join("deck.json"), deck.to_json()?)?;
                for (id, bytes) in &bundle.assets {
                    storage.write(&dir.join("assets").join(id.trim_start_matches("sha256:")), bytes.as_slice())?;
                }
                for (name, bytes) in &bundle.fonts {
                    storage.write(&dir.join("fonts").join(name), bytes.as_slice())?;
                }
                for (version, files) in &bundle.packs {
                    let at = dir.join("packs").join(version.hex());
                    storage.write(&at.join("pack.json"), files.pack_json()?)?;
                    for (name, source) in files.widgets() {
                        storage.write(&at.join("widgets").join(name.file_name()), source.as_bytes())?;
                    }
                }
                out.line("cli-extracted", &[("dir", dir.display().to_string().into())])?;
            }
        }
        Cmd::Strip { input, output } => {
            let pdf = storage.read(&input)?;
            storage.write_atomic(&output, oku_slides_pdf::strip_sources(&pdf)?)?;
            out.line("cli-stripped", &[("path", output.display().to_string().into())])?;
        }
        Cmd::Import { input, mode, pack, theme, output, report } => {
            let args = ImportArgs { input, mode, pack, theme, output, report };
            import::import(platform, out, &app_fonts, args)?;
        }
        Cmd::Bench { deck, iterations } => {
            let deck = read_deck(storage, l, &deck)?;
            let themes = theme_library(platform);
            // Un deck JSON n'embarque rien : son thème se résout par la bibliothèque, sinon un deck Pro serait mesuré en Papier.
            let embedded: BTreeMap<_, _> = deck
                .theme
                .iter()
                .filter_map(|r| themes.version(r.id, &r.version).map(|f| (r.version.clone(), f)))
                .collect();
            let tokens = theme_tokens(&themes, &embedded, &deck);
            bench::bench(out, deck, tokens, app_fonts, iterations, limits)?
        }
        Cmd::Schema => out.data(&serde_json::to_string_pretty(&oku_slides_core::ops_schema()?)?)?,
        Cmd::Serve { input, port, ui } => serve::serve(out, input, port, &ui, app_fonts, platform)?,
    }
    Ok(())
}
```

Messages : dans chaque `crates/oku-slides-i18n/src/locales/<locale>/cli.ftl`, le premier bloc ci-dessous va à la fin du premier groupe de clés (après `cli-exposed`, avant la ligne vide qui précède `cli-about`), le second à la fin du fichier. Variables : `cli-imported` a `$path $slides $pages $losses $requests` (sélecteurs pluriels sur les quatre nombres), `cli-import-unknown-pack` et `cli-import-bad-pack` ont `$pack`, `cli-import-write-failed` a `$path`, les autres aucune. En chinois et en japonais, le nombre colle au classificateur (`{ $slides }张`, `{ $losses }件`).

`ar/cli.ftl` :

```ftl
cli-imported = { $path }: { $slides ->
       [zero] لا شرائح
       [one] شريحة واحدة
       [two] شريحتان
       [few] { $slides } شرائح
       [many] { $slides } شريحة
      *[other] { $slides } شريحة
    }، { $pages ->
       [zero] لا صفحات
       [one] صفحة واحدة
       [two] صفحتان
       [few] { $pages } صفحات
       [many] { $pages } صفحة
      *[other] { $pages } صفحة
    }، { $losses ->
       [zero] لا فقد
       [one] فقد واحد
       [two] فقدان
       [few] { $losses } حالات فقد
       [many] { $losses } حالة فقد
      *[other] { $losses } حالة فقد
    }، { $requests ->
       [zero] لا طلبات أدوات
       [one] طلب أداة واحد
       [two] طلبا أداة
       [few] { $requests } طلبات أدوات
       [many] { $requests } طلب أداة
      *[other] { $requests } طلب أداة
    }
cli-import-label = استيراد من PowerPoint
cli-import-pack-required = الخيار --pack مطلوب في وضع retrofit: معرّف حزمة القوالب أو مجلدها أو اسمها
cli-import-unknown-pack = حزمة قوالب غير معروفة: { $pack }
cli-import-bad-pack = حزمة قوالب غير مقروءة { $pack }: ملف pack.json الخاص بها لا يصف حزمة قوالب
cli-import-write-failed = تعذّرت كتابة العرض المستورد { $path }
cli-import-hifi-later = الاستيراد عالي الدقة (--mode hifi) غير متاح بعد: استخدم --mode retrofit
```

```ftl
cli-cmd-import = استيراد عرض PowerPoint (.pptx) بصيغة .oku-slides.pdf
cli-arg-pptx = عرض PowerPoint (.pptx)
cli-arg-mode = retrofit: المحتوى على حزمة قوالب وسمة موجودتين؛ hifi: نسخة مطابقة (غير متاحة بعد)
cli-arg-pack = حزمة القوالب الهدف: المعرّف أو مجلد الحزمة (pack.json، widgets/) أو الاسم
cli-arg-import-theme = السمة الهدف: سمة مدمجة أو من المكتبة، بمعرّفها أو اسمها؛ الافتراضي: السمة الافتراضية
cli-arg-report = كتابة تقرير الاستيراد (JSON) أيضًا: المفقودات والبدائل والتغطية
```

`bn/cli.ftl` :

```ftl
cli-imported = { $path }: { $slides ->
       [one] { $slides }টি স্লাইড
      *[other] { $slides }টি স্লাইড
    }, { $pages ->
       [one] { $pages } পৃষ্ঠা
      *[other] { $pages } পৃষ্ঠা
    }, { $losses ->
       [one] { $losses }টি ক্ষতি
      *[other] { $losses }টি ক্ষতি
    }, { $requests ->
       [one] { $requests }টি উইজেট অনুরোধ
      *[other] { $requests }টি উইজেট অনুরোধ
    }
cli-import-label = PowerPoint ইমপোর্ট
cli-import-pack-required = retrofit মোডে --pack আবশ্যক: টেমপ্লেট প্যাকের আইডি, ফোল্ডার বা নাম
cli-import-unknown-pack = অজানা টেমপ্লেট প্যাক: { $pack }
cli-import-bad-pack = অপাঠযোগ্য টেমপ্লেট প্যাক { $pack }: এর pack.json কোনো টেমপ্লেট প্যাক বর্ণনা করে না
cli-import-write-failed = ইমপোর্ট করা ডেক { $path } লেখা যায়নি
cli-import-hifi-later = উচ্চ বিশ্বস্ততার ইমপোর্ট (--mode hifi) এখনো উপলব্ধ নয়: --mode retrofit ব্যবহার করুন
```

```ftl
cli-cmd-import = একটি PowerPoint প্রেজেন্টেশন (.pptx) .oku-slides.pdf হিসেবে ইমপোর্ট করুন
cli-arg-pptx = PowerPoint প্রেজেন্টেশন (.pptx)
cli-arg-mode = retrofit: বিদ্যমান টেমপ্লেট প্যাক ও থিমে বিষয়বস্তু; hifi: হুবহু কপি (এখনো উপলব্ধ নয়)
cli-arg-pack = লক্ষ্য টেমপ্লেট প্যাক: আইডি, প্যাক ফোল্ডার (pack.json, widgets/) বা নাম
cli-arg-import-theme = লক্ষ্য থিম: অন্তর্নির্মিত বা লাইব্রেরির থিম, আইডি বা নাম দিয়ে; ডিফল্ট: ডিফল্ট থিম
cli-arg-report = ইমপোর্ট রিপোর্টও (JSON) লিখুন: ক্ষতি, বিকল্প, কভারেজ
```

`de/cli.ftl` :

```ftl
cli-imported = { $path }: { $slides ->
       [one] { $slides } Folie
      *[other] { $slides } Folien
    }, { $pages ->
       [one] { $pages } Seite
      *[other] { $pages } Seiten
    }, { $losses ->
       [one] { $losses } Verlust
      *[other] { $losses } Verluste
    }, { $requests ->
       [one] { $requests } Widget-Anfrage
      *[other] { $requests } Widget-Anfragen
    }
cli-import-label = PowerPoint-Import
cli-import-pack-required = --pack ist im Modus retrofit erforderlich: ID, Ordner oder Name eines Vorlagenpakets
cli-import-unknown-pack = unbekanntes Vorlagenpaket: { $pack }
cli-import-bad-pack = unlesbares Vorlagenpaket { $pack }: seine pack.json beschreibt kein Vorlagenpaket
cli-import-write-failed = das importierte Deck { $path } konnte nicht geschrieben werden
cli-import-hifi-later = der originalgetreue Import (--mode hifi) ist noch nicht verfügbar: verwenden Sie --mode retrofit
```

```ftl
cli-cmd-import = Eine PowerPoint-Präsentation (.pptx) als .oku-slides.pdf importieren
cli-arg-pptx = PowerPoint-Präsentation (.pptx)
cli-arg-mode = retrofit: der Inhalt auf einem vorhandenen Vorlagenpaket und Design; hifi: originalgetreue Kopie (noch nicht verfügbar)
cli-arg-pack = Ziel-Vorlagenpaket: ID, Paketordner (pack.json, widgets/) oder Name
cli-arg-import-theme = Ziel-Design: ein mitgeliefertes oder eines aus der Bibliothek, per ID oder Name; Standard: das Standarddesign
cli-arg-report = Zusätzlich den Importbericht (JSON) schreiben: Verluste, Ersatzlösungen, Abdeckung
```

`en/cli.ftl` :

```ftl
cli-imported = { $path }: { $slides ->
       [one] { $slides } slide
      *[other] { $slides } slides
    }, { $pages ->
       [one] { $pages } page
      *[other] { $pages } pages
    }, { $losses ->
       [one] { $losses } loss
      *[other] { $losses } losses
    }, { $requests ->
       [one] { $requests } widget request
      *[other] { $requests } widget requests
    }
cli-import-label = PowerPoint import
cli-import-pack-required = --pack is required in retrofit mode: a pack id, folder or name
cli-import-unknown-pack = unknown pack: { $pack }
cli-import-bad-pack = unreadable pack { $pack }: its pack.json does not describe a template pack
cli-import-write-failed = could not write the imported deck { $path }
cli-import-hifi-later = high-fidelity import (--mode hifi) is not available yet: use --mode retrofit
```

```ftl
cli-cmd-import = Import a PowerPoint presentation (.pptx) as an .oku-slides.pdf deck
cli-arg-pptx = PowerPoint presentation (.pptx)
cli-arg-mode = retrofit: the content on an existing pack and theme; hifi: faithful copy (not available yet)
cli-arg-pack = Target pack: id, pack folder (pack.json, widgets/) or name
cli-arg-import-theme = Target theme: a built-in or library theme, by id or name; default: the default theme
cli-arg-report = Also write the import report (JSON): losses, fallbacks, coverage
```

`es/cli.ftl` :

```ftl
cli-imported = { $path }: { $slides ->
       [one] { $slides } diapositiva
      *[other] { $slides } diapositivas
    }, { $pages ->
       [one] { $pages } página
      *[other] { $pages } páginas
    }, { $losses ->
       [one] { $losses } pérdida
      *[other] { $losses } pérdidas
    }, { $requests ->
       [one] { $requests } solicitud de widget
      *[other] { $requests } solicitudes de widget
    }
cli-import-label = Importación de PowerPoint
cli-import-pack-required = --pack es obligatorio en el modo retrofit: el id, la carpeta o el nombre de un paquete
cli-import-unknown-pack = paquete desconocido: { $pack }
cli-import-bad-pack = paquete ilegible { $pack }: su pack.json no describe un paquete de plantillas
cli-import-write-failed = no se pudo escribir la presentación importada { $path }
cli-import-hifi-later = la importación de alta fidelidad (--mode hifi) aún no está disponible: use --mode retrofit
```

```ftl
cli-cmd-import = Importar una presentación de PowerPoint (.pptx) como .oku-slides.pdf
cli-arg-pptx = Presentación de PowerPoint (.pptx)
cli-arg-mode = retrofit: el contenido sobre un paquete y un tema existentes; hifi: copia fiel (aún no disponible)
cli-arg-pack = Paquete de destino: id, carpeta del paquete (pack.json, widgets/) o nombre
cli-arg-import-theme = Tema de destino: uno incluido o de la biblioteca, por su id o nombre; por defecto: el tema predeterminado
cli-arg-report = Escribir también el informe de importación (JSON): pérdidas, sustituciones, cobertura
```

`fr/cli.ftl` :

```ftl
cli-imported = { $path } : { $slides ->
       [one] { $slides } slide
      *[other] { $slides } slides
    }, { $pages ->
       [one] { $pages } page
      *[other] { $pages } pages
    }, { $losses ->
       [one] { $losses } perte
      *[other] { $losses } pertes
    }, { $requests ->
       [one] { $requests } demande de widget
      *[other] { $requests } demandes de widget
    }
cli-import-label = Import PowerPoint
cli-import-pack-required = --pack est obligatoire en mode retrofit : un id, un dossier ou un nom de pack
cli-import-unknown-pack = pack inconnu : { $pack }
cli-import-bad-pack = pack illisible { $pack } : son pack.json ne décrit pas un pack de modèles
cli-import-write-failed = impossible d'écrire le deck importé { $path }
cli-import-hifi-later = l'import haute fidélité (--mode hifi) n'est pas encore disponible : utilisez --mode retrofit
```

```ftl
cli-cmd-import = Importe une présentation PowerPoint (.pptx) en deck .oku-slides.pdf
cli-arg-pptx = Présentation PowerPoint (.pptx)
cli-arg-mode = retrofit : le contenu sur un pack et un thème existants ; hifi : copie fidèle (pas encore disponible)
cli-arg-pack = Pack cible : id, dossier de pack (pack.json, widgets/) ou nom
cli-arg-import-theme = Thème cible : un thème intégré ou de la bibliothèque, par son id ou son nom ; défaut : le thème par défaut
cli-arg-report = Écrit aussi le rapport d'import (JSON) : pertes, replis, couverture
```

`hi/cli.ftl` :

```ftl
cli-imported = { $path }: { $slides ->
       [one] { $slides } स्लाइड
      *[other] { $slides } स्लाइड
    }, { $pages ->
       [one] { $pages } पेज
      *[other] { $pages } पेज
    }, { $losses ->
       [one] { $losses } हानि
      *[other] { $losses } हानियाँ
    }, { $requests ->
       [one] { $requests } विजेट अनुरोध
      *[other] { $requests } विजेट अनुरोध
    }
cli-import-label = PowerPoint आयात
cli-import-pack-required = retrofit मोड में --pack ज़रूरी है: टेम्पलेट पैक की आईडी, फ़ोल्डर या नाम
cli-import-unknown-pack = अज्ञात टेम्पलेट पैक: { $pack }
cli-import-bad-pack = अपठनीय टेम्पलेट पैक { $pack }: इसकी pack.json टेम्पलेट पैक का वर्णन नहीं करती
cli-import-write-failed = आयात किया गया डेक { $path } लिखा नहीं जा सका
cli-import-hifi-later = उच्च-निष्ठा आयात (--mode hifi) अभी उपलब्ध नहीं है: --mode retrofit इस्तेमाल करें
```

```ftl
cli-cmd-import = PowerPoint प्रेज़ेंटेशन (.pptx) को .oku-slides.pdf के रूप में आयात करें
cli-arg-pptx = PowerPoint प्रेज़ेंटेशन (.pptx)
cli-arg-mode = retrofit: मौजूदा टेम्पलेट पैक और थीम पर सामग्री; hifi: हूबहू प्रति (अभी उपलब्ध नहीं)
cli-arg-pack = लक्ष्य टेम्पलेट पैक: आईडी, पैक फ़ोल्डर (pack.json, widgets/) या नाम
cli-arg-import-theme = लक्ष्य थीम: बिल्ट-इन या लाइब्रेरी की थीम, आईडी या नाम से; डिफ़ॉल्ट: डिफ़ॉल्ट थीम
cli-arg-report = आयात रिपोर्ट (JSON) भी लिखें: हानियाँ, विकल्प, कवरेज
```

`it/cli.ftl` :

```ftl
cli-imported = { $path }: { $slides ->
       [one] { $slides } diapositiva
      *[other] { $slides } diapositive
    }, { $pages ->
       [one] { $pages } pagina
      *[other] { $pages } pagine
    }, { $losses ->
       [one] { $losses } perdita
      *[other] { $losses } perdite
    }, { $requests ->
       [one] { $requests } richiesta di widget
      *[other] { $requests } richieste di widget
    }
cli-import-label = Importazione PowerPoint
cli-import-pack-required = --pack è obbligatorio in modalità retrofit: l'id, la cartella o il nome di un pacchetto
cli-import-unknown-pack = pacchetto sconosciuto: { $pack }
cli-import-bad-pack = pacchetto illeggibile { $pack }: il suo pack.json non descrive un pacchetto di modelli
cli-import-write-failed = impossibile scrivere la presentazione importata { $path }
cli-import-hifi-later = l'importazione ad alta fedeltà (--mode hifi) non è ancora disponibile: usa --mode retrofit
```

```ftl
cli-cmd-import = Importa una presentazione PowerPoint (.pptx) come .oku-slides.pdf
cli-arg-pptx = Presentazione PowerPoint (.pptx)
cli-arg-mode = retrofit: il contenuto su un pacchetto e un tema esistenti; hifi: copia fedele (non ancora disponibile)
cli-arg-pack = Pacchetto di destinazione: id, cartella del pacchetto (pack.json, widgets/) o nome
cli-arg-import-theme = Tema di destinazione: uno incluso o della libreria, per id o nome; predefinito: il tema predefinito
cli-arg-report = Scrivi anche il rapporto di importazione (JSON): perdite, sostituzioni, copertura
```

`ja/cli.ftl` :

```ftl
cli-imported = { $path }：スライド { $slides }枚、{ $pages }ページ、損失 { $losses }件、ウィジェット要求 { $requests }件
cli-import-label = PowerPoint のインポート
cli-import-pack-required = retrofit モードでは --pack が必要です（テンプレートパックの ID、フォルダー、または名前）
cli-import-unknown-pack = 不明なテンプレートパック：{ $pack }
cli-import-bad-pack = 読み取れないテンプレートパック { $pack }：pack.json がテンプレートパックを記述していません
cli-import-write-failed = インポートしたデッキ { $path } を書き込めませんでした
cli-import-hifi-later = 高忠実度インポート（--mode hifi）はまだ利用できません。--mode retrofit を使用してください
```

```ftl
cli-cmd-import = PowerPoint プレゼンテーション（.pptx）を .oku-slides.pdf としてインポートします
cli-arg-pptx = PowerPoint プレゼンテーション（.pptx）
cli-arg-mode = retrofit：既存のテンプレートパックとテーマに内容を載せます。hifi：忠実なコピー（まだ利用できません）
cli-arg-pack = 対象のテンプレートパック：ID、パックのフォルダー（pack.json、widgets/）、または名前
cli-arg-import-theme = 対象のテーマ：組み込みまたはライブラリのテーマを ID か名前で指定。既定：既定のテーマ
cli-arg-report = インポートレポート（JSON）も書き出します：損失、代替、カバレッジ
```

`ko/cli.ftl` :

```ftl
cli-imported = { $path }: 슬라이드 { $slides }개, { $pages }페이지, 손실 { $losses }건, 위젯 요청 { $requests }건
cli-import-label = PowerPoint 가져오기
cli-import-pack-required = retrofit 모드에서는 --pack이 필요합니다: 템플릿 팩의 ID, 폴더 또는 이름
cli-import-unknown-pack = 알 수 없는 템플릿 팩: { $pack }
cli-import-bad-pack = 읽을 수 없는 템플릿 팩 { $pack }: pack.json이 템플릿 팩을 설명하지 않습니다
cli-import-write-failed = 가져온 덱 { $path }을(를) 쓸 수 없습니다
cli-import-hifi-later = 고충실도 가져오기(--mode hifi)는 아직 사용할 수 없습니다: --mode retrofit을 사용하세요
```

```ftl
cli-cmd-import = PowerPoint 프레젠테이션(.pptx)을 .oku-slides.pdf로 가져오기
cli-arg-pptx = PowerPoint 프레젠테이션 (.pptx)
cli-arg-mode = retrofit: 기존 템플릿 팩과 테마에 내용을 담음; hifi: 충실한 복사본 (아직 사용할 수 없음)
cli-arg-pack = 대상 템플릿 팩: ID, 팩 폴더(pack.json, widgets/) 또는 이름
cli-arg-import-theme = 대상 테마: 기본 제공 테마 또는 라이브러리 테마를 ID나 이름으로 지정; 기본값: 기본 테마
cli-arg-report = 가져오기 보고서(JSON)도 쓰기: 손실, 대체, 포함 범위
```

`nl/cli.ftl` :

```ftl
cli-imported = { $path }: { $slides ->
       [one] { $slides } dia
      *[other] { $slides } dia's
    }, { $pages ->
       [one] { $pages } pagina
      *[other] { $pages } pagina's
    }, { $losses ->
       [one] { $losses } verlies
      *[other] { $losses } verliezen
    }, { $requests ->
       [one] { $requests } widgetverzoek
      *[other] { $requests } widgetverzoeken
    }
cli-import-label = PowerPoint-import
cli-import-pack-required = --pack is verplicht in de modus retrofit: de id, map of naam van een sjabloonpakket
cli-import-unknown-pack = onbekend sjabloonpakket: { $pack }
cli-import-bad-pack = onleesbaar sjabloonpakket { $pack }: de pack.json beschrijft geen sjabloonpakket
cli-import-write-failed = kon de geïmporteerde presentatie { $path } niet schrijven
cli-import-hifi-later = import met hoge getrouwheid (--mode hifi) is nog niet beschikbaar: gebruik --mode retrofit
```

```ftl
cli-cmd-import = Een PowerPoint-presentatie (.pptx) importeren als .oku-slides.pdf
cli-arg-pptx = PowerPoint-presentatie (.pptx)
cli-arg-mode = retrofit: de inhoud op een bestaand sjabloonpakket en thema; hifi: getrouwe kopie (nog niet beschikbaar)
cli-arg-pack = Doelpakket: id, pakketmap (pack.json, widgets/) of naam
cli-arg-import-theme = Doelthema: een ingebouwd thema of een uit de bibliotheek, op id of naam; standaard: het standaardthema
cli-arg-report = Ook het importrapport (JSON) schrijven: verliezen, vervangingen, dekking
```

`pl/cli.ftl` :

```ftl
cli-imported = { $path }: { $slides ->
       [one] { $slides } slajd
       [few] { $slides } slajdy
       [many] { $slides } slajdów
      *[other] { $slides } slajdu
    }, { $pages ->
       [one] { $pages } strona
       [few] { $pages } strony
       [many] { $pages } stron
      *[other] { $pages } strony
    }, { $losses ->
       [one] { $losses } strata
       [few] { $losses } straty
       [many] { $losses } strat
      *[other] { $losses } straty
    }, { $requests ->
       [one] { $requests } prośba o widżet
       [few] { $requests } prośby o widżet
       [many] { $requests } próśb o widżet
      *[other] { $requests } prośby o widżet
    }
cli-import-label = Import z PowerPointa
cli-import-pack-required = --pack jest wymagany w trybie retrofit: identyfikator, folder lub nazwa pakietu
cli-import-unknown-pack = nieznany pakiet: { $pack }
cli-import-bad-pack = nieczytelny pakiet { $pack }: jego pack.json nie opisuje pakietu szablonów
cli-import-write-failed = nie udało się zapisać zaimportowanej prezentacji { $path }
cli-import-hifi-later = import o wysokiej wierności (--mode hifi) nie jest jeszcze dostępny: użyj --mode retrofit
```

```ftl
cli-cmd-import = Zaimportuj prezentację PowerPoint (.pptx) jako .oku-slides.pdf
cli-arg-pptx = Prezentacja PowerPoint (.pptx)
cli-arg-mode = retrofit: treść na istniejącym pakiecie i motywie; hifi: wierna kopia (jeszcze niedostępna)
cli-arg-pack = Pakiet docelowy: identyfikator, folder pakietu (pack.json, widgets/) lub nazwa
cli-arg-import-theme = Motyw docelowy: wbudowany lub z biblioteki, wskazany identyfikatorem lub nazwą; domyślnie: motyw domyślny
cli-arg-report = Zapisz też raport importu (JSON): straty, zamienniki, pokrycie
```

`pt/cli.ftl` :

```ftl
cli-imported = { $path }: { $slides ->
       [one] { $slides } slide
      *[other] { $slides } slides
    }, { $pages ->
       [one] { $pages } página
      *[other] { $pages } páginas
    }, { $losses ->
       [one] { $losses } perda
      *[other] { $losses } perdas
    }, { $requests ->
       [one] { $requests } pedido de widget
      *[other] { $requests } pedidos de widget
    }
cli-import-label = Importação do PowerPoint
cli-import-pack-required = --pack é obrigatório no modo retrofit: o id, a pasta ou o nome de um pacote
cli-import-unknown-pack = pacote desconhecido: { $pack }
cli-import-bad-pack = pacote ilegível { $pack }: o seu pack.json não descreve um pacote de modelos
cli-import-write-failed = não foi possível gravar a apresentação importada { $path }
cli-import-hifi-later = a importação de alta fidelidade (--mode hifi) ainda não está disponível: use --mode retrofit
```

```ftl
cli-cmd-import = Importar uma apresentação do PowerPoint (.pptx) como .oku-slides.pdf
cli-arg-pptx = Apresentação do PowerPoint (.pptx)
cli-arg-mode = retrofit: o conteúdo sobre um pacote e um tema existentes; hifi: cópia fiel (ainda não disponível)
cli-arg-pack = Pacote de destino: id, pasta do pacote (pack.json, widgets/) ou nome
cli-arg-import-theme = Tema de destino: um tema incluído ou da biblioteca, pelo id ou nome; padrão: o tema padrão
cli-arg-report = Gravar também o relatório de importação (JSON): perdas, substituições, cobertura
```

`ru/cli.ftl` :

```ftl
cli-imported = { $path }: { $slides ->
       [one] { $slides } слайд
       [few] { $slides } слайда
       [many] { $slides } слайдов
      *[other] { $slides } слайда
    }, { $pages ->
       [one] { $pages } страница
       [few] { $pages } страницы
       [many] { $pages } страниц
      *[other] { $pages } страницы
    }, { $losses ->
       [one] { $losses } потеря
       [few] { $losses } потери
       [many] { $losses } потерь
      *[other] { $losses } потери
    }, { $requests ->
       [one] { $requests } запрос виджета
       [few] { $requests } запроса виджетов
       [many] { $requests } запросов виджетов
      *[other] { $requests } запроса виджетов
    }
cli-import-label = Импорт из PowerPoint
cli-import-pack-required = в режиме retrofit нужен --pack: идентификатор, папка или название пакета
cli-import-unknown-pack = неизвестный пакет: { $pack }
cli-import-bad-pack = нечитаемый пакет { $pack }: его pack.json не описывает пакет шаблонов
cli-import-write-failed = не удалось записать импортированную презентацию { $path }
cli-import-hifi-later = импорт с высокой точностью (--mode hifi) пока недоступен: используйте --mode retrofit
```

```ftl
cli-cmd-import = Импортировать презентацию PowerPoint (.pptx) в .oku-slides.pdf
cli-arg-pptx = Презентация PowerPoint (.pptx)
cli-arg-mode = retrofit: содержимое на существующем пакете и теме; hifi: точная копия (пока недоступно)
cli-arg-pack = Целевой пакет: идентификатор, папка пакета (pack.json, widgets/) или название
cli-arg-import-theme = Целевая тема: встроенная или из библиотеки, по идентификатору или названию; по умолчанию: тема по умолчанию
cli-arg-report = Также записать отчёт об импорте (JSON): потери, замены, охват
```

`th/cli.ftl` :

```ftl
cli-imported = { $path }: สไลด์ { $slides } รายการ, { $pages } หน้า, การสูญเสีย { $losses } รายการ, คำขอวิดเจ็ต { $requests } รายการ
cli-import-label = นำเข้าจาก PowerPoint
cli-import-pack-required = โหมด retrofit ต้องระบุ --pack: รหัส โฟลเดอร์ หรือชื่อของแพ็กเทมเพลต
cli-import-unknown-pack = ไม่รู้จักแพ็กเทมเพลต: { $pack }
cli-import-bad-pack = อ่านแพ็กเทมเพลต { $pack } ไม่ได้: pack.json ของแพ็กไม่ได้อธิบายแพ็กเทมเพลต
cli-import-write-failed = เขียนเด็คที่นำเข้า { $path } ไม่ได้
cli-import-hifi-later = การนำเข้าแบบเที่ยงตรงสูง (--mode hifi) ยังไม่พร้อมใช้งาน: ใช้ --mode retrofit
```

```ftl
cli-cmd-import = นำเข้างานนำเสนอ PowerPoint (.pptx) เป็น .oku-slides.pdf
cli-arg-pptx = งานนำเสนอ PowerPoint (.pptx)
cli-arg-mode = retrofit: เนื้อหาบนแพ็กเทมเพลตและธีมที่มีอยู่; hifi: สำเนาที่เหมือนต้นฉบับ (ยังไม่พร้อมใช้งาน)
cli-arg-pack = แพ็กเทมเพลตเป้าหมาย: รหัส โฟลเดอร์แพ็ก (pack.json, widgets/) หรือชื่อ
cli-arg-import-theme = ธีมเป้าหมาย: ธีมในตัวหรือจากคลัง ระบุด้วยรหัสหรือชื่อ; ค่าเริ่มต้น: ธีมเริ่มต้น
cli-arg-report = เขียนรายงานการนำเข้า (JSON) ด้วย: การสูญเสีย ทางเลือกสำรอง ความครอบคลุม
```

`tr/cli.ftl` :

```ftl
cli-imported = { $path }: { $slides ->
       [one] { $slides } slayt
      *[other] { $slides } slayt
    }, { $pages ->
       [one] { $pages } sayfa
      *[other] { $pages } sayfa
    }, { $losses ->
       [one] { $losses } kayıp
      *[other] { $losses } kayıp
    }, { $requests ->
       [one] { $requests } widget isteği
      *[other] { $requests } widget isteği
    }
cli-import-label = PowerPoint içe aktarma
cli-import-pack-required = retrofit modunda --pack gerekir: bir şablon paketinin kimliği, klasörü ya da adı
cli-import-unknown-pack = bilinmeyen şablon paketi: { $pack }
cli-import-bad-pack = okunamayan şablon paketi { $pack }: pack.json dosyası bir şablon paketi tanımlamıyor
cli-import-write-failed = içe aktarılan sunu { $path } yazılamadı
cli-import-hifi-later = yüksek doğruluklu içe aktarma (--mode hifi) henüz kullanılamıyor: --mode retrofit kullanın
```

```ftl
cli-cmd-import = Bir PowerPoint sunusunu (.pptx) .oku-slides.pdf olarak içe aktar
cli-arg-pptx = PowerPoint sunusu (.pptx)
cli-arg-mode = retrofit: içerik mevcut bir şablon paketi ve tema üzerinde; hifi: aslına uygun kopya (henüz kullanılamıyor)
cli-arg-pack = Hedef şablon paketi: kimlik, paket klasörü (pack.json, widgets/) ya da ad
cli-arg-import-theme = Hedef tema: yerleşik ya da kitaplıktaki bir tema, kimliği ya da adıyla; varsayılan: varsayılan tema
cli-arg-report = İçe aktarma raporunu da (JSON) yaz: kayıplar, yedekler, kapsam
```

`uk/cli.ftl` :

```ftl
cli-imported = { $path }: { $slides ->
       [one] { $slides } слайд
       [few] { $slides } слайди
       [many] { $slides } слайдів
      *[other] { $slides } слайда
    }, { $pages ->
       [one] { $pages } сторінка
       [few] { $pages } сторінки
       [many] { $pages } сторінок
      *[other] { $pages } сторінки
    }, { $losses ->
       [one] { $losses } втрата
       [few] { $losses } втрати
       [many] { $losses } втрат
      *[other] { $losses } втрати
    }, { $requests ->
       [one] { $requests } запит віджета
       [few] { $requests } запити віджетів
       [many] { $requests } запитів віджетів
      *[other] { $requests } запиту віджетів
    }
cli-import-label = Імпорт із PowerPoint
cli-import-pack-required = у режимі retrofit потрібен --pack: ідентифікатор, тека або назва пакета
cli-import-unknown-pack = невідомий пакет: { $pack }
cli-import-bad-pack = нечитабельний пакет { $pack }: його pack.json не описує пакет шаблонів
cli-import-write-failed = не вдалося записати імпортовану презентацію { $path }
cli-import-hifi-later = імпорт із високою точністю (--mode hifi) поки недоступний: використовуйте --mode retrofit
```

```ftl
cli-cmd-import = Імпортувати презентацію PowerPoint (.pptx) у .oku-slides.pdf
cli-arg-pptx = Презентація PowerPoint (.pptx)
cli-arg-mode = retrofit: вміст на наявному пакеті й темі; hifi: точна копія (поки недоступно)
cli-arg-pack = Цільовий пакет: ідентифікатор, тека пакета (pack.json, widgets/) або назва
cli-arg-import-theme = Цільова тема: вбудована або з бібліотеки, за ідентифікатором чи назвою; типово: тема за замовчуванням
cli-arg-report = Також записати звіт про імпорт (JSON): втрати, заміни, охоплення
```

`vi/cli.ftl` :

```ftl
cli-imported = { $path }: { $slides } trang chiếu, { $pages } trang, { $losses } mất mát, { $requests } yêu cầu widget
cli-import-label = Nhập từ PowerPoint
cli-import-pack-required = cần --pack ở chế độ retrofit: mã, thư mục hoặc tên của gói mẫu
cli-import-unknown-pack = gói mẫu không xác định: { $pack }
cli-import-bad-pack = không đọc được gói mẫu { $pack }: pack.json của gói không mô tả một gói mẫu
cli-import-write-failed = không thể ghi bản trình bày đã nhập { $path }
cli-import-hifi-later = chế độ nhập độ trung thực cao (--mode hifi) chưa khả dụng: hãy dùng --mode retrofit
```

```ftl
cli-cmd-import = Nhập bản trình bày PowerPoint (.pptx) thành .oku-slides.pdf
cli-arg-pptx = Bản trình bày PowerPoint (.pptx)
cli-arg-mode = retrofit: nội dung trên một gói mẫu và chủ đề có sẵn; hifi: bản sao trung thực (chưa khả dụng)
cli-arg-pack = Gói mẫu đích: mã, thư mục gói (pack.json, widgets/) hoặc tên
cli-arg-import-theme = Chủ đề đích: chủ đề có sẵn hoặc trong thư viện, theo mã hoặc tên; mặc định: chủ đề mặc định
cli-arg-report = Ghi thêm báo cáo nhập (JSON): mất mát, phương án thay thế, độ bao phủ
```

`zh-Hans/cli.ftl` :

```ftl
cli-imported = { $path }：{ $slides }张幻灯片，{ $pages }页，{ $losses }处损失，{ $requests }个小组件请求
cli-import-label = PowerPoint 导入
cli-import-pack-required = retrofit 模式需要 --pack：模板包的 ID、文件夹或名称
cli-import-unknown-pack = 未知的模板包：{ $pack }
cli-import-bad-pack = 无法读取模板包 { $pack }：其 pack.json 未描述模板包
cli-import-write-failed = 无法写入导入的演示文稿 { $path }
cli-import-hifi-later = 高保真导入（--mode hifi）尚不可用：请使用 --mode retrofit
```

```ftl
cli-cmd-import = 将 PowerPoint 演示文稿（.pptx）导入为 .oku-slides.pdf
cli-arg-pptx = PowerPoint 演示文稿（.pptx）
cli-arg-mode = retrofit：把内容放到现有的模板包和主题上；hifi：忠实副本（尚不可用）
cli-arg-pack = 目标模板包：ID、模板包文件夹（pack.json、widgets/）或名称
cli-arg-import-theme = 目标主题：内置主题或库中的主题，按 ID 或名称指定；默认：默认主题
cli-arg-report = 同时写出导入报告（JSON）：损失、替代、覆盖率
```

`zh-Hant/cli.ftl` :

```ftl
cli-imported = { $path }：{ $slides }張投影片，{ $pages }頁，{ $losses }處損失，{ $requests }個小工具請求
cli-import-label = PowerPoint 匯入
cli-import-pack-required = retrofit 模式需要 --pack：範本套件的 ID、資料夾或名稱
cli-import-unknown-pack = 未知的範本套件：{ $pack }
cli-import-bad-pack = 無法讀取範本套件 { $pack }：其 pack.json 未描述範本套件
cli-import-write-failed = 無法寫入匯入的簡報 { $path }
cli-import-hifi-later = 高保真匯入（--mode hifi）尚無法使用：請使用 --mode retrofit
```

```ftl
cli-cmd-import = 將 PowerPoint 簡報（.pptx）匯入為 .oku-slides.pdf
cli-arg-pptx = PowerPoint 簡報（.pptx）
cli-arg-mode = retrofit：將內容放到現有的範本套件與主題上；hifi：忠實副本（尚無法使用）
cli-arg-pack = 目標範本套件：ID、套件資料夾（pack.json、widgets/）或名稱
cli-arg-import-theme = 目標主題：內建主題或資料庫中的主題，以 ID 或名稱指定；預設：預設主題
cli-arg-report = 同時寫出匯入報告（JSON）：損失、替代、涵蓋率
```

- [ ] **Step 4 : lancer, constater le succès**

```bash
cargo nextest run -p oku-slides-cli -p oku-slides-import -p oku-slides-i18n
```

Attendu : tout passe (98 tests pour la CLI au brouillon, dont 11 dans `tests/import.rs` et 9 dans `library_tests` ; 644 pour l'import).

- [ ] **Step 5 : vérifier, commit, sous-tâche**

```bash
cargo fmt --all && cargo clippy -p oku-slides-cli -p oku-slides-import -p oku-slides-i18n --all-targets -- -D warnings && cargo nextest run -p oku-slides-cli -p oku-slides-import -p oku-slides-i18n
git add crates/oku-slides-cli/Cargo.toml Cargo.lock crates/oku-slides-cli/src/main.rs crates/oku-slides-cli/src/import.rs crates/oku-slides-cli/src/library.rs crates/oku-slides-cli/src/library_tests.rs crates/oku-slides-cli/tests/import.rs crates/oku-slides-import/src/plan.rs crates/oku-slides-import/src/plan_tests.rs crates/oku-slides-i18n/src/locales/*/cli.ftl && git diff --cached --name-only
git commit -F <scratchpad>/msg.txt   # feat(cli) : commande import, une présentation PowerPoint en .oku-slides.pdf par retrofit (t-692)
oku cli task done --project slider --id t-807
```

---

### Task 12: Relevés et documents

Plus de code dans cette tâche : les mises en commun (le texte d'un item, `Item::text` et `Item::has_text`, tâche 3 ; l'ajout sans doublon `text::note`, tâche 4 ; la fusion des jetons, tâche 1) sont faites là où elles servent la première fois (révision 1, item 12). Restent les relevés de contrôle, les documents (décisions 1 à 10 de l'en-tête, règles de la révision 1), l'essai manuel et la vérification de l'espace de travail.

**Files:**
- Modify: `crates/oku-slides-import/ARCHITECTURE.md` (réécrit), `crates/oku-slides-core/ARCHITECTURE.md`, `crates/oku-slides-session/ARCHITECTURE.md`, `docs/superpowers/specs/2026-10-09-import-pptx-design.md`, `README.md`

**Interfaces:**
- Consumes : tout le lot (tâches 1 à 11).
- Produces : rien de nouveau.

- [ ] **Step 1 : relevés**

```bash
# unwrap / expect hors tests dans ce que le lot ajoute : rien
grep -rn "\.unwrap()\|\.expect(" crates/oku-slides-import/src crates/oku-slides-cli/src/import.rs crates/oku-slides-cli/src/library.rs crates/oku-slides-core/src/tokens.rs | grep -v "_tests.rs\|test_support"
# fichiers de plus de 500 lignes : seul crates/oku-slides-cli/tests/cli.rs (antérieur au lot, non touché)
find crates/oku-slides-import crates/oku-slides-cli crates/oku-slides-core/src/pack.rs crates/oku-slides-core/src/tokens.rs -name "*.rs" | xargs wc -l | awk '$1 > 500 && $2 != "total"'
# plus aucun allow(dead_code) dans la crate d'import (tests/common garde le sien : module partagé des tests)
grep -rn "allow(dead_code" crates/oku-slides-import/src
# doublons : le texte d'un item, « ajouter si absent », la fusion des jetons
grep -rn "ItemKind::Shape(s) => s.text.as_ref()" crates/oku-slides-import/src
grep -rn "if !losses.contains" crates/oku-slides-import/src
grep -rn "fn merge(base" crates
```

Attendu : les trois premières commandes ne rendent rien (hors `cli.rs`) ; la quatrième ne trouve que `source/mod.rs` (`Item::text`), la cinquième que `retrofit/text.rs` (`note`), la sixième que `crates/oku-slides-core/src/tokens.rs`. `norm` de `coverage.rs` (espaces normalisées pour comparer) et `text::plain` (texte d'un titre, borné) ne font pas la même chose : ils restent à part.

- [ ] **Step 2 : documents**

`crates/oku-slides-import/ARCHITECTURE.md` complet :

```markdown
# oku-slides-import

Import d'une présentation PowerPoint (`.pptx`) : lecture en représentation
intermédiaire, `source::SourceDeck` (neutre vis-à-vis du format, résolue,
chaque valeur stylée avec sa provenance), puis projection en un plan d'ops
Oku. Spec : `docs/superpowers/specs/2026-10-09-import-pptx-design.md`.

## API

- `read_pptx(bytes, &ReadLimits) -> Result<SourceDeck, ImportError>` : fatal
  seulement pour un paquet illisible, pas une présentation, protégé, sans
  master utilisable, ou une limite dépassée.
- `source` : les types de la représentation, sans ré-export à plat.
- `retrofit::project(&SourceDeck, &RetrofitTarget) -> ImportPlan` : le
  contenu projeté sur un pack et un thème existants. `RetrofitTarget` :
  modèles du pack (`oku_slides_core::Template`), registre des widgets,
  `ColorTokens` et `PageTokens` tirés des jetons effectifs du thème.
- `plan` : `ImportPlan { ops, assets, widget_requests, report }`,
  `WidgetRequest` ; `ImportPlan::report_json(l)` (forme de `--report`).
- `report` : `ImportReport`, `SlideReport`, `Loss`/`LossKind` (localisés,
  `import-loss-*`), avertissements du lecteur localisés (`import-warning-*`),
  `Coverage`, `Ratio`, `Missing`.
- `coverage::coverage(&SourceDeck, &Deck, &WidgetRegistry, &origins) -> Coverage` :
  ce que le deck obtenu garde de la source (runs à leurs frontières de mots
  dans l'élément projeté depuis leur item) ; `project` la remplit.
- Ré-exports à la racine : `ImportPlan`, `ImportReport`, `RetrofitTarget`,
  `ImportError`, `ImportErrorKind`, `ReadLimits`, `read_pptx`.

## Couches

- `source/` ne dépend de rien ; `error.rs`, `limits.rs` (`ReadLimits` :
  octets, parts, nœuds XML, slides, avec `Default`) et `collect.rs` (assets,
  avertissements, fatal différé, assainissement) sont neutres vis-à-vis du format.
- `pptx/` (et demain `pdf/`) dépend de `source/`, `error.rs`, `collect.rs`.
  Dans `pptx/`, `shapes` → `frame` → `smartart` → `shapes` forment un cycle
  borné par `GroupCtx.depth` et `in_drawing`.
- Projecteurs (`retrofit/`, au lot 3 `hifi/`) : `source/`, `plan`, `report`,
  `pattern`, `coverage`, `oku-slides-core`, `oku-slides-chart` ; jamais `pptx/`.
  Ils sont purs : ni aléa, ni horloge, ni fichier, ni session. `oku-slides-core`
  ne dépend jamais de cette crate ; elle ne dépend ni de la session, ni du rendu,
  ni de la CLI (qui l'appelle : `oku-slides import`).

## Projecteur retrofit (`retrofit/`)

- `mod.rs` : `project` — accents déduits du deck, décor omis, profils des
  modèles, une slide source après l'autre, assets référencés, demandes de
  widget groupées par signature, couverture mesurée sur les ops appliquées à
  un deck vide.
- `decor.rs` : ce que la règle omet (placeholders date, pied, numéro ;
  en-tête et pied répétés dans les bandes de la page) — un seul prédicat,
  partagé avec `coverage`.
- `slide.rs` : une slide → `AddSlide` (`s<numéro>`, éléments `s<numéro>-<rang>`),
  `SetDeckParam` (première couverture), `SetNotes` ; placement `flow` (blocs,
  agencement du modèle, rangée en pile) ou libre (boîte englobante ajustée
  dans la zone : cadres, ou cellules de la grille si elles ne se recouvrent pas).
- `heading.rs` : titres d'une slide, des placeholders ou déduits (taille,
  position) ; couverture déduite.
- `cluster.rs` : unités de contenu — cartes (forme remplie et textes posés
  dessus), textes fusionnés, items seuls.
- `template.rs` : profil d'un modèle (sa slide d'exemple et le schéma de son
  widget), forme d'une slide source (unités, rangée de colonnes), score et
  choix (à égalité, l'ordre du pack).
- `content.rs` : unité → élément (`text`, `card`, `shape`, filet `line`,
  `image`, `table`, `chart`), pertes par item source ; `group.rs` : repli
  `stack` des groupes et SmartArt dessinés, dit seulement s'il perd quelque chose.
- `text.rs` (riche, plain, bornes du cœur, couleurs lisibles sur leur fond),
  `table.rs`, `chart.rs` (options canoniques, sans couleur), `color.rs`
  (OKLab, ΔE, table du schéma → jetons, accents déduits, contraste WCAG ;
  seules des clés existantes du thème sont émises), `zone.rs` (zone de corps
  approchée par les jetons de page, ajustement `Fit`, `snap` porté de `ui/src/grid.ts`).
- `pattern.rs` (racine) : signature d'un motif, pour grouper les demandes.

## Modules de `pptx/`

- `package` (zip borné, relations en cache), `xml` (analyse bornée en nœuds
  et en profondeur ; `max_xml_nodes` dépassé = `TooLarge`, fatal).
- `context` (contexte d'une part), `placeholder`, `cascade` (seule résolution
  des styles texte), `color`/`fill`/`geometry`/`units`, `theme`, `text`.
- `master`, `slide`, `shapes`, `frame` (tableau, graphique, SmartArt, OLE) ;
  `table`, `chart`, `smartart`, `fonts` + `eot` (polices embarquées).
- `test_support` : seul endroit des paquets et contextes de test du lecteur ;
  `src/test_support.rs` : celles des projecteurs (registre de base, modèles et
  jetons des packs et thèmes intégrés, slides source) ; `tests/common` : celles
  des tests d'intégration.

## Points d'extension

- Nouveau format source (PDF, lot 6) : un module frère de `pptx/` qui produit
  la même `SourceDeck` et réutilise `collect::Sink`.
- Nouveau projecteur (hi-fi, lot 3) : un module frère de `retrofit/` qui rend
  un `ImportPlan` ; `Mode` gagne sa variante.
- Nouvelle perte : une variante de `LossKind`, sa clé `import-loss-*` dans les
  20 `document.ftl` (`message_id`, `Localize for Loss`).

## Dépendances

`zip`, `roxmltree`, `oku-slides-core` (`asset_id`, ops, modèle, `Template`),
`oku-slides-chart` (bornes des tableaux), `oku-slides-i18n`, `serde`,
`serde_json`, `snafu`.
```

Cœur (`tokens.rs`, modèles), session, spec (décisions du plan, règles du retrofit révisées, points d'entrée — `--mode` vaut `retrofit` par défaut —, table des lots) et `README.md` (diff exact ; le README est en anglais) :

````diff
diff --git a/README.md b/README.md
index 00e08a62..fa8bf449 100644
--- a/README.md
+++ b/README.md
@@ -20,7 +20,7 @@ comments and tool output are in French.
 | `crates/oku-slides-render-worker` | Typst rendering in a worker process, bounded in time and memory |
 | `crates/oku-slides-chart` | Charts and tables: ECharts option subset, SVG via charts-rs, table model, data operations |
 | `crates/oku-slides-pdf` | Source blob inside the PDF (`/PieceInfo`): write, read, strip |
-| `crates/oku-slides-import` | PPTX import: `SourceDeck` intermediate representation and reader |
+| `crates/oku-slides-import` | PPTX import: `SourceDeck` intermediate representation, reader, retrofit projector, report and content coverage |
 | `crates/oku-slides-session` | Editing session (open deck, history, rendering, file), shared by the app and the dev server |
 | `crates/oku-slides-platform` | Foundation: logging, settings, storage, languages; bounded accept loop for local HTTP servers |
 | `crates/oku-slides-i18n` | Fluent messages (20 languages, English as the reference) and language selection |
@@ -28,7 +28,7 @@ comments and tool output are in French.
 | `crates/oku-slides-a2ui` | A2UI v0.9 surfaces (component catalog), shared by the panel and the agent |
 | `crates/oku-slides-mcp` | Agent MCP tools (local HTTP, token): reading, `apply`, surfaces, confirmations |
 | `crates/oku-slides-agent` | Voice agent: ACP client (agent as a subprocess), turns, stack for the app and the server; fake agent for tests |
-| `crates/oku-slides-cli` | `oku-slides` tool: render, save, reopen, benchmarks, dev server |
+| `crates/oku-slides-cli` | `oku-slides` tool: render, save, reopen, PPTX import, benchmarks, dev server |
 | `src-tauri` | Tauri 2 desktop app |
 | `ui` | React + TypeScript interface (Papier design system) |
 | `assets/fonts` | Bundled fonts (Newsreader, Instrument Sans, Inter; Noto Sans/Serif for non-Latin scripts), in Git LFS |
@@ -46,6 +46,7 @@ just oku-slides render fixtures/roadmap.json --out out --theme pro  # SVG + layo
 just save fixtures/roadmap.json deck.oku-slides.pdf                 # save as .oku-slides.pdf
 just oku-slides open deck.oku-slides.pdf --out extracted/           # deck.json + assets
 just oku-slides strip deck.oku-slides.pdf copy.pdf                  # copy without sources
+just oku-slides import deck.pptx --pack Essentiel --theme Pro -o deck.oku-slides.pdf --report r.json  # PPTX → deck (retrofit)
 just bench                                                          # render and save latencies (release build)
 just oku-slides schema                                              # JSON schema of operations
 ```
@@ -53,6 +54,14 @@ just oku-slides schema                                              # JSON schem
 `--fonts <dir>` adds fonts (TTF/OTF); without it, Typst uses its built-in fonts
 (Libertinus Serif, New Computer Modern, DejaVu Sans Mono).
 
+`import` maps the content of a `.pptx` onto an existing pack (`--pack`: id, pack
+folder or name) and theme (`--theme`: id or name; default: the default theme):
+the style comes from the target theme. Titles are inferred when the slides have no
+placeholders, drawn cards become `card` elements, repeated headers and footers are
+left to the theme. `--report` writes the JSON report: template chosen per slide,
+losses, fallbacks, widget requests, reader warnings and content coverage.
+`--mode` defaults to `retrofit`; `--mode hifi` (faithful copy) is not available yet.
+
 ## Running the app
 
 Everything runs from the repository root with [just](https://just.systems).
diff --git a/crates/oku-slides-core/ARCHITECTURE.md b/crates/oku-slides-core/ARCHITECTURE.md
index ad07c2a5..1da7ed30 100644
--- a/crates/oku-slides-core/ARCHITECTURE.md
+++ b/crates/oku-slides-core/ARCHITECTURE.md
@@ -18,6 +18,11 @@ sources. Aucune dépendance à Typst ni à Tauri.
 - `DeckReplica` (`replica/`) : le deck dans un `LoroDoc`, fusion et historique ; voir [`src/replica/ARCHITECTURE.md`](src/replica/ARCHITECTURE.md).
 - `PackVersion`/`PackRef`/`PackFiles`/`WidgetFile` (`pack.rs`) : version adressée par contenu
   (sha-256 de `pack.json` à clés triées, `0`, puis nom, `0`, source, `0` par fichier de widget).
+  Le contenu de `pack.json` y vit aussi : `PackManifest` (`from_files`, `to_files`, lecture
+  tolérante des `examples`), `Template` (un modèle : sa slide d'exemple) et `now_created` ; la
+  session les ré-exporte, l'import les lit sans dépendre d'elle.
+- `tokens.rs` : jetons effectifs d'un thème (`effective_tokens` : défauts puis thème) et fusion
+  profonde `merge`, partagés par la session (éditeur de thème), la CLI et l'import.
   `ThemeRef` (`Deck.theme`, atome du replica) : une version de thème est un `PackFiles` sans
   widget dont le manifeste est `theme.json` ; `SetTheme` change la seule référence (`null` : thème
   par défaut) ; embarquer la version dans les sources revient à l'hôte. Jetons : `SetToken` pose `null` tel quel (« vider »),
diff --git a/crates/oku-slides-session/ARCHITECTURE.md b/crates/oku-slides-session/ARCHITECTURE.md
index 4111f4a9..8f057ca8 100644
--- a/crates/oku-slides-session/ARCHITECTURE.md
+++ b/crates/oku-slides-session/ARCHITECTURE.md
@@ -18,6 +18,8 @@ L'app Tauri et le serveur de développement (`oku-slides-cli`, `serve`) exposent
   (`SourceBundle.packs[deck.templates.version]`, jamais la bibliothèque ; sonde par fichier, avis
   `session-widgets-broken`) ; absente : base + `session-pack-missing`. `deck_pack.rs` : deck sans
   pack → version courante du pack par défaut ; `SetTemplates` embarque depuis la bibliothèque.
+  `pack.rs` : `PackInfo` (`PackInfo::of`), `Relation`, `DEFAULT_PACK`, `GRID_PACK` ; `Template`,
+  `PackManifest` et `now_created` viennent du cœur (`oku_slides_core::pack`), ré-exportés ici.
   `library.rs` : `Library<K>` versionnée, générique sur la sorte (`library_kind.rs` : `LibraryKind`, `Templates`) ; `<data>/templates/<uuid>/{current, versions/<hex>/pack.json + widgets/}`. `theme.rs` : sorte `Themes` (`<data>/themes/<uuid>/{current, versions/<hex>/theme.json}`, sans widgets), `ThemeManifest`, `ThemeLibrary`, thèmes intégrés Papier et Pro (`PAPIER_THEME`, `PRO_THEME`, `DEFAULT_THEME`) ; `Session.themes` (chemin `paths.themes()`).
 - Thème du deck (`deck_theme.rs`) : la version embarquée (`SourceBundle.themes[deck.theme.version]`)
   fait foi, jamais la bibliothèque ; deck sans thème → version courante du thème par défaut,
diff --git a/docs/superpowers/specs/2026-10-09-import-pptx-design.md b/docs/superpowers/specs/2026-10-09-import-pptx-design.md
index b480001b..be73913d 100644
--- a/docs/superpowers/specs/2026-10-09-import-pptx-design.md
+++ b/docs/superpowers/specs/2026-10-09-import-pptx-design.md
@@ -57,12 +57,29 @@ S'appuie sur `2026-10-01-packs-portables-design.md` (packs embarqués,
    difficile — cascade des styles et géométrie — est isolée dans le
    lecteur ; les projecteurs sont des fonctions pures testées en snapshots.
 
+Décisions du plan du lot 2 (2026-10-10) :
+
+- **Modèles dans le cœur.** `Template` et `PackManifest` sont dans
+  `oku-slides-core` (`pack.rs`) ; la session les ré-exporte. La crate
+  d'import lit les modèles d'un pack sans dépendre de la session.
+- **Pas de pack enfant au lot 2.** Le retrofit émet les `WidgetRequest`
+  (au plan et au rapport) ; le pack enfant est créé au lot 5, avec le
+  premier widget créé par l'agent. `ImportPlan` n'a pas de champ `pack`
+  au lot 2.
+- **Polices au lot 3.** Le retrofit prend le style du thème cible :
+  aucune police n'est cherchée ni substituée. `fonts.rs` (recherche
+  système, substitution) arrive avec le hi-fi.
+- **Couverture du contenu au lot 2.** `coverage.rs` mesure sur le deck
+  obtenu le rappel des runs, la présence des images et l'égalité des
+  données des tableaux et graphiques ; le lot 4 garde la référence
+  LibreOffice et l'écart visuel (`metrics.rs`).
+
 Défauts retenus sans question :
 
 - **Taille de page.** Hi-fi : les tokens `page` du thème généré prennent le
   ratio source (4:3, perso…). Retrofit : la page du thème cible, la
   projection sur la grille absorbe l'écart de ratio.
-- **Polices.** Ordre de recherche : polices embarquées dans le PPTX →
+- **Polices** (hi-fi, lot 3). Ordre de recherche : polices embarquées dans le PPTX →
   polices système → substitut métriquement compatible (Carlito ↔ Calibri,
   Caladea ↔ Cambria, Liberation ↔ Arial/Times/Courier), signalé au rapport.
   La police retenue est embarquée dans `fonts/` du deck.
@@ -96,19 +113,21 @@ Crate `crates/oku-slides-import` :
 | `pptx/eot.rs` | polices embarquées EOT (`.fntdata`) |
 | `pptx/geometry.rs` | lecture des géométries (`prstGeom`, `custGeom`) ; la conversion preset → path appartient au lot 3 |
 | `pptx/fonts.rs` | inventaire des familles utilisées, polices embarquées (lot 1) |
-| `fonts.rs` | recherche système et substitution des polices (lot 2) |
+| `fonts.rs` | recherche système et substitution des polices (lot 3) |
 | `hifi/` | projecteur hi-fi : thème, générateur Typst, pack, ops |
 | `retrofit/` | projecteur retrofit : score des templates, grille, couleurs, replis |
 | `pattern.rs` | signature de motif (partagée retrofit / regroupement hi-fi) |
 | `plan.rs` | `ImportPlan`, `WidgetRequest` |
 | `reference.rs` | appel `soffice`, rasterisation `hayro` |
-| `metrics.rs` | écart visuel (hi-fi), couverture du contenu (retrofit) |
+| `coverage.rs` | couverture du contenu, mesurée sur le deck obtenu (lot 2) |
+| `metrics.rs` | écart visuel (hi-fi, lot 4) |
 | `report.rs` | `ImportReport` |
 
 Dépendances : `zip` (existant), `roxmltree` (nouveau, DOM lecture seule),
 `hayro` (déjà transitif, devient direct pour `reference.rs`). La crate ne
 dépend ni de la session ni de l'UI : elle prend en entrée le registre de
-widgets, le pack et le thème cibles, et rend un plan.
+widgets, les modèles du pack (`oku_slides_core::Template`) et les jetons du
+thème cibles, et rend un plan.
 
 ## Représentation intermédiaire : `SourceDeck`
 
@@ -155,7 +174,7 @@ mods }`, `PlaceholderLink`, styles par niveau typés, avertissements typés.
 ```rust
 ImportPlan {
     theme: Option<Theme>,          // hi-fi : thème généré
-    pack: Option<Pack>,            // hi-fi : pack généré ; retrofit : pack enfant vide si requests
+    pack: Option<Pack>,            // hi-fi : pack généré (lot 3) ; retrofit : aucun au lot 2, pack enfant au lot 5
     assets, fonts,
     ops: Vec<Op>,                  // ops `apply` existantes
     widget_requests: Vec<WidgetRequest>,
@@ -209,17 +228,35 @@ thème cible.
    champs requis remplissables, slots compatibles avec les types de
    contenu, concordance du `type` de layout source (`title`, `secHead`,
    `obj`, `twoObj`, `titleOnly`, `blank`…). Égalité → ordre du pack.
-2. **Disposition.** Boîtes source projetées sur la grille 12×6 →
-   cellules, ou layout `columns`/`rows` de `content`.
+   Avant le choix (lot 2) : le décor répété (en-tête, pied, logo dans les
+   bandes haute et basse de la page, sur au moins la moitié des slides) et
+   les placeholders date, pied, numéro sont omis, dits une fois au rapport ;
+   sans placeholder de titre, titre, surtitre et sous-titre se déduisent de
+   la taille et de la position des textes, et une couverture d'un titre
+   d'au moins 40 pt ; une forme remplie qui porte des textes devient une
+   carte (titre, surtitre, corps), et les textes voisins dont les cellules
+   se chevauchent fusionnent. Une rangée de cartes fait des colonnes.
+2. **Disposition.** Une slide « titre + un corps » ou une rangée de
+   colonnes va en blocs `flow` (layout `columns`/`split` du modèle ; une
+   rangée entre des textes pleine largeur devient une pile en colonnes).
+   Sinon la boîte englobante des contenus est ajustée dans la zone de corps
+   (échelle uniforme, centrée) : cadres sur un modèle libre, cellules de la
+   grille du modèle si elles ne se recouvrent pas, sinon cadres.
 3. **Couleurs.** `Scheme(accentN)` → table fixe vers les tokens cibles ;
-   `Explicit` → token le plus proche en **ΔE OKLab** sous un seuil, sinon
-   abandonnée. Gras et italique conservés.
+   les couleurs saturées du deck sans jeton proche, rangées par usage,
+   prennent `accent` puis les autres `accent-*` du thème ; `Explicit` →
+   token le plus proche en **ΔE OKLab** sous un seuil, sinon abandonnée.
+   Une couleur de texte n'est émise que lisible (contraste WCAG ≥ 3:1 sur
+   son fond, page ou carte ; sur fond sombre, un jeton de texte clair du
+   thème). Une carte sombre devient une carte en relief. Gras et italique
+   conservés ; les capitales sont écrites en majuscules.
 4. **Motifs sans widget.** Le repli (p. ex. `stack` de `card` pour un
    SmartArt) est appliqué tout de suite : le deck est **complet et
-   rendable**. Si le repli perd de l'information, une `WidgetRequest` est
+   rendable**. Si le repli perd de l'information (formes omises, motif
+   régulier d'au moins trois boîtes, SmartArt), une `WidgetRequest` est
    émise, groupée par **signature de motif** (multiset des types d'items +
    disposition relative normalisée, `pattern.rs`). L'agent la traite au
-   fine-tuning (décision 5).
+   fine-tuning (décision 5), dans un pack enfant créé au lot 5.
 
 ## Rendu de référence et métriques
 
@@ -262,9 +299,11 @@ référence disponible ou non, `widget_requests`. Messages via Fluent
 ## Points d'entrée
 
 - **CLI** (`oku-slides-cli`), purement déterministe :
-  `oku-slides import deck.pptx --mode hifi|retrofit [--pack <id|chemin>]
-  [--theme <id>] [--reference auto|none] -o out.oku-slides.pdf
-  [--report r.json]`. `--pack` est requis en retrofit.
+  `oku-slides import deck.pptx --mode hifi|retrofit [--pack <id|chemin|nom>]
+  [--theme <id|nom>] [--reference auto|none] -o out.oku-slides.pdf
+  [--report r.json]`. `--pack` est requis en retrofit. Au lot 2 : `--mode`
+  vaut `retrofit` par défaut, `hifi` est refusé avec un message localisé
+  (lot 3) ; `--reference` arrive au lot 4.
 - **App** : « Ouvrir » accepte `.pptx` ; un dialogue demande le mode, et en
   retrofit le pack et le thème cibles. L'app crée un deck neuf à partir du
   plan, puis lance l'agent avec les consignes de fine-tuning.
@@ -311,11 +350,13 @@ Un plan et ses sous-tâches par lot, dans cet ordre :
 
 1. *(livré, t-692)* `SourceDeck` + lecteur PPTX (cascade des styles comprise) — plan :
    `docs/superpowers/plans/2026-10-09-import-pptx-lot1.md`.
-2. Projecteur retrofit + CLI — valide la représentation de bout en bout
-   sans générateur ni référence.
-3. Projecteur hi-fi : thème, générateur Typst, pack.
-4. Rendu de référence + métriques.
-5. App + fine-tuning par agent (outils MCP, consignes).
+2. *(livré, t-692)* Projecteur retrofit + CLI + couverture du contenu —
+   valide la représentation de bout en bout sans générateur ni référence ;
+   plan : `docs/superpowers/plans/2026-10-10-import-pptx-lot2.md`.
+3. Projecteur hi-fi : thème, générateur Typst, pack, polices (`fonts.rs`).
+4. Rendu de référence + écart visuel.
+5. App + fine-tuning par agent (outils MCP, consignes, pack enfant des
+   widgets créés pour les `WidgetRequest`).
 6. *(Phase 2, spec séparée)* Lecteur PDF → `SourceDeck` par
    `hayro-interpret`, avec inférence des indices sémantiques (rôles,
    master par éléments répétés, paragraphes).
````

> Le chemin du plan cité par la spec (`docs/superpowers/plans/2026-10-10-import-pptx-lot2.md`) est celui où ce plan est enregistré ; l'ajuster s'il est rangé ailleurs. La ligne de `crates/oku-slides-session/ARCHITECTURE.md` s'insère dans le paragraphe des packs (`pack.rs`), quel que soit le texte qui l'entoure sur la branche principale.

- [ ] **Step 3 : essai manuel**

Données de l'app isolées dans un dossier temporaire (le vrai dossier de l'utilisateur n'est pas touché) :

```bash
tmp="$(mktemp -d "${TMPDIR:-/tmp}/oku-import.XXXXXX")"
export OKU_SLIDES_PATHS__DATA="$tmp/data" OKU_SLIDES_PATHS__DECKS="$tmp/decks"
just oku-slides import fixtures/import/alto-wealth.pptx --mode retrofit --pack Essentiel -o "$tmp/alto.oku-slides.pdf" --report "$tmp/alto-report.json"
just oku-slides open "$tmp/alto.oku-slides.pdf"
pdftoppm -r 60 -png -f 1 -l 8 "$tmp/alto.oku-slides.pdf" "$tmp/page"   # si poppler est installé : regarder les pages
```

Relevé au brouillon (locale `fr`, `--fonts assets/fonts`, thème par défaut Papier) :

```text
$tmp/alto.oku-slides.pdf : 8 slides, 8 pages, 17 pertes, 0 demande de widget
$tmp/alto.oku-slides.pdf : 8 slides, 1 assets, 0 polices

alto-report.json (résumé) :
  slides      s1 libre (grid), s2 cartes (flow), s3 libre (grid), s4 cartes, s5 colonnes, s6 à s8 cartes (flow)
  pertes      repeated_decor ×3 (« CONFIDENTIAL » 8 slides, logo « Amundi » 7, pied « 1 · Wealth Bank as an API · … » 6) ;
              color_dropped #00bef4 ×8, #7c93b8 ×3, #8fa3c2 ×3
  coverage    runs 127/127, images 4/4, tables 0/0, charts 0/0, missing []
  warnings    0 ; widget_requests 0
  exemple     {"item": "s2/19", "kind": {"type": "repeated_decor", "detail": {"text": "Amundi", "slides": 7}},
               "message": "En-tête ou pied « Amundi » répété sur 7 slides omis : le thème dessine les siens"}
```

> Le deck d'Alto n'a aucun placeholder (PptxGenJS) : titres et surtitres sont déduits, le titre de la couverture remplit le deck ; les slides de cartes prennent `cartes` ou `colonnes`, la couverture et la slide 3 (deux rangées de cartes reliées par des flèches) `libre`, en cadres. Pied, mention « CONFIDENTIAL » et logo répétés sont omis, dits une fois chacun ; le thème dessine les siens. Les couleurs perdues sont justifiées : le cyan de la marque (`#00bef4`) reste sans jeton dans Papier (ses deux clés d'accent vont aux deux bleus les plus employés), deux gris bleutés (`#7c93b8`, `#8fa3c2`) sont à plus de 12 de tout jeton. Rien ne manque à la couverture ; aucun repli de groupe, donc aucune demande de widget. Avec `--theme Pro` (la palette d'Alto) : les trois pertes de décor seules.

- [ ] **Step 4 : vérification de l'espace de travail**

```bash
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run -p oku-slides-core -p oku-slides-session -p oku-slides-import -p oku-slides-cli -p oku-slides-i18n
```

Attendu : tout passe (3031 tests (core, session, import, cli, i18n) au brouillon).

- [ ] **Step 5 : commit, sous-tâche**

```bash
git add crates/oku-slides-import/ARCHITECTURE.md crates/oku-slides-core/ARCHITECTURE.md crates/oku-slides-session/ARCHITECTURE.md docs/superpowers/specs/2026-10-09-import-pptx-design.md README.md && git diff --cached --name-only
git commit -F <scratchpad>/msg.txt   # docs(import) : documents du lot 2, architecture et spec du retrofit (t-692)
oku cli task done --project slider --id t-808
```

## Hors de ce lot

Projecteur hi-fi, générateur Typst et polices (lot 3) ; rendu de référence LibreOffice et écart visuel (lot 4) ; app, pack enfant et fine-tuning par agent qui traite les `WidgetRequest` (lot 5) ; lecteur PDF (lot 6). Limite connue (t-816) : en flux, un contenu plus haut que sa zone déborde de la page sans que l'import le voie. Cela arrive avec un thème cible au corps plus grand (Papier, slides 6 et 7 d'alto), mais aussi avec le thème par défaut quand un graphique ou une image remplit une colonne en flux `columns` (kitchen, slide 2, sur Essentiel et Pro : « Gauche » et « Droite » sont coupés sous le graphique). La couverture du contenu mesure le modèle de deck, pas le rendu : elle ne le voit pas. Backlog lié : t-752 à t-755, t-757.
