//! Results of a continuous engine applied to the dictation text.

use crate::clean;
use crate::num::samples;
use crate::stream::Event;
use crate::utterance::{Phrases, Spoken};

/// Applies a result; returns `false` if the event is not one.
///
/// A partial text replaces the preview; a final text joins the phrases and clears the
/// preview. A final is placed on the listening clock: its stream times minus `stream_base`
/// samples. One that began before `stream_base` (audio from before a resume) is dropped.
pub(crate) fn apply(phrases: &mut Phrases, partial: &mut String, event: &Event, stream_base: usize) -> bool {
    match event {
        Event::Partial(text) => {
            *partial = clean(text);
            true
        }
        Event::Final { text, start, end } => {
            let text = clean(text);
            let start = samples(*start).checked_sub(stream_base);
            let end = samples(*end).checked_sub(stream_base);
            if let (false, Some(start), Some(end)) = (text.is_empty(), start, end) {
                let spoken = Spoken { text: Some(text), start, end: end.max(start) };
                if phrases.push(spoken).is_none() {
                    tracing::error!("phrase indices exhausted: final dropped");
                }
            }
            partial.clear();
            true
        }
        _ => false,
    }
}

#[cfg(test)]
#[path = "results_tests.rs"]
mod results_tests;
