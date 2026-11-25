//! What the agent holds of the blocks sent once, then again on change: guides and knowledge index.
//!
//! Each block remembers the ACP session it last went to with the digest of its content, so a
//! turn on another session (a new thread, the local agent standing in, the main agent back)
//! gets it again. A failed `session/prompt` forgets both ([`Session::forget_sent_blocks`]):
//! what it carried may never have reached the agent.

use crate::Session;

/// A block's last send: the ACP session it went to and the digest of its content.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Sent {
    session: String,
    digest: [u8; 32],
}

/// The last send of one block; empty when none was sent or it was forgotten.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SentBlock(Option<Sent>);

impl SentBlock {
    /// True if ACP session `session` holds the block of `digest`.
    pub(crate) fn holds(&self, session: &str, digest: &[u8; 32]) -> bool {
        self.0.as_ref().is_some_and(|s| s.session == session && s.digest == *digest)
    }

    /// The block of `digest` went to ACP session `session`.
    pub(crate) fn record(&mut self, session: &str, digest: [u8; 32]) {
        self.0 = Some(Sent { session: session.to_owned(), digest });
    }

    /// The digest last sent, whichever session it went to.
    pub(crate) fn digest(&self) -> Option<&[u8; 32]> {
        self.0.as_ref().map(|s| &s.digest)
    }

    /// The session holding the block now holds `digest`: the agent wrote that change itself.
    pub(crate) fn redigest(&mut self, digest: [u8; 32]) {
        if let Some(s) = &mut self.0 {
            s.digest = digest;
        }
    }
}

/// The blocks a tab's agent was sent, each with its session and digest.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SentBlocks {
    /// The guides block (`agent/guides.rs`).
    pub(crate) guides: SentBlock,
    /// The knowledge index block (`agent/knowledge_index.rs`).
    pub(crate) knowledge: SentBlock,
}

impl Session {
    /// Forgets every block sent: the next turn sends the guides and the knowledge index again.
    ///
    /// Called when a `session/prompt` request fails, its blocks perhaps never received.
    pub fn forget_sent_blocks(&mut self) {
        self.sent_blocks = SentBlocks::default();
    }
}

#[cfg(test)]
#[path = "sent_blocks_tests.rs"]
mod sent_blocks_tests;
