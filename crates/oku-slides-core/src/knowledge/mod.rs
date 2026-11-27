//! The deck knowledge bundle: Open Knowledge Format files kept in the deck.

mod assets;
mod extract;
mod frontmatter;
mod issues;
pub mod limits;
mod normalize;
mod path;
mod sanitize;
mod stamp;

pub use assets::{ASSETS_DIR, cited_images};
pub use extract::{
    Cut, CutKind, DocumentType, ExtractLimits, ExtractRefusal, Extracted, ExtractedImage, MAX_TITLE_CHARS, RASTERS,
};
pub use frontmatter::{Frontmatter, check_text, frontmatter, scalar_mapping, split};
pub use issues::DropCause;
pub(crate) use normalize::repair;
pub use normalize::{knowledge_over_limits, normalize};
pub use path::{KnowledgePath, is_file_id, knowledge_file_id};
pub use sanitize::{clean_text, escape_inline, escape_specials, is_clean_text};
#[cfg(any(test, feature = "test-support"))]
pub use stamp::unstamped;
pub use stamp::{Provenance, json_string, stamp, unstamped_without_keys};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One file of the knowledge bundle: its bundle path and its text, stored byte for byte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgeFile {
    /// Bundle-absolute path (`/a/b.md`).
    pub path: String,
    /// Content, verbatim.
    pub text: String,
}
