# Dictée par le moteur natif Apple — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** sur macOS 26+, la dictée passe par le moteur du système (`SpeechAnalyzer` / `SpeechTranscriber`, en continu, Neural Engine) : le texte final arrive ~0,1 s après la fin de la voix au lieu de ~4–8 s avec Whisper turbo. Il devient un modèle de plus, « Apple (système) », choisi par défaut ; Whisper reste le moteur ailleurs.

**Architecture:** le crate `oku-slides-stt` gagne un trait interne `StreamEngine` (moteurs en continu) avec deux implémentations : `AppleEngine` (pont Swift statique, ABI C `@_cdecl`, compilé par `build.rs` via swift-rs, macOS seulement) et `NoStreamEngine` (ailleurs, ou réglage `stt.apple = false`). `listen_loop` garde micro, 16 kHz, earshot et arrêt automatique ; l'audio part soit en jobs Whisper (inchangés, déplacés dans `dictation_whisper.rs`), soit dans un `Stream` dont les événements (`Partial`, `Final`, `Done`, `Failed`) sont appliqués par une fonction pure (`results::apply`) sur un fil dédié (`dictation_stream.rs`). `Status` ne change pas de forme hors `ModelInfo.available` et `Config.version`.

**Tech Stack:** Rust 2024, snafu, speculoos, hound (tests), sys-locale, swift-rs 1.0.8 (`build`), Swift 6.4 (manifeste 5.9, mode Swift 5), frameworks Speech / AVFoundation / CoreMedia (macOS 26 SDK, déploiement 12.0) ; React 19 + TypeScript, bun test ; Fluent.

**Spec:** `docs/superpowers/specs/2026-10-03-dictee-native-macos-design.md` (t-393)

## Global Constraints

- Plateformes : moteur d'Apple seulement si `target_os = "macos"` **et** macOS 26+ à l'exécution (`#available(macOS 26.0, *)`) **et** `stt.apple = true`. `tauri.conf.json` passe à `minimumSystemVersion: 12.0`, comme `MACOSX_DEPLOYMENT_TARGET` (`.cargo/config.toml`) et le paquet Swift : le pont lie le runtime Swift Concurrency, livré par l'OS depuis macOS 12 seulement.
- `Model` (sérialisé en minuscules) : `apple`, `turbo`, `large`, `base` ; `Model::ALL = [Apple, Turbo, Large, Base]`. `Model::Apple.size() == 0`.
- `Config.version: u32` ; `CONFIG_VERSION = 1`. `Config::default().version == 0` (une config sans version, ou neuve, est migrée une fois au chargement). `configure` écrit toujours `CONFIG_VERSION`.
- Migration : version < 1 → `model = Apple` si le moteur en continu est `supported()` et que la langue retenue (choisie → interface → système) y est `Ready` ou `Missing`, puis `version = 1` (en mémoire, pas réenregistrée). `model == Apple` sur une machine sans moteur → `Turbo`.
- Langue d'Apple : `locale::resolve(choisie, langue de l'UI (Localizer::locale), locale système (sys_locale), disponibilité)` → première étiquette non `Unsupported`, ou `None`. `detected` = code de langue de l'étiquette retenue (`fr-FR` → `fr`) quand le modèle est Apple.
- API : `SttRequest::Start { vocabulary: Vec<String> }` (`#[serde(default)]`) remplace `prompt`. Whisper : termes non vides joints par `". "`, 600 caractères au plus (`MAX_PROMPT_CHARS`).
- Réglages `[stt]` (TOML embarqué `crates/oku-slides-platform/src/config.default.toml`) : `apple = true`, `apple_finish_timeout_secs = 5.0`.
- Pont : codes partagés `stream::raw` ↔ Swift : événements `PARTIAL 0`, `FINAL 1`, `PROGRESS 2`, `DONE 3`, `ERROR 4` ; erreurs `ANALYZER 1`, `LOCALE 2`, `ASSET 3`, `DENIED 4`. Le Swift appelle `free(ctx)` **une seule fois**, après son dernier événement ; il n'a ni `try!` ni `!`.
- Erreurs (Snafu, `backtrace` dans chaque variante, display en minuscules) : `SpeechDenied`, `LocaleUnsupported { locale }`, `AssetInstall { reason }`, `Analyzer { reason }`, `FinishTimeout`, `NotDownloadable` ; nouvelle sorte `SttErrorKind::Permission`.
- Fluent, **20 langues** (`crates/oku-slides-i18n/src/locales/<l>/`) : fr et en verbatim ci-dessous, **traduire les 18 autres** (ar, bn, de, es, hi, it, ja, ko, nl, pl, pt, ru, th, tr, uk, vi, zh-Hans, zh-Hant) ; le test `every_locale_has_exactly_the_reference_keys` l'exige.
- `unsafe_code = "deny"` au niveau du workspace : seul `apple.rs` le lève (`#![expect(unsafe_code, reason = "…")]`), chaque bloc avec `// SAFETY:`.
- Pas de `.unwrap()`/`.expect()` hors tests (`unwrap_or_else(|e| panic!(…))` dans les aides de test) ; pas de `std::fs` en production ; doc en français, première phrase < 15 mots ; fichiers ≤ 500 lignes visés, 1000 plafond (`lib.rs` : 691 lignes au départ).
- Tests côte à côte `foo_tests.rs` (`#[cfg(test)] #[path = "foo_tests.rs"] mod foo_tests;`), speculoos, nommage `<unité>_<scénario>_<résultat>`, TDD.
- Commits : `<type>(<scope>) : <description> (t-393)`, rupture → `feat(<scope>)! : …`, **sans** ligne `Co-Authored-By`. `git add` dans une commande séparée du `git commit`. Dans ce worktree SDD, une fois : `git config --worktree oku.review deferred` (vérifier `git config --show-scope --get-all oku.review`), retiré en fin de plan.
- Vérifications : `cargo nextest run -p oku-slides-stt` (et `-p oku-slides-platform`, `-p oku-slides-i18n` quand touchés) ; `cargo clippy -p oku-slides-stt --all-targets -- -D warnings` ; UI : `cd ui && bun test && bun run typecheck` ; en fin de plan : `just check`.

---

### Task 1: Extraire la boucle Whisper de lib.rs

Refactor sans changement de comportement : `lib.rs` doit faire de la place.

**Files:**
- Create: `crates/oku-slides-stt/src/dictation_whisper.rs`
- Modify: `crates/oku-slides-stt/src/lib.rs` (supprimer `enum Job`, `fn engine`, `fn send_final`, `fn transcribe_loop` ; ajouter `Inner::complete`, `Inner::reset`)

**Interfaces:**
- Produces: `dictation_whisper::Job` (`Final { index, audio }`, `Preview { audio }`, `Done`) ; `impl Stt { fn engine(&self) -> Result<Arc<Transcriber>, SttError>; fn send_final(&self, seg: &Segmenter, phrase: Phrase, jobs: &mpsc::Sender<Job>); fn transcribe_loop(&self, session: u64, rx: mpsc::Receiver<Job>, busy: Arc<AtomicBool>) }` ; `Inner::complete(&mut self)` (texte rendu, `seq += 1`, retour au repos) ; `Inner::reset(&mut self)` (retour au repos sans texte).

- [ ] **Step 1: Vérifier que la suite est verte avant de toucher**

Run: `cargo nextest run -p oku-slides-stt`
Expected: PASS (tous).

- [ ] **Step 2: Ajouter `complete` et `reset` à `Inner` (lib.rs)**

Juste après `impl Inner { fn committed(&self) -> String { … } }`, compléter le bloc :

```rust
impl Inner {
    fn committed(&self) -> String {
        join(self.phrases.iter().flatten())
    }

    /// Fin d'une dictée : le texte assemblé est rendu, retour au repos.
    fn complete(&mut self) {
        self.text = Some(self.committed());
        self.seq += 1;
        self.phrases.clear();
        self.partial.clear();
        self.state = State::Idle;
        self.stop_in = None;
        self.level = 0.0;
        self.speaking = false;
        self.session += 1;
    }

    /// Dictée abandonnée : retour au repos, sans texte.
    fn reset(&mut self) {
        self.state = State::Idle;
        self.started = None;
        self.phrases.clear();
        self.partial.clear();
        self.stop_in = None;
        self.level = 0.0;
        self.speaking = false;
        self.session += 1;
    }
}
```

Et `cancel` devient :

```rust
    pub fn cancel(&self) {
        self.cancel_download.store(true, Ordering::Relaxed);
        let mut g = self.state();
        if matches!(g.state, State::Recording | State::Transcribing) {
            g.reset();
        }
        g.start_after_download = false;
    }
```

- [ ] **Step 3: Déplacer la boucle Whisper dans `dictation_whisper.rs`**

Créer le fichier avec, **à l'identique**, `enum Job`, `fn engine`, `fn send_final` et `fn transcribe_loop` coupés de `lib.rs` ; seule différence : la branche `Job::Done` de `transcribe_loop` appelle `g.complete()` au lieu de remettre les champs un par un.

```rust
//! Dictée par Whisper : phrases closes en qualité finale, aperçu de la phrase en cours.
//!
//! `listen_loop` (lib.rs) envoie des [`Job`] ; ce fil les transcrit dans l'ordre,
//! les phrases closes avant l'aperçu.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::Instant;

use oku_slides_i18n::Localize;

use crate::vad::{Phrase, Segmenter};
use crate::{SttError, Stt, Transcriber, context_of, keep};

/// Travail pour le fil de transcription.
pub(crate) enum Job {
    /// Phrase close : texte définitif, rangé à l'indice donné.
    Final { index: usize, audio: Vec<f32> },
    /// Phrase en cours : aperçu.
    Preview { audio: Vec<f32> },
    /// Fin de la dictée : assembler le texte.
    Done,
}

impl Stt {
    // ⟵ coller ici `fn engine`, `fn send_final`, `fn transcribe_loop`
    //    tels qu'ils étaient dans lib.rs, en `pub(crate) fn`.
}
```

Dans `transcribe_loop`, la fin de la branche `Job::Done` devient :

```rust
                Job::Done => {
                    if pending.iter().any(|j| matches!(j, Job::Final { .. })) {
                        // Des phrases restent : on les traite d'abord.
                        pending.push(Job::Done);
                        continue;
                    }
                    let mut g = self.state();
                    if g.session != session {
                        return;
                    }
                    g.complete();
                    return;
                }
```

Dans `lib.rs` : ajouter `mod dictation_whisper;` (ordre alphabétique des `mod`) et `use dictation_whisper::Job;`, retirer les `use` devenus inutiles (le compilateur les signale). `State` et `Inner` restent dans `lib.rs` : les modules enfants y ont accès.

- [ ] **Step 4: Vérifier**

Run: `cargo nextest run -p oku-slides-stt && cargo clippy -p oku-slides-stt --all-targets -- -D warnings && wc -l crates/oku-slides-stt/src/lib.rs`
Expected: PASS, aucun avertissement, `lib.rs` sous 560 lignes.

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-stt/src/lib.rs crates/oku-slides-stt/src/dictation_whisper.rs
```
```bash
git commit -m "refactor(stt) : boucle Whisper hors de lib.rs, fin et abandon d'une dictée factorisés (t-393)"
```

---

### Task 2: Vocabulaire en liste pour la dictée

`start { prompt: string }` devient `start { vocabulary: string[] }` (rupture d'API, UI comprise).

**Files:**
- Modify: `crates/oku-slides-stt/src/lib.rs` (`SttRequest::Start`, `Inner.prompt` → `vocabulary`, `start`, `context_of`, `whisper_prompt`, `transcribe_samples`)
- Modify: `crates/oku-slides-stt/src/dictation_whisper.rs` (`context_of(&g.vocabulary, …)`)
- Modify: `crates/oku-slides-stt/src/lib_tests.rs`
- Modify: `crates/oku-slides-stt/tests/dictation.rs` (3 appels `Start`)
- Modify: `ui/src/api.ts`, `ui/src/Dictation.tsx`, `ui/src/App.tsx:625-627,891`, `ui/src/Dictation.render.test.tsx`

**Interfaces:**
- Produces: `SttRequest::Start { vocabulary: Vec<String> }` ; `Stt::start(&self, vocabulary: Vec<String>)` ; `Inner.vocabulary: Vec<String>` ; `fn whisper_prompt(vocabulary: &[String]) -> String` ; `fn context_of(vocabulary: &[String], committed: &str) -> String` ; TS `stt.start(vocabulary: string[])`, prop `Dictation.vocabulary: string[]`.

- [ ] **Step 1: Tests Rust en échec (lib_tests.rs)**

```rust
#[test]
fn whisper_prompt_terms_are_joined_with_periods() {
    let vocabulary = vec!["Bilan".to_owned(), "  ".to_owned(), "Q3".to_owned()];
    assert_that!(whisper_prompt(&vocabulary)).is_equal_to("Bilan. Q3".to_owned());
}

#[test]
fn whisper_prompt_long_vocabulary_is_cut_at_600_chars() {
    let vocabulary = vec!["é".repeat(700)];
    assert_that!(whisper_prompt(&vocabulary).chars().count()).is_equal_to(600);
}

#[test]
fn context_of_vocabulary_comes_before_dictated_text() {
    assert_that!(context_of(&["Bilan".to_owned()], " déjà dit ")).is_equal_to("Bilan déjà dit".to_owned());
}

#[test]
fn start_request_reads_vocabulary_list() {
    let req: SttRequest = serde_json::from_str(r#"{"cmd":"start","vocabulary":["Bilan","Q3"]}"#).unwrap();
    assert_that!(matches!(req, SttRequest::Start { vocabulary } if vocabulary == ["Bilan", "Q3"])).is_true();
}
```

Run: `cargo nextest run -p oku-slides-stt whisper_prompt context_of start_request`
Expected: FAIL à la compilation (`whisper_prompt` introuvable, `vocabulary` inconnu).

- [ ] **Step 2: Implémenter (lib.rs)**

```rust
    /// Démarre la dictée ; `vocabulary` aide la reconnaissance (titres du deck, noms propres).
    Start {
        #[serde(default)]
        vocabulary: Vec<String>,
    },
```

`Inner` : remplacer `prompt: Option<String>` par `/// Vocabulaire de la dictée en cours. vocabulary: Vec<String>,` (initialisé à `Vec::new()`). `handle` : `SttRequest::Start { vocabulary } => self.start(vocabulary),`. `start(&self, vocabulary: Vec<String>)` : `g.vocabulary = vocabulary;`. Dans `transcribe_samples` : `(g.config.language.clone(), g.vocabulary.clone())` puis `context_of(&vocabulary, &join(texts.iter()))`. Dans `dictation_whisper.rs` : `context_of(&g.vocabulary, &g.committed())`.

```rust
/// Longueur maximale du vocabulaire donné à Whisper (caractères).
const MAX_PROMPT_CHARS: usize = 600;

/// Le vocabulaire en une phrase pour Whisper : termes joints par « . », tronqués.
fn whisper_prompt(vocabulary: &[String]) -> String {
    let joined = vocabulary.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(". ");
    joined.chars().take(MAX_PROMPT_CHARS).collect()
}

/// Contexte donné à Whisper : vocabulaire du deck, puis ce qui vient d'être dicté.
fn context_of(vocabulary: &[String], committed: &str) -> String {
    let prompt = whisper_prompt(vocabulary);
    [prompt.as_str(), committed].iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ")
}
```

`tests/dictation.rs` : `Start { prompt: Some("Oku Slides".into()) }` → `Start { vocabulary: vec!["Oku Slides".into()] }` ; `Start { prompt: None }` → `Start { vocabulary: Vec::new() }` (deux fois).

Run: `cargo nextest run -p oku-slides-stt`
Expected: PASS.

- [ ] **Step 3: Test UI en échec (Dictation.render.test.tsx)**

Dans `fakeStt`, garder les arguments de `start` :

```ts
  const starts: string[][] = [];
  // …
    spyOn(stt, "start").mockImplementation((vocabulary: string[]) => (calls.push("start"), starts.push(vocabulary), set("recording"))),
  // …
  return { calls, starts, restore: () => spies.forEach((s) => s.mockRestore()) };
```

`mount(agentBusy = false, vocabulary: string[] = [])` passe `vocabulary` (au lieu de `prompt: ""`). Nouveau test :

```ts
describe("vocabulaire", () => {
  test("Dictation start sends the deck vocabulary", async () => {
    await mount(false, ["Bilan", "Q3"]);
    await press("d", { meta: true });
    expect(engine.starts).toEqual([["Bilan", "Q3"]]);
  });
});
```

Run: `cd ui && bun test src/Dictation.render.test.tsx`
Expected: FAIL (typecheck/valeur : `prompt` attendu, `starts` vide ou `[""]`).

- [ ] **Step 4: Implémenter (UI)**

`api.ts` :

```ts
  | { cmd: "start"; vocabulary: string[] }
// …
  start: (vocabulary: string[]) => sttCall({ cmd: "start", vocabulary }),
```

`Dictation.tsx` : prop `/** Mots utiles à la reconnaissance : titres, noms propres du deck. */ vocabulary: string[];` à la place de `prompt` ; `send(stt.start(vocabulary));` ; dépendance `[status, vocabulary, send]`.

`App.tsx` (remplace `whisperPrompt`, l. 625-627) :

```ts
  const dictationVocabulary = useMemo(
    () => (snap ? [snap.title, ...snap.slides.map((s) => s.title)].filter((s): s is string => !!s) : []),
    [snap],
  );
```

et `vocabulary={dictationVocabulary}` (l. 891).

Run: `cd ui && bun test && bun run typecheck`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-stt/src/lib.rs crates/oku-slides-stt/src/dictation_whisper.rs crates/oku-slides-stt/src/lib_tests.rs crates/oku-slides-stt/tests/dictation.rs ui/src/api.ts ui/src/Dictation.tsx ui/src/App.tsx ui/src/Dictation.render.test.tsx
```
```bash
git commit -m "feat(stt)! : la dictée reçoit un vocabulaire en liste plutôt qu'un prompt (t-393)"
```

---

### Task 3: Événements du moteur en continu et erreurs

Types et traits des moteurs en continu, accumulateur pur, nouvelles erreurs. Rien n'est encore branché.

**Files:**
- Create: `crates/oku-slides-stt/src/stream.rs`, `crates/oku-slides-stt/src/stream_tests.rs`
- Create: `crates/oku-slides-stt/src/results.rs`, `crates/oku-slides-stt/src/results_tests.rs`
- Modify: `crates/oku-slides-stt/src/error.rs`, `crates/oku-slides-stt/src/error_tests.rs`
- Modify: `crates/oku-slides-stt/src/lib.rs` (`mod results; mod stream;`)
- Modify: `crates/oku-slides-i18n/src/locales/*/stt.ftl` (20 langues)

**Interfaces:**
- Produces (crate) :
  - `pub(crate) enum Availability { Unsupported, Missing, Ready }` (`Debug, Clone, Copy, PartialEq, Eq`) ; hors de l'API publique (`ModelInfo` n'expose que des booléens).
  - `pub(crate) enum Event { Partial(String), Final { text: String, start: f32, end: f32 }, Progress { received: u64, total: u64 }, Done, Failed(SttError) }` et `Event::from_raw(kind: i32, text: String, a: f64, b: f64) -> Event`.
  - `pub(crate) mod raw` (constantes du pont).
  - `pub(crate) trait Stream: Send { fn push(&mut self, samples: &[f32]); fn finish(&mut self); fn cancel(&mut self); }`
  - `pub(crate) trait StreamEngine: Send + Sync { fn supported(&self) -> bool; fn availability(&self, locale: &str) -> Availability; fn install(&self, locale: &str) -> mpsc::Receiver<Event>; fn open(&self, locale: &str, vocabulary: &[String]) -> (Box<dyn Stream>, mpsc::Receiver<Event>); }`
  - `pub(crate) struct NoStreamEngine;` ; `pub(crate) fn engine(settings: &SttSettings) -> Arc<dyn StreamEngine>` (renvoie `NoStreamEngine` ; Task 6 y branche Apple).
  - `pub(crate) fn results::apply(phrases: &mut Vec<Option<String>>, partial: &mut String, event: &Event) -> bool`.
  - Sélecteurs Snafu `SpeechDeniedSnafu`, `LocaleUnsupportedSnafu { locale }`, `AssetInstallSnafu { reason }`, `AnalyzerSnafu { reason }`, `FinishTimeoutSnafu`, `NotDownloadableSnafu` ; `SttErrorKind::Permission`.

- [ ] **Step 1: Tests d'erreurs en échec (error_tests.rs)**

Ajouter au vecteur `cases` de `localize_each_kind_speaks_to_the_user` :

```rust
        (
            SpeechDeniedSnafu.build().into(),
            SttErrorKind::Permission,
            "Autorisez la reconnaissance vocale dans Réglages Système › Confidentialité et sécurité",
        ),
        (
            LocaleUnsupportedSnafu { locale: "xx" }.build().into(),
            SttErrorKind::Model,
            "Le moteur d'Apple ne prend pas en charge cette langue (xx)",
        ),
        (AssetInstallSnafu { reason: "x" }.build().into(), SttErrorKind::Download, "Le modèle n'a pas pu être téléchargé"),
        (AnalyzerSnafu { reason: "x" }.build().into(), SttErrorKind::Transcription, "La transcription a échoué"),
        (FinishTimeoutSnafu.build().into(), SttErrorKind::Transcription, "La transcription a échoué"),
        (NotDownloadableSnafu.build().into(), SttErrorKind::Model, "Modèle de dictée illisible"),
```

Run: `cargo nextest run -p oku-slides-stt localize_each_kind`
Expected: FAIL à la compilation (sélecteurs inconnus).

- [ ] **Step 2: Implémenter les erreurs (error.rs)**

`SttErrorKind` : ajouter `/// Autorisation refusée (reconnaissance vocale). Permission,`. `SttInner` :

```rust
    #[snafu(display("speech recognition not allowed"))]
    SpeechDenied { backtrace: Backtrace },
    #[snafu(display("language not supported by the system engine: {locale}"))]
    LocaleUnsupported { locale: String, backtrace: Backtrace },
    #[snafu(display("language download failed: {reason}"))]
    AssetInstall { reason: String, backtrace: Backtrace },
    #[snafu(display("system transcription failed: {reason}"))]
    Analyzer { reason: String, backtrace: Backtrace },
    #[snafu(display("system transcription did not finish in time"))]
    FinishTimeout { backtrace: Backtrace },
    #[snafu(display("this model is installed by the system"))]
    NotDownloadable { backtrace: Backtrace },
```

`kind()` : `SpeechDenied => Permission` ; `LocaleUnsupported | NotDownloadable` rejoignent `Model` ; `AssetInstall` rejoint `Download` ; `Analyzer | FinishTimeout` rejoignent `Transcription`. `localize()` :

```rust
            SpeechDenied { .. } => l.tr("stt-speech-denied"),
            LocaleUnsupported { locale, .. } => {
                l.tr_with("stt-locale-unsupported", &[("language", locale.as_str().into())])
            }
```

`NotDownloadable` rejoint le bras `stt-model`, `AssetInstall` le bras `stt-download`, `Analyzer | FinishTimeout` le bras `stt-transcription`.

Fluent, `stt.ftl`, après `stt-cancelled` :

fr :
```
stt-speech-denied = Autorisez la reconnaissance vocale dans Réglages Système › Confidentialité et sécurité
stt-locale-unsupported = Le moteur d'Apple ne prend pas en charge cette langue ({ $language })
```
en :
```
stt-speech-denied = Allow speech recognition in System Settings › Privacy & Security
stt-locale-unsupported = Apple's engine does not support this language ({ $language })
```
Traduire dans les 18 autres langues (même place).

Run: `cargo nextest run -p oku-slides-stt -p oku-slides-i18n`
Expected: PASS.

- [ ] **Step 3: Tests du flux d'événements en échec (stream_tests.rs, results_tests.rs)**

`stream_tests.rs` :

```rust
use speculoos::prelude::*;

use super::*;
use crate::SttErrorKind;

#[test]
fn from_raw_partial_keeps_text() {
    let event = Event::from_raw(raw::PARTIAL, "bonj".into(), 0.0, 0.0);
    assert_that!(matches!(event, Event::Partial(t) if t == "bonj")).is_true();
}

#[test]
fn from_raw_final_carries_times() {
    let event = Event::from_raw(raw::FINAL, "Bonjour.".into(), 1.5, 2.5);
    assert_that!(matches!(event, Event::Final { text, start, end } if text == "Bonjour." && start == 1.5 && end == 2.5))
        .is_true();
}

#[test]
fn from_raw_progress_carries_counts() {
    let event = Event::from_raw(raw::PROGRESS, String::new(), 10.0, 40.0);
    assert_that!(matches!(event, Event::Progress { received: 10, total: 40 })).is_true();
}

#[test]
fn from_raw_done_ends_the_stream() {
    assert_that!(matches!(Event::from_raw(raw::DONE, String::new(), 0.0, 0.0), Event::Done)).is_true();
}

#[test]
fn from_raw_error_codes_map_to_error_kinds() {
    let cases = [
        (raw::ERR_ANALYZER, SttErrorKind::Transcription),
        (raw::ERR_LOCALE, SttErrorKind::Model),
        (raw::ERR_ASSET, SttErrorKind::Download),
        (raw::ERR_DENIED, SttErrorKind::Permission),
        (99.0, SttErrorKind::Transcription),
    ];
    for (code, kind) in cases {
        let Event::Failed(e) = Event::from_raw(raw::ERROR, "x".into(), code, 0.0) else {
            panic!("code {code} : pas une erreur");
        };
        asserting(&format!("code {code}")).that(&e.kind()).is_equal_to(kind);
    }
}

#[test]
fn no_stream_engine_supports_nothing() {
    assert_that!(NoStreamEngine.supported()).is_false();
    assert_that!(NoStreamEngine.availability("fr")).is_equal_to(Availability::Unsupported);
}

#[test]
fn no_stream_engine_open_reports_unsupported_language() {
    let (_stream, events) = NoStreamEngine.open("fr", &[]);
    let event = events.recv().unwrap();
    assert_that!(matches!(event, Event::Failed(e) if e.kind() == SttErrorKind::Model)).is_true();
}
```

`results_tests.rs` :

```rust
use speculoos::prelude::*;

use super::*;

#[test]
fn apply_partial_replaces_preview() {
    let (mut phrases, mut partial) = (Vec::new(), "ancien".to_owned());
    assert_that!(apply(&mut phrases, &mut partial, &Event::Partial("  bonjour  à ".into()))).is_true();
    assert_that!(partial).is_equal_to("bonjour à".to_owned());
}

#[test]
fn apply_final_appends_clean_text_and_clears_preview() {
    let (mut phrases, mut partial) = (vec![Some("Un.".to_owned())], "deu".to_owned());
    apply(&mut phrases, &mut partial, &Event::Final { text: " Deux. ".into(), start: 0.0, end: 1.0 });
    assert_that!(phrases).is_equal_to(vec![Some("Un.".to_owned()), Some("Deux.".to_owned())]);
    assert_that!(partial).is_equal_to(String::new());
}

#[test]
fn apply_empty_final_adds_nothing() {
    let (mut phrases, mut partial) = (Vec::new(), String::new());
    apply(&mut phrases, &mut partial, &Event::Final { text: "   ".into(), start: 0.0, end: 1.0 });
    assert_that!(phrases).is_empty();
}

#[test]
fn apply_other_events_are_not_results() {
    let (mut phrases, mut partial) = (Vec::new(), String::new());
    assert_that!(apply(&mut phrases, &mut partial, &Event::Done)).is_false();
}
```

Run: `cargo nextest run -p oku-slides-stt from_raw apply no_stream`
Expected: FAIL à la compilation.

- [ ] **Step 4: Implémenter `stream.rs` et `results.rs`**

`stream.rs` :

```rust
//! Moteurs de dictée en continu : on leur pousse l'audio, ils répondent par des événements.
//!
//! - [`StreamEngine`] : un moteur (Apple sur macOS 26+, voir `apple.rs`).
//! - [`Stream`] : une dictée ouverte sur ce moteur.
//! - [`Event`] : ses résultats, traduits du pont par [`Event::from_raw`].

use std::sync::{Arc, mpsc};

use oku_slides_platform::config::SttSettings;

use crate::error::{AnalyzerSnafu, AssetInstallSnafu, LocaleUnsupportedSnafu, SpeechDeniedSnafu, SttError};

/// Disponibilité d'une langue pour un moteur en continu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Availability {
    /// Moteur absent, ou langue inconnue de lui.
    Unsupported,
    /// Langue connue, à télécharger.
    Missing,
    Ready,
}

/// Ce qu'un moteur en continu rend.
#[derive(Debug)]
pub(crate) enum Event {
    /// Texte provisoire de ce qui suit le dernier texte définitif.
    Partial(String),
    /// Texte définitif, et sa place dans l'audio (s).
    Final { text: String, start: f32, end: f32 },
    /// Téléchargement d'une langue.
    Progress { received: u64, total: u64 },
    /// Plus rien ne viendra.
    Done,
    Failed(SttError),
}

/// Codes du pont Swift (`swift/Sources/OkuSpeech/OkuSpeech.swift`).
pub(crate) mod raw {
    pub(crate) const PARTIAL: i32 = 0;
    pub(crate) const FINAL: i32 = 1;
    pub(crate) const PROGRESS: i32 = 2;
    pub(crate) const DONE: i32 = 3;
    pub(crate) const ERROR: i32 = 4;
    pub(crate) const ERR_ANALYZER: f64 = 1.0;
    pub(crate) const ERR_LOCALE: f64 = 2.0;
    pub(crate) const ERR_ASSET: f64 = 3.0;
    pub(crate) const ERR_DENIED: f64 = 4.0;
}

impl Event {
    /// Traduit un événement brut du pont (`a`, `b` : temps, compteurs ou code d'erreur).
    pub(crate) fn from_raw(kind: i32, text: String, a: f64, b: f64) -> Event {
        match kind {
            raw::PARTIAL => Event::Partial(text),
            raw::FINAL => Event::Final { text, start: a as f32, end: b as f32 },
            raw::PROGRESS => Event::Progress { received: a.max(0.0) as u64, total: b.max(0.0) as u64 },
            raw::DONE => Event::Done,
            _ => Event::Failed(bridge_error(a, text)),
        }
    }
}

/// Erreur du pont : son code, et son message (une langue pour `ERR_LOCALE`).
fn bridge_error(code: f64, message: String) -> SttError {
    if code == raw::ERR_LOCALE {
        LocaleUnsupportedSnafu { locale: message }.build().into()
    } else if code == raw::ERR_ASSET {
        AssetInstallSnafu { reason: message }.build().into()
    } else if code == raw::ERR_DENIED {
        SpeechDeniedSnafu.build().into()
    } else {
        AnalyzerSnafu { reason: message }.build().into()
    }
}

/// Une dictée ouverte sur un moteur en continu.
pub(crate) trait Stream: Send {
    /// Audio mono à 16 kHz.
    fn push(&mut self, samples: &[f32]);
    /// Fin de l'audio : les derniers textes définitifs, puis `Done`.
    fn finish(&mut self);
    /// Abandon : plus rien d'utile ne viendra.
    fn cancel(&mut self);
}

/// Un moteur de dictée en continu.
///
/// Implémentez ce trait pour brancher un moteur qui transcrit au fil de
/// l'audio (le moteur d'Apple ; un moteur factice dans les tests).
pub(crate) trait StreamEngine: Send + Sync {
    /// Le moteur existe sur cette machine.
    fn supported(&self) -> bool;
    /// Disponibilité de la langue `locale` (`fr`, `fr-FR`).
    fn availability(&self, locale: &str) -> Availability;
    /// Installe la langue : `Progress`…, puis `Done` ou `Failed`.
    fn install(&self, locale: &str) -> mpsc::Receiver<Event>;
    /// Ouvre une dictée ; ses résultats arrivent sur le canal rendu.
    fn open(&self, locale: &str, vocabulary: &[String]) -> (Box<dyn Stream>, mpsc::Receiver<Event>);
}

/// Le moteur en continu de cette machine, selon les réglages.
pub(crate) fn engine(_settings: &SttSettings) -> Arc<dyn StreamEngine> {
    Arc::new(NoStreamEngine)
}

/// Pas de moteur en continu (hors macOS, ou `stt.apple = false`).
#[derive(Debug)]
pub(crate) struct NoStreamEngine;

impl StreamEngine for NoStreamEngine {
    fn supported(&self) -> bool {
        false
    }

    fn availability(&self, _locale: &str) -> Availability {
        Availability::Unsupported
    }

    fn install(&self, locale: &str) -> mpsc::Receiver<Event> {
        unsupported(locale)
    }

    fn open(&self, locale: &str, _vocabulary: &[String]) -> (Box<dyn Stream>, mpsc::Receiver<Event>) {
        (Box::new(NoStream), unsupported(locale))
    }
}

/// Un canal qui ne dit qu'une chose : langue inconnue.
fn unsupported(locale: &str) -> mpsc::Receiver<Event> {
    let (tx, rx) = mpsc::channel();
    tx.send(Event::Failed(LocaleUnsupportedSnafu { locale }.build().into())).ok();
    rx
}

#[derive(Debug)]
struct NoStream;

impl Stream for NoStream {
    fn push(&mut self, _samples: &[f32]) {}
    fn finish(&mut self) {}
    fn cancel(&mut self) {}
}

#[cfg(test)]
#[path = "stream_tests.rs"]
mod stream_tests;
```

`results.rs` :

```rust
//! Résultats d'un moteur en continu appliqués au texte de la dictée.

use crate::clean;
use crate::stream::Event;

/// Applique un résultat : renvoie `false` si l'événement n'en est pas un.
///
/// Un texte provisoire remplace l'aperçu ; un texte définitif s'ajoute aux
/// phrases et vide l'aperçu.
pub(crate) fn apply(phrases: &mut Vec<Option<String>>, partial: &mut String, event: &Event) -> bool {
    match event {
        Event::Partial(text) => {
            *partial = clean(text);
            true
        }
        Event::Final { text, .. } => {
            let text = clean(text);
            if !text.is_empty() {
                phrases.push(Some(text));
            }
            partial.clear();
            true
        }
        _ => false,
    }
}

#[cfg(test)]
#[path = "results_tests.rs"]
mod results_tests;
```

`lib.rs` : `mod results; mod stream;` (ordre alphabétique). Rien ne les utilise encore hors tests : en tête de `stream.rs` et de `results.rs`, juste après le commentaire de module, ajouter `#![cfg_attr(not(test), expect(dead_code, reason = "branché par dictation_stream (Task 5, t-393)"))]`. Les deux lignes sont retirées à la Task 5 (à ce moment, `expect` échouerait s'il restait sans objet).

Run: `cargo nextest run -p oku-slides-stt && cargo clippy -p oku-slides-stt --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-stt/src crates/oku-slides-i18n/src/locales
```
```bash
git commit -m "feat(stt) : événements des moteurs de dictée en continu et erreurs du moteur d'Apple (t-393)"
```

---

### Task 4: Modèle Apple, langue et bascule des réglages

`Model::Apple` apparaît dans `Status` quand un moteur en continu existe ; la langue se résout ; les anciens réglages basculent une fois.

**Files:**
- Create: `crates/oku-slides-stt/src/locale.rs`, `crates/oku-slides-stt/src/locale_tests.rs`
- Modify: `crates/oku-slides-stt/src/models.rs` (`Model::Apple`, `ALL`, `repo`, `files`, `size`, `label`, `is_installed`, `download` refuse Apple)
- Modify: `crates/oku-slides-stt/src/lib.rs` (`Config.version`, `CONFIG_VERSION`, `migrate`, `ModelInfo.available`, `Inner.apple`, `Stt.streaming`, `Stt.apple_supported`, `with_engine`, `refresh_apple`, `model_info`, `configure`, `start`, `report`, `wanted_language`)
- Modify: `crates/oku-slides-stt/src/test_support.rs` (`ScriptedEngine`, `french_platform`)
- Modify: `crates/oku-slides-stt/src/lib_tests.rs`
- Modify: `crates/oku-slides-stt/Cargo.toml` (`sys-locale.workspace = true`)
- Modify: `crates/oku-slides-platform/src/config.rs` (`SttSettings.apple`), `config.default.toml`, `config_tests.rs`
- Modify: `crates/oku-slides-stt/tests/dictation.rs` (`settings.stt.apple = false`)
- Modify: `crates/oku-slides-i18n/src/locales/*/stt.ftl` (`stt-model-apple`, 20 langues)

**Interfaces:**
- Consumes: Task 3 (`Availability`, `Event`, `Stream`, `StreamEngine`, `NoStreamEngine`, `stream::engine`, `LocaleUnsupportedSnafu`, `NotDownloadableSnafu`).
- Produces:
  - `Model::Apple` ; `pub struct ModelInfo { …, pub available: bool }` ; `pub struct Config { …, pub version: u32 }` ; `const CONFIG_VERSION: u32 = 1` ; `fn migrate(config: Config, apple: bool) -> Config`.
  - `Inner.apple: Option<(String, Availability)>` (étiquette de langue retenue et sa disponibilité).
  - `Stt { streaming: Arc<dyn StreamEngine>, apple_supported: bool, … }` ; `pub(crate) fn Stt::with_engine(platform: &Platform, streaming: Arc<dyn StreamEngine>) -> Stt` ; `fn refresh_apple(&self)` ; `fn report(&self, e: SttError)` ; `fn wanted_language(&self) -> String`.
  - `locale::resolve(chosen: Option<&str>, ui: &str, system: Option<&str>, availability: impl Fn(&str) -> Availability) -> Option<(String, Availability)>` ; `locale::language(tag: &str) -> String`.
  - `SttSettings.apple: bool`.
  - Tests : `test_support::ScriptedEngine::new(french: Availability, finals: &[&str]) -> ScriptedEngine` (champs `hang`, `fail`, `pushed`, `cancelled`) ; `test_support::french_platform(dir: &Path, tune: impl FnOnce(&mut SttSettings)) -> Platform` ; `test_support::wait_until(stt: &Stt, what: &str, done: impl Fn(&Status) -> bool) -> Status`.

- [ ] **Step 1: Réglage `stt.apple` (plateforme), test d'abord**

`config_tests.rs`, à côté des tests des défauts `stt` :

```rust
#[test]
fn defaults_stt_offers_apple_engine() {
    let s = Settings::defaults().unwrap();
    assert_that!(s.stt.apple).is_true();
}
```

Run: `cargo nextest run -p oku-slides-platform defaults_stt_offers_apple_engine` → FAIL (champ inconnu).

`config.rs`, `SttSettings` (avant `vad`) : `/// Proposer le moteur de dictée d'Apple (macOS 26+). pub apple: bool,`. `config.default.toml`, `[stt]`, avant `# Micro simulé` :

```toml
# Proposer le moteur de dictée d'Apple (macOS 26+), choisi par défaut ;
# false : Whisper seulement.
apple = true
```

Run: `cargo nextest run -p oku-slides-platform` → PASS.

- [ ] **Step 2: `locale.rs`, test d'abord**

`locale_tests.rs` :

```rust
use speculoos::prelude::*;

use super::*;

/// Le français est prêt, l'allemand à télécharger, le reste inconnu.
fn engine(tag: &str) -> Availability {
    match language(tag).as_str() {
        "fr" => Availability::Ready,
        "de" => Availability::Missing,
        _ => Availability::Unsupported,
    }
}

#[test]
fn resolve_auto_takes_interface_language() {
    assert_that!(resolve(None, "fr", Some("de-DE"), engine)).is_equal_to(Some(("fr".to_owned(), Availability::Ready)));
}

#[test]
fn resolve_auto_unknown_interface_language_falls_back_to_system() {
    assert_that!(resolve(None, "th", Some("de-DE"), engine))
        .is_equal_to(Some(("de-DE".to_owned(), Availability::Missing)));
}

#[test]
fn resolve_chosen_language_ignores_interface() {
    assert_that!(resolve(Some("de"), "fr", None, engine)).is_equal_to(Some(("de".to_owned(), Availability::Missing)));
}

#[test]
fn resolve_chosen_unknown_language_returns_none() {
    assert_that!(resolve(Some("xx"), "fr", Some("fr-FR"), engine)).is_none();
}

#[test]
fn language_region_tag_returns_language_code() {
    assert_that!(language("fr_FR")).is_equal_to("fr".to_owned());
}
```

Run: `cargo nextest run -p oku-slides-stt resolve language_region` → FAIL (module absent).

`locale.rs` :

```rust
//! Langue du moteur d'Apple : il en exige une, il ne la détecte pas.

use crate::stream::Availability;

/// Première langue connue du moteur : la choisie, sinon l'interface, sinon le système.
///
/// Renvoie l'étiquette retenue (`fr`, `fr-FR`) et sa disponibilité ; `None`
/// si le moteur n'en connaît aucune. Une langue choisie n'a pas de repli.
pub(crate) fn resolve(
    chosen: Option<&str>,
    ui: &str,
    system: Option<&str>,
    availability: impl Fn(&str) -> Availability,
) -> Option<(String, Availability)> {
    let candidates: Vec<&str> = match chosen {
        Some(language) => vec![language],
        None => [Some(ui), system].into_iter().flatten().collect(),
    };
    candidates.into_iter().find_map(|tag| match availability(tag) {
        Availability::Unsupported => None,
        found => Some((tag.to_owned(), found)),
    })
}

/// Code de langue d'une étiquette : `fr-FR` → `fr`.
pub(crate) fn language(tag: &str) -> String {
    tag.split(['-', '_']).next().unwrap_or(tag).to_ascii_lowercase()
}

#[cfg(test)]
#[path = "locale_tests.rs"]
mod locale_tests;
```

`lib.rs` : `mod locale;`. Run → PASS.

- [ ] **Step 3: `Model::Apple` (models.rs) et `stt-model-apple`**

Test (lib_tests.rs) :

```rust
#[test]
fn download_apple_model_is_refused() {
    let dir = test_support::fresh_dir("apple-download");
    let settings = oku_slides_platform::config::Settings::defaults().unwrap();
    let storage = oku_slides_platform::storage::Storage::local().unwrap();
    let err = download(Model::Apple, dir.path(), &settings.stt, &storage, |_, _| true).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SttErrorKind::Model);
}
```

Run → FAIL. Implémenter dans `models.rs` :

```rust
pub enum Model {
    /// Moteur d'Apple (macOS 26+) : en continu, langue installée par le système.
    Apple,
    /// large-v3-turbo quantifié int8 : très bon compromis qualité / vitesse.
    #[default]
    Turbo,
    // … Large, Base inchangés
}

impl Model {
    pub const ALL: [Model; 4] = [Model::Apple, Model::Turbo, Model::Large, Model::Base];
```

- `repo()` : `Model::Apple => "apple",` (dossier jamais créé).
- `files()` : en tête, `if self == Model::Apple { return Vec::new(); }`.
- `size()` : `Model::Apple => 0,`. `label()` : `Model::Apple => "stt-model-apple",`.
- `is_installed` : `self != Model::Apple && storage.exists(…)…`.
- `download` : en tête, `ensure!(model != Model::Apple, NotDownloadableSnafu);` (importer `snafu::ensure` et `NotDownloadableSnafu`).

Fluent `stt.ftl`, après `stt-model-base` : fr `stt-model-apple = Apple (système)`, en `stt-model-apple = Apple (system)`, et les 18 autres langues (« système » traduit, « Apple » tel quel).

Run: `cargo nextest run -p oku-slides-stt -p oku-slides-i18n` → PASS (le tableau `installed` suit `Model::ALL.len()`).

- [ ] **Step 4: Bascule des réglages, test d'abord (lib_tests.rs)**

```rust
fn config(model: Model, version: u32) -> Config {
    Config { model, version, ..Config::default() }
}

#[test]
fn migrate_old_config_switches_to_apple_when_available() {
    assert_that!(migrate(config(Model::Turbo, 0), true).model).is_equal_to(Model::Apple);
}

#[test]
fn migrate_old_config_without_apple_keeps_model() {
    assert_that!(migrate(config(Model::Large, 0), false).model).is_equal_to(Model::Large);
}

#[test]
fn migrate_current_config_keeps_user_choice() {
    assert_that!(migrate(config(Model::Base, CONFIG_VERSION), true).model).is_equal_to(Model::Base);
}

#[test]
fn migrate_apple_on_machine_without_it_falls_back_to_turbo() {
    assert_that!(migrate(config(Model::Apple, CONFIG_VERSION), false).model).is_equal_to(Model::Turbo);
}

#[test]
fn config_json_without_version_reads_version_zero() {
    let c: Config = serde_json::from_str(r#"{"model":"turbo"}"#).unwrap();
    assert_that!(c.version).is_equal_to(0);
}
```

Run → FAIL. Implémenter (lib.rs) :

```rust
/// Version des réglages de la dictée : 1 depuis le moteur d'Apple.
const CONFIG_VERSION: u32 = 1;

pub struct Config {
    // … champs existants
    /// Version des réglages : 0 avant le moteur d'Apple (bascule unique).
    pub version: u32,
}
// Default : version: 0

/// Réglages d'avant le moteur d'Apple : on y passe une fois, s'il existe ici.
fn migrate(mut config: Config, apple: bool) -> Config {
    if config.version < CONFIG_VERSION {
        if apple {
            config.model = Model::Apple;
        }
        config.version = CONFIG_VERSION;
    }
    if config.model == Model::Apple && !apple {
        config.model = Model::Turbo;
    }
    config
}
```

Run → PASS.

- [ ] **Step 5: Outillage de test `ScriptedEngine` (test_support.rs)**

```rust
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::{Duration, Instant};

use oku_slides_platform::Platform;
use oku_slides_platform::config::{Settings, SttSettings};

use crate::stream::{Availability, Event, Stream, StreamEngine};
use crate::{Status, Stt, locale};

/// Un moteur en continu factice : français seulement, rejoue `finals` à `finish`.
#[derive(Debug)]
pub(crate) struct ScriptedEngine {
    french: Mutex<Availability>,
    finals: Vec<String>,
    /// `finish` ne rend jamais `Done`.
    pub hang: bool,
    /// Le premier `push` rend une erreur du moteur.
    pub fail: bool,
    /// Échantillons reçus ; `cancel` appelé.
    pub pushed: Arc<AtomicUsize>,
    pub cancelled: Arc<AtomicBool>,
    /// Canaux gardés ouverts par `hang`.
    kept: Arc<Mutex<Vec<mpsc::Sender<Event>>>>,
}

impl ScriptedEngine {
    pub(crate) fn new(french: Availability, finals: &[&str]) -> Self {
        Self {
            french: Mutex::new(french),
            finals: finals.iter().map(|s| (*s).to_owned()).collect(),
            hang: false,
            fail: false,
            pushed: Arc::default(),
            cancelled: Arc::default(),
            kept: Arc::default(),
        }
    }

    fn french(&self) -> std::sync::MutexGuard<'_, Availability> {
        self.french.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl StreamEngine for ScriptedEngine {
    fn supported(&self) -> bool {
        true
    }

    fn availability(&self, tag: &str) -> Availability {
        if locale::language(tag) == "fr" { *self.french() } else { Availability::Unsupported }
    }

    fn install(&self, _tag: &str) -> mpsc::Receiver<Event> {
        *self.french() = Availability::Ready;
        let (tx, rx) = mpsc::channel();
        tx.send(Event::Progress { received: 1, total: 2 }).ok();
        tx.send(Event::Progress { received: 2, total: 2 }).ok();
        tx.send(Event::Done).ok();
        rx
    }

    fn open(&self, _tag: &str, _vocabulary: &[String]) -> (Box<dyn Stream>, mpsc::Receiver<Event>) {
        let (tx, rx) = mpsc::channel();
        let stream = ScriptedStream {
            tx,
            finals: self.finals.clone(),
            hang: self.hang,
            fail: self.fail,
            pushed: self.pushed.clone(),
            cancelled: self.cancelled.clone(),
            kept: self.kept.clone(),
        };
        (Box::new(stream), rx)
    }
}

#[derive(Debug)]
struct ScriptedStream {
    tx: mpsc::Sender<Event>,
    finals: Vec<String>,
    hang: bool,
    fail: bool,
    pushed: Arc<AtomicUsize>,
    cancelled: Arc<AtomicBool>,
    kept: Arc<Mutex<Vec<mpsc::Sender<Event>>>>,
}

impl Stream for ScriptedStream {
    fn push(&mut self, samples: &[f32]) {
        if self.pushed.fetch_add(samples.len(), Ordering::Relaxed) == 0 {
            let event = if self.fail {
                Event::Failed(crate::error::AnalyzerSnafu { reason: "panne" }.build().into())
            } else {
                Event::Partial(" bonjour ".into())
            };
            self.tx.send(event).ok();
        }
    }

    fn finish(&mut self) {
        for (i, text) in self.finals.iter().enumerate() {
            let start = i as f32;
            self.tx.send(Event::Final { text: text.clone(), start, end: start + 1.0 }).ok();
        }
        if self.hang {
            self.kept.lock().unwrap_or_else(PoisonError::into_inner).push(self.tx.clone());
        } else {
            self.tx.send(Event::Done).ok();
        }
    }

    fn cancel(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

/// Un WAV muet de `secs` secondes à 16 kHz, en guise de micro.
pub(crate) fn silent_wav(dir: &Path, secs: f32) -> PathBuf {
    let path = dir.join("micro.wav");
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&path, spec).unwrap_or_else(|e| panic!("wav : {e}"));
    for _ in 0..(secs * 16_000.0) as usize {
        writer.write_sample(0i16).unwrap_or_else(|e| panic!("wav : {e}"));
    }
    writer.finalize().unwrap_or_else(|e| panic!("wav : {e}"));
    path
}

/// Une plateforme en français, ses modèles dans `dir`, un micro muet ; `tune` ajuste la dictée.
pub(crate) fn french_platform(dir: &Path, tune: impl FnOnce(&mut SttSettings)) -> Platform {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.models = Some(dir.join("models"));
    settings.i18n.locale = Some("fr".into());
    settings.stt.fake_mic = Some(silent_wav(dir, 1.0));
    tune(&mut settings.stt);
    Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme : {e}"))
}

/// Attend que l'état de la dictée vérifie `done` (30 s au plus).
pub(crate) fn wait_until(stt: &Stt, what: &str, done: impl Fn(&Status) -> bool) -> Status {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let status = stt.status();
        if done(&status) {
            return status;
        }
        assert!(Instant::now() < deadline, "attente trop longue : {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}
```

(Le `use` de `PathBuf` sert déjà à `fresh_dir` ; garder les imports cohérents.)

- [ ] **Step 6: Statut, langue et départ avec Apple, test d'abord (lib_tests.rs)**

```rust
use std::sync::Arc;

use crate::stream::{Availability, NoStreamEngine};
use crate::test_support::{ScriptedEngine, french_platform, fresh_dir};

fn apple_stt(dir: &Path, french: Availability) -> Stt {
    Stt::with_engine(&french_platform(dir, |_| {}), Arc::new(ScriptedEngine::new(french, &[])))
}

#[test]
fn status_with_stream_engine_lists_apple_first_and_ready() {
    let dir = fresh_dir("apple-status");
    let status = apple_stt(dir.path(), Availability::Ready).status();
    let apple = &status.models[0];
    assert_that!((apple.id, apple.installed, apple.available)).is_equal_to((Model::Apple, true, true));
}

#[test]
fn new_config_with_stream_engine_chooses_apple() {
    let dir = fresh_dir("apple-default");
    assert_that!(apple_stt(dir.path(), Availability::Ready).status().config.model).is_equal_to(Model::Apple);
}

#[test]
fn status_with_apple_auto_language_reports_interface_language() {
    let dir = fresh_dir("apple-detected");
    assert_that!(apple_stt(dir.path(), Availability::Ready).status().detected).is_equal_to(Some("fr".to_owned()));
}

#[test]
fn status_without_stream_engine_omits_apple() {
    let dir = fresh_dir("apple-none");
    let stt = Stt::with_engine(&french_platform(dir.path(), |_| {}), Arc::new(NoStreamEngine));
    assert_that!(stt.status().models.iter().any(|m| m.id == Model::Apple)).is_false();
}

#[test]
fn configure_unknown_language_makes_apple_unavailable() {
    let dir = fresh_dir("apple-xx");
    let stt = apple_stt(dir.path(), Availability::Ready);
    stt.configure(Config { language: Some("xx".into()), ..stt.status().config });
    assert_that!(stt.status().models[0].available).is_false();
}

#[test]
fn configure_always_writes_current_version() {
    let dir = fresh_dir("apple-version");
    let stt = apple_stt(dir.path(), Availability::Ready);
    stt.configure(Config { version: 0, ..stt.status().config });
    assert_that!(stt.status().config.version).is_equal_to(CONFIG_VERSION);
}

#[test]
fn start_apple_with_unknown_language_reports_error() {
    let dir = fresh_dir("apple-start-xx");
    let stt = apple_stt(dir.path(), Availability::Ready);
    stt.configure(Config { language: Some("xx".into()), ..stt.status().config });
    stt.start(Vec::new());
    assert_that!(stt.status().error.as_deref())
        .is_equal_to(Some("Le moteur d'Apple ne prend pas en charge cette langue (xx)"));
}
```

Run: `cargo nextest run -p oku-slides-stt` → FAIL (compilation).

- [ ] **Step 7: Implémenter (lib.rs)**

`Cargo.toml` du crate : `sys-locale.workspace = true`.

```rust
#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub id: Model,
    pub label: String,
    pub size: u64,
    pub installed: bool,
    /// Faux : le modèle ne gère pas la langue choisie.
    pub available: bool,
}
```

`Inner` : `/// Langue du moteur d'Apple et sa disponibilité (None : aucune langue connue). apple: Option<(String, Availability)>,` (initialisé à `None`). `Stt` : `/// Moteur en continu (Apple), et s'il existe ici. streaming: Arc<dyn StreamEngine>, apple_supported: bool,`.

```rust
    /// La dictée de la plateforme : dossier des modèles et rythmes de
    /// l'écoute (réglages), fichiers, langue des messages.
    pub fn new(platform: &Platform) -> Self {
        Self::with_engine(platform, stream::engine(&platform.settings.stt))
    }

    /// La dictée, avec ce moteur en continu (Apple, ou un moteur factice en test).
    pub(crate) fn with_engine(platform: &Platform, streaming: Arc<dyn StreamEngine>) -> Self {
        let dir = platform.settings.paths.models();
        let storage = platform.storage.clone();
        let apple_supported = streaming.supported();
        // Réglages absents ou illisibles : ceux par défaut.
        let config =
            storage.read(&config_path(&dir)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let config = migrate(config, apple_supported);
        let stt = Self {
            inner: Arc::new(Mutex::new(Inner { /* … champs existants …, */ apple: None, /* … */ })),
            dir,
            settings: Arc::new(platform.settings.stt.clone()),
            storage,
            localizer: platform.localizer.clone(),
            cancel_download: Arc::new(AtomicBool::new(false)),
            streaming,
            apple_supported,
        };
        stt.refresh_apple();
        stt
    }

    /// Langue du moteur d'Apple et sa disponibilité, d'après les réglages.
    fn refresh_apple(&self) {
        if !self.apple_supported {
            return;
        }
        let chosen = self.state().config.language.clone();
        let system = sys_locale::get_locale();
        let resolved = locale::resolve(chosen.as_deref(), self.localizer.locale(), system.as_deref(), |tag| {
            self.streaming.availability(tag)
        });
        let mut g = self.state();
        if g.config.model == Model::Apple {
            g.detected = resolved.as_ref().map(|(tag, _)| locale::language(tag));
        }
        g.apple = resolved;
    }

    /// Langue voulue pour la dictée : la choisie, sinon celle de l'interface.
    fn wanted_language(&self) -> String {
        self.state().config.language.clone().unwrap_or_else(|| self.localizer.locale().to_owned())
    }

    /// Signale une erreur à l'interface (et au journal).
    fn report(&self, e: SttError) {
        tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "dictée");
        self.state().error = Some(e.localize(&self.localizer));
    }

    fn model_info(&self, model: Model, installed: bool, g: &Inner) -> ModelInfo {
        let (installed, available) = match model {
            Model::Apple => (matches!(g.apple, Some((_, Availability::Ready))), g.apple.is_some()),
            _ => (installed, true),
        };
        ModelInfo { id: model, label: model.label(&self.localizer), size: model.size(), installed, available }
    }
```

`status()` :

```rust
            models: Model::ALL
                .iter()
                .zip(g.installed)
                .filter(|(m, _)| **m != Model::Apple || self.apple_supported)
                .map(|(&m, installed)| self.model_info(m, installed, &g))
                .collect(),
```

`configure()` : `config.version = CONFIG_VERSION;` après le `clamp` ; le corps existant passe dans un bloc `{ let mut g = self.state(); … g.config = config; }`, suivi de `self.refresh_apple();`.

`start()` :

```rust
    /// Démarre la dictée (téléchargement du modèle d'abord s'il manque).
    pub fn start(&self, vocabulary: Vec<String>) {
        let (model, apple) = {
            let mut g = self.state();
            if g.state != State::Idle {
                return;
            }
            g.error = None;
            g.vocabulary = vocabulary;
            (g.config.model, g.apple.clone())
        };
        if model == Model::Apple {
            if apple.is_none() {
                self.report(LocaleUnsupportedSnafu { locale: self.wanted_language() }.build().into());
            }
            return;
        }
        if model.is_installed(&self.storage, &self.dir) {
            self.begin_recording();
        } else {
            self.download(model, true);
        }
    }
```

(Les cas `Ready` et `Missing` d'Apple arrivent à la Task 5 ; d'ici là, aucun moteur réel n'existe : `stream::engine` rend `NoStreamEngine`. Retirer le `let _streaming` provisoire de la Task 3.)

`tests/dictation.rs`, `platform()`, et `lib_tests.rs`, `platform()` : ajouter `settings.stt.apple = false;` (les tests de Whisper passant par `Stt::new` restent déterministes sur un Mac sous macOS 26, une fois le vrai moteur branché à la Task 6).

Run: `cargo nextest run -p oku-slides-stt -p oku-slides-platform -p oku-slides-i18n && cargo clippy -p oku-slides-stt --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add crates/oku-slides-stt crates/oku-slides-platform/src crates/oku-slides-i18n/src/locales Cargo.lock
```
```bash
git commit -m "feat(stt) : modèle Apple, langue résolue et bascule unique des réglages (t-393)"
```

---

### Task 5: Dictée en continu

`listen_loop` alimente le moteur en continu ; un fil applique ses résultats ; la langue se télécharge ; `transcribe_samples` sait passer par Apple.

**Files:**
- Create: `crates/oku-slides-stt/src/dictation_stream.rs`, `crates/oku-slides-stt/src/dictation_stream_tests.rs`
- Modify: `crates/oku-slides-stt/src/lib.rs` (`Sink`, `begin_recording`, `listen_loop`, `pull`, `download`, `finish_download`, `start`, `transcribe_samples`)
- Modify: `crates/oku-slides-stt/src/stream.rs`, `src/results.rs` (retirer le `expect(dead_code)` de la Task 3)
- Modify: `crates/oku-slides-platform/src/config.rs`, `config.default.toml`, `config_tests.rs` (`apple_finish_timeout_secs`)

**Interfaces:**
- Consumes: Tasks 1, 3, 4 (`Job`, `send_final`, `transcribe_loop`, `Inner::complete`, `Inner::reset`, `Event`, `Stream`, `results::apply`, `Inner.apple`, `refresh_apple`, `report`, `wanted_language`, `ScriptedEngine`, `french_platform`, `wait_until`).
- Produces: `enum Sink { Whisper { jobs, busy }, Stream(Box<dyn Stream>) }` ; `Stt::stream_loop(&self, session: u64, events: mpsc::Receiver<Event>)` ; `Stt::download_language(&self, then_start: bool)` ; `Stt::install(&self, tag: &str) -> Result<(), SttError>` ; `Stt::transcribe_stream(&self, audio: &[f32]) -> Result<(String, Vec<TimedPhrase>), SttError>` ; `Stt::finish_download(&self, model: Model, result: Result<(), SttError>) -> bool` ; `SttSettings.apple_finish_timeout_secs: f32`.

- [ ] **Step 1: Réglage du délai (plateforme), test d'abord**

```rust
#[test]
fn defaults_stt_apple_finish_timeout_is_five_seconds() {
    let s = Settings::defaults().unwrap();
    assert_that!(s.stt.apple_finish_timeout_secs).is_equal_to(5.0);
}
```

Run → FAIL. `SttSettings` : `/// Attente maximale du texte final du moteur d'Apple après l'arrêt (s). pub apple_finish_timeout_secs: f32,`. TOML, sous `apple = true` :

```toml
# Attente maximale du texte final du moteur d'Apple après l'arrêt (s).
apple_finish_timeout_secs = 5.0
```

Run: `cargo nextest run -p oku-slides-platform` → PASS.

- [ ] **Step 2: Tests du flux en échec (dictation_stream_tests.rs)**

```rust
use std::sync::Arc;
use std::sync::atomic::Ordering;

use speculoos::prelude::*;

use crate::stream::Availability;
use crate::test_support::{ScriptedEngine, french_platform, fresh_dir, wait_until};
use crate::{State, Stt, SttRequest};

fn stt_with(dir: &std::path::Path, engine: ScriptedEngine, tune: impl FnOnce(&mut oku_slides_platform::config::SttSettings)) -> (Stt, Arc<ScriptedEngine>) {
    let engine = Arc::new(engine);
    (Stt::with_engine(&french_platform(dir, tune), engine.clone()), engine)
}

#[test]
fn apple_dictation_shows_partial_while_listening() {
    let dir = fresh_dir("stream-partial");
    let (stt, _) = stt_with(dir.path(), ScriptedEngine::new(Availability::Ready, &["Bonjour à tous."]), |_| {});
    stt.handle(SttRequest::Start { vocabulary: vec!["Oku".into()] });
    let live = wait_until(&stt, "aperçu", |s| s.partial == "bonjour");
    assert_that!(live.state).is_equal_to(State::Recording);
}

#[test]
fn apple_dictation_stop_returns_final_text() {
    let dir = fresh_dir("stream-stop");
    let (stt, _) = stt_with(dir.path(), ScriptedEngine::new(Availability::Ready, &["Bonjour à tous."]), |_| {});
    stt.handle(SttRequest::Start { vocabulary: Vec::new() });
    wait_until(&stt, "aperçu", |s| s.partial == "bonjour");
    stt.handle(SttRequest::Stop);
    let done = wait_until(&stt, "fin", |s| s.state == State::Idle);
    assert_that!((done.text.as_deref(), done.seq)).is_equal_to((Some("Bonjour à tous."), 1));
}

#[test]
fn apple_missing_language_is_downloaded_then_listening_starts() {
    let dir = fresh_dir("stream-install");
    let (stt, _) = stt_with(dir.path(), ScriptedEngine::new(Availability::Missing, &[]), |_| {});
    stt.handle(SttRequest::Start { vocabulary: Vec::new() });
    let live = wait_until(&stt, "écoute après téléchargement", |s| s.state == State::Recording);
    assert_that!(live.models[0].installed).is_true();
}

#[test]
fn apple_dictation_cancel_cancels_the_stream() {
    let dir = fresh_dir("stream-cancel");
    let (stt, engine) = stt_with(dir.path(), ScriptedEngine::new(Availability::Ready, &[]), |_| {});
    stt.handle(SttRequest::Start { vocabulary: Vec::new() });
    wait_until(&stt, "audio reçu", |_| engine.pushed.load(Ordering::Relaxed) > 0);
    stt.handle(SttRequest::Cancel);
    wait_until(&stt, "flux annulé", |_| engine.cancelled.load(Ordering::Relaxed));
}

#[test]
fn apple_stop_without_done_times_out_keeping_final_text() {
    let dir = fresh_dir("stream-hang");
    let mut engine = ScriptedEngine::new(Availability::Ready, &["Début."]);
    engine.hang = true;
    let (stt, _) = stt_with(dir.path(), engine, |s| s.apple_finish_timeout_secs = 0.3);
    stt.handle(SttRequest::Start { vocabulary: Vec::new() });
    wait_until(&stt, "aperçu", |s| s.partial == "bonjour");
    stt.handle(SttRequest::Stop);
    let done = wait_until(&stt, "délai dépassé", |s| s.state == State::Idle);
    assert_that!((done.text.as_deref(), done.error.as_deref()))
        .is_equal_to((Some("Début."), Some("La transcription a échoué")));
}

#[test]
fn apple_engine_failure_reports_error_without_text() {
    let dir = fresh_dir("stream-fail");
    let mut engine = ScriptedEngine::new(Availability::Ready, &[]);
    engine.fail = true;
    let (stt, _) = stt_with(dir.path(), engine, |_| {});
    stt.handle(SttRequest::Start { vocabulary: Vec::new() });
    let done = wait_until(&stt, "erreur", |s| s.state == State::Idle && s.error.is_some());
    assert_that!((done.error.as_deref(), done.seq)).is_equal_to((Some("La transcription a échoué"), 0));
}

#[test]
fn apple_nothing_heard_auto_stop_finishes_the_stream() {
    let dir = fresh_dir("stream-auto");
    let (stt, _) = stt_with(dir.path(), ScriptedEngine::new(Availability::Ready, &["Fin."]), |s| {
        s.nothing_heard_secs = 0.3;
    });
    stt.handle(SttRequest::Start { vocabulary: Vec::new() });
    let done = wait_until(&stt, "arrêt automatique", |s| s.seq == 1);
    assert_that!(done.text.as_deref()).is_equal_to(Some("Fin."));
}

#[test]
fn apple_transcribe_samples_returns_timed_finals() {
    let dir = fresh_dir("stream-samples");
    let (stt, _) = stt_with(dir.path(), ScriptedEngine::new(Availability::Ready, &["Un.", "Deux."]), |_| {});
    let (text, phrases) = stt.transcribe_samples(&[0.0; 16_000]).unwrap();
    assert_that!(text).is_equal_to("Un. Deux.".to_owned());
    assert_that!(phrases[1].clone()).is_equal_to((1.0, 2.0, "Deux.".to_owned()));
}
```

`lib.rs` : `mod dictation_stream;`. Run: `cargo nextest run -p oku-slides-stt apple_` → FAIL (`stream_loop`, etc. absents ; avec Apple, `start` ne fait encore rien).

- [ ] **Step 3: Implémenter `dictation_stream.rs`**

```rust
//! Dictée par un moteur en continu (Apple) : ses résultats, sa langue, un audio complet.
//!
//! `listen_loop` (lib.rs) pousse l'audio dans le [`Stream`](crate::stream::Stream) ;
//! [`Stt::stream_loop`] applique ses événements à l'état, jusqu'à `Done`.

use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

use oku_slides_i18n::Localize;

use crate::error::{AnalyzerSnafu, AssetInstallSnafu, CancelledSnafu, FinishTimeoutSnafu, LocaleUnsupportedSnafu};
use crate::stream::{Availability, Event};
use crate::{Download, Inner, Model, SAMPLE_RATE, State, Stt, SttError, TimedPhrase, clean, join, results};

impl Stt {
    /// Résultats d'un moteur en continu, jusqu'à la fin de la dictée.
    pub(crate) fn stream_loop(&self, session: u64, events: mpsc::Receiver<Event>) {
        let tick = Duration::from_millis(self.settings.tick_ms);
        let timeout = Duration::from_secs_f32(self.settings.apple_finish_timeout_secs);
        let mut finishing: Option<Instant> = None;
        loop {
            let event = events.recv_timeout(tick);
            let mut g = self.state();
            if g.session != session {
                return; // dictée abandonnée
            }
            match event {
                Ok(Event::Done) => {
                    g.complete();
                    return;
                }
                Ok(Event::Failed(e)) => {
                    self.fail(&mut g, &e);
                    return;
                }
                Ok(event) => {
                    let Inner { phrases, partial, .. } = &mut *g;
                    results::apply(phrases, partial, &event);
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    self.fail(&mut g, &AnalyzerSnafu { reason: "bridge closed" }.build().into());
                    return;
                }
            }
            // Arrêt demandé : le texte final doit arriver à temps.
            if g.state == State::Transcribing {
                let since = *finishing.get_or_insert_with(Instant::now);
                if since.elapsed() >= timeout {
                    let e: SttError = FinishTimeoutSnafu.build().into();
                    tracing::warn!(error = %e, "dictée en continu");
                    g.error = Some(e.localize(&self.localizer));
                    g.complete(); // le texte déjà définitif est gardé
                    return;
                }
            }
        }
    }

    /// Le moteur a échoué : message, puis retour au repos sans texte.
    fn fail(&self, g: &mut Inner, e: &SttError) {
        tracing::warn!(error = %oku_slides_platform::errors::one_line(e), "dictée en continu");
        g.error = Some(e.localize(&self.localizer));
        g.reset();
    }

    /// Télécharge la langue du moteur d'Apple (par le système), puis reprend la dictée.
    pub(crate) fn download_language(&self, then_start: bool) {
        let tag = {
            let mut g = self.state();
            let Some((tag, _)) = g.apple.clone() else { return };
            if g.state != State::Idle {
                return;
            }
            g.state = State::Downloading;
            g.download = Some(Download { model: Model::Apple, received: 0, total: 0 });
            g.start_after_download = then_start;
            tag
        };
        self.cancel_download.store(false, Ordering::Relaxed);
        let me = self.clone();
        std::thread::spawn(move || {
            let result = me.install(&tag);
            me.refresh_apple();
            if me.finish_download(Model::Apple, result) {
                me.begin_recording();
            }
        });
    }

    /// Attend l'installation d'une langue ; la progression va dans l'état.
    fn install(&self, tag: &str) -> Result<(), SttError> {
        let events = self.streaming.install(tag);
        loop {
            match events.recv_timeout(Duration::from_millis(self.settings.tick_ms)) {
                Ok(Event::Progress { received, total }) => {
                    self.state().download = Some(Download { model: Model::Apple, received, total });
                }
                Ok(Event::Done) => return Ok(()),
                Ok(Event::Failed(e)) => return Err(e),
                Ok(_) => {}
                Err(RecvTimeoutError::Timeout) => {
                    if self.cancel_download.load(Ordering::Relaxed) {
                        return CancelledSnafu.fail()?;
                    }
                }
                Err(RecvTimeoutError::Disconnected) => return AssetInstallSnafu { reason: "bridge closed" }.fail()?,
            }
        }
    }

    /// Transcrit un audio complet (mono 16 kHz) par le moteur en continu.
    pub(crate) fn transcribe_stream(&self, audio: &[f32]) -> Result<(String, Vec<TimedPhrase>), SttError> {
        let (apple, vocabulary) = {
            let g = self.state();
            (g.apple.clone(), g.vocabulary.clone())
        };
        let Some((tag, availability)) = apple else {
            return LocaleUnsupportedSnafu { locale: self.wanted_language() }.fail()?;
        };
        if availability == Availability::Missing {
            let installed = self.install(&tag);
            self.state().download = None;
            installed?;
            self.refresh_apple();
        }
        let (mut stream, events) = self.streaming.open(&tag, &vocabulary);
        for chunk in audio.chunks(SAMPLE_RATE as usize / 10) {
            stream.push(chunk);
        }
        stream.finish();
        let timeout = Duration::from_secs_f32(self.settings.apple_finish_timeout_secs);
        let mut phrases: Vec<TimedPhrase> = Vec::new();
        loop {
            match events.recv_timeout(timeout) {
                Ok(Event::Final { text, start, end }) => {
                    let text = clean(&text);
                    if !text.is_empty() {
                        phrases.push((start, end, text));
                    }
                }
                Ok(Event::Done) => break,
                Ok(Event::Failed(e)) => return Err(e),
                Ok(_) => {}
                Err(RecvTimeoutError::Timeout) => return FinishTimeoutSnafu.fail()?,
                Err(RecvTimeoutError::Disconnected) => return AnalyzerSnafu { reason: "bridge closed" }.fail()?,
            }
        }
        let text = join(phrases.iter().map(|(_, _, t)| t));
        Ok((text, phrases))
    }
}

#[cfg(test)]
#[path = "dictation_stream_tests.rs"]
mod dictation_stream_tests;
```

- [ ] **Step 4: Brancher le moteur dans lib.rs**

`start()` : la branche Apple devient

```rust
        if model == Model::Apple {
            match apple {
                Some((_, Availability::Ready)) => self.begin_recording(),
                Some(_) => self.download(Model::Apple, true),
                None => self.report(LocaleUnsupportedSnafu { locale: self.wanted_language() }.build().into()),
            }
            return;
        }
```

`download()` : en tête, `if model == Model::Apple { self.download_language(then_start); return; }`. La fin du fil de téléchargement Whisper (bloc `let start = { … }`) devient `let start = me.finish_download(model, result.map(|_| ()));`, avec :

```rust
    /// Fin d'un téléchargement : état, message, et reprise de la dictée demandée.
    fn finish_download(&self, model: Model, result: Result<(), SttError>) -> bool {
        let mut g = self.state();
        g.download = None;
        g.state = State::Idle;
        g.installed = Model::ALL.map(|m| m.is_installed(&self.storage, &self.dir));
        match result {
            Ok(()) => {
                tracing::info!(?model, "{}", self.localizer.tr("log-model-downloaded"));
                std::mem::take(&mut g.start_after_download)
            }
            Err(e) if e.kind() == SttErrorKind::Cancelled => false,
            Err(e) => {
                tracing::warn!(?model, error = %snafu::Report::from_error(&e), "téléchargement du modèle");
                g.error = Some(e.localize(&self.localizer));
                false
            }
        }
    }
```

`Sink` et `begin_recording` :

```rust
/// Où part l'audio d'une dictée.
enum Sink {
    /// Whisper : phrases closes et aperçus, en jobs.
    Whisper { jobs: mpsc::Sender<Job>, busy: Arc<AtomicBool> },
    /// Moteur en continu : tout l'audio, au fil de l'eau.
    Stream(Box<dyn Stream>),
}

    fn begin_recording(&self) {
        let recorder = match audio::Recorder::start(self.settings.fake_mic.as_deref(), &self.storage) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(error = %snafu::Report::from_error(&e), "micro indisponible");
                self.state().error = Some(e.localize(&self.localizer));
                return;
            }
        };
        let (session, model, apple, vocabulary) = {
            let mut g = self.state();
            g.session += 1;
            g.state = State::Recording;
            g.phrases.clear();
            g.partial.clear();
            g.detected = None;
            if g.config.model == Model::Apple {
                g.detected = g.apple.as_ref().map(|(tag, _)| locale::language(tag));
            }
            g.stop_in = None;
            g.started = Some(Instant::now());
            (g.session, g.config.model, g.apple.clone(), g.vocabulary.clone())
        };
        let sink = match (model, apple) {
            (Model::Apple, Some((tag, _))) => {
                let (stream, events) = self.streaming.open(&tag, &vocabulary);
                let worker = self.clone();
                std::thread::spawn(move || worker.stream_loop(session, events));
                Sink::Stream(stream)
            }
            (Model::Apple, None) => {
                // `start` l'empêche ; par prudence, pas d'écoute sans langue.
                self.state().reset();
                self.report(LocaleUnsupportedSnafu { locale: self.wanted_language() }.build().into());
                return;
            }
            _ => {
                let (jobs, rx) = mpsc::channel();
                let busy = Arc::new(AtomicBool::new(false));
                let worker = self.clone();
                let worker_busy = busy.clone();
                std::thread::spawn(move || worker.transcribe_loop(session, rx, worker_busy));
                Sink::Whisper { jobs, busy }
            }
        };
        let listener = self.clone();
        std::thread::spawn(move || listener.listen_loop(session, recorder, sink));
    }
```

`listen_loop` et `pull` (remplacent l'ancien `listen_loop` et sa fermeture `pull`) :

```rust
    /// Écoute : micro → 16 kHz → détection de la voix → moteur.
    fn listen_loop(&self, session: u64, mut recorder: audio::Recorder, mut sink: Sink) {
        let s = &self.settings;
        let mut seg = Segmenter::new(s.vad);
        let mut resampler: Option<audio::Resampler> = None;
        let mut fresh = Vec::new();
        let mut previewed = 0usize;
        loop {
            std::thread::sleep(Duration::from_millis(s.tick_ms));
            let (state, auto_stop, silence_limit) = {
                let g = self.state();
                if g.session != session {
                    // Dictée abandonnée : le moteur en continu s'arrête aussi.
                    if let Sink::Stream(stream) = &mut sink {
                        stream.cancel();
                    }
                    return;
                }
                (g.state, g.config.auto_stop, g.config.silence)
            };
            self.pull(&recorder, &mut resampler, &mut fresh, &mut seg, &mut sink);

            if state != State::Recording {
                // Arrêt demandé : derniers échantillons, dernière phrase.
                recorder.finish();
                self.pull(&recorder, &mut resampler, &mut fresh, &mut seg, &mut sink);
                match &mut sink {
                    Sink::Whisper { jobs, .. } => {
                        if let Some(phrase) = seg.close() {
                            self.send_final(&seg, phrase, jobs);
                        }
                        jobs.send(Job::Done).ok();
                    }
                    Sink::Stream(stream) => stream.finish(),
                }
                return;
            }

            // Aperçu de la phrase en cours (Whisper), quand le fil de transcription est libre.
            let current = seg.current();
            if let Sink::Whisper { jobs, busy } = &sink
                && let Some(p) = current
                && !busy.load(Ordering::Relaxed)
                && p.end.saturating_sub(previewed.max(p.start)) as f32 >= s.preview_step_secs * SAMPLE_RATE as f32
            {
                previewed = p.end;
                busy.store(true, Ordering::Relaxed);
                jobs.send(Job::Preview { audio: seg.slice(p).to_vec() }).ok();
            }

            // Arrêt automatique après un silence (ou si rien n'est dit).
            let silence = seg.silence();
            let limit = if seg.heard { silence_limit } else { s.nothing_heard_secs };
            let stop_in = (auto_stop && current.is_none() && seg.heard && silence > s.countdown_after_secs)
                .then(|| (limit - silence).max(0.0));
            let stop = auto_stop && silence >= limit;
            {
                let mut g = self.state();
                g.level = recorder.level();
                g.speaking = seg.speaking;
                g.stop_in = stop_in;
                // L'aperçu de Whisper suit la phrase en cours ; celui du moteur en continu, ses résultats.
                if current.is_none() && matches!(sink, Sink::Whisper { .. }) {
                    g.partial.clear();
                }
                if stop && g.state == State::Recording {
                    g.state = State::Transcribing;
                    g.started = None;
                }
            }
        }
    }

    /// Échantillons arrivés : 16 kHz, détection de la voix, puis le moteur.
    fn pull(
        &self,
        recorder: &audio::Recorder,
        resampler: &mut Option<audio::Resampler>,
        fresh: &mut Vec<f32>,
        seg: &mut Segmenter,
        sink: &mut Sink,
    ) {
        let (samples, rate) = recorder.take();
        if samples.is_empty() {
            return;
        }
        fresh.clear();
        resampler.get_or_insert_with(|| audio::Resampler::new(rate)).push(&samples, fresh);
        let phrases = seg.push(fresh);
        match sink {
            Sink::Whisper { jobs, .. } => {
                for phrase in phrases {
                    self.send_final(seg, phrase, jobs);
                }
            }
            Sink::Stream(stream) => stream.push(fresh),
        }
    }
```

`transcribe_samples` : en tête, `let model = self.state().config.model; if model == Model::Apple { return self.transcribe_stream(audio); }`.

`stream.rs` et `results.rs` : retirer le `#![cfg_attr(not(test), expect(dead_code, …))]` de la Task 3.

Run: `cargo nextest run -p oku-slides-stt -p oku-slides-platform && cargo clippy -p oku-slides-stt --all-targets -- -D warnings && wc -l crates/oku-slides-stt/src/lib.rs`
Expected: PASS ; `lib.rs` sous 800 lignes.

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-stt/src crates/oku-slides-platform/src
```
```bash
git commit -m "feat(stt) : dictée par un moteur en continu, langue téléchargée à la demande (t-393)"
```

---

### Task 6: Pont Swift vers SpeechAnalyzer

Le vrai moteur d'Apple, compilé et lié sur macOS ; un test d'intégration réel (ignoré par défaut) ; l'exemple `transcribe` accepte `apple`.

**Files:**
- Create: `crates/oku-slides-stt/swift/Package.swift`
- Create: `crates/oku-slides-stt/swift/Sources/OkuSpeech/OkuSpeech.swift`
- Create: `crates/oku-slides-stt/build.rs`
- Create: `crates/oku-slides-stt/src/apple.rs`
- Create: `crates/oku-slides-stt/tests/apple.rs`
- Create: `fixtures/stt/dictee-fr.wav`
- Create: `crates/oku-slides-stt/ARCHITECTURE.md`
- Modify: `Cargo.toml` (workspace : `swift-rs = "1.0.8"`), `crates/oku-slides-stt/Cargo.toml` (`[build-dependencies]`, description)
- Modify: `crates/oku-slides-stt/src/stream.rs` (`engine`), `src/lib.rs` (`#[cfg(target_os = "macos")] mod apple;`, doc du module)
- Modify: `crates/oku-slides-stt/examples/transcribe.rs`, `justfile` (`test-apple`)
- Modify: `.gitignore` si `swift/.build` n'y est pas

**Interfaces:**
- Consumes: Task 3 (`Event::from_raw`, `raw`, `Stream`, `StreamEngine`, `Availability`), Task 5 (`transcribe_samples` → `transcribe_stream`).
- Produces: `apple::AppleEngine` (`StreamEngine`), ABI C `oku_speech_supported`, `oku_speech_availability`, `oku_speech_install`, `oku_speech_start`, `oku_speech_push`, `oku_speech_finish`, `oku_speech_cancel`, `oku_speech_release`.

- [ ] **Step 1: Fixture et test d'intégration réel (en échec)**

```bash
say -v Thomas "Pour la prochaine diapositive, ajoute un titre : les résultats du troisième trimestre." -o fixtures/stt/dictee-fr.wav --file-format=WAVE --data-format=LEI16@16000
```

`tests/apple.rs` :

```rust
//! Le moteur d'Apple pour de vrai (macOS 26, français installé) : `just test-apple`.
#![cfg(target_os = "macos")]

use std::time::{Duration, Instant};

use oku_slides_platform::Platform;
use oku_slides_platform::config::Settings;
use oku_slides_stt::{Config, Model, Stt, SttRequest};
use speculoos::prelude::*;

mod common;
use common::workdir;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/stt/dictee-fr.wav");

#[test]
#[ignore = "macOS 26 et langue française installée : just test-apple"]
fn apple_transcribes_french_fixture_quickly() {
    let dir = workdir("apple");
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.data = Some(dir.join("data"));
    settings.paths.models = Some(dir.join("models"));
    settings.i18n.locale = Some("fr".into());
    let stt = Stt::new(&Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme : {e}")));
    stt.handle(SttRequest::Configure(Config { model: Model::Apple, language: Some("fr".into()), ..stt.status().config }));
    let mut reader = hound::WavReader::open(FIXTURE).unwrap_or_else(|e| panic!("fixture : {e}"));
    let audio: Vec<f32> = reader.samples::<i16>().map(|s| f32::from(s.unwrap_or(0)) / 32768.0).collect();

    let started = Instant::now();
    let (text, phrases) = stt.transcribe_samples(&audio).unwrap_or_else(|e| panic!("transcription : {e}"));

    assert_that!(started.elapsed()).is_less_than(Duration::from_secs(2));
    assert_that!(phrases).is_not_empty();
    let lower = text.to_lowercase();
    assert_that!(lower.contains("diapositive") && lower.contains("trimestre")).is_true();
}
```

`justfile`, près de `transcribe` :

```make
# Dictée par le moteur d'Apple, pour de vrai (macOS 26, français installé).
test-apple:
    cargo nextest run -p oku-slides-stt --test apple --run-ignored ignored-only
```

Run: `just test-apple`
Expected: FAIL (`Stt::new` rend `NoStreamEngine` : erreur `LocaleUnsupported`).

- [ ] **Step 2: Paquet Swift**

`swift/Package.swift` :

```swift
// swift-tools-version:5.9
// Pont Swift du moteur de dictée d'Apple, lié en bibliothèque statique (build.rs).
import PackageDescription

let package = Package(
    name: "OkuSpeech",
    platforms: [.macOS(.v12)],
    products: [.library(name: "OkuSpeech", type: .static, targets: ["OkuSpeech"])],
    targets: [.target(name: "OkuSpeech", path: "Sources/OkuSpeech")]
)
```

`swift/Sources/OkuSpeech/OkuSpeech.swift` :

```swift
// OkuSpeech : le moteur de dictée d'Apple (SpeechAnalyzer, macOS 26+) pour Rust.
//
// ABI C (@_cdecl), lue par crates/oku-slides-stt/src/apple.rs ; codes partagés
// avec stream.rs (`raw`). Un appel asynchrone rend ses événements par
// `cb(ctx, …)`, puis appelle `free(ctx)` une seule fois. Ni `try!` ni `!` :
// un plantage ici ferait tomber l'app.

import AVFoundation
import CoreMedia
import Foundation
import Speech

public typealias OkuCallback = @convention(c) (UnsafeMutableRawPointer?, Int32, UnsafePointer<CChar>?, Double, Double) -> Void
public typealias OkuFree = @convention(c) (UnsafeMutableRawPointer?) -> Void

private let kindPartial: Int32 = 0
private let kindFinal: Int32 = 1
private let kindProgress: Int32 = 2
private let kindDone: Int32 = 3
private let kindError: Int32 = 4
private let errAnalyzer = 1.0
private let errLocale = 2.0
private let errAsset = 3.0

/// Retour vers Rust : des événements, puis `free` une seule fois.
final class Sink: @unchecked Sendable {
    private let ctx: UnsafeMutableRawPointer?
    private let cb: OkuCallback
    private let free: OkuFree
    private let lock = NSLock()
    private var closed = false

    init(_ ctx: UnsafeMutableRawPointer?, _ cb: OkuCallback, _ free: OkuFree) {
        self.ctx = ctx
        self.cb = cb
        self.free = free
    }

    func send(_ kind: Int32, _ text: String = "", _ a: Double = 0, _ b: Double = 0) {
        lock.lock()
        defer { lock.unlock() }
        if closed { return }
        text.withCString { cb(ctx, kind, $0, a, b) }
    }

    /// Dernier événement (`Done` ou erreur), puis `free`.
    func close(_ kind: Int32, _ text: String = "", _ code: Double = 0) {
        lock.lock()
        defer { lock.unlock() }
        if closed { return }
        closed = true
        text.withCString { cb(ctx, kind, $0, code, 0) }
        free(ctx)
    }
}

private final class ResultBox<T>: @unchecked Sendable {
    var value: T?
}

/// Attend une tâche asynchrone depuis un fil Rust (jamais depuis le pool de Swift).
private func blocking<T>(_ fallback: T, _ body: @escaping @Sendable () async -> T) -> T {
    let box = ResultBox<T>()
    let done = DispatchSemaphore(value: 0)
    Task {
        box.value = await body()
        done.signal()
    }
    done.wait()
    return box.value ?? fallback
}

@available(macOS 26.0, *)
private func transcriber(_ locale: Locale) -> SpeechTranscriber {
    SpeechTranscriber(
        locale: locale,
        transcriptionOptions: [],
        // `.fastResults` : sans lui, le premier aperçu arrive au bout de 12 s.
        reportingOptions: [.volatileResults, .fastResults],
        attributeOptions: [.audioTimeRange]
    )
}

@_cdecl("oku_speech_supported")
public func okuSpeechSupported() -> Bool {
    if #available(macOS 26.0, *) {
        return SpeechTranscriber.isAvailable
    }
    return false
}

/// 0 : inconnue, 1 : à télécharger, 2 : prête.
@_cdecl("oku_speech_availability")
public func okuSpeechAvailability(_ locale: UnsafePointer<CChar>) -> Int32 {
    guard #available(macOS 26.0, *) else { return 0 }
    let id = String(cString: locale)
    return blocking(Int32(0)) {
        guard let found = await SpeechTranscriber.supportedLocale(equivalentTo: Locale(identifier: id)) else { return 0 }
        switch await AssetInventory.status(forModules: [transcriber(found)]) {
        case .installed: return 2
        case .unsupported: return 0
        default: return 1
        }
    }
}

@_cdecl("oku_speech_install")
public func okuSpeechInstall(
    _ locale: UnsafePointer<CChar>, _ ctx: UnsafeMutableRawPointer?, _ cb: OkuCallback, _ free: OkuFree
) {
    let sink = Sink(ctx, cb, free)
    let id = String(cString: locale)
    guard #available(macOS 26.0, *) else {
        sink.close(kindError, id, errLocale)
        return
    }
    Task {
        guard let found = await SpeechTranscriber.supportedLocale(equivalentTo: Locale(identifier: id)) else {
            sink.close(kindError, id, errLocale)
            return
        }
        do {
            if let request = try await AssetInventory.assetInstallationRequest(supporting: [transcriber(found)]) {
                let watch = request.progress.observe(\.fractionCompleted) { progress, _ in
                    sink.send(kindProgress, "", Double(progress.completedUnitCount), Double(progress.totalUnitCount))
                }
                try await request.downloadAndInstall()
                watch.invalidate()
            }
            sink.close(kindDone)
        } catch {
            sink.close(kindError, String(describing: error), errAsset)
        }
    }
}

/// Une dictée : l'audio arrive avant que l'analyseur soit prêt, il attend dans `pending`.
@available(macOS 26.0, *)
final class Session: @unchecked Sendable {
    private let sink: Sink
    private let module: SpeechTranscriber
    private let analyzer: SpeechAnalyzer
    private let input: AsyncStream<AnalyzerInput>.Continuation
    private let source: AVAudioFormat?
    private let lock = NSLock()
    private var target: AVAudioFormat?
    private var converter: AVAudioConverter?
    private var pending: [Float] = []
    private var ended = false
    private var setup: Task<Bool, Never>?
    private var results: Task<Void, Never>?

    init(locale: Locale, vocabulary: [String], sink: Sink) {
        self.sink = sink
        module = transcriber(locale)
        analyzer = SpeechAnalyzer(modules: [module])
        let (stream, input) = AsyncStream<AnalyzerInput>.makeStream()
        self.input = input
        source = AVAudioFormat(commonFormat: .pcmFormatFloat32, sampleRate: 16_000, channels: 1, interleaved: false)
        let (module, analyzer) = (self.module, self.analyzer)
        results = Task {
            do {
                for try await result in module.results {
                    let text = String(result.text.characters)
                    if result.isFinal {
                        sink.send(kindFinal, text, result.range.start.seconds, result.range.end.seconds)
                    } else {
                        sink.send(kindPartial, text)
                    }
                }
            } catch {
                sink.close(kindError, String(describing: error), errAnalyzer)
            }
        }
        setup = Task { [self] in
            do {
                guard let format = await SpeechAnalyzer.bestAvailableAudioFormat(compatibleWith: [module]) else {
                    sink.close(kindError, "no audio format", errAnalyzer)
                    return false
                }
                if !vocabulary.isEmpty {
                    let context = AnalysisContext()
                    context.contextualStrings[.general] = vocabulary
                    try await analyzer.setContext(context)
                }
                try await analyzer.start(inputSequence: stream)
                ready(format)
                return true
            } catch {
                sink.close(kindError, String(describing: error), errAnalyzer)
                return false
            }
        }
    }

    /// Format de l'analyseur connu : on convertit et on transmet ce qui attendait.
    private func ready(_ format: AVAudioFormat) {
        lock.lock()
        defer { lock.unlock() }
        target = format
        if let source, source != format {
            converter = AVAudioConverter(from: source, to: format)
        }
        let waiting = pending
        pending = []
        waiting.withUnsafeBufferPointer { yield($0) }
    }

    func push(_ samples: UnsafeBufferPointer<Float>) {
        lock.lock()
        defer { lock.unlock() }
        if ended { return }
        if target == nil {
            pending.append(contentsOf: samples)
            return
        }
        yield(samples)
    }

    /// Convertit au format de l'analyseur et transmet (verrou tenu).
    private func yield(_ samples: UnsafeBufferPointer<Float>) {
        guard !samples.isEmpty, let source, let target, let base = samples.baseAddress,
              let buffer = AVAudioPCMBuffer(pcmFormat: source, frameCapacity: AVAudioFrameCount(samples.count)),
              let channel = buffer.floatChannelData
        else { return }
        buffer.frameLength = AVAudioFrameCount(samples.count)
        channel[0].update(from: base, count: samples.count)
        guard let converter else {
            input.yield(AnalyzerInput(buffer: buffer))
            return
        }
        let capacity = AVAudioFrameCount(Double(samples.count) * target.sampleRate / source.sampleRate) + 32
        guard let out = AVAudioPCMBuffer(pcmFormat: target, frameCapacity: capacity) else { return }
        var fed = false
        var error: NSError?
        _ = converter.convert(to: out, error: &error) { _, status in
            if fed {
                status.pointee = .noDataNow
                return nil
            }
            fed = true
            status.pointee = .haveData
            return buffer
        }
        if error == nil, out.frameLength > 0 {
            input.yield(AnalyzerInput(buffer: out))
        }
    }

    /// Fin de l'audio : derniers textes définitifs, puis `Done`.
    func finish() {
        lock.lock()
        ended = true
        lock.unlock()
        let (setup, results, input, analyzer, sink) = (setup, results, input, analyzer, sink)
        Task {
            // Analyseur jamais démarré : l'erreur est déjà partie.
            guard await setup?.value == true else { return }
            input.finish()
            do {
                try await analyzer.finalizeAndFinishThroughEndOfInput()
            } catch {
                sink.close(kindError, String(describing: error), errAnalyzer)
                return
            }
            await results?.value
            sink.close(kindDone)
        }
    }

    /// Abandon : on arrête tout, sans attendre de texte.
    func cancel() {
        lock.lock()
        ended = true
        lock.unlock()
        input.finish()
        let (analyzer, results, sink) = (analyzer, results, sink)
        Task {
            await analyzer.cancelAndFinishNow()
            results?.cancel()
            sink.close(kindDone)
        }
    }
}

@available(macOS 26.0, *)
private func session(_ handle: UnsafeMutableRawPointer) -> Session {
    Unmanaged<Session>.fromOpaque(handle).takeUnretainedValue()
}

@_cdecl("oku_speech_start")
public func okuSpeechStart(
    _ locale: UnsafePointer<CChar>, _ words: UnsafePointer<UnsafePointer<CChar>?>?, _ count: Int32,
    _ ctx: UnsafeMutableRawPointer?, _ cb: OkuCallback, _ free: OkuFree
) -> UnsafeMutableRawPointer? {
    let sink = Sink(ctx, cb, free)
    let id = String(cString: locale)
    guard #available(macOS 26.0, *) else {
        sink.close(kindError, id, errLocale)
        return nil
    }
    var vocabulary: [String] = []
    if let words {
        for i in 0..<Int(max(count, 0)) {
            if let word = words[i] { vocabulary.append(String(cString: word)) }
        }
    }
    let found = blocking(Locale?.none) { await SpeechTranscriber.supportedLocale(equivalentTo: Locale(identifier: id)) }
    guard let found else {
        sink.close(kindError, id, errLocale)
        return nil
    }
    return Unmanaged.passRetained(Session(locale: found, vocabulary: vocabulary, sink: sink)).toOpaque()
}

@_cdecl("oku_speech_push")
public func okuSpeechPush(_ handle: UnsafeMutableRawPointer?, _ samples: UnsafePointer<Float>?, _ count: Int32) {
    guard #available(macOS 26.0, *), let handle, let samples, count > 0 else { return }
    session(handle).push(UnsafeBufferPointer(start: samples, count: Int(count)))
}

@_cdecl("oku_speech_finish")
public func okuSpeechFinish(_ handle: UnsafeMutableRawPointer?) {
    guard #available(macOS 26.0, *), let handle else { return }
    session(handle).finish()
}

@_cdecl("oku_speech_cancel")
public func okuSpeechCancel(_ handle: UnsafeMutableRawPointer?) {
    guard #available(macOS 26.0, *), let handle else { return }
    session(handle).cancel()
}

@_cdecl("oku_speech_release")
public func okuSpeechRelease(_ handle: UnsafeMutableRawPointer?) {
    guard #available(macOS 26.0, *), let handle else { return }
    Unmanaged<Session>.fromOpaque(handle).release()
}
```

Ce code suit les API de macOS 26 telles que le spike les a utilisées (`SpeechTranscriber`, `SpeechAnalyzer`, `AssetInventory`, `AnalyzerInput`) ; celles que le spike n'a pas exercées (`SpeechTranscriber.isAvailable`, `AssetInventory.status(forModules:)`, `AnalysisContext.contextualStrings[.general]`, `result.range`) sont à confirmer à la compilation. Si l'une diffère du SDK, prendre l'équivalent du SDK sans changer l'ABI C ni les codes, et le signaler dans le rapport de la tâche. Sans `isAvailable`, `okuSpeechSupported` rend `true` sous `#available(macOS 26.0, *)` : la disponibilité par langue reste la vraie garde.

Run: `cd crates/oku-slides-stt/swift && swift build -c release 2>&1 | tail -20`
Expected: `Build complete!`, sans erreur (avertissements de concurrence acceptés en mode Swift 5).

- [ ] **Step 3: Compilation et liaison (build.rs, Cargo)**

`Cargo.toml` (workspace, `[workspace.dependencies]`) : `swift-rs = "1.0.8"`. `crates/oku-slides-stt/Cargo.toml` :

```toml
description = "Dictée d'Oku Slides : micro, détection de la voix, Whisper (CTranslate2) ou moteur d'Apple (macOS 26+)."
# …
[build-dependencies]
swift-rs = { workspace = true, features = ["build"] }
```

`build.rs` :

```rust
//! Compile le pont Swift du moteur d'Apple (macOS seulement).

fn main() {
    println!("cargo:rerun-if-changed=swift/Package.swift");
    println!("cargo:rerun-if-changed=swift/Sources");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    swift_rs::SwiftLinker::new("12.0").with_package("OkuSpeech", "swift/").link();
    for framework in ["Speech", "AVFoundation", "CoreMedia"] {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
}
```

`.gitignore` : ajouter `crates/oku-slides-stt/swift/.build/` s'il n'y est pas.

- [ ] **Step 4: `apple.rs` et branchement**

```rust
//! Moteur de dictée d'Apple (SpeechAnalyzer, macOS 26+), par le pont Swift `swift/`.
//!
//! Seul fichier `unsafe` du crate. Le pont appelle [`on_event`] depuis ses
//! propres fils, puis [`free_ctx`] une seule fois : le contexte (un
//! `Sender<Event>` en boîte) vit jusque-là.
#![expect(unsafe_code, reason = "pont FFI vers Swift : seul fichier unsafe du crate")]

use std::ffi::{CStr, CString, c_char, c_void};
use std::sync::mpsc;

use crate::error::LocaleUnsupportedSnafu;
use crate::stream::{Availability, Event, Stream, StreamEngine};

type Callback = extern "C" fn(*mut c_void, i32, *const c_char, f64, f64);
type Free = extern "C" fn(*mut c_void);

unsafe extern "C" {
    fn oku_speech_supported() -> bool;
    fn oku_speech_availability(locale: *const c_char) -> i32;
    fn oku_speech_install(locale: *const c_char, ctx: *mut c_void, cb: Callback, free: Free);
    fn oku_speech_start(
        locale: *const c_char,
        words: *const *const c_char,
        count: i32,
        ctx: *mut c_void,
        cb: Callback,
        free: Free,
    ) -> *mut c_void;
    fn oku_speech_push(handle: *mut c_void, samples: *const f32, count: i32);
    fn oku_speech_finish(handle: *mut c_void);
    fn oku_speech_cancel(handle: *mut c_void);
    fn oku_speech_release(handle: *mut c_void);
}

/// Un événement du pont, transmis au canal de la dictée.
extern "C" fn on_event(ctx: *mut c_void, kind: i32, text: *const c_char, a: f64, b: f64) {
    if ctx.is_null() {
        return;
    }
    // SAFETY: `ctx` vient de `Box::into_raw` d'un `Sender<Event>` (`context`) ;
    // le pont ne l'utilise plus après `free_ctx`, et sérialise ses appels.
    let tx = unsafe { &*ctx.cast::<mpsc::Sender<Event>>() };
    let text = if text.is_null() {
        String::new()
    } else {
        // SAFETY: chaîne C terminée par zéro, valide pendant l'appel (`withCString`).
        unsafe { CStr::from_ptr(text) }.to_string_lossy().into_owned()
    };
    tx.send(Event::from_raw(kind, text, a, b)).ok();
}

/// Le pont a fini avec ce contexte.
extern "C" fn free_ctx(ctx: *mut c_void) {
    if ctx.is_null() {
        return;
    }
    // SAFETY: appelé une seule fois par le pont, après son dernier `on_event`.
    drop(unsafe { Box::from_raw(ctx.cast::<mpsc::Sender<Event>>()) });
}

/// Un canal d'événements, et son contexte pour le pont.
fn context() -> (*mut c_void, mpsc::Receiver<Event>) {
    let (tx, rx) = mpsc::channel();
    (Box::into_raw(Box::new(tx)).cast::<c_void>(), rx)
}

/// Le moteur de dictée d'Apple.
#[derive(Debug)]
pub(crate) struct AppleEngine;

impl StreamEngine for AppleEngine {
    fn supported(&self) -> bool {
        // SAFETY: fonction sans argument ni état partagé.
        unsafe { oku_speech_supported() }
    }

    fn availability(&self, locale: &str) -> Availability {
        let Ok(tag) = CString::new(locale) else { return Availability::Unsupported };
        // SAFETY: `tag` vit pendant l'appel, qui est synchrone.
        match unsafe { oku_speech_availability(tag.as_ptr()) } {
            2 => Availability::Ready,
            1 => Availability::Missing,
            _ => Availability::Unsupported,
        }
    }

    fn install(&self, locale: &str) -> mpsc::Receiver<Event> {
        let Ok(tag) = CString::new(locale) else {
            let (tx, rx) = mpsc::channel();
            tx.send(Event::Failed(LocaleUnsupportedSnafu { locale }.build().into())).ok();
            return rx;
        };
        let (ctx, rx) = context();
        // SAFETY: `tag` est copiée par le pont pendant l'appel ; il garde `ctx` jusqu'à `free_ctx`.
        unsafe { oku_speech_install(tag.as_ptr(), ctx, on_event, free_ctx) };
        rx
    }

    fn open(&self, locale: &str, vocabulary: &[String]) -> (Box<dyn Stream>, mpsc::Receiver<Event>) {
        let tag = CString::new(locale).unwrap_or_default();
        let words: Vec<CString> = vocabulary.iter().filter_map(|w| CString::new(w.as_str()).ok()).collect();
        let pointers: Vec<*const c_char> = words.iter().map(|w| w.as_ptr()).collect();
        let count = i32::try_from(pointers.len()).unwrap_or(i32::MAX);
        let (ctx, rx) = context();
        // SAFETY: chaînes et tableau vivants pendant l'appel (copiés par le pont) ;
        // `ctx` gardé par le pont jusqu'à `free_ctx`. Handle nul : l'erreur part par `ctx`.
        let handle = unsafe { oku_speech_start(tag.as_ptr(), pointers.as_ptr(), count, ctx, on_event, free_ctx) };
        (Box::new(AppleStream { handle, ended: false }), rx)
    }
}

/// Une dictée ouverte sur le pont ; relâchée au `drop`.
#[derive(Debug)]
struct AppleStream {
    handle: *mut c_void,
    ended: bool,
}

// SAFETY: le handle désigne un objet Swift qui protège son état par un verrou
// et peut être appelé depuis n'importe quel fil.
unsafe impl Send for AppleStream {}

impl Stream for AppleStream {
    fn push(&mut self, samples: &[f32]) {
        if self.handle.is_null() || self.ended {
            return;
        }
        let count = i32::try_from(samples.len()).unwrap_or(i32::MAX);
        // SAFETY: `samples` vit pendant l'appel ; le pont copie l'audio avant de rendre la main.
        unsafe { oku_speech_push(self.handle, samples.as_ptr(), count) };
    }

    fn finish(&mut self) {
        if self.handle.is_null() || self.ended {
            return;
        }
        self.ended = true;
        // SAFETY: handle valide jusqu'à `oku_speech_release` (dans `drop`).
        unsafe { oku_speech_finish(self.handle) };
    }

    fn cancel(&mut self) {
        if self.handle.is_null() || self.ended {
            return;
        }
        self.ended = true;
        // SAFETY: handle valide jusqu'à `oku_speech_release` (dans `drop`).
        unsafe { oku_speech_cancel(self.handle) };
    }
}

impl Drop for AppleStream {
    fn drop(&mut self) {
        // Ni fini ni annulé : on annule, sinon l'analyseur attendrait de l'audio pour toujours.
        self.cancel();
        if !self.handle.is_null() {
            // SAFETY: relâche la référence prise par `oku_speech_start`, une seule fois.
            unsafe { oku_speech_release(self.handle) };
        }
    }
}
```

`stream.rs`, `engine` :

```rust
/// Le moteur en continu de cette machine, selon les réglages.
pub(crate) fn engine(settings: &SttSettings) -> Arc<dyn StreamEngine> {
    #[cfg(target_os = "macos")]
    if settings.apple {
        return Arc::new(crate::apple::AppleEngine);
    }
    #[cfg(not(target_os = "macos"))]
    let _ = settings;
    Arc::new(NoStreamEngine)
}
```

`lib.rs` : `#[cfg(target_os = "macos")] mod apple;` ; doc du module : ajouter une puce « Sur macOS 26+, le moteur d'Apple (SpeechAnalyzer) transcrit en continu ; Whisper reste le moteur ailleurs. ».

Run: `cargo nextest run -p oku-slides-stt && just test-apple && cargo clippy -p oku-slides-stt --all-targets -- -D warnings`
Expected: PASS, y compris `apple_transcribes_french_fixture_quickly`.

- [ ] **Step 5: Exemple `transcribe` avec `apple`**

`examples/transcribe.rs` : doc `[base|turbo|large|apple]` ; `Some("apple") => Model::Apple,` ; le téléchargement Whisper seulement si `model != Model::Apple && !model.is_installed(…)` (la langue d'Apple s'installe dans `transcribe_samples`).

Run: `just transcribe fixtures/stt/dictee-fr.wav apple fr`
Expected: le texte français et une durée de transcription bien sous la seconde.

- [ ] **Step 6: `ARCHITECTURE.md` du crate (moins de 50 lignes)**

```markdown
# oku-slides-stt

Dictée d'Oku Slides : micro → détection de la voix → moteur → texte. Interrogé
par sondage (`Stt::status`), pour l'app Tauri comme pour le serveur de dev.

## Moteurs

- **Apple** (`Model::Apple`, macOS 26+, réglage `stt.apple`) : `SpeechAnalyzer`
  en continu, par un pont Swift statique (`swift/`, `build.rs`, `apple.rs`).
  Langue résolue par `locale.rs`, installée par le système.
- **Whisper** (Turbo, Large, Base) : CTranslate2 (`whisper.rs`), modèles
  téléchargés depuis Hugging Face (`models.rs`), phrase par phrase
  (`dictation_whisper.rs`).

## Flux

`listen_loop` (lib.rs) : micro (`audio.rs`) → 16 kHz → earshot (`vad.rs`,
arrêt automatique) → `Sink` : jobs Whisper, ou `Stream` d'un moteur en continu
dont `dictation_stream.rs` applique les événements (`results.rs`).

## Points d'extension

- `stream::StreamEngine` : brancher un autre moteur en continu.

## Dépendances

`oku-slides-platform` (réglages, stockage), `oku-slides-i18n` (messages).
```

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock .gitignore justfile fixtures/stt crates/oku-slides-stt
```
```bash
git commit -m "feat(stt) : moteur de dictée d'Apple par un pont Swift vers SpeechAnalyzer (t-393)"
```

---

### Task 7: Interface du modèle Apple

**Files:**
- Modify: `ui/src/api.ts` (`SttModel`, `SttConfig.version`, `models[].available`)
- Modify: `ui/src/Dictation.tsx` (`languages`, liste des modèles, `modelNote`, intro, « à télécharger »)
- Modify: `ui/src/Dictation.render.test.tsx`
- Modify: `crates/oku-slides-i18n/src/locales/*/ui.ftl` (20 langues)

**Interfaces:**
- Consumes: `Status.models[].available`, `Status.config.version`, `Status.detected` (Tasks 4–5).
- Produces: aucune pour d'autres tâches.

- [ ] **Step 1: Tests UI en échec**

Dans `fakeStt(heard = "bonjour", change: Partial<SttStatus> = {})` : `current = { …défauts…, ...change }` ; défauts : `config: { model: "turbo", language: null, auto_stop: false, silence: 2, version: 1 }`, `models: [{ id: "turbo", label: "Large v3 Turbo (rapide)", size: 1, installed: true, available: true }]`.

```ts
import { setMessages } from "./i18n";

async function openSettings(host: HTMLElement) {
  await act(async () => (host.querySelector('[aria-label="ui-dictation-settings"]') as HTMLButtonElement).click());
  await settle();
}

const apple = (available: boolean) => ({ id: "apple" as const, label: "Apple (système)", size: 0, installed: available, available });

describe("modèle Apple", () => {
  test("Dictation Apple without the chosen language is disabled and says why", async () => {
    engine.restore();
    engine = fakeStt("bonjour", {
      config: { model: "turbo", language: "xx", auto_stop: false, silence: 2, version: 1 },
      models: [apple(false), { id: "turbo", label: "Large v3 Turbo (rapide)", size: 1, installed: true, available: true }],
    });
    const { host } = await mount();
    await openSettings(host);
    const radio = host.querySelector('input[name="stt-model"]') as HTMLInputElement;
    expect([radio.disabled, host.querySelector(".model .badge")?.textContent]).toEqual([true, "ui-dictation-model-unavailable"]);
  });

  test("Dictation Apple auto language shows the resolved language", async () => {
    setMessages("fr", ["ui-dictation-language-auto-resolved = Automatique ({ $language })"]);
    engine.restore();
    engine = fakeStt("bonjour", {
      config: { model: "apple", language: null, auto_stop: false, silence: 2, version: 1 },
      models: [apple(true)],
      detected: "fr",
    });
    const { host } = await mount();
    await openSettings(host);
    expect(host.querySelector("select option")?.textContent).toBe("Automatique (Français)");
    setMessages("en", []);
  });

  test("Dictation Apple language to download shows no size", async () => {
    engine.restore();
    engine = fakeStt("bonjour", {
      config: { model: "apple", language: null, auto_stop: false, silence: 2, version: 1 },
      models: [{ ...apple(true), installed: false }],
    });
    const { host } = await mount();
    expect(host.querySelector(".dictation-line .caption")?.textContent).toBe("ui-dictation-to-download-language");
  });
});
```

Run: `cd ui && bun test src/Dictation.render.test.tsx` → FAIL.

- [ ] **Step 2: Implémenter (UI)**

`api.ts` :

```ts
export type SttModel = "apple" | "turbo" | "large" | "base";

export interface SttConfig {
  // … champs existants
  /** Version des réglages (tenue par l'hôte). */
  version: number;
}

  /** `available` faux : le modèle ne gère pas la langue choisie. */
  models: { id: SttModel; label: string; size: number; installed: boolean; available: boolean }[];
```

`Dictation.tsx` :

```ts
const NAMED: [string, string][] = [
  ["fr", "Français"],
  ["en", "English"],
  ["de", "Deutsch"],
  ["es", "Español"],
  ["it", "Italiano"],
  ["pt", "Português"],
  ["nl", "Nederlands"],
];

/** Langues de la dictée, chacune dans sa langue ; « auto » dit celle retenue (`resolved`) s'il la connaît. */
const languages = (resolved: string | null = null): [string | null, string][] => {
  const name = resolved ? NAMED.find(([code]) => code === resolved)?.[1] : undefined;
  const auto = name ? t("ui-dictation-language-auto-resolved", { language: name }) : t("ui-dictation-language-auto");
  return [[null, auto], ...NAMED];
};
```

- `const langs = languages(status.config.model === "apple" ? status.detected : null);`
- Repos, « à télécharger » : `{t(model.size > 0 ? "ui-dictation-to-download" : "ui-dictation-to-download-language", { size: mo(model.size) })}`.
- Intro : `<p className="caption">{t(status.models.some((m) => m.id === "apple") ? "ui-dictation-model-intro-apple" : "ui-dictation-model-intro")}</p>`.
- Liste : `<input … disabled={!m.available} … />` ; badge :

```tsx
<span className={`badge${m.available && m.installed ? " ok" : ""}`}>
  {t(!m.available ? "ui-dictation-model-unavailable" : m.installed ? "ui-dictation-installed" : "ui-dictation-not-installed")}
</span>
```

- `modelNote` : `case "apple": return t("ui-dictation-note-apple");`.

Fluent `ui.ftl`, près des clés `ui-dictation-note-*` / `ui-dictation-model-*` :

fr :
```
ui-dictation-note-apple = Le plus rapide, intégré au système
ui-dictation-model-unavailable = Langue non prise en charge
ui-dictation-model-intro-apple = Apple, intégré au système, ou Whisper, sur cet ordinateur (CTranslate2). Tous comprennent plusieurs langues.
ui-dictation-language-auto-resolved = Automatique ({ $language })
ui-dictation-to-download-language = Langue à télécharger
```
en :
```
ui-dictation-note-apple = The fastest, built into the system
ui-dictation-model-unavailable = Language not supported
ui-dictation-model-intro-apple = Apple, built into the system, or Whisper, on this computer (CTranslate2). All of them understand several languages.
ui-dictation-language-auto-resolved = Automatic ({ $language })
ui-dictation-to-download-language = Language to download
```
Traduire dans les 18 autres langues.

Run: `cd ui && bun test && bun run typecheck && cd .. && cargo nextest run -p oku-slides-i18n`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add ui/src/api.ts ui/src/Dictation.tsx ui/src/Dictation.render.test.tsx crates/oku-slides-i18n/src/locales
```
```bash
git commit -m "feat(ui) : modèle de dictée Apple, langue retenue et langue non prise en charge (t-393)"
```

---

### Task 8: App macOS, autorisation et vérification finale

**Files:**
- Modify: `src-tauri/Info.plist`
- Modify (selon le constat du Step 2) : `crates/oku-slides-stt/swift/Sources/OkuSpeech/OkuSpeech.swift`, `crates/oku-slides-stt/src/stream_tests.rs`

**Interfaces:**
- Consumes: tout ce qui précède.

- [ ] **Step 1: Description d'usage de la reconnaissance vocale**

`src-tauri/Info.plist`, après `NSMicrophoneUsageDescription` :

```xml
  <key>NSSpeechRecognitionUsageDescription</key>
  <string>Oku Slides transcrit votre voix avec le moteur de dictée d'Apple, sur votre Mac.</string>
```

- [ ] **Step 2: Vérifier dans l'app empaquetée si l'autorisation est exigée (avec l'utilisateur)**

Run: `cargo tauri build --debug --bundles app`, puis ouvrir `target/debug/bundle/macos/Oku Slides.app`.

Demander à l'utilisateur de dicter 10–20 s en français (⌘D), puis de répondre : une demande d'autorisation « Reconnaissance vocale » est-elle apparue ? Le texte est-il arrivé, et vite ? Réglages Système › Confidentialité et sécurité › Reconnaissance vocale liste-t-il Oku Slides ?

- **Pas de demande, texte reçu** → l'autorisation n'est pas exigée : passer au Step 4, noter le constat dans la spec (section « Erreurs et autorisations »).
- **Erreur ou demande** → Step 3.

- [ ] **Step 3 (seulement si exigée) : demander l'autorisation avant la dictée**

Test d'abord (`stream_tests.rs`, déjà couvert : `ERR_DENIED` → `Permission`). Dans `OkuSpeech.swift`, `okuSpeechStart`, avant `blocking(Locale?.none)` :

```swift
    let allowed = blocking(false) {
        await withCheckedContinuation { (done: CheckedContinuation<Bool, Never>) in
            if SFSpeechRecognizer.authorizationStatus() == .authorized { return done.resume(returning: true) }
            SFSpeechRecognizer.requestAuthorization { done.resume(returning: $0 == .authorized) }
        }
    }
    guard allowed else {
        sink.close(kindError, "", errDenied)
        return nil
    }
```

avec `private let errDenied = 4.0` à côté des autres codes. Rebâtir l'app, refaire le Step 2 : accepter → la dictée marche ; refuser (Réglages Système) → message « Autorisez la reconnaissance vocale… ».

- [ ] **Step 4: Vérification complète**

Run: `just check && just test-apple && (cd ui && bun test && bun run typecheck)`
Expected: PASS.

Puis dans l'app (avec l'utilisateur) : première ouverture après mise à jour → modèle « Apple » choisi ; dictée française de 20–30 s, texte final perçu comme immédiat ; langue « xx » impossible à choisir dans la liste, mais une langue non gérée grise l'entrée Apple ; retour à « Large v3 Turbo » depuis la liste, dictée toujours fonctionnelle.

- [ ] **Step 5: Commit et fin**

```bash
git add src-tauri/Info.plist crates/oku-slides-stt/swift docs/superpowers/specs/2026-10-03-dictee-native-macos-design.md
```
```bash
git commit -m "feat(app) : autorisation de la reconnaissance vocale pour le moteur d'Apple (t-393)"
```

Retirer l'exemption de revue du worktree : `git config --worktree --unset oku.review`.
