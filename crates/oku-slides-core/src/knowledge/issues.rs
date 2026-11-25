//! The issues a knowledge read reports: one constructor per problem, on [`Issue`].
//!
//! Every peer-controlled string (an id, a frontmatter key) goes through `peer_text`.

use oku_slides_i18n::Message;

use crate::error::{FrontmatterReason, KnowledgeLimit, PathReason};
use crate::validate::{Issue, peer_text};

/// Why entries of a forged replica were dropped at the read ceiling ([`super::limits::READ_FACTOR`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropCause {
    /// Past the most files read.
    Files,
    /// A file text past the most bytes read for one file.
    FileBytes,
    /// Past the most bytes read for the whole bundle.
    TotalBytes,
    /// Unreadable entries past those reported one by one.
    Unreadable,
}

impl DropCause {
    /// The Fluent select value of the cause in `issue-knowledge-entries-dropped`.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            DropCause::Files => "files",
            DropCause::FileBytes => "file-bytes",
            DropCause::TotalBytes => "total-bytes",
            DropCause::Unreadable => "unreadable",
        }
    }
}

impl Issue {
    /// A knowledge entry that does not read as a file: malformed id, value or container.
    #[must_use]
    pub fn knowledge_unreadable(id: &str) -> Self {
        let message = Message::new("issue-knowledge-unreadable").arg("id", peer_text(id));
        Self { slide: None, element: None, message }
    }

    /// The bundle passes the ceiling `limit`: `count` files are past it, the first being
    /// `first`. Every file is kept.
    #[must_use]
    pub fn knowledge_over_limit(limit: KnowledgeLimit, first: &str, count: usize) -> Self {
        let message = Message::new("issue-knowledge-over-limit")
            .arg("limit", limit.key())
            .arg("id", peer_text(first))
            .arg("count", count);
        Self { slide: None, element: None, message }
    }

    /// A knowledge file whose invalid path was replaced by `/recovered/<id>.md`.
    #[must_use]
    pub fn knowledge_path_invalid(id: &str, reason: PathReason) -> Self {
        let message = Message::new("issue-knowledge-path-invalid").arg("id", peer_text(id)).arg("reason", reason.key());
        Self { slide: None, element: None, message }
    }

    /// Two knowledge files on `path`: the one with the higher id was moved.
    #[must_use]
    pub fn knowledge_path_collision(path: &str) -> Self {
        Self { slide: None, element: None, message: Message::new("issue-knowledge-path-collision").arg("path", path) }
    }

    /// A knowledge file kept although its text breaks a frontmatter rule.
    #[must_use]
    pub fn knowledge_invalid(path: &str, reason: &FrontmatterReason) -> Self {
        let message = Message::new("issue-knowledge-invalid")
            .arg("path", path)
            .arg("reason", reason.key())
            .arg("key", peer_text(reason.subject().unwrap_or_default()));
        Self { slide: None, element: None, message }
    }

    /// `count` entries of a forged replica dropped unread at the read ceiling, for `cause`.
    #[must_use]
    pub fn knowledge_entries_dropped(cause: DropCause, count: usize) -> Self {
        let message = Message::new("issue-knowledge-entries-dropped").arg("cause", cause.key()).arg("count", count);
        Self { slide: None, element: None, message }
    }
}

#[cfg(test)]
#[path = "issues_tests.rs"]
mod issues_tests;
