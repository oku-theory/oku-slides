# Compilation Typst bornée — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** tout Typst venu d'un pack tourne dans un sous-processus persistant, borné en temps et en mémoire ; au-delà, il est tué et relancé, et la session se replie sur les widgets de base.

**Architecture:** nouveau crate `oku-slides-render-worker` : `serve()` (boucle du travailleur, emplacements de `Renderer`), `RenderWorker` (lien hôte : transport `Process` ou `InProcess`, chien de garde délai + RSS, relance, rejeu, empreintes fautives), `RenderSlot` (même surface que `Renderer`), `RemoteCompiled` (poignée d'une génération). La session remplace ses `Renderer` par des `RenderSlot` d'un même `RenderWorker` et porte la politique de repli.

**Tech Stack:** Rust 2024, Typst 0.15.1, serde/serde_json, snafu, sysinfo (nouveau), speculoos, rstest, Fluent.

**Spec:** `docs/superpowers/specs/2026-09-29-compilation-bornee-design.md` (t-327)

## Global Constraints

- Réglages `[render]` : `timeout_ms = 10000`, `memory_mib = 2048`, `max_frame_mib = 512` ; variables `OKU_SLIDES_RENDER__TIMEOUT_MS`, `…__MEMORY_MIB`, `…__MAX_FRAME_MIB`.
- Relevé de la RSS toutes les **100 ms**.
- Trame : `u32` big-endian (longueur de l'en-tête) · en-tête JSON · `u32` big-endian (longueur de la charge) · charge brute.
- Drapeau du travailleur : premier argument `--render-worker`, suivi de `--max-frame-bytes <n>`.
- `Platform::load()` → `RenderLaunch::Process { program: current_exe() }` ; `Platform::with_settings()` → `RenderLaunch::InProcess`.
- `sysinfo` : `default-features = false`, `features = ["system"]`, version `0.37`.
- Erreurs : Snafu, backtrace dans chaque variante, messages en minuscules ; tout texte visible via Fluent, **20 langues** (`crates/oku-slides-i18n/src/locales/<l>/`).
- Pas de `.unwrap()`/`.expect()` hors tests ; pas de `std::fs` direct (OpenDAL/`Storage`) — le crate n'a pas de fichier à lire.
- Tests côte à côte `foo_tests.rs` (`#[cfg(test)] #[path = "foo_tests.rs"] mod foo_tests;`), speculoos, nommage `<unité>_<scénario>_<résultat>`.
- Commits : `<type>(<scope>) : <description> (t-327)`, **sans** ligne `Co-Authored-By`. `git add` dans une commande séparée du `git commit` (le hook de revue bloque sinon toute la commande).
- Vérification d'un crate : `cargo nextest run -p <crate>` ; en fin de plan : `just check`.

---

## Carte des fichiers

| Fichier | Rôle |
|---|---|
| `crates/oku-slides-render/src/diagnostic.rs` | `Diagnostic`, `Severity` sérialisables |
| `crates/oku-slides-render/src/error.rs` | variantes `Timeout`, `OutOfMemory`, `WorkerLost`, `Frame`, `Stale`, `PackDisabled`, `Remote` ; `RenderErrorKind::is_bounded` |
| `crates/oku-slides-render/src/wire.rs` (nouveau) | `WireError` : erreurs de rendu et de registre à travers le tube |
| `crates/oku-slides-render/src/registry.rs` | `probe_registry`, `registry_from_probe`, `base_registry` public, `PackWidgets::fingerprint`, `Registries::probe` |
| `crates/oku-slides-render/src/fonts.rs` | `Fonts::files` |
| `crates/oku-slides-render/src/lib.rs` | `Layout: Deserialize`, réexports |
| `crates/oku-slides-platform/src/config.rs` + `config.default.toml` | `RenderSettings` |
| `crates/oku-slides-platform/src/lib.rs` | `RenderLaunch`, champ `Platform::render_launch` |
| `crates/oku-slides-render-worker/src/frame.rs` | trames |
| `crates/oku-slides-render-worker/src/protocol.rs` | `Request`, `Response`, `CompiledHeader`, `WirePack` |
| `crates/oku-slides-render-worker/src/serve.rs` | boucle du travailleur, `exit_if_worker` |
| `crates/oku-slides-render-worker/src/rss.rs` | mémoire résidente d'un processus |
| `crates/oku-slides-render-worker/src/link.rs` | transports, chien de garde, relance, rejeu |
| `crates/oku-slides-render-worker/src/client.rs` | `RenderWorker`, `RenderSlot`, `RemoteCompiled` |
| `crates/oku-slides-render-worker/src/bin/oku-slides-render-worker.rs` | binaire de test (jamais empaqueté) |
| `crates/oku-slides-session/src/{lib,deck_widgets,render_fonts,history,templates_mode,workspace,workspace_same_deck}.rs`, `agent/variants.rs` | migration |
| `crates/oku-slides-session/tests/regressions/issue_0327_pack_qui_boucle.rs` | régression, vrai processus |
| `src-tauri/src/main.rs`, `crates/oku-slides-cli/src/main.rs` | `exit_if_worker()` en tête |

---

### Task 1 : erreurs bornées et erreurs à travers le tube (render)

**Files:**
- Modify: `crates/oku-slides-render/src/diagnostic.rs:7-21`
- Modify: `crates/oku-slides-render/src/error.rs`
- Create: `crates/oku-slides-render/src/wire.rs`, `crates/oku-slides-render/src/wire_tests.rs`
- Modify: `crates/oku-slides-render/src/lib.rs` (mod + réexports ; `Layout` dérive `Deserialize`)
- Modify: `crates/oku-slides-i18n/src/locales/*/document.ftl` (20 langues)

**Interfaces:**
- Produces :
  - `RenderErrorKind::{Timeout, OutOfMemory, WorkerLost, Frame, Stale, PackDisabled}` et `RenderErrorKind::is_bounded(self) -> bool` (vrai pour `Timeout`, `OutOfMemory`, `WorkerLost`, `PackDisabled`).
  - Constructeurs `RenderError::timeout(ms: u64)`, `::out_of_memory(mib: u64)`, `::worker_lost(message: impl Into<String>)`, `::frame(message: impl Into<String>)`, `::stale()`, `::pack_disabled(pack: impl Into<String>)`.
  - `RegistryError::worker(source: RenderError) -> RegistryError` (kind `Setup`) et `RegistryError::render_error(&self) -> Option<&RenderError>`.
  - `pub enum WireError` (serde) avec `WireError::from_render(&RenderError)`, `WireError::from_registry(&RegistryError)`, `WireError::into_render(self) -> RenderError`, `WireError::into_registry(self) -> RegistryError`.
  - `Diagnostic`, `Severity`, `Layout` : `Serialize + Deserialize`.

- [ ] **Step 1 : tests en échec** — `crates/oku-slides-render/src/wire_tests.rs`

```rust
use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::{Diagnostic, RenderError, RenderErrorKind, Severity};

fn diag() -> Diagnostic {
    Diagnostic { severity: Severity::Error, place: Some("packs/p/widgets.typ:3".into()), message: "boom".into(), hints: vec!["h".into()] }
}

#[rstest]
#[case(RenderError::timeout(10_000), RenderErrorKind::Timeout)]
#[case(RenderError::out_of_memory(2048), RenderErrorKind::OutOfMemory)]
#[case(RenderError::worker_lost("exit 9"), RenderErrorKind::WorkerLost)]
#[case(RenderError::frame("too large"), RenderErrorKind::Frame)]
#[case(RenderError::stale(), RenderErrorKind::Stale)]
#[case(RenderError::pack_disabled("mien"), RenderErrorKind::PackDisabled)]
fn wire_error_round_trip_keeps_render_kind(#[case] e: RenderError, #[case] kind: RenderErrorKind) {
    let json = serde_json::to_string(&WireError::from_render(&e)).unwrap();
    let back: WireError = serde_json::from_str(&json).unwrap();
    assert_that!(back.into_render().kind()).is_equal_to(kind);
}

#[test]
fn wire_error_round_trip_keeps_compile_diagnostics() {
    let e: RenderError = crate::error::CompileSnafu { diagnostics: vec![diag()] }.build().into();
    let back = WireError::from_render(&e).into_render();
    assert_that!(back.diagnostics().to_vec()).is_equal_to(vec![diag()]);
}

#[test]
fn wire_error_round_trip_keeps_registry_compile_diagnostics() {
    let e: crate::RegistryError = crate::error::RegistryCompileSnafu { diagnostics: vec![diag()] }.build().into();
    let back = WireError::from_registry(&e).into_registry();
    assert_that!(back.kind()).is_equal_to(crate::RegistryErrorKind::Compile);
}

#[test]
fn registry_worker_error_exposes_its_render_error() {
    let e = crate::RegistryError::worker(RenderError::timeout(5));
    assert_that!(e.render_error().map(RenderError::kind)).is_equal_to(Some(RenderErrorKind::Timeout));
}

#[rstest]
#[case(RenderErrorKind::Timeout, true)]
#[case(RenderErrorKind::OutOfMemory, true)]
#[case(RenderErrorKind::WorkerLost, true)]
#[case(RenderErrorKind::PackDisabled, true)]
#[case(RenderErrorKind::Compile, false)]
#[case(RenderErrorKind::Stale, false)]
fn render_error_kind_is_bounded_matches_worker_failures(#[case] kind: RenderErrorKind, #[case] bounded: bool) {
    assert_that!(kind.is_bounded()).is_equal_to(bounded);
}

#[test]
fn layout_round_trips_through_json() {
    let layout = crate::Layout { slides: vec![crate::SlidePage { slide: "s".into(), page: 1 }], ..Default::default() };
    let back: crate::Layout = serde_json::from_str(&serde_json::to_string(&layout).unwrap()).unwrap();
    assert_that!(back.slides).is_equal_to(layout.slides);
}
```

Ajouter `rstest.workspace = true` aux `[dev-dependencies]` de `oku-slides-render`.

- [ ] **Step 2 : lancer** — `cargo nextest run -p oku-slides-render wire` → FAIL (module `wire` absent).

- [ ] **Step 3 : implémenter**

`diagnostic.rs` : `#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]` sur `Severity`, `#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]` sur `Diagnostic`.

`lib.rs` : `#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]` sur `Layout` (le `#[serde(skip)]` de `tokens` le laisse à `None` en lecture : il voyage à part). `mod wire;` et `pub use wire::WireError;`.

`error.rs` — `RenderErrorKind` gagne :

```rust
    /// Le rendu a dépassé `render.timeout_ms`.
    Timeout,
    /// Le processus de rendu a dépassé `render.memory_mib`.
    OutOfMemory,
    /// Le processus de rendu s'est arrêté (plantage, panique de Typst).
    WorkerLost,
    /// Un message du processus de rendu est illisible ou trop grand.
    Frame,
    /// Le rendu demandé a été remplacé (processus relancé).
    Stale,
    /// Les widgets de ce pack ont déjà fait échouer le rendu : ils ne sont plus exécutés.
    PackDisabled,
```

et

```rust
impl RenderErrorKind {
    /// Un échec du processus de rendu (délai, mémoire, arrêt, pack écarté) :
    /// la session se replie sur les widgets de base.
    pub fn is_bounded(self) -> bool {
        matches!(self, Self::Timeout | Self::OutOfMemory | Self::WorkerLost | Self::PackDisabled)
    }
}
```

`RenderInner` gagne :

```rust
    #[snafu(display("rendering took more than {ms} ms"))]
    Timeout { ms: u64, backtrace: Backtrace },
    #[snafu(display("rendering used more than {mib} MiB"))]
    OutOfMemory { mib: u64, backtrace: Backtrace },
    #[snafu(display("the rendering process stopped: {message}"))]
    WorkerLost { message: String, backtrace: Backtrace },
    #[snafu(display("unreadable message from the rendering process: {message}"))]
    Frame { message: String, backtrace: Backtrace },
    #[snafu(display("the render was replaced"))]
    Stale { backtrace: Backtrace },
    #[snafu(display("the widgets of pack {pack} are disabled"))]
    PackDisabled { pack: String, backtrace: Backtrace },
    #[snafu(display("rendering failed: {message}"))]
    Remote { message: String, backtrace: Backtrace },
```

`kind()` : chacune vers son kind ; `Remote` → `Setup`. Constructeurs :

```rust
impl RenderError {
    pub fn timeout(ms: u64) -> Self { TimeoutSnafu { ms }.build().into() }
    pub fn out_of_memory(mib: u64) -> Self { OutOfMemorySnafu { mib }.build().into() }
    pub fn worker_lost(message: impl Into<String>) -> Self { WorkerLostSnafu { message: message.into() }.build().into() }
    pub fn frame(message: impl Into<String>) -> Self { FrameSnafu { message: message.into() }.build().into() }
    pub fn stale() -> Self { StaleSnafu.build().into() }
    pub fn pack_disabled(pack: impl Into<String>) -> Self { PackDisabledSnafu { pack: pack.into() }.build().into() }
}
```

`Localize for RenderError` : clés `render-timeout` (`$seconds` = `ms / 1000`, au moins 1), `render-out-of-memory` (`$mib`), `render-worker-lost`, `render-frame`, `render-stale`, `render-pack-disabled` (`$pack`) ; `Remote` → `render-setup`.

`RegistryInner` gagne `Worker { source: Box<RenderError>, backtrace: Backtrace }` (display `"the widget probe failed"`, kind `Setup`, `localize` → `source.localize(l)`), et `RemoteSchema { message: String, backtrace: Backtrace }` (kind `Schema`, `localize` → `registry-schema-remote` avec `$message`). Puis :

```rust
impl RegistryError {
    /// La sonde n'a pas abouti dans le processus de rendu.
    pub fn worker(source: RenderError) -> Self { WorkerSnafu { source: Box::new(source) }.build().into() }

    /// L'erreur de rendu à l'origine de celle-ci (délai, mémoire, arrêt).
    pub fn render_error(&self) -> Option<&RenderError> {
        match &self.0 {
            RegistryInner::Worker { source, .. } => Some(source),
            _ => None,
        }
    }
}
```

`wire.rs` :

```rust
//! Erreurs de rendu et de registre, sérialisées pour traverser le tube du
//! processus de rendu (`oku-slides-render-worker`). Les backtraces restent
//! de leur côté ; nature, diagnostics et paramètres passent.

use crate::diagnostic::Diagnostic;
use crate::error::{
    CompileSnafu, ImageSnafu, NoPageSnafu, PdfSnafu, RegistryCompileSnafu, RegistryInner, RemoteSchemaSnafu,
    RemoteSnafu, RenderInner, BadPackIdSnafu, MissingSnafu,
};
use crate::{RegistryError, RenderError};

/// Une [`RenderError`] ou une [`RegistryError`] à travers le tube.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "error", rename_all = "kebab-case")]
pub enum WireError {
    Compile { diagnostics: Vec<Diagnostic> },
    Pdf { diagnostics: Vec<Diagnostic> },
    NoPage { index: usize },
    Image { message: String },
    Timeout { ms: u64 },
    OutOfMemory { mib: u64 },
    WorkerLost { message: String },
    Frame { message: String },
    Stale,
    PackDisabled { pack: String },
    Setup { message: String },
    RegistryBadPackId { id: String },
    RegistryCompile { diagnostics: Vec<Diagnostic> },
    RegistryMissing,
    RegistrySchema { message: String },
    RegistrySetup { message: String },
}

impl WireError {
    pub fn from_render(e: &RenderError) -> Self {
        match &e.0 {
            RenderInner::Compile { diagnostics, .. } => Self::Compile { diagnostics: diagnostics.clone() },
            RenderInner::Pdf { diagnostics, .. } => Self::Pdf { diagnostics: diagnostics.clone() },
            RenderInner::NoPage { index, .. } => Self::NoPage { index: *index },
            RenderInner::Image { message, .. } => Self::Image { message: message.clone() },
            RenderInner::Timeout { ms, .. } => Self::Timeout { ms: *ms },
            RenderInner::OutOfMemory { mib, .. } => Self::OutOfMemory { mib: *mib },
            RenderInner::WorkerLost { message, .. } => Self::WorkerLost { message: message.clone() },
            RenderInner::Frame { message, .. } => Self::Frame { message: message.clone() },
            RenderInner::Stale { .. } => Self::Stale,
            RenderInner::PackDisabled { pack, .. } => Self::PackDisabled { pack: pack.clone() },
            RenderInner::Remote { message, .. } => Self::Setup { message: message.clone() },
            other => Self::Setup { message: first_line(other) },
        }
    }

    pub fn from_registry(e: &RegistryError) -> Self {
        match &e.0 {
            RegistryInner::BadPackId { id, .. } => Self::RegistryBadPackId { id: id.clone() },
            RegistryInner::RegistryCompile { diagnostics, .. } => Self::RegistryCompile { diagnostics: diagnostics.clone() },
            RegistryInner::Missing { .. } => Self::RegistryMissing,
            RegistryInner::Schema { source, .. } => Self::RegistrySchema { message: source.to_string() },
            RegistryInner::RemoteSchema { message, .. } => Self::RegistrySchema { message: message.clone() },
            RegistryInner::Setup { source, .. } | RegistryInner::Worker { source, .. } => Self::from_render(source),
        }
    }

    pub fn into_render(self) -> RenderError {
        match self {
            Self::Compile { diagnostics } => CompileSnafu { diagnostics }.build().into(),
            Self::Pdf { diagnostics } => PdfSnafu { diagnostics }.build().into(),
            Self::NoPage { index } => NoPageSnafu { index }.build().into(),
            Self::Image { message } => ImageSnafu { message }.build().into(),
            Self::Timeout { ms } => RenderError::timeout(ms),
            Self::OutOfMemory { mib } => RenderError::out_of_memory(mib),
            Self::WorkerLost { message } => RenderError::worker_lost(message),
            Self::Frame { message } => RenderError::frame(message),
            Self::Stale => RenderError::stale(),
            Self::PackDisabled { pack } => RenderError::pack_disabled(pack),
            Self::Setup { message }
            | Self::RegistrySchema { message }
            | Self::RegistrySetup { message } => RemoteSnafu { message }.build().into(),
            Self::RegistryBadPackId { id } => RemoteSnafu { message: format!("invalid template pack identifier: {id}") }.build().into(),
            Self::RegistryCompile { diagnostics } => CompileSnafu { diagnostics }.build().into(),
            Self::RegistryMissing => RemoteSnafu { message: "the widget probe emitted no registry".to_owned() }.build().into(),
        }
    }

    pub fn into_registry(self) -> RegistryError {
        match self {
            Self::RegistryBadPackId { id } => BadPackIdSnafu { id }.build().into(),
            Self::RegistryCompile { diagnostics } => RegistryCompileSnafu { diagnostics }.build().into(),
            Self::RegistryMissing => MissingSnafu.build().into(),
            Self::RegistrySchema { message } => RemoteSchemaSnafu { message }.build().into(),
            other => RegistryError::worker(other.into_render()),
        }
    }
}

/// Une ligne pour les variantes internes qui ne traversent pas le tube telles quelles.
fn first_line(inner: &RenderInner) -> String {
    inner.to_string().lines().next().unwrap_or_default().to_owned()
}
```

`RenderError(pub(crate) RenderInner)` et `RegistryError(pub(crate) RegistryInner)` : le champ est déjà `pub(crate)`, accessible depuis `wire.rs`. Les sélecteurs Snafu (`TimeoutSnafu`, …) sont générés `pub(crate)` par `#[snafu(visibility(pub(crate)))]`.

Fluent — ajouter à `document.ftl` de **chaque** langue (fr et en ci-dessous ; traduire les 18 autres dans le même registre que leurs clés `render-*` voisines) :

```ftl
# fr
render-timeout = Le rendu a pris plus de { $seconds } s ; il a été interrompu
render-out-of-memory = Le rendu a demandé plus de { $mib } Mio de mémoire ; il a été interrompu
render-worker-lost = Le processus de rendu s'est arrêté
render-frame = Message illisible du processus de rendu
render-stale = Le rendu a été remplacé ; réessayez
render-pack-disabled = Les widgets du pack « { $pack } » sont désactivés pour cette session
registry-schema-remote = Registre des widgets invalide : { $message }
```

```ftl
# en
render-timeout = Rendering took more than { $seconds } s and was stopped
render-out-of-memory = Rendering needed more than { $mib } MiB of memory and was stopped
render-worker-lost = The rendering process stopped
render-frame = Unreadable message from the rendering process
render-stale = The render was replaced; try again
render-pack-disabled = The widgets of pack “{ $pack }” are disabled for this session
registry-schema-remote = Invalid widget registry: { $message }
```

- [ ] **Step 4 : lancer** — `cargo nextest run -p oku-slides-render` puis `cargo nextest run -p oku-slides-i18n` (le test de complétude des locales échoue si une langue manque une clé) → PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-render crates/oku-slides-i18n
```
```bash
git commit -m "feat(render) : erreurs bornées du rendu et erreurs sérialisables pour le processus de rendu (t-327)"
```

---

### Task 2 : sonde brute, registre depuis la sonde, polices en octets (render)

**Files:**
- Modify: `crates/oku-slides-render/src/registry.rs`, `crates/oku-slides-render/src/registry_tests.rs`
- Modify: `crates/oku-slides-render/src/fonts.rs` (+ `fonts_tests.rs` côte à côte, à créer si absent)
- Modify: `crates/oku-slides-render/src/lib.rs` (réexports)

**Interfaces:**
- Produces :
  - `pub fn probe_registry(fonts: &Fonts, pack: Option<&PackWidgets>) -> Result<serde_json::Value, RegistryError>` — compile la sonde, rend son JSON.
  - `pub fn registry_from_probe(json: serde_json::Value) -> Result<WidgetRegistry, RegistryError>` — pur, sans Typst.
  - `load_registry` = `probe_registry` puis `registry_from_probe` (inchangé pour ses appelants).
  - `pub fn base_registry(fonts: &Fonts) -> Result<Arc<WidgetRegistry>, RegistryError>` (devient public).
  - `PackWidgets::fingerprint(&self) -> [u8; 32]` (sha-256 de `id`, `0`, `source` — celui de `Registries` aujourd'hui) et `PackWidgets::source(&self) -> &str` public.
  - `Registries::probe(&mut self, pack: &PackWidgets) -> Result<Arc<serde_json::Value>, RegistryError>` — JSON mémoïsé par empreinte, partagé avec `get`.
  - `Fonts::files(&self) -> impl Iterator<Item = &[u8]>` — fichiers fournis (hors polices intégrées à Typst), dans l'ordre d'ajout.

- [ ] **Step 1 : tests en échec** — dans `registry_tests.rs` :

```rust
#[test]
fn registry_from_probe_of_base_probe_equals_load_registry() {
    let json = probe_registry(&Fonts::default(), None).unwrap();
    let from = registry_from_probe(json).unwrap();
    assert_that!(from).is_equal_to(load_registry(&Fonts::default(), None).unwrap());
}

#[test]
fn pack_fingerprint_changes_with_source() {
    let a = PackWidgets::new("p", "#let registry = ()").unwrap();
    let b = PackWidgets::new("p", "#let registry = () ").unwrap();
    assert_that!(a.fingerprint()).is_not_equal_to(b.fingerprint());
}

#[test]
fn registries_probe_is_memoized_by_fingerprint() {
    let mut r = Registries::new(Fonts::default());
    let pack = PackWidgets::new("p", "#let registry = ()").unwrap();
    let first = r.probe(&pack).unwrap();
    let second = r.probe(&pack).unwrap();
    assert_that!(Arc::ptr_eq(&first, &second)).is_true();
}
```

Dans `fonts_tests.rs` :

```rust
use speculoos::prelude::*;
use super::*;

#[test]
fn fonts_files_lists_added_files_in_order() {
    let f = Fonts::new(vec![vec![1u8], vec![2u8]]).with(vec![vec![3u8]]);
    let files: Vec<Vec<u8>> = f.files().map(<[u8]>::to_vec).collect();
    assert_that!(files).is_equal_to(vec![vec![1u8], vec![2u8], vec![3u8]]);
}

#[test]
fn fonts_default_has_no_files() {
    assert_that!(Fonts::default().files().count()).is_equal_to(0);
}
```

(Des octets qui ne sont pas des polices sont gardés comme fichiers : `parse` les ignore comme polices, mais `files` les renvoie — le travailleur reçoit exactement ce que l'hôte a reçu.)

- [ ] **Step 2 : lancer** — `cargo nextest run -p oku-slides-render registry fonts` → FAIL.

- [ ] **Step 3 : implémenter**

`fonts.rs` : `Inner` gagne `files: Vec<typst::foundations::Bytes>`. `new(data)` : `let files: Vec<Bytes> = data.into_iter().map(Bytes::new).collect();` puis polices = intégrées + `files.iter().flat_map(|b| Font::iter(b.clone()))`. `with(data)` : `files` = anciens + nouveaux. `from_fonts` prend `(fonts, files)`. `files()` : `self.0.files.iter().map(|b| b.as_slice())`. Déclarer `#[cfg(test)] #[path = "fonts_tests.rs"] mod fonts_tests;`.

`registry.rs` :

```rust
impl PackWidgets {
    /// Empreinte du pack (identifiant et source) : clé des caches de registre.
    pub fn fingerprint(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(self.id.as_bytes());
        h.update([0]);
        h.update(self.source.as_bytes());
        h.finalize().into()
    }

    pub fn source(&self) -> &str { &self.source }
}

/// Compile la sonde et rend son JSON brut (schémas de la base et du pack).
pub fn probe_registry(fonts: &Fonts, pack: Option<&PackWidgets>) -> Result<Json, RegistryError> {
    // corps actuel de load_registry jusqu'à `probe_json(...)`, qui est rendu tel quel
}

/// Le registre (base surchargée par le pack) décrit par le JSON d'une sonde.
pub fn registry_from_probe(json: Json) -> Result<WidgetRegistry, RegistryError> {
    Ok(WidgetRegistry::from_probe(json).map_err(Box::new).context(SchemaSnafu)?)
}

pub fn load_registry(fonts: &Fonts, pack: Option<&PackWidgets>) -> Result<WidgetRegistry, RegistryError> {
    registry_from_probe(probe_registry(fonts, pack)?)
}
```

`base_registry` : `pub fn` (doc inchangée). `Registries` : `cache: HashMap<[u8; 32], (Arc<Json>, Arc<WidgetRegistry>)>` ; `probe(pack)` compile au premier appel (`probe_registry`), construit aussi le registre (`registry_from_probe` sur un clone) et range les deux ; `get(Some(pack))` passe par la même entrée ; `get(None)` → `base_registry`. `lib.rs` réexporte `probe_registry`, `registry_from_probe`, `base_registry`.

- [ ] **Step 4 : lancer** — `cargo nextest run -p oku-slides-render` → PASS (golden compris).

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-render
```
```bash
git commit -m "feat(render) : sonde du registre en JSON brut, registre depuis la sonde, fichiers des polices (t-327)"
```

---

### Task 3 : réglages `[render]` et lancement du rendu (platform)

**Files:**
- Modify: `crates/oku-slides-platform/src/config.default.toml`, `crates/oku-slides-platform/src/config.rs`, `crates/oku-slides-platform/src/config_tests.rs`
- Modify: `crates/oku-slides-platform/src/lib.rs` (+ test dans `lib_tests.rs` s'il existe, sinon dans `config_tests.rs`)

**Interfaces:**
- Produces :
  - `pub struct RenderSettings { pub timeout_ms: u64, pub memory_mib: u64, pub max_frame_mib: u64 }` avec `timeout(&self) -> Duration`, `memory_bytes(&self) -> u64`, `max_frame_bytes(&self) -> usize` ; champ `Settings::render`.
  - `pub enum RenderLaunch { InProcess, Process { program: PathBuf } }` (`Debug, Clone, PartialEq, Eq`) ; champ `Platform::render_launch` ; `Platform::with_render_launch(self, launch) -> Self`.
  - `PlatformError::CurrentExe { source: std::io::Error, backtrace }`.

- [ ] **Step 1 : tests en échec** — `config_tests.rs` :

```rust
#[test]
fn settings_default_render_limits_match_config_defaults() {
    let s = Settings::defaults().unwrap();
    assert_that!((s.render.timeout_ms, s.render.memory_mib, s.render.max_frame_mib)).is_equal_to((10_000, 2048, 512));
}

#[test]
fn render_settings_memory_bytes_converts_mib() {
    let r = RenderSettings { timeout_ms: 1, memory_mib: 3, max_frame_mib: 1 };
    assert_that!(r.memory_bytes()).is_equal_to(3 * 1024 * 1024);
}

#[test]
fn render_settings_timeout_is_in_milliseconds() {
    let r = RenderSettings { timeout_ms: 1500, memory_mib: 1, max_frame_mib: 1 };
    assert_that!(r.timeout()).is_equal_to(std::time::Duration::from_millis(1500));
}
```

Pour `Platform` (dans `config_tests.rs`, qui voit déjà `Settings`) :

```rust
#[test]
fn platform_with_settings_renders_in_process() {
    let p = crate::Platform::with_settings(Settings::defaults().unwrap()).unwrap();
    assert_that!(p.render_launch).is_equal_to(crate::RenderLaunch::InProcess);
}

#[test]
fn platform_with_render_launch_replaces_the_launch() {
    let program = std::path::PathBuf::from("/bin/x");
    let p = crate::Platform::with_settings(Settings::defaults().unwrap())
        .unwrap()
        .with_render_launch(crate::RenderLaunch::Process { program: program.clone() });
    assert_that!(p.render_launch).is_equal_to(crate::RenderLaunch::Process { program });
}
```

- [ ] **Step 2 : lancer** — `cargo nextest run -p oku-slides-platform` → FAIL.

- [ ] **Step 3 : implémenter** — dans `config.default.toml`, après `[sources]` :

```toml
[render]
# Durée maximale d'une compilation ou d'un export Typst (ms) ; au-delà, le
# processus de rendu est relancé et le deck se rend avec les widgets de base.
timeout_ms = 10000
# Mémoire maximale du processus de rendu (Mio).
memory_mib = 2048
# Taille maximale d'un message échangé avec le processus de rendu (Mio).
max_frame_mib = 512
```

`config.rs` : `RenderSettings` (`#[derive(Debug, Clone, PartialEq, Deserialize)] #[non_exhaustive]`, doc en tête « Bornes du processus de rendu. »), méthodes de conversion (`saturating_mul(1024 * 1024)`), champ `pub render: RenderSettings` dans `Settings`.

`lib.rs` :

```rust
/// Où tourne le rendu Typst.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderLaunch {
    /// Sur un fil du processus : aucune borne appliquée (tests, outils).
    InProcess,
    /// Dans un sous-processus `program --render-worker`, borné par `[render]`.
    Process { program: PathBuf },
}
```

`Platform` gagne `pub render_launch: RenderLaunch`. `with_settings` pose `InProcess`. `load()` :

```rust
pub fn load() -> Result<Self, PlatformError> {
    let program = std::env::current_exe().context(CurrentExeSnafu)?;
    Ok(Self::with_settings(Settings::load().context(SettingsSnafu)?)?.with_render_launch(RenderLaunch::Process { program }))
}

/// Cette plateforme, avec le rendu lancé par `launch`.
pub fn with_render_launch(mut self, launch: RenderLaunch) -> Self {
    self.render_launch = launch;
    self
}
```

- [ ] **Step 4 : lancer** — `cargo nextest run -p oku-slides-platform` → PASS ; `cargo check --workspace` → OK.

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-platform
```
```bash
git commit -m "feat(platform) : réglages [render] et lancement du processus de rendu (t-327)"
```

---

### Task 4 : crate `oku-slides-render-worker`, trames

**Files:**
- Create: `crates/oku-slides-render-worker/Cargo.toml`, `src/lib.rs`, `src/error.rs`, `src/frame.rs`, `src/frame_tests.rs`, `ARCHITECTURE.md`
- Modify: `Cargo.toml` (workspace : `oku-slides-render-worker = { path = "crates/oku-slides-render-worker" }`, `sysinfo = { version = "0.37", default-features = false, features = ["system"] }`)

**Interfaces:**
- Produces :
  - `pub(crate) struct Frame { pub header: Vec<u8>, pub payload: Vec<u8> }`
  - `pub(crate) fn write_frame(w: &mut impl Write, header: &impl Serialize, payload: &[u8]) -> Result<(), FrameError>`
  - `pub(crate) fn read_frame(r: &mut impl Read, max: usize) -> Result<Option<Frame>, FrameError>` — `None` : fin propre, entre deux trames.
  - `pub(crate) enum FrameError` (Snafu) : `Io`, `Truncated`, `TooLarge { len, max }`, `Json`.

- [ ] **Step 1 : manifeste**

```toml
[package]
name = "oku-slides-render-worker"
description = "Rendu Typst dans un processus de travail, borné en temps et en mémoire."
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
publish.workspace = true

[[bin]]
# Réservé aux tests du crate : jamais empaqueté (les binaires se relancent eux-mêmes).
name = "oku-slides-render-worker"
test = false
doc = false

[dependencies]
oku-slides-core.workspace = true
oku-slides-i18n.workspace = true
oku-slides-platform.workspace = true
oku-slides-render.workspace = true
serde.workspace = true
serde_json.workspace = true
sha2.workspace = true
hex.workspace = true
snafu.workspace = true
sysinfo.workspace = true
tracing.workspace = true

[dev-dependencies]
speculoos.workspace = true
rstest.workspace = true

[lints]
workspace = true
```

`src/bin/oku-slides-render-worker.rs` (complété en Task 5) :

```rust
//! Binaire de test : le travailleur seul (`--render-worker`).

fn main() {
    oku_slides_render_worker::exit_if_worker();
    eprintln!("usage: oku-slides-render-worker --render-worker --max-frame-bytes <n>");
    std::process::exit(2);
}
```

(Tant que `exit_if_worker` n'existe pas — Task 5 — mettre ce fichier en place à la Task 5 ; ici seulement le manifeste, `lib.rs`, `error.rs`, `frame.rs`.)

- [ ] **Step 2 : tests en échec** — `frame_tests.rs` :

```rust
use speculoos::prelude::*;
use super::*;

#[test]
fn frame_round_trip_keeps_header_and_payload() {
    let mut buf = Vec::new();
    write_frame(&mut buf, &serde_json::json!({"op": "x"}), b"abc").unwrap();
    let f = read_frame(&mut buf.as_slice(), 1024).unwrap().unwrap();
    assert_that!(serde_json::from_slice::<serde_json::Value>(&f.header).unwrap()).is_equal_to(serde_json::json!({"op": "x"}));
    assert_that!(f.payload).is_equal_to(b"abc".to_vec());
}

#[test]
fn read_frame_at_clean_end_returns_none() {
    assert_that!(read_frame(&mut [].as_slice(), 1024).unwrap()).is_none();
}

#[test]
fn read_frame_truncated_returns_truncated() {
    let mut buf = Vec::new();
    write_frame(&mut buf, &serde_json::json!({}), b"abcdef").unwrap();
    buf.truncate(buf.len() - 2);
    assert_that!(matches!(read_frame(&mut buf.as_slice(), 1024), Err(FrameError::Truncated { .. }))).is_true();
}

#[test]
fn read_frame_over_max_returns_too_large() {
    let mut buf = Vec::new();
    write_frame(&mut buf, &serde_json::json!({}), &[0u8; 64]).unwrap();
    assert_that!(matches!(read_frame(&mut buf.as_slice(), 16), Err(FrameError::TooLarge { .. }))).is_true();
}
```

- [ ] **Step 3 : lancer** — `cargo nextest run -p oku-slides-render-worker` → FAIL.

- [ ] **Step 4 : implémenter** — `frame.rs` :

```rust
//! Trames du tube : longueur de l'en-tête (u32 big-endian), en-tête JSON,
//! longueur de la charge (u32 big-endian), charge brute.

use std::io::{Read, Write};

use serde::Serialize;
use snafu::{ResultExt, ensure};

use crate::error::{FrameError, IoSnafu, JsonSnafu, TooLargeSnafu, TruncatedSnafu};

#[derive(Debug)]
pub(crate) struct Frame {
    pub(crate) header: Vec<u8>,
    pub(crate) payload: Vec<u8>,
}

pub(crate) fn write_frame(w: &mut impl Write, header: &impl Serialize, payload: &[u8]) -> Result<(), FrameError> {
    let header = serde_json::to_vec(header).context(JsonSnafu)?;
    for part in [&header[..], payload] {
        let len = u32::try_from(part.len()).map_err(|_| TooLargeSnafu { len: part.len(), max: u32::MAX as usize }.build())?;
        w.write_all(&len.to_be_bytes()).context(IoSnafu)?;
        w.write_all(part).context(IoSnafu)?;
    }
    w.flush().context(IoSnafu)
}

pub(crate) fn read_frame(r: &mut impl Read, max: usize) -> Result<Option<Frame>, FrameError> {
    let mut len = [0u8; 4];
    // Fin propre : aucun octet avant la trame.
    match r.read(&mut len[..1]).context(IoSnafu)? {
        0 => return Ok(None),
        _ => read_exact(r, &mut len[1..])?,
    }
    let header = read_part(r, u32::from_be_bytes(len) as usize, max)?;
    read_exact(r, &mut len)?;
    let payload = read_part(r, u32::from_be_bytes(len) as usize, max)?;
    Ok(Some(Frame { header, payload }))
}

fn read_part(r: &mut impl Read, len: usize, max: usize) -> Result<Vec<u8>, FrameError> {
    ensure!(len <= max, TooLargeSnafu { len, max });
    let mut buf = vec![0u8; len];
    read_exact(r, &mut buf)?;
    Ok(buf)
}

fn read_exact(r: &mut impl Read, buf: &mut [u8]) -> Result<(), FrameError> {
    r.read_exact(buf).map_err(|e| match e.kind() {
        std::io::ErrorKind::UnexpectedEof => TruncatedSnafu.build(),
        _ => FrameError::from(IoSnafu.into_error(e)),
    })
}
```

`error.rs` :

```rust
//! Erreurs des trames du tube.

use snafu::{Backtrace, Snafu};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum FrameError {
    #[snafu(display("pipe i/o failed"))]
    Io { source: std::io::Error, backtrace: Backtrace },
    #[snafu(display("truncated frame"))]
    Truncated { backtrace: Backtrace },
    #[snafu(display("frame of {len} bytes exceeds {max}"))]
    TooLarge { len: usize, max: usize, backtrace: Backtrace },
    #[snafu(display("frame header is not valid json"))]
    Json { source: serde_json::Error, backtrace: Backtrace },
}
```

(`read_exact` ci-dessus : écrire la branche `Io` avec `Err(e).context(IoSnafu)` si `into_error` ne compile pas avec la version de snafu — même sens.)

`lib.rs` : doc de module (« Rendu Typst dans un processus de travail… » + table des matières des types publics, remplie aux tâches suivantes), `mod error; mod frame;`.

`ARCHITECTURE.md` (moins de 50 lignes) : ce que fait le crate, types clés (à compléter en Task 7), dépendances.

- [ ] **Step 5 : lancer** — `cargo nextest run -p oku-slides-render-worker` → PASS ; `cargo deny check` → OK pour `sysinfo` (licence MIT).

- [ ] **Step 6 : commit**

```bash
git add Cargo.toml Cargo.lock crates/oku-slides-render-worker
```
```bash
git commit -m "feat(render-worker) : crate du processus de rendu, trames du tube (t-327)"
```

---

### Task 5 : protocole et boucle du travailleur (`serve`, emplacements, générations)

**Files:**
- Create: `crates/oku-slides-render-worker/src/protocol.rs`, `src/serve.rs`, `src/serve_tests.rs`, `src/bin/oku-slides-render-worker.rs`
- Modify: `crates/oku-slides-render-worker/src/lib.rs`

**Interfaces:**
- Consumes : `read_frame`/`write_frame` (Task 4), `WireError`, `probe_registry`, `Registries`, `Fonts::files`, `PackWidgets` (Tasks 1-2).
- Produces :

```rust
pub(crate) type SlotId = u32;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct WirePack { pub(crate) id: String, pub(crate) source: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case")]
pub(crate) enum Request {
    SetLanguage { locale: String },
    PutFont { id: String },   // charge : octets
    PutAsset { id: String },  // charge : octets
    Probe { pack: WirePack },
    OpenSlot { slot: SlotId, fonts: Vec<String> },
    CloseSlot { slot: SlotId },
    SetWidgets { slot: SlotId, pack: Option<WirePack> },
    Render { slot: SlotId, deck: oku_slides_core::Deck, assets: Vec<String> },
    Svg { slot: SlotId, generation: u64, page: usize },
    Png { slot: SlotId, generation: u64, page: usize, pixel_per_pt: f32 },
    Pdf { slot: SlotId, generation: u64 },
    Release { slot: SlotId, generations: Vec<u64> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CompiledHeader {
    pub(crate) generation: u64,
    pub(crate) page_count: usize,
    /// `u128` en hexadécimal (32 chiffres) : JSON ne porte pas d'entier de 128 bits.
    pub(crate) page_hashes: Vec<String>,
    pub(crate) layout: oku_slides_render::Layout,
    pub(crate) tokens: Option<serde_json::Value>,
    pub(crate) warnings: Vec<oku_slides_render::Diagnostic>,
    pub(crate) chart_palette: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "reply", rename_all = "kebab-case")]
pub(crate) enum Response {
    Ok,
    Probe { json: serde_json::Value },
    Compiled(CompiledHeader),
    Bytes,  // charge : octets
    Error { error: oku_slides_render::WireError },
}

/// Sert les requêtes de `input` jusqu'à sa fin.
pub fn serve(input: impl Read, output: impl Write, max_frame: usize) -> Result<(), ServeError>;

/// Si le processus a été lancé avec `--render-worker`, sert stdin/stdout et termine le processus.
pub fn exit_if_worker();
```

Règles du travailleur (à documenter en tête de `serve.rs`) :
- `fonts`, `assets` : par empreinte ; `PutFont`/`PutAsset` rangent la charge.
- `OpenSlot` : `Fonts::new(ids → octets)` (un id inconnu → `Error Setup`), `Renderer::new`, puis la langue courante si posée.
- `SetWidgets { pack }` : `Registries::get(pack)` puis `Renderer::set_widgets`.
- `Render` : `renderer.render(&deck, &assets)` ; nouvelle génération (compteur **global**, croissant) ; `Compiled` rangé dans `slot.generations` ; après le rendu, les assets hors de l'union des `assets` du dernier `Render` de chaque emplacement sont retirés (l'hôte applique la même règle, Task 6).
- `Svg`/`Png`/`Pdf` d'une génération absente → `Error { WireError::Stale }`.
- `Release` retire les générations nommées. `CloseSlot` retire l'emplacement.
- Une requête qui échoue répond `Error` et la boucle continue ; une trame illisible arrête `serve` (`ServeError::Frame`).

- [ ] **Step 1 : tests en échec** — `serve_tests.rs`, en mémoire (le travailleur tourne sur un fil, tubes `std::io::pipe()`) :

```rust
use std::io::{PipeReader, PipeWriter};
use std::thread::JoinHandle;

use speculoos::prelude::*;

use super::*;
use crate::frame::{read_frame, write_frame};
use crate::protocol::{Request, Response};

const MAX: usize = 64 * 1024 * 1024;

struct Harness { tx: PipeWriter, rx: PipeReader, _worker: JoinHandle<()> }

impl Harness {
    fn new() -> Self {
        let (req_r, req_w) = std::io::pipe().unwrap();
        let (resp_r, resp_w) = std::io::pipe().unwrap();
        let worker = std::thread::spawn(move || { let _ = serve(req_r, resp_w, MAX); });
        Self { tx: req_w, rx: resp_r, _worker: worker }
    }

    fn call(&mut self, req: &Request, payload: &[u8]) -> (Response, Vec<u8>) {
        write_frame(&mut self.tx, req, payload).unwrap();
        let f = read_frame(&mut self.rx, MAX).unwrap().unwrap();
        (serde_json::from_slice(&f.header).unwrap(), f.payload)
    }

    fn roadmap(&mut self) -> u64 {
        assert!(matches!(self.call(&Request::OpenSlot { slot: 1, fonts: vec![] }, &[]).0, Response::Ok));
        match self.call(&Request::Render { slot: 1, deck: fixture_deck(), assets: vec![] }, &[]).0 {
            Response::Compiled(h) => h.generation,
            other => panic!("{other:?}"),
        }
    }
}

fn fixture_deck() -> oku_slides_core::Deck {
    let json = include_str!("../../../fixtures/roadmap.json");
    oku_slides_core::Deck::from_json(json).unwrap()
}

#[test]
fn serve_render_then_svg_of_same_generation_returns_svg() {
    let mut h = Harness::new();
    let generation = h.roadmap();
    let (resp, svg) = h.call(&Request::Svg { slot: 1, generation, page: 0 }, &[]);
    assert_that!(matches!(resp, Response::Bytes)).is_true();
    assert_that!(String::from_utf8(svg).unwrap()).starts_with("<svg");
}

#[test]
fn serve_svg_of_released_generation_returns_stale() {
    let mut h = Harness::new();
    let generation = h.roadmap();
    h.call(&Request::Release { slot: 1, generations: vec![generation] }, &[]);
    let (resp, _) = h.call(&Request::Svg { slot: 1, generation, page: 0 }, &[]);
    assert_that!(matches!(resp, Response::Error { error: oku_slides_render::WireError::Stale })).is_true();
}

#[test]
fn serve_keeps_an_older_generation_until_released() {
    let mut h = Harness::new();
    let first = h.roadmap();
    h.call(&Request::Render { slot: 1, deck: fixture_deck(), assets: vec![] }, &[]);
    let (resp, _) = h.call(&Request::Svg { slot: 1, generation: first, page: 0 }, &[]);
    assert_that!(matches!(resp, Response::Bytes)).is_true();
}

#[test]
fn serve_probe_of_broken_pack_returns_registry_compile_error() {
    let mut h = Harness::new();
    let pack = WirePack { id: "casse".into(), source: "#let registry = (".into() };
    let (resp, _) = h.call(&Request::Probe { pack }, &[]);
    assert_that!(matches!(resp, Response::Error { error: oku_slides_render::WireError::RegistryCompile { .. } })).is_true();
}

#[test]
fn serve_open_slot_with_unknown_font_returns_error() {
    let mut h = Harness::new();
    let (resp, _) = h.call(&Request::OpenSlot { slot: 1, fonts: vec!["inconnue".into()] }, &[]);
    assert_that!(matches!(resp, Response::Error { .. })).is_true();
}
```

(Vérifier le chemin exact de la fixture : `ls fixtures/` — adapter `include_str!` au fichier du deck `roadmap` en v2.)

- [ ] **Step 2 : lancer** — `cargo nextest run -p oku-slides-render-worker serve` → FAIL.

- [ ] **Step 3 : implémenter** — `serve.rs` :

```rust
//! Boucle du travailleur : un `Renderer` par emplacement, polices et assets
//! par empreinte, un `Compiled` par génération jusqu'à sa libération.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{Read, Write};

use oku_slides_i18n::Localizer;
use oku_slides_render::{Compiled, Fonts, PackWidgets, Registries, RenderError, Renderer, WireError, probe_registry};
use snafu::{ResultExt, Snafu};

use crate::frame::{read_frame, write_frame};
use crate::protocol::{CompiledHeader, Request, Response, SlotId, WirePack};

/// La boucle du travailleur s'est arrêtée sur un tube illisible.
#[derive(Debug, Snafu)]
pub struct ServeError(crate::error::FrameError);

struct Slot { renderer: Renderer, generations: BTreeMap<u64, Compiled>, last_assets: Vec<String> }

#[derive(Default)]
struct Worker {
    fonts: HashMap<String, Vec<u8>>,
    assets: BTreeMap<String, Vec<u8>>,
    slots: BTreeMap<SlotId, Slot>,
    registries: Option<Registries>,
    localizer: Option<Localizer>,
    next_generation: u64,
}

pub fn serve(mut input: impl Read, mut output: impl Write, max_frame: usize) -> Result<(), ServeError> {
    let mut w = Worker::default();
    while let Some(frame) = read_frame(&mut input, max_frame).map_err(ServeError)? {
        let (reply, payload) = match serde_json::from_slice::<Request>(&frame.header) {
            Ok(req) => w.handle(req, frame.payload),
            Err(e) => (Response::Error { error: WireError::from_render(&RenderError::frame(e.to_string())) }, Vec::new()),
        };
        write_frame(&mut output, &reply, &payload).map_err(ServeError)?;
    }
    Ok(())
}

impl Worker {
    fn handle(&mut self, req: Request, payload: Vec<u8>) -> (Response, Vec<u8>) {
        let result = match req {
            Request::SetLanguage { locale } => self.set_language(&locale),
            Request::PutFont { id } => { self.fonts.insert(id, payload); Ok((Response::Ok, Vec::new())) }
            Request::PutAsset { id } => { self.assets.insert(id, payload); Ok((Response::Ok, Vec::new())) }
            Request::Probe { pack } => return self.probe(pack),
            Request::OpenSlot { slot, fonts } => self.open_slot(slot, &fonts),
            Request::CloseSlot { slot } => { self.slots.remove(&slot); self.evict_assets(); Ok((Response::Ok, Vec::new())) }
            Request::SetWidgets { slot, pack } => self.set_widgets(slot, pack),
            Request::Render { slot, deck, assets } => self.render(slot, &deck, assets),
            Request::Svg { slot, generation, page } => self.with(slot, generation, |c| c.svg(page).map(String::into_bytes)),
            Request::Png { slot, generation, page, pixel_per_pt } => self.with(slot, generation, |c| c.png(page, pixel_per_pt)),
            Request::Pdf { slot, generation } => self.with(slot, generation, Compiled::pdf),
            Request::Release { slot, generations } => {
                if let Some(s) = self.slots.get_mut(&slot) { for g in generations { s.generations.remove(&g); } }
                Ok((Response::Ok, Vec::new()))
            }
        };
        result.unwrap_or_else(|e| (Response::Error { error: WireError::from_render(&e) }, Vec::new()))
    }

    fn fonts_of(&self, ids: &[String]) -> Result<Fonts, RenderError> {
        let files = ids.iter().map(|id| self.fonts.get(id).cloned().ok_or_else(|| RenderError::frame(format!("unknown font {id}"))));
        Ok(Fonts::new(files.collect::<Result<Vec<_>, _>>()?))
    }

    fn registries(&mut self) -> &mut Registries {
        // Les schémas ne dépendent pas des polices : celles de Typst suffisent.
        self.registries.get_or_insert_with(|| Registries::new(Fonts::default()))
    }

    fn probe(&mut self, pack: WirePack) -> (Response, Vec<u8>) {
        let reply = PackWidgets::new(pack.id, pack.source)
            .and_then(|p| self.registries().probe(&p))
            .map(|json| Response::Probe { json: (*json).clone() })
            .unwrap_or_else(|e| Response::Error { error: WireError::from_registry(&e) });
        (reply, Vec::new())
    }

    fn set_language(&mut self, locale: &str) -> Result<(Response, Vec<u8>), RenderError> {
        let l = Localizer::new(Some(locale));
        for s in self.slots.values_mut() { s.renderer.set_language(&l)?; }
        self.localizer = Some(l);
        Ok((Response::Ok, Vec::new()))
    }

    fn open_slot(&mut self, slot: SlotId, fonts: &[String]) -> Result<(Response, Vec<u8>), RenderError> {
        let mut renderer = Renderer::new(self.fonts_of(fonts)?)?;
        if let Some(l) = &self.localizer { renderer.set_language(l)?; }
        self.slots.insert(slot, Slot { renderer, generations: BTreeMap::new(), last_assets: Vec::new() });
        Ok((Response::Ok, Vec::new()))
    }

    fn set_widgets(&mut self, slot: SlotId, pack: Option<WirePack>) -> Result<(Response, Vec<u8>), RenderError> {
        let pack = pack.map(|p| PackWidgets::new(p.id, p.source)).transpose().map_err(|e| WireError::from_registry(&e).into_render())?;
        let registry = self.registries().get(pack.as_ref()).map_err(|e| WireError::from_registry(&e).into_render())?;
        let s = self.slots.get_mut(&slot).ok_or_else(|| RenderError::frame(format!("unknown slot {slot}")))?;
        s.renderer.set_widgets(pack.as_ref(), registry)?;
        Ok((Response::Ok, Vec::new()))
    }

    fn render(&mut self, slot: SlotId, deck: &oku_slides_core::Deck, assets: Vec<String>) -> Result<(Response, Vec<u8>), RenderError> {
        let s = self.slots.get_mut(&slot).ok_or_else(|| RenderError::frame(format!("unknown slot {slot}")))?;
        let compiled = s.renderer.render(deck, &self.assets)?;
        self.next_generation += 1;
        let generation = self.next_generation;
        let header = CompiledHeader {
            generation,
            page_count: compiled.page_count(),
            page_hashes: (0..compiled.page_count()).map(|i| format!("{:032x}", compiled.page_hash(i).unwrap_or_default())).collect(),
            layout: compiled.layout().clone(),
            tokens: compiled.layout().tokens.clone(),
            warnings: compiled.warnings.clone(),
            chart_palette: s.renderer.chart_palette().to_vec(),
        };
        s.generations.insert(generation, compiled);
        s.last_assets = assets;
        self.evict_assets();
        Ok((Response::Compiled(header), Vec::new()))
    }

    /// Ne garde que les assets du dernier rendu d'un emplacement (règle partagée avec l'hôte).
    fn evict_assets(&mut self) {
        let live: BTreeSet<&String> = self.slots.values().flat_map(|s| &s.last_assets).collect();
        self.assets.retain(|id, _| live.contains(id));
    }

    fn with(&self, slot: SlotId, generation: u64, f: impl FnOnce(&Compiled) -> Result<Vec<u8>, RenderError>)
        -> Result<(Response, Vec<u8>), RenderError> {
        let c = self.slots.get(&slot).and_then(|s| s.generations.get(&generation)).ok_or_else(RenderError::stale)?;
        Ok((Response::Bytes, f(c)?))
    }
}
```

`exit_if_worker` (dans `serve.rs`) :

```rust
/// Si ce processus a été lancé en travailleur (`--render-worker
/// --max-frame-bytes <n>`), sert stdin/stdout puis termine le processus ;
/// sinon ne fait rien. À appeler en tête de `main`, avant toute initialisation.
pub fn exit_if_worker() {
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    if args.first().map(|a| a == "--render-worker") != Some(true) {
        return;
    }
    oku_slides_platform::telemetry::init(); // journaux sur stderr : stdout est le tube
    let max = args.iter().position(|a| a == "--max-frame-bytes")
        .and_then(|i| args.get(i + 1)?.to_str()?.parse::<usize>().ok())
        .unwrap_or(512 * 1024 * 1024);
    let code = match serve(std::io::stdin().lock(), std::io::stdout().lock(), max) {
        Ok(()) => 0,
        Err(e) => { tracing::error!(error = %e, "processus de rendu"); 1 }
    };
    std::process::exit(code);
}
```

Créer `src/bin/oku-slides-render-worker.rs` (code en Task 4, Step 1). `lib.rs` : `mod protocol; mod serve; pub use serve::{ServeError, exit_if_worker, serve};`.

- [ ] **Step 4 : lancer** — `cargo nextest run -p oku-slides-render-worker` → PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-render-worker crates/oku-slides-render
```
```bash
git commit -m "feat(render-worker) : protocole et boucle du travailleur, emplacements et générations (t-327)"
```

---

### Task 6 : lien hôte — transports, chien de garde, relance et rejeu

**Files:**
- Create: `crates/oku-slides-render-worker/src/rss.rs`, `src/rss_tests.rs`, `src/link.rs`, `src/link_tests.rs`

**Interfaces:**
- Consumes : `Request`, `Response`, trames, `RenderLaunch`, `RenderSettings` (Task 3).
- Produces :

```rust
pub(crate) fn resident_bytes(sys: &mut sysinfo::System, pid: u32) -> Option<u64>;

pub(crate) struct Link { /* … */ }
impl Link {
    pub(crate) fn new(launch: RenderLaunch, settings: RenderSettings) -> Self;
    /// Envoie une requête et attend sa réponse, dans les bornes. Relance le
    /// travailleur au besoin (rejeu de `state`). Un échec borné tue le travailleur.
    pub(crate) fn call(&mut self, req: &Request, payload: &[u8]) -> Result<(Response, Vec<u8>), RenderError>;
    pub(crate) state: ReplayState,   // langue, polices, emplacements (polices + pack)
    pub(crate) sent_fonts: HashSet<String>, pub(crate) sent_assets: HashSet<String>,
    pub(crate) restarts: u64,
}
pub(crate) struct ReplayState {
    pub(crate) locale: Option<String>,
    pub(crate) fonts: HashMap<String, Arc<[u8]>>,
    pub(crate) slots: BTreeMap<SlotId, SlotState>,
}
pub(crate) struct SlotState { pub(crate) fonts: Vec<String>, pub(crate) pack: Option<WirePack>, pub(crate) last_assets: Vec<String> }
```

Fonctionnement de `call` :
1. Pas de connexion → `connect()` : `Process` : `Command::new(program).args(["--render-worker", "--max-frame-bytes", &max.to_string()]).stdin(piped).stdout(piped).stderr(inherit).spawn()` ; `InProcess` : deux `std::io::pipe()` et `serve` sur un fil nommé `oku-render-worker`. Un fil lecteur lit les trames de sortie et les pousse dans un `mpsc::channel::<Result<Frame, FrameError>>` ; sur fin ou erreur il pousse l'erreur et s'arrête. Puis **rejeu** (via `raw_call`, qui n'appelle pas `connect`) : `SetLanguage`, `PutFont` de chaque police de `state.fonts` citée par un emplacement, `OpenSlot` et `SetWidgets` de chaque emplacement, **sauf** un pack marqué fautif (voir Task 7 : `SetWidgets(None)` et l'emplacement est dit « écarté »). `sent_fonts` = polices envoyées ; `sent_assets` vidé ; `restarts += 1` (sauf au premier lancement).
2. `raw_call` : `write_frame` puis boucle : `rx.recv_timeout(min(100 ms, reste))` ; à chaque réveil, en `Process`, `resident_bytes` > `memory_bytes` → `kill` → `Err(out_of_memory(mib))` ; délai écoulé → `kill` (`Process`) ou abandon du fil (`InProcess`) → `Err(timeout(ms))` ; canal fermé ou trame en erreur → `Err(worker_lost(…))` (statut de sortie du fils s'il est connu).
3. Tout `Err` de `raw_call` : connexion abandonnée (`self.conn = None`, le `Drop` de `Conn` tue le fils : `child.kill()` puis `child.wait()`), l'erreur est rendue telle quelle.
4. Une réponse `Response::Error { error }` n'est pas un échec du lien : rendue comme `Ok((Response::Error{..}, …))` ; l'appelant la convertit.

`Drop for Link` : ferme le tube d'entrée (le travailleur voit la fin et sort), attend le fils jusqu'à 500 ms (`try_wait` toutes les 20 ms), puis `kill`.

- [ ] **Step 1 : tests en échec** — `rss_tests.rs` :

```rust
use speculoos::prelude::*;
use super::*;

#[test]
fn resident_bytes_of_this_process_is_positive() {
    let mut sys = sysinfo::System::new();
    assert_that!(resident_bytes(&mut sys, std::process::id()).unwrap_or(0)).is_greater_than(0);
}

#[test]
fn resident_bytes_of_missing_process_is_none() {
    let mut sys = sysinfo::System::new();
    assert_that!(resident_bytes(&mut sys, u32::MAX - 1)).is_none();
}
```

`link_tests.rs` (transport `InProcess`) :

```rust
use speculoos::prelude::*;
use oku_slides_platform::{RenderLaunch, RenderSettings};

use super::*;
use crate::protocol::{Request, Response};

fn settings(timeout_ms: u64) -> RenderSettings {
    RenderSettings { timeout_ms, memory_mib: 4096, max_frame_mib: 64 }
}

#[test]
fn link_in_process_answers_open_slot() {
    let mut link = Link::new(RenderLaunch::InProcess, settings(10_000));
    let (resp, _) = link.call(&Request::OpenSlot { slot: 1, fonts: vec![] }, &[]).unwrap();
    assert_that!(matches!(resp, Response::Ok)).is_true();
}

#[test]
fn link_in_process_probe_that_loops_times_out_and_restarts() {
    let mut link = Link::new(RenderLaunch::InProcess, settings(300));
    let pack = WirePack { id: "boucle".into(), source: crate::test_packs::LOOP_ON_IMPORT.into() };
    let err = link.call(&Request::Probe { pack }, &[]).unwrap_err();
    assert_that!(err.kind()).is_equal_to(oku_slides_render::RenderErrorKind::Timeout);
    // La requête suivante relance un travailleur neuf.
    link.call(&Request::OpenSlot { slot: 1, fonts: vec![] }, &[]).unwrap();
    assert_that!(link.restarts).is_equal_to(1);
}

#[test]
fn link_replays_open_slots_after_restart() {
    let mut link = Link::new(RenderLaunch::InProcess, settings(300));
    link.call(&Request::OpenSlot { slot: 7, fonts: vec![] }, &[]).unwrap();
    link.state.slots.insert(7, SlotState { fonts: vec![], pack: None, last_assets: vec![] });
    let pack = WirePack { id: "boucle".into(), source: crate::test_packs::LOOP_ON_IMPORT.into() };
    let _ = link.call(&Request::Probe { pack }, &[]);
    let (resp, _) = link.call(&Request::Render { slot: 7, deck: crate::test_packs::tiny_deck(), assets: vec![] }, &[]).unwrap();
    assert_that!(matches!(resp, Response::Compiled(_))).is_true();
}
```

Créer `src/test_packs.rs` (`#[cfg(test)] pub(crate) mod test_packs;` dans `lib.rs`, et aussi utilisé par les tests d'intégration via une copie `tests/common/packs.rs` à la Task 7) :

```rust
//! Packs de widgets hostiles et deck minimal, pour les tests.

/// Boucle au chargement du pack (sonde et `SetWidgets`), hors des gardes de Typst.
pub(crate) const LOOP_ON_IMPORT: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let _spin = { let n = 0; for _ in range(1000000000000) { n += 1 }; n }
#let registry = ()
"#;

/// Boucle au rendu du widget `boucle` seulement.
pub(crate) const LOOP_ON_RENDER: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let boucle = widget("boucle", params: (body: p.plain()),
  (args, ctx) => { let n = 0; for _ in range(1000000000000) { n += 1 }; [#n] })
#let registry = (boucle,)
"#;

/// Alloue sans fin au chargement.
pub(crate) const ALLOCATE: &str = r#"#let _big = range(400000000).map(i => str(i))
#let registry = ()
"#;

pub(crate) fn tiny_deck() -> oku_slides_core::Deck {
    oku_slides_core::Deck::from_json(r#"{"version":2,"params":{"title":"T"},"slides":[{"id":"s","kind":"title","params":{}}]}"#).unwrap()
}
```

- [ ] **Step 2 : lancer** — `cargo nextest run -p oku-slides-render-worker link rss` → FAIL.

- [ ] **Step 3 : implémenter** — `rss.rs` :

```rust
//! Mémoire résidente d'un processus (sysinfo, mêmes appels sur les trois OS).

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

pub(crate) fn resident_bytes(sys: &mut System, pid: u32) -> Option<u64> {
    let pid = Pid::from_u32(pid);
    sys.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, ProcessRefreshKind::nothing().with_memory());
    sys.process(pid).map(sysinfo::Process::memory)
}
```

`link.rs` : suivre le fonctionnement décrit plus haut. Squelette :

```rust
//! Lien avec un travailleur : transport, bornes, relance et rejeu.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::{Duration, Instant};

use oku_slides_platform::{RenderLaunch, RenderSettings};
use oku_slides_render::RenderError;

use crate::error::FrameError;
use crate::frame::{Frame, read_frame, write_frame};
use crate::protocol::{Request, Response, SlotId, WirePack};
use crate::rss::resident_bytes;

/// Intervalle du relevé de mémoire et du contrôle du délai.
const TICK: Duration = Duration::from_millis(100);
/// Attente du travailleur à la fermeture, avant `kill`.
const GRACE: Duration = Duration::from_millis(500);

struct Conn {
    input: Box<dyn Write + Send>,
    output: Receiver<Result<Frame, FrameError>>,
    child: Option<Child>,
}

impl Drop for Conn {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
```

La suite de `link.rs` :

```rust
#[derive(Debug, Clone, Default)]
pub(crate) struct SlotState {
    pub(crate) fonts: Vec<String>,
    pub(crate) pack: Option<WirePack>,
    pub(crate) last_assets: Vec<String>,
}

/// Ce qu'un travailleur neuf doit recevoir pour reprendre là où l'ancien était.
#[derive(Debug, Default)]
pub(crate) struct ReplayState {
    pub(crate) locale: Option<String>,
    pub(crate) fonts: HashMap<String, Arc<[u8]>>,
    pub(crate) slots: BTreeMap<SlotId, SlotState>,
}

pub(crate) struct Link {
    launch: RenderLaunch,
    settings: RenderSettings,
    conn: Option<Conn>,
    sys: sysinfo::System,
    pub(crate) state: ReplayState,
    pub(crate) sent_fonts: HashSet<String>,
    pub(crate) sent_assets: HashSet<String>,
    /// Empreintes (hex) des packs fautifs : rejoués avec la base.
    pub(crate) faulty_packs: HashSet<String>,
    pub(crate) restarts: u64,
    started: bool,
}

impl Link {
    pub(crate) fn new(launch: RenderLaunch, settings: RenderSettings) -> Self {
        Self {
            launch, settings, conn: None, sys: sysinfo::System::new(), state: ReplayState::default(),
            sent_fonts: HashSet::new(), sent_assets: HashSet::new(), faulty_packs: HashSet::new(),
            restarts: 0, started: false,
        }
    }

    pub(crate) fn call(&mut self, req: &Request, payload: &[u8]) -> Result<(Response, Vec<u8>), RenderError> {
        if self.conn.is_none() {
            self.connect()?;
        }
        self.raw_call(req, payload)
    }

    fn connect(&mut self) -> Result<(), RenderError> {
        let max = self.settings.max_frame_bytes();
        let (input, output, child): (Box<dyn Write + Send>, Box<dyn std::io::Read + Send>, Option<Child>) =
            match &self.launch {
                RenderLaunch::Process { program } => {
                    let mut child = Command::new(program)
                        .args(["--render-worker", "--max-frame-bytes", &max.to_string()])
                        .stdin(Stdio::piped())
                        .stdout(Stdio::piped())
                        .stderr(Stdio::inherit())
                        .spawn()
                        .map_err(|e| RenderError::worker_lost(format!("spawn {}: {e}", program.display())))?;
                    let stdin = child.stdin.take().ok_or_else(|| RenderError::worker_lost("no stdin"))?;
                    let stdout = child.stdout.take().ok_or_else(|| RenderError::worker_lost("no stdout"))?;
                    (Box::new(stdin), Box::new(stdout), Some(child))
                }
                RenderLaunch::InProcess => {
                    let (req_r, req_w) = std::io::pipe().map_err(|e| RenderError::worker_lost(e.to_string()))?;
                    let (resp_r, resp_w) = std::io::pipe().map_err(|e| RenderError::worker_lost(e.to_string()))?;
                    std::thread::Builder::new()
                        .name("oku-render-worker".into())
                        .spawn(move || { let _ = crate::serve(req_r, resp_w, max); })
                        .map_err(|e| RenderError::worker_lost(e.to_string()))?;
                    (Box::new(req_w), Box::new(resp_r), None)
                }
            };
        let (tx, rx) = channel();
        let mut output = output;
        std::thread::Builder::new()
            .name("oku-render-reader".into())
            .spawn(move || loop {
                match read_frame(&mut output, max) {
                    Ok(Some(frame)) => { if tx.send(Ok(frame)).is_err() { return; } }
                    Ok(None) => return,
                    Err(e) => { let _ = tx.send(Err(e)); return; }
                }
            })
            .map_err(|e| RenderError::worker_lost(e.to_string()))?;
        self.conn = Some(Conn { input, output: rx, child });
        if self.started { self.restarts += 1; }
        self.started = true;
        self.sent_fonts.clear();
        self.sent_assets.clear();
        self.replay()
    }

    /// Rejoue langue, polices et emplacements ; un pack fautif est rejoué par la base.
    fn replay(&mut self) -> Result<(), RenderError> {
        if let Some(locale) = self.state.locale.clone() {
            self.expect_ok(&Request::SetLanguage { locale }, &[])?;
        }
        let slots: Vec<(SlotId, SlotState)> = self.state.slots.iter().map(|(k, v)| (*k, v.clone())).collect();
        for (slot, st) in slots {
            for id in &st.fonts {
                if self.sent_fonts.insert(id.clone()) {
                    let bytes = self.state.fonts.get(id).cloned().unwrap_or_else(|| Arc::from(Vec::new()));
                    self.expect_ok(&Request::PutFont { id: id.clone() }, &bytes)?;
                }
            }
            self.expect_ok(&Request::OpenSlot { slot, fonts: st.fonts.clone() }, &[])?;
            let pack = st.pack.filter(|p| !self.faulty_packs.contains(&fingerprint_hex(&p.id, &p.source)));
            if pack.is_some() {
                self.expect_ok(&Request::SetWidgets { slot, pack }, &[])?;
            }
        }
        Ok(())
    }

    fn expect_ok(&mut self, req: &Request, payload: &[u8]) -> Result<(), RenderError> {
        match self.raw_call(req, payload)? {
            (Response::Ok, _) => Ok(()),
            (Response::Error { error }, _) => Err(error.into_render()),
            (other, _) => Err(RenderError::frame(format!("unexpected reply {other:?}"))),
        }
    }

    fn raw_call(&mut self, req: &Request, payload: &[u8]) -> Result<(Response, Vec<u8>), RenderError> {
        let result = self.exchange(req, payload);
        if result.is_err() {
            // Tout échec du lien le rend inutilisable : le suivant est neuf.
            self.conn = None;
        }
        result
    }

    fn exchange(&mut self, req: &Request, payload: &[u8]) -> Result<(Response, Vec<u8>), RenderError> {
        let (timeout, memory) = (self.settings.timeout(), self.settings.memory_bytes());
        let conn = self.conn.as_mut().ok_or_else(|| RenderError::worker_lost("not connected"))?;
        write_frame(&mut conn.input, req, payload).map_err(|e| RenderError::worker_lost(e.to_string()))?;
        let started = Instant::now();
        loop {
            let left = timeout.saturating_sub(started.elapsed());
            match conn.output.recv_timeout(left.min(TICK)) {
                Ok(Ok(frame)) => {
                    let resp: Response = serde_json::from_slice(&frame.header).map_err(|e| RenderError::frame(e.to_string()))?;
                    return Ok((resp, frame.payload));
                }
                Ok(Err(e)) => return Err(RenderError::frame(e.to_string())),
                Err(RecvTimeoutError::Disconnected) => {
                    let status = conn.child.as_mut().and_then(|c| c.try_wait().ok().flatten());
                    return Err(RenderError::worker_lost(status.map_or_else(|| "pipe closed".to_owned(), |s| s.to_string())));
                }
                Err(RecvTimeoutError::Timeout) => {
                    if let Some(child) = &conn.child
                        && resident_bytes(&mut self.sys, child.id()).is_some_and(|b| b > memory)
                    {
                        return Err(RenderError::out_of_memory(self.settings.memory_mib));
                    }
                    if started.elapsed() >= timeout {
                        return Err(RenderError::timeout(self.settings.timeout_ms));
                    }
                }
            }
        }
    }

    /// Tue le travailleur sans toucher à l'état (tests de relance).
    pub(crate) fn kill(&mut self) {
        if let Some(child) = self.conn.as_mut().and_then(|c| c.child.as_mut()) {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for Link {
    /// Fermeture douce : fin du tube d'entrée, le travailleur sort de lui-même ;
    /// au-delà de `GRACE`, `Conn::drop` le tue.
    fn drop(&mut self) {
        let Some(mut conn) = self.conn.take() else { return };
        conn.input = Box::new(std::io::sink());
        if let Some(child) = &mut conn.child {
            let deadline = Instant::now() + GRACE;
            while Instant::now() < deadline {
                if matches!(child.try_wait(), Ok(Some(_))) { break; }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
}
```

Définir dans `link.rs` en `pub(crate) fn fingerprint_hex(id: &str, source: &str) -> String` (sha-256 de `id`, `0`, `source`, en hex) ; `client.rs` (Task 7) la réutilise — une seule définition. Elle doit donner le même résultat que `hex::encode(PackWidgets::fingerprint())` (test : `fingerprint_hex_matches_pack_fingerprint`).

En `InProcess`, un délai dépassé abandonne le fil du travailleur (il n'est pas tuable) : il finit seul quand son calcul se termine, écrit dans un tube fermé et sort. Documenter cette limite en tête de `link.rs`.

- [ ] **Step 4 : lancer** — `cargo nextest run -p oku-slides-render-worker` → PASS. Le test de délai doit finir en moins de 2 s.

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-render-worker
```
```bash
git commit -m "feat(render-worker) : lien avec le travailleur, délai, mémoire, relance et rejeu (t-327)"
```

---

### Task 7 : `RenderWorker`, `RenderSlot`, `RemoteCompiled` et tests en vrai processus

**Files:**
- Create: `crates/oku-slides-render-worker/src/client.rs`, `src/client_tests.rs`, `tests/process.rs`
- Modify: `crates/oku-slides-render-worker/src/lib.rs`, `ARCHITECTURE.md`

**Interfaces:**
- Consumes : `Link` (Task 6), `registry_from_probe`, `PackWidgets::fingerprint`, `Fonts::files` (Task 2).
- Produces :

```rust
/// Un travailleur de rendu partagé par les rendus d'une session.
#[derive(Clone)]
pub struct RenderWorker(Arc<Mutex<Link>>, Arc<Shared>);
impl RenderWorker {
    pub fn new(launch: RenderLaunch, settings: RenderSettings) -> Self;
    /// Le registre des widgets de `pack` (base surchargée), sonde exécutée dans le travailleur, gardé par empreinte.
    pub fn probe(&self, pack: &PackWidgets) -> Result<Arc<WidgetRegistry>, RegistryError>;
    /// `pack` a fait échouer le rendu (délai, mémoire, arrêt) : il n'est plus exécuté dans ce travailleur.
    pub fn is_faulty(&self, pack: &PackWidgets) -> bool;
    /// Ouvre un emplacement avec ces polices, les widgets de base et la langue `l`.
    pub fn slot(&self, fonts: &Fonts, l: &Localizer, base: Arc<WidgetRegistry>) -> Result<RenderSlot, RenderError>;
}

pub struct RenderSlot { /* worker, id, pack, registry, chart_palette */ }
impl RenderSlot {
    pub fn set_widgets(&mut self, pack: Option<&PackWidgets>, registry: Arc<WidgetRegistry>) -> Result<(), RenderError>;
    pub fn registry(&self) -> &Arc<WidgetRegistry>;
    pub fn set_language(&mut self, l: &Localizer) -> Result<(), RenderError>;
    pub fn render(&mut self, deck: &Deck, assets: &BTreeMap<String, Vec<u8>>) -> Result<RemoteCompiled, RenderError>;
    pub fn chart_palette(&self) -> &[String];
}
// Drop for RenderSlot : met `CloseSlot` en file (envoyé avant la requête suivante).

pub struct RemoteCompiled { pub warnings: Vec<Diagnostic>, /* worker, slot, generation, hashes, layout */ }
impl RemoteCompiled {
    pub fn page_count(&self) -> usize;
    pub fn page_hash(&self, index: usize) -> Option<u128>;
    pub fn layout(&self) -> &Layout;          // tokens reposés depuis l'en-tête
    pub fn svg(&self, index: usize) -> Result<String, RenderError>;
    pub fn png(&self, index: usize, pixel_per_pt: f32) -> Result<Vec<u8>, RenderError>;
    pub fn pdf(&self) -> Result<Vec<u8>, RenderError>;
}
// Drop for RemoteCompiled : met `Release { slot, [generation] }` en file.
```

`Shared` : `deferred: Mutex<Vec<Request>>` des requêtes différées (`Release`, `CloseSlot`) — un verrou à part, pour qu'un `Drop` ne prenne jamais le verrou du lien ; chaque méthode de `RenderWorker`/`RenderSlot`/`RemoteCompiled`, une fois le verrou du lien pris, envoie d'abord ces requêtes (`link.call`, réponses ignorées ; un échec du lien les jette, le travailleur relancé ne les connaît plus) ; `registries: Mutex<HashMap<String, Arc<WidgetRegistry>>>` par empreinte hex. Les packs fautifs sont dans `Link::faulty_packs` (Task 6), clés `fingerprint_hex`.

Règles :
- `probe` : faute connue → `Err(RegistryError::worker(RenderError::pack_disabled(id)))` sans appel ; sinon `Request::Probe`, `Response::Probe { json }` → `registry_from_probe(json)`, gardé par empreinte ; `Response::Error` → `into_registry()` ; un échec borné du lien marque l'empreinte fautive puis `RegistryError::worker(e)`.
- `slot` : envoie les polices absentes de `sent_fonts` (`PutFont { id: hex(sha256) }`), `OpenSlot`, enregistre `SlotState` ; identifiants d'emplacement croissants.
- `RenderSlot::set_widgets(pack)` : pack fautif → `Err(pack_disabled)` ; sinon `SetWidgets`, puis `SlotState.pack` ; un échec borné marque le pack fautif.
- `RenderSlot::render` : si `SlotState.pack` est fautif → `Err(pack_disabled)` **sans appel** (un travailleur relancé a rejoué l'emplacement avec la base : rendre en silence avec d'autres widgets que ceux de la session casserait l'invariant de `deck_widgets.rs`). Sinon `PutAsset` des ids absents de `sent_assets`, `Render { slot, deck, assets: ids }` ; `sent_assets` = union des `last_assets` (même règle que le travailleur) ; un échec borné marque le pack de l'emplacement fautif.
- `RemoteCompiled::svg/png/pdf` : `Response::Bytes` → octets ; `Response::Error` → `into_render()` (dont `Stale`).
- Le rejeu (Task 6) d'un emplacement dont le pack est fautif envoie `SetWidgets(None)`.

- [ ] **Step 1 : tests en échec** — `client_tests.rs` (`InProcess`) :

```rust
use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_i18n::Localizer;
use oku_slides_platform::{RenderLaunch, RenderSettings};
use oku_slides_render::{Fonts, PackWidgets, RenderErrorKind, base_registry};
use speculoos::prelude::*;

use super::*;
use crate::test_packs::{LOOP_ON_IMPORT, LOOP_ON_RENDER, tiny_deck};

fn worker(timeout_ms: u64) -> RenderWorker {
    RenderWorker::new(RenderLaunch::InProcess, RenderSettings { timeout_ms, memory_mib: 4096, max_frame_mib: 64 })
}

fn slot(w: &RenderWorker) -> RenderSlot {
    w.slot(&Fonts::default(), &Localizer::new(Some("fr")), base_registry(&Fonts::default()).unwrap()).unwrap()
}

#[test]
fn slot_render_then_svg_returns_svg() {
    let w = worker(10_000);
    let mut s = slot(&w);
    let c = s.render(&tiny_deck(), &BTreeMap::new()).unwrap();
    assert_that!(c.svg(0).unwrap()).starts_with("<svg");
}

#[test]
fn probe_is_cached_by_fingerprint() {
    let w = worker(10_000);
    let pack = PackWidgets::new("vide", "#let registry = ()").unwrap();
    assert_that!(Arc::ptr_eq(&w.probe(&pack).unwrap(), &w.probe(&pack).unwrap())).is_true();
}

#[test]
fn probe_that_loops_marks_pack_faulty() {
    let w = worker(300);
    let pack = PackWidgets::new("boucle", LOOP_ON_IMPORT).unwrap();
    let err = w.probe(&pack).unwrap_err();
    assert_that!(err.render_error().map(|e| e.kind())).is_equal_to(Some(RenderErrorKind::Timeout));
    assert_that!(w.is_faulty(&pack)).is_true();
}

#[test]
fn render_with_looping_widget_times_out_then_is_disabled_without_retry() {
    let w = worker(300);
    let pack = PackWidgets::new("rendu", LOOP_ON_RENDER).unwrap();
    let registry = w.probe(&pack).unwrap();
    let mut s = slot(&w);
    s.set_widgets(Some(&pack), registry).unwrap();
    let deck = oku_slides_core::Deck::from_json(r#"{"version":2,"params":{"title":"T"},"slides":[{"id":"s","kind":"content","params":{"blocks":[{"id":"b","kind":"boucle","params":{"body":"x"}}]}}]}"#).unwrap();
    assert_that!(s.render(&deck, &BTreeMap::new()).unwrap_err().kind()).is_equal_to(RenderErrorKind::Timeout);
    let started = std::time::Instant::now();
    assert_that!(s.render(&deck, &BTreeMap::new()).unwrap_err().kind()).is_equal_to(RenderErrorKind::PackDisabled);
    assert_that!(started.elapsed()).is_less_than(std::time::Duration::from_millis(100));
}

#[test]
fn dropping_remote_compiled_releases_its_generation() {
    let w = worker(10_000);
    let mut s = slot(&w);
    let first = s.render(&tiny_deck(), &BTreeMap::new()).unwrap();
    let generation = first.generation;
    drop(first);
    let second = s.render(&tiny_deck(), &BTreeMap::new()).unwrap();
    // La génération libérée n'est plus servie.
    let stale = RemoteCompiled { generation, ..second.clone_handle_for_test() };
    assert_that!(stale.svg(0).unwrap_err().kind()).is_equal_to(RenderErrorKind::Stale);
}
```

(`clone_handle_for_test` : `#[cfg(test)] pub(crate) fn` sur `RemoteCompiled` qui copie la poignée sans enregistrer de `Release` au `Drop`.)

`tests/process.rs` (vrai sous-processus) :

```rust
//! Le travailleur dans un vrai sous-processus : bornes et relance.

use std::collections::BTreeMap;
use std::path::PathBuf;

use oku_slides_i18n::Localizer;
use oku_slides_platform::{RenderLaunch, RenderSettings};
use oku_slides_render::{Fonts, PackWidgets, RenderErrorKind, Renderer, base_registry};
use oku_slides_render_worker::RenderWorker;
use speculoos::prelude::*;

#[path = "../src/test_packs.rs"]
#[allow(dead_code, reason = "packs partagés avec les tests unitaires")]
mod test_packs;

fn launch() -> RenderLaunch {
    RenderLaunch::Process { program: PathBuf::from(env!("CARGO_BIN_EXE_oku-slides-render-worker")) }
}

fn worker(timeout_ms: u64, memory_mib: u64) -> RenderWorker {
    RenderWorker::new(launch(), RenderSettings { timeout_ms, memory_mib, max_frame_mib: 256 })
}

fn roadmap() -> oku_slides_core::Deck {
    oku_slides_core::Deck::from_json(include_str!("../../../fixtures/roadmap.json")).unwrap()
}

#[test]
fn process_render_is_identical_to_in_memory_render() {
    let l = Localizer::new(Some("fr"));
    let mut local = Renderer::new(Fonts::default()).unwrap();
    local.set_language(&l).unwrap();
    let expected = local.render(&roadmap(), &BTreeMap::new()).unwrap();
    let w = worker(30_000, 4096);
    let mut s = w.slot(&Fonts::default(), &l, base_registry(&Fonts::default()).unwrap()).unwrap();
    let got = s.render(&roadmap(), &BTreeMap::new()).unwrap();
    assert_that!(got.page_count()).is_equal_to(expected.page_count());
    for i in 0..expected.page_count() {
        assert_that!(got.svg(i).unwrap()).is_equal_to(expected.svg(i).unwrap());
    }
    assert_that!(serde_json::to_value(got.layout()).unwrap()).is_equal_to(serde_json::to_value(expected.layout()).unwrap());
}

#[test]
fn process_probe_that_loops_times_out() {
    let w = worker(500, 4096);
    let err = w.probe(&PackWidgets::new("boucle", test_packs::LOOP_ON_IMPORT).unwrap()).unwrap_err();
    assert_that!(err.render_error().map(|e| e.kind())).is_equal_to(Some(RenderErrorKind::Timeout));
}

#[test]
fn process_probe_that_allocates_is_stopped_out_of_memory() {
    let w = worker(60_000, 300);
    let err = w.probe(&PackWidgets::new("gros", test_packs::ALLOCATE).unwrap()).unwrap_err();
    assert_that!(err.render_error().map(|e| e.kind())).is_equal_to(Some(RenderErrorKind::OutOfMemory));
}

#[test]
fn process_killed_from_outside_is_reported_lost_then_restarted() {
    let w = worker(30_000, 4096);
    let l = Localizer::new(Some("fr"));
    let mut s = w.slot(&Fonts::default(), &l, base_registry(&Fonts::default()).unwrap()).unwrap();
    s.render(&roadmap(), &BTreeMap::new()).unwrap();
    w.kill_for_test();
    assert_that!(s.render(&roadmap(), &BTreeMap::new()).unwrap_err().kind()).is_equal_to(RenderErrorKind::WorkerLost);
    assert_that!(s.render(&roadmap(), &BTreeMap::new()).map(|c| c.page_count()).unwrap()).is_greater_than(0);
}
```

`RenderWorker::kill_for_test(&self)` : `#[doc(hidden)] pub fn`, tue le fils sans toucher à l'état du lien (le test d'intégration ne voit que l'API publique).

- [ ] **Step 2 : lancer** — `cargo nextest run -p oku-slides-render-worker` → FAIL.

- [ ] **Step 3 : implémenter** `client.rs` selon les règles ; `lib.rs` : `pub use client::{RemoteCompiled, RenderSlot, RenderWorker};` et doc de module qui liste `RenderWorker`, `RenderSlot`, `RemoteCompiled`, `serve`, `exit_if_worker`. `ARCHITECTURE.md` : types clés, point d'extension (« une nouvelle sortie d'une page : une requête dans `protocol.rs`, sa branche dans `serve.rs`, une méthode sur `RemoteCompiled` »), dépendances.

- [ ] **Step 4 : lancer** — `cargo nextest run -p oku-slides-render-worker` → PASS (les tests de processus ensemble en moins de 30 s).

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-render-worker
```
```bash
git commit -m "feat(render-worker) : RenderWorker, RenderSlot et RemoteCompiled, bornes vérifiées en vrai processus (t-327)"
```

---

### Task 8 : la session rend par le travailleur et se replie sur la base

**Files:**
- Modify: `crates/oku-slides-session/Cargo.toml` (dépendance `oku-slides-render-worker`)
- Modify: `crates/oku-slides-session/src/lib.rs:34,286-340,381-393,723-735`, `deck_widgets.rs`, `render_fonts.rs`, `history.rs:81-90`, `templates_mode.rs:191-200`, `agent/variants.rs:105-113`, `workspace.rs:331,370`, `workspace_same_deck.rs:50`
- Modify: `crates/oku-slides-session/src/deck_widgets_tests.rs`
- Modify: `crates/oku-slides-i18n/src/locales/*/session.ftl` (20 langues)

**Interfaces:**
- Consumes : `RenderWorker::{new, probe, is_faulty, slot}`, `RenderSlot`, `RemoteCompiled` (Task 7) ; `Platform::render_launch`, `Settings::render` (Task 3) ; `base_registry` (Task 2) ; `RenderErrorKind::is_bounded` (Task 1).
- Produces :
  - `Session.worker: RenderWorker` ; `renderer`, `template_renderer`, `variant_renderer` : `RenderSlot` ; `compiled: Option<RemoteCompiled>`.
  - `new_renderer(worker: &RenderWorker, fonts: Fonts, l: &Localizer, pack: Option<&PackWidgets>, registry: &Arc<WidgetRegistry>) -> Result<RenderSlot>`.
  - `Session::render_deck(&mut self, deck: &Deck) -> Result<RemoteCompiled>` (crate) : rendu du deck avec repli.

- [ ] **Step 1 : tests en échec** — `deck_widgets_tests.rs` (reprendre ses aides : bibliothèque de modèles temporaire, `library.save_widgets`) :

Ajouter en tête de `deck_widgets_tests.rs`, à côté de `QUOTE` et `BROKEN` :

```rust
/// Un widget `boucle` qui boucle au rendu, hors des gardes de Typst.
const LOOP_ON_RENDER: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let boucle = widget("boucle", params: (body: p.plain()),
  (args, ctx) => { let n = 0; for _ in range(1000000000000) { n += 1 }; [#n] })
#let registry = (boucle,)
"#;

/// Un pack qui boucle dès son chargement (sonde).
const LOOP_ON_IMPORT: &str = r#"#let _spin = { let n = 0; for _ in range(1000000000000) { n += 1 }; n }
#let registry = ()
"#;

/// Comme `session_in`, avec un délai de rendu court.
fn session_with_timeout(data: &Path, timeout_ms: u64) -> Session {
    let mut settings = Settings::defaults().unwrap();
    settings.paths.data = Some(data.to_owned());
    settings.paths.decks = Some(data.join("decks"));
    settings.i18n.locale = Some("fr".into());
    settings.render.timeout_ms = timeout_ms;
    let mut s = Session::new(Fonts::default(), &Platform::with_settings(settings).unwrap()).unwrap();
    s.open(&fixture_path()).unwrap();
    s
}

/// Un élément `boucle` ajouté aux blocs de `s-constat`.
fn add_loop() -> Op {
    op(json!({
        "op": "add_element",
        "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "element": { "id": "l", "kind": "boucle", "params": { "body": "x" } }
    }))
}
```

Tests :

```rust
#[test]
fn apply_with_a_widget_that_loops_falls_back_to_the_base_registry_with_a_notice() {
    let data = data_dir();
    write_pack(&data.0, "boucle", LOOP_ON_RENDER);
    let mut s = session_with_timeout(&data.0, 300);
    let base = s.registry.clone();
    // Le pack est installé, puis son widget boucle au rendu : l'élément reste, la base le rend.
    s.apply("Pack", vec![set_templates(Some("boucle")), add_loop()], None).unwrap();
    assert_that!(Arc::ptr_eq(&s.registry, &base)).is_true();
    assert_that!(s.deck().slide("s-constat").is_some_and(|sl| sl.locate("l").is_some())).is_true();
    assert_that!(s.doc.notice.clone().unwrap_or_default().as_str()).contains("boucle");
    assert_registry_agrees(&s);
}

#[test]
fn apply_set_templates_to_a_pack_that_loops_keeps_the_base_registry_and_sets_a_notice() {
    let data = data_dir();
    write_pack(&data.0, "boucle", LOOP_ON_IMPORT);
    let mut s = session_with_timeout(&data.0, 300);
    let base = s.registry.clone();
    s.apply("Pack", vec![set_templates(Some("boucle"))], None).unwrap();
    assert_that!(Arc::ptr_eq(&s.registry, &base)).is_true();
    assert_that!(s.doc.notice.is_some()).is_true();
    assert_registry_agrees(&s);
}

#[test]
fn a_pack_that_looped_is_not_retried_in_the_session() {
    let data = data_dir();
    write_pack(&data.0, "boucle", LOOP_ON_IMPORT);
    let mut s = session_with_timeout(&data.0, 300);
    s.apply("Pack", vec![set_templates(Some("boucle"))], None).unwrap();
    let started = std::time::Instant::now();
    s.apply("Pack", vec![set_templates(None)], None).unwrap();
    s.apply("Pack", vec![set_templates(Some("boucle"))], None).unwrap();
    assert_that!(started.elapsed()).is_less_than(std::time::Duration::from_millis(250));
}
```

(`assert_registry_agrees` lit `s.renderer.registry()` : `RenderSlot::registry` a la même signature que `Renderer::registry`.)

- [ ] **Step 2 : lancer** — `cargo nextest run -p oku-slides-session deck_widgets` → FAIL (compile : pas de `RenderWorker`).

- [ ] **Step 3 : implémenter**

`lib.rs` :
- `use oku_slides_render_worker::{RemoteCompiled, RenderSlot, RenderWorker};` ; retirer `Compiled`, `Registries`, `Renderer` des imports de render.
- Champs : `worker: RenderWorker` ; `renderer: RenderSlot` ; `template_renderer`/`variant_renderer: Option<RenderSlot>` ; `compiled: Option<RemoteCompiled>` ; retirer `registries`.
- `new_renderer` :

```rust
pub(crate) fn new_renderer(
    worker: &RenderWorker,
    fonts: Fonts,
    l: &Localizer,
    pack: Option<&PackWidgets>,
    registry: &Arc<WidgetRegistry>,
) -> Result<RenderSlot> {
    let base = oku_slides_render::base_registry(&fonts).context(RegistrySnafu)?;
    let mut slot = worker.slot(&fonts, l, base).context(RenderSnafu)?;
    if pack.is_some() {
        slot.set_widgets(pack, registry.clone()).context(RenderSnafu)?;
    }
    Ok(slot)
}
```

- `Session::new` : `let worker = RenderWorker::new(platform.render_launch.clone(), settings.render.clone());` ; `let registry = oku_slides_render::base_registry(&app_fonts).context(RegistrySnafu)?;` ; `renderer: new_renderer(&worker, app_fonts.clone(), l, None, &registry)?`.
- `install(&mut self, compiled: RemoteCompiled)` : inchangé sinon.
- `rerender` : `let compiled = self.render_deck(&self.doc.sources.deck().clone())?;` (le clone évite l'emprunt simultané ; `render_deck` prend `&Deck`).

`deck_widgets.rs` :
- `resolve_widgets` : `PackWidgets::new(id, source)` puis `self.worker.probe(&pack)` (au lieu de `self.registries.get`) ; base : `oku_slides_render::base_registry(&self.app_fonts)`. Un pack déjà fautif sort par la branche d'erreur existante (avis, base) : `probe` rend `pack_disabled`.
- `commit` : `self.render_deck(&next)?` au lieu de `self.renderer.render(...)`.
- Nouvelle fonction :

```rust
    /// Rend `deck` avec les widgets installés. Si leur pack a fait échouer
    /// le rendu (délai, mémoire, arrêt du processus de rendu), les widgets de
    /// base les remplacent — un avis nomme le pack et la cause — et le rendu
    /// est refait une fois.
    pub(crate) fn render_deck(&mut self, deck: &Deck) -> Result<RemoteCompiled> {
        let e = match self.renderer.render(deck, &self.doc.sources.assets) {
            Ok(compiled) => return Ok(compiled),
            Err(e) => e,
        };
        let faulty = self.pack.as_ref().is_some_and(|p| self.worker.is_faulty(p));
        if !(e.kind().is_bounded() && faulty) {
            return Err(e).context(RenderSnafu)?;
        }
        let pack = self.pack.as_ref().map(|p| p.id().to_owned()).unwrap_or_default();
        tracing::warn!(pack, error = %oku_slides_platform::errors::one_line(&e), "pack de widgets écarté : widgets de base");
        let notice = self.localizer.tr_with(
            "session-pack-disabled",
            &[("pack", pack.as_str().into()), ("reason", e.localize(&self.localizer).into())],
        );
        let base = oku_slides_render::base_registry(&self.app_fonts).context(RegistrySnafu)?;
        self.install_widgets(DeckWidgets { templates: self.widgets_templates.clone(), pack: None, registry: base })?;
        self.doc.sources.set_registry(self.registry.clone());
        self.set_notice(notice);
        Ok(self.renderer.render(deck, &self.doc.sources.assets).context(RenderSnafu)?)
    }
```

`install_widgets` : sur un emplacement dont le pack est fautif, `set_widgets(None, base)` doit réussir — c'est le cas (Task 7 : seul un pack fautif est refusé). Le retour arrière existant (`set_widgets` des widgets d'avant) reste.

`render_fonts.rs` : `fonts_renderer` appelle `new_renderer(&self.worker, …)` ; `FontsRenderer.renderer: RenderSlot`.

`history.rs` : `version_document` → `let compiled = self.render_deck(&sources.deck().clone())?;` et type de retour `(Document, RemoteCompiled)`.

`templates_mode.rs`, `agent/variants.rs` : `new_renderer(&self.worker, …)`.

`workspace.rs` : retirer le champ `registries` ; `workspace_same_deck.rs:50` → `oku_slides_render::base_registry(&w.fonts).context(RegistrySnafu)?`.

Fluent — `session.ftl` de chaque langue :

```ftl
# fr
session-pack-disabled = Les widgets du pack « { $pack } » ont été désactivés pour cette session : { $reason }. Le deck s'affiche avec les widgets de base.
# en
session-pack-disabled = The widgets of pack “{ $pack }” were disabled for this session: { $reason }. The deck is shown with the base widgets.
```

(Traduire pour les 18 autres langues.)

- [ ] **Step 4 : lancer** — `cargo nextest run -p oku-slides-session` → PASS (tous : les sessions de test sont en `InProcess`). Puis `cargo nextest run -p oku-slides-mcp -p oku-slides-agent` → PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/oku-slides-session crates/oku-slides-i18n
```
```bash
git commit -m "feat(session) : rendu par le processus de rendu, repli sur les widgets de base d'un pack fautif (t-327)"
```

---

### Task 9 : binaires et régression en vrai processus

**Files:**
- Modify: `src-tauri/src/main.rs` (tête de `main`), `src-tauri/Cargo.toml`
- Modify: `crates/oku-slides-cli/src/main.rs` (tête de `main`), `crates/oku-slides-cli/Cargo.toml`
- Create: `crates/oku-slides-session/tests/regressions/issue_0327_pack_qui_boucle.rs`
- Modify: `crates/oku-slides-session/Cargo.toml` (`[[test]]`)

**Interfaces:**
- Consumes : `exit_if_worker` (Task 5), `Platform::with_render_launch` (Task 3).

- [ ] **Step 1 : test de régression en échec** — `Cargo.toml` de session :

```toml
[[test]]
# Binaire à main propre : il sert aussi de travailleur (`--render-worker`).
name = "issue_0327_pack_qui_boucle"
path = "tests/regressions/issue_0327_pack_qui_boucle.rs"
harness = false
```

`issue_0327_pack_qui_boucle.rs` :

```rust
//! t-327 : un pack dont un widget boucle ne gèle plus la session.
//!
//! Binaire sans harnais : lancé avec `--render-worker`, il est le processus
//! de rendu ; sinon il est le test.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use oku_slides_core::Op;
use oku_slides_platform::RenderLaunch;
use oku_slides_session::{Fonts, Platform, Request, Session, Settings};

fn main() {
    oku_slides_render_worker::exit_if_worker();
    looping_pack_render_returns_within_timeout_with_base_widgets();
    println!("issue_0327_pack_qui_boucle: ok");
}

const LOOP_ON_RENDER: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let boucle = widget("boucle", params: (body: p.plain()),
  (args, ctx) => { let n = 0; for _ in range(1000000000000) { n += 1 }; [#n] })
#let registry = (boucle,)
"#;

fn looping_pack_render_returns_within_timeout_with_base_widgets() {
    let data = tempfile::Builder::new().prefix("oku-slides-0327-").tempdir().unwrap();
    let pack = data.path().join("templates").join("boucle");
    std::fs::create_dir_all(&pack).unwrap();
    std::fs::write(pack.join("pack.json"), r#"{"id":"boucle","name":"Boucle","templates":[]}"#).unwrap();
    std::fs::write(pack.join("widgets.typ"), LOOP_ON_RENDER).unwrap();

    let mut settings = Settings::defaults().unwrap();
    settings.paths.data = Some(data.path().to_owned());
    settings.paths.decks = Some(data.path().join("decks"));
    settings.i18n.locale = Some("fr".into());
    settings.render.timeout_ms = 1_000;
    let program = std::env::current_exe().unwrap();
    let platform = Platform::with_settings(settings).unwrap().with_render_launch(RenderLaunch::Process { program });
    let mut s = Session::new(Fonts::default(), &platform).unwrap();
    s.open(&PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"))).unwrap();

    let ops: Vec<Op> = serde_json::from_value(serde_json::json!([
        { "op": "set_templates", "templates": "boucle" },
        { "op": "add_element",
          "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
          "element": { "id": "l", "kind": "boucle", "params": { "body": "x" } } }
    ]))
    .unwrap();
    let started = Instant::now();
    s.apply("Pack", ops, None).unwrap();
    let elapsed = started.elapsed();
    assert!(elapsed < Duration::from_millis(1_000 + 3_000), "session gelée : {elapsed:?}");
    let widgets: serde_json::Value = serde_json::from_str(&s.handle(Request::Widgets).unwrap()).unwrap();
    assert!(widgets["widgets"].get("boucle").is_none(), "le pack fautif est encore installé");
}
```

Dev-dépendances de session à vérifier : `oku-slides-core`, `oku-slides-platform` (pour `RenderLaunch`), `oku-slides-render-worker`, `tempfile`, `serde_json`.

- [ ] **Step 2 : lancer** — `cargo nextest run -p oku-slides-session --test issue_0327_pack_qui_boucle` → PASS attendu si Task 8 est complète ; **vérifier qu'il échoue** en remplaçant temporairement `RenderLaunch::Process` par `InProcess` et `timeout_ms` par `600_000` : le test doit alors dépasser la borne (on l'interrompt) — preuve qu'il mesure le gel. Remettre le code.

- [ ] **Step 3 : binaires** — première ligne de `main` dans `src-tauri/src/main.rs` et `crates/oku-slides-cli/src/main.rs`, avant `color_eyre::install()` :

```rust
    // Relancé en processus de rendu (`--render-worker`) : sert le tube, puis termine.
    oku_slides_render_worker::exit_if_worker();
```

Ajouter `oku-slides-render-worker.workspace = true` aux deux manifestes. La CLI garde `Renderer` pour `render`/`save` (widgets de base seuls, voir la spec).

- [ ] **Step 4 : vérification manuelle** — `cargo build -p oku-slides-cli` puis :

```bash
printf '' | target/debug/oku-slides --render-worker --max-frame-bytes 1024; echo $?
```

Attendu : `0` (entrée vide : fin propre). Puis `just check` → PASS.

- [ ] **Step 5 : commit**

```bash
git add src-tauri crates/oku-slides-cli crates/oku-slides-session
```
```bash
git commit -m "feat(app) : binaires relancés en processus de rendu, régression du pack qui boucle (t-327)"
```

---

### Task 10 : documentation et fermeture

**Files:**
- Modify: `crates/oku-slides-render/ARCHITECTURE.md` (erreurs bornées, `wire`, sonde brute, `base_registry` public, `Fonts::files`)
- Modify: `crates/oku-slides-session/ARCHITECTURE.md` (`RenderWorker`, emplacements, `render_deck` et repli)
- Modify: `crates/oku-slides-platform/ARCHITECTURE.md` s'il existe (`[render]`, `RenderLaunch`)
- Modify: `README.md` si une section décrit l'architecture du rendu

- [ ] **Step 1** : mettre à jour les documents ci-dessus (chacun sous 50 lignes).
- [ ] **Step 2** : passe de consolidation (G10) — `rg "Registries::new|Renderer::new" crates src-tauri` : seuls restent le travailleur (`serve.rs`), la CLI (`render`, `save`, `bench`) et les tests de render ; `rg "load_registry\(" crates` : aucun appel hors de render et de ses tests.
- [ ] **Step 3** : `just check` → PASS.
- [ ] **Step 4 : commit**

```bash
git add crates/*/ARCHITECTURE.md README.md
```
```bash
git commit -m "docs(render) : architecture du processus de rendu borné (t-327)"
```
