//! Self-echo: what the microphone heard of Oku's own recent speech.
//!
//! The deaf window ([`crate::Speaker::hears`]) drops what began while Oku spoke; an echo that
//! a slow engine or a reverberant room places just after it is caught here by its words.
//! [`Recent`] keeps the folded text of the last [`RECENT`] speeches in memory only: it is never
//! logged nor shown, not even through `Debug`.

use std::collections::VecDeque;
use std::fmt;
use std::time::{Duration, Instant};

use oku_slides_i18n::fold;

use crate::speak::SpeechId;
use crate::speaker::later;

/// How long after a speech ended an utterance that repeats it is still taken for its echo.
pub const ECHO_HORIZON: Duration = Duration::from_secs(3);

/// How many of the last speeches an utterance is compared with.
const RECENT: usize = 3;

/// The fewest words of a spaced script an echo repeats: a one-word reply is never an echo.
const MIN_WORDS: usize = 2;

/// The fewest characters of an unspaced script an echo repeats.
const MIN_CHARS: usize = 3;

/// True for a sign of a script written without spaces between words: CJK or Thai.
fn unspaced_char(c: char) -> bool {
    matches!(c,
        '\u{0E00}'..='\u{0E7F}'     // Thai
        | '\u{3040}'..='\u{30FF}'   // Hiragana, Katakana
        | '\u{3400}'..='\u{4DBF}'   // CJK Extension A
        | '\u{4E00}'..='\u{9FFF}'   // CJK Unified Ideographs
        | '\u{F900}'..='\u{FAFF}'   // CJK Compatibility Ideographs
        | '\u{FF66}'..='\u{FF9F}'   // Halfwidth Katakana
        | '\u{20000}'..='\u{2FA1F}' // CJK Extensions B to F, Compatibility Supplement
    )
}

/// True when `text` holds a sign of a script written without spaces (CJK, Thai).
pub(crate) fn unspaced(text: &str) -> bool {
    text.chars().any(unspaced_char)
}

/// The words of `folded`, split on everything but letters and digits.
fn words(folded: &str) -> Vec<&str> {
    folded.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect()
}

/// The letters and digits of `folded`, spaces and punctuation left out.
fn signs(folded: &str) -> Vec<char> {
    folded.chars().filter(|c| c.is_alphanumeric()).collect()
}

/// True when `said`, at least `min` long, is whole a run of `spoken`.
fn within<T: PartialEq>(said: &[T], spoken: &[T], min: usize) -> bool {
    said.len() >= min.max(1) && spoken.windows(said.len()).any(|run| run == said)
}

/// True when `said` repeats, in a row, a run of `spoken`, both case and accent folded.
///
/// Spaced scripts compare words: all of `said`, at least [`MIN_WORDS`], in a row in `spoken`.
/// Unspaced scripts (CJK, Thai) compare signs: all of `said`, at least [`MIN_CHARS`], within
/// `spoken`. A single word is never an echo, even of a one-word speech. Speeches that ask
/// the user something are never compared ([`crate::Speech::asks`]): their words are answers;
/// nor are those read under barge-in, whose echo the echo canceller removes.
pub(crate) fn repeats(said: &str, spoken: &str) -> bool {
    let (said, spoken) = (fold(said), fold(spoken));
    if unspaced(&said) || unspaced(&spoken) {
        within(&signs(&said), &signs(&spoken), MIN_CHARS)
    } else {
        within(&words(&said), &words(&spoken), MIN_WORDS)
    }
}

/// One recent speech: its folded text and when it ended (`None` while it plays).
struct Spoken {
    id: SpeechId,
    folded: String,
    end: Option<Instant>,
}

/// The last speeches read aloud, newest last, to recognise their echo.
#[derive(Default)]
pub(crate) struct Recent(VecDeque<Spoken>);

impl fmt::Debug for Recent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Spoken text is never logged: only how many speeches are kept.
        f.debug_tuple("Recent").field(&self.0.len()).finish()
    }
}

impl Recent {
    /// Keeps speech `id` of `text`, playing, forgetting the oldest beyond [`RECENT`].
    pub(crate) fn started(&mut self, id: SpeechId, text: &str) {
        if self.0.len() == RECENT {
            self.0.pop_front();
        }
        self.0.push_back(Spoken { id, folded: fold(text), end: None });
    }

    /// Forgets every speech kept.
    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }

    /// Speech `id` ended at `at`.
    pub(crate) fn ended(&mut self, id: SpeechId, at: Instant) {
        if let Some(s) = self.0.iter_mut().find(|s| s.id == id) {
            s.end = Some(at);
        }
    }

    /// True when `said`, begun at `started`, repeats a speech playing or ended within the horizon.
    pub(crate) fn echoes(&self, said: &str, started: Instant) -> bool {
        self.0.iter().any(|s| {
            let near = s.end.is_none_or(|end| started <= later(end, ECHO_HORIZON));
            near && repeats(said, &s.folded)
        })
    }
}

#[cfg(test)]
#[path = "echo_tests.rs"]
mod echo_tests;
