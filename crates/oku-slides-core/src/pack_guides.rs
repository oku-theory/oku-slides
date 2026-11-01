//! The agent's guides of a pack or theme version: how to edit it, how to use it.
//!
//! Stored in the version's manifest (`guides`), skipped when empty so a version
//! without guides keeps its digest. Text is stored as written; the session
//! sanitizes it when it sends it to the agent.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Which of a version's two guides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GuideKind {
    /// How to edit the pack or theme: read by the agent of its pack or theme tab.
    Edit,
    /// How to use it: read by the agent of a deck that pins it.
    Use,
}

impl GuideKind {
    /// The guide's name, as serde writes it: `edit` or `use`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Edit => "edit",
            Self::Use => "use",
        }
    }
}

/// The agent's guides of a version: how to edit it, how to use it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Guides {
    /// How to edit it; empty: none.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub edit: String,
    /// How to use it; empty: none.
    #[serde(default, rename = "use", skip_serializing_if = "String::is_empty")]
    pub use_: String,
}

impl Guides {
    /// True when both guides are empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.edit.is_empty() && self.use_.is_empty()
    }

    /// The text of guide `which`.
    #[must_use]
    pub fn get(&self, which: GuideKind) -> &str {
        match which {
            GuideKind::Edit => &self.edit,
            GuideKind::Use => &self.use_,
        }
    }

    /// Replaces guide `which` with `text`.
    pub fn set(&mut self, which: GuideKind, text: String) {
        match which {
            GuideKind::Edit => self.edit = text,
            GuideKind::Use => self.use_ = text,
        }
    }
}

#[cfg(test)]
#[path = "pack_guides_tests.rs"]
mod pack_guides_tests;
