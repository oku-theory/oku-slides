//! Hard ceilings of the deck knowledge bundle; core owns them, configuration may only lower them.

/// Most files in a bundle.
pub const MAX_FILES: usize = 1024;
/// Most bytes in one file.
pub const MAX_FILE_BYTES: usize = 1 << 20;
/// Most bytes in the whole bundle.
pub const MAX_TOTAL_BYTES: usize = 16 << 20;
/// Most bytes in a frontmatter block.
pub const MAX_FRONTMATTER_BYTES: usize = 16 << 10;
/// Deepest nesting of a frontmatter YAML value.
pub const MAX_YAML_DEPTH: usize = 16;
/// Most bytes in a knowledge path.
pub const PATH_MAX_BYTES: usize = 255;
/// Most segments in a knowledge path.
pub const PATH_MAX_SEGMENTS: usize = 8;
