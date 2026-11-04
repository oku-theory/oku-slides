//! The Windows voice: the `WinRT` `SpeechSynthesizer`, played by a `MediaPlayer` on its own thread.
//!
//! Each speech gets a thread that synthesizes the plain text (`SynthesizeTextToStreamAsync`,
//! never SSML), plays it, and waits for `MediaEnded`, `MediaFailed` or a cut. Both handlers
//! share one take-once DONE with the thread, and both event tokens are revoked when the speech
//! ends or is cut. `WinRT` activation on that thread joins the MTA by itself (windows-core).

use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex, PoisonError, mpsc};

use ::windows::Foundation::TypedEventHandler;
use ::windows::Media::Core::MediaSource;
use ::windows::Media::Playback::{MediaPlayer, MediaPlayerFailedEventArgs};
use ::windows::Media::SpeechSynthesis::{SpeechSynthesizer, VoiceInformation};
use ::windows::core::{HSTRING, IInspectable};

use super::{
    SpeechDone, SpeechEngine, SpeechId, SpeechRate, SpeechStatus, UnavailableReason, VoiceInfo, chosen, for_language,
    install,
};
use crate::error::{SttError, VoiceFailedSnafu, VoiceUnavailableSnafu, VoiceWinRtSnafu};

/// The DONE of one speech, taken once by whichever end comes first.
type TakeOnce = Mutex<Option<SpeechDone>>;

/// The Windows OS voice.
#[derive(Debug, Default)]
pub(crate) struct WindowsSpeechEngine {
    /// The speech last started and the way to cut it, until it is replaced or cut.
    current: Mutex<Option<(SpeechId, mpsc::Sender<()>)>>,
}

/// What one speech thread says, and how.
struct Speech {
    id: SpeechId,
    text: String,
    /// The voice `speak` found: the thread never lists the voices again.
    voice: VoiceInformation,
    /// Factor of the default rate, as `SpeakingRate` takes it.
    rate: f64,
}

impl fmt::Debug for Speech {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Spoken text is never logged.
        f.debug_struct("Speech").field("id", &self.id).field("rate", &self.rate).finish_non_exhaustive()
    }
}

/// The installed voice `id`, if any.
fn find_voice(id: &str) -> ::windows::core::Result<Option<VoiceInformation>> {
    let list = SpeechSynthesizer::AllVoices()?;
    for i in 0..list.Size()? {
        let candidate = list.GetAt(i)?;
        if candidate.Id()? == id {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

/// Every installed voice.
fn all_voices() -> ::windows::core::Result<Vec<VoiceInfo>> {
    let list = SpeechSynthesizer::AllVoices()?;
    let mut voices = Vec::new();
    for i in 0..list.Size()? {
        let voice = list.GetAt(i)?;
        voices.push(VoiceInfo {
            id: voice.Id()?.to_string_lossy(),
            name: voice.DisplayName()?.to_string_lossy(),
            lang: voice.Language()?.to_string_lossy(),
        });
    }
    Ok(voices)
}

/// Calls the speech's DONE if no one has yet.
fn finish(once: &TakeOnce, id: SpeechId) {
    let done = once.lock().unwrap_or_else(PoisonError::into_inner).take();
    if let Some(done) = done {
        done.call(id);
    }
}

/// Synthesizes and plays `speech`, until it ends, fails or `cut` receives.
fn play(
    speech: &Speech,
    once: &Arc<TakeOnce>,
    ended: &mpsc::Sender<()>,
    cut: &mpsc::Receiver<()>,
) -> ::windows::core::Result<()> {
    let Speech { id, text, voice, rate } = speech;
    let id = *id;
    let synth = SpeechSynthesizer::new()?;
    // Never the default voice instead: one removed since `speak` found it fails here.
    synth.SetVoice(voice)?;
    synth.Options()?.SetSpeakingRate(*rate)?;
    let stream = synth.SynthesizeTextToStreamAsync(&HSTRING::from(text.as_str()))?.join()?;
    let player = MediaPlayer::new()?;
    player.SetSource(&MediaSource::CreateFromStream(&stream, &stream.ContentType()?)?)?;
    let on_ended = {
        let (once, ended) = (Arc::clone(once), ended.clone());
        TypedEventHandler::<MediaPlayer, IInspectable>::new(move |_, _| {
            finish(&once, id);
            ended.send(()).ok();
            Ok(())
        })
    };
    let on_failed = {
        let (once, ended) = (Arc::clone(once), ended.clone());
        TypedEventHandler::<MediaPlayer, MediaPlayerFailedEventArgs>::new(move |_, _| {
            finish(&once, id);
            ended.send(()).ok();
            Ok(())
        })
    };
    let ended_token = player.MediaEnded(&on_ended)?;
    let failed_token = match player.MediaFailed(&on_failed) {
        Ok(token) => token,
        Err(e) => {
            player.RemoveMediaEnded(ended_token).ok();
            player.Close().ok();
            return Err(e);
        }
    };
    if cut.try_recv().is_ok() {
        // Cut while synthesizing: never start playing.
        player.RemoveMediaEnded(ended_token).ok();
        player.RemoveMediaFailed(failed_token).ok();
        player.Close().ok();
        return Ok(());
    }
    let started = player.Play();
    if started.is_ok() {
        // MediaEnded, MediaFailed or a cut; an error means the engine is gone: a cut too.
        cut.recv().ok();
    }
    player.RemoveMediaEnded(ended_token).ok();
    player.RemoveMediaFailed(failed_token).ok();
    player.Close().ok();
    started
}

impl SpeechEngine for WindowsSpeechEngine {
    fn status(&self, lang: &str) -> SpeechStatus {
        match all_voices() {
            Ok(voices) if !voices.is_empty() => for_language(lang, voices),
            Ok(_) => SpeechStatus::Unavailable { reason: UnavailableReason::NoEngine },
            Err(e) => {
                tracing::warn!(code = e.code().0, "Windows voices unreadable");
                SpeechStatus::Unavailable { reason: UnavailableReason::NoEngine }
            }
        }
    }

    fn speak(
        &self,
        id: SpeechId,
        text: &str,
        voice: Option<&str>,
        rate: SpeechRate,
        done: SpeechDone,
    ) -> Result<(), SttError> {
        let wanted = chosen(voice)?;
        let found = find_voice(wanted)
            .map_err(|e| SttError::from(VoiceWinRtSnafu { step: "voices", code: e.code().0 }.build()))?;
        let Some(found) = found else {
            return Err(VoiceUnavailableSnafu { reason: UnavailableReason::NoVoiceForLanguage }.build().into());
        };
        let (tx, rx) = mpsc::channel();
        let ended = tx.clone();
        let previous = install(&mut self.current.lock().unwrap_or_else(PoisonError::into_inner), id, tx);
        if let Some(cut) = previous {
            cut.send(()).ok();
        }
        let speech = Speech { id, text: text.to_owned(), voice: found, rate: f64::from(rate.get()) };
        let spawned = std::thread::Builder::new().name("oku-voice".into()).spawn(move || {
            let once = Arc::new(Mutex::new(Some(done)));
            let played = catch_unwind(AssertUnwindSafe(|| play(&speech, &once, &ended, &rx)));
            match played {
                Ok(Ok(())) => {}
                Ok(Err(e)) => tracing::warn!(id = id.get(), code = e.code().0, "Windows voice failed"),
                Err(_) => tracing::error!(id = id.get(), "Windows voice thread panicked"),
            }
            // DONE once, however the speech ended.
            finish(&once, id);
        });
        if let Err(e) = spawned {
            // No thread: `done` was dropped with the closure.
            let mut g = self.current.lock().unwrap_or_else(PoisonError::into_inner);
            if g.as_ref().is_some_and(|(current, _)| *current == id) {
                *g = None;
            }
            tracing::warn!(error = %e, "Windows voice thread did not start");
            return Err(VoiceFailedSnafu { step: "thread" }.build().into());
        }
        Ok(())
    }

    fn stop(&self, id: SpeechId) -> Result<(), SttError> {
        let cut = {
            let mut g = self.current.lock().unwrap_or_else(PoisonError::into_inner);
            if g.as_ref().is_some_and(|(current, _)| *current == id) { g.take() } else { None }
        };
        if let Some((_, cut)) = cut {
            cut.send(()).ok();
        }
        Ok(())
    }
}

impl Drop for WindowsSpeechEngine {
    fn drop(&mut self) {
        let current = self.current.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some((_, cut)) = current {
            cut.send(()).ok();
        }
    }
}
