# Dictée par le moteur natif Apple — design

Date : 2026-10-03 · Tâche : t-393

## Objectif

La dictée doit rendre le texte dès qu'on a fini de parler. Aujourd'hui,
Whisper large-v3-turbo (CTranslate2, CPU + Accelerate, beam 5 en final) fait
attendre ~4–8 s après la dernière phrase. Sur macOS 26+, le moteur du
système (`SpeechAnalyzer` / `SpeechTranscriber`) transcrit en continu sur le
Neural Engine : le texte final arrive ~0,1 s après la fin de la voix.

Il devient le modèle par défaut sur macOS 26+ ; Whisper reste le moteur sur
Windows, Linux et macOS < 26, et un choix possible sur Mac.

Hors périmètre : moteur natif Windows (`Windows.Media.SpeechRecognition`) et
Linux ; détection automatique de la langue côté Apple.

## Mesures du spike

23,8 s de dictée française (voix de synthèse « Thomas », `say`), M1 Max,
machine chargée (load average 160–178) pour tous les essais.

| Moteur | Prêt en | Fichier entier | Attente après la parole |
|---|---|---|---|
| Apple `SpeechTranscriber` | 0,26 s | 1,0–1,6 s (15–24× le temps réel) | 0,05–0,1 s |
| Apple `DictationTranscriber` | — | 1,0 s | 0,8 s |
| Whisper turbo | 0,9–1,3 s | 19–31 s (0,8–1,2×) | ≈ 4–8 s (estimée) |
| Whisper base | 0,2–0,4 s | 4,4–21 s | ≈ 1–5 s (estimée) |

Qualité comparable : Apple formate les nombres (« 12 % ») et ponctue bien,
mais confond des homophones (« mes trois puces », « réduit ») et ne met ni
guillemets ni point final ; Whisper turbo est un peu meilleur en
orthographe. Sans l'option `.fastResults`, le premier aperçu Apple n'arrive
qu'au bout de 12 s ; avec, à 1 s. Le modèle français était déjà installé
sur le système.

## Décisions

- **Un modèle de plus.** `Model::Apple` (« Apple (système) ») rejoint Turbo,
  Large et Base dans la liste existante ; l'UI ne change presque pas.
- **« Auto » = langue de l'interface.** `SpeechTranscriber` exige une
  locale : « Auto » prend la langue de l'UI (localizer), à défaut la locale
  du système. L'UI affiche « Auto (français) » via `status.detected`.
- **Bascule unique.** `Config` gagne `version` ; une config sans version,
  sur une machine où Apple est `Ready` ou `Missing`, passe à `Apple` au
  chargement, en mémoire seulement : la bascule est idempotente, et une mise
  à jour du système la déclenche plus tard. Le fichier n'est réécrit qu'au
  prochain `configure`. Ensuite, le choix de l'utilisateur est respecté.
- **Pont Swift statique.** `SpeechAnalyzer` n'existe qu'en Swift
  (async/`AsyncSequence`), sans API Objective-C : une petite bibliothèque
  Swift statique, en ABI C (`@_cdecl`), liée au crate `oku-slides-stt`.
  Écartés : un binaire Swift séparé (cycle de vie, signature, deux
  protocoles) et `swift-bridge` (trop lourd pour six fonctions).
- **Vocabulaire en liste.** `start { prompt: string }` devient
  `start { vocabulary: string[] }` (titres du deck). Apple les reçoit en
  `contextualStrings` ; Whisper les rejoint par « . » (tronqués à 600
  caractères) pour son prompt. Pas de rétrocompatibilité.
- **macOS 12 minimum.** Le pont Swift lie le runtime Swift Concurrency,
  livré par l'OS depuis macOS 12 : l'app, le CLI et le paquet Swift se
  déploient en 12.0 (`tauri.conf.json`, `.cargo/config.toml`, `build.rs`,
  `Package.swift`). Le moteur d'Apple reste réservé à macOS 26+.
- **Pas de repli silencieux.** Une langue que le moteur Apple ne gère pas
  grise l'entrée Apple ; on ne bascule pas en douce sur Whisper.

## Modules

```
crates/oku-slides-stt/
├── swift/                     paquet Swift (macOS seulement), ~200 lignes
│   ├── Package.swift
│   └── Sources/OkuSpeech/OkuSpeech.swift
├── build.rs                   compile swift/ via swift-rs, cfg(target_os = "macos")
└── src/
    ├── models.rs              Model::Apple
    ├── stream.rs              Availability, Event, traits Stream et StreamEngine,
    │                          NoStreamEngine (hors macOS ou `stt.apple = false`)
    ├── apple.rs               StreamEngine d'Apple, FFI au-dessus de swift/ (seul `unsafe`)
    ├── locale.rs              « Auto » → langue de l'UI → locale système
    ├── results.rs             accumulateur pur des résultats en continu
    ├── dictation_whisper.rs   boucle de transcription Whisper (sortie de lib.rs, inchangée)
    ├── dictation_stream.rs    boucle des résultats en continu, téléchargement de la langue
    └── lib.rs                 listen_loop alimente le moteur choisi
```

`lib.rs` fait 691 lignes : la boucle Whisper en sort d'abord
(`dictation_whisper.rs`), sans changer son comportement.

### `Model::Apple` et disponibilité

```rust
pub enum Availability { Unsupported, Missing, Ready }
```

- `availability(locale)` interroge le système (`SpeechTranscriber.
  installedLocales` / `supportedLocales`) ; hors macOS ou sans le moteur,
  `NoStreamEngine` (`stream.rs`) renvoie toujours `Unsupported`. Les modèles
  Whisper gardent `is_installed` (marqueur `.complete`).
- `Model::ALL` liste Apple en premier. `status()` omet Apple quand il est
  `Unsupported` pour la plateforme (hors Mac, macOS < 26).
- `ModelInfo` gagne `available: bool` : `false` quand Apple ne gère pas la
  langue choisie (entrée grisée, « Langue non prise en charge »).
- `Model::Apple.size()` vaut 0 ; la progression de téléchargement vient
  d'`AssetInventory`.

### `StreamEngine`

Traits internes (pas `pub`) des moteurs **en continu** : `StreamEngine`
(`supported`, `availability`, `install`, `open`) ouvre un `Stream` (`push`,
`finish`, `cancel`) et rend un `mpsc::Receiver<Event>`. Whisper ne les
implémente pas : il garde ses jobs. `Stt` tient un
`Arc<dyn StreamEngine>` : `AppleEngine` sur macOS quand le réglage
`stt.apple` vaut `true` (défaut), sinon `NoStreamEngine` (toujours
`Unsupported`). Les tests injectent un moteur factice.

`listen_loop` reste propriétaire du micro, du
rééchantillonnage à 16 kHz, du niveau, d'earshot (`speaking`, `stop_in`,
arrêt automatique) ; seule la destination de l'audio change :

- Whisper : phrases closes et aperçus en jobs `Final` / `Preview` vers
  `transcribe_loop`, comme aujourd'hui ;
- Apple : chaque bloc rééchantillonné part dans `apple::Session::push()` ;
  les phrases d'earshot ne servent qu'à l'arrêt automatique.

### Pont Swift (`swift/` + `apple.rs`)

ABI C exposée par `@_cdecl` :

| Fonction | Rôle |
|---|---|
| `oku_speech_availability(locale) -> i32` | `Unsupported` / `Missing` / `Ready` |
| `oku_speech_install(locale, ctx, cb)` | télécharge la langue, progression par `cb` |
| `oku_speech_start(locale, vocab, n, ctx, cb) -> handle` | ouvre une session |
| `oku_speech_push(handle, samples, len)` | f32 mono 16 kHz |
| `oku_speech_finish(handle)` | finalise, puis émet `Done` |
| `oku_speech_cancel(handle)` | abandonne (`cancelAndFinishNow`) |

- Le Swift convertit le 16 kHz vers `bestAvailableAudioFormat` par
  `AVAudioConverter` ; Rust n'envoie qu'un seul format, celui de Whisper.
- `SpeechTranscriber` avec `reportingOptions: [.volatileResults,
  .fastResults]` et `attributeOptions: [.audioTimeRange]`.
- Un seul callback C porte tous les événements :
  `Partial(texte)`, `Final(texte, début, fin)`, `Progress(reçu, total)`,
  `Done`, `Error(code, message)`.
- Le callback arrive sur un fil Swift : il ne fait qu'un `send` dans un
  `mpsc`. Seul le fil Rust de la dictée écrit dans `Inner`.
- Le Swift n'a ni `try!` ni `!` : un plantage Swift ferait tomber l'app.
  Toute erreur devient `Error(code, message)`.
- `apple.rs` enveloppe le handle dans `Session` (`Drop` appelle
  `oku_speech_cancel`) et contient tout l'`unsafe` du crate, chaque bloc
  avec son commentaire `SAFETY:`.

## Déroulé d'une dictée Apple

```
UI ─ start(vocabulary) ─▶ Stt::start
                           │ locale = locale::resolve(config.language, langue UI, système)
                           │ availability(locale) ─ Missing ─▶ download(Apple) ─▶ reprise
                           ▼ Ready
                   begin_recording
                     ├─ micro (cpal)                          en parallèle
                     └─ apple::Session::start(locale, vocab)  (≈0,3 s ; Swift met en tampon)
                           │
listen_loop (tick) : micro → 16 kHz ─┬─▶ earshot : speaking, silence, stop_in, arrêt auto
                                     └─▶ session.push(échantillons)
                           │
fil Swift : SpeechTranscriber ─ callback ─▶ mpsc ─▶ fil « résultats » Rust
                                              ├─ Partial → Inner.partial
                                              └─ Final   → Inner.phrases (clean())
                           │
Stop (bouton ou auto) : session.finish() → derniers Final, puis Done
   └─ fil résultats : text = committed, seq += 1, state = Idle
Cancel : session.cancel(), session += 1 → les callbacks tardifs sont ignorés
```

- `results::apply(&mut Inner, Event)` est une fonction pure : `Partial`
  remplace `partial` ; `Final` pousse `clean(texte)` dans `phrases` et vide
  `partial`.
- `detected` reçoit la locale résolue.
- Pas de `context_of` pour Apple : la session est un seul flux continu, le
  moteur garde lui-même le contexte de ce qui précède.
- `transcribe_samples` (exemple `transcribe`, tests) suit le même chemin :
  tout pousser, `finish`, rassembler les `Final` ; les `TimedPhrase`
  viennent d'`audioTimeRange`.
- Téléchargement de la langue : `Stt::download(Model::Apple)` appelle
  `oku_speech_install` ; même état `Downloading`, même `Download` dans
  `Status`, même reprise de la dictée à la fin.

## Erreurs et autorisations

Nouvelles variantes Snafu (avec `backtrace`) dans `error.rs` :

| Variante | `SttErrorKind` | Message |
|---|---|---|
| `SpeechDenied` | `Permission` (nouveau) | `stt-speech-denied` |
| `LocaleUnsupported { locale }` | `Model` | `stt-locale-unsupported` |
| `AssetInstall { reason }` | `Download` | `stt-download` |
| `Analyzer { reason }` | `Transcription` | `stt-transcription` |
| `NotDownloadable` | `Model` | `stt-model` |
| `FinishTimeout` | `Transcription` | `stt-transcription` |

- `apple.rs` traduit les codes du pont en variantes ; la table de
  correspondance est testée.
- `src-tauri/Info.plist` gagne `NSSpeechRecognitionUsageDescription`.
- `NotDownloadable` : `models::download(Model::Apple)` refuse (la langue
  d'Apple s'installe par `AssetInventory`, pas par Hugging Face).
- Autorisation « Reconnaissance vocale » : le spike, lancé depuis le
  terminal, ne l'a pas demandée. **Dès que l'app sait dicter avec Apple,
  vérifier dans l'app empaquetée si `SpeechAnalyzer` l'exige** (il faut
  pouvoir dicter pour le savoir : avant-dernière tâche du plan). Si oui, `start` appelle
  `SFSpeechRecognizer.requestAuthorization` quand le statut est
  `notDetermined`, et un refus donne `SpeechDenied`.
  **Constat (2026-10-03, app empaquetée, macOS 27) : aucune demande
  d'autorisation, la dictée fonctionne.** `SpeechAnalyzer` n'exige pas
  l'autorisation de reconnaissance vocale : pas de `requestAuthorization`.
  `SpeechDenied` et le code `DENIED` restent dans l'interface du pont,
  sans émetteur côté Swift.
- Disponibilité d'une langue : d'après `SpeechTranscriber.installedLocales`
  / `supportedLocales` (constat : `AssetInventory.status` ne dit
  « installé » que pour les langues réservées par l'app). Une langue sans
  région prend la région du système (`fr` → `fr_FR` ; sinon le système
  choisissait `fr_BE`).
- Si `Done` n'arrive pas après `finish()` : délai
  `stt.apple_finish_timeout_secs` (`SttSettings`, défaut dans le TOML
  embarqué), puis `FinishTimeout` et retour à `Idle` ; le texte déjà
  finalisé est gardé.
- Réglages (`[stt]`, TOML embarqué) : `apple = true` (proposer le moteur
  d'Apple ; `false` le retire, et les tests l'éteignent pour rester
  déterministes sur un Mac sous macOS 26) et `apple_finish_timeout_secs`.
- Nouvelles clés Fluent dans les 20 locales : `stt-speech-denied`,
  `stt-locale-unsupported`, `stt-model-apple` (stt.ftl) ;
  `ui-dictation-note-apple`, `ui-dictation-model-unavailable`,
  `ui-dictation-language-auto-resolved`, `ui-dictation-to-download-language`,
  `ui-dictation-model-intro-apple` (ui.ftl ; l'intro qui cite les deux
  moteurs ne s'affiche que si Apple est proposé).

## Tests

1. **Unitaires, toutes plateformes** : `locale::resolve` (auto, langue d'UI
   non gérée, langue explicite) ; migration de `Config` (sans version +
   Ready/Missing → Apple, sans version + Unsupported → inchangé, avec
   version → inchangé) ; vocabulaire Whisper (jointure, troncature) ;
   `results::apply` (ordre des finaux, `partial` vidé, `clean`) ; codes du
   pont → variantes.
2. **Flux avec moteur factice** : `ScriptedEngine` (`#[cfg(test)]`, un
   `StreamEngine`) émet des événements scriptés. Cas : start → résultats → stop →
   `Idle` + `text` ; cancel avec callback tardif ignoré ; `finish` sans
   `Done` → `FinishTimeout` + `Idle` ; arrêt automatique par earshot →
   `finish`. Réutilise `fake_mic` et `wait_until` de `tests/dictation.rs`.
3. **Intégration Apple réelle** : `tests/apple.rs`,
   `#[cfg(target_os = "macos")]`, `#[ignore = "macOS 26 + langue fr
   installée"]`, lancé par `just test-apple`. Fixture
   `fixtures/stt/dictee-fr.wav` (~8 s, `say`, 16 kHz). Vérifie des
   mots-clés (« diapositive », « trimestre ») et une attente après
   `finish` < 2 s.
4. **UI** (`bun test`) : entrée Apple grisée si `available: false` ;
   « Auto (français) » depuis `detected` ; `stt.start(vocabulary)`.
5. **Outil** : l'exemple `transcribe` accepte `apple`
   (`just transcribe <wav> apple fr`) pour comparer les moteurs.

## Validation manuelle

Dans l'app sur macOS 26 : première dictée après mise à jour (bascule vers
Apple), dictée française de 20–30 s avec sa propre voix, texte final perçu
comme immédiat ; choix d'une langue non gérée (entrée grisée) ; retour à
Whisper Turbo depuis la liste.
