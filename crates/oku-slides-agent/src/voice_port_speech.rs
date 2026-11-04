//! The read-aloud in the settings box and `set_speech`: its view, checks and saving.
//!
//! The settings live in the dictation's `config.json` ([`Stt::configure_speech`]); the
//! speaker says which voices read the interface language. A missing voice is told by
//! [`UnavailableReason`], localized here. `Speaker::status` may block: never on async threads.

use std::sync::PoisonError;

use oku_slides_i18n::Localize;
use oku_slides_mcp::{Checked, VoiceApplied, VoiceError, VoiceErrorKind};
use oku_slides_session::{SpeechPatch, SpeechStatusView, SpeechView, SpeechVoiceView};
use oku_slides_stt::{Speaker, SpeechConfig, SpeechRate, SpeechStatus, Stt, UnavailableReason};

use super::{AppVoice, refused};

/// The read-aloud's status for `lang`: its voices, or why there is none.
fn status_view(speaker: Option<&Speaker>, lang: &str, l: &oku_slides_i18n::Localizer) -> SpeechStatusView {
    let status = speaker.map_or(SpeechStatus::Unavailable { reason: UnavailableReason::NoEngine }, |s| s.status(lang));
    match status {
        SpeechStatus::Ready { voices } => SpeechStatusView::Ready {
            voices: voices.into_iter().map(|v| SpeechVoiceView { id: v.id, name: v.name }).collect(),
        },
        SpeechStatus::Unavailable { reason } => SpeechStatusView::Unavailable { reason: reason.localize(l) },
    }
}

/// The settings `patch` leads to from `read_aloud` and `old`, checked against `status`.
///
/// # Errors
///
/// `BadRate` outside 0.5 to 2, `UnknownVoice` for a voice the language lacks, and
/// `SpeechUnavailable` to turn it on, or pick a voice, without any voice.
pub(super) fn check_speech(
    patch: &SpeechPatch,
    status: &SpeechStatusView,
    read_aloud: Option<bool>,
    old: &SpeechConfig,
) -> Result<(Option<bool>, SpeechConfig), VoiceError> {
    let mut new = old.clone();
    if let Some(rate) = patch.rate {
        new.rate = SpeechRate::new(rate).map_err(|_| refused(VoiceErrorKind::BadRate))?.get();
    }
    let voices = match status {
        SpeechStatusView::Ready { voices } => Some(voices),
        SpeechStatusView::Unavailable { .. } => None,
    };
    match patch.voice.as_deref() {
        None => {}
        Some("") => new.voice = None,
        Some(id) => {
            let voices = voices.ok_or_else(|| refused(VoiceErrorKind::SpeechUnavailable))?;
            if !voices.iter().any(|v| v.id == id) {
                return Err(refused(VoiceErrorKind::UnknownVoice));
            }
            new.voice = Some(id.to_owned());
        }
    }
    if let Some(on) = patch.in_presentation {
        new.in_presentation = on;
    }
    let read_aloud = match patch.read_aloud {
        Some(true) if voices.is_none() => return Err(refused(VoiceErrorKind::SpeechUnavailable)),
        Some(on) => Some(on),
        None => read_aloud,
    };
    Ok((read_aloud, new))
}

impl AppVoice {
    /// The read-aloud as the box shows it; `None` without dictation, where it is saved.
    pub(super) fn speech_view(&self) -> Option<SpeechView> {
        let stt = self.stt.as_ref()?;
        let (read_aloud, config) = stt.speech_settings();
        let l = self.localizer();
        let status = status_view(self.speaker.as_ref(), l.locale(), &l);
        Some(SpeechView {
            read_aloud: read_aloud == Some(true),
            voice: config.voice,
            rate: config.rate,
            in_presentation: config.in_presentation,
            status,
        })
    }

    /// Checks `patch` on the saved settings; nothing to confirm.
    pub(super) fn check_speech(
        &self,
        patch: &SpeechPatch,
    ) -> Result<(Checked, Option<bool>, SpeechConfig), VoiceError> {
        let stt = self.stt()?;
        let (read_aloud, old) = stt.speech_settings();
        let l = self.localizer();
        let status = status_view(self.speaker.as_ref(), l.locale(), &l);
        let (now, new) = check_speech(patch, &status, read_aloud, &old)?;
        let unchanged = now == read_aloud && new == old;
        Ok((Checked { unchanged, ..Checked::default() }, now, new))
    }

    /// Checks `patch` again and saves it; listening goes on.
    pub(super) fn apply_speech(&self, patch: &SpeechPatch) -> Result<VoiceApplied, VoiceError> {
        let stt = self.stt()?;
        {
            let _edit = self.edit.lock().unwrap_or_else(PoisonError::into_inner);
            let (checked, read_aloud, config) = self.check_speech(patch)?;
            if checked.unchanged {
                return Ok(VoiceApplied::Unchanged);
            }
            stt.configure_speech(read_aloud, config);
        }
        self.bump();
        Ok(VoiceApplied::Now)
    }
}

/// Hands-free just turned on: the read-aloud too, the first time only.
///
/// Set even with no voice yet, unlike `set_speech`: the switch is remembered for when one
/// appears, and the speaker skips each speech meanwhile.
pub(super) fn follow_hands_free_on(stt: &Stt) {
    let (read_aloud, config) = stt.speech_settings();
    let next = oku_slides_session::read_aloud_after_hands_free(read_aloud, true);
    if next != read_aloud {
        stt.configure_speech(next, config);
    }
}

#[cfg(test)]
#[path = "voice_port_speech_tests.rs"]
mod voice_port_speech_tests;
