//! Applying a dictation patch, and hands-free listening in it: the user turns it on, the agent only off.
//!
//! A prompt injection must not be able to start an always-on microphone, so the agent's
//! `set_dictation` may only turn it off. The setting is saved, but no launch starts listening
//! by itself: listening begins at a user gesture (the box switch, the indicator, ⌘D).

use std::sync::PoisonError;

use oku_slides_mcp::{Origin, VoiceApplied, VoiceError, VoiceErrorKind};
use oku_slides_session::DictationPatch;
use oku_slides_stt::{Config, State, Stt};

use super::{AppVoice, check_patch, confirmed, refused};

/// Applies `patch.hands_free` to `new`; the agent may only turn it off.
///
/// # Errors
///
/// `HandsFreeReservedToUser` when the agent asks to turn it on.
pub(super) fn check_hands_free(patch: &DictationPatch, agent: bool, new: &mut Config) -> Result<(), VoiceError> {
    match patch.hands_free {
        Some(true) if agent => Err(VoiceError::new(VoiceErrorKind::HandsFreeReservedToUser)),
        Some(on) => {
            new.hands_free = on;
            Ok(())
        }
        None => Ok(()),
    }
}

/// Once settings apply, listening follows the switch: on starts it, off stops it.
///
/// Only a user can turn it on ([`check_hands_free`]): the box switch is the gesture.
fn follow_hands_free(stt: &Stt, was: bool, now: bool) {
    match (was, now) {
        (false, true) => stt.listen(Vec::new()),
        (true, false) if stt.status().state == State::Listening => stt.stop(),
        _ => {}
    }
}

impl AppVoice {
    /// Checks `patch` again, applies and saves it, then lets listening follow the switch.
    pub(super) fn apply_patch(&self, patch: &DictationPatch, origin: &Origin) -> Result<VoiceApplied, VoiceError> {
        let stt = self.stt()?;
        let (applied, was, now) = {
            let _edit = self.edit.lock().unwrap_or_else(PoisonError::into_inner);
            let status = stt.status();
            let (checked, config) = check_patch(&status, patch, origin)?;
            confirmed(&checked, origin)?;
            if checked.unchanged {
                return Ok(VoiceApplied::Unchanged);
            }
            let (was, now) = (status.config.hands_free, config.hands_free);
            let applied = stt.configure(config);
            // Under the same edit lock, so no concurrent speech change is lost.
            if applied && !was && now {
                super::speech::follow_hands_free_on(stt);
            }
            (applied, was, now)
        };
        if !applied {
            return Err(refused(VoiceErrorKind::Busy));
        }
        follow_hands_free(stt, was, now);
        self.bump();
        Ok(VoiceApplied::Now)
    }
}

#[cfg(test)]
#[path = "voice_port_hands_free_tests.rs"]
mod voice_port_hands_free_tests;
