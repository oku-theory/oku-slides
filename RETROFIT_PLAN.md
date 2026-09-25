# Rust Retrofit Audit — Écho (slider)

**Date :** 2026-09-25 · **Portée :** tout le workspace (8 crates, dont `slider-chart` et `src-tauri`)
**Référence :** Oku Theory Rust Guidelines (les fichiers `docs/…` cités par le skill ne sont pas
livrés avec le plugin : l'audit suit les règles chargées dans la session).
**Vérité terrain :** `cargo nextest run --workspace --exclude slider-app` → 57/57 ✓

## Avancement

Audit : `180ee21`. Corrections : étape par étape, chaque commit vérifié
(`cargo test --workspace`, nextest, clippy `-D warnings`, `cargo deny`).

| Étape | Audit | Après correction | Commit |
|---|---|---|---|
| 1. Structure & outillage | ! PARTIEL | ✓ `deny.toml`, `clippy.toml`, lints du workspace, `just ci/audit/watch/setup/coverage` | `27f05b1` |
| 2. Erreurs (Snafu) | ✗ ÉCHEC | ✓ Snafu opaque partout, backtraces, `unwrap_used`/`expect_used` = deny | `20b886b` |
| 3. Clippy / rustfmt | ! PARTIEL | ✓ `Debug` partout (lint `missing_debug_implementations`) | `27f05b1` |
| 4. Dépendances | ✗ ÉCHEC | ✓ plus de thiserror, `time` au lieu de `chrono`, versions dans le workspace, `cargo deny` ok | `27f05b1`, `09f12c2` |
| 5. Async (Tokio) | — N/A | — (Tokio seulement sous OpenDAL, runtime privé) | — |
| 6. Tests | ✗ ÉCHEC | ✓ speculoos, noms `<unité>_<cas>_<résultat>`, fichiers `_tests.rs`, `tests/regressions/`, couverture 80,3 % | ce commit |
| 7. Sécurité | ! PARTIEL | ✓ 2 failles corrigées (asset → panique / injection Typst), `serve` ne lit plus `..` ; avis quick-xml documentés dans `deny.toml` | `20b886b`, `2ba7bc0` |
| 8. i18n (Fluent) | ✗ ÉCHEC | ✓ `slider-i18n`, `en` de référence + `fr` ; erreurs, CLI (aide comprise), textes créés | `3255bbf` |
| 9. Stockage (OpenDAL) | ✗ ÉCHEC | ✓ `slider_platform::storage`, plus de `std::fs` en production | `2ba7bc0` |
| 10. Configuration | ! PARTIEL | ✓ `config` : défauts embarqués, `config.toml`, `ECHO_<SECTION>__<CLÉ>` | `20b886b` |
| 11. Observabilité | ✗ ÉCHEC | ✓ `tracing` (INFO fixe en production), `color-eyre` ; la CLI écrit ses résultats sans `println!` | `09f12c2`, `3255bbf` |

## Restes (hors de portée de ce retrofit, à planifier)

- **Tâches Oku** : le plan n'est rattaché à aucune tâche parente (outils de
  tâches absents de la session) ; les sous-tâches restent à créer.
- **Langues** : 18 langues cibles sur 20 restent à traduire (une tâche par
  langue) ; l'interface React n'est pas traduite (d'où `locale = "fr"` par
  défaut) ; les libellés par défaut des graphiques (« Série 1 »,
  « Catégorie »…) dépendent de l'intégration de `slider-chart`, en cours.
- **Couverture** : `slider-stt/src/whisper.rs` (~20 %) demande un vrai
  modèle Whisper ; `slider-cli/src/serve.rs` est testé mais le processus
  tué n'écrit pas son profil de couverture.
- **Stockage** : CTranslate2 lit les poids du modèle par chemin (code
  natif, hors OpenDAL) ; `Storage::local` suppose des chemins Unix (racine
  `/`), à revoir pour Windows.

---|---|---|---|
| 1. Structure & outillage | ! PARTIEL | 1 | 2 |
| 2. Erreurs (Snafu) | ✗ ÉCHEC | 3 | 1 |
| 3. Clippy / rustfmt | ! PARTIEL | 1 | 1 |
| 4. Dépendances | ✗ ÉCHEC | 3 | 1 |
| 5. Async (Tokio) | — N/A | 0 | 1 |
| 6. Tests | ✗ ÉCHEC | 3 | 1 |
| 7. Sécurité | ! PARTIEL | 1 | 1 |
| 8. i18n (Fluent) | ✗ ÉCHEC | 1 | 0 |
| 9. Stockage (OpenDAL) | ✗ ÉCHEC | 1 | 0 |
| 10. Configuration | ! PARTIEL | 0 | 2 |
| 11. Observabilité | ✗ ÉCHEC | 1 | 0 |

---

## Étape 1 — Structure & outillage

- ✓ `[workspace]`, `resolver = "3"`, `edition = "2024"`, `rust-version = "1.88"` (≥ 1.85).
- ✓ Aucun fichier > 1000 lignes. Au-dessus de la cible de 500 : `slider-stt/src/lib.rs` (706),
  `slider-session/src/lib.rs` (629), `slider-core/src/ops.rs` (503).
- ✗ **E1.1** `clippy.toml` et `deny.toml` absents (seul `rustfmt.toml` existe).
- ! **W1.1** `slider-stt/src/lib.rs` mélange état, boucles de fils et nettoyage de texte : à
  découper (`engine.rs`, `listen.rs`, `text.rs`).
- ! **W1.2** `justfile` n'a ni `just ci`, ni `just audit`, ni `just watch`, ni `just setup`.

## Étape 2 — Erreurs

- ✗ **E2.1** `thiserror` partout (core, pdf, render, session, stt, chart) ; aucun `snafu`, aucun
  `backtrace: Backtrace`. 7 énumérations d'erreur à migrer : `OpError`, `SourcesError`,
  `PdfError`, `RenderError`, `SessionError`, `DownloadError`, `ChartError`.
- ✗ **E2.2** 8 `Result<_, String>` dans `slider-stt` (`Recorder::start`, `Transcriber::load`,
  `transcribe`, `engine`…) : pas de type d'erreur.
- ✗ **E2.3** 48 `unwrap`/`expect` hors tests : 22 `Mutex::lock().unwrap()` (dont 20 dans
  `slider-stt/src/lib.rs`), le reste des invariants (`expect("sérialisable")`,
  `expect("chemin non vide")`, `expect("l'inverse d'une op appliquée s'applique toujours")`…).
  La règle les interdit ; les invariants deviennent des erreurs ou des `unreachable!` documentés,
  les verrous empoisonnés un `SttError::Poisoned` (ou `lock().unwrap_or_else(PoisonError::into_inner)`).
- ! **W2.1** 37 `map_err` (préférer `.context(…)` avec Snafu) ; pas d'erreurs opaques pour les
  API de bibliothèque. Messages déjà en minuscules ✓.

## Étape 3 — Clippy / rustfmt

- ✓ `cargo fmt --check` ; ✓ `cargo clippy --workspace --all-targets -D warnings` ; ✓ aucun
  `#[allow]` ; ✓ aucun `get_` ; ✓ `//!` sur tous les modules sauf `slider-chart/src/error.rs`.
- ✗ **E3.1** Types publics sans `Debug` : `Walk` (core), `Fonts` (Debug manuel ✓ — faux positif),
  `Renderer`, `Compiled` (render), `Session` (session), `Stt`, `Recorder`, `Captured`,
  `Resampler`, `Segmenter`, `Transcriber` (stt).
- ! **W3.1** `slider-chart/src/error.rs` sans documentation de module.

## Étape 4 — Dépendances

- ✗ **E4.1** Crates interdits : `thiserror` (6 crates), `chrono` (pdf, render) au lieu de `time`.
- ✗ **E4.2** Versions hors `[workspace.dependencies]` : `tiny_http`, `typst-layout`, `ct2rs`,
  `mel_spec`, `ndarray`, `earshot`, `cpal`, `ureq`, `hound`, `pretty_assertions`, `tauri`,
  `tauri-build`, `tauri-plugin-dialog`.
- ✗ **E4.3** `cargo deny` : les 8 crates du workspace n'ont pas de `license`, et sans `deny.toml`
  toutes les licences tierces sont refusées (782 rejets).
- ! **W4.1** Pré-1.0 non épinglées au mineur : `tiny_http = "0.12"`, `ct2rs = "0.10"`,
  `mel_spec = "0.3"`, `ndarray = "0.16"` (le caret de Cargo épingle déjà le mineur en 0.x : OK
  de fait, à écrire explicitement si la règle l'exige).

## Étape 5 — Async

- — Pas de Tokio : le projet est synchrone (Tauri appelle des commandes synchrones, dictée sur
  fils `std::thread`, serveur de dev `tiny_http`). Aucune règle async ne s'applique.
- ! **W5.1** `Arc<Mutex<Inner>>` (état de `Stt`) et `Arc<Mutex<Captured>>` (micro) : l'état est
  sondé par l'interface, un canal ne remplacerait pas l'état partagé ; `Captured` pourrait passer
  par un canal (`mpsc`) du rappel cpal vers la boucle d'écoute.

## Étape 6 — Tests

- ✓ nextest fonctionne (57 tests).
- ✗ **E6.1** `speculoos` absent : 190 `assert!`/`assert_eq!`/`assert_ne!`.
- ✗ **E6.2** Pas de `tests/regressions/`.
- ! **W6.1** Tests en ligne (`#[cfg(test)] mod tests`) ou dans `tests/`, pas en fichiers
  `*_tests.rs` côte à côte ; noms en français sans le motif `<unit>_<scenario>_<result>`.
- ✗ **E6.3** Couverture des lignes **66,7 %** (régions 63,6 %, fonctions 67,7 %) pour 80 % visés.
  Trous : `slider-cli` 0 %, `slider-stt` `whisper.rs` 16 %, `audio.rs` 17 %, `lib.rs` 27 %,
  `models.rs` 26 % (micro et modèle requis : il faut des doublures), `session/files.rs` 43 %,
  `render/fonts.rs` 48 %, `core/validate.rs` 65 %, `chart/option.rs` 56 %. Bien couverts : core
  (sauf validate), pdf 97 %, render 86–94 %, session 86–95 %, `vad.rs` 89 %.

## Étape 7 — Sécurité

- ✓ Aucun `unsafe` dans le code du projet (cargo-geiger non installé : dépendances non
  mesurées) ; ✓ aucun secret en dur ; pas de secrets à manipuler (pas de `secrecy` nécessaire).
- ✗ **E7.1** `cargo audit` : **RUSTSEC-2026-0194 / 0195** (`quick-xml` 0.38.4, déni de service),
  arrivé par `typst-library → hayagriva → citationberg` (bibliographies CSL). Écho ne donne
  jamais de fichier de bibliographie de l'utilisateur à Typst : chemin non atteignable ;
  correction = monter Typst quand `citationberg` suit. 12 avertissements « non maintenu »
  (bincode, paste, proc-macro-error, rustybuzz, ttf-parser, unic-*…), tous transitifs.
- ! **W7.1** Validation des entrées : les requêtes JSON passent par serde (types stricts), sans
  `validator` ; les bornes métier sont dans `Op::apply` (grille 1–24, cadre > 0…).

## Étape 8 — i18n

- ✗ **E8.1** Pas de Fluent : tous les textes (erreurs, libellés, messages CLI, interface React)
  sont en français en dur. Pas de locale `en`, pas de `sys-locale`.

## Étape 9 — Stockage

- ✗ **E9.1** Pas d'OpenDAL : `std::fs` direct dans cli (13), session `files.rs` (3) et
  `templates.rs` (4), render `fonts.rs` (2), stt (11).

## Étape 10 — Configuration

- ✓ Pas de secrets ; hôte du serveur de dev (`127.0.0.1`) et port par défaut (`4870`) sont des
  arguments `clap`.
- ! **W10.1** Pas de crate `config` ni de `config.default.toml` ; réglages par variables
  d'environnement isolées : `ECHO_DATA`, `ECHO_MODELS`, `ECHO_FAKE_MIC`, `SLIDER_FONTS` (préfixe
  incohérent, pas de motif `{PREFIX}_{SECTION}__{KEY}`).
- ! **W10.2** Valeurs en dur : URL Hugging Face, tailles des modèles, délais de la dictée
  (`TICK`, `NOTHING_HEARD`, `VadConfig::default`), limite d'historique (500).

## Étape 11 — Observabilité

- ✗ **E11.1** Pas de `tracing` : 22 `println!/eprintln!` dans la CLI (sorties voulues d'un
  outil en ligne de commande), 1 `eprintln!` dans `slider-stt/src/audio.rs` (erreur du flux
  micro, à passer en `tracing::warn!`). Aucun abonné configuré dans l'app.

---

## Corrections automatiques possibles (`--fix`)

- [ ] `#[derive(Debug)]` (ou Debug manuel) sur 10 types publics (E3.1)
- [ ] `//!` sur `slider-chart/src/error.rs` (W3.1)
- [ ] Versions déplacées dans `[workspace.dependencies]` (E4.2)
- [ ] `license` dans `[workspace.package]` + `deny.toml` + `clippy.toml` (E1.1, E4.3)
- [ ] `eprintln!` de `audio.rs` → `tracing::warn!` (E11.1, demande d'ajouter `tracing`)

## Corrections manuelles (à planifier)

1. `thiserror` → `snafu` avec backtraces, erreurs opaques ; `String` → `SttError` (E2.1, E2.2)
2. `unwrap`/`expect` de production (E2.3)
3. `chrono` → `time` (E4.1)
4. Tests : `speculoos`, `tests/regressions/`, nommage, couverture 80 % (E6.1–E6.3, W6.1)
5. Fluent + locale `en` + tâches par langue (E8.1) — décision produit : l'app est française
6. OpenDAL (E9.1) — à discuter : app de bureau à fichiers locaux, peu de gain
7. `tracing` dans l'app, la session et la dictée (E11.1)
8. `config` pour les réglages de la dictée et de l'app (W10.1, W10.2)
