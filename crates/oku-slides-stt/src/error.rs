//! Erreurs de la dictée : micro, fichier audio, modèle, transcription,
//! téléchargement.

use std::path::PathBuf;

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_platform::models::{DownloadError, DownloadErrorKind};
use oku_slides_platform::storage::StorageError;
use snafu::{Backtrace, Snafu};

/// Erreur d'une bibliothèque qui n'expose pas de type précis (CTranslate2,
/// tokenizers).
pub(crate) type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub(crate) fn boxed(e: impl Into<BoxError>) -> BoxError {
    e.into()
}

/// Une dictée impossible.
#[derive(Debug, Snafu)]
pub struct SttError(Box<SttInner>);

/// Nature d'une [`SttError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SttErrorKind {
    /// Pas de micro, micro inutilisable ou muet.
    Microphone,
    /// Fichier audio illisible (micro simulé, transcription d'un fichier).
    AudioFile,
    /// Modèle absent ou illisible.
    Model,
    Transcription,
    Download,
    /// Téléchargement annulé par l'utilisateur.
    Cancelled,
    /// Autorisation refusée (reconnaissance vocale).
    Permission,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum SttInner {
    #[snafu(display("no microphone found"))]
    NoMicrophone { backtrace: Backtrace },
    #[snafu(display("unusable microphone"))]
    MicrophoneConfig { source: cpal::Error, backtrace: Backtrace },
    #[snafu(display("unusable microphone"))]
    MicrophoneStream { source: cpal::Error, backtrace: Backtrace },
    #[snafu(display("the microphone does not start"))]
    MicrophonePlay { source: cpal::Error, backtrace: Backtrace },
    #[snafu(display("unsupported audio format: {format}"))]
    UnsupportedFormat { format: String, backtrace: Backtrace },
    #[snafu(display("the microphone does not respond"))]
    MicrophoneTimeout { backtrace: Backtrace },
    #[snafu(display("unreadable audio file {}", path.display()))]
    AudioFile { path: PathBuf, source: StorageError, backtrace: Backtrace },
    #[snafu(display("invalid WAV file: {reason}"))]
    Wav { reason: &'static str, backtrace: Backtrace },
    #[snafu(display("unreadable dictation model ({})", dir.display()))]
    ModelLoad { dir: PathBuf, source: BoxError, backtrace: Backtrace },
    #[snafu(display("unreadable model file {}", path.display()))]
    ModelFile { path: PathBuf, source: StorageError, backtrace: Backtrace },
    #[snafu(display("dictation model has no {name} ({})", dir.display()))]
    MissingModelFile { dir: PathBuf, name: &'static str, backtrace: Backtrace },
    #[snafu(display("invalid model settings"))]
    ModelSettings { source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("invalid model settings: {field} must be positive"))]
    ZeroSetting { field: &'static str, backtrace: Backtrace },
    #[snafu(display("invalid mel filters"))]
    MelFilters { source: ndarray::ShapeError, backtrace: Backtrace },
    #[snafu(display("transcription failed ({step})"))]
    Transcribe { step: &'static str, source: BoxError, backtrace: Backtrace },
    #[snafu(display("transcription failed ({step})"))]
    Features { step: &'static str, source: ndarray::ShapeError, backtrace: Backtrace },
    #[snafu(display("language not detected"))]
    NoLanguage { backtrace: Backtrace },
    #[snafu(display("audio features are not contiguous"))]
    NotContiguous { backtrace: Backtrace },
    #[snafu(display("download cancelled"))]
    Cancelled { backtrace: Backtrace },
    #[snafu(display("model download failed"))]
    Download { source: DownloadError, backtrace: Backtrace },
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
}

impl SttError {
    pub fn kind(&self) -> SttErrorKind {
        use SttInner::*;
        match *self.0 {
            NoMicrophone { .. }
            | MicrophoneConfig { .. }
            | MicrophoneStream { .. }
            | MicrophonePlay { .. }
            | UnsupportedFormat { .. }
            | MicrophoneTimeout { .. } => SttErrorKind::Microphone,
            AudioFile { .. } | Wav { .. } => SttErrorKind::AudioFile,
            ModelLoad { .. }
            | ModelFile { .. }
            | MissingModelFile { .. }
            | ModelSettings { .. }
            | ZeroSetting { .. }
            | MelFilters { .. }
            | LocaleUnsupported { .. }
            | NotDownloadable { .. } => SttErrorKind::Model,
            Transcribe { .. }
            | Features { .. }
            | NoLanguage { .. }
            | NotContiguous { .. }
            | Analyzer { .. }
            | FinishTimeout { .. } => SttErrorKind::Transcription,
            Cancelled { .. } => SttErrorKind::Cancelled,
            Download { ref source, .. } if source.kind() == DownloadErrorKind::Cancelled => SttErrorKind::Cancelled,
            Download { .. } | AssetInstall { .. } => SttErrorKind::Download,
            SpeechDenied { .. } => SttErrorKind::Permission,
        }
    }
}

impl Localize for SttError {
    fn localize(&self, l: &Localizer) -> String {
        use SttInner::*;
        match &*self.0 {
            NoMicrophone { .. } => l.tr("stt-microphone-missing"),
            MicrophoneConfig { .. } | MicrophoneStream { .. } | MicrophonePlay { .. } => {
                l.tr("stt-microphone-unusable")
            }
            MicrophoneTimeout { .. } => l.tr("stt-microphone-silent"),
            UnsupportedFormat { format, .. } => l.tr_with("stt-audio-format", &[("format", format.as_str().into())]),
            AudioFile { .. } | Wav { .. } => l.tr("stt-audio-file"),
            ModelLoad { .. }
            | ModelFile { .. }
            | MissingModelFile { .. }
            | ModelSettings { .. }
            | ZeroSetting { .. }
            | MelFilters { .. }
            | NotDownloadable { .. } => l.tr("stt-model"),
            Transcribe { .. }
            | Features { .. }
            | NoLanguage { .. }
            | NotContiguous { .. }
            | Analyzer { .. }
            | FinishTimeout { .. } => l.tr("stt-transcription"),
            Download { source, .. } => match source.kind() {
                DownloadErrorKind::Cancelled => l.tr("stt-cancelled"),
                DownloadErrorKind::Incomplete { got, expected } => {
                    l.tr_with("stt-download-incomplete", &[("got", got.into()), ("expected", expected.into())])
                }
                _ => l.tr("stt-download"),
            },
            AssetInstall { .. } => l.tr("stt-download"),
            Cancelled { .. } => l.tr("stt-cancelled"),
            SpeechDenied { .. } => l.tr("stt-speech-denied"),
            LocaleUnsupported { locale, .. } => {
                l.tr_with("stt-locale-unsupported", &[("language", locale.as_str().into())])
            }
        }
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod error_tests;
