//! Erreurs des trames du tube.

use snafu::{Backtrace, Snafu};

/// Échec de lecture ou d'écriture d'une trame.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum FrameError {
    #[snafu(display("pipe i/o failed"))]
    Io { source: std::io::Error, backtrace: Backtrace },
    #[snafu(display("truncated frame"))]
    Truncated { backtrace: Backtrace },
    /// `payload` : la charge dépasse sa borne (sinon l'en-tête).
    #[snafu(display("frame of {len} bytes exceeds {max}"))]
    TooLarge { len: usize, max: usize, payload: bool, backtrace: Backtrace },
    #[snafu(display("frame header is not valid json"))]
    Json { source: serde_json::Error, backtrace: Backtrace },
}
