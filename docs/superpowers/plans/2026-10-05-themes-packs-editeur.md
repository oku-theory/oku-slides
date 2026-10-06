# Thèmes en packs versionnés et éditeur de jetons — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `papier` et `pro` deviennent des thèmes intégrés d'une bibliothèque versionnée embarquée par le deck, et la perspective modèles gagne un onglet Thème qui édite tous les jetons de design.

**Architecture:** Les jetons (défauts et thèmes intégrés) passent en JSON, seule source de vérité lue par Typst et Rust. `Library` devient générique sur une sorte (modèles / thèmes) ; le deck référence un `ThemeRef { id, version }` embarqué dans le zip (`themes/<hex>/theme.json`) et transmis au rendu par `Renderer::set_theme`. En mode modèles, le deck de travail porte le brouillon complet des jetons du thème édité ; une surface A2UI `host-theme` (construite en Rust, rendue par `HostSettings`) l'édite par `set_token` / `unset_token`, et `save_templates` en fait une version de thème.

**Tech Stack:** Rust (snafu, serde_json, uuid, loro, typst), React + TypeScript (Bun test, e2e Playwright `ui/e2e/*.cjs`), Fluent (20 langues).

**Spec:** `docs/superpowers/specs/2026-10-05-themes-packs-editeur-design.md`

## Global Constraints

- Tâche Oku : **t-554** ; chaque commit finit par `(t-554)` ; type conventionnel (`feat`, `fix`, `refactor`, `test`, `docs`), en français comme l'historique (`feat(session) : …`).
- Pas de ligne `Co-Authored-By` dans les commits (consigne utilisateur).
- `git add` dans un appel Bash séparé du `git commit` ; `git diff --cached --stat` juste avant le commit.
- Worktree : `.claude/worktrees/themes-editeur`, branche `554-feature/themes-editeur`, `oku.review=deferred` (portée worktree) déjà posé.
- Toute exécution de l'app, du serveur ou de `just e2e` : `OKU_SLIDES_PATHS__DATA=<tmp>` et `OKU_SLIDES_PATHS__DECKS=<tmp>` dans le scratchpad, jamais les vraies données.
- Pas de rétrocompatibilité : `DECK_VERSION` 3 → 4, `SOURCES_VERSION` 5 → 6.
- Pas de `.unwrap()` / `.expect()` hors tests ; erreurs Snafu avec `backtrace`, messages en minuscules, traduits via `Localize`.
- Fichiers ≤ 500 lignes visés, 1000 max ; tests côte à côte `foo_tests.rs` (`#[cfg(test)] #[path = "foo_tests.rs"] mod foo_tests;`) ; Speculoos (`assert_that!`).
- Régressions dans `tests/regressions/issue_0554_<sujet>.rs` du crate concerné.
- Tout texte visible passe par Fluent ; en et fr à jour à chaque tâche (`ui/src/i18n-keys.test.ts` l'impose pour `ui.ftl`) ; les 18 autres langues à la tâche 13.
- UUID constants : `PAPIER_THEME = 0x2a7d4c91_5e3b_4f08_b6a1_8c9e0d1f2a31`, `PRO_THEME = 0x7e1b9c3d_2f4a_4d6e_a8b0_5c7d9e1f3b42`.
- Vérification par tâche : `cargo nextest run -p <crates touchés>` ; UI : `cd ui && bun test` et `bun run typecheck` (voir `ui/package.json`) ; fin : `just check`.

---

## Phase 1 — Thèmes en packs

### Task 1: Jetons et thèmes intégrés en JSON

**Files:**
- Create: `typst/oku-slides/src/default-tokens.json`
- Create: `typst/oku-slides/themes/papier.json`, `typst/oku-slides/themes/pro.json`
- Delete: `typst/oku-slides/themes/papier.typ`, `typst/oku-slides/themes/pro.typ`
- Modify: `typst/oku-slides/src/tokens.typ` (`default-tokens`, l. 11), `typst/oku-slides/lib.typ:7-8`
- Modify: `crates/oku-slides-render/src/world.rs:33-48` (`LIBRARY`), `typst/oku-slides/README.md` (sections « Jetons », « Thèmes fournis »)
- Test: snapshots existants `crates/oku-slides-render/tests/snapshots/golden__reference_pages.snap`

**Interfaces:**
- Produces: `default-tokens.json` (objet des jetons par défaut) ; `themes/<nom>.json` de forme `{ id, name, description, parent: null, created, tokens }` avec les UUID constants des Global Constraints ; `lib.typ` exporte toujours `papier` et `pro` (dictionnaires de jetons).

- [ ] **Step 1: Convertir les dictionnaires**

Traduire mécaniquement `default-tokens` (`tokens.typ`) et les deux thèmes en JSON : `none` → `null`, tuples `("Inter",)` → `["Inter"]`, clés à tirets inchangées, ordre des clés conservé. Garder les commentaires utiles dans `tokens.typ` / README (le JSON n'en a pas). Manifestes :

```json
{ "id": "2a7d4c91-5e3b-4f08-b6a1-8c9e0d1f2a31", "name": "Papier",
  "description": "", "parent": null, "created": "2026-10-05T00:00:00Z",
  "tokens": { "color": { "bg": "#fffdf9", … } } }
```

(`pro` : id `7e1b9c3d-2f4a-4d6e-a8b0-5c7d9e1f3b42`, nom « Pro ».)

- [ ] **Step 2: Lire le JSON côté Typst**

```typ
// tokens.typ
#let default-tokens = json("default-tokens.json")
// lib.typ
#let papier = json("themes/papier.json").tokens
#let pro = json("themes/pro.json").tokens
```

Typst lit un `null` JSON comme `none` et un tableau comme un tableau : la fusion `merge` (`tokens.typ:152`) donne le même résultat.

- [ ] **Step 3: Embarquer les fichiers**

Dans `world.rs`, `LIBRARY` liste `"src/default-tokens.json"`, `"themes/papier.json"`, `"themes/pro.json"` au lieu des `.typ`. Vérifier comment le monde sert un fichier non-`.typ` (`source_of` ne couvre que les `Source`) : si `json()` passe par `file()`, servir ces trois chemins comme `Bytes` (ajouter une table `LIBRARY_DATA: &[(&str, &[u8])]` avec `include_bytes!`, et la consulter dans `OkuSlidesWorld::file`).

- [ ] **Step 4: Vérifier que le rendu est identique**

Run: `cargo nextest run -p oku-slides-render`
Expected: PASS, aucun snapshot modifié (`git status` ne montre aucun `.snap.new`). Un écart = conversion fautive : comparer clé par clé.

- [ ] **Step 5: Commit**

```bash
git add typst/oku-slides crates/oku-slides-render/src/world.rs
git commit -m "refactor(typst) : jetons par défaut et thèmes intégrés en JSON (t-554)"
```

### Task 2: Le thème transmis au rendu (`Renderer::set_theme`)

**Files:**
- Modify: `crates/oku-slides-render/src/world.rs` (`main_source`, `OkuSlidesWorld`, `set_deck`, `redraw_charts`, `refresh_main`)
- Modify: `crates/oku-slides-render/src/lib.rs` (`Renderer::set_theme`, doc du module ; `THEMES` reste jusqu'à la tâche 7)
- Modify: `crates/oku-slides-render/src/charts.rs` (supprimer `body_font`, police lue dans les jetons résolus)
- Modify: `crates/oku-slides-session/src/lib.rs` et les autres appelants du renderer (`widget_preview.rs`, `deck_widgets.rs`, `help.rs`) : pont provisoire
- Create: `crates/oku-slides-session/src/theme_bridge.rs` (provisoire, supprimé tâche 7)
- Test: `crates/oku-slides-render/src/world_tests.rs`, `crates/oku-slides-render/tests/regressions/issue_0554_police_graphiques_du_theme.rs` (+ `mod` dans `tests/regressions/main.rs`)

**Interfaces:**
- Consumes: JSON de la tâche 1.
- Produces: `Renderer::set_theme(&mut self, tokens: Option<serde_json::Map<String, Value>>) -> Result<(), RenderError>` ; le monde sert `/theme.json` ; `main_source` écrit `theme: json("/theme.json")` si un thème est posé, sinon `theme: none`. `deck.theme` n'est plus lu par le rendu. Pont provisoire `theme_bridge::builtin_tokens(name: Option<&str>) -> Option<Map>` (lit les JSON intégrés par `include_str!`).

- [ ] **Step 1: Tests qui échouent**

```rust
// world_tests.rs
#[test]
fn main_source_with_theme_reads_theme_json() {
    let src = main_source(true, None, std::iter::empty(), &Labels::default());
    assert_that!(src.contains(r#"theme: json("/theme.json")"#)).is_true();
}

#[test]
fn main_source_without_theme_passes_none() {
    let src = main_source(false, None, std::iter::empty(), &Labels::default());
    assert_that!(src.contains("theme: none")).is_true();
}
```

```rust
// tests/regressions/issue_0554_police_graphiques_du_theme.rs
// Un thème qui change font.body change la police des graphiques (avant : nom du thème codé en dur).
#[test]
fn chart_font_follows_theme_font_body() {
    let mut r = test_renderer();                 // helper existant des régressions
    let deck = deck_with_one_chart();            // idem (voir issue_0544_*.rs)
    r.set_theme(Some(json!({"font": {"body": ["Inter"]}}).as_object().cloned().unwrap())).unwrap();
    let svg = r.render(&deck, &BTreeMap::new()).unwrap().svg(0).unwrap();
    assert_that!(svg.contains("Inter")).is_true();
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo nextest run -p oku-slides-render main_source chart_font_follows`
Expected: FAIL (signature de `main_source`, `set_theme` absent).

- [ ] **Step 3: Implémenter**

- `OkuSlidesWorld` : champ `theme: Option<Bytes>` (remplace `theme: Option<String>`), `theme_id: FileId` (`/theme.json`) servi par `file()`. `set_theme(tokens)` sérialise la map, puis `redraw_charts`.
- `main_source(has_theme: bool, …)`.
- `charts.rs` : `Charts` garde `font: Option<String>` posé par `set_font`, comme `set_palette` ; `Renderer::render` fait déjà un second passage pour la palette (`set_chart_palette` depuis `Layout.tokens`) : y ajouter la première famille de `font.body` des jetons résolus (`layout.tokens["font"]["body"][0]`), défaut `"Libertinus Serif"`. `attach` ne reçoit plus le thème.
- Session : partout où `deck.theme` (nom) arrive au renderer, appeler `renderer.set_theme(theme_bridge::builtin_tokens(deck.theme.as_deref()))` quand le nom change (comparer au dernier posé). Le pont porte en tête `// MVP: t-554 — supprimé à la tâche 7`.

- [ ] **Step 4: Vérifier**

Run: `cargo nextest run -p oku-slides-render -p oku-slides-session`
Expected: PASS, snapshots golden inchangés.

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-render crates/oku-slides-session
git commit -m "feat(render) : le thème arrive au rendu en jetons (set_theme), police des graphiques lue dans font.body (t-554)"
```

### Task 3: `SetTheme` dans le protocole du worker de rendu

**Files:**
- Modify: `crates/oku-slides-render-worker/src/protocol.rs` (`Request::SetTheme { slot, tokens }`), `serve.rs`, `slot.rs`, `client.rs`
- Test: `crates/oku-slides-render-worker/src/serve_tests.rs`, `client_tests.rs`

**Interfaces:**
- Consumes: `Renderer::set_theme` (tâche 2).
- Produces: `Request::SetTheme { slot: SlotId, tokens: Option<serde_json::Map<String, Value>> }` ; côté client, la même méthode `set_theme(slot, tokens)` que celle utilisée par la session pour `SetWidgets` (suivre le chemin de `SetWidgets` de bout en bout).

- [ ] **Step 1: Test qui échoue** — dans `serve_tests.rs`, sur le modèle du test de `SetWidgets` : ouvrir un slot, envoyer `SetTheme` avec `{"color": {"bg": "#000000"}}`, rendre un deck d'une slide, vérifier que le SVG contient `#000000` (fond).
- [ ] **Step 2: Run** `cargo nextest run -p oku-slides-render-worker set_theme` → FAIL (variante inconnue).
- [ ] **Step 3: Implémenter** la variante, le traitement dans le slot (`renderer.set_theme(tokens)`), la méthode du client ; la session (là où elle appelle déjà `set_widgets` sur le client) appelle `set_theme` au lieu du pont direct quand elle passe par le worker.
- [ ] **Step 4: Run** `cargo nextest run -p oku-slides-render-worker -p oku-slides-session` → PASS.
- [ ] **Step 5: Commit** `feat(render-worker) : requête SetTheme (t-554)`.

### Task 4: `ThemeRef`, ops de jetons et thèmes dans le zip (core)

**Files:**
- Modify: `crates/oku-slides-core/src/pack.rs` (`ThemeRef`, à côté de `PackRef`)
- Modify: `crates/oku-slides-core/src/model.rs` (`Deck.theme: Option<ThemeRef>`, `DECK_VERSION = 4`)
- Modify: `crates/oku-slides-core/src/ops/mod.rs` (`SetTheme { theme: Option<ThemeRef> }`, `SetToken` garde `null`, `UnsetToken { path }`, `SetTokens { tokens: Value }`)
- Modify: `crates/oku-slides-core/src/replica/schema.rs:63` (`(Deck, "theme") => Kind::Atom`)
- Modify: `crates/oku-slides-core/src/sources.rs` (`themes`, `THEMES_DIR = "themes/"`, `THEME_JSON = "theme.json"`, `SOURCES_VERSION = 6`, lecture/écriture/merge/garde par historique, `theme_issues`)
- Modify: `crates/oku-slides-core/src/validate.rs` (`Issue::ThemeVersionMissing`, `Issue::ThemeVersionMismatch`), `crates/oku-slides-core/src/lib.rs` (ré-exports explicites), `crates/oku-slides-core/ARCHITECTURE.md`
- Modify: appelants qui écrivent `deck.theme` (session, cli, mcp) : compiler en posant `None` / en passant par la tâche 7 — ici seulement l'adaptation de type minimale
- Test: `pack_tests.rs`, `ops/*_tests.rs` (fichier de tests des ops existant), `sources_tests.rs`, `replica/*_tests.rs`, `model_tests.rs`

**Interfaces:**
- Produces:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct ThemeRef {
    #[schemars(with = "String")]
    pub id: Uuid,
    pub version: PackVersion,
}
pub struct SourceBundle { …, pub themes: BTreeMap<PackVersion, PackFiles> }
impl SourceBundle { pub fn add_theme(&mut self, files: PackFiles) -> PackVersion; }
// Op
SetTheme { theme: Option<ThemeRef> },
SetToken { path: Vec<String>, value: Value },   // null posé tel quel
UnsetToken { path: Vec<String> },               // retire la clé ; objet vide → retiré
SetTokens { tokens: Value },                    // remplace tout deck.tokens (objet ou null)
```

- Une version de thème est un `PackFiles` sans widget dont le manifeste est `theme.json`.

- [ ] **Step 1: Tests qui échouent**

```rust
// ops tests
#[test]
fn set_token_null_keeps_explicit_null() {
    let deck = apply_all(deck(), [Op::SetToken { path: p(&["component", "header", "left"]), value: Value::Null }]).unwrap();
    assert_that!(deck.tokens.pointer("/component/header/left")).is_equal_to(Some(&Value::Null));
}

#[test]
fn unset_token_removes_key_and_empty_parents() {
    let d = apply_all(deck(), [Op::SetToken { path: p(&["color", "accent"]), value: json!("#ff0000") }]).unwrap();
    let d = apply_all(d, [Op::UnsetToken { path: p(&["color", "accent"]) }]).unwrap();
    assert_that!(d.tokens).is_equal_to(Value::Null);
}

#[test]
fn set_tokens_replaces_all_tokens() {
    let d = apply_all(deck(), [Op::SetTokens { tokens: json!({"size": {"body": 30}}) }]).unwrap();
    assert_that!(d.tokens).is_equal_to(json!({"size": {"body": 30}}));
}
```

```rust
// sources_tests.rs
#[test]
fn theme_versions_round_trip_through_zip() {
    let mut b = bundle();
    let files = theme_files("Mon thème");                 // PackFiles::new(manifest, BTreeMap::new())
    let v = b.add_theme(files.clone());
    set_deck_theme(&mut b, ThemeRef { id: ID, version: v.clone() });
    let read = SourceBundle::from_zip(&b.to_zip().unwrap(), limits(), registry()).unwrap().0;
    assert_that!(read.themes.get(&v)).is_equal_to(Some(&files));
}

#[test]
fn theme_with_wrong_hash_is_dropped_with_issue() { /* écrire themes/<autre hex>/theme.json à la main, lire, attendre Issue::ThemeVersionMismatch et themes vide */ }

#[test]
fn invalid_theme_path_refuses_zip() { /* themes/abc/theme.json → Err(BadName) ; themes/<hex>/x/theme.json → Err */ }

#[test]
fn missing_deck_theme_version_gives_issue() { /* deck.theme référence une version absente → Issue::ThemeVersionMissing */ }
```

```rust
// replica tests
#[test]
fn theme_ref_is_an_atom_and_tokens_keep_null() {
    let mut d = deck();
    d.theme = Some(ThemeRef { id: ID, version: v() });
    d.tokens = json!({"component": {"header": {"left": null}}});
    let r = DeckReplica::from_deck(&d, registry()).unwrap();
    assert_that!(r.deck().0).is_equal_to(d);
}
```

- [ ] **Step 2: Run** `cargo nextest run -p oku-slides-core` → FAIL.
- [ ] **Step 3: Implémenter** en suivant les packs ligne à ligne (`pack_entry` → `theme_entry`, `pack_issues` → `theme_issues`, garde des versions référencées par l'historique, `merge`). Si le replica perd un `null` de `tokens` (LWW `null` relu comme absent), le corriger dans `replica/read.rs` / `write.rs` et le couvrir par le test ci-dessus. Docs du module `sources.rs` : version 6, dossier `themes/`.
- [ ] **Step 4: Run** `cargo nextest run --workspace` → PASS (les autres crates compilent avec l'adaptation minimale de type).
- [ ] **Step 5: Commit** `feat(core)! : ThemeRef, ops set_token/unset_token/set_tokens, thèmes embarqués dans les sources v6 (t-554)`.

### Task 5: `Library` générique sur la sorte (modèles inchangés)

**Files:**
- Modify: `crates/oku-slides-session/src/library.rs` (générique), `crates/oku-slides-session/src/pack.rs` (`impl VersionManifest for PackManifest`)
- Create: `crates/oku-slides-session/src/library_kind.rs` (trait `LibraryKind`, trait `VersionManifest`, `Templates`)
- Modify: tous les usages de `Library` (`lib.rs`, `deck_pack.rs`, `templates_mode.rs`, cli `main.rs`, tests, régressions) → `PackLibrary`
- Test: `library_tests.rs` (inchangés hormis le nom de type)

**Interfaces:**
- Produces:

```rust
pub trait VersionManifest: Sized + Clone + std::fmt::Debug {
    fn from_files(files: &PackFiles) -> Result<Self, serde_json::Error>;
    fn id(&self) -> Uuid;
    fn name(&self) -> &str;
    fn parent(&self) -> Option<&PackVersion>;
    fn created(&self) -> &str;
}
pub trait LibraryKind: std::fmt::Debug + Clone + 'static {
    /// Dossier sous `<data>` et nom du manifeste dans une version.
    const DIR: &'static str;
    const MANIFEST_FILE: &'static str;
    /// Les sortes à fichiers de widgets les lisent (`widgets/<nom>.typ`).
    const HAS_WIDGETS: bool;
    type Manifest: VersionManifest;
    fn builtins() -> &'static [StoredVersion<Self::Manifest>];
}
pub struct StoredVersion<M> { pub version: PackVersion, pub files: PackFiles, pub manifest: M }
pub struct Library<K: LibraryKind> { dir: Option<PathBuf>, storage: Storage, kind: PhantomData<K> }
pub type PackLibrary = Library<Templates>;
```

Les méthodes publiques gardent leurs noms et signatures (`versions`, `version`, `current`, `current_version`, `add_version`, `set_current`, `replace_version`, `relation`, `delete`, `is_builtin`) ; `packs()` devient `entries() -> Vec<StoredVersion<K::Manifest>>` (courantes, intégrées d'abord puis par nom) et `PackLibrary::packs()` reste une fonction de commodité qui mappe vers `PackInfo`. Erreurs : `NoTemplatesDir` devient `NoLibraryDir { kind: &'static str }`.

- [ ] **Step 1:** Renommer et paramétrer sans changer le comportement ; les tests de `library_tests.rs` restent la référence.
- [ ] **Step 2: Run** `cargo nextest run -p oku-slides-session -p oku-slides-cli` → PASS (aucun test modifié hormis les noms de types).
- [ ] **Step 3: Commit** `refactor(session) : bibliothèque générique sur la sorte de version (t-554)`.

### Task 6: Bibliothèque de thèmes et thèmes intégrés

**Files:**
- Create: `crates/oku-slides-session/src/theme.rs` (`ThemeManifest`, `ThemeInfo`, `Themes` kind, `PAPIER_THEME`, `PRO_THEME`, `DEFAULT_THEME = PAPIER_THEME`), `theme_tests.rs`
- Modify: `crates/oku-slides-session/src/lib.rs` (modules, ré-exports, `Session.themes: ThemeLibrary` construit avec `settings.paths.themes()`)
- Modify: `crates/oku-slides-platform` (chemin `paths.themes()` = `<data>/themes`, à côté de `templates()`)
- Modify: `crates/oku-slides-session/ARCHITECTURE.md`

**Interfaces:**
- Consumes: `LibraryKind`, `VersionManifest`, `Library<K>` (tâche 5) ; JSON intégrés (tâche 1) via `include_str!("../../../typst/oku-slides/themes/papier.json")`.
- Produces:

```rust
pub const PAPIER_THEME: Uuid = Uuid::from_u128(0x2a7d4c91_5e3b_4f08_b6a1_8c9e0d1f2a31);
pub const PRO_THEME: Uuid = Uuid::from_u128(0x7e1b9c3d_2f4a_4d6e_a8b0_5c7d9e1f3b42);
pub const DEFAULT_THEME: Uuid = PAPIER_THEME;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeManifest {
    pub id: Uuid, pub name: String, #[serde(default)] pub description: String,
    #[serde(default)] pub parent: Option<PackVersion>, pub created: String,
    pub tokens: serde_json::Map<String, Value>,
}
impl ThemeManifest { pub fn to_files(&self) -> Result<PackFiles, serde_json::Error>; }
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ThemeInfo { pub id: Uuid, pub version: PackVersion, pub name: String, pub builtin: bool, pub default: bool }
pub type ThemeLibrary = Library<Themes>;
impl ThemeLibrary { pub fn themes(&self) -> Vec<ThemeInfo>; }
```

- [ ] **Step 1: Tests qui échouent**

```rust
#[test]
fn builtin_themes_have_constant_ids_in_order() {
    let ids: Vec<Uuid> = Themes::builtins().iter().map(|b| b.manifest.id).collect();
    assert_that!(ids).is_equal_to(vec![PAPIER_THEME, PRO_THEME]);
}

#[test]
fn builtin_theme_is_current_and_listed_first() {
    let (lib, _dir) = temp_theme_library();
    let names: Vec<String> = lib.themes().into_iter().map(|t| t.name).collect();
    assert_that!(names).is_equal_to(vec!["Papier".to_owned(), "Pro".to_owned()]);
}

#[test]
fn user_theme_version_is_stored_under_themes_dir() {
    let (lib, dir) = temp_theme_library();
    let m = ThemeManifest { id: Uuid::new_v4(), name: "Mien".into(), description: String::new(), parent: None,
        created: "2026-10-05T10:00:00Z".into(), tokens: json!({"color": {"accent": "#ff0000"}}).as_object().cloned().unwrap() };
    lib.add_version(m.id, &m.to_files().unwrap()).unwrap();
    assert_that!(dir.path().join(m.id.to_string()).join("versions").exists()).is_true();
    assert_that!(lib.current(m.id).is_some()).is_true();
}
```

- [ ] **Step 2: Run** `cargo nextest run -p oku-slides-session theme_tests` → FAIL.
- [ ] **Step 3: Implémenter** `Themes` (`DIR = "themes"`, `MANIFEST_FILE = "theme.json"`, `HAS_WIDGETS = false`).
- [ ] **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat(session) : bibliothèque de thèmes versionnée, Papier et Pro intégrés (t-554)`.

### Task 7: Le deck embarque son thème (session, CLI, MCP, aide)

**Files:**
- Create: `crates/oku-slides-session/src/deck_theme.rs` (+ `deck_theme_tests.rs`) : `adopt_default_theme`, `Session::deck_theme`, `embed_theme_from_library`, `Session::set_theme(id)`, `Session::theme_tokens()`
- Delete: `crates/oku-slides-session/src/theme_bridge.rs` ; `oku_slides_render::THEMES`
- Modify: `crates/oku-slides-session/src/lib.rs` (deck neuf l. 264 : plus de `theme: Some("papier")`, adoption à côté de `adopt_default_pack` l. 301 et 638 ; `set_theme` au renderer sur changement de version), `views.rs:56-57,238` (`theme: Option<ThemeRef>`, `themes: Vec<ThemeInfo>` ; `TabInfo.theme: Option<Uuid>`), `request.rs` (requête `set_theme { id }`), `help.rs` (thème de l'aide par id, version courante de la bibliothèque, embarquée dans le bundle de l'aide), `workspace_help.rs`, `workspace.rs`, `resume.rs` (`help_theme: Option<Uuid>`), `agent/context.rs:98` (nom du thème), `error.rs` (`UnknownTheme { id }`, `ThemeVersionMissing`, `ThemeIdMismatch`, `ThemeJson`), locales `session.ftl` en/fr
- Modify: `crates/oku-slides-cli/src/main.rs` (`--theme <nom>` : thème intégré ou de la bibliothèque dont le nom correspond, insensible à la casse ; embarqué comme le pack), `crates/oku-slides-cli/src/pack_render.rs`, `cli.ftl` en/fr
- Modify: `crates/oku-slides-mcp/src/read_tools.rs:34` (`theme` : `{ id, name }`, `themes` : liste), schéma des ops (généré)
- Test: `deck_theme_tests.rs`, `crates/oku-slides-session/tests/session.rs`, `help_tests.rs`, CLI tests existants

**Interfaces:**
- Consumes: `ThemeLibrary`, `ThemeManifest`, `DEFAULT_THEME` (tâche 6) ; `ThemeRef`, `SourceBundle::add_theme`, `Op::SetTheme` (tâche 4) ; `Renderer::set_theme` (tâches 2-3).
- Produces:

```rust
pub fn adopt_default_theme(themes: &ThemeLibrary, sources: &mut SourceBundle) -> Result<()>;
impl Session {
    pub(crate) fn deck_theme(&self) -> Option<&ThemeRef>;        // du deck de l'utilisateur
    pub(crate) fn embed_theme_from_library(&mut self, r: &ThemeRef) -> Result<()>;
    pub fn set_theme(&mut self, id: Uuid) -> Result<()>;          // embarque la courante + SetTheme, annulable
    pub(crate) fn theme_manifest(&self, r: &ThemeRef) -> Option<ThemeManifest>; // version embarquée
}
// requête UI
{ cmd: "set_theme", id: string }
```

- [ ] **Step 1: Tests qui échouent**

```rust
#[test]
fn new_deck_adopts_current_default_theme() {
    let s = test_session();
    let r = s.deck().theme.clone().unwrap();
    assert_that!(r.id).is_equal_to(DEFAULT_THEME);
    assert_that!(s.doc.sources.themes.contains_key(&r.version)).is_true();
}

#[test]
fn set_theme_embeds_and_is_undoable() {
    let mut s = test_session();
    s.set_theme(PRO_THEME).unwrap();
    assert_that!(s.deck().theme.as_ref().map(|t| t.id)).is_equal_to(Some(PRO_THEME));
    s.undo().unwrap();
    assert_that!(s.deck().theme.as_ref().map(|t| t.id)).is_equal_to(Some(DEFAULT_THEME));
}

#[test]
fn deck_renders_with_embedded_version_not_library_current() {
    // ajouter une version utilisateur sous PAPIER… non : sous un thème utilisateur U (accent rouge), le deck l'adopte,
    // puis une nouvelle version courante de U (accent bleu) arrive dans la bibliothèque :
    // le rendu garde le rouge (le cache fait foi).
}

#[test]
fn set_theme_unknown_id_is_refused() {
    let mut s = test_session();
    assert_that!(s.set_theme(Uuid::new_v4())).is_err();
}
```

- [ ] **Step 2: Run** `cargo nextest run -p oku-slides-session deck_theme` → FAIL.
- [ ] **Step 3: Implémenter**, supprimer le pont et `THEMES` ; `rg -n "theme_bridge|THEMES" crates` doit être vide.
- [ ] **Step 4: Run** `cargo nextest run --workspace` → PASS.
- [ ] **Step 5: Commit** `feat(session)! : le deck embarque son thème, choisi dans la bibliothèque des thèmes (t-554)`.

### Task 8: Menu des thèmes de l'interface

**Files:**
- Modify: `ui/src/api.ts` (`Snapshot.theme: ThemeRef | null`, `themes: ThemeInfo[]`, requête `set_theme`, `TabApi.setTheme(id)`), `ui/src/App.tsx:633-640` (options = `snap.themes`, valeur = `snap.theme?.id`, `onChange` → `run(api.setTheme(id))` ; plus d'option « par défaut »), `ui/src/App.tsx:176` (`look` : `snap.theme?.version`)
- Modify: `crates/oku-slides-i18n/src/locales/{en,fr}/ui.ftl` (retirer `ui-toolbar-theme-default` s'il n'est plus utilisé ; badge « intégré » si affiché)
- Test: test de rendu existant du menu, ou nouveau `ui/src/ThemeMenu.render.test.tsx` si le sélecteur est extrait

**Interfaces:**
- Consumes: snapshot et requête de la tâche 7.
- Produces: `export interface ThemeRef { id: string; version: string }`, `export interface ThemeInfo { id: string; version: string; name: string; builtin: boolean; default: boolean }`.

- [ ] **Step 1: Test** (rendu) : avec un snapshot de deux thèmes, le `<select>` montre « Papier », « Pro », et choisir « Pro » appelle `setTheme("7e1b9c3d-…")`.
- [ ] **Step 2: Run** `cd ui && bun test` → FAIL.
- [ ] **Step 3: Implémenter.**
- [ ] **Step 4: Run** `cd ui && bun test && bun run typecheck` → PASS ; `just e2e` isolé (Global Constraints) sur les e2e qui touchent le menu.
- [ ] **Step 5: Commit** `feat(ui) : le menu des thèmes liste la bibliothèque des thèmes (t-554)`.

---

## Phase 2 — Éditeur de jetons

### Task 9: Schéma des jetons et validation

**Files:**
- Create: `crates/oku-slides-session/src/theme_schema.json`, `crates/oku-slides-session/src/theme_schema.rs` (+ `theme_schema_tests.rs`)
- Modify: `crates/oku-slides-session/src/error.rs` (`InvalidTokenValue { path, reason }`, `TokenInUse { path, users }`), `session.ftl` en/fr
- Modify: `crates/oku-slides-render/src/palette.rs` (`resolve_color` refuse un `#` non hexadécimal — t-194) et son test

**Interfaces:**
- Produces:

```rust
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "control", rename_all = "kebab-case")]
pub enum Control {
    Color, Pt { min: f64, max: f64, refs: Option<String> }, Em { min: f64, max: f64 },
    Weight, Fonts, Choice { options: Vec<String> }, Bool, Text,
    Palette, Number { min: f64, max: f64, step: f64 },
}
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TokenSpec { pub path: Vec<String>, pub section: Section, #[serde(flatten)] pub control: Control, #[serde(default)] pub nullable: bool }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Section { Colors, Typography, Spacing, Shapes, Page, Charts, Components, AppLight, AppDark, Scripts }
pub fn schema() -> &'static [TokenSpec];          // lu une fois (LazyLock), include_str!
pub fn spec(path: &[String]) -> Option<&'static TokenSpec>;   // `color.<libre>` → spec générique Color
pub fn validate(path: &[String], value: &Value, tokens: &Map<String, Value>) -> Result<()>;
pub fn users(path: &[String], tokens: &Map<String, Value>) -> Vec<String>; // chemins qui y renvoient
```

Règles de `validate` : `Color` = `#rgb`, `#rrggbb`, `#rrggbbaa` ou nom d'une clé de `color` existante ; `Pt`/`Em`/`Number` = nombre dans les bornes, ou (si `refs`) nom d'une clé de la catégorie `refs` ; `Weight` = 100..=900 par 100 ou nom de clé `weight` ; `Fonts` = tableau non vide de chaînes non vides ; `Choice` = une des options ; `Bool` ; `Text` = chaîne ; `Palette` = tableau de couleurs valides ; `null` seulement si `nullable`. Chemin inconnu → `InvalidTokenValue { reason: "unknown token" }`. Nom libre sous `color` : `[a-z][a-z0-9-]*`.

- [ ] **Step 1: Tests qui échouent**

```rust
#[test]
fn schema_covers_exactly_default_tokens_paths() {
    let defaults: Value = serde_json::from_str(include_str!("../../../typst/oku-slides/src/default-tokens.json")).unwrap();
    let mut leaves = leaf_paths(&defaults);                // tableaux = feuilles
    leaves.retain(|p| !(p[0] == "page" && (p[1] == "width" || p[1] == "height")));
    let mut covered: Vec<Vec<String>> = schema().iter().map(|s| s.path.clone()).collect();
    leaves.sort(); covered.sort();
    assert_that!(covered).is_equal_to(leaves);
}

#[rstest]
#[case(&["color", "accent"], json!("#3f6b5e"), true)]
#[case(&["color", "accent"], json!("ink"), true)]
#[case(&["color", "accent"], json!("#zzz"), false)]
#[case(&["size", "body"], json!(-4), false)]
#[case(&["weight", "title"], json!(450), false)]
#[case(&["component", "header", "left"], Value::Null, true)]
#[case(&["size", "body"], Value::Null, false)]
fn validate_checks_control(#[case] path: &[&str], #[case] value: Value, #[case] ok: bool) {
    let tokens = defaults_map();
    assert_that!(validate(&p(path), &value, &tokens).is_ok()).is_equal_to(ok);
}

#[test]
fn users_lists_tokens_referencing_a_color() {
    let tokens = json!({"color": {"brand-ink": "#00154e"}, "component": {"title": {"color": "brand-ink"}}});
    assert_that!(users(&p(&["color", "brand-ink"]), tokens.as_object().unwrap())).is_equal_to(vec!["component.title.color".to_owned()]);
}
```

- [ ] **Step 2: Run** `cargo nextest run -p oku-slides-session theme_schema` → FAIL.
- [ ] **Step 3: Écrire `theme_schema.json`** (une entrée par feuille de `default-tokens.json`, bornes : `pt` 0..=400 sauf `page.*` 0..=1080, `em` 0..=2, `opacity` 0..=1, `ratio` 0..=1, `tracking` -10..=20) et le module.
- [ ] **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat(session) : schéma des jetons de design et validation des valeurs (t-554)`.

### Task 10: Brouillon de thème en mode modèles

**Files:**
- Create: `crates/oku-slides-session/src/theme_edit.rs` (+ `theme_edit_tests.rs`)
- Modify: `crates/oku-slides-session/src/templates_mode.rs` (`styled_like` : `theme: None`, `tokens` = jetons du thème édité ; `same_style` ; `save_templates` appelle `save_theme_edit` ; `close_templates` abandonne), `views.rs` (`TemplateMode.theme: ThemeEditView { base: ThemeRef, name: String, builtin: bool, dirty: bool }`), `request.rs` (`set_edited_theme { id }`, `rename_edited_theme { name }`), `session.ftl` en/fr (`theme-copy-name = { $name } (copie)`, libellés d'annulation)

**Interfaces:**
- Consumes: `ThemeLibrary`, `ThemeManifest`, `Session::theme_manifest`, `embed_theme_from_library` (tâches 6-7) ; `Op::SetTokens`, `SetToken`, `UnsetToken` (tâche 4) ; `validate`, `users` (tâche 9).
- Produces:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeEdit { pub base: ThemeRef, pub name: String }
impl Session {
    pub fn set_edited_theme(&mut self, id: Uuid) -> Result<()>;      // SetTokens annulable, base change
    pub fn rename_edited_theme(&mut self, name: String) -> Result<()>;
    pub(crate) fn edited_tokens(&self) -> Map<String, Value>;        // deck de travail
    pub(crate) fn base_tokens(&self) -> Map<String, Value>;          // version de base
    fn save_theme_edit(&mut self) -> Result<Option<ThemeRef>>;       // appelé par save_templates
}
```

`Mode::Templates` gagne `theme: ThemeEdit` (persisté dans l'état de reprise comme le reste du mode). `apply` en mode modèles fait passer chaque `SetToken`/`UnsetToken` par `validate` / `users` (refus → erreur, rien d'appliqué). Hors mode modèles, `apply` valide aussi tout `SetToken` dont le contrôle est `Color` ou `Palette` (surcharges du deck, règle t-194) ; test : `deck_override_with_invalid_color_is_refused`.

- [ ] **Step 1: Tests qui échouent**

```rust
#[test]
fn entering_templates_loads_full_theme_tokens_without_theme_ref() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    assert_that!(s.deck().theme).is_none();
    assert_that!(s.deck().tokens.pointer("/color/accent")).is_equal_to(Some(&json!("#3f6b5e")));
}

#[test]
fn saving_edited_builtin_creates_a_copy_used_by_the_deck() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.apply("x", vec![Op::SetToken { path: p(&["color", "accent"]), value: json!("#ff0000") }], None).unwrap();
    s.save_templates(false).unwrap();
    let r = s.user_deck().theme.clone().unwrap();
    assert_that!(r.id).is_not_equal_to(PAPIER_THEME);
    let m = s.theme_manifest(&r).unwrap();
    assert_that!(m.name.as_str()).is_equal_to("Papier (copie)");
    assert_that!(m.parent.is_some()).is_true();
    assert_that!(m.tokens["color"]["accent"]).is_equal_to(json!("#ff0000"));
}

#[test]
fn saving_edited_user_theme_adds_a_current_version() { /* thème utilisateur U, édition, save → même id, nouvelle version courante, parent = l'ancienne */ }

#[test]
fn saving_without_theme_change_creates_nothing() { /* save → le deck garde le même ThemeRef, bibliothèque des thèmes inchangée */ }

#[test]
fn closing_without_saving_discards_theme_draft() { /* SetToken, close_templates → le deck garde son ThemeRef d'origine */ }

#[test]
fn removing_a_referenced_color_is_refused() {
    let mut s = session_with_theme(PRO_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    let r = s.apply("x", vec![Op::UnsetToken { path: p(&["color", "brand-ink"]) }], None);
    assert_that!(r).is_err();
}

#[test]
fn choosing_another_theme_replaces_draft_in_one_undo_step() {
    let mut s = session_with_theme(PAPIER_THEME);
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.set_edited_theme(PRO_THEME).unwrap();
    assert_that!(s.deck().tokens.pointer("/font/body/0")).is_equal_to(Some(&json!("Inter")));
    s.undo().unwrap();
    assert_that!(s.deck().tokens.pointer("/font/body/0")).is_equal_to(Some(&json!("Instrument Sans")));
}
```

- [ ] **Step 2: Run** `cargo nextest run -p oku-slides-session theme_edit` → FAIL.
- [ ] **Step 3: Implémenter.** Le brouillon diffère de la base si `edited_tokens() != base_tokens()` ou `name` différent du nom de la base. Copie d'un intégré : `Uuid::new_v4()`, `parent: Some(base.version)`, nom `theme-copy-name` si le nom n'a pas été changé. Thème utilisateur : `add_version` + `set_current`. Dans les deux cas : `embed_theme_from_library`, et le deck mis de côté reçoit `theme` (sans étape d'annulation, comme le pack).
- [ ] **Step 4: Run** `cargo nextest run -p oku-slides-session` → PASS.
- [ ] **Step 5: Commit** `feat(session) : brouillon du thème en mode modèles, enregistré en version ou en copie (t-554)`.

### Task 11: Surface `host-theme`

**Files:**
- Create: `crates/oku-slides-session/src/theme_panel.rs` (+ `theme_panel_tests.rs`)
- Modify: `crates/oku-slides-session/src/agent/forms.rs` (extraire la fabrique de composant d'un champ, aujourd'hui interne à `params_with`, en `pub(crate) fn field_components(...)` réutilisée — pas de copie), `request.rs` (`host_theme { filter }` → `Option<Surface>`, `host_token { path, action, value }` → `Snapshot`), `session.ftl` en/fr (libellés `theme-token-<chemin-à-tirets>` pour chaque entrée du schéma, sections `theme-section-*`, états `theme-state-default|defined|modified`, actions `theme-action-reset|remove|clear`)
- Modify: `crates/oku-slides-a2ui` seulement si un contrôle manque (liste de familles : `ListTable` à une colonne texte ; palette : `ListTable` à une colonne couleur)

**Interfaces:**
- Consumes: `schema`, `spec`, `Section`, `Control` (tâche 9) ; `edited_tokens`, `base_tokens` (tâche 10) ; `Component::{column,row,text,button}`, `Disclosure`, `Badge`, `ColorField`, `Stepper`, `Slider`, `Select`, `TextField`, `ListTable` (a2ui).
- Produces:

```rust
pub(crate) const HOST_THEME: &str = "host-theme";
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TokenState { Default, Defined, Modified }
pub(crate) fn token_state(path: &[String], draft: &Map<String, Value>, base: &Map<String, Value>) -> TokenState;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TokenAction { Set, Reset, Remove, Clear }
impl Session {
    pub(crate) fn host_theme(&self, filter: &str) -> Result<Option<Surface>>;
    pub(crate) fn host_token(&mut self, path: Vec<String>, action: TokenAction, value: Option<Value>) -> Result<()>;
}
```

Surface : une `Disclosure` par `Section` (repliée sauf « Couleurs »), une ligne par jeton : libellé, `Badge` d'état, contrôle, bouton ↺ (si `Modified`), « Retirer du thème » (si `Defined`), « Vider » (si `nullable` et valeur non nulle). Chaque action envoie `host_token`. `Reset` → `SetToken(valeur de base)` ou `UnsetToken` si absente de la base ; `Remove` → `UnsetToken` ; `Clear` → `SetToken(null)`. Filtre : sous-chaîne, insensible à la casse, sur le libellé traduit ou le chemin ; sections vides omises. Les couleurs libres de `color` ont en plus un champ « Nouvelle couleur » (nom + couleur).

- [ ] **Step 1: Tests qui échouent**

```rust
#[rstest]
#[case(json!({}), json!({}), TokenState::Default)]
#[case(json!({"size": {"body": 30}}), json!({"size": {"body": 30}}), TokenState::Defined)]
#[case(json!({"size": {"body": 32}}), json!({"size": {"body": 30}}), TokenState::Modified)]
fn token_state_compares_draft_and_base(#[case] draft: Value, #[case] base: Value, #[case] want: TokenState) {
    assert_that!(token_state(&p(&["size", "body"]), draft.as_object().unwrap(), base.as_object().unwrap())).is_equal_to(want);
}

#[test]
fn host_theme_filter_keeps_matching_rows_only() {
    let s = templates_session(PAPIER_THEME);
    let json = serde_json::to_string(&s.host_theme("accent").unwrap().unwrap()).unwrap();
    assert_that!(json.contains("color.accent")).is_true();
    assert_that!(json.contains("size.body")).is_false();
}

#[test]
fn host_token_reset_restores_base_value() {
    let mut s = templates_session(PAPIER_THEME);
    s.host_token(p(&["color", "accent"]), TokenAction::Set, Some(json!("#ff0000"))).unwrap();
    s.host_token(p(&["color", "accent"]), TokenAction::Reset, None).unwrap();
    assert_that!(s.deck().tokens.pointer("/color/accent")).is_equal_to(Some(&json!("#3f6b5e")));
}

#[test]
fn every_schema_entry_has_an_english_and_french_label() {
    for spec in schema() {
        let id = format!("theme-token-{}", spec.path.join("-"));
        assert_that!(english().has(&id)).is_true();
        assert_that!(french().has(&id)).is_true();
    }
}
```

- [ ] **Step 2: Run** `cargo nextest run -p oku-slides-session theme_panel` → FAIL.
- [ ] **Step 3: Implémenter** (fichier ≤ 500 lignes : si la construction des contrôles déborde, la mettre dans `theme_controls.rs`).
- [ ] **Step 4: Run** `cargo nextest run -p oku-slides-session -p oku-slides-a2ui` → PASS.
- [ ] **Step 5: Commit** `feat(session) : surface host-theme des jetons, par section, avec état et actions (t-554)`.

### Task 12: Onglet Thème dans la perspective modèles

**Files:**
- Modify: `ui/src/TemplateTabs.tsx` (onglet fixe « Thème », clé `"theme"` dans l'ordre clavier après « Modèles »), `ui/src/widgetTabs.ts` / `useTemplateWidgets.ts` (onglet actif : `null` = Modèles, `"theme"`, ou id de widget)
- Create: `ui/src/ThemePanel.tsx` (+ `ThemePanel.render.test.tsx`), `ui/src/useThemeSettings.ts` (+ `useThemeSettings.render.test.tsx`)
- Modify: `ui/src/App.tsx` (mode modèles : onglet Thème → pile à gauche, `Stage` du modèle choisi au centre, `ThemePanel` à droite ; retirer `themePicker` du menu ⋯ du mode modèles, l. 679), `ui/src/useHostSettings.ts` (`HOST_THEME` reconnu par le routeur), `ui/src/api.ts` (`hostTheme(filter)`, `hostToken(path, action, value)`, `setEditedTheme(id)`, `renameEditedTheme(name)`, type `TemplateMode.theme`), `ui/src/styles.css` (colonne droite `.theme-panel`)
- Modify: `crates/oku-slides-i18n/src/locales/{en,fr}/ui.ftl` (`ui-theme-tab`, `ui-theme-picker`, `ui-theme-name`, `ui-theme-filter`, `ui-theme-unsaved`)

**Interfaces:**
- Consumes: requêtes des tâches 10-11 ; `HostSettings` (composant existant).
- Produces: `useThemeSettings(api, snap, filter, run, say): HostSettings` (même contrat que `useHostSettings`, clé de relecture : `snap.editing?.theme`, `snap.undo_label`, `snap.redo_label`, filtre).

- [ ] **Step 1: Tests qui échouent**

```tsx
// ThemePanel.render.test.tsx
test("ThemePanel lists themes and renames the edited theme", async () => {
  const api = fakeApi();                       // helper existant des tests de rendu
  render(<ThemePanel api={api} snap={snapEditingTheme("Papier")} … />);
  expect(screen.getByRole("combobox", { name: "Theme" })).toHaveValue(PAPIER_ID);
  await user.clear(screen.getByRole("textbox", { name: "Theme name" }));
  await user.type(screen.getByRole("textbox", { name: "Theme name" }), "Maison{Enter}");
  expect(api.renameEditedTheme).toHaveBeenCalledWith("Maison");
});

// TemplateTabs : flèche droite depuis « Modèles » active « Thème »
test("TemplateTabs ArrowRight from Templates activates Theme tab", …);
```

- [ ] **Step 2: Run** `cd ui && bun test ThemePanel TemplateTabs` → FAIL.
- [ ] **Step 3: Implémenter.**
- [ ] **Step 4: Run** `cd ui && bun test && bun run typecheck` → PASS ; e2e existants du mode modèles (`ui/e2e/panel.cjs`, `widgets.cjs`) en données isolées → PASS.
- [ ] **Step 5: Commit** `feat(ui) : onglet Thème du mode modèles, panneau des jetons à droite (t-554)`.

### Task 13: Parcours e2e, traductions, documentation

**Files:**
- Create: `ui/e2e/theme-editor.cjs` (et son entrée dans le runner e2e existant)
- Modify: `crates/oku-slides-i18n/src/locales/<18 autres>/{ui,session,cli}.ftl`
- Modify: `README.md:95-99` (thèmes), `typst/oku-slides/README.md`, `crates/oku-slides-{core,session,render}/ARCHITECTURE.md`, `docs/aide/{en,fr}.json` (aide : section thèmes si elle parle de papier/pro codés en dur)

- [ ] **Step 1: e2e** : ouvrir « Modèles et widgets », onglet Thème, filtrer « accent », choisir `#ff0000` pour `color.accent`, vérifier que le SVG de la scène contient `#ff0000`, valider ✓, vérifier que le menu des thèmes contient « Papier (copie) » sélectionné. Lancer avec `OKU_SLIDES_PATHS__DATA` et `OKU_SLIDES_PATHS__DECKS` dans le scratchpad.
- [ ] **Step 2: Traductions** : toutes les clés ajoutées en en/fr, dans les 18 autres langues (traduction réelle, pas de copie de l'anglais). Vérifier : `cargo nextest run -p oku-slides-i18n` (tests de complétude existants).
- [ ] **Step 3: Docs** à jour.
- [ ] **Step 4: Run** `just check` puis e2e complet isolé → PASS.
- [ ] **Step 5: Commit** `test(ui) : parcours e2e de l'éditeur de thème ; traductions et docs (t-554)`.

### Task 14: Passe de consolidation

- [ ] **Step 1:** `rg -n "THEMES|theme_bridge|papier\.typ|pro\.typ|body_font|MVP: t-554" crates ui typst` → vide.
- [ ] **Step 2:** Chercher les doublons (G10) : une seule lecture de manifeste par sorte, un seul parcours de chemins de jetons (`theme_schema` et `theme_panel` partagent `leaf_paths`/`spec`), aucune copie de `params_with`.
- [ ] **Step 3:** `just check` → PASS ; mettre à jour les sous-tâches Oku et le corps de t-554.
- [ ] **Step 4: Commit** (s'il y a des retouches) `refactor : consolidation des thèmes en packs (t-554)`.
