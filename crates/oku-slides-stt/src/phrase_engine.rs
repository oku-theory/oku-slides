//! Engines that transcribe a whole phrase: local Parakeet, remote API.

use crate::SttError;

/// A transcription's result.
#[derive(Debug, Clone, Default)]
pub struct Transcript {
    /// Transcribed text, as returned by the model.
    pub text: String,
    /// Language the engine reports (`fr`, `en`…); `None` when it reports none (Parakeet).
    pub language: Option<String>,
    /// Probability that there is no speech (0..1); 0 when the engine does not estimate it.
    pub no_speech: f32,
}

/// An engine that transcribes a whole phrase.
///
/// Implement this trait to plug in a phrase-by-phrase engine (Parakeet, an API).
pub(crate) trait PhraseEngine: Send + Sync {
    /// Transcribes mono 16 kHz audio; `quick`: a preview (fast decoding).
    fn transcribe(
        &self,
        audio: &[f32],
        language: Option<&str>,
        context: Option<&str>,
        quick: bool,
    ) -> Result<Transcript, SttError>;
}
