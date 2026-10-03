# oku-slides-stt

Dictée d'Oku Slides : micro → détection de la voix → moteur → texte. Interrogé
par sondage (`Stt::status`), pour l'app Tauri comme pour le serveur de dev.

## Moteurs

- **Apple** (`Model::Apple`, macOS 26+, réglage `stt.apple`) : `SpeechAnalyzer`
  en continu, par un pont Swift statique (`swift/`, `build.rs`, `apple.rs`).
  Langue résolue par `locale.rs`, installée par le système.
  Compiler sur macOS exige Xcode avec le SDK de macOS 26 ou plus ; l'app
  tourne dès macOS 12 (Whisper seul avant macOS 26).
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
