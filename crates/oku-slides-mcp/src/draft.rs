//! The user's request, passed by `open_pack` or `open_theme` to the opened tab's draft.
//!
//! The request is agent-written data: it only waits in that tab's composer, for the user to
//! review and send. Nothing here starts a turn.

/// The longest request passed to a draft, in UTF-8 bytes; a longer one is never truncated.
pub const MAX_REQUEST_BYTES: usize = 2048;

/// Implement this trait to let a deck's agent prefill another tab's draft.
///
/// The host sets the draft of tab `tab` to `text` only when that draft is empty; it never
/// starts the tab's agent nor sends the text.
pub trait DraftPort: Send + Sync + std::fmt::Debug {
    /// Sets `tab`'s draft to `text` unless it already holds something.
    ///
    /// Blocking (it may start the tab's entry): call it off the async threads.
    fn prefill(&self, tab: &str, text: String) -> Prefill;
}

/// What a [`DraftPort::prefill`] did to the target tab's draft.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prefill {
    /// The draft was empty and now holds the request.
    Set,
    /// The draft already held text or images: it is kept as is.
    Kept,
    /// No such tab, or no port wired: nothing was set.
    Unavailable,
}

/// A port without drafts, for tests that never prefill one.
#[cfg(any(test, feature = "test-support"))]
#[derive(Debug, Default)]
pub struct NoDrafts;

#[cfg(any(test, feature = "test-support"))]
impl DraftPort for NoDrafts {
    fn prefill(&self, _: &str, _: String) -> Prefill {
        Prefill::Unavailable
    }
}

/// The `draft` field of an open's result: the prefill's outcome, or a request over the cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DraftOutcome {
    /// See [`Prefill::Set`].
    Set,
    /// See [`Prefill::Kept`].
    Kept,
    /// Over [`MAX_REQUEST_BYTES`]: the port was not called.
    TooLong,
    /// See [`Prefill::Unavailable`].
    Unavailable,
}

impl DraftOutcome {
    /// Its name in the result and the note's `$draft` selector.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Set => "set",
            Self::Kept => "kept",
            Self::TooLong => "too_long",
            Self::Unavailable => "unavailable",
        }
    }
}

impl From<Prefill> for DraftOutcome {
    fn from(p: Prefill) -> Self {
        match p {
            Prefill::Set => Self::Set,
            Prefill::Kept => Self::Kept,
            Prefill::Unavailable => Self::Unavailable,
        }
    }
}

/// The request, cleaned: control characters (but `\n` and `\t`) and bidi overrides dropped,
/// then trimmed; `None` when nothing is left.
pub(crate) fn normalize_request(request: Option<String>) -> Option<String> {
    let kept: String = request?.chars().filter(|&c| !dropped(c)).collect();
    let trimmed = kept.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

/// A control character other than `\n` and `\t`, or a bidi embedding, override or isolate.
fn dropped(c: char) -> bool {
    (c.is_control() && c != '\n' && c != '\t')
        || ('\u{202a}'..='\u{202e}').contains(&c)
        || ('\u{2066}'..='\u{2069}').contains(&c)
}

/// The request fits a draft: at most [`MAX_REQUEST_BYTES`] bytes.
pub(crate) fn fits(request: &str) -> bool {
    request.len() <= MAX_REQUEST_BYTES
}

#[cfg(test)]
#[path = "draft_tests.rs"]
mod draft_tests;
