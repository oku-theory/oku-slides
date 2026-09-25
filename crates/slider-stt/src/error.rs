//! Erreurs de la dictée : micro, fichier audio, modèle, transcription,
//! téléchargement.

use std::path::PathBuf;

use slider_i18n::{Localize, Localizer};
use slider_platform::storage::StorageError;
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
    #[snafu(display("invalid model settings"))]
    ModelSettings { source: serde_json::Error, backtrace: Backtrace },
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
    #[snafu(display("download impossible: {file}"))]
    Http { file: String, source: Box<ureq::Error>, backtrace: Backtrace },
    #[snafu(display("download interrupted: {file}"))]
    HttpRead { file: String, source: std::io::Error, backtrace: Backtrace },
    #[snafu(display("writing the model failed ({})", path.display()))]
    ModelWrite { path: PathBuf, source: StorageError, backtrace: Backtrace },
    #[snafu(display("incomplete file: {got} bytes out of {expected}"))]
    Incomplete { got: u64, expected: u64, backtrace: Backtrace },
    #[snafu(display("download cancelled"))]
    Cancelled { backtrace: Backtrace },
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
            ModelLoad { .. } | ModelFile { .. } | ModelSettings { .. } | MelFilters { .. } => SttErrorKind::Model,
            Transcribe { .. } | Features { .. } | NoLanguage { .. } | NotContiguous { .. } => {
                SttErrorKind::Transcription
            }
            Http { .. } | HttpRead { .. } | ModelWrite { .. } | Incomplete { .. } => SttErrorKind::Download,
            Cancelled { .. } => SttErrorKind::Cancelled,
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
            ModelLoad { .. } | ModelFile { .. } | ModelSettings { .. } | MelFilters { .. } => l.tr("stt-model"),
            Transcribe { .. } | Features { .. } | NoLanguage { .. } | NotContiguous { .. } => l.tr("stt-transcription"),
            Incomplete { got, expected, .. } => {
                l.tr_with("stt-download-incomplete", &[("got", (*got).into()), ("expected", (*expected).into())])
            }
            Http { .. } | HttpRead { .. } | ModelWrite { .. } => l.tr("stt-download"),
            Cancelled { .. } => l.tr("stt-cancelled"),
        }
    }
}
