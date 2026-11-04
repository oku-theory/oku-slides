//! What the read-aloud says for a surface, built where the surface is built.
//!
//! Only host templates (`voice-say-*`) and cleaned pieces: each piece through `bounds::label`
//! (options) or whitespace-collapsed and clipped (titles, facts, questions), the whole clipped
//! to [`SPOKEN_MAX`] characters, isolation marks and hidden controls removed. A permission or a
//! guarded confirmation reads a template alone (and the verb to say), never agent text nor the
//! confirmation's message, which may name a tab or hold a path. Never logged.

use oku_slides_i18n::Localizer;

use super::bounds::{NOTE_CHARS, clip, label};
use super::receipt::FACTS;

/// The longest text read aloud for one surface, in characters.
pub const SPOKEN_MAX: usize = 400;

/// What kind of surface a spoken text reads; a question cuts the speech before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpokenKind {
    /// A turn's receipt: its title and facts.
    Receipt,
    /// A question, confirmation or permission as it opens.
    Question,
    /// An error shown to the user.
    Error,
}

/// Why a turn failed, as the read-aloud says it: a host sentence with no arguments.
///
/// Built from the error's variant, never from its message, which may hold agent text or a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpokenFailure {
    /// The agent refused the request.
    Refused,
    /// The agent stopped.
    Stopped,
    /// The agent could not start.
    NoStart,
    /// The tab's tools are unavailable.
    NoTools,
}

impl SpokenFailure {
    /// Its host template's Fluent key.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Refused => "voice-say-agent-refused",
            Self::Stopped => "voice-say-agent-stopped",
            Self::NoStart => "voice-say-agent-no-start",
            Self::NoTools => "voice-say-agent-no-tools",
        }
    }
}

/// A text the read-aloud may say, in the language it was built in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpokenText {
    text: String,
    lang: String,
    kind: SpokenKind,
}

/// `text` on one line, without hidden controls nor isolation marks, at most `max` characters.
///
/// `[[` and `]]` are dropped: macOS reads `[[slnc 500]]` as a command to its synthesizer.
fn piece(text: &str, max: usize) -> String {
    let mut text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    while text.contains("[[") || text.contains("]]") {
        text = text.replace("[[", "[").replace("]]", "]");
    }
    clip(&text, max)
}

impl SpokenText {
    fn new(text: &str, kind: SpokenKind, l: &Localizer) -> Self {
        Self { text: piece(text, SPOKEN_MAX), lang: l.locale().to_owned(), kind }
    }

    /// A receipt's title, then its first facts (four at most); `None` without a title.
    #[must_use]
    pub fn receipt(title: &str, facts: &[String], l: &Localizer) -> Option<Self> {
        let title = piece(title, NOTE_CHARS);
        if title.is_empty() {
            return None;
        }
        let facts = facts.iter().map(|f| piece(f, NOTE_CHARS)).filter(|f| !f.is_empty()).take(FACTS.1);
        let lines: Vec<String> = std::iter::once(title).chain(facts).collect();
        Some(Self::new(&lines.join(". "), SpokenKind::Receipt, l))
    }

    /// A question and its options, numbered in their order so an ordinal answer makes sense.
    #[must_use]
    pub fn question(question: &str, options: &[&str], l: &Localizer) -> Self {
        let mut parts = vec![l.tr_with("voice-say-question", &[("question", piece(question, NOTE_CHARS).into())])];
        for (n, option) in (1u64..).zip(options) {
            parts.push(l.tr_with("voice-say-option", &[("n", n.into()), ("option", label(option).into())]));
        }
        Self::new(&parts.join(" "), SpokenKind::Question, l)
    }

    /// The agent's own yes-or-no question, then how to answer it.
    ///
    /// `verb` is the danger button's label, host text; `None` for a plain yes or no.
    #[must_use]
    pub fn agent_confirm(question: &str, verb: Option<&str>, l: &Localizer) -> Self {
        let question = l.tr_with("voice-say-question", &[("question", piece(question, NOTE_CHARS).into())]);
        Self::new(&format!("{question} {}", Self::confirm(verb, l).text), SpokenKind::Question, l)
    }

    /// A guarded confirmation: the template and the verb to say, never its message.
    ///
    /// `verb` is the danger button's label, host text; `None` for a plain yes or no.
    #[must_use]
    pub fn confirm(verb: Option<&str>, l: &Localizer) -> Self {
        let text = match verb {
            Some(verb) => l.tr_with("voice-say-confirm", &[("verb", label(verb).into())]),
            None => l.tr("voice-say-confirm-yes"),
        };
        Self::new(&text, SpokenKind::Question, l)
    }

    /// An ACP permission: the template alone, never the agent's title nor its option labels.
    #[must_use]
    pub fn permission(l: &Localizer) -> Self {
        Self::new(&l.tr("voice-say-permission"), SpokenKind::Question, l)
    }

    /// A turn's failure: its host template alone, never the error's message.
    #[must_use]
    pub fn error(failure: SpokenFailure, l: &Localizer) -> Self {
        Self::new(&l.tr(failure.key()), SpokenKind::Error, l)
    }

    /// The text to say.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The language it was built in, a shipped locale code.
    #[must_use]
    pub fn lang(&self) -> &str {
        &self.lang
    }

    /// What it reads.
    #[must_use]
    pub fn kind(&self) -> SpokenKind {
        self.kind
    }
}

#[cfg(test)]
#[path = "spoken_tests.rs"]
mod spoken_tests;
