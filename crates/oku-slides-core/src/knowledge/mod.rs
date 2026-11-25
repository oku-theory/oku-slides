//! The deck knowledge bundle: Open Knowledge Format files kept in the deck.

mod assets;
mod frontmatter;
pub mod limits;
mod normalize;
mod path;

pub use assets::{ASSETS_DIR, cited_assets};
pub use frontmatter::{Frontmatter, check_text, frontmatter, split};
pub use normalize::normalize;
pub(crate) use normalize::repair;
pub use path::{KnowledgePath, is_file_id, knowledge_file_id};

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
