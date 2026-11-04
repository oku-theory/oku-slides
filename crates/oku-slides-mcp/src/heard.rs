//! An utterance as the hub sees it: no dependency on the dictation crate.

use std::time::Instant;

/// A closed hands-free utterance, as the host voice router hands it on.
///
/// Its text is never logged; logs carry `seq` only.
#[derive(Debug, Clone)]
pub struct HeardUtterance {
    /// Its number, never reused (the dictation's `UtteranceSeq`).
    pub seq: u64,
    /// What was said, trimmed and never empty.
    pub text: String,
    /// When its first phrase began.
    pub started: Instant,
}
