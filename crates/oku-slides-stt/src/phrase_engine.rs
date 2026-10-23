//! Moteurs qui transcrivent une phrase entière : Whisper local, API distante.

use crate::{SttError, Transcriber, Transcript};

/// Un moteur qui transcrit une phrase entière.
///
/// Implémentez ce trait pour brancher un moteur phrase par phrase (Whisper, une API).
pub(crate) trait PhraseEngine: Send + Sync {
    /// Transcrit un audio mono 16 kHz ; `quick` : aperçu (décodage rapide).
    fn transcribe(
        &self,
        audio: &[f32],
        language: Option<&str>,
        context: Option<&str>,
        quick: bool,
    ) -> Result<Transcript, SttError>;
}

impl PhraseEngine for Transcriber {
    fn transcribe(
        &self,
        audio: &[f32],
        language: Option<&str>,
        context: Option<&str>,
        quick: bool,
    ) -> Result<Transcript, SttError> {
        Transcriber::transcribe(self, audio, language, context, quick)
    }
}
