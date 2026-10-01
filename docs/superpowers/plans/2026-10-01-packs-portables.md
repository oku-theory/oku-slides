# Packs portables : identité, versions et pack embarqué — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** un deck se rend partout comme chez son auteur : son fichier embarque la version exacte de son pack de modèles (modèles et widgets), la bibliothèque de l'utilisateur devient versionnée (UUID v4, versions adressées par contenu, une version courante par pack), et une autre version dans la bibliothèque ne donne qu'un bandeau, jamais un autre rendu.

**Architecture:** le cœur gagne `PackVersion` (sha-256 du contenu canonique), `PackRef` (id + version), `PackFiles` (`pack.json` + `widgets.typ`), `Deck.templates: Option<PackRef>` (atome du replica) et `SourceBundle.packs` (zip v4, `packs/<hex>/…`, vérifiés à la lecture, unis à la fusion). La session remplace sa bibliothèque par `library::Library` (`<data>/templates/<uuid>/{current, versions/<hex>/…}`, packs intégrés à id constant), rend toujours depuis la version embarquée (`PackWidgets::new(<hex>, …)` dans `RenderWorker::probe`), ajoute en silence les versions reçues à l'ouverture et pose le bandeau `pack_notice` (deux actions). Le mode modèles crée des versions. La CLI embarque le pack à `save` et passe par le `RenderWorker` quand il a des widgets. L'UI montre le bandeau et liste les packs par id.

**Tech Stack:** Rust 2024, serde/serde_json (`preserve_order`), sha2/hex, uuid 1 (`v4`, `serde`, nouveau au workspace), Loro, OpenDAL (`Storage`), snafu, speculoos, rstest, insta, Fluent ; React 19 + TypeScript, bun test.

**Spec:** `docs/superpowers/specs/2026-10-01-packs-portables-design.md` (t-328)

## Global Constraints

- `DECK_VERSION = 3`, `SOURCES_VERSION = 4` ; `manifest.json` : `{ format: "oku-slides-sources", version: 4 }`. Aucune rétrocompatibilité : decks v2, sources v3 et packs au format slug ne sont plus lus ; fixtures, modèles intégrés et tests réécrits.
- Zip des sources :
  ```
  manifest.json     { format: "oku-slides-sources", version: 4 }
  deck.loro
  assets/<hex>
  fonts/<nom>
  packs/<hex>/pack.json
  packs/<hex>/widgets.typ    (absent si le pack n'a pas de widgets)
  ```
- `PackVersion` : `"sha256:<hex>"`, `<hex>` = 64 chiffres hexadécimaux **minuscules**, sha-256 du **contenu canonique** : `pack.json` sérialisé en JSON compact à clés triées, un octet `0`, puis `widgets.typ` tel quel (vide s'il est absent). Nom et description sont dans `pack.json` : les changer crée une version. Un `widgets.typ` vide vaut absent.
- `pack.json` : `{ id, name, description, parent: Option<PackVersion>, created, templates }` ; le slug disparaît. `created` : UTC à la seconde, `AAAA-MM-JJTHH:MM:SSZ` (comparable comme chaîne).
- Bibliothèque :
  ```
  <data>/templates/<uuid>/current            # PackVersion de la version courante
  <data>/templates/<uuid>/versions/<hex>/pack.json
  <data>/templates/<uuid>/versions/<hex>/widgets.typ
  ```
  Écriture atomique (`Storage::write_atomic`), jamais `std::fs` hors des tests.
- Packs intégrés, id constants (`parent: null`, `created: "2026-10-01T00:00:00Z"`) :
  - `essentiel` → `DEFAULT_PACK` = `6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10`
  - `grilles` → `GRID_PACK` = `b8e2d4f6-1a3c-4e5b-8d7f-9c0a2e4b6d81`
- Rendu : `RenderWorker::probe(PackWidgets::new(<hex>, widgets.typ))` — l'id de pack dans le monde Typst est le hash (64 hex minuscules : `is_pack_id` l'accepte).
- `uuid = { version = "1", features = ["v4", "serde"] }` au workspace (aujourd'hui seulement transitive dans `Cargo.lock`, 1.26.1).
- Erreurs : core `Issue` `issue-pack-version-mismatch` / `issue-pack-version-missing` (constructeurs `Issue::pack_version_mismatch`, `Issue::pack_version_missing`), chemin `packs/` invalide → `SourcesErrorKind::BadName` ; session `UnknownPack { id }`, `PackVersionMissing { id, version }`, `PackJson { id }` — Snafu, `backtrace` dans chaque variante, messages en minuscules.
- Fluent, **20 langues** (`crates/oku-slides-i18n/src/locales/<l>/*.ftl`) : fr et en donnés ci-dessous, traduire les 18 autres dans le registre des clés voisines. Clés : `issue-pack-version-mismatch`, `issue-pack-version-missing` (core.ftl) ; `session-unknown-pack`, `session-pack-version-missing`, `session-invalid-pack`, `session-pack-missing`, `templates-adopt-label`, `pack-notice-older`, `pack-notice-newer`, `pack-notice-parallel`, `pack-notice-use-mine`, `pack-notice-make-current` (session.ftl) ; `ui-pack-notice-close` (ui.ftl) ; `session-no-template-set`, `session-invalid-template-set`, `ui-status-builtin` retirées.
- Pas de `.unwrap()`/`.expect()` hors tests ; commentaires de doc en français, première phrase de moins de 15 mots ; `ARCHITECTURE.md` de chaque crate sous 50 lignes.
- Tests côte à côte `foo_tests.rs` (`#[cfg(test)] #[path = "foo_tests.rs"] mod foo_tests;`), speculoos, nommage `<unité>_<scénario>_<résultat>`, TDD (test en échec d'abord).
- Commits : `<type>(<scope>) : <description> (t-328)`, **sans** ligne `Co-Authored-By`. `git add` dans une commande séparée du `git commit` (le hook de revue bloque sinon toute la commande).
- Vérification d'un crate : `cargo nextest run -p <crate>` ; en fin de plan : `just check`.

### Règle de verdeur (choisie pour ce plan)

Le changement du modèle (`Deck.templates: Option<PackRef>`, Task 3) casse à la compilation tout ce qui lit `deck.templates` en chaîne. Règle : **à chaque tâche, le crate en cours et ceux en amont sont verts** ; les crates en aval peuvent ne pas compiler entre la Task 3 et leur migration :

| Après la tâche | Verts |
|---|---|
| 1, 2 | tout le workspace |
| 3, 4 | core, chart, i18n, platform, render, render-worker, pdf |
| 5, 6 | + session |
| 7 | + mcp, agent, src-tauri |
| 8 | tout le workspace (`cargo build --workspace`) |
| 9, 10 | tout, UI comprise ; `just check` à la Task 10 |

La bibliothèque versionnée (Task 2) est écrite **avant** le changement du modèle, à côté de l'ancienne, pour être testée sur un crate qui compile ; l'ancienne disparaît à la Task 5.

---

## Carte des fichiers

| Fichier | Rôle |
|---|---|
| `Cargo.toml` | `uuid` au workspace |
| `crates/oku-slides-core/src/pack.rs` (nouveau) | `PackVersion`, `PackRef`, `PackFiles`, contenu canonique |
| `crates/oku-slides-core/src/error.rs` | `PackVersionError` |
| `crates/oku-slides-core/src/model.rs` | `DECK_VERSION = 3`, `Deck.templates: Option<PackRef>` |
| `crates/oku-slides-core/src/ops/mod.rs` | `Op::SetTemplates { templates: Option<PackRef> }` |
| `crates/oku-slides-core/src/replica/schema.rs` | `templates` : atome |
| `crates/oku-slides-core/src/sources.rs` | `SourceBundle.packs`, zip v4, intégrité, fusion |
| `crates/oku-slides-core/src/validate.rs` | `Issue::pack_version_mismatch`, `Issue::pack_version_missing` |
| `crates/oku-slides-pdf/src/lib.rs`, `ARCHITECTURE.md` | description du zip v4 |
| `crates/oku-slides-session/src/pack.rs` (nouveau) | `PackManifest`, `Template`, `PackInfo`, `Relation`, `DEFAULT_PACK`, `GRID_PACK`, `now_created` |
| `crates/oku-slides-session/src/library.rs` (nouveau) | `Library` versionnée, `StoredVersion`, packs intégrés |
| `crates/oku-slides-session/src/templates.rs` | supprimé (Task 5) |
| `crates/oku-slides-session/src/deck_pack.rs` (nouveau) | pack du deck : adoption du pack par défaut, embarquement depuis la bibliothèque |
| `crates/oku-slides-session/src/pack_notice.rs` (nouveau) | versions reçues, bandeau et ses actions |
| `crates/oku-slides-session/src/{lib,deck_widgets,templates_mode,history,resume,merge,files,views,error}.rs`, `agent/context.rs` | migration |
| `crates/oku-slides-session/tests/regressions/issue_0328_pack_recu_garde_la_version_courante.rs` | régression |
| `crates/oku-slides-mcp/src/edit_tools.rs` | `SetArgs.set: Uuid` |
| `crates/oku-slides-cli/src/pack_render.rs` (nouveau) | rendu en mémoire ou par le `RenderWorker` |
| `crates/oku-slides-cli/src/main.rs` | `save` embarque le pack ; `render`/`save` par `pack_render` |
| `templates/essentiel/pack.json`, `templates/grilles/pack.json` | id constants, `parent`, `created` |
| `fixtures/roadmap.json`, `typst/oku-slides/{README.md,examples/deck.json,src/data.typ}` | deck v3 |
| `ui/src/api.ts`, `ui/src/PackNoticeBar.tsx` (nouveau), `ui/src/App.tsx`, `ui/src/TemplatePicker.tsx`, `ui/src/autosave.ts` | bandeau, packs par id |

---

### Task 1 : types de pack et contenu canonique (core)

**Files:**
- Modify: `Cargo.toml` (après `getrandom = …`, ligne 32)
- Modify: `crates/oku-slides-core/Cargo.toml` (`[dependencies]`)
- Create: `crates/oku-slides-core/src/pack.rs`, `crates/oku-slides-core/src/pack_tests.rs`
- Modify: `crates/oku-slides-core/src/error.rs` (fin du fichier : `PackVersionError`)
- Modify: `crates/oku-slides-core/src/lib.rs` (mod + réexports)

**Interfaces:**
- Produces :
  - `pub struct PackVersion(String)` — `Clone + Eq + Ord + Hash + Serialize + Deserialize + JsonSchema + Display` ; `PackVersion::of(&PackFiles) -> PackVersion`, `PackVersion::parse(&str) -> Option<PackVersion>`, `PackVersion::from_hex(&str) -> Option<PackVersion>`, `PackVersion::hex(&self) -> &str`, `PackVersion::as_str(&self) -> &str` ; sérialisé `"sha256:<hex>"`, refusé sinon.
  - `pub struct PackRef { pub id: Uuid, pub version: PackVersion }` — `Clone + Eq + Hash + Serialize + Deserialize + JsonSchema`.
  - `pub struct PackFiles` (privé : `manifest: Map<String, Value>`, `widgets: Option<String>`) — `PackFiles::new(Map<String, Value>, Option<String>) -> PackFiles`, `PackFiles::from_bytes(&[u8], Option<&[u8]>) -> Option<PackFiles>`, `manifest(&self) -> &Map<String, Value>`, `widgets(&self) -> Option<&str>`, `version(&self) -> PackVersion`, `canonical(&self) -> Vec<u8>`, `pack_json(&self) -> serde_json::Result<Vec<u8>>`.
  - `pub struct PackVersionError` (Snafu, `backtrace`).

- [ ] **Step 1 : tests en échec** — `crates/oku-slides-core/src/pack_tests.rs`

```rust
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

#[test]
fn pack_version_of_known_content_is_stable() {
    // Canonique : {"a":[{"c":"x","d":2}],"b":1} 0x00 w
    let files = PackFiles::new(object(json!({ "b": 1, "a": [{ "d": 2, "c": "x" }] })), Some("w".into()));
    assert_that!(files.version().as_str())
        .is_equal_to("sha256:3356f340f7d5efcbd0e55f5f06a60729765c18942d666015444191d545839756");
}

#[test]
fn pack_version_without_widgets_hashes_an_empty_widgets_file() {
    let files = PackFiles::new(object(json!({ "b": 1, "a": [{ "d": 2, "c": "x" }] })), None);
    assert_that!(files.version().as_str())
        .is_equal_to("sha256:99b62953dd3d649f39732438515923d2db4f402b3e1ee7a15ab5b343a4974d3a");
    let empty = PackFiles::new(files.manifest().clone(), Some(String::new()));
    assert_that!(empty.version()).is_equal_to(files.version());
    assert_that!(empty.widgets()).is_none();
}

#[test]
fn pack_version_ignores_key_order_at_every_depth() {
    let a = PackFiles::new(object(json!({ "name": "P", "templates": [{ "id": "t", "name": "T" }] })), None);
    let b = PackFiles::new(object(json!({ "templates": [{ "name": "T", "id": "t" }], "name": "P" })), None);
    assert_that!(a.version()).is_equal_to(b.version());
}

#[test]
fn pack_version_changes_with_name_or_widgets() {
    let base = PackFiles::new(object(json!({ "name": "P" })), Some("#let registry = ()".into()));
    let renamed = PackFiles::new(object(json!({ "name": "Q" })), Some("#let registry = ()".into()));
    let other = PackFiles::new(object(json!({ "name": "P" })), Some("#let registry = () ".into()));
    assert_that!(renamed.version()).is_not_equal_to(base.version());
    assert_that!(other.version()).is_not_equal_to(base.version());
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
    assert_that!(PackFiles::from_bytes(b"[1]", None)).is_none();
    assert_that!(PackFiles::from_bytes(b"pas du json", None)).is_none();
    assert_that!(PackFiles::from_bytes(br#"{"name":"P"}"#, Some(&[0xff_u8, 0xfe][..]))).is_none();
    let files = PackFiles::from_bytes(br#"{"name":"P"}"#, Some(b"#let registry = ()")).unwrap();
    assert_that!(files.widgets()).is_equal_to(Some("#let registry = ()"));
}

#[test]
fn pack_files_pack_json_reads_back_to_the_same_version() {
    let files = PackFiles::new(object(json!({ "name": "P", "id": "x" })), Some("w".into()));
    let back = PackFiles::from_bytes(&files.pack_json().unwrap(), files.widgets().map(str::as_bytes)).unwrap();
    assert_that!(back.version()).is_equal_to(files.version());
}
```

Ajouter `uuid.workspace = true` aux `[dependencies]` de `oku-slides-core` (déjà nécessaire au code) ; `serde_json` a `preserve_order` au workspace : `json!` garde l'ordre écrit, ce qui rend le test d'ordre des clés significatif.

- [ ] **Step 2 : lancer** — `cargo nextest run -p oku-slides-core pack` → FAIL (module `pack` absent, `uuid` inconnu).

- [ ] **Step 3 : implémenter**

`Cargo.toml` (workspace), après la ligne `getrandom = …` :

```toml
# Identité des packs de modèles (t-328) : UUID v4.
uuid = { version = "1", features = ["v4", "serde"] }
```

`crates/oku-slides-core/Cargo.toml` : `uuid.workspace = true` dans `[dependencies]`.

`crates/oku-slides-core/src/error.rs`, à la fin :

```rust
/// Une chaîne qui n'est pas une [`PackVersion`](crate::PackVersion).
#[derive(Debug, Snafu)]
#[snafu(display("invalid pack version: {value}"), visibility(pub(crate)))]
pub struct PackVersionError {
    value: String,
    backtrace: Backtrace,
}
```

`crates/oku-slides-core/src/pack.rs` :

```rust
//! Packs de modèles portables : identité, version et fichiers.
//!
//! Un pack a un id ([`PackRef::id`], UUID v4) et des versions adressées par
//! contenu ([`PackVersion`]). Le contenu canonique d'une version : son
//! `pack.json` en JSON compact à clés triées, un octet `0`, puis son
//! `widgets.typ` tel quel (vide s'il est absent). La version en est le
//! sha-256, écrit `sha256:<hex>` (64 chiffres hexadécimaux minuscules).

use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::{PackVersionError, PackVersionSnafu};

/// Préfixe d'une [`PackVersion`].
const PREFIX: &str = "sha256:";

/// 64 chiffres hexadécimaux minuscules : une empreinte, et un nom de dossier sûr.
fn is_hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Version d'un pack : `sha256:<hex>` de son contenu canonique.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
#[schemars(transparent)]
pub struct PackVersion(String);

impl PackVersion {
    /// La version de `files`.
    pub fn of(files: &PackFiles) -> Self {
        Self(format!("{PREFIX}{}", hex::encode(Sha256::digest(files.canonical()))))
    }

    /// Lit `sha256:<hex>` ; `None` pour toute autre forme.
    pub fn parse(s: &str) -> Option<Self> {
        s.strip_prefix(PREFIX).and_then(Self::from_hex)
    }

    /// Depuis l'empreinte seule (nom de dossier `packs/<hex>`).
    pub fn from_hex(hex: &str) -> Option<Self> {
        is_hex64(hex).then(|| Self(format!("{PREFIX}{hex}")))
    }

    /// L'empreinte sans préfixe : nom de dossier, id du pack dans Typst.
    pub fn hex(&self) -> &str {
        self.0.strip_prefix(PREFIX).unwrap_or(&self.0)
    }

    /// La forme écrite, `sha256:<hex>`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for PackVersion {
    type Error = PackVersionError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(&s).ok_or_else(|| PackVersionSnafu { value: s }.build())
    }
}

impl From<PackVersion> for String {
    fn from(v: PackVersion) -> Self {
        v.0
    }
}

impl fmt::Display for PackVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Le pack d'un deck : son id et la version embarquée.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct PackRef {
    #[schemars(with = "String")]
    pub id: Uuid,
    pub version: PackVersion,
}

/// Les fichiers d'une version : `pack.json` (un objet) et `widgets.typ`.
#[derive(Debug, Clone, PartialEq)]
pub struct PackFiles {
    manifest: Map<String, Value>,
    widgets: Option<String>,
}

impl PackFiles {
    /// Un `widgets` vide vaut absent : même contenu canonique.
    pub fn new(manifest: Map<String, Value>, widgets: Option<String>) -> Self {
        Self { manifest, widgets: widgets.filter(|w| !w.is_empty()) }
    }

    /// Depuis des octets lus (zip, disque) ; `None` si `pack.json` n'est pas
    /// un objet JSON ou si `widgets.typ` n'est pas de l'UTF-8.
    pub fn from_bytes(pack_json: &[u8], widgets: Option<&[u8]>) -> Option<Self> {
        let Ok(Value::Object(manifest)) = serde_json::from_slice(pack_json) else { return None };
        let widgets = match widgets {
            Some(bytes) => Some(String::from_utf8(bytes.to_vec()).ok()?),
            None => None,
        };
        Some(Self::new(manifest, widgets))
    }

    /// Le `pack.json`, tel que reçu.
    pub fn manifest(&self) -> &Map<String, Value> {
        &self.manifest
    }

    /// Le `widgets.typ` ; `None` pour un pack sans widgets.
    pub fn widgets(&self) -> Option<&str> {
        self.widgets.as_deref()
    }

    /// La version de ces fichiers.
    pub fn version(&self) -> PackVersion {
        PackVersion::of(self)
    }

    /// `pack.json` à écrire : indenté, clés dans l'ordre reçu.
    pub fn pack_json(&self) -> serde_json::Result<Vec<u8>> {
        serde_json::to_vec_pretty(&self.manifest)
    }

    /// Contenu canonique : `pack.json` compact à clés triées, `0`, `widgets.typ`.
    pub fn canonical(&self) -> Vec<u8> {
        // Un `Value` s'écrit toujours : ses clés sont des chaînes.
        let mut out = serde_json::to_vec(&sorted(&Value::Object(self.manifest.clone()))).unwrap_or_default();
        out.push(0);
        out.extend_from_slice(self.widgets.as_deref().unwrap_or_default().as_bytes());
        out
    }
}

/// `v` aux clés triées à toute profondeur (`preserve_order` garde l'ordre d'insertion).
fn sorted(v: &Value) -> Value {
    match v {
        Value::Object(m) => {
            let mut entries: Vec<(&String, &Value)> = m.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            Value::Object(entries.into_iter().map(|(k, v)| (k.clone(), sorted(v))).collect())
        }
        Value::Array(a) => Value::Array(a.iter().map(sorted).collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
#[path = "pack_tests.rs"]
mod pack_tests;
```

`crates/oku-slides-core/src/lib.rs` : `pub mod pack;` (après `pub mod ops;`), `pub use pack::{PackFiles, PackRef, PackVersion};`, `PackVersionError` ajouté au `pub use error::{…}`. Doc du module : une puce « [`PackRef`], [`PackVersion`], [`PackFiles`] : pack de modèles d'un deck, version adressée par contenu. »

- [ ] **Step 4 : lancer** — `cargo nextest run -p oku-slides-core` → PASS ; `cargo build --workspace` → PASS (rien d'autre ne change).

- [ ] **Step 5 : commit**

```bash
git add Cargo.toml Cargo.lock crates/oku-slides-core
```
```bash
git commit -m "feat(core) : versions de pack adressées par contenu, référence et fichiers d'un pack (t-328)"
```

---

### Task 2 : bibliothèque versionnée (session, à côté de l'ancienne)

**Files:**
- Create: `crates/oku-slides-session/src/pack.rs`, `pack_tests.rs`, `library.rs`, `library_tests.rs`
- Modify: `crates/oku-slides-session/src/templates.rs` (`Template` déplacé dans `pack.rs` ; `DEFAULT_SET` = l'UUID de `DEFAULT_PACK`)
- Modify: `crates/oku-slides-session/src/lib.rs:36-58` (mods), `:73` (réexports)
- Modify: `crates/oku-slides-session/src/error.rs` (`UnknownPack`, `PackVersionMissing`, `PackJson`)
- Modify: `crates/oku-slides-session/src/test_support.rs` (aides de packs)
- Modify: `crates/oku-slides-session/Cargo.toml` (`uuid.workspace = true`)
- Modify: `templates/essentiel/pack.json`, `templates/grilles/pack.json`
- Modify (ids intégrés écrits en dur) : `crates/oku-slides-session/src/templates_tests.rs:111,166-167`, `tests/errors.rs:124,133,147`, `tests/session.rs:228,230,264,265,267,314,333`, `tests/merge.rs:86`, `tests/snapshots/agent__agent_context_describes_mode_slide_selection_and_drains_notes.snap:8`
- Modify: `crates/oku-slides-i18n/src/locales/*/session.ftl` (20 langues)

**Interfaces:**
- Consumes : `PackFiles`, `PackVersion` (Task 1).
- Produces (`oku_slides_session::pack`, réexportés à la racine) :
  - `pub const DEFAULT_PACK: Uuid`, `pub const GRID_PACK: Uuid`.
  - `pub struct Template { pub id: String, pub name: String, pub description: String, pub slide: Slide }` (déplacé, inchangé).
  - `pub struct PackManifest { pub id: Uuid, pub name: String, pub description: String, pub parent: Option<PackVersion>, pub created: String, pub templates: Vec<Template> }` ; `PackManifest::from_files(&PackFiles) -> serde_json::Result<PackManifest>`, `PackManifest::to_files(&self, widgets: Option<String>) -> serde_json::Result<PackFiles>`, `PackManifest::info(&self, version: PackVersion, builtin: bool) -> PackInfo`.
  - `pub struct PackInfo { pub id: Uuid, pub version: PackVersion, pub name: String, pub description: String, pub builtin: bool, pub default: bool, pub count: usize }` (`Serialize`).
  - `pub enum Relation { Same, Ancestor, Descendant, Sibling }` (`Serialize`, `snake_case`) : ce qu'est `a` par rapport à `b`.
  - `pub fn now_created() -> String`.
- Produces (`oku_slides_session::library`, réexporté à la racine à la Task 5 seulement) :
  - `pub struct StoredVersion { pub version: PackVersion, pub files: PackFiles, pub manifest: PackManifest }`, `pub fn builtin_versions() -> &'static [StoredVersion]`.
  - `Library::new(Option<PathBuf>, Storage)`, `Library::is_builtin(Uuid) -> bool`, `packs(&self) -> Vec<PackInfo>`, `versions(&self, Uuid) -> Vec<StoredVersion>`, `version(&self, Uuid, &PackVersion) -> Option<PackFiles>`, `current(&self, Uuid) -> Option<PackVersion>`, `current_version(&self, Uuid) -> Option<StoredVersion>`, `add_version(&self, Uuid, &PackFiles) -> Result<bool>`, `set_current(&self, Uuid, &PackVersion) -> Result<()>`, `replace_version(&self, Uuid, old: &PackVersion, &PackFiles) -> Result<PackVersion>` (ajoute la nouvelle, la rend courante, supprime `old` de la bibliothèque sauf version intégrée), `relation(&self, Uuid, &PackVersion, &PackVersion) -> Relation`, `delete(&self, Uuid) -> Result<()>`.
  - `SessionErrorKind::{UnknownPack, PackVersionMissing}` ; `PackJson` → `InvalidDocument`.
  - `test_support::{manifest, add_version}` (tests).

- [ ] **Step 1 : tests en échec**

`crates/oku-slides-session/src/test_support.rs`, à la fin :

```rust
/// Un `pack.json` sans modèles : pack `id`, nom `name`, parente `parent`, créé à `created`.
pub(crate) fn manifest(
    id: uuid::Uuid,
    name: &str,
    parent: Option<&oku_slides_core::PackVersion>,
    created: &str,
) -> crate::pack::PackManifest {
    crate::pack::PackManifest {
        id,
        name: name.into(),
        description: String::new(),
        parent: parent.cloned(),
        created: created.into(),
        templates: Vec::new(),
    }
}

/// Ajoute `manifest` (et `widgets`) à `library` ; sa référence.
pub(crate) fn add_version(
    library: &crate::library::Library,
    manifest: &crate::pack::PackManifest,
    widgets: Option<&str>,
) -> oku_slides_core::PackRef {
    let files = manifest.to_files(widgets.map(str::to_owned)).unwrap_or_else(|e| panic!("pack.json : {e}"));
    library.add_version(manifest.id, &files).unwrap_or_else(|e| panic!("version : {}", e.one_line()));
    oku_slides_core::PackRef { id: manifest.id, version: files.version() }
}
```

`crates/oku-slides-session/src/pack_tests.rs` :

```rust
use speculoos::prelude::*;
use uuid::Uuid;

use super::*;
use crate::test_support::manifest;

#[test]
fn pack_manifest_round_trips_through_pack_files() {
    let m = manifest(Uuid::new_v4(), "Mien", None, "2026-10-01T10:00:00Z");
    let files = m.to_files(Some("#let registry = ()".into())).unwrap();
    assert_that!(PackManifest::from_files(&files).unwrap()).is_equal_to(&m);
    assert_that!(files.widgets()).is_equal_to(Some("#let registry = ()"));
}

#[test]
fn pack_manifest_without_uuid_id_is_refused() {
    let json = serde_json::json!({ "id": "essentiel", "name": "E", "parent": null, "created": "x", "templates": [] });
    let files = oku_slides_core::PackFiles::new(json.as_object().cloned().unwrap(), None);
    assert_that!(PackManifest::from_files(&files).is_err()).is_true();
}

#[test]
fn pack_manifest_info_marks_the_default_pack() {
    let m = manifest(DEFAULT_PACK, "Essentiel", None, "2026-10-01T00:00:00Z");
    let v = m.to_files(None).unwrap().version();
    let info = m.info(v.clone(), true);
    assert_that!((info.id, info.version, info.default, info.builtin)).is_equal_to((DEFAULT_PACK, v, true, true));
}

#[test]
fn now_created_is_utc_to_the_second() {
    let s = now_created();
    assert_that!(s.len()).is_equal_to(20);
    assert_that!(s.ends_with('Z') && s.as_bytes()[10] == b'T').is_true();
}
```

`crates/oku-slides-session/src/library_tests.rs` :

```rust
use std::path::Path;

use oku_slides_core::PackVersion;
use speculoos::prelude::*;
use uuid::Uuid;

use super::*;
use crate::pack::{DEFAULT_PACK, GRID_PACK, Relation};
use crate::test_support::{add_version, fresh_dir, manifest};

fn library(dir: &Path) -> Library {
    Library::new(Some(dir.to_owned()), Storage::local().unwrap())
}

#[test]
fn builtin_versions_have_the_constant_ids() {
    let ids: Vec<Uuid> = builtin_versions().iter().map(|b| b.manifest.id).collect();
    assert_that!(ids).is_equal_to(vec![DEFAULT_PACK, GRID_PACK]);
    for b in builtin_versions() {
        assert_that!(b.manifest.parent).is_none();
        assert_that!(b.manifest.created.as_str()).is_equal_to("2026-10-01T00:00:00Z");
    }
}

#[test]
fn builtin_pack_is_current_by_default_and_read_only() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let builtin = builtin_versions()[0].version.clone();
    assert_that!(lib.current(DEFAULT_PACK)).is_equal_to(Some(builtin.clone()));
    assert_that!(Library::is_builtin(DEFAULT_PACK)).is_true();
    // Rien n'est écrit pour une version intégrée.
    assert_that!(dir.path().join(DEFAULT_PACK.to_string()).exists()).is_false();
    assert_that!(lib.version(DEFAULT_PACK, &builtin).is_some()).is_true();
}

#[test]
fn add_version_of_unknown_pack_creates_it_with_that_version_current() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let id = Uuid::new_v4();
    let r = add_version(&lib, &manifest(id, "Mien", None, "2026-10-01T10:00:00Z"), Some("#let registry = ()"));
    assert_that!(lib.current(id)).is_equal_to(Some(r.version.clone()));
    let on_disk = dir.path().join(id.to_string()).join("versions").join(r.version.hex());
    assert_that!(on_disk.join("pack.json").exists() && on_disk.join("widgets.typ").exists()).is_true();
}

#[test]
fn add_version_does_not_change_current() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let id = Uuid::new_v4();
    let first = add_version(&lib, &manifest(id, "A", None, "2026-10-01T10:00:00Z"), None);
    // Plus récente : la date ne fait pas la version courante.
    let later = add_version(&lib, &manifest(id, "B", Some(&first.version), "2026-10-02T10:00:00Z"), None);
    assert_that!(lib.current(id)).is_equal_to(Some(first.version));
    assert_that!(lib.versions(id).len()).is_equal_to(2);
    assert_that!(lib.add_version(id, &lib.version(id, &later.version).unwrap()).unwrap()).is_false();
}

#[test]
fn add_version_under_a_builtin_id_keeps_the_builtin_current() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let builtin = builtin_versions()[0].version.clone();
    add_version(&lib, &manifest(DEFAULT_PACK, "Sœur", Some(&builtin), "2026-10-02T10:00:00Z"), None);
    assert_that!(lib.current(DEFAULT_PACK)).is_equal_to(Some(builtin));
    assert_that!(lib.versions(DEFAULT_PACK).len()).is_equal_to(2);
}

#[test]
fn set_current_makes_a_version_current_even_over_a_builtin() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let builtin = builtin_versions()[0].version.clone();
    let mine = add_version(&lib, &manifest(DEFAULT_PACK, "Le mien", Some(&builtin), "2026-10-02T10:00:00Z"), None);
    lib.set_current(DEFAULT_PACK, &mine.version).unwrap();
    assert_that!(lib.current(DEFAULT_PACK)).is_equal_to(Some(mine.version));
}

/// Une session d'édition des modèles n'a qu'une version : chaque enregistrement remplace la précédente.
#[test]
fn replace_version_keeps_the_parent_removes_the_old_one_and_updates_current() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let builtin = builtin_versions()[0].version.clone();
    let first = add_version(&lib, &manifest(DEFAULT_PACK, "Un", Some(&builtin), "2026-10-02T10:00:00Z"), None);
    lib.set_current(DEFAULT_PACK, &first.version).unwrap();
    let next = manifest(DEFAULT_PACK, "Deux", Some(&builtin), "2026-10-02T10:05:00Z").to_files(None).unwrap();
    let replaced = lib.replace_version(DEFAULT_PACK, &first.version, &next).unwrap();
    assert_that!(replaced).is_equal_to(next.version());
    assert_that!(lib.current(DEFAULT_PACK)).is_equal_to(Some(next.version()));
    assert_that!(lib.version(DEFAULT_PACK, &first.version)).is_none();
    assert_that!(dir.path().join(DEFAULT_PACK.to_string()).join("versions").join(first.version.hex()).exists()).is_false();
    let current = lib.current_version(DEFAULT_PACK).unwrap();
    assert_that!(current.manifest.parent).is_equal_to(Some(builtin));
    assert_that!(lib.versions(DEFAULT_PACK).len()).is_equal_to(2);
}

#[test]
fn replace_version_never_deletes_the_builtin_version() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let builtin = builtin_versions()[0].version.clone();
    let next = manifest(DEFAULT_PACK, "Mien", Some(&builtin), "2026-10-02T10:00:00Z").to_files(None).unwrap();
    lib.replace_version(DEFAULT_PACK, &builtin, &next).unwrap();
    assert_that!(lib.version(DEFAULT_PACK, &builtin).is_some()).is_true();
    assert_that!(lib.current(DEFAULT_PACK)).is_equal_to(Some(next.version()));
}

#[test]
fn set_current_to_an_absent_version_returns_pack_version_missing() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let absent = PackVersion::from_hex(&"0".repeat(64)).unwrap();
    let err = lib.set_current(DEFAULT_PACK, &absent).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::PackVersionMissing);
    let err = lib.set_current(Uuid::new_v4(), &absent).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::UnknownPack);
}

#[test]
fn current_pointing_to_an_absent_version_falls_back_to_the_latest() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let id = Uuid::new_v4();
    let a = add_version(&lib, &manifest(id, "A", None, "2026-10-01T10:00:00Z"), None);
    let b = add_version(&lib, &manifest(id, "B", Some(&a.version), "2026-10-03T10:00:00Z"), None);
    std::fs::write(dir.path().join(id.to_string()).join("current"), format!("sha256:{}", "e".repeat(64))).unwrap();
    assert_that!(lib.current(id)).is_equal_to(Some(b.version));
}

#[test]
fn version_folder_whose_content_differs_is_ignored() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let id = Uuid::new_v4();
    let a = add_version(&lib, &manifest(id, "A", None, "2026-10-01T10:00:00Z"), None);
    let forged = dir.path().join(id.to_string()).join("versions").join("f".repeat(64));
    std::fs::create_dir_all(&forged).unwrap();
    std::fs::copy(
        dir.path().join(id.to_string()).join("versions").join(a.version.hex()).join("pack.json"),
        forged.join("pack.json"),
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join(id.to_string()).join("versions").join("pas-une-version")).unwrap();
    assert_that!(lib.versions(id).len()).is_equal_to(1);
}

#[rstest::rstest]
#[case::same(0, 0, Relation::Same)]
#[case::older(0, 1, Relation::Ancestor)]
#[case::newer(1, 0, Relation::Descendant)]
#[case::grand_child(2, 0, Relation::Descendant)]
#[case::parallel(1, 3, Relation::Sibling)]
fn relation_follows_the_parent_chain(#[case] a: usize, #[case] b: usize, #[case] expected: Relation) {
    // v0 ← v1 ← v2 ; v0 ← v3
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let id = Uuid::new_v4();
    let v0 = add_version(&lib, &manifest(id, "v0", None, "2026-10-01T10:00:00Z"), None).version;
    let v1 = add_version(&lib, &manifest(id, "v1", Some(&v0), "2026-10-02T10:00:00Z"), None).version;
    let v2 = add_version(&lib, &manifest(id, "v2", Some(&v1), "2026-10-03T10:00:00Z"), None).version;
    let v3 = add_version(&lib, &manifest(id, "v3", Some(&v0), "2026-10-04T10:00:00Z"), None).version;
    let v = [v0, v1, v2, v3];
    assert_that!(lib.relation(id, &v[a], &v[b])).is_equal_to(expected);
}

#[test]
fn packs_lists_builtins_first_then_user_packs_by_name() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let (z, a) = (Uuid::new_v4(), Uuid::new_v4());
    add_version(&lib, &manifest(z, "Zeta", None, "2026-10-01T10:00:00Z"), None);
    add_version(&lib, &manifest(a, "alpha", None, "2026-10-01T10:00:00Z"), None);
    std::fs::create_dir_all(dir.path().join("pas-un-uuid")).unwrap();
    let ids: Vec<(Uuid, bool, bool)> = lib.packs().into_iter().map(|p| (p.id, p.builtin, p.default)).collect();
    assert_that!(ids).is_equal_to(vec![(DEFAULT_PACK, true, true), (GRID_PACK, true, false), (a, false, false), (z, false, false)]);
}

#[test]
fn delete_removes_every_version_of_a_user_pack() {
    let dir = fresh_dir("oku-slides-library-");
    let lib = library(dir.path());
    let id = Uuid::new_v4();
    add_version(&lib, &manifest(id, "A", None, "2026-10-01T10:00:00Z"), None);
    lib.delete(id).unwrap();
    assert_that!(lib.current(id)).is_none();
    assert_that!(dir.path().join(id.to_string()).exists()).is_false();
}

#[test]
fn add_version_without_templates_dir_returns_no_templates_dir() {
    let lib = Library::new(None, Storage::local().unwrap());
    let files = manifest(Uuid::new_v4(), "A", None, "2026-10-01T10:00:00Z").to_files(None).unwrap();
    let err = lib.add_version(Uuid::new_v4(), &files).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::NoTemplatesDir);
}
```

Ajouter `rstest.workspace = true` aux `[dev-dependencies]` de la session s'il manque (il manque aujourd'hui).

- [ ] **Step 2 : lancer** — `cargo nextest run -p oku-slides-session library pack` → FAIL (modules absents).

- [ ] **Step 3 : implémenter**

`templates/essentiel/pack.json` : la ligne `"id": "essentiel",` devient `"id": "6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10",` ; après la ligne `"description": …,` insérer `"parent": null,` puis `"created": "2026-10-01T00:00:00Z",`. Même chose pour `templates/grilles/pack.json` avec `"id": "b8e2d4f6-1a3c-4e5b-8d7f-9c0a2e4b6d81",`.

`crates/oku-slides-session/src/pack.rs` :

```rust
//! Le contenu d'un pack de modèles (`pack.json`) et son résumé.
//!
//! Un pack regroupe des modèles (des slides complètes, nommées). Il est
//! indépendant du thème : ses modèles ne parlent qu'en jetons.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use time::macros::format_description;
use uuid::Uuid;

use oku_slides_core::{PackFiles, PackVersion, Slide};

/// Pack par défaut, `templates/essentiel` : id constant.
pub const DEFAULT_PACK: Uuid = Uuid::from_u128(0x6f3c1e2a_8b4d_4c1e_9a57_2d0e5b7c9f10);
/// Pack intégré `templates/grilles` : id constant.
pub const GRID_PACK: Uuid = Uuid::from_u128(0xb8e2d4f6_1a3c_4e5b_8d7f_9c0a2e4b6d81);

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
}

impl PackManifest {
    /// Lit le `pack.json` de `files`.
    pub fn from_files(files: &PackFiles) -> serde_json::Result<Self> {
        serde_json::from_value(Value::Object(files.manifest().clone()))
    }

    /// Les fichiers de cette version, avec `widgets` pour `widgets.typ`.
    pub fn to_files(&self, widgets: Option<String>) -> serde_json::Result<PackFiles> {
        match serde_json::to_value(self)? {
            Value::Object(manifest) => Ok(PackFiles::new(manifest, widgets)),
            _ => Err(serde::ser::Error::custom("pack manifest is not an object")),
        }
    }

    /// Résumé de cette version pour les listes.
    pub fn info(&self, version: PackVersion, builtin: bool) -> PackInfo {
        PackInfo {
            id: self.id,
            version,
            name: self.name.clone(),
            description: self.description.clone(),
            builtin,
            default: self.id == DEFAULT_PACK,
            count: self.templates.len(),
        }
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

/// Maintenant, au format de `created`.
pub fn now_created() -> String {
    OffsetDateTime::now_utc()
        .format(format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]Z"))
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "pack_tests.rs"]
mod pack_tests;
```

`crates/oku-slides-session/src/library.rs` :

```rust
//! Bibliothèque des packs de modèles, versionnée.
//!
//! Un pack est un id et des versions adressées par contenu ; chaque version
//! connaît sa parente (`parent` de son `pack.json`). Sur disque :
//!
//! ```text
//! <data>/templates/<uuid>/current                     version courante
//! <data>/templates/<uuid>/versions/<hex>/pack.json
//! <data>/templates/<uuid>/versions/<hex>/widgets.typ  (facultatif)
//! ```
//!
//! Packs intégrés (`templates/essentiel`, `templates/grilles` du dépôt) :
//! id constant, version de l'app en lecture seule, courante tant que
//! l'utilisateur n'a pas de fichier `current`. La version courante ne change
//! que par [`Library::set_current`], ou à la création d'un pack inconnu.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::{PackFiles, PackVersion};
use oku_slides_platform::errors::one_line;
use oku_slides_platform::storage::Storage;

use crate::Result;
use crate::error::{NoTemplatesDirSnafu, PackJsonSnafu, PackVersionMissingSnafu, UnknownPackSnafu, WriteSnafu};
use crate::pack::{PackInfo, PackManifest, Relation};

/// Version courante d'un pack, dans son dossier.
const CURRENT_FILE: &str = "current";
/// Versions d'un pack, dans son dossier.
const VERSIONS_DIR: &str = "versions";
const PACK_FILE: &str = "pack.json";
const WIDGETS_FILE: &str = "widgets.typ";

/// Une version lue et vérifiée : empreinte, fichiers, `pack.json` lu.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredVersion {
    pub version: PackVersion,
    pub files: PackFiles,
    pub manifest: PackManifest,
}

impl StoredVersion {
    /// `files` lus ; sinon l'erreur, sur une ligne.
    fn read(files: PackFiles) -> std::result::Result<Self, String> {
        let manifest = PackManifest::from_files(&files).map_err(|e| e.to_string())?;
        Ok(Self { version: files.version(), files, manifest })
    }
}

/// Packs intégrés, lus une fois par processus, dans l'ordre.
static BUILTIN: LazyLock<Vec<StoredVersion>> = LazyLock::new(|| {
    [include_str!("../../../templates/essentiel/pack.json"), include_str!("../../../templates/grilles/pack.json")]
        .iter()
        .filter_map(|json| {
            let read = PackFiles::from_bytes(json.as_bytes(), None)
                .ok_or_else(|| "pack.json is not a json object".to_owned())
                .and_then(StoredVersion::read);
            // Couvert par les tests : n'arrive qu'avec un fichier du dépôt cassé.
            read.inspect_err(|e| tracing::error!(error = %e, "pack intégré illisible")).ok()
        })
        .collect()
});

/// Versions des packs intégrés, dans l'ordre.
pub fn builtin_versions() -> &'static [StoredVersion] {
    &BUILTIN
}

/// La plus récente par `created` (la dernière vue à égalité).
fn latest(versions: &[StoredVersion]) -> Option<PackVersion> {
    versions.iter().max_by(|a, b| a.manifest.created.cmp(&b.manifest.created)).map(|s| s.version.clone())
}

/// `child` descend-il de `ancestor` ? Un cycle (fichiers forgés) s'arrête.
fn descends(parents: &HashMap<PackVersion, Option<PackVersion>>, child: &PackVersion, ancestor: &PackVersion) -> bool {
    let mut seen = HashSet::new();
    let mut at = parents.get(child).cloned().flatten();
    while let Some(v) = at {
        if v == *ancestor {
            return true;
        }
        if !seen.insert(v.clone()) {
            return false;
        }
        at = parents.get(&v).cloned().flatten();
    }
    false
}

/// Le fichier `current` d'un pack.
enum Current {
    Absent,
    At(PackVersion),
    /// Illisible ou mal formé.
    Broken,
}

/// Bibliothèque : packs intégrés et packs de l'utilisateur, versionnés.
#[derive(Debug, Clone)]
pub struct Library {
    dir: Option<PathBuf>,
    storage: Storage,
}

impl Library {
    /// `dir` : `<data>/templates`, créé à la première écriture.
    pub fn new(dir: Option<PathBuf>, storage: Storage) -> Self {
        Self { dir, storage }
    }

    /// Un pack intégré à l'app ?
    pub fn is_builtin(id: Uuid) -> bool {
        BUILTIN.iter().any(|b| b.manifest.id == id)
    }

    fn builtin(id: Uuid) -> Option<&'static StoredVersion> {
        BUILTIN.iter().find(|b| b.manifest.id == id)
    }

    /// Dossier du pack `id` ; `None` sans dossier des modèles.
    fn pack_dir(&self, id: Uuid) -> Option<PathBuf> {
        Some(self.dir.as_ref()?.join(id.to_string()))
    }

    /// Versions de l'utilisateur ; une version illisible ou altérée est journalisée et ignorée.
    fn user_versions(&self, id: Uuid) -> Vec<StoredVersion> {
        let Some(dir) = self.pack_dir(id).map(|d| d.join(VERSIONS_DIR)) else { return Vec::new() };
        let folders = self.storage.list_dirs(&dir).unwrap_or_else(|e| {
            tracing::warn!(dir = %dir.display(), error = %one_line(&e), "versions du pack illisibles");
            Vec::new()
        });
        folders
            .iter()
            .filter_map(|folder| {
                let read = self.read_version(folder).and_then(|s| {
                    if s.manifest.id == id { Ok(s) } else { Err(format!("version of pack {}", s.manifest.id)) }
                });
                read.inspect_err(|e| tracing::warn!(dir = %folder.display(), error = %e, "version de pack ignorée")).ok()
            })
            .collect()
    }

    /// Une version : son dossier s'appelle `<hex>` et son contenu y correspond.
    fn read_version(&self, folder: &Path) -> std::result::Result<StoredVersion, String> {
        let name = folder.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        let expected = PackVersion::from_hex(name).ok_or_else(|| format!("not a version folder: {name}"))?;
        let pack = self.storage.read(&folder.join(PACK_FILE)).map_err(|e| one_line(&e))?;
        let path = folder.join(WIDGETS_FILE);
        let widgets = match self.storage.len(&path).map_err(|e| one_line(&e))? {
            Some(_) => Some(self.storage.read(&path).map_err(|e| one_line(&e))?),
            None => None,
        };
        let files = PackFiles::from_bytes(&pack, widgets.as_deref()).ok_or_else(|| "unreadable pack files".to_owned())?;
        let stored = StoredVersion::read(files)?;
        if stored.version != expected {
            return Err(format!("content does not match {expected}"));
        }
        Ok(stored)
    }

    /// Les versions du pack `id` : l'intégrée d'abord, puis celles de l'utilisateur.
    pub fn versions(&self, id: Uuid) -> Vec<StoredVersion> {
        let mut out: Vec<StoredVersion> = Self::builtin(id).into_iter().cloned().collect();
        for v in self.user_versions(id) {
            if !out.iter().any(|o| o.version == v.version) {
                out.push(v);
            }
        }
        out
    }

    /// Les fichiers de la version `v` du pack `id`.
    pub fn version(&self, id: Uuid, v: &PackVersion) -> Option<PackFiles> {
        self.versions(id).into_iter().find(|s| s.version == *v).map(|s| s.files)
    }

    /// La version courante du pack `id` ; `None` pour un pack inconnu.
    pub fn current(&self, id: Uuid) -> Option<PackVersion> {
        self.current_version(id).map(|s| s.version)
    }

    /// La version courante, lue. `current` absent : l'intégrée, sinon la plus
    /// récente ; `current` vers une version absente : la plus récente.
    pub fn current_version(&self, id: Uuid) -> Option<StoredVersion> {
        let versions = self.versions(id);
        let chosen = match self.read_current(id) {
            Current::At(v) if versions.iter().any(|s| s.version == v) => Some(v),
            Current::Absent => Self::builtin(id).map(|b| b.version.clone()).or_else(|| latest(&versions)),
            Current::At(_) | Current::Broken => {
                tracing::warn!(pack = %id, "version courante absente : la plus récente la remplace");
                latest(&versions)
            }
        }?;
        versions.into_iter().find(|s| s.version == chosen)
    }

    fn read_current(&self, id: Uuid) -> Current {
        let Some(path) = self.pack_dir(id).map(|d| d.join(CURRENT_FILE)) else { return Current::Absent };
        let read = match self.storage.len(&path) {
            Ok(None) => return Current::Absent,
            Ok(Some(_)) => self.storage.read(&path),
            Err(e) => Err(e),
        };
        match read {
            Ok(bytes) => String::from_utf8(bytes)
                .ok()
                .and_then(|s| PackVersion::parse(s.trim()))
                .map_or(Current::Broken, Current::At),
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %one_line(&e), "version courante illisible");
                Current::Broken
            }
        }
    }

    /// Ajoute une version sans toucher la version courante ; vrai si elle
    /// était nouvelle. Un pack inconnu est créé, cette version courante.
    pub fn add_version(&self, id: Uuid, files: &PackFiles) -> Result<bool> {
        let dir = self.pack_dir(id).context(NoTemplatesDirSnafu)?;
        let version = files.version();
        let known = self.versions(id);
        if known.iter().any(|s| s.version == version) {
            return Ok(false);
        }
        let at = dir.join(VERSIONS_DIR).join(version.hex());
        // `widgets.typ` d'abord : un `pack.json` présent dit la version complète.
        if let Some(source) = files.widgets() {
            let path = at.join(WIDGETS_FILE);
            self.storage.write_atomic(&path, source.as_bytes()).context(WriteSnafu { path: &path })?;
        }
        let path = at.join(PACK_FILE);
        let json = files.pack_json().context(PackJsonSnafu { id })?;
        self.storage.write_atomic(&path, json).context(WriteSnafu { path: &path })?;
        if known.is_empty() {
            self.write_current(id, &version)?;
        }
        Ok(true)
    }

    /// Rend la version `v` courante pour le pack `id`.
    pub fn set_current(&self, id: Uuid, v: &PackVersion) -> Result<()> {
        let versions = self.versions(id);
        ensure!(!versions.is_empty(), UnknownPackSnafu { id });
        ensure!(versions.iter().any(|s| s.version == *v), PackVersionMissingSnafu { id, version: v.clone() });
        self.write_current(id, v)
    }

    /// Remplace la version `old` par `files`, rendue courante ; `old` quitte
    /// la bibliothèque (jamais une version intégrée). Une session d'édition
    /// des modèles n'a ainsi qu'une version. Rend la nouvelle version.
    pub fn replace_version(&self, id: Uuid, old: &PackVersion, files: &PackFiles) -> Result<PackVersion> {
        let version = files.version();
        self.add_version(id, files)?;
        self.set_current(id, &version)?;
        let builtin = Self::builtin(id).is_some_and(|b| b.version == *old);
        if *old != version && !builtin {
            let dir = self.pack_dir(id).context(NoTemplatesDirSnafu)?.join(VERSIONS_DIR).join(old.hex());
            self.storage.delete_dir(&dir).context(WriteSnafu { path: dir })?;
        }
        Ok(version)
    }

    fn write_current(&self, id: Uuid, v: &PackVersion) -> Result<()> {
        let path = self.pack_dir(id).context(NoTemplatesDirSnafu)?.join(CURRENT_FILE);
        self.storage.write_atomic(&path, v.as_str().as_bytes()).context(WriteSnafu { path: &path })?;
        Ok(())
    }

    /// `a` par rapport à `b` dans le pack `id`, par la chaîne des `parent`.
    pub fn relation(&self, id: Uuid, a: &PackVersion, b: &PackVersion) -> Relation {
        if a == b {
            return Relation::Same;
        }
        let parents: HashMap<PackVersion, Option<PackVersion>> =
            self.versions(id).into_iter().map(|s| (s.version, s.manifest.parent)).collect();
        if descends(&parents, b, a) {
            Relation::Ancestor
        } else if descends(&parents, a, b) {
            Relation::Descendant
        } else {
            Relation::Sibling
        }
    }

    /// Tous les packs dans leur version courante : intégrés d'abord, puis
    /// ceux de l'utilisateur, par nom.
    pub fn packs(&self) -> Vec<PackInfo> {
        let mut out: Vec<PackInfo> = BUILTIN.iter().filter_map(|b| self.info(b.manifest.id)).collect();
        let mut user: Vec<PackInfo> =
            self.user_ids().into_iter().filter(|id| !Self::is_builtin(*id)).filter_map(|id| self.info(id)).collect();
        user.sort_by_key(|p| p.name.to_lowercase());
        out.extend(user);
        out
    }

    fn info(&self, id: Uuid) -> Option<PackInfo> {
        let s = self.current_version(id)?;
        Some(s.manifest.info(s.version, Self::is_builtin(id)))
    }

    /// Ids des packs de l'utilisateur : dossiers nommés par un UUID canonique.
    fn user_ids(&self) -> Vec<Uuid> {
        let Some(dir) = &self.dir else { return Vec::new() };
        let folders = self.storage.list_dirs(dir).unwrap_or_else(|e| {
            tracing::warn!(dir = %dir.display(), error = %one_line(&e), "packs de modèles illisibles");
            Vec::new()
        });
        folders
            .iter()
            .filter_map(|f| {
                let name = f.file_name()?.to_str()?;
                let id = Uuid::parse_str(name).ok()?;
                // Un seul nom par pack : la forme canonique (minuscules, tirets).
                (id.to_string() == name).then_some(id)
            })
            .collect()
    }

    /// Supprime le pack `id` de l'utilisateur, toutes versions comprises.
    pub fn delete(&self, id: Uuid) -> Result<()> {
        let dir = self.pack_dir(id).context(NoTemplatesDirSnafu)?;
        self.storage.delete_dir(&dir).context(WriteSnafu { path: dir })?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "library_tests.rs"]
mod library_tests;
```

`crates/oku-slides-session/src/error.rs` — `SessionErrorKind` gagne (après `NoTemplatesDir`) :

```rust
    /// Pack de modèles inconnu.
    UnknownPack,
    /// Version de pack introuvable.
    PackVersionMissing,
```

`SessionInner` gagne :

```rust
    #[snafu(display("template pack not found: {id}"))]
    UnknownPack { id: uuid::Uuid, backtrace: Backtrace },
    #[snafu(display("version {version} of template pack {id} not found"))]
    PackVersionMissing { id: uuid::Uuid, version: oku_slides_core::PackVersion, backtrace: Backtrace },
    #[snafu(display("invalid pack.json for template pack {id}"))]
    PackJson { id: uuid::Uuid, source: serde_json::Error, backtrace: Backtrace },
```

`kind()` : `UnknownPack { .. } => SessionErrorKind::UnknownPack`, `PackVersionMissing { .. } => SessionErrorKind::PackVersionMissing`, `PackJson { .. }` rejoint la branche `InvalidDocument`. `localize` :

```rust
            UnknownPack { id, .. } => l.tr_with("session-unknown-pack", &[("id", id.to_string().into())]),
            PackVersionMissing { id, version, .. } => l.tr_with(
                "session-pack-version-missing",
                &[("id", id.to_string().into()), ("version", version.as_str().into())],
            ),
            PackJson { id, .. } => l.tr_with("session-invalid-pack", &[("id", id.to_string().into())]),
```

`crates/oku-slides-session/src/templates.rs` : supprimer la `struct Template` (lignes 51-58) et ajouter `pub use crate::pack::Template;` ; la ligne 34 devient

```rust
pub const DEFAULT_SET: &str = "6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10";
```

(l'ancienne bibliothèque lit les `pack.json` intégrés, dont l'id est désormais cet UUID ; elle disparaît à la Task 5). Ajouter à `templates_tests.rs` :

```rust
#[test]
fn default_set_is_the_default_pack() {
    assert_that!(DEFAULT_SET.to_owned()).is_equal_to(crate::pack::DEFAULT_PACK.to_string());
}
```

`lib.rs` : `pub mod library;` et `pub mod pack;` (ordre alphabétique des `mod`, lignes 36-58) ; ligne 73 : `pub use pack::{DEFAULT_PACK, GRID_PACK, PackInfo, PackManifest, Relation, Template};` et `pub use templates::{DEFAULT_SET, Library, SetInfo, TemplateSet};` (le `Library` exporté reste l'ancien jusqu'à la Task 5). `Cargo.toml` de session : `uuid.workspace = true` dans `[dependencies]`, `rstest.workspace = true` dans `[dev-dependencies]`.

Ids intégrés écrits en dur dans les tests : `"essentiel"` → `DEFAULT_SET` (ou `oku_slides_session::DEFAULT_SET`), `"grilles"` → `GRID_PACK.to_string()` (`oku_slides_session::GRID_PACK.to_string()` dans `tests/`), aux lignes listées dans **Files**. `templates_tests.rs:166-167` attend `(DEFAULT_SET.to_owned(), true)` puis `(crate::pack::GRID_PACK.to_string(), true)`. Instantané `agent__agent_context…snap:8` : `template_set: 6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10` (relancer avec `INSTA_UPDATE=always`, relire le diff : seule cette ligne change).

Fluent — `session.ftl` de **chaque** langue (fr et en ci-dessous ; traduire les 18 autres) :

```ftl
# fr
session-unknown-pack = Pack de modèles introuvable : { $id }
session-pack-version-missing = La version { $version } du pack de modèles { $id } est introuvable
session-invalid-pack = Pack de modèles illisible : { $id }
```

```ftl
# en
session-unknown-pack = Template pack not found: { $id }
session-pack-version-missing = Version { $version } of template pack { $id } not found
session-invalid-pack = Unreadable template pack: { $id }
```

- [ ] **Step 4 : lancer** — `cargo nextest run -p oku-slides-session` puis `cargo nextest run -p oku-slides-i18n` → PASS ; `cargo build --workspace` → PASS.

- [ ] **Step 5 : commit**

```bash
git add templates crates/oku-slides-session crates/oku-slides-i18n Cargo.lock
```
```bash
git commit -m "feat(session) : bibliothèque de packs versionnée, packs intégrés à id constant (t-328)"
```

---

### Task 3 : deck v3, `templates: Option<PackRef>` atomique (core)

**Files:**
- Modify: `crates/oku-slides-core/src/model.rs:15-16` (`DECK_VERSION`), `:34-36` (`templates`), `:7-13` (imports)
- Modify: `crates/oku-slides-core/src/ops/mod.rs:177-180` (`SetTemplates`)
- Modify: `crates/oku-slides-core/src/replica/schema.rs:56-58` (catégorie de `templates`)
- Modify: `crates/oku-slides-core/src/model_tests.rs`, `crates/oku-slides-core/src/replica/mod_tests.rs`, `crates/oku-slides-core/tests/ops.rs:93`
- Modify (`"version": 2` → `3`) : `crates/oku-slides-core/src/model_tests.rs:18`, `widget_tests.rs:264`, `validate_tests.rs:11`, `crates/oku-slides-render/tests/widgets.rs:197,276,293`, `render.rs:216`, `fields.rs:123`, `golden/widgets.json:496`, `crates/oku-slides-render-worker/src/client_tests.rs:62`, `test_packs.rs:31`, `fixtures/roadmap.json:2`, `typst/oku-slides/examples/deck.json:2`, `typst/oku-slides/README.md:103,113`, `typst/oku-slides/src/data.typ:1,4`

**Interfaces:**
- Consumes : `PackRef`, `PackVersion` (Task 1).
- Produces : `DECK_VERSION = 3` ; `Deck.templates: Option<PackRef>` ; `Op::SetTemplates { templates: Option<PackRef> }` ; dans le replica, `templates` est une feuille LWW entière.

- [ ] **Step 1 : tests en échec**

`model_tests.rs` :

```rust
const REF: &str = r#"{"id":"6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10","version":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#;

#[test]
fn deck_from_json_reads_templates_as_a_pack_ref() {
    let deck = Deck::from_json(&format!(r#"{{"version":3,"templates":{REF},"slides":[]}}"#)).unwrap();
    let r = deck.templates.unwrap();
    assert_that!(r.id.to_string()).is_equal_to("6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10".to_owned());
    assert_that!(r.version.hex()).is_equal_to(&*"a".repeat(64));
}

#[test]
fn deck_from_json_with_a_slug_templates_is_refused() {
    assert_that!(Deck::from_json(r#"{"version":3,"templates":"essentiel","slides":[]}"#).is_err()).is_true();
}

#[test]
fn deck_from_json_version_2_is_refused() {
    assert_that!(Deck::from_json(r#"{"version":2,"slides":[]}"#).is_err()).is_true();
}
```

`replica/mod_tests.rs` (aides du fichier : `test_registry::shared()`, `DeckReplica::from_snapshot(&snap, max, registry)`) :

```rust
/// Deux pairs changent chacun un champ différent du pack : la fusion garde
/// une référence entière, jamais l'id de l'un et la version de l'autre.
#[test]
fn merge_of_concurrent_templates_keeps_one_whole_pack_ref() {
    use crate::{PackRef, PackVersion};
    let v = |c: &str| PackVersion::from_hex(&c.repeat(64)).unwrap();
    let (id1, id2) = (uuid::Uuid::from_u128(1), uuid::Uuid::from_u128(2));
    let mut base = Deck::empty();
    base.templates = Some(PackRef { id: id1, version: v("a") });
    let snap = DeckReplica::new(&base, crate::test_registry::shared()).unwrap().snapshot().unwrap();
    let open = || DeckReplica::from_snapshot(&snap, 1 << 24, crate::test_registry::shared()).unwrap().0;
    let (mut a, mut b) = (open(), open());
    let only_version = PackRef { id: id1, version: v("b") };
    let only_id = PackRef { id: id2, version: v("a") };
    let mut da = base.clone();
    da.templates = Some(only_version.clone());
    a.apply(&da, "A", None).unwrap();
    let mut db = base.clone();
    db.templates = Some(only_id.clone());
    b.apply(&db, "B", None).unwrap();
    a.merge(&b).unwrap();
    let merged = a.deck().0.templates;
    assert_that!(merged == Some(only_version) || merged == Some(only_id)).is_true();
}
```

(Échoue aujourd'hui à la compilation, puis — `templates` classé `Nested(Opaque)` — en mêlant `id2` et `v("b")`.) `tests/ops.rs:93` : `Op::SetTemplates { templates: Some(PackRef { id: uuid::Uuid::from_u128(7), version: PackVersion::from_hex(&"c".repeat(64)).unwrap() }) }` et l'assertion correspondante. Ajouter `uuid.workspace = true` aux `[dev-dependencies]` de core n'est pas nécessaire (dépendance normale).

- [ ] **Step 2 : lancer** — `cargo nextest run -p oku-slides-core` → FAIL (compilation : `templates` est encore une `String`).

- [ ] **Step 3 : implémenter**

`model.rs` : `use crate::pack::PackRef;` ; ligne 16 `pub const DECK_VERSION: u32 = 3;` ; lignes 34-36 :

```rust
    /// Pack de modèles du deck, dans la version embarquée par ses sources ;
    /// `None` : le pack par défaut, dans sa version courante (deck neuf).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub templates: Option<PackRef>,
```

et le commentaire de `from_json` (ligne 363) : « relu comme un v3 ». `ops/mod.rs:177-180` :

```rust
    /// Pack de modèles du deck (id et version) ; `null` : le pack par défaut.
    SetTemplates {
        templates: Option<PackRef>,
    },
```

(`use crate::pack::PackRef;` en tête ; l'application, lignes 634-637, ne change pas.) `replica/schema.rs`, dans `Ctx::kind`, avant `(Deck, "slides")` :

```rust
            // Une référence de pack entière : l'id et la version ne se mêlent jamais.
            (Deck, "templates") => Kind::Atom,
```

Les occurrences `"version": 2` (et `"version":2`) des fichiers listés deviennent `3` ; `typst/oku-slides/README.md:103` : « ## Deck JSON (v3) » ; `data.typ:1` : « Rendu d'un deck JSON v3 ». Aucune autre ligne de ces fichiers ne change (les decks de test n'ont pas de `templates`).

- [ ] **Step 4 : lancer** — `cargo nextest run -p oku-slides-core -p oku-slides-render -p oku-slides-render-worker -p oku-slides-pdf` → PASS. (Session et aval ne compilent plus : voir la règle de verdeur.)

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-core crates/oku-slides-render crates/oku-slides-render-worker fixtures typst
```
```bash
git commit -m "feat(core)!: deck v3, pack de modèles référencé par id et version, atome du replica (t-328)"
```

---
### Task 4 : packs embarqués dans le zip des sources v4 (core)

**Files:**
- Modify: `crates/oku-slides-core/src/sources.rs` (doc `:1-17`, `SOURCES_VERSION` `:38`, `SourceBundle` `:62-81`, `new`/`from_replica` `:138-156`, `merge` `:279-293`, `with_validation` `:299-307`, `write_zip` `:320-349`, `read_zip` `:363-421`)
- Modify: `crates/oku-slides-core/src/validate.rs` (après `impl Display for Issue`, ligne 37)
- Modify: `crates/oku-slides-core/src/sources_tests.rs`, `crates/oku-slides-core/tests/zip_limits.rs:56`, `crates/oku-slides-core/src/replica/authors_tests.rs:71,122`
- Modify: `crates/oku-slides-pdf/src/lib.rs:15-27`, `crates/oku-slides-pdf/ARCHITECTURE.md:24-29`
- Modify: `crates/oku-slides-i18n/src/locales/*/core.ftl` (20 langues)

**Interfaces:**
- Consumes : `PackFiles`, `PackVersion`, `PackRef`, `Deck.templates` (Tasks 1, 3).
- Produces :
  - `SOURCES_VERSION = 4` ; `SourceBundle.packs: BTreeMap<PackVersion, PackFiles>` (public, en ajout seul, compte dans `PartialEq`).
  - `SourceBundle::add_pack(&mut self, files: PackFiles) -> PackVersion`.
  - `SourceBundle::from_replica(replica, read, assets, fonts, packs: BTreeMap<PackVersion, PackFiles>) -> (Self, Vec<Issue>)` — les problèmes comprennent `issue-pack-version-missing` si `deck.templates` vise une version absente.
  - `merge` : union des packs ; `with_validation` (fusion, restauration) signale aussi la version manquante.
  - `from_zip` : `packs/<hex>/pack.json` et `packs/<hex>/widgets.typ` ; `<hex>` hors de 64 hex minuscules → `SourcesErrorKind::BadName` ; contenu qui ne donne pas `<hex>` (ou `pack.json` absent, illisible, `widgets.typ` non UTF-8) → version écartée et `issue-pack-version-mismatch` ; les packs comptent dans `total_bytes`.
  - `Issue::pack_version_mismatch(hex: &str) -> Issue`, `Issue::pack_version_missing(version: &PackVersion) -> Issue` (`slide` et `element` à `None`).

- [ ] **Step 1 : tests en échec** — `sources_tests.rs` : `MANIFEST` passe à `version":4` ; le test `from_zip_with_sources_version_2_returns_not_sources` devient `…_version_3_…` avec `"version":3`. Ajouter :

```rust
use crate::{PackFiles, PackRef, PackVersion};

/// Un pack de test : `pack.json` minimal, `widgets` facultatif.
fn pack(name: &str, widgets: Option<&str>) -> PackFiles {
    let manifest = serde_json::json!({ "id": "6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10", "name": name });
    PackFiles::new(manifest.as_object().cloned().unwrap(), widgets.map(str::to_owned))
}

/// `roadmap` qui référence `files`, et son bundle qui l'embarque.
fn bundle_with(files: &PackFiles) -> SourceBundle {
    let mut deck = roadmap();
    deck.templates = Some(PackRef { id: uuid::Uuid::from_u128(1), version: files.version() });
    let mut b = SourceBundle::new(deck, crate::test_registry::shared()).unwrap();
    b.add_pack(files.clone());
    b
}

#[test]
fn to_zip_then_from_zip_keeps_packs_and_their_widgets() {
    let files = pack("P", Some("#let registry = ()\n"));
    let b = bundle_with(&files);
    let zip = b.to_zip().unwrap();
    let za = zip::ZipArchive::new(Cursor::new(zip.clone())).unwrap();
    let hex = files.version().hex().to_owned();
    assert_that!(za.file_names().any(|n| n == format!("packs/{hex}/pack.json"))).is_true();
    assert_that!(za.file_names().any(|n| n == format!("packs/{hex}/widgets.typ"))).is_true();
    let (back, issues) = SourceBundle::from_zip(&zip, MAX, crate::test_registry::shared()).unwrap();
    assert_that!(issues).is_empty();
    assert_that!(back.packs.get(&files.version())).is_equal_to(Some(&files));
}

#[test]
fn to_zip_of_a_pack_without_widgets_writes_no_widgets_file() {
    let files = pack("P", None);
    let zip = bundle_with(&files).to_zip().unwrap();
    let za = zip::ZipArchive::new(Cursor::new(zip)).unwrap();
    assert_that!(za.file_names().any(|n| n.ends_with("widgets.typ"))).is_false();
}

#[test]
fn from_zip_pack_whose_content_differs_from_its_hash_is_discarded_and_reported() {
    let loro = deck_loro();
    let forged = "f".repeat(64);
    let name = format!("packs/{forged}/pack.json");
    let entries: &[(&str, &[u8])] =
        &[("manifest.json", MANIFEST), ("deck.loro", &loro), (name.as_str(), br#"{"name":"P"}"#)];
    let (b, issues) = SourceBundle::from_zip(&zip(entries), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(b.packs).is_empty();
    let ids: Vec<&str> = issues.iter().map(|i| i.message.id()).collect();
    assert_that!(ids).is_equal_to(vec!["issue-pack-version-mismatch"]);
}

#[rstest::rstest]
#[case("packs/abc/pack.json")]
#[case("packs/FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF/pack.json")]
#[case("packs/../pack.json")]
fn from_zip_pack_with_an_invalid_path_returns_bad_name(#[case] name: &str) {
    let loro = deck_loro();
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.loro", &loro), (name, b"{}")];
    let err = SourceBundle::from_zip(&zip(entries), MAX, crate::test_registry::shared()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::BadName);
}

#[test]
fn from_zip_deck_referencing_an_absent_pack_reports_it() {
    let files = pack("P", None);
    let mut b = bundle_with(&files);
    b.packs.clear();
    let (_, issues) = SourceBundle::from_zip(&b.to_zip().unwrap(), MAX, crate::test_registry::shared()).unwrap();
    let ids: Vec<&str> = issues.iter().map(|i| i.message.id()).collect();
    assert_that!(ids).is_equal_to(vec!["issue-pack-version-missing"]);
}

#[test]
fn from_zip_packs_count_in_the_total_limit() {
    let loro = deck_loro();
    let files = pack(&"x".repeat(4096), None);
    let name = format!("packs/{}/pack.json", files.version().hex());
    let json = files.pack_json().unwrap();
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.loro", &loro), (name.as_str(), &json)];
    let limits = SourceLimits { total_bytes: MANIFEST.len() + loro.len() + 100, ..MAX };
    let err = SourceBundle::from_zip(&zip(entries), limits, crate::test_registry::shared()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::TooLarge);
}

#[test]
fn merge_unions_the_packs_of_both_bundles() {
    let (p, q) = (pack("P", None), pack("Q", Some("#let registry = ()")));
    let mut a = bundle_with(&p);
    let mut b = SourceBundle::from_zip(&a.to_zip().unwrap(), MAX, crate::test_registry::shared()).unwrap().0;
    b.add_pack(q.clone());
    a.merge(b).unwrap();
    assert_that!(a.packs.keys().cloned().collect::<Vec<_>>())
        .is_equal_to({ let mut v = vec![p.version(), q.version()]; v.sort(); v });
}

/// Le mode historique restaure aussi le pack d'une version passée : toute
/// version que le replica a référencée reste dans le fichier.
#[test]
fn versions_referenced_by_the_history_are_kept_on_save() {
    let (p, q) = (pack("P", None), pack("Q", None));
    let mut b = bundle_with(&p);
    b.add_pack(q.clone());
    let mut next = b.deck().clone();
    next.templates = Some(PackRef { id: uuid::Uuid::from_u128(1), version: q.version() });
    b.apply(next, "pack", None).unwrap();
    let (back, _) = SourceBundle::from_zip(&b.to_zip().unwrap(), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(back.packs.contains_key(&p.version()) && back.packs.contains_key(&q.version())).is_true();
    let preview = back.history();
    let mut restored = back;
    restored.restore(&preview, 0, "restaurer").unwrap();
    assert_that!(restored.deck().templates.as_ref().map(|r| r.version.clone())).is_equal_to(Some(p.version()));
}
```

`replica/authors_tests.rs:71,122` : `SourceBundle::from_replica(replica, read, Default::default(), Default::default(), Default::default())`. `tests/zip_limits.rs:56` : `"version":4`.

- [ ] **Step 2 : lancer** — `cargo nextest run -p oku-slides-core sources` → FAIL (`packs`, `add_pack` absents).

- [ ] **Step 3 : implémenter**

`validate.rs`, après l'`impl Display for Issue` :

```rust
impl Issue {
    /// Une version de pack écartée : son contenu ne donne pas son empreinte.
    pub fn pack_version_mismatch(hex: &str) -> Self {
        Self { slide: None, element: None, message: Message::new("issue-pack-version-mismatch").arg("version", hex) }
    }

    /// La version de pack du deck manque dans ses sources.
    pub fn pack_version_missing(version: &PackVersion) -> Self {
        let message = Message::new("issue-pack-version-missing").arg("version", version.as_str());
        Self { slide: None, element: None, message }
    }
}
```

(`use crate::pack::PackVersion;`.)

`sources.rs` — doc du module, lignes 1-3 :

```rust
//! Le zip des sources (format `oku-slides-sources`, version 4) : le replica
//! Loro du deck (`deck.loro`), les assets d'origine, les polices et les
//! versions de packs de modèles (`packs/<hex>/`). C'est ce blob qui est
//! stocké dans le PDF.
```

et la phrase des lignes 13-14 : « Un zip d'une version antérieure (1 : `deck.json` ; 2 et 3 : deck v1 ou v2) n'est plus lu. » Ligne 38 : `pub const SOURCES_VERSION: u32 = 4;`. Constante :

```rust
/// Dossier des versions de packs dans le zip.
const PACKS_DIR: &str = "packs/";
```

`SourceBundle` gagne, après `fonts` :

```rust
    /// Versions de packs de modèles par empreinte, en ajout seul : celle du
    /// deck et toutes celles que son historique a référencées.
    pub packs: BTreeMap<PackVersion, PackFiles>,
```

`PartialEq` : `… && self.fonts == other.fonts && self.packs == other.packs`. `new` : `packs: BTreeMap::new()`. `from_replica` :

```rust
    pub fn from_replica(
        replica: DeckReplica,
        (deck, mut issues): DeckRead,
        assets: BTreeMap<String, Vec<u8>>,
        fonts: BTreeMap<String, Vec<u8>>,
        packs: BTreeMap<PackVersion, PackFiles>,
    ) -> (Self, Vec<Issue>) {
        let bundle = Self { replica, deck, assets, fonts, packs };
        issues.extend(bundle.pack_issues());
        (bundle, issues)
    }

    /// La version de pack du deck manque-t-elle dans les sources ?
    fn pack_issues(&self) -> Vec<Issue> {
        match &self.deck.templates {
            Some(r) if !self.packs.contains_key(&r.version) => vec![Issue::pack_version_missing(&r.version)],
            _ => Vec::new(),
        }
    }

    /// Ajoute une version de pack et renvoie son empreinte.
    pub fn add_pack(&mut self, files: PackFiles) -> PackVersion {
        let version = files.version();
        self.packs.entry(version.clone()).or_insert(files);
        version
    }
```

`merge` : après la boucle des polices,

```rust
        for (version, files) in other.packs {
            self.packs.entry(version).or_insert(files);
        }
```

(doc : « … ses assets, polices et packs … »). `with_validation` : avant `issues` final,

```rust
        for issue in self.pack_issues() {
            if !issues.contains(&issue) {
                issues.push(issue);
            }
        }
```

`write_zip`, après la boucle des polices :

```rust
            for (version, files) in &self.packs {
                let dir = format!("{PACKS_DIR}{}", version.hex());
                zw.start_file(format!("{dir}/pack.json"), deflate).context(ZipSnafu)?;
                zw.write_all(&files.pack_json().context(JsonSnafu { file: "pack.json" })?).context(IoSnafu)?;
                if let Some(source) = files.widgets() {
                    zw.start_file(format!("{dir}/widgets.typ"), deflate).context(ZipSnafu)?;
                    zw.write_all(source.as_bytes()).context(IoSnafu)?;
                }
            }
```

`read_zip` : une fonction de chemin,

```rust
/// `packs/<hex>/<fichier>` : l'empreinte (64 hex minuscules, sinon refusée)
/// et le fichier (`pack.json`, `widgets.typ` ; autre nom : ignoré).
fn pack_entry(name: &str) -> Result<(&str, &str), SourcesInner> {
    let rest = name.strip_prefix(PACKS_DIR).unwrap_or(name);
    let (hex, file) = rest.split_once('/').unwrap_or((rest, ""));
    ensure!(PackVersion::from_hex(hex).is_some() && !file.contains('/'), BadNameSnafu { name });
    Ok((hex, file))
}
```

Dans la boucle, avant le calcul de `known` :

```rust
            if name.starts_with(PACKS_DIR) {
                let (hex, file) = pack_entry(&name)?;
                if file != "pack.json" && file != "widgets.typ" {
                    continue; // Dossier ou fichier inconnu : ni lu, ni compté.
                }
                let data = read_entry(f, size, usize::MAX, &mut budget, limits.total_bytes)?;
                let slot = raw_packs.entry(hex.to_owned()).or_insert((None, None));
                if file == "pack.json" { slot.0 = Some(data) } else { slot.1 = Some(data) }
                continue;
            }
```

avec `let mut raw_packs: BTreeMap<String, (Option<Vec<u8>>, Option<Vec<u8>>)> = BTreeMap::new();` déclaré avec `assets`/`fonts`. Après la lecture du replica :

```rust
        let mut packs = BTreeMap::new();
        let mut mismatched = Vec::new();
        for (hex, (pack_json, widgets)) in raw_packs {
            let files = pack_json.and_then(|p| PackFiles::from_bytes(&p, widgets.as_deref()));
            match files {
                Some(files) if files.version().hex() == hex => {
                    packs.insert(files.version(), files);
                }
                _ => {
                    tracing::warn!(version = %hex, "version de pack altérée écartée");
                    mismatched.push(Issue::pack_version_mismatch(&hex));
                }
            }
        }
        let (bundle, mut issues) = Self::from_replica(replica, read, assets, fonts, packs);
        issues.extend(mismatched);
        Ok((bundle, issues))
```

(`use crate::pack::{PackFiles, PackVersion};`.)

`crates/oku-slides-pdf/src/lib.rs:15-27` et `ARCHITECTURE.md:24-29` : le bloc devient

```text
//! Le zip des sources (version 4) :
//!
//! manifest.json          { format: "oku-slides-sources", version: 4 }
//! deck.loro              snapshot complet du replica Loro du deck
//! assets/<hex>           tels quels
//! fonts/<nom>            tels quels
//! packs/<hex>/pack.json  version de pack de modèles (hex = sha-256 du contenu canonique)
//! packs/<hex>/widgets.typ  absent si le pack n'a pas de widgets
```

Fluent — `core.ftl` de **chaque** langue, après `issue-missing-asset` :

```ftl
# fr
issue-pack-version-mismatch = version de pack altérée, écartée : { $version }
issue-pack-version-missing = version de pack absente des sources : { $version }
```

```ftl
# en
issue-pack-version-mismatch = altered pack version discarded: { $version }
issue-pack-version-missing = pack version missing from the sources: { $version }
```

- [ ] **Step 4 : lancer** — `cargo nextest run -p oku-slides-core -p oku-slides-pdf -p oku-slides-i18n` → PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-core crates/oku-slides-pdf crates/oku-slides-i18n
```
```bash
git commit -m "feat(core)!: sources v4, versions de packs embarquées, vérifiées et unies à la fusion (t-328)"
```

---

### Task 5 : la session rend depuis le pack embarqué (migration)

**Files:**
- Delete: `crates/oku-slides-session/src/templates.rs`, `templates_tests.rs` (tests des intégrés repris dans `library_tests.rs`)
- Create: `crates/oku-slides-session/src/deck_pack.rs`, `deck_pack_tests.rs`, `templates_mode_tests.rs`
- Modify: `crates/oku-slides-session/src/lib.rs`, `deck_widgets.rs`, `templates_mode.rs`, `history.rs`, `resume.rs`, `merge.rs`, `files.rs`, `views.rs`, `error.rs`, `agent/context.rs`
- Modify (tests réécrits en v3) : `src/{deck_widgets_tests,lib_tests,history_tests,resume_tests,workspace_tests,files_tests}.rs`, `tests/{session,errors,history,merge,agent,workspace}.rs`, `tests/regressions/issue_0327_pack_qui_boucle.rs`, `tests/snapshots/agent__agent_context…snap`
- Modify: `crates/oku-slides-i18n/src/locales/*/session.ftl`

**Interfaces:**
- Consumes : `Library` (dont `replace_version`), `PackManifest`, `StoredVersion`, `DEFAULT_PACK` (Task 2) ; `SourceBundle::{packs, add_pack, from_replica}` (Task 4).
- Produces :
  - `lib.rs` : `pub use library::{Library, StoredVersion, builtin_versions};` ; `Request::Templates { set: Option<Uuid> }`, `Request::EditTemplates { set: Uuid }`, `Request::DeleteTemplateSet { set: Uuid }` ; `Snapshot.templates: Option<&PackRef>` ; `Session::edit_templates(&mut self, id: Uuid)`, `Session::templates_view(&mut self, set: Option<Uuid>)`.
  - `views.rs` : `TemplateMode { set: Uuid, name: String, builtin: bool, base: PackVersion, parent: Option<PackVersion>, session_version: Option<PackVersion> }` (une version par session d'édition) ; `TemplatesView { set: PackInfo, sets: Vec<PackInfo>, templates }`.
  - `deck_pack.rs` : `pub(crate) fn adopt_default_pack(library: &Library, sources: &mut SourceBundle) -> Result<()>` ; `Session::embed_from_library(&mut self, r: &PackRef) -> Result<()>` ; `Session::deck_pack(&self) -> Option<&PackRef>` ; `Session::pack_name(&self) -> Option<String>`.
  - `deck_widgets.rs` : `DeckWidgets.templates: Option<PackRef>` ; `Session::resolve_widgets(&self, templates: Option<&PackRef>, packs: &BTreeMap<PackVersion, PackFiles>) -> Result<(DeckWidgets, Option<String>)>`.
  - Erreurs : `NoTemplateSet` et `TemplateJson` retirées (remplacées par `UnknownPack`, `PackJson`).

- [ ] **Step 1 : tests en échec**

Aide commune des tests unitaires (`test_support.rs`) : `session()` existe ; ajouter

```rust
/// Une session isolée sur `data`, en français, sans deck ouvert.
pub(crate) fn session_on(data: &std::path::Path) -> Session {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.data = Some(data.to_owned());
    settings.paths.decks = Some(data.join("decks"));
    settings.i18n.locale = Some("fr".into());
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme : {e}"));
    Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session : {}", e.one_line()))
}

/// La bibliothèque de la session `s`.
pub(crate) fn library_of(s: &Session) -> &crate::library::Library {
    &s.library
}
```

`crates/oku-slides-session/src/deck_pack_tests.rs` :

```rust
use oku_slides_core::{Op, PackRef, PackVersion};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::library::builtin_versions;
use crate::pack::DEFAULT_PACK;
use crate::test_support::{add_version, fresh_dir, library_of, manifest, session_on};

#[test]
fn new_session_blank_deck_embeds_and_references_the_default_pack() {
    let data = fresh_dir("oku-slides-deck-pack-");
    let s = session_on(data.path());
    let builtin = builtin_versions()[0].version.clone();
    assert_that!(s.deck().templates.clone()).is_equal_to(Some(PackRef { id: DEFAULT_PACK, version: builtin.clone() }));
    assert_that!(s.sources_for_tests().packs.contains_key(&builtin)).is_true();
    assert_that!(s.tab_info().pristine).is_true();
}

#[test]
fn new_deck_takes_the_current_version_of_the_default_pack() {
    let data = fresh_dir("oku-slides-deck-pack-");
    let mut s = session_on(data.path());
    let builtin = builtin_versions()[0].version.clone();
    let mine = add_version(library_of(&s), &manifest(DEFAULT_PACK, "Le mien", Some(&builtin), "2026-10-02T10:00:00Z"), None);
    library_of(&s).set_current(DEFAULT_PACK, &mine.version).unwrap();
    s.new_deck().unwrap();
    assert_that!(s.deck().templates.clone()).is_equal_to(Some(mine.clone()));
    assert_that!(s.sources_for_tests().packs.contains_key(&mine.version)).is_true();
}

#[test]
fn json_deck_without_templates_adopts_the_default_pack() {
    let s = crate::test_support::session();
    assert_that!(s.deck().templates.as_ref().map(|r| r.id)).is_equal_to(Some(DEFAULT_PACK));
    assert_that!(s.sources_for_tests().packs.len()).is_equal_to(1);
}

#[test]
fn set_templates_embeds_the_library_version_in_the_sources() {
    let data = fresh_dir("oku-slides-deck-pack-");
    let mut s = session_on(data.path());
    let r = add_version(library_of(&s), &manifest(Uuid::new_v4(), "Mien", None, "2026-10-01T10:00:00Z"), None);
    s.apply("Pack", vec![Op::SetTemplates { templates: Some(r.clone()) }], None).unwrap();
    assert_that!(s.deck().templates.clone()).is_equal_to(Some(r.clone()));
    assert_that!(s.sources_for_tests().packs.contains_key(&r.version)).is_true();
}

#[test]
fn set_templates_to_null_references_the_default_pack_explicitly() {
    let data = fresh_dir("oku-slides-deck-pack-");
    let mut s = session_on(data.path());
    let r = add_version(library_of(&s), &manifest(Uuid::new_v4(), "Mien", None, "2026-10-01T10:00:00Z"), None);
    s.apply("Pack", vec![Op::SetTemplates { templates: Some(r) }], None).unwrap();
    s.apply("Défaut", vec![Op::SetTemplates { templates: None }], None).unwrap();
    assert_that!(s.deck().templates.as_ref().map(|r| r.id)).is_equal_to(Some(DEFAULT_PACK));
}

#[test]
fn set_templates_to_an_unknown_version_returns_pack_version_missing() {
    let data = fresh_dir("oku-slides-deck-pack-");
    let mut s = session_on(data.path());
    let r = PackRef { id: DEFAULT_PACK, version: PackVersion::from_hex(&"0".repeat(64)).unwrap() };
    let err = s.apply("Pack", vec![Op::SetTemplates { templates: Some(r) }], None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::PackVersionMissing);
}
```

`deck_widgets_tests.rs` : `write_pack` est remplacé par une version qui passe par la bibliothèque de la session, et `set_templates` prend la référence :

```rust
/// Ajoute à la bibliothèque de `s` un pack de l'utilisateur, `widgets` pour `widgets.typ` ; sa référence.
fn user_pack(s: &Session, name: &str, widgets: &str) -> PackRef {
    let m = crate::test_support::manifest(uuid::Uuid::new_v4(), name, None, "2026-10-01T10:00:00Z");
    crate::test_support::add_version(&s.library, &m, Some(widgets))
}

fn set_templates(r: Option<&PackRef>) -> Op {
    Op::SetTemplates { templates: r.cloned() }
}
```

Chaque test `write_pack(&data.0, "quote", QUOTE)` puis `set_templates(Some("quote"))` devient `let quote = user_pack(&s, "Citation", QUOTE);` puis `set_templates(Some(&quote))` (la session est créée avant le pack). Tests ajoutés :

```rust
/// Le cache fait foi : une autre version courante ne change pas le rendu.
#[test]
fn render_uses_the_embedded_version_even_when_the_library_has_another() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    let quote = user_pack(&s, "Citation", QUOTE);
    s.apply("Pack", vec![set_templates(Some(&quote)), add_quote()], None).unwrap();
    let sans = crate::test_support::manifest(quote.id, "Citation", Some(&quote.version), "2026-10-02T10:00:00Z");
    let plain = crate::test_support::add_version(&s.library, &sans, None);
    s.library.set_current(quote.id, &plain.version).unwrap();
    let path = data.0.join("decks").join("cache.oku-slides.pdf");
    s.handle(Request::Save { path: Some(path.clone()), overwrite: false }).unwrap();
    let mut other = session_in(&data.0);
    other.open(&path).unwrap();
    assert_that!(widgets_have_quote(&mut other)).is_true();
    assert_that!(other.deck().templates.clone()).is_equal_to(Some(quote));
}

#[test]
fn version_absent_from_the_sources_renders_base_widgets_with_a_notice() {
    let data = data_dir();
    let path = data.0.join("absent.json");
    let deck = json!({ "version": 3, "templates": { "id": uuid::Uuid::new_v4(), "version": format!("sha256:{}", "0".repeat(64)) },
        "slides": [{ "id": "s", "kind": "title" }] });
    std::fs::write(&path, serde_json::to_vec(&deck).unwrap()).unwrap();
    let mut s = session_in(&data.0);
    s.open(&path).unwrap();
    let snap: serde_json::Value = serde_json::from_str(&s.handle(Request::Snapshot).unwrap()).unwrap();
    assert_that!(snap["notice"].as_str().unwrap_or_default()).contains("manque dans le fichier");
    assert_that!(widgets_have_quote(&mut s)).is_false();
}

#[test]
fn pack_without_widgets_renders_base_widgets_without_a_notice() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    let m = crate::test_support::manifest(uuid::Uuid::new_v4(), "Sans", None, "2026-10-01T10:00:00Z");
    let r = crate::test_support::add_version(&s.library, &m, None);
    s.handle(Request::Snapshot).unwrap();
    s.apply("Pack", vec![set_templates(Some(&r))], None).unwrap();
    let snap: serde_json::Value = serde_json::from_str(&s.handle(Request::Snapshot).unwrap()).unwrap();
    assert_that!(snap["notice"].is_null()).is_true();
}

/// L'historique restaure aussi le pack d'une version passée.
#[test]
fn history_restore_brings_back_the_pack_of_that_version() {
    let data = data_dir();
    let mut s = session_in(&data.0);
    let quote = user_pack(&s, "Citation", QUOTE);
    s.apply("Pack", vec![set_templates(Some(&quote)), add_quote()], None).unwrap();
    let plain = crate::test_support::add_version(
        &s.library,
        &crate::test_support::manifest(uuid::Uuid::new_v4(), "Sans", None, "2026-10-01T10:00:00Z"),
        None,
    );
    s.apply("Autre", vec![set_templates(Some(&plain))], None).unwrap();
    assert_that!(widgets_have_quote(&mut s)).is_false();
    s.history_open().unwrap();
    let count = s.history_versions().unwrap().len();
    s.history_seek(count - 2).unwrap();
    s.history_restore().unwrap();
    assert_that!(s.deck().templates.clone()).is_equal_to(Some(quote));
    assert_that!(widgets_have_quote(&mut s)).is_true();
}
```

`crates/oku-slides-session/src/templates_mode_tests.rs` :

```rust
use speculoos::prelude::*;

use crate::library::builtin_versions;
use crate::pack::{DEFAULT_PACK, PackManifest};
use crate::test_support::{fresh_dir, library_of, session_on};

#[test]
fn save_templates_creates_a_version_whose_parent_is_the_old_current() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    let builtin = builtin_versions()[0].version.clone();
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.apply("Nom", vec![serde_json::from_value(serde_json::json!(
        { "op": "set_deck_param", "key": "title", "value": "Essentiel à moi" })).unwrap()], None).unwrap();
    s.save_templates(false).unwrap();
    let lib = library_of(&s);
    let current = lib.current_version(DEFAULT_PACK).unwrap();
    assert_that!(current.manifest.parent.clone()).is_equal_to(Some(builtin));
    assert_that!(current.manifest.name.as_str()).is_equal_to("Essentiel à moi");
    // Modifier un pack intégré : la version de l'utilisateur, sous l'id du pack intégré.
    assert_that!(data.path().join("templates").join(DEFAULT_PACK.to_string()).join("versions")
        .join(current.version.hex()).join("pack.json").exists()).is_true();
    s.close_templates().unwrap();
    assert_that!(s.deck().templates.as_ref().map(|r| r.version.clone())).is_equal_to(Some(current.version.clone()));
    assert_that!(s.sources_for_tests().packs.contains_key(&current.version)).is_true();
}

/// Une version par session d'édition : les enregistrements suivants la remplacent.
#[test]
fn saving_twice_in_one_session_keeps_one_version_and_the_deck_follows_in_one_undo_step() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    let before = s.deck().templates.clone();
    let builtin = builtin_versions()[0].version.clone();
    s.edit_templates(DEFAULT_PACK).unwrap();
    let rename = |t: &str| serde_json::from_value(serde_json::json!(
        { "op": "set_deck_param", "key": "title", "value": t })).unwrap();
    s.apply("Nom", vec![rename("Un")], None).unwrap();
    s.save_templates(false).unwrap();
    let first = library_of(&s).current(DEFAULT_PACK).unwrap();
    s.apply("Nom", vec![rename("Deux")], None).unwrap();
    s.save_templates(false).unwrap();
    let second = library_of(&s).current_version(DEFAULT_PACK).unwrap();
    // Intégrée + une seule version de la session ; la première a disparu.
    assert_that!(library_of(&s).versions(DEFAULT_PACK).len()).is_equal_to(2);
    assert_that!(library_of(&s).version(DEFAULT_PACK, &first)).is_none();
    assert_that!(second.manifest.parent.clone()).is_equal_to(Some(builtin));
    assert_that!(s.snapshot().editing.and_then(|e| e.session_version.clone())).is_equal_to(Some(second.version.clone()));
    s.close_templates().unwrap();
    assert_that!(s.deck().templates.as_ref().map(|r| r.version.clone())).is_equal_to(Some(second.version));
    s.undo().unwrap();
    assert_that!(s.deck().templates.clone()).is_equal_to(before);
}

/// Quitter puis revenir : une nouvelle session, donc une nouvelle version, fille de la précédente.
#[test]
fn a_new_editing_session_creates_a_second_version() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    let rename = |t: &str| serde_json::from_value(serde_json::json!(
        { "op": "set_deck_param", "key": "title", "value": t })).unwrap();
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.apply("Nom", vec![rename("Un")], None).unwrap();
    s.save_templates(false).unwrap();
    let first = library_of(&s).current(DEFAULT_PACK).unwrap();
    s.close_templates().unwrap();
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.apply("Nom", vec![rename("Deux")], None).unwrap();
    s.save_templates(false).unwrap();
    let second = library_of(&s).current_version(DEFAULT_PACK).unwrap();
    assert_that!(library_of(&s).versions(DEFAULT_PACK).len()).is_equal_to(3);
    assert_that!(second.manifest.parent).is_equal_to(Some(first));
}

#[test]
fn save_templates_as_copy_creates_a_new_id_and_the_session_continues_on_it() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.save_templates(true).unwrap();
    let edit = s.snapshot().editing.cloned().unwrap();
    assert_that!(edit.set).is_not_equal_to(DEFAULT_PACK);
    let copy = library_of(&s).current_version(edit.set).unwrap();
    assert_that!(copy.manifest.parent.clone()).is_none();
    assert_that!(copy.manifest.name.as_str()).is_equal_to("Essentiel (copie)");
    assert_that!(library_of(&s).current(DEFAULT_PACK)).is_equal_to(Some(builtin_versions()[0].version.clone()));
    // Les enregistrements suivants remplacent la version de la copie, toujours sans parente.
    s.apply("Nom", vec![serde_json::from_value(serde_json::json!(
        { "op": "set_deck_param", "key": "title", "value": "Ma charte" })).unwrap()], None).unwrap();
    s.save_templates(false).unwrap();
    let again = library_of(&s).current_version(edit.set).unwrap();
    assert_that!(library_of(&s).versions(edit.set).len()).is_equal_to(1);
    assert_that!(again.manifest.parent).is_none();
    assert_that!(again.manifest.name.as_str()).is_equal_to("Ma charte");
}

#[test]
fn edit_templates_of_an_unknown_pack_returns_unknown_pack() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    let err = s.edit_templates(uuid::Uuid::new_v4()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::UnknownPack);
}

#[test]
fn templates_view_of_the_deck_pack_shows_the_embedded_version() {
    let data = fresh_dir("oku-slides-templates-mode-");
    let mut s = session_on(data.path());
    let embedded = s.deck().templates.clone().unwrap();
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.apply("Nom", vec![serde_json::from_value(serde_json::json!(
        { "op": "remove_slide", "slide": "texte" })).unwrap()], None).unwrap();
    s.save_templates(false).unwrap();
    // Le deck mis de côté est passé à la nouvelle version : on revient à l'ancienne.
    s.close_templates().unwrap();
    s.undo().unwrap();
    let view = s.templates_view(None).unwrap();
    assert_that!(view.set.version.clone()).is_equal_to(embedded.version.clone());
    let manifest = PackManifest::from_files(&builtin_versions()[0].files).unwrap();
    assert_that!(view.templates.len()).is_equal_to(manifest.templates.len());
}
```

`library_tests.rs` reprend les tests de contenu des intégrés de l'ancien `templates_tests.rs` (`builtin_sets_every_template_has_a_unique_id`, `…_converted_to_v2_keep_their_text`, ceux des lignes 85-135) en lisant `builtin_versions().iter().map(|b| &b.manifest)` au lieu de `builtin_sets()` ; les tests de `slug`, `free_id`, `save`, `widgets`, `save_widgets`, `delete("..")` et du fichier `<id>.json` disparaissent avec l'ancienne bibliothèque.

Réécriture mécanique des autres tests de session (même intention, nouvelle API) :
- `"cmd": "edit_templates", "set": DEFAULT_SET` → `"set": DEFAULT_PACK` (`json!` l'écrit en chaîne) ; `edit_templates(DEFAULT_SET)` / `edit_templates(&GRID_PACK.to_string())` → `edit_templates(DEFAULT_PACK)` / `edit_templates(GRID_PACK)` ; `edit_templates("inconnu")` → `edit_templates(Uuid::new_v4())` et `SessionErrorKind::UnknownPack` (`tests/errors.rs:107-108`) ; `delete_template_set` d'un intégré : `"set": DEFAULT_PACK` (`BuiltinTemplateSet`, inchangé).
- `tests/session.rs:228-256` (copie de `grilles`) : la copie a un nouvel id (`snap.editing.set != GRID_PACK`), son nom finit par « (copie) », `<data>/templates/<id>/versions/<hex>/pack.json` existe, `templates_view(Some(id))` la montre, `view.sets` la liste avec `builtin == false`. `tests/session.rs:359` (`edit_templates("a-blocs")`) : pack ajouté par `Library::add_version` sur le dossier de la session, puis `edit_templates(id)`.
- Les `"set": "oku"`/`"mine"` de `tests/history.rs:147-150` deviennent des `Uuid::new_v4()` (la requête est refusée avant toute recherche).
- `resume_tests.rs:92-96` : `TemplateMode { set: DEFAULT_PACK, name: "Essentiel".into(), builtin: true, base: v.clone(), parent: Some(v), session_version: None }` avec `let v = builtin_versions()[0].version.clone();`.
- `files_tests.rs:27,90`, `tests/errors.rs:193` : `"version":3`.
- `tests/regressions/issue_0327_pack_qui_boucle.rs:40-62` : le pack s'écrit par `Library::new(Some(data.path().join("templates")), Storage::local()).add_version(id, &PackManifest { id, name: "Boucle".into(), description: String::new(), parent: None, created: "2026-10-01T00:00:00Z".into(), templates: vec![] }.to_files(Some(LOOP_ON_RENDER.into())).unwrap())`, et l'op devient `{ "op": "set_templates", "templates": { "id": id, "version": version } }`.
- Instantané `agent__agent_context…snap:8` : `template_set: 6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10` (inchangé depuis la Task 2 : l'id seul est écrit).

- [ ] **Step 2 : lancer** — `cargo nextest run -p oku-slides-session` → FAIL (compilation : `deck.templates` n'est plus une `String`, `from_replica` attend les packs).

- [ ] **Step 3 : implémenter**

`deck_pack.rs` :

```rust
//! Le pack du deck : la version embarquée dans ses sources fait foi.
//!
//! Un deck sans pack (`templates: None` : deck neuf, deck JSON de
//! développement) prend la version courante du pack par défaut, embarquée
//! et référencée dès son chargement. Changer de pack (`SetTemplates`)
//! embarque d'abord la version voulue, prise dans la bibliothèque.

use snafu::{OptionExt, ResultExt};

use oku_slides_core::{PackRef, SourceBundle};

use crate::error::{PackVersionMissingSnafu, SourcesSnafu, UnknownPackSnafu};
use crate::library::Library;
use crate::pack::{DEFAULT_PACK, PackManifest};
use crate::{Result, Session};

/// Un deck sans pack prend la version courante du pack par défaut,
/// embarquée et référencée sans étape d'annulation.
pub(crate) fn adopt_default_pack(library: &Library, sources: &mut SourceBundle) -> Result<()> {
    if sources.deck().templates.is_some() {
        return Ok(());
    }
    let current = library.current_version(DEFAULT_PACK).context(UnknownPackSnafu { id: DEFAULT_PACK })?;
    let version = sources.add_pack(current.files);
    let mut deck = sources.deck().clone();
    deck.templates = Some(PackRef { id: DEFAULT_PACK, version });
    sources.apply_silent(deck).context(SourcesSnafu)?;
    Ok(())
}

impl Session {
    /// Le pack du deck de l'utilisateur, même pendant l'édition des modèles.
    pub(crate) fn deck_pack(&self) -> Option<&PackRef> {
        self.user_deck().templates.as_ref()
    }

    /// Embarque la version `r` dans les sources du document, depuis la
    /// bibliothèque, si elle n'y est pas déjà.
    pub(crate) fn embed_from_library(&mut self, r: &PackRef) -> Result<()> {
        if self.doc.sources.packs.contains_key(&r.version) {
            return Ok(());
        }
        let files = self
            .library
            .version(r.id, &r.version)
            .context(PackVersionMissingSnafu { id: r.id, version: r.version.clone() })?;
        self.doc.sources.add_pack(files);
        Ok(())
    }

    /// Nom du pack dont les widgets sont installés, lu dans sa version embarquée.
    pub(crate) fn pack_name(&self) -> Option<String> {
        let r = self.widgets_templates.as_ref()?;
        let files = self.doc.sources.packs.get(&r.version)?;
        PackManifest::from_files(files).ok().map(|m| m.name)
    }
}

#[cfg(test)]
#[path = "deck_pack_tests.rs"]
mod deck_pack_tests;
```

`deck_widgets.rs` — doc du module, lignes 1-3 : « Widgets du deck : la base, surchargée par le `widgets.typ` de la version de pack qu'il embarque (`deck.templates`, cherchée dans `SourceBundle.packs`, jamais dans la bibliothèque). » `DeckWidgets.templates: Option<PackRef>` (ligne 32, doc « La référence de pack dont ils viennent. »). `resolve_widgets` (lignes 49-74) :

```rust
    /// Les widgets de la version de pack `templates`, lue dans `packs` (les
    /// sources du document) : la base, surchargée par son `widgets.typ`.
    ///
    /// Version absente des sources : la base, et l'avis `session-pack-missing`.
    /// Pas de `widgets.typ` : la base, sans avis. Un pack cassé (Typst qui ne
    /// compile pas, schéma refusé, sonde hors de ses bornes, pack écarté) :
    /// journalisé, la base, et l'avis de son erreur. Seule la base qui ne se
    /// lit pas est une erreur.
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
        let Some(source) = files.widgets() else { return base(None) };
        let loaded = PackWidgets::new(r.version.hex(), source).and_then(|pack| Ok((self.worker.probe(&pack)?, pack)));
        match loaded {
            Ok((registry, pack)) => Ok((DeckWidgets { templates: Some(r.clone()), pack: Some(pack), registry }, None)),
            Err(e) => {
                tracing::warn!(pack = %r.id, error = %oku_slides_platform::errors::one_line(&e), "pack de widgets cassé : widgets de base");
                base(Some(e.localize(&self.localizer)))
            }
        }
    }
```

`apply` (lignes 115-131) : le choix du pack devient

```rust
        // `set_templates` à `null` : le pack par défaut, référencé dans sa version courante.
        for op in &mut ops {
            if let Op::SetTemplates { templates: t @ None } = op {
                let version = self.library.current(DEFAULT_PACK).context(UnknownPackSnafu { id: DEFAULT_PACK })?;
                *t = Some(PackRef { id: DEFAULT_PACK, version });
            }
        }
        let templates = ops.iter().rev().find_map(|op| match op {
            Op::SetTemplates { templates } => Some(templates.clone()),
            _ => None,
        });
        let (widgets, notice) = match templates {
            Some(t) if t != self.widgets_templates => {
                // La version voulue entre dans les sources avant d'être lue (en ajout seul).
                if let Some(r) = &t {
                    self.embed_from_library(r)?;
                }
                let (widgets, notice) = self.resolve_widgets(t.as_ref(), &self.doc.sources.packs)?;
                (Some(widgets), notice)
            }
            _ => (None, None),
        };
```

`fall_back_to_base` (ligne 191) : `let pack = self.pack_name().unwrap_or_else(|| self.pack.as_ref().map(|p| p.id().to_owned()).unwrap_or_default());`. `sync_widgets` (lignes 235-237) :

```rust
        let templates = self.doc.sources.deck().templates.clone();
        if templates != self.widgets_templates {
            let (widgets, notice) = self.resolve_widgets(templates.as_ref(), &self.doc.sources.packs)?;
```

Imports : `use std::collections::BTreeMap;`, `use oku_slides_core::{Deck, Op, PackFiles, PackRef, PackVersion, WidgetRegistry};`, `use crate::error::{…, UnknownPackSnafu};`, `use crate::pack::DEFAULT_PACK;` ; `DEFAULT_SET` disparaît.

`lib.rs` :
- mods (36-58) : retirer `pub mod templates;`, ajouter `mod deck_pack;` ; ligne 73 : `pub use library::{Library, StoredVersion, builtin_versions};` (l'ancien `pub use templates::…` disparaît) ; imports : `PackRef` de core, `uuid::Uuid`.
- `Request` (lignes 183-203) :

```rust
    /// Modèles d'un pack (celui du deck par défaut, dans sa version embarquée), rendus avec le thème du deck.
    Templates {
        #[serde(default)]
        set: Option<Uuid>,
    },
    /// Liste des packs de modèles, dans leur version courante.
    TemplateSets,
    /// Passe en édition de la version courante d'un pack (le deck est mis de côté).
    EditTemplates {
        set: Uuid,
    },
    /// Enregistre le pack édité : la version de cette session d'édition ; `as_new` : une copie (nouvel id).
    SaveTemplates {
        #[serde(default)]
        as_new: bool,
    },
    /// Quitte l'édition des modèles et retrouve le deck.
    CloseTemplates,
    /// Supprime un pack de l'utilisateur, toutes versions comprises.
    DeleteTemplateSet {
        set: Uuid,
    },
```

- champ `widgets_templates: Option<PackRef>` (ligne 296, doc « Référence de pack (`deck.templates`) pour laquelle… »).
- `Session::new` (lignes 388-425) : la bibliothèque est construite d'abord, le deck vierge adopte le pack par défaut :

```rust
        let library = Library::new(Some(settings.paths.templates()), storage.clone());
        let mut blank = SourceBundle::new(blank_deck(l), registry.clone()).context(SourcesSnafu)?;
        deck_pack::adopt_default_pack(&library, &mut blank)?;
```

  et `library,` dans le littéral ; `s.sync_widgets()?;` avant `s.rerender()?` (le pack par défaut de l'utilisateur peut avoir des widgets).
- `deck_set` et `get_set` (lignes 532-539) supprimés (remplacés par `deck_pack`).
- `handle` (581-602) :

```rust
            Request::TemplateSets => return json(&self.library.packs()),
            Request::DeleteTemplateSet { set } => {
                ensure!(!Library::is_builtin(set), BuiltinTemplateSetSnafu { id: set.to_string() });
                self.library.delete(set)?;
                self.template_cache = None;
                return json(&self.library.packs());
            }
            …
            Request::EditTemplates { set } => self.edit_templates(set)?,
```

- `load_document` (661-689) : en tête,

```rust
        deck_pack::adopt_default_pack(&self.library, &mut doc.sources)?;
        let (widgets, notice) = self.resolve_widgets(doc.sources.deck().templates.as_ref(), &doc.sources.packs)?;
```

- `snapshot` (ligne 862) : `templates: self.deck_pack(),`.

`views.rs` : import `use crate::pack::{PackInfo, Template};` et `oku_slides_core::{PackRef, PackVersion, …}` ; `Snapshot.templates: Option<&'a PackRef>` (doc « Pack de modèles du deck (id et version embarquée). ») ; 

```rust
/// Édition d'un pack de modèles : chaque slide est un modèle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMode {
    pub set: uuid::Uuid,
    pub name: String,
    /// Pack intégré : l'enregistrer crée la version de l'utilisateur sous son id.
    pub builtin: bool,
    /// Version dont viennent les widgets et la description (la dernière enregistrée).
    pub base: PackVersion,
    /// Parente des versions de cette session : la courante à son début ; `None` pour une copie.
    pub parent: Option<PackVersion>,
    /// La version de cette session, une fois enregistrée : les enregistrements suivants la remplacent.
    pub session_version: Option<PackVersion>,
}

/// Aperçus d'un pack de modèles, et la liste des packs.
#[derive(Debug, Serialize)]
pub struct TemplatesView {
    pub set: PackInfo,
    pub sets: Vec<PackInfo>,
    pub templates: Arc<[TemplateView]>,
}
```

`templates_mode.rs` (réécrit ; `styled_like` et `same_style` inchangés, lignes 23-40) :

```rust
//! Édition d'un pack de modèles : le deck attend de côté.
//!
//! On édite la version courante du pack. Une version par session
//! d'édition : le premier enregistrement (à la main ou automatique) crée une
//! version (`parent` = la courante au début de la session), la rend
//! courante, l'embarque dans le deck mis de côté, qui y passe
//! (`SetTemplates`, une seule étape d'annulation) ; les suivants la
//! remplacent (`Library::replace_version`, même parente, l'intermédiaire
//! quitte la bibliothèque ; les sources du deck restent en ajout seul),
//! jusqu'à la sortie du mode modèles. « Enregistrer comme copie » : un
//! nouvel id, sans parente, et la session continue sur la copie. Un pack
//! intégré suit la même règle, sous son id.

use std::collections::BTreeMap;
use std::sync::Arc;

use snafu::{OptionExt, ResultExt};
use uuid::Uuid;

use oku_slides_core::{Deck, PackRef, Slide, SourceBundle};

use crate::error::{InHistorySnafu, NotEditingTemplatesSnafu, PackJsonSnafu, RenderSnafu, SourcesSnafu, UnknownPackSnafu};
use crate::library::{Library, StoredVersion};
use crate::pack::{DEFAULT_PACK, PackInfo, PackManifest, Template, now_created};
use crate::{Document, Mode, Result, Session, TemplateMode, TemplateView, TemplatesView, new_renderer};

/// Clé d'étape d'annulation : les enregistrements du pack qui se suivent n'en font qu'une.
const ADOPT_MERGE: &str = "templates-adopt";

/// Aperçus de modèles en cache, avec ce dont ils dépendent : la version du
/// pack et le style du deck (le deck d'aperçu les porte).
pub(crate) struct Previews {
    pub(crate) key: PackRef,
    pub(crate) deck: Deck,
    pub(crate) views: Arc<[TemplateView]>,
}

/// La slide d'un modèle, nommée comme lui, pour l'éditer.
fn template_slide(t: Template) -> Slide {
    let mut s = t.slide;
    s.id = t.id;
    s.name = Some(t.name);
    s.description = (!t.description.is_empty()).then_some(t.description);
    s
}

impl Session {
    /// Passe en édition de la version courante du pack `id` : ses modèles
    /// deviennent les slides, rendus avec ses widgets.
    pub fn edit_templates(&mut self, id: Uuid) -> Result<()> {
        if matches!(self.mode, Mode::History { .. }) {
            return InHistorySnafu.fail()?;
        }
        let stored = self.library.current_version(id).context(UnknownPackSnafu { id })?;
        let r = PackRef { id, version: stored.version.clone() };
        let slides = stored.manifest.templates.iter().cloned().map(template_slide).collect();
        let mut deck = styled_like(self.user_deck(), slides);
        deck.params.title = Some(stored.manifest.name.clone());
        deck.templates = Some(r.clone());
        let packs = BTreeMap::from([(r.version.clone(), stored.files.clone())]);
        let (widgets, notice) = self.resolve_widgets(Some(&r), &packs)?;
        let mut sources = SourceBundle::new(deck, widgets.registry.clone()).context(SourcesSnafu)?;
        sources.add_pack(stored.files);
        let templates = Document::new(sources, None, &self.doc_defaults);
        let edit = TemplateMode {
            set: id,
            name: stored.manifest.name,
            builtin: Library::is_builtin(id),
            base: r.version.clone(),
            parent: Some(r.version),
            session_version: None,
        };
        // Les widgets du pack s'installent avant de changer de document : en
        // cas d'échec, rien ne change.
        self.install_widgets(widgets)?;
        // … la suite des lignes 75-93 actuelles, inchangée …
    }

    /// Le `pack.json` que montre `deck`, le deck des modèles en cours d'édition.
    fn edited_manifest(&self, deck: &Deck, id: Uuid, parent: Option<oku_slides_core::PackVersion>, description: String) -> PackManifest {
        let name = deck
            .params
            .title
            .clone()
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| self.localizer.tr("templates-default-name"));
        let templates = deck
            .slides
            .iter()
            .map(|s| {
                let mut slide = s.clone();
                let name = slide
                    .name
                    .take()
                    .or_else(|| slide.params.text("title").map(str::to_owned))
                    .unwrap_or_else(|| self.localizer.tr("template-default-name"));
                let description = slide.description.take().unwrap_or_default();
                slide.id = format!("tpl-{}", s.id);
                Template { id: s.id.clone(), name, description, slide }
            })
            .collect();
        PackManifest { id, name, description, parent, created: now_created(), templates }
    }

    /// Enregistre le pack édité dans la version de cette session (créée au
    /// premier enregistrement, remplacée ensuite), rendue courante ; le deck
    /// mis de côté l'embarque et y passe. `as_new` : une copie, sur laquelle
    /// la session continue.
    pub fn save_templates(&mut self, as_new: bool) -> Result<()> {
        let Mode::Templates { edit, .. } = &self.mode else {
            return NotEditingTemplatesSnafu.fail()?;
        };
        let edit = edit.clone();
        let mut deck = self.doc.sources.deck().clone();
        if as_new && deck.params.title.as_deref() == Some(edit.name.as_str()) {
            let name = self.localizer.tr_with("templates-copy-name", &[("name", edit.name.as_str().into())]);
            deck.params.title = Some(name);
        }
        let base = self.doc.sources.packs.get(&edit.base).and_then(|f| PackManifest::from_files(f).ok().map(|m| (m, f.widgets().map(str::to_owned))));
        let (description, widgets) = base.map(|(m, w)| (m.description, w)).unwrap_or_default();
        let (id, parent) = if as_new { (Uuid::new_v4(), None) } else { (edit.set, edit.parent.clone()) };
        let manifest = self.edited_manifest(&deck, id, parent.clone(), description);
        let files = manifest.to_files(widgets).context(PackJsonSnafu { id })?;
        // Une version par session : la première est créée, les suivantes la remplacent.
        let version = match edit.session_version.as_ref().filter(|_| !as_new) {
            Some(old) => self.library.replace_version(id, old, &files)?,
            None => {
                self.library.add_version(id, &files)?;
                self.library.set_current(id, &files.version())?;
                files.version()
            }
        };
        let r = PackRef { id, version: version.clone() };
        // Le document des modèles passe à la nouvelle version, sans étape d'annulation.
        self.doc.sources.add_pack(files.clone());
        deck.templates = Some(r.clone());
        self.doc.sources.apply_silent(deck).context(SourcesSnafu)?;
        let label = self.localizer.tr_with("templates-adopt-label", &[("name", manifest.name.as_str().into())]);
        if let Mode::Templates { stash, edit } = &mut self.mode {
            stash.sources.add_pack(files);
            let mut user = stash.sources.deck().clone();
            user.templates = Some(r);
            stash.sources.apply(user, &label, Some(ADOPT_MERGE)).context(SourcesSnafu)?;
            stash.dirty = true;
            *edit = TemplateMode {
                set: id,
                name: manifest.name,
                builtin: Library::is_builtin(id),
                base: version.clone(),
                parent,
                session_version: Some(version),
            };
        }
        self.sync_widgets()?;
        self.doc.dirty = false;
        self.template_cache = None;
        self.rerender()?;
        Ok(())
    }

    // `close_templates` : inchangé (lignes 160-178).

    /// La version d'un pack à montrer : celle qu'embarque le deck pour son
    /// propre pack, la courante pour un autre ; un pack disparu : celui du deck.
    fn shown_version(&self, set: Option<Uuid>) -> Option<StoredVersion> {
        let embedded = |r: &PackRef| {
            let files = self.user_sources().packs.get(&r.version)?.clone();
            let manifest = PackManifest::from_files(&files).ok()?;
            Some(StoredVersion { version: r.version.clone(), files, manifest })
        };
        let own = self.deck_pack().cloned();
        match (set, own) {
            (Some(id), Some(r)) if id != r.id => self.library.current_version(id).or_else(|| embedded(&r)),
            (Some(id), None) => self.library.current_version(id),
            (_, Some(r)) => embedded(&r).or_else(|| self.library.current_version(r.id)),
            (None, None) => self.library.current_version(DEFAULT_PACK),
        }
    }

    /// Aperçus d'un pack de modèles, rendus avec le style du deck ; en cache
    /// tant que rien ne change. Un pack qui fait échouer l'aperçu : repli sur
    /// la base, avec son avis, et aperçu refait une fois.
    pub fn templates_view(&mut self, set: Option<Uuid>) -> Result<TemplatesView> {
        let shown = self.shown_version(set).context(UnknownPackSnafu { id: set.unwrap_or(DEFAULT_PACK) })?;
        let key = PackRef { id: shown.manifest.id, version: shown.version.clone() };
        let cached = match &self.template_cache {
            Some(p) if p.key == key && same_style(&p.deck, self.user_deck()) => Some(p.views.clone()),
            _ => None,
        };
        let views = match cached {
            Some(views) => views,
            None => {
                let templates = shown.manifest.templates.clone();
                let deck = styled_like(self.user_deck(), templates.iter().map(|t| t.slide.clone()).collect());
                let views = self.with_fallback(|s| s.render_previews(&deck, &templates))?;
                self.template_cache = Some(Previews { key, deck, views: views.clone() });
                views
            }
        };
        let info: PackInfo = shown.manifest.info(shown.version, Library::is_builtin(shown.manifest.id));
        Ok(TemplatesView { set: info, sets: self.library.packs(), templates: views })
    }

    /// Rend `deck`, les modèles `templates` au style du deck, dans l'emplacement des aperçus.
    fn render_previews(&mut self, deck: &Deck, templates: &[Template]) -> Result<Arc<[TemplateView]>> {
        // … corps actuel (lignes 207-227), `set.templates` → `templates` …
    }
}

#[cfg(test)]
#[path = "templates_mode_tests.rs"]
mod templates_mode_tests;
```

`user_sources()` : à ajouter dans `lib.rs` à côté de `user_deck` (lignes 438-444), même `match`, rendant `&SourceBundle`.

`history.rs` (lignes 35-46) : les blobs portent les packs.

```rust
/// Images, polices et packs d'un document.
type Blobs = (BTreeMap<String, Vec<u8>>, BTreeMap<String, Vec<u8>>, BTreeMap<PackVersion, PackFiles>);

fn take_blobs(from: &mut Document) -> Blobs {
    let s = &mut from.sources;
    (std::mem::take(&mut s.assets), std::mem::take(&mut s.fonts), std::mem::take(&mut s.packs))
}

fn put_blobs(to: &mut Document, (assets, fonts, packs): Blobs) {
    to.sources.assets = assets;
    to.sources.fonts = fonts;
    to.sources.packs = packs;
}
```

(doc du module : « Images, polices et packs passent d'un document à l'autre sans copie ».)

`resume.rs` :
- `Blobs` (74-87) gagne `pub packs: BTreeMap<PackVersion, PackFiles>` ; `matches` : `… && self.packs.keys().eq(sources.packs.keys())` (adressés par contenu) ; `with_blobs` (126) : `packs: sources.packs.clone()`.
- `write_snapshot` (244-280) : après les polices,

```rust
        // Les packs sont adressés par contenu : `<hex>.json` et `<hex>.typ`.
        let packs_dir = dir.join("packs");
        let mut keep_packs = BTreeSet::new();
        for (version, files) in &doc.blobs.packs {
            let json = packs_dir.join(format!("{}.json", version.hex()));
            if !self.storage.exists(&json).context(AccessSnafu { path: &json })? {
                if let Some(source) = files.widgets() {
                    self.put(&packs_dir.join(format!("{}.typ", version.hex())), source.as_bytes().to_vec())?;
                }
                self.put(&json, files.pack_json().context(JsonSnafu { path: &json })?)?;
            }
            keep_packs.insert(format!("{}.json", version.hex()));
            if files.widgets().is_some() {
                keep_packs.insert(format!("{}.typ", version.hex()));
            }
        }
```

  et `self.prune(&packs_dir, &keep_packs)?;` avec les autres.
- `read_doc` (398-426) :

```rust
        let raw = self.read_files(&dir.join("packs"))?;
        let mut packs = BTreeMap::new();
        for (name, bytes) in &raw {
            let Some(hex) = name.strip_suffix(".json") else { continue };
            let widgets = raw.get(&format!("{hex}.typ")).map(Vec::as_slice);
            match PackFiles::from_bytes(bytes, widgets).filter(|f| f.version().hex() == hex) {
                Some(files) => {
                    packs.insert(files.version(), files);
                }
                None => tracing::warn!(version = hex, "pack de reprise altéré ignoré"),
            }
        }
        let (sources, issues) = SourceBundle::from_replica(replica, read, assets, fonts, packs);
```

- doc du module (lignes 4-8) : « … plus ses assets, polices et packs, écrits une fois par nom. »

`merge.rs` (`absorb`, lignes 72-80) : `before` et l'après comptent aussi `self.doc.sources.packs.len()` ; commentaire « La fusion n'ajoute que des polices, des assets et des packs ».

`files.rs::is_unreadable` (lignes 62-65) : une version de pack écartée est une perte :

```rust
    id.starts_with("issue-unreadable-") || id == "issue-slot-too-deep" || id == "issue-pack-version-mismatch"
```

`error.rs` : retirer `NoTemplateSet` (kind, variante, `localize`) et `TemplateJson` ; `BuiltinTemplateSet { id: String }` reste.

`agent/context.rs` (lignes 70, 78-83) : `format!("templates(set={})", edit.set)` inchangé (Uuid s'affiche) ; `self.deck_set()` → `self.deck_pack().map_or_else(|| "none".to_owned(), |r| r.id.to_string())`.

Fluent — `session.ftl` de **chaque** langue ; retirer `session-no-template-set` et `session-invalid-template-set` ; ajouter :

```ftl
# fr
session-pack-missing = Le pack de modèles de ce deck manque dans le fichier : il s'affiche avec les widgets de base.
templates-adopt-label = Modèles : { $name }
```

```ftl
# en
session-pack-missing = This deck's template pack is missing from the file: it is shown with the base widgets.
templates-adopt-label = Templates: { $name }
```

- [ ] **Step 4 : lancer** — `cargo nextest run -p oku-slides-session` (dont `--test issue_0327_pack_qui_boucle`) puis `cargo nextest run -p oku-slides-i18n` → PASS. `rg "DEFAULT_SET|TemplateSet|SetInfo|free_id|slug\(" crates/oku-slides-session` → rien (hors `DeleteTemplateSet`/`BuiltinTemplateSet`).

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-session crates/oku-slides-i18n
```
```bash
git commit -m "feat(session)!: rendu depuis le pack embarqué, mode modèles en versions (t-328)"
```

---
### Task 6 : ouverture — versions reçues en silence, bandeau et ses deux actions (session)

**Files:**
- Create: `crates/oku-slides-session/src/pack_notice.rs`, `pack_notice_tests.rs`
- Modify: `crates/oku-slides-session/src/lib.rs` (mod, champs, `Request`, `handle`, `edits`, `changes_deck`, `load_document`, `open`, `snapshot`)
- Modify: `crates/oku-slides-session/src/views.rs` (`Snapshot.pack_notice`)
- Modify: `crates/oku-slides-session/src/merge.rs` (`absorb`)
- Modify: `crates/oku-slides-i18n/src/locales/*/session.ftl`

**Interfaces:**
- Consumes : `Library::{add_version, current, current_version, relation, set_current}` (Task 2) ; `Session::apply` qui embarque la version visée (Task 5).
- Produces :
  - `pub struct PackNoticeView { pub pack: Uuid, pub name: String, pub relation: Relation, pub message: String, pub use_mine: String, pub make_current: String }` (réexporté) ; `Snapshot.pack_notice: Option<PackNoticeView>`.
  - `Request::{PackUseMine, PackMakeCurrent, PackNoticeClose}` (`"cmd": "pack_use_mine" | "pack_make_current" | "pack_notice_close"`).
  - `Session::pack_use_mine(&mut self) -> Result<()>`, `Session::pack_make_current(&mut self) -> Result<()>`, `Session::pack_notice_close(&mut self)`.
  - `Session::receive_packs(&self)` (crate), appelé par `open` et par la fusion.

- [ ] **Step 1 : tests en échec** — `crates/oku-slides-session/src/pack_notice_tests.rs`

```rust
use std::path::{Path, PathBuf};

use oku_slides_core::{Op, PackFiles, PackRef, PackVersion};
use rstest::rstest;
use speculoos::prelude::*;
use uuid::Uuid;

use crate::pack::{PackManifest, Relation};
use crate::test_support::{fresh_dir, manifest, session_on};
use crate::{Request, Session};

fn files(id: Uuid, name: &str, parent: Option<&PackVersion>, created: &str) -> PackFiles {
    manifest(id, name, parent, created).to_files(None).unwrap()
}

/// Un fichier enregistré par une autre session (dossier `them`), dont le deck embarque `pack`.
fn received_file(them: &Path, pack: &PackFiles) -> PathBuf {
    let mut other = session_on(them);
    let id = PackManifest::from_files(pack).unwrap().id;
    other.library.add_version(id, pack).unwrap();
    let r = PackRef { id, version: pack.version() };
    other.apply("Pack", vec![Op::SetTemplates { templates: Some(r) }], None).unwrap();
    let path = them.join("recu.oku-slides.pdf");
    other.handle(Request::Save { path: Some(path.clone()), overwrite: false }).unwrap();
    path
}

fn notice(s: &mut Session) -> serde_json::Value {
    let snap: serde_json::Value = serde_json::from_str(&s.handle(Request::Snapshot).unwrap()).unwrap();
    snap["pack_notice"].clone()
}

#[test]
fn open_adds_received_versions_silently_without_changing_current() {
    let (me, them) = (fresh_dir("oku-slides-notice-me-"), fresh_dir("oku-slides-notice-them-"));
    let id = Uuid::new_v4();
    let v0 = files(id, "P", None, "2026-10-01T10:00:00Z");
    let v1 = files(id, "P", Some(&v0.version()), "2026-10-02T10:00:00Z");
    let path = received_file(them.path(), &v1);
    let mut s = session_on(me.path());
    s.library.add_version(id, &v0).unwrap();
    s.open(&path).unwrap();
    assert_that!(s.library.versions(id).len()).is_equal_to(2);
    assert_that!(s.library.current(id)).is_equal_to(Some(v0.version()));
    assert_that!(s.deck().templates.as_ref().map(|r| r.version.clone())).is_equal_to(Some(v1.version()));
}

#[test]
fn open_with_an_unknown_pack_creates_it_with_the_received_version_current() {
    let (me, them) = (fresh_dir("oku-slides-notice-me-"), fresh_dir("oku-slides-notice-them-"));
    let id = Uuid::new_v4();
    let v = files(id, "Nouveau", None, "2026-10-01T10:00:00Z");
    let mut s = session_on(me.path());
    s.open(&received_file(them.path(), &v)).unwrap();
    assert_that!(s.library.current(id)).is_equal_to(Some(v.version()));
    assert_that!(notice(&mut s).is_null()).is_true();
}

/// v0 ← v1 ; v0 ← v2. `mine` : ma version courante ; `theirs` : celle du fichier.
#[rstest]
#[case::newer(0, 1, "descendant", "plus récente")]
#[case::older(1, 0, "ancestor", "plus ancienne")]
#[case::parallel(1, 2, "sibling", "en parallèle")]
fn open_with_another_version_shows_the_banner_with_its_relation(
    #[case] mine: usize,
    #[case] theirs: usize,
    #[case] relation: &str,
    #[case] words: &str,
) {
    let (me, them) = (fresh_dir("oku-slides-notice-me-"), fresh_dir("oku-slides-notice-them-"));
    let id = Uuid::new_v4();
    let v0 = files(id, "Charte", None, "2026-10-01T10:00:00Z");
    let v1 = files(id, "Charte", Some(&v0.version()), "2026-10-02T10:00:00Z");
    let v2 = files(id, "Charte", Some(&v0.version()), "2026-10-03T10:00:00Z");
    let v = [v0, v1, v2];
    let mut s = session_on(me.path());
    s.library.add_version(id, &v[0]).unwrap();
    s.library.add_version(id, &v[1]).unwrap();
    s.library.set_current(id, &v[mine].version()).unwrap();
    s.open(&received_file(them.path(), &v[theirs])).unwrap();
    let n = notice(&mut s);
    assert_that!(n["relation"].as_str()).is_equal_to(Some(relation));
    assert_that!(n["message"].as_str().unwrap_or_default()).contains(words);
    assert_that!(n["message"].as_str().unwrap_or_default()).contains("Charte");
    assert_that!(s.library.current(id)).is_equal_to(Some(v[mine].version()));
}

#[test]
fn open_with_the_current_version_shows_no_banner() {
    let (me, them) = (fresh_dir("oku-slides-notice-me-"), fresh_dir("oku-slides-notice-them-"));
    let id = Uuid::new_v4();
    let v = files(id, "P", None, "2026-10-01T10:00:00Z");
    let mut s = session_on(me.path());
    s.library.add_version(id, &v).unwrap();
    s.open(&received_file(them.path(), &v)).unwrap();
    assert_that!(notice(&mut s).is_null()).is_true();
}

/// Ma version courante v0, le fichier en v1 : la session ouverte, et les deux versions.
fn opened_on_newer() -> (tempfile::TempDir, tempfile::TempDir, Session, PackFiles, PackFiles) {
    let (me, them) = (fresh_dir("oku-slides-notice-me-"), fresh_dir("oku-slides-notice-them-"));
    let id = Uuid::new_v4();
    let v0 = files(id, "P", None, "2026-10-01T10:00:00Z");
    let v1 = files(id, "P", Some(&v0.version()), "2026-10-02T10:00:00Z");
    let path = received_file(them.path(), &v1);
    let mut s = session_on(me.path());
    s.library.add_version(id, &v0).unwrap();
    s.open(&path).unwrap();
    (me, them, s, v0, v1)
}

#[test]
fn pack_use_mine_switches_the_deck_and_undo_brings_theirs_back() {
    let (_me, _them, mut s, v0, v1) = opened_on_newer();
    s.handle(Request::PackUseMine).unwrap();
    assert_that!(s.deck().templates.as_ref().map(|r| r.version.clone())).is_equal_to(Some(v0.version()));
    assert_that!(s.sources_for_tests().packs.contains_key(&v0.version())).is_true();
    assert_that!(notice(&mut s).is_null()).is_true();
    s.handle(Request::Undo).unwrap();
    assert_that!(s.deck().templates.as_ref().map(|r| r.version.clone())).is_equal_to(Some(v1.version()));
}

#[test]
fn pack_make_current_makes_theirs_current_and_closes_the_banner() {
    let (_me, _them, mut s, _v0, v1) = opened_on_newer();
    let id = s.deck().templates.as_ref().map(|r| r.id).unwrap();
    s.handle(Request::PackMakeCurrent).unwrap();
    assert_that!(s.library.current(id)).is_equal_to(Some(v1.version()));
    assert_that!(notice(&mut s).is_null()).is_true();
    assert_that!(s.deck().templates.as_ref().map(|r| r.version.clone())).is_equal_to(Some(v1.version()));
}

#[test]
fn pack_notice_close_hides_the_banner_for_the_session() {
    let (_me, them, mut s, _v0, _v1) = opened_on_newer();
    s.handle(Request::PackNoticeClose).unwrap();
    assert_that!(notice(&mut s).is_null()).is_true();
    s.open(&them.path().join("recu.oku-slides.pdf")).unwrap();
    assert_that!(notice(&mut s).is_null()).is_true();
}

#[test]
fn pack_use_mine_is_refused_while_editing_templates() {
    let (_me, _them, mut s, _v0, _v1) = opened_on_newer();
    s.edit_templates(crate::pack::DEFAULT_PACK).unwrap();
    let err = s.handle(Request::PackUseMine).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::EditingTemplates);
}
```

- [ ] **Step 2 : lancer** — `cargo nextest run -p oku-slides-session pack_notice` → FAIL (`PackUseMine`, `pack_notice` absents).

- [ ] **Step 3 : implémenter** — `pack_notice.rs` :

```rust
//! Versions de pack reçues avec un fichier, et le bandeau qui les signale.
//!
//! À l'ouverture, chaque version embarquée absente de la bibliothèque y est
//! ajoutée en silence (un id inconnu crée le pack). Si la version du deck
//! n'est pas la courante de son pack, un bandeau le dit, avec deux actions :
//! passer le deck à ma version (annulable), ou faire de la sienne ma version
//! courante. Il ne bloque jamais l'ouverture ; fermé, il ne revient plus
//! pour ce deck dans cette session.

use serde::Serialize;
use snafu::OptionExt;
use uuid::Uuid;

use oku_slides_core::{Op, PackRef};

use crate::error::UnknownPackSnafu;
use crate::pack::{PackManifest, Relation};
use crate::{Result, Session};

/// Le bandeau : la version du deck n'est pas la courante de son pack.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PackNotice {
    /// Le pack du deck, dans sa version embarquée.
    pub(crate) pack: PackRef,
    pub(crate) name: String,
    /// La version du deck par rapport à ma version courante.
    pub(crate) relation: Relation,
}

/// Le bandeau tel que l'interface le montre, dans la langue de l'utilisateur.
#[derive(Debug, Clone, Serialize)]
pub struct PackNoticeView {
    pub pack: Uuid,
    pub name: String,
    pub relation: Relation,
    pub message: String,
    /// Libellé de « Passer à ma version ».
    pub use_mine: String,
    /// Libellé de « Faire de la sienne ma version courante ».
    pub make_current: String,
}

impl Session {
    /// Ajoute en silence à la bibliothèque les versions embarquées qu'elle n'a pas.
    pub(crate) fn receive_packs(&self) {
        for files in self.doc.sources.packs.values() {
            let id = match PackManifest::from_files(files) {
                Ok(m) => m.id,
                Err(e) => {
                    tracing::warn!(version = %files.version(), error = %e, "pack reçu illisible");
                    continue;
                }
            };
            if let Err(e) = self.library.add_version(id, files) {
                tracing::warn!(pack = %id, error = %e.one_line(), "pack reçu non ajouté à la bibliothèque");
            }
        }
    }

    /// Le bandeau du deck ouvert, s'il en faut un.
    pub(crate) fn pack_notice_for_deck(&self) -> Option<PackNotice> {
        let r = self.doc.sources.deck().templates.clone()?;
        if self.pack_notices_closed.contains(&r) {
            return None;
        }
        let current = self.library.current(r.id)?;
        let relation = self.library.relation(r.id, &r.version, &current);
        if relation == Relation::Same {
            return None;
        }
        let name = self
            .doc
            .sources
            .packs
            .get(&r.version)
            .and_then(|f| PackManifest::from_files(f).ok())
            .map(|m| m.name)
            .unwrap_or_default();
        Some(PackNotice { pack: r, name, relation })
    }

    /// Le bandeau à montrer, s'il y en a un.
    pub(crate) fn pack_notice_view(&self) -> Option<PackNoticeView> {
        let n = self.pack_notice.as_ref()?;
        let key = match n.relation {
            Relation::Ancestor => "pack-notice-older",
            Relation::Descendant => "pack-notice-newer",
            Relation::Same | Relation::Sibling => "pack-notice-parallel",
        };
        let l = &self.localizer;
        Some(PackNoticeView {
            pack: n.pack.id,
            name: n.name.clone(),
            relation: n.relation,
            message: l.tr_with(key, &[("name", n.name.as_str().into())]),
            use_mine: l.tr("pack-notice-use-mine"),
            make_current: l.tr("pack-notice-make-current"),
        })
    }

    /// « Passer à ma version » : le deck passe à ma version courante, en une étape annulable.
    pub fn pack_use_mine(&mut self) -> Result<()> {
        let Some(n) = self.pack_notice.clone() else { return Ok(()) };
        let id = n.pack.id;
        let version = self.library.current(id).context(UnknownPackSnafu { id })?;
        let label = self.localizer.tr("pack-notice-use-mine");
        self.apply(&label, vec![Op::SetTemplates { templates: Some(PackRef { id, version }) }], None)?;
        self.pack_notice_close();
        Ok(())
    }

    /// « Faire de la sienne ma version courante » : le deck ne change pas.
    pub fn pack_make_current(&mut self) -> Result<()> {
        let Some(n) = self.pack_notice.clone() else { return Ok(()) };
        self.library.set_current(n.pack.id, &n.pack.version)?;
        self.template_cache = None;
        self.pack_notice_close();
        Ok(())
    }

    /// Ferme le bandeau ; il ne revient plus pour ce deck dans cette session.
    pub fn pack_notice_close(&mut self) {
        if let Some(n) = self.pack_notice.take() {
            self.pack_notices_closed.insert(n.pack);
        }
    }
}

#[cfg(test)]
#[path = "pack_notice_tests.rs"]
mod pack_notice_tests;
```

`lib.rs` :
- `mod pack_notice;` et `pub use pack_notice::PackNoticeView;`.
- `Session` gagne, après `agent_notes` :

```rust
    /// Bandeau du pack du deck ouvert (autre version que la courante).
    pack_notice: Option<pack_notice::PackNotice>,
    /// Bandeaux fermés : ils ne reviennent plus dans cette session.
    pack_notices_closed: HashSet<PackRef>,
```

  (`pack_notice: None, pack_notices_closed: HashSet::new(),` dans `Session::new`).
- `Request`, après `SaveCopy` :

```rust
    /// Bandeau du pack : le deck passe à ma version courante (annulable).
    PackUseMine,
    /// Bandeau du pack : sa version devient ma version courante.
    PackMakeCurrent,
    /// Ferme le bandeau du pack pour la session.
    PackNoticeClose,
```

  `changes_deck` : `Self::PackUseMine` avec les `true`, `Self::PackMakeCurrent | Self::PackNoticeClose` avec les `false`. `handle` : le refus pendant les modèles (ligne 549) gagne `| Request::PackUseMine` ; `documents` (lignes 568-575) gagne `| Request::PackMakeCurrent | Request::PackNoticeClose` ; bras :

```rust
            Request::PackUseMine => self.pack_use_mine()?,
            Request::PackMakeCurrent => self.pack_make_current()?,
            Request::PackNoticeClose => self.pack_notice_close(),
```

  `edits` (lignes 623-642) gagne `| Request::PackUseMine`.
- `load_document` : `self.pack_notice = None;` avant `self.rerender()`.
- `open` (lignes 692-706), après `self.load(…)?;` :

```rust
        // Les versions reçues entrent en silence ; une autre version que la courante : le bandeau.
        self.receive_packs();
        self.pack_notice = self.pack_notice_for_deck();
```

- `snapshot` : `pack_notice: self.pack_notice_view(),`.

`views.rs`, dans `Snapshot` après `editing` :

```rust
    /// Bandeau du pack : le deck embarque une autre version que ma version courante.
    pub pack_notice: Option<crate::PackNoticeView>,
```

`merge.rs` (`absorb`, après `log_issues`) : `self.receive_packs();` (les versions d'un fichier fusionné entrent aussi en silence, sans bandeau).

Fluent — `session.ftl` de **chaque** langue :

```ftl
# fr
pack-notice-older = Ce deck utilise une version plus ancienne de « { $name } » que la vôtre.
pack-notice-newer = Ce deck utilise une version plus récente de « { $name } » que la vôtre.
pack-notice-parallel = Ce deck utilise une version de « { $name } » faite en parallèle de la vôtre.
pack-notice-use-mine = Passer à ma version
pack-notice-make-current = Faire de la sienne ma version courante
```

```ftl
# en
pack-notice-older = This deck uses an older version of “{ $name }” than yours.
pack-notice-newer = This deck uses a newer version of “{ $name }” than yours.
pack-notice-parallel = This deck uses a version of “{ $name }” made in parallel with yours.
pack-notice-use-mine = Switch to my version
pack-notice-make-current = Make theirs my current version
```

- [ ] **Step 4 : lancer** — `cargo nextest run -p oku-slides-session` puis `cargo nextest run -p oku-slides-i18n` → PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-session crates/oku-slides-i18n
```
```bash
git commit -m "feat(session) : versions de pack reçues ajoutées en silence, bandeau et ses deux actions (t-328)"
```

---

### Task 7 : régression t-328, MCP et agent

**Files:**
- Create: `crates/oku-slides-session/tests/regressions/issue_0328_pack_recu_garde_la_version_courante.rs`
- Modify: `crates/oku-slides-session/tests/regressions/main.rs` (mod)
- Modify: `crates/oku-slides-mcp/src/edit_tools.rs:31-35` (`SetArgs`), `guarded_tools.rs:191-198` (`delete_template_set`), `tools.rs:199,203,215` (descriptions), `crates/oku-slides-mcp/Cargo.toml`, `crates/oku-slides-mcp/tests/edit.rs`
- Vérifier : `crates/oku-slides-agent`, `src-tauri` (compilation seule : ils n'utilisent ni `templates` ni `DEFAULT_SET`)

**Interfaces:**
- Consumes : `Library`, `PackManifest`, `DEFAULT_PACK`, `builtin_versions` (Tasks 2, 5) ; `Request::Save` ; `Session::open`.
- Produces : `SetArgs { set: Uuid }` côté MCP (un id mal formé est refusé par `parse`).

- [ ] **Step 1 : tests en échec**

`issue_0328_pack_recu_garde_la_version_courante.rs` :

```rust
//! t-328 : un deck reçu avec le même id de pack mais un autre contenu ne
//! change jamais la version courante de la bibliothèque.
//!
//! Avant t-328, un pack était un dossier nommé par son slug : recevoir un
//! deck dont le pack portait le même nom revenait à remplacer le sien.

use std::path::{Path, PathBuf};

use oku_slides_core::{Op, PackFiles, PackRef, PackVersion};
use oku_slides_session::{DEFAULT_PACK, Library, PackManifest, Request, Storage, builtin_versions};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::common::{data_dir, session_in};

fn library(data: &Path) -> Library {
    Library::new(Some(data.join("templates")), Storage::local().unwrap())
}

fn version(id: Uuid, name: &str, parent: Option<&PackVersion>, created: &str) -> PackFiles {
    let m = PackManifest {
        id,
        name: name.into(),
        description: String::new(),
        parent: parent.cloned(),
        created: created.into(),
        templates: Vec::new(),
    };
    m.to_files(None).unwrap()
}

/// Un fichier enregistré par quelqu'un d'autre, dont le deck embarque `pack`.
fn received(them: &Path, id: Uuid, pack: &PackFiles) -> PathBuf {
    library(them).add_version(id, pack).unwrap();
    let mut s = session_in(them);
    let r = PackRef { id, version: pack.version() };
    s.apply("Pack", vec![Op::SetTemplates { templates: Some(r) }], None).unwrap();
    let path = them.join("recu.oku-slides.pdf");
    s.handle(Request::Save { path: Some(path.clone()), overwrite: false }).unwrap();
    path
}

#[test]
fn received_deck_with_the_same_pack_id_never_changes_the_current_version() {
    let (me, them) = (data_dir(), data_dir());
    let id = Uuid::new_v4();
    let mine = version(id, "Charte", None, "2026-10-01T10:00:00Z");
    // La sienne est plus récente et porte le même nom : rien n'en fait la mienne.
    let theirs = version(id, "Charte", None, "2026-10-05T10:00:00Z");
    let path = received(&them, id, &theirs);
    library(&me).add_version(id, &mine).unwrap();
    let mut s = session_in(&me);
    s.open(&path).unwrap();
    s.open(&path).unwrap();
    s.handle(Request::Merge { path: path.clone() }).unwrap();
    let lib = library(&me);
    assert_that!(lib.current(id)).is_equal_to(Some(mine.version()));
    assert_that!(lib.versions(id).len()).is_equal_to(2);
    assert_that!(s.deck().templates.as_ref().map(|r| r.version.clone())).is_equal_to(Some(theirs.version()));
}

#[test]
fn received_version_of_a_builtin_pack_never_replaces_the_builtin_current() {
    let (me, them) = (data_dir(), data_dir());
    let builtin = builtin_versions()[0].version.clone();
    let theirs = version(DEFAULT_PACK, "Essentiel retouché", Some(&builtin), "2026-10-05T10:00:00Z");
    let path = received(&them, DEFAULT_PACK, &theirs);
    let mut s = session_in(&me);
    s.open(&path).unwrap();
    assert_that!(library(&me).current(DEFAULT_PACK)).is_equal_to(Some(builtin));
}
```

`regressions/main.rs` : `mod issue_0328_pack_recu_garde_la_version_courante;` après `issue_0264_…`.

`crates/oku-slides-mcp/tests/edit.rs`, à côté de `edit_then_close_templates_round_trips_mode` (qui garde `let set = oku_slides_session::DEFAULT_PACK;`) :

```rust
#[tokio::test(flavor = "multi_thread")]
async fn edit_templates_with_a_slug_is_refused() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "edit_templates", json!({ "set": "essentiel" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(h.session.lock().unwrap().snapshot().editing.is_none()).is_true();
}
```

- [ ] **Step 2 : lancer** — `cargo nextest run -p oku-slides-session --test regressions issue_0328` → PASS attendu si les Tasks 5-6 sont complètes ; **vérifier qu'il mord** : remplacer temporairement, dans `Library::current_version`, la branche `Current::At(v) if …` par `latest(&versions)` (l'ancienne règle « le dernier reçu gagne ») → le premier test doit échouer ; remettre le code. `cargo nextest run -p oku-slides-mcp` → FAIL (compilation : `SetArgs.set` est une `String`).

- [ ] **Step 3 : implémenter**

`crates/oku-slides-mcp/Cargo.toml` : `uuid.workspace = true`. `edit_tools.rs:31-35` :

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SetArgs {
    /// Id d'un pack de modèles (UUID, voir `templates`).
    #[schemars(with = "String")]
    pub set: uuid::Uuid,
}
```

`guarded_tools.rs` (`delete_template_set`) : `("name", set.to_string().into())`. `tools.rs` : descriptions

```rust
            tool::<NoArgs>("templates", "Template packs (built-in and the user's), each in its current version, with their templates."),
            tool::<crate::edit_tools::SetArgs>("edit_templates", "Switch to editing the current version of a template pack (the deck is put aside, nothing is lost)."),
            tool::<crate::guarded_tools::SaveTemplatesArgs>("save_templates", "Save the template pack being edited as a new version (as_new: a copy with a new id). The user confirms first."),
```

(L'outil `widgets` ne change pas : il lit le registre installé, celui de la version embarquée.)

- [ ] **Step 4 : lancer** — `cargo nextest run -p oku-slides-session -p oku-slides-mcp -p oku-slides-agent` puis `cargo build -p oku-slides-app` (nom du paquet de `src-tauri`, voir son `Cargo.toml`) → PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-session/tests crates/oku-slides-mcp Cargo.lock
```
```bash
git commit -m "test(session) : un pack reçu de même id ne change jamais la version courante ; mcp : packs par id (t-328)"
```

---

### Task 8 : CLI — `save` embarque le pack, rendu par le processus de rendu

**Files:**
- Create: `crates/oku-slides-cli/src/pack_render.rs`
- Modify: `crates/oku-slides-cli/src/main.rs:7-11` (mod), `:21-25` (imports), `:214-244` (`Render`), `:245-276` (`Save`)
- Modify: `crates/oku-slides-cli/Cargo.toml` (`[dev-dependencies]` : `uuid`)
- Modify: `crates/oku-slides-cli/tests/cli.rs`

**Interfaces:**
- Consumes : `SourceBundle.packs` (Task 4), `Library`, `DEFAULT_PACK` (Tasks 2, 5), `RenderWorker::{new, probe, slot}`, `RenderSlot::{set_widgets, render}`, `Platform::render_launch` (t-327).
- Produces : `pack_render::render(platform: &Platform, l: &Localizer, fonts: Fonts, bundle: &SourceBundle, deck: &Deck) -> Result<Rendered>` ; `Rendered::{page_count, warnings, layout, svg, pdf}`.

- [ ] **Step 1 : tests en échec** — `crates/oku-slides-cli/tests/cli.rs` :

```rust
use std::path::Path;
use std::sync::Arc;

use oku_slides_core::{Deck, PackFiles, PackRef, SourceBundle, SourceLimits, WidgetRegistry};

const LIMITS: SourceLimits = SourceLimits { replica_bytes: 1 << 24, total_bytes: 1 << 26 };

/// Le pack `quote` du plan 1.
const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let quote = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
#let registry = (quote,)
"#;

/// Un widget `boucle` qui boucle au rendu, hors des gardes de Typst.
const LOOP_ON_RENDER: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let boucle = widget("boucle", params: (body: p.plain()),
  (args, ctx) => { let n = 0; for _ in range(100000) { for _ in range(100000) { n += 1 } }; [#n] })
#let registry = (boucle,)
"#;

fn base_registry() -> Arc<WidgetRegistry> {
    let probe = serde_json::from_str(include_str!("../../../fixtures/widgets/base-registry.json")).unwrap();
    Arc::new(WidgetRegistry::from_probe(probe).unwrap())
}

fn pack_files(id: uuid::Uuid, widgets: &str) -> PackFiles {
    let manifest = serde_json::json!({ "id": id, "name": "Pack", "description": "", "parent": null,
        "created": "2026-10-01T00:00:00Z", "templates": [] });
    PackFiles::new(manifest.as_object().cloned().unwrap(), Some(widgets.to_owned()))
}

/// Écrit un pack de l'utilisateur dans la bibliothèque de `dir` ; sa référence.
fn user_pack(dir: &Path, widgets: &str) -> PackRef {
    let id = uuid::Uuid::new_v4();
    let files = pack_files(id, widgets);
    let pack = dir.join("data").join("templates").join(id.to_string());
    let at = pack.join("versions").join(files.version().hex());
    std::fs::create_dir_all(&at).unwrap();
    std::fs::write(at.join("pack.json"), files.pack_json().unwrap()).unwrap();
    std::fs::write(at.join("widgets.typ"), widgets).unwrap();
    std::fs::write(pack.join("current"), files.version().as_str()).unwrap();
    PackRef { id, version: files.version() }
}

fn sources_of(pdf: &Path) -> SourceBundle {
    let found = oku_slides_pdf::extract_sources(&std::fs::read(pdf).unwrap()).unwrap().unwrap();
    SourceBundle::from_zip(&found.zip, LIMITS, base_registry()).unwrap().0
}

#[test]
fn save_of_a_deck_without_templates_embeds_the_default_pack() {
    let dir = workdir("pack-default");
    let pdf = dir.join("deck.oku-slides.pdf");
    assert_that!(run(oku_slides(&dir, "en").args(["save", FIXTURE]).arg(&pdf)).status.success()).is_true();
    let bundle = sources_of(&pdf);
    let r = bundle.deck().templates.clone().unwrap();
    assert_that!(r.id).is_equal_to(oku_slides_session::DEFAULT_PACK);
    assert_that!(bundle.packs.contains_key(&r.version)).is_true();
}

#[test]
fn save_embeds_the_referenced_pack_and_renders_its_widgets() {
    let dir = workdir("pack-quote");
    let r = user_pack(&dir, QUOTE);
    let deck = dir.join("deck.json");
    std::fs::write(&deck, serde_json::to_vec(&serde_json::json!({ "version": 3, "templates": r,
        "slides": [{ "id": "s", "kind": "content", "params": { "title": "T", "blocks": [
            { "id": "q", "kind": "quote", "params": { "body": [{ "runs": [{ "t": "Tout coule." }] }], "source": "H" } }
        ] } }] })).unwrap()).unwrap();
    let pdf = dir.join("deck.oku-slides.pdf");
    let saved = run(oku_slides(&dir, "en").arg("save").arg(&deck).arg(&pdf));
    assert_that!(saved.status.success()).is_true();
    assert_that!(stdout(&saved)).contains(": 1 pages, PDF");
    assert_that!(sources_of(&pdf).packs.contains_key(&r.version)).is_true();
    let out = dir.join("out");
    assert_that!(run(oku_slides(&dir, "en").arg("render").arg(&pdf).arg("--out").arg(&out)).status.success()).is_true();
}

/// Un PDF reçu dont le pack boucle : la CLI s'arrête au délai, sans geler.
#[test]
fn render_of_a_pdf_whose_pack_loops_stops_at_the_timeout() {
    let dir = workdir("pack-loop");
    let base = dir.join("base.oku-slides.pdf");
    assert_that!(run(oku_slides(&dir, "en").args(["save", FIXTURE]).arg(&base)).status.success()).is_true();
    let pdf = oku_slides_pdf::strip_sources(&std::fs::read(&base).unwrap()).unwrap();
    let id = uuid::Uuid::new_v4();
    let files = pack_files(id, LOOP_ON_RENDER);
    let deck: Deck = serde_json::from_value(serde_json::json!({ "version": 3,
        "templates": { "id": id, "version": files.version() },
        "slides": [{ "id": "s", "kind": "content", "params": { "blocks": [
            { "id": "l", "kind": "boucle", "params": { "body": "x" } }] } }] })).unwrap();
    let mut bundle = SourceBundle::new(deck, base_registry()).unwrap();
    bundle.add_pack(files);
    let looping = dir.join("boucle.oku-slides.pdf");
    std::fs::write(&looping, oku_slides_pdf::embed_sources(&pdf, &bundle.to_zip().unwrap()).unwrap()).unwrap();

    let started = Instant::now();
    let mut child = oku_slides(&dir, "en")
        .env("OKU_SLIDES_RENDER__TIMEOUT_MS", "1000")
        .arg("render")
        .arg(&looping)
        .arg("--out")
        .arg(dir.join("out"))
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() > Duration::from_secs(30) {
            let _ = child.kill();
            panic!("la CLI est gelée par le pack qui boucle");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    assert_that!(status.success()).is_false();
    let mut err = String::new();
    child.stderr.take().unwrap().read_to_string(&mut err).unwrap();
    assert_that!(err).contains("1000 ms");
}
```

`crates/oku-slides-cli/Cargo.toml`, `[dev-dependencies]` : `uuid.workspace = true` (`oku-slides-core`, `oku-slides-pdf`, `oku-slides-session` sont des dépendances normales, visibles des tests).

- [ ] **Step 2 : lancer** — `cargo nextest run -p oku-slides-cli --test cli pack` → FAIL (compilation de la CLI : `deck.templates`, puis `save` sans pack embarqué, rendu en mémoire qui gèle — le test du délai panique à 30 s).

- [ ] **Step 3 : implémenter** — `crates/oku-slides-cli/src/pack_render.rs` :

```rust
//! Rendu d'un deck pour la CLI : en mémoire, ou par le processus de rendu
//! quand le pack qu'il embarque a des widgets. Le code d'un pack reçu y est
//! isolé et borné (délai, mémoire : spec t-327).

use color_eyre::eyre::Result;

use oku_slides_core::{Deck, SourceBundle};
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_platform::Platform;
use oku_slides_render::{Compiled, Diagnostic, Fonts, Layout, PackWidgets, RenderError, Renderer};
use oku_slides_render_worker::{RemoteCompiled, RenderSlot, RenderWorker};

/// Un rendu, fait en mémoire ou par le processus de rendu.
pub(crate) enum Rendered {
    Local(Compiled),
    /// L'emplacement vit autant que le rendu : ses pages s'exportent à la demande.
    Remote { compiled: RemoteCompiled, _slot: RenderSlot },
}

impl Rendered {
    pub(crate) fn page_count(&self) -> usize {
        match self {
            Self::Local(c) => c.page_count(),
            Self::Remote { compiled, .. } => compiled.page_count(),
        }
    }

    pub(crate) fn warnings(&self) -> &[Diagnostic] {
        match self {
            Self::Local(c) => &c.warnings,
            Self::Remote { compiled, .. } => &compiled.warnings,
        }
    }

    pub(crate) fn layout(&self) -> &Layout {
        match self {
            Self::Local(c) => c.layout(),
            Self::Remote { compiled, .. } => compiled.layout(),
        }
    }

    pub(crate) fn svg(&self, index: usize) -> Result<String, RenderError> {
        match self {
            Self::Local(c) => c.svg(index),
            Self::Remote { compiled, .. } => compiled.svg(index),
        }
    }

    pub(crate) fn pdf(&self) -> Result<Vec<u8>, RenderError> {
        match self {
            Self::Local(c) => c.pdf(),
            Self::Remote { compiled, .. } => compiled.pdf(),
        }
    }
}

/// Rend `deck` (la vue de `bundle`, thème éventuellement changé) avec les
/// widgets du pack embarqué : par le processus de rendu s'il en a, en
/// mémoire sinon. Un pack cassé laisse la base, comme dans l'app.
pub(crate) fn render(platform: &Platform, l: &Localizer, fonts: Fonts, bundle: &SourceBundle, deck: &Deck) -> Result<Rendered> {
    let widgets = deck.templates.as_ref().and_then(|r| Some((r, bundle.packs.get(&r.version)?.widgets()?)));
    let Some((r, source)) = widgets else {
        let mut renderer = Renderer::new(fonts)?;
        renderer.set_language(l)?;
        renderer.set_deck(deck, &bundle.assets)?;
        return Ok(Rendered::Local(renderer.compile()?));
    };
    let worker = RenderWorker::new(platform.render_launch.clone(), platform.settings.render.clone());
    let base = oku_slides_render::base_registry(&fonts)?;
    let mut slot = worker.slot(&fonts, l, base)?;
    let pack = PackWidgets::new(r.version.hex(), source)?;
    match worker.probe(&pack) {
        Ok(registry) => slot.set_widgets(Some(&pack), registry)?,
        Err(e) => tracing::warn!("{}", e.localize(l)),
    }
    let compiled = slot.render(deck, &bundle.assets)?;
    Ok(Rendered::Remote { compiled, _slot: slot })
}
```

`main.rs` : `mod pack_render;` ; imports `use oku_slides_core::{Deck, PackFiles, PackRef, SourceBundle, SourceLimits, WidgetRegistry, validate};`, `use oku_slides_render::Fonts;` (`Renderer` n'y sert plus : il passe dans `pack_render`), `use oku_slides_session::{DEFAULT_PACK, Library, Storage, read_document};`. Fonction :

```rust
/// Le pack que `deck` embarquera : la version qu'il référence, prise dans la
/// bibliothèque, ou la courante du pack par défaut (alors référencée). Une
/// version absente est signalée : le deck s'enregistre sans elle.
fn embed_pack(library: &Library, l: &Localizer, deck: &mut Deck) -> Result<Option<PackFiles>> {
    let r = match &deck.templates {
        Some(r) => r.clone(),
        None => {
            let unknown = || l.tr_with("session-unknown-pack", &[("id", DEFAULT_PACK.to_string().into())]);
            let version = library.current(DEFAULT_PACK).ok_or_else(|| color_eyre::eyre::eyre!(unknown()))?;
            let r = PackRef { id: DEFAULT_PACK, version };
            deck.templates = Some(r.clone());
            r
        }
    };
    let files = library.version(r.id, &r.version);
    if files.is_none() {
        let args = [("id", r.id.to_string().into()), ("version", r.version.as_str().into())];
        tracing::warn!("{}", l.tr_with("session-pack-version-missing", &args));
    }
    Ok(files)
}
```

Bras `Render` (lignes 221-226) :

```rust
            let t = Instant::now();
            let doc = pack_render::render(platform, l, app_fonts.with(bundle.fonts.values().cloned()), &bundle, &deck)?;
            let compile = t.elapsed();
            for w in doc.warnings() {
```

(le reste inchangé : `doc.page_count()`, `doc.svg(i)?`, `doc.layout()`). Bras `Save` (lignes 245-276) :

```rust
        Cmd::Save { deck, output, assets, embed_fonts, theme } => {
            let registry = registry(&app_fonts)?;
            let mut deck = read_deck(storage, l, &deck)?;
            if theme.is_some() {
                deck.theme = theme;
            }
            let library = Library::new(Some(platform.settings.paths.templates()), storage.clone());
            let pack = embed_pack(&library, l, &mut deck)?;
            let mut bundle = SourceBundle::new(deck, registry.clone())?;
            if let Some(files) = pack {
                bundle.add_pack(files);
            }
            // … assets et polices : lignes 254-262 inchangées …
            report_issues(l, bundle.deck(), &registry, &bundle.assets);
            let doc = pack_render::render(platform, l, app_fonts.clone(), &bundle, &bundle.deck().clone())?;
            let pdf = doc.pdf()?;
            // … zip, `embed_sources`, écriture et ligne `cli-saved` inchangés …
        }
```

Le commentaire de `registry` (lignes 167-169) : « … partagé avec le rendu en mémoire ; le pack d'un deck passe par `pack_render`. » `bench.rs` ne change pas (decks JSON sans pack embarqué).

- [ ] **Step 4 : lancer** — `cargo nextest run -p oku-slides-cli` → PASS ; `cargo build --workspace` → PASS (tout le workspace compile de nouveau).

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-cli Cargo.lock
```
```bash
git commit -m "feat(cli) : save embarque le pack du deck, rendu par le processus de rendu quand il a des widgets (t-328)"
```

---

### Task 9 : UI — bandeau du pack, packs par id

**Files:**
- Create: `ui/src/PackNoticeBar.tsx`, `ui/src/PackNoticeBar.render.test.tsx`
- Modify: `ui/src/api.ts:35-60` (`Snapshot`), `:95-116` (`TemplateMode`, `SetInfo` → `PackInfo`, `TemplatesView`), `:302-326` (`Request`), `:466-495` (`tabApi`)
- Modify: `ui/src/App.tsx:2` (imports), `:482-506` (modèles), `:701-710` (menu), `:733-757` (bandeau, légende), `:889-895` (sélecteur)
- Modify: `ui/src/TemplatePicker.tsx:4-25,46-50`, `ui/src/autosave.ts:10-29`, `ui/src/autosave.test.ts:31-32`, `ui/src/styles.css` (après `.history-actions`, ligne 699)
- Modify: `crates/oku-slides-i18n/src/locales/*/ui.ftl` (20 langues)

**Interfaces:**
- Consumes : `Snapshot.templates: PackRef | null`, `Snapshot.pack_notice`, requêtes `pack_use_mine`, `pack_make_current`, `pack_notice_close`, `PackInfo` (Tasks 5-6).
- Produces :
  - `export interface PackRef { id: string; version: string }`, `export type PackRelation = "ancestor" | "descendant" | "sibling"`, `export interface PackNotice { pack: string; name: string; relation: PackRelation; message: string; use_mine: string; make_current: string }`, `export interface PackInfo { id: string; version: string; name: string; description: string; builtin: boolean; default: boolean; count: number }`.
  - `tabApi(tab).{packUseMine, packMakeCurrent, packNoticeClose}: () => Promise<Snapshot>`.
  - `PackNoticeBar({ notice, onUseMine, onMakeCurrent, onClose })`.

- [ ] **Step 1 : tests en échec**

`ui/src/PackNoticeBar.render.test.tsx` :

```tsx
// Rendu du bandeau du pack dans un DOM happy-dom.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";

// Voir ./a2ui/catalog.render.test.tsx : React se charge après le DOM.
const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const PackNoticeBar = (await import("./PackNoticeBar")).default;
const { expectIconOnly } = await import("./test/iconOnly");

type Root = ReturnType<typeof createRoot>;
let mounted: { root: Root; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const notice = {
  pack: "6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10",
  name: "Charte",
  relation: "descendant" as const,
  message: "Ce deck utilise une version plus récente de « Charte » que la vôtre.",
  use_mine: "Passer à ma version",
  make_current: "Faire de la sienne ma version courante",
};

async function mount(calls: string[]) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => {
    root.render(createElement(PackNoticeBar, {
      notice,
      onUseMine: () => calls.push("mine"),
      onMakeCurrent: () => calls.push("current"),
      onClose: () => calls.push("close"),
    }));
  });
  return host;
}

describe("PackNoticeBar", () => {
  test("PackNoticeBar shows the message and both actions from the session", async () => {
    const host = await mount([]);
    expect(host.querySelector(".pack-notice-text")?.textContent).toBe(notice.message);
    const [mine, current] = host.querySelectorAll(".pack-notice-actions button");
    expect(mine.textContent).toBe(notice.use_mine);
    expect(current.textContent).toBe(notice.make_current);
  });

  test("PackNoticeBar buttons call their action", async () => {
    const calls: string[] = [];
    const host = await mount(calls);
    const [mine, current, close] = host.querySelectorAll("button");
    await act(async () => (mine as HTMLButtonElement).click());
    await act(async () => (current as HTMLButtonElement).click());
    await act(async () => (close as HTMLButtonElement).click());
    expect(calls).toEqual(["mine", "current", "close"]);
  });

  test("PackNoticeBar close button shows an icon only", async () => {
    const host = await mount([]);
    expectIconOnly(host.querySelector(".pack-notice-close"));
  });
});
```

`ui/src/autosave.test.ts:31-32` : les modèles s'enregistrent toujours, intégrés compris (la version de la session d'édition, remplacée à chaque enregistrement) :

```ts
    expect(wantsAutosave({ dirty: true, path: null, editing: { builtin: false } }, false)).toBe(true);
    expect(wantsAutosave({ dirty: true, path: null, editing: { builtin: true } }, false)).toBe(true);
```

- [ ] **Step 2 : lancer** — `cd ui && bun test PackNoticeBar autosave` → FAIL (composant absent, intégré non enregistré).

- [ ] **Step 3 : implémenter**

`ui/src/PackNoticeBar.tsx` :

```tsx
// Bandeau du pack : le deck embarque une autre version que ma version
// courante. Textes et libellés viennent de la session, dans la langue de
// l'utilisateur ; il ne bloque rien et se ferme pour la session.

import type { PackNotice } from "./api";
import { t } from "./i18n";
import { Icon } from "./icons";

export default function PackNoticeBar({ notice, onUseMine, onMakeCurrent, onClose }: {
  notice: PackNotice;
  /** Passer le deck à ma version courante (annulable). */
  onUseMine: () => void;
  /** Faire de la sienne ma version courante (le deck ne change pas). */
  onMakeCurrent: () => void;
  onClose: () => void;
}) {
  const close = t("ui-pack-notice-close");
  return (
    <div className="object pack-notice" role="status">
      <span className="pack-notice-text">{notice.message}</span>
      <span className="pack-notice-actions">
        <button className="token" onClick={onUseMine}>{notice.use_mine}</button>
        <button className="token ghost" onClick={onMakeCurrent}>{notice.make_current}</button>
      </span>
      <button className="token ghost pack-notice-close" aria-label={close} title={close} onClick={onClose}>{Icon.close}</button>
    </div>
  );
}
```

`api.ts` : ajouter `PackRef`, `PackRelation`, `PackNotice`, `PackInfo` (voir **Interfaces**) ; dans `Snapshot` :

```ts
  /** Pack de modèles du deck : id et version embarquée. */
  templates: PackRef | null;
  /** Édition d'un pack de modèles en cours. */
  editing: TemplateMode | null;
  /** Le deck embarque une autre version de son pack que ma version courante. */
  pack_notice: PackNotice | null;
```

`TemplateMode` gagne `/** Version dont viennent widgets et description. */ base: string; /** Parente des versions de la session. */ parent: string | null; /** Version de cette session d'édition, une fois enregistrée. */ session_version: string | null;` et sa doc de `builtin` devient « Pack intégré : l'enregistrer crée ma version sous son id. » ; `SetInfo` est remplacé par `PackInfo` (et `TemplatesView.set: PackInfo; sets: PackInfo[]`) ; `Request` gagne `| { cmd: "pack_use_mine" } | { cmd: "pack_make_current" } | { cmd: "pack_notice_close" }` ; `tabApi` :

```ts
    templateSets: () => call<PackInfo[]>({ cmd: "template_sets" }),
    deleteTemplateSet: (set: string) => call<PackInfo[]>({ cmd: "delete_template_set", set }),
    /** Bandeau du pack : le deck passe à ma version courante (annulable). */
    packUseMine: () => call<Snapshot>({ cmd: "pack_use_mine" }),
    /** Bandeau du pack : sa version devient ma version courante. */
    packMakeCurrent: () => call<Snapshot>({ cmd: "pack_make_current" }),
    packNoticeClose: () => call<Snapshot>({ cmd: "pack_notice_close" }),
```

`App.tsx` :
- import : `PackInfo` à la place de `SetInfo` ; `import PackNoticeBar from "./PackNoticeBar";`.
- `editTemplates` (482-488) : `if (!snap?.templates) return; if (await run(api.editTemplates(snap.templates.id))) { … }`.
- `finishTemplates` : le commentaire « Seuls des modèles intégrés modifiés restent à enregistrer (en copie). » devient « Un enregistrement refusé laisse des modèles non enregistrés. ».
- `adoptSet` (503-506) :

```tsx
  const adoptSet = (set: PackInfo) => {
    if (!snap || snap.editing || set.id === snap.templates?.id) return;
    apply(t("ui-undo-templates", { name: set.name }), [{ op: "set_templates", templates: { id: set.id, version: set.version } }]);
  };
```

- menu des modèles (701-705) : l'entrée « Enregistrer une copie » est toujours là, `{ kind: "item", label: t("ui-toolbar-save-copy"), onSelect: () => saveTemplates(true) },` sans condition sur `builtin`.
- légende (752-755) : `status: t(snap.dirty ? "ui-status-unsaved" : "ui-status-saved"),`.
- bandeau : juste après `) : (<>` (ligne 741), avant le `<header>` :

```tsx
      {snap.pack_notice && (
        <PackNoticeBar
          notice={snap.pack_notice}
          onUseMine={() => run(api.packUseMine())}
          onMakeCurrent={() => run(api.packMakeCurrent())}
          onClose={() => run(api.packNoticeClose())}
        />
      )}
```

- sélecteur (892) : `initialSet={snap.editing ? snap.editing.set : snap.templates?.id ?? null}`.

`TemplatePicker.tsx` : `PackInfo` à la place de `SetInfo` ; `initialSet: string | null` (doc « Pack montré à l'ouverture ; `null` : celui du deck. ») ; `useState<string | null>(initialSet)`.

`autosave.ts` : `Saveable.editing: unknown` (doc « Modèles en cours d'édition : enregistrer crée la version de la session, puis la remplace. ») ; `wantsAutosave` :

```ts
/**
 * Faut-il enregistrer ? Un deck modifié, avec ou sans fichier (il ira au
 * dossier par défaut) ; des modèles modifiés, intégrés compris : une version
 * par session d'édition, créée au premier enregistrement puis remplacée. `paused` : un enregistrement a été refusé (fichier
 * remplacé). Jamais pendant l'historique (la session le refuserait) : le
 * deck mis de côté s'enregistre à sa fermeture.
 */
export function wantsAutosave(s: Saveable, paused: boolean): boolean {
  return !paused && s.dirty && !s.history;
}
```

`styles.css`, après `.history-actions` :

```css
.pack-notice { display: flex; align-items: center; gap: var(--space-2); }
.pack-notice-text { flex: 1; }
.pack-notice-actions { display: flex; gap: var(--space-2); }
```

Fluent — `ui.ftl` de **chaque** langue : retirer `ui-status-builtin` ; ajouter

```ftl
# fr
ui-pack-notice-close = Fermer
```

```ftl
# en
ui-pack-notice-close = Close
```

- [ ] **Step 4 : lancer** — `cd ui && bun run typecheck && bun run test` → PASS (dont `i18n-keys.test.ts` : clé utilisée et présente en fr/en) ; `cargo nextest run -p oku-slides-i18n` → PASS.

- [ ] **Step 5 : commit**

```bash
git add ui crates/oku-slides-i18n
```
```bash
git commit -m "feat(ui) : bandeau du pack embarqué, packs de modèles par id (t-328)"
```

---

### Task 10 : documentation, consolidation et fermeture

**Files:**
- Modify: `crates/oku-slides-core/ARCHITECTURE.md` (`Deck`, `SourceBundle`, nouveau `pack.rs`, dépendance `uuid`)
- Modify: `crates/oku-slides-session/ARCHITECTURE.md:10-18` (bibliothèque versionnée, pack embarqué, bandeau)
- Modify: `crates/oku-slides-mcp/ARCHITECTURE.md:20` (si la ligne parle d'ensembles)
- Vérifier : `typst/oku-slides/README.md` (v3, Task 3), `crates/oku-slides-pdf/ARCHITECTURE.md` (Task 4)

- [ ] **Step 1 : documents** (chacun **sous 50 lignes** ; core en compte 50 aujourd'hui : condenser une puce existante en même temps).
  - core : puce `Deck` « … `templates` (`PackRef` : id et version embarquée, atome du replica) » ; nouvelle puce « `PackVersion`/`PackRef`/`PackFiles` (`pack.rs`) : version adressée par contenu (sha-256 de `pack.json` à clés triées, `0`, `widgets.typ`) » ; puce `SourceBundle` « Zip v4 : snapshot Loro, assets, polices, `packs/<hex>/` en ajout seul, vérifiés à la lecture (`issue-pack-version-mismatch`), unis à la fusion » ; dépendances : `uuid`.
  - session, lignes 12-18 réécrites : « `deck_widgets.rs` : `DeckWidgets`, la base surchargée par le `widgets.typ` de la version de pack **embarquée** (`SourceBundle.packs[deck.templates.version]`, jamais la bibliothèque) ; absente : base + `session-pack-missing`. `deck_pack.rs` : deck sans pack → version courante du pack par défaut, embarquée et référencée ; `SetTemplates` embarque depuis la bibliothèque. `library.rs` : `Library` versionnée (`<data>/templates/<uuid>/{current, versions/<hex>/}`), packs intégrés d'id constant (`pack.rs`). `pack_notice.rs` : versions reçues ajoutées en silence, bandeau `pack_notice` (Passer à ma version / Faire de la sienne ma version courante). » ; la phrase sur `is_pack_id` et le dossier `<id>/` disparaît.
- [ ] **Step 2 : consolidation** —
  - `rg "DEFAULT_SET|TemplateSet\b|SetInfo|free_id|is_pack_id\(&set|templates\.as_deref" crates ui/src src-tauri` → rien (hors `DeleteTemplateSet`, `BuiltinTemplateSet`, `templateSets`).
  - `rg '"version": ?2' fixtures crates/*/src crates/*/tests typst ui/src` → seuls les `workspace.json` et `state.json` de test (autres formats).
  - `rg "std::fs::" crates/oku-slides-session/src crates/oku-slides-core/src crates/oku-slides-cli/src -g '!*_tests.rs' -g '!test_support.rs'` → rien de nouveau.
  - `rg "session-no-template-set|session-invalid-template-set|ui-status-builtin" crates/oku-slides-i18n` → rien.
- [ ] **Step 3** : `just check` → PASS.
- [ ] **Step 4 : commit**

```bash
git add crates/*/ARCHITECTURE.md
```
```bash
git commit -m "docs(core) : packs portables, bibliothèque versionnée et bandeau du pack (t-328)"
```
