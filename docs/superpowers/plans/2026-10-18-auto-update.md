# Automatic updates Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The desktop app finds newer GitHub releases, offers them, and installs one with a restart that keeps every tab.

**Architecture:** `tauri-plugin-updater` driven by the Tauri host (`src-tauri/src/updates.rs`): one check task, commands for workspace windows, `state::settle` and the install in one blocking job, then `request_restart`. Pure, testable decisions (install mode, settings, progress and size cap, trusted URL, error keys, wire types) live in `oku-slides-platform::updates`, because `just test` excludes `src-tauri`. `release.yml` signs updater bundles in a protected environment, gates `latest.json` before publishing; the React UI shows an `UpdateBar` per workspace window.

**Tech Stack:** Rust (Tauri 2.12, tauri-plugin-updater 2.14 without default features, tauri-plugin-opener 2.7, tokio `time`/`sync`/`macros`, Snafu, Fluent), React/TS (Bun tests, happy-dom), GitHub Actions (`tauri-apps/tauri-action`, pinned by SHA).

**Spec:** `docs/superpowers/specs/2026-10-18-auto-update-design.md`

**Task:** parent t-967; one sub-task per plan task below (t-970 … t-975).

## Global Constraints

- Endpoint: `https://github.com/oku-theory/oku-slides/releases/latest/download/latest.json`.
- Releases page (`NotifyOnly`): `https://github.com/oku-theory/oku-slides/releases/latest`.
- Downloads accepted only from `https://github.com/oku-theory/oku-slides/releases/download/`; at most 512 MiB.
- A check gives up after 30 s.
- Config: `[updates] check_automatically = true`, `check_interval_hours = 6`; no automatic check in debug builds.
- `NotifyOnly` when: Linux without `APPIMAGE`; macOS run from App Translocation or a mounted image (`/Volumes/`); or an install of that same version failed before.
- Install only inside the blocking job that ran `state::settle` first; restart with `request_restart`, never from the webview. A failed install restarts the current version and remembers the version.
- Update commands answer workspace windows only (never `agent_window::LABEL`); no `updater:*` or `opener:*` permission in any capability file.
- Release notes are not carried to the UI (`latest.json` is unsigned; its body is the CI text).
- No `unwrap`/`expect` outside tests (`app.state()` counts: use `State<>` parameters or `try_state`); Snafu errors, opaque, a backtrace per variant, Fluent messages.
- Every new Fluent key in all 20 locales (`crates/oku-slides-i18n/src/locales/*`), same position and order everywhere, translated (never English copied).
- Dependency versions in root `[workspace.dependencies]`, `.workspace = true` in crates.
- Rust unit tests in sibling `x_tests.rs`, speculoos, names `unit_scenario_result`, rstest for tables.
- Every pub item documented, first sentence under 15 words; code comments in French; docs in English.
- Commits: `type(scope) : description (t-967)`, message written to a scratchpad file and passed with `git commit -F`; no `Co-Authored-By` line.
- Runs outside `just e2e` set `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS` to a fresh temp dir.

## Deviations from the spec (approved with this plan; the spec is updated in the same commit)

1. `just e2e` drives the CLI `serve` dev server, never the Tauri host: no e2e env var and no e2e spec.
   The UI hides every update control outside Tauri.
2. A failed install **after** `settle` restarts the current version and records the version, which
   is then offered as `NotifyOnly`.
3. Names: `auto_check_enabled` is `UpdateSettings::auto_check`; `UpdateState` is `Updates`;
   `check_update` returns `Option<UpdateStatus>`-style data (`UpdateStatus` with `offer: null` means
   up to date); the hook's `dismiss` is `later`.
4. Error stages: check (timeouts included), download (signature verification included: the plugin
   reports both from `download`), install, open, unavailable, refused.
5. `notes` is dropped from the offer (unsigned, unused, CI text).
6. `createUpdaterArtifacts` is set by the release workflow only (`--config`), not in
   `tauri.conf.json`, so `just build` keeps working without the signing key.
7. `update-available` becomes `update-changed`, a signal without payload: every window re-reads
   `update_status`, which also carries `installing`.

---

### Task 0: Updater signing key and release environment (the user)

Not delegated: only the user holds the key. Task 2 is blocked until Step 4 is done.

- [ ] **Step 1:** `cargo tauri signer generate -w ~/.tauri/oku-slides.key`, with a password.
- [ ] **Step 2:** Store the key file and its password in the password manager (offline backup).
  Losing it strands every installed app; a leak lets anyone sign an update.
- [ ] **Step 3:** GitHub → oku-theory/oku-slides → Settings → Environments → New `release`:
  deployment branches = `main` only. Add the **environment** secrets `TAURI_SIGNING_PRIVATE_KEY`
  (content of `~/.tauri/oku-slides.key`) and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. No repo-level
  secret with these names.
- [ ] **Step 4:** Hand the one-line content of `~/.tauri/oku-slides.key.pub` to the implementer of
  Task 2, then delete `~/.tauri/oku-slides.key` from disk (the backup stays).

---

### Task 1: `platform::updates` and the `[updates]` settings

**Files:**
- Create: `crates/oku-slides-platform/src/updates.rs`, `crates/oku-slides-platform/src/updates_tests.rs`
- Modify: `crates/oku-slides-platform/src/config.rs` (`Settings::updates`, `UpdateSettings`)
- Modify: `crates/oku-slides-platform/src/config.default.toml` (append `[updates]`)
- Modify: `crates/oku-slides-platform/src/config_tests.rs`
- Modify: `crates/oku-slides-platform/src/lib.rs` (`pub mod updates;`, module list in the crate doc)
- Modify: `crates/oku-slides-platform/Cargo.toml` (dev-dependency `rstest`)
- Modify: `crates/oku-slides-i18n/src/locales/<all 20>/session.ftl`
- Modify: `crates/oku-slides-platform/ARCHITECTURE.md`, `README.md` (Settings)

**Interfaces:**
- Produces (exact names, used by Tasks 2 and 4):
  - `config::UpdateSettings { pub check_automatically: bool, pub check_interval_hours: NonZeroU64 }`, `fn interval(&self) -> Duration`, `fn auto_check(&self, debug_build: bool) -> bool`; field `Settings::updates`.
  - `updates::InstallMode { Install, NotifyOnly }` (`Serialize`, snake_case), `fn detect(host: &Host<'_>) -> Self`.
  - `updates::Host<'a> { pub os: &'a str, pub appimage: bool, pub exe: &'a Path, pub failed_before: bool }`.
  - `updates::UpdateOffer { pub version: String, pub mode: InstallMode }` (`Serialize`).
  - `updates::UpdateStatus { pub offer: Option<UpdateOffer>, pub installing: bool }` (`Serialize`).
  - `updates::UpdateProgress { pub received: u64, pub total: Option<u64> }` (`Serialize`).
  - `updates::Progress`, `fn new() -> Self`, `fn advance(&mut self, chunk: usize, total: Option<u64>) -> Advance`; `enum Advance { Report(UpdateProgress), Quiet, TooLarge }`.
  - `updates::trusted_download(url: &str) -> bool`.
  - `updates::UpdateStage { Check, Download, Install, Open, Unavailable, Refused }` (`#[non_exhaustive]`), `fn key(self) -> &'static str`, `impl Localize`.
  - consts `RELEASES_URL`, `DOWNLOAD_PREFIX`, `MAX_DOWNLOAD_BYTES: u64 = 512 * 1024 * 1024`, `CHECK_TIMEOUT: Duration = 30 s`, `FAILED_VERSION_FILE: &str = "updates/failed-version"`.

- [ ] **Step 1: Dev-dependency** — in `crates/oku-slides-platform/Cargo.toml` `[dev-dependencies]`, add `rstest.workspace = true` (not there today; the workspace pins `rstest = "0.26"`).

- [ ] **Step 2: Write the failing settings tests** (append to `config_tests.rs`)

```rust
#[test]
fn updates_defaults_check_every_six_hours() {
    let s = Settings::defaults().unwrap();
    assert_that!(s.updates.check_automatically).is_true();
    assert_that!(s.updates.interval()).is_equal_to(std::time::Duration::from_secs(6 * 3600));
}

#[test]
fn updates_env_false_turns_automatic_check_off() {
    let s = Settings::from_sources(None, env(&[("OKU_SLIDES_UPDATES__CHECK_AUTOMATICALLY", "false")])).unwrap();
    assert_that!(s.updates.check_automatically).is_false();
}

#[test]
fn updates_env_interval_sets_hours() {
    let s = Settings::from_sources(None, env(&[("OKU_SLIDES_UPDATES__CHECK_INTERVAL_HOURS", "12")])).unwrap();
    assert_that!(s.updates.interval()).is_equal_to(std::time::Duration::from_secs(12 * 3600));
}

#[test]
fn updates_zero_interval_is_rejected() {
    let s = Settings::from_sources(None, env(&[("OKU_SLIDES_UPDATES__CHECK_INTERVAL_HOURS", "0")]));
    assert_that!(s).is_err();
}

#[rstest::rstest]
#[case(true, false, true)]
#[case(true, true, false)]
#[case(false, false, false)]
fn updates_auto_check_follows_setting_and_build(#[case] on: bool, #[case] debug: bool, #[case] expected: bool) {
    let mut s = Settings::defaults().unwrap().updates;
    s.check_automatically = on;
    assert_that!(s.auto_check(debug)).is_equal_to(expected);
}
```

The `12` case proves the `0` case fails on `NonZeroU64`, not on a misspelled key.

- [ ] **Step 3: Run them to see them fail**

Run: `touch crates/*/src/lib.rs && cargo nextest run -p oku-slides-platform updates_`
Expected: compile error, `no field updates on type Settings`.

- [ ] **Step 4: Add the settings**

`config.default.toml`, appended:

```toml
[updates]
# Chercher une version plus récente au lancement, puis toutes les `check_interval_hours` heures
# (jamais dans une version de développement). Off : seulement par « Rechercher des mises à jour… ».
check_automatically = true
check_interval_hours = 6
```

`config.rs`: field `pub updates: UpdateSettings,` in `Settings` after `agent`, and:

```rust
/// Mises à jour de l'app depuis les releases GitHub.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct UpdateSettings {
    /// Chercher seul au lancement puis à intervalle régulier.
    pub check_automatically: bool,
    /// Heures entre deux recherches automatiques.
    pub check_interval_hours: std::num::NonZeroU64,
}

impl UpdateSettings {
    /// Délai entre deux recherches automatiques.
    pub fn interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.check_interval_hours.get().saturating_mul(3600))
    }

    /// Recherche automatique permise : réglage actif, et jamais en développement.
    pub fn auto_check(&self, debug_build: bool) -> bool {
        self.check_automatically && !debug_build
    }
}
```

- [ ] **Step 5: Run the settings tests**

Run: `touch crates/*/src/lib.rs && cargo nextest run -p oku-slides-platform updates_`
Expected: 7 passed (the rstest table counts 3).

- [ ] **Step 6: Declare the empty module so its tests compile**

`lib.rs`: `pub mod updates;` next to `pub mod storage;`, and in the crate doc's module list add
`//! - [`updates`] : ce que décide la mise à jour de l'app, sans Tauri (hôte seulement).`

`updates.rs`, for now only:

```rust
//! Mises à jour de l'app : ce qui se décide sans Tauri, testé ici.

#[cfg(test)]
#[path = "updates_tests.rs"]
mod updates_tests;
```

- [ ] **Step 7: Write the failing `updates` tests** (`updates_tests.rs`)

```rust
use std::path::Path;

use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use speculoos::prelude::*;

use super::*;

fn host<'a>(os: &'a str, appimage: bool, exe: &'a str, failed_before: bool) -> Host<'a> {
    Host { os, appimage, exe: Path::new(exe), failed_before }
}

#[rstest]
#[case(host("macos", false, "/Applications/Oku Slides.app/Contents/MacOS/oku-slides", false), InstallMode::Install)]
#[case(host("windows", false, "C:\\Program Files\\Oku Slides\\oku-slides.exe", false), InstallMode::Install)]
#[case(host("linux", true, "/tmp/.mount_oku/usr/bin/oku-slides", false), InstallMode::Install)]
#[case(host("linux", false, "/usr/bin/oku-slides", false), InstallMode::NotifyOnly)]
#[case(host("macos", false, "/private/var/folders/x/AppTranslocation/1/d/Oku Slides.app/Contents/MacOS/oku-slides", false), InstallMode::NotifyOnly)]
#[case(host("macos", false, "/Volumes/Oku Slides/Oku Slides.app/Contents/MacOS/oku-slides", false), InstallMode::NotifyOnly)]
#[case(host("macos", false, "/Applications/Oku Slides.app/Contents/MacOS/oku-slides", true), InstallMode::NotifyOnly)]
fn install_mode_detect_by_host(#[case] h: Host<'_>, #[case] expected: InstallMode) {
    assert_that!(InstallMode::detect(&h)).is_equal_to(expected);
}

#[test]
fn update_status_serializes_snake_case() {
    let status = UpdateStatus {
        offer: Some(UpdateOffer { version: "2.2.0".into(), mode: InstallMode::NotifyOnly }),
        installing: false,
    };
    assert_that!(serde_json::to_value(&status).unwrap()).is_equal_to(serde_json::json!({
        "offer": { "version": "2.2.0", "mode": "notify_only" },
        "installing": false
    }));
}

#[test]
fn progress_first_chunk_is_reported() {
    let mut p = Progress::new();
    assert_that!(p.advance(10, Some(1000))).is_equal_to(Advance::Report(UpdateProgress { received: 10, total: Some(1000) }));
}

#[test]
fn progress_below_one_percent_is_quiet() {
    let mut p = Progress::new();
    p.advance(10, Some(1000));
    assert_that!(p.advance(5, Some(1000))).is_equal_to(Advance::Quiet);
}

#[test]
fn progress_one_percent_more_is_reported() {
    let mut p = Progress::new();
    p.advance(10, Some(1000));
    assert_that!(p.advance(10, Some(1000))).is_equal_to(Advance::Report(UpdateProgress { received: 20, total: Some(1000) }));
}

#[test]
fn progress_unknown_total_reports_each_mebibyte() {
    let mut p = Progress::new();
    p.advance(1, None);
    assert_that!(p.advance(512 * 1024, None)).is_equal_to(Advance::Quiet);
    assert_that!(p.advance(512 * 1024, None)).is_equal_to(Advance::Report(UpdateProgress { received: 1 + 1024 * 1024, total: None }));
}

#[test]
fn progress_reaching_total_is_reported() {
    let mut p = Progress::new();
    p.advance(995, Some(1000));
    assert_that!(p.advance(5, Some(1000))).is_equal_to(Advance::Report(UpdateProgress { received: 1000, total: Some(1000) }));
}

#[test]
fn progress_past_total_reports_once() {
    let mut p = Progress::new();
    p.advance(1000, Some(1000));
    assert_that!(p.advance(1, Some(1000))).is_equal_to(Advance::Quiet);
}

#[test]
fn progress_zero_total_is_treated_as_unknown() {
    let mut p = Progress::new();
    assert_that!(p.advance(10, Some(0))).is_equal_to(Advance::Report(UpdateProgress { received: 10, total: None }));
}

#[test]
fn progress_over_cap_is_too_large() {
    let mut p = Progress::new();
    p.advance(usize::try_from(MAX_DOWNLOAD_BYTES).unwrap(), None);
    assert_that!(p.advance(1, None)).is_equal_to(Advance::TooLarge);
}

#[test]
fn progress_announced_total_over_cap_is_too_large() {
    let mut p = Progress::new();
    assert_that!(p.advance(1, Some(MAX_DOWNLOAD_BYTES + 1))).is_equal_to(Advance::TooLarge);
}

#[rstest]
#[case("https://github.com/oku-theory/oku-slides/releases/download/v2.2.0/Oku.Slides_aarch64.app.tar.gz", true)]
#[case("http://github.com/oku-theory/oku-slides/releases/download/v2.2.0/x", false)]
#[case("https://github.com/someone/else/releases/download/v2.2.0/x", false)]
#[case("https://github.com.evil.test/oku-theory/oku-slides/releases/download/v2.2.0/x", false)]
#[case("https://github.com/oku-theory/oku-slides/releases/download/../../../x", false)]
fn trusted_download_accepts_only_our_release_assets(#[case] url: &str, #[case] expected: bool) {
    assert_that!(trusted_download(url)).is_equal_to(expected);
}

#[rstest]
#[case(UpdateStage::Check, "update-error-check")]
#[case(UpdateStage::Download, "update-error-download")]
#[case(UpdateStage::Install, "update-error-install")]
#[case(UpdateStage::Open, "update-error-open")]
#[case(UpdateStage::Unavailable, "update-error-unavailable")]
#[case(UpdateStage::Refused, "update-error-refused")]
fn update_stage_key_exists_in_english_and_french(#[case] stage: UpdateStage, #[case] key: &str) {
    assert_that!(stage.key()).is_equal_to(key);
    assert_that!(Localizer::new(Some("en")).has(key)).is_true();
    assert_that!(Localizer::new(Some("fr")).has(key)).is_true();
}

#[test]
fn update_stage_localize_speaks_french() {
    let fr = UpdateStage::Check.localize(&Localizer::new(Some("fr")));
    assert_that!(fr).is_not_equal_to(UpdateStage::Check.localize(&Localizer::new(Some("en"))));
}
```

`Localizer::has` checks that locale only (no English fallback), so the French row catches a missing key.

- [ ] **Step 8: Run them to see them fail**

Run: `touch crates/*/src/lib.rs && cargo nextest run -p oku-slides-platform updates`
Expected: compile errors, `cannot find type InstallMode` (and the other names).

- [ ] **Step 9: Write `updates.rs`**

```rust
//! Mises à jour de l'app : ce qui se décide sans Tauri, testé ici.
//!
//! Utilisé par l'hôte Tauri seulement ; vit ici pour que `just test` le couvre.
//! L'hôte (`src-tauri/src/updates.rs`) cherche, télécharge et installe ; ce
//! module dit s'il peut installer ([`InstallMode`]), ce qu'il montre
//! ([`UpdateStatus`], [`UpdateProgress`]), quand signaler l'avancée ou couper
//! ([`Progress`]), d'où il accepte un paquet ([`trusted_download`]) et dans
//! quels mots dire un échec ([`UpdateStage`]).

use std::path::Path;
use std::time::Duration;

use oku_slides_i18n::{Localize, Localizer};
use serde::Serialize;

/// Page des releases, ouverte quand l'app ne peut pas s'installer elle-même.
pub const RELEASES_URL: &str = "https://github.com/oku-theory/oku-slides/releases/latest";
/// Seul préfixe accepté pour télécharger un paquet (`latest.json` n'est pas signé).
pub const DOWNLOAD_PREFIX: &str = "https://github.com/oku-theory/oku-slides/releases/download/";
/// Taille maximale d'un paquet téléchargé, vérifiée avant la signature.
pub const MAX_DOWNLOAD_BYTES: u64 = 512 * 1024 * 1024;
/// Durée maximale d'une recherche de version.
pub const CHECK_TIMEOUT: Duration = Duration::from_secs(30);
/// Fichier (sous le dossier de données) de la dernière version qui n'a pas pu s'installer.
pub const FAILED_VERSION_FILE: &str = "updates/failed-version";

/// Ce que l'app peut faire d'une nouvelle version sur ce système.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallMode {
    /// Télécharger, installer et redémarrer.
    Install,
    /// Seulement prévenir : l'utilisateur installe depuis la page des releases.
    NotifyOnly,
}

/// Ce que l'hôte sait de lui-même pour choisir un [`InstallMode`].
#[derive(Debug, Clone, Copy)]
pub struct Host<'a> {
    /// `std::env::consts::OS`.
    pub os: &'a str,
    /// `APPIMAGE` est posée (Linux : lancé depuis un AppImage).
    pub appimage: bool,
    /// Exécutable courant (`std::env::current_exe`).
    pub exe: &'a Path,
    /// L'installation de cette même version a déjà échoué.
    pub failed_before: bool,
}

impl InstallMode {
    /// Le mode d'après l'hôte : paquet du système, image disque, quarantaine ou échec passé.
    pub fn detect(host: &Host<'_>) -> Self {
        let exe = host.exe.to_string_lossy();
        let unwritable = match host.os {
            "linux" => !host.appimage,
            // Lancée depuis Téléchargements (quarantaine) ou depuis le .dmg monté : le
            // paquet ne peut pas être remplacé.
            "macos" => exe.contains("/AppTranslocation/") || exe.starts_with("/Volumes/"),
            _ => false,
        };
        if unwritable || host.failed_before { Self::NotifyOnly } else { Self::Install }
    }
}

/// Une version plus récente, telle que l'interface la montre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateOffer {
    /// Version proposée, sans `v`.
    pub version: String,
    /// Installer ou seulement prévenir.
    pub mode: InstallMode,
}

/// État partagé par les fenêtres : l'offre gardée, et une installation en cours.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateStatus {
    /// Offre trouvée ; `None` : l'app est à jour (ou rien n'a encore été cherché).
    pub offer: Option<UpdateOffer>,
    /// Un téléchargement ou une installation a commencé.
    pub installing: bool,
}

/// Avancée d'un téléchargement, diffusée aux fenêtres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct UpdateProgress {
    /// Octets reçus.
    pub received: u64,
    /// Taille annoncée ; `None` si inconnue (ou annoncée nulle).
    pub total: Option<u64>,
}

/// Ce que [`Progress::advance`] demande de faire du morceau reçu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Advance {
    /// Signaler cette avancée.
    Report(UpdateProgress),
    /// Rien à signaler.
    Quiet,
    /// Au-delà de [`MAX_DOWNLOAD_BYTES`] : couper le téléchargement.
    TooLarge,
}

/// Octets entre deux signalements quand la taille totale est inconnue.
const UNKNOWN_TOTAL_STEP: u64 = 1024 * 1024;

/// Avancée d'un téléchargement : pas de 1 % (ou de 1 Mio), coupé au-delà du plafond.
#[derive(Debug, Default)]
pub struct Progress {
    received: u64,
    reported: Option<u64>,
}

impl Progress {
    /// Rien de reçu.
    pub fn new() -> Self {
        Self::default()
    }

    /// Ajoute `chunk` octets et dit s'il faut signaler, se taire ou couper.
    pub fn advance(&mut self, chunk: usize, total: Option<u64>) -> Advance {
        let total = total.filter(|&t| t > 0);
        self.received = self.received.saturating_add(u64::try_from(chunk).unwrap_or(u64::MAX));
        if self.received > MAX_DOWNLOAD_BYTES || total.is_some_and(|t| t > MAX_DOWNLOAD_BYTES) {
            return Advance::TooLarge;
        }
        let step = total.map_or(UNKNOWN_TOTAL_STEP, |t| (t / 100).max(1));
        let due = match self.reported {
            None => true,
            Some(last) => {
                let crossed_total = total.is_some_and(|t| self.received >= t && last < t);
                (self.received.saturating_sub(last) >= step && total.is_none_or(|t| last < t)) || crossed_total
            }
        };
        if !due {
            return Advance::Quiet;
        }
        self.reported = Some(self.received);
        Advance::Report(UpdateProgress { received: self.received, total })
    }
}

/// Le paquet vient bien d'une release d'Oku Slides, en HTTPS, sans remonter de dossier.
pub fn trusted_download(url: &str) -> bool {
    url.strip_prefix(DOWNLOAD_PREFIX).is_some_and(|rest| !rest.split('/').any(|part| part == ".."))
}

/// Étape d'une mise à jour qui a échoué : choisit le message montré.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum UpdateStage {
    /// Recherche d'une version (réseau, délai, `latest.json`).
    Check,
    /// Téléchargement, taille, origine ou signature du paquet.
    Download,
    /// Installation du paquet.
    Install,
    /// Ouverture de la page des releases.
    Open,
    /// Aucune version à installer ici (rien de trouvé, ou paquet à installer à la main).
    Unavailable,
    /// Demande venue d'une fenêtre qui n'y a pas droit, ou app en train de quitter.
    Refused,
}

impl UpdateStage {
    /// Clé Fluent du message de cette étape.
    pub fn key(self) -> &'static str {
        match self {
            Self::Check => "update-error-check",
            Self::Download => "update-error-download",
            Self::Install => "update-error-install",
            Self::Open => "update-error-open",
            Self::Unavailable => "update-error-unavailable",
            Self::Refused => "update-error-refused",
        }
    }
}

impl Localize for UpdateStage {
    fn localize(&self, l: &Localizer) -> String {
        l.tr(self.key())
    }
}

#[cfg(test)]
#[path = "updates_tests.rs"]
mod updates_tests;
```

`Option::is_none_or` is stable since Rust 1.82 (workspace is 1.92+).

- [ ] **Step 10: Add the host Fluent keys**

In every `session.ftl`, insert `menu-check-updates` right after `menu-agent-detached` (in `## Menu`,
line ~186 in `en`), and append a new section at the end of the file:

English (`en/session.ftl`):

```ftl
menu-check-updates = Check for Updates…
```

```ftl

## Mises à jour

update-error-check = Could not look for updates. Check your connection and try again.
update-error-download = The update could not be downloaded or verified.
update-error-install = The update could not be installed. Oku Slides restarts on the current version.
update-error-open = The releases page could not be opened.
update-error-unavailable = There is no update to install.
update-error-refused = This window cannot install updates.
```

French (`fr/session.ftl`):

```ftl
menu-check-updates = Rechercher des mises à jour…
```

```ftl

## Mises à jour

update-error-check = Impossible de chercher des mises à jour. Vérifiez votre connexion et réessayez.
update-error-download = La mise à jour n'a pas pu être téléchargée ou vérifiée.
update-error-install = La mise à jour n'a pas pu être installée. Oku Slides redémarre sur la version actuelle.
update-error-open = La page des versions n'a pas pu s'ouvrir.
update-error-unavailable = Aucune mise à jour à installer.
update-error-refused = Cette fenêtre ne peut pas installer de mise à jour.
```

The 18 other locales (`ar bn de es hi it ja ko nl pl pt ru th tr uk vi zh-Hans zh-Hant`): translate
the English, following `crates/oku-slides-i18n/AGENTS.md` (pt Brazilian, keep `…`). Then check no
English was copied:

```bash
for l in ar bn de es hi it ja ko nl pl pt ru th tr uk vi zh-Hans zh-Hant; do
  for k in menu-check-updates update-error-check update-error-download update-error-install update-error-open update-error-unavailable update-error-refused; do
    a=$(grep "^$k = " crates/oku-slides-i18n/src/locales/en/session.ftl); b=$(grep "^$k = " crates/oku-slides-i18n/src/locales/$l/session.ftl)
    [ -n "$b" ] && [ "$a" != "$b" ] || echo "$l $k : absent ou copié"
  done
done
```

Expected: no output.

- [ ] **Step 11: Run the platform and i18n tests**

Run: `touch crates/*/src/lib.rs && cargo nextest run -p oku-slides-platform -p oku-slides-i18n`
Expected: all pass, locale-parity tests included.

- [ ] **Step 12: Docs**

`crates/oku-slides-platform/ARCHITECTURE.md` (48 lines → 50, no merge needed), after the
`config_write` bullet:

```markdown
- `updates`: the app updater's decisions without Tauri (host only, here so `just test` covers it):
  `InstallMode`, `UpdateStatus`, `Progress` (1 % steps, 512 MiB cap), `trusted_download`, `UpdateStage`.
```

`README.md`, Settings table (around line 310), two rows:

```markdown
| `OKU_SLIDES_UPDATES__CHECK_AUTOMATICALLY` | `false`: no automatic update check, only "Check for Updates…" |
| `OKU_SLIDES_UPDATES__CHECK_INTERVAL_HOURS` | hours between automatic checks (default 6) |
```

Match the table's existing column layout. Run `just agents-lint`.

- [ ] **Step 13: Commit**

```bash
git add crates/oku-slides-platform crates/oku-slides-i18n/src/locales README.md
git commit -F "$SCRATCH/msg"   # feat(platform) : réglages et décisions des mises à jour automatiques (t-967)
```

---

### Task 2: The host updater (`src-tauri`)

**Blocked by Task 0 Step 4** (the public key). Without it, stop and report.

**Files:**
- Create: `src-tauri/src/updates.rs`, `src-tauri/src/updates_tests.rs`
- Modify: `Cargo.toml`, `Cargo.lock`, `src-tauri/Cargo.toml`, `deny.toml` and `.cargo/audit.toml` (only if Step 1 finds `zip` < 9)
- Modify: `src-tauri/tauri.conf.json` (`plugins.updater`)
- Modify: `src-tauri/src/state.rs`, `src-tauri/src/main.rs`, `src-tauri/src/menu.rs`
- Modify: `src-tauri/AGENTS.md`

**Interfaces:**
- Consumes (Task 1): every name of Task 1's Produces block; Fluent `menu-check-updates`.
- Produces (for Task 4):
  - commands, all `async`, all refusing the agent window (`update-error-refused`): `update_status() -> Result<UpdateStatus, String>`; `check_update() -> Result<UpdateStatus, String>`; `install_update() -> Result<(), String>`; `open_release_page() -> Result<(), String>`.
  - events: `update-changed` (no payload, to all windows); `update-progress` (`UpdateProgress`, to all windows); `update-check-requested` (no payload, to the focused workspace window).
  - `state::settle(app: &AppHandle)`; `state::shutdown` = `settle` + `app.exit(0)`.

`just test` excludes `oku-slides-app`; this task's checks are `cargo clippy -p oku-slides-app
--all-targets -- -D warnings`, `cargo nextest run -p oku-slides-app` and a manual run.

- [ ] **Step 1: Dependencies**

Before editing, record: `cargo tree --target all -d -e normal | grep -E 'reqwest|rustls|hyper|native-tls|openssl' > "$SCRATCH/tls-before"`.

Root `Cargo.toml`, after `tauri-plugin-single-instance`:

```toml
# Sans `zip` (défaut) : les paquets v2 (AppImage brut, setup.exe) n'en ont pas besoin, et zip 4
# tombe sous RUSTSEC-2026-0367/0368/0369 (ignorés ailleurs parce qu'on n'appelle pas `extract`).
tauri-plugin-updater = { version = "2.14", default-features = false, features = ["rustls-tls", "system-proxy"] }
tauri-plugin-opener = "2.7"
```

`src-tauri/Cargo.toml` `[dependencies]`:

```toml
tauri-plugin-updater.workspace = true
tauri-plugin-opener.workspace = true
snafu.workspace = true
tokio = { workspace = true, features = ["time", "sync", "macros"] }
```

Checks, all required:
- `cargo tree --target all -i zip@4` → no match (zip 4 gone). If it is still pulled, stop and report.
- `cargo tree --target all -d -e normal | grep -E 'reqwest|rustls|hyper|native-tls|openssl' | diff "$SCRATCH/tls-before" -` → no difference. (`deny.toml` allows multiple versions, so deny alone would not catch a new TLS stack.) A difference: stop and report.
- `just audit` → green.

- [ ] **Step 2: Plugin config** — `src-tauri/tauri.conf.json`, top-level after `bundle`:

```json
"plugins": {
  "updater": {
    "pubkey": "<the line from Task 0 Step 4>",
    "endpoints": ["https://github.com/oku-theory/oku-slides/releases/latest/download/latest.json"]
  }
}
```

Then `! grep -q '<the line' src-tauri/tauri.conf.json` must succeed. `bundle.createUpdaterArtifacts`
stays absent (Task 3 sets it in CI). Windows install mode stays the plugin default (`passive`).

- [ ] **Step 3: Split `state::shutdown`** (`src-tauri/src/state.rs`)

First read `Workspace::settle` (`crates/oku-slides-session`) and `Stack::shutdown`
(`crates/oku-slides-agent`): both run twice on every update (here, then on `RunEvent::Exit` after
`request_restart`). Confirm a second call is a no-op; if not, stop and report.

```rust
/// Prépare la fin sans quitter : marque `closing`, garde les fenêtres, arrête
/// les agents et enregistre l'espace de travail.
///
/// Jamais sur le fil principal : `Workspace::settle` attend un verrou qui peut
/// dépendre d'une opération de fenêtre dispatchée sur ce fil. Appelée par
/// `shutdown` et par `updates::install` (avant d'installer) ; rappelée sans
/// danger, `RunEvent::Exit` rappelant aussi `Workspace::settle`.
pub fn settle(app: &AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        state.closing.store(true, Ordering::SeqCst);
        // Les fenêtres existent encore : taille et position gardées sans attendre.
        crate::window_state::save(app);
        // Hors d'un contexte async : `block_on` y est permis.
        tauri::async_runtime::block_on(state.stack.shutdown());
        Workspace::settle(&state.workspace);
    }
}
```

`shutdown` keeps its doc comment (minus the paragraph moved to `settle`) and becomes
`settle(app); app.exit(0);`.

- [ ] **Step 4: Write `src-tauri/src/updates.rs`**

```rust
//! Mises à jour depuis les releases GitHub : recherche, téléchargement, installation.
//!
//! Une tâche cherche au lancement puis à intervalle (`[updates]`) ; l'offre est
//! gardée dans [`Updates`] et chaque changement est signalé (`update-changed`,
//! sans contenu) : les fenêtres relisent `update_status`. Seules les fenêtres de
//! l'espace de travail peuvent chercher, installer ou ouvrir la page des
//! releases. Installer : télécharger (origine vérifiée, taille plafonnée,
//! signature vérifiée par le greffon), puis, dans un même travail bloquant,
//! `state::settle` et l'installation, puis `request_restart` (qui repasse par
//! `RunEvent::Exit`). Sous Windows, l'installeur ferme lui-même le processus.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use oku_slides_platform::config::UpdateSettings;
use oku_slides_platform::errors::one_line;
use oku_slides_platform::updates::{
    Advance, CHECK_TIMEOUT, FAILED_VERSION_FILE, Host, InstallMode, Progress, RELEASES_URL, UpdateOffer, UpdateStage,
    UpdateStatus, trusted_download,
};
use oku_slides_session::{Localize, Localizer, Storage};
use snafu::{Backtrace, ResultExt, Snafu};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::agent_window;
use crate::state::AppState;

/// Échec d'une mise à jour ; le message dépend de l'étape.
#[derive(Debug, Snafu)]
pub struct UpdateError(Inner);

/// Chaque variante a son [`UpdateStage`] (`stage_of`, exhaustif : une variante
/// ajoutée ne compile pas sans son message).
#[derive(Debug, Snafu)]
enum Inner {
    #[snafu(display("update check failed"))]
    Check { source: tauri_plugin_updater::Error, backtrace: Backtrace },
    #[snafu(display("update check timed out"))]
    CheckTimeout { backtrace: Backtrace },
    #[snafu(display("update download failed"))]
    Download { source: tauri_plugin_updater::Error, backtrace: Backtrace },
    #[snafu(display("update download over the size cap"))]
    TooLarge { backtrace: Backtrace },
    #[snafu(display("update download from an untrusted url: {url}"))]
    Untrusted { url: String, backtrace: Backtrace },
    #[snafu(display("releases page not opened"))]
    Open { source: tauri_plugin_opener::Error, backtrace: Backtrace },
    #[snafu(display("no update to install"))]
    Unavailable { backtrace: Backtrace },
    #[snafu(display("update request refused: {why}"))]
    Refused { why: &'static str, backtrace: Backtrace },
}

fn stage_of(inner: &Inner) -> UpdateStage {
    match inner {
        Inner::Check { .. } | Inner::CheckTimeout { .. } => UpdateStage::Check,
        Inner::Download { .. } | Inner::TooLarge { .. } | Inner::Untrusted { .. } => UpdateStage::Download,
        Inner::Open { .. } => UpdateStage::Open,
        Inner::Unavailable { .. } => UpdateStage::Unavailable,
        Inner::Refused { .. } => UpdateStage::Refused,
    }
}

impl Localize for UpdateError {
    fn localize(&self, l: &Localizer) -> String {
        stage_of(&self.0).localize(l)
    }
}

/// Offre trouvée et installation en cours, partagées par les fenêtres.
pub struct Updates {
    found: Mutex<Option<(Update, UpdateOffer)>>,
    installing: AtomicBool,
    /// Pour lire et écrire [`FAILED_VERSION_FILE`] (hors contexte async seulement).
    storage: Storage,
    failed_file: PathBuf,
}

impl std::fmt::Debug for Updates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Updates").finish_non_exhaustive()
    }
}

impl Updates {
    /// État vide ; `data` : dossier de données de l'app.
    pub fn new(storage: Storage, data: &Path) -> Self {
        Self { found: Mutex::new(None), installing: AtomicBool::new(false), storage, failed_file: data.join(FAILED_VERSION_FILE) }
    }

    fn status(&self) -> UpdateStatus {
        let offer = self.found.lock().unwrap_or_else(PoisonError::into_inner).as_ref().map(|(_, o)| o.clone());
        UpdateStatus { offer, installing: self.installing.load(Ordering::SeqCst) }
    }

    /// Version dont l'installation a échoué (absente : aucune) ; travail bloquant.
    fn failed_version(&self) -> Option<String> {
        match self.storage.read_limited(&self.failed_file, 64) {
            Ok(bytes) => String::from_utf8(bytes).ok().map(|v| v.trim().to_owned()),
            Err(e) => {
                tracing::debug!(error = %one_line(&e), "aucune version en échec lue");
                None
            }
        }
    }
}

fn changed(app: &AppHandle) {
    if let Err(e) = app.emit("update-changed", ()) {
        tracing::warn!(error = %e, "changement de mise à jour non diffusé");
    }
}

/// Refuse la fenêtre de l'agent : elle affiche du contenu venu de l'agent et des imports.
fn workspace_window(window: &WebviewWindow) -> Result<(), UpdateError> {
    if window.label() == agent_window::LABEL {
        return Err(RefusedSnafu { why: "agent window" }.build().into());
    }
    Ok(())
}

fn closing(app: &AppHandle) -> bool {
    app.try_state::<AppState>().is_some_and(|s| s.closing.load(Ordering::SeqCst))
}

/// Cherche une version ; garde le résultat (même « à jour ») et le signale.
async fn check(app: &AppHandle) -> Result<UpdateStatus, UpdateError> {
    let Some(updates) = app.try_state::<Updates>() else { return Err(RefusedSnafu { why: "not ready" }.build().into()) };
    let updater = app.updater().context(CheckSnafu)?;
    let found = tokio::time::timeout(CHECK_TIMEOUT, updater.check())
        .await
        .map_err(|_| UpdateError::from(CheckTimeoutSnafu.build()))?
        .context(CheckSnafu)?;
    if updates.installing.load(Ordering::SeqCst) {
        return Ok(updates.status());
    }
    let offer = match &found {
        Some(update) => {
            // Exécutable introuvable : chemin vide, donc ni quarantaine ni image disque supposées.
            let exe = std::env::current_exe().unwrap_or_else(|e| {
                tracing::warn!(error = %e, "exécutable courant inconnu");
                PathBuf::new()
            });
            let failed_before = {
                let app = app.clone();
                let version = update.version.clone();
                match tauri::async_runtime::spawn_blocking(move || {
                    app.try_state::<Updates>().and_then(|u| u.failed_version()).is_some_and(|v| v == version)
                })
                .await
                {
                    Ok(failed) => failed,
                    Err(e) => {
                        tracing::warn!(error = %e, "version en échec non lue");
                        false
                    }
                }
            };
            let host = Host { os: std::env::consts::OS, appimage: std::env::var_os("APPIMAGE").is_some(), exe: &exe, failed_before };
            Some(UpdateOffer { version: update.version.clone(), mode: InstallMode::detect(&host) })
        }
        None => None,
    };
    *updates.found.lock().unwrap_or_else(PoisonError::into_inner) = found.zip(offer);
    changed(app);
    Ok(updates.status())
}

/// Recherche automatique : au lancement, puis à chaque intervalle ; un échec est seulement journalisé.
///
/// L'horloge de `tokio::time::sleep` est monotone : un Mac en veille repousse la
/// recherche suivante d'autant (accepté, t-976).
pub fn spawn_checks(app: &AppHandle, settings: &UpdateSettings) {
    if !settings.auto_check(cfg!(debug_assertions)) {
        tracing::debug!("recherche automatique des mises à jour désactivée");
        return;
    }
    let app = app.clone();
    let interval = settings.interval();
    tauri::async_runtime::spawn(async move {
        while !closing(&app) {
            match check(&app).await {
                Ok(UpdateStatus { offer: Some(offer), .. }) => tracing::info!(version = %offer.version, "mise à jour disponible"),
                Ok(_) => tracing::debug!("aucune mise à jour"),
                Err(e) => tracing::warn!(error = %one_line(&e), "recherche de mise à jour"),
            }
            tokio::time::sleep(interval).await;
        }
    });
}

fn refuse(state: &AppState, e: &UpdateError) -> String {
    tracing::warn!(error = %one_line(e), "mise à jour refusée");
    e.localize(&state.localizer)
}

/// L'état gardé, pour une fenêtre ouverte ou prévenue après coup.
#[tauri::command]
pub async fn update_status(window: WebviewWindow, state: State<'_, AppState>, updates: State<'_, Updates>) -> Result<UpdateStatus, String> {
    workspace_window(&window).map_err(|e| refuse(&state, &e))?;
    Ok(updates.status())
}

/// Recherche demandée (menu) ; `offer` absente : l'app est à jour.
#[tauri::command]
pub async fn check_update(app: AppHandle, window: WebviewWindow, state: State<'_, AppState>) -> Result<UpdateStatus, String> {
    workspace_window(&window).map_err(|e| refuse(&state, &e))?;
    check(&app).await.map_err(|e| refuse(&state, &e))
}

/// Télécharge, puis enregistre l'espace de travail, installe et redémarre.
#[tauri::command]
pub async fn install_update(app: AppHandle, window: WebviewWindow, state: State<'_, AppState>) -> Result<(), String> {
    workspace_window(&window).map_err(|e| refuse(&state, &e))?;
    install(&app).await.map_err(|e| refuse(&state, &e))
}

async fn install(app: &AppHandle) -> Result<(), UpdateError> {
    let Some(updates) = app.try_state::<Updates>() else { return Err(RefusedSnafu { why: "not ready" }.build().into()) };
    if closing(app) {
        return Err(RefusedSnafu { why: "app closing" }.build().into());
    }
    let found = updates.found.lock().unwrap_or_else(PoisonError::into_inner).clone();
    let Some((update, offer)) = found.filter(|(_, o)| o.mode == InstallMode::Install) else {
        return Err(UnavailableSnafu.build().into());
    };
    if !trusted_download(update.download_url.as_str()) {
        return Err(UntrustedSnafu { url: update.download_url.to_string() }.build().into());
    }
    // Une autre fenêtre installe déjà : rien à refaire, mais elle le sait (`update-changed`).
    if updates.installing.swap(true, Ordering::SeqCst) {
        return Err(RefusedSnafu { why: "already installing" }.build().into());
    }
    changed(app);
    let bytes = match download(app, &update).await {
        Ok(bytes) => bytes,
        Err(e) => {
            updates.installing.store(false, Ordering::SeqCst);
            changed(app);
            return Err(e);
        }
    };
    if closing(app) {
        // L'utilisateur a quitté pendant le téléchargement : la fin suit son cours.
        return Err(RefusedSnafu { why: "app closing" }.build().into());
    }
    tracing::info!(version = %offer.version, "mise à jour téléchargée : enregistrement puis installation");
    let job = app.clone();
    let version = offer.version.clone();
    let done = tauri::async_runtime::spawn_blocking(move || {
        crate::state::settle(&job);
        let installed = update.install(bytes);
        if let Err(e) = &installed
            && let Some(updates) = job.try_state::<Updates>()
            && let Err(w) = updates.storage.write_atomic(&updates.failed_file, version.into_bytes())
        {
            tracing::error!(error = %e, write = %one_line(&w), "version en échec non gardée");
        }
        installed
    })
    .await;
    match done {
        Ok(Ok(())) => tracing::info!("mise à jour installée : redémarrage"),
        Ok(Err(e)) => tracing::error!(error = %e, "installation de la mise à jour : redémarrage sur la version actuelle"),
        // `settle` a paniqué : rien n'a été installé ; `RunEvent::Exit` rappelle `Workspace::settle`.
        Err(e) => tracing::error!(error = %e, "enregistrement avant mise à jour"),
    }
    app.request_restart();
    Ok(())
}

/// Télécharge le paquet ; coupe au-delà du plafond (avant la vérification de la signature).
async fn download(app: &AppHandle, update: &Update) -> Result<Vec<u8>, UpdateError> {
    let too_large = Arc::new(tokio::sync::Notify::new());
    let over = Arc::new(AtomicBool::new(false));
    let mut progress = Progress::new();
    let emitter = app.clone();
    let (notify, flag) = (Arc::clone(&too_large), Arc::clone(&over));
    let fetch = update.download(
        move |chunk, total| match progress.advance(chunk, total) {
            Advance::Report(p) => {
                if let Err(e) = emitter.emit("update-progress", p) {
                    tracing::debug!(error = %e, "avancée non diffusée");
                }
            }
            Advance::Quiet => {}
            Advance::TooLarge => {
                if !flag.swap(true, Ordering::SeqCst) {
                    notify.notify_one();
                }
            }
        },
        || {},
    );
    tokio::select! {
        bytes = fetch => bytes.context(DownloadSnafu).map_err(UpdateError::from),
        () = too_large.notified() => Err(TooLargeSnafu.build().into()),
    }
}

/// Ouvre la page des releases (paquet à installer à la main).
#[tauri::command]
pub async fn open_release_page(app: AppHandle, window: WebviewWindow, state: State<'_, AppState>) -> Result<(), String> {
    workspace_window(&window).map_err(|e| refuse(&state, &e))?;
    app.opener()
        .open_url(RELEASES_URL, None::<&str>)
        .context(OpenSnafu)
        .map_err(|e| refuse(&state, &UpdateError::from(e)))
}

#[cfg(test)]
#[path = "updates_tests.rs"]
mod updates_tests;
```

Notes for the implementer (facts, checked in the registry sources):
- tauri-plugin-updater 2.14: `Update` is `Clone`, with `pub version: String` and `pub download_url: Url`; `download(FnMut(usize, Option<u64>), FnOnce()) -> Result<Vec<u8>>` checks the signature; `install(impl AsRef<[u8]>)` is synchronous.
- tauri 2.12: `AppHandle::request_restart()` goes through `RunEvent::Exit`; never use `restart()` (it parks the calling thread).
- `Notify::notify_one` stores a permit, so a cap hit before `select!` polls `notified()` is not lost.
- `Storage` is re-exported by `oku_slides_session` (as `main.rs` uses it); `read_limited`/`write_atomic` are synchronous: only call them inside `spawn_blocking`, as above.
- `#[derive(Snafu)]` on the tuple struct gives `From<Inner>`; no `visibility` attribute (selectors stay in this module).
- If clippy rejects the `if let … && let …` chain on the edition in use, split it into nested `if let`.

- [ ] **Step 5: Test the stage mapping** (`src-tauri/src/updates_tests.rs`)

```rust
use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
#[case(CheckTimeoutSnafu.build(), UpdateStage::Check)]
#[case(TooLargeSnafu.build(), UpdateStage::Download)]
#[case(UntrustedSnafu { url: "x".to_owned() }.build(), UpdateStage::Download)]
#[case(UnavailableSnafu.build(), UpdateStage::Unavailable)]
#[case(RefusedSnafu { why: "agent window" }.build(), UpdateStage::Refused)]
fn stage_of_error_names_its_message(#[case] inner: Inner, #[case] expected: UpdateStage) {
    assert_that!(stage_of(&inner)).is_equal_to(expected);
}
```

Add `rstest.workspace = true` to `src-tauri/Cargo.toml` `[dev-dependencies]`. Variants with a
plugin `source` are covered by the exhaustive `match`.

- [ ] **Step 6: Wire it in `main.rs`**

- `mod updates;` in the module list.
- Module doc: replace "Cinq chemins de sortie s'y retrouvent : … la vue vide (`windows::sync`)." by
  adding a sixth path: "… la vue vide (`windows::sync`) et la mise à jour (`updates::install`, par
  `state::settle` puis `request_restart`)." and change "Un sixième, `RunEvent::Exit`" to "Un septième,
  `RunEvent::Exit`". Add one sentence: "`updates` cherche les nouvelles versions et les installe."
- Plugins, after `tauri_plugin_window_state`: `.plugin(tauri_plugin_updater::Builder::new().build())`
  and `.plugin(tauri_plugin_opener::init())`. No capability file changes.
- In `setup`, after `app.manage(window_state::Saver::new(...))`:

```rust
app.manage(updates::Updates::new(platform.storage.clone(), &platform.settings.paths.data()));
updates::spawn_checks(app.handle(), &platform.settings.updates);
```

- In `on_menu_event`:

```rust
if event.id() == menu::CHECK_UPDATES_ID {
    let focused = app.try_state::<AppState>().and_then(|s| s.try_workspace().map(|ws| ws.focused().to_owned()));
    match focused {
        Some(label) => {
            if let Err(e) = app.emit_to(label.as_str(), "update-check-requested", ()) {
                tracing::warn!(error = %e, "recherche de mise à jour non transmise");
            }
        }
        None => tracing::debug!("recherche de mise à jour : espace de travail occupé, clic ignoré"),
    }
}
```

- `generate_handler!`: add `updates::update_status, updates::check_update, updates::install_update, updates::open_release_page`.

- [ ] **Step 7: macOS menu entry** (`menu.rs`)

```rust
/// Identifiant de « Rechercher des mises à jour… ».
pub const CHECK_UPDATES_ID: &str = "check-updates";
```

In `build`: `let check_updates = MenuItem::with_id(app, CHECK_UPDATES_ID, l.tr("menu-check-updates"), true, None::<&str>)?;`
and `&check_updates,` right after `&about` in the app submenu. Doc of `build`: "Le menu :
application (à propos, mises à jour, masquer, quitter), édition, fenêtre."

- [ ] **Step 8: Rules** — `src-tauri/AGENTS.md`:

`## Invariants`, last lines:

```markdown
- Install an update only after state::settle in the same blocking job, then request_restart; never restart from the webview. (spec:2026-10-18-auto-update-design)
- Grant no updater or opener permission in any capability, and refuse update commands from the agent window. (spec:2026-10-18-auto-update-design)
```

`## Pitfalls`, last line:

```markdown
- The updater installs ring as the global rustls provider on first check; keep every rustls user on ring. (spec:2026-10-18-auto-update-design)
```

- [ ] **Step 9: Build, lint, tests**

Run: `touch crates/*/src/lib.rs && cargo clippy -p oku-slides-app --all-targets -- -D warnings && cargo nextest run -p oku-slides-app && just agents-lint`
Expected: no warnings, all pass.

- [ ] **Step 10: Manual run (debug)**

`OKU_SLIDES_PATHS__DATA=$(mktemp -d) OKU_SLIDES_PATHS__DECKS=$(mktemp -d) just dev`:
- logs show `recherche automatique des mises à jour désactivée`;
- on macOS the app menu shows "Rechercher des mises à jour…" under About;
- ⌘Q, relaunch with the same dirs: tabs restore as before (the `shutdown` split changed nothing).

- [ ] **Step 11: Commit**

```bash
git add Cargo.toml Cargo.lock src-tauri
git commit -F "$SCRATCH/msg"   # feat(app) : mises à jour depuis les releases GitHub, pilotées par l'hôte (t-967)
```

---

### Task 3: Signed updater bundles and a gated `latest.json` in `release.yml`

**Files:**
- Modify: `.github/workflows/release.yml`

**Interfaces:**
- Consumes: the `release` environment and its secrets (Task 0); `plugins.updater` (Task 2).
- Produces: a published release whose `latest.json` has `darwin-aarch64`, `darwin-x86_64`, `linux-x86_64`, `windows-x86_64`, each with `url` and `signature`; a partial release stays a draft (releases are immutable once published).

- [ ] **Step 1: Pin every action to a commit SHA**

For each `uses:` in the workflow (`actions/checkout@v7`, `dtolnay/rust-toolchain@1.98.1`,
`Swatinem/rust-cache@v2`, `taiki-e/install-action@v2`, `oven-sh/setup-bun@v2`,
`tauri-apps/tauri-action@v0`), resolve the SHA:

```bash
gh api repos/tauri-apps/tauri-action/commits/v0 --jq .sha
```

and write `uses: tauri-apps/tauri-action@<sha> # v0`. The build job holds the signing key: no
mutable tag may run there.

- [ ] **Step 2: Matrix updater keys** — in the `targets` step's `jq`, give each entry its
  `latest.json` key: `updater: "darwin-aarch64"` (macOS Apple Silicon), `"darwin-x86_64"`
  (Intel), `"linux-x86_64"`, `"windows-x86_64"`. Add a job output `updaters`:
  `jq -c '[.include[].updater]'` of the matrix.

- [ ] **Step 3: Build job** — `environment: release` on the `build` job (environment secrets only
  reach jobs that name it). The `tauri-action` step:

```yaml
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          # Signature des paquets de mise à jour (minisign, t-967) : secrets de l'environnement
          # `release`, réservé à main. Ce pas exécute aussi le build (npm, build.rs) : d'où les
          # actions figées par SHA ; signer dans un job séparé est t-977.
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
          APPLE_SIGNING_IDENTITY: "-"
        with:
          tauriScript: cargo tauri
          # Paquets de mise à jour ici seulement : `just build` reste sans clé.
          args: --config '{"bundle":{"createUpdaterArtifacts":true}}'
          # `latest.json` (lu par l'app) : chaque plateforme y ajoute la sienne.
          includeUpdaterJson: true
          # Windows : l'installeur NSIS, pas le MSI.
          updaterJsonPreferNsis: true
          # … tagName, releaseName, releaseBody, releaseDraft, prerelease unchanged …
```

Check the input names against the pinned `action.yml`:
`gh api "repos/tauri-apps/tauri-action/contents/action.yml?ref=<sha>" --jq .content | base64 -d | grep -E '^  (args|includeUpdaterJson|updaterJsonPreferNsis):'`
(three lines expected; a missing name: stop and report).

- [ ] **Step 4: Publish gate** — the `publish` job, before `gh release edit`:

```yaml
      # Chaque plateforme construite doit figurer, signée, dans `latest.json`. Une release partielle
      # reste un brouillon : publiée, elle serait immuable et ne pourrait plus être complétée.
      - name: latest.json vérifié
        id: gate
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          TAG: v${{ needs.version.outputs.version }}
          WANT: ${{ needs.version.outputs.updaters }}
        run: |
          gh release download "$TAG" --repo "$GITHUB_REPOSITORY" -p latest.json -O latest.json
          jq -e --argjson want "$WANT" '
            . as $r | all($want[]; . as $k | $r.platforms[$k] | (.url // "") != "" and (.signature // "") != "")
          ' latest.json > /dev/null || { echo "::error::latest.json incomplet"; jq . latest.json; exit 1; }
          # Les quatre plateformes, celles des runs précédents comprises, chacune avec URL et signature.
          n=$(jq '[.platforms | to_entries[]
                   | select(.key | IN("darwin-aarch64","darwin-x86_64","linux-x86_64","windows-x86_64"))
                   | select((.value.url // "") != "" and (.value.signature // "") != "")] | length' latest.json)
          if [ "$n" -eq 4 ]; then echo "complete=true" >> "$GITHUB_OUTPUT"; else
            echo "::warning::$n plateformes sur 4 : $TAG reste un brouillon, à compléter par une release « aucune »"
            echo "complete=false" >> "$GITHUB_OUTPUT"; fi
```

and the publish step gets `if: steps.gate.outputs.complete == 'true'` and becomes
`gh release edit "$TAG" --repo "$GITHUB_REPOSITORY" --draft=false --latest --target "$SHA"`.
The `aucune` branch of the version job fails early when `v<version>` is already published
(`gh release view … --json isDraft`): its builds could not upload to an immutable release. Only
"release not found" lets it through; any other gh error stops the run.

- [ ] **Step 5: Header comment** — add to the workflow's header: the release also carries signed
  updater bundles and `latest.json` (read by installed apps once published as latest); the
  signing secrets live in the `release` environment; local `just build` makes no updater bundle
  and needs no key.

- [ ] **Step 6: Lint** — `actionlint .github/workflows/release.yml` (install with
  `brew install actionlint` if missing). Expected: no error.

- [ ] **Step 7: Commit**

```bash
git add .github/workflows/release.yml
git commit -F "$SCRATCH/msg"   # ci(release) : paquets de mise à jour signés et latest.json vérifié (t-967)
```

---

### Task 4: UI — `UpdateBar`, `useUpdate`, `useUpdateCheck`, menu entry

**Files:**
- Create: `ui/src/updateApi.ts` (host calls and events, Tauri only)
- Create: `ui/src/useUpdate.ts`, `ui/src/useUpdate.render.test.tsx`
- Create: `ui/src/useUpdateCheck.ts`, `ui/src/useUpdateCheck.render.test.tsx`
- Create: `ui/src/UpdateBar.tsx`, `ui/src/UpdateBar.render.test.tsx`
- Modify: `ui/src/api.ts` (named re-exports, header), `ui/src/Workspace.tsx`, `ui/src/App.tsx`, `ui/src/styles.css`
- Modify: `crates/oku-slides-i18n/src/locales/<all 20>/ui.ftl`

**Interfaces:**
- Consumes (Task 2): the four commands and three events, payload types mirroring `UpdateStatus` / `UpdateProgress`.
- Produces:
  - `updateApi.ts`: `type InstallMode = "install" | "notify_only"`; `interface UpdateOffer { version: string; mode: InstallMode }`; `interface UpdateStatus { offer: UpdateOffer | null; installing: boolean }`; `interface UpdateProgress { received: number; total: number | null }`; `interface UpdateEvents { onChanged(): void; onProgress(p: UpdateProgress): void; onCheckRequested(): void }`; `interface UpdateApi { status(): Promise<UpdateStatus>; check(): Promise<UpdateStatus>; install(): Promise<void>; openRelease(): Promise<void>; subscribe(h: UpdateEvents): Promise<() => void> }`; `const tauriUpdates: UpdateApi | null`. All re-exported by name from `api.ts`.
  - `useUpdate(api: UpdateApi | null, onError: (text: string) => void): UpdateState` with `interface UpdateState { offer: UpdateOffer | null; installing: boolean; percent: number | null; requested: number; check(): Promise<UpdateStatus | null>; install(): void; later(): void; openRelease(): void }` (`check` resolves `null` without host; rejects on host error).
  - `useUpdateCheck(updates: AppUpdates | undefined, say: (text: string, error?: boolean) => void): (() => void) | undefined` with `interface AppUpdates { check(): Promise<UpdateStatus | null>; requested: number }`.
  - `UpdateBar({ offer, installing, percent, onInstall, onLater, onOpenRelease })`.
  - `App` prop `updates?: AppUpdates`.

- [ ] **Step 1: UI Fluent keys** (end of every `ui.ftl`, this order)

English:

```ftl
ui-update-available = Oku Slides { $version } is available.
ui-update-install = Install and Restart
ui-update-see-release = See the Release
ui-update-later = Later
ui-update-installing = Downloading the update… { $percent }%
ui-update-downloading = Downloading the update…
ui-update-up-to-date = Oku Slides is up to date.
ui-menu-check-updates = Check for Updates…
```

French:

```ftl
ui-update-available = Oku Slides { $version } est disponible.
ui-update-install = Installer et redémarrer
ui-update-see-release = Voir la version
ui-update-later = Plus tard
ui-update-installing = Téléchargement de la mise à jour… { $percent } %
ui-update-downloading = Téléchargement de la mise à jour…
ui-update-up-to-date = Oku Slides est à jour.
ui-menu-check-updates = Rechercher des mises à jour…
```

The 18 others translated from the English (rules of Task 1 Step 10; in ja, ko, zh write
`{ $percent }%` with no space). Run the copy check of Task 1 Step 10 with these keys and `ui.ftl`,
then `touch crates/*/src/lib.rs && cargo nextest run -p oku-slides-i18n`.

- [ ] **Step 2: Write `ui/src/updateApi.ts`**

```ts
// Mises à jour de l'app : commandes et événements de l'hôte Tauri (absents en développement web).

import { inTauri, listenTauri } from "./hostCall";

/** Installer, ou seulement prévenir (paquet à installer à la main). */
export type InstallMode = "install" | "notify_only";

/** Une version plus récente, telle que l'hôte la décrit. */
export interface UpdateOffer {
  version: string;
  mode: InstallMode;
}

/** État gardé par l'hôte, le même pour toutes les fenêtres. */
export interface UpdateStatus {
  offer: UpdateOffer | null;
  installing: boolean;
}

/** Avancée du téléchargement ; `total` inconnu : `null`. */
export interface UpdateProgress {
  received: number;
  total: number | null;
}

/** Réactions d'une fenêtre aux événements de mise à jour de l'hôte. */
export interface UpdateEvents {
  /** L'état a changé : le relire (`status`), jamais le deviner. */
  onChanged(): void;
  onProgress(p: UpdateProgress): void;
  /** Le menu de l'app demande une recherche (macOS). */
  onCheckRequested(): void;
}

/** Ce que l'interface demande à l'hôte ; remplacé par un faux dans les tests. */
export interface UpdateApi {
  status(): Promise<UpdateStatus>;
  check(): Promise<UpdateStatus>;
  install(): Promise<void>;
  openRelease(): Promise<void>;
  /** Abonne `h` ; la promesse se résout une fois tous les écouteurs posés. */
  subscribe(h: UpdateEvents): Promise<() => void>;
}

async function call<T>(command: string): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(command);
}

/** Un événement de l'hôte diffusé à toutes les fenêtres ; résolu une fois l'écouteur posé. */
async function on<P>(name: string, cb: (payload: P) => void): Promise<() => void> {
  const { listen } = await import("@tauri-apps/api/event");
  return listen<P>(name, (e) => cb(e.payload));
}

/** Un nombre fini et positif, sinon `null` : les événements ne sont pas des données sûres. */
function count(v: unknown): number | null {
  return typeof v === "number" && Number.isFinite(v) && v >= 0 ? v : null;
}

/** L'hôte Tauri ; `null` hors de l'app (serveur de développement, e2e). */
export const tauriUpdates: UpdateApi | null = inTauri
  ? {
      status: () => call<UpdateStatus>("update_status"),
      check: () => call<UpdateStatus>("check_update"),
      install: () => call<void>("install_update"),
      openRelease: () => call<void>("open_release_page"),
      subscribe: async (h) => {
        const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
        const stops = await Promise.all([
          on<unknown>("update-changed", () => h.onChanged()),
          on<{ received?: unknown; total?: unknown } | null>("update-progress", (p) => {
            const received = count(p?.received);
            if (received !== null) h.onProgress({ received, total: count(p?.total) || null });
          }),
          getCurrentWebviewWindow().listen("update-check-requested", () => h.onCheckRequested()),
        ]);
        return () => stops.forEach((stop) => stop());
      },
    }
  : null;

/** Abonnement suivant la vie d'un effet (StrictMode compris), puis `ready` une fois posé. */
export function subscribeUpdates(api: UpdateApi, h: UpdateEvents, ready: () => void): () => void {
  return listenTauri(async () => {
    const stop = await api.subscribe(h);
    ready();
    return stop;
  });
}
```

`api.ts`: next to the other host re-exports, add
`export { subscribeUpdates, tauriUpdates } from "./updateApi";` and
`export type { InstallMode, UpdateApi, UpdateEvents, UpdateOffer, UpdateProgress, UpdateStatus } from "./updateApi";`,
and name `updateApi.ts` in the header comment's list of host files. Every other file imports these
names from `./api`.

- [ ] **Step 3: Write the failing hook tests** (`ui/src/useUpdate.render.test.tsx`)

```tsx
// Mise à jour : état relu à l'abonnement et à chaque signal, « Plus tard », installation.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { UpdateApi, UpdateEvents, UpdateOffer, UpdateStatus } from "./api";

const { act, createElement, StrictMode } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useUpdate } = await import("./useUpdate");

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const offer: UpdateOffer = { version: "2.2.0", mode: "install" };
const status = (o: Partial<UpdateStatus> = {}): UpdateStatus => ({ offer: null, installing: false, ...o });

/** Un hôte factice : `state` est ce que `status()` rend, `fire` déclenche ses événements. */
function fakeApi(o: { check?: () => Promise<UpdateStatus>; install?: () => Promise<void>; openRelease?: () => Promise<void> } = {}) {
  const calls: string[] = [];
  const live = new Set<UpdateEvents>();
  const fake = {
    state: status(),
    calls,
    live,
    api: {
      status: async () => (calls.push("status"), fake.state),
      check: o.check ?? (async () => fake.state),
      install: o.install ?? (async () => void calls.push("install")),
      openRelease: o.openRelease ?? (async () => void calls.push("open")),
      subscribe: async (h: UpdateEvents) => {
        calls.push("subscribe");
        live.add(h);
        return () => live.delete(h);
      },
    } satisfies UpdateApi,
    changed: () => live.forEach((h) => h.onChanged()),
    progress: (received: number, total: number | null) => live.forEach((h) => h.onProgress({ received, total })),
    requested: () => live.forEach((h) => h.onCheckRequested()),
  };
  return fake;
}

const errors: string[] = [];
const onError = (text: string) => void errors.push(text);

async function mount(api: UpdateApi | null, strict = false) {
  errors.length = 0;
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const out: { hook: ReturnType<typeof useUpdate> | null } = { hook: null };
  function Probe() {
    out.hook = useUpdate(api, onError);
    return null;
  }
  const tree = strict ? createElement(StrictMode, null, createElement(Probe)) : createElement(Probe);
  await act(async () => root.render(tree));
  return { hook: () => out.hook! };
}

describe("useUpdate", () => {
  test("useUpdate without host does nothing", async () => {
    const m = await mount(null);
    await act(async () => {
      expect(await m.hook().check()).toBeNull();
      m.hook().install();
      m.hook().later();
      m.hook().openRelease();
    });
    expect([m.hook().offer, errors]).toEqual([null, []]);
  });

  test("useUpdate reads the status once subscribed", async () => {
    const f = fakeApi();
    f.state = status({ offer });
    const m = await mount(f.api);
    expect([m.hook().offer, f.calls]).toEqual([offer, ["subscribe", "status"]]);
  });

  test("useUpdate rerender keeps a single subscription", async () => {
    const f = fakeApi();
    const m = await mount(f.api);
    await act(async () => f.progress(10, 100));
    expect([f.calls.filter((c) => c === "subscribe").length, f.live.size, m.hook().percent]).toEqual([1, 1, 10]);
  });

  test("useUpdate strict mode ends with one live handler", async () => {
    const f = fakeApi();
    await mount(f.api, true);
    expect(f.live.size).toBe(1);
  });

  test("useUpdate unmount unsubscribes", async () => {
    const f = fakeApi();
    await mount(f.api);
    await act(async () => mounted!.root.unmount());
    mounted = null;
    expect(f.live.size).toBe(0);
  });

  test("useUpdate changed signal rereads the status", async () => {
    const f = fakeApi();
    const m = await mount(f.api);
    f.state = status({ offer });
    await act(async () => f.changed());
    expect(m.hook().offer).toEqual(offer);
  });

  test("useUpdate keeps only the latest status reply", async () => {
    const f = fakeApi();
    let first: (s: UpdateStatus) => void = () => {};
    let calls = 0;
    f.api.status = () => (++calls === 1 ? new Promise((r) => (first = r)) : Promise.resolve(status({ offer })));
    const m = await mount(f.api);
    await act(async () => f.changed());
    await act(async () => first(status()));
    expect(m.hook().offer).toEqual(offer);
  });

  test("useUpdate later hides this version", async () => {
    const f = fakeApi();
    f.state = status({ offer });
    const m = await mount(f.api);
    await act(async () => m.hook().later());
    await act(async () => f.changed());
    expect(m.hook().offer).toBeNull();
  });

  test("useUpdate later still shows a newer version", async () => {
    const f = fakeApi();
    f.state = status({ offer });
    const m = await mount(f.api);
    await act(async () => m.hook().later());
    f.state = status({ offer: { ...offer, version: "2.3.0" } });
    await act(async () => f.changed());
    expect(m.hook().offer?.version).toBe("2.3.0");
  });

  test("useUpdate installing shows the offer even after later", async () => {
    const f = fakeApi();
    f.state = status({ offer });
    const m = await mount(f.api);
    await act(async () => m.hook().later());
    f.state = status({ offer, installing: true });
    await act(async () => f.changed());
    expect([m.hook().offer, m.hook().installing]).toEqual([offer, true]);
  });

  test("useUpdate install ending rereads installing false", async () => {
    const f = fakeApi();
    f.state = status({ offer, installing: true });
    const m = await mount(f.api);
    f.state = status({ offer });
    await act(async () => f.changed());
    expect(m.hook().installing).toBe(false);
  });

  test("useUpdate progress gives a percent", async () => {
    const f = fakeApi();
    const m = await mount(f.api);
    await act(async () => f.progress(250, 1000));
    expect(m.hook().percent).toBe(25);
  });

  test("useUpdate progress of unknown size has no percent", async () => {
    const f = fakeApi();
    const m = await mount(f.api);
    await act(async () => f.progress(250, null));
    expect(m.hook().percent).toBeNull();
  });

  test("useUpdate install calls the host", async () => {
    const f = fakeApi();
    f.state = status({ offer });
    const m = await mount(f.api);
    await act(async () => m.hook().install());
    expect(f.calls.at(-1)).toBe("install");
  });

  test("useUpdate install failure reports it and stops installing", async () => {
    const f = fakeApi({ install: async () => { throw new Error("réseau"); } });
    f.state = status({ offer });
    const m = await mount(f.api);
    await act(async () => m.hook().install());
    expect([m.hook().installing, errors]).toEqual([false, ["réseau"]]);
  });

  test("useUpdate open release calls the host", async () => {
    const f = fakeApi();
    const m = await mount(f.api);
    await act(async () => m.hook().openRelease());
    expect(f.calls.at(-1)).toBe("open");
  });

  test("useUpdate open release failure reports it", async () => {
    const f = fakeApi({ openRelease: async () => { throw new Error("refusé"); } });
    const m = await mount(f.api);
    await act(async () => m.hook().openRelease());
    expect(errors).toEqual(["refusé"]);
  });

  test("useUpdate check after later shows the offer again", async () => {
    const f = fakeApi({ check: async () => status({ offer }) });
    f.state = status({ offer });
    const m = await mount(f.api);
    await act(async () => m.hook().later());
    await act(async () => void (await m.hook().check()));
    expect(m.hook().offer).toEqual(offer);
  });

  test("useUpdate check-requested event counts a request", async () => {
    const f = fakeApi();
    const m = await mount(f.api);
    await act(async () => f.requested());
    expect(m.hook().requested).toBe(1);
  });
});
```

`onError` is defined once at module level, like a stable `useToast().say`; the single-subscription
test would fail if the effect depended on it.

- [ ] **Step 4: Run them to see them fail**

Run: `cd ui && bun test src/useUpdate.render.test.tsx`
Expected: FAIL, cannot find module `./useUpdate`.

- [ ] **Step 5: Write `ui/src/useUpdate.ts`**

```ts
// Mise à jour de l'app dans une fenêtre : état de l'hôte, « Plus tard », installation.

import { useCallback, useEffect, useRef, useState } from "react";
import { errorText, subscribeUpdates, type UpdateApi, type UpdateOffer, type UpdateStatus } from "./api";

/** Ce qu'une fenêtre montre et peut faire d'une mise à jour. */
export interface UpdateState {
  /** Offre à montrer (masquée par « Plus tard », sauf pendant l'installation). */
  offer: UpdateOffer | null;
  installing: boolean;
  /** Avancée du téléchargement, `null` si inconnue. */
  percent: number | null;
  /** Recherches demandées par le menu de l'app, comptées. */
  requested: number;
  /** Recherche ; `null` sans hôte, rejet sur une erreur de l'hôte. */
  check(): Promise<UpdateStatus | null>;
  install(): void;
  later(): void;
  openRelease(): void;
}

/** État de la mise à jour pour une fenêtre ; `api` absent : rien à montrer. */
export function useUpdate(api: UpdateApi | null, onError: (text: string) => void): UpdateState {
  const [status, setStatus] = useState<UpdateStatus>({ offer: null, installing: false });
  const [dismissed, setDismissed] = useState<string | null>(null);
  const [percent, setPercent] = useState<number | null>(null);
  const [requested, setRequested] = useState(0);
  const [pending, setPending] = useState(false);
  // Toujours le dernier rappel, sans réabonner l'effet quand il change.
  const report = useRef(onError);
  report.current = onError;
  // Seule la dernière lecture demandée compte (réponses dans le désordre).
  const seq = useRef(0);

  const reread = useCallback(() => {
    if (!api) return;
    const n = ++seq.current;
    api.status().then(
      (s) => {
        if (n !== seq.current) return;
        setStatus(s);
        // L'hôte a fini (ou n'a jamais commencé) : le clic en attente est retombé.
        if (!s.installing) setPending(false);
      },
      (e) => console.error("mise à jour", e),
    );
  }, [api]);

  useEffect(() => {
    if (!api) return;
    return subscribeUpdates(
      api,
      {
        onChanged: reread,
        onProgress: (p) => setPercent(p.total ? Math.min(100, Math.floor((p.received / p.total) * 100)) : null),
        onCheckRequested: () => setRequested((n) => n + 1),
      },
      reread,
    );
  }, [api, reread]);

  const check = useCallback(async () => {
    if (!api) return null;
    const s = await api.check();
    seq.current++;
    setDismissed(null);
    setStatus(s);
    return s;
  }, [api]);

  const install = useCallback(() => {
    if (!api) return;
    setPending(true);
    api.install().catch((e) => {
      setPending(false);
      report.current(errorText(e));
    });
  }, [api]);

  const later = useCallback(() => setDismissed(status.offer?.version ?? null), [status.offer]);

  const openRelease = useCallback(() => {
    api?.openRelease().catch((e) => report.current(errorText(e)));
  }, [api]);

  const installing = status.installing || pending;
  const offer = status.offer && (installing || status.offer.version !== dismissed) ? status.offer : null;
  return { offer, installing, percent, requested, check, install, later, openRelease };
}
```

`pending` covers the click before the host's first `update-changed`; a host error or a re-read
with `installing: false` clears it.

- [ ] **Step 6: Run the hook tests**

Run: `cd ui && bun test src/useUpdate.render.test.tsx`
Expected: 19 pass.

- [ ] **Step 7: Write the failing `useUpdateCheck` tests** (`ui/src/useUpdateCheck.render.test.tsx`)

```tsx
// Recherche de mise à jour depuis le menu de l'onglet : résultat dit par l'avis de l'onglet.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { UpdateStatus } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const { useUpdateCheck } = await import("./useUpdateCheck");

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const said: Array<[string, boolean]> = [];
const say = (text: string, error = false) => void said.push([text, error]);

async function mount(check: () => Promise<UpdateStatus | null>, requested = 0) {
  said.length = 0;
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  const out: { run: (() => void) | undefined } = { run: undefined };
  function Probe({ n }: { n: number }) {
    out.run = useUpdateCheck({ check, requested: n }, say);
    return null;
  }
  await act(async () => root.render(createElement(Probe, { n: requested })));
  return { run: () => out.run!(), rerender: (n: number) => act(async () => root.render(createElement(Probe, { n }))) };
}

describe("useUpdateCheck", () => {
  test("useUpdateCheck without updates gives no entry", async () => {
    let run: (() => void) | undefined = () => {};
    function Probe() {
      run = useUpdateCheck(undefined, say);
      return null;
    }
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    mounted = { root, host };
    await act(async () => root.render(createElement(Probe)));
    expect(run).toBeUndefined();
  });

  test("useUpdateCheck up to date says so", async () => {
    const m = await mount(async () => ({ offer: null, installing: false }));
    await act(async () => m.run());
    expect(said).toEqual([["ui-update-up-to-date", false]]);
  });

  test("useUpdateCheck offer says nothing", async () => {
    const m = await mount(async () => ({ offer: { version: "2.2.0", mode: "install" }, installing: false }));
    await act(async () => m.run());
    expect(said).toEqual([]);
  });

  test("useUpdateCheck failure says the error", async () => {
    const m = await mount(async () => { throw new Error("réseau"); });
    await act(async () => m.run());
    expect(said).toEqual([["réseau", true]]);
  });

  test("useUpdateCheck menu request runs a check", async () => {
    const m = await mount(async () => ({ offer: null, installing: false }));
    await m.rerender(1);
    expect(said).toEqual([["ui-update-up-to-date", false]]);
  });

  test("useUpdateCheck mount does not check", async () => {
    await mount(async () => ({ offer: null, installing: false }), 3);
    expect(said).toEqual([]);
  });
});
```

`t()` returns the message id in tests (empty bundle, `ui/src/i18n.ts:36-38`).

- [ ] **Step 8: Run them to see them fail**

Run: `cd ui && bun test src/useUpdateCheck.render.test.tsx`
Expected: FAIL, cannot find module `./useUpdateCheck`.

- [ ] **Step 9: Write `ui/src/useUpdateCheck.ts`**

```ts
// Recherche de mise à jour lancée depuis un onglet (menu, ou menu de l'app) ; son avis dit le résultat.

import { useCallback, useEffect, useRef } from "react";
import { errorText, type UpdateStatus } from "./api";
import { t } from "./i18n";

/** Ce que la fenêtre confie à son onglet actif pour chercher une mise à jour. */
export interface AppUpdates {
  check(): Promise<UpdateStatus | null>;
  /** Recherches demandées par le menu de l'app ; chaque hausse en lance une. */
  requested: number;
}

/** L'action « Rechercher des mises à jour… » ; absente sans hôte. */
export function useUpdateCheck(updates: AppUpdates | undefined, say: (text: string, error?: boolean) => void): (() => void) | undefined {
  const check = updates?.check;
  const run = useCallback(() => {
    if (!check) return;
    check().then(
      (s) => s && !s.offer && say(t("ui-update-up-to-date")),
      (e) => say(errorText(e), true),
    );
  }, [check, say]);
  // Au montage, les demandes déjà comptées ne relancent rien.
  const seen = useRef(updates?.requested ?? 0);
  useEffect(() => {
    const n = updates?.requested ?? 0;
    if (n > seen.current) run();
    seen.current = n;
  }, [updates?.requested, run]);
  return check ? run : undefined;
}
```

- [ ] **Step 10: Run them**

Run: `cd ui && bun test src/useUpdateCheck.render.test.tsx`
Expected: 6 pass.

- [ ] **Step 11: Write the failing bar tests** (`ui/src/UpdateBar.render.test.tsx`)

```tsx
// Bandeau de mise à jour : actions selon le mode, avancée du téléchargement.

import { afterEach, describe, expect, test } from "bun:test";
import "./test/dom";
import type { UpdateOffer } from "./api";

const { act, createElement } = await import("react");
const { createRoot } = await import("react-dom/client");
const UpdateBar = (await import("./UpdateBar")).default;

let mounted: { root: ReturnType<typeof createRoot>; host: HTMLElement } | null = null;
afterEach(async () => {
  const m = mounted;
  mounted = null;
  if (!m) return;
  await act(async () => m.root.unmount());
  m.host.remove();
});

const offer: UpdateOffer = { version: "2.2.0", mode: "install" };

async function mount(calls: string[], o: { offer?: UpdateOffer; installing?: boolean; percent?: number | null } = {}) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  await act(async () => {
    root.render(createElement(UpdateBar, {
      offer: o.offer ?? offer,
      installing: o.installing ?? false,
      percent: o.percent ?? null,
      onInstall: () => calls.push("install"),
      onLater: () => calls.push("later"),
      onOpenRelease: () => calls.push("open"),
    }));
  });
  return host;
}

const buttons = (host: HTMLElement) => [...host.querySelectorAll("button")];
const labels = (host: HTMLElement) => buttons(host).map((b) => b.textContent);
const click = (host: HTMLElement, i: number) => act(async () => buttons(host)[i]!.click());

describe("UpdateBar", () => {
  test("UpdateBar install mode offers install and later", async () => {
    expect(labels(await mount([]))).toEqual(["ui-update-install", "ui-update-later"]);
  });

  test("UpdateBar notify-only mode offers the release page", async () => {
    expect(labels(await mount([], { offer: { ...offer, mode: "notify_only" } }))).toEqual(["ui-update-see-release", "ui-update-later"]);
  });

  test("UpdateBar install button calls onInstall", async () => {
    const calls: string[] = [];
    await click(await mount(calls), 0);
    expect(calls).toEqual(["install"]);
  });

  test("UpdateBar release button calls onOpenRelease", async () => {
    const calls: string[] = [];
    await click(await mount(calls, { offer: { ...offer, mode: "notify_only" } }), 0);
    expect(calls).toEqual(["open"]);
  });

  test("UpdateBar later button calls onLater", async () => {
    const calls: string[] = [];
    await click(await mount(calls), 1);
    expect(calls).toEqual(["later"]);
  });

  test("UpdateBar installing disables its buttons", async () => {
    const host = await mount([], { installing: true, percent: 40 });
    expect(buttons(host).every((b) => b.disabled)).toBe(true);
  });

  test("UpdateBar installing shows the percent", async () => {
    const host = await mount([], { installing: true, percent: 40 });
    expect([host.querySelector(".update-bar-text")?.textContent, host.querySelector<HTMLProgressElement>("progress")?.value]).toEqual(["ui-update-installing", 40]);
  });

  test("UpdateBar unknown size shows an indeterminate bar", async () => {
    const host = await mount([], { installing: true, percent: null });
    expect([host.querySelector(".update-bar-text")?.textContent, host.querySelector("progress")?.hasAttribute("value")]).toEqual(["ui-update-downloading", false]);
  });
});
```

- [ ] **Step 12: Run them to see them fail**

Run: `cd ui && bun test src/UpdateBar.render.test.tsx`
Expected: FAIL, cannot find module `./UpdateBar`.

- [ ] **Step 13: Write `ui/src/UpdateBar.tsx`**

```tsx
// Bandeau de mise à jour, en haut de la fenêtre : une version plus récente,
// l'installer (ou voir la release), ou plus tard ; pendant le téléchargement,
// son avancée, boutons désactivés.

import type { UpdateOffer } from "./api";
import { t } from "./i18n";

/** Bandeau proposant une version plus récente de l'app. */
export default function UpdateBar({ offer, installing, percent, onInstall, onLater, onOpenRelease }: {
  offer: UpdateOffer;
  installing: boolean;
  /** Avancée du téléchargement, `null` si la taille est inconnue. */
  percent: number | null;
  onInstall: () => void;
  onLater: () => void;
  onOpenRelease: () => void;
}) {
  const text = !installing
    ? t("ui-update-available", { version: offer.version })
    : percent === null ? t("ui-update-downloading") : t("ui-update-installing", { percent });
  return (
    <div className="notice-bar update-bar" role="status">
      <span className="notice-bar-text update-bar-text">{text}</span>
      {installing && (percent === null
        ? <progress className="update-bar-progress" max={100} />
        : <progress className="update-bar-progress" max={100} value={percent} />)}
      <span className="notice-bar-actions">
        {offer.mode === "install" ? (
          <button type="button" className="btn primary" disabled={installing} onClick={onInstall}>{t("ui-update-install")}</button>
        ) : (
          <button type="button" className="btn primary" disabled={installing} onClick={onOpenRelease}>{t("ui-update-see-release")}</button>
        )}
        <button type="button" className="btn ghost" disabled={installing} onClick={onLater}>{t("ui-update-later")}</button>
      </span>
    </div>
  );
}
```

- [ ] **Step 14: Run the bar tests**

Run: `cd ui && bun test src/UpdateBar.render.test.tsx`
Expected: 8 pass.

- [ ] **Step 15: Styles** (`ui/src/styles.css`)

Share the notice surface instead of copying it. Rename in place: the `.pack-notice` rule's
surface declarations (`display` … `transition`) move to a new `.notice-bar` rule;
`.pack-notice-text` / `.pack-notice-actions` / `.pack-notice .btn` become
`.notice-bar-text` / `.notice-bar-actions` / `.notice-bar .btn`; `.pack-notice` keeps only
`grid-area`, `position`, `z-index`, `margin`. Update `PackNoticeBar.tsx` to
`className="notice-bar pack-notice"` and the inner classes; its render test must still pass.

After `.workspace-error` (line ~716), the bar in normal flow (never over the macOS titlebar):

```css
/* Bandeau de mise à jour : dans le flux, sous la barre de titre, centré. */
.update-bar { flex: none; align-self: center; width: min(640px, calc(100% - 2 * var(--space-4))); margin: var(--space-2) 0; }
.update-bar-progress { flex: 1 1 120px; accent-color: var(--accent-fill); }
```

- [ ] **Step 16: Wire it in `Workspace.tsx`**

Imports from `./api`: add `tauriUpdates`; plus `import UpdateBar from "./UpdateBar";` and
`import { useUpdate } from "./useUpdate";`. Right after `setError` (before any early return —
hooks must stay above the `if (!view …) return` guard near line 222):

```tsx
  // Mise à jour de l'app (hôte Tauri seulement) : bandeau ici ; la recherche passe par l'onglet actif,
  // dont l'avis dit le résultat (un seul avis par fenêtre).
  const update = useUpdate(tauriUpdates, setError);
  const appUpdates = useMemo(() => (tauriUpdates ? { check: update.check, requested: update.requested } : undefined), [update.check, update.requested]);
```

(add `useMemo` to the React import). In the JSX, after `{error && …}`:

```tsx
      {update.offer && (
        <UpdateBar
          offer={update.offer}
          installing={update.installing}
          percent={update.percent}
          onInstall={update.install}
          onLater={update.later}
          onOpenRelease={update.openRelease}
        />
      )}
```

and pass `updates={appUpdates}` to `<App …/>`.

- [ ] **Step 17: Menu entry in `App.tsx`**

Import `useUpdateCheck` and `type AppUpdates` from `./useUpdateCheck`. Prop, next to `onHelp`:

```tsx
  /** Recherche de mise à jour de la fenêtre ; absente hors de l'app (pas d'entrée). */
  updates?: AppUpdates;
```

After `const { say, node: toastNode } = useToast();`:

```tsx
  const checkUpdates = useUpdateCheck(updates, say);
```

In both `menuEntries` branches, right before the Help entry:

```tsx
        ...(checkUpdates ? [{ kind: "item" as const, label: t("ui-menu-check-updates"), onSelect: checkUpdates }] : []),
```

`App.tsx` goes from 908 to about 915 lines; stay under 1000, no refactor here.

- [ ] **Step 18: UI checks**

Run: `just test` and `cd ui && bunx tsc --noEmit`. Expected: all pass, no type errors.

- [ ] **Step 19: Commit**

```bash
git add ui/src crates/oku-slides-i18n/src/locales
git commit -F "$SCRATCH/msg"   # feat(ui) : bandeau de mise à jour et « Rechercher des mises à jour… » (t-967)
```

---

### Task 5: Full gate, then the first real updates (after merge)

- [ ] **Step 1:** `just ci` on the branch, green; `just e2e 4999` still passes (no update control
  outside Tauri). `just build` still works without any signing key in the env.
- [ ] **Step 2 (user, after merge and Task 0):** Release, bump `patch`, **only macOS (Apple
  Silicon)** ticked. Expected: the gate warns "1 plateformes sur 4" and leaves the release a
  draft; the draft's `latest.json` has `darwin-aarch64` with a signature.
- [ ] **Step 3 (user):** Release again, bump `aucune`, the three other platforms ticked. Expected:
  `latest.json` on the same draft now has four keys (tauri-action merged, nothing lost), and the
  gate publishes it as latest. This is the spec's `aucune` merge check.
  First run (2026-10-24, v3.0.1) published the partial release, which the repository's immutable
  releases froze: step 3 failed with "Cannot upload assets to an immutable release". The gate now
  keeps a partial release a draft and `aucune` refuses a published version; rerun on v3.0.2 (t-1145).
- [ ] **Step 4 (user):** install that version on each platform, then Release a `patch` with every
  platform. In the installed app:
  - macOS arm64 (from `/Applications`): the bar appears at launch; "Install and Restart"
    restarts on the new version **with the app running again** (single-instance on) and the same
    tabs; note whether macOS asks for the microphone again, or for anything else tied to the
    signature (Keychain, other TCC grants): report it on t-968.
  - macOS run from the `.dmg`: the bar shows "See the Release".
  - Windows: same as macOS arm64, through the passive NSIS installer; the app relaunches.
  - Linux AppImage: same; a `.deb` install shows "See the Release", which opens the releases page.
  - Manual "Check for Updates…" on the latest version: "Oku Slides is up to date."
- [ ] **Step 5:** mark t-967 and its sub-tasks done once the user confirms Step 4.
</content>
